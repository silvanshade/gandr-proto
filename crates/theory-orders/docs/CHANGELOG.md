# Changelog

The format is hand-maintained and grows only with real changes; it is not auto-generated.

## Unreleased

### Added

* Initial implementation (2026-06-21): the self-contained order-maintenance structure for the A2 incremental pipeline — `OrderMaintenance<T>` (single-level list-labeling, O(1) comparison, O(log² n) amortized insertion), generation- and structure-checked `Pos` handles, the `Interval` pre/post-order containment query, the reference-oracle property test, and the `order` criterion bench.
