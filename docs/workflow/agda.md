# Workflow: the Agda metatheory

> Read when: touching `metatheory/`.
> Agda is the sole proof vehicle.
> **Standing rule, whatever the task:** before recording that something does not apply, is not needed, or cannot be done, read [review.md](review.md) §"Declining is a claim too" and §"Refutations bind only with owner sign-off" — a refutation binds only with the owner's sign-off.

## What this document is, and is not

This file owns the **workflow**: layout, flags, gates, dependency policy, commit shape, and the done-rule.
It deliberately does **not** carry doctrine — the mathematical plan lives in the Agda module headers themselves, and the lane's scope and rationale live on its tracker epic.
That split is a decision, not an omission: the reboot has no separate design document, so a header and the code beneath it can never drift apart, and there is exactly one place to read for "why is this module shaped this way".

Consequently: do not restate a theorem, a substrate decision, or a scope fence here.
Record it in the module header that owns it, and reference the epic.

## Substrate: port-as-source

The metatheory is built **port-as-source** under the `Gandr.*` namespace.
There is no internal-univalence submodule, no `metatheory/upstream/`, and no `iu:check` pin gate.

The sister internal-univalence library remains a _reference_ — its structures are read, understood, and re-derived here under gandr's own naming and its own scope.
A submodule would import that library's research frontier and its release cadence into this gate, which is the wrong coupling for a tree whose purpose is to justify gandr's design.
Every ported module records its divergences in a port-delta note so the debt stays auditable.

House policy on external research artifacts applies unchanged: read and cite, never vendor, port, or depend on a companion mechanization, regardless of license.

## Representation: familial first

**The standing rule of this tree.** **Before writing a structure, ask what it is a family _over_, and index it by that.** Prefer an inductive family indexed by the data that determines its shape; reach for a record or a Σ only for what genuinely varies independently of the index.
Functions into data — `Fin n → A` and its relatives — are the last resort, not the default.

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

This is the encoding-layer instance of a general failure, and the general form is worth knowing because it recurs in literature sweeps and design analysis: see [review.md](review.md) §"Declining is a claim too — the counterfactual test".
The shared shape is a judgement that holds our setting fixed and therefore resolves against whatever is being judged.

**What does _not_ trip it.** Functions as _operations_ are fine and pervasive: an `∞Map`'s cell action, a category's composition, a profunctor's actions, derived operations such as concatenation, and accessors over a family.
The rule is about the **encoding** — what the structure _is_ — not about its interface.
The question to ask is whether the function is standing in for data that could be carried directly.

**What to do when it trips.** Stop and surface it.
Name the structure, what it is a family over, the encoding you were about to write and the indexed alternative, and what each costs.
Do not route around a missing equality; do not weaken a statement to fit the encoding; do not record the obstruction as a located wall and continue — a wall that is really an encoding defect is worse than an open obligation, because it looks discharged.

This plays to the strength of the setting rather than working around it.
Five things follow, and they compound:

* **Impossible cases stop being expressible, so nobody writes them.** `Gandr.Graph`'s coproduct is the exemplar and is worth reading before designing anything here.
  The naive encoding gives `δ°` by cases on a sum, so the mixed `inl`/`inr` pairs must be assigned `𝟘` and _every consumer at every dimension_ then discharges two cases that have no inhabitants.
  Carrying the boundary constraint in the constructors instead — `Σ⊕δ`, indexed by the pair — means the mixed homs have no constructors at all, `[_,_]` is two clauses per level, and coverage discharges the rest.
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
A family can _over-determine_: a term calculus for a structure may admit several derivations of one object, and when it does, the redundancy is real and `Gandr.Rigid` is what reconciles it — do not pretend a canonical section is free.
And this is a rule about the _metatheory's_ presentation, not about gandr's storage layout, which stays flat and tabular; the section discipline is the bridge between them.

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
  **This is a representation defect**, it is what the rule above is for, and the repair is to carry the data.
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

## Build the residual now; parameterize what will not close

**Implement the remaining pieces and the suggested lemmas as they occur.** Recording an owed lemma at its site and moving on is no longer the default.

**Where a piece will not fully close, discharge what closes and make the rest parameters of a module** — so what is being assumed, and what is left to prove, appears in a signature rather than in a comment.
The form already exists in this tree: `Gandr.Shape.Graft`'s unit laws sit in `module _ (uipᵒ : UIP Ob) (uipˡ : UIP (List Ob))` and are discharged at `Ob = ⊤` below.
This section extends that from h-level conditions to **any** undischarged obligation.
The surrounding discipline is unchanged: zero silent postulates, an assumption appears in the signature and never as a postulate, and a mid-proof module is a declared holey leaf gated on its own line.

**Why this is a rule and not a preference.** A deferred residual is a claim that nothing depends on it.
Three residuals taken rather than deferred, in consecutive sessions on the cell shape, each found something structural: that the edge set named half-edges and every predicate above it was wrong on exactly the shapes the cut had been added to express; that the colour involution was load-bearing for the incidence rather than a legitimacy predicate; and that a cut's ports being unordered constrains which theorem about merging is _statable_.
Each was a wrong assumption that would otherwise have been built over.

**What this is not.** It is not a licence to assume a hard result and continue over it.
An assumed hypothesis must be named, stated as a parameter of the smallest module that needs it, recorded in that module's header with what it would take to discharge, and filed on the tracker.
A hypothesis nobody can say how to discharge is a design smell, not a parameter.

## Package layout

Three packages, split by what a thing **is**, not by when it was built.

* **`Gandr.Prelude.*`** — generic type theory: list positions, insertion and concatenation relations and their views, sum and product plumbing, decidability and h-level helpers.
  Nothing that knows about an algebraic structure.
  _Test:_ would this make sense in a library that had never heard of gandr?
* **`Gandr.Foundations.*`** — the mathematics gandr is built **on**, including everything that exists in the literature prior to gandr: ∞-graphs, setoids, the category-theory tower, monoidal and monadic machinery, the circuit-algebra carrier and its operations, arenas, nerves, Reedy structure.
  _Test:_ is this gandr's own contribution, or the ground it stands on?
* **`Gandr.Metatheory.*`** — gandr's own theory: the machine, the term representation and its interpretation, the CwF instance, the judgement encodings, decidability and normalization results, and the account of how the circuit-algebra machinery combines with the rest of the language.
  _Test:_ would it be wrong to attribute this to anyone but gandr?

**Within a package, split by role rather than by topic**, on the stdlib pattern:

```text
X/Base.agda         definitions, constructors, derived operations. No theorems.
X/Properties.agda   the lemmas and theorems about Base.
X/Structure.agda    the categorical instances (below). Fold into Properties when
                    small; keep apart when it carries the interface.
X/Examples.agda     worked instances, computational pins, refutations.
```

Each split module carries the part of the old header that belongs to it.
The headers are the design record; a split that leaves a header behind has lost it.
`Migrate, never duplicate` applies throughout.

## Characterize before building, at the most precise structure available

**Before building a structure or an operation, say what it is categorically and lay the instances out — then build.** Setoid where appropriate, then the category and/or groupoid, then the monoidal structure if there is one, then the monad or relative monad if there is one; say what is functorial and what is natural.
**Define the instances.** **Naming them is not doing it.**

Two reasons, and the second is the one a long build loses sight of:

1. It makes the tree legible — one instance replaces a hundred loose lemmas.
2. **It enumerates the obligations.** An instance you cannot fill is a hole you did not know you had.
   Running this once over the tree as it stood turned up two that were on no list: associativity of the wiring composition, and of grafting.
   A `Category` instance would have refused to typecheck without them.

### Characterize at the most precise structure, and prefer the lightest coherence burden

Two demands, in this order.
**Precision:** name the finest structure the thing actually has, not the nearest familiar one.
**Coherence burden:** among characterizations that fit, **strongly prefer the one whose coherence is most manageable** — most decidable, least dependent on a strictness theorem.

Concretely: **prefer `SkewMonoidal` to `Monoidal` where it fits.** Dropping invertibility of the structural maps is not a weakening to apologize for — it is what makes coherence tractable, and it is in character for this tree, whose recurring devices are carrying a witness instead of an equation, ordering a representation as a section rather than a quotient, and localizing a choice where a global gluing property fails.
Those are all preferences for directed, non-invertible structure with a decision procedure over invertible structure with a strictness theorem.

**Characterizing something more finely than the literature does is a result, not a liberty.** Where we can show a structure is skew-monoidal, or lax where the literature says strong, or relative-monadic where it says monadic, that is a sharper statement and it should be taken.
Record what the finer characterization buys, so a later reader does not "simplify" it back.

### The machinery inventory is open, and demand-driven

The category-theory layer is not a fixed list to be completed once.
**When a characterization needs a structure the tree does not have, build it** — and build it against the consumer that demanded it, never speculatively.

At the time of writing the tree has categories, functors, natural transformations, profunctors and their (di)naturality, the Yoneda material, and the ∞-graph ambient.
Structures known to be wanted and not yet present include monoidal and skew-monoidal categories, monads and relative monads, comonads, algebras and coalgebras for a (co)monad, bialgebras and Hopf algebras, distributive laws, adjunctions, isomorphisms and groupoids, **duoidal and produoidal categories, and lax promonoidal structures (equivalently, multicategories)**, and universal properties stated inside a `Category` rather than only as ∞-graph constructions.
**That enumeration is a seed, not a boundary.** Anything else a characterization turns out to need — Kleisli and Eilenberg–Moore categories, ends and coends, enrichment, fibrations, presheaves and nerves, Reedy structure — is in scope on the same terms.

**A structure with two tensors may be lax where a single tensor would be strong, and that is a place to look before reaching for `Monoidal`.** Where two tensors are mixed into one — a sequential composition glued along a parallel one — the mixed product is typically unital on both sides but only **lax associative**, with the laxity indexed by permutations, and what it presents is a **lax promonoidal structure, i.e. a multicategory**, rather than a monoidal category.
Under the precision-and-coherence-burden rule that is the better characterization when it fits: it is finer, and its coherence obligation is an inclusion of permutations rather than an invertible associator to construct.
Reach for it when an operation has unitors that hold on the nose and an associativity that does not.

Two of those are load-bearing rather than speculative and are worth naming: **algebras for a monad**, because the objects the nerve theory is about are exactly algebras of a monad on graphical species; and **distributive laws**, because the published route to the circuit-algebra monad is an iterated one.
Both are cited in the module headers that will need them.

### How a `Set`-level structure presents as a `Category`

`Category` is a structure over an `∞Graph`, so 2-cells are where its laws live; most structures in this tree are `Set`-level, with propositional `_≡_` for equality.
The bridge is the **discrete setoid on the identity type**, one dimension up from `Gandr.Graph`'s `disc`:

* 0-cells — the objects;
* the hom at `(x, y)` — an ∞-graph whose 0-cells are the morphisms and whose **1-cells are `f ≡ g`**, so the setoid relation _is_ the identity type and `Category.homˢ` is the identity setoid;
* **above that, `𝟘`.**

That ∞-graph is **`Gandr.Graph.𝔾.≡°`**, and the `Setoid` on it is **`Gandr.Setoid.≡ˢ`**; both carry the reasoning below.
`Category`'s fields all land at or below that level — `mon-λ`, `mon-ρ`, `mon-α` and `seq↕` are `≡`-cells — which is the record being _lawless at its last dimension_: it states the laws and imposes no coherence among them.

**Empty above, never trivial, and this is not a truncation.** Three things must be kept apart:

* **`𝟘` above the last dimension that carries content** — the correct choice.
  It says there are no cells there.
  It asserts nothing and discharges nothing, and if a structure later turns out to have genuine higher cells the dimension opens up.
* **`𝟙` above** — **forbidden by default.** A terminal hom makes every coherence hold automatically, silently discharging obligations nobody checked.
  That is the failure mode, and it is what "do not truncate prematurely" is about.
  Use it only with a stated reason — `Setoids`' homotopies are the one place in the tree that has one.
  A convention that fills trivial higher dimensions with `𝟙` as a matter of course is **not** this tree's convention, and a reader arriving with that habit should read this bullet as the correction.
* **Forcing `UIP`** — **out of scope entirely.** The `--without-K` mandate is binding and neither UIP nor definitional proof-irrelevance may enter through any shortcut.
  Note that using `_≡_` as a structure's 1-cells carries no UIP claim: nothing above it is asserted, so no two proofs of `f ≡ g` are ever identified.
  Where a specific result genuinely needs set-ness, it takes `UIP Ob` as a **parameter**, as the grafting unit laws do.

A fourth choice exists and is also wrong here: `Gandr.Graph.𝔾.Id` continues with the identity type at **every** dimension.
It is honest — nothing is truncated — but it offers a whole tower no consumer asks for, so `≡°` is `Id` stopped at dimension 1 by `𝟘` rather than by `𝟙`.

The same pattern repeats for the discrete category, the discrete groupoid, and the rest.

### A structure that stops declares a region; it is not extended to reach one

Truncating with `𝟘` has a consequence that must be met head-on rather than routed around: a **dimension-wise certification cannot hold** of such a carrier, because certifying at every address demands cells at every address and there are none above the content.
The tempting repair is to extend the carrier upward until the certification holds.
**Do not.** Both ways of doing it were built and checked, and both cost more than they pay:

* **with `𝟙`** — the certification above the content becomes a _vacuous_ structure whose laws are discharged by `tt`, and the whole reasoning suite is then available at those addresses returning `tt`, with nothing at the use site distinguishing an informative application from an empty one.
  That is precisely the silent discharge the `𝟙` bullet above forbids, re-entering through the door that uniformity opens.
* **with `Id`** — honest, and it does hold, but it leaves _"the identity tower is the intended content"_ and _"the tower is filler for a region nobody observes"_ indistinguishable in the type.

**The region is the parameter instead.** `Gandr.Graph.At 𝒮 Ξ P` certifies `𝒮` at exactly the addresses admitted by `P`; `Everywhere` is the total region (`Total`), a structure whose content stops at the carrier is the singleton region (`Only⋆`, supplied by `at⋆`), and bounded depth sits between.
So `≡°` keeps its `𝟘`, nothing is extended, nothing is marked, and where a structure has content is **stated in its signature** rather than discovered by whoever next needs it.

**This gates rather than labels, and the address code is what does it.** `Disc` has injective constructors, so an out-of-region address is discharged by **constructor disjointness** — `at⋆`'s second clause is `()`.
Reasoning above a declared region is therefore not merely unwise and not merely unmarked: the region witness has no inhabitant, so the suite cannot be formed there.
That is a second dividend from reifying the address — the code was introduced so a statement could _bind_ its dimension, and it also lets a statement _refuse_ one.

**The region is per doctrine, not per carrier, and the two must not be conflated.** `Setoid` has content at dimensions 0–1 and `Category` at 0–2, so one carrier can serve the first and fail the second: over `≡°`, `Setoid` is inhabited (`≡ˢ`) while `Category` is not (`ℂ.≡°-not-category`).
A `Set`-level _category_ accordingly presents with `≡°` on each **hom** — `δ° x y = ≡° (H x y)` — and that carrier's region is `Only⋆`.
Reading a refutation of the certification as evidence that the certification is _stronger than the structure_ is an error this tree made and shipped; check the bare structure first.

### Equational proofs use setoid reasoning, everywhere

**Any multi-step equational argument is written as a reasoning chain, not as a nest of `trans`.** The vocabulary is `Relation.Binary.Reasoning.MultiSetoid`, re-exported by `Gandr.Category.Reasoning`, with `Gandr.Setoid.bundle` turning a `Setoid` into the stdlib bundle the syntax takes; `Gandr.Category.Reasoning.homᵇ` produces the hom-setoid bundle from a `Category`.
`Gandr.Profunctor.Yoneda` is the worked example — `begin⟨ bundle (P .std a b) ⟩ … ≈⟨ … ⟩ … ∎`.

**This applies to the `Set`-level structures too, and that is the point.** Under the discrete-setoid presentation above, a hom-setoid's relation _is_ `_≡_`, so a reasoning chain there is exactly a chain of `trans` — the same proof, written in the vocabulary the rest of the tree uses.
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

## House style

Purpose-built records over raw sigma types; explicit record instances; record types imported at file top with projections opened at the use site; `using` listing one name per line; no `private variable` blocks; copattern style for record values; eager arrow-leading line breaks; and **every definition carries a comment**.

Five rules the one-line summary does not carry, each of which has cost something somewhere:

* **Never rename a record field into local domain jargon.** Projecting `seq₀` to a local `compose` or `⊗` hides which algebra discharges the step, and the whole point of opening the instance at the use site is that the discharging structure is one `open` away.
  Jargon worth having becomes an actual structure with its own record, not an alias.
* **Package operations; fuse a local data or record module into one external view.** `open X public hiding (module X)` is the shape, so a consumer opens one module rather than three.
* **Name strictness honestly.** A strict structure says so in its name — `FreeStrictInvolutiveWordCategory`, not `FreeCategory`.
  See the marking rule below.
* **Weak by default; the marks go on strictness and decidability.** Every structure and law here reads as weak unless marked: no `weak`/`Weak` prefixes, no E-prefixes, no "up to higher cells" call-outs.
  The literature marks weakness because its ambient default is strict; ours is not, and importing that convention would decorate the normal case while leaving the exceptional case unmarked.
  Conversely, **every definition or proof that is strict, or that consumes decidable equality of cells, carries a definition-site comment saying so** — `-- STRICT: <what>` / `-- DECIDABLE EQUALITY: of <what>` — because that is exactly where collapse and the K-floor live, and exactly what a reader auditing the trust story must be able to find.
  `Gandr.Category`'s private `≡` module — propositional equality read as the strict category on a `Set` — is the tree's worked example and the only strict instance in it.
* **Parallel modules keep parallel order.** Where two modules deliberately mirror each other's vocabulary — the `Set` layer against the ∞-graph layer, a `Base` against its `Properties` — corresponding definitions appear in the same order.
  The mirroring is load-bearing documentation: it lets the two be read side by side, and order drift breaks that reading.
  Reorder only when genuinely landing the missing counterparts, never speculatively.
* **Write a boundary in context style, not as a projection spine.** `Ξ ▸ᵍ a ⇴ b ▸ᵍ f ⇴ f′ ϶`, not `Ξ .δ° a b .δ° f f′ .ϵ°`.
  The formers are the projections — `_▸ᵍ_⇴_` **is** `δ°` and `_϶` **is** `ϵ°`, defined beside the ∞-graph record — so the two are the same type on the nose and nothing is paid for the readable one.
  The reason it is a rule rather than a taste: the `DISPLAY` pragmas already rewrite spines _to_ these formers, so a spine in the source means the source and every goal, error and reduced type disagree, and the reader translates by hand.
  Fixity note, since it is the one thing that bites: `_϶` is `infix 0`, the loosest in the file, so it wants to be the last token of its type or parenthesized — after an arrow and inside `(x : … ϶)` it is fine, which covers essentially every field.

### Telescopes where the address is bound; spines where the address is literal

Two devices name a position in the tower and they are not interchangeable.

**A statement generic in the dimension binds `(Θ : Disc Ξ)` and reads `⟦Disc⟧ Ξ Θ`.** Nothing is inferred from a cell, so the negative result above never fires, and the telescope absorbs the per-variable boundary ascriptions.

**A structure record does not.** Its field types name _literal_ dimensions, and pushing telescopes into them is wrong on three counts, the first decisive:

* **A telescope names one address; a record's fields are multi-address relations.** Two-cell composition relates homs at `(a,b)`, `(b,c)` and `(a,c)` — three separate codes whose shared prefixes the syntax cannot factor.
* **There is no dimension to abstract.** A structure record certifies at _one_ address by design, which is exactly why the region-indexed certification layers over it rather than being baked in.
* **The ergonomic payoff does not materialize.** The quantified endpoints still have to be bound, because the field needs them at specific dimensions in specific combinations, so the telescoped field is longer rather than shorter.

**And the hazard behind the rule, which is easy to misdiagnose.** A telescope applied to a constructor tree _reduces_, so it raises no matching obligation at a use site — that is not what goes wrong.
What goes wrong is one step further: `⟦Disc⟧` is a **defined function** and therefore non-injective for unification, so the moment an address must be _recovered_ rather than _given_, it is stuck.
That is this document's own "a defined function must never appear in a matchable index", and it is why the telescope is safe in a reasoning module — there `Θ` is a bound parameter and nothing is inverted.

Two disciplines are load-bearing rather than cosmetic.
Both are instances of the representation rule above, and both exist to keep structures computing under `--without-K`:

* **Witness syntax stays first-order and constructor-headed.** A defined function must never appear in a matchable index.
  Indices may carry the arity monad's **units** (`[]`, `_∷_`, `leaf`); its **multiplication** — append, flatten, graft, substitution — never does, and enters instead as the inductive _graph_ of that operation, a witness relation.
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
`Gandr.Shape.Graph`'s `split`, `origin` and `dest` and `Gandr.Shape.Graft`'s listing algebra are the worked examples; both headers say why.

**Migrate, never duplicate.** When a definition belongs in a different module than the one it sits in, move it and update its importers.
Never write a second copy: two definitions of the same thing are _definitionally equal_, so the gate cannot see the drift, and the copies diverge silently the first time one is edited.
The rule is symmetric and applies across the whole tree; the split between `Gandr.Category`'s carrier-level instances and `Gandr.Category.Instances`' constructed ones is the worked example, and both headers state it.

**Agda-DbC stance.** The type is the contract; do not port the Rust `# Contract` comment block.
Load-bearing insight lives in the module header and the code cites it.
Mandatory marks are reserved for genuine trust-story exceptions: signature parameters standing for assumptions, strict or decidable-equality-consuming definitions, and any future with-K or unsafe island.

## Reading the literature is part of the build, and it is ordered

**Reading that would change what gets built is scheduled before the building.** This is the same rule that governs the module split — moving a boundary twice is where the cost is — applied to characterizations rather than to files.
A source that tells us which record an operation fills is cheaper read than rebuilt around.

**Rank sources by what they gate, then by closeness.** Topical closeness alone is the wrong criterion: the nearest paper to a subject is often the one that settles nothing we are about to do.
The ladder, in order:

1. the source whose development **is** ours, and which names the structures we are about to define;
2. the source that carries the theorem the arc is aimed at, and gates the machinery built for it;
3. the machinery that **discharges** coherence, which is only useful once we know which coherence we owe;
4. sources that sharpen statements _about_ our object — presentations, deltas, near-misses — and gate nothing;
5. attribution and translation debts, which are documentation rather than build.

The arc's current assignment of sources to these rungs lives on its tracker epic, not here; it changes as the arc moves, and this section is the rule that produces it.

### Terminology follows the ladder, and a name may not assert an unchecked correspondence

**Where a structure we build already has a name in a higher-ranked source, use that name.** Minting a parallel vocabulary for a structure the literature has already named is how a tree becomes unreadable to everyone but its author, and it hides the fact that a result is available.

Three guards, and the third is the one that has already cost something here:

* Where two sources name the same structure, **the higher rung owns the name**.
* Where a source's structure is _more general_ than ours, take the name and **state the restriction** rather than inventing a diminutive.
* **A name is a claim.** Adopt it where the correspondence is _proved_; mark it explicitly as a candidate where it is conjectured; and never let a name assert a correspondence nobody has checked.
  The disambiguation recorded below under the circuit carrier's layer letter is the worked example of what the third guard is for: two available words, both ambiguous, and the ambiguities running in opposite directions.

## Namespacing, the layer letters, and the structure dress

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

#### Wiring, not Feynman — the distinction, kept because it is easy to get backwards

Both words are live in the neighbouring literature and both are ambiguous; what settles it is **which way each ambiguity runs**.

| term                 | what it retrieves                                                                                                                                                                                                      | fit to this carrier                                                                                     |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| **wiring diagram**   | Spivak's _operad of wiring diagrams_ and the applied-category-theory line built on it; **circuit algebras** in the Bar-Natan–Dancso sense, which are defined by non-planar wiring diagrams; and, as noise, electronics | the second **is** this carrier; the first is a neighbouring formalism for the same idea                 |
| **Feynman category** | Kaufmann–Ward's _Feynman categories_ — a specific formalism, equivalent to groupoid-coloured operads                                                                                                                   | **a different object.** This carrier is a circuit algebra on the nonunital rung, not a Feynman category |
| **Feynman graph**    | Joyal–Kock's Feynman-graph formalism and graphical species, which the circuit-algebra source builds on                                                                                                                 | right for the **shape**, wrong for the **wiring** — it names half the layer                             |

So: wiring's ambiguity is between two formalisms of the _same_ idea, and a reader who imports the wrong one is close to right.
Feynman's ambiguity is between two _different_ objects, and a reader who imports the wrong one has been told something false — which is the failure mode this design record has already been corrected for twice, both times on attributions of exactly this shape.

**Where Feynman is right, it is a citation and not a name.** The shape carrier does correspond to Joyal–Kock's Feynman graphs, the translation lemma between the two presentations is a **known-owed obligation**, and both belong in the shape module's header when the package move lands.

Two neighbours are worth naming so the near-misses are not rediscovered as identifications:

* D. I. Spivak (2013), "The operad of wiring diagrams: formalizing a graphical language for databases, recursion, and plug-and-play circuits", arXiv:1305.0297.
  Its wiring diagrams are **hierarchically nested boxes with ports**, not a matching datum — a different object under the same word, which is why the word needs the disambiguation above rather than a citation.
* S. Libkind and D. J. Myers (2025), "Towards a double operadic theory of systems", arXiv:2505.18329.
  Its **undirected** wiring diagrams are cospans of finite sets, so arbitrary merging is allowed; this carrier's wiring is **downward** — every sink hit exactly once, no cup, and the nodeless loop inexpressible — so it sits strictly **below** the undirected operad.
  That paper's §8 also reads diagrammatic interaction patterns as the **free** processes of a doctrine, which is a candidate characterization of the wiring layer itself and is filed as one.

The Kaufmann–Ward and Bar-Natan–Dancso attributions above remain recall-grade; verify before either reaches a citation-bearing surface.

**The doctrines live under `ℂ`.** `Monoidal`, `SkewMonoidal`, `Monad`, `Algebra`, `RelativeMonad`, `DistributiveLaw`, `Adjunction` and the rest are certifications at the 1-category layer and speak its vocabulary, so they share its letter rather than spending one each.
`ℂ` already holds `Category`, `Map` and `Nat`; the inventory grows inside it.

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

## The boundary telescope

A cell's type is a spine of projections — `Ξ .δ° a b .δ° f g .ϵ°` — and it grows with dimension.
**Implicit arguments can never elide it, and this is a theorem about the unifier rather than a limitation to work around.** Reconstructing the prefix from a cell's type poses a constraint of the form `ϵ° ?Ξ ≈ ϵ° Ξ₀` — a metavariable under a projection — which Agda solves only by eta-expanding the metavariable, and **coinductive records have no eta**.
Anything that tries to infer a carrier or a boundary from a cell will get stuck, every time, and no amount of restructuring the record changes that.

The route around it is not inference, and it is not a macro.
It is to **bind the telescope explicitly, as a first-order inductive code**.

### The code, and why it is a code

`Gandr.Graph`'s `Base` module carries the reified telescope: `Disc` — `⋆` for the carrier itself, `Θ ▸ᵈ x ⇴ y` to descend one parallel pair — together with `⟦Disc⟧`, which interprets a telescope as the ∞-graph it lands in.
`Disc` is **inductive with injective constructors**, which is what the projection spine is not: it can be matched on, recursed over, and discriminated.

This is one instance of a sort discipline worth stating once, because the known failure modes of higher-dimensional formalization are violations of it:

| the piece needs to be…                   | render it as…                                               | here                                                                                        |
| ---------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| discriminated, matched, or recursed over | a **code** — inductive, injective constructors, first-order | `Disc`, the witness relations, the arity kits' graphs                                       |
| witnessed or transported along           | a **certificate** — a cell, proof-relevant, carried         | 2-cells, the law witnesses, `Append` and friends where they are carried rather than matched |
| neither                                  | **unobserved**                                              | object equality; record identity                                                            |

Forcing a certificate into a code is what UIP and premature truncation are; leaving no code column is what makes an instance unwritable, because unification has nothing to grip.

### What it buys, verified

A statement generic in the dimension **binds one `Θ` as a parameter** and is written over `⟦Disc⟧ Ξ Θ`.
Nothing is inferred from a cell, so the negative result above never fires.
Four properties were checked directly rather than assumed:

* `⟦Disc⟧ Ξ ⋆` is `Ξ` — by `refl`;
* `⟦Disc⟧ Ξ (⋆ ▸ᵈ x ⇴ y)` is `Ξ ▸ᵍ x ⇴ y`, and the two-step case likewise — by `refl`.
  So **a generic statement instantiates at a concrete address definitionally**: the telescope form is not a second, parallel way of saying things, it is the same signature with the depth abstracted.
* A structure can be certified at **every** dimension by a coinductive record carrying the structure at this dimension and the same record one dimension up;
* and the address lookup is a recursion **on the code**, so `at 𝒞 ⋆` is the carrier-level structure on the nose.

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

## Flags and the gate

* Per-file `OPTIONS`: `--safe --without-K --hidden-argument-puns` on every module under `metatheory/src`, enforced by the Rust `source_policy` sweep (`options-policy` subcommand; exemptions are enumerated per flag with a justification).
  The without-K mandate is binding: neither UIP nor definitional proof-irrelevance may enter through any shortcut.
* `--guardedness` is need-based and **infective**: any module that transitively imports a coinductive carrier must carry it.
  Reasoning is such a need — `Gandr.Setoid` is over the ∞-graph carrier — and a `Set`-level module takes the flag rather than reason in a second vocabulary.
  A module carrying the flag for that reason alone says so at the top of the file.
  Being flag-free is a property of a module that only _defines_; it is never a reason to reshape one.
* **Strict root / holey leaf.** `Gandr.Everything` is the strict root — everything it imports is `--safe` and green.
  Mid-proof work lives in a _declared holey leaf_: a module the root does not import, checked on its own gate line with `--expected-code UnsolvedInteractionMetas`.
  Zero silent postulates, ever.
  Add a leaf's gate line in the same change as the leaf; a line ahead of its module is a gate that cannot fail.
* `mise run agda:check` = the strict root through aifix plus the OPTIONS-policy sweep.

## Dependencies

Adding any Agda library or tool requires maintainer sign-off **first** — deliberately stricter than the Rust and TypeScript trees.

`agda-stdlib` is **admitted** (pinned v2.4, verified under Agda 2.8.0) and is imported **directly**.

An earlier revision of this file required a house facade under `Gandr.Prelude.*` and forbade direct imports.
That facade is **withdrawn**: maintaining a parallel vocabulary over a library this tree wants to lean on heavily cost more than the foundation-swap freedom it bought, and the swap it insured against is not on the arc.
What replaces it is per-module repackaging — a `private module` that re-exports the stdlib names a module actually uses, under the names that module wants (`Gandr.Arena.Structure`'s `module Fin` / `module ℕ`, `Gandr.Graph`'s `module 𝕊`).
That keeps the vocabulary local and legible at each use site without a tree-wide surface to maintain.

**The `Gandr.Prelude.*` namespace is reinstated (2026-07-28), with a different meaning, and the facade stays withdrawn.** What is withdrawn is a **facade over `agda-stdlib`**: a parallel vocabulary that re-exports stdlib names and a ban on importing stdlib directly.
Both stay withdrawn — direct stdlib imports remain mandatory, and per-module repackaging remains the way to localize vocabulary.
What `Gandr.Prelude.*` now means is a home for **gandr's own generic definitions**: list positions and their views, insertion and concatenation relations, sum and product plumbing, decidability and h-level helpers — anything that would make sense in a library that had never heard of gandr.
Today these sit wherever they were first needed, which is why the namespace is wanted; see the package layout above.
The test is ownership, not provenance: if we defined it and it knows nothing about an algebraic structure, it is prelude; if stdlib defines it, import it directly.

`agda:deps` vendors stdlib into the gitignored `metatheory/vendor/`, so `agda:check` passes `-i metatheory/vendor/agda-stdlib/src` and **a fresh checkout must run `agda:deps` before its first `agda:check`**.
It stays a separate task rather than a gate dependency so a warm tree does not re-enter the fetch path on every run.

## Solvers

Proofs reach for a solver before they are written by hand, and the reach is **on demand** — a solver is a prerequisite of the first proof that wants it, never a speculative port.

1. **Use the stdlib solver if one fits.** `Gandr.Arena.Offset` against `Data.Nat.Solver` is the exemplar.
2. **If none fits, build it first**, packaged exactly as stdlib packages its own (`Algebra.Solver.Monoid`'s `Expression` / `Normal` / `Solver` / facade split), so local and provided solvers present one interface.
3. **Until then, leave the obligation by hand with a code note naming the solver that should discharge it.** An unmarked hand proof of solver-shaped work is the drift this rule exists to prevent.

Goals are **quoted by hand** into the solver's expression syntax, as `Gandr.Arena.Offset` does.
Reflection-based tactic macros (`Tactic.RingSolver`, `Tactic.MonoidSolver`) are declined as too brittle; proof-by-reflection solvers built on `Relation.Binary.Reflection` are not macros and are the intended target.
This is the same line the telescope section draws: **the trusted content is an object-level function with a soundness proof, never a metaprogram**, and nothing in this tree quotes or unquotes syntax.

The direction of record for a future coherence solver, so it is not re-derived: its kernel should be **this tree's own machinery instantiated at the free structure it decides** — the normal-form function as the normalizer, the rewrite path as the emitted coherence cell — so the solver is the machinery's first consumer and a demonstrator that it computes, rather than a bespoke normalizer bolted on beside it.
No solver lands before a proof demands it.

## Opacity

`opaque` is the default for a definition whose unfolding is a cost to be controlled, with `unfolding` naming each consumer that needs it — `Gandr.Arena.Offset`'s `⊗-ix` family is the exemplar.

The placement policy, stated as three classes:

* **Never opaque — the compute surface.** Definitions whose definitional computation _is_ the design: the carrier layer, `⟦Disc⟧` and its disappearing-boundary behaviour, and any future normal-form function.
  `Gandr.Graph`'s definitions exist to be unfolded — every consumer meets them through copattern matching on `ϵ°`/`δ°`, and sealing them would sever the definitional equalities the whole tower is built from.
* **Opaque by default — derived reasoning and law surfaces off the compute path.** Combinator kits and law witnesses assembled over a primitive eliminator or over other combinators, where a use site should consume the type rather than the reduction behaviour.
* **Opaque as unfolding control**, where a deep coinductive tower makes normalization a performance concern.

**Every `opaque unfolding` block names its computation dependence** — a one-line comment saying _why_ reduction is needed at that site.
Blanket unfolding, whether whole-module or an unfocused name list, is a defect.
A module that opts out of the default states why in its header.

One dividend worth knowing, because it looks like a coincidence otherwise: an opaque definition is a rigid head the elaborator unifies spine-wise, the same way a record field is, so sealing a derived combinator can make previously-pinned implicits inferable.

## The done-rule

A metatheory milestone is done only when `agda:check` is green **and** its documentation face lands in the same motion — the module header for new structure, or the port-delta note for a ported layer.
Gate-green alone is half a milestone.

**At every session close, sweep the durable faces in this order and update each one that the session's work moved.** Not only the artifact you happened to touch: a claim left standing in a document nobody re-read is how a retracted diagnosis gets cited downstream, which is the failure this tree has already paid for twice.

1. **Module headers** — the design record for the structure they own; new content, retracted claims, and located walls all live here first.
2. **This file** — a rule that recurred, a gate change, a dependency or commit-shape change.
3. **`docs/gandr/spec/proposal-metatheory-consolidated.md`** — the prose design record: §2.4's substrate table, the section owning the changed layer, and §15's build-order status lines.
   Say explicitly when no decision or commitment moved.
4. **The arc's own work list and decision log**, wherever the project's contributor notes keep them — next steps, status, and the lessons whose authoritative home is nonetheless item 2 or 3.
5. **The tracker** — progress as a comment on the owning item, then push.

Check the _whole_ of each artifact, not the section you edited last: status tables, build-order rows, and "as built" amendments go stale silently, and a superseded sketch must be marked superseded rather than deleted, so the amendment above it stays legible.

A green gate is also not proof of _meaning_.
State residuals honestly in the module header: a theorem that is reduced but not discharged says so, and a scope cut says what it cut and why it does not weaken the result.

Two ways a module passes the gate while proving nothing, both of which the author must close rather than the gate:

* **A parameterized module carrying assumptions must be instantiated somewhere.** Agda type-checks a module body whether or not its parameters can ever be supplied, so a module whose hypotheses are jointly unsatisfiable is green and vacuous.
  Discharge the parameters at a concrete witness in the same change, and say in the header that the witness is what makes the assumptions satisfiable.
  `Gandr.Rigid`'s `Multiset` against the natural numbers is the exemplar.
* **A predicate that nothing refutes may be vacuous.** A structure defined over a predicate proves nothing if no object fails it.
  Exhibit a counterexample alongside the examples — `Gandr.Shape.Graph`'s diamond and wheel are what stop its connectivity and wheel-freeness lemmas from being statements about an empty type.

## Commits

Keep `metatheory/**` in a **separate commit** from the Rust it mirrors — it is a distinct artifact whose history may be reorganized independently.
Repository plumbing (mise tasks, gates, this document) rides with whichever side it serves.

Commit messages follow the repository convention; `.commitlintrc.mts` is authoritative, including the canonical agent-trailer registry.
