# Metrics

Coverage (2026-06-21 baseline, `mise run cargo:llvm-cov`): ~98.3% line / ~95.0% region.
`interval.rs` is at 100%; `order.rs` is at ~98.3% line / ~95.0% region.

The uncovered remainder is **defensive totality code** the no-panic lint wall requires, not behavioural gaps:

* the `.ok_or(OrderError::CapacityExhausted)?` guards on invariants the construction always satisfies (e.g. an occupied element's link resolving to an occupied slot, a `checked_shl`/`checked_shr` with a bounded shift);
* the unreachable match arms that keep totality (`Slot::Occupied(_) => None` in a free-list read, `Slot::Free(_) => None` after `resolve` proved a slot occupied);
* `spread_label`'s `None` arm, reachable only on an arithmetic conversion the density contract precludes.

These exist solely so the surface stays total without a panic; exercising them would require corrupting the structure's internal invariants, which the public API cannot do.

Complexity (asymptotic, documented not measured): `cmp` is O(1); insertion is O(log² n) amortized (single-level list-labeling, uniform half-density relabel).
The two-level O(1)-amortized refinement is deferred (`OPTIMIZATION.md`).

Performance: the `order` criterion bench (`cargo bench -p gandr-theory-orders`) measures sequential append, endpoint comparison, and the adversarial same-spot insertion that maximizes relabeling.
No thresholds are gated yet.
