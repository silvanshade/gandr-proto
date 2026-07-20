# Status

Stage 1 (milestone A1) is complete: the core CBPV bidirectional type system of `docs/gandr/spec/type-system.md` §3, realized as both the recursive `checker` and the defunctionalized `machine`, with the two held in step-for-step conformance.

Implemented:

* CBPV bidirectional checker and the defunctionalized typing machine (`checker`, `machine`, `control`).
* The grade semiring carrier with overflow clamped to `ω` (`grade`) and the grade structural operations `dup` / `drop`.
* The effect layer: effect-graded returners `F^ε`, the sealed effect-row carrier, and the `perform` / `handle` terms (A3.2 `+effects`, `effect`).
* The control layer: first-class reified stacks `Stk(B, C)`, `resume`, and delimited `reset` / `shift`, with the two-zone context `Γ; Σ` (A3.3 `+control`, `stack`, `ctx`).
* Subtyping — reflexive, but not transitive once `Unknown` participates (`subtype`).
* The two spec-grounded A2 extensions: A2.1 integer literals and A2.2 holes with the `Unknown` type.
* The total semantic marking layer (`mark`): per-node dual-type + mark + dirty-bit decoration, the Pantograph `{t}_{T1/T2}` boundary, the reconciled mark taxonomy, and the unchanged-type interner — oracle-tested against the checker.
* The conformance suite exercising checker ≡ machine agreement, totality, and the grade laws.
* The frozen-core CEK evaluator (`eval`): the current pipeline driver and the differential oracle for the polarized L machine, with environment-backed call-by-need, algebraic effects/deep handlers, and delimited control.
* The live value/data carriers used by lowering: numeric and string atoms, sums, products, lists, records, thunks/functions, native builtins, and the prelude environment.
* Identity rung 1 (ADR-76): `Path`, `here`, and full Martin-Löf `walk` with explicit motives and definitional walk-β, under the without-K discipline.
* The per-run type interner and reflexive-subtyping pointer fast path.

Tests are green (the conformance and marking suites run under `mise run cargo:nextest`).

Not yet built: the linear context `Σ` is the committed shape but **vacuous in v0** (no obligation source; the discipline is unit-tested over `Sigma` directly); its obligation sources (sessions, sharing, worlds) are deferred `+feature`s.
The A2.3 incremental checkpoint/diff engine, process-soup dynamics (`fork` / `acquire` / `migrate` and async signals), unions / intersections, polymorphism, and the row-polymorphic open tail `ρ` remain frozen in `docs/gandr/spec/core-ir-contract.md` §0.
L1 realization and promotion belong to `gandr-sequent`; they are not missing frozen-core forms.
