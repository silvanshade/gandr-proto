# storage-prolly-trees

> **Provenance.** This crate (`gandr-storage-prolly-trees`) is a storage-tier skeleton absorbed directly from the owner's unpublished `mach` `prolly-bao` crate (Apache-2.0, same owner; source commit `fb78601`).
> It is a direct source absorption adapted to gandr's storage tier and lint discipline — not an external dependency, and not wired into any export path — per the ratified vendor plan in `docs/research/massive-term-design.md` §6.1.
> The proof machinery is behind the default `proofs` feature (feature-gated, not stripped); the design-lineage name "Prolly-Bao" is retained in the prose below as absorbed.
> Line-number anchors in this doc set are carried from the source and are approximate after absorption.

Crate-root orientation for maintainers.
The deeper design notes live under `docs/`.

## Status

`current` — `storage-prolly-trees` is Mach's ordered-record Merkle search tree and proof substrate.
It builds over sorted, unique byte key/value records; commits explicit `TreeParams`; uses opaque BLAKE3 node/root identity over canonical encoded node bytes; and verifies membership, non-membership, range, witness transcript, and snapshot facts against an agreed root and parameter context.

`current` — The crate is not a SQL engine, Git-style version graph, persistent storage engine, IPLD/CAR codec, Iroh transport, Bao byte-stream proof format, or merge-policy owner.
The `storage-prolly-trees` name is Bao-inspired: compact root first, requested authenticated material, fail-closed client verification.
Native proofs verify ordered-map facts, not byte offsets in a Bao blob.

## Local contract

`current` — The core contract is intentionally narrow:

- sorted canonical records in;
- duplicate or unsorted input rejected, not merged;
- local `storage-chunker` parameter commitments consumed as consensus material;
- rolling/chunker boundary state treated as layout metadata, not node identity;
- BLAKE3 over versioned, domain-separated encoded node bytes;
- root manifests binding tree kind, encoding version, hash marker, separator convention, record count, chunker parameters, and root-node hash;
- proof verification independent of any backing store.

That contract is the thing to preserve when changing internals.
Stores and adapters may make proof generation faster, but proof consumers must still be able to verify from the root, parameters, query material, and carried proof bytes.

## Relationship to prior art

`current` — The representation is not a new broad data-structure class.
It is a Prolly/Merkle-search-tree composition with a Mach-specific root, record, proof, and failure contract.

The prior art was model-only because the mismatches are load-bearing:

- **Dolt:** useful for node-layout, cursor, chunking, and packed-block ideas.
  Not adopted because Dolt is a SQL database with static-schema tuples, SHA-family chunk identity, product-owned commits/merges/push/pull, and store traversal as the main integrity surface.
  Mach needs canonical byte records, BLAKE3 identity, portable proof transcripts, and no hidden tuple or merge policy in core.
- **Okra:** useful as an LMDB overlay pattern: ordered key/value storage can support Merkle nodes and snapshots.
  Not adopted as core because the verifier must remain store-independent, and persistent backend choice/history ownership are still outside this crate.
- **IPLD / Noms / CAR:** useful for explicit configuration/profile thinking and interchange adapters.
  Not adopted because CID/DAG-CBOR/SHA2 defaults, ecosystem block traversal, duplicate handling, CAR root selection, and codec failure modes do not match the current BLAKE3 encoded-node proof contract.
- **mnem:** useful for small-core boundaries, checked blockstore discipline, canonical encoding habits, CAR-aware adapter posture, and benchmark discipline.
  Not adopted because mnem is a memory graph with CID/DAG-CBOR and retrieval surfaces, not an ordered-record BLAKE3 proof tree.
- **zhangfengcdt/prollytree:** useful for sorted streaming construction, recursive parent chunkers, hard caps, first-key pivots, in-memory store seams, append/fast-forward ideas, and history-independence tests.
  Not adopted because its identity, proof model, duplicate handling, and broader API shape do not match Mach's domain-separated BLAKE3 nodes, strict duplicate rejection, explicit root context, and portable membership/non-membership/range witnesses.

`designed direction` — Keep borrowing mechanics through benchmarks, adapters, and reviewed profiles.
Do not turn prior-art storage, transport, schema, or version models into core behavior by accident.

## Current limitations

`current` — Proofs currently carry enough encoded node bytes for simple, store-independent verification.
`designed direction` — Compact sibling/path witnesses and diff witnesses are not current proof surfaces.

`current` — `BlockStore`, `InMemoryBlockStore`, and `PackedSegmentStore` are narrow encoded-node surfaces.
`PackedSegmentStore` is an in-memory prototype, not a selected persistent backend or stable on-disk segment format.

`current` — Store-backed opening checks that the root node hash exists and is hash-valid in the store.
It does not expose a full store-backed lookup/range/proof producer surface.

`current` — Bao evidence is limited to canonical snapshot bytes in development checks and adapter thinking.
Native witnesses are not Bao proofs, and this crate does not claim Bao wire compatibility.

## Open direction

`designed direction` — Keep exact root/parameter/query verification as the stable boundary while improving compact proofs, witness streaming, allocation behavior, packed segments, and adapter profiles only after the failure modes are explicit.

`open decision` — Persistent backend selection, IPLD/CAR adapter shape, Iroh wire/storage strategy, relational adapter shape, version graph ownership, multi-writer merge semantics, regression thresholds, allocation baselines, and compact-proof-size targets remain unresolved.
