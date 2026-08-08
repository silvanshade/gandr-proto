# Proof-engineering roadmap

The discipline-side backlog for [[../proof-engineering|the proof-engineering track]].
Items here are about how the tree is mechanized and organized; gandr-specific mathematical obligations are in [[../metatheory/roadmap]].

## Structural moves

* **The package move**: reorganize `metatheory/src/Gandr` into the `Prelude` / `Foundations` / `Metatheory` layout with role splits (base / properties / structure / examples), headers migrating with their content; apply the circuit-carrier layer letter (`𝕎`) in the same motion (decided, not yet applied).
* **The category-theory inventory, demand-driven**: monoidal and skew-monoidal categories, monads and relative monads, comonads, (co)algebras, bialgebras and Hopf algebras, distributive laws, adjunctions, groupoids, duoidal and produoidal categories, lax promonoidal structures, ends and coends, presheaves and nerves, Reedy structure — each built against its first consumer.
  Two are load-bearing now: **algebras for a monad** (the nerve theory's objects are algebras of a monad on graphical species) and **distributive laws** (the published route to the circuit monad is an iterated one).
* **The `Set`-level instance sweep**: present each landed `Set`-level structure through the discrete-setoid former with its strictness mark, and keep parallel modules in parallel order.

## Owed statements

* **The tower-versus-Reedy-fibrant equivalence**: the coinductive certification tower carries all coherence laws explicitly; the explicit-coherence literature proves its diagrams equivalent to Reedy-fibrant ones at each finite level, and the tree owes the statement (not necessarily the proof) that its ω-tower is the guarded-coinductive form of the same data — this is the warrant the coinduction currency currently lacks.
* **Port-delta notes** for every layer re-derived from the sister library, so the divergence debt stays auditable.
* **The coherence-solver direction of record** (the tree's own normal-form machinery as the solver kernel) stays recorded ahead of any implementation; no solver lands before a proof demands it.
* **The inherited proof targets, one disposition each.** The predecessor tree's signature layer carried four named targets about the **object language's** reduction — subject reduction, progress, confluence, canonicity — and this track has never said which survive.
  Each owes one line: retired, or restated at this substrate and owned by a phase.
  The `Gandr.Metatheory.*` namespace is reserved for exactly the results that survive, so leaving the question open reserves a name for an unknown set.
  The confluence named here is **not** the one [[../implementation/circuit-terms]] settles: that result is decidability of confluence for convex double-pushout rewriting with interfaces, a statement about the circuit-term rewriting system, and it discharges nothing on this list.
* **Whether a proof obligation may be tied to a build phase at all.** The kernel-first policy asks for per-phase obligations built in tandem with implementation phases, and nothing here is phase-tied — the demand-driven discipline that governs the category-theory inventory ("each built against its first consumer") points the other way.
  State which discipline governs, because the two give different answers about when a proof is late.

## Standing backlogs

* **The `trans`-ladder conversion**: existing `trans` nests convert to reasoning chains when their module is next touched; the modules under the cell shape are the named backlog.
* **Header hygiene at session close**: the done-rule's durable-face sweep (module headers → the workflow file → the spec tracks → contributor notes → the tracker) — the spec-track face of that sweep now lands in `docs/gandr/spec/` rather than the retired consolidated proposal.
* **Vacuity audits**: each parameterized module instantiated at a witness; each predicate exhibited with a refuter; recheck when new predicates land.

## Watch items

* `--hidden-argument-puns` and the options-policy sweep stay per-file and enforced; any exemption is enumerated with a justification.
* The `𝔻` layer letter stays reserved for virtual double categories; nothing claims it without a note in the workflow file.
* **Reflection-based tactic _macros_** stay out — not "not yet", but under a recorded decision; revisiting is a decision to record, not a call-site judgement.
  The recorded decision is narrower than a blanket "no reflection", and the narrowing is the load-bearing half: `docs/workflow/agda.md` §"Solvers" declines `Tactic.RingSolver` and `Tactic.MonoidSolver` as too brittle **and names proof-by-reflection solvers built on `Relation.Binary.Reflection` as the intended target**, because those are object-level functions with soundness proofs rather than metaprograms.
  What the decision actually forbids is quoting or unquoting syntax, which nothing in the tree does — `Gandr.Arena.Offset` reaches `Data.Nat.Solver`'s `+-*-Solver` with hand-quoted goals and keeps the reflection-based `Data.Nat.Tactic.RingSolver` import commented out at the site.
