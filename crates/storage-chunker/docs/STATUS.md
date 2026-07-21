# Status

This status file is source-grounded for `storage-chunker` and uses the project status vocabulary exactly.

## current

* The crate is a `#![no_std]` Rust crate with public documentation warnings enabled (`crates/storage-chunker/src/lib.rs:1`, `crates/storage-chunker/src/lib.rs:2`).
* The crate owns deterministic record-safe boundary detection over already-canonical ordered records only.
  It does not serialize records, build Prolly-Bao trees, hash nodes with BLAKE3, store blocks, produce proofs, or talk to storage and transport adapters (`crates/storage-chunker/src/lib.rs:6`).
* The implemented algorithm profile is local, deterministic, FastCDC-2020 inspired, and Gear-based: `AlgorithmVersion::FASTCDC_2020` plus `GearTableVersion::MACH_V1` (`crates/storage-chunker/src/lib.rs:368`, `crates/storage-chunker/src/lib.rs:417`).
* `ChunkerParams::new` validates algorithm version, Gear table version, seed policy, normalization policy, record-boundary rule, and limits before scanning (`crates/storage-chunker/src/lib.rs:848`).
* `ChunkLimits::new` rejects zero limits, inverted byte limits, inverted record limits, and target byte sizes above the current release-build validation cap (`crates/storage-chunker/src/lib.rs:684`).
* Valid parameters expose stable fixed-order commitment bytes through `ChunkerParams::commitment_bytes` (`crates/storage-chunker/src/lib.rs:951`).
* The supported normalization policy preserves caller-provided canonical bytes exactly (`NormalizationPolicy::NONE` at `crates/storage-chunker/src/lib.rs:565`).
* The supported record-boundary rule emits cuts only between complete canonical records (`RecordBoundaryRule::BETWEEN_RECORDS` at `crates/storage-chunker/src/lib.rs:614`).
* Public scanning entry points are:
  + `chunk_record_slices`, which chunks borrowed canonical record slices without copying record payloads (`crates/storage-chunker/src/lib.rs:1463`).
  + `chunk_spans`, which chunks a contiguous canonical byte stream using precomputed record byte spans that must form an exact contiguous partition (`crates/storage-chunker/src/lib.rs:1503`).
* Output is a `Vec<ChunkSpan>` carrying half-open byte spans, half-open record spans, and a boundary reason (`crates/storage-chunker/src/lib.rs:1117`).
* Boundary reasons are hash predicate, maximum byte cap, maximum record cap, and final remainder (`crates/storage-chunker/src/lib.rs:1176`).
* The scanner consumes complete records and may emit a boundary only after a complete record.
  It returns precise errors for oversized records, cap conflicts, non-contiguous spans, out-of-bounds spans, uncovered bytes, unsupported committed values, invalid limits, and checked arithmetic overflow (`crates/storage-chunker/src/lib.rs:1230`, `crates/storage-chunker/src/lib.rs:1641`).
* Gear rolling hashes are non-cryptographic boundary metadata only.
  They are not identity, integrity, BLAKE3, Bao, Merkle, or proof material (`crates/storage-chunker/docs/CHANGELOG.md:48`).
* Criterion, `fastcdc`, and `blake3` are dev-only benchmark dependencies.
  `fastcdc` is used only as a raw byte-slice comparator, and `blake3` is used only by the Okra-style prior-art comparator.
  Neither is a runtime dependency.
* Benchmark source includes Mach record-safe borrowed-record rows, Mach flattened-span rows over one canonical byte buffer plus record spans, FastCDC v2020 level-1 raw-byte comparator rows, and deterministic prior-art candidate rows (`crates/storage-chunker/benches/chunker.rs:1`, `crates/storage-chunker/benches/chunker.rs:90`).
* Prior-art candidate rows cover Okra-style unkeyed BLAKE3 complete-record thresholds, Dolt-style key-only salted hash/CDF-like size pressure, and hybrid Mach Gear hard-cap profiles.
  They are non-consensus and not Prolly-Bao proof equivalence.
* Current prior-art benchmark fixture coverage includes source-file-like, task-record-like, low-entropy key, fixed-width value update, large-value-reference, and adversarial boundary-seeking records (`crates/storage-chunker/benches/chunker.rs:997`, `crates/storage-chunker/benches/chunker.rs:1030`, `crates/storage-chunker/benches/chunker.rs:1060`, `crates/storage-chunker/benches/chunker.rs:1088`, `crates/storage-chunker/benches/chunker.rs:1123`, `crates/storage-chunker/benches/chunker.rs:1153`).
* The runtime/default chunker profile did not change for these benchmark rows (`crates/storage-chunker/benches/chunker.rs:415`, `crates/storage-chunker/benches/chunker.rs:855`).
* Benchmark startup guards validate representative prior-art fixtures for exact byte/record coverage, monotonic ranges, and expected Okra/Dolt/hybrid trigger categories before timed loops are registered.
* Flattened-span rows check their `chunk_spans` output against `chunk_record_slices` before timed loops are registered.

## designed direction

* Downstream Prolly-Bao code should commit `ChunkerParams::commitment_bytes` into its own root/proof context so records are not replayed under incompatible chunking parameters.
  This is outside this crate's proof/node identity behavior (`crates/storage-chunker/src/lib.rs:11`).
* Boundary-only metrics should stay separate from materialized payload chunk, storage, BLAKE3, Bao proof, or Prolly-Bao tree-construction metrics (`crates/storage-chunker/docs/METRICS.md:155`).
* Use flattened-span measurements to separate fixture-layout effects from scanner internals before making low-level hot-path changes.
* Output allocation should be right-sized after measurement instead of reserving one `ChunkSpan` per record when a bytes/target estimate plus margin is safe (`crates/storage-chunker/docs/TODO.md:19`).
* Tuning should remain profile-guided and preserve exact record-boundary semantics, byte/record caps, and precise public errors (`crates/storage-chunker/docs/TODO.md:21`, `crates/storage-chunker/docs/TODO.md:30`).
* Additional algorithm profiles may be added through explicit committed profile values if stronger adversarial boundary-grinding mitigations are selected (`crates/storage-chunker/src/lib.rs:15`).
* Prior-art candidate rows should remain benchmark-only unless a later ADR selects a runtime profile and committed profile values.

## open decision

* Stronger adversarial boundary-grinding mitigations may add algorithm profiles; the only supported profile now is the local deterministic FastCDC-2020-inspired Gear scanner (`crates/storage-chunker/src/lib.rs:15`).
* Regression thresholds are not selected yet (`crates/storage-chunker/docs/METRICS.md:165`).
* Allocation-count baselines for boundary output allocation versus payload-copy avoidance do not exist yet (`crates/storage-chunker/docs/METRICS.md:167`).
* Whether to split a validated public entry point from an infallible internal hot path remains undecided and must be measurement-driven (`crates/storage-chunker/docs/OPTIMIZATION.md:36`).
