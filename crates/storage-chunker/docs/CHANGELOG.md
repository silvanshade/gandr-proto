# Changelog

## 2026-06-05 - Scanner state hot-path cleanup

* `current`: `ChunkScan` now copies immutable scan-local limits, initial Gear state, and cut mask instead of retaining a `ChunkerParams` borrow for repeated hot-loop access.
* `current`: No runtime/default chunker profile, public API, parameter commitment, proof semantics, or dependency changed.

## 2026-06-05 - Gear table hot-path cleanup

* `current`: Replaced the unreachable `gear_for(u8)` fallback branch with an invariant-justified unchecked read from the 256-entry Mach v1 Gear table.
* `current`: No runtime/default chunker profile, public API, parameter commitment, proof semantics, or dependency changed.

## 2026-06-05 - Flattened span benchmark rows

* `current`: Added Mach `mach-record-safe/flattened-spans/...` Criterion rows that call `chunk_spans` over one canonical byte buffer plus record spans.
* `current`: Each flattened-span row checks equivalence against `chunk_record_slices` before registering its timed loop.
* `current`: No runtime/default chunker profile, public API, dependency, root identity, proof, witness, or Bao semantics changed.

## 2026-06-05 - Realistic Okra benchmark basis

* `current`: Replaced the Okra-style dependency-free benchmark stand-in with a dev-only unkeyed BLAKE3 complete-record threshold comparator using Okra's reviewed `u32(hash[0..4]) < 2^32 / Q`, `Q = 32` boundary predicate.
* `current`: Added benchmark startup guards for representative prior-art fixtures so candidate rows must cover all bytes and records monotonically and exercise their expected trigger categories before timed loops are registered.
* `current`: The runtime/default chunker profile, public chunker API, and Prolly-Bao proof semantics did not change.

## 2026-06-05 - Prior-art benchmark candidate rows

* `current`: Expanded the `chunker` Criterion benchmark surface with deterministic prior-art candidate rows for Okra-style complete-record hash-threshold, Dolt-style key-only salted hash/CDF-like size pressure, and hybrid Mach Gear hard-cap profiles.
* `current`: Added prior-art fixture coverage for source-file-like, task-record-like, low-entropy key, fixed-width value update, large-value-reference, and adversarial boundary-seeking records.
* `current`: The runtime/default chunker profile did not change.
* `current`: Comparator rows are non-consensus and not Prolly-Bao proof equivalence.

## 2026-06-02 - Mandatory docs closeout

* `current`: Added mandatory crate-scoped `ADR.md` and `STATUS.md` records for prior crate-modifying work.
* `current`: Refreshed `TODO.md` so the docs-gap note names omitted `ADR.md`, `STATUS.md`, and `TODO.md`, rather than `TODO.md` alone.
* `current`: Confirmed existing `OPTIMIZATION.md` remains the crate-level optimization document for the recorded benchmark and tuning notes.
* `current`: This closeout records documentation state only; it does not claim Rust or API behavior changes.

## 2026-06-02 - Boundary-only benchmarks and crate notes

* `current`: Added the `chunker` Criterion benchmark target for Mach record-safe boundary-only throughput and FastCDC v2020 level-1 raw-byte comparison over canonical record fixtures.
* `current`: Benchmark fixtures cover small records, medium records, low-entropy repeated-window records, and local-edit perturbations for insert, update, and delete cases.
* `current`: Mach record-safe timed loops call the public chunker API over borrowed record slices and track returned boundary output through `black_box`; they do not materialize payload chunks.
* `current`: The scanner is Mach-local and FastCDC-inspired so record-safe boundary behavior and parameter commitments remain under this crate's control.
* `current`: `fastcdc` is a dev-only benchmark comparator for raw byte-slice CDC throughput, not a runtime dependency.
* `current`: Runtime dependencies on `chunk`, semantic text chunkers, storage layers, SQL/DataFusion, IPLD/CAR, Iroh, Git, Automerge, async runtimes, random-number crates, and databases are intentionally outside this crate.
* `current`: Boundary decisions are record-safe: the detector evaluates complete canonical record bytes and cuts only between records rather than splitting a record to satisfy a hash predicate.
* `current`: Gear rolling hashes are non-cryptographic boundary metadata only; they are not identity, integrity, BLAKE3, Bao, Merkle, or proof material.
* `current`: BLAKE3 node identity, root identity, Bao proofs, Merkle proof verification, block storage, and future Prolly-Bao tree construction remain outside `storage-chunker`.
* `current`: Recorded the expanded local Criterion baseline:
  + Mach record-safe rows are included.
  + FastCDC v2020 level-1 raw-byte comparator rows are included.
  + Environment: darwin 25.5.0 arm64 Apple M3 Max.
* `current`: FastCDC comparator rows measure raw byte-slice CDC throughput only; they do not enforce Mach record-boundary semantics or prove Prolly-Bao tree/proof equivalence.
* `current`: Detailed measured benchmark rows are recorded in `METRICS.md`.
* `designed direction`: Future `storage-prolly-trees` consumers should commit the chunker's stable parameter bytes into their own root/proof context without making this crate responsible for proof or node identity behavior.
* `open decision`: Regression thresholds and allocation-count baselines are not committed yet.
