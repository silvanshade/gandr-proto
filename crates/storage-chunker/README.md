# storage-chunker

> **Provenance.** This crate (`gandr-storage-chunker`) is a storage-tier skeleton absorbed directly from the owner's unpublished `mach` `prolly-bao-chunker` crate (Apache-2.0, same owner; source commit `fb78601`).
> It is a direct source absorption adapted to gandr's storage tier and lint discipline — not an external dependency — per the ratified vendor plan in `docs/research/massive-term-design.md` §6.1.
> The crate stays `#![no_std]` with zero runtime dependencies (empty `[dependencies]`); the design-lineage name "Prolly-Bao" is retained in the prose below as absorbed.
> Line-number anchors in this doc set are carried from the source and are approximate after absorption.

`storage-chunker` is the boundary-detection crate for Prolly-Bao record streams.

## current

* Deterministic, record-safe chunk boundary detection over already-canonical ordered records.
* A local FastCDC-2020-inspired Gear scanner: `AlgorithmVersion::FASTCDC_2020` with `GearTableVersion::MACH_V1`.
* Validated `ChunkerParams` and `ChunkLimits` produce stable `ChunkerParams::commitment_bytes`.
* Public scanning APIs return `ChunkSpan` values with half-open byte spans, half-open record spans, and a boundary reason.
* Cuts are emitted only between complete canonical records.
* Hard byte and record caps are part of the validated parameter surface.
* `NormalizationPolicy::NONE` preserves caller-provided canonical bytes exactly; there is no hidden normalization.

This crate does not serialize records, build Prolly-Bao trees, hash nodes, store blocks, produce proofs, or talk to storage or transport adapters.

## Relationship to `storage-prolly-trees`

`storage-chunker` provides committed chunk spans and parameter bytes for downstream use by `storage-prolly-trees`.

It is not the Prolly-Bao proof, identity, storage, or transport layer.
Downstream code may bind `ChunkerParams::commitment_bytes` into a root or proof context, but BLAKE3 identity, Bao/Merkle proof semantics, block storage, serialization, and adapter behavior remain outside this crate.

## Prior art and profile status

* `current`: FastCDC-2020-inspired Gear scanning is the only implemented runtime profile.
* `open decision`: Chonkers is a possible future benchmark and profile candidate for stricter edit locality and bounded propagation.
* `open decision`: VectorCDC is a possible future benchmark and profile candidate for throughput-oriented hashless boundary detection.
* `current`: UltraCDC is rejected for Mach planning because the available sources do not pin full operational semantics and public implementations diverge.
* `designed direction`: Dolt/Okra-style boundary ideas are possible future benchmark candidates, not current runtime behavior and not Prolly-Bao proof semantics.

No benchmark target, comparator dependency, or candidate row currently ships.
Any future comparison surface must land as executable benchmark code and remain separate from the committed runtime profile.

## Distinctive contract

* Explicit parameter commitment: algorithm, table, seed policy, normalization policy, record-boundary rule, and byte/record limits are committed as stable bytes.
* Record-boundary-only emission: predicates are applied through complete-record consumption, so output spans never split a canonical record.
* Hard caps: maximum byte and record limits force boundaries and return precise errors when a single record or cap combination violates the contract.
* No hidden normalization: the scanner assumes canonical input and preserves byte identity.
* Boundary metadata only: Gear state is non-cryptographic and is not identity, integrity, BLAKE3, Bao, Merkle, or proof material.

## Current limitations and direction

* `current`: output is allocated as a `Vec<ChunkSpan>`.
* `current`: the only supported algorithm/table profile is the local FastCDC-2020-inspired Mach Gear profile.
* `current`: no benchmark target, comparator dependency, or prior-art candidate row currently ships.
* `designed direction`: Prolly-Bao should commit chunker parameter bytes in its own root or proof context.
* `designed direction`: boundary-only metrics should remain separate from payload chunking, storage, hashing, proof, and tree-construction metrics.
* `open decision`: stronger adversarial boundary-grinding mitigations may add explicit committed algorithm profiles later.
* `open decision`: regression thresholds, allocation-count baselines, and whether to split validated public entry points from an infallible internal hot path are not selected yet.
