# Optimization

Crate scope: `crates/storage-prolly-trees`.

This document records performance and optimization knowledge for the current Prolly-Bao documentation baseline.
It is source-grounded in `METRICS.md`, `ADR.md`, `STATUS.md`, and `TODO.md`; it does not record Rust/API behavior changes.

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

* Benchmark coverage is three Criterion bench sources: `benches/gandr_storage_prolly_trees.rs` with target `gandr_storage_prolly_trees`, `benches/prolly_bao_profile_curve_fixed_width.rs` with target `prolly_bao_profile_curve_fixed_width`, and `benches/prolly_bao_profile_curve_source.rs` with target `prolly_bao_profile_curve_source`.
* The measured baseline is the 2026-06-02 local release Criterion run recorded in `docs/METRICS.md`.
  + Observed command: `cargo bench --package storage-prolly-trees --bench gandr_storage_prolly_trees`.
  + Observed environment: darwin 25.5.0, arm64, Apple M3 Max.
  + Fixtures are deterministic borrowed key/value records.
  + Reusable built trees are prepared outside timed loops where practical.
* Comparator scope is intentionally narrow:
  + No direct Dolt benchmark is recorded.
  + No direct Bao benchmark is recorded.
  + Bao byte-stream results must not be reported as Prolly-Bao tree/proof performance.
* `current`: Benchmark rows and reported estimates:

| Row                                                                     | Time estimate                     | Throughput                                       |
| ----------------------------------------------------------------------- | --------------------------------- | ------------------------------------------------ |
| `gandr_storage_prolly_trees/build/small`                                | `[2.2658 µs 2.2712 µs 2.2768 µs]` | not reported                                     |
| `gandr_storage_prolly_trees/build/medium-throughput`                    | `[2.6228 ms 2.6266 ms 2.6307 ms]` | `[392.00 MiB/s 392.61 MiB/s 393.19 MiB/s]`       |
| `gandr_storage_prolly_trees/lookup/small-hit`                           | `[13.407 ns 13.444 ns 13.481 ns]` | not reported                                     |
| `gandr_storage_prolly_trees/range/small`                                | `[160.20 ns 160.59 ns 160.98 ns]` | not reported                                     |
| `gandr_storage_prolly_trees/range/medium-all-throughput`                | `[157.82 µs 158.35 µs 158.93 µs]` | `[25.772 Melem/s 25.866 Melem/s 25.953 Melem/s]` |
| `gandr_storage_prolly_trees/proof/generate-membership`                  | `[1.2192 µs 1.2213 µs 1.2235 µs]` | not reported                                     |
| `gandr_storage_prolly_trees/proof/generate-non-membership`              | `[265.48 ns 266.24 ns 267.01 ns]` | not reported                                     |
| `gandr_storage_prolly_trees/proof/generate-range`                       | `[317.88 ns 318.69 ns 319.46 ns]` | not reported                                     |
| `gandr_storage_prolly_trees/proof/generate-range/medium-all-throughput` | `[232.50 µs 233.00 µs 233.51 µs]` | `[17.541 Melem/s 17.580 Melem/s 17.617 Melem/s]` |
| `gandr_storage_prolly_trees/proof/verify-membership`                    | `[961.53 ns 963.20 ns 964.84 ns]` | not reported                                     |
| `gandr_storage_prolly_trees/proof/verify-non-membership`                | `[1.9640 µs 1.9678 µs 1.9718 µs]` | not reported                                     |
| `gandr_storage_prolly_trees/proof/verify-range`                         | `[2.0141 µs 2.0177 µs 2.0211 µs]` | not reported                                     |
| `gandr_storage_prolly_trees/proof/verify-range/medium-all-throughput`   | `[4.9690 ms 4.9808 ms 4.9938 ms]` | `[252.28 MiB/s 252.93 MiB/s 253.53 MiB/s]`       |

* These rows are local Criterion estimates, not generalized performance guarantees and not regression thresholds.
* Throughput meaning is row-specific:
  + Build throughput is key+value bytes.
  + Range scan and range proof generation throughput are records/elements.
  + Range proof verification throughput is proof-node bytes.
* Rows marked `not reported` have no throughput estimate.
* The benchmark run did not measure coverage, security/vulnerability numbers, allocation counts, proof byte size, tree node byte size, store byte count, or local-change structural sharing.
* Encoded node layout inspection is `current` and tested.
  Its optimization value is observability: callers can inspect encoded layout shape before deciding whether a future compaction or witness-size experiment improved anything.
  No allocation-count or performance benchmark number has been measured for it.
* `PackedSegmentStore` is `current` as an in-memory packed segment `BlockStore` prototype.
  It is useful for testing packed-node adapter semantics, but no LMDB or other persistent backend is selected and no storage benchmark exists.
* Diff witness and adapter manifest documents are `designed direction` analysis only.
  They are optimization inputs for future API design, not `current` stable Rust APIs.
* Clippy `missing_const_for_fn` is allowed in workspace lint config by user preference.
  That tooling policy does not change Prolly-Bao behavior and is not an optimization measurement.
* `current`: Membership, non-membership, range, and witness paths use compact node material for the implemented one-level internal-root tree shape.
* `current`: Fixed-width and source-file-like profile curves are current benchmark surfaces.
  Compact planner evidence recommends membership-heavy `target-x1`, mixed `target-x2`, and update-heavy `target-x2` for fully compact evidence; source-file-like `target-x3` remains `open decision` policy input only.
  No default tree profile changed.
* Store-backed lookup, range traversal, and proof generation are not `current` stable API behavior.
  `BlockStore` is a narrow encoded-node boundary, the in-memory `PackedSegmentStore` is a prototype under that boundary, and proof verification is store-independent.

## designed direction

* Refresh release-build baselines only through orchestrator-owned benchmark runs.
* Track build, lookup, range, proof generation, and proof verification separately.
* Track proof byte size separately from tree node byte size and store byte count.
* Generalized compact proof work should preserve the explicit verifier contract while extending the current compact one-level proof material to deeper tree shapes only after coverage is specified.
* Encoded layout optimization should start from inspection output, then measure allocation counts, encoded node byte size, and proof/witness byte size with fixtures defined outside timed loops.
* Packed segment store experiments should compare the in-memory prototype against any candidate persistent backend only after the backend contract is selected; LMDB is not selected by this document.
* Store-backed traversal should be added only after public traversal helpers can load and verify reachable nodes from a root without duplicating private decoder logic.
* Structural-sharing metrics should be tracked separately from throughput:
  + changed leaf count
  + affected ancestor count
  + hash-equal unchanged subtree count
* Hash-equal leaves and ancestors may support eager edit evaluation and localized invalidation evidence; exact-byte structural verification remains the acceptance boundary outside Prolly-Bao.
* Native Prolly-Bao witness streams should be optimized as proof transcripts under an agreed root, not as Bao byte streams.
* Allocation-count baselines should be added when optimizing range/proof APIs that currently return owned outputs.
* If a limited Bao benchmark is introduced, keep it scoped to canonical snapshot byte-stream or adapter encode/decode/slice overhead, keep `bao` as a dev-dependency if added, and label benchmark names with `byte-stream`, `snapshot`, or `bao-raw`.
* Diff witness and adapter manifest work should stay document-first until the stable Rust API shape is reviewed.

## open decision

* Regression threshold.
* Allocation-count baseline.
* Hardware-normalized threshold.
* Compact-proof-size target.
* Persistent backend choice and traversal policy, including whether LMDB is appropriate.
* Storage benchmark fixture shape and reporting units.
* Diff witness and adapter manifest API shape.
* Direct Dolt/Bao comparison harness design, if a semantically fair harness is introduced.
* Source-file-like `target-x3` balanced-compromise evidence is policy input only; whether it changes a future mixed-workload planner recommendation remains unresolved.
* Generalized multi-level compact proof selection.
