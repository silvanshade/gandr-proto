# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## Unreleased

### Added

* Landmark posets + entailment (2026-07-13, slice 2): Bezem–Coquand loop-checking (TCS 913, 2022) over declared variable-only constraints — `LandmarkPoset::admit` returning the Corollary 3.5 dichotomy as evidence (`ConsistencyWitness` ℕ-homomorphism XOR replayable `LoopWitness` pumping derivation), Corollary 3.4 entailment `entails_leq`/`entails_lt` with `EntailmentWitness`/`EntailmentCountermodel` evidence, one validator per evidence class, the pinned-bottom constant encoding, the paper's §5.2/§5.3 examples as goldens, and the empty-poset property differential against the slice-1 oracle (the design record's slice-2 acceptance gate).
* Initial implementation (2026-07-13, slice 1): the free-fragment level oracle for the minimal certified kernel — always-canonical `Level` over the ADR-78 `{0, +1, max}` algebra, the domination-based `leq`/`lt` oracle with checkable `LeqWitness`/`LeqRefutation` evidence and validators, the `eval` semantic anchor, and the differential property suite against an independent free-AST semantic reference.
