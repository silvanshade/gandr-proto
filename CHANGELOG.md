# Changelog

Notable changes to the gandr workspace, newest first.
This is the single workspace changelog; the per-crate `docs/` directories it replaces are leaving the tree, and their salvageable history is preserved here.
Entries dated before 2026-07-21 record the relevant tier's lineage before its absorption into this tree.

## 2026-08-10

* The per-crate `docs/` tier (STATUS, ADR, CHANGELOG, METRICS, OPTIMIZATION) begins its retirement: the `storage-artifact`, `storage-chunker`, and `storage-prolly-trees` sets leave the tree, and this file becomes the workspace changelog of record.
  The design material they carried is corrected at its new home rather than copied.
* The retirement continues: the `kernel-core` and `kernel-strata` sets leave the tree.
* And the `theory-orders` set — its order-maintenance decisions, deferrals, and coverage record are corrected at their new home.

## 2026-07-21

* **kernel-core**: retuned the reader budget constants on real corpus telemetry (the export exit gate over the 21 S1-eligible corpus items and 6 kernel-native goldens): `MAX_EXPANDED_TERM_WORK` `1 << 24 → 1 << 20`, `MAX_TABLE_ENTRIES` `1 << 20 → 1 << 18` — the binding floor is the deepest artifact the kernel itself round-trips (~200k entries, ~400k expanded work), and the export boundary goldens now derive from the constants so a future retune needs no hand-editing.
  Same landing: the kernel-native levelled-universe and explicit-lift goldens, and the artifact-total amplification bound `MAX_ARTIFACT_EXPANDED_WORK` with the deterministic `DecodeMetrics` the export exit gate records.
* **storage-artifact**: landed the outer-layer CAS wiring for kernel v1 export artifacts — the record model over `write_segmented`, declaration-granular chunking into a `BlockStore`-backed prolly tree, the canonical versioned `ArtifactManifest`, and `ArtifactIdentity = BLAKE3(manifest)` as the b3sum-provenance successor, with the two-wall discipline (outer integrity, inner validity) pinned in docs and history-independence pinned by a differential.
* **storage-chunker**: reconciled the absorbed documentation — removed benchmark claims that have no executable source in this tree; the crate ships no benchmark target and an empty dev-dependency table.

## 2026-07-20

* **kernel-core**: minted the S1 core of the minimal certified kernel — the closed polarized CBPV term/type vocabulary, the `Def`/`Axiom` declaration vocabulary with prenex level contexts, the append-only environment with the single `add_decl` choke point (one warned bypass, the print-axioms audit, an unforgeable `CheckedId`), the zero-inference bidirectional checker, and the quarantined conversion — followed by the K5 export: the v1 maximal-sharing subterm-table writer with content-keyed dedup, the total validating reader with expanded-work budgets, the canonical re-encode-compare, and the seven format-plane reservations held empty.
* **kernel-core**: restructured S1 terms into the D1(C) per-environment append-only arena with typed node ids — shallow derived `Clone`/`Drop`/equality, flat teardown — retiring the hand-written iterative `Drop`/`Clone` worklists the owned-tree representation had required for adversarial-depth totality.
* **kernel-core**: defunctionalized the S1 checker and retired the depth budget — the mutually recursive checking, synthesis, and type-formation methods became one machine over goal and produced registers, a heap frame stack, and an explicit context stack, with fail-closed register projections; totality on adversarial depth is structural, and the retired rejection tests inverted into ~200k-deep admission witnesses.

## 2026-07-19

* **kernel-strata**: ported into the tree as the first `gandr-kernel-*` crate (the lineage entries below date from the absorbed implementation).

## 2026-07-13

* **kernel-strata**: initial implementation — the free-fragment level oracle: the always-canonical `Level` over the `{0, +1, max}` algebra (derived equality is the level-equality oracle), the domination-based `leq`/`lt` oracle with checkable witness/refutation evidence and validators, and the checked `eval` semantic anchor.
* **kernel-strata**: landmark posets and entailment — Bezem–Coquand loop-checking (TCS 913, 2022) over declared variable-only constraints, admission as the Corollary 3.5 dichotomy with evidence on both sides (`ConsistencyWitness` homomorphism or replayable `LoopWitness`), Corollary 3.4 entailment with `EntailmentWitness`/`EntailmentCountermodel`, the pinned-bottom constant encoding, and the empty-poset property differential against the free oracle.

## 2026-06-21

* **theory-orders**: initial implementation — the self-contained order-maintenance structure for the incremental pipeline: `OrderMaintenance<T>` (single-level list-labeling, O(1) comparison, O(log² n) amortized insertion with the density-capped relabel keeping the structure total), generation- and structure-checked `Pos` handles, the `Interval` pre/post-order containment query, the reference-oracle property test, and the `order` criterion bench.

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
