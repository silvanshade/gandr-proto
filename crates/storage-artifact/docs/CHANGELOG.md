# Changelog

## 2026-07-21 - Outer-layer CAS wiring and manifest identity (B2.3)

* `current`: Landed `gandr-storage-artifact`, the gandr-native outer-layer consumer that turns a kernel v1 export artifact into a content-addressed, canonically-identified object (`docs/research/massive-term-design.md` §6).
* `current`: Added the record model — `ArtifactRecord` (`admission index big-endian key → declaration segment bytes`) and `ArtifactRecordSet`, which extracts the sorted unique keyed record set from a `SegmentedArtifact` (or an `Environment`) and reassembles the canonical artifact.
* `current`: Added `build`, which flows records through record-safe declaration-granular chunking into a `BlockStore`-backed `ProllyTree`, stores every carried node, and mints the artifact identity.
* `current`: Added `ArtifactManifest` (canonical, versioned, fixed-order big-endian, self-delimiting, golden-tested) binding the 85-byte chunker parameter commitment, the record count, the root node hash, and the inner kernel export format version; `decode` refuses a bad magic, an unsupported manifest version, truncation, trailing bytes, and a wrong commitment length.
* `current`: Added `ArtifactIdentity` = `BLAKE3` of the canonical manifest bytes — the b3sum-provenance successor; hashing lives here, outside the kernel TCB.
* `current`: Pinned the two-wall discipline in docs — the manifest identity is the outer integrity wall, `gandr_kernel_core::read` (K2/E3 replay) is the inner validity wall, the hash never substitutes validity, and no replay semantics change.
* `current`: Pinned history-independence as a stated property with a differential — a permuted build order yields the identical root and identity.
* `current`: Consumes `gandr_kernel_core::write_segmented` / `SegmentedArtifact`, a minimal structural framing API added to kernel-core (offsets and lengths only, no hashing, no new dependency) so the outer layer never re-parses the format framing.
