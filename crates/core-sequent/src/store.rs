//! The two-region store of the L machine (the sequent-machines design's §4.2).
//!
//! The store carries two regions in its type: a **heap** region (a nominal cell
//! arena) and a **frame** stack (equation / control administration only).
//! Placement — which region a binding lives in — is a pluggable function that
//! is CONSTANT [`Region::Heap`] at L1 (heap-everything; the region refinement
//! is a gated follow-up). Because the L1 placement never routes a
//! value cell into a frame, the frame region owns no value cells, so the shrink
//! rule (returning through a covalue drops every frame allocated after it) is
//! trivially sound — [`Store::shrink_frames`] is a plain truncation.
//!
//! # Call-by-need cells (carried over from the retired evaluator, exactly)
//!
//! The heap holds the [`Cell`]s that back call-by-need thunk forcing. A cell is
//! the L-machine image of the retired evaluator's memo cell: an
//! interior-mutable [`MemoState`] (`Unforced | InProgress | Forced`) with
//! **black-holing** across the force probe and the CEK's **pure-spine-only
//! write-back** — a body that is not purely reducible clears the black hole and
//! re-runs inline on every force (see `machine::LMachine::force`). Cell
//! identity is **nominal**: a cell is never deduplicated or merged
//! (content-addressing will cover the immutable value graph only), so two
//! thunks are the same cell exactly when they were allocated as one and copied
//! (the shared [`Cell`] handle).
//!
//! The memo policy is **uniform across grades** — no grade-sensitive caching —
//! so the L machine's step count stays parity with the CEK under the shared
//! budget.

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;

use gandr_core_term::grade::Grade;

use crate::boundary::CellCount;
use crate::boundary::CellIndex;
use crate::boundary::FrameHeight;
use crate::boundary::FrameMark;
use crate::machine::LValue;

/// A store region (`proposal-sequent-kernel.md` §4.2).
///
/// L1 places every binding in [`Self::Heap`]; the [`Self::Frame`] arm exists so
/// the region-shaped API is complete for the placement refinement,
/// which is the only thing that will ever return it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Region
{
    /// The persistent heap region (duplicable / hashconsable values and the
    /// call-by-need cells).
    Heap,
    /// The linear frame region (dies with its frame).
    Frame,
}

/// The placement function — which region a grade-`r` binding lives in.
///
/// **Constant `Heap` at L1** (heap-everything; `proposal-sequent-kernel.md`
/// §4.2, the safe default). The grade→region refinement
/// is a gated follow-up; until it lands this ignores the grade.
///
/// # Contract
/// - ensures: [`Region::Heap`] for every grade at L1.
/// - panics: none.
#[inline]
#[must_use]
pub fn placement(_grade: Grade) -> Region
{
    Region::Heap
}

/// A nominal heap-cell address (`proposal-sequent-kernel.md` §4.2).
///
/// The index of a [`Cell`] in a [`Store`]'s heap registry — a stable, never
/// reused, never merged handle (nominal identity), so a machine configuration
/// can name a cell for inspection.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellId(usize);

impl From<usize> for CellId
{
    #[inline]
    fn from(index: usize) -> Self
    {
        Self(index)
    }
}

impl CellId
{
    /// Return this address as a semantic store index.
    #[inline]
    #[must_use]
    pub fn index(self) -> CellIndex
    {
        CellIndex::from(self.0)
    }
}

impl From<CellId> for usize
{
    #[inline]
    fn from(id: CellId) -> Self
    {
        id.0
    }
}

/// The state of a call-by-need [`Cell`] — the L-machine image of the
/// retired evaluator's thunk memo (the sequent-machines design's §4.2; ADR-50
/// call-by-need).
#[derive(Clone, Debug)]
pub enum MemoState
{
    /// Not yet forced.
    Unforced,
    /// Currently being forced — the **black hole**; a re-entrant force (a
    /// cycle) observes it and falls back to running inline. Unreachable in
    /// v0 (no recursion), where a genuine cycle degrades to the shared step
    /// budget.
    InProgress,
    /// Forced to a weak-head normal form: the cached machine value the force
    /// probe reduced the body to (continuation-independent for a pure body, so
    /// reusing it is observationally transparent — see `machine::LMachine`).
    Forced(Rc<LValue>),
}

/// A call-by-need memoization cell — the L-machine image of the retired
/// evaluator's memo cell.
///
/// Interior-mutable (an `Rc<RefCell<…>>`), so a force through any shared copy
/// of a thunk value caches for all copies, exactly as the CEK's `Rc<RefCell>`
/// cell does; the shared handle is what makes cell identity nominal. Its
/// identity is intentionally invisible to `Eq` (a cell is a derived cache, not
/// part of a value's meaning), so two structurally equal thunks compare equal
/// regardless of force state.
#[repr(transparent)]
#[derive(Clone)]
pub struct Cell(Rc<RefCell<MemoState>>);

impl Cell
{
    /// Reads the cell's current state (a clone of the small enum).
    ///
    /// # Contract
    /// - ensures: the current [`MemoState`]; a borrow held only for the read.
    /// - panics: none (no other borrow can be live — the machine is
    ///   single-threaded and never nests a borrow of one cell).
    #[inline]
    #[must_use]
    pub fn state(&self) -> MemoState
    {
        self.0.borrow().clone()
    }

    /// Marks the cell as the in-progress **black hole** (across a force probe).
    #[inline]
    pub fn mark_in_progress(&self)
    {
        *self.0.borrow_mut() = MemoState::InProgress;
    }

    /// Clears the cell back to [`MemoState::Unforced`] (the pure-spine-only
    /// write-back's fallback: a non-purely-reducible body clears the black hole
    /// and re-runs inline on every force).
    #[inline]
    pub fn clear(&self)
    {
        *self.0.borrow_mut() = MemoState::Unforced;
    }

    /// Caches the forced weak-head form (the pure-spine write-back).
    #[inline]
    pub fn set_forced(
        &self,
        value: Rc<LValue>,
    )
    {
        *self.0.borrow_mut() = MemoState::Forced(value);
    }
}

impl PartialEq for Cell
{
    /// A cell carries no semantic content, so all cells compare equal — keeping
    /// [`LValue`]'s equality a structural one that ignores force state (mirrors
    /// the retired evaluator's memo cell).
    #[inline]
    fn eq(
        &self,
        _other: &Self,
    ) -> bool
    {
        true
    }
}

impl Eq for Cell
{
}

impl core::fmt::Debug for Cell
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str("<cell>")
    }
}

/// An administrative frame — equation / control administration only
/// (`proposal-sequent-kernel.md` §4.2).
///
/// At L1 the frame region is not load-bearing (placement is heap-everything, so
/// no value cell lands here); the type is present so the region-shaped API and
/// the shrink rule are complete for the placement refinement.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Frame
{
    /// An equation-administration marker (a returning-through-a-covalue
    /// boundary). Owns no value cell.
    Equation,
}

/// The two-region store (`proposal-sequent-kernel.md` §4.2).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Store
{
    /// The heap region: the nominal call-by-need cell arena. Append-only —
    /// a cell is never removed or merged (nominal identity).
    heap: Vec<Cell>,
    /// The linear frame region: equation / control administration only (no
    /// value cells at L1).
    frames: Vec<Frame>,
}

impl Store
{
    /// An empty store.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Allocates a fresh, unforced call-by-need cell in the heap, returning its
    /// nominal address and a shared handle to it.
    ///
    /// # Contract
    /// - ensures: a distinct [`CellId`] each call (never reused), and a
    ///   [`Cell`] handle aliasing the same interior-mutable state as the stored
    ///   one, so a force through the returned handle is visible at the address.
    /// - panics: none.
    #[inline]
    pub fn alloc_cell(&mut self) -> (CellId, Cell)
    {
        let id = CellId::from(self.heap.len());
        let cell = Cell(Rc::new(RefCell::new(MemoState::Unforced)));
        self.heap.push(cell.clone());
        (id, cell)
    }

    /// The number of cells the heap has allocated (for inspection).
    #[inline]
    #[must_use]
    pub fn heap_len(&self) -> CellCount
    {
        self.heap.len().into()
    }

    /// Reads a cell's state by nominal address (for inspection).
    ///
    /// # Contract
    /// - ensures: `Some(state)` when `id` addresses an allocated cell; `None`
    ///   for an out-of-range address.
    #[inline]
    #[must_use]
    pub fn cell_state(
        &self,
        id: CellId,
    ) -> Option<MemoState>
    {
        self.heap.get(usize::from(id)).map(Cell::state)
    }

    /// Pushes an administrative frame, returning the pre-push height (the mark
    /// a later [`Self::shrink_frames`] returns to).
    #[inline]
    pub fn push_frame(
        &mut self,
        frame: Frame,
    ) -> FrameMark
    {
        let mark = self.frames.len();
        self.frames.push(frame);
        mark.into()
    }

    /// The current frame-region height.
    #[inline]
    #[must_use]
    pub fn frame_height(&self) -> FrameHeight
    {
        self.frames.len().into()
    }

    /// The shrink rule: drop every frame allocated at or after `mark`
    /// (returning through a covalue drops the frames allocated after it).
    ///
    /// Trivially sound at L1: the frame region owns no value cells, so dropping
    /// frames frees only administration — no live value can be stranded.
    ///
    /// # Contract
    /// - ensures: the frame height is `min(mark, current)`; the heap is
    ///   untouched.
    /// - panics: none.
    #[inline]
    pub fn shrink_frames(
        &mut self,
        mark: FrameMark,
    )
    {
        self.frames.truncate(usize::from(mark));
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_term::boundary::GradeBound;

    use super::*;

    /// Placement is constant `Heap` at L1, over every grade.
    #[test]
    fn placement_is_heap_everything()
    {
        for grade in [
            Grade::ZERO,
            Grade::ONE,
            Grade::fin(GradeBound::from(3_u64)),
            Grade::OMEGA,
        ] {
            assert_eq!(
                Region::Heap,
                placement(grade),
                "L1 places everything on the heap"
            );
        }
    }

    /// A cell allocates unforced, and a force through one handle is visible at
    /// the nominal address (shared interior mutability).
    #[test]
    fn cell_write_back_is_shared_and_nominal()
    {
        let mut store = Store::new();
        let (first_id, first) = store.alloc_cell();
        let (second_id, _second) = store.alloc_cell();
        assert_ne!(
            first_id, second_id,
            "distinct cells get distinct nominal addresses"
        );
        assert!(
            matches!(store.cell_state(first_id), Some(MemoState::Unforced)),
            "a fresh cell is unforced"
        );

        first.mark_in_progress();
        assert!(
            matches!(store.cell_state(first_id), Some(MemoState::InProgress)),
            "the black hole is visible at the address through the shared handle"
        );

        first.set_forced(Rc::new(LValue::unit()));
        assert!(
            matches!(store.cell_state(first_id), Some(MemoState::Forced(_))),
            "the write-back caches through the shared handle"
        );

        first.clear();
        assert!(
            matches!(store.cell_state(first_id), Some(MemoState::Unforced)),
            "clearing the black hole restores unforced"
        );
    }

    /// The frame region shrinks by truncation to a mark (trivially sound —
    /// frames own no value cells).
    #[test]
    fn frames_shrink_to_a_mark()
    {
        let mut store = Store::new();
        let mark = store.push_frame(Frame::Equation);
        let _inner = store.push_frame(Frame::Equation);
        assert_eq!(2, usize::from(store.frame_height()), "two frames pushed");
        store.shrink_frames(mark);
        assert_eq!(
            usize::from(store.frame_height()),
            usize::from(mark),
            "shrink drops frames after the mark"
        );
    }
}
