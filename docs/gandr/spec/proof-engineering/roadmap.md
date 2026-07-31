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

## Standing backlogs

* **The `trans`-ladder conversion**: existing `trans` nests convert to reasoning chains when their module is next touched; the modules under the cell shape are the named backlog.
* **Header hygiene at session close**: the done-rule's durable-face sweep (module headers → the workflow file → the spec tracks → contributor notes → the tracker) — the spec-track face of that sweep now lands in `docs/gandr/spec/` rather than the retired consolidated proposal.
* **Vacuity audits**: each parameterized module instantiated at a witness; each predicate exhibited with a refuter; recheck when new predicates land.
  The decidable-equality side of the definition-site mark discipline currently has no landed mark — the shape decidability module is the first candidate; land the mark when the module is next touched.

## Watch items

* `--hidden-argument-puns` and the options-policy sweep stay per-file and enforced; any exemption is enumerated with a justification.
* The `𝔻` layer letter stays reserved for virtual double categories; nothing claims it without a note in the workflow file.
* Reflection and tactic engines stay out — not "not yet", but under a recorded decision; revisiting is a decision to record, not a call-site judgement.
