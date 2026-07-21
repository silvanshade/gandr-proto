# Massive-term design — planes 4 (replay checkpointing) and 2 (streaming decode), after B2.3 (gandr-3ln.1)

> **Status: PROPOSAL for owner ratification — not a decision record.** A design pass over the two open massive-term planes remaining after gandr-5t3 / B2.3, commissioned by gandr-3ln.1 under the four-plane program gandr-3ln.
> Plane 4 is replay checkpointing; plane 2 is the streaming-decode tail.
> No trusted-plane implementation is proposed here — this lane lands documentation only; nothing is adopted until the owner rules.
> Recommendations are marked as such; §9 is the numbered ratification queue and §7 the hazards.
>
> **Ratification-queue continuity.** The numbered queue below **continues the §12 queue of `docs/research/massive-term-design.md`**: RQ-1..RQ-9 live there (RQ-5 resolved at B2.3), and this record's new items begin at **RQ-10**.
>
> **Label conventions and coinage.** The kernel-discipline identifiers — **K1–K5** (kernel invariants), **E1–E6** (export invariants), **C3/C5** (the no-hash-consing-in-TCB and conversion-quarantine constraints), the **R1–R4** format reservations — follow the sibling research docs: the canonical spec node is `spec/kernel-boundary.md` (ADR-77/78), which is **referenced-but-not-yet-materialized** in the corpus tree (`docs/gandr/` holds only `README.md` + `MANIFEST.yml`; see `massive-term-design.md:11,398`), so each is anchored to its in-tree point of use — chiefly the B2 staging comment on gandr-wvd.2 (2026-07-20 15:58) and the kernel-core / storage-artifact rustdocs.
> **C3 pricing, the D1–D3 program decisions, and the RQ-n queue** are `massive-term-design.md`.
> **S0–S3** is the kernel subset ladder (S1 is the shipped subset; S2 is the next).
> This record **coins two plane-scoped decision families and says so explicitly**: **P4-D1..P4-D4** (plane 4) and **P2-D1..P2-D4** (plane 2), each defined at first use in §3/§4 and mapped to an RQ item in §9.
> Citation anchors are crate-relative repo paths (`crates/…:line`), verified by read against this worktree; no machine-local paths or session forensics appear in this file.
> `docs/research/` is outside the `docs/gandr/` corpus MANIFEST, so this file needs no b3sum entry and its cross-references are hand-maintained (the `massive-term-design.md:380` corpus-firewall note applies unchanged).

---

## 1. Executive summary — the two planes

Both planes are **read-path** designs over the B2.3 substrate: the canonical v1 export artifact, its declaration-segmented record model (`storage-artifact`), the three landed reader budgets, and the two-wall discipline.
Neither touches the kernel replay semantics (K2/E3) or the wire format — they are acceleration and delivery layers over an unchanged inner wall.

| Plane                    | Decision                               | Recommendation                                                                                                                                                                                                        | Owner posture                          |
| ------------------------ | -------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| 4 · replay checkpointing | **P4-D1** checkpoint grain             | The **declaration prefix** (E2 admission order), keyed by the outer prolly prefix node-hash; the prefix is the only sound resumable unit (strictly-earlier child refs).                                               | ratify (RQ-10)                         |
| 4 · replay checkpointing | **P4-D2** checkpoint trust posture     | **Cache-accelerator only, C3-clean**: in-process memoization of already-re-checked state; a persisted / cross-trust checkpoint is re-validated through `add_decl`, **never trusted by hash**.                         | ratify (RQ-11); the C3 line            |
| 4 · replay checkpointing | **P4-D3** incremental≡from-scratch     | Mechanize the standing gate at the **replay** plane, mirroring the existing A2.3 checker-plane differential: resume-replay ≡ from-scratch replay (identical `Environment` + `DecodeMetrics` + byte round-trip).       | ratify (RQ-12)                         |
| 4 · replay checkpointing | **P4-D4** B9 independence              | Checkpoints are **producer/host-side, outside the format**; the export-format spec stays the B9 replayer's sole specification — checkpointing adds **zero** B9 spec surface.                                          | ratify the boundary (RQ-13)            |
| 2 · streaming decode     | **P2-D1** verified-streaming substrate | **bao** is the ratified route (`massive-term-design.md:337`), feature-gated and promoted to a runtime dep **when a streaming consumer lands**; today it is dev-only and streaming verification is full-rebuild.       | ratify direction (RQ-14)               |
| 2 · streaming decode     | **P2-D3** incremental budgets          | All three landed budgets are **monotone in the prefix** ⇒ enforce **incrementally** at unchanged magnitudes; E4 canonicality stays a **whole-artifact** closing gate (streaming replay is speculative w.r.t. E4).     | ratify (RQ-15)                         |
| 2 · streaming decode     | **P2-D4** A2.5 demo consumption        | The A2.5 streaming demo composes **two** streams — checker-plane synthesis (A2.3) and storage-plane export decode; plane 2 backs the latter over chunk-granular `BlockStore` access, bao verification off by default. | ratify scope (RQ-17)                   |
| 4 × 2 · interplay        | **budget carry**                       | Checkpoint state carries the **accumulated** budget counters (table entries, artifact-total work) across resume/stream boundaries — else split-artifact evades the whole-artifact caps. Prices against, not around.   | ratify (RQ-16); the headline interplay |

> **Ratification note (owner, 2026-07-21):** all eight rows ratified as recommended (RQ-11 posture (a), RQ-15 posture (ii)), with one amendment — the P2-D1 substrate is the iroh-family `bao-tree`, not plain `bao` (§4.2, §9).

The single governing consequence: **the two walls do not move.** The outer wall (the manifest identity, a BLAKE3 over the record set) is untrusted plumbing that proves provenance, never validity; the inner wall (K2/E3 replay through `add_decl`) is the sole validity authority.
Every acceleration below — a resumed checkpoint, a streamed prefix — is priced by the rule that **integrity never substitutes validity** (`storage-artifact/README.md:26–32`).

---

## 2. Ground truth — the B2.3 substrate as built

Every code-anchored claim re-verified against this worktree.

### 2.1 The record model and why a prefix is the resumable grain

A v1 export artifact **is**, by construction, a sorted unique keyed record set: E2 admission ordering keys each declaration by its admission index, and the format is declaration-segmented (`storage-artifact/src/record.rs:3–8`).
`ArtifactRecord` is `(admission index big-endian key → declaration segment bytes)` (`record.rs:40–46`); `ArtifactRecordSet` is strictly ascending and unique by key, so every identity it feeds is history-independent (`record.rs:97–121`, the `from_records` permutation-canonicalizing constructor `:190`).

The load-bearing structural fact for plane 4: **a declaration prefix is closed under child references, a suffix is not.** Subterm-table entries are indexed by a single global counter running across segments in admission order, and every child index is **strictly earlier** than its own entry's global index — enforced at decode as `MalformedSite::ChildOrder` acyclicity (`kernel-core/src/export/read.rs:15–18,918`; `massive-term-design.md:237` §4.5).
So a segment-*k* entry may reference entries introduced in segments `0..=k`, never later.
Consequently the declaration prefix `0..k` references only entries it introduces — it is a self-contained sub-artifact — whereas a suffix segment may reference prefix entries, which is exactly why `record.rs:14` states a record is a **content-addressing grain, not an independently replayable unit**: replay is whole-artifact over the reassembled bytes.
The asymmetry is the design seam: prefixes resume, suffixes do not.

### 2.2 The replay path and the two walls

`decode` (`read.rs:255`) yields a self-contained `DecodedArtifact` — a `TermArena` plus the admission-ordered declaration sequence plus the `DecodeMetrics` computed en route (`export.rs:415–479`).
`read` (`read.rs:470`) imports each declaration's content into a fresh environment and **re-admits through the choke point** (`env.rs:227` `add_decl`) — K2 re-derivation, E3 audit recomputed never imported, E6 marks preserved.
The `Environment` (`env.rs:151`) holds the admitted declarations in admission order with their marks, the internal arena, the transitive audit (`env.rs:335`), and the **admission watermark** (`env.rs:12–16`, the D1(C) content-start watermark — the Idris `branchDepth`/`staging` transactional-overlay pattern, `impl-models-deep-read.md:107–109`, §5.6 #4).
That `Environment` at index `k` is precisely the state a plane-4 checkpoint snapshots, resumable by admitting past the watermark.

The outer layer (`storage-artifact`) is the untrusted CAS wiring: `ArtifactManifest` binds the 85-byte chunker parameter commitment, the record count, the root node hash, and the inner format version; `ArtifactIdentity = BLAKE3(manifest)` is the b3sum-provenance successor, and hashing lives outside the TCB (`storage-artifact/docs/STATUS.md:14–20`).

### 2.3 The three reader budgets — and why they are monotone in the prefix

Landed at B2.3 (RQ-5 retune), all `D3`-tunable (`kernel-core/src/export.rs`):

| constant                     | value   | axis                                   | anchor          | incremental?                                                      |
| ---------------------------- | ------- | -------------------------------------- | --------------- | ----------------------------------------------------------------- |
| `MAX_EXPANDED_TERM_WORK`     | `1<<20` | per-declaration-root tree work         | `export.rs:125` | yes — a root's `expanded_size` uses only strictly-earlier entries |
| `MAX_TABLE_ENTRIES`          | `1<<18` | distinct-DAG-node count (input-linear) | `export.rs:140` | yes — already enforced "as entries accrue" (`read.rs:775`)        |
| `MAX_ARTIFACT_EXPANDED_WORK` | `1<<24` | artifact-total tree work (gandr-4p3)   | `export.rs:166` | yes — a saturating running sum, non-decreasing in the prefix      |
| `MAX_DECODED_LEVEL_OFFSET`   | `4096`  | level-atom successor offset            | `export.rs:179` | n/a — per-atom, already local                                     |

The budgets are computed in **one forward scan** producing the deterministic `DecodeMetrics` (`read.rs:340–376` `check_budget`; `export.rs:359` `DecodeMetrics`, public on `DecodedArtifact` at `export.rs:475`).
Two owner flags stand from the retune, and both bear on this record: **(1)** the binding floor is the deep-drop hardening witness (~400k expanded / ~200k entries), **not** the corpus (max 7 work units) — the reader must accept every artifact the kernel legitimately round-trips; **(2)** `MAX_TABLE_ENTRIES` carries only ~1.3× headroom over its floor — the first budget to raise.
This record prices **against** these magnitudes, never around them: the streaming and checkpoint designs change **when** the caps are checked and **across which boundaries the accumulators carry**, never the constants.

### 2.4 The storage crates as built (the streaming substrate)

* `storage-chunker` — `#![no_std]`, **zero runtime deps** (`Cargo.toml:14` empty `[dependencies]`); `RecordBoundaryRule::BETWEEN_RECORDS` (`src/lib.rs:614`) cuts only between complete records, so a chunk boundary **is** a declaration boundary; the 85-byte `PARAMETER_COMMITMENT_LEN` (`src/lib.rs:45`, `0x55`) is what the manifest binds.
* `storage-prolly-trees` — the ordered-record Merkle tree with `BlockStore` (`src/store.rs:75`), `InMemoryBlockStore` (`:109`, verifies on every insert and load), and `PackedSegmentStore` (`:243`, in-memory append-only, **explicitly not a persistent backend or stable on-disk format**, `docs/STATUS.md:39–40`).
  Membership / non-membership / range proofs and witness transcripts live in `src/proof.rs`, **feature-gated** behind the default `proofs` feature (`Cargo.toml:22–28`).
* **Two inherited-deferred residuals directly constrain plane 2** (`storage-prolly-trees/src/lib.rs:31–39`): the tree is **two-level** (the scale ceiling), and **incremental/streaming witness verification is deferred — the witness verifier is full-rebuild; `bao` provides verified streaming as dev evidence only.** `bao 0.13.1` is a **dev-dependency** (`storage-prolly-trees/Cargo.toml:20`), exercised only to encode/decode canonical snapshot bytes as adapter evidence in tests; the crate is explicitly **not** a bao byte-stream proof format (`README.md:16,68–69`).
  **There is no production code path that streams-and-verifies today.**

### 2.5 The prior art already in the tree — A2.3 checkpointing and the rkyv posture

Two in-tree facts make plane 4 an adaptation, not a green field:

* **The checker plane already checkpoints and already gates incremental≡from-scratch.** `core-checker/src/checkpoint.rs` implements `checkpoint_program` (`:232`), `resume` (`:281`), `resume_with` (`:312`); `core-checker/tests/incremental.rs:1–8` is the A2.3 differential gate — "incremental validated resume ≡ from-scratch re-typing … the skips never change the answer."
  This is the mechanism plane 4 mirrors, but at the **kernel-replay** (TCB) plane rather than the elaborator plane — a decisive difference the trust posture turns on (§3.2). (Note a STATUS/code tension flagged in §7 H-1: `core-checker/docs/STATUS.md:27` calls A2.3 "frozen" while `checkpoint.rs` and its passing gate exist.)
* **The C3-priced checkpoint posture is already ratified for the kernel plane.** `storage-rkyv-design.md:160` — "kernel replay **never** consumes rkyv states — K2/E3 re-check from canonical bytes; the rkyv path is a checkpoint/cache accelerator … never TCB soundness"; A2 checkpoints are named its first consumer (`:170,181,198`), and the spike lands no crate.
  Plane 4 stays strictly inside this posture; its contribution is to make the grain, the gate, and the anti-evasion price concrete for the post-B2.3 export-artifact replay path.

### 2.6 The program landmarks this record feeds

* **B4** — the "Perf architecture" phase: the glued-NbE hash-consing normalizer adapted to the L machine, which **is** the conversion engine (`PLAN.html` backbone node "B4 Perf architecture"; `b3-module-system-design.md:319–328`).
* **A2.5** — the A2 incrementality lane's first milestone, a **streaming demo** run **alongside B4**: "marks + obligations + goals streamed during synthesis" (`PLAN.html`, the A2 lane / wvd.14).
  It is a synthesis-plane demo; plane 2 (export-artifact streaming decode) is a **storage-plane** substrate that can back its persistence/replay side (§4.4 — the two streams must not be conflated).
* **B9** — the "Certificates + kernel-replay" phase, where **kernel-replay (the independent second checker)** lands.
  The in-tree rule is verbatim: re-deriving records from opaque foreign bytes is "a reader-side framing walk against the format specification, **never shared code**" (`storage-artifact/README.md:44`, `docs/STATUS.md:25`). (The literal terms "clean-room replayer" / "independent replayer" do **not** appear in the tree; the tree's terms are "kernel-replay (independent second checker)" and "the future B9 replayer" — used here.)

---

## 3. Plane 4 — replay checkpointing

The question: how to avoid re-running K2/E3 replay over declaration prefixes shared across artifacts (a base theory + N edits/extensions), without weakening the inner wall.

### 3.1 P4-D1 — checkpoint grain: the declaration prefix, prolly-hash-keyed

**Recommendation: prefix-granular checkpoints, keyed by the outer prolly node-hash of the declaration prefix.**

The prefix is the only sound resumable unit (§2.1): a prefix `0..k` is closed under child references, so replaying it produces a well-defined `Environment` independent of any suffix.
The outer plane already makes "do two artifacts share a prefix of length `k`?"
cheap: because chunk boundaries are declaration boundaries (§2.4) and the prolly tree is a deterministic function of the ordered record set, a shared declaration prefix yields **identical node hashes** — the shared-prefix test is an outer, untrusted, hash comparison, never a term walk.

Options weighed:

* **(A) whole-artifact only (status quo).** No reuse; every replay is from-scratch.
  Reject: forfeits the entire plane.
* **(B) per-declaration (record-granular) resume.** Rejected by construction: a single record is _not_ independently replayable (`record.rs:14`) — a suffix segment may reference prefix table entries, so you cannot resume "at declaration `k` alone" without the `0..k` arena.
* **(C) prefix-granular (recommended).** Resume the `Environment` at a prefix boundary; the prolly prefix-hash is the cache key.
  Aligns the checkpoint grain with the E2 replay grain and the plane-2 stream grain — **one grain, three consumers**.

### 3.2 P4-D2 — checkpoint trust posture: cache-accelerator only (the C3 price)

**Recommendation: in-process memoization of already-re-checked state; a persisted or cross-trust checkpoint is re-validated through `add_decl`, never trusted by hash.**

This restates the `storage-rkyv-design.md:160` posture at the export-artifact replay plane, and prices it precisely.
A checkpoint is a saved `Environment` (arena + admission log + audit) at prefix boundary `k`, tagged by the prefix-hash.
The prefix-hash match is **outer / untrusted plumbing** — it proves provenance (this is the prefix you think it is), never validity.

Two sub-postures, and the C3 line between them:

* **(a) In-process memoization (C3-clean, the sound win).** Resume from an `Environment` **this process already produced by from-scratch `add_decl`**.
  Sound because the inner wall ran once, at checkpoint creation; the hash only _selects which_ memoized state, it never _substitutes_ for checking.
  This is the reuse a build/session gets for free across prefix-sharing artifacts.
* **(b) Persisted / cross-trust checkpoint (C3 violation if trusted).** Loading a serialized `Environment` from disk or a peer and using it _without_ re-deriving it through `add_decl` is exactly integrity-substitutes-validity — a trusted table smuggled into the TCB, the C3 prohibition.
  To admit it soundly you must **re-validate = re-replay** the prefix, at which point the checkpoint is a _performance hint_ (which prefix to expect / prefetch), not a trust shortcut.

**The honest consequence, stated plainly:** because re-validation _is_ re-checking, a persisted checkpoint buys **no sound saving of the checking work** — only of the re-decode/re-parse (the cheaper, already amplification-bounded step).
So the sound speedup is confined to **in-process prefix memoization**; the trust boundary is exactly where the saving stops.
This is what "C3-priced fast path" means here: the price of C3 is that the fast path cannot cross the trust boundary.
The distinction from A2.3 (§2.5) is the crux — the A2.3 checkpoint engine sits **outside** the TCB (elaborator plane), so it may trust its own checkpoints; the plane-4 kernel-replay checkpoint sits **inside** the TCB and may not.

### 3.3 P4-D3 — the incremental≡from-scratch gate, at the replay plane

**Recommendation: mechanize the standing gate as a replay-plane differential, mirroring `core-checker/tests/incremental.rs`.**

The gate: for any artifact and any prefix boundary `k` with a checkpoint, assert `replay_from_scratch(artifact) ≡ replay_resuming_from_checkpoint(artifact, k)`, comparing (i) the resulting `Environment` — admission-ordered declarations, admission marks (E6), and the transitive audit (E3); (ii) the `DecodeMetrics`; and (iii) the byte round-trip (`write_segmented` of the resumed environment is byte-identical to the artifact).
This is the plane-4 analogue of the A2.3 checker-plane gate and the guarantor that the reuse "never changes the answer."
It is also the test that would catch a checkpoint that silently drops audit or watermark state.

### 3.4 P4-D4 — B9 independence: checkpoints are outside the format

**Recommendation: ratify that checkpoints are producer/host-side, never part of the artifact and never consumed by the B9 replayer.**

The B9 kernel-replay (independent second checker) replays from the canonical bytes alone, by the "never shared code" rule (§2.6).
A checkpoint is a producer-side accelerator over _already-trusted-in-this-process_ state; it is therefore **outside** the export-format specification and adds **zero** new spec surface B9 must implement.
This is not merely allowed but required: were a checkpoint part of the artifact, B9 would have to either implement it (impossible under "never shared code") or trust it (a C3 violation).
Keeping checkpoints outside the format is what makes P4-D2's posture coherent with B9 independence — the same two-wall discipline, applied to a third layer.

---

## 4. Plane 2 — streaming decode

The question: deliver and validate a large artifact chunk-by-chunk (feeding the A2.5 demo) without loosening the amplification defence or the canonicality gate.

### 4.1 Ground-truth grain (not an RQ) — streaming is declaration-granular

Because record-safe chunking is declaration-granular (§2.4), a stream of chunks **is** a stream of whole declaration segments.
The stream unit therefore coincides with the E2 replay grain and the P4 prefix-checkpoint grain — the same "one grain, three consumers" as §3.1.
This is fixed by the record model, so it is recorded as ground truth rather than a ratification item.

### 4.2 P2-D1 — verified-streaming substrate: bao, feature-gated, when a consumer lands

**Recommendation: keep bao dev-only until a streaming consumer is real; promote it to a `streaming`-feature runtime dependency of `storage-prolly-trees` at that point.**

`bao` is the ratified verified-streaming route (`massive-term-design.md:337`), but today it is dev-only and no code streams-and-verifies (§2.4).
The honest posture: **do not claim verified streaming as a current capability.** Streaming verification is inherited-deferred alongside the multi-level tree; until the A2.5 demo needs it, chunk-granular access is via `BlockStore` (which verifies each blob on load, `store.rs:137–170`) and whole-artifact verification is full-rebuild. bao is untrusted plumbing (the outer wall), so promoting it is a dependency decision with **no TCB impact** — but it is a new runtime dep and belongs behind a feature until earned.
Options: **(A)** promote bao to runtime now (rejected — no consumer, dead weight); **(B)** feature-gate, promote on first consumer (recommended); **(C)** hold at full-rebuild indefinitely (rejected — forfeits the demo's incrementality).
**Ratification amendment (owner, 2026-07-21):** the promoted substrate is the iroh-family `bao-tree` crate, not plain `bao` — the same BLAKE3/Bao verified-streaming model, chosen to align with the planned iroh adoption; the dev-only `bao 0.13.1` adapter evidence stays as-is until the promotion moment.

### 4.3 P2-D3 — incremental budgets, and the E4 whole-artifact seam

**Recommendation: enforce all three budgets incrementally at unchanged magnitudes; keep E4 canonicality a whole-artifact closing gate and accept that streaming replay is speculative w.r.t.** **E4.**

Every reader budget is **monotone in the prefix** (§2.3), so a streaming reader enforces each as declarations arrive: `MAX_TABLE_ENTRIES` already accrues per entry; a declaration root's `expanded_size` is known when its segment completes (children are strictly earlier); the artifact-total is a non-decreasing saturating sum that rejects on first exceedance.
Streaming changes _when_ the caps fire, never the constants — pricing against the landed magnitudes exactly.

The one genuine seam: **E4 canonicality is not prefix-local.** The whole-artifact re-encode-compare and the "no dead entries" condition (`massive-term-design.md` §4.6 items 2–4; `read.rs:30–35`) can only be settled once the whole table is seen — an entry introduced in segment `k` may be first referenced by segment `k+5`, so a prefix cannot certify "no dead entries."
Therefore a streaming reader that _replays before completion_ is **speculative w.r.t.** **E4**: a late non-canonical byte can reject an artifact whose prefix already replayed.
This is safe for validity — canonicality is a byte-*form* property, not a typing property, so the admitted declarations are themselves valid — but the posture must be chosen: **(i)** hold the admission commit until E4 confirms (safe; defers the incrementality win), or **(ii)** stream-replay eagerly and treat E4 as the closing verdict (the admitted decls stay valid; the artifact-level canonicality stamp is the final gate).
Recommendation for the A2.5 demo: **(ii)** — validity is what the demo shows; canonicality is the closing stamp.

### 4.4 P2-D4 — feeding A2.5: two streams, not one

**Recommendation: back the A2.5 demo's storage side with plane-2 chunk-granular decode over `BlockStore`, bao verification off by default; keep it distinct from the checker-plane synthesis stream.**

A2.5 streams "marks + obligations + goals during synthesis" (§2.6) — that is a **checker-plane / A2.3** stream.
Plane 2 is a **storage-plane** stream: declaration segments delivered chunk-by-chunk from the prolly store, budgets enforced incrementally (§4.3), replay incremental where P2-D3(ii) is chosen.
The demo _composes_ the two; the design must not conflate them (a hazard, §7 H-4).
Concretely, the demo can: stream a large theory's segments from an `InMemoryBlockStore`, enforce the three budgets incrementally, and replay declarations as their prefixes complete — with bao verification as an off-by-default feature until P2-D1 promotes it.

---

## 5. The interplay — checkpoint × streaming budget accounting (the headline)

Plane 4 and plane 2 combine when a streaming reader **resumes from a prefix checkpoint** and streams the suffix.
The charter's binding constraint — _price against the landed budgets, not around them_ — bites exactly here.

The whole-artifact caps (`MAX_ARTIFACT_EXPANDED_WORK`, `MAX_TABLE_ENTRIES`) are properties of the **entire** artifact.
If a checkpoint resumes at boundary `k` with the budget accumulators **reset to zero**, then splitting an artifact into "checkpointed prefix + streamed suffix" evades the whole-artifact caps: the suffix alone stays under `MAX_ARTIFACT_EXPANDED_WORK` even when prefix + suffix would exceed it.
That is a budget-evasion vector introduced _by the combination_, invisible to either plane alone.

**Recommendation (RQ-16): the checkpoint state must include the accumulated `DecodeMetrics` counters — the running table-entry count and the saturating artifact-total expanded work at boundary `k` — and resumption seeds the running budget from them, so the whole-artifact caps remain whole-artifact under resume+stream.** `DecodeMetrics` already carries exactly these quantities (`export.rs:359–369`), so the checkpoint stores what it already computes; the anti-evasion cost is one seeded accumulator, not a new scan.

Two consequences worth recording:

* **Owner flag 2 is aggravated from a new angle.** A heavily-shared prefix consumes table-entry budget that the suffix then cannot use; with cross-declaration sharing, `MAX_TABLE_ENTRIES` (only ~1.3× headroom, §2.3) becomes the binding constraint for large checkpointed/streamed theories — independent corroboration that it is the first budget to raise.
* **The per-declaration cap needs no carry.** `MAX_EXPANDED_TERM_WORK` is per-root and local; only the two _artifact-total_ accumulators cross the boundary.

---

## 6. Plane 3 — posture statement only (S2-gated; not designed here)

Plane 3 (the decision-layer fast-path posture) stays **S2-gated** and is **not designed in this record**.
Its sole standing obligation is that the **S2 conversion design pass must state its fast-path posture against C3**: the reader's expanded-work budget is the S1 answer (no ptr-keyed table in the TCB), and the natural S2 successor is a sharing-aware checker memo, at which point the reader budget becomes a defence-in-depth outer bound rather than the sole bound (`massive-term-design.md:203` §4.4, `:410` open q.3).
No decision is taken here; this paragraph is the posture statement the charter requires.

---

## 7. Hazards

* **H-1 (A2.3 STATUS/code tension — reconcile before relying).** `core-checker/docs/STATUS.md:27` calls the A2.3 incremental checkpoint/diff engine "frozen", yet `core-checker/src/checkpoint.rs` and the passing `tests/incremental.rs` differential exist.
  Plane 4 mirrors that mechanism (§2.5, §3.3), so the owner should reconcile whether A2.3 is built-and-mis-documented or the code is a not-yet-live scaffold, before P4-D3 leans on it as precedent.
* **H-2 (persisted-checkpoint C3 trap — document now, before anyone builds a store).** The `storage-artifact` store already persists artifacts; the temptation to persist replayed `Environment` state alongside and trust it by hash is real and is exactly the integrity-substitutes-validity violation (§3.2b).
  The boundary must be recorded _before_ a checkpoint store is built, not after.
* **H-3 (over-claiming verified streaming).** Any doc or demo that presents bao as delivering verified streaming _today_ is unsupported (§2.4); the witness verifier is full-rebuild and bao is dev-only.
  Claim only the _route_, not the capability.
* **H-4 (two-stream conflation at A2.5).** The A2.5 synthesis stream (checker plane, A2.3) and the export-decode stream (storage plane, plane 2) are different objects (§4.4); a demo that treats them as one will mis-attribute where budgets, checkpoints, and verification apply.
* **H-5 (E4 speculation surprise).** Under P2-D3(ii), a streamed artifact can replay a valid prefix and _then_ be rejected as non-canonical (§4.3); a consumer that equates "prefix replayed" with "artifact accepted" is wrong.
  The closing E4 stamp is authoritative.
* **H-6 (two-level tree scale ceiling).** The prolly tree is two-level (§2.4); the scale plane the whole program targets will eventually need the deferred multi-level tree before checkpoint/stream over a genuinely large corpus is real.
  Not blocking now; load-bearing later.
* **H-7 (doc-drift noticed, out of scope — reported per the standing duty).** `storage-chunker/docs/STATUS.md:25–33` + `README.md:31–38` cite a `benches/chunker.rs` and criterion/fastcdc/blake3 dev-deps that **do not exist** in this worktree (no `benches/`, empty `[dev-dependencies]`).
  Separately, the ported `AGENTS.md` points to `ARCHITECTURE.md` and `docs/HAZARDS.md`, neither of which exists here (the roadmap is `PLAN.html`).
  Both are stale-reference hazards worth a cleanup pass; neither is in this lane's scope.

---

## 8. Verification register — confirmed and corrected against the tree

* **Confirmed by read:** the strictly-earlier child-order invariant making a prefix self-contained (`read.rs:15–18,918`); a record is not independently replayable (`record.rs:14`); the three budgets, magnitudes, and their one-forward-scan `DecodeMetrics` (`export.rs:125,140,166,359`; `read.rs:340–376`); `write_segmented`/`SegmentedArtifact` as a hash-free structural byproduct (`export.rs:497`, `write.rs:163`); the `Environment` + admission-watermark state (`env.rs:12–16,151,227,335`); the storage crates' surfaces and the bao-dev-only / full-rebuild / two-level deferrals (`storage-prolly-trees/src/lib.rs:31–39`, `Cargo.toml:20`, `store.rs`); the A2.3 checkpoint precedent (`core-checker/src/checkpoint.rs`, `tests/incremental.rs`); the ratified checkpoint-as-cache-accelerator posture (`storage-rkyv-design.md:160`).
* **Corrected / sharpened:**
  + The charter's "checkpoint trust as a C3-priced fast path" is sharpened to its exact price: the fast path **cannot cross the trust boundary** — persisted checkpoints buy no sound _checking_ saving (§3.2).
    This distinguishes the plane-4 (TCB) checkpoint from the A2.3 (elaborator) one, which the raw framing conflates.
  + "bao verified streaming" is a **route, not a current capability** (§2.4) — the sweep confirmed no production streams-and-verifies path; prior prose ambiguous on this is corrected here.
  + The program's independent-replayer landmark is **"B9 kernel-replay (independent second checker)"**; the terms "clean-room replayer" / "independent replayer" are **not** in the tree and are avoided.
* **Not independently re-derived (cited as ratified prior art):** the C3 no-hash-consing-in-TCB constraint and the E/K invariants (`massive-term-design.md` §3.2; the wvd.2 15:58 staging comment), the referenced-but-unmaterialized `kernel-boundary.md` node (`massive-term-design.md:11,398`).

---

## 9. Ratification queue — RQ-10.. (continues `massive-term-design.md` §12)

Each item: the decision, its options, the recommendation, and what it blocks.
RQ numbering **continues the §12 queue** (RQ-1..RQ-9 there).

> **RATIFIED (owner, 2026-07-21):** RQ-10..RQ-17 adopted per recommendation — RQ-11 posture (a), RQ-15 posture (ii) — with one amendment to RQ-14: the verified-streaming substrate is the iroh-family `bao-tree` crate rather than plain `bao` (iroh adoption is planned, so the substrate aligns with that trajectory now).
> Ratification record on `gandr-3ln.1`.

**RQ-10 — P4-D1 checkpoint grain.** Options: **(A)** whole-artifact only · **(B)** per-declaration resume · **(C)** declaration-prefix, prolly-hash-keyed.
Recommendation **(C)** (§3.1) — the prefix is the only sound resumable unit (strictly-earlier child refs); the prolly prefix-hash is the outer, untrusted cache key.
Blocks: any replay-cache implementation.

**RQ-11 — P4-D2 checkpoint trust posture (the C3 line).** Options: **(a)** in-process memoization (C3-clean) · **(b)** persisted/cross-trust checkpoint, re-validated through `add_decl` (hint, not trust) · **(reject)** persisted checkpoint trusted by hash (C3 violation).
Recommendation: **(a)** is the sound speedup; **(b)** is a prefetch hint that saves only re-decode, never checking (§3.2).
Blocks: whether a persisted checkpoint store is worth building at all.

**RQ-12 — P4-D3 incremental≡from-scratch gate at the replay plane.** Recommendation: a resume-vs-from-scratch differential asserting identical `Environment` (decls + E6 marks + E3 audit) + `DecodeMetrics` + byte round-trip, mirroring `core-checker/tests/incremental.rs` (§3.3).
No genuine alternative — this is the standing gate made concrete.
Blocks: landing any replay cache soundly.

**RQ-13 — P4-D4 B9 independence boundary.** Recommendation: ratify that checkpoints are producer/host-side, outside the export-format spec, never consumed by the B9 replayer — zero new B9 spec surface (§3.4).
Alternative (reject): a checkpoint carried in-artifact, which forces B9 to trust or reimplement it.
Blocks: nothing structural; closes a C3/B9 coherence question.

**RQ-14 — P2-D1 verified-streaming substrate.** Options: **(A)** promote bao to runtime now · **(B)** feature-gate, promote on first consumer · **(C)** hold at full-rebuild.
Recommendation **(B)** (§4.2) — bao is the ratified route (untrusted plumbing, no TCB impact) but unearned until A2.5 needs it.
Blocks: A2.5 streaming verification.

**RQ-15 — P2-D3 incremental budgets + E4 posture.** Recommendation: enforce all three budgets incrementally at unchanged magnitudes (monotone in the prefix); keep E4 whole-artifact and choose the streaming-replay posture — **(i)** hold admission until E4, or **(ii)** eager stream-replay with E4 as the closing stamp (recommended for A2.5) (§4.3).
Blocks: the streaming reader.

**RQ-16 — checkpoint × streaming budget carry (the interplay).** Recommendation: checkpoint state carries the accumulated `DecodeMetrics` counters (table-entry count, artifact-total expanded work) across the resume/stream boundary, so the whole-artifact caps stay whole-artifact and split-artifact cannot evade them (§5).
No alternative is safe — resetting the accumulators is a cap-evasion vector.
Blocks: any combined checkpoint+stream reader.

**RQ-17 — P2-D4 A2.5 demo scope.** Recommendation: the demo composes two distinct streams (checker-plane synthesis / A2.3, and storage-plane export decode / plane 2); plane 2 backs the storage side over chunk-granular `BlockStore` access with bao verification off by default (§4.4).
Alternative (reject): a single conflated stream.
Blocks: the A2.5 milestone's storage/replay side.

---

## 10. Summary of recommendations

Plane 4 — adopt **prefix-granular checkpoints** keyed by the outer prolly prefix-hash (RQ-10), as a **C3-clean in-process cache accelerator** whose trust never crosses the boundary (RQ-11), guarded by a **replay-plane incremental≡from-scratch differential** mirroring A2.3 (RQ-12), kept **outside the export-format spec** so B9 independence holds (RQ-13).
Plane 2 — enforce the three landed budgets **incrementally** at unchanged magnitudes with **E4 as a whole-artifact closing gate** (RQ-15), adopt **bao as the feature-gated verified-streaming route** promoted on first consumer (RQ-14), and back the **A2.5 demo's storage stream** without conflating it with the synthesis stream (RQ-17).
The headline interplay — a resuming, streaming reader must **carry the artifact-total budget accumulators across the boundary** (RQ-16), pricing against the landed caps rather than around them.
Plane 3 stays S2-gated; its only obligation is the S2 conversion pass's C3 posture statement (§6).
The owner's live calls are **RQ-11** (the C3 trust line), **RQ-15(i-vs-ii)** (the E4 streaming posture), and **RQ-16** (the anti-evasion budget carry); the remainder are recommended-with-alternatives-recorded.
