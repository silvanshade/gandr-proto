# Changelog

## 2026-06-05 - Compact proof node material

* `current`: `PortableProofTree::prove_non_membership` now carries only the root, selected leaf, and optional adjacent successor leaf needed by the current one-level internal-root proof shape.
* `current`: `PortableProofTree::prove_range` now carries the root plus only the contiguous selected leaf interval needed by the requested range for the current one-level internal-root proof shape.
* `current`: Witness transcript verification reuses the compact proof material and rejects omitted required leaves, unnecessary successor leaves, swapped compact children, non-root-first node order, wrong range bounds, incomplete returned records, and unsorted returned records.
* `current`: Added `benches/prolly_bao_profile_curve_fixed_width.rs` for the fixed-width value-update profile curve and `benches/prolly_bao_profile_curve_source.rs` for the source-file-like profile curve.
  No default tree profile changed.

## 2026-06-05 - Prolly-Bao prior-art surface adaptation

* `current`: Exported encoded node layout inspection through `inspect_encoded_node`, `EncodedNodeKind`, and `EncodedNodeLayout`.
  The API inspects existing canonical node bytes and does not change node encoding or BLAKE3 node/root identity.
* `current`: Exported the in-memory `PackedSegmentStore` prototype and `NodeSegmentEntry` while keeping the public storage boundary on unchanged encoded-node `BlockStore` semantics.
* `current`: Added `prolly_bao_node_layout.rs` and `prolly_bao_store_adapters.rs` coverage for successful layout/store adapter behavior and fail-closed rejection cases.
* `designed direction`: Dolt-style offset-table ideas remain review input for adapter/prototype work; LMDB and packed stores remain outside persistent backend selection.
* `designed direction`: Native Prolly-Bao witnesses remain ordered-record proof-response material under `TreeRoot` / `TreeParams`; they are not Bao byte-stream proofs.
* `open decision`: Persistent backend choice, including whether an LMDB, packed-segment, or other backend should become a supported adapter, remains unresolved.

## 2026-06-02 - Minimal Iroh interop rejection evidence

* `current`: Added rejection-focused interop contract tests in `crates/storage-prolly-trees/tests/prolly_bao_iroh_interop.rs` that assert canonical snapshot bytes and native witness bytes are deterministic, exact-byte cacheable under flat BLAKE3 content hashes, and still semantically bound to Prolly-Bao `TreeRoot` / `TreeParams`.
* `current`: The tests cover missing local cache entries, wrong content hashes, mutated bytes, truncated bytes, unsupported snapshot/witness versions, and wrong Prolly-Bao root contexts without adding an Iroh dependency or exposing Iroh endpoint/transport types through the core public API.
* `open decision`: Minimal Iroh interop is rejected for now for this core slice: workspace `iroh` 1.0.0-rc.0 exposes peer-to-peer QUIC endpoint, relay, address lookup, routing, and stream APIs, not a local blob/storage verifier; `iroh-blobs` is adapter evidence but is too broad to add here.
* `current`: No production Iroh dependency, `iroh-blobs` dependency, adapter crate, network daemon, peer discovery, IPLD/CAR scope, or native witness-as-Iroh/Bao proof claim was added.

## 2026-06-02 - Canonical snapshot byte-stream bridge tests

* `current`: Added canonical Prolly-Bao snapshot byte materialization coverage for `PortableProofTree::encode_snapshot_bytes`, `PortableProofTree::to_snapshot_bytes`, `ProllyTree::encode_snapshot_bytes`, `ProllyTree::to_snapshot_bytes`, and `verify_snapshot_bytes`.
* `current`: Added `bao` 0.13.1 as a workspace dev-dependency for `crates/storage-prolly-trees` tests with `default-features = false`.
* `current`: Contract tests use the real Bao verifier path `bao::encode::encode` followed by `bao::decode::decode` with the returned Bao hash to verify the exact canonical snapshot bytes emitted by Prolly-Bao.
* `current`: Snapshot verifier tests cover empty and one-record trees, deterministic equivalent rebuild bytes, tampered bytes, wrong roots, unsupported snapshot versions, malformed length prefixes, truncated streams, unsorted records, duplicate keys, native witness transcript bytes, and Bao combined bytes presented as non-snapshot input.
* `current`: Bao verification of snapshot bytes is adapter evidence only.
  Native Prolly-Bao witness transcripts remain ordered-record query-response transcripts under `TreeRoot` and `TreeParams`; they are not Bao proofs.
* `current`: No Iroh, CLI, SQL/DataFusion, Git, Automerge, IPLD/CAR, persistent backend, or transport scope was added.

## 2026-06-02 - Native witness transcript contract

* `current`: Added native Prolly-Bao witness transcript contract coverage for membership, non-membership, and range query responses under an agreed `TreeRoot` and `TreeParams`.
* `current`: Witness transcript round trips cover `encode`, `to_bytes`, `decode`, `WitnessTranscript::end_summary`, proof `root_node_hash` getters, `WitnessEndSummary` getters, and the verifier entry points that reuse the existing proof verification paths.
* `current`: Transcript bytes terminate with an explicit deterministic end summary binding witness version, witness kind, root hash, root record count, chunker-parameter digest, root-node hash, body digest, proof-node count, proof-nodes digest, and a binding digest over the summary fields.
* `current`: Fail-closed contract tests cover wrong expected root, wrong key/value or range, tampered node bytes, malformed non-membership evidence, inconsistent range bounds, duplicate and reordered range records, unsupported witness version, truncated transcript bytes, and missing, truncated, tampered, or mismatched end-summary bytes.
* `current`: Native witness transcripts remain Prolly-Bao ordered-record query-response transcripts.
  They are not Bao byte-stream proofs and do not add Bao or Iroh scope to the core crate.

## 2026-06-02 - Bao-inspired naming and witness-stream docs

* `current`: Recorded that `storage-prolly-trees` is an evocative Bao-inspired name, not a claim of Bao wire-format compatibility.
* `current`: Clarified that native Prolly-Bao witnesses verify ordered-map facts under an agreed root rather than byte offsets under a Bao blob hash.
* `designed direction`: Added native Prolly-Bao witness streams as the next proof transcript direction for incremental membership, non-membership, and range verification.
* `designed direction`: Scoped Bao/Iroh adapter value to large value blobs, canonical snapshot streams, cacheable witness-response blobs, and byte-transport validation.

## 2026-06-02 - Mandatory docs closeout

* `current`: Added mandatory crate documentation coverage for `ADR.md`, `STATUS.md`, and `OPTIMIZATION.md`.
* `current`: Refreshed `TODO.md` so the prior mandatory-doc gap records omitted `ADR.md`, `STATUS.md`, and `TODO.md`.
* `current`: Documentation-only closeout; this entry records no Rust/API behavior changes.

## 2026-06-02 - First proof API slice

* `current`: Added portable membership, non-membership, and range proof API notes for the first Prolly-Bao implementation slice.
* `current`: Proof verification is store-independent: verifiers recompute BLAKE3 node identity from encoded node bytes and fail closed for malformed or tampered proof material.
* `current`: Root and proof context include encoding version, tree kind, BLAKE3 identity marker, separator convention, tree parameters, and committed `storage-chunker` parameter bytes.
* `current`: Tree construction rejects unsorted input and duplicate keys instead of silently choosing a merge policy.
* `current`: Benchmarks are tracked in `benches/gandr_storage_prolly_trees.rs`.
  + Scope: deterministic borrowed fixtures for build, lookup, range, proof operations, and medium throughput rows.
  + Measured baselines are available for all thirteen current Criterion rows.
* `current`: Direct Dolt/Bao Criterion comparisons are intentionally deferred: Dolt is a Go storage-engine Prolly Tree component and Bao is a flat BLAKE3 byte-stream verifier, so neither is a semantically fair Prolly-Bao tree/proof benchmark without a reviewed harness and shared fixtures.
* `current`: Medium throughput benchmarks and metrics treatment:
  + Benchmarks cover build, full-range scan, full-range proof generation, and full-range proof verification.
  + Metrics separate local Criterion baselines from:
    - Generalized performance guarantees.
    - Regression thresholds.
* `current`: Runtime public APIs do not expose `fastcdc`, `chunk`, semantic text chunkers, SQL/DataFusion, Iroh, Git, Automerge, IPLD/CAR, RocksDB/redb/sled/fjall, async runtimes, filesystem paths, network handles, database schemas, Python, or vector/text search.
* `designed direction`: Future proof-size work can replace full reachable node bytes with compact sibling/path witnesses while preserving the same verification contract.
* `designed direction`: Hash-equal unchanged leaves and ancestors can support eager edit evaluation and localized invalidation evidence; exact-byte structural verification remains the acceptance boundary.
* `open decision`: These choices are not committed in this slice:
  + Persistent backend choice.
  + IPLD/CAR adapter profile.
  + Iroh wire strategy.
  + Relational adapter shape.
  + Version graph ownership.
  + Multi-writer merge semantics.
  + Regression threshold.
  + Allocation-count baseline.
  + Hardware-normalized threshold.
  + Compact-proof-size target.
