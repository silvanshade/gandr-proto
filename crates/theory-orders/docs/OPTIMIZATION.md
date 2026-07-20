# Optimization

No crate-specific tuning has been done beyond the workspace release profile; the structure is correctness-first.
The deferred improvements, in rough priority order:

* **Two-level structure for O(1)-amortized insertion.** The current single-level list-labeling is O(log² n) amortized per insertion (O(1) comparison).
  Bender, Cole, Demaine, Farach-Colton & Zito's two-level structure (a top list of O(log n) sublists) lowers insertion to O(1) amortized.
  Deferred until a real edit-trace profile shows insertion cost matters; comparison — the hot path for the incremental engine — is already O(1).

* **Relabel-window growth.** The relabel currently grows the aligned window one power-of-two level at a time from the anchor.
  A direct smallest-enclosing-node search (or the geometric-threshold variant) would tighten the amortized constant; not worth it before the two-level rewrite, which subsumes it.

* **Merkle-CST `OriginEntry` resync.** When this structure is wired into the pipeline, an incremental reparse that shifts byte ranges but preserves subtrees must re-sync order points without re-inserting unchanged elements — matched by merkle content hash on the CST's reproducible `OriginEntry` identity, not the retired tree-sitter node-address seam.
  This is a sync-layer concern above the order structure, but it is where insertion throughput will actually be stressed.

* **`#[no_panic]` smoke for `cmp`.** Mirror `gandr-core`'s grade-leaf-op pattern (a release link smoke proving the hot leaf is panic-free).
  `#[no_panic]` needs a monomorphic instantiation, so this wants a small non-generic seam or a concrete-`T` example; deferred.
  The lint wall already forbids every panic source in production, so this is belt-and-suspenders, not a correctness gap.
