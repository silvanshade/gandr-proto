# Proof engineering

This track owns how mathematics is mechanized in this project, independent of gandr-specific content: the substrate the Agda development is built over, the representation and organization disciplines, and the coherence-cost engineering that keeps a higher-dimensional formalization tractable.
What the mathematics _says_ about gandr's model is the [[metatheory]] track; the lane's operational workflow — substrate policy, residual discipline, layout mechanics, flags, gates, dependency policy, solvers, opacity, commit shape, and the done-rule — is `docs/workflow/agda.md`, which deliberately carries no doctrine.
**This document is the doctrine's single cross-module home**: the tree-wide rules of representation, characterization, reasoning style, namespacing, and the boundary telescope, which no single module header can own.
Per-module doctrine stays in the module headers, which remain authoritative for the structures they own, and the per-structure mathematical plan stays there too.
Detailed remaining work is in [[proof-engineering/roadmap]].

## The substrate tower

**Everything is built over ∞-graphs**, the coinductive presentation of a globular carrier: a type of cells at each dimension with a coboundary at every parallel pair, observable on demand, never completed.
`Gandr.Graph` is the ambient category — initial and terminal objects, coproducts, products, the discrete and codiscrete inclusions of `Set`, the globes, the exponential, and the disc telescopes.

**Structures are projected out of carriers, not stacked on each other.** A `Setoid` equips a carrier's cells with reflexivity, transitivity, and symmetry and **carries no laws** — the lawless proof-relevant equivalence, the honest floor.
A `Category` is composition-and-identity structure **on** a carrier, not a bundle containing one; a groupoid likewise.
Because a hom-setoid is literally the carrier's coboundary at two objects, read off by projection, two structures over one carrier are comparable without coercion — which is why setoids are _projected out of_ the richer structures rather than the richer structures being built on top of setoids.
One bundle serves every dimension, since any level's coboundary is itself an ∞-graph.

**Laws are cells one dimension up.** Associativity does not say two bracketings are equal; it supplies a 2-cell a consumer transports along explicitly.
A strict law would be an equation in the ambient theory, silently promoting the setoid to a set; stating laws as cells is what keeps the development inside `SETOID`, where its results actually hold.
Consequences that are structural, not stylistic: quotients need not be effective (a canonical representative is _structure_, supplied by `Rigid` — see the metatheory's ambient section); a map respects an equivalence only when shown to; and the Yoneda correspondence is proved as pointwise value cells, never as an equality of records, because the development has no way to compare records and deliberately never does.
`SETOID` itself is constructed as a named category, so statements quantifying over categories genuinely have the ambient in range.

**Above the content, the tower is empty — never trivial, and this is not a truncation.** Three things must be kept apart:

* **`𝟘` above the last dimension that carries content** — the correct choice.
  It says there are no cells there; it asserts nothing, discharges nothing, and if a structure later turns out to have genuine higher cells the dimension opens up.
* **`𝟙` above** — **forbidden by default.** A terminal hom makes every coherence hold automatically, silently discharging obligations nobody checked.
  That is the failure mode, and it is what "do not truncate prematurely" is about.
  Use it only with a stated reason, as `Setoids`' homotopies do: a `Category`'s 2-cells are the equality on its 1-cells, so above `SETOID`'s homotopies there is nothing left to track, and `disc` there would deny a homotopy the reflexivity it has no reason to deny.
  `Gandr.Category.Instances.INDISC` is the deliberate degenerate case kept beside it as a contrast — every field the unique cell, so every law holds for the reason a terminal object makes every law hold, which is what makes it the category that proves nothing.
  A convention that fills trivial higher dimensions with `𝟙` as a matter of course is **not** this tree's convention, and a reader arriving with that habit should read this bullet as the correction.
* **Forcing uniqueness of identity proofs** — **out of scope entirely.** The `--without-K` mandate is binding and neither UIP nor definitional proof-irrelevance may enter through any shortcut.
  Using `_≡_` as a structure's 1-cells carries no UIP claim: nothing above it is asserted, so no two proofs of `f ≡ g` are ever identified.
  Where set-ness is genuinely needed it enters as a `UIP` **parameter** on the index type, as the grafting unit laws take it, never through a shortcut.

A fourth choice exists and is also wrong for a structure that stops: `Gandr.Graph.𝔾.Id` continues with the identity type at **every** dimension.
It is honest — nothing is truncated — but it offers a whole tower no consumer asks for, which is why `≡°` is `Id` stopped at dimension 1 by `𝟘` rather than by `𝟙`.

**Discarding is an assertion, and the rule above is the dimension-axis instance of it.** Wherever an operation produces a quantity its current consumers do not read, carrying it is `𝟘` — it asserts nothing, discharges nothing, and reopens if a consumer appears — while collapsing it to the unit is `𝟙`, and silently discharges a question nobody asked.
The two are not "carry" against "abstain": **the unit is a value**, so the collapse is a commitment, and it is the one commitment no reader can find later because nothing in the tree records it.

The tell is the same one the refuters serve: after the collapse, **no term can witness the other answer**, because the object that would witness it is the object that was dropped.
Weigh it that way and the choice reads correctly, since the collapse is always the cheaper-looking side — it removes a field, and the thing it costs is invisible by construction.

Three instances stand in the tree, and reading them together is what makes the pattern checkable rather than a slogan:

| the collapse                                         | what it looks like                            | what it silently asserts                                                              |
| ---------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------- |
| a terminal hom above the last dimension with content | a truncation                                  | the coherences up there hold                                                          |
| a pool discipline on the cell shape                  | structural wheel-freeness                     | there are no wheels — and `WheelFree` loses its refuter                               |
| dropping the closure's vertexless circles            | a total operation on the carrier as it stands | every object has dimension one, at every colour ([[metatheory#The rung, identified]]) |

This is not the **unobserved** row of the sort table below.
That row is for a piece that is never discriminated; each collapse above drops something the producing operation had to discriminate in order to drop it, so the choice is between carrying what was detected and asserting a value for it.

**And a fourth case hides inside that row, which is the one to watch.** A quantity can go undiscriminated because nothing observes it, or because **the operation that would observe it is not built yet** — and from inside an incomplete structure the two are indistinguishable, since neither leaves a term behind.

| the quantity is…                                       | reading     |
| ------------------------------------------------------ | ----------- |
| produced and carried                                   | `𝟘`         |
| produced and collapsed to the unit                     | `𝟙`         |
| never discriminated by anything                        | unobserved  |
| not discriminated **yet**, its operation being unbuilt | **not yet** |

The fourth is `𝟙` on a delay.
Build the missing operation the obvious way and its degenerate clause returns the unit, so the collapse is made by the implementation rather than by anyone — which is the same failure as before, arriving through a gap instead of through a choice.

**So a carrier identified with a named structure owes that structure's operations, enumerated.** Each is then either built or explicitly owed; what must not happen is leaving the set unenumerated, because an unbuilt operation is indistinguishable from an absent one and the difference is resolved by whoever writes the first implementation.

This is not "build everything", which is not affordable and which this tree deliberately declines all over.
The obligation is the **enumeration**, and building is what you do where you can.

The instrument is the one the arity interface already demonstrates: a **record whose fields the typechecker demands**, rather than a prose list a reader has to remember.
The worked case is the circuit rung — the substrate is identified with the nonunital wheeled props, a wheeled prop's operations include contraction, and contraction was neither built nor recorded as owed until it became a field ([[metatheory#The arity interface, universe-style]]).
Nothing was wrong in the tree while it was missing; it was simply not visible as missing, which is the whole cost.

### How a `Set`-level structure presents as a `Category`

`Category` is a structure over an `∞Graph`, so 2-cells are where its laws live; most structures in this tree are `Set`-level, with propositional `_≡_` for equality.
The bridge is the **discrete setoid on the identity type**, one dimension up from `Gandr.Graph`'s `disc`:

* 0-cells — the objects;
* the hom at `(x, y)` — an ∞-graph whose 0-cells are the morphisms and whose **1-cells are `f ≡ g`**, so the setoid relation _is_ the identity type and `Category.homˢ` is the identity setoid;
* **above that, `𝟘`.**

That ∞-graph is **`Gandr.Graph.𝔾.≡°`**, and the `Setoid` on it is **`Gandr.Setoid.≡ˢ`**; both carry the reasoning discipline below.
`Category`'s fields all land at or below that level — `mon-λ`, `mon-ρ`, `mon-α` and `seq↕` are `≡`-cells — which is the record being _lawless at its last dimension_: it states the laws and imposes no coherence among them.

The same pattern will repeat for the discrete category, the discrete groupoid, and the rest as those modules land.

### A structure that stops declares a region; it is not extended to reach one

**Where a structure has content is declared, not discovered.** What follows is how that is arranged, and why the two obvious alternatives were rejected.

Truncating with `𝟘` has a consequence that must be met head-on rather than routed around: a **dimension-wise certification cannot hold** of such a carrier, because certifying at every address demands cells at every address and there are none above the content.
The tempting repair is to extend the carrier upward until the certification holds.
**Do not.** Both ways of doing it were built and checked, and both cost more than they pay:

* **with `𝟙`** — the certification above the content becomes a _vacuous_ structure whose laws are discharged by `tt`, and the whole reasoning suite is then available at those addresses returning `tt`, with nothing at the use site distinguishing an informative application from an empty one.
  That is precisely the silent discharge the `𝟙` bullet above forbids, re-entering through the door that uniformity opens.
* **with `Id`** — honest, and it does hold, but it leaves _"the identity tower is the intended content"_ and _"the tower is filler for a region nobody observes"_ indistinguishable in the type.

**The region is the parameter instead.** `Gandr.Graph.At 𝒮 Ξ P` certifies `𝒮` at exactly the addresses admitted by `P`; the total region is `Total` (what `Everywhere` supplies, through `everywhere→At`), a structure whose content stops at the carrier is the singleton region (`Only⋆`, supplied by `at⋆`), and bounded depth sits between.
So `≡°` keeps its `𝟘`, nothing is extended, nothing is marked, and where a structure has content is **stated in its signature** rather than discovered by whoever next needs it.

**This gates rather than labels, and the address code is what does it.** `Disc` has injective constructors, so an out-of-region address is discharged by **constructor disjointness** — `at⋆`'s second clause is `()`.
Reasoning above a declared region is therefore not merely unwise and not merely unmarked: the region witness has no inhabitant, so the suite cannot be formed there.
That is a second dividend from reifying the address — the code was introduced so a statement could _bind_ its dimension, and it also lets a statement _refuse_ one.

**The region is per doctrine, not per carrier, and the two must not be conflated.** `Setoid` has content at dimensions 0–1 and `Category` at 0–2, so one carrier can serve the first and fail the second: over `≡°`, `Setoid` is inhabited (`≡ˢ`) while `Category` is not (`Gandr.Category.ℂ.≡°-not-category`).
A `Set`-level _category_ accordingly presents with `≡°` on each **hom** — `δ° x y = ≡° (H x y)` — and that carrier's region is `Only⋆`.
That former is **`Gandr.Graph.𝔾.homs°`**, taking the objects and the hom family.
It is named once rather than inlined because every `Set`-level category instance goes through it, and its name is deliberately descriptive: what the family at the homs composes into is the `Category` instance's claim, never the carrier's.
Reading a refutation of the certification as evidence that the certification is _stronger than the structure_ is an error this tree made and shipped; check the bare structure first.
In the other direction the rule is the same one read backwards: **a refutation of a _certification_ must never be read as a refutation of the bare structure.** **`homs°` is where that separation is real**, and it is the reason the former is worth naming twice over: `Gandr.Shape.Structure.WIRING` inhabits the bare `Category`, and `Gandr.Category.ℂ.homs°-not-everywhere` refutes the dimension-wise certification over the same carrier.

### The sort discipline

**The dimension is bound as a first-order code, never inferred.** A cell's type is a projection spine, and coinductive records have no eta, so anything that tries to infer a carrier or boundary from a cell gets stuck — a theorem about the unifier, not a limitation to engineer around ([[#The boundary telescope]] carries the result and what it buys).
The disc telescope is an inductive code interpreted onto the carrier; generic statements bind one telescope and instantiate at concrete addresses definitionally; the address code lets a statement bind its dimension _and_ refuse one.

This is one instance of a sort discipline worth stating once, because the known failure modes of higher-dimensional formalization are violations of it:

| the piece needs to be…                   | render it as…                                               | here                                                                                        |
| ---------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| discriminated, matched, or recursed over | a **code** — inductive, injective constructors, first-order | `Disc`, the witness relations, the arity kits' graphs                                       |
| witnessed or transported along           | a **certificate** — a cell, proof-relevant, carried         | 2-cells, the law witnesses, `Append` and friends where they are carried rather than matched |
| neither                                  | **unobserved**                                              | object equality; record identity                                                            |

Forcing a certificate into a code is what uniqueness-of-identity-proofs and premature truncation are; leaving no code column is what makes an instance unwritable, because unification has nothing to grip.
The design vocabulary for the tower is the explicit-coherence one — diagrams over an index category with all higher coherence laws carried explicitly [@kraus-sattler-2017-space-valued] — and the tower owes an equivalence statement against the Reedy-fibrant form of the same data ([[proof-engineering/roadmap]]); coinduction under guardedness is the currency that takes this to ω without a primitive, priced against the interval, modality, and finite-level alternatives in the metatheory's ambient-primitive policy.

## The boundary telescope

A cell's type is a spine of projections — `Ξ .δ° a b .δ° f g .ϵ°` — and it grows with dimension.
**Implicit arguments can never elide it, and this is a theorem about the unifier rather than a limitation to work around.** Reconstructing the prefix from a cell's type poses a constraint of the form `ϵ° ?Ξ ≈ ϵ° Ξ₀` — a metavariable under a projection — which Agda solves only by eta-expanding the metavariable, and **coinductive records have no eta**.
Anything that tries to infer a carrier or a boundary from a cell will get stuck, every time, and no amount of restructuring the record changes that.

The route around it is not inference, and it is not a macro.
It is to **bind the telescope explicitly, as a first-order inductive code**.

### The code, and why it is a code

`Gandr.Graph`'s `Base` module carries the reified telescope: `Disc` — `⋆` for the carrier itself, `Θ ▸ᵈ x ⇴ y` to descend one parallel pair — together with `⟦Disc⟧`, which interprets a telescope as the ∞-graph it lands in.
`Disc` is **inductive with injective constructors**, which is what the projection spine is not: it can be matched on, recursed over, and discriminated.
That is the code column of [[#The sort discipline]], instantiated at the one place the whole tower depends on it.
The two are declared mutually, since `_▸ᵈ_⇴_` types its parallel pair through `⟦Disc⟧` of the prefix — small induction-recursion, which the flag regime admits ([[#The toolchain probe matrix]]).
`⟦Disc⟧` appears there in an argument's _type_ and never in an index, which is exactly the line the witness discipline draws.

### What it buys, verified

A statement generic in the dimension **binds one `Θ` as a parameter** and is written over `⟦Disc⟧ Ξ Θ`.
Nothing is inferred from a cell, so the negative result above never fires.
Four properties carry the design, and it matters which of them are definitional and which are pinned by a proof:

* `⟦Disc⟧ Ξ ⋆` is `Ξ`, and `⟦Disc⟧ Ξ (⋆ ▸ᵈ x ⇴ y)` is `Ξ ▸ᵍ x ⇴ y` — these hold by `⟦Disc⟧`'s own defining clauses in `Gandr.Graph.Base`, not by a lemma.
  So **a generic statement instantiates at a concrete address definitionally**: the telescope form is not a second, parallel way of saying things, it is the same signature with the depth abstracted.
* That instantiation is **pinned**, so the definitional claim is exercised rather than asserted: `Gandr.Category.Reasoning.trans-assoc²` is the generic four-fold associativity applied at `⋆`, and its statement mentions no telescope at all.
* A structure can be certified at **every** dimension by a coinductive record carrying the structure at this dimension and the same record one dimension up — `Gandr.Graph.Everywhere`, inhabited by `Gandr.Setoid.Idˢ` and `Gandr.Category.ℂ.Id°`.
* The address lookup is a recursion **on the code**, so `at° 𝒞 ⋆` is the carrier-level structure on the nose: `Gandr.Graph.at°-⋆` and `at⋆-⋆` are both `refl`, and `Gandr.Category.Reasoning.Id°-↑` is the `refl` one dimension up, reading the tower at `⋆ ▸ᵈ x ⇴ y` as the tower one dimension up read at `⋆`.

That last pair is the payoff: a reasoning combinator, a law, or a lemma is written **once** against a structure at a bound telescope and holds at every dimension, instead of being restated per dimension or reached only through a spine nobody can abstract over.

**Ergonomically, what the address absorbs is the boundary ascriptions.** A law stated by hand at dimension `n` writes one type ascription per quantified variable, each a longer projection spine than the last — at dimension 2, `{a b : Ξ .ϵ°}`, then `{f g : Ξ .δ° a b .ϵ°}`, then the cells.
Binding one `Θ : Disc Ξ` replaces that whole chain, because `_▸ᵈ_⇴_` types each boundary pair from the prefix already built, so the iterated boundary requirement **resolves** the ascriptions rather than the author restating them.
The saving grows with dimension exactly as the spine does.

**A statement binds its carrier EXPLICITLY.** This is the same negative result one step on: leaving the carrier implicit under a non-trivial address puts a metavariable under a projection — `⟦Disc⟧ ?Ξ (⋆ ▸ᵈ x ⇴ y)` is `?Ξ .δ° x y` — and Agda reports it blocked, exactly as this section predicts.
With the carrier bound, every address elaborates.
At `⋆` it is still inferable, since `⟦Disc⟧ ?Ξ ⋆` reduces to `?Ξ` with no projection in the way, so the common case writes `_`.

**A structure is supplied at the bound address, and where it _has_ content is the certification's business.** A reasoning module takes the carrier, an address, and the structure there; `Gandr.Graph.At` is where a structure declares the region it is certified over, and `at⋆` / `everywhere→At` are its two ends.
There is deliberately **one** entry point: a suite parameterized by the certification rather than by the structure would fix a region in the interface, which is the one thing that varies between consumers.

### Consequences, and the two things this rules out

* **The write-side macro is unnecessary.** A reflection macro that reconstructs the elided prefix from a cell's type is the obvious alternative design, and the unifier result above is exactly what would force one.
  With the telescope bound rather than inferred, there is nothing to reconstruct.
* **No reflection, and no tactic engine.** Not "not yet" — not at all, under this record.
  Reflection here is fragile in exactly the way that costs a tree its `--safe` story and its debuggability, and the telescope removes the one motivation that made it look necessary.
  Revisiting this is a decision to be recorded, not a judgement call at a call site.
* **`DISPLAY` stays.** `Gandr.Graph` already rewrites the projection spines to the derived formers (`_϶`, `_▸ᵍ_⇴_`), which is a display concern and carries no trust weight.
  **Elided where stated, explicit where computed:** as-written cell types display with their telescopes elided to the top parallel pair; a reduced or computed type keeps the full telescope, and the elided form must stay visually distinct enough to signal that a prefix was dropped.

### The telescope-former dress

The boundary-pair formers are one descent family, tagged by structure with superscript modifier letters: `_▸ᵍ_⇴_` on the carrier, and `⋆` with `_▸ᵈ_⇴_` for disc telescopes.
A sphere telescope, when it arrives, takes `⋆` with `_▸ˢ_⇴_`; the two `⋆` bases overload across their datatypes, both being the empty telescope.
Interpretation transforms the tag — `⟦Disc⟧ (Θ ▸ᵈ x ⇴ y)` is `⟦Disc⟧ Θ ▸ᵍ x ⇴ y`, a disc step interpreting as a graph step.
Tags are lowercase throughout, since Unicode has no modifier capital S and a mixed-case set would be worse than a uniform lowercase one.

**Bare `▸` is reserved.** The tagging exists to free it, and nothing claims it without a note here.

## The representation discipline

**Familial first — the standing rule of this tree.** **Before writing a structure, ask what it is a family _over_, and index it by that.** Prefer an inductive family indexed by the data that determines its shape; reach for a record or a Σ only for what genuinely varies independently of the index.
Functions into data — `Fin n → A` and its relatives — are the last resort, not the default.

This plays to the strength of the setting rather than working around it.
Five things follow, and they compound:

* **Impossible cases stop being expressible, so nobody writes them.** `Gandr.Graph`'s coproduct is the exemplar and is worth reading before designing anything here.
  The naive encoding gives `δ°` by cases on a sum, so the mixed `inl`/`inr` pairs must be assigned `𝟘` and _every consumer at every dimension_ then discharges two cases that have no inhabitants.
  Carrying the boundary constraint in the constructors instead — `Σ⊕δ`, indexed by the pair — means the mixed homs have no constructors at all, the copairing `[_,_]` takes two clauses at each level that meets the sum, and coverage discharges the rest.
  A case you never write is a case that can never drift.
* **The cost of the naive choice scales with dimension.** A four-way split is an annoyance at dimension 0 and is `4ⁿ` at dimension `n`.
  In an ∞-graph tower that is the difference between a usable structure and an unusable one.
* **It is what makes the two witness disciplines below achievable** rather than aspirational.
  A family whose indices are constructor-headed satisfies them by construction; a table forces a projection or a `lookup` into every statement, and proofs then proceed by rewriting instead of by matching.
  Rewriting is where `--without-K` friction accumulates.
* **Equality becomes structural.** Inductive data has decidable propositional equality whenever its payloads do, by ordinary induction.
  Function-typed fields need function extensionality, which `--safe --without-K` does not have and will not be given.
  When a structure's equality is out of reach, treat that as evidence the encoding is wrong before concluding the setting is limited — the two are easy to confuse and the mistake is expensive.
* **The index is usually the interface, which is what later abstraction quantifies over.** A carrier with no index cannot instantiate an interface that has one.

Two honest limits, so the rule is applied rather than recited.
**A family can over-determine**: a term calculus for a structure may admit several derivations of one object, and when it does the redundancy is real and `Gandr.Rigid` is what reconciles a multi-derivation term calculus with a canonical stored form — `Rigid` is the effective quotient by a decidable canonicalization, and a canonical section is never pretended free.
And this is a rule about the _metatheory's_ presentation, not about gandr's storage layout, which stays flat and tabular; the section discipline is the bridge between them.

### STOP: a functional or higher-order encoding requires design input first

**If you find yourself needing — or merely inclined toward — a functional or higher-order encoding for a structure's _data_, stop and raise it with the maintainer before writing it.** This is a hard gate, not a preference to weigh against convenience.
Proceeding under a stated assumption is **not** available here: the cost of the wrong encoding is not paid in the module that chooses it, but by every consumer afterwards, and by the abstraction that later cannot be extracted over it.

**What trips the gate.** Any of these, on their own:

* a field or carrier typed `Fin n → A` — a finite table written as a function;
* a structure stored as a function where an inductive family or a `Vec` would carry the same information;
* a record whose _identity_ matters (it will be compared, stored, addressed, or canonicalized) and which has function-typed fields;
* wanting function extensionality, or reaching for a bespoke pointwise relation to stand in for an equality that "cannot" be proved;
* writing a lemma whose only job is to refute a configuration the encoding permits and the object does not have;
* catching yourself **explaining** a limitation as inherent to the setting.

That last one is the failure mode this rule exists for, and it is the one that does real damage: an encoding defect described as a property of the theory reads as settled, gets cited downstream, and stops being questioned.
`--without-K` and SETOID-not-SET are both genuine and both load-bearing — which is exactly why an encoding artifact dressed in their language survives review.
Before attributing a wall to the foundation, produce the counterexample that shows the same statement holds under an inductive encoding, or stop.

This is the encoding-layer instance of a general failure, and the general form is worth knowing because it recurs in literature sweeps and design analysis: `docs/workflow/review.md` §"Declining is a claim too — the counterfactual test" is the general statement, and its §"Refutations bind only with owner sign-off" is what binds recording one.
The shared shape is a judgement that holds our setting fixed and therefore resolves against whatever is being judged.

**What does _not_ trip it.** Functions as _operations_ are fine and pervasive: an `∞Map`'s cell action, a category's composition, a profunctor's actions, derived operations such as concatenation, and accessors over a family.
The rule is about the **encoding** — what the structure _is_ — not about its interface.
The question to ask is whether the function is standing in for data that could be carried directly.

**What to do when it trips.** Stop and surface it.
Name the structure, what it is a family over, the encoding you were about to write and the indexed alternative, and what each costs.
Do not route around a missing equality; do not weaken a statement to fit the encoding; do not record the obstruction as a located wall and continue — a wall that is really an encoding defect is worse than an open obligation, because it looks discharged.

### Decidable equality is spiked first, never deferred

**The moment a design suggests it will need decidable equality — or any propositional-equality statement about a structure's data — stop and spike it before building on the representation.** This takes priority over almost anything else in flight.
It is not a nice-to-have check: the answer _determines_ the representation, and the representation is the one thing that is expensive to retrofit.

The failure mode this prevents is specific.
A decidability question that is deferred does not sit still — work continues over an encoding whose equality theory nobody has checked, consumers accumulate against it, and by the time the question is asked the answer can no longer change anything.
A deferred decidability question is a wrong-path generator.

**The spike must produce a typechecked decision procedure, or a located failure with the exact stuck unification.** A plan for one does not count, and neither does a reasoned argument that it will work out.

**Two obstructions look identical from the symptom side, and only one of them is a representation defect.** Telling them apart is most of the value of running the spike early:

* **Function-typed fields** — a structure stores `Fin n → A` where a `Vec`/`All`/inductive family would carry the same information.
  Pointwise agreement then cannot be upgraded to propositional equality without function extensionality, which `--safe --without-K` does not have.
  **This is a representation defect**, it is what the STOP above is for, and the repair is to carry the data.
* **Forced-index deletion** — two witnesses of an inductive family are compared at _fixed_ indices, and matching the second one has to eliminate a reflexive equation such as `x = x` or `ys = ys`.
  **This is not a representation defect and not a foundation limit.** It is a gap in what pattern matching alone can do, and it has a standard discharge.

The discharge, recorded because it is otherwise re-derived each time: **concentrate the whole debt into one injectivity lemma**, rather than letting it spread across every proof that needs a comparison.
Send the witness to a **recursively computed code** built from `⊥`/`×`/`⊎`/`≡`, whose inhabitants compare without matching any index, and prove the round trip is the identity — every split is then on a single argument or on a plain list.
Where a constructor carries an argument that appears in a _later_ argument's type, or an existential implicit, route its injectivity through a **view plus a UIP-based projection** rather than through `refl`.

**The price is an h-level condition, not decidability, and the two must not be conflated.** What closes the residual reflexive equation is `UIP` on the **index type alone** — not on generators, not on the structure itself.
Decidable equality is the standard constructive _supplier_ of that set-ness, through Hedberg, and is genuinely required only where a decision is actually **computed**.
So parameterize a uniqueness or injectivity lemma by `UIP Ob`, and reserve `DecidableEquality Ob` for the decision procedures themselves; a consumer that needs only the law layer must not be made to pay for decidability.
Whichever is taken, it appears in the signature — never as a postulate, and marked at its definition site as the trust-story exception it is.

Three corollaries, each of which was got wrong before it was checked:

* **A blocked `refl` match is not evidence the statement is false.** Uniqueness of a graph-of-multiplication witness reads like it needs K and does not — the fact holds, only the pattern match fails.
  Record a wall only after the code route has been tried.
* **Refutations project, they do not match.** Once a `with` has identified one component, `no λ { refl → … }` will fail on the component already identified; discharge it with `cong` through a projection instead.
* **Relocating the obligation is not discharging it.** A view refactor, a re-indexing for constructor-headed invertibility, or carrying the equation as data will each move the K-step somewhere else without removing it.
  When the obligation is genuinely the K-step, meet it at the h-level condition rather than redesigning around it a fourth time.

### The witness discipline

Two disciplines are load-bearing rather than cosmetic.
Both are instances of the familial-first rule above, and both exist to keep structures computing under `--without-K`:

* **Witness syntax stays first-order and constructor-headed.** A defined function must never appear in a matchable index.
  Indices may carry the arity monad's **units** (`[]` and `_∷_`, and a tree kit's `leaf` when one lands); its **multiplication** — append, flatten, graft, substitution — never does, and enters instead as the inductive _graph_ of that operation, a witness relation carried in constructors.
* **No identity-shaped constructor repeats a frame variable across its result indices.** Identity and diagonal cases are derived, never adjoined as constructor shapes.

**These two are not to be re-derived locally, and a local typecheck is not evidence that an exception is safe.** A defined function in an index is stuck unification waiting to happen ("green slime"): it may check fine at the site that introduces it, because that site's own indices are still variables, and then fail at the first consumer that has to match a specific index shape.
Three shapes trip the first rule and are worth recognising by sight: a **declared diagonal** (`nil : ∀ Γ → Web Γ Γ`); a **chunked index type**, where a flattening function reaches into index position; and **singleton-chunk index expressions**.

The strongest form of the rule, and the one that cost the most to learn: **index a datatype syntactically, never by its own interpretation.** A cell complex indexed by the fold that interprets it — rather than by the syntactic spheres — breaks in two directions at once.
Case splitting degrades, because the fold sits in a matchable position; and the `--safe` size-change termination checker cannot certify a recursion whose sibling-sphere sub-terms are themselves fold applications, so the fold needs a `TERMINATING` pragma and its module loses `--safe`.
Re-indexing syntactically retires both at once and lets the interpretation descend an explicit well-founded measure instead.
Any construction tempted to index a datatype by something it computes should read that outcome as the expected one.

**Never `with` on a recursive call in a definition that will be reasoned about.** A `with` compiles to an auxiliary function the caller cannot name, so `f (c x)` becomes a term stuck on something no lemma can reach: `cong` and `rewrite` cannot see the recursive call, and every fact about `f` has to be re-established by matching at each use site instead of once.
Write the recursive clause as an **application** — the sum eliminator, a `map`, or a projection out of a record, since records have eta and their projections compute on any term at all.
The recursive call is then a visible subterm and one `cong` reaches it.

This is cheap up front and expensive to retrofit, and it does not announce itself: the `with` form type-checks, computes correctly on closed data, and only fails when the first lemma _about_ the function is attempted — which can be a session or a module later.
The tell is a definition of the shape `f (c x) with f x`, and the repair is mechanical.
`Gandr.Shape.Graph`'s `split`, `ends` and `route` and `Gandr.Shape.Graft`'s listing algebra are the worked examples; both files say why at the definitions, and `split-left`/`split-right`/`swap-follow` are what the discipline buys there.

**The same rule fires from the CONSUMER's side, and that half is easy to miss.** A proof about such a definition must not meet the definition's own scrutinee with a `with` either: the with-abstraction rewrites the goal into the definition's internal auxiliary — Agda's error names it — and any lemma stated about the definition can no longer reach the goal.
Pass the scrutinee to a helper as an **argument** together with its defining equation, and split on the argument; the goal then stays in the vocabulary the lemmas are about.
`Gandr.Shape.Graft`'s associativity proof is the worked example: one auxiliary per case the scrutinee forces, and no `with` anywhere in the proof.
The pair of halves is what makes a well-founded definition reasonable about at all — an unfolding lemma per head form on the definition's side, arguments-with-equations on the proof's.

## Organizing structures

**Before building a structure or an operation, say what it is categorically and lay the instances out — then build.** Setoid where appropriate, then the category and/or groupoid, then the monoidal structure if there is one, then the monad or relative monad if there is one; say what is functorial and what is natural.
**Define the instances.** **Naming them is not doing it.**

Two reasons, and the second is the one a long build loses sight of:

1. It makes the tree legible — one instance replaces a hundred loose lemmas.
2. **It enumerates the obligations.** An instance you cannot fill is a hole you did not know you had.
   Running this once over the tree turned up two that were on no list: associativity of the wiring composition, and of grafting.
   A `Category` instance would have refused to typecheck without them.

### Characterize at the most precise structure, and prefer the lightest coherence burden

Two demands, in this order.
**Precision:** name the finest structure the thing actually has, not the nearest familiar one.
**Coherence burden:** among characterizations that fit, **strongly prefer the one whose coherence is most manageable** — most decidable, least dependent on a strictness theorem.

Concretely: **prefer `SkewMonoidal` to `Monoidal` where it fits.** Dropping invertibility of the structural maps is not a weakening to apologize for — it is what makes coherence tractable, and it is in character for this tree, whose recurring devices are carrying a witness instead of an equation, ordering a representation as a section rather than a quotient, and localizing a choice where a global gluing property fails.
Those are all preferences for directed, non-invertible structure with a decision procedure over invertible structure with a strictness theorem.

**Characterizing something more finely than the literature does is a result, not a liberty.** Where we can show a structure is skew-monoidal, or lax where the literature says strong, or relative-monadic where it says monadic, that is a sharper statement and it should be taken.
Record what the finer characterization buys, so a later reader does not "simplify" it back.

### A presentation is a characterization, and its completeness is arity-scoped

**Where a structure is named by generators and relations, write the presentation down and prove it, exactly as for any other characterization.** A presentation is not a description of what some laws resemble; it is the claim that the relations listed **generate every relation there is**.
Naming one and listing some of its relations is the "naming them is not doing it" failure in the form that is hardest to see, because the partial list keeps working for a while.

**And it keeps working for a reason that must be checked rather than assumed: a presentation can be complete at one arity and incomplete one step up, because the missing relation has no instance at the smaller arity.** That is what makes this failure quiet — the check passes at the size it was run at, and nothing about the small case hints that a relation is absent.

**So the trigger is arity, and it is cheap.** When a consumer needs the structure at a **larger arity than the presentation was checked at**, re-derive the standard presentation for the arity now in play and compare it against what is proved, _before_ proving anything new over it.

`Gandr.Shape.Graft`'s exchange is the worked example, and it cost a ladder.
The involution and the braid relation genuinely **are** the symmetric group's presentation on a three-layer tower: three layers give two adjacent transpositions, there is no non-adjacent pair, so the commutation relation has no instance and its absence is invisible.
Four layers admit the first non-adjacent pair.
The relation was never stated, so the four-layer coherence could only be discharged as bespoke work — and each further arity was then reached by writing another fixed-arity record with its own routes and its own coherence, until the ladder was measured and found to gain a **family** rather than a rung at the fifth.
Stating the third relation retired the whole ladder at once: one structure indexed by the list of layers, three relations, and every coherence above them a word equation derived by chaining those three at any arity.

**The second tell from that episode is worth recognising on its own, because it was read backwards.** A coherence whose cases are discharged by the coherence **below** it is a **derived law, not a generator**.
That is evidence to state the presentation and stop, not evidence that the ladder is cheap to keep climbing — deriving instances one arity at a time is precisely what a presentation exists to make unnecessary.
Measuring a rung as inexpensive answers "what does this rung cost", which is the wrong question once the rung is known to be derivable.

The naming half of this rule is the terminology guard in [[#Terminology follows the ladder, and a name may not assert an unchecked correspondence]]: a presented structure's name asserts completeness, so the arity at which that was checked belongs in the module header beside the name.

### The machinery inventory is open, and demand-driven

The category-theory layer is not a fixed list to be completed once.
**When a characterization needs a structure the tree does not have, build it** — and build it against the consumer that demanded it, never speculatively.

The tree has categories, functors, natural transformations, profunctors and their (di)naturality, the Yoneda material, and the ∞-graph ambient.
Structures known to be wanted and not yet present include monoidal and skew-monoidal categories, monads and relative monads, comonads, algebras and coalgebras for a (co)monad, bialgebras and Hopf algebras, distributive laws, adjunctions, isomorphisms and groupoids, **duoidal and produoidal categories, and lax promonoidal structures (equivalently, multicategories)**, and universal properties stated inside a `Category` rather than only as ∞-graph constructions.
**That enumeration is a seed, not a boundary.** Anything else a characterization turns out to need — Kleisli and Eilenberg–Moore categories, ends and coends, enrichment, fibrations, presheaves and nerves, Reedy structure — is in scope on the same terms.

**A structure with two tensors may be lax where a single tensor would be strong, and that is a place to look before reaching for `Monoidal`.** Where two tensors are mixed into one — a sequential composition glued along a parallel one — the mixed product is typically unital on both sides but only **lax associative**, with the laxity indexed by permutations, and what it presents is a **lax promonoidal structure, i.e. a multicategory**, rather than a monoidal category.
Under the precision-and-coherence-burden rule that is the better characterization when it fits: it is finer, and its coherence obligation is an inclusion of permutations rather than an invertible associator to construct.
Reach for it when an operation has unitors that hold on the nose and an associativity that does not.

Two of those are load-bearing rather than speculative and are worth naming: **algebras for a monad**, because the objects the nerve theory is about are exactly algebras of a monad on graphical species; and **distributive laws**, because the published route to the circuit-algebra monad is an iterated one.
Both are **owed** citations in the module headers that will need them; neither header exists yet, and the Agda tree carries no bibliographic citation of either today.

### Weak by default; the marks go on strictness and decidability

**Every structure and law here reads as weak unless marked**: no `weak`/`Weak` prefixes, no E-prefixes, no "up to higher cells" call-outs.
The literature marks weakness because its ambient default is strict; ours is not, and importing that convention would decorate the normal case while leaving the exceptional case unmarked.

Conversely, **every definition or proof that is strict, or that consumes decidable equality of cells, carries a definition-site comment saying so** — `-- STRICT: <what>` / `-- DECIDABLE EQUALITY: of <what>` — because that is exactly where collapse and the K-floor live, and exactly what a reader auditing the trust story must be able to find.
`Gandr.Category`'s private `≡` module — propositional equality read as the strict category on a `Set` — is the worked example, and the `-- STRICT:` mark sits on the `Id` instance built from it.
It is **not** the only strict definition, and the reason is structural rather than incidental: a `Set`-level structure presents with `_≡_` at its 2-cells ([[#How a `Set`-level structure presents as a `Category`]]), so its laws are equations by construction and **every such instance is strict in that sense and takes the mark** — `Gandr.Shape.Structure.WIRING` is the worked one.
`Gandr.Category.Instances.SETOID` is strict for the neighbouring reason that its coherences are reflexive, and is **owed** the mark: its strictness is currently argued in its module header rather than carried at its definition site, which is the one place a trust audit looks.

**Decidability is marked in the tree's own form**: a structure that needs set-ness takes `UIP` as a signature parameter — never as a postulate, and on the index type alone — and the consumer discharges it by Hedberg from a decision procedure, so the set is bought with a computation and no K-island is opened.
Spent, declined, and deferred uses are all documented at their definition sites: the grafting unit laws spend `UIP Ob` and are discharged at `⊤`; the record-level equality of shape maps declines it, because every consumer wants the setoid relation; the linear kit's decidable-`Same` route is deferred and says so.

Names follow the highest-ranked source that owns them; a name is a claim, adopted where the correspondence is proved and marked candidate where conjectured ([[#Terminology follows the ladder, and a name may not assert an unchecked correspondence]]).

### Package layout

**Package layout is by what a thing is**: generic type theory (`Prelude`), the mathematics gandr stands on (`Foundations`), gandr's own theory (`Metatheory`); within a package, split by role (base / properties / structure / examples), with headers migrating with their content.

**Migrate, never duplicate.** When a definition belongs in a different module than the one it sits in, move it and update its importers.
Never write a second copy: two definitions of the same thing are _definitionally equal_, so the gate cannot see the drift, and the copies diverge silently the first time one is edited.
The rule is symmetric and applies across the whole tree; the split between `Gandr.Category`'s carrier-level instances and `Gandr.Category.Instances`' constructed ones is the worked example, and both headers state it.

## House style

Purpose-built records over raw sigma types; explicit record instances; record types imported at file top with projections opened at the use site; `using` listing one name per line; no `private variable` blocks; copattern style for record values; eager arrow-leading line breaks; and **every definition carries a comment**.

Six rules the one-line summary does not carry, each of which has cost something somewhere:

* **Never rename a record field into local domain jargon.** Projecting `seq₀` to a local `compose` or `⊗` hides which algebra discharges the step, and the whole point of opening the instance at the use site is that the discharging structure is one `open` away.
  Jargon worth having becomes an actual structure with its own record, not an alias.
* **Package operations; fuse a local data or record module into one external view.** `open X public hiding (module X)` is the shape, so a consumer opens one module rather than three.
* **Name strictness honestly.** A strict structure says so in its name — `FreeStrictInvolutiveWordCategory`, not `FreeCategory`.
* **The marks, not the names, carry strictness and decidability.** A name says "strict" only where a weak variant of the _same_ structure exists to be confused with it, which is why `FreeStrictInvolutiveWordCategory` is marked in its name and a `Set`-level instance is not.
  Where the marks go, and what they are owed, is [[#Weak by default; the marks go on strictness and decidability]].
* **Parallel modules keep parallel order.** Where two modules deliberately mirror each other's vocabulary — the `Set` layer against the ∞-graph layer, a `Base` against its `Properties` — corresponding definitions appear in the same order.
  The mirroring is load-bearing documentation: it lets the two be read side by side, and order drift breaks that reading.
  Reorder only when genuinely landing the missing counterparts, never speculatively.
* **Write a boundary in context style, not as a projection spine.** `Ξ ▸ᵍ a ⇴ b ▸ᵍ f ⇴ f′ ϶`, not `Ξ .δ° a b .δ° f f′ .ϵ°`.
  The formers are the projections — `_▸ᵍ_⇴_` **is** `δ°` and `_϶` **is** `ϵ°`, defined beside the ∞-graph record — so the two are the same type on the nose and nothing is paid for the readable one.
  The reason it is a rule rather than a taste: the `DISPLAY` pragmas already rewrite spines _to_ these formers, so a spine in the source means the source and every goal, error and reduced type disagree, and the reader translates by hand.
  Fixity note, since it is the one thing that bites: `_϶` is `infix 0`, the loosest in the file, so it wants to be the last token of its type or parenthesized — after an arrow and inside `(x : … ϶)` it is fine, which covers essentially every field.

**Agda-DbC stance.** The type is the contract; do not port the Rust `# Contract` comment block.
Load-bearing insight lives in the module header and the code cites it.
Mandatory marks are reserved for genuine trust-story exceptions: signature parameters standing for assumptions, strict or decidable-equality-consuming definitions, and any future with-K or unsafe island.

### Telescopes where the address is bound; spines where the address is literal

Two devices name a position in the tower and they are not interchangeable.

**A statement generic in the dimension binds `(Θ : Disc Ξ)` and reads `⟦Disc⟧ Ξ Θ`.** Nothing is inferred from a cell, so the negative result of [[#The boundary telescope]] never fires, and the telescope absorbs the per-variable boundary ascriptions.

**A structure record does not.** Its field types name _literal_ dimensions, and pushing telescopes into them is wrong on three counts, the first decisive:

* **A telescope names one address; a record's fields are multi-address relations.** Two-cell composition relates homs at `(a,b)`, `(b,c)` and `(a,c)` — three separate codes whose shared prefixes the syntax cannot factor.
* **There is no dimension to abstract.** A structure record certifies at _one_ address by design, which is exactly why the region-indexed certification layers over it rather than being baked in.
* **The ergonomic payoff does not materialize.** The quantified endpoints still have to be bound, because the field needs them at specific dimensions in specific combinations, so the telescoped field is longer rather than shorter.

**And the hazard behind the rule, which is easy to misdiagnose.** A telescope applied to a constructor tree _reduces_, so it raises no matching obligation at a use site — that is not what goes wrong.
What goes wrong is one step further: `⟦Disc⟧` is a **defined function** and therefore non-injective for unification, so the moment an address must be _recovered_ rather than _given_, it is stuck.
That is the witness discipline's own "a defined function must never appear in a matchable index", and it is why the telescope is safe in a reasoning module — there `Θ` is a bound parameter and nothing is inverted.

## Namespacing, the layer letters, and the dresses

**Namespacing is an engineering concern and is taken seriously.** The apparatus — packaging, local instance opens, one-name-per-line imports, the layer letters below — exists to optimize the ergonomics of referring to the right definition, and formalization at this scale is reached _through_ that discipline rather than despite it.
**Every new `data`, `record` or `module` performs the analysis explicitly:** what is the bare working vocabulary in each scope; which names stay qualified and under which qualifier; what stages behind an auxiliary namespace; what the instance-open reads like at the use site.
An addition that leaks awkward qualification to its use sites is a defect, not a matter of taste.

### The layer letters

One combinator vocabulary is deliberately reused at every layer — `idn`, `seq`, `inv` mean the same thing one dimension apart — so bare names collide exactly where the style wants them reused.
One letter per layer makes each use site precise:

| layer                                                              | letter | status                                                     |
| ------------------------------------------------------------------ | ------ | ---------------------------------------------------------- |
| `Set`                                                              | `𝕊`    | landed (`Gandr.Graph`, `Gandr.Category`)                   |
| ∞-graphs — the ambient                                             | `𝔾`    | landed                                                     |
| 1-categories **and their doctrines**                               | `ℂ`    | landed                                                     |
| the circuit-algebra carrier — wirings, shapes, the listing algebra | `𝕎`    | **decided, not yet applied** — lands with the package move |
| 1-groupoids                                                        | `ℾ`    | when the module materializes                               |
| ∞-groupoids                                                        | `ℾ∞`   | suffix, not subscript, in code                             |
| free / cellular-extension formers                                  | `𝔉`    | when the module materializes                               |
| virtual double categories                                          | `𝔻`    | **reserved** — nothing claims it without a note here       |

Two decisions inside that table are worth their reasons.

**`𝕎` for the circuit-algebra carrier.** The carrier already mirrors the vocabulary — `idn` and `idn-match` sit unqualified beside `𝔾.idn` and `ℂ.idn₀` — so it is a layer in the sense that matters, and it gets a letter rather than a rename.

**The doctrines live under `ℂ`.** `Monoidal`, `SkewMonoidal`, `Monad`, `Algebra`, `RelativeMonad`, `DistributiveLaw`, `Adjunction` and the rest are certifications at the 1-category layer and speak its vocabulary, so they share its letter rather than spending one each.
`ℂ` already holds `Category`, `Map` and `Nat`; the inventory grows inside it.

#### Wiring, not Feynman — the distinction, kept because it is easy to get backwards

Both words are live in the neighbouring literature and both are ambiguous; what settles it is **which way each ambiguity runs**.

| term                 | what it retrieves                                                                                                                                                                                                      | fit to this carrier                                                                                     |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| **wiring diagram**   | Spivak's _operad of wiring diagrams_ and the applied-category-theory line built on it; **circuit algebras** in the Bar-Natan–Dancso sense, which are defined by non-planar wiring diagrams; and, as noise, electronics | the second **is** this carrier; the first is a neighbouring formalism for the same idea                 |
| **Feynman category** | Kaufmann–Ward's _Feynman categories_ — a specific formalism, equivalent to groupoid-coloured operads                                                                                                                   | **a different object.** This carrier is a circuit algebra on the nonunital rung, not a Feynman category |
| **Feynman graph**    | Joyal–Kock's Feynman-graph formalism and graphical species, which the circuit-algebra source builds on                                                                                                                 | right for the **shape**, wrong for the **wiring** — it names half the layer                             |

So: wiring's ambiguity is between two formalisms of the _same_ idea, and a reader who imports the wrong one is close to right.
Feynman's ambiguity is between two _different_ objects, and a reader who imports the wrong one has been told something false — which is a failure this tree has made twice, both times on attributions of exactly this shape.

**Where Feynman is right, it is a citation and not a name.** The shape carrier does correspond to Joyal–Kock's Feynman graphs, the translation lemma between the two presentations is a **known-owed obligation**, and both belong in the shape module's header when the package move lands.

Two neighbours are worth naming so the near-misses are not rediscovered as identifications:

* Spivak's operad of wiring diagrams [@spivak-2013-operad-wiring-diagrams] has wiring diagrams that are **hierarchically nested boxes with ports**, not a matching datum — a different object under the same word, which is why the word needs the disambiguation above rather than a citation.
* Libkind and Myers' double operadic theory of systems [@libkind-myers-2025-double-operadic-systems] has **undirected** wiring diagrams that are cospans of finite sets, so arbitrary merging is allowed; this carrier's wiring is **downward** — every sink hit exactly once, no cup, and the nodeless loop inexpressible — so it sits strictly **below** the undirected operad.
  That paper's section 8 also reads diagrammatic interaction patterns as the **free** processes of a doctrine, which is a candidate characterization of the wiring layer itself and is filed as one.

The Kaufmann–Ward and Bar-Natan–Dancso attributions above remain recall-grade; verify before either reaches a citation-bearing surface.

### The variable dress

| dress         | meaning                                                                               |
| ------------- | ------------------------------------------------------------------------------------- |
| `A`, `B`      | carriers                                                                              |
| `𝒜`, `ℬ`, `𝒞` | calligraphic — the structure over a carrier (smooth / semantic)                       |
| `𝔄`           | fraktur — the free structure or cellular extension over a carrier (rigid / syntactic) |
| `𝐀`, `𝐁`      | bold — indexed families of carriers; bold means bundled                               |
| `Ξ`           | carriers in records                                                                   |
| `Θ`           | telescopes (discs)                                                                    |
| `Φ`           | spheres                                                                               |

### The `#`-dress, for staging

A module named `#X` is an auxiliary namespace whose purpose is to **free the bare name `X`** for the current scope's public definition: `#X` holds the components, and the public `X` is then defined by projection out of it.
Typically `private`; `#` is a legal Agda name character that marks auxiliary status visually, greps as a family, and can never collide with mathematical notation.

**Staging is the only sanctioned use here.** The other shape the convention allows elsewhere — repackaging a layer's or a library's same-named operations under `#L` — is not adopted: this tree's requalification modules (`Gandr.Graph`'s `𝕊`, `Gandr.Arena.Structure`'s `Fin` and `ℕ`) free no bare name, so a `#` on them would be noise, and the layer letters already cover the mirror-vocabulary case.

## Coherence-cost engineering

The four-tier policy of the metatheory track, read as mechanization practice:

1. **don't generate** — a cheap decision means the witness is never built;
2. **dissolve** — one theorem over a semantic class closed under the syntax settles a whole family at once, at a cost independent of dimension; the worked instance is the arena's rigid-coherence theorem, and the same trade appears in a neighbouring mechanization as intrinsic surface embedding — choose a representation in which the structural hierarchy has no content [@altenmuller-2026-string-diagrams];
3. **decide** — a normal form for the residue, polynomial in the word;
4. **generate** — off the trusted base only, verified by replay rather than elaboration; the measured blowup elsewhere is roughly an order of magnitude per dimension _in the typechecker_, which is the wall storage-layer sharing does not touch [@benjamin-markakis-offord-sarti-vicary-2025-naturality].

Supporting disciplines:

* **staged obligations**: acyclicity, tractability, and termination are separate named obligations with separate suppliers, never one monolithic convergence proof;
* **assumptions live in signatures**: where a piece will not close, discharge what closes and make the rest parameters of the smallest module that needs them — zero silent postulates, and a parameterized module must be instantiated at a concrete witness in the same change, or it is green and vacuous;
* **build the residual now**: three consecutive residuals taken rather than deferred each exposed a structural defect; a deferred residual is a claim that nothing depends on it;
* **refutable predicates need refuters**: an invariant is structural or refutable, never both in one type, and a predicate nothing refutes may be vacuous — the counterexample suite is part of the content;
* **computational pins**: where distinct data share a type (every wiring at a one-colour interface), predict normal forms by hand and pin them, because typechecking alone cannot catch a wrong construction;
* **the three-routes positioning**: to avoid a quotient, one can make the invariant intrinsic (cannot express non-instances — fatal where refuters are the content), go to higher structure (a HIT and a topologically sensitive ambient), or carry a decidable section — this development takes the third, the only one keeping a computational interpretation without either cost.

## Lessons with no other home

Hard-won rules that belong to no single module header (each now verified against its source in the pre-reboot consolidation corpus):

* **The lemma-list diagnostic** (the cheapest early-warning signal, checkable by reading a file's lemma list): _how many of my lemmas exist only to refute a case the encoding permits?_ A nonzero answer is the familial-first STOP firing early.
* **Check the direction you did not prove.** When writing "is" or "are exactly", check the converse; if it fails, say so and say why — a false converse is usually more informative than the theorem.
  The canonical instance: the converse of "an effective quotient by decidable canonicalization is a decidable-equality set" is **false** — decidable equality on the carrier is strictly stronger, and it is what a content-addressed store actually has (the quotient buys set-ness only at the canonical representatives).
* **The h-level charge moves, it does not vanish**: demanded function-side it lands on the unit laws, graph-side on functionality; what removes it from both is the heterogeneous structural comparison, which can ignore the witness layer propositional equality has to identify.
  The companion rule: before paying for a discipline across a whole tower, build the operation and **measure** what the discipline buys — the measurement is a theorem, not an estimate — and when an interface is about to be extracted, _attempt the extraction against every instance before building any instance's obligations_; the attempt is a review technique that has found defects reading did not.
* **The `Tower` device**: the moment a coherence's intermediate object is existential (determined by neither endpoint), carry the intermediates as **fields of a record, not indices** — stated with indices the coherence is heterogeneous and needs a transport before it can even be written; packaged, the two routes compare by an ordinary equation and the whole coherence is one congruence.
* **The statement, not the proof, is often the blocker**: a wall that resists for sessions can fall the day its statement is re-quantified — audit the statement's alphabet before escalating the proof effort.
  The verified instance (the sibling development's coherence complex): the blanket all-cells base statement provably entails uniqueness of identity proofs for arbitrary sets — underivable without K — so the letter alphabet was restricted to coherence-witnessed cells, and the base case became a theorem over one presentation-side ticket (`UIP` of the names, itself a theorem for named presentations); the refutation is kept compile-checked there.
  The companion instance, the **frame-bound impossibility**: the K-debt is an indexing artifact (objects are the family's index, generators are fiber data), so an impossibility record stated for words indexed by one object type says nothing about the same statement re-indexed by a presentation's own 0-cells — under the presentation/interpretation split the needed UIP is a theorem for syntactic presentations, and the sibling layer's single termination-checker exemption is root-caused to semantic indexing of its cell datatype, with a definitional fold over fully syntactic indices as the fix target.
  And the **observation-grade ledger** behind both: a decision-demanding hypothesis overstates the price wherever only proof-irrelevance is used — the confluence port's hypothesis is UIP, not decidable equality, because the decision procedure runs only inside the collapse on a pair already known reflexive.
* **Compare-sites are existence-grade** almost everywhere: nearly every place two constructions are compared demands only that _some_ mediating cell exists, and per-type J is the one consumer demanding a chosen one — so filler _existence_ machinery (a generator, a completion) suffices at every site but that one.
  The verified inventory (the sibling raise audit) sorts every compare-site into four classes: (I) new congruence operations — mechanical, no filler demand; (II) existence-grade filler demands — the laws' carried components, where any filler works, exactly what completion supplies; (III) chosen-computing demands — the β-class, exactly one at depth one: per-type J needs a chosen contraction datum, not a filler; (IV) instance-side tower demands — structure, not record fields: hom valued in groupoids needs the rung-2 inverse interface at the named conjugation sites, iterating one new site per fiber depth.
  The shape/witness ledger that grades a candidate arity monad's factorization witness sits beside it: compute the shapes, present the graph of the shape-level multiplication first-order, and read its naturality squares — pullbacks place the witness in the lemma layer (polynomial/familial; prop-like relations: compositions, admissible cuts), weak pullbacks in the certificate layer (analytic/species; the decomposition groupoid: partitions with block-symmetry iso-cells), anything less means specification-register only.

## The toolchain probe matrix

Thirteen typechecked probes (six rows below) pin what the flag regime admits — recorded from the pre-reboot probe suite, whose artifacts live outside this tree; none should be re-run to rediscover:

| probe                                                                       | result                                                                                                                                                                                             |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| any `postulate`; `--rewriting`; `--sized-types`; `--irrelevant-projections` | **rejected** under `--safe` — ruling out artifacts that depend on them                                                                                                                             |
| `--safe --cubical`; `--safe --without-K --prop`                             | **accepted** — so higher inductive types are blocked by gandr's own no-HIT policy, _not_ by the safety checker: a negotiable architectural choice with a recorded reason, like every other decline |
| small induction-recursion                                                   | typechecks under plain `--safe --without-K`                                                                                                                                                        |
| the Segal condition over a **semi**-simplicial object                       | typechecks — no degeneracies, no truncation, no HIT (Segal conditions mention only inert maps, which are injective)                                                                                |
| the augmented and full simplex categories                                   | typecheck with category laws by structural induction — no funext, no UIP, no termination pragma; hom counts pinned by `refl`                                                                       |
| symmetric species as carrier-plus-symmetry-relation                         | definable, but the fixpoint exists only for the raw unquotiented functor, whose identity type is that of ordered trees                                                                             |

The species probe's ceiling is a theorem, not an impression: an E-category promotes to an H-category only if uniqueness of identity proofs already holds [@palmgren-2019-equality-objects] — which is the theorem behind the three-routes positioning above: the decidable-section route is the only one that keeps a computational interpretation without a HIT or a topologically sensitive ambient.

## Reasoning and proof style

**Any multi-step equational argument is written as a reasoning chain, not as a nest of `trans`.** The vocabulary is `Relation.Binary.Reasoning.MultiSetoid`, re-exported by `Gandr.Category.Reasoning`, with `Gandr.Setoid.bundle` turning a `Setoid` into the stdlib bundle the syntax takes; that module's `Reasoning.homᵇ` produces the hom-setoid bundle from a `Category`.
`Gandr.Profunctor.Yoneda` is the worked example — `begin⟨ bundle (P .std a b) ⟩ … ≈⟨ … ⟩ … ∎`.

**This applies to the `Set`-level structures too, and that is the point.** Under the discrete-setoid presentation of [[#How a `Set`-level structure presents as a `Category`]], a hom-setoid's relation _is_ `_≡_`, so a reasoning chain there is exactly a chain of `trans` — the same proof, written in the vocabulary the rest of the tree uses.
The bundle to name is **`bundle (≡ˢ _)`**, and `Gandr.Shape.Graft`'s `cap-swap` is the worked example on that side.
Nothing about it is more expensive.

**A `Set`-level module pays `--guardedness` for this, and that is the accepted trade.** `Gandr.Setoid` sits over the coinductive ∞-graph carrier, so the flag is infective and reaches any module that reasons.
Take it rather than reaching for `≡-Reasoning`: **one vocabulary everywhere is worth more than one flag saved**, because a second style is a standing invitation to a third.
Under a role split that means `X/Properties` and `X/Structure` carry the flag as a matter of course, and only `X/Base` — which proves nothing — comes out free of it.
**Never reshape a module, move a definition, or split a proof to chase the flag.** Take the precision where it is free; it is worth nothing where it is not.

Two reasons it is a rule rather than a taste:

* **The chain names its intermediate terms.** `trans (cong f p) (trans q (cong g r))` hides what is being rewritten to what; a chain shows the sequence, and a reader can check a single step without reconstructing the whole ladder.
* **It survives the structure gaining 2-cells.** A proof written against the hom-setoid does not change when a structure later has a genuine equivalence in place of `_≡_`; a `trans` ladder is rewritten from scratch.
  Given that this tree's whole direction is to characterize structures more finely, that is not hypothetical.

Single-step arguments — one `cong`, one `refl`, one lemma applied — stay as they are; the rule is about ladders.
Existing `trans` ladders are converted when their module is next touched, and the modules under the cell shape are the standing backlog.

Two neighbouring rules complete the proof style.
Solvers are reached for before hand proofs, on demand, quoted by hand, never reflection macros — the trusted content is an object-level function with a soundness proof, and nothing in the tree quotes or unquotes syntax; the recorded direction for a future coherence solver is the tree's own normal-form machinery instantiated at the free structure it decides.
Opacity is the default for definitions whose unfolding is a cost, with the compute surface (the carrier layer, the telescope interpretation, normal-form functions) never opaque and every unfolding block naming its computation dependence.
The mechanics of both — which solver to reach for, how an `opaque unfolding` block is written — are workflow, owned by `docs/workflow/agda.md`.

### How a chain is laid out, and the four things that do not go in one

A chain is read down its **terms**; the steps are apparatus beside them, and the layout says so.
`Gandr.Shape.Graft`'s `assoc-∷-capped` is the one to copy: it works the layout rule, the reverse step, the head-marker step, and the nested-`begin` rule together.
The fourth marker and the no-`subst` rule are worked elsewhere in the same file, at the `tower-swap` chains and at `insert-shrink` respectively.

* **`begin`, every step, and `∎` hold one column; the terms are indented two further in.** `begin⟨ … ⟩` and `∎` take the statement's own indentation, each `≈` mark starts a line at that same indentation, and the terms sit between them two columns in.
  So the chain reads as a ladder — an outer column of relations with the terms nested inside it — and a step's own length never disturbs the terms:

  ```agda
  begin⟨ bundle (≡ˢ _) ⟩
    [ z ↦ match-comp z o ]· match-comp (i ∷ m) n
  ≈·⟨ match-comp-∷-capped i m n ins body p m′ eq₁ eq₄ ⟩
    match-comp (cap p (match-comp m′ body)) o
  ≈⟨ match-comp-cap p (match-comp m′ body) o ⟩
    …
  ∎
  ```

  A step whose proof does not fit continues **two in from its own marker**, and its closing `⟩` goes on a line of its own back in the step column.
  **Nothing is aligned to the right.** Mirroring the steps into a right-hand column — the `agda-categories` presentation — is declined: it is re-flowed by hand on every edit, and it degrades exactly when a step grows, which is when the reader most needs the layout to hold.
* **A reverse step is `≈⁻¹⟨ p ⟩`, never `sym p` and not stdlib's `≈⟨ p ⟨`.** Taking the proof the other way round is right — a `sym` wrapped around a multi-line proof hides the direction inside the step — but stdlib puts the mark on the **closing** bracket, which is the far end of exactly the steps that need it, and an opening and a closing angle differing only in orientation do not survive skimming.
  So the direction goes on the relation, as an inverse.
  `Gandr.Setoid.step-≈⁻¹` **is** stdlib's backward step re-syntaxed and nothing more: no combinator is reimplemented, the transitivity and the `IsRelatedTo` stay the library's, and stdlib's own form remains in scope and is not an error.
  It is a general-setoid step, so it also replaces an `invˢ` or an `inv₁` applied to a whole step — `Gandr.Profunctor.Yoneda`'s `yoneda-to` is the worked example on that side.
* **A step that rewrites under something marks the head rather than wrapping the step in `cong`.** Most `Set`-level steps are congruences, so `≈⟨ cong f p ⟩` repeats `cong` down the whole chain.
  Write the term as `[ x ↦ f ]· u` — the same term `f[x := u]`, with the rewritten position named — and the step as `≈·⟨ p ⟩`; the combinator is `Gandr.Setoid.step-≈·`.
  **The head is a binder, not a function slot**, because the rewritten position is usually not the last argument: `[ x ↦ cap q x ]·`, `[ x ↦ spot₂ ∷ x ]·` and `[ r ↦ removal-comp r o ]·` all need the hole named, and a section or a bare lambda there reads as apparatus rather than as the term.
  Agda's `syntax` binds it directly, so nothing in the term is spelled twice.
  Two further things about it are forced rather than chosen, and both are worth knowing before reaching for a variant.
  **The head and the argument must be separate slots.** A step of the shape `x ≈[ f ]⟨ p ⟩` cannot be written at all: it must build `x ≡ f v` from `cong f p : f u ≡ f v`, so it needs `x ≡ f u`, which is unavailable for an abstract `x` — Agda rejects the definition with `f _x != x`.
  Taking the head and `u` separately makes the step's own index `f u`, and there is nothing left to reconcile.
  **And the marker cannot be plain `≈⟨`**, because the argument slot swallows `u ≈⟨ p ⟩ rest` and Agda reports the ambiguity; the dot appearing in both halves of the device is the better reading anyway.
  This is `Set`-level vocabulary, because `cong` is what makes it: a general setoid wants a congruence witness rather than a function, which is the structure's own business.
* **The two markers above are independent, so there are four and not three.** A step can rewrite under something _and_ run the other way, and that one is **`≈·⁻¹⟨ p ⟩`** (`Gandr.Setoid.step-≈·⁻¹`).
  Its proof reads in the same direction as `≈·⟨ p ⟩`'s — `p : u ≡ v` — and what moves is which end carries the marked term: the term written is `f v` and the chain continues at `f u`.
  Closing the grid is not tidiness: without it a backwards congruence is written `≈⁻¹⟨ cong f p ⟩`, which is the `cong` the third bullet removes and the `sym` the second one removes, both back in the same step.
* **No `trans` inside a chain: a nested argument that needs several steps is its own `begin` block.** A step whose proof is a `trans` ladder has a chain hidden inside a chain, which is the very thing the outer chain was written to stop.
  Open a second `begin⟨ … ⟩ … ∎` in the argument position instead, parenthesized, laid out by the same rule one level in — its steps in the argument's column, its terms two further.
  `assoc-∷-fuse′` carries two of them, of four and five terms.
* **No `subst` blob inside a chain.** A transported arithmetic side-condition spread over three lines says less than the one-line lemma it stands for.
  Name the little lemma; if the pattern repeats, hoist it beside the definition it serves.
  `insert-shrink` is the worked example: the length decrease it needs is an ordinary induction on the insertion rather than a `subst`-and-`<-trans` chase over `insert-length` at five sites, which is also what keeps `match-comp-acc` short.

## Reading the literature, and the names it owns

**Reading that would change what gets built is scheduled before the building.** This is the same rule that governs the module split — moving a boundary twice is where the cost is — applied to characterizations rather than to files.
A source that tells us which record an operation fills is cheaper read than rebuilt around.

**Rank sources by what they gate, then by closeness.** Topical closeness alone is the wrong criterion: the nearest paper to a subject is often the one that settles nothing we are about to do.
The ladder, in order:

1. the source whose development **is** ours, and which names the structures we are about to define;
2. the source that carries the theorem the arc is aimed at, and gates the machinery built for it;
3. the machinery that **discharges** coherence, which is only useful once we know which coherence we owe;
4. sources that sharpen statements _about_ our object — presentations, deltas, near-misses — and gate nothing;
5. attribution and translation debts, which are documentation rather than build.

The assignment of sources to these rungs lives on the arc's tracker epic rather than here; it changes as the arc moves, and this section is the rule that produces it.

### Terminology follows the ladder, and a name may not assert an unchecked correspondence

**Where a structure we build already has a name in a higher-ranked source, use that name.** Minting a parallel vocabulary for a structure the literature has already named is how a tree becomes unreadable to everyone but its author, and it hides the fact that a result is available.

Three guards, and the third is the one that has already cost something here:

* Where two sources name the same structure, **the higher rung owns the name**.
* Where a source's structure is _more general_ than ours, take the name and **state the restriction** rather than inventing a diminutive.
* **A name is a claim.** Adopt it where the correspondence is _proved_; mark it explicitly as a candidate where it is conjectured; and never let a name assert a correspondence nobody has checked.
  The disambiguation recorded above under the circuit carrier's layer letter is the worked example of what the third guard is for: two available words, both ambiguous, and the ambiguities running in opposite directions.
  **A structure named by a presentation carries a fourth obligation**, because such a name asserts that the listed relations are _all_ of them: say **at which arity** the presentation was checked, in the module header beside the name, and re-check it when a consumer exceeds that arity.
  See [[#A presentation is a characterization, and its completeness is arity-scoped]] for why the gap is invisible from below, and for what it has already cost here.

## Flags, gates, and scope

`--safe --without-K` on every module, with `--guardedness` infective from the coinductive carrier and accepted rather than routed around; the strict root / declared-holey-leaf split; stdlib imported directly with per-module repackaging (the facade is withdrawn).
These are workflow rules owned by `docs/workflow/agda.md`; they are listed here only so this track is a complete map.

## Sub-documents

* [[proof-engineering/roadmap]] — the discipline-side backlog and owed statements.
