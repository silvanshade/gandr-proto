//! **Overlap enumeration at cut seams** — the multi-sum-shaped enumerator
//! (the sequent-machines design's §7.3.2; VDC addendum §7.4), generic over
//! the [`CellAlphabet`] (the executed meta-spike-01).
//!
//! For each ordered pair of cells the enumerator returns the *complete family*
//! of unifications at a cut, never a single chosen one (§7.3.2: "enumerate
//! every overlap"; the fan-out is the mathematically-forced multi-sum of the
//! addendum §7.4). Two flavors, both rooted at the seam ("cuts make overlaps
//! shallow — the seam is the root"):
//!
//! - [`OverlapKind::Confluence`] — the Knuth–Bendix **critical pair**: the two
//!   left-hand sides unify, so the peak `σ(lₐ)` reduces two ways (apply left,
//!   apply right). Completion (`crate::completion`) normalizes both reducts and
//!   either joins them (a coherence [`crate::tracelet::Tracelet`]) or orients
//!   the pair into a new cell.
//! - [`OverlapKind::Composition`] — the Behr–Harmer–Krivine **sequential
//!   composition**: the left cell's right-hand side unifies (at a command
//!   position) with the right cell's left-hand side, so applying left then
//!   right is a two-step derivation whose composite is the **fused cell**
//!   `σ(lₐ) ~> (σ(rₐ) with the right cell fired at the seam)` — deforestation
//!   as a derived 2-cell (§7.2), certified by the fused≡two-step tracelet.
//!
//! Both are single root/seam unifications, which is exactly why the enumeration
//! is tractable where tree rewriting would need full subterm traversal
//! (§7.3.2).
//!
//! The enumerator emits **seam data** — span-level overlap descriptions (the
//! unifier, the seam position, the superposition peak, and the apartness-
//! renamed right leg), never just booleans or reduced results; a cell store
//! that could answer overlaps but not describe them would starve the
//! convolution face. This holds for every alphabet, and the enumerator stays
//! off-TCB (the module-level warning of [`crate::alphabet`]).

use alloc::vec::Vec;

use crate::alphabet::CellAlphabet;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::sequent::SequentAlphabet;

/// The kind of overlap (`proposal-sequent-kernel.md` §7.2–§7.3.2).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OverlapKind
{
    /// The two left-hand sides unify — a confluence critical pair (§7.3.3).
    Confluence,
    /// The left right-hand side unifies with the right left-hand side — a
    /// sequential composition / fusion overlap (§7.2).
    Composition,
}

/// One **overlap** between two cells at a seam (`proposal-sequent-kernel.md`
/// §7.3, the `Overlap` struct).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Overlap<A: CellAlphabet = SequentAlphabet>
{
    /// The left cell.
    pub left: CellId,
    /// The right cell.
    pub right: CellId,
    /// The overlap kind.
    pub kind: OverlapKind,
    /// The most general unifier at the seam.
    pub unifier: A::Subst,
    /// The seam position (the root for a confluence overlap; the command
    /// position in the left right-hand side for a composition overlap).
    pub seam: A::Pos,
    /// The superposition term `σ(lₐ)` the overlap is rooted at.
    pub peak: A::Cmd,
    /// The right cell renamed apart from the left (so unification kept the two
    /// cells' metavariables disjoint); the reduct methods contract against it.
    right_renamed: Cell<A>,
}

impl<A: CellAlphabet> Overlap<A>
{
    /// The left reduct — the peak contracted by the **left** cell at the root.
    ///
    /// # Contract
    /// - ensures: `Some(σ(rₗₑ𝒻ₜ))`, the one-step contraction of the peak by the
    ///   left cell; `None` only if the left id is stale.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn left_reduct(
        &self,
        store: &CellStore<A>,
    ) -> Option<A::Cmd>
    {
        let left = store.get(self.left)?;
        Some(A::apply_subst(&self.unifier, &left.rhs))
    }

    /// The right reduct of a **confluence** overlap — the peak contracted by
    /// the **right** cell at the root.
    ///
    /// # Contract
    /// - requires: `self.kind == OverlapKind::Confluence`.
    /// - ensures: `Some(σ(rᵣᵢ𝓰ₕₜ))`, the other one-step contraction of the
    ///   peak; the confluence critical pair is `(left_reduct, right_reduct)`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn right_reduct(
        &self,
        _store: &CellStore<A>,
    ) -> Option<A::Cmd>
    {
        Some(A::apply_subst(&self.unifier, &self.right_renamed.rhs))
    }

    /// The right cell **renamed apart** from the left — the span's right leg
    /// (TA-8: the enumerator's seam data, consumed by the crDC suite, the
    /// convolution face, and the overlap-support cache).
    ///
    /// The enumerator renames the right cell's metavariables until they are
    /// disjoint from the left cell's before unifying, so the unifier's bindings
    /// on the right side read against this renamed cell, never the original.
    /// The renaming is structure-preserving (same pattern shapes, occurrence-
    /// parallel metavariable lists), so the original right leg is recoverable
    /// by zipping the two cells' metavariable occurrences.
    ///
    /// # Contract
    /// - ensures: the apartness-renamed right cell the [`Self::unifier`] and
    ///   [`Self::right_reduct`] / [`Self::composite`] reducts contract against.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn right_renamed(&self) -> &Cell<A>
    {
        &self.right_renamed
    }

    /// The composite of a **composition** overlap — apply the left cell at the
    /// root, then the right cell at the seam (the fused cell's right-hand
    /// side).
    ///
    /// # Contract
    /// - requires: `self.kind == OverlapKind::Composition`.
    /// - ensures: `Some(result)`, the two-step contraction `peak → (left) →
    ///   (right at seam)`; the fused cell is `peak ~> result`. `None` if the
    ///   seam no longer addresses a command after the left contraction (never,
    ///   for a well-formed composition overlap).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn composite(
        &self,
        store: &CellStore<A>,
    ) -> Option<A::Cmd>
    {
        let after_left = self.left_reduct(store)?;
        let right_rhs = A::apply_subst(&self.unifier, &self.right_renamed.rhs);
        A::splice_cmd_at(&after_left, &self.seam, right_rhs)
    }
}

/// Enumerate the **complete family** of overlaps among the store's cells
/// (`proposal-sequent-kernel.md` §7.3.2).
///
/// # Contract
/// - ensures: every confluence overlap (a unifiable left/left pair, excluding a
///   cell with itself) and every composition overlap (a unifiable right/left
///   pair at a command seam) among the store's cells, in a deterministic order.
///   The list is the multi-sum family — one entry per unifier, never collapsed.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the sequent-alphabet suite asserts the same
///   deterministic multi-sum family as before the lift (behavior preserved),
///   and the toy alphabet drives the same generic loop to a real composition
///   overlap (polymorphism exercised).
/// - witness: `overlap::tests::the_frame_and_add_cells_compose_into_the_commutation_cell`
/// - witness: `overlap::tests::overlaps_are_a_deterministic_family`
/// - witness: `toy_alphabet::tests::the_enumerator_finds_the_toy_composition_overlap`
#[inline]
#[must_use]
pub fn enumerate_overlaps<A>(store: &CellStore<A>) -> Vec<Overlap<A>>
where
    A: CellAlphabet,
{
    let mut out = Vec::new();
    for (id_left, cell_left) in store.iter() {
        for (id_right, cell_right) in store.iter() {
            out.extend(overlaps_between(
                (id_left, cell_left),
                (id_right, cell_right),
            ));
        }
    }
    out
}

/// The overlap family of **one ordered cell pair** — the pair-keyed query
/// [`enumerate_overlaps`] iterates, exposed on its own.
///
/// Independence is keyed by cell pair and is therefore cacheable, which is what
/// lets a per-pair consumer ask about two cells without paying the quadratic
/// store-wide sweep. [`crate::shift::derive_shift_equivalence`] is that
/// consumer: its second conjunct is "this cell pair has trivial overlap", and
/// trivial means *this family is empty*.
///
/// # Contract
/// - requires: `left` and `right` are `(id, cell)` pairs the same store handed
///   out, so the emitted [`Overlap::left`] / [`Overlap::right`] ids address the
///   cells they name.
/// - ensures: the confluence overlap of the two left-hand sides when they unify
///   — suppressed when the two ids are equal, whose reducts are identical by
///   construction — followed by one composition overlap per command seam of the
///   left cell's right-hand side that unifies with the right cell's left-hand
///   side, in the alphabet's own seam order. The family is the multi-sum, one
///   entry per unifier, never collapsed.
/// - provides: the same entries, in the same order, that [`enumerate_overlaps`]
///   emits for this pair.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the extraction is behavior-preserving, so the
///   store-wide family stays byte-identical and deterministic, and the pair
///   query is separated from it by an empty family for a non-overlapping pair
///   and a nonempty one for the composition pair.
/// - witness: `overlap::tests::overlaps_are_a_deterministic_family`
/// - witness: `overlap::tests::the_pair_query_agrees_with_the_store_wide_family`
/// - witness: `shift::tests::the_cong2_cell_pair_has_trivial_overlap`
#[inline]
#[must_use]
pub fn overlaps_between<A>(
    left: (CellId, &Cell<A>),
    right: (CellId, &Cell<A>),
) -> Vec<Overlap<A>>
where
    A: CellAlphabet,
{
    let (id_left, cell_left) = left;
    let (id_right, cell_right) = right;
    let mut out = Vec::new();
    let (renamed_lhs, renamed_rhs) = A::rename_apart(
        (&cell_left.lhs, &cell_left.rhs),
        (&cell_right.lhs, &cell_right.rhs),
    );
    let renamed = Cell::new(
        renamed_lhs,
        renamed_rhs,
        cell_right.orient,
        cell_right.provenance,
    );
    // Confluence: unify the two left-hand sides (skip the trivial
    // self-overlap, whose reducts are identical by construction).
    if id_left != id_right {
        let mut unifier = A::Subst::default();
        if bool::from(A::unify_cmd(&cell_left.lhs, &renamed.lhs, &mut unifier)) {
            let peak = A::apply_subst(&unifier, &cell_left.lhs);
            out.push(Overlap {
                left: id_left,
                right: id_right,
                kind: OverlapKind::Confluence,
                unifier,
                seam: A::root_position(),
                peak,
                right_renamed: renamed.clone(),
            });
        }
    }
    // Composition: unify the left RHS (at each command position) with the
    // right LHS.
    for seam in A::command_positions(&cell_left.rhs) {
        let Some(sub) = A::subterm_cmd_at(&cell_left.rhs, &seam)
        else {
            continue;
        };
        let mut unifier = A::Subst::default();
        if bool::from(A::unify_cmd(&sub, &renamed.lhs, &mut unifier)) {
            let peak = A::apply_subst(&unifier, &cell_left.lhs);
            out.push(Overlap {
                left: id_left,
                right: id_right,
                kind: OverlapKind::Composition,
                unifier,
                seam,
                peak,
                right_renamed: renamed.clone(),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;

    use super::*;
    use crate::pattern::CmdPat;
    use crate::pattern::ConsPat;
    use crate::pattern::ProdPat;
    use crate::pattern::Sym;
    use crate::sequent::CellProvenance;
    use crate::sequent::Orientation;
    use crate::sequent::frame_defining_cell;

    #[test]
    fn the_frame_and_add_cells_compose_into_the_commutation_cell()
    {
        // (Succ⁻-def) then (add-S) compose to Succ⁻(add(n;α)) ~> add(n;Succ⁻(α)).
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let overlaps = enumerate_overlaps(&store);
        let composition = overlaps
            .iter()
            .find(|o| o.kind == OverlapKind::Composition && o.left == frame && o.right == add)
            .expect("the frame RHS ⟨Succ(v)|β⟩ unifies with add-S's LHS ⟨Succ(m)|add(n;α)⟩");
        let peak = &composition.peak;
        let composite = composition.composite(&store).expect("the composite exists");
        // Peak: ⟨v | Succ⁻(add(n;α))⟩ ; composite: ⟨v | add(n; Succ⁻(α))⟩.
        assert!(
            matches!(peak, CmdPat::Cut {
                cons: ConsPat::Frame { .. },
                ..
            }),
            "the peak feeds a Succ⁻ frame into add"
        );
        assert!(
            matches!(composite, CmdPat::Cut {
                cons: ConsPat::Op { .. },
                ..
            }),
            "the composite drops the intermediate Succ, leaving add with a pushed-in frame"
        );
    }

    #[test]
    fn overlaps_are_a_deterministic_family()
    {
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        store.insert(add_s());
        let first = enumerate_overlaps(&store);
        let second = enumerate_overlaps(&store);
        assert_eq!(first, second, "enumeration is deterministic");
    }

    #[test]
    fn the_pair_query_agrees_with_the_store_wide_family()
    {
        // The pair-keyed query is the store-wide sweep's own inner step, so
        // reassembling it pair by pair must reproduce the family exactly —
        // including the suppressed confluence self-overlap.
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        store.insert(add_s());
        let mut reassembled = Vec::new();
        for (id_left, cell_left) in store.iter() {
            for (id_right, cell_right) in store.iter() {
                reassembled.extend(overlaps_between(
                    (id_left, cell_left),
                    (id_right, cell_right),
                ));
            }
        }
        assert_eq!(
            enumerate_overlaps(&store),
            reassembled,
            "the pair query is the store-wide family, one ordered pair at a time"
        );
        assert!(
            !reassembled.is_empty(),
            "and the fixture is one that actually overlaps"
        );
    }

    /// (add-S): ⟨Succ(m) | add(n; α)⟩ ~> ⟨m | add(n; Succ⁻(α))⟩.
    fn add_s() -> Cell
    {
        let lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
        );
        let rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("m"),
            ConsPat::op(
                "add",
                [ProdPat::meta("n")],
                ConsPat::frame("Succ", ConsPat::meta("alpha")),
            ),
        );
        Cell::new(
            lhs,
            rhs,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }
}
