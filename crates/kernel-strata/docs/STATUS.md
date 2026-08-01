# Status

The crate is the level-oracle slice of the minimal certified kernel (`docs/gandr/spec/implementation.md` §"The trusted base"): the ADR-78 level algebra's **free fragment** plus the **landmark poset** — admission and entailment by Bezem–Coquand loop-checking (TCS 913, 2022) — complete and green.

Implemented (slice 1, landed 2026-07-13):

* `Level` — the always-canonical form `max(c, x_1+o_1, …, x_k+o_k)` of a `{0, +1, max}` level term; smart constructors (`zero`, `constant`, `var`, `succ`, `max`) maintain canonicality, so a non-canonical level is unrepresentable and derived `Eq` **is** the ADR-78 level-equality oracle.
* The order oracle — `leq` / `lt` (strict = shifted comparison, total even at the numeric ceiling) decided by domination, with evidence-returning faces `leq_with_evidence` / `lt_with_evidence`.
* Evidence — `LeqWitness` (per-atom domination bounds + constant bound, inspectable, unforgeable outside the crate) and `LeqRefutation` (a concrete counter-valuation); `validate_witness` / `validate_refutation` check either against its levels with a closed `EvidenceError` rejection vocabulary.
* `Level::eval` — the semantic anchor (wide `u128` arithmetic, checked; adversarial overflow rejected, never wrapped).

Implemented (slice 2):

* `LandmarkConstraint` — declared `≤` / `=` constraints over **variable-only** levels (the recorded API restriction: constant sides are rejected at construction; `crate::poset`'s docs carry the soundness argument for the pinned-bottom constant encoding this buys).
* `LandmarkPoset::admit` — TCS 913 Corollary 3.5 as evidence-carrying data (`AdmissionOutcome`): an admitted poset holds a `ConsistencyWitness` (an explicit `ℕ`-homomorphism, checked by `validate_consistency` through direct evaluation), a refused set yields a `LoopWitness` (a replayable pumping derivation, checked by `validate_loop_witness`).
  Exactly one exists; the certificate is what keeps `U_l : U_l` underivable under hypotheses.
* Entailment — `entails_leq` / `entails_lt` (+ `_with_evidence` faces) decide queries under the poset's hypotheses per Corollary 3.4 (Theorem 3.2 minimal models over the finitely represented shifted Horn system, Lemma 2.1's uniform one-shift query transformation, constants across the paper's constant-free algebra via the pinned bottom generator).
  Positive evidence is an `EntailmentWitness` (a forward derivation, replayed by `validate_entailment_witness`); negative evidence is an `EntailmentCountermodel` (a model of the whole query system refuting a goal atom, checked by `validate_entailment_countermodel`).
* Totality: no panics, no unbounded recursion, no bare arithmetic; failures are the closed `PosetError` vocabulary — including `UnexpectedDivergence` / `EvidenceIncomplete`, which the theory excludes and the code therefore _surfaces_ rather than trusts.

Tests are green under `mise run cargo:nextest` (74 total): the slice-1 suites unchanged; slice-2 unit suites pinning the paper's §5.2 worked example on both dichotomy arms (minimal model `01431` over `abcde`; the `e → a` loop variant with `W = abcde`, shift `m = 2`; the exact certificate `h = 6 − g`), the §5.3 non-total-order refusal, one hand-perturbed rejection per validator arm; and `tests/poset_differential.rs` — **the slice-2 acceptance gate**: property agreement of empty-poset entailment with the slice-1 oracle on every generated input (both strictness modes; the deciders share no decision machinery), evidence validation on every decided query under empty and fixed nonempty posets, `lt ≡ leq ∘ succ` under hypotheses, hypothesis monotonicity, and the order laws.

Complexity posture (the design record's C3 discipline, recorded): admission is weakly polynomial — in the _values_ of declared offsets, which are hand-written prelude data (the paper's own algorithms are values-polynomial; §4).
Entailment on an admitted poset converges Bellman-Ford-style (no positive-gain cycles exist post-admission) with the Corollary 4.2 small-model bound as backstop; a query that somehow diverged would surface as `UnexpectedDivergence`, never loop or lie.

Deliberately **not** here (ADR-78 / the design record's boundary facts): level inference, unification, generalization, displacement, constraint hypotheses beyond the declared landmark poset, `imax`, cumulativity, `Prop`, and constants in declared constraints (variable-only restriction; lifting it needs its own design pass).
The universe rule itself (`U_l : U_m` iff `lt`) belongs to the kernel-core crate (slice 3).

The crate is `#![no_std]` over `core`/`alloc` with zero runtime dependencies — the design record's TCB dependency wall in its sharpest form.
Possible later additions, gated on kernel-core needs, not current concerns: a non-allocating boolean fast path, and an inspectable clause-view API for reviewing evidence indices against the documented compilation without reading the source.
