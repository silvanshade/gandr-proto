# Architecture Decision Record

This crate-scoped ADR records durable architecture decisions for `storage-chunker`.
It documents architecture changes/refinements, not a per-task diary.

Source anchors used for this document:

* `crates/storage-chunker/src/lib.rs:4`: deterministic record-safe chunk boundary detection for future Prolly-Bao.
* `crates/storage-chunker/src/lib.rs:6`: the crate owns boundary detection over already-canonical ordered records only.
* `crates/storage-chunker/src/lib.rs:11`: callers are expected to commit `ChunkerParams::commitment_bytes` into Prolly-Bao root/proof context as a designed direction outside this crate.
* `crates/storage-chunker/src/lib.rs:15`: stronger adversarial boundary-grinding mitigations are an open decision.
* `crates/storage-chunker/docs/CHANGELOG.md:38`: the scanner is Mach-local and FastCDC-inspired.
* `crates/storage-chunker/docs/METRICS.md:147`: Criterion and `fastcdc` are dev-only benchmark dependencies.
* `crates/storage-chunker/benches/chunker.rs:101`: deterministic prior-art candidate row labels for the benchmark-only comparison surface.

## ADR-001: Keep boundary detection Mach-local and record-safe

**Status:** current

### Context (ADR-001)

Prolly-Bao needs deterministic content-defined boundaries for canonical ordered records, but generic byte-stream CDC libraries do not know Mach's canonical record boundaries or commitment requirements.

### Decision (ADR-001)

`storage-chunker` implements a Mach-local, deterministic, FastCDC-2020-inspired Gear scanner.
The crate returns record-safe boundary spans and boundary reasons for already-canonical ordered records.
It does not delegate runtime boundary semantics to `fastcdc`, `chunk`, semantic text chunkers, storage layers, databases, async runtimes, transport adapters, or exchange formats.

### Rationale (ADR-001)

Keeping the scanner local makes record-boundary behavior, parameter validation, commitment byte layout, and failure modes reviewable in this repository.
It also prevents runtime behavior from drifting with a generic dependency whose byte stream model is not Mach's record-safe model.

### Trade-offs (ADR-001)

* Mach owns the maintenance burden for the scanner and its tests.
* Throughput comparisons against generic CDC crates are useful context but are not semantic equivalence evidence.
* Optimization work must preserve record-safe behavior rather than blindly matching raw byte-stream CDC behavior.

## ADR-002: Make chunker parameters runtime-validated and committed

**Status:** current

### Context (ADR-002)

Boundary output depends on algorithm version, Gear table version, seed policy, normalization policy, record-boundary rule, and byte/record limits.
Replaying the same canonical records under incompatible parameters would produce different boundaries.

### Decision (ADR-002)

The public parameter surface is represented by `ChunkerParams` and `ChunkLimits`.
Construction validates supported algorithm/table/policy values and zero, inverted, or capped limits before scanning.
Valid parameters produce stable fixed-order commitment bytes through `ChunkerParams::commitment_bytes`.

The current supported profile is `AlgorithmVersion::FASTCDC_2020` with `GearTableVersion::MACH_V1`, `NormalizationPolicy::NONE`, and `RecordBoundaryRule::BETWEEN_RECORDS`.
Seed policy supports deterministic no-salt operation and caller-provided public salt.

### Rationale (ADR-002)

Treating parameters as committed data makes the boundary contract explicit and lets downstream Prolly-Bao code bind its own root/proof context to the chunking configuration without moving proof or node identity behavior into this crate.
Runtime validation keeps unsupported profiles and invalid limits fail-closed.

### Trade-offs (ADR-002)

* Parameter construction is stricter than a loose configuration struct.
* Adding a new algorithm, table, normalization policy, seed kind, or boundary rule requires an explicit committed value and validation path.
* This crate exposes commitment bytes but does not decide how a downstream proof layer binds them.

## ADR-003: Cut only between complete canonical records

**Status:** current

### Context (ADR-003)

The input model is already-canonical ordered records.
Splitting inside one canonical record would make boundary output cheaper to align with hash predicates, but it would violate the record-safe contract needed by consumers.

### Decision (ADR-003)

The scanner consumes one complete canonical record at a time and may emit a boundary only after that complete record.
Hash-predicate cuts, maximum-byte-cap cuts, maximum-record-cap cuts, and final-remainder cuts all produce `ChunkSpan` values whose byte and record spans end at record boundaries.

The public entry points reflect the two supported input shapes:

* `chunk_record_slices` accepts borrowed canonical record slices.
* `chunk_spans` accepts one contiguous canonical byte stream plus record spans that must form an exact, contiguous partition.

### Rationale (ADR-003)

A between-record-only rule preserves canonical record integrity and makes the returned byte spans and record spans describe the same chunks.
It also keeps payload copying out of the scanner: the crate computes boundaries and returns spans rather than materializing payload chunks.

### Trade-offs (ADR-003)

* A hash predicate that fires inside a record is delayed until a complete-record boundary.
* A single record larger than the hard byte cap is an error rather than a split.
* If cap pressure conflicts with minimum byte/record limits, the scanner returns a precise error instead of silently emitting an unsafe boundary.

## ADR-004: Treat Gear hashes as non-cryptographic boundary metadata

**Status:** current

### Context (ADR-004)

The scanner uses a local Gear table and rolling state to decide candidate content-defined boundaries.
That state is suitable for deterministic boundary selection, not for identity or integrity.

### Decision (ADR-004)

Gear values and Gear-derived predicates are non-cryptographic boundary metadata only.
They are not BLAKE3 identity, Bao proof material, Merkle proof material, block identity, storage identity, or tamper-evidence.

### Rationale (ADR-004)

Keeping Gear state out of identity/proof semantics prevents callers from mistaking boundary selection metadata for cryptographic evidence.
It also keeps this crate focused on deterministic scanning while leaving integrity and proof semantics to the Prolly-Bao layer.

### Trade-offs (ADR-004)

* `ChunkSpan` output must be paired with proof, identity, or storage systems from other crates when those properties are required.
* Boundary reproducibility is not the same thing as content authenticity.

## ADR-005: Keep `fastcdc` as a dev-only comparator

**Status:** current

### Context (ADR-005)

The crate needs performance context for a FastCDC-inspired scanner, but the available comparator scans raw byte slices and does not enforce Mach's between-record boundary rule.

### Decision (ADR-005)

`fastcdc` is used only as a dev-only Criterion benchmark comparator for raw byte-slice CDC throughput.
It is not a runtime dependency and is not the source of this crate's boundary semantics.

2026-06-05 refinement: the benchmark also includes `mach-record-safe/flattened-spans/...` rows that call the public `chunk_spans` API over one canonical byte buffer plus record spans.
These rows make the Mach scanner/input-layout comparison closer to the raw-byte FastCDC fixture shape without changing the runtime/default profile or treating FastCDC as semantic equivalence evidence.

### Rationale (ADR-005)

A comparator gives maintainers a useful performance reference without importing a generic raw-byte chunking model into the runtime contract.

### Trade-offs (ADR-005)

* Existing comparator rows are not apples-to-apples semantic comparisons against Mach record-safe scanning.
* Benchmark interpretation must remain explicit: raw-byte CDC throughput does not prove Prolly-Bao tree/proof equivalence or record-boundary safety.
* The flattened-span rows are better input-layout evidence than scattered record-slice rows, but they still return Mach `ChunkSpan` metadata and enforce between-record cuts.

## ADR-006: Keep prior-art candidates benchmark-only

**Status:** current

### Context (ADR-006)

The chunker benchmark now includes deterministic prior-art candidate rows for Okra-style complete-record hash-threshold, Dolt-style key-only salted hash/CDF-like size pressure, and hybrid Mach Gear hard-cap profiles.
These rows exercise source-file-like, task-record-like, low-entropy key, fixed-width value update, large-value-reference, and adversarial boundary-seeking records.

### Decision (ADR-006)

Keep the prior-art candidates benchmark-only.
They remain Criterion rows labelled as non-consensus and not Prolly-Bao proof equivalence.
They do not change the runtime/default chunker profile, public committed parameter profile, proof semantics, or node identity semantics.

2026-06-05 refinement: the Okra-style row is now a dev-only unkeyed BLAKE3 complete-record comparator using Okra's reviewed `u32(hash[0..4]) < 2^32 / Q`, `Q = 32` predicate.
This makes the benchmark basis more concrete without selecting Okra as a runtime profile.

### Rationale (ADR-006)

The rows are useful comparison surfaces for reasoning about record-shape and candidate-boundary behavior, but they are stand-ins rather than selected consensus algorithms.
Keeping them out of the runtime contract avoids accidentally making exploratory benchmark candidates part of Prolly-Bao proof equivalence.

### Trade-offs (ADR-006)

* Benchmark readers must keep candidate rows separate from implemented runtime behavior.
* Selecting any candidate for runtime use still requires a separate architecture decision and committed profile values.

## ADR-007: Exclude proof, identity, storage, and adapter concerns

**Status:** current

### Context (ADR-007)

Boundary detection is only one part of the Prolly-Bao design.
BLAKE3 identity, Bao/Merkle proof verification, block storage, IPLD/CAR, Iroh, Git, Automerge, databases, and async runtimes have separate semantics and failure modes.

### Decision (ADR-007)

`storage-chunker` excludes BLAKE3 node/root identity, Bao proof generation or verification, Merkle proof behavior, block storage, Prolly-Bao tree construction, serialization of canonical records, and transport/storage adapters.

### Rationale (ADR-007)

A narrow crate boundary makes the scanner easier to audit and prevents the first chunking implementation from becoming the owner of proof, storage, transport, or canonicalization semantics.
Downstream crates can commit the chunker parameter bytes into their own contexts without making this crate a proof system.

### Trade-offs (ADR-007)

* Consumers must compose this crate with separate identity/proof/storage layers.
* This crate cannot by itself answer whether a chunk belongs to a Prolly-Bao proof, root, blockstore, CAR, or transport exchange.
