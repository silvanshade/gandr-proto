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

## House style

Purpose-built records over raw sigma types; explicit record instances; record types imported at file top with projections opened at the use site; `hiding`/`using` listing one name per line; no `private variable` blocks; copattern style for record values; eager arrow-leading line breaks; the flat proof-term ladder rather than deep `where` nesting; and **every definition carries a comment**.

Two disciplines are load-bearing rather than cosmetic.
Both are instances of the representation rule above, and both exist to keep structures computing under `--without-K`:

* **Witness syntax stays first-order and constructor-headed.** A defined function must never appear in a matchable index.
  Where an operation would otherwise enter an index, speak it through the inductive _graph_ of that operation as a witness relation.
* **No identity-shaped constructor repeats a frame variable across its result indices.** Identity and diagonal cases are derived, never adjoined as constructor shapes.

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
