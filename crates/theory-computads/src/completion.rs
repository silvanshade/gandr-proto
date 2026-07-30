//! **Knuth–Bendix / Squier completion** over the convergent slice, with an
//! explicit **completion budget** (`proposal-sequent-kernel.md` §7.3.3; ADR-46
//! Decision B).
//!
//! [`complete`] runs confluence completion: it normalizes both reducts of every
//! [`OverlapKind::Confluence`] critical pair; a joinable pair emits a coherence
//! [`Tracelet`]; a non-joinable pair is **oriented** by the reduction order
//! ([`crate::pattern::reduction_cmp`]) into a new derived cell, whose fresh
//! overlaps re-enter the worklist. Termination is bounded by the
//! [`CompletionBudget`]: exhausting the step or cell budget is a **defined
//! decline** carrying the pending overlaps ([`CompletionOutcome::Declined`]),
//! never divergence or a panic — the same posture as the machine's step budget
//! (§7.3.3, §4.1).
//!
//! Fusion (the "requested shortcut" of §7.3.3) is the separate
//! [`crate::tracelet::derive_fused`] on an [`OverlapKind::Composition`]
//! overlap; completion here is the confluence engine.

use alloc::vec::Vec;

use crate::boundary::CompletionCellBudget;
use crate::boundary::CompletionStatus;
use crate::boundary::CompletionStepBudget;
use crate::boundary::NormalizationBudget;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellProvenance;
use crate::cell::CellStore;
use crate::cell::Orientation;
use crate::overlap::Overlap;
use crate::overlap::OverlapKind;
use crate::overlap::enumerate_overlaps;
use crate::pattern::reduction_cmp;
use crate::rewrite::normalize;
use crate::tracelet::Tracelet;
use crate::tracelet::confluence_tracelet;

/// The **completion budget** — the ceilings that make completion terminate with
/// a defined decline rather than diverge (`proposal-sequent-kernel.md` §7.3.3).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_structs,
    reason = "the budget is exactly its three ceilings; a caller sets all three and the engine reads them"
)]
pub struct CompletionBudget
{
    /// The maximum number of critical pairs the loop processes.
    pub max_steps: CompletionStepBudget,
    /// The maximum number of cells the store may hold before completion
    /// declines.
    pub max_cells: CompletionCellBudget,
    /// The per-normalization step budget (passed to [`normalize`]).
    pub norm_budget: NormalizationBudget,
}

impl CompletionBudget
{
    /// A budget from its three ceilings.
    #[inline]
    #[must_use]
    pub fn new(
        max_steps: CompletionStepBudget,
        max_cells: CompletionCellBudget,
        norm_budget: NormalizationBudget,
    ) -> Self
    {
        Self {
            max_steps,
            max_cells,
            norm_budget,
        }
    }
}

/// Why completion declined (`proposal-sequent-kernel.md` §7.3.3, decline-and-
/// report).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeclineReason
{
    /// The step ceiling ([`CompletionBudget::max_steps`]) was reached.
    StepBudget,
    /// The cell ceiling ([`CompletionBudget::max_cells`]) was reached.
    CellBudget,
}

/// The outcome of [`complete`] — either a completed convergent slice or a
/// defined decline carrying what was left (`proposal-sequent-kernel.md`
/// §7.3.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompletionOutcome
{
    /// Completion ran the whole worklist within budget.
    Completed
    {
        /// The final cell store (the original cells plus every oriented cell).
        store: CellStore,
        /// The ids of the cells completion derived, in derivation order.
        derived: Vec<CellId>,
        /// The coherence certificates emitted for joinable critical pairs.
        certificates: Vec<Tracelet>,
    },
    /// Completion declined on a budget ceiling, carrying the pending overlaps.
    Declined
    {
        /// The cell store as of the decline.
        store: CellStore,
        /// The cells derived before the decline.
        derived: Vec<CellId>,
        /// The certificates emitted before the decline.
        certificates: Vec<Tracelet>,
        /// The overlaps left unprocessed (what remained).
        pending: Vec<Overlap>,
        /// Which ceiling was reached.
        reason: DeclineReason,
    },
}

impl CompletionOutcome
{
    /// The final (or as-of-decline) cell store.
    ///
    /// # Contract
    /// - ensures: a reference to the store of whichever variant this is.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn store(&self) -> &CellStore
    {
        match *self {
            | Self::Completed { ref store, .. } | Self::Declined { ref store, .. } => store,
        }
    }

    /// The coherence certificates emitted.
    ///
    /// # Contract
    /// - ensures: a reference to the certificates of whichever variant this is.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn certificates(&self) -> &[Tracelet]
    {
        match *self {
            | Self::Completed {
                ref certificates, ..
            }
            | Self::Declined {
                ref certificates, ..
            } => certificates,
        }
    }

    /// The ids of the derived (oriented) cells.
    ///
    /// # Contract
    /// - ensures: a reference to the derived-cell ids of whichever variant this
    ///   is.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn derived(&self) -> &[CellId]
    {
        match *self {
            | Self::Completed { ref derived, .. } | Self::Declined { ref derived, .. } => derived,
        }
    }

    /// Whether completion ran to convergence (as opposed to declining).
    ///
    /// # Contract
    /// - ensures: `true` iff this is [`CompletionOutcome::Completed`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn is_completed(&self) -> CompletionStatus
    {
        CompletionStatus::from(matches!(*self, Self::Completed { .. }))
    }
}

/// Run **confluence completion** over `store` under `budget`.
///
/// # Contract
/// - ensures: [`CompletionOutcome::Completed`] when every confluence critical
///   pair (including those of derived cells) was processed within budget — each
///   joinable pair contributing a certificate, each divergence an oriented
///   derived cell; [`CompletionOutcome::Declined`] carrying the pending
///   overlaps when the step or cell ceiling is reached. The loop always
///   terminates in at most `budget.max_steps` iterations, never diverging or
///   panicking.
/// - panics: none.
#[inline]
#[must_use]
pub fn complete(
    mut store: CellStore,
    budget: CompletionBudget,
) -> CompletionOutcome
{
    let mut derived: Vec<CellId> = Vec::new();
    let mut certificates: Vec<Tracelet> = Vec::new();
    let mut worklist: Vec<Overlap> = confluence_overlaps(&store);
    let mut steps: usize = 0;
    while let Some(overlap) = worklist.pop() {
        if steps >= usize::from(budget.max_steps) {
            worklist.push(overlap);
            return CompletionOutcome::Declined {
                store,
                derived,
                certificates,
                pending: worklist,
                reason: DeclineReason::StepBudget,
            };
        }
        steps = steps.saturating_add(1);
        let (Some(left_reduct), Some(right_reduct)) =
            (overlap.left_reduct(&store), overlap.right_reduct(&store))
        else {
            continue;
        };
        let norm_left = normalize(&store, &left_reduct, budget.norm_budget);
        let norm_right = normalize(&store, &right_reduct, budget.norm_budget);
        if norm_left.exhausted || norm_right.exhausted {
            // A reduct did not reach a normal form within the budget — an
            // obstruction we cannot decide; leave it (the pair is not certified
            // and not oriented).
            continue;
        }
        if norm_left.normal == norm_right.normal {
            if let Some(tracelet) = confluence_tracelet(&overlap, &store, budget.norm_budget) {
                certificates.push(tracelet);
            }
            continue;
        }
        // Orient the divergence into a new rule (Knuth–Bendix).
        let ordered = orient(norm_left.normal, norm_right.normal);
        let Some((bigger, smaller)) = ordered
        else {
            // Equal-size divergence: incomparable by this order; an obstruction
            // left for a stronger order (a listed limit, not a failure).
            continue;
        };
        let new_cell = Cell::new(
            bigger,
            smaller,
            Orientation::CompletionDerived,
            CellProvenance::DerivedByCompletion,
        );
        let already_present = store.iter().any(|(_, cell)| *cell == new_cell);
        if already_present {
            continue;
        }
        if usize::from(store.len()) >= usize::from(budget.max_cells) {
            return CompletionOutcome::Declined {
                store,
                derived,
                certificates,
                pending: worklist,
                reason: DeclineReason::CellBudget,
            };
        }
        let id = store.insert(new_cell);
        derived.push(id);
        for candidate in confluence_overlaps(&store) {
            if candidate.left == id || candidate.right == id {
                worklist.push(candidate);
            }
        }
    }
    CompletionOutcome::Completed {
        store,
        derived,
        certificates,
    }
}

/// The confluence overlaps of a store (the completion worklist seed).
///
/// # Contract
/// - ensures: exactly the [`OverlapKind::Confluence`] entries of
///   [`enumerate_overlaps`], in the same deterministic order.
/// - panics: none.
#[inline]
fn confluence_overlaps(store: &CellStore) -> Vec<Overlap>
{
    enumerate_overlaps(store)
        .into_iter()
        .filter(|overlap| overlap.kind == OverlapKind::Confluence)
        .collect()
}

/// Orient a divergent pair `(left, right)` by the reduction order, larger side
/// on the left of the new rule.
///
/// # Contract
/// - ensures: `Some((bigger, smaller))` when the two terms differ in size;
///   `None` when they are equal in size (incomparable by the size order, left
///   as an obstruction).
/// - panics: none.
#[inline]
fn orient(
    left: crate::pattern::CmdPat,
    right: crate::pattern::CmdPat,
) -> Option<(crate::pattern::CmdPat, crate::pattern::CmdPat)>
{
    match reduction_cmp(&left, &right) {
        | core::cmp::Ordering::Greater => Some((left, right)),
        | core::cmp::Ordering::Less => Some((right, left)),
        | core::cmp::Ordering::Equal => None,
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;

    use super::*;
    use crate::cell::CellProvenance;
    use crate::cell::Orientation;
    use crate::pattern::CmdPat;
    use crate::pattern::ConsPat;
    use crate::pattern::ProdPat;

    #[test]
    fn completion_processes_within_budget()
    {
        let outcome = complete(
            overlapping_rules(),
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        assert!(
            bool::from(outcome.is_completed()),
            "the small system completes within budget"
        );
    }

    #[test]
    fn a_starved_budget_declines_with_pending()
    {
        let outcome = complete(
            overlapping_rules(),
            CompletionBudget::new(0_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        match outcome {
            | CompletionOutcome::Declined {
                reason, pending, ..
            } => {
                assert_eq!(
                    DeclineReason::StepBudget,
                    reason,
                    "the step ceiling was zero"
                );
                assert!(!pending.is_empty(), "the pending overlaps are carried");
            },
            | CompletionOutcome::Completed { .. } => {
                panic!("a zero step budget must decline")
            },
        }
    }

    /// Two rules that overlap on `⟨Zero | f(α)⟩` with divergent right-hand
    /// sides — a genuine critical pair completion must resolve.
    fn overlapping_rules() -> CellStore
    {
        // r1: ⟨Zero | f(α)⟩ ~> ⟨Zero | α⟩
        let r1 = Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("f", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        );
        // r2: ⟨x | f(α)⟩ ~> ⟨x | g(α)⟩ (a broader rule overlapping r1 at x=Zero)
        let r2 = Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::op("f", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::op("g", [], ConsPat::meta("alpha")),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        );
        let mut store = CellStore::new();
        store.insert(r1);
        store.insert(r2);
        store
    }
}
