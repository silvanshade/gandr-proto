# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## Unreleased

### Added

* Initial implementation (slice 3, stage B2.1): the S1 pure polarized core of the minimal certified kernel — the closed de-novo CBPV term/type language, the `Def`/`Axiom` declaration vocabulary, the per-declaration prenex level context with the universe rule over `gandr-kernel-strata`, the append-only environment with its single `add_decl` choke point (the one warned `add_decl_unchecked` bypass and the `#print axioms` audit, on an unforgeable `CheckedId`), the zero-inference bidirectional S1 checker (total on adversarial depth via a recursion-depth budget), the C5-quarantined iterative conversion (type-only at S1, coinciding with structural equality; the value-fragment α-equality present and quarantined), the closed evidence-carrying `KernelError` vocabulary, kernel-native golden fixtures, and the conversion/checker property differentials.
