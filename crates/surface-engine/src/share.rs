#![allow(
    unknown_lints,
    reason = "The local dylint policy is unavailable to rustc outside its owning check."
)]
//! The engine value domain's **sharing overlay** — sharing-bearing reduction
//! syntax beside the existing unshared pipeline, in structural-lambda-calculus
//! form with closures at a distance.
//!
//! This is the first rung of the staged sharing adoption for the conversion
//! path: the value domain gains the syntax that can state sharing at all.
//! Nothing here is serialized, nothing here evaluates, and nothing in the
//! trusted base changes — the overlay is engine-side, untrusted machinery, and
//! every share-carrying term erases to the ordinary unshared spelling the
//! checker, the machines, conversion, and the kernel bridge already consume.
//!
//! # The node inventory
//!
//! Four node families — value, computation, value type, computation type —
//! each a flat append-only arena table addressed by kind-marked `u32` ids
//! ([`ShareId`], the `NodeId<Kind>` discipline of `gandr-core-checker`'s
//! canonical carrier). Per family exactly four node shapes exist:
//!
//! * [`ShareNode::Opaque`] — an existing unshared term held whole. The overlay
//!   never pattern-matches the payload; erasure clones it out untouched.
//! * [`ShareNode::Bound`] — a nameless reference `{distance, position}`: the
//!   `position`-th occurrence of the `distance`-th enclosing
//!   [`ShareNode::Share`] closure. References carry no names, so α-equivalence
//!   of overlays is syntactic identity.
//! * [`ShareNode::Share`] — the sharing former `t[x̄ ← u]`: an `arity` (the
//!   occurrence count, at least one at this rung — weakening waits for the
//!   garbage-collection rules), a family-tagged `shared` leg `u`, and a
//!   same-family `body`. Occurrences live in the body and are numbered by first
//!   occurrence, preorder; [`validate`] checks the numbering.
//! * [`ShareNode::Graft`] — a host-syntax `template` plus family-tagged
//!   `children`. The template marks plug positions with globally fresh seam
//!   names `seam.0 … seam.n-1`, canonically enumerated by first occurrence,
//!   each exactly once; a repeated seam is invalid, because a repeated seam
//!   would duplicate a child with no sharing former naming the duplication.
//!
//! Distributors, phantom abstractions, and covers arrive with the future
//! duplication rung. They remain tagless and absent from normal forms; rung
//! one carries none of them.
//!
//! # The reserved tag block, restated
//!
//! The export format's reserved node-tag block `0x18..=0x1F` stands behind
//! the four families, in [`ShareFamily::RESERVED_ORDER`]: `0x18` value,
//! `0x19` computation, `0x1A` value type, `0x1B` computation type, with
//! `0x1C..=0x1F` held (an explicit-weakening form if erasure ever becomes
//! explicit, and second-generation sharing variants). The mapping is
//! documentation and a constant ([`ShareFamily::reserved_tag`]); this module
//! has no codec, and the overlay changes no wire byte.
//!
//! # Erasure is total, policy-free, and capture-permitting
//!
//! [`erase_value`] and its siblings fold an overlay to the existing unshared
//! pipeline: an [`ShareNode::Opaque`] resolves to its payload, a
//! [`ShareNode::Bound`] looks the erased shared term up in the closure
//! environment, a [`ShareNode::Share`] erases its shared leg once, pushes an
//! arity-wide environment entry, and erases its body, and a
//! [`ShareNode::Graft`] erases its children and **seam-plugs** them into its
//! template. The plug is an engine-owned traversal of the public legacy
//! syntax that replaces each `seam.k` variable with the `k`-th erased child.
//! It is deliberately **capture-permitting** — it never invokes the core's
//! capture-avoiding substitution — so under-binder sharing is expressible and
//! observable: `λx. t` sharing a `u` with `x` free in `u` erases to a term in
//! which that `x` is bound. Erasure takes no policy and is invariant under
//! every future one: full unsharing duplicates by definition, whatever the
//! reduction-time policy later says about copying.
//!
//! # The policy parameter
//!
//! [`DuplicationPolicy`] is the named parameter every duplication entry
//! takes as a runtime argument — which part of a shared abstraction is
//! copied when it is forced into a redex stays policy, never a hardcoded
//! whole-value clone. It is a **closed set** of two stances:
//! [`DuplicationPolicy::EraseAndClone`], the conservative baseline,
//! behaviorally the unshared pipeline of today; and
//! [`DuplicationPolicy::Spinal`], the staged path's fourth rung, gated
//! behind the uninhabited [`SpinalSeat`] until the conversion trace seam
//! exists to certify the strategy's results. The parameter enters through
//! [`duplicate_value`] and [`duplicate_comp`]; the erasure path takes no
//! policy and is invariant under every stance, because the baseline's
//! result *is* the erasure and every other stance replays against it.
//!
//! # Validation
//!
//! [`validate`] is the overlay's well-formedness checker: child-before-parent
//! within each family table (the minting invariant, checked mechanically),
//! family agreement between a graft's children and its seam positions,
//! nonzero arity, canonical first-occurrence position numbering (overlay-side,
//! preorder), and seam freshness, linearity, and canonical enumeration
//! (template-side). Totality is inductive off the arena: child ids are
//! strictly earlier than their parents, so every walk descends a well-founded
//! measure, and both the erasure fold and the validation walk run as explicit
//! heap worklists (no input-scaled recursion). One honest cost: the
//! canonical-position check is a preorder walk over the overlay's *tree*
//! unfolding, so a deeply shared overlay validates in tree order, not node
//! order; nothing at this rung mints overlays from untrusted input, and the
//! amplification posture is revisited when a decode path exists.
//!
//! # References
//!
//! * David Sherratt, Willem Heijltjes, Tom Gundersen, Michel Parigot, "Spinal
//!   Atomic Lambda-Calculus", `FoSSaCS` 2020, LNCS 12077, pp. 582–601,
//!   `doi:10.1007/978-3-030-45231-5_30` — the sharing discipline the staged
//!   adoption targets.
//! * Tom Gundersen, Willem Heijltjes, Tom Gundersen, Michel Parigot, "Atomic
//!   Lambda-Calculus: A Typed Lambda-Calculus with Explicit Sharing", LICS
//!   2013, pp. 311–320, `doi:10.1109/LICS.2013.37` — the closure calculus the
//!   overlay's syntax follows.
//! * Fanny He, "The Atomic Lambda-Mu Calculus", `PhD` thesis, University of
//!   Bath, 2018,
//!   `https://researchportal.bath.ac.uk/en/studentTheses/the-atomic-lambda-mu-calculus`
//!   — the sharing discipline carried over classical control: the
//!   μ-distributor the spinal stance's control-construct arms follow
//!   (duplicating a stack-capturing abstraction freezes the constructor and
//!   duplicates the body tuple-wise), chapters 2 and 7.

use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;

use gandr_core_term::boundary::EffectSignatureName;
use gandr_core_term::boundary::OperationName;
use gandr_core_term::classifier::Classifier;
use gandr_core_term::effect::EffectOp;
use gandr_core_term::effect::EffectRow;
use gandr_core_term::effect::EffectSig;
use gandr_core_term::grade::Grade;
use gandr_core_term::prim::NativePrim;
use gandr_core_term::static_term::FamilyApp;
use gandr_core_term::static_term::StaticArg;
use gandr_core_term::static_term::StaticBinder;
use gandr_core_term::static_term::StaticNeutral;
use gandr_core_term::static_term::StaticTerm;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::OpClause;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::SplitMotive;
use gandr_core_term::syntax::Stack;
use gandr_core_term::syntax::Value;
use gandr_core_term::syntax::WalkBase;
use gandr_core_term::syntax::WalkMotive;
use gandr_core_term::types::CompType;
use gandr_core_term::types::DataId;
use gandr_core_term::types::SealId;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

/// The seam-name prefix a [`ShareNode::Graft`] template uses for its plug
/// positions: `seam.0`, `seam.1`, … in canonical enumeration.
pub const SEAM_PREFIX: &str = "seam.";

/// A raw arena index (the stored identity of a [`ShareId`]).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShareIndex(u32);

impl From<u32> for ShareIndex
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<ShareIndex> for u32
{
    #[inline]
    fn from(value: ShareIndex) -> Self
    {
        value.0
    }
}

/// A binder distance: how many [`ShareNode::Share`] closures outward a
/// [`ShareNode::Bound`] reference points.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BinderDistance(u32);

impl From<u32> for BinderDistance
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<BinderDistance> for u32
{
    #[inline]
    fn from(value: BinderDistance) -> Self
    {
        value.0
    }
}

/// A position inside one closure's occurrence vector: the `position`-th
/// first-occurrence-numbered occurrence of the shared term.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VectorPosition(u32);

impl From<u32> for VectorPosition
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<VectorPosition> for u32
{
    #[inline]
    fn from(value: VectorPosition) -> Self
    {
        value.0
    }
}

/// The number of occurrences a [`ShareNode::Share`] closure binds (at least
/// one at this rung: zero is weakening, which waits for the garbage rules).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Arity(u32);

impl From<u32> for Arity
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<Arity> for u32
{
    #[inline]
    fn from(value: Arity) -> Self
    {
        value.0
    }
}

/// A seam's canonical index inside a [`ShareNode::Graft`] template.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SeamIndex(u32);

impl From<u32> for SeamIndex
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<SeamIndex> for u32
{
    #[inline]
    fn from(value: SeamIndex) -> Self
    {
        value.0
    }
}

/// A count of a node's same-sort children (or a graft's children).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChildCount(usize);

impl From<usize> for ChildCount
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<ChildCount> for usize
{
    #[inline]
    fn from(value: ChildCount) -> Self
    {
        value.0
    }
}

/// A family table's length, as recorded in a [`ShareWatermark`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FamilyLength(usize);

impl From<usize> for FamilyLength
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<FamilyLength> for usize
{
    #[inline]
    fn from(value: FamilyLength) -> Self
    {
        value.0
    }
}

/// A count of occurrences consumed during canonical-position validation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OccurrenceCount(u32);

impl From<u32> for OccurrenceCount
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<OccurrenceCount> for u32
{
    #[inline]
    fn from(value: OccurrenceCount) -> Self
    {
        value.0
    }
}

/// An export node tag, named so the reserved block has a typed spelling.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ReservedTag(u8);

impl From<u8> for ReservedTag
{
    #[inline]
    fn from(value: u8) -> Self
    {
        Self(value)
    }
}

impl From<ReservedTag> for u8
{
    #[inline]
    fn from(value: ReservedTag) -> Self
    {
        value.0
    }
}

/// The first held tag of the reserved block (`0x1C`): not a sharing family.
pub const HELD_TAGS_FIRST: ReservedTag = ReservedTag(0x1C);

/// The last held tag of the reserved block (`0x1F`).
pub const HELD_TAGS_LAST: ReservedTag = ReservedTag(0x1F);

/// One of the overlay's four node families, in the reserved tag block's
/// order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShareFamily
{
    /// The value family (tag `0x18`).
    Value,
    /// The computation family (tag `0x19`).
    Comp,
    /// The value-type family (tag `0x1A`).
    ValueType,
    /// The computation-type family (tag `0x1B`).
    CompType,
}

impl ShareFamily
{
    /// The families in the reserved block's order: `0x18..=0x1B`
    /// positionally.
    pub const RESERVED_ORDER: [Self; 4] =
        [Self::Value, Self::Comp, Self::ValueType, Self::CompType];

    /// The export node tag reserved for this family's sharing former.
    ///
    /// # Contract
    /// - ensures: `0x18`, `0x19`, `0x1A`, `0x1B` in [`Self::RESERVED_ORDER`]
    ///   order; the tag is a named reservation only — nothing serializes it.
    /// - provides: the typed spelling of the reserved block.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the four-arm mapping is separated by asserting each
    ///   family's exact tag, the order's positional agreement with the `0x18`
    ///   base, and the held range's two boundaries.
    /// - witness: `share::tests::reserved_tags_map_families_in_order`
    #[inline]
    #[must_use]
    pub fn reserved_tag(self) -> ReservedTag
    {
        match self {
            | Self::Value => ReservedTag(0x18),
            | Self::Comp => ReservedTag(0x19),
            | Self::ValueType => ReservedTag(0x1A),
            | Self::CompType => ReservedTag(0x1B),
        }
    }
}

/// A stable, kind-marked identifier for one node in a [`ShareArena`] family
/// table.
///
/// The `L` parameter is the family's unshared leaf type
/// (`gandr_core_checker`'s `Value`, `Comp`, `ValueType`, or `CompType`), a
/// zero-cost tag that keeps the four tables unmixable while the stored
/// identity stays a compact `u32`. The trait impls are written by hand, the
/// `NodeId<Kind>` discipline, so they carry no `L` bounds.
pub struct ShareId<L>
{
    /// Zero-based index into the family table.
    index: ShareIndex,
    /// Invariant tag tying the index to one family.
    marker: PhantomData<fn() -> L>,
}

impl<L> Copy for ShareId<L>
{
}

impl<L> Clone for ShareId<L>
{
    #[inline]
    fn clone(&self) -> Self
    {
        *self
    }
}

impl<L> core::fmt::Debug for ShareId<L>
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.debug_tuple("ShareId")
            .field(&u32::from(self.index))
            .finish()
    }
}

impl<L> Eq for ShareId<L>
{
}

impl<L> PartialEq for ShareId<L>
{
    #[inline]
    fn eq(
        &self,
        other: &Self,
    ) -> bool
    {
        self.index == other.index
    }
}

impl<L> core::hash::Hash for ShareId<L>
{
    #[inline]
    fn hash<H>(
        &self,
        state: &mut H,
    ) where
        H: core::hash::Hasher,
    {
        core::hash::Hash::hash(&self.index, state);
    }
}

impl<L> Ord for ShareId<L>
{
    #[inline]
    fn cmp(
        &self,
        other: &Self,
    ) -> core::cmp::Ordering
    {
        self.index.cmp(&other.index)
    }
}

impl<L> PartialOrd for ShareId<L>
{
    #[inline]
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<core::cmp::Ordering>
    {
        Some(self.cmp(other))
    }
}

impl<L> ShareId<L>
{
    /// Builds an id from its table index (the arena's own mint path).
    ///
    /// # Contract
    /// - ensures: preserves `index` exactly.
    /// - panics: none.
    #[inline]
    #[must_use]
    fn from_index(index: ShareIndex) -> Self
    {
        Self {
            index,
            marker: PhantomData,
        }
    }

    /// Returns the raw table index.
    ///
    /// # Contract
    /// - ensures: the index the arena minted this id with.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn index(self) -> ShareIndex
    {
        self.index
    }
}

/// A family-tagged overlay node id: the cross-family reference form a
/// [`Sharing`] shared leg and a [`Graft`] child take.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AnyShareId
{
    /// A value-family node.
    Value(ShareId<Value>),
    /// A computation-family node.
    Comp(ShareId<Comp>),
    /// A value-type-family node.
    ValueType(ShareId<ValueType>),
    /// A computation-type-family node.
    CompType(ShareId<CompType>),
}

impl AnyShareId
{
    /// The family the id addresses.
    ///
    /// # Contract
    /// - ensures: the family's tag matches the variant.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn family(self) -> ShareFamily
    {
        match self {
            | Self::Value(_) => ShareFamily::Value,
            | Self::Comp(_) => ShareFamily::Comp,
            | Self::ValueType(_) => ShareFamily::ValueType,
            | Self::CompType(_) => ShareFamily::CompType,
        }
    }
}

impl From<ShareId<Value>> for AnyShareId
{
    #[inline]
    fn from(value: ShareId<Value>) -> Self
    {
        Self::Value(value)
    }
}

impl From<ShareId<Comp>> for AnyShareId
{
    #[inline]
    fn from(value: ShareId<Comp>) -> Self
    {
        Self::Comp(value)
    }
}

impl From<ShareId<ValueType>> for AnyShareId
{
    #[inline]
    fn from(value: ShareId<ValueType>) -> Self
    {
        Self::ValueType(value)
    }
}

impl From<ShareId<CompType>> for AnyShareId
{
    #[inline]
    fn from(value: ShareId<CompType>) -> Self
    {
        Self::CompType(value)
    }
}

/// A nameless shared-occurrence reference: the `position`-th occurrence of
/// the `distance`-th enclosing [`ShareNode::Share`] closure.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Bound
{
    /// How many sharing closures outward the reference points (`0` is the
    /// nearest).
    pub distance: BinderDistance,
    /// Which first-occurrence-numbered occurrence of that closure this is.
    pub position: VectorPosition,
}

/// The sharing former `t[x̄ ← u]`: `shared` is the shared leg `u` (any
/// family), `body` is `t` (the closure's own family), and `arity` counts the
/// occurrences of `u` in `t`, numbered by first occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sharing<L>
{
    /// The occurrence count (at least one at this rung).
    pub arity: Arity,
    /// The shared leg, family-tagged.
    pub shared: AnyShareId,
    /// The body the occurrences live in.
    pub body: ShareId<L>,
}

/// A host-syntax template plus the children its seams plug: the graft is how
/// composite unshared syntax participates in sharing without the overlay
/// reaching into it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Graft<L>
{
    /// The host term carrying fresh, linear, canonically enumerated seams
    /// `seam.0 … seam.n-1`.
    pub template: L,
    /// The family-tagged children, one per seam, in seam order.
    pub children: Vec<AnyShareId>,
}

/// One overlay node: an opaque unshared leaf, a shared-occurrence reference,
/// the sharing former, or a graft.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShareNode<L>
{
    /// An existing unshared term held whole (never inspected by the overlay).
    Opaque(L),
    /// A nameless reference into an enclosing closure.
    Bound(Bound),
    /// The sharing former `t[x̄ ← u]`.
    Share(Sharing<L>),
    /// A host template with family-tagged children.
    Graft(Graft<L>),
}

/// The duplication policy: which part of a shared abstraction is copied when
/// it is forced into a redex.
///
/// A **closed set** of two stances, taken as a runtime argument by every
/// duplication entry ([`duplicate_value`], [`duplicate_comp`]); nothing here
/// hardcodes a whole-value clone. The erasure path stays policy-free and
/// total: the baseline stance's result *is* the erasure, and every other
/// stance's results replay against it, so the unshared pipeline remains the
/// reference implementation whatever the parameter says.
///
/// The control-construct arms the second stance installs at its rung follow
/// Fanny He's atomic λμ-calculus (the module's References): duplicating a
/// stack-capturing abstraction freezes the constructor and duplicates the
/// body tuple-wise, never a whole-abstraction clone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DuplicationPolicy
{
    /// The conservative baseline: erase and clone. Duplicating under it is
    /// the total erasure the overlay already owes — every occurrence
    /// receives the same unshared spelling — so it is behaviorally the
    /// pipeline that exists today, and the default stance.
    EraseAndClone,
    /// The spinal policy of the staged path's fourth rung: maximal free
    /// subexpressions stay shared and only binder-to-occurrence paths copy.
    /// The stance is **gated** — [`SpinalSeat`] is uninhabited until the
    /// conversion trace seam exists to certify the strategy's results — so
    /// naming it is impossible today, and installing it is a deliberate
    /// change to that type rather than an accident of construction.
    Spinal(SpinalSeat),
}

/// The spinal stance's seat: uninhabited until the conversion trace seam
/// lands, at which point this type gains the seat the replayed trace
/// certifies through.
///
/// An empty enum is the mechanical form of "not constructible": no function
/// can receive [`DuplicationPolicy::Spinal`] today, and a `match` over the
/// seat stays total with no unreachable arm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpinalSeat {}

impl Default for DuplicationPolicy
{
    /// The baseline stance — behaviorally the pipeline that exists today.
    #[inline]
    fn default() -> Self
    {
        Self::EraseAndClone
    }
}

/// A typed failure while appending an overlay node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MintError
{
    /// The family table has exhausted its representable `u32` index space.
    CapacityExhausted,
    /// A child id does not resolve in this arena/run.
    ChildOutOfBounds(AnyShareId),
}

/// A snapshot of the four family lengths, the overlay analogue of the kernel
/// arena's admission watermark.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShareWatermark
{
    /// The value table's length.
    values: FamilyLength,
    /// The computation table's length.
    comps: FamilyLength,
    /// The value-type table's length.
    value_types: FamilyLength,
    /// The computation-type table's length.
    comp_types: FamilyLength,
}

/// The append-only arena owning one overlay's nodes across the four families.
///
/// Minting is constructor-only over already-allocated children, so a child id
/// always resolves and is always strictly earlier than its parent within one
/// table; resolution is a checked `get`, and truncation disposes intermediates
/// wholesale. There is no `Rc` anywhere in the overlay itself: opaque leaves
/// are owned syntax moved into the arena. Ids are scoped to one arena/run;
/// callers must not retain them across [`Self::truncate_to`] boundaries.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ShareArena
{
    /// The value nodes, in allocation (child-before-parent) order.
    values: Vec<ShareNode<Value>>,
    /// The computation nodes.
    comps: Vec<ShareNode<Comp>>,
    /// The value-type nodes.
    value_types: Vec<ShareNode<ValueType>>,
    /// The computation-type nodes.
    comp_types: Vec<ShareNode<CompType>>,
}

/// Append a node to one family table, refusing before a representational
/// exhaustion can duplicate an id.
///
/// # Contract
/// - ensures: on success, the id names the pushed node; on failure, the table
///   and its length are unchanged.
/// - fails: [`MintError::CapacityExhausted`] when no fresh `u32` index remains.
/// - panics: none.
#[inline]
fn mint<L>(
    table: &mut Vec<ShareNode<L>>,
    node: ShareNode<L>,
) -> Result<ShareId<L>, MintError>
{
    mint_with_limit(table, node, usize::try_from(u32::MAX).unwrap_or(usize::MAX))
}

/// The bounded seam behind [`mint`], kept private so capacity refusal is
/// testable without allocating the full representable arena.
#[inline]
#[expect(
    primitive_signature,
    reason = "The private capacity seam accepts a machine limit solely for bounded tests."
)]
fn mint_with_limit<L>(
    table: &mut Vec<ShareNode<L>>,
    node: ShareNode<L>,
    limit: usize,
) -> Result<ShareId<L>, MintError>
{
    if table.len() >= limit {
        return Err(MintError::CapacityExhausted);
    }
    let index = u32::try_from(table.len()).map_err(|_error| MintError::CapacityExhausted)?;
    table.push(node);
    Ok(ShareId::from_index(ShareIndex::from(index)))
}

/// Resolve an id in one family table (checked, never indexed).
///
/// - ensures: `Some` of the node iff `id`'s compact index names a live entry in
///   this table; `None` on a dangling id, the fail-closed reading. Ids have no
///   arena-generation tag, so callers must keep them with their arena.
/// - panics: none.
#[inline]
fn lookup<L>(
    table: &[ShareNode<L>],
    id: ShareId<L>,
) -> Option<&ShareNode<L>>
{
    let offset = usize::try_from(u32::from(id.index())).ok()?;
    table.get(offset)
}

impl ShareArena
{
    /// Creates an empty overlay arena.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// The current watermark (the four family lengths).
    ///
    /// # Contract
    /// - ensures: restoring it with [`Self::truncate_to`] drops exactly the
    ///   nodes allocated after this call.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn watermark(&self) -> ShareWatermark
    {
        ShareWatermark {
            values: FamilyLength::from(self.values.len()),
            comps: FamilyLength::from(self.comps.len()),
            value_types: FamilyLength::from(self.value_types.len()),
            comp_types: FamilyLength::from(self.comp_types.len()),
        }
    }

    /// Truncates every family back to `watermark`, dropping later
    /// allocations. Watermarks are run-boundary checkpoints: no id minted
    /// above the watermark survives truncation.
    ///
    /// # Contract
    /// - requires: `watermark` came from this arena's [`Self::watermark`] and
    ///   no retained id crosses this run boundary.
    /// - ensures: each table keeps exactly its recorded prefix, in flat `Vec`
    ///   truncations — total on any depth, no recursive teardown.
    /// - provides: wholesale disposal of intermediates.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — truncation is separated by minting past a mark,
    ///   truncating, and asserting both the restored resolution of a kept id
    ///   and the fail-closed resolution of a disposed id.
    /// - witness: `share::tests::truncation_disposes_intermediates`
    #[inline]
    pub fn truncate_to(
        &mut self,
        watermark: ShareWatermark,
    )
    {
        self.values.truncate(usize::from(watermark.values));
        self.comps.truncate(usize::from(watermark.comps));
        self.value_types
            .truncate(usize::from(watermark.value_types));
        self.comp_types.truncate(usize::from(watermark.comp_types));
    }
    /// Check that an erased id resolves in this arena/run.
    #[inline]
    fn check_id(
        &self,
        id: AnyShareId,
    ) -> Result<(), MintError>
    {
        let resolves = match id {
            | AnyShareId::Value(id) => lookup(&self.values, id).is_some(),
            | AnyShareId::Comp(id) => lookup(&self.comps, id).is_some(),
            | AnyShareId::ValueType(id) => lookup(&self.value_types, id).is_some(),
            | AnyShareId::CompType(id) => lookup(&self.comp_types, id).is_some(),
        };
        if resolves {
            Ok(())
        }
        else {
            Err(MintError::ChildOutOfBounds(id))
        }
    }

    /// Check graft children before the parent enters the append-only arena.
    #[inline]
    fn check_graft_children(
        &self,
        children: &[AnyShareId],
    ) -> Result<(), MintError>
    {
        for &child in children {
            self.check_id(child)?;
        }
        Ok(())
    }

    /// Check a sharing former's shared leg and same-family body.
    #[inline]
    fn check_sharing<L>(
        &self,
        sharing: &Sharing<L>,
    ) -> Result<(), MintError>
    where
        L: Family,
    {
        self.check_id(sharing.shared)?;
        self.check_id(L::tag(sharing.body))?;
        Ok(())
    }

    /// Mint an opaque value leaf.
    ///
    /// # Errors
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn value_opaque(
        &mut self,
        leaf: Value,
    ) -> Result<ShareId<Value>, MintError>
    {
        mint(&mut self.values, ShareNode::Opaque(leaf))
    }

    /// Mint a value-family occurrence reference.
    ///
    /// # Errors
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn value_bound(
        &mut self,
        bound: Bound,
    ) -> Result<ShareId<Value>, MintError>
    {
        mint(&mut self.values, ShareNode::Bound(bound))
    }

    /// Mint a value-family sharing former after checking all referenced legs.
    ///
    /// # Errors
    /// [`MintError::ChildOutOfBounds`] for an unresolved leg, or
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn value_share(
        &mut self,
        sharing: Sharing<Value>,
    ) -> Result<ShareId<Value>, MintError>
    {
        self.check_sharing(&sharing)?;
        mint(&mut self.values, ShareNode::Share(sharing))
    }

    /// Mint a value-family graft after checking every child.
    ///
    /// # Errors
    /// [`MintError::ChildOutOfBounds`] for an unresolved child, or
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn value_graft(
        &mut self,
        graft: Graft<Value>,
    ) -> Result<ShareId<Value>, MintError>
    {
        self.check_graft_children(&graft.children)?;
        mint(&mut self.values, ShareNode::Graft(graft))
    }

    /// Mint an opaque computation leaf.
    ///
    /// # Errors
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn comp_opaque(
        &mut self,
        leaf: Comp,
    ) -> Result<ShareId<Comp>, MintError>
    {
        mint(&mut self.comps, ShareNode::Opaque(leaf))
    }

    /// Mint a computation-family occurrence reference.
    ///
    /// # Errors
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn comp_bound(
        &mut self,
        bound: Bound,
    ) -> Result<ShareId<Comp>, MintError>
    {
        mint(&mut self.comps, ShareNode::Bound(bound))
    }

    /// Mint a computation-family sharing former after checking all referenced
    /// legs.
    ///
    /// # Errors
    /// [`MintError::ChildOutOfBounds`] for an unresolved leg, or
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn comp_share(
        &mut self,
        sharing: Sharing<Comp>,
    ) -> Result<ShareId<Comp>, MintError>
    {
        self.check_sharing(&sharing)?;
        mint(&mut self.comps, ShareNode::Share(sharing))
    }

    /// Mint a computation-family graft after checking every child.
    ///
    /// # Errors
    /// [`MintError::ChildOutOfBounds`] for an unresolved child, or
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn comp_graft(
        &mut self,
        graft: Graft<Comp>,
    ) -> Result<ShareId<Comp>, MintError>
    {
        self.check_graft_children(&graft.children)?;
        mint(&mut self.comps, ShareNode::Graft(graft))
    }

    /// Mint an opaque value-type leaf.
    ///
    /// # Errors
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn value_type_opaque(
        &mut self,
        leaf: ValueType,
    ) -> Result<ShareId<ValueType>, MintError>
    {
        mint(&mut self.value_types, ShareNode::Opaque(leaf))
    }

    /// Mint a value-type-family occurrence reference.
    ///
    /// # Errors
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn value_type_bound(
        &mut self,
        bound: Bound,
    ) -> Result<ShareId<ValueType>, MintError>
    {
        mint(&mut self.value_types, ShareNode::Bound(bound))
    }

    /// Mint a value-type-family sharing former after checking all referenced
    /// legs.
    ///
    /// # Errors
    /// [`MintError::ChildOutOfBounds`] for an unresolved leg, or
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn value_type_share(
        &mut self,
        sharing: Sharing<ValueType>,
    ) -> Result<ShareId<ValueType>, MintError>
    {
        self.check_sharing(&sharing)?;
        mint(&mut self.value_types, ShareNode::Share(sharing))
    }

    /// Mint a value-type-family graft after checking every child.
    ///
    /// # Errors
    /// [`MintError::ChildOutOfBounds`] for an unresolved child, or
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn value_type_graft(
        &mut self,
        graft: Graft<ValueType>,
    ) -> Result<ShareId<ValueType>, MintError>
    {
        self.check_graft_children(&graft.children)?;
        mint(&mut self.value_types, ShareNode::Graft(graft))
    }

    /// Mint an opaque computation-type leaf.
    ///
    /// # Errors
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn comp_type_opaque(
        &mut self,
        leaf: CompType,
    ) -> Result<ShareId<CompType>, MintError>
    {
        mint(&mut self.comp_types, ShareNode::Opaque(leaf))
    }

    /// Mint a computation-type-family occurrence reference.
    ///
    /// # Errors
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn comp_type_bound(
        &mut self,
        bound: Bound,
    ) -> Result<ShareId<CompType>, MintError>
    {
        mint(&mut self.comp_types, ShareNode::Bound(bound))
    }

    /// Mint a computation-type-family sharing former after checking all
    /// referenced legs.
    ///
    /// # Errors
    /// [`MintError::ChildOutOfBounds`] for an unresolved leg, or
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn comp_type_share(
        &mut self,
        sharing: Sharing<CompType>,
    ) -> Result<ShareId<CompType>, MintError>
    {
        self.check_sharing(&sharing)?;
        mint(&mut self.comp_types, ShareNode::Share(sharing))
    }

    /// Mint a computation-type-family graft after checking every child.
    ///
    /// # Errors
    /// [`MintError::ChildOutOfBounds`] for an unresolved child, or
    /// [`MintError::CapacityExhausted`] if the family table is full.
    #[inline]
    pub fn comp_type_graft(
        &mut self,
        graft: Graft<CompType>,
    ) -> Result<ShareId<CompType>, MintError>
    {
        self.check_graft_children(&graft.children)?;
        mint(&mut self.comp_types, ShareNode::Graft(graft))
    }

    /// Resolve a value id (checked; `None` fails closed).
    #[inline]
    #[must_use]
    pub fn value_node(
        &self,
        id: ShareId<Value>,
    ) -> Option<&ShareNode<Value>>
    {
        lookup(&self.values, id)
    }

    /// Resolve a computation id (checked; `None` fails closed).
    #[inline]
    #[must_use]
    pub fn comp_node(
        &self,
        id: ShareId<Comp>,
    ) -> Option<&ShareNode<Comp>>
    {
        lookup(&self.comps, id)
    }

    /// Resolve a value-type id (checked; `None` fails closed).
    #[inline]
    #[must_use]
    pub fn value_type_node(
        &self,
        id: ShareId<ValueType>,
    ) -> Option<&ShareNode<ValueType>>
    {
        lookup(&self.value_types, id)
    }

    /// Resolve a computation-type id (checked; `None` fails closed).
    #[inline]
    #[must_use]
    pub fn comp_type_node(
        &self,
        id: ShareId<CompType>,
    ) -> Option<&ShareNode<CompType>>
    {
        lookup(&self.comp_types, id)
    }
}

/// An erased host term of any family: the environment entry and result form
/// of the erasure fold.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnyHost
{
    /// An erased value.
    Value(Value),
    /// An erased computation.
    Comp(Comp),
    /// An erased value type.
    ValueType(ValueType),
    /// An erased computation type.
    CompType(CompType),
}

impl AnyHost
{
    /// The family the erased term belongs to.
    ///
    /// # Contract
    /// - ensures: the family's tag matches the variant.
    /// - panics: none.
    #[inline]
    #[must_use]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Family dispatch intentionally matches the borrowed erased host by variant."
    )]
    pub fn family(&self) -> ShareFamily
    {
        match self {
            | Self::Value(_) => ShareFamily::Value,
            | Self::Comp(_) => ShareFamily::Comp,
            | Self::ValueType(_) => ShareFamily::ValueType,
            | Self::CompType(_) => ShareFamily::CompType,
        }
    }
}

impl From<Value> for AnyHost
{
    #[inline]
    fn from(value: Value) -> Self
    {
        Self::Value(value)
    }
}

impl From<Comp> for AnyHost
{
    #[inline]
    fn from(value: Comp) -> Self
    {
        Self::Comp(value)
    }
}

impl From<ValueType> for AnyHost
{
    #[inline]
    fn from(value: ValueType) -> Self
    {
        Self::ValueType(value)
    }
}

impl From<CompType> for AnyHost
{
    #[inline]
    fn from(value: CompType) -> Self
    {
        Self::CompType(value)
    }
}

/// The family dispatch behind the generic erasure step: one table accessor
/// and one id tag per family.
trait Family: Clone + Into<AnyHost>
{
    /// The family's arena table.
    fn table(arena: &ShareArena) -> &[ShareNode<Self>];

    /// The family-tagged form of one of the family's ids.
    fn tag(id: ShareId<Self>) -> AnyShareId;

    /// The family's tag.
    fn family() -> ShareFamily;
}

impl Family for Value
{
    #[inline]
    fn table(arena: &ShareArena) -> &[ShareNode<Self>]
    {
        &arena.values
    }

    #[inline]
    fn tag(id: ShareId<Self>) -> AnyShareId
    {
        AnyShareId::Value(id)
    }

    #[inline]
    fn family() -> ShareFamily
    {
        ShareFamily::Value
    }
}
impl Family for Comp
{
    #[inline]
    fn table(arena: &ShareArena) -> &[ShareNode<Self>]
    {
        &arena.comps
    }

    #[inline]
    fn tag(id: ShareId<Self>) -> AnyShareId
    {
        AnyShareId::Comp(id)
    }

    #[inline]
    fn family() -> ShareFamily
    {
        ShareFamily::Comp
    }
}

impl Family for ValueType
{
    #[inline]
    fn table(arena: &ShareArena) -> &[ShareNode<Self>]
    {
        &arena.value_types
    }

    #[inline]
    fn tag(id: ShareId<Self>) -> AnyShareId
    {
        AnyShareId::ValueType(id)
    }

    #[inline]
    fn family() -> ShareFamily
    {
        ShareFamily::ValueType
    }
}

impl Family for CompType
{
    #[inline]
    fn table(arena: &ShareArena) -> &[ShareNode<Self>]
    {
        &arena.comp_types
    }

    #[inline]
    fn tag(id: ShareId<Self>) -> AnyShareId
    {
        AnyShareId::CompType(id)
    }

    #[inline]
    fn family() -> ShareFamily
    {
        ShareFamily::CompType
    }
}

/// A seam-plug failure or an overlay fault, as one typed erasure error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EraseError
{
    /// A [`ShareNode::Bound`] reference escaped its closures: the distance
    /// exceeded the enclosing share count, or the position exceeded the
    /// closure's arity.
    DanglingBound(Bound),
    /// An id did not resolve in the arena (dangling or outside this run).
    ArenaFault(AnyShareId),
    /// A template violates seam syntax or linearity before plugging.
    Seam(SeamProblem),
    /// A graft child's family disagreed with its seam position (seam
    /// positions are value positions at this rung).
    FamilyMismatch
    {
        /// The position's family.
        expected: ShareFamily,
        /// The child's family.
        found: ShareFamily,
    },
    /// A template seam had no child to plug (an index past the child count).
    UnresolvedSeam(SeamIndex),
    /// The fold's own stack discipline broke — unreachable on well-formed
    /// input, kept fail-closed.
    TraversalInvariant,
}
impl From<SeamProblem> for EraseError
{
    #[inline]
    fn from(value: SeamProblem) -> Self
    {
        Self::Seam(value)
    }
}

/// The canonical name of a template's `index`-th seam.
///
/// # Contract
/// - ensures: `seam.k` with `k` the index in decimal, no leading zeros —
///   exactly the form [`parse_seam`] accepts.
/// - provides: the only constructor templates and tests should use for seam
///   names.
/// - panics: none.
#[inline]
#[must_use]
pub fn seam_name(index: SeamIndex) -> String
{
    alloc::format!("{SEAM_PREFIX}{}", u32::from(index))
}

/// Parse a strict seam name (`seam.k`, decimal, no leading zeros).
///
/// # Contract
/// - ensures: `Some` iff `name` is exactly a canonical seam spelling; a name
///   that merely starts with the prefix but is not canonical is `None`, which
///   the template validator reports as a malformed seam claim.
/// - provides: the plug's seam recognizer.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the recognizer is separated by the canonical spellings
///   `seam.0` and `seam.37`, the prefix-only, non-numeric, and leading-zero
///   (`seam.00`) rejects.
/// - witness: `share::tests::seam_names_parse_strictly`
#[inline]
#[must_use]
#[expect(
    primitive_signature,
    reason = "The public seam parser bridges syntax text into the semantic SeamIndex wrapper."
)]
pub fn parse_seam(name: &str) -> Option<SeamIndex>
{
    let digits = name.strip_prefix(SEAM_PREFIX)?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return None;
    }
    let index = digits.parse::<u32>().ok()?;
    Some(SeamIndex::from(index))
}

/// One closure environment entry of the erasure fold: the erased shared term
/// plus the arity that bounds reference positions.
struct ShareEntry
{
    /// The closure's occurrence count.
    arity: Arity,
    /// The erased shared term, computed once per closure activation.
    erased: AnyHost,
}

/// One pending step of the erasure fold (first-order task data, the worklist
/// discipline).
enum EraseTask
{
    /// Evaluate one overlay node.
    Eval(AnyShareId),
    /// Move a just-erased shared leg into the closure environment.
    FinishShare
    {
        /// The closure's occurrence count.
        arity: Arity,
    },
    /// Leave one closure's scope.
    PopEnv,
    /// Seam-plug erased children into a template.
    Plug
    {
        /// The template, carried whole.
        template: AnyHost,
        /// How many erased children the plug consumes.
        children: ChildCount,
    },
}

/// Resolve a [`Bound`] reference against the closure environment.
///
/// # Contract
/// - ensures: the erased shared term of the `distance`-th enclosing closure
///   when both the distance and the position are in range.
/// - fails: [`EraseError::DanglingBound`] on an out-of-range distance or
///   position (fail-closed).
/// - panics: none.
fn resolve_bound(
    env: &[ShareEntry],
    bound: Bound,
) -> Result<&AnyHost, EraseError>
{
    let distance = usize::try_from(u32::from(bound.distance)).unwrap_or(usize::MAX);
    if distance >= env.len() {
        return Err(EraseError::DanglingBound(bound));
    }
    let index = env.len().saturating_sub(1).saturating_sub(distance);
    let Some(entry) = env.get(index)
    else {
        return Err(EraseError::DanglingBound(bound));
    };
    let position = usize::try_from(u32::from(bound.position)).unwrap_or(usize::MAX);
    let arity = usize::try_from(u32::from(entry.arity)).unwrap_or(usize::MAX);
    if position >= arity {
        return Err(EraseError::DanglingBound(bound));
    }
    Ok(&entry.erased)
}

/// Queue one overlay node's evaluation (the per-family dispatch of the fold).
///
/// # Contract
/// - requires: every child id of the node resolves in `arena`.
/// - ensures: the node's erased form lands on `results` once the queued tasks
///   drain; a [`ShareNode::Share`] queues its shared leg, then the environment
///   push, then its body, then the scope pop, so the shared leg is erased
///   exactly once per closure activation, under the environment enclosing the
///   closure.
/// - fails: [`EraseError::ArenaFault`] on a dangling id,
///   [`EraseError::DanglingBound`] on an escaping reference.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Erasure dispatch intentionally matches borrowed arena nodes by variant."
)]
fn eval_node<L>(
    arena: &ShareArena,
    id: ShareId<L>,
    tasks: &mut Vec<EraseTask>,
    env: &[ShareEntry],
    results: &mut Vec<AnyHost>,
) -> Result<(), EraseError>
where
    L: Family + TemplateWalk,
{
    let Some(node) = lookup(L::table(arena), id)
    else {
        return Err(EraseError::ArenaFault(L::tag(id)));
    };
    match node {
        | ShareNode::Opaque(leaf) => {
            results.push(leaf.clone().into());
        },
        | ShareNode::Bound(bound) => {
            let erased = resolve_bound(env, *bound)?;
            results.push(erased.clone());
        },
        | ShareNode::Share(Sharing {
            arity,
            shared,
            body,
        }) => {
            tasks.push(EraseTask::PopEnv);
            tasks.push(EraseTask::Eval(L::tag(*body)));
            tasks.push(EraseTask::FinishShare { arity: *arity });
            tasks.push(EraseTask::Eval(*shared));
        },
        | ShareNode::Graft(graft) => {
            check_template(graft).map_err(|error| match error {
                | ValidationError::Seam(problem) => EraseError::from(problem),
                | _ => EraseError::TraversalInvariant,
            })?;
            tasks.push(EraseTask::Plug {
                template: graft.template.clone().into(),
                children: ChildCount::from(graft.children.len()),
            });
            for child in graft.children.iter().rev() {
                tasks.push(EraseTask::Eval(*child));
            }
        },
    }
    Ok(())
}

/// Run the erasure fold to one host term (the shared engine of the four
/// public faces).
///
/// # Contract
/// - requires: `root` resolves in `arena` (a dangling root fails closed as
///   [`EraseError::ArenaFault`]).
/// - ensures: the overlay's full unsharing — the ordinary unshared spelling the
///   existing pipeline consumes. The fold drains an explicit task stack; the
///   environment and result stacks stay balanced by the task discipline
///   ([`EraseError::TraversalInvariant`] guards the invariant fail-closed).
/// - provides: the policy-free resolver into the unshared pipeline.
/// - fails: [`EraseError`] on a dangling id, an escaping reference, a family
///   mismatch at a seam, or an unplugged seam.
/// - panics: none.
fn erase_root(
    arena: &ShareArena,
    root: AnyShareId,
) -> Result<AnyHost, EraseError>
{
    let mut tasks = Vec::new();
    tasks.push(EraseTask::Eval(root));
    let mut env: Vec<ShareEntry> = Vec::new();
    let mut results: Vec<AnyHost> = Vec::new();
    while let Some(task) = tasks.pop() {
        match task {
            | EraseTask::Eval(AnyShareId::Value(id)) => {
                eval_node::<Value>(arena, id, &mut tasks, &env, &mut results)?;
            },
            | EraseTask::Eval(AnyShareId::Comp(id)) => {
                eval_node::<Comp>(arena, id, &mut tasks, &env, &mut results)?;
            },
            | EraseTask::Eval(AnyShareId::ValueType(id)) => {
                eval_node::<ValueType>(arena, id, &mut tasks, &env, &mut results)?;
            },
            | EraseTask::Eval(AnyShareId::CompType(id)) => {
                eval_node::<CompType>(arena, id, &mut tasks, &env, &mut results)?;
            },
            | EraseTask::FinishShare { arity } => {
                let Some(erased) = results.pop()
                else {
                    return Err(EraseError::TraversalInvariant);
                };
                env.push(ShareEntry { arity, erased });
            },
            | EraseTask::PopEnv => {
                if env.pop().is_none() {
                    return Err(EraseError::TraversalInvariant);
                }
            },
            | EraseTask::Plug {
                ref template,
                children,
            } => {
                plug_step(template, children, &mut results)?;
            },
        }
    }
    if !env.is_empty() {
        return Err(EraseError::TraversalInvariant);
    }
    let Some(result) = results.pop()
    else {
        return Err(EraseError::TraversalInvariant);
    };
    if !results.is_empty() {
        return Err(EraseError::TraversalInvariant);
    }
    Ok(result)
}

/// Consume erased children off the result stack and seam-plug them into one
/// template.
///
/// # Contract
/// - requires: `results` holds at least `children` entries.
/// - ensures: the plugged term replaces the consumed children on the stack.
/// - fails: [`EraseError::TraversalInvariant`] on a stack underflow, or any
///   plug failure of [`plug_host`].
/// - panics: none.
fn plug_step(
    template: &AnyHost,
    children: ChildCount,
    results: &mut Vec<AnyHost>,
) -> Result<(), EraseError>
{
    let count = usize::from(children);
    if results.len() < count {
        return Err(EraseError::TraversalInvariant);
    }
    let erased = results.split_off(results.len().saturating_sub(count));
    let plugged = plug_host(template, &erased)?;
    results.push(plugged);
    Ok(())
}

/// Extract the value children a template's seams expect (family agreement at
/// the value positions this rung supports) and run the family-specific plug.
///
/// # Contract
/// - ensures: the template with every `seam.k` replaced by the `k`-th child,
///   capture-permitting; every child must be value-family.
/// - fails: [`EraseError::FamilyMismatch`] on a non-value child.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Host plugging intentionally matches borrowed erased templates by variant."
)]
fn plug_host(
    template: &AnyHost,
    children: &[AnyHost],
) -> Result<AnyHost, EraseError>
{
    let mut values: Vec<Value> = Vec::with_capacity(children.len());
    for child in children {
        match child {
            | AnyHost::Value(value) => values.push(value.clone()),
            | _ => {
                return Err(EraseError::FamilyMismatch {
                    expected: ShareFamily::Value,
                    found: child.family(),
                });
            },
        }
    }
    Ok(match template {
        | AnyHost::Value(template) => {
            AnyHost::Value(plug_root(PlugTask::Value(template), &values)?)
        },
        | AnyHost::Comp(template) => AnyHost::Comp(plug_root(PlugTask::Comp(template), &values)?),
        | AnyHost::ValueType(template) => {
            AnyHost::ValueType(plug_root(PlugTask::ValueType(template), &values)?)
        },
        | AnyHost::CompType(template) => {
            AnyHost::CompType(plug_root(PlugTask::CompType(template), &values)?)
        },
    })
}

/// Erase a value-family overlay to its unshared spelling.
///
/// # Contract
/// - requires: `root` resolves in `arena` (else [`EraseError::ArenaFault`]).
/// - ensures: the full unsharing of the overlay; policy-invariant (see the
///   module doc).
/// - provides: the value-family resolver into the existing pipeline.
/// - fails: [`EraseError`] on a dangling id, an escaping reference, a family
///   mismatch at a seam, or an unplugged seam.
/// - panics: none.
///
/// # Errors
/// [`EraseError`], per the variants.
///
/// # Adequacy
/// - hypothesis: L2/L3 — the resolver agrees with the hand-built unshared
///   spelling on share-carrying shapes (the L2 oracle is the spelling itself,
///   external to the fold); the L3 residues are opaque passthrough, repeated
///   occurrences, nested closures, and the fail-closed boundaries.
/// - witness: `share::tests::opaque_value_erases_to_itself`
/// - witness: `share::tests::a_bound_resolves_to_the_shared_value`
/// - witness: `share::tests::repeated_occurrences_plug_the_same_erasure`
/// - witness: `share::tests::value_grafts_plug_through_every_composite_shape`
/// - witness: `share::tests::under_binder_sharing_erases_capture_permitting`
/// - witness: `share::tests::a_dangling_bound_fails_closed`
#[inline]
pub fn erase_value(
    arena: &ShareArena,
    root: ShareId<Value>,
) -> Result<Value, EraseError>
{
    match erase_root(arena, AnyShareId::Value(root))? {
        | AnyHost::Value(value) => Ok(value),
        | _ => Err(EraseError::TraversalInvariant),
    }
}

/// Erase a computation-family overlay to its unshared spelling.
///
/// # Contract
/// - requires: `root` resolves in `arena` (else [`EraseError::ArenaFault`]).
/// - ensures: the full unsharing of the overlay; policy-invariant.
/// - provides: the computation-family resolver into the existing pipeline.
/// - fails: [`EraseError`], as [`erase_value`].
/// - panics: none.
///
/// # Errors
/// [`EraseError`], per the variants.
///
/// # Adequacy
/// - hypothesis: L2/L3 — as [`erase_value`]; the checking-path differential
///   lives in the integration suite.
/// - witness: `share::tests::a_shared_comp_erases_to_the_unshared_spelling`
/// - witness: `share::tests::under_binder_sharing_erases_capture_permitting`
/// - witness: `tests::share::a_share_carrying_value_checks_identically_to_its_unshared_spelling`
#[inline]
pub fn erase_comp(
    arena: &ShareArena,
    root: ShareId<Comp>,
) -> Result<Comp, EraseError>
{
    match erase_root(arena, AnyShareId::Comp(root))? {
        | AnyHost::Comp(comp) => Ok(comp),
        | _ => Err(EraseError::TraversalInvariant),
    }
}

/// Erase a value-type-family overlay to its unshared spelling.
///
/// # Contract
/// - requires: `root` resolves in `arena` (else [`EraseError::ArenaFault`]).
/// - ensures: the full unsharing of the overlay; policy-invariant.
/// - provides: the value-type-family resolver.
/// - fails: [`EraseError`], as [`erase_value`].
/// - panics: none.
///
/// # Errors
/// [`EraseError`], per the variants.
///
/// # Adequacy
/// - hypothesis: L3 — separated by a `Path` template whose endpoint seams plug
///   values under a type former.
/// - witness: `share::tests::value_type_seams_plug_under_path`
#[inline]
pub fn erase_value_type(
    arena: &ShareArena,
    root: ShareId<ValueType>,
) -> Result<ValueType, EraseError>
{
    match erase_root(arena, AnyShareId::ValueType(root))? {
        | AnyHost::ValueType(value_type) => Ok(value_type),
        | _ => Err(EraseError::TraversalInvariant),
    }
}

/// Erase a computation-type-family overlay to its unshared spelling.
///
/// # Contract
/// - requires: `root` resolves in `arena` (else [`EraseError::ArenaFault`]).
/// - ensures: the full unsharing of the overlay; policy-invariant.
/// - provides: the computation-type-family resolver.
/// - fails: [`EraseError`], as [`erase_value`].
/// - panics: none.
///
/// # Errors
/// [`EraseError`], per the variants.
///
/// # Adequacy
/// - hypothesis: L3 — separated by an `F`/`Arrow` template whose value-type
///   seams plug through the row and the arrow spine.
/// - witness: `share::tests::comp_type_seams_plug_through_the_arrow`
#[inline]
pub fn erase_comp_type(
    arena: &ShareArena,
    root: ShareId<CompType>,
) -> Result<CompType, EraseError>
{
    match erase_root(arena, AnyShareId::CompType(root))? {
        | AnyHost::CompType(comp_type) => Ok(comp_type),
        | _ => Err(EraseError::TraversalInvariant),
    }
}

/// A typed failure of a duplication entry.
///
/// Refusal is data, as it is for the projection and the erasure: a policy
/// answers negatively where it cannot produce a result, never by panic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DuplicateError
{
    /// The baseline stance's erasure failed: the overlay did not erase, so
    /// there is no unshared spelling to hand the caller.
    Erase(EraseError),
}

/// Duplicate a shared value under the installed policy.
///
/// The parameter every duplication entry takes as a runtime argument, and
/// the first entry to take it: under [`DuplicationPolicy::EraseAndClone`]
/// the result is the total erasure — one owned unshared spelling, cloned by
/// the caller per occurrence, behaviorally the pipeline that exists today.
/// [`DuplicationPolicy::Spinal`] is unanswerable today because its seat is
/// uninhabited; the arm stays total with no `unreachable!`, and the fourth
/// rung installs the strategy into this signature without breaking callers.
///
/// Only the two evaluation families gain an entry: duplication is what a
/// forced redex does to a **term**, and types are never forced into
/// redexes, so the type families keep their policy-free erasure alone.
///
/// # Contract
/// - requires: `root` resolves in `arena` (else [`DuplicateError::Erase`] from
///   the baseline's erasure).
/// - ensures: under the baseline stance, exactly [`erase_value`]'s result — the
///   reference spelling every later stance's result replays against.
/// - provides: the dispatch site the policy parameter enters through.
/// - fails: [`DuplicateError`], per the variant.
/// - panics: none.
///
/// # Errors
/// [`DuplicateError`], per the variant.
///
/// # Adequacy
/// - hypothesis: L3 — the one answerable stance is separated by a
///   share-carrying pair whose duplication must equal its erasure, and by the
///   default-stance check that keeps the baseline the stance an engine gets
///   without asking.
/// - witness: `share::tests::erase_and_clone_duplicates_a_value_by_erasure`
/// - witness: `share::tests::the_baseline_policy_is_the_default`
#[inline]
pub fn duplicate_value(
    arena: &ShareArena,
    root: ShareId<Value>,
    policy: DuplicationPolicy,
) -> Result<Value, DuplicateError>
{
    match policy {
        | DuplicationPolicy::EraseAndClone => {
            erase_value(arena, root).map_err(DuplicateError::Erase)
        },
        | DuplicationPolicy::Spinal(seat) => match seat {},
    }
}

/// Duplicate a shared computation under the installed policy.
///
/// # Contract
/// - requires: `root` resolves in `arena` (else [`DuplicateError::Erase`] from
///   the baseline's erasure).
/// - ensures: under the baseline stance, exactly [`erase_comp`]'s result.
/// - provides: the computation-family dispatch site, the [`duplicate_value`]
///   discipline mirrored.
/// - fails: [`DuplicateError`], per the variant.
/// - panics: none.
///
/// # Errors
/// [`DuplicateError`], per the variant.
///
/// # Adequacy
/// - hypothesis: L3 — separated by a shared computation whose duplication must
///   equal its erasure.
/// - witness: `share::tests::erase_and_clone_duplicates_a_comp_by_erasure`
#[inline]
pub fn duplicate_comp(
    arena: &ShareArena,
    root: ShareId<Comp>,
    policy: DuplicationPolicy,
) -> Result<Comp, DuplicateError>
{
    match policy {
        | DuplicationPolicy::EraseAndClone => {
            erase_comp(arena, root).map_err(DuplicateError::Erase)
        },
        | DuplicationPolicy::Spinal(seat) => match seat {},
    }
}

/// One pending step of the seam-plug traversal (descend one node, or finish
/// one rebuild).
enum PlugTask<'template>
{
    /// Descend a value.
    Value(&'template Value),
    /// Descend a computation.
    Comp(&'template Comp),
    /// Descend a reified stack.
    Stack(&'template Stack),
    /// Descend a value type.
    ValueType(&'template ValueType),
    /// Descend a computation type.
    CompType(&'template CompType),
    /// Descend a static argument.
    StaticArg(&'template StaticArg),
    /// Descend a static neutral.
    StaticNeutral(&'template StaticNeutral),
    /// Descend a static term.
    StaticTerm(&'template StaticTerm),
    /// Descend a ground type.
    Ty(&'template Ty),
    /// Descend an effect signature.
    Sig(&'template EffectSig),
    /// Descend one signature operation.
    Op(&'template EffectOp),
    /// Descend an effect row.
    Row(&'template EffectRow),
    /// Rebuild one node from the result stacks.
    Finish(PlugFinish),
}

/// One handler-clause's name data, carried by [`PlugFinish::CompHandle`].
struct OpClauseMeta
{
    /// The handled operation's name.
    op: String,
    /// The payload binder.
    payload: String,
    /// The resumption binder.
    resume: String,
}

/// The rebuild instruction of a [`PlugTask::Finish`] frame: which stacks to
/// pop and which node to push.
enum PlugFinish
{
    /// Pop two values, push the pair.
    ValuePair,
    /// Pop one value, push the injection.
    ValueInj(Side),
    /// Pop `count` values, push the list.
    ValueList(ChildCount),
    /// Pop `labels.len()` values, zip with labels, push the record.
    ValueRecord(Vec<String>),
    /// Pop one computation, push the thunk.
    ValueThunk(Grade),
    /// Pop one computation, push the pure-computation embedding.
    ValueRun,
    /// Pop one value type and one value, push the annotation.
    ValueAnnot,
    /// Pop one stack, push the reified-stack value.
    ValueStk,
    /// Pop one value, push the reflexivity proof.
    ValueHere,
    /// Pop one value, push the constructor value.
    ValueCtor
    {
        /// The datatype's nominal identity.
        id: DataId,
        /// The constructor's tag.
        tag: usize,
    },
    /// Pop the payload and `count` witness types, push a packed value.
    ValuePack(ChildCount),
    /// Pop one computation (and one value type when annotated), push the
    /// abstraction.
    CompAbs
    {
        /// The bound variable.
        name: String,
        /// Whether the binder carries an annotation.
        annotated: bool,
    },
    /// Pop one value and one computation, push the application.
    CompApp,
    /// Pop one value, push the returner.
    CompRet,
    /// Pop two computations, push the sequencing.
    CompBind
    {
        /// The bound variable.
        name: String,
    },
    /// Pop one value, push the force.
    CompForce,
    /// Pop two computations and one value, push the sum case.
    CompCase
    {
        /// The first arm's binder.
        fst_name: String,
        /// The second arm's binder.
        snd_name: String,
    },
    /// Pop `binders.len()` computations and one value, push the data case.
    CompDataCase
    {
        /// The per-constructor binders, in arm order.
        binders: Vec<String>,
    },
    /// Pop two computations and one value, push the list case.
    CompListCase
    {
        /// The cons arm's head binder.
        head: String,
        /// The cons arm's tail binder.
        tail: String,
    },
    /// Pop one computation, one value, and (with a motive) one computation
    /// type, push the split.
    CompSplit
    {
        /// The first component's binder.
        fst_name: String,
        /// The second component's binder.
        snd_name: String,
        /// The motive's scrutinee binder, when present.
        motive: Option<String>,
    },
    /// Pop one value, push the record projection.
    CompRecordProj
    {
        /// The projected label.
        label: String,
    },
    /// Pop two computations, push the lazy pair.
    CompWith,
    /// Pop one computation, push the projection.
    CompPrj(Side),
    /// Pop one value, push the grade split.
    CompDup,
    /// Pop one value, push the grade drop.
    CompDrop,
    /// Pop one signature and one value, push the perform.
    CompPerform
    {
        /// The operation's name.
        name: String,
    },
    /// Pop one signature, `2 + metas.len()` computations, push the handler.
    CompHandle
    {
        /// The return clause's binder.
        ret_name: String,
        /// The clauses' name data, in source order.
        metas: Vec<OpClauseMeta>,
    },
    /// Pop one computation and one value, push the resume.
    CompResume,
    /// Pop one computation, push the reset.
    CompReset,
    /// Pop one computation, push the shift.
    CompShift
    {
        /// The continuation binder.
        name: String,
    },
    /// Pop one computation, push the fixpoint.
    CompFix
    {
        /// The self-reference binder.
        name: String,
    },
    /// Pop `argc` values, push the native.
    CompNative
    {
        /// The primitive tag.
        prim: NativePrim,
        /// The accumulated argument count.
        argc: ChildCount,
    },
    /// Pop one computation, one computation type, and one value, push the
    /// identity eliminator.
    CompWalk
    {
        /// The motive's left-endpoint binder.
        motive_x: String,
        /// The motive's right-endpoint binder.
        motive_y: String,
        /// The motive's path binder.
        motive_q: String,
        /// The base's diagonal binder.
        base_x: String,
    },
    /// Pop the body, signature, and scrutinee, push a package elimination.
    CompUnpack
    {
        /// The atom ids introduced by the elimination.
        atoms: Vec<SealId>,
        /// The module binder.
        binder: String,
    },
    /// Pop one value and one stack, push the argument frame.
    StackArg,
    /// Pop one computation and one stack, push the bind frame.
    StackBind
    {
        /// The binder.
        name: String,
    },
    /// Pop one stack, push the projection frame.
    StackPrj(Side),
    /// Pop two value types, push the product.
    ValueTypeProd,
    /// Pop two value types, push the sum.
    ValueTypeSum,
    /// Pop one value type, push the list type.
    ValueTypeList,
    /// Pop `labels.len()` value types, zip with labels, push the record type.
    ValueTypeRecord
    {
        /// The field labels, in canonical order.
        labels: Vec<String>,
    },
    /// Pop one computation type, push the thunk type.
    ValueTypeThunk(Grade),
    /// Pop two computation types, push the stack type.
    ValueTypeStk,
    /// Pop two values and one value type, push the identity type.
    ValueTypePath,
    /// Pop one static term, push a typed static argument.
    StaticArgType,
    /// Pop one value, push a value-index static argument.
    StaticArgValue,
    /// Pop one static neutral and one static argument, push the application.
    StaticNeutralApp,
    /// Pop one quoted type, push a static quote.
    StaticTermQuote,
    /// Pop one static term, push a static Pi.
    StaticTermPi(StaticBinder),
    /// Pop one static term, push a static lambda.
    StaticTermLam(StaticBinder),
    /// Pop one static term and one static argument, push the application.
    StaticTermApp,
    /// Pop one static neutral, push a neutral static term.
    StaticTermNeutral,
    /// Pop one value type, push a quoted value type.
    TyValue,
    /// Pop one computation type, push a quoted computation type.
    TyComp,
    /// Pop one rebuilt static neutral, push the value type-family application.
    ValueTypeFamily
    {
        /// The family result classifier.
        result: Classifier,
    },
    /// Pop one rebuilt static neutral, push the computation type-family
    /// application.
    CompTypeFamily
    {
        /// The family result classifier.
        result: Classifier,
    },
    /// Pop `argc` value types, push the data application.
    ValueTypeData
    {
        /// The datatype's nominal identity.
        id: DataId,
        /// The argument count.
        argc: ChildCount,
    },
    /// Pop two value types, push the dependent pair.
    ValueTypeSigma
    {
        /// The head binder.
        binder: String,
    },
    /// Pop one value type, push a package type.
    ValueTypePackage
    {
        /// The package grade.
        grade: Grade,
        /// The abstract component binders.
        abstracts: Vec<String>,
    },
    /// Pop one value type and one row, push the returner type.
    CompTypeF,
    /// Pop one computation type and one value type, push the function type
    /// with the binder it was taken apart with.
    CompTypeArrow(Option<String>),
    /// Pop two computation types, push the lazy product.
    CompTypeWith,
    /// Pop two value types, push one operation.
    OpFinish
    {
        /// The operation's name.
        name: String,
    },
    /// Pop `count` operations, push the signature.
    SigFinish
    {
        /// The signature's name.
        name: String,
        /// The operation count.
        count: ChildCount,
    },
    /// Pop `count` signatures, push the row.
    RowFinish
    {
        /// The signature count.
        count: ChildCount,
    },
}

/// The seam-plug engine: one worklist over the public legacy syntax, one
/// result stack per syntactic sort.
struct PlugEngine<'template>
{
    /// The erased children the seams plug, in seam order.
    children: &'template [Value],
    /// The pending traversal steps.
    tasks: Vec<PlugTask<'template>>,
    /// Rebuilt values.
    values: Vec<Value>,
    /// Rebuilt computations.
    comps: Vec<Comp>,
    /// Rebuilt stacks.
    stacks: Vec<Stack>,
    /// Rebuilt value types.
    value_types: Vec<ValueType>,
    /// Rebuilt computation types.
    comp_types: Vec<CompType>,
    /// Rebuilt ground types quoted in static terms.
    tys: Vec<Ty>,
    /// Rebuilt static arguments.
    static_args: Vec<StaticArg>,
    /// Rebuilt static neutrals.
    static_neutrals: Vec<StaticNeutral>,
    /// Rebuilt static terms.
    static_terms: Vec<StaticTerm>,
    /// Rebuilt operations.
    ops: Vec<EffectOp>,
    /// Rebuilt signatures.
    sigs: Vec<EffectSig>,
    /// Rebuilt rows.
    rows: Vec<EffectRow>,
}

/// Run a plug to completion and return the single rebuilt term of the root's
/// sort.
///
/// # Contract
/// - ensures: the template with every `seam.k` replaced by the `k`-th child
///   (capture-permitting: no binder is renamed, no freshness is invented);
///   every other variable is preserved verbatim. The walk is an explicit
///   worklist — total on any template depth.
/// - provides: the shared engine behind the family-specific plugs.
/// - fails: [`EraseError::UnresolvedSeam`] on a seam past the child count,
///   [`EraseError::TraversalInvariant`] on a stack-discipline break.
/// - panics: none.
fn plug_root<'template, Sort>(
    root: PlugTask<'template>,
    children: &'template [Value],
) -> Result<Sort, EraseError>
where
    PlugEngine<'template>: PlugRoot<Sort>,
{
    let mut engine = PlugEngine {
        children,
        tasks: Vec::new(),
        values: Vec::new(),
        comps: Vec::new(),
        stacks: Vec::new(),
        value_types: Vec::new(),
        comp_types: Vec::new(),
        tys: Vec::new(),
        static_args: Vec::new(),
        static_neutrals: Vec::new(),
        static_terms: Vec::new(),
        ops: Vec::new(),
        sigs: Vec::new(),
        rows: Vec::new(),
    };
    engine.tasks.push(root);
    while let Some(task) = engine.tasks.pop() {
        engine.step(task)?;
    }
    engine.finish_root()
}

/// The root-result extraction of [`plug_root`]: which stack holds the
/// completed term.
trait PlugRoot<Sort>
{
    /// Pop the single rebuilt root, checking every stack balanced.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on an unbalanced stack.
    fn finish_root(&mut self) -> Result<Sort, EraseError>;
}

impl PlugRoot<Value> for PlugEngine<'_>
{
    fn finish_root(&mut self) -> Result<Value, EraseError>
    {
        let result = self.pop_value()?;
        self.expect_balanced()?;
        Ok(result)
    }
}

impl PlugRoot<Comp> for PlugEngine<'_>
{
    fn finish_root(&mut self) -> Result<Comp, EraseError>
    {
        let result = self.pop_comp()?;
        self.expect_balanced()?;
        Ok(result)
    }
}

impl PlugRoot<ValueType> for PlugEngine<'_>
{
    fn finish_root(&mut self) -> Result<ValueType, EraseError>
    {
        let result = self.pop_value_type()?;
        self.expect_balanced()?;
        Ok(result)
    }
}

impl PlugRoot<CompType> for PlugEngine<'_>
{
    fn finish_root(&mut self) -> Result<CompType, EraseError>
    {
        let result = self.pop_comp_type()?;
        self.expect_balanced()?;
        Ok(result)
    }
}

impl<'template> PlugEngine<'template>
{
    /// Every result stack empty (the post-order balance invariant).
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] when any stack retains a term.
    fn expect_balanced(&self) -> Result<(), EraseError>
    {
        if self.values.is_empty()
            && self.comps.is_empty()
            && self.stacks.is_empty()
            && self.value_types.is_empty()
            && self.comp_types.is_empty()
            && self.tys.is_empty()
            && self.static_args.is_empty()
            && self.static_neutrals.is_empty()
            && self.static_terms.is_empty()
            && self.ops.is_empty()
            && self.sigs.is_empty()
            && self.rows.is_empty()
        {
            Ok(())
        }
        else {
            Err(EraseError::TraversalInvariant)
        }
    }

    /// Pop one rebuilt value.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_value(&mut self) -> Result<Value, EraseError>
    {
        self.values.pop().ok_or(EraseError::TraversalInvariant)
    }

    /// Pop one rebuilt computation.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_comp(&mut self) -> Result<Comp, EraseError>
    {
        self.comps.pop().ok_or(EraseError::TraversalInvariant)
    }

    /// Pop one rebuilt stack.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_stack(&mut self) -> Result<Stack, EraseError>
    {
        self.stacks.pop().ok_or(EraseError::TraversalInvariant)
    }

    /// Pop one rebuilt value type.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_value_type(&mut self) -> Result<ValueType, EraseError>
    {
        self.value_types.pop().ok_or(EraseError::TraversalInvariant)
    }

    /// Pop one rebuilt computation type.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_comp_type(&mut self) -> Result<CompType, EraseError>
    {
        self.comp_types.pop().ok_or(EraseError::TraversalInvariant)
    }
    /// Pop one rebuilt ground type.
    fn pop_ty(&mut self) -> Result<Ty, EraseError>
    {
        self.tys.pop().ok_or(EraseError::TraversalInvariant)
    }

    /// Pop one rebuilt static argument.
    fn pop_static_arg(&mut self) -> Result<StaticArg, EraseError>
    {
        self.static_args.pop().ok_or(EraseError::TraversalInvariant)
    }

    /// Pop one rebuilt static neutral.
    fn pop_static_neutral(&mut self) -> Result<StaticNeutral, EraseError>
    {
        self.static_neutrals
            .pop()
            .ok_or(EraseError::TraversalInvariant)
    }

    /// Pop one rebuilt static term.
    fn pop_static_term(&mut self) -> Result<StaticTerm, EraseError>
    {
        self.static_terms
            .pop()
            .ok_or(EraseError::TraversalInvariant)
    }

    /// Pop one rebuilt signature.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_sig(&mut self) -> Result<EffectSig, EraseError>
    {
        self.sigs.pop().ok_or(EraseError::TraversalInvariant)
    }

    /// Pop one rebuilt row.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_row(&mut self) -> Result<EffectRow, EraseError>
    {
        self.rows.pop().ok_or(EraseError::TraversalInvariant)
    }

    /// Pop `count` rebuilt values, preserving their order.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_values(
        &mut self,
        count: ChildCount,
    ) -> Result<Vec<Value>, EraseError>
    {
        let count = usize::from(count);
        if self.values.len() < count {
            return Err(EraseError::TraversalInvariant);
        }
        Ok(self
            .values
            .split_off(self.values.len().saturating_sub(count)))
    }

    /// Pop `count` rebuilt computations, preserving their order.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_comps(
        &mut self,
        count: ChildCount,
    ) -> Result<Vec<Comp>, EraseError>
    {
        let count = usize::from(count);
        if self.comps.len() < count {
            return Err(EraseError::TraversalInvariant);
        }
        Ok(self.comps.split_off(self.comps.len().saturating_sub(count)))
    }

    /// Pop `count` rebuilt value types, preserving their order.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_value_types(
        &mut self,
        count: ChildCount,
    ) -> Result<Vec<ValueType>, EraseError>
    {
        let count = usize::from(count);
        if self.value_types.len() < count {
            return Err(EraseError::TraversalInvariant);
        }
        Ok(self
            .value_types
            .split_off(self.value_types.len().saturating_sub(count)))
    }

    /// Pop `count` rebuilt operations, preserving their order.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_ops(
        &mut self,
        count: ChildCount,
    ) -> Result<Vec<EffectOp>, EraseError>
    {
        let count = usize::from(count);
        if self.ops.len() < count {
            return Err(EraseError::TraversalInvariant);
        }
        Ok(self.ops.split_off(self.ops.len().saturating_sub(count)))
    }

    /// Pop `count` rebuilt signatures, preserving their order.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on underflow.
    fn pop_sigs(
        &mut self,
        count: ChildCount,
    ) -> Result<Vec<EffectSig>, EraseError>
    {
        let count = usize::from(count);
        if self.sigs.len() < count {
            return Err(EraseError::TraversalInvariant);
        }
        Ok(self.sigs.split_off(self.sigs.len().saturating_sub(count)))
    }

    /// Execute one traversal step.
    ///
    /// # Errors
    /// [`EraseError::UnresolvedSeam`] on a seam past the child count;
    /// [`EraseError::TraversalInvariant`] on a stack-discipline break.
    fn step(
        &mut self,
        task: PlugTask<'template>,
    ) -> Result<(), EraseError>
    {
        match task {
            | PlugTask::Value(value) => self.visit_value(value),
            | PlugTask::Comp(comp) => self.visit_comp(comp),
            | PlugTask::Stack(stack) => self.visit_stack(stack),
            | PlugTask::ValueType(value_type) => self.visit_value_type(value_type),
            | PlugTask::CompType(comp_type) => self.visit_comp_type(comp_type),
            | PlugTask::StaticArg(argument) => self.visit_static_arg(argument),
            | PlugTask::StaticNeutral(neutral) => self.visit_static_neutral(neutral),
            | PlugTask::StaticTerm(term) => self.visit_static_term(term),
            | PlugTask::Ty(ty) => self.visit_ty(ty),
            | PlugTask::Sig(sig) => self.visit_sig(sig),
            | PlugTask::Op(op) => self.visit_op(op),
            | PlugTask::Row(row) => self.visit_row(row),
            | PlugTask::Finish(finish) => self.finish(finish),
        }
    }

    /// Descend one value: seams plug, leaves clone, composites queue.
    ///
    /// # Errors
    /// [`EraseError::UnresolvedSeam`] on a seam past the child count.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_value(
        &mut self,
        value: &'template Value,
    ) -> Result<(), EraseError>
    {
        match value {
            | Value::Var(name) => match parse_seam(name) {
                | Some(seam) => {
                    let index = usize::try_from(u32::from(seam)).unwrap_or(usize::MAX);
                    let Some(child) = self.children.get(index)
                    else {
                        return Err(EraseError::UnresolvedSeam(seam));
                    };
                    self.values.push(child.clone());
                },
                | None => self.values.push(Value::Var(name.clone())),
            },
            | Value::Unit => self.values.push(Value::Unit),
            | Value::Int(literal) => self.values.push(Value::Int(*literal)),
            | Value::Str(text) => self.values.push(Value::Str(text.clone())),
            | Value::Num(literal) => self.values.push(Value::Num(*literal)),
            | Value::Hole(id) => self.values.push(Value::Hole(*id)),
            | Value::Pair(first, second) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValuePair));
                self.tasks.push(PlugTask::Value(second));
                self.tasks.push(PlugTask::Value(first));
            },
            | Value::Inj(side, payload) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValueInj(*side)));
                self.tasks.push(PlugTask::Value(payload));
            },
            | Value::List(elements) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValueList(ChildCount::from(
                        elements.len(),
                    ))));
                for element in elements.iter().rev() {
                    self.tasks.push(PlugTask::Value(element));
                }
            },
            | Value::Record(fields) => {
                let labels: Vec<String> = fields.keys().cloned().collect();
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValueRecord(labels)));
                for field in fields.values().rev() {
                    self.tasks.push(PlugTask::Value(field));
                }
            },
            | Value::Thunk(grade, body) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValueThunk(*grade)));
                self.tasks.push(PlugTask::Comp(body));
            },
            | Value::Run(body) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueRun));
                self.tasks.push(PlugTask::Comp(body));
            },
            | Value::Annot(inner, ty) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueAnnot));
                self.tasks.push(PlugTask::ValueType(ty));
                self.tasks.push(PlugTask::Value(inner));
            },
            | Value::Pack { witnesses, payload } => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValuePack(ChildCount::from(
                        witnesses.len(),
                    ))));
                self.tasks.push(PlugTask::Value(payload));
                for witness in witnesses.iter().rev() {
                    self.tasks.push(PlugTask::ValueType(witness));
                }
            },
            | Value::Stk(stack) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueStk));
                self.tasks.push(PlugTask::Stack(stack));
            },
            | Value::Here(witness) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueHere));
                self.tasks.push(PlugTask::Value(witness));
            },
            | Value::Ctor { id, tag, payload } => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueCtor {
                    id: id.clone(),
                    tag: *tag,
                }));
                self.tasks.push(PlugTask::Value(payload));
            },
        }
        Ok(())
    }

    /// Descend one computation.
    ///
    /// # Errors
    /// Never directly (descendants report their own).
    #[expect(
        clippy::pattern_type_mismatch,
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_comp(
        &mut self,
        comp: &'template Comp,
    ) -> Result<(), EraseError>
    {
        match comp {
            | Comp::Abs(name, annot, body) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompAbs {
                    name: name.clone(),
                    annotated: annot.is_some(),
                }));
                if let Some(ty) = annot {
                    self.tasks.push(PlugTask::ValueType(ty));
                }
                self.tasks.push(PlugTask::Comp(body));
            },
            | Comp::App(head, argument) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompApp));
                self.tasks.push(PlugTask::Value(argument));
                self.tasks.push(PlugTask::Comp(head));
            },
            | Comp::Ret(value) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompRet));
                self.tasks.push(PlugTask::Value(value));
            },
            | Comp::Bind(bound, name, continuation) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompBind {
                    name: name.clone(),
                }));
                self.tasks.push(PlugTask::Comp(continuation));
                self.tasks.push(PlugTask::Comp(bound));
            },
            | Comp::Force(value) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompForce));
                self.tasks.push(PlugTask::Value(value));
            },
            | Comp::Case(scrutinee, fst, snd) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompCase {
                    fst_name: fst.0.clone(),
                    snd_name: snd.0.clone(),
                }));
                self.tasks.push(PlugTask::Comp(&snd.1));
                self.tasks.push(PlugTask::Comp(&fst.1));
                self.tasks.push(PlugTask::Value(scrutinee));
            },
            | Comp::DataCase(scrutinee, arms) => {
                let binders: Vec<String> = arms.iter().map(|arm| arm.0.clone()).collect();
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::CompDataCase { binders }));
                for arm in arms.iter().rev() {
                    self.tasks.push(PlugTask::Comp(&arm.1));
                }
                self.tasks.push(PlugTask::Value(scrutinee));
            },
            | Comp::ListCase {
                scrut,
                nil,
                head,
                tail,
                cons,
            } => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompListCase {
                    head: head.clone(),
                    tail: tail.clone(),
                }));
                self.tasks.push(PlugTask::Comp(cons));
                self.tasks.push(PlugTask::Comp(nil));
                self.tasks.push(PlugTask::Value(scrut));
            },
            | Comp::Split {
                scrut,
                fst_name,
                snd_name,
                motive,
                body,
            } => {
                let motive_binder = motive.as_ref().map(|split| split.binder.clone());
                self.tasks.push(PlugTask::Finish(PlugFinish::CompSplit {
                    fst_name: fst_name.clone(),
                    snd_name: snd_name.clone(),
                    motive: motive_binder,
                }));
                if let Some(split) = motive {
                    self.tasks.push(PlugTask::CompType(&split.body));
                }
                self.tasks.push(PlugTask::Comp(body));
                self.tasks.push(PlugTask::Value(scrut));
            },
            | Comp::RecordProj { record, label } => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::CompRecordProj {
                        label: label.clone(),
                    }));
                self.tasks.push(PlugTask::Value(record));
            },
            | Comp::With(first, second) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompWith));
                self.tasks.push(PlugTask::Comp(second));
                self.tasks.push(PlugTask::Comp(first));
            },
            | Comp::Prj(side, computation) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::CompPrj(*side)));
                self.tasks.push(PlugTask::Comp(computation));
            },
            | Comp::Dup(value) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompDup));
                self.tasks.push(PlugTask::Value(value));
            },
            | Comp::Drop(value) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompDrop));
                self.tasks.push(PlugTask::Value(value));
            },
            | Comp::Perform(sig, name, payload) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompPerform {
                    name: name.clone(),
                }));
                self.tasks.push(PlugTask::Value(payload));
                self.tasks.push(PlugTask::Sig(sig));
            },
            | Comp::Handle {
                sig,
                scrutinee,
                ret,
                ops,
            } => {
                let metas: Vec<OpClauseMeta> = ops
                    .iter()
                    .map(|clause| OpClauseMeta {
                        op: clause.op.clone(),
                        payload: clause.payload.clone(),
                        resume: clause.resume.clone(),
                    })
                    .collect();
                self.tasks.push(PlugTask::Finish(PlugFinish::CompHandle {
                    ret_name: ret.0.clone(),
                    metas,
                }));
                for clause in ops.iter().rev() {
                    self.tasks.push(PlugTask::Comp(&clause.body));
                }
                self.tasks.push(PlugTask::Comp(&ret.1));
                self.tasks.push(PlugTask::Comp(scrutinee));
                self.tasks.push(PlugTask::Sig(sig));
            },
            | Comp::Resume(value, computation) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompResume));
                self.tasks.push(PlugTask::Comp(computation));
                self.tasks.push(PlugTask::Value(value));
            },
            | Comp::Reset(computation) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompReset));
                self.tasks.push(PlugTask::Comp(computation));
            },
            | Comp::Shift(name, computation) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompShift {
                    name: name.clone(),
                }));
                self.tasks.push(PlugTask::Comp(computation));
            },
            | Comp::Fix(name, computation) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::CompFix { name: name.clone() }));
                self.tasks.push(PlugTask::Comp(computation));
            },
            | Comp::Hole(id) => self.comps.push(Comp::Hole(*id)),
            | Comp::Native { prim, args } => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompNative {
                    prim: *prim,
                    argc: ChildCount::from(args.len()),
                }));
                for argument in args.iter().rev() {
                    self.tasks.push(PlugTask::Value(argument));
                }
            },
            | Comp::Unpack {
                scrut,
                signature,
                atoms,
                binder,
                body,
            } => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompUnpack {
                    atoms: atoms.clone(),
                    binder: binder.clone(),
                }));
                self.tasks.push(PlugTask::Comp(body));
                self.tasks.push(PlugTask::ValueType(signature));
                self.tasks.push(PlugTask::Value(scrut));
            },
            | Comp::Walk {
                scrut,
                motive,
                base,
            } => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompWalk {
                    motive_x: motive.x.clone(),
                    motive_y: motive.y.clone(),
                    motive_q: motive.q.clone(),
                    base_x: base.x.clone(),
                }));
                self.tasks.push(PlugTask::Comp(&base.body));
                self.tasks.push(PlugTask::CompType(&motive.body));
                self.tasks.push(PlugTask::Value(scrut));
            },
        }
        Ok(())
    }

    /// Descend one reified stack.
    ///
    /// # Errors
    /// Never directly.
    #[expect(
        clippy::pattern_type_mismatch,
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_stack(
        &mut self,
        stack: &'template Stack,
    ) -> Result<(), EraseError>
    {
        match stack {
            | Stack::Empty => self.stacks.push(Stack::Empty),
            | Stack::Arg(value, rest) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::StackArg));
                self.tasks.push(PlugTask::Stack(rest));
                self.tasks.push(PlugTask::Value(value));
            },
            | Stack::Bind(name, continuation, rest) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::StackBind {
                    name: name.clone(),
                }));
                self.tasks.push(PlugTask::Stack(rest));
                self.tasks.push(PlugTask::Comp(continuation));
            },
            | Stack::Prj(side, rest) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::StackPrj(*side)));
                self.tasks.push(PlugTask::Stack(rest));
            },
        }
        Ok(())
    }

    /// Descend one value type.
    ///
    /// # Errors
    /// Never directly.
    #[expect(
        clippy::pattern_type_mismatch,
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_value_type(
        &mut self,
        value_type: &'template ValueType,
    ) -> Result<(), EraseError>
    {
        match value_type {
            | ValueType::Atom(name) => self.value_types.push(ValueType::Atom(name.clone())),
            | ValueType::Unit => self.value_types.push(ValueType::Unit),
            | ValueType::Universe { .. } => self.value_types.push(value_type.clone()),
            | ValueType::Unknown => self.value_types.push(ValueType::Unknown),
            | ValueType::Sealed(id) => self.value_types.push(ValueType::Sealed(id.clone())),
            | ValueType::Prod(first, second) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueTypeProd));
                self.tasks.push(PlugTask::ValueType(second));
                self.tasks.push(PlugTask::ValueType(first));
            },
            | ValueType::Sum(first, second) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueTypeSum));
                self.tasks.push(PlugTask::ValueType(second));
                self.tasks.push(PlugTask::ValueType(first));
            },
            | ValueType::List(element) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueTypeList));
                self.tasks.push(PlugTask::ValueType(element));
            },
            | ValueType::Record(fields) => {
                let labels: Vec<String> = fields.keys().cloned().collect();
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValueTypeRecord { labels }));
                for field in fields.values().rev() {
                    self.tasks.push(PlugTask::ValueType(field));
                }
            },
            | ValueType::Thunk(grade, body) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValueTypeThunk(*grade)));
                self.tasks.push(PlugTask::CompType(body));
            },
            | ValueType::Stk(consumes, delivers) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueTypeStk));
                self.tasks.push(PlugTask::CompType(delivers));
                self.tasks.push(PlugTask::CompType(consumes));
            },
            | ValueType::Path { ty, lhs, rhs } => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueTypePath));
                self.tasks.push(PlugTask::Value(rhs));
                self.tasks.push(PlugTask::Value(lhs));
                self.tasks.push(PlugTask::ValueType(ty));
            },
            | ValueType::Package {
                grade,
                abstracts,
                payload,
            } => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValueTypePackage {
                        grade: *grade,
                        abstracts: abstracts.clone(),
                    }));
                self.tasks.push(PlugTask::ValueType(payload));
            },
            | ValueType::Family(application) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValueTypeFamily {
                        result: application.result().clone(),
                    }));
                self.tasks
                    .push(PlugTask::StaticNeutral(application.neutral()));
            },
            | ValueType::Data { id, args } => {
                self.tasks.push(PlugTask::Finish(PlugFinish::ValueTypeData {
                    id: id.clone(),
                    argc: ChildCount::from(args.len()),
                }));
                for argument in args.iter().rev() {
                    self.tasks.push(PlugTask::ValueType(argument));
                }
            },
            | ValueType::Sigma { fst, binder, snd } => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::ValueTypeSigma {
                        binder: binder.clone(),
                    }));
                self.tasks.push(PlugTask::ValueType(snd));
                self.tasks.push(PlugTask::ValueType(fst));
            },
        }
        Ok(())
    }

    /// Descend one computation type.
    ///
    /// # Errors
    /// Never directly.
    #[expect(
        clippy::pattern_type_mismatch,
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_comp_type(
        &mut self,
        comp_type: &'template CompType,
    ) -> Result<(), EraseError>
    {
        match comp_type {
            | CompType::F(payload, row) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompTypeF));
                self.tasks.push(PlugTask::Row(row));
                self.tasks.push(PlugTask::ValueType(payload));
            },
            | CompType::Arrow {
                binder,
                arg: argument,
                res: result,
            } => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::CompTypeArrow(binder.clone())));
                self.tasks.push(PlugTask::CompType(result));
                self.tasks.push(PlugTask::ValueType(argument));
            },
            | CompType::With(first, second) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::CompTypeWith));
                self.tasks.push(PlugTask::CompType(second));
                self.tasks.push(PlugTask::CompType(first));
            },
            | CompType::Family(application) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::CompTypeFamily {
                        result: application.result().clone(),
                    }));
                self.tasks
                    .push(PlugTask::StaticNeutral(application.neutral()));
            },
            | CompType::Unknown => self.comp_types.push(CompType::Unknown),
        }
        Ok(())
    }

    /// Descend one quoted ground type.
    #[expect(
        clippy::pattern_type_mismatch,
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_ty(
        &mut self,
        ty: &'template Ty,
    ) -> Result<(), EraseError>
    {
        match ty {
            | Ty::Value(value_type) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::TyValue));
                self.tasks.push(PlugTask::ValueType(value_type));
            },
            | Ty::Comp(comp_type) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::TyComp));
                self.tasks.push(PlugTask::CompType(comp_type));
            },
        }
        Ok(())
    }

    /// Descend one static argument.
    #[expect(
        clippy::pattern_type_mismatch,
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_static_arg(
        &mut self,
        argument: &'template StaticArg,
    ) -> Result<(), EraseError>
    {
        match argument {
            | StaticArg::Level(_) | StaticArg::Sort(_) => {
                self.static_args.push(argument.clone());
            },
            | StaticArg::Type(term) => {
                self.tasks.push(PlugTask::Finish(PlugFinish::StaticArgType));
                self.tasks.push(PlugTask::StaticTerm(term));
            },
            | StaticArg::Value(value) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::StaticArgValue));
                self.tasks.push(PlugTask::Value(value));
            },
        }
        Ok(())
    }

    /// Descend one static neutral.
    #[expect(
        clippy::pattern_type_mismatch,
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_static_neutral(
        &mut self,
        neutral: &'template StaticNeutral,
    ) -> Result<(), EraseError>
    {
        match neutral {
            | StaticNeutral::Head(_) => self.static_neutrals.push(neutral.clone()),
            | StaticNeutral::App { head, argument } => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::StaticNeutralApp));
                self.tasks.push(PlugTask::StaticArg(argument));
                self.tasks.push(PlugTask::StaticNeutral(head));
            },
        }
        Ok(())
    }

    /// Descend one static term.
    #[expect(
        clippy::pattern_type_mismatch,
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_static_term(
        &mut self,
        term: &'template StaticTerm,
    ) -> Result<(), EraseError>
    {
        match term {
            | StaticTerm::Var(_) | StaticTerm::Universe(_) => {
                self.static_terms.push(term.clone());
            },
            | StaticTerm::Quote(ty) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::StaticTermQuote));
                self.tasks.push(PlugTask::Ty(ty));
            },
            | StaticTerm::Pi { binder, codomain } => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::StaticTermPi(binder.clone())));
                self.tasks.push(PlugTask::StaticTerm(codomain));
            },
            | StaticTerm::Lam { binder, body } => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::StaticTermLam(binder.clone())));
                self.tasks.push(PlugTask::StaticTerm(body));
            },
            | StaticTerm::App { function, argument } => {
                self.tasks.push(PlugTask::Finish(PlugFinish::StaticTermApp));
                self.tasks.push(PlugTask::StaticArg(argument));
                self.tasks.push(PlugTask::StaticTerm(function));
            },
            | StaticTerm::Neutral(neutral) => {
                self.tasks
                    .push(PlugTask::Finish(PlugFinish::StaticTermNeutral));
                self.tasks.push(PlugTask::StaticNeutral(neutral));
            },
        }
        Ok(())
    }

    /// Descend one effect signature.
    ///
    /// # Errors
    /// Never directly.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_sig(
        &mut self,
        sig: &'template EffectSig,
    ) -> Result<(), EraseError>
    {
        self.tasks.push(PlugTask::Finish(PlugFinish::SigFinish {
            name: sig.name().as_ref().to_owned(),
            count: ChildCount::from(sig.ops().len()),
        }));
        for op in sig.ops().iter().rev() {
            self.tasks.push(PlugTask::Op(op));
        }
        Ok(())
    }

    /// Descend one signature operation.
    ///
    /// # Errors
    /// Never directly.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_op(
        &mut self,
        op: &'template EffectOp,
    ) -> Result<(), EraseError>
    {
        self.tasks.push(PlugTask::Finish(PlugFinish::OpFinish {
            name: op.name().as_ref().to_owned(),
        }));
        self.tasks.push(PlugTask::ValueType(op.reply()));
        self.tasks.push(PlugTask::ValueType(op.payload()));
        Ok(())
    }

    /// Descend one effect row.
    ///
    /// # Errors
    /// Never directly.
    #[expect(
        clippy::unnecessary_wraps,
        reason = "Visitor results share one fallible worklist protocol; descendant failures propagate through the uniform Result seam."
    )]
    fn visit_row(
        &mut self,
        row: &'template EffectRow,
    ) -> Result<(), EraseError>
    {
        let sigs: Vec<&EffectSig> = row.signatures().collect();
        self.tasks.push(PlugTask::Finish(PlugFinish::RowFinish {
            count: ChildCount::from(sigs.len()),
        }));
        for sig in sigs.into_iter().rev() {
            self.tasks.push(PlugTask::Sig(sig));
        }
        Ok(())
    }

    /// Rebuild one node from the result stacks.
    ///
    /// # Errors
    /// [`EraseError::TraversalInvariant`] on a stack underflow.
    #[expect(
        clippy::too_many_lines,
        reason = "one match arm per legacy-syntax former; splitting the rebuild table across functions would scatter the one place the traversal's post-order discipline is auditable"
    )]
    fn finish(
        &mut self,
        finish: PlugFinish,
    ) -> Result<(), EraseError>
    {
        match finish {
            | PlugFinish::ValuePair => {
                let second = self.pop_value()?;
                let first = self.pop_value()?;
                self.values.push(Value::pair(first, second));
            },
            | PlugFinish::ValueInj(side) => {
                let payload = self.pop_value()?;
                self.values.push(Value::Inj(side, Rc::new(payload)));
            },
            | PlugFinish::ValueList(count) => {
                let elements = self.pop_values(count)?;
                self.values.push(Value::list(elements));
            },
            | PlugFinish::ValueRecord(labels) => {
                let count = ChildCount::from(labels.len());
                let fields = self.pop_values(count)?;
                self.values
                    .push(Value::record(labels.into_iter().zip(fields)));
            },
            | PlugFinish::ValueThunk(grade) => {
                let body = self.pop_comp()?;
                self.values.push(Value::Thunk(grade, Rc::new(body)));
            },
            | PlugFinish::ValueRun => {
                let body = self.pop_comp()?;
                self.values.push(Value::Run(Rc::new(body)));
            },
            | PlugFinish::ValueAnnot => {
                let ty = self.pop_value_type()?;
                let inner = self.pop_value()?;
                self.values.push(Value::Annot(Rc::new(inner), Rc::new(ty)));
            },
            | PlugFinish::ValueStk => {
                let stack = self.pop_stack()?;
                self.values.push(Value::Stk(Rc::new(stack)));
            },
            | PlugFinish::ValueHere => {
                let witness = self.pop_value()?;
                self.values.push(Value::Here(Rc::new(witness)));
            },
            | PlugFinish::ValueCtor { id, tag } => {
                let payload = self.pop_value()?;
                self.values.push(Value::Ctor {
                    id,
                    tag,
                    payload: Rc::new(payload),
                });
            },
            | PlugFinish::ValuePack(count) => {
                let payload = self.pop_value()?;
                let witnesses = self.pop_value_types(count)?;
                self.values.push(Value::Pack {
                    witnesses: witnesses.into_iter().map(Rc::new).collect(),
                    payload: Rc::new(payload),
                });
            },
            | PlugFinish::CompAbs { name, annotated } => {
                let body = self.pop_comp()?;
                let annot = if annotated {
                    let ty = self.pop_value_type()?;
                    Some(Rc::new(ty))
                }
                else {
                    None
                };
                self.comps.push(Comp::Abs(name, annot, Rc::new(body)));
            },
            | PlugFinish::CompApp => {
                let argument = self.pop_value()?;
                let head = self.pop_comp()?;
                self.comps.push(Comp::App(Rc::new(head), Rc::new(argument)));
            },
            | PlugFinish::CompRet => {
                let value = self.pop_value()?;
                self.comps.push(Comp::Ret(Rc::new(value)));
            },
            | PlugFinish::CompBind { name } => {
                let continuation = self.pop_comp()?;
                let bound = self.pop_comp()?;
                self.comps
                    .push(Comp::Bind(Rc::new(bound), name, Rc::new(continuation)));
            },
            | PlugFinish::CompForce => {
                let value = self.pop_value()?;
                self.comps.push(Comp::Force(Rc::new(value)));
            },
            | PlugFinish::CompCase { fst_name, snd_name } => {
                let snd_body = self.pop_comp()?;
                let fst_body = self.pop_comp()?;
                let scrutinee = self.pop_value()?;
                self.comps.push(Comp::Case(
                    Rc::new(scrutinee),
                    (fst_name, Rc::new(fst_body)),
                    (snd_name, Rc::new(snd_body)),
                ));
            },
            | PlugFinish::CompDataCase { binders } => {
                let count = ChildCount::from(binders.len());
                let bodies = self.pop_comps(count)?;
                let scrutinee = self.pop_value()?;
                let arms: Vec<(String, Rc<Comp>)> = binders
                    .into_iter()
                    .zip(bodies)
                    .map(|(binder, body)| (binder, Rc::new(body)))
                    .collect();
                self.comps.push(Comp::DataCase(Rc::new(scrutinee), arms));
            },
            | PlugFinish::CompListCase { head, tail } => {
                let cons = self.pop_comp()?;
                let nil = self.pop_comp()?;
                let scrut = self.pop_value()?;
                self.comps.push(Comp::ListCase {
                    scrut: Rc::new(scrut),
                    nil: Rc::new(nil),
                    head,
                    tail,
                    cons: Rc::new(cons),
                });
            },
            | PlugFinish::CompSplit {
                fst_name,
                snd_name,
                motive,
            } => {
                let body = self.pop_comp()?;
                let motive = match motive {
                    | Some(binder) => {
                        let motive_body = self.pop_comp_type()?;
                        Some(Box::new(SplitMotive {
                            binder,
                            body: Rc::new(motive_body),
                        }))
                    },
                    | None => None,
                };
                let scrut = self.pop_value()?;
                self.comps.push(Comp::Split {
                    scrut: Rc::new(scrut),
                    fst_name,
                    snd_name,
                    motive,
                    body: Rc::new(body),
                });
            },
            | PlugFinish::CompRecordProj { label } => {
                let record = self.pop_value()?;
                self.comps.push(Comp::RecordProj {
                    record: Rc::new(record),
                    label,
                });
            },
            | PlugFinish::CompWith => {
                let second = self.pop_comp()?;
                let first = self.pop_comp()?;
                self.comps.push(Comp::With(Rc::new(first), Rc::new(second)));
            },
            | PlugFinish::CompPrj(side) => {
                let computation = self.pop_comp()?;
                self.comps.push(Comp::Prj(side, Rc::new(computation)));
            },
            | PlugFinish::CompDup => {
                let value = self.pop_value()?;
                self.comps.push(Comp::Dup(Rc::new(value)));
            },
            | PlugFinish::CompDrop => {
                let value = self.pop_value()?;
                self.comps.push(Comp::Drop(Rc::new(value)));
            },
            | PlugFinish::CompPerform { name } => {
                let payload = self.pop_value()?;
                let sig = self.pop_sig()?;
                self.comps
                    .push(Comp::Perform(Box::new(sig), name, Rc::new(payload)));
            },
            | PlugFinish::CompHandle { ret_name, metas } => {
                let count = ChildCount::from(metas.len());
                let bodies = self.pop_comps(count)?;
                let ret_body = self.pop_comp()?;
                let scrutinee = self.pop_comp()?;
                let sig = self.pop_sig()?;
                let ops: Vec<OpClause> = metas
                    .into_iter()
                    .zip(bodies)
                    .map(|(meta, body)| OpClause {
                        op: meta.op,
                        payload: meta.payload,
                        resume: meta.resume,
                        body: Rc::new(body),
                    })
                    .collect();
                self.comps.push(Comp::Handle {
                    sig: Box::new(sig),
                    scrutinee: Rc::new(scrutinee),
                    ret: (ret_name, Rc::new(ret_body)),
                    ops,
                });
            },
            | PlugFinish::CompResume => {
                let computation = self.pop_comp()?;
                let value = self.pop_value()?;
                self.comps
                    .push(Comp::Resume(Rc::new(value), Rc::new(computation)));
            },
            | PlugFinish::CompReset => {
                let computation = self.pop_comp()?;
                self.comps.push(Comp::Reset(Rc::new(computation)));
            },
            | PlugFinish::CompShift { name } => {
                let computation = self.pop_comp()?;
                self.comps.push(Comp::Shift(name, Rc::new(computation)));
            },
            | PlugFinish::CompFix { name } => {
                let computation = self.pop_comp()?;
                self.comps.push(Comp::Fix(name, Rc::new(computation)));
            },
            | PlugFinish::CompNative { prim, argc } => {
                let args = self.pop_values(argc)?;
                self.comps.push(Comp::Native {
                    prim,
                    args: args.into_iter().map(Rc::new).collect(),
                });
            },
            | PlugFinish::CompWalk {
                motive_x,
                motive_y,
                motive_q,
                base_x,
            } => {
                let base_body = self.pop_comp()?;
                let motive_body = self.pop_comp_type()?;
                let scrut = self.pop_value()?;
                self.comps.push(Comp::Walk {
                    scrut: Rc::new(scrut),
                    motive: Box::new(WalkMotive {
                        x: motive_x,
                        y: motive_y,
                        q: motive_q,
                        body: Rc::new(motive_body),
                    }),
                    base: WalkBase {
                        x: base_x,
                        body: Rc::new(base_body),
                    },
                });
            },
            | PlugFinish::CompUnpack { atoms, binder } => {
                let body = self.pop_comp()?;
                let signature = self.pop_value_type()?;
                let scrut = self.pop_value()?;
                self.comps.push(Comp::Unpack {
                    scrut: Rc::new(scrut),
                    signature: Rc::new(signature),
                    atoms,
                    binder,
                    body: Rc::new(body),
                });
            },
            | PlugFinish::StackArg => {
                let rest = self.pop_stack()?;
                let value = self.pop_value()?;
                self.stacks.push(Stack::Arg(Rc::new(value), Rc::new(rest)));
            },
            | PlugFinish::StackBind { name } => {
                let rest = self.pop_stack()?;
                let continuation = self.pop_comp()?;
                self.stacks
                    .push(Stack::Bind(name, Rc::new(continuation), Rc::new(rest)));
            },
            | PlugFinish::StackPrj(side) => {
                let rest = self.pop_stack()?;
                self.stacks.push(Stack::Prj(side, Rc::new(rest)));
            },
            | PlugFinish::StaticArgType => {
                let term = self.pop_static_term()?;
                self.static_args.push(StaticArg::Type(Rc::new(term)));
            },
            | PlugFinish::StaticArgValue => {
                let value = self.pop_value()?;
                self.static_args.push(StaticArg::Value(Rc::new(value)));
            },
            | PlugFinish::StaticNeutralApp => {
                let argument = self.pop_static_arg()?;
                let head = self.pop_static_neutral()?;
                self.static_neutrals
                    .push(StaticNeutral::app(head, argument));
            },
            | PlugFinish::StaticTermQuote => {
                let ty = self.pop_ty()?;
                self.static_terms.push(StaticTerm::Quote(Rc::new(ty)));
            },
            | PlugFinish::StaticTermPi(binder) => {
                let codomain = self.pop_static_term()?;
                self.static_terms.push(StaticTerm::Pi {
                    binder,
                    codomain: Rc::new(codomain),
                });
            },
            | PlugFinish::StaticTermLam(binder) => {
                let body = self.pop_static_term()?;
                self.static_terms.push(StaticTerm::Lam {
                    binder,
                    body: Rc::new(body),
                });
            },
            | PlugFinish::StaticTermApp => {
                let argument = self.pop_static_arg()?;
                let function = self.pop_static_term()?;
                self.static_terms.push(StaticTerm::App {
                    function: Rc::new(function),
                    argument,
                });
            },
            | PlugFinish::StaticTermNeutral => {
                let neutral = self.pop_static_neutral()?;
                self.static_terms.push(StaticTerm::Neutral(neutral));
            },
            | PlugFinish::TyValue => {
                let value_type = self.pop_value_type()?;
                self.tys.push(Ty::Value(value_type));
            },
            | PlugFinish::TyComp => {
                let comp_type = self.pop_comp_type()?;
                self.tys.push(Ty::Comp(comp_type));
            },
            | PlugFinish::ValueTypeFamily { result } => {
                let neutral = self.pop_static_neutral()?;
                self.value_types
                    .push(ValueType::Family(FamilyApp::new(neutral, result)));
            },
            | PlugFinish::ValueTypeProd => {
                let second = self.pop_value_type()?;
                let first = self.pop_value_type()?;
                self.value_types
                    .push(ValueType::Prod(Rc::new(first), Rc::new(second)));
            },
            | PlugFinish::ValueTypeSum => {
                let second = self.pop_value_type()?;
                let first = self.pop_value_type()?;
                self.value_types
                    .push(ValueType::Sum(Rc::new(first), Rc::new(second)));
            },
            | PlugFinish::ValueTypeList => {
                let element = self.pop_value_type()?;
                self.value_types.push(ValueType::List(Rc::new(element)));
            },
            | PlugFinish::ValueTypeRecord { labels } => {
                let count = ChildCount::from(labels.len());
                let fields = self.pop_value_types(count)?;
                let record: alloc::collections::BTreeMap<String, Rc<ValueType>> = labels
                    .into_iter()
                    .zip(fields)
                    .map(|(label, field)| (label, Rc::new(field)))
                    .collect();
                self.value_types.push(ValueType::Record(record));
            },
            | PlugFinish::ValueTypeThunk(grade) => {
                let body = self.pop_comp_type()?;
                self.value_types
                    .push(ValueType::Thunk(grade, Rc::new(body)));
            },
            | PlugFinish::ValueTypeStk => {
                let delivers = self.pop_comp_type()?;
                let consumes = self.pop_comp_type()?;
                self.value_types
                    .push(ValueType::Stk(Rc::new(consumes), Rc::new(delivers)));
            },
            | PlugFinish::ValueTypePath => {
                let rhs = self.pop_value()?;
                let lhs = self.pop_value()?;
                let ty = self.pop_value_type()?;
                self.value_types.push(ValueType::Path {
                    ty: Rc::new(ty),
                    lhs: Rc::new(lhs),
                    rhs: Rc::new(rhs),
                });
            },
            | PlugFinish::ValueTypeData { id, argc } => {
                let args = self.pop_value_types(argc)?;
                self.value_types.push(ValueType::Data {
                    id,
                    args: args.into_iter().map(Rc::new).collect(),
                });
            },
            | PlugFinish::ValueTypeSigma { binder } => {
                let snd = self.pop_value_type()?;
                let fst = self.pop_value_type()?;
                self.value_types.push(ValueType::Sigma {
                    fst: Rc::new(fst),
                    binder,
                    snd: Rc::new(snd),
                });
            },
            | PlugFinish::ValueTypePackage { grade, abstracts } => {
                let payload = self.pop_value_type()?;
                self.value_types.push(ValueType::Package {
                    grade,
                    abstracts,
                    payload: Rc::new(payload),
                });
            },
            | PlugFinish::CompTypeF => {
                let row = self.pop_row()?;
                let payload = self.pop_value_type()?;
                self.comp_types.push(CompType::F(Rc::new(payload), row));
            },
            | PlugFinish::CompTypeArrow(binder) => {
                let result = self.pop_comp_type()?;
                let argument = self.pop_value_type()?;
                self.comp_types.push(CompType::Arrow {
                    binder,
                    arg: Rc::new(argument),
                    res: Rc::new(result),
                });
            },
            | PlugFinish::CompTypeWith => {
                let second = self.pop_comp_type()?;
                let first = self.pop_comp_type()?;
                self.comp_types
                    .push(CompType::With(Rc::new(first), Rc::new(second)));
            },
            | PlugFinish::CompTypeFamily { result } => {
                let neutral = self.pop_static_neutral()?;
                self.comp_types
                    .push(CompType::Family(FamilyApp::new(neutral, result)));
            },
            | PlugFinish::OpFinish { name } => {
                let reply = self.pop_value_type()?;
                let payload = self.pop_value_type()?;
                self.ops.push(EffectOp::new(
                    OperationName::from(name.as_str()),
                    payload,
                    reply,
                ));
            },
            | PlugFinish::SigFinish { name, count } => {
                let ops = self.pop_ops(count)?;
                self.sigs.push(EffectSig::new(
                    EffectSignatureName::from(name.as_str()),
                    ops,
                ));
            },
            | PlugFinish::RowFinish { count } => {
                let sigs = self.pop_sigs(count)?;
                let mut row = EffectRow::EMPTY;
                for sig in sigs {
                    row = row.union(&EffectRow::singleton(sig));
                }
                self.rows.push(row);
            },
        }
        Ok(())
    }
}

/// A name claiming the seam namespace without being a canonical seam
/// spelling, or colliding with it from a binder.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SeamText(String);

impl From<String> for SeamText
{
    #[inline]
    fn from(value: String) -> Self
    {
        Self(value)
    }
}

impl AsRef<str> for SeamText
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0.as_str()
    }
}

/// A template-side seam violation, as found by [`validate`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SeamProblem
{
    /// A name with the seam prefix that is not a canonical seam spelling.
    Malformed(SeamText),
    /// A binder named into the seam namespace (a seam must name a plug
    /// position, never something the template binds).
    BinderCollision(SeamText),
    /// A seam occurring twice (a repeated seam would duplicate a child with
    /// no sharing former naming the duplication).
    Repeated(SeamIndex),
    /// A seam out of canonical first-occurrence order.
    OutOfOrder
    {
        /// The index the enumeration expected here.
        expected: SeamIndex,
        /// The index found.
        found: SeamIndex,
    },
    /// The enumeration ended short of the child count.
    Missing
    {
        /// The first index no template position carried.
        expected: SeamIndex,
        /// The child count the enumeration had to cover.
        children: ChildCount,
    },
}

/// A well-formedness failure of an overlay, as found by [`validate`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationError
{
    /// A sharing former with no occurrences (weakening waits for the garbage
    /// rules).
    ArityZero,
    /// A [`ShareNode::Bound`] reference escaping its closures.
    DanglingBound(Bound),
    /// An occurrence position out of canonical first-occurrence order.
    NonCanonicalPosition
    {
        /// The position the preorder expected here.
        expected: VectorPosition,
        /// The position found.
        found: VectorPosition,
    },
    /// A closure whose body held fewer occurrences than its arity.
    IncompletePositions
    {
        /// The declared occurrence count.
        arity: Arity,
        /// The occurrences the body actually held.
        found: OccurrenceCount,
    },
    /// A graft child whose result family disagrees with its seam position
    /// (value positions at this rung).
    ChildFamily
    {
        /// The position's family.
        expected: ShareFamily,
        /// The child's result family (what it erases to).
        found: ShareFamily,
    },
    /// A template-side seam violation.
    Seam(SeamProblem),
    /// An id that does not resolve in the arena.
    ArenaFault(AnyShareId),
    /// A child minted after its parent within one family table (unreachable
    /// under constructor-only minting, checked mechanically).
    ChildAfterParent(AnyShareId),
    /// The walk's own stack discipline broke — kept fail-closed.
    TraversalInvariant,
}

impl From<SeamProblem> for ValidationError
{
    #[inline]
    fn from(value: SeamProblem) -> Self
    {
        Self::Seam(value)
    }
}

/// One closure frame of the validation walk: the declared arity, the result
/// family of the shared leg (what a reference to this closure erases to), and
/// the next canonical position the preorder expects.
struct FrameState
{
    /// The closure's declared occurrence count.
    arity: Arity,
    /// The shared leg's result family.
    family: ShareFamily,
    /// The next expected first-occurrence position.
    next: VectorPosition,
}

/// One pending step of the validation walk.
enum ValidateTask
{
    /// Visit one overlay node.
    Enter(AnyShareId),
    /// Open one closure's scope.
    PushFrame
    {
        /// The closure's declared occurrence count.
        arity: Arity,
        /// The shared leg's result family.
        family: ShareFamily,
    },
    /// Close one closure's scope, checking its occurrence count.
    PopFrame,
}

/// The same-family projection of an [`AnyShareId`], for the child-order
/// check.
trait UnwrapId: Sized
{
    /// The id if it addresses this family.
    fn unwrap_id(id: AnyShareId) -> Option<ShareId<Self>>;
}

impl UnwrapId for Value
{
    #[inline]
    fn unwrap_id(id: AnyShareId) -> Option<ShareId<Self>>
    {
        match id {
            | AnyShareId::Value(id) => Some(id),
            | _ => None,
        }
    }
}

impl UnwrapId for Comp
{
    #[inline]
    fn unwrap_id(id: AnyShareId) -> Option<ShareId<Self>>
    {
        match id {
            | AnyShareId::Comp(id) => Some(id),
            | _ => None,
        }
    }
}

impl UnwrapId for ValueType
{
    #[inline]
    fn unwrap_id(id: AnyShareId) -> Option<ShareId<Self>>
    {
        match id {
            | AnyShareId::ValueType(id) => Some(id),
            | _ => None,
        }
    }
}

impl UnwrapId for CompType
{
    #[inline]
    fn unwrap_id(id: AnyShareId) -> Option<ShareId<Self>>
    {
        match id {
            | AnyShareId::CompType(id) => Some(id),
            | _ => None,
        }
    }
}

/// Assert one child precedes its parent within the family table.
///
/// # Contract
/// - ensures: `Ok` iff the child's index is strictly earlier.
/// - fails: [`ValidationError::ChildAfterParent`] otherwise.
/// - panics: none.
fn check_child<L>(
    parent: ShareId<L>,
    child: ShareId<L>,
    tagged: AnyShareId,
) -> Result<(), ValidationError>
{
    if u32::from(child.index()) >= u32::from(parent.index()) {
        return Err(ValidationError::ChildAfterParent(tagged));
    }
    Ok(())
}

/// The child-before-parent check for one node, run once per node.
///
/// # Contract
/// - ensures: every same-family child of `node` precedes `id`; cross-family
///   references order against a different table and are not compared.
/// - fails: [`ValidationError::ChildAfterParent`] on a violation.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Child ordering intentionally matches borrowed arena nodes by variant."
)]
fn check_child_order<L>(
    tagged: AnyShareId,
    id: ShareId<L>,
    node: &ShareNode<L>,
) -> Result<(), ValidationError>
where
    L: UnwrapId,
{
    match node {
        | ShareNode::Share(Sharing { shared, body, .. }) => {
            check_child(id, *body, tagged)?;
            if let Some(shared) = L::unwrap_id(*shared) {
                check_child(id, shared, tagged)?;
            }
        },
        | ShareNode::Graft(Graft { children, .. }) => {
            for child in children {
                if let Some(child) = L::unwrap_id(*child) {
                    check_child(id, child, tagged)?;
                }
            }
        },
        | ShareNode::Opaque(_) | ShareNode::Bound(_) => {},
    }
    Ok(())
}

/// The template-walk root of one family, for the seam collection.
trait TemplateWalk: Sized
{
    /// The walk task rooted at this family's template.
    fn walk_root(template: &Self) -> WalkTask<'_>;
}

impl TemplateWalk for Value
{
    #[inline]
    fn walk_root(template: &Self) -> WalkTask<'_>
    {
        WalkTask::Value(template)
    }
}

impl TemplateWalk for Comp
{
    #[inline]
    fn walk_root(template: &Self) -> WalkTask<'_>
    {
        WalkTask::Comp(template)
    }
}

impl TemplateWalk for ValueType
{
    #[inline]
    fn walk_root(template: &Self) -> WalkTask<'_>
    {
        WalkTask::ValueType(template)
    }
}

impl TemplateWalk for CompType
{
    #[inline]
    fn walk_root(template: &Self) -> WalkTask<'_>
    {
        WalkTask::CompType(template)
    }
}

/// The family environment as a plain slice, for seeding [`result_family`].
///
/// # Contract
/// - ensures: the families of the live frames, outermost first.
/// - panics: none.
fn frame_families(env: &[FrameState]) -> Vec<ShareFamily>
{
    env.iter().map(|frame| frame.family).collect()
}

/// One pending step of the result-family walk.
enum FamilyTask
{
    /// Evaluate one overlay node.
    Eval(AnyShareId),
    /// Move a just-computed leg family into the environment.
    PushFam,
    /// Leave one closure's scope.
    PopFam,
}

/// The family of the term an overlay node erases to.
///
/// A node's own family and its result family differ exactly at
/// [`ShareNode::Bound`]: a reference of any family resolves to the shared leg
/// of the closure it names, and that leg may carry any family. Family
/// agreement at a graft is stated on result families, never on node families.
///
/// # Contract
/// - requires: `env` holds the result families of the closures enclosing `root`
///   (innermost last), exactly as the validation walk's frames do.
/// - ensures: the family of the term `root` erases to: an opaque node's own
///   family; a reference's closure-leg family; a share's body family (with the
///   leg's family in scope); a graft's template family.
/// - fails: [`ValidationError::ArenaFault`] on a dangling id,
///   [`ValidationError::DanglingBound`] on an escaping reference,
///   [`ValidationError::TraversalInvariant`] on a stack-discipline break.
/// - panics: none.
fn result_family(
    arena: &ShareArena,
    root: AnyShareId,
    env: &[ShareFamily],
) -> Result<ShareFamily, ValidationError>
{
    let mut env: Vec<ShareFamily> = env.to_vec();
    let mut results: Vec<ShareFamily> = Vec::new();
    let mut tasks = Vec::new();
    tasks.push(FamilyTask::Eval(root));
    while let Some(task) = tasks.pop() {
        match task {
            | FamilyTask::Eval(id) => match id {
                | AnyShareId::Value(id) => {
                    eval_family_node(arena, id, &mut tasks, &env, &mut results)?;
                },
                | AnyShareId::Comp(id) => {
                    eval_family_node(arena, id, &mut tasks, &env, &mut results)?;
                },
                | AnyShareId::ValueType(id) => {
                    eval_family_node(arena, id, &mut tasks, &env, &mut results)?;
                },
                | AnyShareId::CompType(id) => {
                    eval_family_node(arena, id, &mut tasks, &env, &mut results)?;
                },
            },
            | FamilyTask::PushFam => {
                let Some(family) = results.pop()
                else {
                    return Err(ValidationError::TraversalInvariant);
                };
                env.push(family);
            },
            | FamilyTask::PopFam => {
                if env.pop().is_none() {
                    return Err(ValidationError::TraversalInvariant);
                }
            },
        }
    }
    let Some(result) = results.pop()
    else {
        return Err(ValidationError::TraversalInvariant);
    };
    if !results.is_empty() {
        return Err(ValidationError::TraversalInvariant);
    }
    Ok(result)
}

/// Queue one node's result-family evaluation (the per-family dispatch of
/// [`result_family`]).
///
/// # Contract
/// - requires: every child id of the node resolves in `arena`.
/// - ensures: the node's result family lands on `results` once the queued tasks
///   drain; a share queues its leg, then the environment push, then its body,
///   then the scope pop, so the leg's family is in scope for the body's
///   references.
/// - fails: [`ValidationError::ArenaFault`] on a dangling id,
///   [`ValidationError::DanglingBound`] on an escaping reference.
/// - panics: none.
#[expect(
    clippy::match_same_arms,
    reason = "Family evaluation returns one family marker for every non-composite node."
)]
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Family validation intentionally matches borrowed arena nodes by variant."
)]
fn eval_family_node<L>(
    arena: &ShareArena,
    id: ShareId<L>,
    tasks: &mut Vec<FamilyTask>,
    env: &[ShareFamily],
    results: &mut Vec<ShareFamily>,
) -> Result<(), ValidationError>
where
    L: Family,
{
    let Some(node) = lookup(L::table(arena), id)
    else {
        return Err(ValidationError::ArenaFault(L::tag(id)));
    };
    match node {
        | ShareNode::Opaque(_) => results.push(L::family()),
        | ShareNode::Bound(bound) => {
            let distance = usize::try_from(u32::from(bound.distance)).unwrap_or(usize::MAX);
            if distance >= env.len() {
                return Err(ValidationError::DanglingBound(*bound));
            }
            let index = env.len().saturating_sub(1).saturating_sub(distance);
            let Some(family) = env.get(index)
            else {
                return Err(ValidationError::DanglingBound(*bound));
            };
            results.push(*family);
        },
        | ShareNode::Share(Sharing { shared, body, .. }) => {
            tasks.push(FamilyTask::PopFam);
            tasks.push(FamilyTask::Eval(L::tag(*body)));
            tasks.push(FamilyTask::PushFam);
            tasks.push(FamilyTask::Eval(*shared));
        },
        | ShareNode::Graft(_) => results.push(L::family()),
    }
    Ok(())
}

/// Visit one overlay node for validation.
///
/// # Contract
/// - ensures: the node's own checks run once (child order, template seams), its
///   occurrence references resolve and consume canonical positions, and its
///   children are queued so every *visit* counts its occurrences (a shared node
///   visited twice contributes two textual occurrences).
/// - fails: the [`ValidationError`] variant naming the violated rule.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "Validation intentionally matches borrowed arena nodes by variant."
)]
fn validate_node<L>(
    arena: &ShareArena,
    id: ShareId<L>,
    tasks: &mut Vec<ValidateTask>,
    env: &mut [FrameState],
    visited: &mut BTreeSet<AnyShareId>,
) -> Result<(), ValidationError>
where
    L: Family + UnwrapId + TemplateWalk,
{
    let Some(node) = lookup(L::table(arena), id)
    else {
        return Err(ValidationError::ArenaFault(L::tag(id)));
    };
    let tagged = L::tag(id);
    if visited.insert(tagged) {
        check_child_order(tagged, id, node)?;
        if let ShareNode::Graft(graft) = node {
            check_template(graft)?;
        }
    }
    match node {
        | ShareNode::Opaque(_) => {},
        | ShareNode::Bound(bound) => {
            let depth = env.len();
            let distance = usize::try_from(u32::from(bound.distance)).unwrap_or(usize::MAX);
            if distance >= depth {
                return Err(ValidationError::DanglingBound(*bound));
            }
            let index = depth.saturating_sub(1).saturating_sub(distance);
            let Some(frame) = env.get_mut(index)
            else {
                return Err(ValidationError::TraversalInvariant);
            };
            if frame.next != bound.position {
                return Err(ValidationError::NonCanonicalPosition {
                    expected: frame.next,
                    found: bound.position,
                });
            }
            frame.next = VectorPosition::from(u32::from(frame.next).saturating_add(1));
        },
        | ShareNode::Share(Sharing {
            arity,
            shared,
            body,
        }) => {
            if u32::from(*arity) == 0 {
                return Err(ValidationError::ArityZero);
            }
            let seed = frame_families(env);
            let family = result_family(arena, *shared, &seed)?;
            tasks.push(ValidateTask::PopFrame);
            tasks.push(ValidateTask::Enter(L::tag(*body)));
            tasks.push(ValidateTask::PushFrame {
                arity: *arity,
                family,
            });
            tasks.push(ValidateTask::Enter(*shared));
        },
        | ShareNode::Graft(Graft { children, .. }) => {
            let seed = frame_families(env);
            for child in children {
                let found = result_family(arena, *child, &seed)?;
                if found != ShareFamily::Value {
                    return Err(ValidationError::ChildFamily {
                        expected: ShareFamily::Value,
                        found,
                    });
                }
            }
            for child in children.iter().rev() {
                tasks.push(ValidateTask::Enter(*child));
            }
        },
    }
    Ok(())
}

/// Validate an overlay rooted at `root` against every rule of this rung.
///
/// # Contract
/// - requires: nothing — a dangling id fails closed as
///   [`ValidationError::ArenaFault`].
/// - ensures: `Ok(())` exactly when the overlay is well-formed: children
///   precede parents within each family, nonzero arities, canonical
///   first-occurrence position numbering (preorder over the tree unfolding,
///   occurrences counted per visit), graft families agree with seam positions,
///   and every template's seams are fresh, linear, and canonically enumerated
///   against its child count.
/// - provides: the well-formedness checker the module's contracts appeal to.
/// - fails: the [`ValidationError`] variant naming the first violated rule.
/// - panics: none.
///
/// # Errors
/// [`ValidationError`], per the variants.
///
/// # Adequacy
/// - hypothesis: L3 — each rule is separated by one accepting witness and one
///   rejecting witness at its own boundary: canonical positions accepted in
///   order and rejected permuted, arity zero rejected, arity overrun rejected,
///   dangling distances rejected at and beyond the depth, family mismatches
///   rejected, and each seam problem rejected by name.
/// - witness: `share::tests::a_canonical_overlay_validates`
/// - witness: `share::tests::permuted_positions_are_rejected`
/// - witness: `share::tests::zero_arity_is_rejected`
/// - witness: `share::tests::an_incomplete_body_is_rejected`
/// - witness: `share::tests::a_dangling_bound_is_rejected`
/// - witness: `share::tests::a_comp_child_is_rejected_during_erase`
/// - witness: `share::tests::repeated_seams_are_rejected`
/// - witness: `share::tests::seam_binder_collisions_are_rejected`
/// - witness: `share::tests::malformed_seam_claims_are_rejected`
#[inline]
pub fn validate(
    arena: &ShareArena,
    root: AnyShareId,
) -> Result<(), ValidationError>
{
    let mut tasks: Vec<ValidateTask> = Vec::new();
    tasks.push(ValidateTask::Enter(root));
    let mut env: Vec<FrameState> = Vec::new();
    let mut visited: BTreeSet<AnyShareId> = BTreeSet::new();
    while let Some(task) = tasks.pop() {
        match task {
            | ValidateTask::Enter(AnyShareId::Value(id)) => {
                validate_node::<Value>(arena, id, &mut tasks, &mut env, &mut visited)?;
            },
            | ValidateTask::Enter(AnyShareId::Comp(id)) => {
                validate_node::<Comp>(arena, id, &mut tasks, &mut env, &mut visited)?;
            },
            | ValidateTask::Enter(AnyShareId::ValueType(id)) => {
                validate_node::<ValueType>(arena, id, &mut tasks, &mut env, &mut visited)?;
            },
            | ValidateTask::Enter(AnyShareId::CompType(id)) => {
                validate_node::<CompType>(arena, id, &mut tasks, &mut env, &mut visited)?;
            },
            | ValidateTask::PushFrame { arity, family } => {
                env.push(FrameState {
                    arity,
                    family,
                    next: VectorPosition::from(0),
                });
            },
            | ValidateTask::PopFrame => {
                let Some(frame) = env.pop()
                else {
                    return Err(ValidationError::TraversalInvariant);
                };
                if u32::from(frame.next) != u32::from(frame.arity) {
                    return Err(ValidationError::IncompletePositions {
                        arity: frame.arity,
                        found: OccurrenceCount::from(u32::from(frame.next)),
                    });
                }
            },
        }
    }
    if !env.is_empty() {
        return Err(ValidationError::TraversalInvariant);
    }
    Ok(())
}

/// One pending step of the template seam collection (descend-only).
enum WalkTask<'template>
{
    /// Descend a value.
    Value(&'template Value),
    /// Descend a computation.
    Comp(&'template Comp),
    /// Descend a reified stack.
    Stack(&'template Stack),
    /// Descend a value type.
    ValueType(&'template ValueType),
    /// Descend a computation type.
    CompType(&'template CompType),
    /// Descend a static argument.
    StaticArg(&'template StaticArg),
    /// Descend a static neutral.
    StaticNeutral(&'template StaticNeutral),
    /// Descend a static term.
    StaticTerm(&'template StaticTerm),
    /// Descend a ground type.
    Ty(&'template Ty),
    /// Descend an effect signature.
    Sig(&'template EffectSig),
    /// Descend one signature operation.
    Op(&'template EffectOp),
    /// Descend an effect row.
    Row(&'template EffectRow),
}

/// Check one graft template's seams against its child count.
///
/// # Contract
/// - ensures: `Ok(())` iff the template's seams are exactly `seam.0 …
///   seam.(n-1)` in first-occurrence order, each once, with no binder named
///   into the seam namespace and no malformed seam claim.
/// - fails: the [`SeamProblem`] naming the violation.
/// - panics: none.
fn check_template<L>(graft: &Graft<L>) -> Result<(), ValidationError>
where
    L: TemplateWalk,
{
    let mut seams = Vec::new();
    collect_template_seams(L::walk_root(&graft.template), &mut seams)?;
    check_seam_enumeration(&seams, ChildCount::from(graft.children.len()))
}

/// Reject a binder named into the seam namespace.
///
/// # Contract
/// - ensures: `Ok(())` iff `name` does not carry the seam prefix.
/// - fails: [`SeamProblem::BinderCollision`] otherwise.
/// - panics: none.
#[expect(
    primitive_signature,
    reason = "Validation helpers consume legacy syntax names before semantic seam conversion."
)]
fn check_binder(name: &str) -> Result<(), ValidationError>
{
    if name.starts_with(SEAM_PREFIX) {
        return Err(SeamProblem::BinderCollision(SeamText::from(name.to_owned())).into());
    }
    Ok(())
}

/// Record one variable's seam claim, if it makes one.
///
/// # Contract
/// - ensures: a canonical seam name is appended to `seams`; a name outside the
///   seam namespace is ignored.
/// - fails: [`SeamProblem::Malformed`] on a malformed seam claim.
/// - panics: none.
#[expect(
    primitive_signature,
    reason = "Validation helpers consume legacy syntax names before semantic seam conversion."
)]
fn claim_seam(
    name: &str,
    seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    if !name.starts_with(SEAM_PREFIX) {
        return Ok(());
    }
    match parse_seam(name) {
        | Some(seam) => {
            seams.push(seam);
            Ok(())
        },
        | None => Err(SeamProblem::Malformed(SeamText::from(name.to_owned())).into()),
    }
}

/// Check the collected seams against the canonical enumeration.
///
/// # Contract
/// - ensures: `Ok(())` iff `seams` is exactly `0, 1, …, n-1` in order, where
///   `n` is the child count.
/// - fails: [`SeamProblem::Repeated`], [`SeamProblem::OutOfOrder`], or
///   [`SeamProblem::Missing`] as the sequence dictates.
/// - panics: none.
fn check_seam_enumeration(
    seams: &[SeamIndex],
    children: ChildCount,
) -> Result<(), ValidationError>
{
    let mut expected = 0_u32;
    for seam in seams {
        let found = u32::from(*seam);
        match found.cmp(&expected) {
            | core::cmp::Ordering::Equal => expected = expected.saturating_add(1),
            | core::cmp::Ordering::Less => return Err(SeamProblem::Repeated(*seam).into()),
            | core::cmp::Ordering::Greater => {
                return Err(SeamProblem::OutOfOrder {
                    expected: SeamIndex::from(expected),
                    found: *seam,
                }
                .into());
            },
        }
    }
    let count = u32::try_from(usize::from(children)).unwrap_or(u32::MAX);
    if expected != count {
        return Err(SeamProblem::Missing {
            expected: SeamIndex::from(expected),
            children,
        }
        .into());
    }
    Ok(())
}

/// Drain the template walk, collecting seam claims and binder collisions.
///
/// # Contract
/// - ensures: every `seam.k` variable in the template is appended to `seams` in
///   preorder; every binder is checked against the seam namespace. The walk is
///   an explicit worklist — total on any template depth.
/// - fails: [`SeamProblem::Malformed`] or [`SeamProblem::BinderCollision`].
/// - panics: none.
#[expect(
    clippy::needless_collect,
    reason = "Collecting signatures gives the row a stable owned traversal order."
)]
fn collect_template_seams(
    root: WalkTask<'_>,
    seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    let mut tasks = Vec::new();
    tasks.push(root);
    while let Some(task) = tasks.pop() {
        match task {
            | WalkTask::Value(value) => walk_value(value, &mut tasks, seams)?,
            | WalkTask::Comp(comp) => walk_comp(comp, &mut tasks, seams)?,
            | WalkTask::Stack(stack) => walk_stack(stack, &mut tasks, seams)?,
            | WalkTask::ValueType(value_type) => walk_value_type(value_type, &mut tasks, seams)?,
            | WalkTask::CompType(comp_type) => walk_comp_type(comp_type, &mut tasks, seams)?,
            | WalkTask::StaticArg(argument) => walk_static_arg(argument, &mut tasks, seams)?,
            | WalkTask::StaticNeutral(neutral) => walk_static_neutral(neutral, &mut tasks, seams)?,
            | WalkTask::StaticTerm(term) => walk_static_term(term, &mut tasks, seams)?,
            | WalkTask::Ty(ty) => walk_ty(ty, &mut tasks, seams)?,
            | WalkTask::Sig(sig) => {
                for op in sig.ops().iter().rev() {
                    tasks.push(WalkTask::Op(op));
                }
            },
            | WalkTask::Op(op) => {
                tasks.push(WalkTask::ValueType(op.reply()));
                tasks.push(WalkTask::ValueType(op.payload()));
            },
            | WalkTask::Row(row) => {
                let sigs: Vec<&EffectSig> = row.signatures().collect();
                for sig in sigs.into_iter().rev() {
                    tasks.push(WalkTask::Sig(sig));
                }
            },
        }
    }
    Ok(())
}

/// Descend one template value.
///
/// # Errors
/// [`SeamProblem`] from [`claim_seam`].
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The seam walk intentionally matches borrowed legacy syntax by variant."
)]
#[expect(
    clippy::match_same_arms,
    reason = "The seam walk mirrors legacy syntax with identical child traversal arms."
)]
fn walk_value<'template>(
    value: &'template Value,
    tasks: &mut Vec<WalkTask<'template>>,
    seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    match value {
        | Value::Var(name) => claim_seam(name, seams)?,
        | Value::Unit | Value::Int(_) | Value::Str(_) | Value::Num(_) | Value::Hole(_) => {},
        | Value::Pair(first, second) => {
            tasks.push(WalkTask::Value(second));
            tasks.push(WalkTask::Value(first));
        },
        | Value::Inj(_, payload) => tasks.push(WalkTask::Value(payload)),
        | Value::List(elements) => {
            for element in elements.iter().rev() {
                tasks.push(WalkTask::Value(element));
            }
        },
        | Value::Record(fields) => {
            for field in fields.values().rev() {
                tasks.push(WalkTask::Value(field));
            }
        },
        | Value::Thunk(_, body) | Value::Run(body) => tasks.push(WalkTask::Comp(body)),
        | Value::Annot(inner, ty) => {
            tasks.push(WalkTask::ValueType(ty));
            tasks.push(WalkTask::Value(inner));
        },
        | Value::Stk(stack) => tasks.push(WalkTask::Stack(stack)),
        | Value::Here(witness) => tasks.push(WalkTask::Value(witness)),
        | Value::Ctor { payload, .. } => tasks.push(WalkTask::Value(payload)),
        | Value::Pack { witnesses, payload } => {
            for witness in witnesses.iter().rev() {
                tasks.push(WalkTask::ValueType(witness));
            }
            tasks.push(WalkTask::Value(payload));
        },
    }
    Ok(())
}

/// Descend one template computation, checking its binders.
///
/// # Errors
/// [`SeamProblem`] from [`claim_seam`] or [`check_binder`].
#[expect(
    clippy::too_many_lines,
    reason = "one match arm per legacy-syntax former; the seam walk must see every binder position, and splitting the inventory across functions would scatter the one auditable checklist"
)]
#[expect(
    clippy::match_same_arms,
    clippy::pattern_type_mismatch,
    reason = "The seam walk mirrors each legacy syntax former while preserving one explicit worklist."
)]
fn walk_comp<'template>(
    comp: &'template Comp,
    tasks: &mut Vec<WalkTask<'template>>,
    _seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    match comp {
        | Comp::Abs(name, annot, body) => {
            check_binder(name)?;
            tasks.push(WalkTask::Comp(body));
            if let Some(ty) = annot {
                tasks.push(WalkTask::ValueType(ty));
            }
        },
        | Comp::App(head, argument) => {
            tasks.push(WalkTask::Value(argument));
            tasks.push(WalkTask::Comp(head));
        },
        | Comp::Ret(value) => tasks.push(WalkTask::Value(value)),
        | Comp::Bind(bound, name, continuation) => {
            check_binder(name)?;
            tasks.push(WalkTask::Comp(continuation));
            tasks.push(WalkTask::Comp(bound));
        },
        | Comp::Force(value) => tasks.push(WalkTask::Value(value)),
        | Comp::Case(scrutinee, fst, snd) => {
            check_binder(&fst.0)?;
            check_binder(&snd.0)?;
            tasks.push(WalkTask::Comp(&snd.1));
            tasks.push(WalkTask::Comp(&fst.1));
            tasks.push(WalkTask::Value(scrutinee));
        },
        | Comp::DataCase(scrutinee, arms) => {
            for arm in arms {
                check_binder(&arm.0)?;
            }
            for arm in arms.iter().rev() {
                tasks.push(WalkTask::Comp(&arm.1));
            }
            tasks.push(WalkTask::Value(scrutinee));
        },
        | Comp::ListCase {
            scrut,
            nil,
            head,
            tail,
            cons,
        } => {
            check_binder(head)?;
            check_binder(tail)?;
            tasks.push(WalkTask::Comp(cons));
            tasks.push(WalkTask::Comp(nil));
            tasks.push(WalkTask::Value(scrut));
        },
        | Comp::Split {
            scrut,
            fst_name,
            snd_name,
            motive,
            body,
        } => {
            check_binder(fst_name)?;
            check_binder(snd_name)?;
            if let Some(split) = motive {
                check_binder(&split.binder)?;
            }
            tasks.push(WalkTask::Comp(body));
            if let Some(split) = motive {
                tasks.push(WalkTask::CompType(&split.body));
            }
            tasks.push(WalkTask::Value(scrut));
        },
        | Comp::RecordProj { record, .. } => tasks.push(WalkTask::Value(record)),
        | Comp::With(first, second) => {
            tasks.push(WalkTask::Comp(second));
            tasks.push(WalkTask::Comp(first));
        },
        | Comp::Prj(_, computation) => tasks.push(WalkTask::Comp(computation)),
        | Comp::Dup(value) | Comp::Drop(value) => tasks.push(WalkTask::Value(value)),
        | Comp::Perform(sig, _, payload) => {
            tasks.push(WalkTask::Value(payload));
            tasks.push(WalkTask::Sig(sig));
        },
        | Comp::Handle {
            sig,
            scrutinee,
            ret,
            ops,
        } => {
            check_binder(&ret.0)?;
            for clause in ops {
                check_binder(&clause.payload)?;
                check_binder(&clause.resume)?;
            }
            for clause in ops.iter().rev() {
                tasks.push(WalkTask::Comp(&clause.body));
            }
            tasks.push(WalkTask::Comp(&ret.1));
            tasks.push(WalkTask::Comp(scrutinee));
            tasks.push(WalkTask::Sig(sig));
        },
        | Comp::Resume(value, computation) => {
            tasks.push(WalkTask::Comp(computation));
            tasks.push(WalkTask::Value(value));
        },
        | Comp::Reset(computation) => tasks.push(WalkTask::Comp(computation)),
        | Comp::Shift(name, computation) | Comp::Fix(name, computation) => {
            check_binder(name)?;
            tasks.push(WalkTask::Comp(computation));
        },
        | Comp::Hole(_) => {},
        | Comp::Native { args, .. } => {
            for argument in args.iter().rev() {
                tasks.push(WalkTask::Value(argument));
            }
        },
        | Comp::Walk {
            scrut,
            motive,
            base,
        } => {
            check_binder(&motive.x)?;
            check_binder(&motive.y)?;
            check_binder(&motive.q)?;
            check_binder(&base.x)?;
            tasks.push(WalkTask::Comp(&base.body));
            tasks.push(WalkTask::CompType(&motive.body));
            tasks.push(WalkTask::Value(scrut));
        },
        | Comp::Unpack {
            scrut,
            signature,
            atoms: _,
            binder,
            body,
        } => {
            check_binder(binder)?;
            tasks.push(WalkTask::Comp(body));
            tasks.push(WalkTask::ValueType(signature));
            tasks.push(WalkTask::Value(scrut));
        },
    }
    Ok(())
}

/// Descend one template reified stack, checking its binders.
///
/// # Errors
/// [`SeamProblem`] from [`check_binder`].
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The seam walk intentionally matches borrowed legacy syntax by variant."
)]
fn walk_stack<'template>(
    stack: &'template Stack,
    tasks: &mut Vec<WalkTask<'template>>,
    _seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    match stack {
        | Stack::Empty => {},
        | Stack::Arg(value, rest) => {
            tasks.push(WalkTask::Stack(rest));
            tasks.push(WalkTask::Value(value));
        },
        | Stack::Bind(name, continuation, rest) => {
            check_binder(name)?;
            tasks.push(WalkTask::Stack(rest));
            tasks.push(WalkTask::Comp(continuation));
        },
        | Stack::Prj(_, rest) => tasks.push(WalkTask::Stack(rest)),
    }
    Ok(())
}

/// Descend one template value type, checking its binders.
///
/// # Errors
/// [`SeamProblem`] from [`claim_seam`] or [`check_binder`].
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The template walk uses one fallible Result protocol across all syntax families."
)]
fn walk_value_type<'template>(
    value_type: &'template ValueType,
    tasks: &mut Vec<WalkTask<'template>>,
    _seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    match value_type {
        | ValueType::Atom(_)
        | ValueType::Unit
        | ValueType::Universe { .. }
        | ValueType::Unknown
        | ValueType::Sealed(_) => {},
        | ValueType::Prod(first, second) | ValueType::Sum(first, second) => {
            tasks.push(WalkTask::ValueType(second));
            tasks.push(WalkTask::ValueType(first));
        },
        | ValueType::List(element) => tasks.push(WalkTask::ValueType(element)),
        | ValueType::Record(fields) => {
            for field in fields.values().rev() {
                tasks.push(WalkTask::ValueType(field));
            }
        },
        | ValueType::Thunk(_, body) => tasks.push(WalkTask::CompType(body)),
        | ValueType::Stk(consumes, delivers) => {
            tasks.push(WalkTask::CompType(delivers));
            tasks.push(WalkTask::CompType(consumes));
        },
        | ValueType::Path { ty, lhs, rhs } => {
            tasks.push(WalkTask::Value(rhs));
            tasks.push(WalkTask::Value(lhs));
            tasks.push(WalkTask::ValueType(ty));
        },
        | ValueType::Family(application) => {
            tasks.push(WalkTask::StaticNeutral(application.neutral()));
        },
        | ValueType::Data { args, .. } => {
            for argument in args.iter().rev() {
                tasks.push(WalkTask::ValueType(argument));
            }
        },
        | ValueType::Sigma { fst, binder, snd } => {
            check_binder(binder)?;
            tasks.push(WalkTask::ValueType(snd));
            tasks.push(WalkTask::ValueType(fst));
        },
        | ValueType::Package {
            grade: _,
            abstracts,
            payload,
        } => {
            for abstract_name in abstracts {
                check_binder(abstract_name)?;
            }
            tasks.push(WalkTask::ValueType(payload));
        },
    }
    Ok(())
}

/// Descend one quoted ground type.
#[expect(
    clippy::pattern_type_mismatch,
    clippy::unnecessary_wraps,
    reason = "The template walk uses one fallible Result protocol across all syntax families."
)]
fn walk_ty<'template>(
    ty: &'template Ty,
    tasks: &mut Vec<WalkTask<'template>>,
    _seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    match ty {
        | Ty::Value(value_type) => tasks.push(WalkTask::ValueType(value_type)),
        | Ty::Comp(comp_type) => tasks.push(WalkTask::CompType(comp_type)),
    }
    Ok(())
}

/// Descend one static argument.
#[expect(
    clippy::pattern_type_mismatch,
    clippy::unnecessary_wraps,
    reason = "The template walk uses one fallible Result protocol across all syntax families."
)]
fn walk_static_arg<'template>(
    argument: &'template StaticArg,
    tasks: &mut Vec<WalkTask<'template>>,
    _seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    match argument {
        | StaticArg::Level(_) | StaticArg::Sort(_) => {},
        | StaticArg::Type(term) => tasks.push(WalkTask::StaticTerm(term)),
        | StaticArg::Value(value) => tasks.push(WalkTask::Value(value)),
    }
    Ok(())
}

/// Descend one static neutral.
#[expect(
    clippy::pattern_type_mismatch,
    clippy::unnecessary_wraps,
    reason = "The template walk uses one fallible Result protocol across all syntax families."
)]
fn walk_static_neutral<'template>(
    neutral: &'template StaticNeutral,
    tasks: &mut Vec<WalkTask<'template>>,
    _seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    match neutral {
        | StaticNeutral::Head(_) => {},
        | StaticNeutral::App { head, argument } => {
            tasks.push(WalkTask::StaticArg(argument));
            tasks.push(WalkTask::StaticNeutral(head));
        },
    }
    Ok(())
}

/// Descend one static term.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "The template walk matches borrowed static nodes by reference."
)]
fn walk_static_term<'template>(
    term: &'template StaticTerm,
    tasks: &mut Vec<WalkTask<'template>>,
    _seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    match term {
        | StaticTerm::Var(_) | StaticTerm::Universe(_) => {},
        | StaticTerm::Quote(ty) => tasks.push(WalkTask::Ty(ty)),
        | StaticTerm::Pi { binder, codomain }
        | StaticTerm::Lam {
            binder,
            body: codomain,
        } => {
            check_binder(binder.variable().name().as_ref())?;
            tasks.push(WalkTask::StaticTerm(codomain));
        },
        | StaticTerm::App { function, argument } => {
            tasks.push(WalkTask::StaticArg(argument));
            tasks.push(WalkTask::StaticTerm(function));
        },
        | StaticTerm::Neutral(neutral) => tasks.push(WalkTask::StaticNeutral(neutral)),
    }
    Ok(())
}

/// Descend one template computation type.
///
/// # Errors
/// [`SeamProblem`] from [`claim_seam`].
#[expect(
    clippy::pattern_type_mismatch,
    clippy::unnecessary_wraps,
    reason = "The template walk uses one fallible Result protocol across all syntax families."
)]
fn walk_comp_type<'template>(
    comp_type: &'template CompType,
    tasks: &mut Vec<WalkTask<'template>>,
    _seams: &mut Vec<SeamIndex>,
) -> Result<(), ValidationError>
{
    match comp_type {
        | CompType::F(payload, row) => {
            tasks.push(WalkTask::Row(row));
            tasks.push(WalkTask::ValueType(payload));
        },
        | CompType::Arrow {
            arg: argument,
            res: result,
            ..
        } => {
            tasks.push(WalkTask::CompType(result));
            tasks.push(WalkTask::ValueType(argument));
        },
        | CompType::With(first, second) => {
            tasks.push(WalkTask::CompType(second));
            tasks.push(WalkTask::CompType(first));
        },
        | CompType::Family(application) => {
            tasks.push(WalkTask::StaticNeutral(application.neutral()));
        },
        | CompType::Unknown => {},
    }
    Ok(())
}

/// Unit witnesses for the overlay, its erasure fold, the seam plug, and the
/// validation walk.
#[cfg(test)]
mod tests
{
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;

    use gandr_core_term::classifier::GroundSort;
    use gandr_core_term::classifier::SortExpr;
    use gandr_core_term::static_term::StaticVar;
    use gandr_kernel_strata::Level;

    use super::*;

    /// A [`Bound`] reference from its two coordinates.
    #[expect(
        primitive_signature,
        reason = "Test fixtures construct semantic values from compact primitive witnesses."
    )]
    fn bound(
        distance: u32,
        position: u32,
    ) -> Bound
    {
        Bound {
            distance: BinderDistance::from(distance),
            position: VectorPosition::from(position),
        }
    }

    /// A variable value.
    #[expect(
        primitive_signature,
        reason = "Test fixtures construct semantic values from compact primitive witnesses."
    )]
    fn var(name: &str) -> Value
    {
        Value::var(name)
    }

    /// An integer literal value.
    #[expect(
        primitive_signature,
        reason = "Test fixtures construct semantic values from compact primitive witnesses."
    )]
    fn int(literal: i64) -> Value
    {
        Value::int(literal)
    }

    /// The canonical `k`-th seam name.
    #[expect(
        primitive_signature,
        reason = "Test fixtures construct semantic values from compact primitive witnesses."
    )]
    fn seam(index: u32) -> String
    {
        seam_name(SeamIndex::from(index))
    }

    /// The repeated-occurrences value overlay: `(u, u)` with one shared `u`
    /// (`3`) bound at arity two.
    fn repeated_occurrences_overlay() -> (ShareArena, ShareId<Value>)
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(int(3)).expect("mint value opaque");
        let first = arena.value_bound(bound(0, 0)).expect("mint value bound");
        let second = arena.value_bound(bound(0, 1)).expect("mint value bound");
        let graft = arena
            .value_graft(Graft {
                template: Value::pair(Value::Var(seam(0)), Value::Var(seam(1))),
                children: vec![AnyShareId::from(first), AnyShareId::from(second)],
            })
            .expect("mint value graft");
        let root = arena
            .value_share(Sharing {
                arity: Arity::from(2),
                shared: AnyShareId::from(shared),
                body: graft,
            })
            .expect("mint value share");
        (arena, root)
    }

    #[test]
    fn reserved_tags_map_families_in_order()
    {
        assert_eq!(ShareFamily::Value.reserved_tag(), ReservedTag::from(0x18));
        assert_eq!(ShareFamily::Comp.reserved_tag(), ReservedTag::from(0x19));
        assert_eq!(
            ShareFamily::ValueType.reserved_tag(),
            ReservedTag::from(0x1A)
        );
        assert_eq!(
            ShareFamily::CompType.reserved_tag(),
            ReservedTag::from(0x1B)
        );
        assert_eq!(
            ShareFamily::RESERVED_ORDER,
            [
                ShareFamily::Value,
                ShareFamily::Comp,
                ShareFamily::ValueType,
                ShareFamily::CompType,
            ],
            "the family's order is the reserved block's order"
        );
        assert_eq!(u8::from(HELD_TAGS_FIRST), 0x1C);
        assert_eq!(u8::from(HELD_TAGS_LAST), 0x1F);
    }

    #[test]
    fn truncation_disposes_intermediates()
    {
        let mut arena = ShareArena::new();
        let kept = arena.value_opaque(int(1)).expect("mint value opaque");
        let mark = arena.watermark();
        let dropped = arena.value_opaque(int(2)).expect("mint value opaque");
        arena.truncate_to(mark);
        assert!(
            arena.value_node(kept).is_some(),
            "a node minted before the watermark survives truncation"
        );
        assert!(
            arena.value_node(dropped).is_none(),
            "a node minted past the watermark is disposed of wholesale"
        );
    }

    #[test]
    fn capacity_refusal_leaves_length_unchanged()
    {
        let mut table = vec![ShareNode::Opaque(int(1))];
        assert_eq!(
            mint_with_limit(&mut table, ShareNode::Opaque(int(2)), 1),
            Err(MintError::CapacityExhausted),
            "capacity refusal is fallible before append"
        );
        assert_eq!(table.len(), 1, "failed mint does not change table length");
    }

    #[test]
    fn seam_names_parse_strictly()
    {
        assert_eq!(parse_seam("seam.0"), Some(SeamIndex::from(0)));
        assert_eq!(parse_seam("seam.37"), Some(SeamIndex::from(37)));
        assert_eq!(parse_seam(&seam(12)), Some(SeamIndex::from(12)));
        for rejected in ["seam.", "seam.x", "seam.00", "seam", "other.0", ""] {
            assert_eq!(
                parse_seam(rejected),
                None,
                "`{rejected}` is not a canonical seam"
            );
        }
    }

    #[test]
    fn opaque_value_erases_to_itself()
    {
        let mut arena = ShareArena::new();
        let leaf = arena
            .value_opaque(Value::pair(int(1), int(2)))
            .expect("mint value opaque");
        assert_eq!(
            erase_value(&arena, leaf),
            Ok(Value::pair(int(1), int(2))),
            "an opaque leaf resolves to its payload untouched"
        );
    }

    #[test]
    fn a_bound_resolves_to_the_shared_value()
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(int(7)).expect("mint value opaque");
        let body = arena.value_bound(bound(0, 0)).expect("mint value bound");
        let root = arena
            .value_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(shared),
                body,
            })
            .expect("mint value share");
        assert_eq!(erase_value(&arena, root), Ok(int(7)));
        assert_eq!(validate(&arena, AnyShareId::from(root)), Ok(()));
    }

    #[test]
    fn repeated_occurrences_plug_the_same_erasure()
    {
        let (arena, root) = repeated_occurrences_overlay();
        assert_eq!(
            erase_value(&arena, root),
            Ok(Value::pair(int(3), int(3))),
            "both occurrences plug the one erased shared value"
        );
    }

    #[test]
    fn a_canonical_overlay_validates()
    {
        let (arena, root) = repeated_occurrences_overlay();
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Ok(()),
            "arity two with first-occurrence positions 0 then 1 is canonical"
        );
    }
    #[test]
    fn type_family_plug_preserves_typed_static_arguments()
    {
        let result = Classifier::new(GroundSort::Value, Level::zero());
        let arguments = [
            StaticArg::Level(Level::zero()),
            StaticArg::Sort(SortExpr::value()),
            StaticArg::Type(Rc::new(StaticTerm::Quote(Rc::new(Ty::Value(
                ValueType::Unit,
            ))))),
            StaticArg::Value(Rc::new(Value::Var(seam(0)))),
        ];
        let neutral = arguments.into_iter().fold(
            StaticNeutral::head(StaticVar::new("Family")),
            StaticNeutral::app,
        );
        let template =
            AnyHost::ValueType(ValueType::family(FamilyApp::new(neutral, result.clone())));
        let children = [AnyHost::Value(int(41))];
        let plugged = plug_host(&template, &children).expect("family plug");
        let expected_arguments = [
            StaticArg::Level(Level::zero()),
            StaticArg::Sort(SortExpr::value()),
            StaticArg::Type(Rc::new(StaticTerm::Quote(Rc::new(Ty::Value(
                ValueType::Unit,
            ))))),
            StaticArg::Value(Rc::new(int(41))),
        ];
        let expected_neutral = expected_arguments.into_iter().fold(
            StaticNeutral::head(StaticVar::new("Family")),
            StaticNeutral::app,
        );
        assert_eq!(
            plugged,
            AnyHost::ValueType(ValueType::family(FamilyApp::new(expected_neutral, result,)))
        );
    }

    #[test]
    fn value_grafts_plug_through_every_composite_shape()
    {
        let child = int(41);
        let templates = vec![
            (
                Value::pair(Value::Var(seam(0)), Value::Unit),
                Value::pair(child.clone(), Value::Unit),
            ),
            (Value::inj1(Value::Var(seam(0))), Value::inj1(child.clone())),
            (
                Value::list(vec![Value::Var(seam(0)), Value::Unit]),
                Value::list(vec![child.clone(), Value::Unit]),
            ),
            (
                Value::record([(String::from("field"), Value::Var(seam(0)))]),
                Value::record([(String::from("field"), child.clone())]),
            ),
            (
                Value::thunk(Grade::OMEGA, Comp::ret(Value::Var(seam(0)))),
                Value::thunk(Grade::OMEGA, Comp::ret(child.clone())),
            ),
            (
                Value::annot(Value::Var(seam(0)), ValueType::integer()),
                Value::annot(child.clone(), ValueType::integer()),
            ),
            (
                Value::pack([ValueType::integer()], Value::Var(seam(0))),
                Value::pack([ValueType::integer()], child.clone()),
            ),
            (Value::here(Value::Var(seam(0))), Value::here(child.clone())),
        ];

        for (template, expected) in templates {
            let mut arena = ShareArena::new();
            let child_id = arena
                .value_opaque(child.clone())
                .expect("mint value opaque");
            let root = arena
                .value_graft(Graft {
                    template,
                    children: vec![AnyShareId::from(child_id)],
                })
                .expect("mint value graft");
            assert_eq!(
                erase_value(&arena, root),
                Ok(expected),
                "the seam child survives every composite value reconstruction"
            );
            assert_eq!(
                validate(&arena, AnyShareId::from(root)),
                Ok(()),
                "every reconstructed graft remains a valid overlay"
            );
        }
    }

    #[test]
    fn under_binder_sharing_erases_capture_permitting()
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(var("x")).expect("mint value opaque");
        let reference = arena.comp_bound(bound(0, 0)).expect("mint comp bound");
        let graft = arena
            .comp_graft(Graft {
                template: Comp::lam("x", Comp::ret(Value::Var(seam(0)))),
                children: vec![AnyShareId::from(reference)],
            })
            .expect("mint comp graft");
        let root = arena
            .comp_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(shared),
                body: graft,
            })
            .expect("mint comp share");
        assert_eq!(
            erase_comp(&arena, root),
            Ok(Comp::lam("x", Comp::ret(var("x")))),
            "`λx. ret seam.0` sharing `x` erases to `λx. ret x`: the plug is capture-permitting"
        );
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Ok(()),
            "the under-binder shape is well-formed"
        );
    }

    #[test]
    fn a_shared_comp_erases_to_the_unshared_spelling()
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(int(3)).expect("mint value opaque");
        let first = arena.comp_bound(bound(0, 0)).expect("mint comp bound");
        let second = arena.comp_bound(bound(0, 1)).expect("mint comp bound");
        let graft = arena
            .comp_graft(Graft {
                template: Comp::ret(Value::pair(Value::Var(seam(0)), Value::Var(seam(1)))),
                children: vec![AnyShareId::from(first), AnyShareId::from(second)],
            })
            .expect("mint comp graft");
        let root = arena
            .comp_share(Sharing {
                arity: Arity::from(2),
                shared: AnyShareId::from(shared),
                body: graft,
            })
            .expect("mint comp share");
        assert_eq!(
            erase_comp(&arena, root),
            Ok(Comp::ret(Value::pair(int(3), int(3)))),
            "the share-carrying computation folds to the ordinary unshared spelling"
        );
        assert_eq!(validate(&arena, AnyShareId::from(root)), Ok(()));
    }

    #[test]
    fn value_type_seams_plug_under_path()
    {
        let mut arena = ShareArena::new();
        let left = arena.value_opaque(var("a")).expect("mint value opaque");
        let right = arena.value_opaque(var("b")).expect("mint value opaque");
        let root = arena
            .value_type_graft(Graft {
                template: ValueType::Path {
                    ty: Rc::new(ValueType::Atom(String::from("A"))),
                    lhs: Rc::new(Value::Var(seam(0))),
                    rhs: Rc::new(Value::Var(seam(1))),
                },
                children: vec![AnyShareId::from(left), AnyShareId::from(right)],
            })
            .expect("mint value type graft");
        assert_eq!(
            erase_value_type(&arena, root),
            Ok(ValueType::Path {
                ty: Rc::new(ValueType::Atom(String::from("A"))),
                lhs: Rc::new(var("a")),
                rhs: Rc::new(var("b")),
            }),
            "seams plug at the identity type's value endpoints"
        );
        assert_eq!(validate(&arena, AnyShareId::from(root)), Ok(()));
    }

    #[test]
    fn comp_type_seams_plug_through_the_arrow()
    {
        let mut arena = ShareArena::new();
        let left = arena.value_opaque(var("a")).expect("mint value opaque");
        let right = arena.value_opaque(var("b")).expect("mint value opaque");
        let root = arena
            .comp_type_graft(Graft {
                template: CompType::Arrow {
                    binder: None,
                    arg: Rc::new(ValueType::Path {
                        ty: Rc::new(ValueType::Atom(String::from("A"))),
                        lhs: Rc::new(Value::Var(seam(0))),
                        rhs: Rc::new(Value::Var(seam(1))),
                    }),
                    res: Rc::new(CompType::returner(ValueType::Atom(String::from("B")))),
                },
                children: vec![AnyShareId::from(left), AnyShareId::from(right)],
            })
            .expect("mint comp type graft");
        assert_eq!(
            erase_comp_type(&arena, root),
            Ok(CompType::Arrow {
                binder: None,
                arg: Rc::new(ValueType::Path {
                    ty: Rc::new(ValueType::Atom(String::from("A"))),
                    lhs: Rc::new(var("a")),
                    rhs: Rc::new(var("b")),
                }),
                res: Rc::new(CompType::returner(ValueType::Atom(String::from("B")))),
            }),
            "seams plug through the arrow's value-type argument"
        );
        assert_eq!(validate(&arena, AnyShareId::from(root)), Ok(()));
    }

    #[test]
    fn effect_row_seams_plug_through_signature_ops()
    {
        let mut arena = ShareArena::new();
        let left = arena.value_opaque(var("a")).expect("mint value opaque");
        let right = arena.value_opaque(var("b")).expect("mint value opaque");
        let payload = |lhs: Value, rhs: Value| ValueType::Path {
            ty: Rc::new(ValueType::Atom(String::from("A"))),
            lhs: Rc::new(lhs),
            rhs: Rc::new(rhs),
        };
        let sig_with = |payload: ValueType| {
            EffectSig::new(EffectSignatureName::from("State"), vec![EffectOp::new(
                OperationName::from("get"),
                payload,
                ValueType::Atom(String::from("B")),
            )])
        };
        let root = arena
            .comp_graft(Graft {
                template: Comp::Perform(
                    Box::new(sig_with(payload(Value::Var(seam(0)), Value::Var(seam(1))))),
                    String::from("get"),
                    Rc::new(Value::Unit),
                ),
                children: vec![AnyShareId::from(left), AnyShareId::from(right)],
            })
            .expect("mint comp graft");
        let expected = Comp::Perform(
            Box::new(sig_with(payload(var("a"), var("b")))),
            String::from("get"),
            Rc::new(Value::Unit),
        );
        assert_eq!(
            erase_comp(&arena, root),
            Ok(expected),
            "seams plug through a signature's operation types on a perform"
        );
        assert_eq!(validate(&arena, AnyShareId::from(root)), Ok(()));
    }
    #[test]
    fn value_type_share_erases_checked()
    {
        let mut arena = ShareArena::new();
        let shared = arena
            .value_type_opaque(ValueType::Atom(String::from("A")))
            .expect("mint value type opaque");
        let body = arena
            .value_type_bound(bound(0, 0))
            .expect("mint value type bound");
        let root = arena
            .value_type_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(shared),
                body,
            })
            .expect("mint value type share");
        assert_eq!(
            erase_value_type(&arena, root),
            Ok(ValueType::Atom(String::from("A"))),
            "value-type sharing erases through the checked family face"
        );
        assert_eq!(validate(&arena, AnyShareId::from(root)), Ok(()));
    }

    #[test]
    fn comp_type_share_erases_checked()
    {
        let mut arena = ShareArena::new();
        let shared = arena
            .comp_type_opaque(CompType::returner(ValueType::Atom(String::from("A"))))
            .expect("mint comp type opaque");
        let body = arena
            .comp_type_bound(bound(0, 0))
            .expect("mint comp type bound");
        let root = arena
            .comp_type_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(shared),
                body,
            })
            .expect("mint comp type share");
        assert_eq!(
            erase_comp_type(&arena, root),
            Ok(CompType::returner(ValueType::Atom(String::from("A")))),
            "computation-type sharing erases through the checked family face"
        );
        assert_eq!(validate(&arena, AnyShareId::from(root)), Ok(()));
    }

    #[test]
    fn nested_shares_resolve_by_distance()
    {
        let mut arena = ShareArena::new();
        let outer_leg = arena.value_opaque(int(1)).expect("mint value opaque");
        let inner_leg = arena.value_opaque(int(2)).expect("mint value opaque");
        let at_inner = arena.value_bound(bound(0, 0)).expect("mint value bound");
        let at_outer = arena.value_bound(bound(1, 0)).expect("mint value bound");
        let graft = arena
            .value_graft(Graft {
                template: Value::pair(Value::Var(seam(0)), Value::Var(seam(1))),
                children: vec![AnyShareId::from(at_inner), AnyShareId::from(at_outer)],
            })
            .expect("mint value graft");
        let inner = arena
            .value_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(inner_leg),
                body: graft,
            })
            .expect("mint value share");
        let outer = arena
            .value_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(outer_leg),
                body: inner,
            })
            .expect("mint value share");
        assert_eq!(
            erase_value(&arena, outer),
            Ok(Value::pair(int(2), int(1))),
            "distance 0 resolves the nearest closure, distance 1 the enclosing one"
        );
        assert_eq!(validate(&arena, AnyShareId::from(outer)), Ok(()));
    }

    #[test]
    fn a_shared_leg_erases_under_the_enclosing_environment()
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(int(7)).expect("mint value opaque");
        let leg = arena.value_bound(bound(0, 0)).expect("mint value bound");
        let body = arena.value_bound(bound(0, 0)).expect("mint value bound");
        let inner = arena
            .value_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(leg),
                body,
            })
            .expect("mint value share");
        let outer = arena
            .value_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(shared),
                body: inner,
            })
            .expect("mint value share");
        assert_eq!(
            erase_value(&arena, outer),
            Ok(int(7)),
            "the inner closure's shared leg resolves in the enclosing closure's environment"
        );
        assert_eq!(validate(&arena, AnyShareId::from(outer)), Ok(()));
    }

    #[test]
    fn a_dangling_bound_fails_closed()
    {
        let mut arena = ShareArena::new();
        let bare = arena.value_bound(bound(0, 0)).expect("mint value bound");
        assert_eq!(
            erase_value(&arena, bare),
            Err(EraseError::DanglingBound(bound(0, 0))),
            "a reference with no enclosing closure dangles"
        );
        let shared = arena.value_opaque(int(1)).expect("mint value opaque");
        let overrun = arena.value_bound(bound(0, 1)).expect("mint value bound");
        let root = arena
            .value_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(shared),
                body: overrun,
            })
            .expect("mint value share");
        assert_eq!(
            erase_value(&arena, root),
            Err(EraseError::DanglingBound(bound(0, 1))),
            "a position past the closure's arity dangles"
        );
    }

    #[test]
    fn ids_are_scoped_to_one_arena_run()
    {
        let mut first = ShareArena::new();
        let mut second = ShareArena::new();
        let first_id = first.value_opaque(int(1)).expect("mint value opaque");
        let second_id = second.value_opaque(int(2)).expect("mint value opaque");
        assert_eq!(
            first_id, second_id,
            "compact ids are meaningful only within their arena run"
        );
    }

    #[test]
    fn child_bounds_are_checked_at_mint()
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(int(1)).expect("mint value opaque");
        let forged = ShareId::from_index(ShareIndex::from(99));
        assert_eq!(
            arena.value_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(shared),
                body: forged,
            }),
            Err(MintError::ChildOutOfBounds(AnyShareId::Value(forged))),
            "a sharing body must resolve before its parent is appended"
        );
    }

    #[test]
    fn permuted_positions_are_rejected()
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(int(3)).expect("mint value opaque");
        let second = arena.value_bound(bound(0, 1)).expect("mint value bound");
        let first = arena.value_bound(bound(0, 0)).expect("mint value bound");
        let graft = arena
            .value_graft(Graft {
                template: Value::pair(Value::Var(seam(0)), Value::Var(seam(1))),
                children: vec![AnyShareId::from(second), AnyShareId::from(first)],
            })
            .expect("mint value graft");
        let root = arena
            .value_share(Sharing {
                arity: Arity::from(2),
                shared: AnyShareId::from(shared),
                body: graft,
            })
            .expect("mint value share");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::NonCanonicalPosition {
                expected: VectorPosition::from(0),
                found: VectorPosition::from(1),
            }),
            "the first occurrence must take position 0"
        );
        assert_eq!(
            erase_value(&arena, root),
            Ok(Value::pair(int(3), int(3))),
            "erasure is position-independent: positions are canonical bookkeeping, not semantics"
        );
    }

    #[test]
    fn zero_arity_is_rejected()
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(int(1)).expect("mint value opaque");
        let body = arena.value_opaque(int(2)).expect("mint value opaque");
        let root = arena
            .value_share(Sharing {
                arity: Arity::from(0),
                shared: AnyShareId::from(shared),
                body,
            })
            .expect("mint value share");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::ArityZero),
            "weakening waits for the garbage rules"
        );
    }

    #[test]
    fn an_incomplete_body_is_rejected()
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(int(1)).expect("mint value opaque");
        let body = arena.value_bound(bound(0, 0)).expect("mint value bound");
        let root = arena
            .value_share(Sharing {
                arity: Arity::from(2),
                shared: AnyShareId::from(shared),
                body,
            })
            .expect("mint value share");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::IncompletePositions {
                arity: Arity::from(2),
                found: OccurrenceCount::from(1),
            }),
            "one occurrence cannot satisfy arity two"
        );
    }

    #[test]
    fn a_dangling_bound_is_rejected()
    {
        let mut arena = ShareArena::new();
        let bare = arena.value_bound(bound(0, 0)).expect("mint value bound");
        assert_eq!(
            validate(&arena, AnyShareId::from(bare)),
            Err(ValidationError::DanglingBound(bound(0, 0))),
            "a reference with no enclosing closure is ill-formed"
        );
        let shared = arena.value_opaque(int(1)).expect("mint value opaque");
        let escaping = arena.value_bound(bound(1, 0)).expect("mint value bound");
        let root = arena
            .value_share(Sharing {
                arity: Arity::from(1),
                shared: AnyShareId::from(shared),
                body: escaping,
            })
            .expect("mint value share");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::DanglingBound(bound(1, 0))),
            "a distance past the enclosing closures is ill-formed"
        );
    }
    #[test]
    fn a_comp_child_is_rejected_during_erase()
    {
        let mut arena = ShareArena::new();
        let child = arena
            .comp_opaque(Comp::ret(int(1)))
            .expect("mint comp opaque");
        let root = arena
            .value_graft(Graft {
                template: Value::Var(seam(0)),
                children: vec![AnyShareId::from(child)],
            })
            .expect("mint value graft");
        assert_eq!(
            erase_value(&arena, root),
            Err(EraseError::FamilyMismatch {
                expected: ShareFamily::Value,
                found: ShareFamily::Comp,
            }),
            "a computation child cannot plug a value seam"
        );
    }

    #[test]
    fn repeated_seams_are_rejected()
    {
        let mut arena = ShareArena::new();
        let left = arena.value_opaque(var("a")).expect("mint value opaque");
        let right = arena.value_opaque(var("b")).expect("mint value opaque");
        let root = arena
            .value_graft(Graft {
                template: Value::pair(Value::Var(seam(0)), Value::Var(seam(0))),
                children: vec![AnyShareId::from(left), AnyShareId::from(right)],
            })
            .expect("mint value graft");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::Seam(SeamProblem::Repeated(
                SeamIndex::from(0)
            ))),
            "a repeated seam duplicates a child with no sharing former naming it"
        );
        assert_eq!(
            erase_value(&arena, root),
            Err(EraseError::Seam(SeamProblem::Repeated(SeamIndex::from(0)))),
            "repeated seams are refused before any child plug"
        );
    }

    #[test]
    fn out_of_order_seams_are_rejected()
    {
        let mut arena = ShareArena::new();
        let left = arena.value_opaque(var("a")).expect("mint value opaque");
        let right = arena.value_opaque(var("b")).expect("mint value opaque");
        let root = arena
            .value_graft(Graft {
                template: Value::pair(Value::Var(seam(1)), Value::Var(seam(0))),
                children: vec![AnyShareId::from(left), AnyShareId::from(right)],
            })
            .expect("mint value graft");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::Seam(SeamProblem::OutOfOrder {
                expected: SeamIndex::from(0),
                found: SeamIndex::from(1),
            })),
            "the enumeration is first occurrence, not child order of appearance"
        );
    }

    #[test]
    fn missing_seams_are_rejected()
    {
        let mut arena = ShareArena::new();
        let left = arena.value_opaque(var("a")).expect("mint value opaque");
        let right = arena.value_opaque(var("b")).expect("mint value opaque");
        let root = arena
            .value_graft(Graft {
                template: Value::Var(seam(0)),
                children: vec![AnyShareId::from(left), AnyShareId::from(right)],
            })
            .expect("mint value graft");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::Seam(SeamProblem::Missing {
                expected: SeamIndex::from(1),
                children: ChildCount::from(2),
            })),
            "every child needs a seam"
        );
    }

    #[test]
    fn seam_binder_collisions_are_rejected()
    {
        let mut arena = ShareArena::new();
        let root = arena
            .comp_graft(Graft {
                template: Comp::lam("seam.0", Comp::ret(Value::Unit)),
                children: Vec::new(),
            })
            .expect("mint comp graft");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::Seam(SeamProblem::BinderCollision(
                SeamText::from(String::from("seam.0"),)
            ))),
            "a binder must not name the seam namespace"
        );
    }

    #[test]
    fn malformed_seam_claims_are_rejected()
    {
        let mut arena = ShareArena::new();
        let child = arena.value_opaque(var("a")).expect("mint value opaque");
        let root = arena
            .value_graft(Graft {
                template: Value::Var(String::from("seam.00")),
                children: vec![AnyShareId::from(child)],
            })
            .expect("mint value graft");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::Seam(SeamProblem::Malformed(
                SeamText::from(String::from("seam.00"),)
            ))),
            "a non-canonical seam claim is malformed, not a seam"
        );
    }

    #[test]
    fn aliased_bound_nodes_count_as_two_occurrences()
    {
        let mut arena = ShareArena::new();
        let shared = arena.value_opaque(int(3)).expect("mint value opaque");
        let occurrence = arena.value_bound(bound(0, 0)).expect("mint value bound");
        let graft = arena
            .value_graft(Graft {
                template: Value::pair(Value::Var(seam(0)), Value::Var(seam(1))),
                children: vec![AnyShareId::from(occurrence), AnyShareId::from(occurrence)],
            })
            .expect("mint value graft");
        let root = arena
            .value_share(Sharing {
                arity: Arity::from(2),
                shared: AnyShareId::from(shared),
                body: graft,
            })
            .expect("mint value share");
        assert_eq!(
            validate(&arena, AnyShareId::from(root)),
            Err(ValidationError::NonCanonicalPosition {
                expected: VectorPosition::from(1),
                found: VectorPosition::from(0),
            }),
            "one arena node referenced twice is two textual occurrences"
        );
    }
}
