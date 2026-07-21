# storage-artifact

> **Origin.** This crate (`gandr-storage-artifact`) is gandr-native — the outer-layer CAS wiring and manifest identity for kernel export artifacts (B2.3), not an absorption from another tree.
> It is a `storage-*` tier crate: **untrusted plumbing** by the kernel-boundary naming rule (only `gandr-kernel-*` is trusted).
> Design of record: `docs/research/massive-term-design.md` §6 (the layered-both CAS decision, the record model, the artifact identity, the two-wall discipline).

`storage-artifact` is the consumer layer that turns a kernel v1 export artifact into a content-addressed, canonically-identified object.

## current

* A v1 export artifact **is**, by construction, a sorted unique keyed record set: E2 admission ordering keys each declaration by its admission index, and the format is declaration-segmented.
  Records are `(admission index as a fixed-width big-endian key → declaration segment bytes)`.
* `ArtifactRecordSet` extracts those records from a `SegmentedArtifact` (or directly from an `Environment`), and reassembles the canonical artifact bytes from them.
* `build` flows the records through record-safe (declaration-granular) chunking into a `BlockStore`-backed prolly tree, storing every node, and mints the artifact identity.
* `ArtifactManifest` is the canonical, versioned outer manifest binding the chunker parameter commitment (85 bytes), the record count, the root node hash, and the inner kernel export format version.
  `ArtifactIdentity` is `BLAKE3` of the manifest — the b3sum-provenance successor.

## Relationship to the tiers below and beside it

* **`gandr-kernel-core`** (trusted) — supplies the canonical v1 bytes and their declaration-segment framing (`write_segmented` → `SegmentedArtifact`); it gains **no** dependency and does **no** hashing.
* **`gandr-storage-chunker`** — the record-safe boundary detector and its 85-byte parameter commitment.
* **`gandr-storage-prolly-trees`** — the generic ordered-record Merkle tree and its `BlockStore`; it carries **no** declaration semantics, and this crate is a consumer supplying the record model to its generic sorted-record interface.

## The two walls

Integrity never substitutes validity.

* The manifest identity is the **outer** wall: it addresses and authenticates bytes.
  It binds the inner format version so the identity commits to _which_ canonical inner encoding the records carry, but it does not re-check them.
* K2/E3 replay (`gandr_kernel_core::read`) is the **inner** wall and the sole validity authority: it re-derives every typing and well-formedness obligation from the canonical inner bytes.
* A matching identity proves provenance, never validity; the hash is untrusted plumbing.
  This crate changes **no** replay semantics.

## Canonicality is a stated property

The record set is canonical by construction — sorted and unique by admission key — and the prolly tree is a deterministic function of the sorted set.
So the artifact identity is **history-independent**: a permuted build or insertion order yields the identical root and identity.
This is a tested claim (the history-independence differential), not an accident.

A declaration segment may reference subterm entries an earlier segment introduced (cross-declaration sharing), so a record is a content-addressing grain, not an independently replayable unit: replay is whole-artifact over the reassembled bytes.

## current limitations and direction

* `current`: identity is produced from an environment in hand (the B2.3 producer path); re-deriving records from opaque foreign bytes (the future B9 replayer) is out of scope here and would use a reader-side framing walk against the format specification, never shared code.
* `designed direction`: the manifest layout is versioned; a layout change bumps `MANIFEST_FORMAT_VERSION_V1` and refreshes the golden.
* `open decision`: the prolly tree's inherited-deferred residuals (multi-level tree, persistent store backend, anti-boundary-grinding) live in `storage-prolly-trees`, not here.
