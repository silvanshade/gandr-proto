# Implementation roadmap

What remains on the Rust side, beyond the phase table in [[../implementation|the implementation track]].
The phase plan itself is `PLAN.html` plus the tracker; this file carries the standing constraints and residuals a phase author must not rediscover.
The **reference** feature ordering — what each feature presupposes, and the acceptance criterion that says when it is done — is separate and does not expire with a plan: [[feature-staging]].

## The anticipation register

Nine binding forward-compatibility constraints on rungs not yet built, each preventing a named failure.
The two most load-bearing:

* **Never conflate certificate equivalence with type-level identity.** The certificate normal-form relation is a quotient on derivation _data_, never a type former; conflating it with `Path`/`Flow` is a metatheoretic category error that would surface inside the kernel — the implementation-side twin of "the normal form is a cost fast path, never a decidability result".
* **The overlap enumerator emits seam data**, span-level overlap descriptions, not just boolean or reduced results — a cell store that can answer overlaps but cannot describe them starves the convolution face (landed as the renamed-right-leg accessor).

Also standing: content-addressed identity is structural only — no arena addresses, generation stamps, or session state in hashed term identity.

## The engine↔metatheory statement contract

The dovetail between the Rust fast paths and what the Agda side must prove: identity is replay-equivalence; laws are witnessed cells at value-setoid grade; composition is strict on data and coherent on behaviour; loose composites need not exist.
The tractability witness (`VTractableAt`-shaped: a record per tractability reason, with the normal-form/shift-equivalence decision as its first inhabitant) **does not exist yet** (engine side tracked as `gandr-s9q`) and is where the `cells_equal` fast path's soundness certificate is born; the fast path is **TCB-adjacent** — a guard plus the witness, never documentation — and the off-TCB framing applies to the enumerator only.
The tractability axis (convergent-fragment versus certificate-carried) must stay separable from the invertible/directed mode axis; they coincide in today's two-band design and will not once a convergent directed fragment exists.

## The performance-architecture phase residuals

Carried from the design basis in [[performance-architecture]], each landing with the phase that owns it:

* **Term-face gluing** (origin-`NodeId` caching on values) and **unfolding-face gluing with the hints table** — one normalizer design, landed together; the quote/unfold shared table is the deliverable.
* **Smart unfolding on case progress** — after the case-tree representation is final.
* **Blocker-carrying values** — designed before the solver consumes them.
* **Transactional staging overlay on the arena** — the incremental lane's checkpoint mechanism.
* **The intrusive cached word** (hash + flags + range + depth) — with the arena's node-layout change, not a side-table retrofit.
* **jit≡eval fallback interop** — the backend phase's driver shape.
* **Frozen-meta boundaries per definition** — the incremental lane's staged-elaboration discipline.
* **Transparency defaults policy** — owed by the performance-architecture phase; heights from the definition DAG are mechanical, defaults are not.
* **The explicit thunk-cell budget** — a standing cost note; no host laziness exists to ride.

## Phase residuals worth pinning

* **Kernel-replay** (the certificates phase): an independent reader-side framing walk against the format specification, never shared code; the annotation plane (variance/directedness) must be parseable by both checkers, which is why its slot is reserved early.
* **Modules**: primitive modules with coercive matching, abstract-type sealing with export replay, generative functors, first-class modules with the package Σ.
* **The alphabet-polymorphic enumerator** (shared with the metatheory roadmap's top spike): **executed** — `enumerate_overlaps`, `completion::complete`, `rewrite::normalize`, and the composition and tracelet machinery are generic over the `CellAlphabet` trait, with `SequentAlphabet` as the first inhabitant and an external toy alphabet proving the interface implementable; the shape-layer and directed rule layers can now instantiate the completion machinery.
  Residual: an alphabet with no variance story (an empty `hole_flow`) makes `compose_directed` permissive — documented in the trait contract; instantiators must note it.
* **Runtime-host capability model**: the capability/grant **design landed** as [[implementation/capability-model]] — grants as (signature, operation) atoms over the closed as-built vocabulary, a pre-dispatch check point in the driver, denial as a third runtime outcome, and the multi-shot linear zone made non-vacuous; the handler soundness note cites the certified-implementation criterion [@oliveira-vale-mellies-shao-koenig-stefanesco-2022-layered].
  The document's two-axis review is deferred (`gandr-rzi`), and the model is **not yet implemented** — the crate still ships ambient always-resume with no sandbox, owed before the shell language lands.
* **The NDA/RNTA build-out** (`theory-nominal-automata`): the adopted design's phase content is the model inventory (RNNA/NDA/RNTA), the decision-procedure catalogue, the orbit-finite representation, and the shared name-dropping / bounded-alphabet / reduce-to-classical module, with the deterministic forward runtime monitor and interleaved scopes as driving applications ([[nominal-automata]]).
  **First slice landed**: the three models as defunctionalized data over sort-tagged atoms (the strong-nominal-set representation), the 17-entry machine-checkable catalogue, NDA literal-language membership (the deterministic forward monitor on DDA; α-closure membership under name-dropping), and the name-dropping construction.
  Every remaining catalogue procedure bottlenecks on S-restriction plus the classical NFA/NFTA back-end (`gandr-odr`); the RNTA membership runner, NDA determinization, and the Kleene compiler remain after it.
* **The identity layer's dynamics are new construction, not a port**: the pre-reboot sequent lane declined any program mentioning the path intro/elim forms, so Walk-β and the directed eliminator's β are built new against the L machine at the identity-layer phase — no dynamics survive to port.
* **The CSL fibration property suite**: **landed** — the four right-lifting conditions (frame preservation; nominal identity and alias coherence; black-hole discipline under re-entry; write-back purity) are green proptest properties over the nominal memo-cell half of the heap (`crates/core-sequent/tests/csl_fibration.rs`); separating conjunction degenerates on the content-addressed half, so the nominal half carries the whole suite.
  **Provenance caveat:** no source for the phrase "the four right-lifting conditions" was found in any reachable record — the conditions are reconstructed from the heap's contract and recorded as such in the module docs; if a specific source was intended, replace the provenance section with the named citation.
* **The engines' first external consumer**: nothing outside their own tests reaches `theory-computads` or `theory-virtual-doctrines`; the integration phase is a demo obligation, not construction.
* **Incrementality lane**: the standing gate is incremental ≡ from-scratch on every landed increment.
* **Coverage floor seeding**: enforcement is per production file, and the floor table is currently unseeded.
* **Storage**: persistent backend, deeper tree, and boundary-grinding hardening are declared deficits; wire compatibility is unclaimed.

## Decisions of record pinned for their phases

* **Binder escape resolves by dependent motive**, not a scope check (it also unlocks families) — the dependent-core phase inherits this.
* **Naming**: `Integer` renames to `Int` with `Nat` added when arbitrary precision lands; the semidecision type is named after Sierpiński, never spelled as a Σ; and Sierpiński truth is never encoded as a boolean or optional boolean.
* **Corpus policy**: features land _with_ their corpus examples — the executable witness is part of the feature, and the residuals discipline gains a corpus face.
* **Modules**: the historical conflict between modules-as-compile-time-namespaces and modules-elaborating-to-canonical-records is superseded by the phase commitment to modules as their **own primitive layer**; neither old reading should be cited as current.
* **Certificate verdicts are three-valued** (holds / refuted / declined-within-budget) wherever budget-gated checks feed the store — decline is stuck, never refuted.
* **Handler clauses fall through to the live ambient consumer**, with the continuation edge explicit in IL checking.

## The elaborator half of universe stratification

The kernel half (the loop-checking oracle and landmark posets) is built and deliberately excludes inference; the **elaborator half has no home yet** and is pinned here: run the loop-checking as the complete solver on the elaborator side; generalize each definition's residual level variables prenex-ly; offer displacement as the zero-solving default reuse mode.
The stuck max-plus-equation user experience (entailment and benign loops, never most general unifiers) is an unsolved surface gandr owns.

## The corpus witness inventory

The corpus's binding-guard coverage is one witness (the K-derivation program, wired to a named checker error).
The spec's other binding guards have **no pathological witnesses yet** and each needs one on the K-derivation precedent (tracked as `gandr-u85`): the filler ban; declined horizontal composition; the acyclicity gate's decline; fan-out-family honesty; the shift-equivalent-divergent-replay kill signal; the symmetry-derivation refusal for the directed former; and the constant-map and constant-literal degeneracy guards (at whichever phase first carries a directed certificate stock).
Features land with their corpus examples — this inventory is that policy applied to the guards.

## Stale-documentation repairs owed

Found by inventory against the live tree; each is a one-line-to-small fix in its owning artifact.
The first group landed in the 2026-08 repair pass (`gandr-3o5`); the rest remain owed:

* **Landed:** `ARCHITECTURE.md` re-counted and re-tiered to the live workspace (twenty-four active members over twenty-five directories, the parked doc-class tool uncounted); the retired `gandr-corpus`/`gandr-pro` names dropped from `docs/WORKFLOW.md`; `docs/workflow/corpus.md` and `docs/workflow/docs.md` repointed off the nonexistent `docs/KNOWLEDGE.md` and at live crates, gates, and trust carriers; and the dangling corpus-prefix links repaired from the referring side, so the `kernel-boundary.md`, `core-ir-contract.md`, and `proposal-inspection-protocol.md` citations in the kernel crates' manifests and status files, the checker crate's ADR file, and the render-remote status file now resolve to [[implementation]] sections.
* `PLAN.html`'s current-state section and its kernel-core row predate the build-out and should be read through the tracker; a precise repair proposal is recorded on `gandr-kx5` — the owner maintains `PLAN.html`.
* Further pointers found by that pass ride `gandr-kx5`: two more `docs/KNOWLEDGE.md` citations in `docs/WORKFLOW.md` and one in `docs/gandr/README.md`; the bare (non-prefixed) kernel-boundary/core-ir-contract citations anchoring the K/E/C identifier vocabulary in kernel and checker rustdoc (a deliberate pass, not a mechanical one); the surface-grammar highlight test's dangling adequacy witness; and two research records claiming `docs/gandr/` holds only its README and manifest.
* the spec tracks are registered in `docs/gandr/MANIFEST.yml` as of this revision — the manifest-drift gate covers them (a deliberate registration, since an unregistered corpus document is a fatal gate finding, not a neutral state);
* the pre-reboot spec corpus carried hundreds of dangling citations and references to a retired decision-record directory — a reason its content is absorbed here rather than linked;
* the five pre-reboot records' absorption is **closed by the phase-2 sweep**, with this disposition: the **kernel-boundary** record is retired — its content is verified as substantially carried by this track's trusted-base account (the TCB wall and naming rule, the single admission choke point with its unforgeable checked-id and warned bypass, the canonical level algebra and landmark posets, the export/replay disciplines, the unsoundness threat model, effects never in checked conversion); the **type-system** record lives in this track's checked-language section (the frozen core's grammar as built, the dependent formers, subtyping) and the surface track's spellings; the **typing-machine** record lives in this track's checker account and the checkpoint/staging disciplines of the incremental lane and [[performance-architecture]]; the **effects/control/shell** record lives in this track's runtime-host account and the surface track's shell document; the **incremental-pipeline** record lives in this track's surface-pipeline account and the same checkpoint disciplines.
  The one surviving reference detail not duplicated: the typing-machine record's full frame inventory (twenty core frames and twenty-one wider-language frames) — declined as reference duplication, since the machine's own source and the pre-reboot manual's typing-machine chapter carry it; if the manual is ever retired, the inventory imports into a sub-document of this track instead;
* the directed identity former's surface notation and the rename that produced its current name are carried only by a research study that disclaims decision-record status — ratify or relocate before the identity-layer phase.
