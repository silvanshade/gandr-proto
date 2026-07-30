//! The order-maintenance structure ([`OrderMaintenance`]) and its handle
//! ([`Pos`]) — see the crate-root documentation for the problem statement,
//! algorithm, and complexity.

use alloc::vec::Vec;
use core::cmp::Ordering;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering as AtomicOrdering;

/// A process-wide counter minting a distinct identity for each
/// [`OrderMaintenance`], so a [`Pos`] from one structure is detected as foreign
/// to another rather than silently resolving against an unrelated element.
static NEXT_STRUCTURE_ID: StructureIdCounter = StructureIdCounter(AtomicU64::new(0));

/// A structure's process-unique identity, stamped into every [`Pos`] it
/// mints; a handle used against a different structure is rejected.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct StructureId(u64);

/// A counter minting distinct [`StructureId`]s; exhaustion fails rather than
/// wrapping, so a reissued id can never alias an extant [`Pos`].
#[repr(transparent)]
struct StructureIdCounter(AtomicU64);

impl StructureIdCounter
{
    #[cfg(test)]
    #[inline]
    fn nearly_exhausted() -> Self
    {
        return Self(AtomicU64::new(u64::MAX.wrapping_sub(1)));
    }

    /// Mints the next distinct [`StructureId`] from this counter.
    ///
    /// # Contract
    /// - ensures: on `Ok`, the returned id was never issued by this counter
    ///   before.
    /// - fails: returns [`OrderError::StructureIdExhausted`] once the counter
    ///   would wrap, leaving the final id permanently unissued.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`OrderError::StructureIdExhausted`] when no distinct id
    /// remains.
    #[inline]
    fn allocate(&self) -> Result<StructureId, OrderError>
    {
        return self
            .0
            .try_update(AtomicOrdering::Relaxed, AtomicOrdering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map(StructureId)
            .map_err(|_current| OrderError::StructureIdExhausted);
    }
}

/// The number of label bits in the default universe: labels live in
/// `[0, 2^62)`.
///
/// Chosen below 64 with headroom so that the relabel arithmetic (an aligned
/// window of size `2^h` for `h ≤ MAX_LABEL_BITS`, and label offsets bounded by
/// the window) never approaches `u64::MAX`, and so the density cap `2^61`
/// dwarfs the `u32` handle-index ceiling that binds first in practice.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct LabelBits(u32);

impl LabelBits
{
    /// The narrowest supported label-universe width.
    const MIN: Self = Self(1);
    /// The widest supported label-universe width: labels live in `[0, 2^62)`.
    const MAX: Self = Self(62);

    /// This width clamped to the supported range between [`Self::MIN`] and
    /// [`Self::MAX`].
    #[inline]
    fn clamped(self) -> Self
    {
        return Self(self.0.clamp(Self::MIN.0, Self::MAX.0));
    }
}

/// The number of label bits in the default universe: labels live in
/// `[0, 2^62)`.
///
/// Chosen below 64 with headroom so that the relabel arithmetic (an aligned
/// window of size `2^h` for `h ≤ MAX_LABEL_BITS`, and label offsets bounded by
/// the window) never approaches `u64::MAX`, and so the density cap `2^61`
/// dwarfs the `u32` handle-index ceiling that binds first in practice.
const MAX_LABEL_BITS: LabelBits = LabelBits::MAX;

/// An element's order label; strictly increasing in list order.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Label(u64);

/// The label-universe size `2^label_bits` (cached from the structure's
/// [`LabelBits`]).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct LabelCapacity(u64);

/// The size of a relabel window: a power-of-two count of consecutive label
/// values, redistributed evenly when the window is relabeled.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct LabelRangeSize(u64);

/// An element's slot index in the arena.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SlotIndex(u32);
#[cfg(test)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SlotLimit(usize);

impl TryFrom<usize> for SlotIndex
{
    type Error = <u32 as TryFrom<usize>>::Error;

    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error>
    {
        return u32::try_from(value).map(Self);
    }
}

impl TryFrom<SlotIndex> for usize
{
    type Error = <Self as TryFrom<u32>>::Error;

    #[inline]
    fn try_from(value: SlotIndex) -> Result<Self, Self::Error>
    {
        return Self::try_from(value.0);
    }
}

/// The number of live elements in a relabel window, counting the element
/// being inserted.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct OccupantCount(usize);

/// An element's position within the sequence spread evenly across a relabel
/// window.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SequencePosition(usize);

/// The number of live elements in an [`OrderMaintenance`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LiveLen(usize);

impl LiveLen
{
    /// The length of an empty structure.
    const ZERO: Self = Self(0);
}

impl From<LiveLen> for usize
{
    #[inline]
    fn from(value: LiveLen) -> Self
    {
        return value.0;
    }
}

impl core::fmt::Display for LiveLen
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        return self.0.fmt(f);
    }
}

/// Whether an [`OrderMaintenance`] currently has no live elements.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OrderIsEmpty(bool);

impl From<OrderIsEmpty> for bool
{
    #[inline]
    fn from(value: OrderIsEmpty) -> Self
    {
        return value.0;
    }
}

impl core::ops::Not for OrderIsEmpty
{
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output
    {
        return Self(!self.0);
    }
}

/// Whether a handle refers to a live element of an [`OrderMaintenance`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct HandleMembership(bool);

impl From<HandleMembership> for bool
{
    #[inline]
    fn from(value: HandleMembership) -> Self
    {
        return value.0;
    }
}

impl core::ops::Not for HandleMembership
{
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output
    {
        return Self(!self.0);
    }
}

/// Whether one interval contains another in the maintained order.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IntervalContainment(bool);

impl From<IntervalContainment> for bool
{
    #[inline]
    fn from(value: IntervalContainment) -> Self
    {
        return value.0;
    }
}

impl core::ops::Not for IntervalContainment
{
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output
    {
        return Self(!self.0);
    }
}

/// An opaque, stable handle to an element of an [`OrderMaintenance`].
///
/// A `Pos` stays valid while *other* elements are inserted and removed; it is
/// invalidated only by removing *its own* element. Handles are generation-
/// checked, so a handle to a removed element does not silently alias a later
/// element that reuses the freed slot — operations on a stale handle report
/// failure (`None` / [`OrderError::UnknownPosition`]) instead.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct Pos
{
    /// The identity of the [`OrderMaintenance`] this handle belongs to; a
    /// handle used against a different structure is rejected.
    structure_id: StructureId,
    /// The element's slot index in the arena.
    index: SlotIndex,
    /// The slot generation at the time this handle was minted; a later reuse
    /// of the slot bumps the slot's generation past this value.
    generation: u32,
}

/// A failure of an [`OrderMaintenance`] operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum OrderError
{
    /// An insertion was requested relative to a handle that does not refer to
    /// a live element (it was removed, or it came from a different structure).
    #[error("order-maintenance insertion relative to an unknown or stale handle")]
    UnknownPosition,
    /// No distinct structure identifier remains. Construction fails rather
    /// than reusing an identifier that could alias an extant [`Pos`].
    #[error("order-maintenance structure identifiers are exhausted")]
    StructureIdExhausted,
    /// The structure has reached the maximum number of live, free, or retired
    /// slots representable by [`Pos`], or the label-universe density cap, so no
    /// fresh element can be admitted.
    #[error("order-maintenance structure is at capacity")]
    CapacityExhausted,
}

/// One arena slot: either live, reusable, or permanently retired.
enum Slot<T>
{
    /// A live element.
    Occupied(Occupied<T>),
    /// A freed slot, threaded onto the free list.
    Free(Free),
    /// A slot whose generation space is exhausted and must never be reused.
    Retired,
}

/// A live element's slot payload.
struct Occupied<T>
{
    /// The slot's current generation (matched against a [`Pos`]).
    generation: u32,
    /// The element's order label; strictly increasing in list order.
    label: Label,
    /// The previous element in list order, if any.
    prev: Option<SlotIndex>,
    /// The next element in list order, if any.
    next: Option<SlotIndex>,
    /// The caller's payload.
    value: T,
}

/// A freed slot's payload.
struct Free
{
    /// The generation that the next occupant of this slot will carry.
    generation: u32,
    /// The next free slot on the free list, if any.
    next_free: Option<SlotIndex>,
}

/// A total order over payload-carrying elements with O(1) comparison.
///
/// See the crate-root documentation for the algorithm and complexity. The
/// structure owns its elements; iteration ([`Self::iter`]) and navigation
/// ([`Self::next`] / [`Self::prev`]) visit them in order.
#[non_exhaustive]
pub struct OrderMaintenance<T>
{
    /// The element arena, indexed by [`Pos::index`].
    slots: Vec<Slot<T>>,
    /// The head of the free list (slots available for reuse), if any.
    free_head: Option<SlotIndex>,
    /// Test-only ceiling for representable slots, used to exercise exhaustion
    /// without allocating a `u32::MAX`-sized arena.
    #[cfg(test)]
    slot_limit: Option<SlotLimit>,
    /// The first element in list order, if any.
    head: Option<SlotIndex>,
    /// The last element in list order, if any.
    tail: Option<SlotIndex>,
    /// The number of live elements.
    len: LiveLen,
    /// The label-universe width: labels live in `[0, 2^label_bits)`.
    label_bits: LabelBits,
    /// The label-universe size `2^label_bits` (cached from `label_bits`).
    capacity: LabelCapacity,
    /// This structure's process-unique identity, stamped into every [`Pos`] it
    /// mints (see [`NEXT_STRUCTURE_ID`]).
    structure_id: StructureId,
}

impl<T> OrderMaintenance<T>
{
    /// Creates an empty structure over the default `[0, 2^62)` label universe.
    ///
    /// # Contract
    /// - ensures: returns an empty structure (`len == 0`) ready for insertion.
    /// - provides: a total order whose comparison is O(1).
    /// - fails: returns [`OrderError::StructureIdExhausted`] once every
    ///   non-sentinel structure identifier has been issued.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`OrderError::StructureIdExhausted`] when no distinct structure
    /// id remains.
    #[inline]
    pub fn new() -> Result<Self, OrderError>
    {
        return Self::with_label_bits(MAX_LABEL_BITS);
    }

    /// Creates an empty structure over the `[0, 2^label_bits)` label universe,
    /// clamping the requested label width to the supported maximum.
    ///
    /// The narrow-universe form exists for tests that need to provoke
    /// relabeling and capacity exhaustion without inserting `2^61` elements; in
    /// production [`Self::new`] selects the full universe.
    #[inline]
    fn with_label_bits(label_bits: LabelBits) -> Result<Self, OrderError>
    {
        return Self::with_label_bits_from_counter(label_bits, &NEXT_STRUCTURE_ID);
    }

    /// Creates an empty structure over the clamped `[0, 2^label_bits)` label
    /// universe, drawing its identity from `counter` instead of the
    /// process-wide [`NEXT_STRUCTURE_ID`].
    ///
    /// The injectable counter exists for tests that exercise structure-id
    /// exhaustion without minting `u64::MAX` structures.
    ///
    /// # Errors
    /// Returns [`OrderError::StructureIdExhausted`] when `counter` has no
    /// fresh id to issue, or [`OrderError::CapacityExhausted`] when the
    /// clamped universe size is not representable.
    #[inline]
    fn with_label_bits_from_counter(
        label_bits: LabelBits,
        counter: &StructureIdCounter,
    ) -> Result<Self, OrderError>
    {
        let structure_id = counter.allocate()?;
        return Self::with_label_bits_and_structure_id(label_bits, structure_id);
    }

    /// Creates an empty structure over the clamped `[0, 2^label_bits)` label
    /// universe, stamped with the pre-minted `structure_id`.
    ///
    /// # Errors
    /// Returns [`OrderError::CapacityExhausted`] when the universe size
    /// `2^label_bits` is not representable as a `u64` (unreachable while
    /// [`LabelBits::clamped`] caps the width at [`LabelBits::MAX`]).
    #[inline]
    fn with_label_bits_and_structure_id(
        label_bits: LabelBits,
        structure_id: StructureId,
    ) -> Result<Self, OrderError>
    {
        let bits = label_bits.clamped();
        let capacity = 1u64
            .checked_shl(bits.0)
            .map(LabelCapacity)
            .ok_or(OrderError::CapacityExhausted)?;
        return Ok(Self {
            slots: Vec::new(),
            free_head: None,
            #[cfg(test)]
            slot_limit: None,
            head: None,
            tail: None,
            len: LiveLen::ZERO,
            label_bits: bits,
            capacity,
            structure_id,
        });
    }

    #[cfg(test)]
    #[inline]
    fn with_label_bits_and_slot_limit(
        label_bits: LabelBits,
        slot_limit: SlotLimit,
    ) -> Result<Self, OrderError>
    {
        let mut order = Self::with_label_bits(label_bits)?;
        order.slot_limit = Some(slot_limit);
        return Ok(order);
    }

    /// The number of live elements.
    #[inline]
    #[must_use]
    pub fn len(&self) -> LiveLen
    {
        return self.len;
    }

    /// Whether the structure has no live elements.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> OrderIsEmpty
    {
        return OrderIsEmpty(self.len == LiveLen::ZERO);
    }

    /// The first element in order, if any.
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<Pos>
    {
        let index = self.head?;
        return self.pos_at(index);
    }

    /// The last element in order, if any.
    #[inline]
    #[must_use]
    pub fn last(&self) -> Option<Pos>
    {
        let index = self.tail?;
        return self.pos_at(index);
    }

    /// Whether `pos` refers to a live element of this structure.
    #[inline]
    #[must_use]
    pub fn contains(
        &self,
        pos: Pos,
    ) -> HandleMembership
    {
        return HandleMembership(self.resolve(pos).is_some());
    }

    /// The payload of `pos`, or `None` if the handle is stale or foreign.
    ///
    /// # Contract
    /// - ensures: returns `Some(&value)` iff `pos` refers to a live element.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        pos: Pos,
    ) -> Option<&T>
    {
        return self.resolve(pos).map(|occupied| &occupied.value);
    }

    /// Compares two elements in the order, in O(1).
    ///
    /// # Contract
    /// - requires: `left` and `right` are handles to this structure.
    /// - ensures: returns `Some(ordering)` where `ordering` is the relative
    ///   order of the two elements, or `Some(Equal)` iff the handles refer to
    ///   the same live element; distinct live elements never compare `Equal`.
    /// - fails: returns `None` if either handle is stale or foreign.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn cmp(
        &self,
        left: Pos,
        right: Pos,
    ) -> Option<Ordering>
    {
        let left_occupied = self.resolve(left)?;
        let right_occupied = self.resolve(right)?;
        let left_label = left_occupied.label;
        let right_label = right_occupied.label;
        return Some(left_label.cmp(&right_label));
    }

    /// The element immediately after `pos` in order, or `None` at the end (or
    /// if the handle is stale).
    #[inline]
    #[must_use]
    pub fn next(
        &self,
        pos: Pos,
    ) -> Option<Pos>
    {
        let occupied = self.resolve(pos)?;
        let next_index = occupied.next?;
        return self.pos_at(next_index);
    }

    /// The element immediately before `pos` in order, or `None` at the start
    /// (or if the handle is stale).
    #[inline]
    #[must_use]
    pub fn prev(
        &self,
        pos: Pos,
    ) -> Option<Pos>
    {
        let occupied = self.resolve(pos)?;
        let prev_index = occupied.prev?;
        return self.pos_at(prev_index);
    }

    /// Iterates `(handle, &payload)` over every element in order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (Pos, &T)>
    {
        let mut cursor = self.head;
        return core::iter::from_fn(move || {
            let index = cursor?;
            let occupied = self.occupied(index)?;
            cursor = occupied.next;
            Some((
                Pos {
                    structure_id: self.structure_id,
                    index,
                    generation: occupied.generation,
                },
                &occupied.value,
            ))
        });
    }

    /// Inserts `value` as the first element in order.
    ///
    /// # Contract
    /// - ensures: on `Ok`, `value` is the new first element and the returned
    ///   handle refers to it.
    /// - fails: [`OrderError::CapacityExhausted`] if no element can be
    ///   admitted.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`OrderError::CapacityExhausted`] when the structure is full.
    #[inline]
    pub fn push_front(
        &mut self,
        value: T,
    ) -> Result<Pos, OrderError>
    {
        match self.head {
            | Some(head_index) => self.insert_between(None, Some(head_index), value),
            | None => self.insert_between(None, None, value),
        }
    }

    /// Inserts `value` as the last element in order.
    ///
    /// # Contract
    /// - ensures: on `Ok`, `value` is the new last element and the returned
    ///   handle refers to it.
    /// - fails: [`OrderError::CapacityExhausted`] if no element can be
    ///   admitted.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`OrderError::CapacityExhausted`] when the structure is full.
    #[inline]
    pub fn push_back(
        &mut self,
        value: T,
    ) -> Result<Pos, OrderError>
    {
        match self.tail {
            | Some(tail_index) => self.insert_between(Some(tail_index), None, value),
            | None => self.insert_between(None, None, value),
        }
    }

    /// Inserts `value` immediately after `pos` in order.
    ///
    /// # Contract
    /// - requires: `pos` is a live handle to this structure.
    /// - ensures: on `Ok`, `value` sits immediately after `pos` and before
    ///   whatever previously followed `pos`; the returned handle refers to it.
    /// - fails: [`OrderError::UnknownPosition`] if `pos` is stale or foreign;
    ///   [`OrderError::CapacityExhausted`] if no element can be admitted.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`OrderError::UnknownPosition`] for a stale handle and
    /// [`OrderError::CapacityExhausted`] when the structure is full.
    #[inline]
    pub fn insert_after(
        &mut self,
        pos: Pos,
        value: T,
    ) -> Result<Pos, OrderError>
    {
        let occupied = self.resolve(pos).ok_or(OrderError::UnknownPosition)?;
        let next_index = occupied.next;
        return self.insert_between(Some(pos.index), next_index, value);
    }

    /// Inserts `value` immediately before `pos` in order.
    ///
    /// # Contract
    /// - requires: `pos` is a live handle to this structure.
    /// - ensures: on `Ok`, `value` sits immediately before `pos` and after
    ///   whatever previously preceded `pos`; the returned handle refers to it.
    /// - fails: [`OrderError::UnknownPosition`] if `pos` is stale or foreign;
    ///   [`OrderError::CapacityExhausted`] if no element can be admitted.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`OrderError::UnknownPosition`] for a stale handle and
    /// [`OrderError::CapacityExhausted`] when the structure is full.
    #[inline]
    pub fn insert_before(
        &mut self,
        pos: Pos,
        value: T,
    ) -> Result<Pos, OrderError>
    {
        let occupied = self.resolve(pos).ok_or(OrderError::UnknownPosition)?;
        let prev_index = occupied.prev;
        return self.insert_between(prev_index, Some(pos.index), value);
    }

    /// Removes the element `pos` refers to, returning its payload.
    ///
    /// # Contract
    /// - ensures: on `Some`, the element is unlinked, its slot is freed for
    ///   reuse, and `pos` (and any copy of it) is thereafter stale.
    /// - fails: returns `None` if `pos` is stale or foreign.
    /// - panics: none.
    #[inline]
    pub fn remove(
        &mut self,
        pos: Pos,
    ) -> Option<T>
    {
        let occupied = self.resolve(pos)?;
        let prev_index = occupied.prev;
        let next_index = occupied.next;
        let generation = occupied.generation;
        // Unlink from the neighbours and the head/tail before freeing the slot.
        match prev_index {
            | Some(prev) => self.set_next(prev, next_index),
            | None => self.head = next_index,
        }
        match next_index {
            | Some(next) => self.set_prev(next, prev_index),
            | None => self.tail = prev_index,
        }
        let slot_index = usize::try_from(pos.index).ok()?;
        let slot = self.slots.get_mut(slot_index)?;
        let next_generation = generation.checked_add(1);
        let replacement = match next_generation {
            | Some(free_generation) => Slot::Free(Free {
                generation: free_generation,
                next_free: self.free_head,
            }),
            | None => Slot::Retired,
        };
        let freed = core::mem::replace(slot, replacement);
        if next_generation.is_some() {
            self.free_head = Some(pos.index);
        }
        let next_len = self.len.0.checked_sub(1)?;
        self.len = LiveLen(next_len);
        match freed {
            | Slot::Occupied(occupied_slot) => Some(occupied_slot.value),
            // Unreachable: `resolve` proved the slot occupied above.
            | Slot::Free(_) | Slot::Retired => None,
        }
    }

    /// Tests whether `outer` contains `inner` as pre/post-order intervals: a
    /// tree node whose interval is `[lo, hi]` contains another iff its `lo` is
    /// no later and its `hi` is no earlier.
    ///
    /// # Contract
    /// - requires: all four endpoints are live handles to this structure, and
    ///   each interval's `lo` is no later than its `hi`.
    /// - ensures: returns `Some` wrapping containment iff `outer.lo ⊑ inner.lo`
    ///   and `inner.hi ⊑ outer.hi` in the order.
    /// - fails: returns `None` if any endpoint is stale or foreign.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn interval_contains(
        &self,
        outer: crate::interval::Interval,
        inner: crate::interval::Interval,
    ) -> Option<IntervalContainment>
    {
        let lo_ordering = self.cmp(outer.lo, inner.lo)?;
        let hi_ordering = self.cmp(inner.hi, outer.hi)?;
        let lo_ok = lo_ordering.is_le();
        let hi_ok = hi_ordering.is_le();
        return Some(IntervalContainment(lo_ok && hi_ok));
    }

    // ----- internal helpers ------------------------------------------------

    /// The handle for slot `index`, if the slot is occupied.
    #[inline]
    fn pos_at(
        &self,
        index: SlotIndex,
    ) -> Option<Pos>
    {
        let occupied = self.occupied(index)?;
        return Some(Pos {
            structure_id: self.structure_id,
            index,
            generation: occupied.generation,
        });
    }

    /// The slot at `index`, if any.
    #[inline]
    fn slot(
        &self,
        index: SlotIndex,
    ) -> Option<&Slot<T>>
    {
        let position = usize::try_from(index).ok()?;
        return self.slots.get(position);
    }

    /// The occupied payload at `index`, if the slot exists and is occupied.
    #[inline]
    fn occupied(
        &self,
        index: SlotIndex,
    ) -> Option<&Occupied<T>>
    {
        match *self.slot(index)? {
            | Slot::Occupied(ref occupied) => Some(occupied),
            | Slot::Free(_) | Slot::Retired => None,
        }
    }

    /// The mutable occupied payload at `index`, if the slot exists and is
    /// occupied.
    #[inline]
    fn occupied_mut(
        &mut self,
        index: SlotIndex,
    ) -> Option<&mut Occupied<T>>
    {
        let position = usize::try_from(index).ok()?;
        let slot = self.slots.get_mut(position)?;
        match *slot {
            | Slot::Occupied(ref mut occupied) => Some(occupied),
            | Slot::Free(_) | Slot::Retired => None,
        }
    }

    /// The occupied payload `pos` refers to, if the handle is live (slot
    /// occupied and generation matching).
    #[inline]
    fn resolve(
        &self,
        pos: Pos,
    ) -> Option<&Occupied<T>>
    {
        if pos.structure_id != self.structure_id {
            return None;
        }
        let occupied = self.occupied(pos.index)?;
        return (occupied.generation == pos.generation).then_some(occupied);
    }

    /// The label of the occupied slot at `index`, if any.
    #[inline]
    fn label_of(
        &self,
        index: SlotIndex,
    ) -> Option<Label>
    {
        return self.occupied(index).map(|occupied| occupied.label);
    }

    /// Sets the `prev` link of the occupied slot at `index` (no-op if absent).
    #[inline]
    fn set_prev(
        &mut self,
        index: SlotIndex,
        prev: Option<SlotIndex>,
    )
    {
        if let Some(occupied) = self.occupied_mut(index) {
            occupied.prev = prev;
        }
    }

    /// Sets the `next` link of the occupied slot at `index` (no-op if absent).
    #[inline]
    fn set_next(
        &mut self,
        index: SlotIndex,
        next: Option<SlotIndex>,
    )
    {
        if let Some(occupied) = self.occupied_mut(index) {
            occupied.next = next;
        }
    }

    /// Sets the `label` of the occupied slot at `index` (no-op if absent).
    #[inline]
    fn set_label(
        &mut self,
        index: SlotIndex,
        label: Label,
    )
    {
        if let Some(occupied) = self.occupied_mut(index) {
            occupied.label = label;
        }
    }

    /// Allocates a slot holding `value` with the given label and links,
    /// reusing a freed slot when possible.
    ///
    /// The slot's order links and the structure's head/tail are *not* patched
    /// here — the caller wires them. Does not change [`Self::len`] callers'
    /// neighbours; it increments `len` for the new element.
    ///
    /// # Errors
    /// Returns [`OrderError::CapacityExhausted`] when no `u32` slot index is
    /// available.
    #[inline]
    fn alloc(
        &mut self,
        label: Label,
        prev: Option<SlotIndex>,
        next: Option<SlotIndex>,
        value: T,
    ) -> Result<Pos, OrderError>
    {
        let next_len = self
            .len
            .0
            .checked_add(1)
            .map(LiveLen)
            .ok_or(OrderError::CapacityExhausted)?;
        if let Some(free_index) = self.free_head {
            let free_position =
                usize::try_from(free_index).map_err(|_ignored| OrderError::CapacityExhausted)?;
            let slot = self
                .slots
                .get_mut(free_position)
                .ok_or(OrderError::CapacityExhausted)?;
            let generation = match *slot {
                | Slot::Free(ref free) => free.generation,
                // Unreachable: the free list only threads freed slots.
                | Slot::Occupied(_) | Slot::Retired => return Err(OrderError::CapacityExhausted),
            };
            let next_free = match *slot {
                | Slot::Free(ref free) => free.next_free,
                | Slot::Occupied(_) | Slot::Retired => None,
            };
            *slot = Slot::Occupied(Occupied {
                generation,
                label,
                prev,
                next,
                value,
            });
            self.free_head = next_free;
            self.len = next_len;
            return Ok(Pos {
                structure_id: self.structure_id,
                index: free_index,
                generation,
            });
        }
        #[cfg(test)]
        if self
            .slot_limit
            .is_some_and(|limit| self.slots.len() >= limit.0)
        {
            return Err(OrderError::CapacityExhausted);
        }
        let index = SlotIndex::try_from(self.slots.len())
            .map_err(|_ignored| OrderError::CapacityExhausted)?;
        self.slots.push(Slot::Occupied(Occupied {
            generation: 0,
            label,
            prev,
            next,
            value,
        }));
        self.len = next_len;
        return Ok(Pos {
            structure_id: self.structure_id,
            index,
            generation: 0,
        });
    }

    /// Inserts `value` between the adjacent elements `prev_index` and
    /// `next_index` (either may be `None` at a list end; both `None` only for
    /// an empty structure).
    ///
    /// Takes the midpoint of the neighbours' label gap when one exists,
    /// otherwise relabels a minimal aligned window to open room (see
    /// [`Self::relabel_insert`]).
    ///
    /// # Errors
    /// Returns [`OrderError::CapacityExhausted`] when no element can be
    /// admitted.
    #[inline]
    fn insert_between(
        &mut self,
        prev_index: Option<SlotIndex>,
        next_index: Option<SlotIndex>,
        value: T,
    ) -> Result<Pos, OrderError>
    {
        // The lowest admissible label is just above the predecessor (or `0`);
        // the exclusive upper bound is the successor's label (or the capacity).
        let mut lower = Label(0);
        if let Some(prev) = prev_index {
            let prev_label = self.label_of(prev).ok_or(OrderError::CapacityExhausted)?;
            let next_label = prev_label
                .0
                .checked_add(1)
                .ok_or(OrderError::CapacityExhausted)?;
            lower = Label(next_label);
        }
        let mut upper = Label(self.capacity.0);
        if let Some(next) = next_index {
            upper = self.label_of(next).ok_or(OrderError::CapacityExhausted)?;
        }
        if lower < upper {
            let label = Label(u64::midpoint(lower.0, upper.0));
            return self.link_new(prev_index, next_index, label, value);
        }
        return self.relabel_insert(prev_index, next_index, value);
    }

    /// Allocates `value` with `label` and wires it between `prev_index` and
    /// `next_index`, updating the neighbours' links and the head/tail.
    ///
    /// # Errors
    /// Returns [`OrderError::CapacityExhausted`] when no slot is available.
    #[inline]
    fn link_new(
        &mut self,
        prev_index: Option<SlotIndex>,
        next_index: Option<SlotIndex>,
        label: Label,
        value: T,
    ) -> Result<Pos, OrderError>
    {
        let pos = self.alloc(label, prev_index, next_index, value)?;
        match prev_index {
            | Some(prev) => self.set_next(prev, Some(pos.index)),
            | None => self.head = Some(pos.index),
        }
        match next_index {
            | Some(next) => self.set_prev(next, Some(pos.index)),
            | None => self.tail = Some(pos.index),
        }
        return Ok(pos);
    }

    /// Opens room for a new element between `prev_index` and `next_index` by
    /// relabeling the smallest power-of-two-aligned label window around the
    /// insertion point that is at most half full, then inserting `value` into
    /// the evenly-redistributed window.
    ///
    /// At least one neighbour is `Some` here (the empty and gap-available cases
    /// are handled by [`Self::insert_between`]). The density cap guarantees the
    /// whole-universe window (`h == label_bits`) is sparse enough, so the
    /// search always terminates with room.
    ///
    /// # Errors
    /// Returns [`OrderError::CapacityExhausted`] when even the whole-universe
    /// window cannot accommodate the new element (the structure is at the
    /// density cap), or when no slot is available.
    #[inline]
    fn relabel_insert(
        &mut self,
        prev_index: Option<SlotIndex>,
        next_index: Option<SlotIndex>,
        value: T,
    ) -> Result<Pos, OrderError>
    {
        let anchor = prev_index
            .or(next_index)
            .ok_or(OrderError::CapacityExhausted)?;
        let anchor_label = self.label_of(anchor).ok_or(OrderError::CapacityExhausted)?;
        let mut height = LabelBits::MIN;
        while height <= self.label_bits {
            let range_size_value = 1u64
                .checked_shl(height.0)
                .ok_or(OrderError::CapacityExhausted)?;
            let range_size = LabelRangeSize(range_size_value);
            // The aligned window `[start, start + range_size)` containing the
            // anchor: clear the low `height` bits of the anchor's label.
            let start_value = anchor_label
                .0
                .checked_shr(height.0)
                .and_then(|shifted| shifted.checked_shl(height.0))
                .ok_or(OrderError::CapacityExhausted)?;
            let start = Label(start_value);
            let end_value = start
                .0
                .checked_add(range_size.0)
                .ok_or(OrderError::CapacityExhausted)?;
            let end = Label(end_value);
            let window = self.collect_window(anchor, start, end)?;
            // The new element plus the window's existing elements must fit at
            // density ≤ 1/2 so the even redistribution leaves gaps: `2·m ≤
            // range_size`.
            let occupants = window
                .len()
                .checked_add(1)
                .map(OccupantCount)
                .ok_or(OrderError::CapacityExhausted)?;
            let fits = u64::try_from(occupants.0)
                .ok()
                .and_then(|count| count.checked_mul(2))
                .is_some_and(|doubled| doubled <= range_size.0);
            if fits {
                return self.redistribute(&window, prev_index, start, range_size, occupants, value);
            }
            let next_height = height
                .0
                .checked_add(1)
                .ok_or(OrderError::CapacityExhausted)?;
            height = LabelBits(next_height);
        }
        return Err(OrderError::CapacityExhausted);
    }

    /// Collects, in list order, every element whose label lies in
    /// `[start, end)` around `anchor` (whose label is in the range). The
    /// result is a contiguous list segment because labels increase in list
    /// order.
    ///
    /// # Errors
    /// Returns [`OrderError::CapacityExhausted`] only on an internally
    /// inconsistent structure (an occupied link pointing at a non-occupied
    /// slot), which the construction never produces.
    #[inline]
    fn collect_window(
        &self,
        anchor: SlotIndex,
        start: Label,
        end: Label,
    ) -> Result<Vec<SlotIndex>, OrderError>
    {
        // Walk left to the first in-range element.
        let mut leftmost = anchor;
        loop {
            let occupied = self
                .occupied(leftmost)
                .ok_or(OrderError::CapacityExhausted)?;
            match occupied.prev {
                | Some(prev)
                    if self.label_of(prev).ok_or(OrderError::CapacityExhausted)? >= start =>
                {
                    leftmost = prev;
                },
                | _ => break,
            }
        }
        // Walk right from the leftmost, collecting while in range.
        let mut window: Vec<SlotIndex> = Vec::new();
        let mut cursor = Some(leftmost);
        while let Some(index) = cursor {
            let occupied = self.occupied(index).ok_or(OrderError::CapacityExhausted)?;
            if occupied.label >= end {
                break;
            }
            window.push(index);
            cursor = occupied.next;
        }
        return Ok(window);
    }

    /// Redistributes the `window` elements (plus the new `value`) evenly across
    /// the aligned label range `[start, start + range_size)` and wires the
    /// resulting contiguous segment back into the list.
    ///
    /// `occupants == window.len() + 1` and `2 · occupants ≤ range_size` (the
    /// caller's density check), so the assigned labels are distinct, strictly
    /// increasing, and strictly inside the range — preserving the global order
    /// against the un-relabeled neighbours just outside it.
    ///
    /// # Errors
    /// Returns [`OrderError::CapacityExhausted`] when no slot is available for
    /// the new element.
    #[inline]
    fn redistribute(
        &mut self,
        window: &[SlotIndex],
        prev_index: Option<SlotIndex>,
        start: Label,
        range_size: LabelRangeSize,
        occupants: OccupantCount,
        value: T,
    ) -> Result<Pos, OrderError>
    {
        // The new element's position within the rebuilt sequence: immediately
        // after its predecessor when that predecessor is in the window (it
        // always is — it is the anchor), otherwise at the front.
        let insert_at = match prev_index {
            | Some(prev) => window
                .iter()
                .position(|&index| index == prev)
                .and_then(|position| position.checked_add(1))
                .unwrap_or(window.len()),
            | None => 0,
        };
        // The records just outside the window keep their labels and bound the
        // rebuilt segment.
        let left_outer = window
            .first()
            .and_then(|&first| self.occupied(first))
            .and_then(|occupied| occupied.prev);
        let right_outer = window
            .last()
            .and_then(|&last| self.occupied(last))
            .and_then(|occupied| occupied.next);
        // Allocate the new element (label and links are set below).
        let new_pos = self.alloc(start, None, None, value)?;
        // Build the rebuilt ordered sequence: the window with the new element
        // spliced in at `insert_at`.
        let mut sequence: Vec<SlotIndex> = Vec::with_capacity(occupants.0);
        for slot_position in 0 .. occupants.0 {
            if slot_position == insert_at {
                sequence.push(new_pos.index);
            }
            else {
                // Window indices skip the inserted slot: positions before
                // `insert_at` map directly, positions after shift down by one.
                let window_position = if slot_position < insert_at {
                    slot_position
                }
                else {
                    slot_position
                        .checked_sub(1)
                        .ok_or(OrderError::CapacityExhausted)?
                };
                match window.get(window_position) {
                    | Some(&index) => sequence.push(index),
                    | None => sequence.push(new_pos.index),
                }
            }
        }
        // Assign evenly-spaced labels and re-link the segment.
        self.assign_labels(&sequence, start, range_size, occupants);
        self.relink_segment(&sequence, left_outer, right_outer);
        return Ok(new_pos);
    }

    /// Assigns evenly-spaced labels in `(start, start + range_size)` to the
    /// `sequence`, in order.
    ///
    /// The `i`-th element gets `start + ((i + 1) · range_size) / (occupants +
    /// 1)`. With `2 · occupants ≤ range_size` the step exceeds `1`, so labels
    /// are distinct, strictly increasing, and strictly inside the range.
    #[inline]
    fn assign_labels(
        &mut self,
        sequence: &[SlotIndex],
        start: Label,
        range_size: LabelRangeSize,
        occupants: OccupantCount,
    )
    {
        for (position, &index) in sequence.iter().enumerate() {
            if let Some(label) =
                Self::spread_label(start, range_size, occupants, SequencePosition(position))
            {
                self.set_label(index, label);
            }
        }
    }

    /// The evenly-spaced label for sequence position `position` of `occupants`
    /// within `[start, start + range_size)`.
    ///
    /// # Contract
    /// - requires: `position < occupants` and `2 · occupants ≤ range_size`.
    /// - ensures: returns `start + ((position + 1) · range_size) / (occupants +
    ///   1)`, which lies strictly inside the range.
    /// - fails: returns `None` only on an arithmetic conversion that the
    ///   contract precludes.
    /// - panics: none.
    #[inline]
    fn spread_label(
        start: Label,
        range_size: LabelRangeSize,
        occupants: OccupantCount,
        position: SequencePosition,
    ) -> Option<Label>
    {
        let numerator_position = position.0.checked_add(1)?;
        let numerator_index = u128::try_from(numerator_position).ok()?;
        let divisor_occupants = occupants.0.checked_add(1)?;
        let divisor = u128::try_from(divisor_occupants).ok()?;
        let scaled_offset = numerator_index.checked_mul(u128::from(range_size.0))?;
        let offset_wide = scaled_offset.checked_div(divisor)?;
        let offset = u64::try_from(offset_wide).ok()?;
        return start.0.checked_add(offset).map(Label);
    }

    /// Re-links the `sequence` into a contiguous list segment bounded by the
    /// records `left_outer` (before it) and `right_outer` (after it), updating
    /// the structure's head/tail when a bound is absent.
    #[inline]
    fn relink_segment(
        &mut self,
        sequence: &[SlotIndex],
        left_outer: Option<SlotIndex>,
        right_outer: Option<SlotIndex>,
    )
    {
        for (position, &index) in sequence.iter().enumerate() {
            let prev_link = match position.checked_sub(1) {
                | Some(before) => sequence.get(before).copied(),
                | None => left_outer,
            };
            let successor = position
                .checked_add(1)
                .and_then(|after| sequence.get(after))
                .copied();
            let next_link = match successor {
                | Some(after) => Some(after),
                | None => right_outer,
            };
            self.set_prev(index, prev_link);
            self.set_next(index, next_link);
        }
        match (left_outer, sequence.first()) {
            | (Some(outer), Some(&first)) => self.set_next(outer, Some(first)),
            | (None, Some(&first)) => self.head = Some(first),
            | (_, None) => {},
        }
        match (right_outer, sequence.last()) {
            | (Some(outer), Some(&last)) => self.set_prev(outer, Some(last)),
            | (None, Some(&last)) => self.tail = Some(last),
            | (_, None) => {},
        }
    }
}

#[cfg(test)]
mod tests
{
    use alloc::vec;
    use alloc::vec::Vec;
    use core::cmp::Ordering;

    use super::LabelBits;
    use super::OrderError;
    use super::OrderMaintenance;
    use super::Pos;
    use super::SlotLimit;
    use super::StructureIdCounter;
    use crate::interval::Interval;

    mod support
    {
        use super::*;

        pub(super) fn new_order<T>() -> OrderMaintenance<T>
        {
            return OrderMaintenance::new()
                .expect("structure id allocation succeeds in focused tests");
        }

        pub(super) fn narrow_order<T>(label_bits: LabelBits) -> OrderMaintenance<T>
        {
            return OrderMaintenance::with_label_bits(label_bits)
                .expect("structure id allocation succeeds in focused tests");
        }

        /// Asserts the structural invariant: labels strictly increase along the
        /// list, the forward and backward link chains agree on length, and
        /// head/tail bound the chain.
        pub(super) fn assert_invariant<T>(order: &OrderMaintenance<T>)
        {
            let mut count: usize = 0;
            let mut previous_label = None;
            let mut cursor = order.head;
            let mut last_seen = None;
            while let Some(index) = cursor {
                let occupied = order.occupied(index).expect("listed slot is occupied");
                if let Some(label) = previous_label {
                    assert!(
                        occupied.label > label,
                        "labels strictly increase in list order"
                    );
                }
                previous_label = Some(occupied.label);
                last_seen = Some(index);
                count = count.saturating_add(1);
                cursor = occupied.next;
            }
            assert_eq!(count, usize::from(order.len), "forward length matches len");
            assert_eq!(last_seen, order.tail, "tail is the last listed element");

            let mut back_count: usize = 0;
            let mut back_cursor = order.tail;
            let mut first_seen = None;
            while let Some(index) = back_cursor {
                let occupied = order.occupied(index).expect("listed slot is occupied");
                first_seen = Some(index);
                back_count = back_count.saturating_add(1);
                back_cursor = occupied.prev;
            }
            assert_eq!(
                back_count,
                usize::from(order.len),
                "backward length matches len"
            );
            assert_eq!(first_seen, order.head, "head is the first listed element");
        }

        /// Asserts O(1) comparison agrees with list order for every pair, and
        /// that each element compares equal only to itself.
        pub(super) fn assert_cmp_consistent<T>(order: &OrderMaintenance<T>)
        {
            let handles = positions(order);
            for (left_rank, &left) in handles.iter().enumerate() {
                for (right_rank, &right) in handles.iter().enumerate() {
                    assert_eq!(
                        Some(left_rank.cmp(&right_rank)),
                        order.cmp(left, right),
                        "cmp matches list order for every pair"
                    );
                }
            }
        }

        /// The handles in list order.
        pub(super) fn positions<T>(order: &OrderMaintenance<T>) -> Vec<Pos>
        {
            order.iter().map(|(pos, _value)| pos).collect()
        }

        /// The payloads in list order.
        pub(super) fn ordered<T>(order: &OrderMaintenance<T>) -> Vec<T>
        where
            T: Copy,
        {
            order.iter().map(|(_, &value)| value).collect()
        }
    }

    use support::*;

    #[test]
    fn new_is_empty()
    {
        let order: OrderMaintenance<u64> = new_order();
        assert!(bool::from(order.is_empty()), "a fresh structure is empty");
        assert_eq!(
            0,
            usize::from(order.len()),
            "a fresh structure has no elements"
        );
        assert_eq!(None, order.first(), "no first element");
        assert_eq!(None, order.last(), "no last element");
        assert_eq!(Vec::<u64>::new(), ordered(&order), "iteration is empty");
    }
    #[test]
    fn push_back_preserves_order()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        for value in 0 .. 6 {
            order.push_back(value).expect("push_back succeeds");
        }
        assert_eq!(
            vec![0, 1, 2, 3, 4, 5],
            ordered(&order),
            "push_back appends in order"
        );
        assert_eq!(6, usize::from(order.len()), "length tracks insertions");
        assert_invariant(&order);
        assert_cmp_consistent(&order);
    }
    #[test]
    fn push_front_reverses()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        for value in 0 .. 6 {
            order.push_front(value).expect("push_front succeeds");
        }
        assert_eq!(
            vec![5, 4, 3, 2, 1, 0],
            ordered(&order),
            "push_front prepends"
        );
        assert_invariant(&order);
        assert_cmp_consistent(&order);
    }
    #[test]
    fn insert_after_and_before_place_correctly()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        let first = order.push_back(0).expect("push_back succeeds");
        let last = order.push_back(9).expect("push_back succeeds");
        let middle = order.insert_after(first, 5).expect("insert_after succeeds");
        order
            .insert_before(middle, 3)
            .expect("insert_before succeeds");
        order
            .insert_before(last, 7)
            .expect("insert_before succeeds");
        assert_eq!(
            vec![0, 3, 5, 7, 9],
            ordered(&order),
            "inserts land at the right spots"
        );
        assert_invariant(&order);
        assert_cmp_consistent(&order);
    }
    #[test]
    fn remove_middle_unlinks_and_invalidates()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        order.push_back(0).expect("push_back succeeds");
        let middle = order.push_back(1).expect("push_back succeeds");
        order.push_back(2).expect("push_back succeeds");
        assert_eq!(Some(1), order.remove(middle), "remove returns the payload");
        assert_eq!(vec![0, 2], ordered(&order), "the element is unlinked");
        assert!(
            !bool::from(order.contains(middle)),
            "the removed handle is stale"
        );
        assert_eq!(None, order.get(middle), "get on a stale handle is None");
        assert_eq!(None, order.remove(middle), "double remove is None");
        assert_invariant(&order);
        assert_cmp_consistent(&order);
    }
    #[test]
    fn remove_head_and_tail()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        let head = order.push_back(0).expect("push_back succeeds");
        order.push_back(1).expect("push_back succeeds");
        let tail = order.push_back(2).expect("push_back succeeds");
        assert_eq!(Some(0), order.remove(head), "remove head");
        assert_eq!(Some(2), order.remove(tail), "remove tail");
        assert_eq!(vec![1], ordered(&order), "only the middle survives");
        assert_invariant(&order);
    }
    #[test]
    fn remove_only_element_empties()
    {
        const SOLE_ELEMENT_VALUE: u64 = 42;

        let mut order: OrderMaintenance<u64> = new_order();
        let only = order
            .push_back(SOLE_ELEMENT_VALUE)
            .expect("push_back succeeds");
        assert_eq!(
            Some(SOLE_ELEMENT_VALUE),
            order.remove(only),
            "remove the sole element"
        );
        assert!(bool::from(order.is_empty()), "structure is empty again");
        order.push_back(7).expect("reuse after emptying");
        assert_eq!(vec![7], ordered(&order), "can insert after emptying");
    }
    #[test]
    fn relabel_after_anchor_preserves_order()
    {
        const RELABEL_SENTINEL_VALUE: u64 = 100;

        // A narrow universe forces relabeling after only a few insertions; the
        // repeated insert-after-the-same-anchor stacks elements in reverse.
        let mut order: OrderMaintenance<u64> = narrow_order(LabelBits(4));
        let anchor = order.push_back(0).expect("push_back succeeds");

        order
            .push_back(RELABEL_SENTINEL_VALUE)
            .expect("push_back succeeds");
        for value in 1 ..= 6 {
            order
                .insert_after(anchor, value)
                .expect("insert_after succeeds under relabel");
            assert_invariant(&order);
            assert_cmp_consistent(&order);
        }
        assert_eq!(
            vec![0, 6, 5, 4, 3, 2, 1, 100],
            ordered(&order),
            "relabeling preserves the list order"
        );
    }
    #[test]
    fn relabel_at_front_preserves_order()
    {
        // Exhaust the front gap so a push_front must relabel with no
        // predecessor (the `prev_index == None` relabel branch).
        let mut order: OrderMaintenance<u64> = narrow_order(LabelBits(4));
        for value in 0 .. 7 {
            order
                .push_front(value)
                .expect("push_front succeeds under relabel");
            assert_invariant(&order);
            assert_cmp_consistent(&order);
        }
        assert_eq!(
            vec![6, 5, 4, 3, 2, 1, 0],
            ordered(&order),
            "front relabeling preserves order"
        );
    }
    #[test]
    fn capacity_exhausts_in_a_tiny_universe()
    {
        // With two label bits the universe is `{0, 1, 2, 3}`; a third
        // append has no gap and the whole-universe window cannot fit three
        // elements at density 1/2, so the relabel reports exhaustion.
        let mut order: OrderMaintenance<u64> = narrow_order(LabelBits(2));
        order.push_back(0).expect("first append fits");
        order.push_back(1).expect("second append fits");
        assert_eq!(
            Err(OrderError::CapacityExhausted),
            order.push_back(2),
            "a relabel that cannot reach density 1/2 reports exhaustion"
        );
        // The structure is still well-formed and usable after the rejection.
        assert_eq!(
            vec![0, 1],
            ordered(&order),
            "the rejected element left no trace"
        );
        assert_invariant(&order);
    }

    #[test]
    fn structure_id_exhaustion_is_typed()
    {
        let counter = StructureIdCounter::nearly_exhausted();
        let mut first: OrderMaintenance<u64> =
            OrderMaintenance::with_label_bits_from_counter(LabelBits(4), &counter)
                .expect("the last non-sentinel structure id is available");
        let old = first.push_back(1).expect("insertion succeeds");

        let exhausted =
            OrderMaintenance::<u64>::with_label_bits_from_counter(LabelBits(4), &counter);

        assert!(
            matches!(exhausted, Err(OrderError::StructureIdExhausted)),
            "structure id exhaustion is typed"
        );
        assert_eq!(
            Some(&1),
            first.get(old),
            "an extant position is not aliased by a wrapped structure id"
        );
    }

    #[test]
    fn exhausted_generation_retires_slot_and_allocates_another()
    {
        let mut order: OrderMaintenance<u64> = narrow_order(LabelBits(4));
        let first = order.push_back(0).expect("initial insert succeeds");
        let exhausted = Pos {
            generation: u32::MAX,
            ..first
        };
        order
            .occupied_mut(first.index)
            .expect("slot is occupied")
            .generation = u32::MAX;

        assert_eq!(
            Some(0),
            order.remove(exhausted),
            "remove the exhausted slot"
        );
        assert!(
            matches!(order.slot(exhausted.index), Some(super::Slot::Retired)),
            "the exhausted slot is retired permanently"
        );
        let replacement = order.push_back(1).expect("a fresh slot is allocated");
        assert_ne!(
            exhausted.index, replacement.index,
            "the retired slot index is not reused"
        );
        assert_eq!(
            None,
            order.get(exhausted),
            "the stale exhausted-generation handle does not alias the replacement"
        );
        assert_eq!(Some(&1), order.get(replacement), "the replacement resolves");
    }

    #[test]
    fn retired_slot_capacity_exhaustion_is_typed()
    {
        let mut order: OrderMaintenance<u64> =
            OrderMaintenance::with_label_bits_and_slot_limit(LabelBits(4), SlotLimit(1))
                .expect("bounded test structure can be constructed");
        let first = order
            .push_back(0)
            .expect("initial insert consumes the slot");
        let exhausted = Pos {
            generation: u32::MAX,
            ..first
        };
        order
            .occupied_mut(first.index)
            .expect("slot is occupied")
            .generation = u32::MAX;

        assert_eq!(Some(0), order.remove(exhausted), "retire the only slot");
        assert_eq!(
            Err(OrderError::CapacityExhausted),
            order.push_back(1),
            "when every representable slot is retired, insertion reports typed exhaustion"
        );
        assert_eq!(
            None,
            order.get(exhausted),
            "the retired slot remains unavailable to stale handles"
        );
    }

    #[test]
    fn navigation_walks_both_ways()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        let first = order.push_back(0).expect("push_back succeeds");
        let second = order.push_back(1).expect("push_back succeeds");
        let third = order.push_back(2).expect("push_back succeeds");
        assert_eq!(Some(second), order.next(first), "next walks forward");
        assert_eq!(None, order.next(third), "next at the tail is None");
        assert_eq!(Some(second), order.prev(third), "prev walks backward");
        assert_eq!(None, order.prev(first), "prev at the head is None");
        assert_eq!(Some(&1), order.get(second), "get returns the payload");
    }

    #[test]
    fn slot_reuse_distinguishes_generation()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        let first = order.push_back(0).expect("push_back succeeds");
        order.push_back(1).expect("push_back succeeds");
        assert_eq!(Some(0), order.remove(first), "free the first slot");
        // The next allocation reuses the freed slot with a bumped generation.
        let reused = order.push_back(2).expect("push_back reuses the slot");
        assert_eq!(reused.index, first.index, "the freed slot index is reused");
        assert_ne!(
            first.generation, reused.generation,
            "the generation is bumped"
        );
        assert_eq!(
            None,
            order.get(first),
            "the stale handle does not alias the reuse"
        );
        assert_eq!(Some(&2), order.get(reused), "the fresh handle resolves");
    }

    #[test]
    fn stale_handle_insert_errors()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        let only = order.push_back(0).expect("push_back succeeds");
        order.remove(only).expect("remove succeeds");
        assert_eq!(
            Err(OrderError::UnknownPosition),
            order.insert_after(only, 1),
            "insert_after a stale handle errors"
        );
        assert_eq!(
            Err(OrderError::UnknownPosition),
            order.insert_before(only, 1),
            "insert_before a stale handle errors"
        );
    }

    #[test]
    fn foreign_handle_is_rejected()
    {
        let mut one: OrderMaintenance<u64> = new_order();
        let mut two: OrderMaintenance<u64> = new_order();
        let foreign = one.push_back(0).expect("push_back succeeds");
        let native = two.push_back(1).expect("push_back succeeds");
        assert_eq!(None, two.get(foreign), "a foreign handle does not resolve");
        assert_eq!(
            None,
            two.cmp(foreign, native),
            "comparing across structures is None"
        );
        assert_eq!(
            Err(OrderError::UnknownPosition),
            two.insert_after(foreign, 2),
            "inserting at a foreign handle errors"
        );
        assert!(
            !bool::from(two.contains(foreign)),
            "a foreign handle is not contained"
        );
    }

    #[test]
    fn interval_containment()
    {
        // Pre/post-order points for a tree: outer `[a_lo, a_hi]` wraps inner
        // `[b_lo, b_hi]`; a third point `c` sits after both.
        let mut order: OrderMaintenance<&'static str> = new_order();
        let a_lo = order.push_back("a_lo").expect("push_back succeeds");
        let b_lo = order.push_back("b_lo").expect("push_back succeeds");
        let b_hi = order.push_back("b_hi").expect("push_back succeeds");
        let a_hi = order.push_back("a_hi").expect("push_back succeeds");
        let c = order.push_back("c").expect("push_back succeeds");
        let outer = Interval::new(a_lo, a_hi);
        let inner = Interval::new(b_lo, b_hi);
        let disjoint = Interval::new(c, c);
        assert_eq!(
            Some(true),
            order.interval_contains(outer, inner).map(bool::from),
            "outer contains inner"
        );
        assert_eq!(
            Some(false),
            order.interval_contains(inner, outer).map(bool::from),
            "inner does not contain outer"
        );
        assert_eq!(
            Some(false),
            order.interval_contains(outer, disjoint).map(bool::from),
            "outer does not contain a later point"
        );
        assert_eq!(
            Some(true),
            order.interval_contains(outer, outer).map(bool::from),
            "an interval contains itself"
        );
    }

    #[test]
    fn interval_with_stale_endpoint_is_none()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        let lo = order.push_back(0).expect("push_back succeeds");
        let hi = order.push_back(1).expect("push_back succeeds");
        let gone = order.push_back(2).expect("push_back succeeds");
        order.remove(gone).expect("remove succeeds");
        let live = Interval::new(lo, hi);
        let stale = Interval::new(gone, gone);
        assert_eq!(
            None,
            order.interval_contains(live, stale),
            "a stale endpoint yields None"
        );
    }

    #[test]
    fn comparison_is_reflexive_and_total()
    {
        let mut order: OrderMaintenance<u64> = new_order();
        let first = order.push_back(0).expect("push_back succeeds");
        let second = order.push_back(1).expect("push_back succeeds");
        assert_eq!(
            Some(Ordering::Equal),
            order.cmp(first, first),
            "an element equals itself"
        );
        assert_eq!(
            Some(Ordering::Less),
            order.cmp(first, second),
            "earlier is Less"
        );
        assert_eq!(
            Some(Ordering::Greater),
            order.cmp(second, first),
            "later is Greater"
        );
    }
}
