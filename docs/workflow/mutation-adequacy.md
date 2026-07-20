# Workflow: mutation adequacy — the adequacy ladder

> Read when: writing `# Adequacy` blocks or their tests, triaging mutation survivors, or scheduling a mutation campaign.
> Decisions: ADR-71 (adequacy discipline), ADR-72 (killability as an API-adequacy law).
> Containment/safety for actually running mutants: `docs/HAZARDS.md`; scheduling: [ci.md](ci.md).

The vocabulary, first: a **mutant** is a program variant produced by one small mechanical change (cargo-mutants replaces a function body with a default value, deletes a unary operator, or swaps a binary operator).
A test suite **kills** a mutant when at least one test fails on it; a mutant every test passes is a **survivor**.
**Mutation adequacy** (the score) is the fraction of viable mutants killed — a direct measure of whether the tests can _notice_ the code being wrong, which line coverage alone cannot give (a line can be executed by a test that asserts nothing about it).

Adopted from the 2026-07-09 baseline (69% mutation adequacy against 93.7% line coverage, every survivor hand-classified).
Three moving parts: every nontrivial item documents a falsifiable **adequacy hypothesis** with named **witnesses** ([rust.md](rust.md)), the mutation campaigns are the standing experiment that falsifies hypotheses, and oracle strength is chosen by the **ladder**.

## Campaign lifecycle

Mutation experiments run only as scheduled standalone campaigns through the contained `mise run mutants:*` tasks — never as pre-push, pre-merge, or CI gates.
**A full mutation run is outside the completion scope of every task, feature, and epic.** At the completion of work that adds or substantially refactors production Rust, the consolidated closeout residuals bead names a standalone future campaign, retaining its required `discovered-from` provenance but creating no blocking dependency back to the completed implementation; it names the completed bead/epic, commit range, and intended scope.
A cheap contained run against named mutants MAY serve as focused verification when the tooling supports it; inability to do so never blocks completion.
This campaign is the mutation-adequacy face of the consolidated closeout **residuals bead** ([tracker.md](tracker.md) §“Feature landing and residual closeout”), filed together with the task's manual, corpus, and other-residual faces — tracker.md is the canonical closeout rule.

## The mathematical frame

* **Extension vs intension** (calf; Niu–Sterling–Grodin–Harper, POPL 2022).
  Extensional properties concern _what_ an item returns; intensional properties _how_ it computes.
  `# Contract`'s `requires`/`ensures`/`provides`/`fails`/`panics` are the extensional face; `- intension:` the intensional face.
  **Noninterference** relates them: extensional behavior never depends on an intensional observation; retuning an intension (with its tests) leaves every extensional witness green.
* **Reach, infect, propagate, reveal** (Ammann–Offutt).
  A test kills a mutant only when it reaches the mutated site, infects the state (a distinguishing input), propagates the difference to something observable, and reveals it (the oracle asserts on it).
* **Tests are opens** (the synthetic-topology reading; Escardó, ENTCS 87 (2004)).
  Synthetic topology treats a property as **open** when it is _semidecidable_: a finite computation can confirm it holds (run the test, watch it fail the mutant) but no finite computation can confirm its negation.
  A kill event is such an open; a mutant's survival is the **closed complement** — the property "no test distinguishes this mutant", which no amount of green runs can verify, only fail to refute.
  That is the precise form of "a differential suite proves agreement, not correctness": testing **refutes** equivalence and never confirms it.
  Two consequences do real work here. (1) An operator mutant's disagreement set is **thin** — a measure-zero-like sliver of the input space (`<`→`<=` differs only on the diagonal `a = b`) — which a free product-measure generator almost never charges; the bias in "boundary-biased" is load-bearing, not an optimization. (2) A **finite** disagreement class is compact, i.e. exhaustively searchable: enumerate it totally rather than sampling, and compose enumerations across arguments — finite products stay searchable.

## The survivor taxonomy

A surviving mutant falsifies exactly one item's hypothesis:

| class                        | meaning                                                                                         | the fix lives in                                                              |
| ---------------------------- | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| missing-input (unreached)    | no test executes the mutated line                                                               | the input set — usually a per-file coverage gap                               |
| missing-input (no boundary)  | the line runs, never at a distinguishing input                                                  | boundary inputs (equal operands, exactly-one-true, guard-true)                |
| missing-observation (oracle) | distinguishing input + public observation exist, no assertion notices                           | the test oracle — exact value/variant, declared projection, external evidence |
| missing-projection (API)     | a domain-significant distinction has a clear correct choice but the public API cannot reveal it | the public contract and API first; the witness after                          |
| equivalent (semantic)        | no valid input + declared observation separates the programs, and no domain law requires it     | an exact exclusion with rationale; never a manufactured test                  |
| accepted-noncontractual      | changes only undeclared representation/intension with no principled choice to contract          | an exact exclusion rationale; never a test-only accessor                      |

**Killability is an API-adequacy law (ADR-72).** For every viable mutation class that changes a domain-significant result on a valid input, the public surface MUST provide a semantic observation that can separate correct from mutated.
Apply at the mutation **class**: derive the distinguishing input independently of the current API; ask whether a domain law chooses one behavior in principle; strengthen the oracle if an observation exists; refine design/`# Contract`/API first if not; otherwise record the equivalence/noncontractual rationale.
Never expose raw private state, add a test-only accessor, or promote an arbitrary choice to improve a score; impactful API refinements follow the normal design/ADR discipline.

## The adequacy ladder

Prefer the strongest rung that can carry a decision surface; the hypothesis states the placement:

| rung             | mechanism                                                                                      | kills                                                       |
| ---------------- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| **L0 types**     | no-`Default` values, newtypes, illegal states unrepresentable                                  | the mutant does not compile (70% of the baseline died here) |
| **L1 evidence**  | the API returns a checkable witness/certificate; tests **validate** it against the input       | any mutant that cannot forge valid evidence                 |
| **L2 agreement** | differential vs a naive reference; pinned conformance goldens; corpus stage-boundary artifacts | any divergence on any generated input                       |
| **L3 pointwise** | boundary inputs + exact-variant/value assertions                                               | the residue: tie-breaks, guard arms, boundary comparisons   |

Binding rules (ADR-71 D3–D6):

* **External oracle.** An L1/L2 oracle must be external to the mutated code (a naive reference, a replay checker, a pinned golden).
  Self-agreement is blind to any mutant shifting both runs identically; such surfaces take a pinned external golden.
* **Declared projections.** Intensional assertions go only through declared `- intension:` projections.
  Fingerprints compare pairs of live computations, never serve as pinned goldens.
* **Extensional completeness.** Every `- fails:`/`- ensures:` bullet has a witness asserting the **exact variant or value** on a triggering input.
  `is_err()`-only checks do not witness; `#[should_panic]` is disqualified (it passes on _any_ panic and rewards the mutant).
* **Boundary-biased inputs.** For dense decision surfaces, one boundary-biased property test over scattered unit cases; finite classes enumerated exhaustively (a shared combinator home is a tracked follow-up).
* **Design for adequacy.** Prefer the API shape that returns evidence — mutants become self-incriminating and trust concentrates in small validators, where the L3 rigor is spent.
* **Per-file coverage floors.** Crate-level coverage judgment is banned (`docs/HAZARDS.md`).

## From contract to tests — how to read the blocks

Each clause compiles to a test obligation:

* **`- ensures:`** → one witness per postcondition asserting the exact value (or declared projection) on an ordinary input **plus** every boundary input the hypothesis names.
* **`- fails:` / `# Errors`** → one witness per failure mode: trigger exactly that mode, assert the exact variant (and discriminating payload).
  "An error occurred" witnesses nothing.
* **`- requires:`** → not driven as tests; they define the valid input space, whose _boundary_ generator strategies must cover.
  In-body, back with `debug_assert!`.
* **`- panics: none.`** → testable under mutation with no dedicated test, provided boundary inputs actually run.
* **`- intension:`** → one witness per declared property, only through the declared projection; when retuning an intension, revise these witnesses in the same change and confirm every extensional witness stays green.
* **`- hypothesis:`** → this IS the test plan: L1 → write the validator test (validate evidence against the input, never a predicted answer; the validator itself gets L3 rigor); L2 → write or extend the differential/golden, confirm the oracle is external; L3 residue → enumerate the named boundary inputs exhaustively, pair each with the named observation.
* **`- witness:`** → closing bookkeeping (G0-checked): each named witness must actually assert what the hypothesis says — the standard is a reviewer can apply the mutant by hand and watch the named witness fail.
* **Authoring order.** Contract before implementation where practical; hypothesis before tests; witnesses last.
  A later survivor **falsifies** the hypothesis: classify, strengthen input or oracle or complete the missing projection, update contract+hypothesis+witnesses together.

## Instrument limits, gates, and the success metric

cargo-mutants has no argument-swap or wrong-algorithm operator, so the score under-measures the fault model this discipline defends against (agent-authored code's characteristic faults) — judge the intensional face by the external-oracle rule, not the score.

Gates, staged: **G0** — every `- witness:` path resolves and `# Adequacy` is present on nontrivial new/refactored items; **G1** — survivors joined span-to-item and classified (per-line hit data is ground truth for reachedness).
Per-file floors are a further staged gate.

Scope: mandatory for new or substantially refactored production Rust; existing survivor hotspots stay in the triage lane — no blanket retrofit.
ADR-71 D8's metric: at the first sweep after `gandr-graph` lands, new-crate survival < 5% (baseline 30.8%) with every survivor classified — else the reversal triggers fire.
The campaign bead owns that experiment; the metric is never an acceptance condition for the implementation bead.
