//! **Knuth–Bendix / Squier completion** over the convergent slice, with an
//! explicit **completion budget**.
//!
//! Spec: the sequent-machines design's §7.3.3. Generic over
//! the [`CellAlphabet`] (the executed meta-spike-01).
//!
//! [`complete`] runs confluence completion: it normalizes both reducts of every
//! [`OverlapKind::Confluence`] critical pair; a joinable pair emits a coherence
//! [`Tracelet`]; a non-joinable pair is **oriented** by the reduction order
//! ([`CellAlphabet::reduction_cmp`]) into a new derived cell, whose fresh
//! overlaps re-enter the worklist. Termination is bounded by the
//! [`CompletionBudget`]: exhausting the step or cell budget is a **defined
//! decline carrying the pending overlap batches
//! ([`CompletionOutcome::Declined`]), never divergence or a panic — the same
//! posture as the machine's step budget (§7.3.3, §4.1). The three honest
//! obstruction classes stay part of the contract: a normalization-budget
//! exhaustion, an unorientable equal-size divergence, and a duplicate derived
//! cell are each *left*, never guessed at.
//!
//! Fusion (the "requested shortcut" of §7.3.3) is the separate
//! [`crate::tracelet::derive_fused`] on an [`OverlapKind::Composition`]
//! overlap; completion here is the confluence engine.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use gandr_theory_cell_complexes::alphabet::CellAlphabet;
use gandr_theory_cell_complexes::boundary::CompletionCellBudget;
use gandr_theory_cell_complexes::boundary::CompletionStatus;
use gandr_theory_cell_complexes::boundary::CompletionStepBudget;
use gandr_theory_cell_complexes::boundary::NormalizationBudget;
use gandr_theory_cell_complexes::cell::Cell;
use gandr_theory_cell_complexes::cell::CellId;
use gandr_theory_cell_complexes::cell::CellStore;
use gandr_theory_cell_complexes::sequent::SequentAlphabet;

use crate::overlap::Overlap;
use crate::overlap::OverlapKind;
use crate::overlap::OverlapSupport;
use crate::overlap::enumerate_overlaps;
use crate::rewrite::normalize;
use crate::tracelet::Tracelet;
use crate::tracelet::confluence_tracelet;

/// The **completion budget** — the ceilings that make completion terminate with
/// a defined decline rather than diverge (`proposal-sequent-kernel.md` §7.3.3).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
    /// The supplied source violated the completion input contract.
    InvalidSuppliedOverlap(SuppliedOverlapError),
}

/// The first invalid item in a caller-supplied overlap worklist.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SuppliedOverlapError
{
    /// A batch supplied an overlap of the composition kind.
    NonConfluence
    {
        /// The zero-based batch containing the overlap.
        batch: usize,
        /// The zero-based overlap within that batch.
        overlap: usize,
    },
    /// The overlap's left id is not in the supplied store.
    UnknownLeftCell
    {
        /// The zero-based batch containing the overlap.
        batch: usize,
        /// The zero-based overlap within that batch.
        overlap: usize,
        /// The stale or foreign id.
        cell: CellId,
    },
    /// The overlap's right id is not in the supplied store.
    UnknownRightCell
    {
        /// The zero-based batch containing the overlap.
        batch: usize,
        /// The zero-based overlap within that batch.
        overlap: usize,
        /// The stale or foreign id.
        cell: CellId,
    },
    /// The supplied substitution does not make the two confluence legs meet
    /// at one peak.
    NonUnifyingSubstitution
    {
        /// The zero-based batch containing the overlap.
        batch: usize,
        /// The zero-based overlap within that batch.
        overlap: usize,
    },
}

/// The outcome of [`complete`] — either a completed convergent slice or a
/// defined decline carrying what was left (`proposal-sequent-kernel.md`
/// §7.3.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompletionOutcome<A: CellAlphabet = SequentAlphabet>
{
    /// Completion ran the whole worklist within budget.
    Completed
    {
        /// The final cell store (the original cells plus every oriented cell).
        store: CellStore<A>,
        /// The ids of the cells completion derived, in derivation order.
        derived: Vec<CellId>,
        /// The coherence certificates emitted for joinable critical pairs.
        certificates: Vec<Tracelet<A>>,
    },
    /// Completion declined with either a budget ceiling or invalid supplied
    /// input, carrying what remains unprocessed.
    Declined
    {
        /// The cell store as of the decline.
        store: CellStore<A>,
        /// The cells derived before the decline.
        derived: Vec<CellId>,
        /// The certificates emitted before the decline.
        certificates: Vec<Tracelet<A>>,
        /// The independent overlap batches left unprocessed.
        pending: Vec<Vec<Overlap<A>>>,
        /// The budget ceiling or typed supplied-input refusal.
        reason: DeclineReason,
    },
}

impl<A: CellAlphabet> CompletionOutcome<A>
{
    /// The final (or as-of-decline) cell store.
    ///
    /// # Contract
    /// - ensures: a reference to the store of whichever variant this is.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn store(&self) -> &CellStore<A>
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
    pub fn certificates(&self) -> &[Tracelet<A>]
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

    /// Resume a budget-declined completion without rebuilding its pending
    /// partition; invalid supplied-input declines are terminal.
    ///
    /// # Contract
    /// - ensures: a step- or cell-budget decline resumes exactly its pending
    ///   FIFO batches; an invalid-supplied-input decline is returned unchanged;
    ///   a completed result is returned unchanged.
    /// - provides: transparent decline/resume semantics without allowing an
    ///   invalid overlap to reach the confluence worklist.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — budget declines resume to the uninterrupted result,
    ///   while every invalid supplied-input variant remains a typed terminal
    ///   refusal under resume.
    /// - witness: `completion::tests::decline_resume_matches_uninterrupted_completion`.
    /// - witness: `completion::tests::invalid_supplied_decline_is_terminal_on_resume`.
    #[inline]
    #[must_use]
    pub fn resume(
        self,
        budget: CompletionBudget,
    ) -> Self
    {
        match self {
            // A completed outcome has nothing to resume, and an invalid-supply
            // decline is terminal by contract: resuming it is the defect this
            // repair closes.
            | Self::Completed { .. }
            | Self::Declined {
                reason: DeclineReason::InvalidSuppliedOverlap(_),
                ..
            } => self,
            | Self::Declined {
                store,
                derived,
                certificates,
                pending,
                reason: DeclineReason::StepBudget | DeclineReason::CellBudget,
            } => {
                if let Err(error) = validate_supplied_batches(&store, &pending) {
                    return Self::Declined {
                        store,
                        derived,
                        certificates,
                        pending,
                        reason: DeclineReason::InvalidSuppliedOverlap(error),
                    };
                }
                complete_with_worklist(store, budget, derived, certificates, pending.into())
            },
        }
    }
}

/// Run **confluence completion** over `store` under `budget`.
///
/// # Contract
/// - ensures: [`CompletionOutcome::Completed`] when every confluence critical
///   pair (including those of derived cells) was processed within budget — each
///   joinable pair contributing a certificate, each divergence an oriented
///   derived cell; [`CompletionOutcome::Declined`] carrying the pending overlap
///   batches when the step or cell ceiling is reached. The loop always
///   terminates in at most `budget.max_steps` iterations, never diverging or
///   panicking.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the sequent-alphabet pins (completion within
///   budget, starved-budget decline carrying pending) hold verbatim through the
///   generic loop, and the toy alphabet drives the same loop to an oriented
///   derived cell plus a replaying certificate.
/// - witness: `completion::tests::completion_processes_within_budget`
/// - witness: `completion::tests::a_starved_budget_declines_with_pending`
/// - witness: `toy_alphabet::tests::completion_orients_and_certifies_over_the_toy_alphabet`
#[inline]
#[must_use]
pub fn complete<A>(
    store: CellStore<A>,
    budget: CompletionBudget,
) -> CompletionOutcome<A>
where
    A: CellAlphabet,
{
    complete_with_overlap_source(store, budget, |store| scheduled_confluence_batches(store))
}

/// Run completion with a caller-supplied initial overlap source.
///
/// The source is invoked once, before the generic completion loop starts. Its
/// batches seed the same worklist that [`complete`] uses; any cells derived by
/// the loop are scheduled through the generic [`scheduled_confluence_batches`]
/// relation. This is the instantiation seam for a consumer whose pattern
/// matcher supplies the initial critical-pair family without entering this
/// crate's dependency graph.
///
/// # Contract
/// - requires: every supplied overlap is a [`OverlapKind::Confluence`] whose
///   `left` and `right` ids address cells in the supplied store, and whose
///   supplied substitution makes the apart-renamed right left-hand side agree
///   with the peak.
/// - ensures: valid input makes the completion loop process exactly the
///   supplied initial batches, then the generic confluence batches for cells it
///   derives, within the same step and cell budgets as [`complete`]. Invalid
///   input returns [`CompletionOutcome::Declined`] with
///   [`DeclineReason::InvalidSuppliedOverlap`], preserves the unprocessed
///   supplied batches, and remains terminal under
///   [`CompletionOutcome::resume`].
/// - provides: a matcher-neutral supply point; the source sees the generic
///   [`CellStore`] and returns generic [`Overlap`] values.
/// - panics: none.
#[inline]
#[must_use]
pub fn complete_with_overlap_source<A, F>(
    store: CellStore<A>,
    budget: CompletionBudget,
    source: F,
) -> CompletionOutcome<A>
where
    A: CellAlphabet,
    F: FnOnce(&CellStore<A>) -> Vec<Vec<Overlap<A>>>,
{
    let initial_batches = source(&store);
    if let Err(error) = validate_supplied_batches(&store, &initial_batches) {
        return CompletionOutcome::Declined {
            store,
            derived: Vec::new(),
            certificates: Vec::new(),
            pending: initial_batches,
            reason: DeclineReason::InvalidSuppliedOverlap(error),
        };
    }
    let initial_worklist: VecDeque<Vec<Overlap<A>>> = initial_batches.into();
    complete_with_worklist(store, budget, Vec::new(), Vec::new(), initial_worklist)
}

/// Validate a caller-supplied worklist before it reaches the completion loop.
///
/// # Contract
/// - ensures: `Ok(())` iff every item is a confluence, both of its ids are in
///   `store`, and its supplied substitution makes the apart-renamed right
///   left-hand side agree with the peak; the first invalid item is reported in
///   batch order.
/// - panics: none.
#[inline]
fn validate_supplied_batches<A>(
    store: &CellStore<A>,
    batches: &[Vec<Overlap<A>>],
) -> Result<(), SuppliedOverlapError>
where
    A: CellAlphabet,
{
    for (batch, overlaps) in batches.iter().enumerate() {
        for (overlap, item) in overlaps.iter().enumerate() {
            if item.kind != OverlapKind::Confluence {
                return Err(SuppliedOverlapError::NonConfluence { batch, overlap });
            }
            if store.get(item.left).is_none() {
                return Err(SuppliedOverlapError::UnknownLeftCell {
                    batch,
                    overlap,
                    cell: item.left,
                });
            }
            if store.get(item.right).is_none() {
                return Err(SuppliedOverlapError::UnknownRightCell {
                    batch,
                    overlap,
                    cell: item.right,
                });
            }
            if !bool::from(item.matches_peak()) {
                return Err(SuppliedOverlapError::NonUnifyingSubstitution { batch, overlap });
            }
        }
    }
    Ok(())
}

/// Run the completion loop from a given worklist and accumulated results.
///
/// This is the shared body of [`complete`] and
/// [`CompletionOutcome::resume`]: a fresh run enters it with the scheduled
/// batches and empty accumulators, a resumed one with the batches its decline
/// carried and the store, derived cells, and certificates it had already
/// reached. Decline and resume are therefore transparent by construction
/// rather than by agreement between two loops.
///
/// # Contract
/// - ensures: the same outcome the uninterrupted loop reaches from this state,
///   terminating in at most `budget.max_steps` iterations.
/// - panics: none.
fn complete_with_worklist<A>(
    mut store: CellStore<A>,
    budget: CompletionBudget,
    mut derived: Vec<CellId>,
    mut certificates: Vec<Tracelet<A>>,
    mut worklist: VecDeque<Vec<Overlap<A>>>,
) -> CompletionOutcome<A>
where
    A: CellAlphabet,
{
    let mut steps: usize = 0;
    while let Some(batch) = worklist.pop_front() {
        let mut index = 0;
        while let Some(overlap) = batch.get(index).cloned() {
            if steps >= usize::from(budget.max_steps) {
                let mut pending = Vec::with_capacity(worklist.len().saturating_add(1));
                pending.push(batch.get(index ..).unwrap_or_default().to_vec());
                pending.extend(worklist);
                return CompletionOutcome::Declined {
                    store,
                    derived,
                    certificates,
                    pending,
                    reason: DeclineReason::StepBudget,
                };
            }
            steps = steps.saturating_add(1);
            let (Some(left_reduct), Some(right_reduct)) =
                (overlap.left_reduct(&store), overlap.right_reduct(&store))
            else {
                index = index.saturating_add(1);
                continue;
            };
            let norm_left = normalize(&store, &left_reduct, budget.norm_budget);
            let norm_right = normalize(&store, &right_reduct, budget.norm_budget);
            if norm_left.exhausted || norm_right.exhausted {
                // A reduct did not reach a normal form within the budget — an
                // obstruction we cannot decide; leave it (the pair is not certified
                // and not oriented).
                index = index.saturating_add(1);
                continue;
            }
            if norm_left.normal == norm_right.normal {
                if let Some(tracelet) = confluence_tracelet(&overlap, &store, budget.norm_budget) {
                    certificates.push(tracelet);
                }
                index = index.saturating_add(1);
                continue;
            }
            // Orient the divergence into a new rule (Knuth–Bendix).
            let ordered = orient::<A>(norm_left.normal, norm_right.normal);
            let Some((bigger, smaller)) = ordered
            else {
                // Equal-size divergence: incomparable by this order; an obstruction
                // left for a stronger order (a listed limit, not a failure).
                index = index.saturating_add(1);
                continue;
            };
            let new_cell = Cell::new(
                bigger,
                smaller,
                A::derived_orientation(),
                A::derived_provenance(),
            );
            let already_present = store.iter().any(|(_, cell)| *cell == new_cell);
            if already_present {
                index = index.saturating_add(1);
                continue;
            }
            if usize::from(store.len()) >= usize::from(budget.max_cells) {
                let mut pending = Vec::with_capacity(worklist.len().saturating_add(1));
                pending.push(batch.get(index ..).unwrap_or_default().to_vec());
                pending.extend(worklist);
                return CompletionOutcome::Declined {
                    store,
                    derived,
                    certificates,
                    pending,
                    reason: DeclineReason::CellBudget,
                };
            }
            let id = store.insert(new_cell);
            derived.push(id);
            worklist.extend(scheduled_confluence_batches(&store).into_iter().filter(
                |candidate_batch| {
                    candidate_batch
                        .iter()
                        .any(|candidate| candidate.left == id || candidate.right == id)
                },
            ));
            index = index.saturating_add(1);
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
fn confluence_overlaps<A>(store: &CellStore<A>) -> Vec<Overlap<A>>
where
    A: CellAlphabet,
{
    enumerate_overlaps(store)
        .into_iter()
        .filter(|overlap| overlap.kind == OverlapKind::Confluence)
        .collect()
}

/// Schedule confluence overlaps into deterministic independent batches.
///
/// # Contract
/// - ensures: batch boundaries are preserved for the completion worklist.
/// - provides: independent overlap units without flattening serialization.
/// - panics: none.
#[inline]
#[must_use]
pub fn scheduled_confluence_batches<A>(store: &CellStore<A>) -> Vec<Vec<Overlap<A>>>
where
    A: CellAlphabet,
{
    let overlaps = confluence_overlaps(store);
    let support = OverlapSupport::from_store(store);
    support.batches(&overlaps)
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
fn orient<A>(
    left: A::Cmd,
    right: A::Cmd,
) -> Option<(A::Cmd, A::Cmd)>
where
    A: CellAlphabet,
{
    match A::reduction_cmp(&left, &right) {
        | core::cmp::Ordering::Greater => Some((left, right)),
        | core::cmp::Ordering::Less => Some((right, left)),
        | core::cmp::Ordering::Equal => None,
    }
}
