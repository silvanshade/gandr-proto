# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## Unreleased

### Added

* `host` now owns the canonical alloc-only `Exec` / `Fs` / `Proc` / `Env` signatures beside the representation-independent `HostOp` / `HostHandler` seam.
  Surface lowering and the native runtime share one authority without taking a dependency on each other.
* `kernel_bridge` (stage B2.3): the elaborator-side lowering from the checked core CBPV forms into the minimal certified kernel's closed S1 vocabulary (`gandr-kernel-core`, whose dependency this crate now takes — the permitted direction; the section-2 TCB wall forbids the reverse).
  A total, iterative worklist lowering (no host-stack recursion on term depth) that rejects every out-of-S1 node structurally with a precise `BridgeRejection` (holes/`Unknown`, effects/control, `Native`, declared data, the `List`/`Record`/`With`/`Prj` structural stock, `Sigma`/`Split`, the `Path`/`Here`/`Walk` identity fragment, the un-levelled universe, and machine-numeric literals/atoms), erases the operationally-transparent forms (`Annot` peeled; `dup`/`drop` lowered to their ungraded skeletons, C4), resolves names to de Bruijn indices or cross-declaration `Value::Constant` admission indices through a `BridgeContext`, and applies the value-polarity declaration convention (a computation definition enters as a thunk `U C`, B2.1 decision 3).
  Never panics; the kernel re-derives every obligation (K2).
  Witnessed by exact-variant structural rejections per exclusion class, name resolution, erasure, and value-polarity round-trips.
* Initial port (2026-06-21): the core CBPV bidirectional checker, the defunctionalized typing machine, the grade semiring carrier, subtyping, and the checker ≡ machine conformance suite, plus the A2.1 integer-literal and A2.2 hole / `Unknown` extensions.
* `mark` — the total semantic marking layer (ADR-17, Zhao et al. POPL 2024): a third, additive realization of the type system that decorates every node with its dual analyzed/synthesized type and localized marks, recovering at each abort site instead of failing fast.
  Carries the Pantograph `{t}_{T1/T2}` boundary and the grade-budget / effect-row / thunkability mark kinds, reconciled with the syntactic empty-hole mark into one discipline.
  Oracle-tested against the recursive checker (accept ⟺ no error mark ∧ root type agreement; total on every input).
  Includes a deterministic type content hash and a hash-consing `TypeInterner` for the unchanged-type O(1)-equality optimization.
* `Hash` on the type graph (`Grade`, `EffectRow`, `EffectSig`, `EffectOp`, `ValueType`, `CompType`, `Ty`) — additive, consistent with the derived `Eq`, enabling the marking layer's content hashing.
