//! The **cell store** — oriented 2-cells over a [`CellAlphabet`],
//! content-addressed and carrying derived metadata
//! (`proposal-sequent-kernel.md` §7.3; VDC addendum §A).
//!
//! A [`Cell`] is an oriented rewrite `lhs ~> rhs` between two command patterns
//! of the alphabet `A`, with:
//!
//! - an orientation tag (`A::Orientation` — fixed by the cut polarity (K2) or
//!   chosen by the completion reduction order (§7.3.3), in the sequent
//!   alphabet);
//! - a provenance tag (`A::Provenance` — where the cell came from: a surface
//!   `rule`, the μ/μ̃ critical pair, a return-side frame's defining cell, an η
//!   law, or completion);
//! - a derived metadata (`A::Meta` — per-metavariable variance/linearity and
//!   the `invertible` flag in the sequent alphabet), computed **live** at
//!   construction by [`CellAlphabet::derive_meta`].
//!
//! The store ([`CellStore`]) dedups on **structural identity** ([`Eq`] over the
//! pattern pair — the content-addressing of §7.3.1) and hands out dense
//! [`CellId`]s in insertion order, so iteration and completion are
//! deterministic. Content addressing is by structural content only — no arena
//! addresses, generation stamps, or session state (the module-level trust
//! warning of [`crate::alphabet`]).

use alloc::vec::Vec;

use crate::alphabet::CellAlphabet;
use crate::boundary::CellCount;
use crate::boundary::CellStoreEmptyStatus;
use crate::sequent::SequentAlphabet;

/// An **oriented 2-cell** over the alphabet `A`
/// (`proposal-sequent-kernel.md` §7.3, the `Cell` struct).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Cell<A: CellAlphabet = SequentAlphabet>
{
    /// The left-hand side (the redex pattern).
    pub lhs: A::Cmd,
    /// The right-hand side (the contractum pattern).
    pub rhs: A::Cmd,
    /// How the orientation was fixed.
    pub orient: A::Orientation,
    /// Where the cell came from.
    pub provenance: A::Provenance,
    /// The derived metadata.
    pub meta: A::Meta,
}

impl<A: CellAlphabet> Cell<A>
{
    /// A cell from its two sides, orientation, and provenance; the metadata is
    /// derived live from both faces and the `invertible` flag set from the
    /// provenance (completion-emitted cells are invertible joinability
    /// certificates).
    ///
    /// # Contract
    /// - ensures: `meta == A::derive_meta(&lhs, &rhs,
    ///   A::completion_certificate(&provenance))`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        lhs: A::Cmd,
        rhs: A::Cmd,
        orient: A::Orientation,
        provenance: A::Provenance,
    ) -> Self
    {
        let invertible = A::completion_certificate(&provenance);
        let meta = A::derive_meta(&lhs, &rhs, invertible);
        Self {
            lhs,
            rhs,
            orient,
            provenance,
            meta,
        }
    }
}

/// A dense **cell identifier** — an index into a [`CellStore`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CellId(pub usize);

/// The **cell store** — a content-addressed, insertion-ordered collection of
/// cells (`proposal-sequent-kernel.md` §7.3.1).
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CellStore<A: CellAlphabet = SequentAlphabet>
{
    /// The cells, in insertion order; the index is the [`CellId`].
    cells: Vec<Cell<A>>,
}

impl<A: CellAlphabet> CellStore<A>
{
    /// An empty store.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Insert a cell, deduplicating on structural identity, and return its id.
    ///
    /// # Contract
    /// - ensures: a cell structurally equal (same `lhs`/`rhs`/orientation/
    ///   provenance) to one already present returns that cell's existing id and
    ///   does not grow the store; otherwise the cell is appended and its fresh
    ///   id returned. Content-addressing is thus by structural identity
    ///   (§7.3.1).
    /// - panics: none.
    #[inline]
    pub fn insert(
        &mut self,
        cell: Cell<A>,
    ) -> CellId
    {
        if let Some(index) = self.cells.iter().position(|existing| *existing == cell) {
            return CellId(index);
        }
        let id = CellId(self.cells.len());
        self.cells.push(cell);
        id
    }

    /// The cell with the given id.
    ///
    /// # Contract
    /// - ensures: `Some` iff `id` was handed out by [`CellStore::insert`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn get(
        &self,
        id: CellId,
    ) -> Option<&Cell<A>>
    {
        self.cells.get(id.0)
    }

    /// The number of cells in the store.
    #[inline]
    #[must_use]
    pub fn len(&self) -> CellCount
    {
        CellCount::from(self.cells.len())
    }

    /// Whether the store is empty.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> CellStoreEmptyStatus
    {
        CellStoreEmptyStatus::from(self.cells.is_empty())
    }

    /// The cells with their ids, in insertion order.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (CellId, &Cell<A>)>
    {
        self.cells
            .iter()
            .enumerate()
            .map(|(index, cell)| (CellId(index), cell))
    }
}
