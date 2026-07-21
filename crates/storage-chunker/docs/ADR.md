# Architecture Decision Record

This crate-scoped ADR records durable architecture decisions for `storage-chunker`.
It documents architecture changes/refinements, not a per-task diary.

Source anchors used for this document:

* `crates/storage-chunker/src/lib.rs` module documentation: deterministic record-safe boundary detection over already-canonical ordered records only.
* `AlgorithmVersion::FASTCDC_2020` and `GearTableVersion::MACH_V1`: the only implemented runtime profile.
* `ChunkerParams::commitment_bytes`: stable parameter bytes for downstream root/proof contexts.
* Module `designed direction` / `open decision` documentation: future committed profiles may address stronger adversarial boundary-grinding mitigations.

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

## ADR-005: Keep any `fastcdc` comparator dev-only

**Status:** designed direction

### Context (ADR-005)

The current crate has no benchmark target or benchmark-only development dependencies.
A future FastCDC comparator would scan raw byte slices and would not enforce Mach's between-record boundary rule.

### Decision (ADR-005)

If a FastCDC comparator is added, keep it in an executable development-only benchmark target.
It must not become a runtime dependency or the source of this crate's boundary semantics.
A future benchmark target must include low-entropy and local-edit fixtures, report chunk-count, byte-distribution, cap-hit, and trigger-reason summaries, and keep point-in-time release baselines outside correctness gates until reproducible thresholds are selected.
Fixture serialization and analysis are separate tooling concerns; this crate does not reserve `rkyv` or Arrow dependencies without a concrete need.

### Rationale (ADR-005)

A comparator can give maintainers useful performance context without importing a generic raw-byte chunking model into the runtime contract.

### Trade-offs (ADR-005)

* The current tree provides no FastCDC comparison measurements.
* Raw-byte CDC throughput would not prove Prolly-Bao tree/proof equivalence or record-boundary safety.

## ADR-006: Keep any prior-art candidates benchmark-only

**Status:** designed direction

### Context (ADR-006)

Okra-style complete-record hash thresholds, Dolt-style key-only size pressure, and hybrid Mach Gear hard-cap profiles are possible comparison candidates.
None currently ships as benchmark code or a runtime profile.

### Decision (ADR-006)

If implemented, keep prior-art candidates in executable benchmark targets labelled as non-consensus and not Prolly-Bao proof equivalence.
They must not change the runtime/default chunker profile, public committed parameter profile, proof semantics, or node identity semantics.
Before any candidate becomes a runtime profile, downstream tree construction must compare proof size and edit propagation against the current record-safe profile.
Key-only chunking remains unselected pending that evidence.

### Rationale (ADR-006)

Candidate comparisons can inform record-shape and boundary behavior without accidentally making exploratory algorithms part of Prolly-Bao proof equivalence.

### Trade-offs (ADR-006)

* The current tree provides no candidate-row measurements.
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
