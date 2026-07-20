# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## Unreleased

### Added

* Initial port (2026-06-21): the core CBPV bidirectional checker, the defunctionalized typing machine, the grade semiring carrier, subtyping, and the checker ≡ machine conformance suite, plus the A2.1 integer-literal and A2.2 hole / `Unknown` extensions.
* `mark` — the total semantic marking layer (ADR-17, Zhao et al. POPL 2024): a third, additive realization of the type system that decorates every node with its dual analyzed/synthesized type and localized marks, recovering at each abort site instead of failing fast.
  Carries the Pantograph `{t}_{T1/T2}` boundary and the grade-budget / effect-row / thunkability mark kinds, reconciled with the syntactic empty-hole mark into one discipline.
  Oracle-tested against the recursive checker (accept ⟺ no error mark ∧ root type agreement; total on every input).
  Includes a deterministic type content hash and a hash-consing `TypeInterner` for the unchanged-type O(1)-equality optimization.
* `Hash` on the type graph (`Grade`, `EffectRow`, `EffectSig`, `EffectOp`, `ValueType`, `CompType`, `Ty`) — additive, consistent with the derived `Eq`, enabling the marking layer's content hashing.
