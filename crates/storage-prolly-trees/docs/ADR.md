# Architecture Decisions

Crate scope: `crates/storage-prolly-trees`.

These records capture durable architecture decisions for the current Prolly-Bao implementation, not a per-task change log.
Status vocabulary is limited to `current`, `designed direction`, and `open decision`.

## Decision index

* PB-ADR-0001 (`current`): Use a BLAKE3-addressed ordered-record Merkle search tree core.
* PB-ADR-0002 (`current`): Commit `storage-chunker` parameter bytes into roots and proof envelopes.
* PB-ADR-0003 (`current`): Keep proof verification store-independent and fail closed.
* PB-ADR-0004 (`current`): Keep storage as a narrow encoded-node `BlockStore` boundary.
* PB-ADR-0005 (`current`): Exclude Bao proof/transport, Iroh/IPLD/CAR, SQL/DataFusion, and persistent backend semantics from core.
* PB-ADR-0006 (`current`): Keep `storage-prolly-trees` as an evocative Bao-inspired name while separating native witness streams from Bao byte-stream proofs.
* PB-ADR-0007 (`current`): Encode native witness transcripts as versioned Prolly-Bao proof-response material that reuses proof verifiers.
* PB-ADR-0008 (`current`): Treat canonical snapshot bytes as Prolly-Bao materialization with Bao verifier evidence limited to adapter/dev tests.
* PB-ADR-0009 (`open decision`): Reject a core Iroh interop dependency for now; keep exact-byte cacheability evidence local until a narrow adapter boundary is reviewed.
* PB-ADR-0010 (`current`): Expose encoded-node layout inspection without changing canonical node bytes or BLAKE3 identity.
* PB-ADR-0011 (`current`): Keep packed segment storage as an in-memory prototype behind the unchanged `BlockStore` boundary.

## PB-ADR-0001: BLAKE3-addressed ordered-record Merkle search tree core

### Status (PB-ADR-0001)

`current`

### Context (PB-ADR-0001)

* `NodeHash` is the public opaque BLAKE3 identity for encoded Prolly-Bao nodes (`crates/storage-prolly-trees/src/types.rs:10`).
* `TreeKind::MerkleSearch`, `EncodingVersion::V1`, `HashAlgorithm::Blake3`, and `SeparatorConvention::FirstKey` are the currently supported committed tree parameters (`crates/storage-prolly-trees/src/types.rs:111`, `crates/storage-prolly-trees/src/types.rs:120`, `crates/storage-prolly-trees/src/types.rs:145`, `crates/storage-prolly-trees/src/types.rs:160`).
* `TreeRoot` carries the root hash, committed `TreeParams`, and represented record count (`crates/storage-prolly-trees/src/types.rs:331`).
* `PortableProofTree::build` builds from strictly sorted borrowed records and rejects unsorted input or duplicate keys before constructing node material (`crates/storage-prolly-trees/src/proof.rs:663`, `crates/storage-prolly-trees/src/proof.rs:1212`).

### Decision (PB-ADR-0001)

The crate's current core is an ordered-record Merkle search tree surface addressed by BLAKE3 node/root identity.
Records are interpreted in strict key order, internal separators use the first reachable child key, and roots commit the parameter set and record count.

### Rationale (PB-ADR-0001)

* Strict order and duplicate rejection avoid an implicit merge policy for equal keys.
* Opaque BLAKE3 identities keep the core independent from transport, database, or byte-stream proof formats.
* Encoding version, tree kind, hash algorithm, separator convention, and record count are root context, not caller-side assumptions.

### Trade-offs (PB-ADR-0001)

* The current implementation is a proof-oriented first slice: it carries owned records and encoded nodes inside `PortableProofTree` rather than exposing a fully store-backed traversal API.
* BLAKE3 node identity does not imply Bao wire compatibility or flat byte-stream proof compatibility.

## PB-ADR-0002: Commit local chunker parameter bytes into roots and proofs

### Status (PB-ADR-0002)

`current`

### Context (PB-ADR-0002)

* `TreeParams` stores both `ChunkerParams` and stable `chunker_parameter_bytes` copied from `chunker_params.commitment_bytes()` (`crates/storage-prolly-trees/src/types.rs:175`, `crates/storage-prolly-trees/src/types.rs:196`).
* `TreeParams::current()` uses `ChunkerParams::default_fastcdc()` for the current default profile (`crates/storage-prolly-trees/src/types.rs:219`).
* `PortableProofTree::build` feeds canonical record bytes to `chunk_record_slices` with the committed chunker parameters before building leaves (`crates/storage-prolly-trees/src/proof.rs:679`).
* `ProofEnvelope` copies the root encoding version and chunker parameter bytes into proof metadata (`crates/storage-prolly-trees/src/types.rs:684`).
* Verification rejects mismatched chunker commitment copies (`crates/storage-prolly-trees/src/proof.rs:1133`, `crates/storage-prolly-trees/src/proof.rs:1160`, `crates/storage-prolly-trees/src/proof.rs:1203`).

### Decision (PB-ADR-0002)

Chunker parameters are consensus-sensitive material for Prolly-Bao roots and proofs.
The crate commits the local `storage-chunker` parameter bytes into root material and proof envelopes, while leaving cryptographic node identity to BLAKE3 over encoded node bytes.

### Rationale (PB-ADR-0002)

* Replaying the same ordered records requires the same boundary-detection parameters to derive the same leaf grouping.
* Committing parameter bytes makes proof verification reject roots or proofs interpreted under different chunker settings.
* Keeping chunking separate from identity prevents rolling/Gear boundary metadata from becoming a cryptographic identity surface.

### Trade-offs (PB-ADR-0002)

* Root and proof compatibility is intentionally tied to stable chunker commitment bytes.
* Chunker parameter migration needs explicit versioning or compatibility handling rather than silent reinterpretation.

## PB-ADR-0003: Store-independent proof verification with fail-closed errors

### Status (PB-ADR-0003)

`current`

### Context (PB-ADR-0003)

* `MembershipProof`, `NonMembershipProof`, and `RangeProof` carry proof metadata, root-node identity, query material, returned records or evidence, and encoded proof nodes (`crates/storage-prolly-trees/src/proof.rs:203`, `crates/storage-prolly-trees/src/proof.rs:381`, `crates/storage-prolly-trees/src/proof.rs:508`).
* Verifiers accept explicit expected root and parameters, then validate proof envelopes before decoding node material (`crates/storage-prolly-trees/src/proof.rs:290`, `crates/storage-prolly-trees/src/proof.rs:459`, `crates/storage-prolly-trees/src/proof.rs:586`, `crates/storage-prolly-trees/src/proof.rs:1169`).
* Verification recomputes BLAKE3 identity from carried node bytes and rejects mismatches, duplicate proof nodes, unreachable nodes, malformed bytes, wrong queries, and wrong values or records (`crates/storage-prolly-trees/src/proof.rs:1450`, `crates/storage-prolly-trees/src/proof.rs:1624`, `crates/storage-prolly-trees/src/proof.rs:1688`).
* The public error vocabulary includes unsorted input, duplicate keys, malformed node bytes, unknown node hashes, hash mismatches, incompatible parameters, unsupported encodings, invalid proof shapes, invalid ranges, and chunker errors (`crates/storage-prolly-trees/src/error.rs:7`).

### Decision (PB-ADR-0003)

Proof verification is independent of any `BlockStore`.
A verifier checks explicit root context, committed parameters, proof kind, query material, BLAKE3 node identity, canonical node decoding, reachability, and returned evidence before treating a result as valid.
Invalid or ambiguous material returns `ProllyBaoError`; it is not treated as valid by default.

### Rationale (PB-ADR-0003)

* Portable proof material can be checked without trusting a local store implementation.
* Explicit expected root, parameter, key/range, and value inputs prevent proofs from silently proving a different statement than the verifier requested.
* A fail-closed error vocabulary gives callers precise rejection reasons without introducing permissive fallback behavior.

### Trade-offs (PB-ADR-0003)

* `current` membership, non-membership, and range proofs use compact node material for the implemented one-level internal-root tree shape.
  Generalized multi-level compaction remains future work.
* Store-independent verification duplicates some decoded material work instead of relying on cached store traversal.

## PB-ADR-0004: Narrow encoded-node store boundary

### Status (PB-ADR-0004)

`current`

### Context (PB-ADR-0004)

* `BlockStore` inserts and loads canonical encoded nodes keyed by `NodeHash` (`crates/storage-prolly-trees/src/store.rs:29`).
* `InMemoryBlockStore` is a deterministic `BTreeMap`-backed implementation for local callers and tests (`crates/storage-prolly-trees/src/store.rs:61`).
* `BlockStore::insert` and `BlockStore::load` check encoded bytes with `verify_stored_node` before accepting or returning node material (`crates/storage-prolly-trees/src/store.rs:36`, `crates/storage-prolly-trees/src/store.rs:48`, `crates/storage-prolly-trees/src/store.rs:96`).
* `ProllyTree::build` writes every carried encoded node to a caller-provided `BlockStore`; `ProllyTree::open` verifies only that a named root node is present and hash-valid (`crates/storage-prolly-trees/src/tree.rs:53`, `crates/storage-prolly-trees/src/tree.rs:82`).

### Decision (PB-ADR-0004)

The current storage boundary is limited to encoded node bytes keyed by BLAKE3 identity.
The crate provides a deterministic in-memory store and a narrow `BlockStore` trait, but it does not treat a store as proof verification context or as a persistent database abstraction.

### Rationale (PB-ADR-0004)

* A narrow store contract keeps storage validation local: stored bytes must hash to the requested identity and decode as canonical node material.
* Keeping proof production and verification on `PortableProofTree` avoids duplicating private decoders in store-backed traversal.
* `OpenedProllyTree` exposes only root-presence verification so callers do not mistake a partial store check for full tree reconstruction.

### Trade-offs (PB-ADR-0004)

* Store-backed lookup, range traversal, and proof generation are not `current` API behavior.
* Persistent backend policy remains outside this crate until traversal, failure, and adapter semantics are specified.

## PB-ADR-0005: Adapter exclusions from Prolly-Bao core

### Status (PB-ADR-0005)

`current`

### Context (PB-ADR-0005)

* The crate root documents that SQL, `DataFusion`, `Iroh`, `Git`, `Automerge`, `IPLD/CAR`, filesystem paths, networks, and storage engines stay outside the core public API.
* `CHANGELOG.md` records that runtime public APIs do not expose `fastcdc`, `chunk`, semantic text chunkers, SQL/DataFusion, Iroh, Git, Automerge, IPLD/CAR, RocksDB/redb/sled/fjall, async runtimes, filesystem paths, network handles, database schemas, Python, or vector/text search.
* `METRICS.md` records behavior-test evidence for Bao verification of canonical snapshot bytes, but no performance claims for Bao verification, Bao transport, Iroh transport, IPLD/CAR import/export, SQL/DataFusion, persistent stores, network behavior, async runtimes, filesystem paths, Python, Git, Automerge, or text/vector search.

### Decision (PB-ADR-0005)

Prolly-Bao core excludes Bao proof semantics and transport coupling, Iroh transport, IPLD/CAR import/export, SQL/DataFusion relational execution, and persistent store backend semantics from the current implementation.
Canonical snapshot bytes are current Prolly-Bao materialization, but external transport, exchange, and relational concerns must remain adapter-layer work with their own root/proof mapping, traversal, and failure-mode decisions.

### Rationale (PB-ADR-0005)

* Core roots and proofs are currently defined by ordered-record tree semantics, BLAKE3 node identity, committed parameters, and explicit verifier inputs.
* Transport, exchange, relational, and persistent backend formats each add semantics that can change membership, traversal, failure, or compatibility claims if mixed into core prematurely.
* Excluding adapters prevents documentation or API consumers from treating designed systems as shipped behavior.

### Trade-offs (PB-ADR-0005)

* Integrations that need Bao transport, Iroh, IPLD/CAR, SQL/DataFusion, or persistent stores must wait for separate adapter decisions.
* The core remains smaller and more auditable, but callers do not get database, transport, or exchange-format convenience APIs from this crate.

## PB-ADR-0006: Evocative Bao-inspired naming and native witness streams

### Status (PB-ADR-0006)

`current`

### Context (PB-ADR-0006)

* Project docs now define `Prolly-Bao` as an evocative name for Bao-inspired verified streaming over ordered-record Prolly trees (`CONTEXT.md:46`).
* `docs/analysis/storage-prolly-trees/prolly-bao-implementation.md` records that native Prolly-Bao witnesses verify ordered-map semantics under an agreed root, not byte offsets under a Bao blob hash.
* Bao verification is now adapter/dev evidence over canonical Prolly-Bao snapshot bytes.
  Iroh remains an adapter surface for large value blobs, cacheable witness-response blobs, and byte-transport validation.

### Decision (PB-ADR-0006)

Keep `storage-prolly-trees` as the crate name because it communicates the design inspiration: root-first verification, streamable authenticated fragments, and fail-closed client validation.
Treat native Prolly-Bao witness streams as the core proof-response model.
Do not call native witnesses Bao proofs; canonical snapshot byte verification is separate adapter/dev evidence.

### Rationale (PB-ADR-0006)

* The name records the conceptual origin without forcing the core to pretend it is a flat byte-stream verifier.
* A native witness stream is the right proof target for ordered-map facts: membership, non-membership, and range completeness under a Prolly-Bao root.
* Bao/Iroh compatibility is still useful, but primarily as adapter behavior for blobs, snapshots, response caching, or transport validation.

### Trade-offs (PB-ADR-0006)

* The name is intentionally less technically precise than names like `prolly-blake3` or `prolly-kv`.
* Documentation must repeatedly state that native Prolly-Bao witnesses are not Bao byte-stream proofs.
* Adapter tasks must report what Bao/Iroh verifies without promoting adapter evidence into a core proof-format claim.

## PB-ADR-0007: Versioned native witness transcripts reuse proof verifiers

### Status (PB-ADR-0007)

`current`

### Context (PB-ADR-0007)

* Native witness transcripts serialize membership, non-membership, and range query responses under an agreed Prolly-Bao `TreeRoot` and `TreeParams`.
* The transcript is versioned independently from node encoding.
  The current header uses the `prolly-bao:witness:v1` domain bytes followed by a two-byte big-endian witness version.
* Transcript payloads carry the proof kind, root context, chunker parameter bytes, query key or range, returned membership value or range records, non-membership evidence, root-node hash, and proof nodes.
* Transcript bytes terminate with an explicit deterministic `WitnessEndSummary` that binds version, kind, root hash, root record count, chunker-parameter digest, root-node hash, a body digest covering query/result material, proof-node count, proof-nodes digest, and a binding digest over the summary fields.
* Existing membership, non-membership, and range proof verifiers already define the fail-closed tree semantics for root context, query binding, BLAKE3 node identity, canonical node decoding, returned records, and adjacent absence evidence.

### Decision (PB-ADR-0007)

Keep native witness verification as a serialization layer over the existing proof verifier contract.
`WitnessTranscript` decodes a versioned transcript, validates the terminal end summary, reconstructs the corresponding proof statement, and calls the membership, non-membership, or range verifier instead of duplicating Merkle-search-tree verification logic.

### Rationale (PB-ADR-0007)

* Reusing proof verifiers keeps the authoritative root/query/tree validation in one place.
* Versioning the transcript separately lets the wire shape evolve without relabeling existing node/root encoding versions.
* Keeping the transcript native to Prolly-Bao preserves ordered-record semantics and avoids implying Bao byte-stream compatibility.

### Trade-offs (PB-ADR-0007)

* The current transcript can be encoded to bytes and decoded fail-closed, but it still carries proof-node and record material needed by the current proof API.
* The end summary is redundant by design: it gives decoders a terminal statement/root/parameter/body/node consistency check before verifier dispatch without replacing the proof verifier contract.
* Further allocation reduction or incremental decode surfaces are optimization work, not required for the current native transcript contract.
* Bao verification of canonical snapshot bytes is adapter evidence only.
  Iroh transport and large value blob handling remain adapter work outside this decision.

## PB-ADR-0008: Canonical snapshot bytes and Bao verifier evidence

### Status (PB-ADR-0008)

`current`

### Context (PB-ADR-0008)

* `PortableProofTree::encode_snapshot_bytes` and `PortableProofTree::to_snapshot_bytes` materialize deterministic canonical snapshot bytes for the complete ordered-record tree.
* `ProllyTree::encode_snapshot_bytes` and `ProllyTree::to_snapshot_bytes` delegate to the carried portable proof tree.
* `verify_snapshot_bytes` parses a versioned snapshot stream and verifies it against caller-supplied `TreeRoot` and `TreeParams`.
* The version-1 snapshot byte stream uses the `prolly-bao:snapshot:v1` domain, two-byte big-endian snapshot version `1`, root hash, root record count, length-prefixed chunker parameter bytes, root-node hash, exact record count, and exact key/value length-prefixed records in decoded key order.
* The `bao` crate is present only as a `crates/storage-prolly-trees` dev-dependency for contract tests, selected through the root workspace dependency table with `default-features = false`.

### Decision (PB-ADR-0008)

Canonical snapshot bytes are `current` Prolly-Bao materialization for complete ordered-record snapshots.
Bao verification of those bytes is valid adapter/dev-test evidence that the emitted bytes can be treated as a flat byte stream by Bao, but it does not change the semantic Prolly-Bao verifier anchor: ordered-record facts remain verified under `TreeRoot`, `TreeParams`, and native proof or witness APIs.

### Rationale (PB-ADR-0008)

* Snapshot bytes give adapters a deterministic byte stream without relabeling native ordered-record witnesses as Bao proofs.
* `verify_snapshot_bytes` keeps Prolly-Bao semantics explicit by rebuilding and checking the snapshot against the expected root and parameters.
* Keeping `bao` in dev-dependencies prevents a byte-stream verifier crate from becoming a core runtime coupling before a reviewed adapter decision requires it.

### Trade-offs (PB-ADR-0008)

* Bao can verify the exact emitted snapshot bytes, but Bao verification alone proves a flat byte stream against a Bao hash, not membership, non-membership, or range completeness under a Prolly-Bao root.
* The snapshot format currently materializes the full ordered-record snapshot; partial byte-range adapters, offset indexes, Iroh transport, IPLD/CAR mapping, persistent backends, and SQL/DataFusion projections remain outside this decision.

## PB-ADR-0009: Minimal Iroh interop remains an adapter open decision

### Status (PB-ADR-0009)

`open decision`

### Context (PB-ADR-0009)

* The workspace declares `iroh = { version = "1.0.0-rc.0", default-features = false }`, but the inspected crate is peer-to-peer QUIC connectivity: `Endpoint`, endpoint addresses/keys, relay/address lookup, protocol routing, and QUIC streams.
* `iroh` 1.0.0-rc.0 does not expose a local blob store or local storage verifier API suitable for writing, addressing, reading back, and verifying canonical Prolly-Bao bytes in this core crate.
* `iroh-blobs` 0.101.0 is version-compatible with `iroh` 1.0.0-rc.0 and exposes blob/store concepts, but it is a broad protocol/storage crate with BLAKE3 verified-stream, store, protocol, ticket, RPC, and Iroh coupling.
  Its default features include `fs-store` and `rpc`; even with defaults disabled, the crate is adapter-shaped rather than a narrow core test utility.
* `iroh-blobs` 0.102.0 has already moved to `iroh` 1.0.0-rc.1, so the current line also carries near-term RC version-coupling risk for Mach's pinned workspace dependency.

### Decision (PB-ADR-0009)

Do not add `iroh` or `iroh-blobs` to `crates/storage-prolly-trees` for this slice.
Treat minimal Iroh interop as an `open decision` adapter concern.
The core crate's current evidence is limited to deterministic, exact-byte cacheability of canonical snapshot bytes and native witness bytes under flat BLAKE3 content hashes, followed by Prolly-Bao root-bound verification after readback from a local test cache.

### Rationale (PB-ADR-0009)

* Adding Iroh endpoint, relay, router, or stream types to the core public API would couple Prolly-Bao to transport semantics that the crate deliberately excludes.
* A local cacheability test checks the narrow prerequisite adapters need: snapshot/witness bytes are deterministic and can be addressed by content hash, but their semantic validity still depends on `TreeRoot`, `TreeParams`, and native verification.
* Rejecting `iroh-blobs` here avoids turning a storage/protocol adapter into a core dev dependency before a reviewed adapter profile specifies storage format, outboard/verified-stream behavior, network service expectations, and failure modes.

### Trade-offs (PB-ADR-0009)

* This does not prove Iroh compatibility.
  It records why the current workspace `iroh` dependency is the wrong API shape for local blob storage and why `iroh-blobs` remains future adapter evidence rather than current core scope.
* A future adapter may still use `iroh-blobs`, canonical snapshots, large value blobs, or cacheable witness-response blobs after dependency footprint, protocol mapping, root binding, and supply-chain verification are reviewed.

## PB-ADR-0010: Encoded-node layout inspection preserves canonical bytes

### Status (PB-ADR-0010)

`current`

### Context (PB-ADR-0010)

* The crate now exports `inspect_encoded_node`, `EncodedNodeKind`, and `EncodedNodeLayout` for callers that need to classify existing encoded node bytes and inspect their layout metadata.
* The layout surface works over canonical encoded node bytes that are already the input to BLAKE3 node identity.
* `crates/storage-prolly-trees/tests/prolly_bao_node_layout.rs` covers successful leaf and internal layout inspection and fail-closed rejection of malformed or unsupported node bytes.

### Decision (PB-ADR-0010)

Expose encoded-node layout inspection as a read-only introspection surface over current canonical node bytes.
The API reports node kind and layout details; it does not introduce a second node encoding, alternate identity function, or caller-controlled decoding policy.

### Rationale (PB-ADR-0010)

* Adapter and prototype work need stable visibility into encoded-node shape without copying private decoder logic into storage backends.
* Keeping inspection read-only preserves the existing invariant that BLAKE3 identity is computed over canonical encoded node bytes.
* Fail-closed inspection errors prevent adapters from treating malformed bytes as valid layout metadata.

### Trade-offs (PB-ADR-0010)

* The layout API is an inspection aid, not a generalized mutable node builder.
* Offset-table and segment-index ideas can use this surface as review input, but changing canonical encoding remains a separate architecture decision.

## PB-ADR-0011: Packed segment prototype stays behind BlockStore

### Status (PB-ADR-0011)

`current`

### Context (PB-ADR-0011)

* The crate now exports the in-memory `PackedSegmentStore` prototype and `NodeSegmentEntry`.
* The public storage contract remains `BlockStore`: callers insert and load canonical encoded nodes keyed by `NodeHash`.
* `crates/storage-prolly-trees/tests/prolly_bao_store_adapters.rs` covers successful packed-segment store behavior and fail-closed rejection of missing, malformed, or hash-mismatched node material.

### Decision (PB-ADR-0011)

Keep packed segment storage as a current in-memory prototype that adapts to the unchanged encoded-node `BlockStore` boundary.
It may group node bytes into segment entries for prototype storage behavior, but it does not make persistent backend selection, LMDB policy, filesystem layout, or segment format stability a current core guarantee.

### Rationale (PB-ADR-0011)

* Reusing `BlockStore` keeps proof and tree code coupled only to BLAKE3-keyed canonical node bytes.
* An in-memory prototype is enough to evaluate packed layout ideas without committing to durability, compaction, mmap, locking, or crash-recovery semantics.
* Fail-closed adapter tests preserve the existing storage invariant: bytes must decode canonically and hash to the requested identity before being returned.

### Trade-offs (PB-ADR-0011)

* Persistent backend selection remains an `open decision`.
* LMDB, packed-file, and Dolt-style offset-table ideas remain adapter/prototype review input rather than current supported backend behavior.

## designed direction

* Generalized compact witnesses can extend the current compact one-level proof material to deeper tree shapes while preserving the verifier contract above.
* Store-backed traversal can be added once public traversal helpers can load and verify reachable nodes without duplicating private decoder logic.
* Adapter profiles can translate between Prolly-Bao roots/proofs and external systems only after their mapping, traversal, and failure semantics are recorded separately.
* Dolt-style offset-table ideas remain review input for segment and adapter design; they are not current Prolly-Bao node encoding or proof semantics.
* LMDB and packed persistent stores remain adapter/prototype work behind the encoded-node store boundary.
* Native witnesses remain Prolly-Bao ordered-record proof-response material, not Bao byte-stream proofs.

## open decision

* Persistent backend choice.
* Whether `PackedSegmentStore`, an LMDB adapter, or another backend should become a supported persistent store.
* IPLD/CAR adapter profile.
* Iroh wire strategy.
* Relational adapter shape for SQL/DataFusion use cases.
* Version graph ownership.
* Multi-writer merge semantics.
* Regression threshold, allocation-count baseline, hardware-normalized threshold, and compact-proof-size target.
* Whether source-file-like `target-x3` balanced-compromise evidence should change a future mixed-workload planner recommendation; it is policy input only and does not change the current default tree profile.
* Whether generalized multi-level compact proof selection should be opened as a separate implementation task.
