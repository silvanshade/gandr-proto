# Workflow: the Agda metatheory

> Read when: touching `metatheory/`.
> Agda is the sole proof vehicle.

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

## House style

Purpose-built records over raw sigma types; explicit record instances; record types imported at file top with projections opened at the use site; `hiding`/`using` listing one name per line; no `private variable` blocks; copattern style for record values; eager arrow-leading line breaks; the flat proof-term ladder rather than deep `where` nesting; and **every definition carries a comment**.

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
Mandatory marks are reserved for genuine trust-story exceptions: signature parameters standing for assumptions, and any future with-K or unsafe island.

## Flags and the gate

* Per-file `OPTIONS`: `--safe --without-K --hidden-argument-puns` on every module under `metatheory/src`, enforced by the Rust `source_policy` sweep (`options-policy` subcommand; exemptions are enumerated per flag with a justification).
  The without-K mandate is binding: neither UIP nor definitional proof-irrelevance may enter through any shortcut.
* `--guardedness` is need-based and **infective**: any module that transitively imports a coinductive carrier must carry it.
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

`agda:deps` vendors stdlib into the gitignored `metatheory/vendor/`, so `agda:check` passes `-i metatheory/vendor/agda-stdlib/src` and **a fresh checkout must run `agda:deps` before its first `agda:check`**.
It stays a separate task rather than a gate dependency so a warm tree does not re-enter the fetch path on every run.

## Solvers

Proofs reach for a solver before they are written by hand, and the reach is **on demand** — a solver is a prerequisite of the first proof that wants it, never a speculative port.

1. **Use the stdlib solver if one fits.** `Gandr.Arena.Offset` against `Data.Nat.Solver` is the exemplar.
2. **If none fits, build it first**, packaged exactly as stdlib packages its own (`Algebra.Solver.Monoid`'s `Expression` / `Normal` / `Solver` / facade split), so local and provided solvers present one interface.
3. **Until then, leave the obligation by hand with a code note naming the solver that should discharge it.** An unmarked hand proof of solver-shaped work is the drift this rule exists to prevent.

Goals are **quoted by hand** into the solver's expression syntax, as `Gandr.Arena.Offset` does.
Reflection-based tactic macros (`Tactic.RingSolver`, `Tactic.MonoidSolver`) are declined as too brittle; proof-by-reflection solvers built on `Relation.Binary.Reflection` are not macros and are the intended target.

## Opacity

`opaque` is the default for a definition whose unfolding is a cost to be controlled, with `unfolding` naming each consumer that needs it — `Gandr.Arena.Offset`'s `⊗-ix` family is the exemplar.

The deliberate exception is the carrier layer.
`Gandr.Graph`'s definitions exist to be unfolded: every consumer meets them through copattern matching on `ϵ°`/`δ°`, and sealing them would sever the definitional equalities the whole tower is built from.
A module that opts out states why in its header.

## The done-rule

A metatheory milestone is done only when `agda:check` is green **and** its documentation face lands in the same motion — the module header for new structure, or the port-delta note for a ported layer.
Gate-green alone is half a milestone.

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
