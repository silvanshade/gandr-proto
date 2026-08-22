# storage-artifact

> **Origin.** This crate (`gandr-storage-artifact`) is gandr-native — the outer-layer CAS wiring and manifest identity for kernel export artifacts (B2.3), not an absorption from another tree.
> It is a `storage-*` tier crate: **untrusted plumbing** by the kernel-boundary naming rule (only `gandr-kernel-*` is trusted).
> Design of record: the massive-term design study §6 — the layered-both CAS decision, the record model, the artifact identity, the two-wall discipline.
> That study left this repository with the research corpus; what it ratified is stated in this crate's own contracts, and git history holds the study.

`storage-artifact` is the consumer layer that turns a kernel v1 export artifact into a content-addressed, canonically-identified object.

## current

- A v1 export artifact **is**, by construction, a sorted unique keyed record set: E2 admission ordering keys each declaration by its admission index, and the format is declaration-segmented.
  Records are `(admission index as a fixed-width big-endian key → declaration segment bytes)`.
- `ArtifactRecordSet` extracts those records from a `SegmentedArtifact` (or directly from an `Environment`) using its header-plus-segment span reader, and reassembles the canonical artifact bytes from them.
  The spans are offsets only; they do not make declaration records independently replayable.
- `build` flows the records through record-safe (declaration-granular) chunking into a `BlockStore`-backed prolly tree, storing every node, and mints the artifact identity.
- `ArtifactManifest` is the canonical, versioned outer manifest binding the chunker parameter commitment (93 bytes), the record count, the root node hash, and the inner kernel export format version.
  `ArtifactIdentity` is `BLAKE3` of the manifest — the b3sum-provenance successor.

## Relationship to the tiers below and beside it

- **`gandr-storage-chunker`** — the record-safe boundary detector and its 93-byte versioned parameter commitment.
- **`gandr-storage-prolly-trees`** — the generic ordered-record Merkle tree and its `BlockStore`; it carries **no** declaration semantics, and this crate is a consumer supplying the record model to its generic sorted-record interface.

## The two walls

Integrity never substitutes validity.

- The manifest identity is the **outer** wall: it addresses and authenticates bytes.
  It binds the inner format version so the identity commits to _which_ canonical inner encoding the records carry, but it does not re-check them.
- K2/E3 replay (`gandr_kernel_core::read`) is the **inner** wall and the sole validity authority: it re-derives every typing and well-formedness obligation from the canonical inner bytes.
- A matching identity proves provenance, never validity; the hash is untrusted plumbing.
  This crate changes **no** replay semantics.

## Canonicality is a stated property

The record set is canonical by construction — sorted and unique by admission key — and the prolly tree is a deterministic function of the sorted set.
So the artifact identity is **history-independent**: a permuted build or insertion order yields the identical root and identity.
This is a tested claim (the history-independence differential), not an accident.

A declaration segment may reference subterm entries an earlier segment introduced (cross-declaration sharing), so a record is a content-addressing grain, not an independently replayable unit: replay is whole-artifact over the reassembled bytes.

## The second plane: single massive values

The sections above describe the **keyed plane** — a sorted keyed record set, chunked between records.
A single massive value has no keys to cut between, so it takes the other grain of the same discipline: cuts between **constructors**, with the value's own type as its index and no B-tree beneath it.
That plane lives under `value`.

- `ContentPtr` is the whole address vocabulary: a 32-byte BLAKE3 chunk digest plus a token offset **within that chunk**.
  Nothing about the process, the arena, the insertion order, or the producing machine enters it.
- `cam_commit` walks a value once in preorder and cuts at the constructor exits the committed typed profile chooses, framing each cut suffix as a chunk and splicing a chunk wrapper into the parent in its place.
  Cutting at _exit_ is what makes bottom-up order available: a child's digest is fixed before its parent's body can name it.
- `cam_deref` fetches, verifies and decodes, in that order, splicing child chunks as it crosses seams.
- `ValueProfile` collects every protocol constant two deployments must agree on to share storage, and `ValueManifest` names one committed value under it, so a deployment that disagrees on any of them refuses rather than silently failing to deduplicate.

**The rule the profile is built from**: wherever a choice changes the addresses but no round trip can see it, the manifest is where a disagreeing consumer is made to refuse.
Two deployments differing on such a choice both commit correctly, both dereference correctly, and share nothing, with nothing anywhere telling either of them why — and no test inside one deployment can catch it, because inside one deployment everything works.
Applying that rule found three constants beyond the obvious ones: the **boundary classification** (the export tag table carries two verdict columns, and which one decides cut candidacy changes every chunk at fixed kappa), the **chunk frame version** (the framed preimage is what is hashed), and the **sharing policy** (splicing a repeated subtree as a wrapper versus re-emitting it inline changes the parent body, and so every digest above it).

**The value plane does not ride `BlockStore`.** That trait verifies on both insert and load that its bytes decode as canonical prolly-node material, and a value chunk is not node material.
`value::ChunkStore` is a sibling trait carrying the same verify-on-both-sides rule over a different body; one object may implement both, which is how a single store serves both planes.
Confusion between the two bodies is impossible by construction rather than by convention: a chunk image carries `value::VALUE_CHUNK_MAGIC` inside its own hashed preimage, the same domain-separation rule `transport` applies to step identities.

### planned, not yet implemented

Every `value` body is currently owed; the interface, its contracts, and its contract suite are in place, and each stub carries an expectation attribute that retires itself when the body lands.
Two questions are deliberately left as measurements rather than assumptions:

- **The child index base.** `value::index_base::ChildIndexBase` is a committed mode with two candidates.
  Absolute child indices renumber every downstream chunk under an early insertion, collapsing cross-version sharing to the prefix before the edit; chunk-local bases confine the shift to the edited chunk at the cost of a seam wrapper and an addition per dereference.
  No round trip can tell the two apart, so the choice is settled by `value::index_base::IndexBaseMeasurement` over a real corpus and recorded as an `IndexBaseVerdict` with both measurements beside it.
- **Locality.** `value::locality` records measured chunk counts per edit and reads them against `expected_chunk_bound`.
  The bound is an expectation over the rolling hash rather than a worst case, so what confirms or refutes it is a distribution over a corpus of edits, never a single edit.
  The same run yields the structural-sharing numbers — changed leaves, affected ancestors, hash-equal unchanged subtrees — because they are the same observation counted differently.

### wrong-kind inhabitants, and why the suite is shaped the way it is

Every position on this plane admits a plausible **wrong inhabitant**: a value of the right Rust type standing where a different thing belongs, which round-trips perfectly and reads as success to anything that only checks for an error.
The `value` module map names each one with the witness that separates it, and four tests in `tests/value_contract.rs` are written to **fail** against a specific wrong representation rather than to pass against the right one — a shared subtree stored twice, a prolly node image accepted as a chunk, a word read as a tag, and an interior pointer read as a whole value.
A test that only passes when the representation is right is not evidence about the representation.

### theory it relies on

The boundary discipline, the chunking policy, and the locality result this plane implements are adopted from:

> Michael Rainey, Michael H. Borkowski, Michael Vollmer, Chaitanya S. Koparkar, Mikah Kainen and Vidush Singhal, "LoCalMem: Type-Directed Adaptive Serialization for Location- and Content-Addressable Memory", _Proceedings of the ACM on Programming Languages_ 10 (ICFP), 2026, pages 461–493, doi:10.1145/3828688.

What is taken is the vocabulary and the measure — explicit boundary datatypes, rolling-hash cuts at boundary constructors, and the locality theorem bounding update work by the depth of the modified path rather than by the size of the value.
What is **not** taken is a compilation model: no whole-program monomorphization, no requirement that the checker compute over serialized heaps, and no replacement of the kernel arena.

The prolly-tree literature the keyed plane already stands on is the sibling half of the same idea at a different grain: content-defined chunking between the _keys_ of a sorted record store, where this plane cuts between the _constructors_ of a typed value.

## current limitations and direction

- `current`: identity is produced from an environment in hand (the B2.3 producer path); re-deriving records from opaque foreign bytes (the future B9 replayer) is out of scope here and would use a reader-side framing walk against the format specification, never shared code.
- `designed direction`: the manifest layout is versioned; a layout change bumps `MANIFEST_FORMAT_VERSION_V1` and refreshes the golden.
- `open decision`: the prolly tree's inherited-deferred residuals (multi-level tree, persistent store backend, anti-boundary-grinding) live in `storage-prolly-trees`, not here.
