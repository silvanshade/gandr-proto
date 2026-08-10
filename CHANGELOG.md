# Changelog

Notable changes to the gandr workspace, newest first.
This is the single workspace changelog; the per-crate `docs/` directories it replaces are leaving the tree, and their salvageable history is preserved here.
Entries dated before 2026-07-21 record the storage tier's lineage before its absorption into this tree.

## 2026-08-10

* The per-crate `docs/` tier (STATUS, ADR, CHANGELOG, METRICS, OPTIMIZATION) begins its retirement: the `storage-artifact`, `storage-chunker`, and `storage-prolly-trees` sets leave the tree, and this file becomes the workspace changelog of record.
  The design material they carried is corrected at its new home rather than copied.

## 2026-07-21

* **storage-artifact**: landed the outer-layer CAS wiring for kernel v1 export artifacts — the record model over `write_segmented`, declaration-granular chunking into a `BlockStore`-backed prolly tree, the canonical versioned `ArtifactManifest`, and `ArtifactIdentity = BLAKE3(manifest)` as the b3sum-provenance successor, with the two-wall discipline (outer integrity, inner validity) pinned in docs and history-independence pinned by a differential.
* **storage-chunker**: reconciled the absorbed documentation — removed benchmark claims that have no executable source in this tree; the crate ships no benchmark target and an empty dev-dependency table.

## 2026-06-05

* **storage-prolly-trees**: compact proof node material — non-membership proofs carry only the root, the selected leaf, and an optional required successor leaf; range proofs carry the root plus the contiguous selected leaf interval; witness transcript verification reuses the compact material and rejects omitted or extra leaves, swapped children, and non-root-first order.
  Added the fixed-width and source-file-like profile-curve benches.
* **storage-prolly-trees**: exported encoded-node layout inspection (`inspect_encoded_node`, `EncodedNodeKind`, `EncodedNodeLayout`) and the in-memory `PackedSegmentStore` prototype, both behind the unchanged encoded-node `BlockStore` boundary.
* **storage-chunker**: scanner hot-path cleanup — `ChunkScan` copies immutable scan-local limits, initial Gear state, and cut mask instead of retaining a parameter borrow; no public API, commitment, or dependency changed.

## 2026-06-02

* **storage-prolly-trees**: canonical snapshot byte-stream bridge — snapshot encode, materialize, and verify entry points with contract coverage that runs the real Bao verifier path over the emitted bytes as adapter evidence in dev tests (`bao` stays dev-only).
* **storage-prolly-trees**: native witness transcript contract — versioned transcripts for membership, non-membership, and range query responses under an agreed `TreeRoot` and `TreeParams`, terminating in a deterministic end summary binding version, kind, root, parameters, body, and proof nodes, verified through the existing proof verifiers rather than a second verification path.
* **storage-prolly-trees**: minimal Iroh interop rejection evidence — canonical snapshot and witness bytes are deterministic and exact-byte cacheable under flat BLAKE3 content hashes, with root-bound verification still required after readback; no Iroh dependency added, because the inspected API is peer-to-peer transport, not local blob storage.
* **storage-prolly-trees**: recorded the naming boundary — the crate name is evocative and Bao-inspired; native witnesses verify ordered-map facts under an agreed root, not byte offsets under a Bao blob hash.
* **storage-prolly-trees**: first proof API slice — store-independent membership, non-membership, and range proofs that recompute BLAKE3 node identity and fail closed on malformed or tampered material; tree construction rejects unsorted input and duplicate keys rather than choosing a merge policy; the thirteen-row local Criterion baseline recorded, with direct Dolt and Bao comparisons deferred for want of a semantically fair harness.
