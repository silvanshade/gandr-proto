# Status

This status file is source-grounded for `storage-chunker`, uses stable symbol names as evidence anchors, and follows the project status vocabulary.

## current

* The crate is a `#![no_std]` Rust crate with public documentation warnings enabled.
* The crate owns deterministic record-safe boundary detection over already-canonical ordered records only.
  It does not serialize records, build Prolly-Bao trees, hash nodes with BLAKE3, store blocks, produce proofs, or talk to storage and transport adapters.
* The implemented algorithm profile is local, deterministic, FastCDC-2020 inspired, and Gear-based: `AlgorithmVersion::FASTCDC_2020` plus `GearTableVersion::MACH_V1`.
* `ChunkerParams::new` validates algorithm version, Gear table version, seed policy, normalization policy, record-boundary rule, and limits before scanning.
* `ChunkLimits::new` rejects zero limits, inverted byte limits, inverted record limits, and target byte sizes above the current release-build validation cap.
* Valid parameters expose stable fixed-order commitment bytes through `ChunkerParams::commitment_bytes`.
* `NormalizationPolicy::NONE` preserves caller-provided canonical bytes exactly.
* `RecordBoundaryRule::BETWEEN_RECORDS` emits cuts only between complete canonical records.
* Public scanning entry points are:
  + `chunk_record_slices`, which chunks borrowed canonical record slices without copying record payloads.
  + `chunk_spans`, which chunks a contiguous canonical byte stream using precomputed record byte spans that must form an exact contiguous partition.
* Output is a `Vec<ChunkSpan>` carrying half-open byte spans, half-open record spans, and a boundary reason.
* `BoundaryReason` distinguishes hash predicate, maximum byte cap, maximum record cap, and final remainder.
* The scanner consumes complete records and may emit a boundary only after a complete record.
  It returns precise errors for oversized records, cap conflicts, non-contiguous spans, out-of-bounds spans, uncovered bytes, unsupported committed values, invalid limits, and checked arithmetic overflow.
* Gear rolling hashes are non-cryptographic boundary metadata only.
  They are not identity, integrity, BLAKE3, Bao, Merkle, or proof material.
* No benchmark target, `benches/` source, or benchmark-only development dependency currently ships; `Cargo.toml` has an empty `[dev-dependencies]` table.
  Prior-art comparators and comparative measurements are designed directions, not current evidence.

## designed direction

* Downstream Prolly-Bao code should commit `ChunkerParams::commitment_bytes` into its own root/proof context so records are not replayed under incompatible chunking parameters.
  This is outside this crate's proof/node identity behavior.
* Boundary-only metrics should stay separate from materialized payload chunk, storage, BLAKE3, Bao proof, or Prolly-Bao tree-construction metrics.
* A future benchmark suite should compare borrowed-record and flattened-span scanner inputs only after its target, dependencies, and fixtures land in this crate.
* Output allocation remains a measurement question; any right-sizing change must be justified by a reproducible benchmark suite.
* Tuning must remain profile-guided and preserve exact record-boundary semantics, byte/record caps, and precise public errors.
* Additional algorithm profiles may be added through explicit committed profile values if stronger adversarial boundary-grinding mitigations are selected.
* Prior-art candidates may be added as benchmarks, but cannot become runtime profiles without a later architecture decision and committed profile values.

## open decision

* Stronger adversarial boundary-grinding mitigations may add algorithm profiles; the only supported profile now is the local deterministic FastCDC-2020-inspired Gear scanner.
* Regression thresholds are not selected; no benchmark harness currently exists.
* Allocation-count baselines for boundary output allocation versus payload-copy avoidance do not exist.
* Whether to split a validated public entry point from an infallible internal hot path remains undecided and must be measurement-driven.
