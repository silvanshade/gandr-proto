# Status

The crate is a complete, self-contained order-maintenance structure.

Implemented:

* `OrderMaintenance<T>` — a total order over payload-carrying elements with **O(1)** comparison (`cmp`), via single-level list-labeling over the `[0, 2^62)` label universe.
* Insertion: `push_front`, `push_back`, `insert_after`, `insert_before` — midpoint labeling in the common case, minimal aligned-window even-redistribution relabel when a gap is exhausted.
  Removal: `remove`.
  Navigation: `first`, `last`, `next`, `prev`, `iter`.
  Queries: `cmp`, `get`, `contains`, `len`, `is_empty`.
* `Interval` + `OrderMaintenance::interval_contains` — pre/post-order interval containment in O(1) (the Porter term-enclosure test).
* Generation- and structure-checked handles (`Pos`): a removed-and-reused slot bumps a generation, and each structure carries a process-unique id, so stale and foreign handles are rejected (`None` / `OrderError::UnknownPosition`) instead of silently aliasing an unrelated element.
  Structure-id exhaustion is typed (`OrderError::StructureIdExhausted`), and a slot whose `u32` generation is exhausted is retired permanently instead of wrapping.
* Totality: the density cap (the whole-universe relabel window is always at most half full for any realistic length) makes every relabel succeed; representable slot exhaustion and label-density exhaustion surface as `OrderError::CapacityExhausted`, never a panic.

Test coverage includes 20 in-module unit tests covering ordering, insertion/removal at every position, slot reuse, stale/foreign-handle rejection, relabeling at the anchor and at the front, structure-id exhaustion, generation retirement, slot-capacity exhaustion in a tiny representable arena, label capacity exhaustion in a tiny universe, and interval containment; plus 2 integration tests — a 256-case reference-oracle property test cross-checking the public API against a naive `Vec` model, and a full-universe relabel stress.
Coverage is ~98% lines / ~95% regions (`mise run cargo:llvm-cov`); the uncovered remainder is the defensive totality code the no-panic lint wall requires (see `METRICS.md`).
A criterion `order` bench (`harness = false`) measures sequential append, endpoint comparison, and the adversarial same-spot insertion that maximizes relabeling.

This crate is **only** the order structure and its interval query.
The lowest-enclosing-binder lookup / binding pointers, the per-node dual-type + mark + dirty-bit layout for the marked CBPV core, and the merkle-CST `OriginEntry` resync are separate bricks that _consume_ this one; none lives here.
