# Workflow: soundness testing (checker / machine / evaluator)

> Read when: changing the checker, the typing machine, the subtype relation, effect/grade arithmetic, or an evaluator the differentials compare.

The `checker ≡ machine` differential suite (ADR-9) proves the recursive checker and the typing machine **agree**, not that either is **correct**: a soundness bug both share — the common case, since they are derived from each other — leaves them agreeing on the wrong answer, invisible at any case count.
The same holds for every differential in the project (`eval ≡ run`, the L≡run sequent rows).
A change to any compared surface earns these practices on top of the differential suite (the motivating seam: the A3.2 check-mode `bind` row-escape, `wyrd-e9ou`):

* **Directed coherence oracle.** Relate the two _modes_, not the two implementations: if a term both infers `B'` and checks against `B`, then `B'` must be a consistent subtype of `B` — independent of the differential suite, which structurally cannot see a shared bug.
* **Biased companion — a gate, not a convention (ADR-48).** A free generator is too sparse to bite: the suspect construct reaches check mode against a violating answer only by accident (over the known A3.2 bug the free cross stayed green across tens of thousands of cases).
  Pair every free-generator `*coherence*` oracle with a **biased generator** routing the construct through check mode, plus a **near-miss** that strips the just-added index (an effect row, a grade).
  Worked example: `effectful_bind_subsumption_coherence` in `crates/gandr-core/src/conformance.rs`.
  The Rust `source_policy::run_default_soundness_oracles` gate (`mise run test:soundness-oracles`; CLI subcommand `soundness-oracles`) fails if an oracle lacks a declared companion — tag the oracle `SOUNDNESS-ORACLE-WITNESS: <companion>[, …]` and each companion `SOUNDNESS-ORACLE-COMPANION`.
  Existing companions: the effect-row, integer-literal-defaulting (ADR-39 D4), and graded-thunk legs; the oracle relates modes through `coherence_{value,comp}_subtype` (consistent subtype plus the covariant `Integer ⊑ sized-int` literal relaxation, effect-row and grade legs strict), and a guard test pins that a _variable_ of type `Integer` never widens to a sized-int atom.
* **Both-directions discipline.** Exercise every directed rule in both `Dir::Infer` and `Dir::Check`: the A3.2 hole hid in check mode only (`wyrd-so9w`).
* **Mutation gate.** A surviving mutant over checker/machine/subtype/effect code is a coverage gap, not noise (`wyrd-z87p`); the survivor discipline is [mutation-adequacy.md](mutation-adequacy.md).
* **Standing adversarial pass.** Before committing any substantial checker/machine change, run an independent `precise-analyst` (Opus, xhigh) adversarial pass ([review.md](review.md)) — shared-implementation property tests cannot catch a bug both faces share, which is exactly how the A3.2 hole was found.
