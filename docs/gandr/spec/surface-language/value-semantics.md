# Value semantics

How a gandr program expresses "a changed value", and where the change becomes physical.

Every construct on this page produces a **new** value.
No binding a program already holds can observe a later change to it, because there is no construct through which such a change could travel: gandr has no lvalue, no assignment, no reference, and no aliasable mutable cell.
Update is therefore not a mutation surface that has been made safe — it is a **copy-and-update surface whose safety is structural**, and the physical copy is a separate question answered below the surface.

The forms are landed and exercised by the executable corpus.
What stays open is the calculus that would let a program _say_ something about exclusivity — access modes, references, regions — and that lives in a mode and reference calculus that is not yet written, which this document is deliberately written not to foreclose.

## The design space, and where gandr sits

Four positions, ordered by how much each commits the type system.
The ordering matters because the verdicts are not independent: reading down, each row asks the type system for strictly more, so the top row is the position that owes the least.
Down is _more commitment_, not a chain of extensions — the bottom two are alternative heavy commitments rather than one refining the other.

| model                              | how a program says it                                                                                                                                               | what the type system must carry                            | verdict                                                                          |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------- |
| **pure value semantics**           | functional update and construction; every value is a mathematical value                                                                                             | nothing new — the graded thunk `U_r B` already exists      | **adopted**, and it is what is built                                             |
| **mutable value semantics**        | `inout` / `sink` access modes; in-place mutation that is _semantically_ value-in / value-out [@racordon-shabalin-zheng-abrahams-saeta-2022-mutable-value-semantics] | an exclusivity discipline on parameters                    | **inspires** the vocabulary; the mapping onto gandr's machinery is _not_ adopted |
| **references and cells**           | `ref`, `:=`, aliasing                                                                                                                                               | a store, a region or lifetime discipline, aliasing control | **deferred** to the mode and reference calculus                                  |
| **uniqueness or linear ownership** | uniqueness-typed or borrow-checked in-place update                                                                                                                  | a full borrow and capability calculus                      | **deferred** to the mode and reference calculus                                  |

gandr takes the **top row as its surface** and treats the second row's payoff — in-place execution — as a **runtime** concern licensed by uniqueness, never a surface feature.
The bottom two rows are a real design pass, not a transcription of somebody else's calculus, and this document does not touch them.

**Why value semantics is the right floor rather than a deferral dressed as a decision.** The project's own discipline is to spend rigidity on _authority_ before it is spent on memory-management ergonomics.
The reason is that a retry loop cannot recover from an aliasing bug, whereas the authority failures the design does spend rigidity on are exactly the recoverable kind.
The floor chosen here has no aliasing to have a bug in, and — the load-bearing property — it is the **empty-capability special case** of every heavier model above it, so each of them arrives later as an _addition_ rather than a retrofit.

## The update surface

Three constructs carry the whole update **surface**.
They are not the only callable route to a changed value — the value-model ladder's combinator library reaches the same overlay by another face, and the last subsection below says exactly how — but they are the spellings the language teaches, and **nothing anywhere produces a changed value by mutating an existing one**.

### Functional record update

```text
#{ r | ℓ₁ = v₁, …, ℓₖ = vₖ }
```

Read it as "the record `r`, with fields `ℓᵢ` replaced by `vᵢ`."
The `#{` prefix keeps it in the same prefix-discriminated family as the record literal `#{ℓ = v, …}` and the record type `#{ℓ : A, …}`, so a bare `{ … }` block is untouched — family consistency is why this spelling was chosen over the runner-up `{ r with ℓ = v }`, which is recorded here so a later pass does not rediscover it as novel.

**The grammar discriminates on a tile, not on lookahead.** The precedence-bounded grammar **factors** the update into the `#{` expression family: the update tail `| id = E, … }` shares the `#{ E` prefix with the record literal and is selected by the `|` tile appearing after the leading source expression, recorded as a named adaptation on the `record_expression` rule in `gandr-surface-grammar`'s `surface::term` module.
The pre-reboot design predicted the same conclusion — no conflict — by a different argument, namely a one-token `=`-versus-`|` shift decision under an `LR(1)` parser.
That argument belonged to a parser generator gandr no longer uses, and the as-built rationale is not merely a different route to the same place: the discriminating `|` **sits arbitrarily deep past nested `#{ … }` values, out of any bounded lookahead's reach**, so factoring the family is what makes the form work at all rather than a stylistic preference.
**The conclusion survived; the reasoning did not, and the reasoning is what this document states.**

**Two update flavors fall out of one overlay, both statically checked, neither needing a row variable.**

* **Field replacement** — `ℓ ∈ dom(r)`.
  The field's _type_ may change (a "strong" update), and that is sound for a reason worth stating rather than assuming: the result is a **fresh** value, so there is no aliasing edge along which an old observer could reach the new type.
  Strong update is unsound only where the update is visible through an alias, and here nothing is.
* **Field extension** — `ℓ ∉ dom(r)`.
  The result is a _wider_ closed record type.
  This is expressible over closed records precisely because the result shape is statically known; it is **not** the polymorphic extension that needs a row variable, and it does not anticipate one.

**Elaboration, as built.** The design as first written described update as a _type-directed_ projection-and-rebuild — read every unmentioned field off `r`, emit a record literal:

```text
#{ r | b = v }   ⇓   #{ a = r.a, b = v, c = r.c }        for  r : #{a : A, b : B, c : C}
```

That elaboration needs the **static field set of `r`**, which the lowerer does not have: items lower independently, _before_ checking, so no type flows to the lowerer — the session threads a typing context at check time and never at lower time.
The realized form performs the same projection-and-rebuild **at the value level instead**.
`#{ r | ℓ = v, … }` lowers, in `gandr-surface-engine`'s `lower` module, to `recordupdate r #{ℓ = v, …}` over `NativePrim::RecordUpdate` (arity 2), which overlays the manifest overrides onto the manifest base and returns a **fresh** `Value::Record`: override labels win, new labels extend.
Both flavors above fall out of that one overlay, exactly as the design predicted.
The base and the overrides both lower in value position (a computation base or field value is hoisted, as a record literal's is), and the overrides reuse the record literal's canonicalizing — sorted, last-wins — field lowering.

**The result types gradually, and that is a named limitation rather than a design position.** Precise result typing — "the base's type with `ℓ` retyped or widened" — needs the same static field set the lowerer lacks, so `NativePrim::RecordUpdate` is typed `#{} → #{} → F #{}` in `gandr-core-checker`'s `prim` module, using the empty record as the **top of the width order** — record subtyping by width places a record below every record with fewer fields, so the field-less record sits above them all and any base and any overrides fit it.
The sibling `record.insert` builtin carries the identical limitation.
The user-visible consequence is exactly one thing: a projection **off an update result** — `#{ r | … }.ℓ` — does not statically resolve, because the value is present at runtime and only the static type is gradual.
Lifting this is growth-path work: a type-directed elaboration pass, or the core update former of the growth-path table, once `r`'s type is known at the point of elaboration.

**Update spends no frozen-core budget, and that is the whole point of the cut.** A core `RecordUpdate` _former_ would be a kernel-format addition sequenced behind the export-format discipline.
The realized primitive is an addition at the non-exhaustive `native` node instead, so it touched no kernel slot, needed no mechanized face, and was not gated on any kernel-side migration.
A core former becomes warranted only if the rebuild's cost cannot be met by the runtime elision below — that is the growth path, not this rung.

The elaboration records `ElabKind::RecordUpdate` (in `gandr-surface-engine`'s `origin` module) so a diagnostic can un-sugar `#{ r | ℓ = v }` back to the overlay it denotes, and so a surface type unparser can re-sugar it.

### List functional update

`List(A)` gets its update surface from the prelude's `list` module as **native builtins**, each returning a **new** list.
The iteration is Rust's, applying a gandr closure per element, so no user-level recursion is on this path — which matters because the core carries no user-level fixpoint at this rung.

In the signatures below `?` is the gradual unknown type — these builtins are typed over the gradual surface rather than over a type parameter, which is why the element type does not appear.
A **manifest** value is one that has already reduced to a literal structure (a `Value::List`, a `Value::Record`) at the point the builtin meets it, as opposed to a suspended computation; every operation here requires its collection argument to be manifest.

| surface name                  | arity | signature                                               | note                                                                                                                                                                                                                                       |
| ----------------------------- | ----- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `list.set(xs, i, v)`          | 3     | `List(?) → Integer → ? → F (List(?))`                   | index replacement                                                                                                                                                                                                                          |
| `list.update_at(xs, i, f)`    | 3     | `List(?) → Integer → U(? → F ?) → F (List(?))`          | higher-order; unrolls to `force(f) xs[i] >>= r. ret […, r, …]`                                                                                                                                                                             |
| `list.insert_at(xs, i, v)`    | 3     | `List(?) → Integer → ? → F (List(?))`                   | length-changing; `i == len` appends                                                                                                                                                                                                        |
| `list.remove_at(xs, i)`       | 2     | `List(?) → Integer → F (List(?))`                       | length-changing                                                                                                                                                                                                                            |
| `list.push(xs, v)`            | 2     | `List(?) → ? → F (List(?))`                             | growth by one                                                                                                                                                                                                                              |
| `list.append(xs, ys)`         | 2     | `List(?) → List(?) → F (List(?))`                       | two-list concatenation; **reuses** the `++` operator primitive                                                                                                                                                                             |
| `list.concat(xss)`            | 1     | `List(?) → F (List(?))`                                 | a list of lists to one list; **reuses** `flatten`, so it inherits that primitive's declared type — the nesting the operation actually requires is **not** expressed in the signature, and a non-nested argument is not statically rejected |
| `list.update_where(p, f, xs)` | 3     | `U(? → F Boolean) → U(? → F ?) → List(?) → F (List(?))` | predicate-guarded map; the transform-or-keep dual of `list.where`'s filter                                                                                                                                                                 |

Three details of record:

* **The names are underscore-spelled.** Surface identifiers cannot contain `-`, so the design's `update-at` reads `update_at` and so on.
* **Two names add no primitive.** `append` reuses the list-concatenation primitive and `concat` reuses `flatten`; the other six are new `NativePrim` variants.
  This is not an economy for its own sake — a second primitive with identical behavior is a second thing every oracle and every mutation campaign has to cover.
* **The honest boundary is a defined halt, never a panic.** The index type is `Integer`; an out-of-bounds index — or a non-manifest argument — degenerates to a **gradual hole**, which evaluates to a blame outcome — a defined, reportable halt carrying the site that produced it, in the gradual-typing sense, not an exception and not a panic.
  The corpus pins this as a failure golden.

There is deliberately **no** `xs[i] := v` lvalue and **no** in-place index assignment.

### Declared data — update by construction

Declared data is generative-nominal and its constructors carry a **product** of fields, not a record, so the record-update overlay does not reach inside a constructor.
Its update surface is therefore **construction**:

* **Multi-constructor data** is updated by **match-then-rebuild**: match on the constructor, read the fields you keep, apply a constructor to the new field values.
  No sugar hides the match, and that is the honest reading rather than an ergonomic oversight — _you cannot update a field of a value whose constructor you have not established_, exactly as with any sum.
* **Single-constructor, record-shaped data** was designed to inherit the `#{ d | ℓ = v }` sugar on the assumption that a constructor's payload is a record.
  As built, the payload is a field product, so **the sugar does not apply**.
  A single-constructor value is updated by match-then-rebuild like any other; the sugar's return is growth-path work, not a landed rung.

"An updated `Cons` is a new `Cons`" is the whole of it.

### What is not part of this surface

The **value-model ladder** — the staged sequence of primitive value formers the surface grew, rung by rung: strings, the sized numeric atoms, `List(A)`, and the closed record — brings its own combinators, and those are neighbours of the update surface rather than part of it: `record.get` reads a **dynamic** string label out of a manifest record (the native answer to the dynamic access static projection cannot express), and `record.insert` extends or overrides a single field.
`record.insert` overlaps `#{ r | ℓ = v }` for the single-field case and carries the same gradual result type; the update form is the surface spelling and the builtin is the combinator-library face of it.

**Two ladder rungs would extend update over recursive data, and both are carried rather than settled here.** The design record names a pending **set-operations rung** and a **value-level recursive `Json` type**, either of which widens what "an updated value" can mean.
Neither exists in the tree, and neither is this document's to schedule: the set-operations _syntax_ is a named deferral in [[roadmap#Deferred-with-reasons, collected]], pending the polymorphism and solver lane, and the recursive value type has no corpus home at all — recorded here so its absence is visible rather than silent.

## The state-visibility red line

**No ambient mutation, ever.** After

```text
def r  = #{ x = 1, y = 2 };
def r2 = #{ r | x = 9 };
```

`r` still denotes `#{x = 1, y = 2}` and `r2` denotes `#{x = 9, y = 2}`, and **there is no aliasing edge between them**.

This is the line that makes the whole surface value-semantics-honest, and it is not left as prose: it is **asserted operationally by the corpus harness**, which runs each example through the session engine and the L-machine runtime host and checks its declared last value (`gandr-surface-corpus`'s `lib` module).
Two examples carry that assertion — one binds a record, updates through a _new_ binding, and returns the **original**; the other shares a record across several bindings, updates one path, and reads a shared binding back.
In both, the expected value is the unchanged original, so a regression that let an update travel backwards fails the corpus.

**A caveat on which differential does what**, because the design record predates the current tree and its wording invites the wrong reading. gandr has **one evaluator** — the L machine, which physically replaced its predecessor — so there is no evaluator-versus-machine differential over _evaluation_ to appeal to; evaluation differentials compare against frozen snapshots.
The two-realization differential that does exist is over the **checker**: a recursive realization against a defunctionalized machine, property-tested for step-for-step agreement on a control log.
The red line's operational evidence is therefore the corpus assertions above, not a dual-evaluator oracle.

The red line holds **unconditionally** at this rung, and it holds for a structural reason rather than by discipline: there is no construct in the language through which a change could become visible to a prior binding, so there is nothing to check.
It is worth being precise about what would put it at risk — not a bug in update, but the _arrival_ of a reference calculus, which is why the mode and reference calculus must treat preserving this line as a constraint on itself rather than as an inherited guarantee.

## In-place execution is the runtime's business

The **surface meaning** of every update above is copy-and-update.
Whether the runtime physically copies is a separate, purely operational question, and the answer is allowed to differ per site:

* when the base is provably **unique** at the update site — its last use, no live alias — the runtime **may** mutate it in place and hand the same allocation back, an **unobservable** optimization precisely because value semantics guarantees no other observer;
* when the base is shared, the runtime copies.

This is the intended optimization target, adopted here as a commitment, and the uniqueness analysis stays entirely below the surface: an implementation seam, never a typing rule.
The precedents are three, and they differ in where they put the analysis, which is exactly the choice left open:

* **static last-use with reuse specialization**, as in Perceus [@reinking-xie-de-moura-leijen-2021-perceus], which emits precise reference-counting instructions so that cycle-free programs are garbage-free and reuse analysis yields guaranteed in-place updates;
* **borrowed references with inferred borrow annotations**, as in the reference-counting scheme for a purely functional language of [@ullrich-de-moura-2019-immutable-beans], which minimizes count updates rather than eliminating them;
* **copy-on-write for dynamically sized containers with stack allocation for fixed-size values**, as in the mutable-value-semantics implementation strategy of [@racordon-shabalin-zheng-abrahams-saeta-2022-mutable-value-semantics].

The **stronger** version — where the _type system_ certifies exclusivity, so elision is guaranteed rather than best-effort and part-wise in-place mutation becomes expressible — is the borrow calculus, and it is out of scope here by decision, not by omission.

The corpus's stress subtree is the acceptance corpus for this work: each example has a defined value-semantics answer that its harness expectation already fixes, plus a measurable copy count the optimization must reduce **without changing that answer**.

## The boundary note — what may be assumed, and what stays open

This is the contract that keeps code written today from foreclosing the calculus written later.
It is stated as two lists and four rules because "don't paint yourself into a corner" is not checkable and these are.

**Code may assume (stable; these will not be revoked):**

* pure value semantics — every value is a mathematical value, and no construct exposes aliasing-visible mutation;
* graded thunks `U_r B` — a suspended computation carrying a grade `r` from a preordered semiring that bounds how many times it may be forced — are the **only** quantitative discipline on the surface;
* functional update and construction are the **whole** mutation surface, and any structural sharing beneath them is invisible and unobservable;
* evaluation order is call-by-push-value sequencing — update reads are ordinary computations delivering `F A`.

**These stay open — do not design against their absence _or_ their presence:**

* references and mutable cells (`ref`, `:=`);
* `inout` and exclusive-borrow parameter modes, and the exclusivity law that would justify them;
* regions or scopes for reference lifetimes; mode-bounded polymorphism; the access-capability algebra.

### The four foreclosure rules

Each is a rule about what may be _written now_, and each names the future construct it protects.

#### value-rule-01

**No lvalue and no assignment surface.** Do not introduce `:=`, or any lvalue grammar.
It would collide head-on with a future `inout` and pre-commit the surface to a store semantics the calculus has not chosen.
The surface-language roadmap records this as a hard foreclosure for exactly this reason ([[roadmap#Deferred-with-reasons, collected]]).

#### value-rule-02

**No aliasing leak through any construct.** No builtin and no elaboration may return a value that shares observable mutable state with its input.
Every update builtin returns a fresh value.
This is the red line restated as an obligation on _implementers_ rather than a property of programs.

#### value-rule-03

**No signature that bakes in copy-visibility.** Update builtins are typed `… → F T`, **producing a new value** — never as an identity-preserving or in-place-typed operation.
The purpose is precise: when uniqueness or `inout` arrives, the **implementation** must be able to elide the copy without a **signature** change.
A signature that promised in-place would have to be broken; a signature that promises a fresh value never does.

#### value-rule-04

**No assumption that copies are physical, in either direction.** Code, and the cost intuition attached to it, must not depend on update being an `O(n)` copy — the runtime may make it `O(1)`.
**Equally, it must not depend on it being `O(1)`**: that guarantee needs the borrow calculus.
Both halves bind; the second is the one that gets forgotten.

## The cut, and the growth path

Every growth-path item is an **addition** over what is built, never a retrofit — the same relationship the closed record has to the row-typed record.

| concern                        | as built                                                            | growth path                                                                                                                                                   |
| ------------------------------ | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| record update                  | `#{ r \| ℓ = v }` overlay over closed records, gradual `#{}` result | precise result typing by type-directed elaboration; polymorphic update over a row variable; a core update former if cost demands                              |
| single-constructor data update | match-then-rebuild                                                  | the `#{ d \| ℓ = v }` sugar, once a constructor payload presents as a record                                                                                  |
| list update                    | native builtins returning fresh lists                               | in-place update under proven uniqueness; `inout` list parameters                                                                                              |
| in-place execution             | none — the runtime copies                                           | uniqueness-driven elision, best-effort, below the surface; then type-certified exclusivity giving guaranteed elision and part-wise mutation                   |
| access modes                   | none on the surface                                                 | `let` / `inout` / `sink` / `set` plus mode-bounded polymorphism                                                                                               |
| references                     | none                                                                | `ref` as a computation; regions; the store                                                                                                                    |
| string representation          | plain owned UTF-8                                                   | views, a foreign-interface null-terminated form, copy-on-write with small-string optimization — reopened by a real foreign-interface or self-hosting workload |

## Interactions with the rest of the language

* **Declared data and pattern matching** are the primary consumer: they receive the update surface and the red line, and update-by-construction is the introduction-side dual of matching.
* **Recursion and loops** inherit the loop-state model from this stance: `for` / `while` / `loop` thread state as **values** through a fold, with no mutable loop variable — the accumulator is rebound, never mutated ([[recursion#Loops, and the break/continue discipline]]).
  The loop-carried accumulator is also the **uniqueness hot path**, and the corpus pins it as such.
* **Attributes.** The surface attribute blocks and their payloads are [[declarations#Attributes]]; the design record additionally states that the _entity-attribute storage layer_ keys attributes by stable identity in a content-hash-neutral side table, so an attribute edit is a functional update of the attribute record and is consistent with the red line.
  **That storage claim has no realization in this tree and no other corpus home yet**, so it stands here as carried-from-the-design-record and locator-pending, not as an as-built property.
* **Modules** are immutable values; module and package metadata is typed attribute data, updated by construction ([[declarations#module declarations]]).
* **The foreign interface** is where value semantics meets the memory model, and it is the workload that reopens the string-representation question — a boundary needs a representation, and "plain owned UTF-8" is a choice that a real foreign call can make expensive.
* **Self-hosting** is the workload most likely to hit the performance limits of deep functional update, and is therefore the trigger that puts the elision work on the critical path with concrete benchmarks behind it.
* **A wasm target** turns the elision seam from an interpreter nicety into a code-generation obligation, because linear-memory codegen has to decide the question rather than defer it.
* **Per-assumption grading**, if it is ever adopted, is a value-semantics-of-migration concern; the boundary note keeps `U_r` as the only surface grade precisely so that it stays a clean future addition.

## The corpus treatment

The examples are landed, in the three-tree split under `crates/surface-corpus/examples/`.

**Model examples** — they teach the surface and the red line.
Read the second column precisely: only the rows marked **asserts** carry a harness expectation that would fail if a base binding changed; the rest state the property in prose and assert the update's own value.

| example                                      | what it pins                                                                                                                                            |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `model/15-record-update.gandr`               | the base form; demonstrates in prose that the original binding is unchanged — the assertion is on the update's own value                                |
| `model/16-record-update-field-retype.gandr`  | field replacement that changes the field's type; asserts the retyped result                                                                             |
| `model/17-record-extend.gandr`               | an override label not already present, widening the record; asserts the widened result                                                                  |
| `model/18-nested-record-update.gandr`        | the compositional idiom `#{ r \| inner = #{ r.inner \| ℓ = v } }` — there is no deep-update syntax                                                      |
| `model/19-list-update-ops.gandr`             | all eight list operations exercised; demonstrates in prose that each leaves its base untouched — the assertion is on the predicate-guarded map's result |
| `model/20-value-semantics-no-aliasing.gandr` | **asserts** the red line: bind, update through a new binding, return the original, and the expected value _is_ the original                             |

**Pathological examples** — failure goldens and the stress set:

| example                                        | what it pins                                                                                                                               |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `pathological/record-update-non-record.gandr`  | a non-record base fails to type against the width-top `#{}`                                                                                |
| `pathological/list-update-out-of-bounds.gandr` | an out-of-bounds index is a defined blame outcome, not a panic                                                                             |
| `pathological/deep-record-update-cost.gandr`   | rebuild cost through nesting; the elision opportunity                                                                                      |
| `pathological/wide-record-update.gandr`        | a many-field record with one field changed — the widest copy-versus-elision gap                                                            |
| `pathological/update-in-loop.gandr`            | a fold building successive accumulator versions — the loop-carried uniqueness hot path                                                     |
| `pathological/shared-then-updated.gandr`       | **asserts** the red line under sharing: share across bindings, update one path, read a shared binding — the expected value is the original |
| `pathological/large-list-append-vs-set.gandr`  | functional growth versus indexed replacement at scale                                                                                      |

**One planned example is not landed.** `declared-data-update-by-construction` was deferred when declared data did not exist.
Declared data exists now, so the example is a live corpus obligation rather than a deferral, and it should demonstrate match-then-rebuild over a multi-constructor type — including the fact that the record-update sugar does **not** reach a constructor payload.

## Open questions, with dispositions

Every question the design left open has exactly one disposition here.

### value-question-01

**The update spelling — settled.** Whether `#{ r | ℓ = v }` clears the grammar gates, or whether the runner-up `{ r with ℓ = v }` is forced.
**Settled affirmative**: the form landed, factored into the `#{` family and discriminated on the `|` tile, with no conflict.
The runner-up was not needed.

### value-question-02

**Type-changing replacement — settled.** Whether field replacement should permit a type-changing (strong) update, or restrict to same-type replacement for simplicity.
**Settled affirmative**, and by a route the question did not anticipate: the gradual `#{}` result type exposes no old observer at all, so strong replacement is trivially sound at this rung.
The ergonomic argument the question raised — least surprise — is answered by the fresh-value semantics rather than by a restriction.

### value-question-03

**Where uniqueness analysis lives — carried.** Static last-use, dynamic reference counting, or both; and which the machine adopts first.
Open, and owned by the runtime-versus-calculus boundary.
The three precedents cited above are the candidate answers, and the corpus stress subtree is the acceptance instrument whichever is chosen.

### value-question-04

**Exclusivity without a store — carried to the mode calculus.** Whether an exclusivity law can be extended to ordinary value parameters on gandr's substrate _without_ introducing a store, or whether `inout` inevitably pulls in references.
This is the pivotal question for the shape of the whole calculus, and it belongs to the mode and reference calculus rather than here.

### value-question-05

**Update sugar for declared data — carried.** Whether multi-constructor declared types warrant an update-by-lens or first-class-field-update sugar before row polymorphism arrives, or whether match-then-rebuild is acceptable indefinitely.
Open.
The as-built finding above sharpens it: the single-constructor case, which the design expected to be covered by the record sugar, is **also** match-then-rebuild today, so the ergonomic gap is wider than the question assumed.

## The honest case against this stance

Recorded because a stance whose costs are unwritten reads as free.

* **Value semantics without any uniqueness surface is a real performance cliff before elision exists.** Deep or wide functional update is an `O(n)` copy until the runtime work lands; the stress examples exist because the risk is real, not hypothetical.
  The named dead end is shipping update builtins whose _signatures_ assume physical copies — which [[#value-rule-03|value-rule-03]] forbids precisely so that the cliff can be removed without a surface break.
* **Update-by-construction is verbose**, and more verbose as built than as designed, since the single-constructor case did not inherit the sugar.
  Users accustomed to record-update-everywhere will find it heavy until row polymorphism or lenses arrive.
  This is an honest ergonomics gap and it is not on the critical path.

**Net:** value semantics with functional update as a derived form is the correct floor — minimal, foreclosing nothing, and paid for — provided the elision work is scheduled before a deep-update workload makes the copy cliff bite.

## Source and confidence

Written against the pre-reboot value-semantics design record in full (all thirteen sections, including its own as-built notes) and against the current tree, which is the arbiter wherever the two disagree.
Every as-built claim above names the crate and module it was checked against: `NativePrim::RecordUpdate` and the list update primitives with their signatures and boundary behavior in `gandr-core-checker`'s `prim` module; the lowering, hoisting, and `ElabKind::RecordUpdate` provenance in `gandr-surface-engine`'s `lower` and `origin` modules; the prelude's `list` module bindings in `gandr-surface-engine`'s `prelude` module; the grammar adaptation in `gandr-surface-grammar`'s `surface::term` module; and the corpus files by path.
Three divergences from the design record are stated at the claim rather than silently reconciled: the grammar mechanism, the value-level rather than type-directed elaboration with its gradual result, and the loss of the single-constructor record-update sugar.
