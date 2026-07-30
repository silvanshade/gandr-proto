//! Semantic value wrappers shared by graph algorithms.
//!
//! # Contract
//! - requires: callers keep dense node ids inside the bounds documented by the
//!   algorithm that consumes them.
//! - ensures: primitive graph quantities cross public and crate-local function
//!   boundaries behind graph-domain names.
//! - provides: node identities, node counts, vector positions/capacities, path
//!   lengths, fingerprints, and edge/component identities.
//! - panics: none.
//! - intension: wrappers are transparent newtypes; primitive extraction is
//!   routed through standard conversion traits rather than inherent primitive
//!   accessors.
//!
//! # Adequacy
//! - hypothesis: L3 pointwise — the named graph menu distinguishes node
//!   identities, node counts, path lengths, component identities, and boundary
//!   failures through exact returned rows and typed errors.
//! - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`

use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;

/// Dense graph node identity.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(u32);

impl From<u32> for NodeId
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<NodeId> for u32
{
    #[inline]
    fn from(value: NodeId) -> Self
    {
        value.0
    }
}

impl Display for NodeId
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Dense graph node-count bound.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeCount(u32);

impl NodeCount
{
    /// Returns all dense node ids in `0..self` order.
    #[inline]
    #[must_use]
    pub const fn ids(self) -> NodeIdRange
    {
        NodeIdRange {
            next: 0,
            end: self.0,
        }
    }
}

impl From<u32> for NodeCount
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<NodeCount> for u32
{
    #[inline]
    fn from(value: NodeCount) -> Self
    {
        value.0
    }
}

impl TryFrom<NodeCount> for usize
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: NodeCount) -> Result<Self, Self::Error>
    {
        Self::try_from(u32::from(value))
    }
}

impl Display for NodeCount
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Iterator over dense graph node identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIdRange
{
    /// Next raw dense id to yield.
    next: u32,
    /// Exclusive raw dense id bound.
    end: u32,
}

impl Iterator for NodeIdRange
{
    type Item = NodeId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        (self.next < self.end).then(|| {
            let node = NodeId(self.next);
            self.next = self.next.checked_add(1).unwrap_or(self.end);
            node
        })
    }
}

/// Addressable vector capacity for dense graph node storage.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeCapacity(usize);

impl TryFrom<NodeCount> for NodeCapacity
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: NodeCount) -> Result<Self, Self::Error>
    {
        usize::try_from(u32::from(value)).map(Self)
    }
}

impl From<usize> for NodeCapacity
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<NodeCapacity> for usize
{
    #[inline]
    fn from(value: NodeCapacity) -> Self
    {
        value.0
    }
}

/// Addressable vector position for one dense graph node.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodePosition(usize);

impl TryFrom<NodeId> for NodePosition
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: NodeId) -> Result<Self, Self::Error>
    {
        usize::try_from(u32::from(value)).map(Self)
    }
}

impl TryFrom<NodePosition> for NodeId
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: NodePosition) -> Result<Self, Self::Error>
    {
        u32::try_from(value.0).map(Self)
    }
}

impl From<usize> for NodePosition
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<NodePosition> for usize
{
    #[inline]
    fn from(value: NodePosition) -> Self
    {
        value.0
    }
}

/// Dense directed edge identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EdgeId
{
    /// Source endpoint.
    pub source: NodeId,
    /// Target endpoint.
    pub target: NodeId,
}

impl EdgeId
{
    /// Constructs a directed edge identity.
    #[inline]
    #[must_use]
    pub const fn new(
        source: NodeId,
        target: NodeId,
    ) -> Self
    {
        Self { source, target }
    }
}

impl Display for EdgeId
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        write!(f, "{}>{}", self.source, self.target)
    }
}

/// Canonical strongly-connected-component identity.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComponentIndex(u32);

impl From<u32> for ComponentIndex
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<ComponentIndex> for u32
{
    #[inline]
    fn from(value: ComponentIndex) -> Self
    {
        value.0
    }
}

impl Display for ComponentIndex
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Directed edge between condensation components.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComponentEdge
{
    /// Source component.
    pub source: ComponentIndex,
    /// Target component.
    pub target: ComponentIndex,
}

impl ComponentEdge
{
    /// Constructs a condensation edge identity.
    #[inline]
    #[must_use]
    pub const fn new(
        source: ComponentIndex,
        target: ComponentIndex,
    ) -> Self
    {
        Self { source, target }
    }
}

impl Display for ComponentEdge
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        write!(f, "{}>{}", self.source, self.target)
    }
}

/// Graph path length measured in directed edge count.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PathLength(u32);

impl PathLength
{
    /// Saturating predecessor length.
    #[inline]
    #[must_use]
    pub fn saturating_predecessor(self) -> Self
    {
        Self(self.0.saturating_sub(1))
    }
}

impl From<u32> for PathLength
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl TryFrom<usize> for PathLength
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error>
    {
        u32::try_from(value).map(Self)
    }
}

impl From<PathLength> for u32
{
    #[inline]
    fn from(value: PathLength) -> Self
    {
        value.0
    }
}

impl TryFrom<PathLength> for usize
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: PathLength) -> Result<Self, Self::Error>
    {
        Self::try_from(value.0)
    }
}

impl Display for PathLength
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Height of one swing measured as `nonterminal_count - 1`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SwingHeight(u32);

impl From<u32> for SwingHeight
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<SwingHeight> for u32
{
    #[inline]
    fn from(value: SwingHeight) -> Self
    {
        value.0
    }
}

impl Display for SwingHeight
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Count of non-zero-height swings in a walk.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WalkHeight(u32);

impl From<u32> for WalkHeight
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<WalkHeight> for u32
{
    #[inline]
    fn from(value: WalkHeight) -> Self
    {
        value.0
    }
}

impl Display for WalkHeight
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Alternating walk-chain length measured as `2 * swings - 1`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WalkChainLength(u32);

impl From<u32> for WalkChainLength
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl TryFrom<usize> for WalkChainLength
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error>
    {
        u32::try_from(value).map(Self)
    }
}

impl From<WalkChainLength> for u32
{
    #[inline]
    fn from(value: WalkChainLength) -> Self
    {
        value.0
    }
}

impl TryFrom<WalkChainLength> for usize
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: WalkChainLength) -> Result<Self, Self::Error>
    {
        Self::try_from(value.0)
    }
}

impl Display for WalkChainLength
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Stable graph fingerprint bits.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Fingerprint(u64);

impl From<u64> for Fingerprint
{
    #[inline]
    fn from(value: u64) -> Self
    {
        Self(value)
    }
}

impl From<Fingerprint> for u64
{
    #[inline]
    fn from(value: Fingerprint) -> Self
    {
        value.0
    }
}

impl Display for Fingerprint
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// One byte in a framed fingerprint stream.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FingerprintByte(u8);

impl From<u8> for FingerprintByte
{
    #[inline]
    fn from(value: u8) -> Self
    {
        Self(value)
    }
}

impl From<FingerprintByte> for u8
{
    #[inline]
    fn from(value: FingerprintByte) -> Self
    {
        value.0
    }
}

/// One little-endian `u16` word in a fingerprint stream.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FingerprintWord16(u16);

impl From<u16> for FingerprintWord16
{
    #[inline]
    fn from(value: u16) -> Self
    {
        Self(value)
    }
}

impl From<FingerprintWord16> for u16
{
    #[inline]
    fn from(value: FingerprintWord16) -> Self
    {
        value.0
    }
}

/// One little-endian `u32` word in a fingerprint stream.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FingerprintWord32(u32);

impl From<u32> for FingerprintWord32
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<FingerprintWord32> for u32
{
    #[inline]
    fn from(value: FingerprintWord32) -> Self
    {
        value.0
    }
}

/// One little-endian `u64` word in a fingerprint stream.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FingerprintWord64(u64);

impl From<u64> for FingerprintWord64
{
    #[inline]
    fn from(value: u64) -> Self
    {
        Self(value)
    }
}

impl From<FingerprintWord64> for u64
{
    #[inline]
    fn from(value: FingerprintWord64) -> Self
    {
        value.0
    }
}

/// Borrowed byte payload for fingerprint framing.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FingerprintBytes<'bytes>(&'bytes [u8]);

impl<'bytes> From<&'bytes [u8]> for FingerprintBytes<'bytes>
{
    #[inline]
    fn from(value: &'bytes [u8]) -> Self
    {
        Self(value)
    }
}
impl<'bytes, const N: usize> From<&'bytes [u8; N]> for FingerprintBytes<'bytes>
{
    #[inline]
    fn from(value: &'bytes [u8; N]) -> Self
    {
        Self(value)
    }
}

impl AsRef<[u8]> for FingerprintBytes<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        self.0
    }
}
