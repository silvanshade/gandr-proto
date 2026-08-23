//! **Static pathway queries** — goal-directed synthesis of the compressed
//! derivations that can end in a target cell, computed without evaluating any
//! state.
//!
//! A pathway is a certificate whose last event fires the queried cell and whose
//! earlier events cannot be rearranged to fire it sooner. The search grows
//! pathways backwards from the target: each round composes a surviving pathway
//! with one transition certificate, keeps the composites the
//! [`target_occurs_only_last`] condition admits, and compresses what survives
//! to its normal form. No initial term is supplied and nothing is rewritten, so
//! the answer is a statement about the rule set rather than about a run.
//!
//! # The condition is decided locally
//!
//! Stated over presentations, the condition quantifies over an equivalence
//! class: no rearrangement of the derivation may fire the target earlier. That
//! class is never enumerated here. [`EventOrder::precedes`] is the transitive
//! closure of the direct dependence edges and the canonical schedule is one of
//! its linear extensions, so the rearrangements of a derivation are exactly the
//! linear extensions of its causal order. The condition is therefore an order
//! property: the target fires at exactly one event, and every other event
//! precedes it.
//!
//! # What an acceptance is worth, and what a refutation is worth
//!
//! The two directions are not symmetric, and the verdict type says so rather
//! than leaving it to prose. Independence is *granted* by the shift guard only
//! where the guard can discharge it, so concurrency is earned: an event that
//! does not precede the target witnesses a linear extension that places it
//! after the target, and [`TargetLast::NotLast`] is sound. The converse fails —
//! a conservatively added dependence edge makes maximality easier to satisfy —
//! so [`TargetLast::HoldsUnderGuard`] is relative to the guard and
//! over-approximates the pathways that exist. Its name carries that.
//!
//! # Budget
//!
//! Exhausting a ceiling is a typed decline carrying its report and the frontier
//! the search stopped on, never a truncated success, following the same shape
//! as [`gandr_theory_coherent_resolutions::completion`]. The frontier is what a
//! later continuation would need; this module exposes no entry point that takes
//! one back, so a declined query is **re-asked with a larger budget** rather
//! than resumed in place.

use alloc::vec::Vec;

use gandr_theory_cell_complexes::alphabet::CellAlphabet;
use gandr_theory_cell_complexes::boundary::EventIndex;
use gandr_theory_cell_complexes::boundary::PathwayCandidateBudget;
use gandr_theory_cell_complexes::boundary::PathwayCandidateCount;
use gandr_theory_cell_complexes::boundary::PathwayCompression;
use gandr_theory_cell_complexes::boundary::PathwayLength;
use gandr_theory_cell_complexes::boundary::PathwayLengthBudget;
use gandr_theory_cell_complexes::boundary::TargetEventCount;
use gandr_theory_cell_complexes::cell::CellId;
use gandr_theory_cell_complexes::cell::CellStore;
use gandr_theory_cell_complexes::sequent::SequentAlphabet;
use gandr_theory_coherent_resolutions::rewrite::CellApp;
use gandr_theory_coherent_resolutions::tracelet::Tracelet;
use gandr_theory_deep_inference::causal::EventOrder;
use gandr_theory_deep_inference::normal_form::NormalFormObstruction;
use gandr_theory_deep_inference::normal_form::TraceletNf;
use gandr_theory_deep_inference::normal_form::normalize_certified;

use crate::compose::compose_directed;

/// The verdict of the target-occurs-only-last condition on one derivation.
///
/// The positive variant is named for what it is worth: an acceptance holds
/// against the independence relation the shift guard granted, which is a sound
/// under-approximation of concurrency, so the accepted set over-approximates
/// the pathways that exist. A refutation carries no such qualification.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TargetLast
{
    /// No event fires the target cell, so the derivation is not a pathway to
    /// it.
    TargetAbsent,
    /// More than one event fires the target cell. Some linear extension places
    /// one of them before the other, so the target is not confined to the end.
    TargetRepeated
    {
        /// How many events fire the target.
        fired: TargetEventCount,
    },
    /// An event neither is the target's event nor precedes it, so a linear
    /// extension places it after the target. **Sound**: the guard granted the
    /// concurrency this reads.
    NotLast,
    /// The target fires once and every other event precedes it, **relative to
    /// the independence relation the shift guard granted**.
    HoldsUnderGuard,
}

/// Decide the target-occurs-only-last condition on a recorded derivation's
/// causal order.
///
/// # Contract
/// - requires: `order` is the order of a derivation over the store `target` was
///   interned in, so that a cell identity comparison is meaningful.
/// - ensures: [`TargetLast::HoldsUnderGuard`] exactly when one event fires
///   `target` and every other event precedes it; [`TargetLast::TargetRepeated`]
///   when more than one does; [`TargetLast::TargetAbsent`] when none does;
///   [`TargetLast::NotLast`] when the single firing event has a non-predecessor
///   beside it.
/// - provides: the condition as an order property, without building any
///   rearrangement of the derivation.
/// - panics: none.
/// - intension: one pass to locate the firing events and one pass over the
///   remaining events asking [`EventOrder::precedes`]; no recursion, so
///   derivation length cannot reach the stack.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the decision surface is the four arms plus the
///   count boundary at one firing event, separated by a derivation whose target
///   is absent, one where it fires twice, one where it fires last over a
///   dependent past, and one where a non-predecessor sits beside it.
/// - witness: `pathway::tests::a_derivation_without_the_target_is_not_a_pathway`
/// - witness: `pathway::tests::a_target_firing_twice_is_refused`
/// - witness: `pathway::tests::a_target_last_over_a_dependent_past_holds`
/// - witness: `pathway::tests::a_non_predecessor_beside_the_target_refutes`
#[inline]
#[must_use]
pub fn target_occurs_only_last<A>(
    order: &EventOrder<A>,
    target: CellId,
) -> TargetLast
where
    A: CellAlphabet,
{
    let mut fired: Vec<usize> = Vec::new();
    for (index, event) in order.events().iter().enumerate() {
        if event.step().cell == target {
            fired.push(index);
        }
    }
    let Some((last, earlier)) = fired.split_first()
    else {
        return TargetLast::TargetAbsent;
    };
    if !earlier.is_empty() {
        return TargetLast::TargetRepeated {
            fired: TargetEventCount::from(fired.len()),
        };
    }
    let last = EventIndex::from(*last);
    for index in 0 .. order.events().len() {
        let index = EventIndex::from(index);
        if index == last {
            continue;
        }
        if !bool::from(order.precedes(index, last)) {
            return TargetLast::NotLast;
        }
    }
    TargetLast::HoldsUnderGuard
}

/// The ceilings a pathway search runs under.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PathwayBudget
{
    /// The longest pathway the search may compose.
    pub max_length: PathwayLengthBudget,
    /// The most composite candidates the search may build.
    pub max_candidates: PathwayCandidateBudget,
}

impl PathwayBudget
{
    /// A budget from its two ceilings.
    #[inline]
    #[must_use]
    pub const fn new(
        max_length: PathwayLengthBudget,
        max_candidates: PathwayCandidateBudget,
    ) -> Self
    {
        Self {
            max_length,
            max_candidates,
        }
    }
}

/// Which ceiling a declined search reached.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PathwayDeclineReason
{
    /// The length ceiling ([`PathwayBudget::max_length`]) was reached with the
    /// frontier still growing.
    LengthBudget,
    /// The candidate ceiling ([`PathwayBudget::max_candidates`]) was reached.
    CandidateBudget,
}

/// One synthesized pathway: the certificate and its compressed form.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pathway<A: CellAlphabet = SequentAlphabet>
{
    /// The certificate whose last event fires the queried cell.
    pub certificate: Tracelet<A>,
    /// Its normal form — the compressed representative of its class.
    pub normal_form: TraceletNf<A>,
    /// The causal order the condition was decided on — the same order
    /// certification took, rather than a second one built beside it.
    pub order: EventOrder<A>,
    /// How many composed steps it holds.
    pub length: PathwayLength,
}

/// What a pathway search returns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PathwayOutcome<A: CellAlphabet = SequentAlphabet>
{
    /// The search exhausted its rounds within budget.
    Complete
    {
        /// Every pathway found, compressed, shortest rounds first.
        pathways: Vec<Pathway<A>>,
    },
    /// A ceiling was reached. The pathways already found are returned beside
    /// the frontier the next round would have extended, so the answer says
    /// where it stopped rather than silently truncating. No entry point takes
    /// a frontier back yet, so continuing means re-asking with a larger
    /// budget.
    Declined
    {
        /// The pathways found before the ceiling.
        pathways: Vec<Pathway<A>>,
        /// The frontier the search stopped on.
        frontier: Vec<Pathway<A>>,
        /// Which ceiling was reached.
        reason: PathwayDeclineReason,
        /// How many candidates had been built.
        built: PathwayCandidateCount,
    },
}

impl<A: CellAlphabet> PathwayOutcome<A>
{
    /// The pathways found, whether or not the search declined.
    #[inline]
    #[must_use]
    pub fn pathways(&self) -> &[Pathway<A>]
    {
        match *self {
            | Self::Complete { ref pathways } | Self::Declined { ref pathways, .. } => pathways,
        }
    }
}

/// Synthesize the compressed pathways that can end in `target`.
///
/// # Contract
/// - requires: `seed` is a certificate whose only event fires `target`, and
///   `transitions` are the certificates of the rules that may precede it; all
///   are interned in `store`.
/// - ensures: every returned pathway satisfies [`target_occurs_only_last`]
///   under the guard, is in normal form, and is distinct from every other
///   returned pathway's normal form. Composites the directed composition
///   refuses, and composites that fail the condition, are dropped rather than
///   returned.
/// - provides: the answer as a statement about the rule set — nothing is
///   evaluated on a term and no initial object is supplied.
/// - fails: a composite whose certification cannot be taken surfaces the
///   normal-form obstruction; reaching a ceiling is
///   [`PathwayOutcome::Declined`] rather than a failure.
/// - panics: none.
/// - intension: rounds are breadth-first in pathway length, so pathways are
///   returned shortest first; the loop is iterative over an explicit frontier.
///
/// # Errors
/// Returns the [`NormalFormObstruction`] raised while certifying a composite —
/// a derivation that does not replay, or an event key collision.
///
/// # Adequacy
/// - hypothesis: L1 evidence plus L3 pointwise — each returned pathway carries
///   a normal form validated by certification rather than predicted, and the
///   decision residue is the two ceilings and the compression, separated by a
///   search that declines on length, one that declines on candidates, and one
///   whose duplicate composites collapse to a single representative.
/// - witness: `pathway::tests::a_length_ceiling_declines_with_its_frontier`
/// - witness: `pathway::tests::a_zero_candidate_ceiling_builds_nothing`
/// - witness: `pathway::tests::a_candidate_ceiling_admits_exactly_its_count`
/// - witness: `pathway::tests::synthesized_pathways_are_returned_in_normal_form`
#[inline]
pub fn synthesize_pathways<A>(
    store: &CellStore<A>,
    seed: &Tracelet<A>,
    target: CellId,
    transitions: &[Tracelet<A>],
    budget: PathwayBudget,
) -> Result<PathwayOutcome<A>, NormalFormObstruction<A>>
where
    A: CellAlphabet,
{
    let certified = normalize_certified(store, &seed.overlap.peak, &seed.joins_at, &seed.path_a)?;
    let order = certified.event_order().clone();
    let seed = Pathway {
        certificate: seed.clone(),
        normal_form: certified.into_normal_form(),
        order,
        length: PathwayLength::from(1),
    };
    let mut found: Vec<Pathway<A>> = Vec::new();
    let mut frontier: Vec<Pathway<A>> = alloc::vec![seed];
    let mut built: usize = 0;
    let max_length = usize::from(budget.max_length);
    let max_candidates = usize::from(budget.max_candidates);
    let mut length: usize = 1;
    while !frontier.is_empty() {
        found.extend(frontier.iter().cloned());
        if length >= max_length {
            return Ok(PathwayOutcome::Declined {
                pathways: found,
                frontier,
                reason: PathwayDeclineReason::LengthBudget,
                built: PathwayCandidateCount::from(built),
            });
        }
        let Some(next_length) = length.checked_add(1)
        else {
            return Ok(PathwayOutcome::Declined {
                pathways: found,
                frontier,
                reason: PathwayDeclineReason::LengthBudget,
                built: PathwayCandidateCount::from(built),
            });
        };
        let mut grown: Vec<Pathway<A>> = Vec::new();
        for pathway in &frontier {
            for transition in transitions {
                // The ceiling is the maximum number of candidates built, so
                // it is checked *before* the next attempt: a ceiling of zero
                // declines having built nothing.
                if built >= max_candidates {
                    return Ok(PathwayOutcome::Declined {
                        pathways: found,
                        frontier,
                        reason: PathwayDeclineReason::CandidateBudget,
                        built: PathwayCandidateCount::from(built),
                    });
                }
                let Some(count) = built.checked_add(1)
                else {
                    return Ok(PathwayOutcome::Declined {
                        pathways: found,
                        frontier,
                        reason: PathwayDeclineReason::CandidateBudget,
                        built: PathwayCandidateCount::from(built),
                    });
                };
                built = count;
                // The transition fires first and the pathway after it, so the
                // target stays at the composite's end.
                let Ok(composite) = compose_directed(transition, &pathway.certificate, store)
                else {
                    continue;
                };
                // A composite that composition admitted but that does not
                // replay is not a pathway, so it is dropped. An obstruction
                // that reports a defect rather than describing the candidate
                // reaches the caller instead — `obstruction_severity` owns
                // that split, and this `?` is why it has to.
                let Some(certified) = certify_candidate(
                    store,
                    &composite.overlap.peak,
                    &composite.joins_at,
                    &composite.path_a,
                )?
                else {
                    continue;
                };
                if target_occurs_only_last(&certified.order, target) != TargetLast::HoldsUnderGuard
                {
                    continue;
                }
                let candidate = Pathway {
                    certificate: composite,
                    normal_form: certified.normal_form,
                    order: certified.order,
                    length: PathwayLength::from(next_length),
                };
                if bool::from(compressed_in(&grown, &candidate))
                    || bool::from(compressed_in(&found, &candidate))
                {
                    continue;
                }
                grown.push(candidate);
            }
        }
        frontier = grown;
        length = next_length;
    }
    Ok(PathwayOutcome::Complete { pathways: found })
}

/// Whether an obstruction refuses one candidate or stops the whole query.
///
/// The distinction is the difference between a search answering "not this one"
/// and a search continuing over a broken invariant. Dropping every obstruction
/// alike would turn a kill signal into "no pathway found", which is the one
/// reading those signals exist to prevent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ObstructionSeverity
{
    /// The composite is simply not a pathway. The search drops it and goes on.
    RefusesCandidate,
    /// A kill signal, or a store or invariant violation. Never worked around.
    StopsTheQuery,
}

/// Classify a normal-form obstruction for a **speculative** candidate.
///
/// # Contract
/// - ensures: [`ObstructionSeverity::RefusesCandidate`] exactly for the arms
///   that describe the candidate rather than the machinery — a path that does
///   not fire, a path that misses its join, and the two digest-collision
///   refusals, which decline a normal form rather than report a defect.
///   Everything else is [`ObstructionSeverity::StopsTheQuery`]: both
///   shifted-schedule kill signals, which say the independence relation
///   licensed a transposition the semantics does not have, and the store and
///   replay-bounds violations.
/// - provides: the one place the search is allowed to swallow an obstruction,
///   so the set of swallowed arms is stated rather than implied by a
///   `continue`.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the decision surface is the arm partition, and
///   every arm is asserted exactly, because an arm silently moving to the
///   refusing side is precisely the defect this function exists to prevent.
/// - witness: `pathway::tests::every_obstruction_arm_is_classified_exactly`
/// - witness: `pathway::tests::a_kill_signal_stops_the_query_rather_than_refusing_a_candidate`
#[inline]
#[must_use]
pub const fn obstruction_severity<A>(obstruction: &NormalFormObstruction<A>) -> ObstructionSeverity
where
    A: CellAlphabet,
{
    match *obstruction {
        | NormalFormObstruction::StepDoesNotFire { .. }
        | NormalFormObstruction::PathMissesTheJoin { .. }
        | NormalFormObstruction::ContentAddressCollision { .. }
        | NormalFormObstruction::CanonicalKeyCollision { .. } => {
            ObstructionSeverity::RefusesCandidate
        },
        | NormalFormObstruction::ShiftedScheduleDoesNotFire { .. }
        | NormalFormObstruction::ShiftedScheduleMissesTheJoin { .. }
        | NormalFormObstruction::UnknownCell { .. }
        | NormalFormObstruction::InvalidReplayLevel { .. } => ObstructionSeverity::StopsTheQuery,
    }
}

/// What certifying a speculative candidate produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifiedCandidate<A: CellAlphabet = SequentAlphabet>
{
    /// The compressed representative of the candidate's class.
    pub normal_form: TraceletNf<A>,
    /// The causal order certification took — the same order the normal form
    /// was built from, rather than a second one built beside it.
    pub order: EventOrder<A>,
}

/// Certify one **speculative** candidate derivation.
///
/// # Contract
/// - requires: `path` is a recorded derivation from `peak` over cells `store`
///   holds.
/// - ensures: `Ok(Some(certified))` carrying the normal form and the causal
///   order certification itself took, so the condition is decided on the same
///   order the normal form was; `Ok(None)` when the obstruction refuses the
///   candidate.
/// - provides: the search's single swallow point, classified rather than
///   implicit.
/// - fails: an obstruction [`obstruction_severity`] calls
///   [`ObstructionSeverity::StopsTheQuery`] is returned to the caller — a kill
///   signal reaches the caller rather than reading as no pathway.
/// - panics: none.
///
/// # Errors
/// Returns the obstruction whenever it stops the query rather than refusing the
/// candidate.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the returned values are the ones certification
///   produced rather than values assembled beside them; the refusal/propagation
///   split is separated by a candidate that merely misses its join and by one
///   raising a kill signal.
/// - witness: `pathway::tests::a_candidate_that_misses_its_join_is_refused`
/// - witness: `pathway::tests::a_kill_signal_stops_the_query_rather_than_refusing_a_candidate`
#[inline]
pub fn certify_candidate<A>(
    store: &CellStore<A>,
    peak: &A::Cmd,
    joins_at: &A::Cmd,
    path: &[CellApp<A>],
) -> Result<Option<CertifiedCandidate<A>>, NormalFormObstruction<A>>
where
    A: CellAlphabet,
{
    let witness = match normalize_certified(store, peak, joins_at, path) {
        | Ok(witness) => witness,
        | Err(obstruction) => {
            return match obstruction_severity(&obstruction) {
                | ObstructionSeverity::RefusesCandidate => Ok(None),
                | ObstructionSeverity::StopsTheQuery => Err(obstruction),
            };
        },
    };
    let order = witness.event_order().clone();
    Ok(Some(CertifiedCandidate {
        normal_form: witness.into_normal_form(),
        order,
    }))
}

/// Whether a pathway's compressed form is already represented.
///
/// # Contract
/// - ensures: positive exactly when some held pathway has the same normal form,
///   which is the compression step — one representative per class.
/// - panics: none.
#[inline]
fn compressed_in<A>(
    held: &[Pathway<A>],
    candidate: &Pathway<A>,
) -> PathwayCompression
where
    A: CellAlphabet,
{
    PathwayCompression::from(
        held.iter()
            .any(|pathway| pathway.normal_form == candidate.normal_form),
    )
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_cell_complexes::alphabet::ConvexityDischarge;
    use gandr_theory_cell_complexes::cell::Cell;
    use gandr_theory_cell_complexes::pattern::CmdPat;
    use gandr_theory_cell_complexes::pattern::ConsPat;
    use gandr_theory_cell_complexes::pattern::Pos;
    use gandr_theory_cell_complexes::pattern::ProdPat;
    use gandr_theory_cell_complexes::sequent::CellProvenance;
    use gandr_theory_cell_complexes::sequent::Orientation;
    use gandr_theory_coherent_resolutions::overlap::enumerate_overlaps;
    use gandr_theory_coherent_resolutions::rewrite::CellApp;
    use gandr_theory_deep_inference::causal::DerivationEvent;
    use gandr_theory_deep_inference::normal_form::prim_address;

    use super::*;

    /// (add-S): the one rule every fixture here fires.
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

    /// A store holding one rule, and that rule's id.
    fn one_rule_store() -> (CellStore, CellId)
    {
        let mut store = CellStore::new();
        let add = store.insert(add_s());
        (store, add)
    }

    /// An order over the given `(cell, position)` steps, built the way the
    /// normalizer builds one so the guard decides the dependences.
    fn order_over(
        store: &CellStore,
        steps: &[(CellId, Pos)],
    ) -> EventOrder
    {
        let events = steps
            .iter()
            .map(|step| {
                let application = CellApp {
                    cell: step.0,
                    at: step.1.clone(),
                };
                let address = store.get(step.0).map_or_else(
                    || prim_address(&add_s(), &step.1),
                    |resolved| prim_address(resolved, &step.1),
                );
                DerivationEvent::new(application, address)
            })
            .collect();
        EventOrder::of_events(
            store,
            events,
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
        )
    }

    #[test]
    fn a_derivation_without_the_target_is_not_a_pathway()
    {
        let (mut store, add) = one_rule_store();
        let other = store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("p"),
                ConsPat::meta("beta"),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("p"),
                ConsPat::meta("beta"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        let order = order_over(&store, &[(add, Pos::root())]);
        assert_eq!(
            target_occurs_only_last(&order, other),
            TargetLast::TargetAbsent
        );
    }

    #[test]
    fn a_target_firing_twice_is_refused()
    {
        let (store, add) = one_rule_store();
        let order = order_over(&store, &[(add, Pos::root()), (add, Pos::root())]);
        assert_eq!(
            target_occurs_only_last(&order, add),
            TargetLast::TargetRepeated {
                fired: TargetEventCount::from(2),
            }
        );
    }

    #[test]
    fn a_target_last_over_a_dependent_past_holds()
    {
        let (mut store, add) = one_rule_store();
        let target = store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("n"),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        let order = order_over(&store, &[(add, Pos::root()), (target, Pos::root())]);
        assert_eq!(
            target_occurs_only_last(&order, target),
            TargetLast::HoldsUnderGuard
        );
    }

    #[test]
    fn a_non_predecessor_beside_the_target_refutes()
    {
        let (mut store, add) = one_rule_store();
        let target = store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("n"),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        // Incomparable positions, so the guard grants independence and the
        // order leaves the target without a predecessor beside it.
        let order = order_over(&store, &[
            (target, Pos(alloc::vec![0].into_boxed_slice())),
            (add, Pos(alloc::vec![1].into_boxed_slice())),
        ]);
        assert_eq!(target_occurs_only_last(&order, target), TargetLast::NotLast);
    }

    /// A one-step seed certificate over the store's first self-overlap.
    ///
    /// Construction is **mandatory**: a fixture that cannot be built is a
    /// broken fixture, and a test that skipped itself here would pass without
    /// witnessing anything it claims.
    fn seed_over(
        store: &CellStore,
        cell: CellId,
    ) -> Tracelet
    {
        let overlap = enumerate_overlaps(store)
            .into_iter()
            .next()
            .expect("the one-rule store has a self-overlap to seed from");
        let step = CellApp {
            cell,
            at: Pos::root(),
        };
        let provisional = Tracelet {
            overlap,
            path_a: alloc::vec![step.clone()],
            path_b: alloc::vec![step],
            joins_at: CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("m"),
                ConsPat::meta("alpha"),
            ),
        };
        let gandr_theory_coherent_resolutions::tracelet::ReplayPathOutcome::Reached(reached) =
            provisional.replay_trace(store).path_a.outcome
        else {
            panic!("the seed's single step applies at the overlap's peak")
        };
        Tracelet {
            joins_at: reached,
            ..provisional
        }
    }

    #[test]
    fn a_length_ceiling_declines_with_its_frontier()
    {
        let (store, add) = one_rule_store();
        let seed = seed_over(&store, add);
        let budget = PathwayBudget::new(
            PathwayLengthBudget::from(1),
            PathwayCandidateBudget::from(64),
        );
        let outcome = synthesize_pathways(&store, &seed, add, core::slice::from_ref(&seed), budget)
            .expect("the seed certifies");
        let PathwayOutcome::Declined {
            ref frontier,
            reason,
            ..
        } = outcome
        else {
            panic!("a length ceiling of one declines on the seed round")
        };
        assert_eq!(reason, PathwayDeclineReason::LengthBudget);
        assert_eq!(frontier.len(), 1);
        assert_eq!(outcome.pathways().len(), 1);
    }

    #[test]
    fn every_obstruction_arm_is_classified_exactly()
    {
        let (store, add) = one_rule_store();
        let cell = store.get(add).expect("the rule is stored");
        let step: CellApp = CellApp {
            cell: add,
            at: Pos::root(),
        };
        let cert = gandr_theory_deep_inference::normal_form::PrimCert(step.clone());
        let term = cell.lhs.clone();
        // Refusals: the obstruction describes the candidate.
        for obstruction in [
            NormalFormObstruction::StepDoesNotFire {
                step: alloc::boxed::Box::new(step.clone()),
            },
            NormalFormObstruction::PathMissesTheJoin {
                reached: alloc::boxed::Box::new(term.clone()),
            },
            NormalFormObstruction::ContentAddressCollision {
                address: prim_address(cell, &Pos::root()),
                held: alloc::boxed::Box::new(cert.clone()),
                offered: alloc::boxed::Box::new(cert.clone()),
            },
            NormalFormObstruction::CanonicalKeyCollision {
                earlier: alloc::boxed::Box::new(cert.clone()),
                later: alloc::boxed::Box::new(cert),
                depth: gandr_theory_cell_complexes::boundary::CausalDepth::from(0),
            },
        ] {
            assert_eq!(
                obstruction_severity(&obstruction),
                ObstructionSeverity::RefusesCandidate,
                "this arm describes the candidate: {obstruction:?}"
            );
        }
        // Defects: a kill signal, or a broken store or bound. Swallowing any of
        // these would read as "no pathway found".
        for obstruction in [
            NormalFormObstruction::ShiftedScheduleDoesNotFire {
                step: alloc::boxed::Box::new(step),
            },
            NormalFormObstruction::ShiftedScheduleMissesTheJoin {
                reached: alloc::boxed::Box::new(term),
            },
            NormalFormObstruction::UnknownCell { cell: add },
            NormalFormObstruction::InvalidReplayLevel {
                level: gandr_theory_cell_complexes::boundary::ReplayLevel::from(0),
                levels: gandr_theory_cell_complexes::boundary::CausalDepth::from(0),
            },
        ] {
            assert_eq!(
                obstruction_severity(&obstruction),
                ObstructionSeverity::StopsTheQuery,
                "this arm reports a defect: {obstruction:?}"
            );
        }
    }

    #[test]
    fn a_candidate_that_misses_its_join_is_refused()
    {
        let (store, add) = one_rule_store();
        let seed = seed_over(&store, add);
        // The recorded path replays to its own join, so asking it to land
        // somewhere else refuses the candidate rather than failing the query.
        let elsewhere = store.get(add).expect("the rule is stored").lhs.clone();
        let refused = certify_candidate(&store, &seed.overlap.peak, &elsewhere, &seed.path_a)
            .expect("missing the join refuses the candidate, it does not stop the query");
        assert!(refused.is_none());
    }

    #[test]
    fn a_zero_candidate_ceiling_builds_nothing()
    {
        let (store, add) = one_rule_store();
        let seed = seed_over(&store, add);
        let budget = PathwayBudget::new(
            PathwayLengthBudget::from(8),
            PathwayCandidateBudget::from(0),
        );
        let outcome = synthesize_pathways(&store, &seed, add, core::slice::from_ref(&seed), budget)
            .expect("the seed certifies");
        let PathwayOutcome::Declined { reason, built, .. } = outcome
        else {
            panic!("a candidate ceiling of zero declines before the first attempt")
        };
        assert_eq!(reason, PathwayDeclineReason::CandidateBudget);
        // The ceiling is the maximum *built*, so nothing was attempted.
        assert_eq!(usize::from(built), 0);
    }

    #[test]
    fn a_candidate_ceiling_admits_exactly_its_count()
    {
        let (store, add) = one_rule_store();
        let seed = seed_over(&store, add);
        let transitions = alloc::vec![seed.clone(), seed.clone(), seed.clone()];
        let budget = PathwayBudget::new(
            PathwayLengthBudget::from(8),
            PathwayCandidateBudget::from(2),
        );
        let outcome = synthesize_pathways(&store, &seed, add, &transitions, budget)
            .expect("the seed certifies");
        let PathwayOutcome::Declined { reason, built, .. } = outcome
        else {
            panic!("three transitions against a ceiling of two decline")
        };
        assert_eq!(reason, PathwayDeclineReason::CandidateBudget);
        // Exactly the ceiling was built, and the attempt that would have
        // exceeded it never ran.
        assert_eq!(usize::from(built), 2);
    }

    #[test]
    fn synthesized_pathways_are_returned_in_normal_form()
    {
        let (store, add) = one_rule_store();
        let seed = seed_over(&store, add);
        let budget = PathwayBudget::new(
            PathwayLengthBudget::from(1),
            PathwayCandidateBudget::from(4),
        );
        let outcome =
            synthesize_pathways(&store, &seed, add, &[], budget).expect("the seed certifies");
        for pathway in outcome.pathways() {
            let path = pathway
                .normal_form
                .canonical_path()
                .expect("a certified normal form decompresses");
            assert!(!path.is_empty());
        }
    }
}
