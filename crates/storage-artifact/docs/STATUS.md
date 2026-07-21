# Status

Crate scope: `crates/storage-artifact`.

Status vocabulary in this file is limited to `current`, `designed direction`, and `open decision`.

## current

* The crate is the gandr-native outer-layer CAS wiring for kernel v1 export artifacts (B2.3; `docs/research/massive-term-design.md` §6).
  It is a `storage-*` tier crate — untrusted plumbing by the kernel-boundary naming rule.
* The record model treats a v1 export artifact as the sorted unique keyed record set it is by construction: `ArtifactRecord` is `(admission index big-endian key → declaration segment bytes)`, and `ArtifactRecordSet` extracts the records and reassembles the canonical artifact.
* Record extraction consumes `gandr_kernel_core::write_segmented`, whose `SegmentedArtifact` exposes the header and each declaration segment's bytes — offsets and lengths only, no hashing, added to kernel-core as a minimal structural API so the outer layer never re-parses the format framing.
* `build` flows records through record-safe (declaration-granular) chunking into a `BlockStore`-backed `gandr_storage_prolly_trees::ProllyTree`, stores every carried node, and returns a `BuiltArtifact` with the tree root, root node hash, manifest, and identity.
* `ArtifactManifest` is the canonical, versioned outer manifest binding the 85-byte chunker parameter commitment, the record count, the root node hash, and the inner kernel export format version.
  Its byte layout is fixed-order big-endian and self-delimiting; `decode` refuses a bad magic, an unsupported manifest version, a truncated or over-long buffer, and a wrong commitment length.
* `ArtifactIdentity` is `BLAKE3` of the canonical manifest bytes — the b3sum-provenance successor.
  Hashing lives here, outside the kernel TCB.
* The record set is canonical by construction (sorted and unique by admission key), so the identity is history-independent: a permuted build order yields the identical root — a tested differential, not an accident.
* The two-wall discipline is stated where the wiring lives: the manifest identity is the outer integrity wall; `gandr_kernel_core::read` (K2/E3 replay) is the inner validity wall and the sole validity authority; the hash never substitutes validity and this crate changes no replay semantics.
* Tests: manifest layout golden, decode round-trip and refusals (bad magic, unknown manifest version, truncation at every prefix, trailing bytes, bad commitment length), record-set sort/dedup, record-extraction round-trip to a byte-identical artifact, manifest determinism, identity sensitivity to any field perturbation, the history-independence differential, and store/reopen through `InMemoryBlockStore`.

## designed direction

* Re-deriving records from opaque foreign artifact bytes (the future B9 replayer) is a reader-side framing walk against the format specification, never shared code; the producer-side path here holds the environment.
* A layout change to the manifest bumps `MANIFEST_FORMAT_VERSION_V1` and refreshes the golden (the E4/E5 discipline at the outer layer).
* Chunker parameters other than the default FastCDC profile flow through `TreeParams`; the manifest binds whichever commitment the tree carried.

## open decision

* Whether the outer layer should also accept opaque bytes directly (a `SegmentedArtifact`-free re-extraction) once B9 needs it, and where the reader-side framing walk lands.
* The prolly tree's inherited-deferred residuals (multi-level tree, persistent store backend, incremental witness verification, anti-boundary-grinding) are owned by `storage-prolly-trees`, not this crate.
