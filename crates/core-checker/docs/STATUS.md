# Status

Stage 1 (milestone A1) is complete: the core CBPV bidirectional type system is realized as both the recursive `checker` and the defunctionalized `machine`, with the two held in step-for-step conformance.

Implemented:

* CBPV bidirectional checker and the defunctionalized typing machine (`checker`, `machine`, `control`).
* The grade semiring carrier with overflow clamped to `ω` (`grade`) and the grade structural operations `dup` / `drop`.
* The effect layer: effect-graded returners `F^ε`, the sealed effect-row carrier, and the `perform` / `handle` terms (A3.2 `+effects`, `effect`).
* The host seam (`host`) owns the canonical `Exec` / `Fs` / `Proc` / `Env` signature builders and operation/field constants beside `HostOp` / `HostHandler`.
  Surface lowering and the native runtime consume this alloc-only authority without depending on each other.
* The control layer: first-class reified stacks `Stk(B, C)`, `resume`, and delimited `reset` / `shift`, with the two-zone context `Γ; Σ` (A3.3 `+control`, `stack`, `ctx`).
* Subtyping — reflexive, but not transitive once `Unknown` participates (`subtype`).
* The two spec-grounded A2 extensions: A2.1 integer literals and A2.2 holes with the `Unknown` type.
* The total semantic marking layer (`mark`): per-node dual-type + mark + dirty-bit decoration, the Pantograph `{t}_{T1/T2}` boundary, the reconciled mark taxonomy, and the unchanged-type interner — oracle-tested against the checker.
* The conformance suite exercising checker ≡ machine agreement, totality, and the grade laws.
* Evaluation lives in `gandr-core-sequent`'s L machine, the sole evaluation driver; the frozen-core CEK evaluator is retired and removed, with the differential suite comparing against frozen outcome snapshots rather than a live second machine.
* The live value/data carriers used by lowering: numeric and string atoms, sums, products, lists, records, thunks/functions, native builtins, and the prelude environment.
* Identity rung 1 (ADR-76): `Path`, `here`, and full Martin-Löf `walk` with explicit motives and definitional walk-β, under the without-K discipline.
* The per-run type interner and reflexive-subtyping pointer fast path.
* The **kernel bridge** (`kernel_bridge`, stage B2.3): the elaborator-side, total, iterative lowering from the checked core CBPV forms into the minimal certified kernel's closed S1 vocabulary (`gandr-kernel-core`).
  Out-of-S1 nodes are rejected structurally with a precise `BridgeRejection`; `Annot`/`dup`/`drop` are erased (C4); names resolve to de Bruijn indices or cross-declaration `Value::Constant` admission indices; a computation definition enters the single-polarity kernel as a thunk (`U C`, B2.1 decision 3).
  This crate now depends on `gandr-kernel-core` — the permitted direction (the section-2 TCB wall forbids the reverse).
  The kernel re-derives every obligation (K2); the bridge is untrusted.
* The **A2.3 incremental checkpoint/validated-resume engine** (`checkpoint`) is implemented.
  `checkpoint_program` records per-item terms, dependency footprints, and typings; `resume` internally aligns edited items, invalidates changed-binding dependents, adopts only structurally identical and footprint-clean checkpoints, and degrades to full re-typing on an order anomaly.
  `tests/incremental.rs` differentially checks adoption, invalidation, insertion, deletion, rename, and generated edits against from-scratch typing.

Tests are green (the conformance and marking suites run under `mise run cargo:nextest`).

Not yet built: the linear context `Σ` is the committed shape but **vacuous in v0** (no obligation source; the discipline is unit-tested over `Sigma` directly); its obligation sources (sessions, sharing, worlds) are deferred `+feature`s.
Process-soup dynamics (`fork` / `acquire` / `migrate` and async signals), unions / intersections, polymorphism, and the row-polymorphic open tail `ρ` are not yet built.
L1 realization and promotion belong to `gandr-sequent`; they are not missing frozen-core forms.
