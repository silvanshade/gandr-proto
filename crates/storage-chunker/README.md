# storage-chunker

> **Provenance.** This crate (`gandr-storage-chunker`) is a storage-tier skeleton absorbed directly from the owner's unpublished `mach` `prolly-bao-chunker` crate (Apache-2.0, same owner; source commit `fb78601`).
> It is a direct source absorption adapted to gandr's storage tier and lint discipline — not an external dependency — per the ratified vendor plan in the massive-term design study §6.1, which left this repository with the research corpus.
> The crate stays `#![no_std]` with zero runtime dependencies (empty `[dependencies]`); the design-lineage name "Prolly-Bao" is retained in the prose below as absorbed.
> Symbol-qualified names, rather than numeric line anchors, identify source evidence in this doc set.

`storage-chunker` is the boundary-detection crate for Prolly-Bao record streams.

## current

- Deterministic, record-safe chunk boundary detection over already-canonical ordered records.
- A local FastCDC-2020-inspired Gear scanner: `AlgorithmVersion::FASTCDC_2020` with `GearTableVersion::MACH_V1`.
- A typed content-defined scanner: `AlgorithmVersion::TYPED_CDC` consumes `BoundaryEvent` values and applies committed kappa and token-cap cuts.
- Validated `ChunkerParams` and `TypedChunkerParams` produce stable parameter commitments.
- Public scanning APIs return `ChunkSpan` values with half-open byte spans, half-open record spans, and a boundary reason; typed scanning returns `CutDecision`.
- Cuts are emitted only between complete canonical records for the record-safe profile, and only at caller-reported boundary constructors for the typed profile.
- Hard byte and record caps are part of the validated record-safe parameter surface; the typed profile has a committed hard token cap.
- `NormalizationPolicy::NONE` preserves caller-provided canonical bytes exactly; there is no hidden normalization.

This crate does not serialize records, build Prolly-Bao trees, hash nodes, store blocks, produce proofs, or talk to storage or transport adapters.

## Relationship to `storage-prolly-trees`

`storage-chunker` provides committed chunk spans and parameter bytes for downstream use by `storage-prolly-trees`.

It is not the Prolly-Bao proof, identity, storage, or transport layer.
Downstream code may bind `ChunkerParams::commitment_bytes` or `TypedChunkerParams::commitment_bytes` into a root or proof context, but BLAKE3 identity, Bao/Merkle proof semantics, block storage, serialization, and adapter behavior remain outside this crate.

## Prior art and profile status

- `current`: FastCDC-2020-inspired Gear scanning is implemented as `AlgorithmVersion::FASTCDC_2020`.
- `current`: typed content-defined scanning is implemented as `AlgorithmVersion::TYPED_CDC`; `TypedChunkerParams` commits kappa and the hard token cap.
- `open decision`: Chonkers is a possible future benchmark and profile candidate for stricter edit locality and bounded propagation.
- `open decision`: VectorCDC is a possible future benchmark and profile candidate for throughput-oriented hashless boundary detection.
- `designed direction`: Dolt/Okra-style boundary ideas are possible future benchmark candidates, not current runtime behavior and not Prolly-Bao proof semantics.

No benchmark target, comparator dependency, or candidate row currently ships.
Any future comparison surface must land as executable benchmark code and remain separate from the committed runtime profile.

## Distinctive contract

- Explicit parameter commitment: algorithm, table, seed policy, normalization policy, record-boundary rule, byte/record limits, and typed kappa/cap fields are committed as stable versioned bytes.
- Record-boundary-only emission: record-safe predicates are applied through complete-record consumption, so output spans never split a canonical record.
- Typed boundary-event emission: typed predicates are applied only to caller-reported boundary constructors, with rolling-hash and hard-token-cap decisions owned by the chunker.
- Hard caps: maximum byte and record limits force record-safe boundaries; the typed token cap forces typed boundaries deterministically.
- No hidden normalization: the scanner assumes canonical input and preserves byte identity.
- Boundary metadata only: Gear state and typed residues are non-cryptographic and are not identity, integrity, BLAKE3, Bao, Merkle, or proof material.

## Current limitations and direction

- `current`: output is allocated as a `Vec<ChunkSpan>` for record-safe scans; typed scans return decisions without payload allocation.
- `current`: the local FastCDC-2020-inspired Mach Gear profile and typed boundary-event profile are implemented.
- `current`: no benchmark target, comparator dependency, or prior-art candidate row currently ships.
- `designed direction`: Prolly-Bao should commit chunker parameter bytes in its own root or proof context.
- `designed direction`: boundary-only metrics should remain separate from payload chunking, storage, hashing, proof, and tree-construction metrics.
- `open decision`: stronger adversarial boundary-grinding mitigations and future profiles remain explicit committed-profile additions.
- `open decision`: regression thresholds, allocation-count baselines, and whether to split validated public entry points from an infallible internal hot path are not selected yet.
