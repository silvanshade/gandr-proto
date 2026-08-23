//! The semantic wrappers this crate's signatures carry in place of bare
//! primitives.
//!
//! Several of these cross into the query graph as keys or as input fields, so
//! they carry the hashing and equality the engine needs of a key on top of the
//! nominal boundary the workspace requires of a signature.

use gandr_core_incremental::persistence::CheckpointAddress;

/// The position of one item in a program revision — the engine's item identity.
///
/// Stable across a body edit, which is what makes it usable as a query key: an
/// edit replaces an item's *content*, never the slot it occupies.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SlotIndex(usize);

impl From<usize> for SlotIndex
{
    #[inline]
    fn from(index: usize) -> Self
    {
        Self(index)
    }
}

impl From<SlotIndex> for usize
{
    #[inline]
    fn from(index: SlotIndex) -> Self
    {
        index.0
    }
}

/// A definition's name, as the query graph keys on it.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DefinitionKey(String);

impl From<String> for DefinitionKey
{
    #[inline]
    fn from(name: String) -> Self
    {
        Self(name)
    }
}

impl AsRef<str> for DefinitionKey
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

/// The content address of one item — what the engine's change detection
/// compares.
///
/// Derived through `gandr-core-incremental`'s own program addressing, so an
/// item's digest is the address of the single-item program containing it. Two
/// items with equal digests are structurally equal, which is the property the
/// dirty-marking rests on.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ItemDigest([u8; 32]);

impl From<CheckpointAddress> for ItemDigest
{
    #[inline]
    fn from(address: CheckpointAddress) -> Self
    {
        Self(address.bytes())
    }
}

/// A count of items.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ItemCount(usize);

impl From<usize> for ItemCount
{
    #[inline]
    fn from(count: usize) -> Self
    {
        Self(count)
    }
}

impl From<ItemCount> for usize
{
    #[inline]
    fn from(count: ItemCount) -> Self
    {
        count.0
    }
}

/// A count of query bodies that actually ran.
///
/// The distinction this exists to keep visible: a query the engine *consulted*
/// is not a query the engine *executed*, and conflating the two is how an
/// engine's advantage gets reported without being real.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ExecutionCount(usize);

impl From<usize> for ExecutionCount
{
    #[inline]
    fn from(count: usize) -> Self
    {
        Self(count)
    }
}

impl From<ExecutionCount> for usize
{
    #[inline]
    fn from(count: ExecutionCount) -> Self
    {
        count.0
    }
}

/// A count of memoized values reused after their dependencies verified
/// unchanged.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ValidationCount(usize);

impl From<usize> for ValidationCount
{
    #[inline]
    fn from(count: usize) -> Self
    {
        Self(count)
    }
}

impl From<ValidationCount> for usize
{
    #[inline]
    fn from(count: ValidationCount) -> Self
    {
        count.0
    }
}

/// A count of bytes encoded or decoded at the engine's ownership boundary.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct BoundaryByteCount(usize);

impl From<usize> for BoundaryByteCount
{
    #[inline]
    fn from(count: usize) -> Self
    {
        Self(count)
    }
}

impl From<BoundaryByteCount> for usize
{
    #[inline]
    fn from(count: BoundaryByteCount) -> Self
    {
        count.0
    }
}

/// Elapsed wall-clock nanoseconds for one measured recheck.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ElapsedNanos(u128);

impl From<core::time::Duration> for ElapsedNanos
{
    #[inline]
    fn from(elapsed: core::time::Duration) -> Self
    {
        Self(elapsed.as_nanos())
    }
}

impl From<ElapsedNanos> for u128
{
    #[inline]
    fn from(elapsed: ElapsedNanos) -> Self
    {
        elapsed.0
    }
}

/// A count of bytes the engine reports retained in its own tables.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct RetainedByteCount(usize);

impl From<usize> for RetainedByteCount
{
    #[inline]
    fn from(count: usize) -> Self
    {
        Self(count)
    }
}

impl From<RetainedByteCount> for usize
{
    #[inline]
    fn from(count: RetainedByteCount) -> Self
    {
        count.0
    }
}

/// The number of independent dependency chains in a generated workload.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BlockCount(usize);

impl From<usize> for BlockCount
{
    #[inline]
    fn from(count: usize) -> Self
    {
        Self(count)
    }
}

impl From<BlockCount> for usize
{
    #[inline]
    fn from(count: BlockCount) -> Self
    {
        count.0
    }
}

/// The number of items in one dependency chain of a generated workload.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BlockLength(usize);

impl From<usize> for BlockLength
{
    #[inline]
    fn from(length: usize) -> Self
    {
        Self(length)
    }
}

impl From<BlockLength> for usize
{
    #[inline]
    fn from(length: BlockLength) -> Self
    {
        length.0
    }
}

/// An integer literal a generated item carries as its body.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LiteralValue(i64);

impl From<i64> for LiteralValue
{
    #[inline]
    fn from(literal: i64) -> Self
    {
        Self(literal)
    }
}

impl From<LiteralValue> for i64
{
    #[inline]
    fn from(literal: LiteralValue) -> Self
    {
        literal.0
    }
}

/// The text of one manifest, as the confinement scan reads it.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManifestText<'source>(&'source str);

impl<'source> From<&'source str> for ManifestText<'source>
{
    #[inline]
    fn from(text: &'source str) -> Self
    {
        Self(text)
    }
}

impl<'source> ManifestText<'source>
{
    /// The manifest's lines, in order.
    #[inline]
    pub fn lines(self) -> core::str::Lines<'source>
    {
        self.0.lines()
    }
}

/// The directory name of one workspace member.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MemberName(String);

impl From<String> for MemberName
{
    #[inline]
    fn from(name: String) -> Self
    {
        Self(name)
    }
}

impl AsRef<str> for MemberName
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

/// A short label naming a measured path or demand shape in the reported table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RowLabel(&'static str);

impl From<&'static str> for RowLabel
{
    #[inline]
    fn from(label: &'static str) -> Self
    {
        Self(label)
    }
}

impl core::fmt::Display for RowLabel
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.pad(self.0)
    }
}
