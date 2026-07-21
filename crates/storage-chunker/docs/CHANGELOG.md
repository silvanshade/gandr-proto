# Changelog

## 2026-07-21 - Reconcile reboot documentation

* `current`: The reboot crate ships no benchmark target, `benches/` source, or benchmark-only development dependency.
* `current`: Removed absorbed claims about benchmark rows and point-in-time measurements that have no executable source in this tree.
* `designed direction`: Future comparison work must land its executable harness, fixtures, and dependencies before crate docs present measurements as current evidence.

## 2026-06-05 - Scanner state hot-path cleanup

* `current`: `ChunkScan` now copies immutable scan-local limits, initial Gear state, and cut mask instead of retaining a `ChunkerParams` borrow for repeated hot-loop access.
* `current`: No runtime/default chunker profile, public API, parameter commitment, proof semantics, or dependency changed.
