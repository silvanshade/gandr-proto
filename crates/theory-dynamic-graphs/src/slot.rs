//! The dense slot index both maintenance structures address their per-node
//! tables by.

use gandr_theory_graphs::NodeId;

/// A dense vector position holding one node's per-node state.
///
/// Node identities and the table positions that hold their state are different
/// things, and conflating them is how an identifier from one structure comes to
/// index another's table. This wrapper keeps the conversion explicit and
/// fallible in the one direction that can fail.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SlotIndex(usize);

impl From<usize> for SlotIndex
{
    #[inline]
    fn from(value: usize) -> Self
    {
        return Self(value);
    }
}

impl From<SlotIndex> for usize
{
    #[inline]
    fn from(value: SlotIndex) -> Self
    {
        return value.0;
    }
}

impl TryFrom<NodeId> for SlotIndex
{
    type Error = core::num::TryFromIntError;

    #[inline]
    fn try_from(value: NodeId) -> Result<Self, Self::Error>
    {
        return usize::try_from(u32::from(value)).map(Self);
    }
}
