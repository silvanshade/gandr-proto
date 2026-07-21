# TODO

## current

* Encoded node layout inspection is exported through `inspect_encoded_node`, `EncodedNodeKind`, and `EncodedNodeLayout` over existing canonical node bytes.
  It does not change node encoding or BLAKE3 node/root identity.
* `PackedSegmentStore` and `NodeSegmentEntry` are exported as an in-memory packed-segment prototype behind the unchanged `BlockStore` encoded-node boundary.
* `prolly_bao_node_layout.rs` and `prolly_bao_store_adapters.rs` cover success paths and fail-closed malformed, unsupported, missing, or hash-mismatched node material.
* Native witness transcripts remain Prolly-Bao ordered-record proof-response material under `TreeRoot` and `TreeParams`; they are not Bao byte-stream proofs.
* Canonical snapshot byte materialization and Bao verifier tests remain adapter evidence over bytes, not a change to native proof semantics.
* Current membership, non-membership, range proof, and witness verifier paths use compact node material for the implemented one-level internal-root tree shape.
  Contract tests cover omitted required leaves, extra successor leaves, wrong child order, wrong root-first order, wrong bounds, incomplete returned records, and unsorted returned records.

## designed direction

* Use Dolt-style offset-table ideas as review input for packed segment and adapter design without treating them as current node encoding, identity, or proof behavior.
* Keep LMDB and packed persistent stores as adapter/prototype work behind `BlockStore` until persistence, traversal, and failure-mode semantics are specified.
* Add store-backed traversal only after public helpers can load and verify reachable nodes from a root without duplicating private decoder logic.
* Generalize compact proof material beyond the current one-level internal-root shape only after multi-level proof selection and verifier coverage are specified.
* Keep Bao/Iroh wrapping outside native witness semantics.
  Future adapter work may use large value blobs, cacheable witness-response blobs, byte-transport validation, or reviewed partial snapshot/range adapters.
* Refresh metrics only through orchestrator-owned benchmark runs; do not add benchmark numbers or regression thresholds from documentation-only work.

## open decision

* Persistent backend selection: LMDB, packed-segment storage, another backend, or no supported persistent backend in this crate.
* Persistent traversal policy, including partial stores, corruption handling, compaction, crash recovery, and on-disk segment format stability.
* Iroh wire/storage strategy and whether `iroh-blobs` belongs in an adapter.
* IPLD/CAR adapter profile.
* Relational adapter shape for SQL/DataFusion use cases.
* Version graph ownership.
* Multi-writer merge semantics.
* Direct Dolt/Bao comparison harness design, if a semantically fair harness is introduced.
* Regression threshold.
* Allocation-count baseline.
* Hardware-normalized threshold.
* Compact-proof-size target for generalized or multi-level trees.
* Source-file-like `target-x3` balanced-compromise evidence is policy input only; whether it changes a future mixed-workload planner recommendation remains unresolved.
* Whether generalized multi-level compact proof selection should be opened as a separate implementation task.
