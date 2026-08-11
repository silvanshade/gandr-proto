# The mode and reference calculus

**Proposed.
Nothing in this document is built, and nothing in it is adopted.** It is the design space for the calculus that would let a gandr program say something about _exclusivity_ — access modes, references, regions, borrows — on top of the value-semantics floor that [[../value-semantics]] fixes.

It exists because that floor is deliberately the position that owes the least, and a floor is only safe to stand on if the storey above it has been surveyed.
The survey is the payload: a register of sixteen named decisions the calculus must make, a per-problem comparison against the languages that have made them, the foreign-interface consequences, and the literature the answers would come from.

**Read the status line at each decision.** Some are settled by what gandr already is; most are open; a few are recommendations this document proposes and does not enact.
Every claim about what gandr "gives by construction" is a claim about the **design**, not the build — the honesty split in the next section says exactly which is which, and it is the single most important thing to carry away.

## The substrate, and what is actually built

The design and the build differ here more than anywhere else in the corpus, and the difference is load-bearing: nearly every attractive claim in the design space rests on the linear zone, and **the linear zone is currently vacuous**.

**Built, and verified against the tree at write time.**

- **The two-zone context `Γ; Σ` exists.** `gandr-core-checker`'s `ctx` module carries the **intuitionistic zone** `Γ` — ordinary hypotheses, which may be used any number of times or not at all — as a binding stack, and the **linear zone** `Σ` (the `Sigma` type) — obligations that must be used exactly once — as a list of live obligations with `bind` and `consume` operations.
  `Σ` admits **no contraction** — `consume` is single-shot and yields nothing on a second call, so that law is _enforced_ — and **no weakening**, which at this rung is **detectable rather than enforced**: a live obligation at scope close is observable, and the test that pins it asserts exactly that the zone is non-empty, but no code path rejects the close.
  The distinction matters for every claim below that leans on "cannot be silently dropped".
  The type is named `Sigma` in that module and is **not** the dependent-pair former of the same name in the checker's type module; the two are unrelated.
- **`Σ` is vacuous at v0.** No typing rule populates it, because every obligation source it was designed to hold — session endpoints, held capabilities, acquired shared channels — is a deferred feature that does not exist in the checker.
  The zone's shape is committed now on the reasoning that retrofitting a context shape is expensive; its discipline is in force the moment a first obligation source lands.
  Until then, **a reified stack captures no obligations, and `resume`, `discard`, and duplication are unrestricted**.
  The rules that would bind once it is populated — one-shot capture, abandonment running the recorded unwind obligations, and multi-shot only for an empty captured zone — are [[../../implementation/effects-and-control#One-shot linearity, and what it resolves]].
- **Grades exist and are sealed.** `gandr-core-checker`'s `grade` module carries a single concrete carrier over `ℕ ∪ {ω}`, representation-sealed behind a semiring signature (`ZERO`, `ONE`, `OMEGA`, `fin`, `leq`, `plus`, `times`).
  The order carries the two structural rules — `thunk_r t ⇓ U_s B` requires `s ⊑ r`, and `force v` requires `1 ⊑ r`, each checked **per site with no accumulator**, so a grade-`1` thunk forced twice along one path passes both checks independently.
  `Dup` **is built and does use addition**: it reads its split grades off the expected returner-of-product type and enforces `r + s ⊑ grade`.
  `Drop` **is built**, and its side condition `0 ⊑ r` is **not checked because it is vacuous on the default carrier** — zero is the bottom of the order there, so every graded thunk is droppable, which is a tree-verified form of the central finding below rather than a gap.
  Multiplication and the grade-constraint form are genuinely unused outside the carrier module.
  There is **no per-assumption (binder) grading and no context scaling `r · Γ`**; a binder "carries" a grade only derivatively, as the grade of its bound value's thunk type.
- **The graded operations have normative signatures**: `dup : U_{r+s} B → F (U_r B × U_s B)` and `drop : U_r B → F 1` under `0 ⊑ r`, with grade-contravariant subtyping — `U_r B <: U_s B'` needs `s ⊑ r`.
  They belong to the type system proper and are stated there, with the rules they come from and the two places the build diverges from them: [[../../implementation/type-system#Grades]].
- **The reified stack `Stk(B, C)` is a value type** in `gandr-core-checker`'s `types` module — the evaluation context internalized as data.
- **The runtime host has no capability model at all.** The seam is ambient and always-resume, with no grant, no allowlist, and no denial outcome ([[../../implementation#The runtime host]]); the design that would price it is [[../../implementation/capability-model]].
- **The foreign boundary is the one place several of these decisions already have a home.** [[../../implementation/foreign-interface]] owns the boundary C-type mapping, the calling convention, linkage metadata, and the hidden-return-pointer slot, and links back to this document for the mode-facing half.

**Designed, and not built.** Sessions (binary and multiparty), manifest sharing with acquire/release, worlds and the mobility judgment, the linear-zone obligations that would populate `Σ`, and the typed-unwinding rule under which abandoning a `Σ`-owning stack runs its recorded close, release, and drop obligations.
The rules for all but the last are specified in [[../../implementation/type-system]], which is also where the mobility judgment's clauses are stated normatively rather than in the prose form this document uses.

**Neither designed nor built.** References, mutable cells, borrowing, regions, access modes, mode-bounded polymorphism, and any **internal** value-representation, layout, or address model.

**One qualification, because the pre-reboot analysis's blanket "no ABI model at all" is no longer true of this corpus.** A **boundary** C-type model is built and small — an enumeration of the foreign argument and result shapes with a total mapping from surface types onto it that rejects every composite at lowering (`gandr-surface-engine`'s `ffi` and `lower` modules) — and calling convention, linkage metadata, the hidden-return-pointer slot, and the register-versus-memory classification are owned by [[../../implementation/foreign-interface]].
What remains absent is the _internal_ model: gandr can still neither assert nor refute anything about where a value's bytes live.

**Why this matters for every sentence that follows.** The pre-reboot analysis this document absorbs was written against the _specified_ substrate and repeatedly says gandr "already has" a property.
Read every such statement as **the design has it, the build does not yet**, and check the built list above before relying on one.
The single sharpest instance: linearity is what would make a resource impossible to leak silently, and today nothing is `Σ`-resident, so nothing is protected by it.

## The central problem

**Grades count uses.
They do not exclude aliases.** This is the finding the whole calculus turns on, and it survives every reframing that has been tried on it.

A grade `r` on a thunk bounds _how many times_ that thunk may be forced.
It says nothing about _when_, _from where_, or _how many holders exist concurrently_.
Three consequences follow, and each independently kills the tempting shortcut "a shared borrow is just a graded thunk":

1. **A grade cannot express reader-writer exclusion.** "Many readers XOR one writer" is a statement about simultaneous holders; a forcing count is a statement about a total number of uses along a run.
   The two are not interdefinable.
2. **A grade cannot express a lifetime.** A grade-`1` thunk may be constructed in a callee and returned upward — used once, but _after_ the frame that allocated it would have been dropped.
   Nothing in the semiring notices.
3. **Checking at the current stage does not even accumulate counts.** Each force site is verified independently against the order, so a grade-`1` thunk forced twice along one path passes both checks.
   Context splitting and scaling belong to a later stage.

The complementary half is equally sharp: **in the default semiring, `0 ⊑ r` holds for every `r`**, so _every_ graded thunk is droppable.
Grades therefore impose **no must-consume constraint at all**.
What carries a must-run, no-leak guarantee is `Σ`-residency, and only that.

So the calculus's central open problem, stated once: **supply a shared-XOR-exclusive discipline that gandr does not have, without collapsing it into the grade axis, which cannot carry it.**

There is one **partial precedent already in the design**, and it deserves credit rather than reinvention: manifest sharing's acquire/release is a mutual-exclusion toggle between the intuitionistic and linear zones — acquire _is_ mutual exclusion.
It is not reader-writer, and it applies to shared channels rather than to values; but a freeze-region proposal that ignores it is partly rediscovering it.

## The access-mode vocabulary

The four modes the mutable-value-semantics literature names, and what each would land on here.
This table is the **vocabulary** the calculus would speak; **none of these mappings is adopted**, and the section after next says why two of them are the weakest links in the whole design.

| access mode           | what it means there                           | what it would land on here                                                                                                                                                             |
| --------------------- | --------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **shared read-only**  | an immutable borrow that may be aliased       | a value used read-only — the graded thunk, with the grade bounding the force count. **This is the weakest mapping**: a force count is not a scoped read-borrow ([[#mode-decision-05]]) |
| **exclusive mutable** | exclusive access, value in and value out      | a linear capability — except that a linear capability types **consume**, not borrow, so the mapping types the wrong thing                                                              |
| **consuming**         | ownership transfer at the last use            | linear consumption; grade `1`, exactly once. **The one mapping that lands cleanly**                                                                                                    |
| **initializing**      | definite assignment of an uninitialized slot  | the value-introduction discipline; gandr has no notion of an uninitialized slot to assign into, so this mode has nothing to land on today                                              |
| **mode-generic**      | one operation written once over several modes | a mode sort with bounded quantification, which does not exist ([[#mode-decision-07]])                                                                                                  |

A second language in the same family spells its modes differently and lands them differently, and the correspondence is the concrete thing an implementer of "access modes" would start from: its **owned** mode corresponds to a linear or graded resident, its **mutable** mode to a linear capability, and its **read** mode to a graded thunk delivering a returner.
Its **reference** mode — the mode-polymorphic one — together with **last-use destruction** and **origins** are the genuinely missing pieces, and last-use destruction as a sound surface elaboration is **plausible but unproven and gap-dependent**: inserting a last use is sound only once a region or origin mechanism can prove the source outlives the borrow.

Two gaps travel with that correspondence and are recorded so they are not rediscovered.
**The affine tier is underspecified** — whether dropping a graded thunk, or reaching its last use, actually _runs_ anything is stated nowhere.
And **a first-class borrow-as-value** would need the capture-set relaxation generalized from capturing a linear _name_ to capturing a value _origin_, which is a strictly larger change than the relaxation currently proposed.

**A naming collision to avoid, because it would read as precise.** The consuming mode is spelled `sink` in the language that names these four, and **`sink` already means something else in this corpus**: a wiring datum — the partner every source is matched with, a flow-through wire in the circuit-algebra carrier.
The two senses are unrelated and both are load-bearing.
This document therefore says _consuming mode_ and never `sink`; if the calculus ever surfaces these modes, **the spelling is a decision, not an import**.

**The central claim the literature offers, and what it would cost here.** Value semantics plus statically-checked exclusivity means a compiler may elide copies and mutate in place with no observable difference — so the exclusive-mutable mode is a read-then-initialize that the exclusivity law proves safe to perform in place [@racordon-shabalin-zheng-abrahams-saeta-2022-mutable-value-semantics].
That is precisely [[../value-semantics#In-place execution is the runtime's business]] seen from the _type_ side: where the value-semantics floor leaves uniqueness to the runtime as a best-effort optimization, an exclusivity law moves it into the checker and makes elision guaranteed.
Whether that law extends to ordinary value parameters **without introducing a store** is the pivotal open question, and it is [[#mode-decision-05]].

**An ordering constraint carried from the design record, and it binds.** The reading of the mutable-value-semantics line and the broader borrowing-literature reading **land their adopt, inspire, and reject verdicts before this calculus's own design pass begins** — so the pass is written against settled verdicts rather than deriving them mid-flight.
The same constraint is parked in [[../roadmap#Pending surface lanes]] so it is visible from the schedule and not only from here.

## The decision register

Sixteen decisions the calculus must make.
Each has a heading so it can be linked into directly, and the numbering is **stable**: retiring one leaves its number unused.
The pre-reboot source used bare letter-number codes for these; those codes are gone, and the identifiers below replace them.

### mode-decision-01

**What carries the must-run / no-leak guarantee.** `Σ`-residency, because it is linear.
Grades do not, for the reason above — in the default semiring every graded thunk is droppable.
The consequence is a restriction: **data that must be cleaned up belongs in `Σ` and nowhere else**.

_Status:_ the division of labour is settled in the design; how a _borrow_ maps onto it is **open**.

### mode-decision-02

**The type and effect signature of the abandonment operation.** The design's discard operation is typed to deliver unit under an **empty** effect row, so fallible cleanup — a rollback that can itself fail — is unmodeled, and blocking cleanup has nowhere to block.

_Recommendation:_ upgrade it to carry an effect row at minimum, and consider delivering a success-or-cleanup-error sum.
_Status:_ **open**.

This one is worth flagging as a recurring error source rather than a preference: the pre-reboot analysis's prose repeatedly wrote as though cleanup could await, perform effects, or issue a rollback, and its own adversarial pass had to correct that reading three times over, at three separate claims, plus once as a standing cross-cutting note.
The empty row is what the design says; the effectful row is what everyone assumes it says.

### mode-decision-03

**Does the failure or abort path run unwind on the obligations it unwinds past?** Today only _explicit_ abandonment runs unwind; nothing states that an abort — the shell's error-exit discipline, say — discards rather than silently drops the linear obligations held by the continuations it destroys.

_Recommendation:_ wire the abort handler to the abandonment operation, so rollback and flush run on the error path too.
_Status:_ **open**, and it is the difference between a leak-freedom claim that holds and one that holds only on the happy path.

### mode-decision-04

**Cancel during cancel.** The unwind computation may itself suspend — awaiting a kernel acknowledgement, for instance.
Nothing makes it uninterruptible.

_Recommendation:_ make unwind run to completion, and make its sub-obligations recursively linear.
_Status:_ **open**.

### mode-decision-05

**What a shared borrow mechanically _is_.** The central decision.
Not a graded thunk, for the three reasons in the section above.

_Recommendation:_ introduce a **freeze-region** or fractional-permission discipline that supplies shared-XOR-exclusive, and keep the grade axis as what it is — a copy budget.
Check it against manifest sharing's acquire/release first, which already supplies a mutual-exclusion toggle and may be the right mechanism generalized rather than a competitor to it.
_Status:_ **open, and central**.

### mode-decision-06

**The region, lifetime, or scope mechanism.** Regions are the classical route [@tofte-talpin-1997-regions], and gandr has no construct for them; "scopes are like regions" is an analogy in the design record, not a mechanism.

_Recommendation:_ a new **ordered modal index**, reusing the phase-index template rather than the world modality — **worlds are the wrong shape**, being symmetric places rather than nested extents.
Delimited control is worth exploring as a carrier.
_Status:_ **open**.

### mode-decision-07

**Mode-level bounded polymorphism.** One operation written once over several access modes needs a **mode sort** plus bounded quantification.
The current polymorphism is unbounded and ranges over value-type variables only: no mode sort, no bounds.

_Recommendation:_ add both, gated behind [[#mode-decision-05]] — read and mutate zones need a common interface before anything can be bounded by it.
_Status:_ **open**.

### mode-decision-08

**Implicit scope-exit cleanup.** Whether to grow an implicit-destructor discipline.

_Recommendation:_ keep the **core explicit** — linearity forces the program to write its closes, and only abandonment runs unwind automatically — and add scope-exit _surface sugar_ that elaborates to the guaranteed-consume discipline.
_Status:_ **open**.

### mode-decision-09

**Address-level immovability for the foreign interface.** The **mobility judgment** — the design's derived predicate saying whether a value may be transported to another _world_, in the sense of a distributed place — is **world-level, not address-level**: it says a value may not be transported to another place, never that its bytes may not move within one address space.
Conflating the two is a category error, and it is the error most likely to be made here because both read as "immovable".

_Recommendation:_ for a foreign self-referential type, use the **composite** — linear residency for exclusivity, immobility for cross-place transport, and a code-generation boxing or no-relocation guarantee for the byte axis.
Add a derived address-stability predicate only once a native backend exposes addresses at all.
Never a library wrapper in the style of a retrofitted pinning type.
_Status:_ **open**.

### mode-decision-10

**A customizable relocation hook.** Whether a per-type move hook should exist.

_Recommendation:_ **none by default** — dissolve the self-reference cases through the reified stack, linear residency, and regions instead.
Reserve an opt-in relocation effect _only if_ by-value interop with foreign types that have non-trivial constructors is ever promoted to a goal.
_Status:_ leaning settled (no hook).

### mode-decision-11

**Representation and ABI.** _Recommendation:_ **two representations** — an optimizable default and an opt-in frozen layout — plus gandr-owned ABI-stable boundary sum types (its own option, result, sum, tuple, string, and slice shapes).
**Never make one representation be both optimized and stable**; that is the lesson two independent Rust efforts converged on.
Keep a C-compatible layout as the conservative floor.
_Status:_ **open (recommended)**.

### mode-decision-12

**Foreign-call placement and the trust boundary.** _Recommendation:_ the foreign call is a **capability-gated effect**, and its handler is the minimal declared trusted axiom.
Bake ownership into the calling convention — **linear transfer means owned** (the declaring side runs the destructor) and **a graded thunk means borrowed** — and enforce the declared ABI signature at module load.
_Status:_ **open (recommended)**.

### mode-decision-13

**By-value interop with non-trivial foreign value types.** _Recommendation:_ declare it a deliberate **first-version non-goal**; go handle-based and opaque.
It needs both a relocation hook ([[#mode-decision-10]]) and the whole deferred mutable-reference calculus.
_Status:_ recommended / **open**.

### mode-decision-14

**Foreign value representation.** _Recommendation:_ **declared-capability opaque types** carrying a witness-table contract by default, with by-value crossing allowed only for a checked trivial subset.

The load-bearing hazard: **contraction is unsound on a non-trivially-copyable foreign type.** The intuitionistic zone admits contraction, which is a silent logical duplication; mapping a foreign type with a non-trivial copy constructor onto such a value would let contraction "copy" it without running that constructor.
So any such type must be linear-resident and opaque, with its copy constructor exposed as an explicit effect and **never** reachable through contraction.
_Status:_ **open**.

### mode-decision-15

**A move-only data category.** Whether to generalize the linear zone to host arbitrary linear _values_, rather than only endpoints and capabilities, so that "move" is the existing linear consumption and "drop" is the existing unwind.

_Recommendation:_ generalize.
Encoding move-only-ness in the grade axis is disfavoured by the division of labour, though **that is a consistency argument and not a prohibition** — the design record does not explicitly forbid it, and the default semiring already gives a graded thunk an at-most-once reading.
_Status:_ **open**.

### mode-decision-16

**Generalize cleanup from stacks to all linear residents.** Today the "an abandoned move-only value runs its recorded cleanup" property holds **only for captured stacks**; there is no mechanism for it on any other linear resident.

_Recommendation:_ lift it to every resident of the linear zone.
_Status:_ **open**.
Note the dependency: [[#mode-decision-15]] is what makes this worth doing, and this is what makes that safe.

## The design-space camps, and what the literature actually settles

Four camps have moved off linearity for surface ergonomics; three lines underpin gandr's own substrate.
The distinction between them is not academic — it is the adoption criterion.

**The camps that keep the guarantees without linear types.**

- **Modes.** Uniqueness, affinity, and locality as **mode axes over an adjoint core** [@lorenzen-white-dolan-eisenberg-lindley-2024-oxidizing].
  The pre-reboot sweep judged this the camp closest to gandr's own shape and the one most likely to **confirm** rather than challenge the design, on the grounds that an adjoint core carrying a lattice of modal qualifiers is structurally what gandr builds toward.
That is recorded as the sweep's judgement, not as a settled characterization of gandr — the comparison has not been carried out against the built tree.
- **Capture sets.** Captured variables represented in types, with scoped capabilities giving effects and effect polymorphism [@boruch-gruszecki-odersky-lee-lhotak-brachthauser-2023-capturing].
- **Reachability types.** Aliasing and separation tracked through reachability qualifiers [@bao-wei-bracevac-jiang-he-rompf-2021-reachability], extended to polymorphism [@wei-bracevac-he-bao-rompf-2024-polymorphic-reachability].
- **Boolean-negation effects.** Borrowing recast as temporary _freezing_ via effect types over a Boolean lattice with principal inference, leaving aliasing and capture **unrestricted** [@gao-parreaux-2025-invalidation-safety].
  This is a genuine counter-design to a substructural lean: it tracks _effects_, not capabilities, and its effect domain is a lattice, not a quantale.

**The lines gandr's own substrate descends from.**

- **Graded, modal, and quantitative types** — coeffects, graded monads and comonads, quantitative type theory [@atkey-2018-qtt], and graded modal types with a full language behind them [@orchard-liepelt-eades-2019-graded].
  Linear function types in a practical polymorphic setting [@bernardy-boespflug-newton-peytonjones-spiwack-2018-linear-haskell] sit here too.
  The recurring frontier lesson is one gandr has already acted on: **arbitrary-semiring generality buys expressiveness and costs inference, error messages, and adoption**, which is exactly why gandr fixes one sealed carrier and reserves the parametricity to the rules.
- **Region-based memory and ownership** — regions with inferred allocation and deallocation points [@tofte-talpin-1997-regions], region polymorphism, lifetimes, second-class values, and generational references.
  This is the home of the deferred "scopes are like regions" and of [[#mode-decision-06]].
  The central tension the calculus must navigate: **aliasing versus deallocation precision** (regions permit cycles but free in bulk; ownership is precise but forbids cycles), and **inference versus modularity**.
- **Linear, affine, and capability systems** — linear logic, deny-capabilities, effects-as-capabilities, mutable value semantics [@racordon-shabalin-zheng-abrahams-saeta-2022-mutable-value-semantics], and separation-logic metatheory [@jung-jourdan-krebbers-dreyer-2018-rustbelt].
  The last carries **the honesty constraint of this whole document**: that work shows the surface type system alone cannot justify a language's unsafe escape hatches — soundness rests on the separation-logic metatheory beneath. gandr's analogue is the property-tested derived machine plus a mechanization intent, and the mechanization is unbuilt, so **"sound by construction" is a design posture with a plan, not a proof.**

**Three through-lines, each of which changes what a decision should be.**

- **The capability-effect duality is real, bidirectional, and provably non-dominating.** Rows and capabilities are two strategies of one modal frame, with verified translations in both directions [@tang-lindley-2026-rows-capabilities] — the strongest formal bridge available.
  And minimal effect systems and capability systems are **incomparable in expressiveness**, neither subsuming the other [@bao-rompf-2025-type-ability-effect].
  The decision target is therefore a **modal hybrid**, not "pick a discipline".
  The incomparability is a load-bearing _negative_ that argues _for_ a unifying frame rather than for abandonment.
- **The move off linearity is community-scoped, not field-wide.** The signal concentrates in surface-ergonomics work — people retrofitting Rust-like guarantees into non-linear host languages under an inference and adoption constraint.
  The proof-theory and category-theory fragment has **not** moved off; it is _generalizing_ substructure within the family (quantitative type theory is linearity over a semiring; graded monads and coeffects; modes are substructure as a modal lattice over an adjoint core).
  The non-linear camps trade away exactly the compositional structure — monoidal-closed, graded, adjoint — that gandr's metatheory computes over.
  **The adoption criterion that follows: take them for surface ergonomics, never for substrate.**
- **Sequential and quantale-shaped effects are behaviours over time** [@gordon-2017-flow-sensitive-effects; @gordon-2021-polymorphic-sequential-effects], which gives the trace reading algebraic teeth.
  One trap worth carrying: **"quantale" is a partial false friend.** An effect quantale is a _partial_ join-semilattice and a _partial_ monoid — the undefined compositions are load-bearing — whereas the quantales arising from higher rewriting are _complete_ sup-lattices and total.
  Same silhouette, different objects, and neither literature cites the other.

## The per-problem catalogue

Eight problems where a mode calculus earns its keep, each with where gandr stands, and seven of the eight with a table of how the field handles it — the eighth's comparison rows live in the combined table further down rather than beside it.
Soundness is one of _yes / partial / no / not applicable_; ergonomics one of _good / partial / workaround / poor / not applicable_.

**A standing caveat on the language-mechanism tables.** The mechanism descriptions are **carried from the pre-reboot source and unverified against the vendors at this pass**, and the source's own proposal, RFC, and issue numbers are **deliberately not reproduced** — a locator carried across two documents without being re-checked is exactly the reference that reads as precise and resolves to nothing.
The tables are therefore indicative of the design landscape and **not citable**; restoring them, verified, is tracked work rather than a gap to be filled in passing.
Four locators the source itself flagged as doubtful are worth knowing about even in their absence: a destructor-decorator spelling it marked unverified and version-dependent; a set of Swift evolution numbers of which two were high-confidence, one named an operator rather than a parameter modifier, and two were unverified; a claim about a standard library's short-string optimization that is genuinely implementation-dependent; and one proposal it judged over-formalized as "killed" where the direction was in fact discussed and declined.

### Asynchronous destruction

Run suspending cleanup — a network rollback, a flush-and-await-acknowledgement, a kernel-completion handshake — when a value or scope ends.
The hard core: destructors fire in _any_ context, including panic unwind and explicit leak, but suspension is legal only where a runtime can suspend, so there is frequently **nowhere sound to run suspending cleanup** — and in affine or garbage-collected languages, no guarantee it runs at all.

| language                              | mechanism                                                                                                                                               | sound          | ergonomics |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | ---------- |
| Rust                                  | experimental async drop; an abandoned predecessor RFC for a poll-based ready hook                                                                       | no             | poor       |
| Rust (proposed)                       | poll-cancel, scope-exit `final` blocks, linear or undroppable types; completion futures and async-genericity                                            | partial        | workaround |
| Swift                                 | no async deinitializer; isolated _synchronous_ deinit; the workaround is to spawn a task from deinit                                                    | partial        | workaround |
| Mojo                                  | synchronous destructor; linear or explicit-destroy types must be consumed by a named method that may itself be async — manual, no scope-exit automation | partial        | workaround |
| C++                                   | none — destructors cannot await; the compiler-inserted scope-exit proposal was not adopted; structured-concurrency scopes instead                       | not applicable | workaround |
| C#                                    | an async-disposable interface plus an awaited `using` form; not tied to collection or lifetime                                                          | partial        | good       |
| Python                                | async context managers; the collector's finalizer cannot await                                                                                          | partial        | good       |
| research: linear and uniqueness types | "consumed exactly once" guarantees a possibly-async cleanup is invoked                                                                                  | yes            | poor       |
| research: algebraic effects           | handler finalization; "runs exactly once" depends on the multi-shot treatment                                                                           | partial        | poor       |

**Where gandr sits.** Decisively in the explicit-scope, linear-types school, and the root context mismatch is **structurally dissolved**: there are no implicit destructors, so cleanup is an explicit operation at an explicit site.
The leak-guarantee gap is closed _in the design_ — the linear zone means a value or stack owning an obligation cannot be silently dropped — so what other languages propose as a feature is here the substrate.
**In the build, the zone is empty, so nothing is yet protected.**

The soundness is **conditional, not solved**, on four counts, and all four are open decisions above: the panic and abort path is unproven ([[#mode-decision-03]]); cancel-during-cancel is unprotected ([[#mode-decision-04]]); leak re-entry through a future reference calculus is exactly what [[#mode-decision-05]] must not reintroduce; and the abandonment operation has no error or effect channel ([[#mode-decision-02]]), so "sound" there means "sound once that row is specified".

On ergonomics, three things would lift the effective experience without trading soundness, and two of the three carry caveats the source's own reviewer added: effect-row polymorphism as native maybe-async, **which only holds if suspension is modelled as an effect operation** — if awaiting is a session interaction, as the design otherwise recommends, it is structural and not row-abstractable; scope-exit surface sugar elaborating to the guaranteed-consume discipline ([[#mode-decision-08]]); and the streaming checker rendering an unmet cleanup obligation as live guidance, **which is an extrapolation beyond what the checker's stated obligation set covers — plausible, not established**.

Remaining gaps: no implicit scope-exit cleanup; abort not wired to abandonment; the abandonment effect and error type unspecified; cancel-during-cancel unprotected; the ordering and concurrency of multiple cleanups unspecified (the "sequential reverse-frame order" reading is an assumption about an unbuilt dynamics, not a specified rule); multi-shot handlers cannot resume a continuation holding cleanup obligations, which is _correct_ for soundness; and "async" itself is not formally placed — session interaction or effect operation.

### Completion-based input and output

Keep a buffer sound while a kernel holds an aliasing mutable pointer to it between submission and completion.
Cancel the awaiting computation before completion and the buffer can be freed while the kernel is still writing.
The protocol is a **linear obligation over a resource lent to a party the compiler cannot see**.

| language                         | mechanism                                                                                                                                 | sound          | ergonomics     |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- | -------------- | -------------- |
| Rust (owned-buffer libraries)    | affine ownership transfer: buffer passed by value and returned; on drop the driver keeps the buffer and cancels until completion          | partial        | workaround     |
| Rust (borrowed slice)            | readiness-style borrow valid only for the call frame                                                                                      | no             | good           |
| Rust (proposed)                  | linear must-move types with a leak bound                                                                                                  | yes            | not applicable |
| Swift (non-escapable span)       | the compiler forbids escaping the scope — a correct rejection, but the wrong tool for in-flight ownership                                 | yes            | not applicable |
| Swift (structured, non-copyable) | cooperative cancellation keeps the awaiting frame's owned buffer alive; the foreign boundary uses raw pointers and lifetime by discipline | partial        | good           |
| Mojo                             | owned-parameter and transfer syntax, plus explicit-destroy linear types; no completion-I/O binding built                                  | not applicable | not applicable |
| C++ (Asio)                       | non-owning buffer views; the caller _must_ keep memory valid until the handler runs — convention, undefined behaviour on violation        | no             | good           |
| C++ (sender/receiver)            | operation-state lifetime rules — normative, unenforced                                                                                    | no             | good           |
| Linear Haskell                   | linearity on arrows gives exactly-once submit and complete; the reference enforcement model, but garbage-collected and viscous            | yes            | poor           |

**Where gandr sits.** The design expresses **both halves** natively: the exactly-once obligation is a binary session type on a linear endpoint, and abandonment runs typed unwind.
**A placement correction from the source, because it bounds that sentence.** Completion-based I/O is named in the design record's _catalogue of deferred work_, not in the unwinding rule; the rule supplies the generic mechanism only, and no part of the design targets this problem.

**One correction is load-bearing and must not be lost.** The tempting claim — that sending the buffer over a session endpoint _removes_ it from the program's linear zone, so the program structurally cannot alias or free it — is **false as the rules stand**: the send rule draws its payload from the **intuitionistic** zone, which admits contraction, so a value sent over a session is _not_ removed and _can_ still be aliased.
The structural no-alias guarantee needs two things that do not exist: a **buffer-as-linear-capability type**, and transfer by **delegation**, which currently moves endpoints only.
State the soundness as contingent on this calculus, never as type-enforced today.

Two further corrections in the same family: "reference cycles among linear values cannot form" is _asserted, not derived_ — there is no reference type for cycles to form among yet; and modelling "multiple readers" with grades is the [[#mode-decision-05]] error again, since grades are a forcing count and not a concurrent-reader count.

**Liveness is not delivered by linearity.** Linearity forbids dropping the completion token; it does not forbid stalling on it.
"Eventually observed" is a progress property belonging to the deadlock-freedom question, and must never be folded into a safety claim.

Remaining gaps: no mutable, address-stable buffer type — **the single biggest blocker**; unwind not stated to permit a blocking, uninterruptible foreign await; the kernel is a trusted axiomatized peer; **address pinning is a representation hole the mobility judgment does not cover**; multi-shot operations (one submission, many completions) need a recursive session distinct from the one-shot case; and the completion obligation is linear _even when buffer access is shared_ — keep the two axes orthogonal.

**Do not claim gandr solves completion-based I/O.**

### Cancellation safety

Cancel a task at a suspension point without **state corruption** (intermediate work owned by the suspended computation lost or half-applied) or **leaked obligations** (cleanup the computation still owed — close a handle, return a buffer, release a lock, roll back).
The first is a per-API discipline type systems do not check; the second needs linearity plus an async-capable cleanup path.

| language                          | mechanism                                                                                                                                                    | sound          | ergonomics     |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------- | -------------- |
| Rust                              | cancel is dropping the future at any await; combinators drop siblings; cancel-safety is documented prose, not checked                                        | partial        | poor           |
| Swift                             | cooperative cancellation flag, a cancellation handler, deferred unwind, synchronous deinit — a cancelled task is still running and can await its own cleanup | partial        | good           |
| Mojo                              | no stable async cancellation model                                                                                                                           | not applicable | not applicable |
| C++                               | coroutine destroy (raw and unsafe); a stopped-completion signal with a stop token; synchronous RAII cleanup                                                  | partial        | poor           |
| Kotlin                            | a cancellation exception at suspension points; `try`/`finally`; a non-cancellable context                                                                    | partial        | good           |
| Trio / Python                     | nurseries and cancel scopes; cancellation delivered only at checkpoints                                                                                      | partial        | good           |
| research: linear, affine, session | use-exactly-once and protocol typestate statically forbid a leaked obligation                                                                                | yes            | poor           |
| research: effects and handlers    | cancel is discarding a continuation with finalizer unwinding; alone it does not enforce linearity                                                            | partial        | workaround     |

**Where gandr sits.** At the intersection the research names as the theoretical fix: **algebraic effect handlers plus linear continuations**.
The effects row above is only "partial" _because_ handlers alone do not enforce linearity — and gandr's typed-unwinding rule **is** that missing linearity.

**The leaked-obligation half is the design's strength**, with two corrections.
First: a handler that simply omits to resume is a linearity error; **abandonment must go through the explicit operation — it is mandatory, not automatic**.
Second: the abandonment operation returns a _computation_, so unwind has computational structure, but it is under an **empty effect row** today ([[#mode-decision-02]]) — the "await budget" reading is the proposed upgrade, not the substrate.
And the claim that all three completion-I/O ingredients are present is **overclaimed**: one is present (linearity), one is unspecified (the async cleanup row), and one is only sketched (ownership transfer).

**And the mechanism the problem actually needs is peer notification, which the design does not have.** The mature account in the session-types literature is that an endpoint may be _cancelled_, that cancellation **propagates to the peer** rather than leaving it blocked forever, and that a peer which then communicates on a cancelled channel observes a raised exception. gandr's typed unwind is **control-side and local**: it runs the abandoning side's own close, release, and drop obligations, and **nothing states that it closes the peer's channel with a fidelity-respecting cancellation**.
That gap is the difference between "no obligation is leaked here" and "no participant is left waiting", and only the first is claimed.

**The state-corruption half is not solved.** Typed unwind runs _resource_ cleanup; it does not restore invariants or re-apply partial work.
An owned partial-read buffer is lost on abandonment exactly as it is elsewhere.
What gandr has structurally is the _distinction_ the calculus is about, which positions it to make "cancel-safe means captures no owned-across-suspend state" a **checked predicate** — but the reified stack carries no linear component and no owned-versus-borrowed annotation today; frames record context deltas operationally.
**Recording captured-state ownership on the stack type is the unbuilt task, not an existing capability.**

On delivery semantics, cancellation would be observed at perform points, so checkpoints are type-visible — the safest tier, with the checkpoints lifted into the type.
The mechanism named for this — **asynchronous effects and signals, in the sense of Ahman and Pretnar's work on them** — is **attributed but has no typing rules in the design**: a planned mechanism, not settled substrate, and one this corpus carries no bibliography entry for.
There is no shielded-section construct.

The strongest form of the soundness claim — "stronger than every production language" — is an unimplemented specification measured against shipping systems, and is **overclaimed**.
What holds is: the leaked-obligation discipline is strong _for well-typed programs_, and is not a runtime guarantee under host hard-kill, foreign panic, or crash.
The runtime-versus-typing boundary is itself undocumented.

### Structured concurrency

Scope-bounded child lifetimes, **and** letting children safely borrow — especially mutably — into parent stack data.
The lifetime-structure half is industry-solved; the memory-safety half is open outside garbage collection, because a spawned child holding a non-static reference is sound only if the language can guarantee the child stopped before the borrowed data died.

| language                 | mechanism                                                                                                                                   | sound          | ergonomics     |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------- | -------------- | -------------- |
| Rust (synchronous scope) | non-static borrow plus blocking join                                                                                                        | yes            | good           |
| Rust (async)             | no standard primitive; static-lifetime spawn; sound borrow only through joining combinators; parallel scoped spawn only via an unsafe crate | partial        | poor           |
| Swift                    | task groups; lifetime structure enforced; sendable closures forbid the shared mutable borrow that group semantics would make safe           | yes            | workaround     |
| Mojo                     | work in progress; an origins lifetime system; a structured-async proposal                                                                   | not applicable | not applicable |
| C++                      | RAII-joining threads with a stop token; sender/receiver async scopes (candidate); captured-reference safety is discipline                   | partial        | poor           |
| Trio, Kotlin, Java       | nurseries, coroutine scopes, structured task scopes; garbage collection removes the borrow-safety dimension                                 | not applicable | good           |

**Where gandr sits.** The **lifetime-structure half** is in the design and is arguably richer than a nursery: a nursery is fork plus a session protocol, and the parent cannot complete its derivation without consuming the join endpoints.
**One inherited guarantee is deliberately not inherited, and it is worth being exact about why.** In the linear-logic reading of sessions, fusing channel creation with parallel composition forces the well-typed process topology to be a **tree**, from which deadlock-freedom and global progress follow by construction — at the price that genuinely cyclic networks are not typable at all. gandr's fork **deliberately permits interleaving**, so it does **not** inherit that guarantee; what it has is fidelity and safety structurally, and deadlock-freedom only _within_ a single multiparty session, by projection coherence.
Cross-session and shared-session deadlock-freedom is a reserved question with its hooks placed and its answer unchosen.
The three named blockers of async structured concurrency map onto substrate features Rust lacks — silent forgetting is forbidden by linearity (the decisive advantage, and it **aligns with** ongoing leak-freedom and linear-types discussion rather than being that discussion); non-cooperative cancellation is answered by the one-shot stack plus typed unwind; and the need for a non-blocking executor is replaced by a static consumption obligation.

**Two corrections, both load-bearing.** "The static consumption obligation replaces blocking join" is **overclaimed**: type-level consumption does not by itself guarantee at _runtime_ that a borrowing child stopped before the borrowed data was freed.
And **borrowing is not in the core at all** — what is there is the cooperative-scheduler-as-handler scaffolding plus ordinary _immutable_ read-sharing through contraction; anything mutable needs this calculus.
Read "gandr has it" as "gandr has the concurrency scaffolding; the borrow half is deferred."

For the hard parallel case — disjoint mutable sub-slices handed to children — the mapping is one capability delegated per chunk, but **delegation moves session endpoints, not arbitrary capabilities**; delegating access capabilities is unbuilt.
The central soundness gap: typed unwind runs the _parent's_ obligations, **not** reclamation of a _child's_ still-held capability before the parent's data is freed.
"Unwind blocks until children release" is in the spirit of the rule and is **not specified**.

One more correction with a sharp edge: "shared borrow is a graded thunk" is **not** partially settled and is **not** in the adopted record — the design record states explicitly that none of that mapping is adopted.
Treat it as open, with the semantic mismatch of [[#mode-decision-05]] on top.

Remaining gaps: the exclusive-borrow, region, and split-borrow calculus is deferred and its sketch is unvetted invention; cross-configuration cancellation and unwind operational semantics are unbuilt; dynamic or weakly-structured spawning is unmodelled; the process-soup disjoint-capability non-interference invariant is unstated; cooperative cancellation cannot force-stop a mid-compute child; and multi-world borrowing is forbidden until the mobility validity judgment lands.

### Self-referential data and address stability

A value holding an interior pointer into its own storage, where a move relocates the bytes and the interior pointer keeps addressing the old location.

| language                       | mechanism                                                                                                                                                                                                                            | sound          | ergonomics     |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------- | -------------- |
| Rust                           | a library pinning type plus an auto-trait opt-out; unsafe construction; a marker for address-sensitivity; projection via macros; a sequence of soundness fixes for coercion and aliasing, and a 2026 regression in the pinning macro | partial        | poor           |
| C++ (self-referential movable) | a user move constructor fixes self-pointers on move; opt-in trivial relocation proposals                                                                                                                                             | partial        | good           |
| C++ (coroutine state)          | heap-allocated frame addressed by a stable handle                                                                                                                                                                                    | yes            | good           |
| Swift                          | reference semantics with stable heap identity; heap-allocated async frames; the non-copyable and non-escapable axes target a different concern                                                                                       | not applicable | good           |
| Mojo                           | a move initializer can fix self-pointers; non-movable types proposed; the compiler currently fails on indirect self-referential types                                                                                                | partial        | poor           |
| research                       | the _move model_ — memcpy-affine, move-constructor, or heap-pinned frame — decides whether self-reference is a type-system problem at all                                                                                            | not applicable | not applicable |

**Where gandr sits.** This is the one problem for which the pre-reboot source had **no orientation at all** in its main pass; the treatment below is its later appendix, and it is less developed than the other seven.
It is retained in full because it is the case where a wrong assumption would be most expensive.

**The async case dissolves rather than being patched.** gandr's suspension is a **control primitive, not a data-layout outcome**.
A suspended computation is captured as a first-class stack value — a reified evaluation context held in the value layer — and that continuation reaches its captured locals **by variable binding and environment lookup, never by an address offset into its own storage**.
What makes a lowered async function self-referential elsewhere is purely the implementation decision to lay the state machine out as a flat struct holding a local inline _and_ a pointer to that inline local.
**Self-reference in the async case is a layout phenomenon, not a semantic one** — and gandr models the semantics.
This places gandr's posture near the out-of-line, stable-identity family rather than near a movable inline future, but as a _semantic_ fact about the substrate, not as a committed allocation strategy.

**Linear residency would give the suspension a resource discipline, not an address.** When the captured prefix owns linear obligations, the design makes the reified stack itself linear: not duplicable (one-shot by default; multi-shot only when the captured obligations are empty), and not silently discardable.
That dissolves the _resource-level_ analogue of the aliasing question — exactly one owner exists by construction.
The _byte-level_ aliasing question has no counterpart, because gandr has no aliasing or optimizer model.

**The mobility judgment is a structurally cleaner posture than a retrofitted pinning type — on a different axis.** Mobility is carried from the start: unit and base types mobile, products and sums mobile if their components are, graded thunks generally immobile, endpoints and capabilities never mobile, and a reified stack immobile — "a continuation is the most world-bound object there is."
This is the moral equivalent of a **from-birth** movability bound, and **the default polarity is the safe one**: you must establish mobility to transport a value, and code-bearing and resource-bearing types are conservatively immobile, so a continuation cannot be shipped by accident. gandr pays no backward-compatibility trap because it never shipped a universal-movability assumption.

**And the bound on that claim must be stated every time it is made: this is world-level, not address-level.** A pinning type is about byte-stability within an address space; immobility says "you cannot migrate this continuation to another machine".
A value can be world-mobile yet address-sensitive, or world-immobile for reasons having nothing to do with interior pointers.
So **the discipline transfers** — track non-relocatability as a typed, default-safe property rather than an opt-out auto-trait — and **the mechanism does not**.

**Honest soundness status, one line each.** The async case is _sound-shaped_, which is a consequence of operating above memory and not a proof about any compiled implementation.
The self-referential-struct case has **no story at all** — the value language has no pointer and no interior reference, so the unsound pattern is not expressible: gandr neither suffers nor solves it, and the honest status is _absent_.
The aliasing-annotation class is **not applicable**, for the same reason: absent, not solved.

**Ergonomics.** Attractive in principle — no pinned receivers, no marker types, no unsafe projection, no projection macros — matching the observation that the coroutine-frame languages are ergonomically good precisely because the user never sees the problem.
Two caveats: this is a posture on unimplemented, unsurfaced features with no real-world evidence; and **linearity imposes its own tax, the dual of pinning's** — one-shot-by-default continuations, multi-shot only for obligation-free captures, and explicit abandonment.
For suspension-heavy code that is a real cost, differently shaped, not free.

**What the substrate lacks, enumerated:** no memory, layout, address, or ABI model — gandr can neither assert nor refute "moves are a byte copy", because address-stability lives strictly below its abstraction; no reference types whatsoever; no aliasing model; mobility on the world axis only; no regions; no move semantics in the byte-copy or move-constructor sense, so gandr **cannot be placed on the field's move-model axis at all** because that axis is below it; no specified field projection into a reified stack — suspensions are opaque, consumed only by resume or abandonment; and no in-place self-referential buffer story.

### Move semantics and the absence of a move hook

A move elsewhere is an unconditional bitwise copy that destructively invalidates the source, with no user hook, no move trait, and no opt-out bound.
The contested question is whether that is a limitation or a sound deliberate choice.

| language                    | mechanism                                                                                                                                                                                              | sound   | ergonomics                                              |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------- | ------------------------------------------------------- |
| Rust                        | bitwise move, no hook, destructive, no opt-out; immovability via a library pinning type                                                                                                                | yes     | poor                                                    |
| Swift                       | compiler bitwise relocation, non-customizable; destructive consumption; a non-copyable axis; no pinning construct                                                                                      | yes     | good for move-only; not applicable for customizing move |
| Mojo                        | customizable move and copy initializers plus a transfer operator; move customizable **and** destructive (no valid-but-unspecified husk); trivial and register-passable types barred from a custom move | partial | good                                                    |
| C++                         | a fully customizable move constructor; **non-destructive** (the moved-from object stays valid and is still destructed); proposals to recover byte relocation                                           | partial | workaround                                              |
| research: linear and affine | move is consuming a linear resource; sound and destructive by construction; classic theory adds no hook                                                                                                | yes     | not applicable                                          |

**Where gandr sits.** Squarely in the linear-affine camp, and more principledly than Rust: destructive move **is** structural linearity — no contraction means no double move, no weakening means the source is consumed exactly, and abandonment is forced to run unwind.
The asymmetry Rust is criticized for — customizable copy, non-customizable move — **does not arise**, because copy and move are _both_ structural.
And the customizable half that carries resource-safety weight, destruction, is present through typed unwind, from one mechanism rather than a separate trait.

**Corrections.** "An abandoned move-only value runs its recorded cleanup" holds today **only for stacks that captured obligations** — generalizing it is [[#mode-decision-16]].
"Moving a linear resource into a thunk" overstates the design: the adopted discipline is that a thunk captures **no** linear obligations; the capture-set relaxation is proposed, not adopted.
"Grade-encoded move-only-ness is explicitly forbidden" is an **inference, not an explicit prohibition** ([[#mode-decision-15]]).
And "sound by construction, arguably above Rust and Swift" is a design claim, not a proof — ironically self-undercut, since gandr has no memory model with which to make Rust's actual byte-level guarantee at all.

**Four motivating scenarios, honestly graded.** A future borrowing across a suspension is **dissolved** by the reified stack, with the borrow-capture detail still open.
A self-referential struct is **addressed in direction only**, by the deferred region discipline.
Intrusive-by-value and by-value foreign interop are **not addressed**, from the same root cause as Rust's — there is no relocation event to hook — and the answer is handle-based interop, falling short exactly where Rust does.
Given the design's control-plane framing, the last two are most plausibly **non-goals**.

**Three concept names the calculus must keep apart**, because every existing tool accumulates its complexity precisely by conflating them:

- **duplication** — copying, where the source survives;
- **relocation** — moving, where the source's storage is given up; bitwise or hooked, destructive or not;
- **address-stability** — pinning; a _constraint on_ relocation, not an operation.

### Access modes and parameter passing

Fine-grained, statically sound control over copy versus borrow versus consume versus mutate-in-place — for move-only resources, and for mutating a value nested behind an abstraction _without_ a copy.
Three sub-problems: in-place mutation of nested or computed storage; parameter modes proper; and projections with disjoint or partial borrows _through_ an abstraction boundary.

| language                               | mechanism                                                                                                                                                                                              | sound   | ergonomics                                     |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------- | ---------------------------------------------- |
| Swift                                  | borrowing, consuming, and in-out parameter modes; an explicit consume operator; a non-copyable axis; yield-once accessors plus non-coroutine projections; a law of exclusivity                         | yes     | good for the first two; partial for projection |
| Rust                                   | shared, mutable, and by-value; an index-mut operator returning references into real storage but no operator for computed slots; disjoint field borrows allowed, none through methods (view types open) | yes     | partial                                        |
| Mojo                                   | read, mutable, owned, and a parametric reference mode; a transfer operator; references project storage; origins                                                                                        | partial | good                                           |
| C++                                    | const-reference, reference, rvalue-reference, by-value; proxy references for computed slots, which leak the abstraction; no checking                                                                   | no      | workaround                                     |
| research: linear, graded, quantitative | linearity or graded usage; "use once" is consume and "use freely" is copy, as points of one semiring                                                                                                   | yes     | poor                                           |
| research: regions and permissions      | region, permission, and view typing for partial borrow through an abstraction                                                                                                                          | yes     | poor                                           |

**Where gandr sits, corrected.** The copyable-versus-non-copyable axis maps onto the two zones — contraction is copyable, no contraction is not — but **this is not settled substrate**: the linear zone holds only endpoints, capabilities, and obligation-owning stacks, **not general user data types**, so modelling an arbitrary non-copyable struct requires extending it to host user data ([[#mode-decision-15]]).

**Consume maps cleanly.** Take-and-consume lands soundly on linear consumption, and gandr is on the sound side of the destructive-versus-non-destructive fork.
Destruction-on-give-away maps onto typed unwind — **though that rule is stack-abandonment cleanup, not a per-value destructor, so "more general than Swift" is premature**.

**Borrow and in-out are not free, and this is where the whole calculus concentrates.** "A shared borrow is a graded thunk" is the weakest link — a forcing count is not a scoped read-borrow.
"An exclusive borrow is a linear capability" actually types **consume**, not borrow.
And the appealing claim that a take-and-return thread-through is cleanly typed by a receive-then-send session is **refuted**: capabilities are not session payloads, and no rule sends a capability over a session.

**The yield-accessor case splits cleanly, and the split is instructive.** The **control** half is an unusually good fit — a yield-once accessor _is_ a one-shot delimited continuation: capture the rest, resume to run the write-back.
But **"abandon runs the write-back automatically" is overclaimed** — unwind runs close, release, and drop, **not** the captured continuation; write-back runs on resume.
And the claim that this unifies three things Swift handles separately overreaches on value destructors, which gandr cannot express.
Meanwhile the **data** half — what is lent, namely a mutable place — **does not exist at all**.
Swift needed _both_ a coroutine accessor and a cheap non-coroutine projection; gandr has only the coroutine form, so the overhead tension lands harder here than there.

**Disjointness.** Splitting the linear zone gives a disjoint **ownership partition of separate entries** — that is partition-and-consume, not borrow, and calling it "disjoint borrows for free" inherits the same conflation.
Disjoint or partial access to _one_ aggregate needs a place-projection rule that does not exist, and through-abstraction projection is research-open everywhere.

**One clean correspondence worth keeping.** Static exclusivity corresponds to no aliasing of a linear entry; **dynamic** exclusivity corresponds to manifest sharing's acquire and release, where acquire _is_ mutual exclusion.
The fallback wiring between the two is open.

Remaining gaps: no mutable references, places, or storage; no region or lifetime calculus; no place or capability projection rule; no non-coroutine projection; no mode sort; the borrow concept is not cleanly placed in the linear-versus-graded split; partial borrows through abstraction are open field-wide; and modes for ordinary copyable values are unaccounted for entirely.

### Must-consume resources and the destruction protocol

A must-consume resource whose destructor may take parameters, may fail, and whose non-consumption is a compile error.

**The parameterized, fallible destructor is Vale's "Higher RAII", not Mojo's**, and the pre-reboot source conflated the two; Mojo's own destructor specifics were flagged uncertain there and are not relied on here.

**Where gandr sits — its strongest adopted-design facet.** A must-consume resource is a linear entry, and forgetting it is a type error: the guarantee Rust cannot give (leaking is safe there) and C++ cannot give for a _checked_ must-explicitly-consume type — **noting that "a C++ destructor always runs" is true only for automatic-storage objects, not in general**.

gandr can **overshoot** the parameterized-destructor model by making the resource a session-typed endpoint, so the consumption protocol names the legal outcomes and can fail: a transaction whose type offers a commit branch carrying a result and a rollback branch is a destructor protocol expressed in the type.

**And the correction that must ride with it:** "abandonment automatically runs rollback" is **overclaimed and unspecified**.
Typed unwind runs generic close, release, and drop; **nothing designates a rollback branch as the unwind path**, and nothing supplies the data a commit branch would need.
The primitives are in the design; "abandonment implies rollback" is not.

Two smaller corrections in the same section: there is **no general composite or struct contagion rule** that would make a type non-trivial because a field is — the capture-set relaxation is thunk-specific and proposed, and a linear obligation cannot be a field of a product at all; and **register-passability, ABI, and calling conventions are entirely absent from the corpus**, so recommending that they be separated out is sound but must not be phrased as though a home for them already exists.
The trivial-type predicate such a calculus would key on is likewise undefined.

The three-tier spectrum the design does supply: unrestricted intuitionistic values, affine graded thunks with drop, and linear entries.

## Foreign-interface design impact

**The lead finding.** gandr is conceptually better-positioned than Rust on immovability **and** on the copy-move-destructor axis, for one precise reason — it never assumed universal movability — and with one precise caveat: **it has no memory, layout, address, or ABI model at all.** The dynamics are a substitution and environment machine over terms and values, not a byte-addressed store.
So the foreign-interface-relevant properties are **latent wins requiring a new layer**, not realized mechanisms.

**Copy and move at the boundary are tier-stratified**, neither copy-by-default nor move-by-default: intuitionistic ground data is copyable (contraction is the trivially-copyable tier); graded thunks are copy-budgeted and droppable exactly when the zero grade is below their grade (the affine tier); linear resources are move-only and **destructive by construction**, with no valid-but-live moved-from husk.

Two consequences:

1. gandr lands on the **sound side** of the destructive-versus-non-destructive fork automatically — but, like Swift, **cannot represent a foreign moved-from-but-live source**.
   A foreign move must be synthesized as _construct-at-destination-then-destroy-source_, never an in-place byte move of a non-trivial value.
2. gandr has customizable **destruction** through typed unwind but **no relocation hook**, because there is no relocation event in the dynamics to hook. (And "customizable destruction" overstates the rule: it is typed cleanup of linear obligations, not a user-pluggable per-type destructor.)

**The boundary hazard, restated because it is the sharpest one here.** Contraction is a silent logical duplication.
Mapping a foreign type with a non-trivial copy, move, or destructor onto an intuitionistic value would let contraction "copy" it without running the copy constructor — unsound, and the mirror image of C++'s copy-by-default footgun.
Hence [[#mode-decision-14]].
At the boundary gandr therefore behaves as Rust does: a trivial and relocatable subset by value, everything else opaque behind a handle.
The copy-by-default versus move-by-default mismatch is **side-stepped by refusing implicit foreign value semantics, not reconciled**.

**What the field's ABI efforts teach**, each with the correction the source's own reviewer applied:

- **crABI** (an unmerged Rust RFC) defines its own option and result types, because niche-optimized standard types cannot be reused across a stable boundary — and **leaves unresolved whether a non-trivial destructor runs and who frees**.
  The accurate statement is that it has a _manual_ free convention; it does not standardize _automatic_ destructor execution. gandr answers this better through ownership in the calling convention ([[#mode-decision-12]]).
- **`stabby`** pins a C-based layout and recovers niche-optimized compact sums, at the documented cost of losing pattern matching and of compile time.
  A compiler change degraded its vtable handling — **a _performance_ regression, not a soundness or layout break**; the fair characterization is the fragility of a stable ABI on an unfrozen substrate.
- Extensible-vtable ABI evolution — adding fields and operations without breaking compiled consumers — belongs to **`abi_stable`**, not to `stabby`, which the pre-reboot source had misattributed; the correction is worth nothing unless both crates are named, and the lesson maps onto gandr's module signatures with load-time matching.
- The two efforts converge on the **two-representation discipline** of [[#mode-decision-11]].
- The C++ bridging tools — **cxx**, **autocxx**, and **crubit** — converge on **trivial versus opaque**: trivially relocatable types may cross by value, everything richer is opaque behind indirection.
  **cxx** backs the trivial _claim_ with a generated C++ `static_assert` — **a real check, not an honour system**; the honour-system mechanism is the `trivial_abi` compiler attribute, which is trusted without verification.
  All of them punt on auto-bridging templates and generics.
- **Trivial relocatability is not standardized in C++.** One proposal was voted into a working draft and then **removed**, over comments that relocation as specified could do more than a bitwise copy; a competing proposal was never in that draft, so it cannot have been removed from it.
  The durable lesson holds regardless: **byte-relocatability needs an explicit annotation and is not type-inferable even in C++**, so gandr should design its relocatability query as a **pluggable input**.

**Two positive models to copy.**

- A **value-witness-table** model is the lingua franca: each foreign or opaque type is an abstract type carrying declared lifecycle operations — a foreign copy constructor becomes a copy witness, a foreign move constructor becomes a _take_ (move-then-destroy-source), a foreign destructor becomes destroy.
  This is the per-type move hook Rust lacks; its limit is the same as gandr's, namely no first-class immovable value type.
  It also surfaces foreign copies **explicitly** and keeps foreign containers foreign, refusing implicit conversion.
- Putting the **calling convention in the type system** as an effect on function and function-pointer types, and **enforcing it at load**, converts the classic silent ABI mismatch into a load-time error.
  That language's foreign interface is otherwise C-ABI-only, which is exactly the recommended first-version posture ([[#mode-decision-13]]).

**What the dynamics must grow, in rough order:**

1. a **value-representation and layout model for a trivial tier** — a frozen, C-compatible layout for the by-value-eligible set — kept distinct from the optimizable internal representation ([[#mode-decision-11]]);
2. a **foreign-call effect** with ABI-constrained payloads plus a calling-convention annotation enforced at module load — noting that the load-time check gandr "already plans" is dynamic **signature** matching in the module design, **not** ABI-tag checking, so mapping one onto the other is a proposed extension;
3. a **native world** in which foreign pointers and handles are immobile and, when owning a resource, linear-resident;
4. **foreign destructor execution wired into unwind**, including the refinement that an unwind obligation may run a blocking, uninterruptible, fallible foreign call ([[#mode-decision-02]]) — which is a refinement **this document proposes**, not one the design already flagged;
5. a **minimal declared trusted boundary**: the declared ABI signature plus the declared copy, move, and destroy capabilities of each foreign type.

## The comparison table

One row per problem-and-language pair, with gandr's orientation.
Corrections above are reflected in the gandr column.

| problem                | language                                                   | mechanism                                                                          | sound          | ergonomics            | gandr orientation                                                                                                                                                         |
| ---------------------- | ---------------------------------------------------------- | ---------------------------------------------------------------------------------- | -------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| async destruction      | Rust                                                       | experimental async drop; abandoned poll-ready predecessor                          | no             | poor                  | explicit typed unwind on an abandoned obligation-owning stack; no implicit destructor; **conditional** — abort path, cancel-during-cancel, and the effect row unspecified |
| async destruction      | Swift                                                      | no async deinit; synchronous isolated deinit; task workaround                      | partial        | workaround            | linearity gives the no-leak base the synchronous rule deliberately does not                                                                                               |
| async destruction      | Mojo                                                       | synchronous destructor plus linear explicit-destroy types                          | partial        | workaround            | same family; the explicit abandonment path is what Mojo lacks                                                                                                             |
| async destruction      | C++                                                        | none adopted; structured-concurrency scopes instead                                | not applicable | workaround            | cleanup is a computation, run at an explicit site                                                                                                                         |
| async destruction      | research: linear, uniqueness                               | consumed-exactly-once invokes a possibly-async cleanup                             | yes            | poor                  | gandr is in this tier **as designed substrate**, not as a proposal                                                                                                        |
| completion I/O         | Rust (owned buffers)                                       | by-value buffer; the driver holds it and cancels until completion                  | partial        | workaround            | buffer as a linear capability plus dual session endpoints — **catalogued, not designed**; no mutable address-stable buffer type                                           |
| completion I/O         | Rust (proposed linear)                                     | must-move types with a leak bound                                                  | yes            | not applicable        | the linear obligation is native; the application binding is unproven                                                                                                      |
| completion I/O         | Swift                                                      | cooperative cancel keeps the owned buffer alive; raw-pointer foreign boundary      | partial        | good                  | ties the buffer to a linear obligation, decoupled from the cancellable stack                                                                                              |
| completion I/O         | C++                                                        | non-owning views; lifetime rules normative and unenforced                          | no             | good                  | session fidelity makes skipping submit-then-complete a type error                                                                                                         |
| completion I/O         | Linear Haskell                                             | linearity on arrows; exactly-once protocol                                         | yes            | poor                  | same enforcement tier, plus sessions and typed-unwind cancellation                                                                                                        |
| cancellation safety    | Rust                                                       | cancel is a drop at any await; safety is prose                                     | partial        | poor                  | solves the leaked-obligation half; **state corruption is not solved** — resource cleanup is not invariant restoration                                                     |
| cancellation safety    | Swift                                                      | cooperative flag; deferred unwind; deinit; task still running                      | partial        | good                  | checkpoints are perform points, hence type-visible; delivery timing unpinned; no shielded section                                                                         |
| cancellation safety    | C++                                                        | unsafe coroutine destroy; stopped signal; RAII                                     | partial        | poor                  | handlers plus linear continuations is the research's own fix; cleanup is a computation                                                                                    |
| cancellation safety    | Kotlin                                                     | cancellation exception at suspension; non-cancellable context                      | partial        | good                  | **no shielded-section analogue** — a gap                                                                                                                                  |
| cancellation safety    | research: linear, session                                  | use-exactly-once; protocol typestate                                               | yes            | poor                  | the designed substrate is this, for well-typed programs only                                                                                                              |
| structured concurrency | Rust (sync scope)                                          | non-static borrow plus blocking join                                               | yes            | good                  | fork plus sessions makes "no child outlives the scope" a static obligation                                                                                                |
| structured concurrency | Rust (async)                                               | no standard primitive; unsafe scoped spawn                                         | partial        | poor                  | the forgetting blocker is neutralized by linearity; **abort-path child-capability reclamation is unspecified**                                                            |
| structured concurrency | Swift                                                      | task groups; sendable over-restricts shared mutable borrow                         | yes            | workaround            | could beat this on disjoint mutable fan-out **if** split borrows land — unbuilt                                                                                           |
| structured concurrency | C++                                                        | joining threads, stop tokens, async scopes                                         | partial        | poor                  | dynamic and weakly-structured spawning is unmodelled                                                                                                                      |
| structured concurrency | Trio, Kotlin, Java                                         | nurseries and scopes; collection removes the borrow dimension                      | not applicable | good                  | adds the authority and borrow-safety dimension a collected nursery cannot                                                                                                 |
| self-reference         | Rust                                                       | library pinning type plus opt-out trait; recurring soundness fixes                 | partial        | poor                  | **no dedicated orientation** in the source; the async case dissolves into the reified stack; structs rest on deferred regions                                             |
| self-reference         | C++                                                        | move-constructor pointer fixup; stable coroutine handle                            | partial / yes  | good                  | immobility is world-level, **not** the address-level property this needs                                                                                                  |
| self-reference         | Swift                                                      | stable heap identity; heap async frames                                            | not applicable | good                  | no address model; immobility is derived, not bolted on                                                                                                                    |
| self-reference         | Mojo                                                       | move-initializer fixup; non-movable types proposed                                 | partial        | poor                  | no opt-out trap and no pinning complexity — _if_ a memory model is ever added                                                                                             |
| move semantics         | Rust                                                       | bitwise move; no hook; pinning for immovability                                    | yes            | poor                  | destructive move **is** structural linearity; no copy/move asymmetry; destruction via typed unwind                                                                        |
| move semantics         | Swift                                                      | compiler bitwise relocation; non-copyable axis                                     | yes            | good / not applicable | same camp — sound, non-customizable move; needs no pinning type                                                                                                           |
| move semantics         | Mojo                                                       | customizable move initializer plus transfer; destructive                           | partial        | good                  | declines a move hook ([[#mode-decision-10]]); dissolves the motivating cases instead                                                                                      |
| move semantics         | C++                                                        | customizable move constructor; non-destructive                                     | partial        | workaround            | foreign moves synthesized as construct-then-destroy; no in-place byte move of non-trivial values                                                                          |
| move semantics         | research: linear, affine                                   | move is consuming a linear resource                                                | yes            | not applicable        | the designed substrate is this                                                                                                                                            |
| access modes           | Swift                                                      | borrowing, consuming, in-out; yield-once and direct accessors; exclusivity law     | yes            | good / partial        | consume maps soundly; **borrow and in-out do not**; the accessor's control half fits the reified stack, its data half is absent                                           |
| access modes           | Rust                                                       | shared, mutable, by-value; disjoint fields but not through methods                 | yes            | partial               | zone splitting is ownership partition, **not** borrow; through-abstraction is open everywhere                                                                             |
| access modes           | Mojo                                                       | read, mutable, owned, reference; transfer; last-use destruction; origins           | partial        | good                  | must-consume resources are linearity plus unwind, overshooting via session-typed consumption; register ABI absent from the corpus                                         |
| access modes           | C++                                                        | const-reference and friends; proxy references; no checking                         | no             | workaround            | static exclusivity is no-aliasing of a linear entry; dynamic is acquire and release                                                                                       |
| access modes           | Vale                                                       | generational references, region borrowing, destructors with parameters and results | yes            | good                  | matched or beaten via linearity plus session-typed consumption                                                                                                            |
| access modes           | Austral, Linear Haskell, Granule, quantitative type theory | pure linear, multiplicity, or graded usage                                         | yes            | poor–workaround       | the intuitionistic / graded / linear trichotomy is the same design space                                                                                                  |
| access modes           | Cyclone and the regions line                               | region-based memory; partial borrow through abstraction                            | yes            | poor                  | "scopes are like regions" is an analogy only; no region construct exists ([[#mode-decision-06]])                                                                          |

## Open design choices, with status and revisit conditions

Distinct from the decision register above: these are the shape-of-the-calculus questions rather than its individual knobs.

**Model address-stability at all, or keep it strictly below the abstraction?** Either stay address-agnostic — interior-pointer self-reference is never expressible, the async case dissolves, intrusive structures are a backend concern — or grow an address and region model so the reference calculus can express interior pointers and scoped stability.
_Recommendation:_ stay address-agnostic in the core.
_Status:_ the address-agnostic stance is **settled**, in the sense that it is what the substrate simply is; whether to _ever_ add the other is **open**.
_Revisit_ when a concrete need for intrusive structures or zero-copy buffers enters scope.

**Is the address-sensitivity of a _lowered_ suspension a backend obligation or a type-system concern?** Because the async case dissolves at gandr's level, the movable-inline-future-plus-pinning-tax versus heap-frame tradeoff **reappears at the lowering boundary, undecided by the substrate**.
Either say nothing about how a reified stack is represented, or carry a derived annotation on the address axis to constrain the lowering.
_Recommendation:_ say nothing now; record the annotation as a reserved hook.
_Status:_ **open**.
_Revisit_ when the dynamics lower to a real machine with real memory.

**If references land, which borrow-as-what mapping?** The sketch is: shared borrow as graded thunk, exclusive borrow as linear capability, scopes as regions, plus mode-bounded polymorphism.
_Status:_ **open, and explicitly not adopted** — the design record adopts none of these mappings, and the external rule sketch that accompanied them (permission tokens, a subcapability relation, a split-borrow trace) is **unvetted invention that appears in no specification**.
Treat the whole mapping as the design space, and assume nothing is settled.

**The default polarity of any relocation or address-mobility judgment, if one is introduced.** The lesson from the retrofitted pinning type is that movable-by-default with opt-*into* immobility is the fragile polarity, because a stray opt-out silently reintroduces unsound assumptions. gandr's world-mobility already takes the safer one.
_Recommendation:_ if an address axis is ever added, make non-relocatable the conservative default and require a _proof_ of relocatability.
_Status:_ **open**, contingent on the first question.

**One-shot versus multi-shot for obligation-owning suspensions.** _Status:_ **settled** in the design — one-shot by default, multi-shot only when the captured obligations are empty, with abandonment running typed unwind.
_Revisit_ only if a sound account of multi-shot resumption of obligation-owning stacks emerges.

**Field projection into a reified stack, or keep it opaque?** Leaving it opaque sidesteps the structural-projection problem that defeats a retrofitted pinning type's type system entirely.
_Recommendation:_ opaque for now.
_Status:_ **open**.
_Revisit_ if introspection into suspended jobs — the job-control surface — needs structured field access, at which point the structural-versus-non-structural distinction must be confronted directly.

## A citation-hygiene guard

**A widely-surfaced claim that Rust 1.95 added linear types through a `MustMove` trait is fabricated.** The 1.95 and 1.96 release notes contain no such feature; must-move and linear types remain exploratory there.
The version and the trait name are reproduced deliberately: a guard against a _specific_ fabricated claim only works if enough of the claim survives to be recognized on a second encounter.
The guard generalizes past its instance: treat any "language X now has linear types" claim as unverified until the primary release record is read, because this class of claim is repeated confidently and is cheap to check.

## What this document does not carry

The pre-reboot source ended with a **session-types and typed-concurrency literature survey** — binary and multiparty sessions, the linear-logic correspondence and its tree-topology restriction, deadlock-freedom for cyclic and shared topologies, affine and exceptional sessions with peer-propagating cancellation, and the context-free, dependent, and gradual extensions.
That material is **parked with a reason, not dropped**: it belongs to the sessions-and-effects absorption rather than to the mode calculus, and splitting it here would leave it half-stated in two places.
Two of its findings are load-bearing _for this document_, and both are carried where a reader meets the problem they bear on rather than here: peer-propagating cancellation, in the cancellation-safety section, and the tree-topology restriction gandr's fork declines to inherit, in the structured-concurrency section.

## Source and confidence

Written against four sources, named because a document with an undeclared source set cannot be fidelity-reviewed.

1. The pre-reboot **mode-calculus state-of-the-art analysis** in full — its decision register, its per-problem catalogue, its foreign-interface impact, its comparison dataset, its literature anchors, and its self-reference appendix.
2. The **value-semantics design record's** access-mode sections and boundary note.
3. The **grade-to-store-region design note**, which supplies the finding that residency is justified by escape rather than by grade.
4. A separate pre-reboot **capabilities, effects, and ownership-over-time literature sweep**, which is the source of the design-space camps section and of all three of its through-lines — the capability-and-effect duality and its non-dominance, the community scoping of the move off linearity, and the partial-versus-complete quantale false friend.
   It is contributor-context material rather than a specification, and its conclusions are restated here rather than referenced.

**A provenance fact the reader should know.** That analysis is not in the pre-reboot specification tree: it lived in a separate analysis directory that was deleted, and it is recoverable only from that project's history.
The deleting change stated the material had been moved to a sibling notes repository; **it had not been**, and a filename sweep of that repository finds no trace of it.
Every load-bearing claim from it is therefore restated here in full rather than referenced.

**Confidence, by class.**

- **High** — the as-built substrate statements, each verified against the named module in the tree at write time (`gandr-core-checker`'s `ctx`, `grade`, and `types` modules).
- **High** — the decision register's content and the adversarial corrections carried at each claim, which are transcribed from the source rather than re-derived.
- **Medium** — the literature attributions, whose identifiers were verified against publisher or preprint records at this pass but whose _claims_ were not re-read from the papers.
- **Low, and marked as such at the section** — the per-language mechanism tables' proposal, RFC, and issue numbers, carried unverified; and the self-reference treatment, which the source itself flagged as less developed than its siblings.

Where the design record and the built tree disagree, **the tree wins on status and the design record wins on payload**, and the disagreement is stated at the claim rather than reconciled silently.
