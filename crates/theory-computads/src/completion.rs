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

use crate::alphabet::CellAlphabet;
use crate::boundary::CompletionCellBudget;
use crate::boundary::CompletionStatus;
use crate::boundary::CompletionStepBudget;
use crate::boundary::NormalizationBudget;
use crate::cell::Cell;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::overlap::Overlap;
use crate::overlap::OverlapKind;
use crate::overlap::OverlapSupport;
use crate::overlap::enumerate_overlaps;
use crate::rewrite::normalize;
use crate::sequent::SequentAlphabet;
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
    /// Completion declined on a budget ceiling, carrying the pending overlap
    /// batches in their deterministic first-appearance order.
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
        /// Which ceiling was reached.
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

    /// Resume a declined completion without rebuilding its pending partition.
    ///
    /// # Contract
    /// - ensures: a declined result resumes exactly its pending FIFO batches; a
    ///   completed result is returned unchanged.
    /// - provides: transparent decline/resume semantics.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - witness: `completion::tests::decline_resume_matches_uninterrupted_completion`.
    #[inline]
    #[must_use]
    pub fn resume(
        self,
        budget: CompletionBudget,
    ) -> Self
    {
        match self {
            | Self::Completed { .. } => self,
            | Self::Declined {
                store,
                derived,
                certificates,
                pending,
                ..
            } => complete_with_worklist(store, budget, derived, certificates, pending.into()),
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
    let initial_worklist: VecDeque<Vec<Overlap<A>>> = scheduled_confluence_batches(&store).into();
    complete_with_worklist(store, budget, Vec::new(), Vec::new(), initial_worklist)
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
fn scheduled_confluence_batches<A>(store: &CellStore<A>) -> Vec<Vec<Overlap<A>>>
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
    fn every_generated_certificate_matches_its_replay_plan()
    {
        let outcome = complete(
            overlapping_rules(),
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        let CompletionOutcome::Completed {
            store,
            certificates,
            ..
        } = outcome
        else {
            panic!("the generated fixture completes within budget");
        };
        assert!(
            !certificates.is_empty(),
            "the completion fixture emits a generated certificate family"
        );
        assert_eq!(
            certificates.len(),
            1,
            "the generated fixture emits its exact one-certificate family"
        );
        for certificate in certificates {
            let witness_a = crate::normal_form::normalize_certified(
                &store,
                &certificate.overlap.peak,
                &certificate.joins_at,
                &certificate.path_a,
            )
            .expect("every generated certificate path_a replays");
            let witness_b = crate::normal_form::normalize_certified(
                &store,
                &certificate.overlap.peak,
                &certificate.joins_at,
                &certificate.path_b,
            )
            .expect("every generated certificate path_b replays");
            assert_eq!(
                witness_a.normal_form().joins_at,
                witness_b.normal_form().joins_at,
                "both generated certificate paths reach the same join"
            );
            let plan_a = witness_a.replay_plan();
            let plan_b = witness_b.replay_plan();
            // The two legs of a confluence certificate are different
            // derivations of one boundary — one fires the left cell and
            // whatever normalizes its reduct, the other the right cell and
            // whatever normalizes its own — so their plans schedule different
            // cells. The shared invariant is the join they replay to.
            assert_ne!(
                plan_a.levels(),
                plan_b.levels(),
                "the two certificate legs are different derivations of one boundary"
            );
            let planned_a = plan_a
                .replay_with_fuel(&store, plan_a.critical_path())
                .expect("planned path_a replay does not obstruct")
                .expect("critical-path fuel completes path_a");
            let planned_b = plan_b
                .replay_with_fuel(&store, plan_b.critical_path())
                .expect("planned path_b replay does not obstruct")
                .expect("critical-path fuel completes path_b");
            assert_eq!(
                planned_a, planned_b,
                "planned replay agrees for both generated certificate paths"
            );
            // A plan replays the *skolemized* peak, so it lands on the
            // skolemized join: the certified join still carries the critical
            // pair's metavariables.
            let join = SequentAlphabet::skolemize(&certificate.joins_at);
            assert_eq!(
                join, planned_a,
                "planned replay matches path_a for every generated certificate"
            );
            assert_eq!(
                join, planned_b,
                "planned replay matches path_b for every generated certificate"
            );
        }
    }

    #[test]
    fn cell_budget_decline_preserves_pending_work()
    {
        // The cell ceiling and the step ceiling are reached at the same point
        // in this fixture — the leading batch's second overlap, the first one
        // that diverges — so the two declines must carry the same structural
        // residue and differ only in the reason they carry.
        let scheduled = scheduled_confluence_batches(&independent_rule_clusters());
        let outcome = complete(
            independent_rule_clusters(),
            CompletionBudget::new(64_usize.into(), 6_usize.into(), 64_usize.into()),
        );
        match outcome {
            | CompletionOutcome::Declined {
                reason,
                ref pending,
                ref derived,
                ref certificates,
                ..
            } => {
                assert_eq!(
                    DeclineReason::CellBudget,
                    reason,
                    "the cell ceiling, not the step ceiling, is what stopped this run"
                );
                assert!(
                    derived.is_empty(),
                    "the ceiling is reached before the divergence is oriented, so nothing was \
                     derived"
                );
                assert_eq!(
                    1_usize,
                    certificates.len(),
                    "the joinable leading pair was certified before the ceiling was reached"
                );
                assert_eq!(
                    residue_after_leading_step(&scheduled),
                    *pending,
                    "the cell ceiling preserves the remainder of the leading batch and every \
                     later batch unchanged"
                );
            },
            | CompletionOutcome::Completed { .. } => {
                panic!("the cell ceiling must decline before inserting a derived rule")
            },
        }
    }

    #[test]
    fn decline_resume_matches_uninterrupted_completion()
    {
        let scheduled = scheduled_confluence_batches(&independent_rule_clusters());
        assert_eq!(
            2_usize,
            scheduled.len(),
            "the fixture schedules its six critical pairs into two independent batches"
        );
        assert!(
            scheduled
                .first()
                .is_some_and(|batch| batch.len() == 3_usize),
            "the leading batch holds three independent overlaps, so a one-step budget stops \
             inside it rather than between batches"
        );
        let uninterrupted = complete(
            independent_rule_clusters(),
            CompletionBudget::new(64_usize.into(), 64_usize.into(), 64_usize.into()),
        );
        let declined = complete(
            independent_rule_clusters(),
            CompletionBudget::new(1_usize.into(), 64_usize.into(), 64_usize.into()),
        );
        match declined {
            | CompletionOutcome::Declined {
                reason,
                ref pending,
                ref derived,
                ..
            } => {
                assert_eq!(
                    DeclineReason::StepBudget,
                    reason,
                    "the step ceiling is what stopped this run"
                );
                assert!(
                    derived.is_empty(),
                    "the one step taken was the joinable leading pair, which derives nothing"
                );
                // Exactly one step was taken, so the residue is the leading
                // batch minus its first member, then every later batch
                // untouched. A partition that drops the overlap the ceiling
                // interrupted, or that drops the unfinished batch wholesale,
                // fails here.
                assert_eq!(
                    residue_after_leading_step(&scheduled),
                    *pending,
                    "the decline carries the unprocessed remainder of the leading batch and \
                     every later batch unchanged"
                );
            },
            | CompletionOutcome::Completed { .. } => {
                panic!("a one-step budget must decline inside the leading batch")
            },
        }
        let resumed = declined.resume(CompletionBudget::new(
            64_usize.into(),
            64_usize.into(),
            64_usize.into(),
        ));
        assert!(
            bool::from(resumed.is_completed()),
            "resuming from the carried batches finishes the whole worklist"
        );
        assert_eq!(
            uninterrupted, resumed,
            "and reaches the uninterrupted outcome exactly: same store, same derived cells, same \
             certificates"
        );
    }

    #[test]
    fn a_starved_budget_declines_with_pending()
    {
        let initial = overlapping_rules();
        let expected = scheduled_confluence_batches(&initial);
        let outcome = complete(
            initial,
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
                assert_eq!(
                    expected, pending,
                    "budget decline preserves batch and first-appearance order"
                );
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
    /// A rule `⟨K | op(α)⟩ ~> ⟨K | rhs⟩` over a nullary constructor `K`.
    fn ground_rule(
        ctor: &Sym,
        op: &Sym,
        rhs: ConsPat,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor(ctor.as_ref(), []),
                ConsPat::op(op.as_ref(), [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::ctor(ctor.as_ref(), []), rhs),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// A rule `⟨binder | op(α)⟩ ~> ⟨binder | rhs⟩` over a producer
    /// metavariable.
    fn schematic_rule(
        binder: &Sym,
        op: &Sym,
        rhs: ConsPat,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(binder.as_ref()),
                ConsPat::op(op.as_ref(), [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::meta(binder.as_ref()), rhs),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// Three two-rule clusters over the disjoint operations `f`, `g` and `h`.
    ///
    /// Each cluster overlaps on its own operation and on nothing else, so the
    /// six critical pairs schedule into two batches of three. The leading
    /// cluster's pair joins outright (both rules reduce to `p`); the other two
    /// diverge by size and orient into a derived cell each — so a budget that
    /// stops after one step stops inside the leading batch, before anything is
    /// derived.
    fn independent_rule_clusters() -> CellStore
    {
        let reduced = ConsPat::op("p", [], ConsPat::meta("alpha"));
        let wrapped = ConsPat::op("q", [], ConsPat::op("p", [], ConsPat::meta("alpha")));
        let (f, g, h) = (Sym::new("f"), Sym::new("g"), Sym::new("h"));
        let mut store = CellStore::new();
        store.insert(ground_rule(&Sym::new("Zero"), &f, reduced.clone()));
        store.insert(schematic_rule(&Sym::new("x"), &f, reduced.clone()));
        store.insert(ground_rule(&Sym::new("Nil"), &g, reduced.clone()));
        store.insert(schematic_rule(&Sym::new("y"), &g, wrapped.clone()));
        store.insert(ground_rule(&Sym::new("Unit"), &h, reduced));
        store.insert(schematic_rule(&Sym::new("z"), &h, wrapped));
        store
    }

    /// The pending residue a decline one step into the leading batch must
    /// carry: that batch's unprocessed remainder, then every later batch
    /// unchanged.
    fn residue_after_leading_step(scheduled: &[Vec<Overlap>]) -> Vec<Vec<Overlap>>
    {
        let mut residue = Vec::with_capacity(scheduled.len());
        residue.push(
            scheduled
                .first()
                .expect("the schedule has a leading batch")
                .get(1 ..)
                .expect("the leading batch has a processed head")
                .to_vec(),
        );
        residue.extend(scheduled.iter().skip(1).cloned());
        residue
    }
}
