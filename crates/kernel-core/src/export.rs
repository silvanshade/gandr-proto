//! The K5 re-checkable export (obligations E1–E6).
//!
//! The [`write()`] writer serializes an [`Environment`](crate::Environment) to
//! canonical, self-contained bytes; the [`read()`] validating reader decodes
//! them back through the choke point. The obligations, as this module holds
//! them:
//!
//! * **E1 — self-contained.** The artifact carries names, levels, terms, and
//!   declaration order in the bytes; replay needs no access to the producing
//!   process.
//! * **E2 — declaration-granular, admission-ordered.** The artifact is the
//!   sequence of declarations in admission order; [`read()`] replays them by
//!   re-running the choke point on each in turn —
//!   [`Environment::add_decl`](crate::Environment::add_decl) for a checked
//!   mark, [`Environment::add_decl_unchecked`](crate::Environment::add_decl_unchecked)
//!   for a bypass mark.
//! * **E3 — no derived data trusted.** The precomputed transitive audit sets
//!   (`rested_on`) are **not** in the bytes; the reader recomputes them by
//!   re-admitting each declaration, so a forged audit cannot ride along (no
//!   derived data is trusted).
//! * **E4 — canonical bytes, validating reader.** The v1 format is a
//!   **maximal-sharing subterm table** per declaration segment (massive-term
//!   design §4). Encoding interns nodes bottom-up with **content-keyed dedup**
//!   (the bytes are a function of the abstract environment, never of in-memory
//!   sharing) and iterates admission order and `BTreeMap`-sorted level atoms,
//!   so an identical environment yields a byte-identical artifact. The reader
//!   is a closed-vocabulary streaming parser over the rejection triple
//!   ([`DecodeError`]: truncation, an unknown tag, or a structural violation).
//!   It **decodes through constructors** (levels/literals rebuild canonically)
//!   and mints each entry into a [`TermArena`] once (decode retains sharing).
//!   Canonical form is enforced by a **sharing-aware whole-artifact
//!   re-encode-compare**: a non-maximally-shared (duplicate), mis-ordered, or
//!   dead-entry table, or a non-canonical level/literal, re-encodes differently
//!   and is rejected [`MalformedSite::NonCanonical`]; strictly-earlier and
//!   polarity-correct child references are checked structurally during decode.
//! * **E5 — versioned, with refusal.** The version is **v1**
//!   ([`FORMAT_VERSION_V1`]); a v0 (`version = 0`) or any other version is a
//!   named refusal ([`DecodeError::UnsupportedVersion`]), exercising the E5
//!   machinery against a real predecessor.
//! * **E6 — unchecked admissions visible.** Each declaration's admission mark
//!   (checked or unchecked-bypass) is in the bytes, and `Axiom`s are visible as
//!   `Axiom` declarations, so the §3 audit survives serialization.
//!
//! # Totality and the amplification defence (§4.4)
//!
//! The reader parses adversarial bytes inside the TCB and is **total**: every
//! read is bounds-checked, every tag is matched against the closed `NODE_*`
//! set, and the subterm table is decoded **iteratively** over the flat entry
//! list, never input-scaled recursion. Because decode retains sharing, a small
//! artifact can name a DAG of astronomical *expanded* (tree) size — the
//! billion-laughs attack moved from memory to checker time. The reader carries
//! three expanded-work budgets, enforced before replay: [`MAX_TABLE_ENTRIES`]
//! (as entries accrue), [`MAX_EXPANDED_TERM_WORK`] (per declaration root), and
//! [`MAX_ARTIFACT_EXPANDED_WORK`] (the artifact-total, closing the many-cheap-
//! segments-sharing-one-root amplification) — the last two off one
//! forward scan of memoized saturating `expanded_size`. Level reconstruction is
//! bounded by [`MAX_DECODED_LEVEL_OFFSET`]. The same scan yields the
//! deterministic [`DecodeMetrics`] the export exit gate records.
//!
//! **When a change to this format bumps the version and when it holds** is
//! recorded at [`FORMAT_VERSION_V1`], under `format-decision-01`. Read it
//! before changing a tag byte, a kind byte, or a reserved slot.
//!
//! # The ratified reservations, and which of them the sealing rung filled
//!
//! * **R1 — reserved declaration kinds.** `AbstractType` / `ModuleSig` /
//!   `ModuleDef` / `FunctorDef` have concrete reserved kind tags.
//!   **`AbstractType` is now live**: the writer emits it for a
//!   [`DeclarationContent::AbstractType`](crate::DeclarationContent::AbstractType)
//!   and the reader admits it. The other three stay reserved, and the reader
//!   still rejects them as a distinct [`DecodeError::ReservedDeclarationKind`]
//!   (more honest than a generic bad tag).
//! * **R2 — structured names.** A name is a sequence of segments, never a flat
//!   dotted string. S1 declarations carry no name, so the per-declaration name
//!   record is a segment count pinned to zero at v1; a non-empty name is
//!   rejected as [`DecodeError::ReservedSlotOccupied`]. **Still reserved.**
//! * **R3 — four per-`Def` annotation slots** (erasure, modes/grades,
//!   sealing-provenance, directedness/variance). **The sealing-provenance slot
//!   is now live**; erasure, modes/grades, and directedness/variance stay empty
//!   and are rejected when occupied.
//! * **R4 — the minted-atom table**, admission-ordered. **Now live**, and it is
//!   what makes atom freshness a checked property rather than an imported claim
//!   — see the section below.
//!
//! # Freshness is checked here, and this is exactly what is checked
//!
//! An artifact minting sealed abstract types carries, in its header, the
//! admission positions of every one of them. Both the writer and the reader
//! treat that table as **redundant and therefore falsifiable**: the reader
//! decodes the declaration sequence independently and re-derives the table from
//! it, refusing any artifact where the two disagree. Three properties follow,
//! and all three are decidable from the bytes alone:
//!
//! * **distinctness** — the table is strictly ascending, so no two atoms can
//!   occupy one position and an aliased pair cannot be spelled;
//! * **accounting** — every abstract-type declaration appears, so an atom
//!   cannot be smuggled past the table;
//! * **no forgery** — every entry names a declaration that really is an
//!   abstract type, so the table cannot conjure an atom the declarations do not
//!   contain.
//!
//! Re-minting on replay is deterministic because an atom's kernel identity *is*
//! its admission position, and replay re-admits in admission order. So "these
//! atoms are fresh" is re-derived, never believed — which is the property the
//! atom route was chosen for, and the one an existential presentation offers no
//! analogue of, because it offers nothing to check.
//!
//! **What this does not establish**, stated because the gap is easy to miss:
//! this is freshness *within one artifact*. Two independently produced
//! artifacts can both mint an atom at position 0, so cross-process global
//! uniqueness is a different property that this table does not carry and does
//! not claim. It belongs with the package boundary, where sealed values first
//! cross one.

mod read;
mod write;

use core::error::Error;
use core::fmt;

pub use self::read::decode;
pub use self::read::read;
pub use self::write::write;
pub use self::write::write_segmented;
use crate::arena::TermArena;
use crate::decl::Declaration;
use crate::error::KernelError;

/// The four-byte artifact magic — Gandr Kernel eXport, v-family `1`.
pub const MAGIC: [u8; 4] = *b"GKX1";

/// The current format version tag: v1, the maximal-sharing subterm-table
/// format.
///
/// E5: a real bump from v0 — a v0 `version = 0` artifact is refused
/// `UnsupportedVersion { found: 0 }`, exercising the refusal machinery against
/// a real predecessor; the old v0 goldens are repurposed as refusal fixtures.
///
/// # `format-decision-01` — additive growth holds at v1; a reassignment bumps
///
/// **The sealing rung extended this format without bumping the version, and
/// that was deliberate.** The decision is recorded here rather than in a note
/// somewhere, because this constant is what a later seat reads when it is about
/// to change one — and reflexively bumping is the cheap-looking mistake.
///
/// The rule the rung settled on, which is the thing to carry forward:
///
/// | change                                                | version |
/// | ----------------------------------------------------- | ------- |
/// | assign a previously **unassigned** tag or kind byte    | hold    |
/// | fill a **reserved** slot that framed itself from birth | hold    |
/// | **reassign** an existing byte, or change a field's shape, ordering, or width | **bump** |
///
/// **Why holding is safe for the first two, and it is a property of the reader
/// rather than a convention.** The reader is a closed-vocabulary parser over a
/// rejection triple, so a byte it does not know is a *named refusal at a named
/// site* — `UnknownTag { site: Node, tag: 0x17 }` — never a mis-parse and never
/// a silent misreading. A reserved slot is framed identically whether it is
/// empty or full, so filling one moves no other field. In both cases an older
/// reader meeting a newer artifact stops with an accurate reason, which is
/// exactly the guarantee `UnsupportedVersion` would have bought, obtained
/// structurally instead.
///
/// **Why bumping is required for the third.** Reassigning a byte or moving a
/// field makes an older reader parse *successfully* and *wrongly* — the failure
/// mode the version field exists to prevent, and the only one it can prevent.
///
/// **Why the cost matters here specifically.** A bump refuses every artifact
/// written before it, so it re-blesses every golden and invalidates every
/// stored export. Spending that on an additive change buys nothing the refusal
/// triple does not already give, and it spends the one signal that should stay
/// meaningful for the case that genuinely needs it.
///
/// The sealing rung took `NODE_VT_ABSTRACT` (`0x17`, previously unassigned) and
/// filled the R1 abstract-type kind, the R3 sealing-provenance slot, and the R4
/// minted-atom table. Every byte assignment `0x00..=0x16` means what v1 froze
/// it to mean.
pub const FORMAT_VERSION_V1: u16 = 1;

/// The decode-time cap on the expanded (tree) size of a declaration root — the
/// amplification defence (massive-term design §4.4).
///
/// Decode retains sharing, so a small artifact can name a DAG whose *expanded*
/// (tree) size is astronomical; the recursive S1 checker walks that DAG as a
/// tree, re-checking each shared subterm once per reference, so a shallow-wide
/// DAG is exponential checker work the depth-free defunctionalized machine does
/// not bound. Before replay, the reader computes each entry's memoized
/// `expanded_size` (a saturating `u64`) in one forward scan and rejects any
/// declaration whose declared-type or body root exceeds this cap — bounding
/// checker time without touching the checker.
///
/// **Tunable; retuned once real corpus telemetry existed** from `1 << 24` to
/// `1 << 20`. The binding
/// floor is **not** the S1 corpus (max per-declaration expanded work `5` over
/// the 21-eligible + 6-golden exit-gate corpus) but the deepest artifact the
/// kernel itself round-trips: the `tests/hardening.rs`
/// `deep_decoded_declaration_drop_is_total` witness decodes a ~200k-deep
/// declaration whose expanded work is ~400k, and the reader must accept every
/// artifact the kernel legitimately admits and round-trips. `1 << 20`
/// (1,048,576) clears that ~400k floor with ~2.6× headroom (and is `> 2 ×`
/// [`MAX_TABLE_ENTRIES`], so a maximal flat table stays under it), yet rejects
/// an obvious billion-laughs (`2^30` expanded) with a 1,024× margin.
pub const MAX_EXPANDED_TERM_WORK: u64 = 1 << 20;

/// The decode-time cap on the number of subterm-table entries in an artifact —
/// the table-size defence (massive-term design §4.4), enforced as entries
/// accrue (truncation-cheap).
///
/// **Tunable; retuned on the same telemetry** from `1 << 20` to `1 << 18`.
/// This is the
/// distinct-DAG-node axis, naturally below [`MAX_EXPANDED_TERM_WORK`] since
/// sharing makes *expanded* work exceed the *distinct*-entry count, and it is
/// input-linear (each entry costs ≥ 1 wire byte) so it carries no
/// amplification. Its floor, like its sibling's, is the ~200k-entry table of
/// the deepest kernel-round-tripped artifact (the `hardening.rs` decode
/// witness), not the S1 corpus (max `6` distinct entries); `1 << 18` (262,144)
/// clears that floor. It is the budget most likely to want raising first when a
/// genuinely large theory lands.
pub const MAX_TABLE_ENTRIES: usize = 1 << 18;

/// The decode-time cap on the **artifact-total** expanded (tree) work — the
/// per-artifact amplification defence (massive-term design §4.4).
///
/// [`MAX_EXPANDED_TERM_WORK`] bounds each declaration root in isolation, but
/// nothing there bounds the *sum* across declarations: a declaration count is
/// unbounded, a declaration segment costs ~10 wire bytes at minimum, and
/// declaration roots may **share** subterm-table entries across declarations
/// (the format's cross-declaration sharing), so `N` cheap segments each
/// referencing the same near-`MAX_EXPANDED_TERM_WORK` root force `N ×
/// MAX_EXPANDED_TERM_WORK` checker work — total replay work `∝ bytes × 10^6`
/// where v0 was linear in bytes. The reader closes this by accumulating a
/// **saturating** artifact-total over every declaration root's expanded size
/// (declared-type and, for a `Def`, body) — riding the same one forward scan as
/// the per-declaration check, a single extra compare — and rejecting any
/// artifact over this cap **before replay**. Reader acceptance policy only; no
/// wire-format or E4 consequence. **Tunable** like its two siblings, set
/// to `1 << 24` (16,777,216). It clears both the S1 corpus (max
/// artifact-total expanded work `7`) and a single deepest-round-tripped
/// declaration (~400k, the `hardening.rs` decode witness) — ~42× over the
/// latter, admitting tens of such declarations — while, with
/// [`MAX_EXPANDED_TERM_WORK`] at `1 << 20`, capping a single artifact's total
/// replay work at `2^24` tree-nodes: it catches the
/// N-cheap-segments-sharing-one-near-cap-root amplification (≈16 near-cap
/// sharing roots trip it) that no per-declaration bound sees.
pub const MAX_ARTIFACT_EXPANDED_WORK: u64 = 1 << 24;

/// The decode-time cap on a level variable atom's successor offset.
///
/// A canonical level `max(c, x + o, …)` is rebuilt through the strata smart
/// constructors, and the only way to raise a variable atom to offset `o` is `o`
/// applications of `succ` (strata exposes no direct offset constructor). A
/// small adversarial varint could otherwise demand unbounded reconstruction
/// work, so an atom offset at or above this cap is rejected at decode — bounded
/// work on adversarial input, the reader's totality posture. Real universe
/// levels carry variable offsets of `0` or `1`; a level beyond the cap is a
/// documented non-round-tripping case, liftable when strata
/// exposes an `O(1)` offset constructor.
pub const MAX_DECODED_LEVEL_OFFSET: u64 = 4096;

/// Admission-mark byte: admitted through the checked choke point (E6).
pub const ADMISSION_CHECKED: u8 = 0;
/// Admission-mark byte: admitted through the warned bypass (E6).
pub const ADMISSION_UNCHECKED: u8 = 1;

/// Declaration-kind byte: a typed definition.
pub const KIND_DEF: u8 = 0;
/// Declaration-kind byte: a tracked typed hole.
pub const KIND_AXIOM: u8 = 1;
/// Declaration-kind byte: the R1 reserved abstract-type kind.
pub const KIND_ABSTRACT_TYPE: u8 = 2;
/// Declaration-kind byte: the R1 reserved module-signature kind.
pub const KIND_MODULE_SIG: u8 = 3;
/// Declaration-kind byte: the R1 reserved module-definition kind.
pub const KIND_MODULE_DEF: u8 = 4;
/// Declaration-kind byte: the R1 reserved functor-definition kind.
pub const KIND_FUNCTOR_DEF: u8 = 5;

/// Base-type-atom byte: the integer atom.
pub const BASE_INTEGER: u8 = 0;
/// Base-type-atom byte: the string atom.
pub const BASE_STRING: u8 = 1;
/// Base-type-atom byte: the numeric atom.
pub const BASE_NUMERIC: u8 = 2;

/// Literal-sign byte: non-negative (the canonical zero is non-negative).
pub const SIGN_NON_NEGATIVE: u8 = 0;
/// Literal-sign byte: negative.
pub const SIGN_NEGATIVE: u8 = 1;

/// Injection-side byte: the left injection.
pub const SIDE_LEFT: u8 = 0;
/// Injection-side byte: the right injection.
pub const SIDE_RIGHT: u8 = 1;

/// Literal-kind byte: an integer literal.
pub const LITERAL_INTEGER: u8 = 0;
/// Literal-kind byte: a string literal.
pub const LITERAL_TEXT: u8 = 1;
/// Literal-kind byte: a numeric literal.
pub const LITERAL_NUMERIC: u8 = 2;

/// Landmark-constraint-relation byte: `left ≤ right`.
pub const RELATION_LEQ: u8 = 0;
/// Landmark-constraint-relation byte: `left = right`.
pub const RELATION_EQ: u8 = 1;

// Unified subterm-table node tags (v1): one disjoint enumeration over all four
// families, contiguous `0x00..=0x16`, banded by family in the massive-term
// design §4.5 table order. **This byte assignment is a frozen wire commitment
// under v1** (future formers extend the enumeration under a later version,
// which E5 makes safe). Polarity is recoverable from the tag alone, replacing
// v0's `expect_value_term`/`expect_comp_term` with a table-lookup
// child-polarity check.

/// Node tag: value-type base atom (payload: base-type byte).
pub const NODE_VT_BASE: u8 = 0x00;
/// Node tag: the value-type unit.
pub const NODE_VT_UNIT: u8 = 0x01;
/// Node tag: the universe former (payload: inline level).
pub const NODE_VT_UNIVERSE: u8 = 0x02;
/// Node tag: the product former (children: value-type, value-type).
pub const NODE_VT_PRODUCT: u8 = 0x03;
/// Node tag: the sum former (children: value-type, value-type).
pub const NODE_VT_SUM: u8 = 0x04;
/// Node tag: the value-type thunk former (child: comp-type).
pub const NODE_VT_THUNK: u8 = 0x05;
/// Node tag: the value-type lift (payload: inline target level; child:
/// value-type inner).
pub const NODE_VT_LIFT: u8 = 0x06;
/// Node tag: the returner former (child: value-type).
pub const NODE_CT_RETURNER: u8 = 0x07;
/// Node tag: the arrow former (children: value-type domain, comp-type
/// codomain).
pub const NODE_CT_ARROW: u8 = 0x08;
/// Node tag: a bound value variable (payload: de Bruijn index uvarint).
pub const NODE_V_VARIABLE: u8 = 0x09;
/// Node tag: a constant reference (payload: admission index uvarint).
pub const NODE_V_CONSTANT: u8 = 0x0A;
/// Node tag: the unit value.
pub const NODE_V_UNIT: u8 = 0x0B;
/// Node tag: a literal value (payload: literal kind byte + canonical payload).
pub const NODE_V_LITERAL: u8 = 0x0C;
/// Node tag: a pair value (children: value, value).
pub const NODE_V_PAIR: u8 = 0x0D;
/// Node tag: a sum injection value (payload: side byte; child: value).
pub const NODE_V_INJECTION: u8 = 0x0E;
/// Node tag: a thunk value (child: computation).
pub const NODE_V_THUNK: u8 = 0x0F;
/// Node tag: a value lift (payload: inline target level; child: value).
pub const NODE_V_LIFT: u8 = 0x10;
/// Node tag: a lambda computation (child: computation).
pub const NODE_C_LAMBDA: u8 = 0x11;
/// Node tag: an application (children: computation head, value argument).
pub const NODE_C_APPLICATION: u8 = 0x12;
/// Node tag: a returner computation (child: value).
pub const NODE_C_RETURN: u8 = 0x13;
/// Node tag: a bind computation (children: computation, computation).
pub const NODE_C_BIND: u8 = 0x14;
/// Node tag: a force computation (child: value).
pub const NODE_C_FORCE: u8 = 0x15;
/// Node tag: a case computation (children: value scrutinee, computation,
/// computation).
pub const NODE_C_CASE: u8 = 0x16;
/// Node tag: a sealed abstract type (payload: the admission index of its
/// abstract-type declaration, as a uvarint; no children).
///
/// **Newly assigned, not reassigned.** `0x00..=0x16` keep the meanings v1
/// froze, byte for byte; `0x17` was unassigned and is now the sealed atom. A
/// reader built before this assignment meets `0x17` and refuses
/// [`DecodeError::UnknownTag`] at [`TagSite::Node`] — a precise named refusal
/// from the rejection triple, never a mis-parse — which is why the assignment
/// is additive rather than a version bump. See the sealing decision record for
/// why v1 held.
pub const NODE_VT_ABSTRACT: u8 = 0x17;

/// A declaration's admission mark as it rides in the artifact (E6): a single
/// checked/unchecked bit, never a trust lattice (K3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionMark
{
    /// Admitted through the checked choke point
    /// ([`Environment::add_decl`](crate::Environment::add_decl)).
    Checked,
    /// Admitted through the warned bypass
    /// ([`Environment::add_decl_unchecked`](crate::Environment::add_decl_unchecked)).
    UncheckedBypass,
}

/// Borrowed canonical kernel-export bytes offered to the validating reader.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArtifactImage<'artifact>(&'artifact [u8]);

impl<'artifact> From<&'artifact [u8]> for ArtifactImage<'artifact>
{
    #[inline]
    fn from(bytes: &'artifact [u8]) -> Self
    {
        Self(bytes)
    }
}

impl AsRef<[u8]> for ArtifactImage<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        self.0
    }
}

/// Owned canonical kernel-export byte image.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedArtifact(
    /// Canonical v1 export bytes.
    alloc::vec::Vec<u8>,
);

impl AsRef<[u8]> for EncodedArtifact
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        self.0.as_slice()
    }
}
impl EncodedArtifact
{
    /// Borrow these canonical bytes for validation or replay.
    #[inline]
    #[must_use]
    pub fn as_image(&self) -> ArtifactImage<'_>
    {
        ArtifactImage(self.0.as_slice())
    }
}

impl From<EncodedArtifact> for alloc::vec::Vec<u8>
{
    #[inline]
    fn from(artifact: EncodedArtifact) -> Self
    {
        artifact.0
    }
}

impl core::ops::Deref for EncodedArtifact
{
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        self.0.as_slice()
    }
}

/// Borrowed header prefix of a segmented kernel-export artifact.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArtifactHeaderBytes<'artifact>(&'artifact [u8]);

impl AsRef<[u8]> for ArtifactHeaderBytes<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        self.0
    }
}

/// Number of declaration segments in a kernel-export artifact.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SegmentCount(usize);

impl From<SegmentCount> for usize
{
    #[inline]
    fn from(count: SegmentCount) -> Self
    {
        count.0
    }
}

/// Number of entries in the decoded global subterm table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TableEntryCount(usize);

impl From<usize> for TableEntryCount
{
    #[inline]
    fn from(count: usize) -> Self
    {
        Self(count)
    }
}

impl From<TableEntryCount> for usize
{
    #[inline]
    fn from(count: TableEntryCount) -> Self
    {
        count.0
    }
}
impl fmt::Display for TableEntryCount
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        self.0.fmt(f)
    }
}

/// Saturating expanded-tree work measured during artifact decoding.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ExpandedWork(u64);

impl From<u64> for ExpandedWork
{
    #[inline]
    fn from(work: u64) -> Self
    {
        Self(work)
    }
}

impl From<ExpandedWork> for u64
{
    #[inline]
    fn from(work: ExpandedWork) -> Self
    {
        work.0
    }
}
impl fmt::Display for ExpandedWork
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        self.0.fmt(f)
    }
}

/// The admission position of one minted atom, as the R4 table records it.
///
/// A bare index would be the wrong type here even though the representation is
/// the same integer: the table's entries are *positions in an admission
/// sequence*, and the reader compares them against positions it re-derived. The
/// wrapper is what stops one being crossed with a table index or a byte offset
/// at a signature.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MintedAtom(usize);

impl From<usize> for MintedAtom
{
    #[inline]
    fn from(position: usize) -> Self
    {
        Self(position)
    }
}

impl From<MintedAtom> for usize
{
    #[inline]
    fn from(atom: MintedAtom) -> Self
    {
        atom.0
    }
}

/// Global index of one entry in the artifact subterm table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct GlobalIndex(u32);

/// One byte in the canonical artifact wire image.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct WireByte(u8);

/// One decoded 32-bit wire integer.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct WireU32(u32);

/// One decoded or encoded 64-bit wire integer.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct WireU64(u64);

/// One decoded or encoded host-sized wire count.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct WireUsize(usize);

/// Number of bytes requested from the artifact reader.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ByteCount(usize);

/// Offset of the next unread artifact byte.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ByteOffset(usize);

/// Header and declaration-segment offsets for one encoded artifact.
#[derive(Clone, Debug)]
struct ArtifactFraming
{
    /// Length of the header preceding the first declaration segment.
    header_len: usize,
    /// Exclusive end offset of each declaration segment.
    segment_ends: alloc::vec::Vec<usize>,
}

/// A declaration recovered from an artifact, with its admission mark.
///
/// It is the unit [`decode`] yields and [`read()`] replays through the choke
/// point.
#[derive(Clone, Debug)]
pub struct DecodedDeclaration
{
    /// How the declaration was admitted in the producing environment (E6).
    mark: AdmissionMark,
    /// The recovered declaration.
    declaration: Declaration,
}

impl DecodedDeclaration
{
    /// Pair an admission mark with a recovered declaration.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        mark: AdmissionMark,
        declaration: Declaration,
    ) -> Self
    {
        Self { mark, declaration }
    }

    /// The declaration's admission mark (E6).
    #[inline]
    #[must_use]
    pub const fn mark(&self) -> AdmissionMark
    {
        self.mark
    }

    /// The recovered declaration (its content roots address the owning
    /// [`DecodedArtifact`]'s arena).
    #[inline]
    #[must_use]
    pub const fn declaration(&self) -> &Declaration
    {
        &self.declaration
    }
}

/// The deterministic decode-budget metrics of an artifact — the telemetry
/// floors (massive-term design §4.4; the export exit gate's size/work records).
///
/// Every field is a **function of the canonical bytes alone**, so recording it
/// per corpus item pins the size/work profile the day it moves: an S2 inflation
/// changes a metric and re-blessing is forced. They are computed by the same
/// memoized forward scan the budget check rides — the Lean cached-word-per-node
/// discipline (impl-models-deep-read §2.1): a read off an already-computed
/// `expanded_size` vector, never a second descent — so exposing them costs
/// nothing beyond the check already paid.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DecodeMetrics
{
    /// The number of subterm-table entries (bounded by [`MAX_TABLE_ENTRIES`]).
    table_entries: TableEntryCount,
    /// The maximum expanded (tree) size over every declaration root.
    max_declaration_expanded_work: ExpandedWork,
    /// The saturating sum of expanded (tree) size over every declaration root.
    artifact_expanded_work: ExpandedWork,
}

impl DecodeMetrics
{
    /// Pair the three deterministic decode-budget quantities.
    #[inline]
    #[must_use]
    pub(crate) const fn new(
        table_entries: TableEntryCount,
        max_declaration_expanded_work: ExpandedWork,
        artifact_expanded_work: ExpandedWork,
    ) -> Self
    {
        Self {
            table_entries,
            max_declaration_expanded_work,
            artifact_expanded_work,
        }
    }

    /// The number of subterm-table entries.
    #[inline]
    #[must_use]
    pub const fn table_entries(&self) -> TableEntryCount
    {
        self.table_entries
    }

    /// The maximum per-declaration-root expanded (tree) size.
    #[inline]
    #[must_use]
    pub const fn max_declaration_expanded_work(&self) -> ExpandedWork
    {
        self.max_declaration_expanded_work
    }

    /// The artifact-total expanded (tree) size (the saturating sum over
    /// declaration roots).
    #[inline]
    #[must_use]
    pub const fn artifact_expanded_work(&self) -> ExpandedWork
    {
        self.artifact_expanded_work
    }
}

/// A fully decoded artifact: the [`TermArena`] its declarations' content was
/// decoded into, and the admission-ordered declaration sequence addressing it.
///
/// It also carries the deterministic [`DecodeMetrics`] computed en route (D3
/// telemetry).
///
/// [`decode`] yields this self-contained structure (the term/type content lives
/// in [`Self::arena`], not in owned trees); [`read()`] re-admits it through the
/// choke point by importing each declaration's content into a fresh
/// environment.
#[derive(Clone, Debug, Default)]
pub struct DecodedArtifact
{
    /// The arena every decoded declaration's content lives in.
    arena: TermArena,
    /// The decoded declarations, in admission order.
    declarations: alloc::vec::Vec<DecodedDeclaration>,
    /// The deterministic decode-budget metrics (D3 telemetry).
    metrics: DecodeMetrics,
}

impl DecodedArtifact
{
    /// Pair a decode arena with its admission-ordered declaration sequence and
    /// the decode-budget metrics computed en route.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        arena: TermArena,
        declarations: alloc::vec::Vec<DecodedDeclaration>,
        metrics: DecodeMetrics,
    ) -> Self
    {
        Self {
            arena,
            declarations,
            metrics,
        }
    }

    /// The arena the declarations' content was decoded into.
    #[inline]
    #[must_use]
    pub const fn arena(&self) -> &TermArena
    {
        &self.arena
    }

    /// The decoded declarations, in admission order.
    #[inline]
    #[must_use]
    pub fn declarations(&self) -> &[DecodedDeclaration]
    {
        &self.declarations
    }

    /// The deterministic decode-budget metrics (D3 telemetry: table entries,
    /// max per-declaration expanded work, and artifact-total expanded work).
    #[inline]
    #[must_use]
    pub const fn metrics(&self) -> DecodeMetrics
    {
        self.metrics
    }
}

/// A canonical v1 export artifact together with the byte boundaries of its
/// admission-ordered declaration segments — the outer-layer record grain
/// (massive-term design §6).
///
/// [`write_segmented`] yields this as a purely structural companion to
/// [`write()`]: [`Self::bytes`] is byte-identical to `write(env)`, and
/// [`Self::segments`] exposes where each declaration's self-delimiting segment
/// begins and ends so an outer content-addressed layer can key each declaration
/// by its admission index without re-parsing the framing. It carries **no**
/// hashes and interprets **none** of the payload bytes — offsets and lengths
/// only, the TCB-wall discipline (hashing is untrusted plumbing outside
/// kernel-core). A declaration segment may reference subterm-table entries an
/// earlier segment introduced (cross-declaration sharing), so a segment is a
/// content-addressing grain, **not** an independently replayable unit — replay
/// is whole-artifact ([`read()`]) over the reassembled bytes.
#[derive(Clone, Debug)]
pub struct SegmentedArtifact
{
    /// Canonical bytes shared by whole-artifact and segmented projections.
    bytes: EncodedArtifact,
    /// Header and declaration-segment framing offsets.
    framing: ArtifactFraming,
}

impl SegmentedArtifact
{
    /// Pair canonical artifact bytes with their declaration-segment framing.
    #[inline]
    #[must_use]
    fn new(
        bytes: EncodedArtifact,
        framing: ArtifactFraming,
    ) -> Self
    {
        Self { bytes, framing }
    }

    /// The full canonical artifact bytes — byte-identical to [`write()`].
    #[inline]
    #[must_use]
    pub fn bytes(&self) -> ArtifactImage<'_>
    {
        ArtifactImage(self.bytes.as_ref())
    }

    /// The header bytes preceding the first declaration segment (magic,
    /// version, the reserved minted-atom table, and the declaration count).
    ///
    /// Reassembly is `header()` followed by every [`Self::segments`] slice in
    /// admission order; that concatenation is [`Self::bytes`].
    #[inline]
    #[must_use]
    pub fn header(&self) -> ArtifactHeaderBytes<'_>
    {
        let header = self.bytes.get(.. self.framing.header_len).unwrap_or(&[]);
        ArtifactHeaderBytes(header)
    }

    /// The number of declaration segments (the artifact's declaration count).
    #[inline]
    #[must_use]
    pub fn segment_count(&self) -> SegmentCount
    {
        SegmentCount(self.framing.segment_ends.len())
    }

    /// The declaration segments' bytes, in admission order.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: yields each declaration's self-delimiting segment slice, in
    ///   admission order; concatenated after [`Self::header`] they reproduce
    ///   [`Self::bytes`].
    /// - provides: the outer CAS layer's per-declaration record values.
    /// - fails: never — a malformed internal offset yields no further segment
    ///   rather than panicking (fail-safe, though `write_segmented` never
    ///   produces one).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn segments(&self) -> Segments<'_>
    {
        Segments {
            bytes: self.bytes.as_ref(),
            ends: &self.framing.segment_ends,
            start: self.framing.header_len,
            next: 0,
        }
    }
}

/// An iterator over a [`SegmentedArtifact`]'s declaration segments, in
/// admission order (each item the segment's bytes).
#[derive(Clone, Debug)]
pub struct Segments<'artifact>
{
    /// The full canonical artifact bytes being sliced.
    bytes: &'artifact [u8],
    /// The exclusive segment end offsets.
    ends: &'artifact [usize],
    /// The start offset of the next segment (the previous segment's end, or the
    /// header length initially).
    start: usize,
    /// The index of the next segment end to consume.
    next: usize,
}

impl<'artifact> Iterator for Segments<'artifact>
{
    type Item = &'artifact [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        let end = *self.ends.get(self.next)?;
        let segment = self.bytes.get(self.start .. end)?;
        self.start = end;
        self.next = self.next.saturating_add(1);
        Some(segment)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>)
    {
        let remaining = self.ends.len().saturating_sub(self.next);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for Segments<'_>
{
}

/// One of the R1 reserved declaration kinds still awaiting its rung — a shape
/// the writer never emits and the reader rejects distinctly.
///
/// R1 reserved four kinds and **`AbstractType` is no longer among them**: the
/// sealing rung made it live, so it is written and decoded like `Def` and
/// `Axiom` rather than refused. Its variant is gone rather than
/// retained-unused, because a vocabulary of reserved kinds that lists a live
/// one misdescribes what the reader does; matches on this enum are total by
/// policy, so the removal is compile-visible at every site.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReservedKind
{
    /// A reserved module-signature declaration kind.
    ModuleSig,
    /// A reserved module-definition declaration kind.
    ModuleDef,
    /// A reserved functor-definition declaration kind.
    FunctorDef,
}

impl fmt::Display for ReservedKind
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(match *self {
            | Self::ModuleSig => "ModuleSig",
            | Self::ModuleDef => "ModuleDef",
            | Self::FunctorDef => "FunctorDef",
        })
    }
}

/// A reserved slot or section that must be empty at v0 (R2/R3/R4) but was
/// found occupied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReservedSlot
{
    /// The R4 reserved minted-atom table was non-empty.
    MintedAtomTable,
    /// The R2 structured-name record carried a segment (S1 declarations are
    /// nameless, so v0 pins the segment count to zero).
    StructuredName,
    /// The R3 erasure annotation slot was non-empty.
    ErasureAnnotation,
    /// The R3 modes/grades annotation slot was non-empty.
    ModeGradeAnnotation,
    /// The R3 sealing-provenance annotation slot was non-empty.
    SealingProvenance,
    /// The R3 directedness/variance annotation slot was non-empty.
    DirectednessVariance,
}

impl fmt::Display for ReservedSlot
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(match *self {
            | Self::MintedAtomTable => "the reserved minted-atom table",
            | Self::StructuredName => "the structured-name record",
            | Self::ErasureAnnotation => "the erasure annotation slot",
            | Self::ModeGradeAnnotation => "the modes/grades annotation slot",
            | Self::SealingProvenance => "the sealing-provenance annotation slot",
            | Self::DirectednessVariance => "the directedness/variance annotation slot",
        })
    }
}

/// Where in the closed grammar an unknown tag byte was met.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TagSite
{
    /// A declaration's admission mark byte.
    Admission,
    /// A declaration's kind byte (a value outside `Def`/`Axiom` and the R1
    /// reserved kinds).
    DeclarationKind,
    /// A unified subterm-table node tag (a value outside `NODE_*`).
    Node,
    /// A base-type atom byte.
    BaseType,
    /// A literal sign byte.
    Sign,
    /// A literal kind byte.
    LiteralKind,
    /// An injection side byte.
    Side,
    /// A landmark-constraint relation byte.
    ConstraintRelation,
}

impl fmt::Display for TagSite
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(match *self {
            | Self::Admission => "an admission mark",
            | Self::DeclarationKind => "a declaration kind",
            | Self::Node => "a subterm-table node tag",
            | Self::BaseType => "a base-type atom",
            | Self::Sign => "a literal sign",
            | Self::LiteralKind => "a literal kind",
            | Self::Side => "an injection side",
            | Self::ConstraintRelation => "a constraint relation",
        })
    }
}

/// Which structural invariant a malformed artifact violated (the third leg of
/// the rejection triple).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MalformedSite
{
    /// The magic bytes did not match a gandr kernel export.
    Header,
    /// A varint was overlong (non-minimal) or exceeded the `u64` range.
    Varint,
    /// A decoded index did not fit its target width.
    IndexRange,
    /// A level variable atom's offset met or exceeded
    /// [`MAX_DECODED_LEVEL_OFFSET`], or reconstruction overflowed.
    LevelOffset,
    /// A decoded landmark-constraint side was not variable-only (the strata
    /// smart constructor rejected it).
    ConstraintForm,
    /// A literal payload was not reconstructible: a magnitude or fraction
    /// carried a non-digit byte (the smart constructor rejected it), or a
    /// string literal's bytes were not valid UTF-8.
    LiteralPayload,
    /// A decoded child node's polarity did not fit the slot its parent
    /// constructor required.
    Polarity,
    /// A subterm-table entry's child index was not **strictly earlier** than
    /// the entry's own global index (acyclicity / topological order
    /// violated).
    ChildOrder,
    /// The subterm table exceeded [`MAX_TABLE_ENTRIES`] as its entries accrued.
    TableSize,
    /// A declaration root's memoized expanded (tree) size exceeded
    /// [`MAX_EXPANDED_TERM_WORK`] — the amplification defence (§4.4).
    ExpandedWork,
    /// The artifact-total expanded (tree) work — the saturating sum over every
    /// declaration root's expanded size — exceeded
    /// [`MAX_ARTIFACT_EXPANDED_WORK`] (the per-artifact amplification
    /// defence, §4.4).
    ArtifactExpandedWork,
    /// The bytes decoded but were not the canonical encoding of the recovered
    /// artifact (E4: a non-canonical encoding — a non-maximally-shared,
    /// mis-ordered, or dead-entry table — is rejected).
    NonCanonical,
    /// Bytes remained after a complete artifact was decoded.
    TrailingBytes,
}

impl fmt::Display for MalformedSite
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(match *self {
            | Self::Header => "the artifact magic did not match",
            | Self::Varint => "a varint was overlong or out of range",
            | Self::IndexRange => "a decoded index did not fit its width",
            | Self::LevelOffset => "a level atom offset exceeded the decode cap",
            | Self::ConstraintForm => "a constraint side was not variable-only",
            | Self::LiteralPayload => "a literal payload was not reconstructible",
            | Self::Polarity => "a decoded node had the wrong polarity for its slot",
            | Self::ChildOrder => "a child index was not strictly earlier than its entry",
            | Self::TableSize => "the subterm table exceeded the entry cap",
            | Self::ExpandedWork => "a declaration's expanded size exceeded the work cap",
            | Self::ArtifactExpandedWork => {
                "the artifact-total expanded size exceeded the artifact work cap"
            },
            | Self::NonCanonical => "the bytes were not the canonical encoding",
            | Self::TrailingBytes => "bytes remained after a complete artifact",
        })
    }
}

/// Why decoding an artifact failed.
///
/// The closed rejection vocabulary of the validating reader (E4): a decode
/// failure is a **format** failure, held apart from a typing
/// failure ([`KernelError`]) so the two never blur.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError
{
    /// The artifact ended mid-field (the rejection triple's *truncated*).
    Truncated,
    /// A tag byte fell outside the closed vocabulary at a tagged position (the
    /// rejection triple's *bad tag*).
    UnknownTag
    {
        /// Where the tag was read.
        site: TagSite,
        /// The offending byte.
        tag: u8,
    },
    /// The bytes were structurally decodable but violated an invariant (the
    /// rejection triple's *malformed*).
    Malformed
    {
        /// Which invariant was violated.
        site: MalformedSite,
    },
    /// A reserved declaration kind (R1) was present.
    ReservedDeclarationKind
    {
        /// The reserved kind met.
        kind: ReservedKind,
    },
    /// A reserved slot or section (R2/R3/R4) was non-empty at v0.
    ReservedSlotOccupied
    {
        /// The occupied reserved slot.
        slot: ReservedSlot,
    },
    /// The artifact declared a format version the reader does not implement
    /// (E5: a named refusal, never a guess).
    UnsupportedVersion
    {
        /// The version the artifact declared.
        found: u16,
    },
}

impl fmt::Display for DecodeError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::Truncated => f.write_str("the export artifact ended mid-field"),
            | Self::UnknownTag { site, tag } => {
                write!(f, "unknown tag byte {tag} where {site} was expected")
            },
            | Self::Malformed { site } => write!(f, "malformed export artifact: {site}"),
            | Self::ReservedDeclarationKind { kind } => {
                write!(f, "reserved declaration kind {kind} is not admitted at v0")
            },
            | Self::ReservedSlotOccupied { slot } => {
                write!(f, "{slot} is non-empty but must be empty at v0")
            },
            | Self::UnsupportedVersion { found } => {
                write!(f, "unsupported export format version {found}")
            },
        }
    }
}

impl Error for DecodeError
{
}

/// Why reading an artifact into an [`Environment`](crate::Environment) failed.
///
/// Either the bytes did not decode ([`DecodeError`]) or a recovered declaration
/// did not re-admit through the choke point ([`KernelError`]); the two failure
/// planes stay distinct.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadError
{
    /// The bytes did not decode.
    Decode(DecodeError),
    /// A recovered declaration failed to re-admit through the choke point (E2:
    /// replay is re-running `add_decl`).
    Admit(KernelError),
}

impl fmt::Display for ReadError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::Decode(ref error) => write!(f, "export decode failed: {error}"),
            | Self::Admit(ref error) => write!(f, "export replay failed to re-admit: {error}"),
        }
    }
}

impl Error for ReadError
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match *self {
            | Self::Decode(ref error) => Some(error),
            | Self::Admit(ref error) => Some(error),
        }
    }
}

impl From<DecodeError> for ReadError
{
    #[inline]
    fn from(error: DecodeError) -> Self
    {
        Self::Decode(error)
    }
}

impl From<KernelError> for ReadError
{
    #[inline]
    fn from(error: KernelError) -> Self
    {
        Self::Admit(error)
    }
}
