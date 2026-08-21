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
//! off-TCB (the module-level warning of
//! [`gandr_theory_cell_complexes::alphabet`]).

use alloc::vec::Vec;
use std::collections::HashSet;

use gandr_theory_cell_complexes::alphabet::CellAlphabet;
use gandr_theory_cell_complexes::boundary::CertificateIndex;
use gandr_theory_cell_complexes::boundary::StepIndependence;
use gandr_theory_cell_complexes::boundary::SubstitutionDecision;
use gandr_theory_cell_complexes::cell::Cell;
use gandr_theory_cell_complexes::cell::CellId;
use gandr_theory_cell_complexes::cell::CellStore;
use gandr_theory_cell_complexes::sequent::SequentAlphabet;

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
    pub fn overlaps_are_independent<A>(
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
    /// Build a confluence overlap from evidence supplied by an external
    /// matcher.
    ///
    /// A domain matcher may know a substitution that is not derivable from
    /// this alphabet's generic `unify_cmd` operation. The supplied source is
    /// therefore the authority for deriving the overlap's substitution; this
    /// constructor records that evidence without re-running the generic
    /// matcher. The completion entry point still validates the overlap kind,
    /// both store addresses, and that the supplied substitution makes the
    /// apart-renamed right left-hand side agree with the peak before it accepts
    /// the worklist.
    ///
    /// # Contract
    /// - requires: `left` and `right` address cells in the store observed by
    ///   the caller, and `unifier`/`seam` are the external matcher's induced
    ///   sequent evidence.
    /// - ensures: the returned overlap is a confluence overlap carrying the
    ///   supplied ids, unifier and seam; its peak is the supplied unifier
    ///   applied to the left-hand side, and its right leg is renamed apart
    ///   exactly as ordinary overlap enumeration does.
    /// - provides: the matcher-neutral construction needed by
    ///   `complete_with_overlap_source` without adding matcher vocabulary to
    ///   this crate.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn from_supplied_confluence(
        left: (CellId, &Cell<A>),
        right: (CellId, &Cell<A>),
        unifier: A::Subst,
        seam: A::Pos,
    ) -> Self
    {
        let (id_left, cell_left) = left;
        let (id_right, cell_right) = right;
        let (renamed_lhs, renamed_rhs) = A::rename_apart(
            (&cell_left.lhs, &cell_left.rhs),
            (&cell_right.lhs, &cell_right.rhs),
        );
        let right_renamed = Cell::new(
            renamed_lhs,
            renamed_rhs,
            cell_right.orient,
            cell_right.provenance,
        );
        let peak = A::apply_subst(&unifier, &cell_left.lhs);
        Self {
            left: id_left,
            right: id_right,
            kind: OverlapKind::Confluence,
            unifier,
            seam,
            peak,
            right_renamed,
        }
    }

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

    /// Whether the supplied substitution makes the apart-renamed right
    /// left-hand side meet the left peak.
    ///
    /// # Contract
    /// - ensures: `true` exactly when both confluence legs apply to the same
    ///   supplied peak under `self.unifier`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub(crate) fn matches_peak(&self) -> SubstitutionDecision
    {
        SubstitutionDecision::from(
            A::apply_subst(&self.unifier, &self.right_renamed.lhs) == self.peak,
        )
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
/// store-wide sweep.
/// `gandr_theory_deep_inference::shift::derive_shift_equivalence` is that
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
