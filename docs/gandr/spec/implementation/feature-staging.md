# The feature staging and its acceptance criteria

Two questions have answers that outlive any particular plan: **what must a feature be built after**, and **what would demonstrate that it works**.

This document carries both, as a dependency-ordered staging of the language's features with an acceptance criterion attached to each.

**It is not the build plan.** The phase table in [[../implementation#The build-out at a glance|the implementation track]] and the detailed remaining work in [[roadmap]] are the plan; this ordering is the **reference** one, retained because the two things above are properties of the features rather than of the schedule that happened to be current when they were written.

Read a stage here as a claim of the form _this feature presupposes those_, and its criterion as _this is what would count as done_.
Neither claim expires when the order of construction changes.

## What supersedes it, and what does not

The staging below was the original pedagogical ordering: every feature sequenced after its dependencies, so that each stage is buildable and demonstrable on the one before it.

It was superseded **as an active plan** by a resequencing that optimizes for a different thing — earliest useful capability rather than smoothest dependency slope — and that resequencing is itself no longer the current plan.
Both are recorded here, because the resequencing's value is not its order but its **argument**: it names two places where the dependency slope and the capability slope disagree, and says why.

What is **not** superseded is the dependency ordering itself, which is a fact about the features, and the per-stage acceptance criteria, which are what "done" means.

**Status statements below are restated against this tree**, not inherited from the record this document absorbs.
They were checked against `gandr-core-checker`'s `types` and `syntax` modules for which formers exist, and against the implementation track's phase table for the rest.

## The dependency ordering

Sixteen stages.
Each carries its component breakdown, its deliverable, its acceptance criterion, and the original sizing estimate — which is recorded as an estimate made against a different implementation and is **not** a commitment.

### stage-01

**The core call-by-push-value engine.** Type-check pure values and computations, bidirectionally.

| component | scope                                                                                              |
| --------- | -------------------------------------------------------------------------------------------------- |
| parser    | abstraction, thunk, force, return, bind, pairs, sums, and annotations                              |
| checker   | the recursive bidirectional checker — introductions check, eliminations infer, one subsumption     |
| machine   | derived by continuation-passing and defunctionalization; a descend-and-return control; core frames |
| tests     | property tests: the recursive checker and the machine agree step for step                          |
| surface   | an editor with a derivation tree                                                                   |

**Deliverable.** Abstraction, graded thunk, force, bind, case and split, and holes for partial input.

**Acceptance.** Every core example checks against the executable corpus, and the machine agrees with the recursive checker on a fuzz corpus.

**Status: built**, and the agreement property is the standing differential.
The original estimate was two to four weeks.

### stage-02

**Grades.** Semiring-graded thunks with duplication and discarding, and grade constraints.

| component | scope                                                                                     |
| --------- | ----------------------------------------------------------------------------------------- |
| checker   | the grade semiring, defaulting to the naturals with infinity, and the ordering constraint |
| machine   | a grade-carrying thunk frame; the grade environment in solver state                       |
| surface   | grade badges on thunk and force nodes                                                     |

**Deliverable.** Duplication, discarding, and force-count violations caught as a distinct error.

**Acceptance.** Grade violations are caught, and duplication and discarding accounting is exact.

**Status: built** — `gandr-core-checker` carries a `grade` module and the checker's duplication rule enforces that split grades sum below the thunk's grade.
The original estimate was one to two weeks.

### stage-03

**Polarity-sorted unions and intersections.** Union on values, intersection on computations, with algorithmic subtyping over choice points and a backtracking trail.

| component | scope                                                                                                                                                  |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| checker   | union introduction by subsumption, union elimination at the bind, intersection introduction by typing the same term twice, and the decomposition rules |
| solver    | an invertible-first discipline, a trail of choice points, and an occurs check                                                                          |
| machine   | the intersection-retry and second-union-branch frames; trail-aware checkpoints                                                                         |
| surface   | choice-point markers, with backtracked branches struck through                                                                                         |

**Deliverable.** An overloaded thunk at an intersection of two arrow types, and union-typed bind elimination.

**Acceptance.** The union and intersection examples check with correct backtracking, **and the subtype order is not total** — which is a regression test, not an observation.

**Status: not built.** Neither former is in the checker's type language; both are specified.
The original estimate was two to four weeks.

### stage-04

**Explicit polymorphism.** Universally quantified computations with explicit abstraction and instantiation, and kinding.

| component | scope                                                                                                                             |
| --------- | --------------------------------------------------------------------------------------------------------------------------------- |
| checker   | generalization with the variable bound in the type context, instantiation with the principal premise inferred, and the kind rules |
| machine   | the generalization and instantiation frames                                                                                       |
| surface   | kind display and an instantiation trace                                                                                           |

**Deliverable.** A polymorphic identity and its instantiation at a concrete type.

**Acceptance.** Explicit instantiation examples check, and kinding rejects an ill-kinded instantiation.

**Implicit higher-rank instantiation is a designated later extension**, not part of this stage; the solver is already the right shape for it.

**Status: not built.** The original estimate was two to three weeks.

### stage-05

**Binary session types.** Linear endpoints in the linear zone, duality, the full action set, delegation, and coinductive session subtyping.

| component | scope                                                                                                      |
| --------- | ---------------------------------------------------------------------------------------------------------- |
| grammar   | send, receive, close, select, offer, and fork; the session-type syntax; delegation                         |
| checker   | the linear zone with splitting and neither weakening nor contraction; duality; continuation-carrying rules |
| solver    | coinductive session subtyping, with a visited set over contractive recursive types                         |
| machine   | session frames, and a linearity error for an unconsumed endpoint                                           |
| surface   | protocol badges showing before and after; a state-machine side view                                        |

**Deliverable.** A client and server pair whose session type and its **correct** dual both check.

**Acceptance.** Binary protocol examples check with correct duality, and leftover endpoints are reported.

**Status: not built.** The linear zone exists in the checker's context with a frozen shape and is vacuous at present; no session former exists.
The original estimate was three to five weeks.

### stage-06

**Manifest sharing.** Shared channels with acquire and release, and the equi-synchronizing constraint.

| component | scope                                                                         |
| --------- | ----------------------------------------------------------------------------- |
| grammar   | the shared-channel type shifts, and acquire, release, and shared fork         |
| checker   | the shared zone, and equi-synchronization constraints as a regular-tree check |
| machine   | the acquire, release, and shared-fork-body frames                             |
| surface   | lock and unlock badges; a shared-service view                                 |

**Deliverable.** A shared counter service with concurrent clients, with equi-synchronization violations reported.

**Acceptance.** A shared service with at least two clients checks, and equi-synchronization violations are reported **with their paths** — the path is the part that makes the report actionable.

**A named follow-up, not part of the stage:** manifest deadlock freedom, as extra acquire constraints.

**Status: not built.** The original estimate was two to four weeks.

### stage-07

**Multiparty sessions.** Global types, projection with merge, role-indexed actions, and multiparty initiation.

| component | scope                                                                                                           |
| --------- | --------------------------------------------------------------------------------------------------------------- |
| grammar   | global-type declarations, role-indexed send and receive, and the session-initiation form                        |
| checker   | projection of a global type onto a role — plain merge first, full merge after — and global-type well-formedness |
| solver    | the projection and well-formedness constraint forms                                                             |
| machine   | the multiparty-initiation frame and role-indexed session frames                                                 |
| surface   | a global-type view with a position marker per role                                                              |

**Deliverable.** A two-buyer protocol over three roles, type-checked end to end through projection.

**Acceptance.** The two-buyer protocol checks by projection, and a non-projectable global type is rejected **naming the role and the branch**.

**Status: not built.** The original estimate was three to five weeks.

### stage-08

**Worlds and capability-gated migration.** Located judgements and hypotheses, the at-world modality, holding and migration, mobility, and linear capabilities.

| component | scope                                                                                                  |
| --------- | ------------------------------------------------------------------------------------------------------ |
| grammar   | world declarations, hold, the located let, and migration                                               |
| checker   | a world-annotated context, the mobility judgement, and the location side conditions on the linear zone |
| machine   | the hold, located-let, and migration frames, which save and restore the world                          |
| surface   | world badges, with transitions highlighted at migration frames                                         |

**Deliverable.** A migration returning a mobile remote handle, **with capability consumption visible** in the derivation.

**Acceptance.** Migration checks with capability accounting, and immobile crossings are rejected.

**Status: not built.** The world modality is a designed former outside the current build, and the runtime host has no capability model at all — the design that would price one is [[capability-model|the capability model]].
The original estimate was three to five weeks.

### stage-09

**The incremental pipeline and the read-evaluate loop.** Syntax-tree diffing, dependency-validated checkpoints, hole-tolerant typing, and live feedback.

| component   | scope                                                                                                               |
| ----------- | ------------------------------------------------------------------------------------------------------------------- |
| diff engine | tree comparison and changed-region detection                                                                        |
| checkpoints | save and restore with dependency footprints, and trail-aware invalidation                                           |
| pipeline    | edit, parse, lower with holes, diff, resume from a validated checkpoint, render                                     |
| the loop    | multi-line input, with the ordinary and shared contexts persisting while the linear zone must be consumed per entry |

**Deliverable.** Live type-checking under fifty milliseconds from edit to render, with **sound** suffix reuse.

**Acceptance.** Under fifty milliseconds edit-to-render, and incremental agrees with from-scratch on a mutation corpus.

**Status: in progress.** The incrementality lane is open and its standing gate is exactly the second half of that criterion — incremental agrees with from-scratch on every landed increment.
The original estimate was two to four weeks.

### stage-10 through stage-15

**The module system**, in six rungs, each depending on the one before:

1. structures and functors;
2. sealing, with first-class packages;
3. implicits — world-scoped, fuel-bounded, canonical, and with graded usage;
4. distribution, as offer and take over shared sessions, with located modules;
5. futures, as elaborations of the lazy, spawn, and await forms;
6. transparent existential lifting.

**Acceptance.** Per the module system's own criteria, which the pre-reboot record deferred to the module design rather than restating; the corpus's module design is [[../surface-language/proposed/modules|the module system document]].

**Status: in progress at the first rung.** The implementation track carries modules as their own primitive layer, with primitive modules, sealing with export replay, generative functors, and the package pairing still outstanding.
The original estimate was two to four weeks per rung.

### stage-16

**Dynamics and the runtime.** An operational semantics and an executing runtime.

This stage was **restored from the original design discussion after having been lost in translation into the specifications** — a provenance note worth keeping, because it is the one stage whose absence was an accident rather than a decision.

| component           | scope                                                                                                                                                                                                            |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| dynamics            | a machine derived by the same functional correspondence as the typing machine                                                                                                                                    |
| process runtime     | a configuration soup for sessions and sharing — fork is spawn, acquire is a mutex — with a world as the process or node boundary                                                                                 |
| loop execution      | "show the result type" becomes "run it", with evaluation animated by the same frame surface as typing                                                                                                            |
| effects and control | effects on the returner with handlers, and classical control with first-class stacks, as **core** rather than library concerns, with dynamics as their substrate; the shell language is a later build-out on top |

**Status: built in its sequential core, by a different machine than the one this stage named.** `gandr-core-sequent` is the polarized command intermediate language and its machine, and it is the sole evaluator; the process runtime does not exist and grows with the session features it would execute.
The original estimate was three to five weeks for the sequential core.

## The resequenced ordering, and the two places it disagrees

The resequencing asks a different question of the same stages: **not what is buildable next, but what is useful next** — each milestone independently valuable, each unlocking a concrete capability for a machine consumer rather than only for a human one.

| milestone | contents, as stages                                                  | what it unlocks                                                 |
| --------- | -------------------------------------------------------------------- | --------------------------------------------------------------- |
| first     | the core engine, the derived machine, and the property tests         | the reified checker — the artifact everything else streams from |
| second    | the incremental pipeline with marks, holes, and obligations          | **the streaming checker**, the first thing worth demonstrating  |
| third     | effects and handlers                                                 | tools as effects; sandboxing by handler; one-shot stacks        |
| fourth    | binary sessions and grades                                           | typed tool protocols; budgets; exactly-once actions             |
| fifth     | dynamics, plus the process soup                                      | programs **run**, and the loop becomes an execution environment |
| sixth     | worlds and capabilities                                              | isolation, least authority, typed remote execution              |
| seventh   | sharing and multiparty sessions                                      | multi-agent choreography; contended services                    |
| eighth    | the shell language, and the inspection protocols                     | the control-plane surface: agents operating a machine, governed |
| ninth     | modules and distribution; solver plugins; metaprogramming and phases | skills, verified code, self-extension                           |

**Two inversions against the dependency ordering, and they are the argument.**

**Incrementality moves from ninth position to second**, because for a machine consumer the streaming checker _is_ the product rather than a performance property of one.

**Dynamics moves ahead of worlds**, because a program that runs is a larger step than a program that can be located.

Neither inversion claims the dependency order was wrong.
Both claim that a buildable order and a valuable order are different objects, and that the second is a scheduling choice made with the first in view.

## The decided directions as construction milestones

A separate register: directions that are **settled but unbuilt**, where settled means embodied in the specifications as the direction taken, and construction is the open obligation.

| milestone                     | content                                                                                                                                                                                                    | where it stands                                                                                                                                                 |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the nominal name substrate    | a generic atom and gensym allocator, and the atom-versus-variable sort boundary, extracted as its own crate                                                                                                | landed                                                                                                                                                          |
| the nominal fabric            | the wheeled, polarity-sorted, graded nominal structure as the unifying semantic model — a unification of already-adopted axes rather than a new one — over a supported-sets representation                 | decided; construct                                                                                                                                              |
| the term-level calculus       | the fabric's term face: multi-output, destination-passing terms in the linear zone, terms **as** morphisms, with the feedback wheel as the hiding former — interface dissolution with structural retention | decided; the design pass is sequenced ahead of interpreter optimization and backend hardening, so the multi-value seams are designed in rather than retrofitted |
| polygraphical data            | datatypes and the intermediate language as directed higher-dimensional polygraphs, for specifying laws and for richer solver interaction                                                                   | decided; the convergent slice is partly built, and the untruncated directed coherator is the open construction                                                  |
| the erasible-evidence layer   | a phase-separated evidence sublanguage erasing to the non-dependent runnable core                                                                                                                          | the erasible baseline is decided; **full** dependent types are the intended reach, gated on a compile-and-run feasibility study                                 |
| the package and build manager | content-addressed cache and hydration, endpoint coordinates, an import surface with its lowering, typed module-ascription checks, and distributed builds                                                   | decided — [[proposed/packages]]                                                                                                                                 |
| pretty-printing and layout    | a shared layout engine, a source formatter over the concrete tree, and a separate core printer for the loop and diagnostics — an **output** concern, orthogonal to input layout, which stays insignificant | decided                                                                                                                                                         |
| compilation                   | compiling the checked core, including to machine code, through a textual intermediate representation first and behind a backend seam, with a named non-mainstream native backend                           | decided; the alternative route was **declined, not refuted**, with a revisit trigger                                                                            |

**The construction obligations that register leaves open**, each a tracked research obligation and none of which gates the adoption it belongs to: the untruncated directed coherator, and with it the claim that coherence computes — settled in the finite convergent shadow, which is **not** a completed directed-coherence proof; double-pushout rewriting modulo equivariance; the wheeled-trace and finite-derivation-type axis, whose risk the feedback tier shrinks without discharging; the subsumption half-win against multimodal type theory; the substructural variable-binding computad; directed soundness at two cells and above for the richer substrate; and the full-dependency compile-and-run feasibility study, which is the hinge coupling the evidence layer to compilation.

## The surface programme, one stance per area

Eleven areas of a language-user surface push, each stated as a stance rather than a design, with the corpus document that now owns it where one exists.

| area                          | stance                                                                                                                                                                                                                                                                                                                           | where it lives now                                                                                |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| data declarations             | user data as polygraph presentations: the one-cell case now, with the two-cell production, grade slots, and multi-output result tuples **reserved in the same declaration block from day one** — designed in, never retrofitted                                                                                                  | [[../surface-language/declarations]]                                                              |
| pattern matching              | nested matching over declared constructors, or-patterns and as-patterns, and exhaustiveness compiling to the existing eliminators; matching modulo two-cells is normal-form matching over convergent presentations                                                                                                               | the declaration and circuit-cell documents                                                        |
| recursion and loops           | user-level recursion through thunks — the thunk is the guard — structural recursion over declared data, and loop sugar; termination is a step budget now and obligations later                                                                                                                                                   | [[../surface-language/recursion]] and [[proposed/recursion-former]]                               |
| readable errors               | one policy projecting a complete report and its obligations from checked source-and-state bundles through a single deduplication, ordering, and limit stage; the terminal renderer, the editor protocol, and remote clients are all consumers of it, and remote clients receive one bounded checked frame rather than the report | the transport half is [[inspection-protocol]]; the policy half is not yet in this corpus          |
| entity attributes             | typed attribute schemas on declarations, stored content-hash-neutrally by stable identity; it grows into the host for package metadata and for build configuration written in the language                                                                                                                                       | [[../surface-language/attributes]]                                                                |
| modules and packaging         | a reduced module slice — structures, signatures as ascription, multi-item programs — unblocks typed package-import checking, with module metadata specified as typed attribute data                                                                                                                                              | [[../surface-language/proposed/modules]] and [[proposed/packages]]                                |
| editor integration            | a thin protocol adapter plus a separate render bus, with leaf client builds linking no checker crate                                                                                                                                                                                                                             | [[inspection-protocol]]                                                                           |
| the foreign interface         | external declaration blocks, capability- and effect-gated, with dynamic loading as a capability and abort-on-foreign-unwind as the default; the dynamic path lands on the interpreter before the compiled one                                                                                                                    | [[foreign-interface]]                                                                             |
| self-hosting                  | a staged trajectory — a host-language toolchain, then the language in itself, then recompiling itself — with the fixpoint between the last two stages as the gate; the capstone consumer of every area above, and it runs through the minimal certified kernel as its trust anchor                                               | not yet in this corpus                                                                            |
| the web-assembly target       | emission goes through a dedicated encoder from the intermediate layer rather than from the native backend, which emits none; a browser playground running the interpreter de-risks it first                                                                                                                                      | not yet in this corpus                                                                            |
| value semantics and borrowing | the minimal mutation and update stance the surface needs — functional record and list update, with in-place being the runtime's business — fixed without preempting the borrow and mode calculus                                                                                                                                 | [[../surface-language/value-semantics]] and [[../surface-language/proposed/modes-and-references]] |

**Binding on every area: a feature lands with its executable corpus treatment on the same change.** A feature whose tests pass in its implementing crate is not done until its model examples and their harness coverage land with it, and a design pass carries an example plan.

## Cross-cutting constraints

Four constraints bind on every area above, and each exists because of a specific failure it prevents.

* **A new frozen-core former follows the core contract**: a decision record, a vocabulary resynchronization in the same change, and a proof-assistant face in its own commit.
* **A grammar change holds the grammar gates** — zero conflicts, no external scanner, and the size and state budgets — with corpus-first fixtures.
* **Every new elaboration records its provenance**, so that diagnostics can un-sugar what the elaboration produced.
* **Interpreter-side traversals stay iterative**, never recursive on the host stack.

## Key design decisions

Ten decisions with the reason each binds.
These are the roadmap's own register, and where the corpus documents that own an area restate one, the reason is the same.

| decision                                                                    | why it binds                                                                                                                                                                     |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **derive the machine**, keeping both realizations and property-testing them | it eliminates the frame-payload and stack-convention bug classes **by construction** rather than by testing for them                                                             |
| **polarity-sorted union and intersection**                                  | each connective sits at the polarity where it is well-behaved, and call-by-push-value sequencing discharges the union-elimination value restriction for free                     |
| **grades as a parametric semiring on the thunk**                            | standard graded-modal structure, with zero, one, and unbounded distinct; linearity is policed structurally by the linear zone, never by grades                                   |
| **one linear zone for endpoints and capabilities alike**                    | one discipline covers sessions, sharing, and migration                                                                                                                           |
| **manifest sharing for services**                                           | acquire and release with equi-synchronization as a solver constraint                                                                                                             |
| **multiparty sessions by global types and projection**                      | coherence by construction; the rendezvous primitive is dropped                                                                                                                   |
| **worlds after the located-modality tradition**                             | a held body is typed at its own world and migration is retrieval with mobile results; **capabilities as linear context entries is this design's own addition** to that tradition |
| **distribution is sessions plus worlds**                                    | offering, taking, and futures are elaborations rather than primitives                                                                                                            |
| **the small-and-large module distinction**                                  | it is what buys decidability for first-class modules together with implicits                                                                                                     |
| **validated checkpoints**                                                   | dependency footprints and trail watermarks are what make incremental reuse **sound**; co-contextual typing is the named architectural alternative                                |

## Known challenges, and their mitigations

The presentation and correctness hazards the staging expected, each with the mitigation that was chosen rather than merely proposed.

| challenge                                                                                                                                                        | mitigation                                                                                                   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| visualizing solver search                                                                                                                                        | a trail view with backtracked branches struck through, and collapsible constraint clusters                   |
| session protocol state                                                                                                                                           | a state-machine diagram side by side, with a multiparty global-type view carrying role markers               |
| equi-synchronization and projection diagnostics                                                                                                                  | counterexample paths over the regular tree, rendered on the type itself                                      |
| acquire-acquire deadlocks under sharing                                                                                                                          | a **documented limitation**, with an ordering discipline named as the follow-up                              |
| large derivation trees                                                                                                                                           | virtualized rendering, lazy expansion, and **context deltas rather than snapshots**                          |
| runtime memory                                                                                                                                                   | stream the derivation nodes, and prune checkpoints least-recently-used                                       |
| incremental correctness                                                                                                                                          | dependency footprints and trail watermarks, with the property test that incremental agrees with from-scratch |
| implicit-resolution diagnostics                                                                                                                                  | a fuel-exhaustion report shows the obligation chain, and an ambiguity report lists every candidate           |
| drift between machine and checker                                                                                                                                | both kept in the tree, with their equivalence property-tested from the first stage onward                    |

The derivation-tree mitigation is the one that reappears as a wire contract: **context deltas rather than snapshots** is exactly the shape a derivation node takes in [[inspection-protocol]].

## Naming: self-hosting is not bootstrapping

The two words named different things and were confused, so the distinction is recorded rather than left to context.

**Self-hosting** is the staged trajectory toward a toolchain written in the language itself, and it is the canonical name for that trajectory.
It is never called bootstrapping.

**Bootstrapping** names an architectural posture — architecting so that a bootstrapping future stays available, by way of a minimal certified kernel.
The two feed the same flywheel, and the kernel is the trust anchor the self-hosting trajectory runs through, but they are not the same claim and only one of them is a build milestone.

An earlier, retired use of "bootstrap" named a dogfooding exercise — porting the project's own gate scripts to the language — which is no longer a milestone under either heading.

## The technology stack as it was recorded

The stack the record fixed, carried because several rows are decisions rather than defaults, and marked where this tree has moved.

| layer               | choice                                                                                                                                                                                                                                                                                                                                 |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| the checker         | Rust — its pattern matching fits the machine's frames                                                                                                                                                                                                                                                                                  |
| the reference model | a dependently typed implementation language as the whole-system reference for internals — glued normalization values, and usage-zero erasure feeding code generation — chosen because that arc matches the language's own: dependent surface, erasure, efficient core, backends. The proof assistant stays the metatheory vehicle only |
| the runtime         | the compiled checker artifact, running on a background thread                                                                                                                                                                                                                                                                          |
| the parser          | the precedence-bounded grammar with its push machine, normative; the generated-parser toolkit is retained for parity and for editor tooling only                                                                                                                                                                                       |
| rendering           | a mathematics renderer for rules, with custom vector graphics for the constraint graph, the protocol state machine, and the global-type view                                                                                                                                                                                           |
| build               | the host build system producing a packaged application                                                                                                                                                                                                                                                                                 |

**One row is a recorded refutation rather than a choice.** An earlier draft placed an interpreter layer inside the target runtime; running an interpreter inside a runtime is strictly slower than that runtime's own engine, and a pure checker needs no system interface, so no such shim exists.

## The literature map, and where it went

The source pairs each component with the published work that grounds it — sixty-eight rows naming **seventy-seven distinct works**, covering call-by-push-value, bidirectional typing, unions and intersections, sessions, sharing, multiparty protocols, defunctionalization, grades and coeffects, modal types and worlds, modules, incremental checking, polygraphs, string diagrams, nominal sets, and the erasible-evidence line.

**Roughly twenty of those works are already in [[../bibliography|the corpus bibliography]]**, cited by the documents that own their areas.
The remainder concentrates in areas this corpus has **no document for yet** — sessions, sharing, multiparty protocols, worlds, and the module system — which is also why they are absent: nothing has yet needed them, and the corpus reference rule forbids an entry that nothing cites.

**So the map is carried but not yet transferred**, and where it goes is an open owner decision rather than an oversight.
Its disposition is **carried**, and the transfer is scheduled rather than done.

## Source and confidence

This document absorbs the pre-reboot implementation-roadmap record.
Its plan framing does not survive: the milestone identifiers, decision-record numbers, wave labels, tracker epics, and per-area entry points are provenance rather than design, and the plan they described has been superseded twice.

What is carried is the technical residue the owner ruling names and the standing default preserves: the dependency ordering, the per-stage acceptance criteria, the resequencing argument, the decided-direction register with its construction obligations, the eleven stances, the design decisions with their reasons, the challenge mitigations, the cross-cutting constraints, and the naming rule.

**Two things are marked rather than resolved.**

The stage statuses were restated against this tree rather than inherited, and the check was **narrow**: which type formers exist in `gandr-core-checker`'s type language, which modules exist beside them, and what the implementation track's own phase table records.
It is not a per-stage audit, and a stage marked not built here means its formers are absent, not that no part of its machinery exists.

The sizing estimates are the record's own, made against a different implementation, and are carried as estimates rather than as commitments.
