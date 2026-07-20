# Massive-term design — kernel subterm sharing (D1), export v1 sharing format, and the prolly-bao CAS revival (gandr-bvf)

> **Status: PROPOSAL for owner ratification — not a decision record.** This doc formalizes the coordinator's massive-term proposals (gandr-bvf) into a ratification-ready form.
> The decisions are already proposed; this is formalization, source verification, and byte-level sharpening, not redesign.
> Nothing here is adopted until the owner rules.
> Recommendations are marked as such; §12 is the numbered ratification queue and §13 separates genuinely open questions.
> **Consumers.** The implementation bead `gandr-5t3` (which lands the ratified decisions before B2.3); the four-plane program `gandr-3ln` (D1/D2/D3 coined there); the `gandr-1hu` rkyv spike (parallel; feeds the tree/store API surface); B2.3 (which carries the D3 telemetry floors and the outer-layer wiring).
>
> **Citation conventions.** Kernel-core anchors are crate-relative repo paths (`crates/kernel-core/src/…:line`) and were re-verified by read against the live worktree this pass; a claim marked _verified_ means the cited source was read, not that the tree was compiled this pass.
> The `mach` prolly-bao anchors are `mach @ fb78601:<path>:<line>` (read-only checkout, never modified); the load-bearing ones were spot-verified this pass and are marked, the remainder are carried from the gandr-bvf 21:31 reuse determination as the coordinator's synthesis.
> The kernel discipline identifiers — K1–K5, E1–E6, C3/C5, and the R1–R4 format reservations — follow the sibling research docs' convention of citing `spec/kernel-boundary.md` (ADR-77/78) as the canonical spec node; that node is referenced-but-not-yet-materialized in the corpus tree (`docs/gandr/` currently holds only `README.md` + `MANIFEST.yml`), so each identifier is anchored, where possible, to its in-tree point of use in the kernel-core rustdoc.
> No machine-local paths and no session forensics appear in this file.

---

## 1. Executive summary — the four decisions

The four decisions interlock: decode-retains-sharing (D2, the format) is impossible without a shared in-memory term (D1, the representation), and the CAS layer (D4) chunks on the format's declaration segments.

| #              | Decision                   | Recommendation                                                                                                                                                                   | Owner posture                                                                              |
| -------------- | -------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| D1             | Kernel term representation | **(B)** `Rc` children + a pointer-equality fast path, adopted now (before B2.3 bakes bridge/corpus/exit-gate against the representation)                                         | **owner-veto** (gandr-3ln plane 1)                                                         |
| D2-format      | Export v1 sharing          | One per-artifact **tagged** subterm table over all four families, **maximal** structural sharing, declaration-segmented, decode-retains-sharing with an **expanded-work budget** | ratify (E5 bump-vs-amend is the one open sub-call)                                         |
| D2-compression | Compression posture        | Compression is a storage/transport concern, never a format concern; canonical bytes remain THE bytes                                                                             | ratify now (already ratified by direction, gandr-3ln)                                      |
| D4             | CAS revival                | **Layered both**: inner canonical declaration encoding with the subterm table (E4 plane) + outer prolly-style keyed Merkle over declaration records (untrusted plumbing)         | ratify; placement (`storage-*` tier) and the `storage-chunker` name already owner-ratified |

The single load-bearing consequence that governs the reader design: **decode-retains-sharing moves the billion-laughs attack from memory to checker time.** A shared DAG walked by the recursive S1 checker re-checks each shared subterm once per reference — exponential work from a small artifact, uncaught by the depth budget.
The reader's expanded-work budget is the S1-priced defence (§4.4).

---

## 2. Ground truth — the kernel-core surface as built

Every code-anchored claim in the proposal, re-verified against the live worktree.
Slice 3 of the minimal certified kernel is `crates/kernel-core`, `#![no_std]` over `core`/`alloc`, depending only on `gandr-kernel-strata` (`crates/kernel-core/docs/STATUS.md:78`, verified).

### 2.1 The four term/type enums D1 changes — all `Box`-childed today

| Enum          | File                                     | Child positions (all `Box`)                                                                                                                                     | Leaves (no children)                      |
| ------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| `Value`       | `crates/kernel-core/src/term.rs:98–125`  | `Pair(Box,Box)`, `Injection(Side,Box)`, `Thunk(Box<Computation>)`, `Lift{target,body:Box}`                                                                      | `Variable`, `Constant`, `Unit`, `Literal` |
| `Computation` | `crates/kernel-core/src/term.rs:138–161` | `Lambda(Box)`, `Application(Box,Box<Value>)`, `Return(Box<Value>)`, `Bind(Box,Box)`, `Force(Box<Value>)`, `Case{scrutinee:Box<Value>,on_left:Box,on_right:Box}` | —                                         |
| `ValueType`   | `crates/kernel-core/src/types.rs:36–64`  | `Product(Box,Box)`, `Sum(Box,Box)`, `Thunk(Box<CompType>)`, `Lift{inner:Box,target}`                                                                            | `Base`, `Unit`, `Universe`                |
| `CompType`    | `crates/kernel-core/src/types.rs:75–90`  | `Returner(Box<ValueType>)`, `Arrow{domain:Box<ValueType>,codomain:Box}`                                                                                         | —                                         |

All four derive `#[derive(Clone, Debug, Eq, Hash, PartialEq)]` and import `use alloc::boxed::Box` (`term.rs:19`, `types.rs:21`).
**Verified.** These are exactly the enums whose child positions D1(B) would retype from `Box` to `Rc`; the public enum fields are the API change B2.3 must not bake against — the sequencing rationale.

### 2.2 Claims the proposal rests on — confirmed

| Proposal claim                                                   | Verdict       | Anchor                                                                                                                                                                                                                                                                                                                                                                         |
| ---------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Box trees force **deep `Clone` on error paths**                  | **Confirmed** | `conv.rs:227–229`, `252–254`: `convert_value_type`/`convert_comp_type` `.clone()` both root types to build the mismatch error on divergence. Under `Box`, this is a deep copy; under `Rc`, `Clone` is a refcount bump (O(1)) — closes the hazard structurally.                                                                                                                 |
| Conversion is already **iterative over a heap worklist**         | **Confirmed** | `conv.rs:271–345` (`converge_types`), `358–456` (`converge_terms`) — explicit `Vec` stacks; the module doc (`conv.rs:41–46`) states the derived `PartialEq` would recurse and overflow.                                                                                                                                                                                        |
| The export writer/reader term & type codecs are **iterative**    | **Confirmed** | writer: `write.rs:239–291` (`encode_value_type`), `307–386` (`encode_value`) over an explicit stack; reader: `read.rs:627–720` (`decode_type_tree`), `814–956` (`decode_term_tree`) over an explicit frame worklist.                                                                                                                                                           |
| The **checker is recursive**, bounded only by a **depth** budget | **Confirmed** | `check.rs:71` `Depth::LIMIT = 512`; recursive descent `synth_value`/`check_value`/`synth_comp`/`check_comp`/`value_type_level`/`comp_type_level` (`check.rs:362,436,513,630,269,319`). The budget bounds descent **depth**, not **total work** — a shallow-but-wide DAG is not bounded by it. This is why decode-retains-sharing needs a separate expanded-work budget (§4.4). |
| **No `Rc` and no `Drop` impl** exist in kernel-core today        | **Confirmed** | Repository grep for `\bRc\b`/`alloc::rc`/`impl Drop` over `crates/kernel-core/src` returns nothing. So D1(B)'s `Rc` retype and the `gandr-i3i` worklist-`Drop` hardening are both net-new.                                                                                                                                                                                     |
| Kernel is **single-threaded** (⇒ `Rc`, not `Arc`)                | **Confirmed** | `#![no_std]` over `core`/`alloc`, no threading surface (`STATUS.md:78`); `alloc::rc::Rc` is available.                                                                                                                                                                                                                                                                         |

### 2.3 The v0 export format as built (the v1 baseline)

Verified against `crates/kernel-core/src/export.rs`, `export/write.rs`, `export/read.rs`.

* **Header.** `MAGIC = *b"GKX1"` (4 bytes, `export.rs:82`) — the trailing `1` is a **v-family** marker, independent of the version field.
  Then the format version as a `u16` **big-endian** (`FORMAT_VERSION_V0 = 0`, `export.rs:85`; `write.rs:123` `to_be_bytes`; `read.rs:304–315` `from_be_bytes`).
  Then the R4 reserved minted-atom-table count (uvarint `0`, `write.rs:125`), then the declaration count (uvarint), then the declaration records (`write.rs:119–131`).
* **Declaration record** (`write.rs:145–183`): admission-mark byte (`ADMISSION_CHECKED=0`/`ADMISSION_UNCHECKED=1`), kind byte (`KIND_DEF=0`/`KIND_AXIOM=1`; the R1 reserved kinds `2..=5` are rejected distinctly, `read.rs:451–461`), R2 structured-name segment count (uvarint `0`), the level signature, then the content — `Def`: value type, value body, four R3 annotation slots (each uvarint `0`); `Axiom`: value type.
* **Levels are inline** (`write.rs:207–222`): constant part (uvarint), atom count (uvarint), then each atom as `(variable index uvarint, offset uvarint)` in `BTreeMap`-sorted order.
  The decode-time offset cap `MAX_DECODED_LEVEL_OFFSET = 4096` (`export.rs:98`; `read.rs:560`) bounds `succ`-reconstruction work — the same totality posture as the checker's depth budget.
* **Type/term streams** are preorder, tag-then-children: `TYPE_* = 0..=8` and `TERM_* = 0..=13` (`export.rs:147–193`).
  **Note the two tag spaces overlap** (both 0-based, in separate streams) — a unified subterm table must resolve this (§4.5).
* **Varints** are canonical minimal unsigned LEB128 (`write.rs:465–480`), with the reader rejecting overlong/out-of-range encodings (`read.rs:343–371`: `count > 10`, `shift == 63 && low > 1`, and a trailing zero-continuation are each `Malformed{Varint}`).
* **E4 is enforced by whole-artifact re-encode-compare** (`read.rs:177–181`): `decode` rebuilds the sequence through the domain constructors, re-encodes via the shared `encode_artifact`, and rejects `encode_artifact(decoded) != bytes` as `Malformed{NonCanonical}`.
  Levels, constraints, and literals additionally rebuild through the strata/base smart constructors so a non-canonical value is unrepresentable in memory (`read.rs:8–34`, `537–589`, `996–1058`).
* **Rejection triple** is a closed `DecodeError` vocabulary — `Truncated`, `UnknownTag{site,tag}`, `Malformed{site}` — plus the named refusals `ReservedDeclarationKind`, `ReservedSlotOccupied`, `UnsupportedVersion` (`export.rs:452–491`).
  `read()` unions decode-plane and re-admission-plane failures as `ReadError` (`export.rs:534–541`).
* **Zero external consumers.** B9 is unbuilt, so v0 artifacts have no downstream reader — the fact behind the E5 bump-vs-amend open call (gandr-bvf description).

**Declaration shape for segmentation** (`decl.rs:82–186`, verified): `Declaration = LevelSignature + DeclarationContent`, where `DeclarationContent = Def{declared: ValueType, body: Value} | Axiom{declared: ValueType}`.
So a v1 declaration segment carries its level signature (inline, unchanged) plus **root references** into the subterm table — a declared-type root for both kinds, and a body root for `Def`.

### 2.4 The mach prolly-bao anchors (read-only, `@ fb78601`)

The reuse basis for D4.
Load-bearing claims spot-verified this pass; the checkout is at `fb78601` as pinned.

| Claim                                                                                                                    | Verdict       | Anchor                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------ | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Three crates: `prolly-bao`, `prolly-bao-chunker`, `prolly-bao-cli`                                                       | **Confirmed** | `mach @ fb78601:Cargo.toml` workspace members                                                                                                                                                                                                             |
| Chunker is **no_std, zero runtime deps**                                                                                 | **Confirmed** | `mach @ fb78601:crates/prolly-bao-chunker/Cargo.toml` — `[dependencies]` is empty; `blake3`/`criterion`/`fastcdc` are `[dev-dependencies]` only                                                                                                           |
| `PARAMETER_COMMITMENT_LEN = 85` bytes; fixed-order BE fields; 8-byte magic                                               | **Confirmed** | `mach @ fb78601:crates/prolly-bao-chunker/src/lib.rs:27` (`0x55`), `:30–60` (field offsets; `COMMITMENT_FORMAT_VERSION = 0x0001`); magic bytes at offsets `0x00..=0x07`. The literal magic string (proposal: `MPBCHK01`) was not byte-verified this pass. |
| `RecordBoundaryRule::BETWEEN_RECORDS`; fail-closed on unsorted input                                                     | **Confirmed** | `mach @ fb78601:crates/prolly-bao-chunker/src/lib.rs:618–659`; `crates/prolly-bao/src/error.rs:16` `UnsortedInput` (the `DuplicateKeys` sibling is carried from the reuse determination, not re-read this pass)                                           |
| Node identity = `BLAKE3(encoded node bytes)`, magic `prolly-bao:node:v1`                                                 | **Confirmed** | `mach @ fb78601:crates/prolly-bao/src/proof.rs:45` (`NODE_MAGIC = b"prolly-bao:node:v1"`), `:57` (`HASH_ALGORITHM_BLAKE3 = 0x01`), `:109` (`BLAKE3(encoded_node_bytes)` identity)                                                                         |
| Deps: `blake3 1.8.5`, `thiserror 2.0.18`, `bao 0.13.1` (dev-only in prolly-bao), `iroh 1.0.0-rc.0` (consumed by nothing) | **Confirmed** | `mach @ fb78601:Cargo.toml:33,34,42,53`                                                                                                                                                                                                                   |
| License Apache-2.0 (same owner) — no obstacle                                                                            | **Confirmed** | `mach @ fb78601:Cargo.toml:25` `"Apache-2.0 OR Apache-2.0 WITH LLVM-exception"`                                                                                                                                                                           |

Carried from the reuse determination as coordinator synthesis, not independently re-verified this pass: `~17.5k LOC`; the two-level-tree ceiling; canonicality by-construction-not-spec-asserted; anti-boundary-grinding unimplemented; the monolithic (full-rebuild) witness verifier; the absence of a persistent store backend; PB-ADR-0009's iroh rejection.

---

## 3. Decision D1 — kernel term representation (owner-veto)

**Recommendation: (B) `Rc` children with a pointer-equality fast path, adopted now.**

### 3.1 The options

* **(A) Box trees (status quo).** Zero change.
  Costs: O(size) recomputation on repeated subterms; deep `Clone` on the conversion error paths (§2.2); and — decisive — **decode-retains-sharing is impossible**.
  Without a shared in-memory form, the v1 reader would have to expand the DAG into a tree, which is precisely the amplification surface v1 exists to close.
  Reject: it forecloses the format decision it is supposed to be independent of.
* **(B) `Rc` children (recommended).** Replace `Box` with `alloc::rc::Rc` in the four enums' child positions (§2.1).
  On immutable data, pointer equality implies structural equality, so a `Rc::ptr_eq` short-circuit is a **trivially sound** conversion fast path that admits **no table into the TCB** — the Lean-kernel `is_eqp`-first posture (digest §2.1), exactly ADR-50 B's "ptr-eq first".
  `Clone` becomes O(1) (closes the deep-clone hazard structurally).
  Single-threaded ⇒ `Rc`, not `Arc` (document it).
  Costs: two refcount words per node; constructors take `Rc::new` (the public-field API change).
  The worklist-`Drop` obligation stands (§3.3).
* **(C) Arena indices (`u32` ids + an intrusive cached `u64` word — the Lean/ADR-50 end-state).** The right S2+/performance-era answer, but a whole-crate TCB restructure with **no S1 consumer for the cached word** (S1 conversion is type-only and structural — `conv.rs:1–27`, `STATUS.md:17–19`).
  Reject **for now**; record as the S2-era successor, with Lean's intrusive cached-word (hash + flags + `looseBVarRange`, O(1) guards) as the named template.

### 3.2 Two soundness clarifications the retype must respect

* **The fast path is a new `Rc::ptr_eq` short-circuit, not the derived `PartialEq`.** `Rc<T>`'s derived `Hash`/`Eq`/`PartialEq` deref to the _pointed-to value_ (structural), not to pointer identity.
  So the derived instances keep their current structural meaning after the retype — the writer's content-keyed dedup (§4.2) still works unchanged — and the ptr-eq fast path is an _added_ early-out in `converge_types`/`converge_terms` (`Rc::ptr_eq(a,b) ⇒ Convertible`, skip the subtree), sound because immutability makes ptr-eq ⊆ structural-eq.
* **Sharing is PRESERVED, never CREATED, by the kernel (C3).** No hash-consing/interning table enters the TCB.
  Any sharing-*creation* pass is elaborator-side; `core-checker`'s `intern.rs` (content keys, reflexive/aliased short-circuits, structural-agreement differentials) is the existing home.
  The kernel only _retains_ the sharing the decoder hands it (§4.4).

### 3.3 The worklist-`Drop` obligation (gandr-i3i)

Deep uniquely-owned `Rc` chains still recurse on `Drop` (the Lean `lean_dec_ref_cold` hazard, gandr-3ln 21:06): the reader can build an arbitrarily deep DAG from adversarial bytes, reject it at check time, then stack-overflow _dropping_ it.
Iterative decode and iterative conversion do not cover deallocation.
The fix is an iterative `Drop` per the Lean recipe; with `Rc` it **gates extraction on uniqueness** (`Rc::into_inner` yields the child only at refcount 1 — a shared handle merely decrements), so the worklist walks the uniquely-owned spine and stops at the first shared node.
This is gandr-i3i's scope; D1(B) makes it a hard prerequisite of the same landing rather than a deferred residual.
**Under D1(A) the same hazard exists for `Box` chains** — the retype does not create it, but it does bring it forward.

---

## 4. Decision D2 — export v1 sharing format (the inner E4 plane)

### 4.1 The subterm table (proposal (a)–(d), formalized)

One **per-artifact** subterm table of **tagged** nodes covering all four families in a single index space (types share heavily _across_ declarations; per-family tables forfeit that, and a tag byte per entry is the accepted cost).
Entries reference children by uvarint table index and may reference **only strictly-earlier indices** — acyclicity and topological order by construction, streaming-decodable, prefix-preserving.
The table is **declaration-segmented**: each declaration's section carries the entries it introduces (indices stay global-sequential) plus its root references, so per-declaration byte segments are self-delimiting — the record grain the outer CAS layer chunks on — and editing declaration _k_ invalidates only suffix segments (the E2 replay grain and the plane-4 prefix-cache grain).
Cross-version chunk sharing degrades under index renumbering beyond an edit point — **accepted and recorded**; the alternative (content-addressed 32-byte child references) is a size non-starter, and prefix stability is what the actual consumer needs.

### 4.2 Maximal sharing, writer-computed, C3-safe (proposal (b))

Sharing is **maximal under structural equality**, computed by the writer bottom-up:

1. Walk each declaration in admission order; within it, **post-order** (children before parent — see §4.3).
2. To intern a node: intern its children first (obtaining their global indices), form a **shallow** key `(node_tag, [child indices], canonical inline payload bytes)`, look it up in a writer-local `HashMap<Key, index>`; reuse if present, else assign the next global index and append the entry to the _current_ declaration's segment.

The key is shallow (children are already indices), so interning is O(1) amortized per node and the whole pass is O(total nodes) — the Lean `ShareCommon` boundary pass.
Canonical bytes are thus a function of the **abstract** environment, not of incidental in-memory sharing history: dedup is **content-keyed, never ptr-keyed** (a ptr-keyed dedup would make the bytes depend on decode history and break E4 determinism).
**C3-compatibility argued explicitly:** the writer's dedup map feeds no judgment and is not a trusted fast path — the reader's whole-artifact **re-encode-compare** (the v0 posture, retained and generalized, §4.6) is what enforces canonical form, so a buggy or malicious writer is caught, never trusted.

### 4.3 Table order — a sharpening: post-order first-completion, not preorder

The proposal states two clauses that cannot both hold literally: "children referenced only by **strictly earlier** indices" (4.1) and "table order = **first occurrence in the canonical preorder** walk" (proposal (c)).
A preorder walk emits a parent **before** its children, contradicting strictly-earlier-children.
The consistent rule — and the one "streaming-decodable" + "prefix property survives" already imply, since a streaming reader must have a node's children built before it constructs the node — is:

> **Index assignment order = post-order first-completion** over declarations in admission order.
> A node receives the next free global index the first time its post-order completion is reached (which is after all its children).
> This is deterministic and unique, gives children-before-parent by construction, and preserves the proposal's intent (the _first_ completion of a maximally-shared node is its canonical occurrence).

This is a formalization fix, not a redesign; §12 RQ-8 asks the owner to confirm the corrected wording.
All later machinery (the strictly-earlier invariant, the one-pass expanded-work computation, the re-encode-compare) depends on it.

### 4.4 Reader budgets — the amplification defence (proposal (e), sharpened)

**The load-bearing analysis (previously unstated in the format plane).** Decode-retains-sharing moves billion-laughs from memory to **checker time**: the recursive S1 checker (§2.2) walks the decoded DAG as a tree, re-checking each shared subterm once per reference — up to exponential work — and the depth budget (`Depth::LIMIT = 512`) bounds descent depth, **not** the width-driven expansion, so it does not catch a shallow-but-wide DAG.
Lean answers with ptr-keyed kernel caches; that is a **C3-priced trusted table we do not take at S1**.
Instead the v1 reader carries an **expanded-work budget**, enforced _before_ replay:

* `expanded_size(i)` = `1 +` saturating-sum over `i`'s child indices `j` of `expanded_size(j)`, as a **saturating `u64`**, **memoized** per entry.
  Because indices are post-order (§4.3), a single forward scan over the table computes the whole vector in O(entries).
* Reject any declaration whose **declared-type root or body root** has `expanded_size` exceeding `MAX_EXPANDED_TERM_WORK`.
  This bounds checker time without touching the checker (cheap, deterministic, one pass over the table).
* `MAX_TABLE_ENTRIES` caps the table size (truncation-cheap, enforced as entries accrue).
* Per-entry child count is **implicit in the node tag** (fixed arity).
* `MAX_DECODED_LEVEL_OFFSET = 4096` carries over unchanged (levels stay inline, §4.7).

Decode **retains** sharing (builds each entry once as `Rc`, references clone the handle) and **never expands**.
**Synergy with D1 worth stating:** because canonical decode produces the maximally content-shared form, structurally-equal subterms _are_ the same `Rc` after decode, so D1's `Rc::ptr_eq` fast path fires maximally on decoded artifacts.
Sharing-aware checking (a ptr-keyed memo _in_ the checker) is deferred to the S2 conversion design pass, where gandr-3ln plane 3 already demands the fast-path posture be stated against C3.
**The budget constants are ratification items (§12 RQ-5); D3 telemetry floors (B2.3) tune them with data.**

### 4.5 The concrete v1 byte-layout draft

Consistent with v0's tag-constant and canonical-LEB128 conventions (§2.3).
Deltas from v0 are marked **[v1]**.

**Header** (v0 shape, one field changes):

```text
MAGIC            : 4 bytes  = "GKX1"        (unchanged; the "1" is the v-family marker)
version          : u16 BE   = 1             [v1] (was 0; v0 artifacts now refused, §4.8)
minted_atom_table: uvarint  = 0            (R4, unchanged)
decl_count       : uvarint  = N
segment[0..N]                               [v1] N declaration segments (below)
```

**Declaration segment** (self-delimiting; the CAS record grain):

```text
admission_mark   : u8       (0 checked | 1 unchecked-bypass)          (unchanged)
kind             : u8       (0 Def | 1 Axiom; R1 kinds 2..=5 refused) (unchanged)
name_segments    : uvarint  = 0             (R2 structured name, unchanged)
level_signature  : params uvarint, constraint_count uvarint,
                   then constraints (relation u8 + two inline levels)  (unchanged)
entry_count      : uvarint  = m_k           [v1] entries THIS segment introduces
entry[0..m_k]                               [v1] subterm-table entries (below), global-indexed
root_declared    : uvarint                  [v1] table index of the declared value type
root_body        : uvarint                  [v1] table index of the body value  (Def only)
def_annotations  : 4 × uvarint = 0          (R3 slots, Def only, unchanged)
```

The global index counter runs across segments; segment _k_'s entries occupy a contiguous run continuing from segment _k-1_.
A child index in a segment-*k* entry may point into any earlier segment (cross-declaration sharing) or earlier within _k_, but is always **strictly less than the entry's own global index**.

**Subterm-table entry** — a single **unified node tag byte** (proposal's "a tag byte per entry") followed by inline payload and child index references.
One byte disambiguates family/polarity _and_ former; the v0 `TYPE_*`/`TERM_*` overlap (§2.3) is resolved by a single disjoint enumeration (23 formers, one byte).
Polarity is recoverable from the tag alone — replacing v0's `expect_value_term`/`expect_comp_term` (`read.rs:722–744,958–980`) with a table-lookup check that each child index resolves to an entry of the polarity its parent slot requires.

| Unified tag (proposed) | Family / polarity | Inline payload                        | Child indices (uvarint each)                  |
| ---------------------- | ----------------- | ------------------------------------- | --------------------------------------------- |
| `NODE_VT_BASE`         | value-type        | base-type atom u8                     | —                                             |
| `NODE_VT_UNIT`         | value-type        | —                                     | —                                             |
| `NODE_VT_UNIVERSE`     | value-type        | inline level                          | —                                             |
| `NODE_VT_PRODUCT`      | value-type        | —                                     | 2 (value-type, value-type)                    |
| `NODE_VT_SUM`          | value-type        | —                                     | 2 (value-type, value-type)                    |
| `NODE_VT_THUNK`        | value-type        | —                                     | 1 (comp-type)                                 |
| `NODE_VT_LIFT`         | value-type        | inline target level                   | 1 (value-type inner)                          |
| `NODE_CT_RETURNER`     | comp-type         | —                                     | 1 (value-type)                                |
| `NODE_CT_ARROW`        | comp-type         | —                                     | 2 (value-type domain, comp-type codomain)     |
| `NODE_V_VARIABLE`      | value             | de Bruijn index uvarint               | —                                             |
| `NODE_V_CONSTANT`      | value             | admission index uvarint               | —                                             |
| `NODE_V_UNIT`          | value             | —                                     | —                                             |
| `NODE_V_LITERAL`       | value             | literal (kind u8 + canonical payload) | —                                             |
| `NODE_V_PAIR`          | value             | —                                     | 2 (value, value)                              |
| `NODE_V_INJECTION`     | value             | side u8                               | 1 (value)                                     |
| `NODE_V_THUNK`         | value             | —                                     | 1 (computation)                               |
| `NODE_V_LIFT`          | value             | inline target level                   | 1 (value)                                     |
| `NODE_C_LAMBDA`        | computation       | —                                     | 1 (computation)                               |
| `NODE_C_APPLICATION`   | computation       | —                                     | 2 (computation head, value arg)               |
| `NODE_C_RETURN`        | computation       | —                                     | 1 (value)                                     |
| `NODE_C_BIND`          | computation       | —                                     | 2 (computation, computation)                  |
| `NODE_C_FORCE`         | computation       | —                                     | 1 (value)                                     |
| `NODE_C_CASE`          | computation       | —                                     | 3 (value scrutinee, computation, computation) |

The concrete byte values are a ratification detail (§12 RQ-6); a contiguous `0x00..=0x16` assignment in the table order above is proposed, banded by family, with future formers extending the enumeration under a later version (E5 makes that safe).
`Value::Constant` (an admission-index reference into the environment) and the new subterm-table indices are **distinct addressing spaces** — do not conflate them.

### 4.6 Canonical form (E4) — the re-encode-compare, generalized

The v0 mechanism (`decode` re-encodes the recovered artifact and rejects any mismatch, `read.rs:177–181`) carries over and now enforces the v1 canonical conditions **for free**, because the maximal-sharing writer is the re-encoder.
A v1 artifact is canonical iff:

1. every varint minimal; every inline level canonical with sorted atoms; every literal canonical (v0 conditions, retained);
2. **[v1]** the table is **maximally shared** — no two distinct entries are structurally equal (else re-encode merges them → fewer entries → byte mismatch → `NonCanonical`);
3. **[v1]** entries are in **post-order first-completion order** (§4.3) — any permutation re-encodes to a different index assignment → reject;
4. **[v1]** **no dead entries** — every entry is reachable from some declaration root (an unreferenced entry re-encodes away → reject);
5. **[v1]** every child index is strictly less than its entry's own global index (acyclicity, checked structurally during decode, before re-encode).

**One sharpening the implementer must not miss:** the re-encoder must itself be **sharing-aware** (a memoized DAG walk, O(entries)), or the canonical check is itself an amplification vector.
The expanded-work budget (§4.4) runs first and bounds it, but the re-encode should be DAG-aware on its own terms.

### 4.7 Reservations and levels (proposal (f))

Levels stay **inline** (small, already capped by `MAX_DECODED_LEVEL_OFFSET`; no sharing benefit).
All seven ratified reservations carry over unchanged and format-plane-only: the R1 reserved declaration-kind tags, the R2 structured name (segment count `0`), the four R3 per-`Def` annotation slots, and the R4 minted-atom table.
Their rejection arms (`ReservedDeclarationKind`, `ReservedSlotOccupied`) are unchanged.

### 4.8 E5 posture (proposal (g)) — the one open format sub-call

**Recommendation: a real version bump to v1** (magic unchanged, `version = 1`, v0 artifacts refused) even though v0 has zero external consumers — it exercises the E5 refusal machinery live (`expect_version` → `UnsupportedVersion`, `read.rs:304–315`) and keeps the v0 goldens as **refusal fixtures**.
**Amend-in-place** (reuse `version = 0`, redefine the v0 bytes) is the recorded alternative — cheaper, but wastes the one chance to prove E5 refusal against a real predecessor and leaves no v0/v1 boundary.
This is an **open owner call** (§12 RQ-2); the E5 mechanism is identical either way, only the version constant and the goldens' role differ.

---

## 5. Decision D2 — compression posture (ratify now)

Compression is a **storage/transport** concern, never a format concern.
The canonical bytes remain **THE** bytes — identity, provenance, replay input.
Any `zstd` (or other) codec lives **outside** the reader, which always parses uncompressed canonical bytes so the rejection triple stays clean.
No design or code work is needed; the doc records it for ratification.
Already ratified by direction (gandr-3ln 21:16); listed here for completeness of the ratification package (§12 RQ-7).

---

## 6. Decision D4 — CAS revival (the outer plane)

**Recommendation: layered both** (the gandr-bvf 21:31 convergence).
The container-vs-format fork dissolves because prolly-bao shares at **chunk/declaration** granularity across artifact versions while the subterm table shares at **subterm** granularity _within_ declarations — they are **complementary layers**, not alternatives.

* **Inner layer (E4 plane).** The canonical declaration encoding with the subterm table (§4) — the validating reader's input; the trusted structural-validation wall.
* **Outer layer (CAS / verified-access / sync plane; untrusted plumbing).** A prolly-style keyed Merkle tree over declaration **records**.
  The decisive fact: gandr's export artifact **is a sorted, unique, keyed record set by construction** — E2 admission ordering keys declarations by admission index, satisfying the analyst's conditional ("fits only if artifacts are modeled as sorted keyed records").
  Records = `(admission index, fixed-width big-endian key → declaration segment bytes)`; record-safe chunking = declaration-granular chunking, exactly the E2 replay grain and the plane-4 prefix-cache grain.
* **Artifact identity** = `BLAKE3` of a root manifest binding `{chunker params commitment (the 85-byte pattern, §2.4), record count, root node hash, inner format version}` — the b3sum-provenance successor at B2.3.

**Hard boundary, restated verbatim.** Integrity never substitutes validity.
Hash verification is the **outer** wall; K2/E3 replay re-checks **everything** from the canonical inner bytes (`read()` → `add_decl`); hashes are **untrusted plumbing**.
The two-wall pattern also answers the rkyv spike's zero-copy question (gandr-bvf 22:07): provenance verification = outer integrity wall; structural validity needs its own inner wall (schema-version commitment in the hashed bytes + a bytecheck posture) — one discipline, applied twice.

### 6.1 Vendor plan (placement owner-ratified)

Absorb mach prolly-bao (owner's own unpublished code — direct source absorption with a provenance note, no external-dep ceremony) into a **new `storage-*` tier** (owner-ratified 21:51 — an engineering/storage substrate deliberately separate from the math `theory-*` tier; plausible future sibling `storage-rkyv`).
Untrusted scaffolding by the kernel-boundary naming rule (only `gandr-kernel-*` is trusted).

* **`storage-chunker`** (name owner-confirmed 22:07): the `no_std`, zero-runtime-dep chunker (§2.4).
  Its `PARAMETER_COMMITMENT_LEN = 85` fixed-order commitment is the **highest-leverage lift** — exactly the "deterministic chunking parameters pinned in the format spec" primitive E4 canonicality demands, and the template for how v1 should pin _all_ its parameters.
* **`storage-prolly-trees`** (alloc): `ProllyTree`, the membership/non-membership/range proofs + witness machinery, `BlockStore` + `InMemoryBlockStore` + `PackedSegmentStore`.
  Preserve the two-crate split (keep the chunker's no_std zero-dep property visible at the crate boundary); drop the POC CLI (dogfood value lives in the contract suites).
* **Extensibility (owner directive).** Adapt gandr-first but design for reuse (the wyrd harness may be rebuilt as something gandr _implements_): keep the generic sorted-record interface (the export layer is a **consumer** supplying the declaration record model; **no declaration semantics in the tree crate**); keep the full proof machinery **feature-gated**, not stripped; keep the store trait generic over blob kinds (node-decode as a mode, not a fork); when the multi-level tree lands it goes in the generic crate, never the gandr-specific layer; carry the per-crate doc sets + contract suites through absorption.
* **Absorption checklist.** Commitlint scope entries for the new crates **before first commit** (B2.1 precedent; mind the wvd.23 half-migrated-constants trap), workflow-gates crate-roster entries, crate-port-map/doc-sync rows.
  Reconcile the `blake3` pin (mach `1.8.5`; gandr already carries `blake3` + `thiserror` as workspace deps).
* **Inherited-deferred, carried honestly.** Multi-level tree construction + its proofs (the scale ceiling); the spec-asserted history-independence differential (E4 discipline requires the spec pin + a differential; mach's canonicality is by-construction, not a named theorem); incremental/streaming witness verification (mach's is full-rebuild; depend on `bao 0.13.1` directly for verified streaming); a persistent store backend; anti-boundary-grinding hardening (mach has hard byte/record caps only — acceptable short-term since the writer is ours).

---

## 7. The property-suite delta (deliverable 3)

The current suite (82 tests, `STATUS.md:38`) is the baseline.
Inventory verified in `crates/kernel-core/tests/{export,conversion,checker}.rs`.

**Retained, must still pass unchanged in spirit** (v1-adapted where they touch bytes):

* Round-trip: `the_empty_environment_round_trips`, `round_trip_reproduces_the_environment`, `audits_agree_after_the_round_trip`, `a_bypass_admission_survives_the_round_trip`, and the property `round_trip_reproduces_arbitrary_declarations` (`export.rs:221,239,264,303,769`).
* Determinism: `a_second_write_is_byte_identical` (`export.rs:348`) and the property `write_is_deterministic` (`export.rs:808`).
* Rejection triple + named refusals: `an_unknown_version_is_refused`, `each_reserved_declaration_kind_is_rejected`, the R2/R3/R4 slot rejections, `a_non_canonical_level_encoding_is_rejected`, `a_non_canonical_literal_encoding_is_rejected`, `a_non_digit_magnitude_is_rejected`, `an_overlong_varint_is_rejected`, `a_corrupted_tag_is_rejected`, `truncation_at_every_prefix_is_rejected`, and the property `arbitrary_bytes_never_panic` (`export.rs:359–573,823`).
* Conversion properties (`conversion.rs:176–229`) and `prop_the_choke_point_is_total` (`checker.rs:354`) — unchanged; the D1 retype must not perturb them.

**New for v1 (the delta):**

1. **Sharing round-trip.** An environment whose declarations share subterms (across and within declarations) round-trips byte-identically, and the decoded in-memory form **retains** the sharing (assert `Rc::ptr_eq` on the shared nodes — or, if D1(A), a structural-equality witness).
2. **Sharing determinism / maximality.** Two structurally-equal-but-differently-`Rc`-shared inputs write to **identical** bytes (content-keyed, not ptr-keyed, §4.2).
3. **Canonical-form rejections [v1]:** a non-maximally-shared table (a redundant duplicate entry), a mis-ordered table (non-post-order), a dead (unreferenced) entry, and a forward/self child reference each reject as `Malformed{NonCanonical}` or the acyclicity arm.
4. **Amplification goldens (the point of v1):** a small artifact whose _expanded_ size is astronomical (a repeated-diamond DAG) is **rejected** by `MAX_EXPANDED_TERM_WORK` **before** replay — and, as a differential, is confirmed to _not_ reach the checker; a table exceeding `MAX_TABLE_ENTRIES` rejects.
5. **Sharing-retention budget:** an artifact at the boundary of each budget constant (`MAX_TABLE_ENTRIES`, `MAX_EXPANDED_TERM_WORK`) — accept just under, reject just over (the `MAX_DECODED_LEVEL_OFFSET` golden posture generalized).
6. **E5 boundary [v1, if RQ-2 = bump]:** a v0-magic/v0-version artifact is refused `UnsupportedVersion{found:0}`, with the old v0 goldens repurposed as the refusal fixtures.
7. **Worklist-`Drop` totality (gandr-i3i):** a decoded-then-rejected deep DAG drops without stack overflow (the targeted deep-tree drop test).

`storage-chunker` / `storage-prolly-trees` arrive as a **skeleton** in gandr-5t3 (crates + their contract suites land; **no** wiring into the export path), so their contract suites carry through absorption but do not yet exercise the gandr record model.

---

## 8. Staging — what gandr-5t3 lands (and what it does not)

Per the owner sequencing directive (representation and format changes land while surface area is minimal — B2.3 would bake corpus/exit-gate/bridge against them):

* **gandr-5t3 (this ratification's implementation):** the D1 representation change (per RQ-1); the inner v1 writer/reader with decode-retains-sharing + the budgets; the updated round-trip/rejection/determinism property suites + the sharing-retention and amplification goldens (§7); the worklist-`Drop` hardening (gandr-i3i, now a prerequisite under D1(B)); and the `storage-chunker` + `storage-prolly-trees` absorption **as skeleton only** (contract suites land; no export wiring).
* **B2.3 and later:** outer-layer wiring, the manifest identity, the D3 size/decode-time telemetry floors per corpus item, and the exit-gate harness.
* **Parallel (gandr-1hu rkyv spike):** feeds only the tree/store API surface (state-as-view: get-by-hash returning verified blobs); its findings join this ratification package.

The per-outcome staging for each decision is in the ratification queue (§12).

---

## 9. Where this doc lives, and the corpus firewall

`docs/research/` is **outside** the `docs/gandr/` corpus MANIFEST (confirmed: the `docs:manifest-drift` gate scans only MANIFEST-registered nodes, and `docs:reference-integrity` scans only the registered corpus + the ADR dir — `crates/workflow-gates/src/docs/references.rs:1–6,50`).
So this file needs no MANIFEST b3sum entry and its cross-references are not gate-checked; the citation discipline here is by hand, matching the sibling research docs.

---

## 10. Verification register — claims confirmed and corrected

* **Confirmed against kernel-core** (§2.2): deep-clone-on-error-paths, iterative conversion/codec, recursive-checker-with-depth-budget, no existing `Rc`/`Drop`, single-threaded ⇒ `Rc`.
  The full v0 byte layout and E4 re-encode-compare mechanism (§2.3).
* **Confirmed against mach @ fb78601** (§2.4): the three crates, the chunker's empty `[dependencies]` (zero runtime deps), the 85-byte fixed-order parameter commitment, `RecordBoundaryRule::BETWEEN_RECORDS` + `UnsortedInput`, the `prolly-bao:node:v1` / `BLAKE3(node bytes)` identity, the dep pins, and the Apache-2.0 license.
* **Corrected / sharpened:**
  + The proposal's table order is internally inconsistent (preorder vs strictly-earlier-children); resolved to **post-order first-completion** (§4.3, RQ-8).
  + The v0 `TYPE_*`/`TERM_*` tag spaces **overlap**; a unified table needs a single disjoint enumeration (§4.5).
  + The re-encode-compare canonical check must be made **sharing-aware** or it is itself an amplification vector (§4.6).
  + The `Rc` fast path is `Rc::ptr_eq`, distinct from the derived `PartialEq` (which stays structural) (§3.2).
* **Not independently re-verified this pass** (carried as coordinator/analyst synthesis, flagged where used): the mach `~17.5k LOC` / two-level-tree ceiling / by-construction-canonicality / anti-grinding-unimplemented claims; the `MPBCHK01` literal magic string; the chunker default byte/record thresholds; `DuplicateKeys` and PB-ADR-0009.
* **Anchor caveat:** the K/E/C/S/R discipline identifiers are cited per house convention as `spec/kernel-boundary.md` (ADR-77/78), a referenced-but-unmaterialized corpus node; each is anchored to its in-tree kernel-core rustdoc use where possible (§2, and `export.rs`/`term.rs`/`types.rs` module docs).

---

## 11. Open questions (deliverable 5)

1. **Budget constants have no data yet.** `MAX_EXPANDED_TERM_WORK` and `MAX_TABLE_ENTRIES` are set blind until B2.3's D3 telemetry floors measure real corpus sizes.
   Recommendation: pick conservative launch values (generous enough for the S1 corpus, tight enough to reject obvious billion-laughs), pin + golden-test them, and record them as D3-tunable.
   Owner input welcome on the launch magnitudes.
2. **The unified-tag byte assignment is a public wire commitment.** Once chosen it is frozen under v1 (E5 protects evolution, but churn is churn).
   The banded contiguous scheme (§4.5) is proposed; the owner may prefer reserved gaps per family for the anticipated S2 formers (description codes, `Sigma`, `Path`) to keep future tags family-local.
3. **Does the checker-time bound belong at the reader forever, or migrate into a sharing-aware checker at S2?** The reader budget is the S1 answer (no TCB table). gandr-3ln plane 3 already flags the S2 conversion pass must state its fast-path posture against C3; the ptr-keyed checker memo is the natural S2 successor, at which point the reader budget becomes a defence-in-depth outer bound rather than the sole bound.
   Not decided here.
4. **History-independence as a named theorem.** mach's prolly canonicality is by-construction, not spec-asserted.
   E4 discipline wants a spec pin + a history-independence differential (permuted insertion order → identical root).
   This is inherited-deferred (§6.1) but the owner may want it in-scope earlier if the CAS layer is wired sooner than B2.3.
5. **Two-level tree ceiling.** Fine at current corpus sizes with 16 KiB target chunks, but the scale plane the whole program targets will eventually demand the multi-level tree (and its proofs), which mach deliberately deferred.
   When does that lift become load-bearing — and does it land in `storage-prolly-trees` before or after gandr has a corpus large enough to need it?
6. **`storage-rkyv` coupling.** The gandr-1hu spike may push a `NodeHash`-reference (state-as-view) shape onto the tree/store API.
   If its findings land after gandr-5t3's skeleton, the generic store trait may need a second pass — acceptable, but worth the owner knowing the API is not frozen by the skeleton.

---

## 12. Ratification queue (deliverable 4) — numbered, for owner sign-off

Each item: the decision, its options, the recommendation, and what **gandr-5t3** does under each outcome.
RQ-numbered to avoid collision with the R1–R4 format reservations.

**RQ-1 — D1 kernel term representation (OWNER-VETO).** Options: **(A)** Box trees (status quo) · **(B)** `Rc` children + `Rc::ptr_eq` fast path · **(C)** arena indices + intrusive cached word.
Recommendation: **(B)**, adopted now. 5t3 under **(B)**: retype the four enums' child positions `Box`→`Rc` (§2.1), add the `Rc::ptr_eq` early-out in `converge_*`, land the iterative worklist-`Drop` (gandr-i3i) as a prerequisite, document `Rc`-not-`Arc`; decode builds `Rc` DAGs (enables §4 decode-retains-sharing). 5t3 under **(A)**: keep `Box`; **decode-retains-sharing is impossible**, so the v1 reader must either expand (reintroducing the amplification surface) or D2 is descoped to a non-sharing format — i.e. (A) largely forecloses D2. gandr-i3i still lands (the `Box`-chain drop hazard is independent). 5t3 under **(C)**: out of scope for this landing (whole-TCB restructure); recorded as the S2-era successor.

**RQ-2 — E5 posture: version bump vs amend-in-place (OPEN OWNER CALL).** Options: **(bump)** `version = 1`, refuse v0 · **(amend)** reuse `version = 0`, redefine the bytes.
Recommendation: **(bump)** — exercises E5 refusal live, keeps v0 goldens as refusal fixtures. 5t3 under **(bump)**: set `FORMAT_VERSION` to 1; add the v0-refusal golden (§7 item 6); repurpose existing v0 goldens. 5t3 under **(amend)**: leave the version constant at 0; drop the v0-refusal golden; no v0/v1 boundary tests.
Byte layout is otherwise identical.

**RQ-3 — subterm-table shape: one unified tagged table vs per-family tables.** Recommendation: **one unified tagged table** over all four families (cross-declaration type sharing; one tag byte per entry). 5t3 under unified: the §4.5 layout.
Under per-family: four index spaces, four tables per segment, no cross-family sharing (types still shared within their family) — smaller tag but forfeits the dominant sharing source (types across declarations).
Not recommended.

**RQ-4 — sharing degree: maximal structural vs partial/none.** Recommendation: **maximal under structural equality**, content-keyed writer dedup (§4.2). 5t3 under maximal: the content-keyed intern pass + the maximal-sharing canonical conditions (§4.6).
Under none: v1 degenerates to v0-shaped trees-in-a-table with no dedup — pointless.
No middle option is canonical (any non-maximal choice needs an arbitrary rule and breaks E4 determinism).

**RQ-5 — reader budget constants (RATIFICATION ITEMS; D3-tunable).** `MAX_EXPANDED_TERM_WORK`, `MAX_TABLE_ENTRIES` (`MAX_DECODED_LEVEL_OFFSET = 4096` unchanged).
Recommendation: conservative launch values, pinned + golden-tested at the boundary, flagged D3-tunable. 5t3: define the constants, add the boundary goldens (§7 items 4–5); B2.3 D3 telemetry re-tunes.

**RQ-6 — unified node-tag byte assignment.** Recommendation: contiguous `0x00..=0x16`, banded by family (§4.5). 5t3: freeze the `NODE_*` constants; alternative = reserved per-family gaps (owner preference, open question 2).
Either way the values are a wire commitment under v1.

**RQ-7 — D2 compression posture (ratify now).** Recommendation: **compression outside the format**; canonical bytes remain THE bytes. 5t3: no code — posture only; any codec lives in a later storage/transport layer, never the reader.

**RQ-8 — table index order (formalization fix — confirm the corrected wording).** The proposal's "preorder first occurrence" is inconsistent with strictly-earlier-child references; corrected to **post-order first-completion** (§4.3). 5t3: implement post-order completion indexing; confirm the doc wording. (No genuine alternative — preorder cannot satisfy strictly-earlier children.)

**RQ-9 — D4 CAS: layered-both + vendor plan + skeleton staging.** Recommendation: **layered both**; absorb mach prolly-bao as `storage-chunker` + `storage-prolly-trees` (placement + `storage-chunker` name already owner-ratified); skeleton-only in gandr-5t3. 5t3: land the two crates + their contract suites (no export wiring); commitlint scopes + workflow-gates roster + crate-port-map/doc-sync rows **before first commit**; reconcile the `blake3` pin.
Outer-layer wiring + manifest identity + history-independence differential are B2.3+.

---

## 13. Summary of recommendations

Adopt **D1(B)** (`Rc` + ptr-eq, now), the **unified maximal-shared declaration-segmented subterm table** with the **expanded-work budget** and a **v1 version bump**, the **compression-outside-format** posture, and the **layered-both CAS** with the `storage-chunker`/`storage-prolly-trees` vendor plan as a skeleton in gandr-5t3.
The owner's live calls are **RQ-1** (D1 veto), **RQ-2** (E5 bump vs amend), **RQ-5** (budget magnitudes), and confirmation of the **RQ-8** ordering fix; the remainder are recommended-with-alternatives-recorded or already ratified by direction.
