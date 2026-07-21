# Status

Crate scope: `crates/storage-prolly-trees`.

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

* The crate exposes ordered-record tree, proof, witness transcript, canonical snapshot, encoded-node layout inspection, store, and value APIs through `ProllyTree`, `PortableProofTree`, `MembershipProof`, `NonMembershipProof`, `RangeProof`, `WitnessTranscript`, `WitnessEndSummary`, `WitnessKind`, `verify_snapshot_bytes`, `inspect_encoded_node`, `EncodedNodeKind`, `EncodedNodeLayout`, `BlockStore`, `InMemoryBlockStore`, `PackedSegmentStore`, `NodeSegmentEntry`, `TreeRoot`, `TreeParams`, and `NodeHash`.
* Node identity is opaque BLAKE3 over encoded node bytes.
  Root manifests commit the encoding version, Merkle-search tree kind, BLAKE3 hash marker, first-key separator convention, record count, committed chunker parameter bytes, and root-node hash (`crates/storage-prolly-trees/src/types.rs:175`, `crates/storage-prolly-trees/src/proof.rs:4336`).
* Encoded node layout inspection is `current` through `inspect_encoded_node`, `EncodedNodeKind`, and `EncodedNodeLayout`.
  It inspects existing canonical node bytes and does not change node encoding or BLAKE3 node/root identity.
* Tree construction consumes strictly sorted unique key/value records.
  Unsorted input and duplicate keys return errors instead of selecting a merge policy (`crates/storage-prolly-trees/src/proof.rs:3669`).
* Leaf grouping uses local `storage-chunker` parameter commitments over canonical record bytes.
  Rolling/boundary metadata is not the cryptographic node identity (`crates/storage-prolly-trees/src/types.rs:196`, `crates/storage-prolly-trees/src/proof.rs:1626`).
* Proof verification is store-independent for membership, non-membership, and range proofs.
  Verifiers check explicit root and parameter context, proof kind, query material, node hashes, canonical node decoding, reachability, and returned evidence or records.
* Native witness transcripts serialize membership, non-membership, and range query responses under an agreed `TreeRoot` and `TreeParams`.
  Transcript decode validates the terminal end summary, and transcript verification dispatches to the existing proof verifier methods rather than duplicating Merkle-search-tree verification logic.
* The current witness transcript byte format carries an explicit deterministic end summary binding witness version, witness kind, root hash, root record count, chunker-parameter digest, root-node hash, body digest, proof-node count, proof-nodes digest, and a binding digest over the summary fields.
* Canonical snapshot byte streams are `current` complete ordered-record materializations under an expected `TreeRoot` and `TreeParams`.
  Version 1 uses the `prolly-bao:snapshot:v1` domain, snapshot version `1`, root hash, root record count, length-prefixed chunker parameter bytes, root-node hash, exact record count, and length-prefixed records in decoded key order.
* `PortableProofTree::encode_snapshot_bytes`, `PortableProofTree::to_snapshot_bytes`, `ProllyTree::encode_snapshot_bytes`, and `ProllyTree::to_snapshot_bytes` emit canonical snapshot bytes.
  `verify_snapshot_bytes` parses and rebuilds those bytes against the expected root and parameters.
* Snapshot contract tests use `bao::encode::encode` and `bao::decode::decode` with the returned Bao hash to verify the exact emitted snapshot bytes.
  This is adapter/dev evidence over bytes, not native Prolly-Bao proof semantics.
* Rejection-focused Iroh interop tests in `crates/storage-prolly-trees/tests/prolly_bao_iroh_interop.rs` assert that canonical snapshot bytes and native witness bytes are deterministic, exact-byte cacheable under flat BLAKE3 content hashes, and still require Prolly-Bao root/parameter verification after local readback.
* Those tests deliberately do not import Iroh.
  The inspected workspace `iroh` 1.0.0-rc.0 API is peer-to-peer QUIC endpoint/relay/address lookup/protocol routing/streaming, not a local blob storage verifier.
  No `iroh-blobs` dependency was added for this core slice.
* Witness transcript contract tests cover encode/decode round trips, successful membership/non-membership/range verification, proof `root_node_hash` getters, public end-summary getters, wrong root/query rejection, tampered node bytes, malformed evidence or bounds, duplicate and reordered range records, unsupported witness versions, truncated transcript bytes, and missing, truncated, tampered, or mismatched end-summary bytes (`crates/storage-prolly-trees/tests/prolly_bao_contract.rs`).
* `current` membership proofs carry root/path nodes.
  `current` non-membership proofs carry the root, selected leaf, and optional adjacent successor leaf needed by the current one-level internal-root shape.
  `current` range proofs carry the root plus only the contiguous selected leaf interval needed by the requested range for that same tree shape.
* `BlockStore` is a narrow encoded-node boundary.
  `InMemoryBlockStore` stores nodes in a deterministic `BTreeMap`, and insert/load paths verify stored bytes before accepting or returning them (`crates/storage-prolly-trees/src/store.rs:29`, `crates/storage-prolly-trees/src/store.rs:61`, `crates/storage-prolly-trees/src/store.rs:96`).
* `PackedSegmentStore` is a `current` exported in-memory prototype over `NodeSegmentEntry` records behind the unchanged `BlockStore` boundary.
  It is not a persistent backend selection or stable on-disk segment format.
* `ProllyTree::open` verifies only that a supplied root-node hash is present and hash-valid in a store.
  It does not reconstruct records or expose store-backed lookup, range, or proof-producing methods (`crates/storage-prolly-trees/src/tree.rs:100`).
* `crates/storage-prolly-trees/tests/prolly_bao_node_layout.rs` covers successful encoded node layout inspection and fail-closed malformed/unsupported-node cases.
* `crates/storage-prolly-trees/tests/prolly_bao_store_adapters.rs` covers successful packed-segment store adapter behavior and fail-closed missing, malformed, or hash-mismatched node material.
* Benchmarks are recorded in `docs/METRICS.md` as local Criterion baselines and profile curves, not generalized performance guarantees or regression thresholds (`crates/storage-prolly-trees/docs/METRICS.md`).
* Runtime public APIs do not expose Bao wire-proof semantics, Iroh transport, IPLD/CAR import/export, SQL/DataFusion execution, persistent backend choices, async runtime behavior, filesystem paths, network handles, Git/Automerge integration, Python, or text/vector search (`crates/storage-prolly-trees/docs/CHANGELOG.md`, `crates/storage-prolly-trees/docs/METRICS.md`).
* The `storage-prolly-trees` name is evocative and Bao-inspired, not a claim that native Prolly-Bao witnesses are Bao proofs.
  Bao verification of canonical snapshot bytes is useful adapter evidence, while native witnesses verify ordered-map facts under an agreed root rather than byte offsets under a Bao blob hash (`crates/storage-prolly-trees/docs/ADR.md`).

## designed direction

* Preserve the explicit verifier contract while replacing full reachable node bytes with compact sibling/path witnesses where that can be proven equivalent.
* Improve native witness transcript streaming and allocation behavior only after measured allocation or throughput evidence justifies changing the current byte round-trip API.
* Treat Dolt-style offset-table ideas as design review input for adapter and packed-segment work, not as current Prolly-Bao encoding, identity, or proof semantics.
* Keep LMDB and packed persistent stores as adapter/prototype work behind `BlockStore` until persistent backend policy is decided.
* Keep native witnesses as Prolly-Bao ordered-record proof-response material, not Bao byte-stream proofs.
* Add store-backed traversal only after public traversal helpers can load all reachable nodes from a root without duplicating private decoder logic.
* Keep exact-byte structural verification outside Prolly-Bao as the acceptance boundary; use hash-equal leaves and ancestors only as localized invalidation or eager-evaluation evidence.
* Keep Bao, Iroh, IPLD/CAR, SQL/DataFusion, and persistent backend work at adapter boundaries as `designed direction`, with separate mapping and failure-mode decisions.
  Current Bao evidence is limited to canonical snapshot bytes in dev tests; likely future routes are large value blobs, cacheable witness-response blobs, byte-transport validation, and reviewed partial-range snapshot adapters.
* Keep any future Iroh route in an adapter profile that specifies exact storage API, verified-stream/outboard behavior, root binding, network-service expectations, and failure modes before adding `iroh-blobs` or exposing Iroh types.
* Refresh metrics through orchestrator-owned benchmark runs and continue tracking build, lookup, range, proof generation, proof verification, proof byte size, store byte count, and local-change structural sharing separately.

## open decision

* Persistent backend choice and traversal policy, including whether LMDB, packed-segment storage, or another store implementation should become a supported adapter.
* Iroh wire/storage strategy.
  The current minimal route is rejected for `crates/storage-prolly-trees` core because `iroh` 1.0.0-rc.0 is transport, not local blob storage; `iroh-blobs` remains adapter evidence and future dependency selection work.
* IPLD/CAR adapter profile.
* Relational adapter shape for SQL/DataFusion use cases.
* Version graph ownership.
* Multi-writer merge semantics.
* Direct Dolt/Bao comparison harness design, if a semantically fair harness is introduced.
* Regression threshold.
* Allocation-count baseline.
* Hardware-normalized threshold.
* Compact-proof-size target.
* Source-file-like `target-x3` balanced-compromise evidence is policy input only; whether it changes a future mixed-workload planner recommendation remains unresolved.
* Whether generalized multi-level compact proof selection should be opened as a separate implementation task.
