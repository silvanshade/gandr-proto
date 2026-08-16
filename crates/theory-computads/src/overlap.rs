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
use std::collections::HashSet;

use crate::alphabet::CellAlphabet;
use crate::boundary::CertificateIndex;
use crate::boundary::StepIndependence;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::sequent::SequentAlphabet;
use crate::tracelet::Tracelet;

/// The unordered identity of two primitive cells.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct CellPair(CellId, CellId);

impl CellPair
{
    /// The unordered identity of two cells, normalized to ascending order so
    /// the two argument orders build the same key.
    #[inline]
    #[must_use]
    fn new(
        left: CellId,
        right: CellId,
    ) -> Self
    {
        if left <= right {
            return Self(left, right);
        }
        Self(right, left)
    }
}

/// Memoized support for the overlap relation on cells and certificates.
///
/// # Contract
/// - ensures: cell support is populated from the existing overlap enumerator,
///   and certificate support is the symmetric closure of all inserted
///   certificate primitive supports.
/// - provides: constant-time independence queries after construction.
/// - panics: none.
///
/// # Adequacy
/// - witness: `overlap::tests::overlap_support_is_symmetric_and_certificate_memoized`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OverlapSupport
{
    /// The unordered cell pairs the enumerator answered an overlap for.
    cell_overlaps: HashSet<CellPair>,
    /// The ordered certificate-index keys whose supports meet, ascending.
    certificate_overlaps: HashSet<(usize, usize)>,
    /// The primitive cell support of each inserted certificate, by stable
    /// index.
    certificate_cells: Vec<HashSet<CellId>>,
    /// The next stable index [`Self::add_certificates`] hands out.
    next_certificate_index: usize,
}

impl OverlapSupport
{
    /// Build support for every cell pair in `store`.
    ///
    /// # Contract
    /// - ensures: every enumerated overlap contributes its unordered cell pair.
    /// - provides: an O(1) support relation for subsequent queries.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - witness: `overlap::tests::overlap_support_is_symmetric_and_certificate_memoized`.
    #[inline]
    #[must_use]
    pub fn from_store<A>(store: &CellStore<A>) -> Self
    where
        A: CellAlphabet,
    {
        let mut support = Self::default();
        for (left_id, left) in store.iter() {
            for (right_id, right) in store.iter() {
                if !overlaps_between((left_id, left), (right_id, right)).is_empty() {
                    support
                        .cell_overlaps
                        .insert(CellPair::new(left_id, right_id));
                }
            }
        }
        support
    }
    /// Add certificates to the memoized relation and return their stable keys.
    ///
    /// The keys are returned as the explicit pair `(first, past_the_end)` of a
    /// half-open range rather than as a [`core::ops::Range`], because a range
    /// over a wrapper is not iterable and a caller reaching for one would find
    /// that out at the use site rather than here.
    ///
    /// # Contract
    /// - ensures: new certificates are compared with every prior insertion and
    ///   receive the checked monotone half-open key pair returned on success.
    /// - provides: O(1) certificate-overlap queries after insertion.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - witness: `overlap::tests::overlap_support_is_symmetric_and_certificate_memoized`.
    #[inline]
    pub fn add_certificates<A>(
        &mut self,
        certificates: &[Tracelet<A>],
    ) -> Option<(CertificateIndex, CertificateIndex)>
    where
        A: CellAlphabet,
    {
        let base = self.next_certificate_index;
        let end = base.checked_add(certificates.len())?;
        let new_support: Vec<HashSet<CellId>> = certificates
            .iter()
            .map(|certificate| {
                certificate
                    .path_a
                    .iter()
                    .chain(&certificate.path_b)
                    .map(|step| step.cell)
                    .collect()
            })
            .collect();
        for (local_index, support) in new_support.iter().enumerate() {
            let index = base.saturating_add(local_index);
            self.certificate_overlaps.insert((index, index));
            for (prior_index, prior_support) in self.certificate_cells.iter().enumerate() {
                if !bool::from(self.certificate_supports_independent(support, prior_support)) {
                    let key = if index <= prior_index {
                        (index, prior_index)
                    }
                    else {
                        (prior_index, index)
                    };
                    self.certificate_overlaps.insert(key);
                }
            }
            for (right_offset, right_support) in new_support
                .iter()
                .enumerate()
                .skip(local_index.saturating_add(1_usize))
            {
                if !bool::from(self.certificate_supports_independent(support, right_support)) {
                    self.certificate_overlaps
                        .insert((index, base.saturating_add(right_offset)));
                }
            }
        }
        self.certificate_cells.extend(new_support);
        self.next_certificate_index = end;
        Some((CertificateIndex::from(base), CertificateIndex::from(end)))
    }

    /// Whether two certificate supports are independent — sharing no primitive
    /// cell, and holding no two cells this support calls dependent.
    ///
    /// True means independent, the crate's single polarity; the overlap
    /// recording that consumes this negates it at the decision.
    #[inline]
    #[must_use]
    fn certificate_supports_independent(
        &self,
        left: &HashSet<CellId>,
        right: &HashSet<CellId>,
    ) -> StepIndependence
    {
        StepIndependence::from(!left.iter().any(|left_cell| {
            right.iter().any(|right_cell| {
                left_cell == right_cell || !bool::from(self.independent(*left_cell, *right_cell))
            })
        }))
    }

    /// Whether two stable certificate indices are independent.
    ///
    /// # Contract
    /// - ensures: false exactly when the two certificates' primitive supports
    ///   meet; the result is symmetric, keyed by the stable indices assigned by
    ///   [`Self::add_certificates`] and normalized to ascending order, and a
    ///   certificate is never independent of itself.
    /// - provides: O(1) certificate independence queries.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - witness: `overlap::tests::overlap_support_is_symmetric_and_certificate_memoized`.
    #[inline]
    #[must_use]
    pub fn certificates_independent(
        &self,
        left: CertificateIndex,
        right: CertificateIndex,
    ) -> StepIndependence
    {
        let left = usize::from(left);
        let right = usize::from(right);
        let key = if left <= right {
            (left, right)
        }
        else {
            (right, left)
        };
        StepIndependence::from(!self.certificate_overlaps.contains(&key))
    }

    /// Whether two primitive cells are independent under this support.
    ///
    /// This is the crate's cell-level independence query and the only polarity
    /// the support answers in: an overlap question is this value negated where
    /// it is asked, so no opposite-polarity query can drift from it.
    ///
    /// # Contract
    /// - ensures: false exactly when the enumerator answered an overlap for the
    ///   pair, and the result is independent of argument order.
    /// - provides: an O(1) independence lookup after [`Self::from_store`].
    /// - panics: none.
    ///
    /// # Adequacy
    /// - witness: `overlap::tests::overlap_support_is_symmetric_and_certificate_memoized`.
    #[inline]
    #[must_use]
    pub fn independent(
        &self,
        left: CellId,
        right: CellId,
    ) -> StepIndependence
    {
        StepIndependence::from(!self.cell_overlaps.contains(&CellPair::new(left, right)))
    }
    /// Partition overlap work into deterministic independent batches.
    ///
    /// # Contract
    /// - ensures: every batch is pairwise independent under this support
    ///   relation, and batch order follows first appearance in the input.
    /// - provides: ready parallel work units for completion scheduling.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - witness: `overlap::tests::overlap_support_batches_are_pairwise_independent`.
    #[inline]
    #[must_use]
    pub fn batches<A>(
        &self,
        overlaps: &[Overlap<A>],
    ) -> Vec<Vec<Overlap<A>>>
    where
        A: CellAlphabet,
    {
        let mut batches: Vec<Vec<Overlap<A>>> = Vec::new();
        for overlap in overlaps {
            let mut placed = false;
            for batch in &mut batches {
                if batch
                    .iter()
                    .all(|held| bool::from(self.overlaps_are_independent(held, overlap)))
                {
                    batch.push(overlap.clone());
                    placed = true;
                    break;
                }
            }
            if !placed {
                batches.push(alloc::vec![overlap.clone()]);
            }
        }
        batches
    }

    /// Whether two overlaps share no dependent cell pair across their four
    /// endpoints — the condition [`Self::batches`] places a member under.
    ///
    /// True means independent, the same polarity [`Self::independent`] answers
    /// in, of which this is the four-endpoint conjunction.
    #[inline]
    #[must_use]
    fn overlaps_are_independent<A>(
        &self,
        left: &Overlap<A>,
        right: &Overlap<A>,
    ) -> StepIndependence
    where
        A: CellAlphabet,
    {
        StepIndependence::from(
            bool::from(self.independent(left.left, right.left))
                && bool::from(self.independent(left.left, right.right))
                && bool::from(self.independent(left.right, right.left))
                && bool::from(self.independent(left.right, right.right)),
        )
    }
}
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
    use crate::boundary::CausalDepth;
    use crate::boundary::CellCount;
    use crate::boundary::CompletionStatus;
    use crate::boundary::EventCount;
    use crate::boundary::ReplayLevel;
    use crate::completion::CompletionBudget;
    use crate::completion::CompletionOutcome;
    use crate::completion::complete;
    use crate::normal_form::ReplayWitness;
    use crate::normal_form::normalize_certified;
    use crate::pattern::CmdPat;
    use crate::pattern::ConsPat;
    use crate::pattern::ProdPat;
    use crate::pattern::Sym;
    use crate::rewrite::CellApp;
    use crate::rewrite::rewrite_at;
    use crate::sequent::CellProvenance;
    use crate::sequent::Orientation;
    use crate::sequent::frame_defining_cell;
    use crate::tracelet::confluence_tracelet;

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

    #[test]
    fn overlap_support_batches_are_pairwise_independent()
    {
        let store = independent_rule_clusters();
        let support = OverlapSupport::from_store(&store);
        let overlaps = confluence_family(&store);
        let batches = support.batches(&overlaps);
        // The fixture is ordered: three two-rule clusters over three disjoint
        // operation symbols, so the family alternates between the clusters and
        // a batch can only be built by taking one overlap from each.
        assert_eq!(
            6_usize,
            overlaps.len(),
            "the ordered fixture enumerates each cluster's critical pair in both directions"
        );
        assert!(
            overlaps
                .iter()
                .all(|overlap| overlap.kind == OverlapKind::Confluence),
            "the batched family is the confluence family the completion worklist schedules"
        );
        let mut endpoints: Vec<CellId> = overlaps
            .iter()
            .flat_map(|overlap| [overlap.left, overlap.right])
            .collect();
        endpoints.sort_unstable();
        endpoints.dedup();
        assert_eq!(
            6_usize,
            endpoints.len(),
            "six distinct cells actually take part in an overlap here"
        );
        assert!(
            overlaps.iter().enumerate().all(|(index, left)| {
                overlaps
                    .iter()
                    .skip(index.saturating_add(1_usize))
                    .all(|right| left != right)
            }),
            "the enumerated family carries no duplicate entry, so an input position is exact"
        );
        assert_eq!(
            2_usize,
            batches.len(),
            "the six overlaps schedule into exactly two nonempty batches"
        );
        assert!(
            batches.iter().all(|batch| batch.len() == 3_usize),
            "each batch holds exactly one overlap from each of the three independent clusters"
        );
        assert!(
            batches.iter().any(|batch| batch.len() > 1_usize),
            "at least one batch is genuinely multi-member, so a fully serialized partition — one \
             overlap per batch — fails here"
        );
        // The flatten identity, stated as positions rather than as a length
        // sum: a length sum survives a partition that drops one overlap and
        // duplicates another, and these do not.
        let positions = batch_input_positions(&overlaps, &batches);
        assert!(
            positions.first().is_some_and(|batch| {
                batch.as_slice()
                    == [
                        InputPosition(0_usize),
                        InputPosition(2_usize),
                        InputPosition(4_usize),
                    ]
                    .as_slice()
            }),
            "the first batch is the family's first, third and fifth overlaps, in that order"
        );
        assert!(
            positions.get(1_usize).is_some_and(|batch| {
                batch.as_slice()
                    == [
                        InputPosition(1_usize),
                        InputPosition(3_usize),
                        InputPosition(5_usize),
                    ]
                    .as_slice()
            }),
            "the second batch is the family's second, fourth and sixth overlaps, in that order"
        );
        let mut covered: Vec<InputPosition> = positions.iter().flatten().copied().collect();
        covered.sort_unstable();
        assert_eq!(
            (0 .. overlaps.len())
                .map(InputPosition)
                .collect::<Vec<InputPosition>>(),
            covered,
            "flattening the batches returns the input family with multiplicity, and each batch \
             reads its members in first-appearance order"
        );
        assert!(
            positions.iter().all(|batch| batch.is_sorted()),
            "each batch is a subsequence of the input family, in input order"
        );
        assert!(
            positions
                .iter()
                .map(|batch| batch.first().copied().unwrap_or(InputPosition(0_usize)))
                .collect::<Vec<InputPosition>>()
                .is_sorted(),
            "batch order follows the first appearance of each batch's opening member"
        );
        assert!(
            batches.iter().all(|batch| {
                batch.iter().enumerate().all(|(left_index, left)| {
                    batch
                        .iter()
                        .skip(left_index.saturating_add(1_usize))
                        .all(|right| bool::from(support.overlaps_are_independent(left, right)))
                })
            }),
            "every batch is pairwise independent under the support relation"
        );
        // The same fixture, carried through to an actual replay plan: the
        // leading cluster's critical pair joins, and the certified derivation
        // it produces is replayed three ways.
        let witness = cluster_replay_witness(&store);
        let plan = witness.replay_plan();
        assert_eq!(
            2_usize,
            plan.levels().len(),
            "the certified path schedules two dependency levels"
        );
        assert_eq!(
            CausalDepth::from(plan.levels().len()),
            plan.critical_path(),
            "the critical-path fuel is the number of dependency levels"
        );
        assert_eq!(
            witness.canonical_path().len(),
            plan.levels().iter().map(Vec::len).sum::<usize>(),
            "the plan schedules every certified step exactly once"
        );
        let start = SequentAlphabet::skolemize(witness.peak());
        let sequential = replay_sequentially(&store, &start, &witness.canonical_path());
        let eager = plan
            .replay_with_fuel(&store, plan.critical_path())
            .expect("critical-path fuel does not obstruct the plan")
            .expect("the complete plan returns an outcome");
        assert_eq!(
            sequential, eager,
            "eager planned replay reaches the sequential replay's term"
        );
        let mut on_demand = SequentAlphabet::skolemize(witness.peak());
        for level in 0 .. plan.levels().len() {
            on_demand = plan
                .replay_level(&store, &on_demand, ReplayLevel::from(level))
                .expect("each dependency level replays on demand");
        }
        assert_eq!(
            sequential, on_demand,
            "per-level on-demand replay reaches the sequential replay's term"
        );
        assert_eq!(
            SequentAlphabet::skolemize(witness.joins_at()),
            sequential,
            "and that term is the certified join"
        );
    }

    #[test]
    fn a_relabelled_twin_schedules_and_replays_identically()
    {
        // The twin renames every binder and every metavariable label and fixes
        // every constructor and operation symbol, so nothing name-free about
        // the schedule, the plan, or the completion result may move.
        let store = independent_rule_clusters();
        let twin = relabelled_rule_clusters();
        assert_ne!(
            store, twin,
            "the twin is a genuinely different store, not the same one twice"
        );
        let support = OverlapSupport::from_store(&store);
        let twin_support = OverlapSupport::from_store(&twin);
        let overlaps = confluence_family(&store);
        let twin_overlaps = confluence_family(&twin);
        let batches = support.batches(&overlaps);
        let twin_batches = twin_support.batches(&twin_overlaps);
        assert_eq!(
            batch_shape(&batches),
            batch_shape(&twin_batches),
            "relabelling moves no batch boundary and no batch member"
        );
        assert_eq!(
            batch_input_positions(&overlaps, &batches),
            batch_input_positions(&twin_overlaps, &twin_batches),
            "relabelling moves no flatten position"
        );
        let witness = cluster_replay_witness(&store);
        let twin_witness = cluster_replay_witness(&twin);
        let plan = witness.replay_plan();
        let twin_plan = twin_witness.replay_plan();
        // A `ReplayPlan` also carries the peak it starts from, and the peak is
        // the one part of it that spells metavariable labels — so the plans are
        // equal in their whole scheduling content and unequal as values. That
        // is the finding, not a weakening: the schedule is name-free and the
        // boundary is not.
        assert_eq!(
            plan.levels(),
            twin_plan.levels(),
            "the twin schedules the same cells at the same positions, level for level"
        );
        assert_eq!(
            plan.critical_path(),
            twin_plan.critical_path(),
            "the twin needs the same critical-path fuel"
        );
        let replayed = plan
            .replay_with_fuel(&store, plan.critical_path())
            .expect("critical-path fuel does not obstruct the plan")
            .expect("the complete plan returns an outcome");
        let twin_replayed = twin_plan
            .replay_with_fuel(&twin, twin_plan.critical_path())
            .expect("critical-path fuel does not obstruct the twin plan")
            .expect("the complete twin plan returns an outcome");
        assert_eq!(
            SequentAlphabet::skolemize(witness.joins_at()),
            replayed,
            "the fixture's plan replays to its own certified join"
        );
        assert_eq!(
            SequentAlphabet::skolemize(twin_witness.joins_at()),
            twin_replayed,
            "the twin's plan replays to its own certified join"
        );
        let budget = CompletionBudget::new(64_usize.into(), 64_usize.into(), 64_usize.into());
        assert_eq!(
            completion_shape(&complete(store, budget)),
            completion_shape(&complete(twin, budget)),
            "and the full completion pipeline derives the same cells and emits the same \
             certificate family for both"
        );
    }

    #[test]
    fn overlap_support_is_symmetric_and_certificate_memoized()
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let support = OverlapSupport::from_store(&store);
        assert!(
            !bool::from(support.independent(frame, add)),
            "the frame and add cells have enumerated overlap support, so they are not independent"
        );
        assert!(
            !bool::from(support.independent(add, frame)),
            "and the query answers the same in the other argument order"
        );
        assert_eq!(
            support.independent(frame, add),
            support.independent(add, frame),
            "the cell relation is symmetric"
        );
        let composition = enumerate_overlaps(&store)
            .into_iter()
            .find(|overlap| {
                overlap.kind == OverlapKind::Composition
                    && overlap.left == frame
                    && overlap.right == add
            })
            .expect("the generated composition overlap exists");
        let (_fused, certificate) =
            crate::tracelet::derive_fused(&composition, &mut store).expect("certificate generated");
        let frame_step = certificate
            .path_a
            .iter()
            .find(|step| step.cell == frame)
            .cloned()
            .expect("the certificate path contains the frame step");
        let add_step = certificate
            .path_a
            .iter()
            .find(|step| step.cell == add)
            .cloned()
            .expect("the certificate path contains the add step");
        let mut frame_certificate = certificate.clone();
        frame_certificate.path_a = vec![frame_step.clone()];
        frame_certificate.path_b = vec![frame_step];
        let mut add_certificate = certificate.clone();
        add_certificate.path_a = vec![add_step.clone()];
        add_certificate.path_b = vec![add_step];
        let mut primitive_support = OverlapSupport::from_store(&store);
        primitive_support
            .add_certificates(&[frame_certificate, add_certificate])
            .expect("distinct certificate support range fits");
        assert!(
            !bool::from(primitive_support.certificates_independent(
                CertificateIndex::from(0_usize),
                CertificateIndex::from(1_usize)
            )),
            "distinct certificates inherit dependence from their primitive cells"
        );
        let mut support = OverlapSupport::from_store(&store);
        let first_keys = support
            .add_certificates(core::slice::from_ref(&certificate))
            .expect("first certificate range fits");
        let second_keys = support
            .add_certificates(core::slice::from_ref(&certificate))
            .expect("second certificate range fits");
        assert_eq!(
            first_keys,
            (
                CertificateIndex::from(0_usize),
                CertificateIndex::from(1_usize)
            ),
            "the first insertion takes the opening stable key"
        );
        assert_eq!(
            second_keys,
            (
                CertificateIndex::from(1_usize),
                CertificateIndex::from(2_usize)
            ),
            "and the second takes the next one, monotonically"
        );
        assert!(
            !bool::from(support.certificates_independent(
                CertificateIndex::from(0_usize),
                CertificateIndex::from(0_usize)
            )),
            "a certificate is never independent of itself"
        );
        assert!(
            !bool::from(support.certificates_independent(
                CertificateIndex::from(1_usize),
                CertificateIndex::from(1_usize)
            )),
            "and neither is its separately inserted copy"
        );
        assert!(
            !bool::from(support.certificates_independent(
                CertificateIndex::from(0_usize),
                CertificateIndex::from(1_usize)
            )),
            "identical certificates from separate calls retain cross-support"
        );
        assert_ne!(first_keys, second_keys, "stable keys remain distinct");
        let mut batched = OverlapSupport::from_store(&store);
        let batched_keys = batched
            .add_certificates(&[certificate.clone(), certificate.clone()])
            .expect("batched certificate range fits");
        let mut split = OverlapSupport::from_store(&store);
        let split_first = split
            .add_certificates(core::slice::from_ref(&certificate))
            .expect("split first certificate range fits");
        let split_second = split
            .add_certificates(core::slice::from_ref(&certificate))
            .expect("split second certificate range fits");
        assert_eq!(
            batched_keys,
            (
                CertificateIndex::from(0_usize),
                CertificateIndex::from(2_usize)
            ),
            "one batched call takes the whole half-open key range"
        );
        assert_eq!(
            split_first,
            (
                CertificateIndex::from(0_usize),
                CertificateIndex::from(1_usize)
            ),
            "and splitting it hands out the same range in two pieces"
        );
        assert_eq!(
            split_second,
            (
                CertificateIndex::from(1_usize),
                CertificateIndex::from(2_usize)
            ),
            "the second of those two pieces"
        );
        assert_eq!(
            batched, split,
            "support is invariant under call partitioning"
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

    /// A rule `⟨K | op(label)⟩ ~> ⟨K | rhs⟩` over a nullary constructor `K`.
    fn ground_rule(
        ctor: &Sym,
        op: &Sym,
        label: &Sym,
        rhs: ConsPat,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor(ctor.as_ref(), []),
                ConsPat::op(op.as_ref(), [], ConsPat::meta(label.as_ref())),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::ctor(ctor.as_ref(), []), rhs),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// A rule `⟨binder | op(label)⟩ ~> ⟨binder | rhs⟩` over a producer
    /// metavariable.
    fn schematic_rule(
        binder: &Sym,
        op: &Sym,
        label: &Sym,
        rhs: ConsPat,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(binder.as_ref()),
                ConsPat::op(op.as_ref(), [], ConsPat::meta(label.as_ref())),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::meta(binder.as_ref()), rhs),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// Three two-rule clusters over the disjoint operations `f`, `g` and `h`,
    /// followed by the `p`-rule every joined pair replays through.
    ///
    /// Each cluster's ground rule and schematic rule overlap on their own
    /// operation and on nothing else — no right-hand side head (`p`, `q`, `r`)
    /// is any left-hand side head except `p`'s own rule, which pairs with no
    /// one — so the three critical pairs are mutually independent and a batch
    /// can hold one from each. The labels are parameters so the relabelled twin
    /// is the same call with different names.
    fn labelled_clusters(
        label: &Sym,
        binders: [&Sym; 4],
    ) -> CellStore
    {
        let reduced = ConsPat::op("p", [], ConsPat::meta(label.as_ref()));
        let wrapped = ConsPat::op("q", [], ConsPat::op("p", [], ConsPat::meta(label.as_ref())));
        let (f, g, h, p) = (Sym::new("f"), Sym::new("g"), Sym::new("h"), Sym::new("p"));
        let mut store = CellStore::new();
        store.insert(ground_rule(&Sym::new("Zero"), &f, label, reduced.clone()));
        store.insert(schematic_rule(binders[0], &f, label, reduced.clone()));
        store.insert(ground_rule(&Sym::new("Nil"), &g, label, reduced.clone()));
        store.insert(schematic_rule(binders[1], &g, label, wrapped.clone()));
        store.insert(ground_rule(&Sym::new("Unit"), &h, label, reduced));
        store.insert(schematic_rule(binders[2], &h, label, wrapped));
        store.insert(schematic_rule(
            binders[3],
            &p,
            label,
            ConsPat::op("r", [], ConsPat::meta(label.as_ref())),
        ));
        store
    }

    /// The ordered scheduling fixture.
    fn independent_rule_clusters() -> CellStore
    {
        labelled_clusters(&Sym::new("alpha"), [
            &Sym::new("x"),
            &Sym::new("y"),
            &Sym::new("z"),
            &Sym::new("u"),
        ])
    }

    /// The same fixture with every binder and metavariable label renamed and
    /// every constructor and operation symbol held fixed.
    fn relabelled_rule_clusters() -> CellStore
    {
        labelled_clusters(&Sym::new("gamma"), [
            &Sym::new("w"),
            &Sym::new("t"),
            &Sym::new("s"),
            &Sym::new("n"),
        ])
    }

    /// The confluence family — the overlap kind the completion worklist
    /// batches.
    fn confluence_family(store: &CellStore) -> Vec<Overlap>
    {
        enumerate_overlaps(store)
            .into_iter()
            .filter(|overlap| overlap.kind == OverlapKind::Confluence)
            .collect()
    }

    /// The certified replay witness of the fixture's leading critical pair.
    fn cluster_replay_witness(store: &CellStore) -> ReplayWitness
    {
        let overlaps = confluence_family(store);
        let leading = overlaps
            .first()
            .expect("the fixture enumerates a leading confluence overlap");
        let certificate = confluence_tracelet(leading, store, 64_usize.into())
            .expect("the leading cluster's critical pair joins");
        normalize_certified(
            store,
            &certificate.overlap.peak,
            &certificate.joins_at,
            &certificate.path_a,
        )
        .expect("the joined critical pair replays into a certified witness")
    }

    /// Replay a recorded path one step at a time, independently of any plan.
    fn replay_sequentially(
        store: &CellStore,
        start: &CmdPat,
        path: &[CellApp],
    ) -> CmdPat
    {
        let mut current = start.clone();
        for step in path {
            let cell = store
                .get(step.cell)
                .expect("every recorded step names a live cell");
            current = rewrite_at(cell, &current, &step.at).expect("every recorded step fires");
        }
        current
    }

    /// The position of one overlap in an enumerated overlap family.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct InputPosition(usize);

    /// The name-free shape of a completion outcome: whether it completed, the
    /// cells it derived, how many certificates it emitted, and how large its
    /// store ended up.
    #[derive(Debug, Eq, PartialEq)]
    struct CompletionShape
    {
        /// Whether the run reached convergence.
        completed: CompletionStatus,
        /// The cells the run derived, in derivation order.
        derived: Vec<CellId>,
        /// How many certificates the run emitted.
        certificates: EventCount,
        /// How large the run left its store.
        store: CellCount,
    }

    /// The input position of each batch member, per batch.
    fn batch_input_positions(
        overlaps: &[Overlap],
        batches: &[Vec<Overlap>],
    ) -> Vec<Vec<InputPosition>>
    {
        batches
            .iter()
            .map(|batch| {
                batch
                    .iter()
                    .map(|member| {
                        overlaps
                            .iter()
                            .position(|candidate| candidate == member)
                            .map(InputPosition)
                            .expect("every batch member came from the input family")
                    })
                    .collect()
            })
            .collect()
    }

    /// The name-free shape of a batch partition: each member's ordered cell
    /// endpoints and overlap kind.
    fn batch_shape(batches: &[Vec<Overlap>]) -> Vec<Vec<(CellId, CellId, OverlapKind)>>
    {
        batches
            .iter()
            .map(|batch| {
                batch
                    .iter()
                    .map(|overlap| (overlap.left, overlap.right, overlap.kind))
                    .collect()
            })
            .collect()
    }

    /// The name-free shape of one completion outcome.
    fn completion_shape(outcome: &CompletionOutcome) -> CompletionShape
    {
        match *outcome {
            | CompletionOutcome::Completed {
                ref store,
                ref derived,
                ref certificates,
            } => CompletionShape {
                completed: CompletionStatus::from(true),
                derived: derived.clone(),
                certificates: EventCount::from(certificates.len()),
                store: store.len(),
            },
            | CompletionOutcome::Declined {
                ref store,
                ref derived,
                ref certificates,
                ..
            } => CompletionShape {
                completed: CompletionStatus::from(false),
                derived: derived.clone(),
                certificates: EventCount::from(certificates.len()),
                store: store.len(),
            },
        }
    }
}
