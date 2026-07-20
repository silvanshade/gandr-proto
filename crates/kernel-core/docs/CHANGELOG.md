# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## Unreleased

### Added

* Initial implementation (slice 3, stage B2.1): the S1 pure polarized core of the minimal certified kernel — the closed de-novo CBPV term/type language, the `Def`/`Axiom` declaration vocabulary, the per-declaration prenex level context with the universe rule over `gandr-kernel-strata`, the append-only environment with its single `add_decl` choke point (the one warned `add_decl_unchecked` bypass and the `#print axioms` audit, on an unforgeable `CheckedId`), the zero-inference bidirectional S1 checker (total on adversarial depth via a recursion-depth budget), the C5-quarantined iterative conversion (type-only at S1, coinciding with structural equality; the value-fragment α-equality present and quarantined), the closed evidence-carrying `KernelError` vocabulary, kernel-native golden fixtures, and the conversion/checker property differentials.
* K5 export (slice 3, stage B2.2): the re-checkable export module (`export`) — `write` serializes an `Environment` to self-contained, deterministic canonical bytes (admission-ordered declarations, `BTreeMap`-sorted level atoms, per-declaration admission marks, and a version header; the transitive audit sets are recomputed on replay, not written); `decode` is the total validating reader over a closed error vocabulary (`DecodeError`: the truncation / bad-tag / malformed rejection triple, plus named refusals for reserved declaration kinds, non-empty reserved slots/sections, and an unknown version); `read` replays the decoded sequence through the choke point (`ReadError` holding the decode and re-admission planes distinct).
  Levels rebuild through the strata smart constructors and a whole-artifact canonical-bytes check rejects non-canonical encodings; term and type trees decode iteratively over an explicit worklist.
  The seven ratified format-plane reservations (the four reserved declaration kinds, structured names, the four per-`Def` annotation slots, and the reserved minted-atom table) are present and empty at v0.
  Round-trip, determinism, and rejection-totality property differentials accompany it.
