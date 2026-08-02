# The foreign interface

How a gandr program **names, types, and calls foreign C-ABI code**, and the discipline that makes such a call auditable rather than an escape hatch out of the type system.

The organising observation, and the reason gandr needs no bespoke foreign-function subsystem: **a foreign function is already exactly what gandr calls a tool** — an effect operation whose handler happens to be a native call rather than a gandr clause.
The entire safety model is therefore the effect, capability, and linearity discipline gandr already has, pointed at C. Interception, sandboxing, least authority, and inspectability are **inherited, not built**.

The surface syntax of the `extern` block is [[../surface-language/declarations#extern blocks]]; this document owns the safety model, the boundary type mapping, the two execution paths, and the requirements the boundary imposes on the compilation backend.

## As built, and what is missing

Verified against the tree at write time, with the module named at each claim.

**Built.** The `extern` block parses and lowers.
`gandr-surface-grammar` carries the `extern_block`, `extern_type`, and `extern_function` node kinds; `gandr-surface-engine`'s `lower` module collects every block into a **foreign-module registry** keyed by namespace in a pre-pass before ordinary item lowering, so a block is a _declaration_ and never a runnable item, and the registry **persists across REPL submissions** exactly as definitions do.
`extern_function` lowers each bodiless signature to a `ForeignFn` of `ForeignParam`s, and `boundary_ctype` maps each parameter and result through the boundary: the six numeric atoms to their `CType` counterparts by identity, `CStr` to a string copy, a block-declared opaque handle type to a pointer, and unit to a void slot — with every composite type (functions, products, sums, records, struct-by-value) **rejected at lowering** as outside the boundary.
A call `m.op(args)` on an `extern`-declared module elaborates through the host-effect seam, and an `extern`-declared module **shadows** a host module of the same name.

**Not built: the native handler.** There is no foreign-call runtime crate in the workspace.
`gandr-surface-corpus`'s harness recognizes an `ffi` execution mode with its `expect-ffi-value` and `expect-ffi-error` directives and a corpus feature flag, but its `check_ffi` is deliberately inert: the FFI-mode sources are **kept in the frozen corpus until the runtime crate lands**, so their bytes and directive shape stay covered by the corpus gates while **execution is unavailable**.

**So the honest state is: the boundary is declared and type-checked; nothing crosses it yet.** Everything below marked as adopted is a design position with a landed surface and no runtime; everything marked as growth is a design position with neither.

## The design space

The axes a foreign-interface design must fix, and the tension on each.

| axis                | the choice to make                          | the tension                                                                                                               |
| ------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| declaration surface | where foreign symbols are named and typed   | must fit the brace-delimited grammar and the module layer without a grammar conflict                                      |
| trust model         | what makes a foreign call safe to admit     | the thesis is that types pay at the **authority** layer; an untyped symbol-lookup escape hatch would negate it            |
| call representation | how a call threads the machine              | must be inspectable and serializable, and interceptable for mocking and sandboxing                                        |
| resource lifetime   | libraries, buffers, returned pointers       | foreign memory is unmanaged, and a loaded library's symbols must not outlive it — use after unload is undefined behaviour |
| boundary types      | how gandr values become C values            | gandr's canonical value representations are not C-compatible; the boundary is a copy or an opaque handle                  |
| failure model       | non-zero returns, foreign panics, unwinding | cross-language unwinding is undefined behaviour unless both frames agree, so the default must be abort                    |
| execution paths     | interpreter versus compiled backend         | one surface, two lowerings, which must agree                                                                              |

## The safety model

### A foreign call is an effect operation

Each `extern … from "m"` block elaborates to an **effect signature** named `m` with one operation per bodiless signature — its payload the argument record, its reply the return type — and a call `m.op(a)` elaborates to a `perform` of that operation.
This is the runtime host's hand-written signature-and-handler shape **generalized from hand-written to source-generated**: where the host hard-codes its signatures and dispatches a native handler on the operation name, an `extern` block _generates_ the signature from the declaration, and the foreign-call runtime _is_ the native handler.

Three properties fall out for free, and all three are load-bearing for gandr's agentic thesis rather than incidental:

* **Interception is mocking and sandboxing.** A source-level handler over `m` shadows the native handler — the same handler swap that gives the shell its sandbox story.
  A test runs a program against a pure gandr handler with **no library loaded at all**; a capability-restricted world denies the operation entirely, and the machine blames the unhandled `perform`.
* **The effect row is the honest type.** A foreign function's type carries a row recording that calling it reaches foreign code, so **purity is not silently lost**, and code that threads that row is ordinary row-polymorphic code.
* **The call is inspectable and serializable.** A `perform` is an ordinary node in the addressed arena, so the foreign call is a **visible machine step**, not an opaque host trap — the reified-machine posture extended to the boundary.

The soundness of installing a native handler is inherited from the host seam's: the handler is ambient and always-resume, which is sound while the linear zone is vacuous and resumption is multi-shot.
**When the linear zone is populated, the library-lifetime discipline below is what re-earns that soundness** — this is a debt with a named repayment, not an assumption.

### Loading a library is a capability

The gate distinguishes two cases, and only one of them is a runtime effect:

* **A statically named library** resolves at link or load time, and the program holds no runtime load capability at all.
  What is consumed is a **build-time** capability — which libraries this artifact may link — and that belongs to the package and build manager, not to the runtime.
* **A dynamically loaded library** is a runtime effect requiring an explicit load capability.
  A world that does not grant it **cannot load code**, which is least authority applied to the single most dangerous operation gandr has.

**The current cut ships only the static case**, with the capability collapsed to a link-manifest entry and no runtime loading, because runtime loading wants the linear lifetime discipline below and that discipline is not yet available.
This is a sequencing decision, not an omission.

### A loaded library's lifetime is a linear resource

A dynamically loaded library handle is a **linear resident of the linear zone**.
It _owns_ the symbols resolved from it: a foreign function pointer or opaque handle derived from it is **scoped to the library's lifetime**, and closing the library **consumes** the handle.

The payoff is exact: **use after unload — the sharpest undefined behaviour in dynamic foreign interfacing — becomes a linearity error**, caught by machinery gandr already has rather than by a bespoke check.
This is the cleanest fit in the whole design between a foreign-interface hazard and an existing gandr mechanism, and it is why the runtime-loading path **waits** rather than shipping an unsound intermediate.

### Foreign unwind aborts by default

The default foreign function does not unwind: if foreign code unwinds across the boundary — a C++ exception, a Rust panic, a `longjmp` — the runtime **aborts the process**.
This is the only sound default, because cross-language unwinding is undefined behaviour unless caller and callee frames were compiled to agree on the unwind mechanism.

Opt-in unwinding is a **per-function attribute**, and its status differs by path:

* on the **interpreter path** it is currently un-catchable, because the runtime-signature foreign-call library does not model foreign unwinding; the attribute is accepted and the runtime still aborts.
  **The attribute reserves the type ahead of the mechanism** — which is the point of accepting it at all.
* on the **compiled path** it can lower to a call form that catches, routing a caught unwind into a typed failure channel — **but the Windows structured-exception mechanism is not covered**, so opt-in unwinding on Windows aborts regardless.

The typed channel a caught unwind would land in is a deferred refinement of the unwinding rule: a foreign unwind is a context-erasing unwind that runs the captured frame destructors in reverse.
The named source for that refinement is Congard, Munch-Maccagnoni, and Douence's exceptional-unwind work presented at ESOP 2026; **that locator is carried from the design record and is unverified here**.
Until it lands, opt-in unwinding is type-reserved and abort-backed.

**The current cut has no opt-in unwinding at all.**

### Variadics force the interpreter path

A variadic function is **marked at its declaration**, because **variadic calls are unrepresentable in the compiler backend's IR** — the backend has no variadic call form.
The marker forces the runtime-signature path, which _can_ build a variadic call frame from a runtime type list.

Two consequences follow.
Variadic foreign functions run on the interpreter; and on an ahead-of-time path they must either be lowered to a trampoline into the runtime-signature library — a runtime dependency on that library even for compiled code — or refused.
**The marker exists so the backend detects the case at lowering time rather than failing in code generation**, which is the difference between a diagnostic and a crash.

## The boundary type mapping

**The governing rule: gandr's own value representations never cross.** Foreign code sees copies — scalars, buffers — or opaque handles, and **never a pointer into gandr's arena**, whose nodes may relocate and whose layout is not C-compatible.
**The boundary is a firewall, not a window.**

### Scalars map by identity

The six numeric atoms `u32`, `u64`, `i32`, `i64`, `f32`, and `f64` map **by identity** onto the C fixed-width scalar types.
The choice of fixed-width atoms was made for the value-model ladder rather than for this boundary, and its payoff here is an unplanned windfall: **the numeric boundary mapping is the identity function** — no width negotiation and no implicit conversion.

Consistent with the no-implicit-widening rule, the boundary **demands a concrete atom**: a default integer or an unsuffixed literal must be narrowed before it crosses.
As built, `boundary_ctype` enforces exactly this — the gradual integer type, the string type, the sub-word integers, and any undeclared type identifier are all rejected at lowering.

**The honest gap.** The six atoms lack `i8`, `u8`, `i16`, and `u16`; a pointer-width integer; and a boolean or C character type.
**A faithful C surface needs all of them.** The current boundary is the six widths plus pointers plus the C string type, and the missing scalars are a growth item coupled to the numeric-tower question.

### Strings cross as a null-terminated copy

A boundary type `CStr` bridges gandr's owned UTF-8 string and C's character pointer.

This is precisely the motivation on record for a reserved string representation: an **always-null-terminated owned form** would make the conversion zero-copy.
Until that representation lands, the boundary **copies**: allocate a null-terminated copy of the argument, pass the pointer, free after the call returns — **arguments are borrowed in, and C must not retain them past the call**.

Two rules with teeth:

* **An interior null is a boundary error**, checked at the copy.
  A character pointer would truncate at it and silently pass a shorter string than the program wrote, so it is rejected rather than passed.
* **A character pointer returned from C is an opaque pointer, not a gandr string**, until the program explicitly copies it in and declares who frees the C buffer.
  Auto-marshalling a returned pointer is deferred because **ownership of the returned buffer is undecidable from the type alone** — this is a named dead end, not an unfinished feature.

### Aggregates cross by pointer, and that is pinned early

**Only scalars and pointers cross by value.** Struct-by-value arguments and struct returns are deferred, and the rationale is pinned now precisely to prevent a later retrofit:

* gandr's record former has a **canonical sorted-field representation** that does **not** match C struct field order, so a record-to-struct mapping requires an **explicitly declared layout** — a field-order attribute — and never the record's native shape.
* Struct return is the hidden-pointer ABI corner, whose platform-specific classification — small structs in registers versus in memory — is exactly where hand-rolled foreign interfaces break.
  Getting it right needs the ABI-classification reference below, not a guess.

So the stance is fixed early and conservatively: **scalars and pointers by value, aggregates by pointer**, with the caller marshalling a record into a foreign buffer explicitly.
Struct-by-value is a named growth item with a trigger, not an omission to be improvised.

### Opaque handles, and ownership

A declared foreign type and any returned pointer is an **opaque handle**: an address-sized value tagged with its foreign-type atom.

**The current representation deliberately avoids the frozen core.** Following the same move the string type made — a value _term_ typed by a rigid _atom_, rather than a new value-*type* former — a foreign handle is carried as a machine word and typed by a rigid foreign atom, **not** a new kernel value-type former.
This needs no kernel format change, and the distinct atom is what **prevents a program forging a handle**: a raw machine word cannot be passed where a foreign atom is expected without going through the trusted boundary.

The **distinct foreign-handle value node** — carrying provenance, so the linear lifetime discipline can attach to the handle directly rather than to the library — is the growth, and _that_ is a kernel-format addition under the export-format discipline.

**Ownership at the boundary is explicit and never inferred.** Arguments are borrowed in; returned handles are unmanaged and the program frees them through another foreign call; **no automatic release runs at this rung**.
The linear handle is what makes automatic, checked release sound later.

## The two execution paths over one surface

One `extern` block, one effect signature, **two lowerings of the same `perform`**.

### The interpreter path

The native handler installed at the host seam.
It resolves symbols through the platform loader, builds each call from a **runtime type description derived from the `extern` signature** — so **no per-function code generation is needed and any function the block describes is callable immediately** — and is the **only** path that can call a variadic function.

This is the first path because it needs no backend: it rides the landed machine and the host seam, and its handler is the direct analogue of the runtime host's dispatch, keyed on signature name and then operation name and returning a resume-with-reply.

### The compiled path

The compiled path lowers the same `perform` to a **direct call to an imported symbol**, with the backend's module layer maintaining a symbol table into which foreign functions are declared as imports, resolved at link time with a symbol-lookup fallback for runtime-loaded libraries.

**The ABI work is real, and the reference is named.** The chosen backend does **not** implement the C ABI: the frontend must classify each argument per platform — register, stack, or hidden return pointer.
The complete, maintained implementation of C-ABI lowering over that backend is the Rust compiler's alternative code generator for it, and **that is the reference this path mines**.

**A false premise, recorded so it is not re-assumed.** The idea that small languages already use this backend for their C foreign interfaces — that there is a precedent to copy — is **false**.
Of the three languages usually named: one chose LLVM, one has a hand-written development backend with LLVM for optimized builds, and one compiles through a WebAssembly toolchain.
**There is no drop-in C foreign-interface layer to lift.**

The compiled path is a natural **differential** candidate against the interpreter path: a compiled call must agree with the interpreted call on the same program, which is the oracle discipline applied to the boundary.

## What the boundary requires of the compilation backend

The `extern` surface must be representable in the textual intermediate representation the backend lane targets.
These five requirements are stated here so they are **designed in rather than retrofitted**.

1. **External symbol declarations** — parameter scalar and pointer types, return type, calling convention, the variadic flag, and the unwind flag.
2. **Linkage metadata** — library name, symbol name, and a strong, weak, or dynamically-resolved linkage kind, riding the declaration; the direct-call versus runtime-resolve choice is made here.
3. **A hidden-return-pointer slot, forward-compatible.** Even though no struct crosses by value at this rung, the imported-signature representation must carry the slot **so the deferred struct path does not reshape the IR**.
   This composes with the multi-value returns the representation already models: a C single-value-or-hidden-pointer return is the **degenerate case** of the multi-value form, not a conflicting one.
4. **A variadic escape to a trampoline.** Because variadic calls are unrepresentable in the backend IR, the lowering must be able to emit **either** a direct call **or** a runtime trampoline call, so the representation must carry both call forms.
5. **Inspectable declaration nodes.** A foreign declaration must survive as a first-class, addressable node preserving source origin, so diagnostics and the derivation display can show the boundary.

## The cut, and the growth path

| facet      | the current cut                                                  | growth                                                                              | trigger                                 |
| ---------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------- | --------------------------------------- |
| surface    | `extern "c" from "lib" { type …; def …; }`, static library names | a runtime-load block; ABI strings beyond C                                          | runtime plugins; other ABIs             |
| call model | `perform` to a native handler, mockable                          | direct-call compiled path                                                           | the backend hardens                     |
| execution  | interpreter (runtime-signature calls plus a platform loader)     | backend symbol table, mining the named ABI reference                                | ahead-of-time need                      |
| scalars    | six atoms by identity, plus the C string type                    | `i8`, `u8`, `i16`, `u16`, a pointer-width integer, a boolean and a C character type | the numeric-tower decision              |
| aggregates | scalars and pointers only; records through an explicit buffer    | struct-by-value and hidden-pointer return with a declared layout attribute          | struct-heavy C APIs                     |
| handles    | machine-word-carried, foreign-atom-typed; no kernel change       | a distinct foreign-handle value node                                                | attaching the linear lifetime           |
| lifetime   | static libraries, process-lifetime load                          | a linear library handle with checked unload                                         | the linear zone populated               |
| unwind     | abort only                                                       | the attribute lowering to a catching call form and a typed channel — not on Windows | the exceptional-unwind refinement lands |
| variadics  | interpreter path only                                            | a trampoline linked into the compiled artifact                                      | ahead-of-time variadic need             |
| callbacks  | none                                                             | a gandr closure to C function-pointer trampoline                                    | callback-taking APIs                    |
| capability | a link-manifest entry                                            | a runtime load capability                                                           | runtime loading                         |

**The cut is deliberately the smallest thing that is sound**: static libraries, scalars and strings and handles, effect-mediated calls, abort on unwind, runtime-signature calls.
Everything that would require a kernel-format change, the linear zone, or the backend is growth **with a named trigger**.

## Interactions with the rest of the system

* **The string representation.** The null-terminated owned form is the zero-copy target, and the foreign boundary is its primary motivator.
  Until it lands, the boundary copies.
  See [[../surface-language/value-semantics#The cut, and the growth path]].
* **Self-hosting.** A self-hosted front end would bind its parser — a C library — through this boundary, making it the **capstone consumer** and the driving test case for the opaque-handle and lifetime model.
* **The host-effect seam.** The scoped-handler work proves the effect-signature-with-native-handler shape this generalizes; the `extern` block is that shape _source-generated_ rather than hand-written ([[../implementation#The runtime host]], [[capability-model]]).
* **The compilation backend.** The five requirements above are a direct input to the backend's intermediate-representation design.
* **Data declarations.** A foreign opaque type is the degenerate declared datatype — a sort with no constructors — and a declared C layout is a data-declaration attribute, so the struct-by-value growth path routes through that surface ([[../surface-language/declarations#data declarations]]).
* **Value semantics and modes.** "Arguments are borrowed in, returns are owned" is the **boundary instance of the mode calculus**, and the linear library handle is its exclusive-borrow case.
  The full boundary discipline, and the decisions it turns on — representation and ABI stability, ownership in the calling convention, foreign value representation, and the unsoundness of contraction on a non-trivially-copyable foreign type — are [[../surface-language/proposed/modes-and-references#Foreign-interface design impact]].
* **Attributes.** The unwind, variadic, and layout markers, the calling convention, and the link and symbol names are typed **attribute schemas** on foreign declarations ([[../surface-language/declarations#Attributes]]).
* **Modules and packaging.** Which libraries an artifact may link, and how they are found, is the package and build concern; the link manifest lives there.
* **A wasm target.** A foreign interface over wasm is **imports** — a different ABI with no dynamic loading, no runtime-signature call library, and no C struct classification.
  It is a **separate lowering**, not this path, and the ABI slot is where it grows.

## The corpus treatment

The sources are landed and **frozen**: their bytes and directives are covered by the corpus gates, and execution is unavailable until the runtime crate lands.
Two model examples and two failure goldens exist in the tree; the rest of the plan is recorded here so the graduation change knows what it owes.

| example                                    | status         | what it pins                                                                                                                                                                                |
| ------------------------------------------ | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `model/21-ffi-native-call.gandr`           | landed, frozen | two scalars in and one out plus a string argument, against a hermetic in-repo C fixture; the identity scalar map and the effect row                                                         |
| `model/22-ffi-effect-and-capability.gandr` | landed, runs   | a foreign module called in a world with **no handler installed**: the call type-checks cleanly and then blames the unhandled `perform` — capability denial made visible as a _defined_ halt |
| `pathological/ffi-interior-nul.gandr`      | landed, frozen | an interior null in a string argument is a typed boundary error, never a silent truncation                                                                                                  |
| `pathological/ffi-wrong-arity.gandr`       | landed, runs   | a declared arity the call disagrees with; under total lowering the mismatch becomes a typed hole reported as a goal, rather than a malformed call                                           |
| an effect-and-mock example                 | **owed**       | the same block run twice — once natively, once against a source-level mock handler with no library loaded                                                                                   |
| an opaque-handle lifecycle example         | **owed**       | open, use, and explicit release over a declared opaque type; ownership is explicit                                                                                                          |
| a use-after-close example                  | **owed**       | the pathology the linear lifetime must reject once that discipline lands                                                                                                                    |
| a foreign-unwind example                   | **owed**       | a non-unwinding declaration whose callee unwinds must abort, not corrupt                                                                                                                    |
| a variadic example                         | **owed**       | a variadic declaration runs on the interpreter and is refused or trampolined by the compiled path                                                                                           |
| a forged-handle example                    | **owed**       | passing a raw machine word where a foreign atom is expected must be a type error                                                                                                            |

Each owed example names the feature it waits on: the mock example waits on the runtime crate; the handle and use-after-close examples wait on the distinct foreign-handle atom and the linear zone; the unwind example waits on a genuinely unwinding fixture; the variadic example waits on the attribute grammar.

## Design choices the surface landing made

Where the design record left a question open or under-specified, the landed surface answered it.
These are descriptive of the tree, not a new design face.

* **Grammar.** `extern`, `from`, and `type` are reserved keywords; a block's interior is a flat semicolon-terminated list of opaque type declarations and bodiless function signatures, reusing the existing parameter and type rules with zero grammar conflicts.
* **Member attributes are deferred from the grammar.** The unwind and variadic markers are attribute territory and neither is on the current path, so **the block accepts un-attributed members only**.
* **The module namespace is the library string.** `extern "c" from "m"` binds the namespace `m`, so **the library string must be a valid lowercase identifier** to be selectable.
  A namespace-versus-library split is a growth item for libraries whose names are not identifiers.
* **The payload is the argument record keyed by declared parameter names**, uniformly — including a one-parameter call, and the empty record for a zero-parameter call, which the foreign-call elaboration handles directly rather than through the general zero-argument call form.
* **Boundary types at the surface.** In the elaborated signature the C string type is typed as gandr's string (the value that actually crosses), an opaque handle as a machine word, and a void result as unit; the six atoms are their own rigid atoms.
* **Session persistence.** The registry persists across REPL submissions, so a foreign call on a later line sees a module declared earlier; an `extern` block itself yields no runnable item.

## Open questions, with dispositions

### ffi-question-01

**Sub-word and pointer-width scalars — carried.** The six atoms lack the sub-word integers, a pointer-width integer, and a boolean or C character type.
Either add boundary-only scalar names mapping onto the six, or pull the numeric-tower decision forward.
Open, and coupled to the arbitrary-precision numerics work in [[roadmap]].

#### ffi-question-02

**Effect-row granularity — carried.** One signature per library, one shared foreign row, or one operation per function?
Per-library is what the design proposes and what the surface landed.
The open half is ergonomic: does effect-polymorphic code want a coarser row?

#### ffi-question-03

**Is runtime loading needed at all — carried, and it is the cheapest question here.** If static linking suffices for the self-hosting target, then the load capability and the whole linear lifetime discipline are **pure growth** rather than near-term work.
Settling this reprioritizes a large part of this document.

#### ffi-question-04

**Variadics on a compiled artifact — carried.** Link the runtime-signature library into every compiled artifact, or refuse variadics in compiled code and require the interpreter?
The trade is a runtime dependency against a capability gap.

#### ffi-question-05

**The callback ABI — carried.** When callbacks land, is a gandr-closure-to-function-pointer trampoline feasible given first-order closures, and does it need a per-callback linear lifetime?
The hazard is precise: **a trampoline outliving its closure is undefined behaviour**.

#### ffi-question-06

**The differential oracle at the boundary — carried.** Can a foreign call participate in the compiled-versus-interpreted differential when the callee has side effects and is therefore not reproducible, or must foreign calls be excluded from — or mocked in — the differential?

## The honest case, both ways

**For.**

* The safety model **is** the tool-and-effect model gandr already committed to, so mocking, sandboxing, least authority, and inspectability are inherited rather than built.
* The fixed-width scalar atoms make the numeric boundary an **identity map** — unplanned, and real.
* The linear library lifetime turns the sharpest dynamic-foreign-interface undefined behaviour into a caught linearity error **with no new mechanism**.
* The cut is genuinely small and sound, and rides landed infrastructure.

**Against.**

* **The compiled path's C-ABI work is real, unglamorous, and has no small-language precedent to copy.** The named reference is an implementation to mine, not a library to depend on.
  This is the honest cost centre of the whole design.
* Struct-by-value is deferred, and **a large fraction of real C APIs pass structs**; the boundary will feel toothless against them until that growth lands.
* The distinct foreign-handle node is a kernel-format change, so full type-safe handle provenance and automatic checked release are not near-term.
* Opt-in unwinding is type-reserved but abort-backed on the interpreter and on Windows, and the typed-unwind story depends on a refinement that is itself deferred.

**Three dead ends, named so they are not rediscovered.**

* **A zero-overhead direct binding that bypasses the effect row** — tempting for hot loops — would forfeit interception and audit, and is **not adopted**; it survives only as a reversal-triggered fallback.
* **Auto-marshalling a returned character pointer into a gandr string** is a dead end, because the return buffer's ownership is undecidable from the type.
* **A generic pointer type former** is declined in favour of per-type opaque atoms, which buy everything it would.

**Net:** the safety model is a high-confidence consequence of gandr's existing effect, capability, and linearity design, and the cut is sound and small; the genuine risk and cost sit **entirely** in the compiled path's C ABI and in the deferred struct, handle, and unwind growth — none of which blocks the interpreter.

## Source and confidence

Written against the pre-reboot foreign-interface design record in full — its design-space axes, safety model, boundary mapping, execution paths, backend requirements, growth table, interaction map, corpus plan, outlook, open questions, and its own as-built notes — and against the current tree, which is the arbiter on status.

As-built claims name their module: the `extern` node kinds in `gandr-surface-grammar`; the foreign-module registry, the pre-pass, `extern_block`, `extern_function`, and `boundary_ctype` in `gandr-surface-engine`'s `lower` module; and the frozen `ffi` mode with its inert check in `gandr-surface-corpus`.

Two divergences from the design record are stated at the claim rather than reconciled: **the native handler does not exist in this tree**, so the interpreter path is designed and unbuilt here even though the design record describes it as landed; and the corpus treatment is correspondingly **frozen rather than running**, with the two examples that need no foreign call being the exceptions.

One locator is carried unverified and marked where it is used: the exceptional-unwind refinement attributed to Congard, Munch-Maccagnoni, and Douence at ESOP 2026.
