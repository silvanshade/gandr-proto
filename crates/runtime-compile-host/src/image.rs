//! The program image: the plain-old-data boundary the compilation host reads.
//!
//! The representation mirrors the host's own, because the two have to agree on
//! it byte for byte for anything to cross. What keeps them agreeing is not this
//! comment: it is the parity gate in this crate's test suite, which reads the
//! host's headers and holds every number here to what they declare.

use core::num::TryFromIntError;

/// A node's position in an image's flat arena.
///
/// The wire form carries the position as four little-endian bytes, so the
/// width is part of the boundary rather than a local choice.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct NodeIndex(u32);

impl From<NodeIndex> for u32
{
    #[inline]
    fn from(index: NodeIndex) -> Self
    {
        index.0
    }
}

impl TryFrom<usize> for NodeIndex
{
    type Error = TryFromIntError;

    #[inline]
    fn try_from(position: usize) -> Result<Self, Self::Error>
    {
        let narrowed = u32::try_from(position)?;
        Ok(Self(narrowed))
    }
}

/// How many binders separate a variable from the one it names.
///
/// Zero is the innermost binder. The host's own field is spelled `binder` and
/// its lookup is `environment[len - 1 - binder]`, so this is a de Bruijn index
/// counted inwards, not a level counted outwards.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BinderIndex(u32);

impl From<BinderIndex> for u32
{
    #[inline]
    fn from(index: BinderIndex) -> Self
    {
        index.0
    }
}

impl TryFrom<usize> for BinderIndex
{
    type Error = TryFromIntError;

    #[inline]
    fn try_from(distance: usize) -> Result<Self, Self::Error>
    {
        let narrowed = u32::try_from(distance)?;
        Ok(Self(narrowed))
    }
}

/// An integer literal's payload.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Literal(i64);

impl From<i64> for Literal
{
    #[inline]
    fn from(value: i64) -> Self
    {
        Self(value)
    }
}

impl From<Literal> for i64
{
    #[inline]
    fn from(literal: Literal) -> Self
    {
        literal.0
    }
}

/// How many nodes an image holds.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct NodeCount(usize);

impl From<NodeCount> for usize
{
    #[inline]
    fn from(count: NodeCount) -> Self
    {
        count.0
    }
}

/// How many duplications and discards a run is expected to execute.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AccountedWork
{
    /// The duplications.
    pub duplications: WorkCount,
    /// The discards.
    pub discards: WorkCount,
}

/// A count of executed accounted operations.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct WorkCount(i64);

impl From<i64> for WorkCount
{
    #[inline]
    fn from(count: i64) -> Self
    {
        Self(count)
    }
}

impl From<WorkCount> for i64
{
    #[inline]
    fn from(count: WorkCount) -> Self
    {
        count.0
    }
}

/// The positive-core node kinds.
///
/// The discriminants are the wire form's, so they are pinned here rather than
/// left to declaration order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum NodeKind
{
    /// An integer literal producer.
    Lit = 0,
    /// A reference to an enclosing binder.
    Var = 1,
    /// A positive constructor introduction.
    Ctor = 2,
    /// The grade structural duplication.
    Dup = 3,
    /// The grade structural discard.
    Drop = 4,
    /// The sequencing consumer: the binder frame.
    Bind = 5,
    /// A constructor dispatch consumer.
    Case = 6,
    /// The terminal cut against the top-level consumer.
    Cut = 7,
}

impl NodeKind
{
    /// The kind's wire byte.
    ///
    /// # Contract
    /// - ensures: the byte the host's decoder reads as this kind.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn wire_byte(self) -> WireByte
    {
        WireByte(match self {
            | Self::Lit => 0,
            | Self::Var => 1,
            | Self::Ctor => 2,
            | Self::Dup => 3,
            | Self::Drop => 4,
            | Self::Bind => 5,
            | Self::Case => 6,
            | Self::Cut => 7,
        })
    }
}

/// The constructor tags the slice admits.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum CtorTag
{
    /// The unit value; arity zero.
    #[default]
    Unit = 0,
    /// An eager pair; arity two.
    Pair = 1,
    /// The left sum injection; arity one.
    Inl = 2,
    /// The right sum injection; arity one.
    Inr = 3,
}

impl CtorTag
{
    /// The tag's wire byte.
    ///
    /// # Contract
    /// - ensures: the byte the host's decoder reads as this tag.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn wire_byte(self) -> WireByte
    {
        WireByte(match self {
            | Self::Unit => 0,
            | Self::Pair => 1,
            | Self::Inl => 2,
            | Self::Inr => 3,
        })
    }

    /// How many producer arguments the tag declares.
    ///
    /// # Contract
    /// - ensures: total over the tag set, and equal to the arity the host's
    ///   operation verifier holds a constructor to.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn arity(self) -> CtorArity
    {
        CtorArity(match self {
            | Self::Unit => 0,
            | Self::Pair => 2,
            | Self::Inl | Self::Inr => 1,
        })
    }
}

/// One byte of the wire form.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct WireByte(u8);

impl From<WireByte> for u8
{
    #[inline]
    fn from(byte: WireByte) -> Self
    {
        byte.0
    }
}

/// The number of producer arguments a constructor tag declares.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CtorArity(usize);

impl From<CtorArity> for usize
{
    #[inline]
    fn from(arity: CtorArity) -> Self
    {
        arity.0
    }
}

/// One node of a program image.
///
/// A field a kind does not use is zero, exactly as on the host's side: the
/// record is flat rather than a variant so that the image stays plain old data
/// with no owning pointers to translate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node
{
    /// Which positive-core form this node is.
    pub kind: NodeKind,
    /// The constructor tag, for [`NodeKind::Ctor`].
    pub tag: CtorTag,
    /// The referenced binder's distance, for [`NodeKind::Var`].
    pub binder: BinderIndex,
    /// The integer payload, for [`NodeKind::Lit`].
    pub literal: Literal,
    /// The operand list, in the order the host's evaluator reads it.
    pub operands: Vec<NodeIndex>,
}

/// A complete program image: a flat node arena whose last node is its root.
///
/// The root is positional rather than stored, because the wire form has no
/// root field: the host's decoder takes the last node and requires it to be
/// the terminal cut.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct Image
{
    /// The node arena, in dependency order.
    nodes: Vec<Node>,
}

/// The largest node count the host's decoder admits.
///
/// The host declares the same bound; the parity gate holds the two equal.
pub const MAX_IMAGE_NODES: usize = 4096;

/// The image wire version this crate writes.
pub const IMAGE_WIRE_VERSION: u8 = 1;

impl Image
{
    /// An empty arena.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self { nodes: Vec::new() }
    }

    /// The nodes, in arena order.
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> &[Node]
    {
        &self.nodes
    }

    /// How many nodes the arena holds.
    #[inline]
    #[must_use]
    pub fn len(&self) -> NodeCount
    {
        NodeCount(self.nodes.len())
    }

    /// Whether the arena holds no nodes.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> ArenaEmptiness
    {
        ArenaEmptiness(self.nodes.is_empty())
    }

    /// Appends a node and returns its index.
    ///
    /// # Contract
    /// - requires: every operand of `node` addresses a strictly earlier node,
    ///   which the lowering satisfies by emitting operands first.
    /// - ensures: the returned index addresses `node` in this arena.
    /// - provides: the one way a node enters an image, so the arena's bound is
    ///   enforced in one place.
    /// - fails: [`ImageError::TooManyNodes`] once the arena would exceed the
    ///   host's declared bound.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ImageError::TooManyNodes`] when the arena is already at
    /// [`MAX_IMAGE_NODES`], or when the position does not fit the wire form's
    /// index width.
    ///
    /// # Adequacy
    /// - hypothesis: L0 and L3 — the index width and the arena bound are
    ///   separated by the exact boundary at `MAX_IMAGE_NODES`, and every other
    ///   decision here is carried by the types.
    /// - witness: `image::tests::the_arena_refuses_a_node_past_the_declared_bound`
    #[inline]
    pub fn push(
        &mut self,
        node: Node,
    ) -> Result<NodeIndex, ImageError>
    {
        if self.nodes.len() >= MAX_IMAGE_NODES {
            return Err(ImageError::TooManyNodes);
        }
        let index =
            NodeIndex::try_from(self.nodes.len()).map_err(|_narrowing| ImageError::TooManyNodes)?;
        self.nodes.push(node);
        Ok(index)
    }

    /// The duplications and discards the arena holds.
    ///
    /// # Contract
    /// - ensures: one count per accounted node kind, over the whole arena.
    /// - provides: the external expectation a dispatch-free image's run is held
    ///   to — such a run executes every node exactly once, so the counts are
    ///   the ledger rather than a bound on it.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the counts are compared against the ledger the host
    ///   reports for a dispatch-free program, which is an oracle outside this
    ///   crate entirely.
    /// - witness: `bridge::the_bridge_agrees_with_the_l_machine_on_every_named_program`
    #[inline]
    #[must_use]
    pub fn accounted_work(&self) -> AccountedWork
    {
        let mut duplications: i64 = 0;
        let mut discards: i64 = 0;
        for node in &self.nodes {
            match node.kind {
                | NodeKind::Dup => duplications = duplications.saturating_add(1),
                | NodeKind::Drop => discards = discards.saturating_add(1),
                | _ => {},
            }
        }
        AccountedWork {
            duplications: WorkCount(duplications),
            discards: WorkCount(discards),
        }
    }

    /// Whether the arena holds a dispatch.
    ///
    /// # Contract
    /// - ensures: true exactly when some node is a [`NodeKind::Case`].
    /// - provides: the side condition under which [`Image::accounted_work`] is
    ///   the executed ledger rather than an upper bound, because a dispatch
    ///   runs one arm and skips the other.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn has_dispatch(&self) -> DispatchPresence
    {
        DispatchPresence(self.nodes.iter().any(|node| node.kind == NodeKind::Case))
    }

    /// Encodes the arena into the byte form the host's decoder accepts.
    ///
    /// # Contract
    /// - requires: the arena's last node is the terminal cut, which the
    ///   lowering guarantees and the host's decoder rechecks.
    /// - ensures: little-endian throughout, so the bytes do not depend on this
    ///   machine's byte order.
    /// - provides: the only thing that crosses the boundary.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the host decodes what this writes, and a byte that
    ///   disagreed with its reader would fail to decode or would decode into a
    ///   different program; both are visible in the agreement differential.
    /// - witness: `bridge::the_bridge_agrees_with_the_l_machine_on_every_named_program`
    /// - witness: `image::tests::the_wire_form_leads_with_its_version_and_node_count`
    #[inline]
    #[must_use]
    pub fn encode(&self) -> ImageBytes
    {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.push(IMAGE_WIRE_VERSION);
        let count = u16::try_from(self.nodes.len()).unwrap_or(u16::MAX);
        bytes.extend_from_slice(&count.to_le_bytes());
        for node in &self.nodes {
            bytes.push(u8::from(node.kind.wire_byte()));
            bytes.push(u8::from(node.tag.wire_byte()));
            bytes.extend_from_slice(&u32::from(node.binder).to_le_bytes());
            bytes.extend_from_slice(&i64::from(node.literal).to_le_bytes());
            let operands = u8::try_from(node.operands.len()).unwrap_or(u8::MAX);
            bytes.push(operands);
            for operand in &node.operands {
                bytes.extend_from_slice(&u32::from(*operand).to_le_bytes());
            }
        }
        ImageBytes(bytes)
    }
}

/// Whether an arena holds no nodes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct ArenaEmptiness(bool);

impl From<ArenaEmptiness> for bool
{
    #[inline]
    fn from(emptiness: ArenaEmptiness) -> Self
    {
        emptiness.0
    }
}

/// Whether an arena holds a dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct DispatchPresence(bool);

impl From<DispatchPresence> for bool
{
    #[inline]
    fn from(presence: DispatchPresence) -> Self
    {
        presence.0
    }
}

/// The encoded image, ready to cross the boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct ImageBytes(Vec<u8>);

impl AsRef<[u8]> for ImageBytes
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        &self.0
    }
}

/// What can go wrong while assembling an image.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ImageError
{
    /// The arena would exceed the node count the host's decoder admits.
    #[error("the program image would hold more nodes than the host accepts")]
    TooManyNodes,
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// A node of the given kind with no operands.
    fn leaf(kind: NodeKind) -> Node
    {
        Node {
            kind,
            tag: CtorTag::Unit,
            binder: BinderIndex::default(),
            literal: Literal::default(),
            operands: Vec::new(),
        }
    }

    #[test]
    fn the_arena_refuses_a_node_past_the_declared_bound()
    {
        let mut image = Image::new();
        for _ in 0 .. MAX_IMAGE_NODES {
            assert!(image.push(leaf(NodeKind::Lit)).is_ok());
        }
        assert_eq!(usize::from(image.len()), MAX_IMAGE_NODES);
        assert_eq!(
            image.push(leaf(NodeKind::Lit)),
            Err(ImageError::TooManyNodes)
        );
    }

    #[test]
    fn the_wire_form_leads_with_its_version_and_node_count()
    {
        let mut image = Image::new();
        assert!(image.push(leaf(NodeKind::Lit)).is_ok());
        assert!(image.push(leaf(NodeKind::Lit)).is_ok());
        let bytes = image.encode();
        let encoded: &[u8] = bytes.as_ref();
        assert_eq!(encoded.first(), Some(&IMAGE_WIRE_VERSION));
        assert_eq!(encoded.get(1), Some(&2u8));
        assert_eq!(encoded.get(2), Some(&0u8));
    }

    #[test]
    fn the_wrappers_round_trip_the_values_they_carry()
    {
        assert_eq!(i64::from(WorkCount::from(4_i64)), 4_i64);
        assert_eq!(i64::from(Literal::from(-7_i64)), -7_i64);
        assert_eq!(
            u32::from(BinderIndex::try_from(2_usize).expect("fits")),
            2_u32
        );
        assert_eq!(
            u32::from(NodeIndex::try_from(9_usize).expect("fits")),
            9_u32
        );
        assert!(!bool::from(Image::new().has_dispatch()));
        assert!(bool::from(Image::new().is_empty()));
    }

    #[test]
    fn accounted_work_counts_each_kind_separately()
    {
        let mut image = Image::new();
        assert!(image.push(leaf(NodeKind::Dup)).is_ok());
        assert!(image.push(leaf(NodeKind::Dup)).is_ok());
        assert!(image.push(leaf(NodeKind::Drop)).is_ok());
        let work = image.accounted_work();
        assert_eq!(i64::from(work.duplications), 2);
        assert_eq!(i64::from(work.discards), 1);
        assert!(!bool::from(image.has_dispatch()));
    }
}
