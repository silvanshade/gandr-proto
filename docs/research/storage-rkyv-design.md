# storage-rkyv — provenance-verified fast (de)serialization of machine states (gandr-1hu)

> **Status: PROPOSAL for owner review — not a decision record.** Design spike `gandr-1hu`, commissioned 2026-07-20; runs alongside the massive-term design pass `gandr-bvf` (soft coupling, no hard dependency) and its findings join that ratification package.
> This document formalizes the coordinator proposal recorded on `gandr-1hu` (2026-07-20) and verifies its rkyv/bytecheck claims against current library documentation.
> It is a formalization plus fact-verification pass — **not** a redesign — and **no crate lands from this spike**: the crate arrives with its first real consumer, against the ratified design.
> **Consumers:** the `gandr-bvf` ratification package (the tree/store API surface it feeds is named in §5), and the future `storage-rkyv` crate whose first users are A2 checkpoints and L-machine state (`gandr-fcw.9`).
>
> **Method and provenance.** The two-wall thesis, the envelope shape, the bytecheck posture, and the trust-tier statement are owner-and-coordinator design decisions read verbatim from the `gandr-1hu` / `gandr-bvf` tracker threads (during the reboot, decisions live in the tracker per `docs/WORKFLOW.md` §"Source of truth"; the per-file `docs/adr/` log is deferred).
> Every rkyv library fact in §7 was checked against current rkyv 0.8.x documentation (docs.rs, the rkyv book, and the FlatBuffers docs for validate-once prior art) on **2026-07-20**; each row carries its source and the two documentation ambiguities found are flagged, not papered over.
> Repo paths are corpus-relative; external facts cite URLs.
> No machine-local paths appear in this file.

## 1. Thesis verdict — AFFIRMED, with five conditions

The owner thesis (`gandr-1hu`): if the machine refers to or internally contains prolly-tree structures, rkyv zero-copy (de)serialization of materialized machine states is appropriate **precisely because** content-addressed provenance verifies the materialized bytes.
The earlier objection — zero-copy access trusts an unvalidated layout — is answered by the **same two-wall discipline** `gandr-bvf` pins for the CAS, applied a second time:

- the **outer, integrity wall** is provenance/hash verification (`NodeHash = BLAKE3(blob)`); it excludes tampering and corruption of the bytes _as written_;
- the **inner, structural-validity wall** is separate, because bytes written honestly may still be layout-invalid if the writer was buggy or wrote under a different schema/format version.

**Verdict: AFFIRMED.** The thesis is sound and the two-wall framing is the correct answer to the historical objection.
It is conditional on five design commitments, each developed below:

1. **Schema + format commitment lives _inside_ the hashed bytes** (§4) — so provenance binds the schema and the rkyv configuration, not merely the payload.
   Without this the outer wall verifies the wrong thing.
2. **bytecheck posture (ii): validate-once-per-`NodeHash` with a store-metadata marker, debug-always differential** (§6) — never hand out `access_unchecked` on a blob that has neither been freshly `bytecheck`-validated nor carries a validity marker.
3. **rkyv stays in the untrusted `storage-*` tier; kernel replay never consumes rkyv states** (§8) — residual risk is engineering robustness, never TCB soundness.
4. **The commitment pins the rkyv crate version-band _and_ the format-control configuration** (§4.2, §7 F7–F8) — resolved below: rkyv publishes no independent wire-format version number, so a gandr-owned identifier pins the configuration these bytes were written under.
5. **State-as-view: no inlining of CAS-resident bulk into rkyv states** (§5) — states hold `NodeHash` references; the bulk stays in `storage-prolly-trees`.

## 2. The one-paragraph architecture

A machine state is serialized as a **compact rkyv struct** that holds `NodeHash` references into the content-addressed store; the bulk it refers to (terms, tables) stays resident as `storage-prolly-trees` blobs and is never copied into the state.
Each serialized blob is `COMMITMENT ‖ rkyv-payload`, and its identity is `NodeHash = BLAKE3(blob)` — the commitment is _inside_ the hashed bytes.
Reading a state is hash-following: fetch a blob by `NodeHash`, verify its integrity (the hash), check its commitment header (cheap, constant-time), then either run `bytecheck` (first materialization) or trust a recorded validity marker (repeat access) and hand back a zero-copy `&Archived<T>`.
The kernel never walks this path; it re-derives everything from canonical bytes (§8).

## 3. Terms and citation conventions

- `NodeHash` — the 32-byte BLAKE3 content address used across the `storage-*` tier; the same identity `storage-prolly-trees` uses for its nodes (`gandr-bvf`).
- `storage-*` — the engineering storage substrate tier ratified on `gandr-bvf` (2026-07-20): content-addressed trees (`storage-prolly-trees`), chunking (`storage-chunker`), and serialization (this document's `storage-rkyv`).
  Untrusted by the kernel-boundary naming rule — only `gandr-kernel-*` is trusted.
- The **85-byte parameter-commitment pattern** — `storage-chunker`'s `ChunkerParams::commitment_bytes()` (magic `MPBCHK01`, fixed-order big-endian fields, salt verbatim, regression-pinned; the mach `prolly-bao-chunker` lift recorded on `gandr-bvf`).
  This spike transplants that pattern from _chunking parameters_ to _serialization schema_.
- rkyv library facts cite docs.rs / the rkyv book by URL in §7; bead IDs cite the tracker directly, the reboot-correct decision surface.

## 4. The envelope — schema commitment inside the hashed bytes

The envelope applies the `storage-chunker` 85-byte commitment pattern to serialization: a fixed-layout, spec-pinned, golden-tested header prefixed to the rkyv payload, entirely inside the hashed bytes.

```text
blob      = COMMITMENT ‖ rkyv_payload
NodeHash  = BLAKE3(blob)               // commitment is INSIDE the hash
```

### 4.1 Field-by-field (draft layout — the invariants are load-bearing; exact widths are RATIFY)

Fields in fixed order, big-endian, mirroring `MPBCHK01`:

| #   | Field              | Width (draft)           | Meaning                                                                                                                               |
| --- | ------------------ | ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `magic`            | 8 bytes                 | Fixed ASCII tag identifying the envelope kind and its layout generation (e.g. `STGRKYV1`); the `MPBCHK01` analogue.                   |
| 2   | `envelope_version` | 1 byte (u8)             | The gandr-owned envelope-layout version; bumped when the commitment layout itself changes.                                            |
| 3   | `rkyv_config_id`   | 2 bytes (u16)           | Gandr-owned monotone id pinning the rkyv **crate version-band + format-control configuration** these bytes were written under (§4.2). |
| 4   | `schema_id`        | 4 bytes (u32)           | Per-state-type identifier, **hand-assigned monotone integer**.                                                                        |
| 5   | `schema_version`   | 2 bytes (u16)           | Monotone per `schema_id`; bumped when that state type's archived layout changes.                                                      |
| 6   | `salt_present`     | 1 byte                  | `0` or `1`; whether a domain-separation salt follows.                                                                                 |
| 7   | `salt`             | 32 bytes (when present) | Verbatim public domain-separation salt, as `storage-chunker` carries its salt.                                                        |

Draft total: 18 bytes without salt, 50 with.
The **exact widths and the magic string are ratification items**; what is load-bearing and non-negotiable is the set of invariants in §4.3.

**`schema_id` / `schema_version` are hand-assigned, never derived.** They must never be a Rust `TypeId`, a `#[derive]`-produced layout hash, or any compiler-minted quantity — those are unstable across toolchains and rustc versions and would silently repartition the content-address space across a compiler upgrade.
Hand-assigned monotone integers are the only stable choice, exactly as the chunker pins its parameters by hand.

### 4.2 `rkyv_config_id` — the wire-format-stability resolution

The coordinator flagged an open question: does the commitment pin the rkyv **crate version** or a **format version**?
This is resolved by the verified stability facts (§7 F7–F8):

- rkyv publishes **no independent wire-format version number**.
  Serialized data is re-accessible only while the schema is unchanged, the format-control features are unchanged, and the reader is a **semver-compatible** rkyv version (F7).
- the byte layout is additionally parameterized by **format-control features** — endianness, alignment, pointer width (F8).

Therefore there is nothing upstream to pin _to_.
The commitment instead pins a **gandr-owned identifier** for the whole rkyv wire configuration: `rkyv_config_id` maps, in a spec-side table, to `{ rkyv semver band, endianness, alignment, pointer width }`.
The team bumps `rkyv_config_id` whenever it changes the rkyv minor (for a `0.8.x` dependency, Cargo's semver-compatible band is the `0.8` minor and `0.8 → 0.9` is breaking) **or** any format-control feature.
A reader whose configured `rkyv_config_id` does not match the blob's refuses **before** bytecheck (§4.3), so a configuration drift is a clean refusal, never a silent misread.

### 4.3 The invariants (spec-pinned, regression-tested like `MPBCHK01`)

1. **The commitment is inside the hashed bytes.** `NodeHash = BLAKE3(COMMITMENT ‖ payload)`, so provenance binds `(magic, envelope_version, rkyv_config_id, schema_id, schema_version, salt)` — the schema and configuration, not just the payload.
2. **Fixed order, big-endian, self-delimiting, golden-tested.** The `storage-chunker` commitment discipline verbatim: any change to layout bumps `envelope_version` and updates the regression golden.
3. **Refuse-before-validate.** A reader with a different `(magic, envelope_version, rkyv_config_id, schema_id, schema_version)` expectation refuses at the header — the E5-refusal posture — before any bytecheck traversal runs.
   The header check is the cheap first stage of the inner structural-validity wall; bytecheck (§6) is the second.
4. **The salt is optional and domain-separating**, carried verbatim, for separating content-address spaces across stores or contexts.

## 5. State-as-view — architecture and the tree/store API it requires

**Affirmed as the default shape.** Machine states are compact rkyv structs holding `NodeHash` references into the CAS; the bulk stays resident in `storage-prolly-trees`.
Rehydration is hash-following with a per-blob integrity verify plus the §6 validity marker.

**Anti-pattern to record:** inlining term trees (or any CAS-resident bulk) into an rkyv state.
It duplicates the CAS, breaks the single-source-of-truth identity, and reintroduces the amplification surface `gandr-bvf` closes at the export layer.
A state is a _view_ over the store, never a copy of it.

**Consequences fed to `gandr-bvf` — the tree/store API must provide:**

- **(a) get-by-hash returning verified blobs.** `get(NodeHash) -> Result<VerifiedBlob, _>` where the store performs (or, via the marker, has already performed) the integrity hash-check and surfaces the validity-marker state.
  The state layer must not be able to obtain unverified bytes by this path.
- **(b) an rkyv-friendly `NodeHash` newtype.** `#[repr(transparent)] NodeHash([u8; 32])` whose archived form is itself: a `[u8; 32]` is `Portable`, endianness-neutral, needs no relative pointers, and is 1-aligned, so a `NodeHash` embedded in an rkyv state is genuinely zero-copy and is _never_ a validation hazard (it has no interior pointers for bytecheck to police).
  No allocation, no owned handle.
- **(c) blob-kind discipline, generic over decode modes.** The store trait is generic over blob kind — prolly-tree **nodes** (structured, node-decoded) vs opaque **rkyv state blobs** (hash-verified, then rkyv-accessed) — with node-decode as a _mode, not a fork_.
  This is exactly the owner refinement already pinned on `gandr-bvf` ("keep the store trait generic over blob kinds").

**Alignment caveat (new finding — see §10 Q3).** rkyv's default `aligned` configuration requires the byte slice handed to `access` to be correctly aligned; a `PackedSegmentStore`-style backend that returns arbitrarily-offset slices out of a packed segment will not satisfy that for free.
This couples the store's blob-return contract to the rkyv configuration and must be resolved at absorption (align blob starts, copy into an `AlignedVec`, or adopt the `unaligned` feature at a read-cost — a format-control choice that itself feeds `rkyv_config_id`).

## 6. bytecheck posture — three options, recommend (ii)

The inner structural-validity wall on the payload. rkyv's checked reader is `access::<T, E>` (runs `CheckBytes`); its unchecked reader is `unsafe access_unchecked::<T>` (§7 F2–F4).

- **(i) always-validate.** Every access runs `access` (full bytecheck traversal).
  Simplest and always sound, but pays O(archive size) validation on every read — forfeiting much of zero-copy's point for bulk-resident states read repeatedly.
- **(ii) validate-once-per-`NodeHash`, with a validity marker — RECOMMENDED.** `bytecheck` runs via `access` at the **first** materialization of a given `NodeHash`; the store records a validity marker keyed by that `NodeHash`.
  Subsequent reads of the same hash skip revalidation and use `access_unchecked`.
  **Sound because the store is content-addressed and append-only**: a hash's validity verdict is an immutable function of its bytes, so the cached verdict can never go stale.
  The marker lives in **store metadata, outside the hashed bytes**.
  Debug builds validate always and assert the checked result against the marker path (a differential that catches marker-logic bugs).
- **(iii) debug/gate-only.** Fastest; release rehydration trusts writer correctness with no runtime validation.
  **Rejected while the writer is young** — revisit once by-construction layout guarantees exist and the schema set stabilizes.

**Recommendation: (ii).** It spends exactly one bytecheck traversal per distinct blob and then rides zero-copy, which is the whole point of pairing rkyv with a CAS.

**Soundness, stated precisely.** Option (ii) is the content-addressed instance of the FlatBuffers `Verifier` discipline (§7 F9): _validate once for an untrusted source, then access unchecked once the source is known-good_. rkyv's own `access` / `access_unchecked` split is the same discipline; content-addressing supplies the "known-good source" precondition, because "this exact byte sequence is a valid archive of schema `S`" is a property of the `NodeHash`, decided once.
The residual risk if a marker were ever wrong is bounded to _skipping revalidation of bytes whose integrity hash still matched_ — an engineering-robustness cost, never a soundness one (§8).

**Marker persistence is an open call (§10 Q4):** a per-process marker is trivially sound; a persisted marker (surviving restarts) additionally trusts the marker store's own integrity, an acceptable trade only because the integrity hash still runs on `get` and the cost of a wrong marker is bounded as above.

## 7. Verified-facts table

All rows checked against current rkyv 0.8.x documentation on **2026-07-20**.
Two ambiguities (F8, and the big-O in F5) are flagged rather than smoothed over.

| #   | Claim                        | Verified finding                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Source                                                                             |
| --- | ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| F1  | Current version / line       | **rkyv 0.8.17**, released 2026-07-02; the 0.8.x line is current.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | docs.rs/crate/rkyv/latest                                                          |
| F2  | Safe access API              | `access::<T, E>(bytes: &[u8]) -> Result<&T, E>` where `T: Portable + for<'a> CheckBytes<HighValidator<'a, E>>`, `E: Source`. Validated. Requires the `alloc` **and** `bytecheck` features.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | docs.rs `rkyv::api::high::access`                                                  |
| F3  | Unchecked access API         | `unsafe access_unchecked::<T>(bytes) -> &T` — bypasses validation; caller must guarantee the bytes are a valid archive. A no-alloc low-level `rkyv::api::low::{access, access_pos}` also exists.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | docs.rs `rkyv::api::fn.access_unchecked`, `rkyv::api::low`                         |
| F4  | rancor error strategy        | Error handling is pluggable via the `rancor` crate; the high-level API is generic over `E: rancor::Source`. Concrete strategies: `rancor::Failure` (cheap message), `rancor::Error` (richer trace), `rancor::Panic` (zero-cost, panics — for infallible/no_std). `Strategy<T, E>` type-erases the error so serializer/validator code stays monomorphic.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        | docs.rs rkyv examples; rkyv.org/validation                                         |
| F5  | bytecheck integration + cost | The default-on `bytecheck` feature derives `CheckBytes` per `#[derive(Archive)]` type; `#[rkyv(bytecheck(..))]` forwards args to the derive. Validation uses a "subtree range" model: every relative pointer is checked in-bounds, aligned, and sized for its target, with a context tracking allowed subobject ranges to block cyclic/aliasing/recursion attacks. **Cost:** a full traversal — each archived value visited once — so O(archive size / pointer count). rkyv states validation "carries measurable overhead" vs unchecked but gives **no explicit big-O** (the linear-pass characterization is ours, flagged).                                                                                                                                                                                                                  | rkyv.org/validation                                                                |
| F6  | no_std + alloc               | Default features are `std`, `alloc`, `bytecheck`; rkyv "supports no-std builds." There is **no standalone `no_std` feature** — set `default-features = false` and enable `alloc` (and `bytecheck` if validating). `std`/`alloc`/`bytecheck` **do not alter the serialized format.** `alloc` unlocks `api::high` (`to_bytes`, `access`); no-alloc reads use `api::low`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | docs.rs rkyv "Functionality"/features                                              |
| F7  | Wire-format stability        | **No independent format-version guarantee.** Serialized data is re-accessible only while: (a) the schema is unchanged, (b) format-control features are unchanged, and (c) the reader is a **semver-compatible** rkyv version. There is **no upstream format-version integer**. For a `0.8.x` dep, the semver-compatible band is the `0.8` minor; `0.8 → 0.9` is breaking.                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | rkyv.org overview; docs.rs rkyv "Functionality"                                    |
| F8  | Format-control features      | Endianness `little_endian`(default) / `big_endian`; alignment `aligned`(default) / `unaligned`; pointer width `pointer_width_16` / `_32`(default) / `_64`. Enabling non-default endianness/alignment is "a breaking change to the serialized format"; guidance: "binaries should explicitly choose format control options early." **Ambiguity flagged:** two rkyv doc surfaces conflict on pointer width — the crate-root "Pointer Width Features" text says it "does not change rkyv's serialized format," while the format-control framing and book (relative-pointer-offset sizes; default 32-bit) imply it changes the layout of any archive holding pointers or `isize`/`usize`. Conservative resolution: treat pointer width as format-affecting and pin it in `rkyv_config_id`; confirm with a cross-config differential at absorption. | docs.rs rkyv features; rkyv.org format                                             |
| F9  | Validate-once prior art      | rkyv ships **no** built-in persistent validated-marker cache — the pattern is application-level. Canonical precedent: FlatBuffers' `Verifier` — "validate once for untrusted data sources, then safely access ... knowing it has been verified"; "for trusted data sources, unchecked access is appropriate." rkyv's `access` / `access_unchecked` split is the same discipline; content-addressing supplies the trusted-source precondition.                                                                                                                                                                                                                                                                                                                                                                                                  | flatbuffers.dev / docs.rs flatbuffers `Verifier`; rkyv `access_unchecked` contract |

## 8. Trust-tier boundary (stated verbatim)

Machine states are elaborator-side scaffolding.
Kernel replay **never** consumes rkyv states — K2/E3 re-check from canonical bytes; the rkyv path is a checkpoint/cache accelerator, so residual risk is **engineering robustness** (a bad state costs recompute or a refused checkpoint), never **TCB soundness**.
Wall posture is automatic: `storage-*` is untrusted by the kernel-boundary naming rule (only `gandr-kernel-*` is trusted).

This is why the whole design can afford zero-copy at all: the worst outcome of a validity failure anywhere in this tier is wasted work, because nothing here is on the trusted-recheck path.

## 9. Crate posture and adjacencies

**Crate: `storage-rkyv`, generic** (the name is already anticipated in the `storage-*` tier charter, `gandr-bvf`).
The envelope, commitment, and marker machinery are generic over the schema type; gandr's concrete state schemas live with their **owning crates as consumers**, never in `storage-rkyv`:

- **A2 checkpoints** — the checkpoint format is a consumer schema.
- **L-machine state** (`gandr-fcw.9`) — the machine's state structs are consumer schemas; note the CEK→L-machine pivot (`fcw.9`) means the state shape is still moving, which argues for landing `storage-rkyv` only when the first consumer's schema stabilizes.

`no_std + alloc` is preferred, to match the tier posture, and is confirmed feasible (F6: `default-features = false` + `alloc`, with `api::low` for any no-alloc read path).

**Dependency-footprint note.** rkyv (plus `rancor` and `bytecheck`) lands in the `storage-*` tier only — untrusted plumbing, wall-fine.
A `syft`/`grype` scan is an absorption-time step, not a spike-time one.

**Strategic reuse — the wyrd-harness trajectory.** Keeping `storage-rkyv` generic is not gold-plating: it is the `gandr-bvf` extensibility directive (design gandr-first but for reuse) applied here, and it aligns with the project's **graduation principle — dogfood the stack** (`docs/WORKFLOW.md` §"Standing principles").
The original wyrd agentic harness may be rebuilt as something gandr _implements_ (the killer-app trajectory); a generic content-addressed fast-serialization layer is exactly the kind of substrate that reuse wants, so the generic/consumer split is banked now and paid off later.

**NO crate lands from this spike.** `storage-rkyv` arrives with its first real consumer (A2 checkpoint or L-machine work), built against this ratified design.

## 10. Open questions

1. **Q1 — exact commitment widths and magic string** (RATIFY). §4.1 gives a working draft (18/50 bytes); the invariants in §4.3 are fixed, the widths and the magic literal are the owner's to set, then golden-pin.
2. **Q2 — `rkyv_config_id` table contents** (RATIFY).
   Fix the initial configuration this project targets: almost certainly `{ rkyv 0.8, little_endian, aligned, pointer_width_32 }` (all defaults), but the choice is explicit and must be recorded in the spec-side id table (§4.2).
3. **Q3 — rkyv alignment vs the store's blob-return contract** (design, feeds `gandr-bvf`).
   Default `aligned` rkyv requires aligned read slices; a packed-segment store does not provide that for free.
   Resolve by aligning blob starts, copying into an `AlignedVec` on read, or adopting `unaligned` (a format-control choice that feeds `rkyv_config_id`, at a documented read-cost).
   This is the sharpest engineering coupling this spike surfaces (§5).
4. **Q4 — marker persistence scope** (design).
   Per-process marker (trivially sound) vs persisted marker surviving restarts (also sound given the integrity hash still runs on `get`, with the residual cost bounded to skipped revalidation — §6).
5. **Q5 — a rehydration budget** (design, the massive-term amplification analogue).
   A state referencing many CAS blobs can trigger unbounded hash-following; the `storage-*` tier should carry a rehydration bound (blob-count / depth) mirroring `gandr-bvf`'s `MAX_EXPANDED_TERM_WORK`, so a malformed-but-integral state cannot fan out unboundedly.
   Cheap, deterministic, and it stays entirely in the untrusted tier.
6. **Q6 — landing trigger** (sequencing).
   `storage-rkyv` lands with the first consumer whose schema has stabilized; the `fcw.9` CEK→L-machine pivot means the L-machine state shape is still in motion, so A2 checkpoints may be the earlier-stabilizing first consumer (§9).

## 11. Ratification queue (the bvf-package inputs)

1. **Thesis** — AFFIRMED with the five §1 conditions.
2. **Envelope** — adopt the §4 field set and the §4.3 invariants; ratify widths/magic (Q1) and the `rkyv_config_id` table (Q2).
3. **bytecheck posture** — adopt (ii) validate-once-per-`NodeHash` + store-metadata marker + debug-always differential (§6); decide marker persistence (Q4).
4. **Tree/store API** — the three requirements in §5 (get-by-hash verified blobs; rkyv-friendly `NodeHash` newtype; blob-kind-generic store trait), plus the alignment contract (Q3) — these are the inputs to the `gandr-bvf` API design.
5. **Trust tier** — the §8 boundary stands verbatim.
6. **Crate posture** — `storage-rkyv` generic, schemas live with consumers, no crate lands from this spike (§9).
7. **Open questions** — Q3 and Q5 are the two the owner should note as genuinely new findings from this spike.
