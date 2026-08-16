//! The **earned shift-equivalence witness** — the identification the
//! interchange story earns per redex pair, and never imposes.
//!
//! Spec: `spec:implementation/circuit-terms.md`,
//! `circuit-terms-question-15` and `circuit-terms-spike-07` (the decided
//! guard); `spec:metatheory.md` (the hazard the guard answers).
//!
//! Two adjacent applications at disjoint positions with trivial overlap
//! commute, so a two-redex body's two sequentializations are **one** composite
//! transformation rather than two. That identification is TCB-adjacent — it is
//! what a `cells_equal` normal-form fast path would decide — so it is granted
//! per pair, against a guard, and refused when the guard does not hold.
//!
//! # The guard, and why it is three conjuncts rather than one
//!
//! Disjointness of supports does **not** imply independence once matches must
//! be convex: two rule applications on disjoint hyperedge sets can block one
//! another, because applying one creates a directed path that destroys the
//! other's convexity. The counterexample and the statement numbers behind
//! every claim below are cited at `circuit-terms-spike-07`.
//!
//! So the guard is:
//!
//! 1. **the positions are incomparable** — neither position a prefix of the
//!    other ([`CellAlphabet::position_order`]), which is what makes the two
//!    applications address disjoint subtrees;
//! 2. **the cell pair has trivial overlap** — the overlap enumerator's own
//!    verdict, taken per ordered pair in both orders
//!    ([`crate::overlap::overlaps_between`]);
//! 3. **each match image is still convex in the other's reduct** — the conjunct
//!    the first two do not give you, and the only new one.
//!
//! # The third conjunct is carried, not computed
//!
//! Convexity of one match is a directed reachability sweep over the whole
//! target, and it is keyed by *term and position*, so unlike the overlap
//! conjunct it cannot be cached across terms. On a store certified
//! **left-connected** over an **acyclic** target it is provably constant-true
//! and is skipped rather than run — and the witness carries the name of that
//! warrant ([`ConvexityDischarge`]) instead of a recomputed sweep. That
//! certificate is the first inhabitant of the tractability witness the engine
//! side owes; carrying it is what keeps the identification evidence-bearing
//! rather than assumed.
//!
//! The discharge is a fence over today's alphabet and never a refutation of
//! the hazard: it rests on premises about *this* grammar. An alphabet that
//! admits multi-output or disconnected left-hand sides breaks the forcing
//! argument, must answer [`ConvexityDischarge::ReCheckRequired`], and is
//! refused the witness here until the re-check is built.
//!
//! # Where the extension is empty today, and why that is not a defect
//!
//! Over [`crate::sequent::SequentAlphabet`] a command pattern is a single cut
//! whose children are a producer and a consumer, so the *only* command
//! position in a term is the root: two applications can never be incomparable,
//! and the shift quotient's extension over the sequent alphabet is empty. The
//! guard is therefore a specification the alphabet has not yet reached rather
//! than a patch to an existing fast path, exactly as the spike recorded — and
//! it is live the moment an alphabet nests commands, which is what a
//! circuit-shaped body does.
//!
//! # Cost, and the falsifier the cheapness claim can lose to
//!
//! One guarded commutation costs what one replay step costs, so the guard
//! stays ahead of the replay it accelerates by the factor replay pays over a
//! whole path. What it can lose to is the *number* of questions: a canonical
//! schedule reached by adjacent transpositions asks a quadratic number of
//! independence questions in the path length, at which point the guard costs
//! more than the replay. That bound is measured rather than assumed
//! (`shift::tests::an_adjacent_transposition_schedule_asks_a_quadratic_number_of_questions`
//! in the crate's integration suite).

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::alphabet::CellAlphabet;
use crate::alphabet::ConvexityDischarge;
use crate::alphabet::PositionOrder;
use crate::boundary::ShiftReplay;
use crate::cell::CellId;
use crate::cell::CellStore;
use crate::overlap::Overlap;
use crate::overlap::OverlapSupport;
use crate::overlap::overlaps_between;
use crate::rewrite::CellApp;
use crate::rewrite::rewrite_at;
use crate::sequent::SequentAlphabet;
use crate::tracelet::replay_from_peak;

/// A **shift-equivalence witness** — evidence that two adjacent applications
/// commute, so their two sequentializations are one composite transformation.
///
/// The witness is a boundary (`peak`, `joins_at`) plus the two applications
/// that span it in either order, plus the warrant its convexity conjunct was
/// discharged under. It is **replayed, not trusted**
/// ([`ShiftEquivalence::replay`], ADR-69): holding one is not the claim that
/// the identification is sound, it is the record of which guard granted it and
/// of a boundary that can be re-executed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShiftEquivalence<A: CellAlphabet = SequentAlphabet>
{
    /// The term both sequentializations start from.
    pub peak: A::Cmd,
    /// The application recorded first.
    pub first: CellApp<A>,
    /// The application recorded second.
    pub second: CellApp<A>,
    /// The composite — the term both sequentializations reach.
    pub joins_at: A::Cmd,
    /// The warrant the convexity conjunct was discharged under, carried rather
    /// than recomputed.
    pub convexity: ConvexityDischarge,
}

impl<A: CellAlphabet> ShiftEquivalence<A>
{
    /// The sequentialization that fires [`ShiftEquivalence::first`] first.
    #[inline]
    #[must_use]
    pub fn first_then_second(&self) -> Vec<CellApp<A>>
    {
        alloc::vec![self.first.clone(), self.second.clone()]
    }

    /// The sequentialization that fires [`ShiftEquivalence::second`] first.
    #[inline]
    #[must_use]
    pub fn second_then_first(&self) -> Vec<CellApp<A>>
    {
        alloc::vec![self.second.clone(), self.first.clone()]
    }

    /// **Replay** the identification: re-execute both sequentializations from
    /// the peak and check they both reach the composite.
    ///
    /// This is the tracelet replay criterion ([`crate::tracelet`]) read on a
    /// shift boundary: the two orders share a peak and a join by construction,
    /// so "both replay" is exactly replay-equality of the two derivations, and
    /// it is what confirms the identification instead of asserting it.
    ///
    /// # Contract
    /// - ensures: positive iff, after skolemizing the peak and the composite to
    ///   fresh constants, both orders fire step by step and land on the
    ///   skolemized composite; a step that no longer fires, a stale cell id, or
    ///   either order landing elsewhere yields a negative.
    /// - provides: the confirmation half of the identification, separate from
    ///   the guard that licensed it.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the decision surface is the conjunction of
    ///   two path replays against one boundary, separated by a derived witness
    ///   that replays and by a witness whose composite has been retargeted to a
    ///   term neither order reaches.
    /// - witness: `shift::tests::the_cong2_composite_replays_under_both_sequentializations`
    /// - witness: `shift::tests::a_retargeted_composite_no_longer_replays`
    #[inline]
    #[must_use]
    pub fn replay(
        &self,
        store: &CellStore<A>,
    ) -> ShiftReplay
    {
        let replayed = replay_from_peak(
            store,
            &self.peak,
            &self.joins_at,
            &self.first_then_second(),
            &self.second_then_first(),
        );
        ShiftReplay::from(bool::from(replayed))
    }
}

/// Why a pair of adjacent applications was **refused** the shift-equivalence
/// witness.
///
/// Refusal is data, never a panic and never a silent identity: a pair the guard
/// does not cover keeps its two sequentializations distinct, and the variant
/// says which conjunct or which instance check declined it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShiftObstruction<A: CellAlphabet = SequentAlphabet>
{
    /// An application names a cell the store does not hold, so the pair cannot
    /// be read at all.
    UnknownCell
    {
        /// The identifier that resolved to nothing.
        cell: CellId,
    },
    /// The positions are comparable: one application addresses a subtree
    /// containing the other's, so the two are nested rather than adjacent and
    /// commuting them is not even a well-posed question.
    ComparablePositions
    {
        /// How the two positions relate.
        order: PositionOrder,
    },
    /// The cell pair has a **genuine overlap** — the enumerator's verdict, and
    /// the reason the two applications may interfere through the cells rather
    /// than through the term.
    GenuineOverlap
    {
        /// The first overlap witnessing the interference, in enumeration order.
        overlap: Box<Overlap<A>>,
    },
    /// The convexity conjunct is not discharged on this store, and the per-pair
    /// re-check is not built — so the shift is refused rather than assumed.
    ConvexityNotDischarged,
    /// A recorded application does not fire at its recorded position in the
    /// sequentialization that reaches it.
    StepDoesNotFire
    {
        /// The step that failed to fire.
        step: Box<CellApp<A>>,
    },
    /// Both orders fire but reach different terms, so there is no composite to
    /// identify them at.
    ///
    /// On a carrier where incomparable positions address disjoint subtrees this
    /// is unreachable — splicing at disjoint positions commutes — and it is
    /// kept as the check that would catch a carrier where they do not, rather
    /// than as an outcome today's alphabets can produce.
    SequentializationsDiffer
    {
        /// The term reached by firing the first application first.
        first_then_second: Box<A::Cmd>,
        /// The term reached by firing the second application first.
        second_then_first: Box<A::Cmd>,
    },
}

/// **Derive the shift-equivalence witness** for a pair of adjacent
/// applications in `peak`, or refuse it.
///
/// The three guard conjuncts are *checked*, never assumed: the positions must
/// be incomparable, the cell pair must have trivial overlap in both orders,
/// and the convexity conjunct must be discharged by the store's own certificate
/// ([`CellAlphabet::convexity_discharge`]) rather than by a sweep this engine
/// does not run. A pair that clears the guard is additionally *exercised*: both
/// orders must fire and reach one term, which becomes the composite.
///
/// # Contract
/// - requires: `first` and `second` are applications intended at positions of
///   the same `peak`; the alphabet answers
///   [`CellAlphabet::convexity_discharge`] honestly for `store`.
/// - ensures: `Ok(witness)` only when all three conjuncts hold and both
///   sequentializations fire from `peak` to one common term; the witness
///   records that term as its composite and carries the discharge it was
///   granted under.
/// - provides: the earned identification of a two-redex body's two readings —
///   the per-pair licence a `cells_equal` normal-form fast path would need,
///   with its evidence attached.
/// - fails: [`ShiftObstruction`] — an unresolvable cell id, comparable
///   positions, a genuine overlap (carrying the overlap), an undischarged
///   convexity conjunct, a step that does not fire (carrying the step), or two
///   orders that reach different terms (carrying both).
/// - panics: none.
/// - intension: the conjuncts are decided in the guard's own order — cell
///   resolution, then positions, then overlap, then convexity, then the two
///   sequentializations — so a pair failing several of them is refused by the
///   earliest, and the returned variant is that observation.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L2 — each guard conjunct owns a decision surface, so each is
///   separated by a pair that fails only it: the cong2 pair clears all three; a
///   nested pair fails positions while its cells also overlap (which pins the
///   declared order); an overlapping pair at incomparable positions fails the
///   overlap conjunct; a store whose alphabet withholds the discharge fails the
///   third; and an unknown id and a non-firing step separate the two instance
///   checks.
/// - witness: `shift::tests::the_cong2_pair_earns_its_shift_equivalence_witness`
/// - witness: `shift::tests::a_nested_pair_is_refused_before_the_overlap_conjunct`
/// - witness: `shift::tests::a_genuinely_overlapping_pair_is_refused_the_witness`
/// - witness: `shift::tests::an_undischarged_convexity_conjunct_refuses_the_pair`
/// - witness: `shift::tests::an_unknown_cell_identifier_is_refused`
/// - witness: `shift::tests::a_step_that_does_not_fire_is_refused`
#[inline]
pub fn derive_shift_equivalence<A>(
    store: &CellStore<A>,
    peak: &A::Cmd,
    first: &CellApp<A>,
    second: &CellApp<A>,
) -> Result<ShiftEquivalence<A>, ShiftObstruction<A>>
where
    A: CellAlphabet,
{
    let convexity = A::convexity_discharge(store);
    check_shift_guard(store, first, second, convexity)?;
    let forward = run_pair(store, peak, first, second)?;
    let backward = run_pair(store, peak, second, first)?;
    if forward != backward {
        return Err(ShiftObstruction::SequentializationsDiffer {
            first_then_second: Box::new(forward),
            second_then_first: Box::new(backward),
        });
    }
    Ok(ShiftEquivalence {
        peak: peak.clone(),
        first: first.clone(),
        second: second.clone(),
        joins_at: forward,
        convexity,
    })
}

/// The three guard conjuncts, with the convexity discharge supplied.
///
/// The overlap support is supplied as well, precomputed by the caller: a
/// consumer asking about many pairs over one store builds it once instead of
/// once per query, which is what [`check_shift_guard`] does for a one-off.
///
/// Split out from [`derive_shift_equivalence`] so the third conjunct's refusal
/// is exercisable on today's alphabets, both of which discharge it: the public
/// entry point reads the discharge from the alphabet and never lets a caller
/// assert one.
///
/// It is crate-visible rather than module-private because the crate has exactly
/// **one** independence relation and it lives here: the tracelet normal form
/// ([`crate::normal_form`]) decides which adjacent transpositions its canonical
/// schedule may perform by asking this same guard, so a second, drifting copy
/// of the conjuncts cannot come into existence. That consumer reads any refusal
/// as dependence, which is the conservative direction — refusing to commute is
/// always sound.
///
/// # Contract
/// - ensures: `Ok(())` exactly when the positions are incomparable, the ordered
///   cell pair has no overlap in either order, and `convexity` is
///   [`ConvexityDischarge::LeftConnectedOverAcyclicTarget`].
/// - fails: the corresponding [`ShiftObstruction`] variant, decided in that
///   order.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the conjunct order is itself a decision surface,
///   separated by a nested pair whose cells also overlap and by a store whose
///   discharge is withheld.
/// - witness: `shift::tests::a_nested_pair_is_refused_before_the_overlap_conjunct`
/// - witness: `shift::tests::an_undischarged_convexity_conjunct_refuses_the_pair`
#[inline]
pub(crate) fn check_shift_guard_with_support<A>(
    store: &CellStore<A>,
    first: &CellApp<A>,
    second: &CellApp<A>,
    convexity: ConvexityDischarge,
    support: &OverlapSupport,
) -> Result<(), ShiftObstruction<A>>
where
    A: CellAlphabet,
{
    let Some(first_cell) = store.get(first.cell)
    else {
        return Err(ShiftObstruction::UnknownCell { cell: first.cell });
    };
    let Some(second_cell) = store.get(second.cell)
    else {
        return Err(ShiftObstruction::UnknownCell { cell: second.cell });
    };
    let order = A::position_order(&first.at, &second.at);
    if order != PositionOrder::Incomparable {
        return Err(ShiftObstruction::ComparablePositions { order });
    }
    if !bool::from(support.independent(first.cell, second.cell)) {
        let mut overlaps = overlaps_between((first.cell, first_cell), (second.cell, second_cell));
        overlaps.extend(overlaps_between(
            (second.cell, second_cell),
            (first.cell, first_cell),
        ));
        if let Some(overlap) = overlaps.into_iter().next() {
            return Err(ShiftObstruction::GenuineOverlap {
                overlap: Box::new(overlap),
            });
        }
    }
    if convexity != ConvexityDischarge::LeftConnectedOverAcyclicTarget {
        return Err(ShiftObstruction::ConvexityNotDischarged);
    }
    Ok(())
}
/// Check the independence guard, building the overlap support for this call.
///
/// # Contract
/// - ensures: exactly [`check_shift_guard_with_support`], with the support
///   relation built from `store` rather than supplied by the caller — the
///   spelling for a one-off query, where a caller asking per pair over one
///   store builds the support once and calls the with-support form.
/// - panics: none.
///
/// # Errors
/// See [`check_shift_guard_with_support`].
#[inline]
pub(crate) fn check_shift_guard<A>(
    store: &CellStore<A>,
    first: &CellApp<A>,
    second: &CellApp<A>,
    convexity: ConvexityDischarge,
) -> Result<(), ShiftObstruction<A>>
where
    A: CellAlphabet,
{
    let support = OverlapSupport::from_store(store);
    check_shift_guard_with_support(store, first, second, convexity, &support)
}

/// Fire `lead` then `trail` from `term`, refusing the step that does not fire.
///
/// # Contract
/// - ensures: `Ok(result)` when both steps' cells fire at their recorded
///   positions in sequence.
/// - fails: [`ShiftObstruction::UnknownCell`] for a stale id and
///   [`ShiftObstruction::StepDoesNotFire`] for a position that carries no redex
///   for its cell.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
#[inline]
fn run_pair<A>(
    store: &CellStore<A>,
    term: &A::Cmd,
    lead: &CellApp<A>,
    trail: &CellApp<A>,
) -> Result<A::Cmd, ShiftObstruction<A>>
where
    A: CellAlphabet,
{
    let after_lead = fire_step(store, term, lead)?;
    fire_step(store, &after_lead, trail)
}

/// Fire one recorded step, refusing rather than returning `None`.
///
/// # Contract
/// - ensures: `Ok(result)` when `step`'s cell fires at its recorded position.
/// - fails: [`ShiftObstruction::UnknownCell`] for a stale id and
///   [`ShiftObstruction::StepDoesNotFire`] when no redex is there.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
#[inline]
fn fire_step<A>(
    store: &CellStore<A>,
    term: &A::Cmd,
    step: &CellApp<A>,
) -> Result<A::Cmd, ShiftObstruction<A>>
where
    A: CellAlphabet,
{
    let Some(cell) = store.get(step.cell)
    else {
        return Err(ShiftObstruction::UnknownCell { cell: step.cell });
    };
    match rewrite_at(cell, term, &step.at) {
        | Some(result) => Ok(result),
        | None => Err(ShiftObstruction::StepDoesNotFire {
            step: Box::new(step.clone()),
        }),
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;

    use super::*;
    use crate::alphabet::path_order;
    use crate::boundary::PositionStep;
    use crate::cell::Cell;
    use crate::pattern::CmdPat;
    use crate::pattern::ConsPat;
    use crate::pattern::Pos;
    use crate::pattern::ProdPat;
    use crate::pattern::Sym;
    use crate::sequent::CellProvenance;
    use crate::sequent::Orientation;
    use crate::sequent::frame_defining_cell;

    /// A position from child indices.
    fn pos<Steps>(steps: Steps) -> Pos
    where
        Steps: IntoIterator<Item = usize>,
    {
        Pos::from_indices(steps.into_iter().collect::<Vec<_>>())
    }

    /// The path order of two child-index paths.
    fn order_of(
        left: &Pos,
        right: &Pos,
    ) -> PositionOrder
    {
        path_order(
            left.as_ref().iter().copied().map(PositionStep::from),
            right.as_ref().iter().copied().map(PositionStep::from),
        )
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

    /// The two-cell store whose pair the enumerator reports overlapping.
    fn overlapping_store() -> (CellStore, CellId, CellId)
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        (store, frame, add)
    }

    /// (add-zero): ⟨Zero | add(Zero; ★)⟩ ~> ⟨Zero | ★⟩.
    ///
    /// A ground rule whose right-hand side offers no seam any left-hand side
    /// unifies with, so the cell overlaps nothing — not even itself. It is the
    /// sequent-alphabet fixture for the conjuncts *after* the overlap one.
    fn add_zero_ground() -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// The single-cell store whose only cell overlaps nothing.
    fn trivial_overlap_store() -> (CellStore, CellId)
    {
        let mut store = CellStore::new();
        let ground = store.insert(add_zero_ground());
        (store, ground)
    }

    #[test]
    fn the_path_order_separates_its_four_outcomes()
    {
        assert_eq!(
            PositionOrder::Same,
            order_of(&pos([0, 1]), &pos([0, 1])),
            "equal paths are the same position"
        );
        assert_eq!(
            PositionOrder::Encloses,
            order_of(&pos([0]), &pos([0, 1])),
            "a proper prefix encloses"
        );
        assert_eq!(
            PositionOrder::EnclosedBy,
            order_of(&pos([0, 1]), &pos([0])),
            "and the relation is symmetric up to swapping the two nesting sides"
        );
        assert_eq!(
            PositionOrder::Incomparable,
            order_of(&pos([0, 1]), &pos([0, 2])),
            "paths diverging at a shared depth address disjoint subtrees"
        );
        assert_eq!(
            PositionOrder::Encloses,
            order_of(&Pos::root(), &pos([1])),
            "the root encloses every other position"
        );
    }

    #[test]
    fn the_sequent_alphabet_offers_only_the_root_command_position()
    {
        // The extension of the shift quotient over this alphabet is EMPTY, and
        // this is why: a command pattern is one cut whose children are a
        // producer and a consumer, neither of which is a command, so no term
        // has two positions to be incomparable at. The guard is a
        // specification the alphabet has not reached, not a patch to a fast
        // path (`circuit-terms-spike-07`).
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::ctor("Zero", [])]),
            ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
        );
        assert_eq!(
            alloc::vec![Pos::root()],
            SequentAlphabet::command_positions(&term),
            "a nested-looking sequent term still has exactly one command position"
        );
    }

    #[test]
    fn two_applications_at_one_position_are_refused()
    {
        let (store, frame, add) = overlapping_store();
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let refusal = derive_shift_equivalence(
            &store,
            &term,
            &CellApp {
                cell: frame,
                at: Pos::root(),
            },
            &CellApp {
                cell: add,
                at: Pos::root(),
            },
        )
        .expect_err("one position is not two");
        assert_eq!(
            ShiftObstruction::ComparablePositions {
                order: PositionOrder::Same
            },
            refusal,
            "the same position is refused by the first conjunct"
        );
    }

    #[test]
    fn a_nested_pair_is_refused_before_the_overlap_conjunct()
    {
        // The cells of this pair DO overlap, so the pair fails two conjuncts;
        // the declared order says the positions decide it.
        let (store, frame, add) = overlapping_store();
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let refusal = derive_shift_equivalence(
            &store,
            &term,
            &CellApp {
                cell: frame,
                at: Pos::root(),
            },
            &CellApp {
                cell: add,
                at: pos([0]),
            },
        )
        .expect_err("a nested pair is refused");
        assert_eq!(
            ShiftObstruction::ComparablePositions {
                order: PositionOrder::Encloses
            },
            refusal,
            "the position conjunct is decided before the overlap conjunct"
        );
    }

    #[test]
    fn a_genuinely_overlapping_pair_is_refused_the_witness()
    {
        // Incomparable positions, so the first conjunct passes; the cell pair
        // composes at a seam, so the second refuses it. (The positions are
        // fabricated: this alphabet has no two command positions to fire at,
        // which is exactly the emptiness the module documents.)
        let (store, frame, add) = overlapping_store();
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let refusal = derive_shift_equivalence(
            &store,
            &term,
            &CellApp {
                cell: frame,
                at: pos([0]),
            },
            &CellApp {
                cell: add,
                at: pos([1]),
            },
        )
        .expect_err("an overlapping cell pair is refused");
        let ShiftObstruction::GenuineOverlap { overlap } = refusal
        else {
            panic!("the overlap conjunct is what refuses this pair: {refusal:?}");
        };
        assert_eq!(
            frame, overlap.left,
            "the refusal carries the overlap that witnesses the interference"
        );
        assert_eq!(add, overlap.right, "in the order the enumerator found it");
    }

    #[test]
    fn an_undischarged_convexity_conjunct_refuses_the_pair()
    {
        // The first two conjuncts are cleared here (a cell that overlaps
        // nothing, at incomparable positions), and the identical pair is still
        // refused when the discharge is withheld — the third conjunct is a
        // check, not a comment.
        let (store, ground) = trivial_overlap_store();
        let step = CellApp {
            cell: ground,
            at: pos([0]),
        };
        let other = CellApp {
            cell: ground,
            at: pos([1]),
        };
        assert_eq!(
            Ok(()),
            check_shift_guard(
                &store,
                &step,
                &other,
                ConvexityDischarge::LeftConnectedOverAcyclicTarget
            ),
            "the guard passes when the store carries the discharge"
        );
        assert_eq!(
            Err(ShiftObstruction::ConvexityNotDischarged),
            check_shift_guard(&store, &step, &other, ConvexityDischarge::ReCheckRequired),
            "and refuses the identical pair when it does not"
        );
    }

    #[test]
    fn a_step_that_does_not_fire_is_refused()
    {
        // The guard passes — the cell overlaps nothing and the positions are
        // incomparable — and the instance check is what declines: neither
        // fabricated position carries a redex, because a sequent term has no
        // command subterm below the root.
        let (store, ground) = trivial_overlap_store();
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
        );
        let step = CellApp {
            cell: ground,
            at: pos([0]),
        };
        let other = CellApp {
            cell: ground,
            at: pos([1]),
        };
        let refusal = derive_shift_equivalence(&store, &term, &step, &other)
            .expect_err("no redex sits at either fabricated position");
        assert_eq!(
            ShiftObstruction::StepDoesNotFire {
                step: Box::new(step)
            },
            refusal,
            "the refusal names the step that did not fire, and the guard is not what declined"
        );
    }

    #[test]
    fn the_sequent_store_carries_the_left_connected_discharge()
    {
        let (store, _frame, _add) = overlapping_store();
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            SequentAlphabet::convexity_discharge(&store),
            "every expressible left-hand side is cut-rooted, so the re-check is constant-true"
        );
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            SequentAlphabet::convexity_discharge(&CellStore::new()),
            "the warrant is the grammar's, so an empty store carries it too"
        );
    }

    #[test]
    fn an_unknown_cell_identifier_is_refused()
    {
        let (store, frame, _add) = overlapping_store();
        let term = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::frame("Succ", ConsPat::Top),
        );
        let missing = CellId(99);
        let refusal = derive_shift_equivalence(
            &store,
            &term,
            &CellApp {
                cell: frame,
                at: pos([0]),
            },
            &CellApp {
                cell: missing,
                at: pos([1]),
            },
        )
        .expect_err("an unresolvable id is refused");
        assert_eq!(
            ShiftObstruction::UnknownCell { cell: missing },
            refusal,
            "the refusal names the identifier that resolved to nothing"
        );
    }
}
