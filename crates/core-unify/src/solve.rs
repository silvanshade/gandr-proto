//! The solver machine: an explicit goal stack, driven one step at a time.
//!
//! # Shape
//!
//! The machine holds a LIFO stack of goals, each a pair of semantic nodes plus
//! the constraint it came from. One step pops one goal and does exactly one of
//! four things: discharges it, replaces it by smaller goals, binds a
//! metavariable, or records a named stop. Depth follows the heap, so a term
//! deep enough to overflow a host stack costs memory instead.
//!
//! # Why the machine defers to conversion wherever it can
//!
//! Every semantic node carries a guard word, and the guard word knows whether
//! anything beneath it is a hole. A goal whose two sides are both hole-free
//! contains no metavariable, so it is a **conversion** question rather than a
//! unification question, and the machine hands it to
//! [`conv::converts_values`] or [`conv::converts_comps`] whole. Unfolding,
//! smart unfolding, and the three-state speculation therefore have exactly one
//! implementation in the crate, and the equational theory of the solver is the
//! equational theory of the checker by construction rather than by agreement.
//!
//! The machine only walks a pair itself when a metavariable is somewhere
//! inside, because that is when a conversion verdict of *distinct* would be
//! premature: a metavariable can block a reduction that binding it would
//! release.
//!
//! # Termination
//!
//! Not by an invariant, by a budget. The driver spends one unit per popped
//! goal, and when the budget runs out every remaining goal is postponed with
//! [`PostponeReason::BudgetExhausted`]. The solver is a service reached from
//! untrusted input, so a bound stated in the type is worth more than a bound
//! argued in a comment; the budget is generous enough that no constraint inside
//! the fragment reaches it.

use alloc::rc::Rc;
use alloc::vec::Vec;

use gandr_core_nbe::Normalizer;
use gandr_core_nbe::conv;
use gandr_core_nbe::eval;
use gandr_core_nbe::eval::ForceMode;
use gandr_core_nbe::intern::canonically_equal_value_types;
use gandr_core_nbe::quote::QuoteMode;
use gandr_core_nbe::quote::level_name;
use gandr_core_nbe::quote::quote_comp;
use gandr_core_nbe::quote::quote_value;
use gandr_core_nbe::sem::Elim;
use gandr_core_nbe::sem::NeutralHead;
use gandr_core_nbe::sem::Rigid;
use gandr_core_nbe::sem::SemArena;
use gandr_core_nbe::sem::SemCompId;
use gandr_core_nbe::sem::SemCompNode;
use gandr_core_nbe::sem::SemError;
use gandr_core_nbe::sem::SemValueId;
use gandr_core_nbe::sem::SemValueNode;
use gandr_core_nbe::sem::ValueUnfold;
use gandr_core_term::boundary::ConstraintIndex;
use gandr_core_term::boundary::HoleId;
use gandr_core_term::boundary::MetaFreedom;
use gandr_core_term::boundary::RigidStatus;
use gandr_core_term::boundary::SolverBudget;
use gandr_core_term::boundary::SolverSteps;
use gandr_core_term::boundary::UnfoldPermission;
use gandr_core_term::boundary::ValueEquality;
use gandr_core_term::boundary::VariableLevel;
use gandr_core_term::grade::Grade;
use gandr_core_term::subst::HoleRepl;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Value;

use crate::Constraint;
use crate::certify::Certificate;
use crate::certify::Postponed;
use crate::certify::Verdict;
use crate::frag::PostponeReason;
use crate::frag::Refusal;
use crate::frag::Refutation;
use crate::frag::SpineShape;
use crate::frag::classify_spine;
use crate::meta::MetaContext;
use crate::meta::MetaSort;
use crate::scan;

/// One pending equation, in the semantic domain.
#[derive(Clone, Copy, Debug)]
enum Pair
{
    /// Two values to unify.
    Value(SemValueId, SemValueId),
    /// Two computations to unify.
    Comp(SemCompId, SemCompId),
}

/// One goal on the machine's stack.
#[derive(Clone, Copy, Debug)]
struct Goal
{
    /// The equation.
    pair: Pair,
    /// The constraint the equation descends from.
    source: ConstraintIndex,
    /// Whether the constraint this goal descends from mentioned no hole at all,
    /// which makes the goal a conversion question outright.
    metafree: MetaFreedom,
    /// Whether this goal may still spend an unfolding.
    unfold: UnfoldPermission,
}

/// What one step decided.
enum Step
{
    /// The goal is satisfied and needs nothing further.
    Discharged,
    /// The goal was replaced by the goals the step pushed.
    Pushed,
    /// The goal is outside the fragment, for this reason and these blockers.
    Postpone(PostponeReason, Vec<HoleId>),
    /// No substitution satisfies the goal.
    Refute(Refutation),
    /// The current conversion relation declined the goal.
    Refuse(Refusal),
}

/// A metavariable at the head of one side, with the spine it wears.
#[derive(Clone, Debug)]
enum FlexHead
{
    /// A value metavariable in value position, which wears no spine because
    /// every eliminator of a positive type is a computation.
    Bare(HoleId),
    /// A computation metavariable under a spine.
    Computation
    {
        /// The metavariable.
        meta: HoleId,
        /// The eliminators it is stuck under, outermost last.
        spine: Vec<Elim>,
    },
    /// A value metavariable reached through a `force`, under a spine, whose
    /// solutions are thunks at the declared grade.
    Thunk
    {
        /// The metavariable.
        meta: HoleId,
        /// The grade its solutions are thunked at.
        grade: Grade,
        /// The eliminators it is stuck under, outermost last.
        spine: Vec<Elim>,
    },
    /// A value metavariable reached through a `force` with no declared grade,
    /// which cannot be solved without inventing one.
    Ungraded(HoleId),
}

impl FlexHead
{
    /// The metavariable at this head.
    fn meta(&self) -> HoleId
    {
        match *self {
            | Self::Bare(meta)
            | Self::Ungraded(meta)
            | Self::Computation { meta, .. }
            | Self::Thunk { meta, .. } => meta,
        }
    }

    /// The spine this head wears.
    fn spine(&self) -> &[Elim]
    {
        match *self {
            | Self::Bare(_) | Self::Ungraded(_) => &[],
            | Self::Computation { ref spine, .. } | Self::Thunk { ref spine, .. } => spine,
        }
    }
}

/// The solver machine.
struct Machine
{
    /// Pending goals, processed last-in-first-out.
    goals: Vec<Goal>,
    /// The postponed constraints, at most one entry per source constraint.
    residual: Vec<Postponed>,
    /// The refutation that stopped the run, when one did.
    refutation: Option<Refutation>,
    /// The refusal that stopped the run, when one did.
    refusal: Option<Refusal>,
    /// Steps spent.
    steps: SolverSteps,
    /// Steps allowed.
    budget: SolverBudget,
}

/// Solves `constraints`, returning the certificate its answer carries.
///
/// # Contract
/// - requires: every metavariable occurring in `constraints` is declared in
///   `metas`, and no hole in `constraints` has an identity at or above the
///   watermark `metas` was built with.
/// - ensures: every binding the certificate carries is a most general solution
///   of the constraints it was derived from, closed, and free of the
///   metavariable it binds; every constraint the certificate does not discharge
///   appears in its residual with a named reason; a refutation is reported only
///   where no substitution can satisfy the constraint, while a refusal names a
///   conversion decision that may change with the environment. The semantic
///   nodes the run built are truncated away before the certificate is returned.
/// - provides: the answer of the solver-machine service, as evidence a caller
///   re-checks rather than a verdict a caller trusts.
/// - fails: [`SemError`] when the arena is exhausted or an id does not resolve;
///   the run is truncated before the error propagates.
/// - panics: none.
/// - intension: the driver spends one step per popped goal and never more than
///   `budget` steps in total. The step count is a declared projection, reported
///   by the certificate.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
///
/// # Adequacy
/// - hypothesis: L1 — the answer is a certificate, and the witnesses validate
///   it by substituting and asking ordinary conversion rather than by comparing
///   against a predicted solution. L3 covers the four verdicts pointwise: a
///   solved constraint, a postponed one, a refuted one, and a refused one.
/// - witness: `unify::tests::a_pattern_solution_replays_through_ordinary_conversion`
/// - witness: `unify::tests::a_prunable_occurrence_postpones_rather_than_refuting`
/// - witness: `unify::tests::a_rigid_clash_is_refuted`
pub(crate) fn solve(
    nbe: &mut Normalizer,
    metas: &mut MetaContext,
    constraints: &[Constraint],
    budget: SolverBudget,
) -> Result<Certificate, SemError>
{
    let mark = nbe.watermark();
    let outcome = drive(nbe, metas, constraints, budget);
    nbe.truncate_to(mark);
    outcome
}

/// The fallible core of [`solve`], run inside the caller's arena watermark.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn drive(
    nbe: &mut Normalizer,
    metas: &mut MetaContext,
    constraints: &[Constraint],
    budget: SolverBudget,
) -> Result<Certificate, SemError>
{
    let mut machine = Machine {
        goals: Vec::with_capacity(constraints.len()),
        residual: Vec::new(),
        refutation: None,
        refusal: None,
        steps: SolverSteps::from(0_u32),
        budget,
    };
    for (index, constraint) in constraints.iter().enumerate() {
        let source = ConstraintIndex::from(index);
        let pair = evaluate(nbe, constraint)?;
        machine.goals.push(Goal {
            pair,
            source,
            metafree: constraint_is_metafree(constraint),
            unfold: UnfoldPermission::from(true),
        });
    }
    // The stack was seeded in constraint order, so popping would take the last
    // one first; reversing keeps the residual in the caller's order.
    machine.goals.reverse();
    while let Some(goal) = machine.goals.pop() {
        if u32::from(machine.steps) >= u32::from(machine.budget) {
            machine.postpone(goal.source, PostponeReason::BudgetExhausted, Vec::new());
            for pending in core::mem::take(&mut machine.goals) {
                machine.postpone(pending.source, PostponeReason::BudgetExhausted, Vec::new());
            }
            break;
        }
        machine.steps = SolverSteps::from(u32::from(machine.steps).saturating_add(1));
        let step = match goal.pair {
            | Pair::Value(lhs, rhs) => step_value(&mut machine, nbe, metas, goal, lhs, rhs)?,
            | Pair::Comp(lhs, rhs) => step_comp(&mut machine, nbe, metas, goal, lhs, rhs)?,
        };
        match step {
            | Step::Discharged | Step::Pushed => {},
            | Step::Postpone(reason, blockers) => machine.postpone(goal.source, reason, blockers),
            | Step::Refute(refutation) => {
                machine.refutation = Some(refutation);
                machine.goals.clear();
                break;
            },
            | Step::Refuse(refusal) => {
                machine.refusal = Some(refusal);
                machine.goals.clear();
                break;
            },
        }
    }
    let verdict = match (
        machine.refutation,
        machine.refusal,
        machine.residual.is_empty(),
    ) {
        | (Some(refutation), ..) => Verdict::Refuted(refutation),
        | (_, Some(refusal), _) => Verdict::Refused(refusal),
        | (None, None, false) => Verdict::Postponed,
        | (None, None, true) => Verdict::Solved,
    };
    Ok(Certificate::new(
        verdict,
        metas.zonked(),
        machine.residual,
        machine.steps,
    ))
}

impl Machine
{
    /// Records a postponement, keeping the first reason recorded per source.
    fn postpone(
        &mut self,
        source: ConstraintIndex,
        reason: PostponeReason,
        blockers: Vec<HoleId>,
    )
    {
        if self
            .residual
            .iter()
            .any(|postponed| postponed.source() == source)
        {
            return;
        }
        self.residual.push(Postponed::new(source, reason, blockers));
    }

    /// Pushes one value goal, inheriting the parent goal's unfolding budget.
    fn push_value(
        &mut self,
        goal: Goal,
        lhs: SemValueId,
        rhs: SemValueId,
    )
    {
        self.goals.push(Goal {
            pair: Pair::Value(lhs, rhs),
            source: goal.source,
            metafree: goal.metafree,
            unfold: UnfoldPermission::from(true),
        });
    }

    /// Pushes one computation goal, inheriting the parent goal's source.
    fn push_comp(
        &mut self,
        goal: Goal,
        lhs: SemCompId,
        rhs: SemCompId,
    )
    {
        self.goals.push(Goal {
            pair: Pair::Comp(lhs, rhs),
            source: goal.source,
            metafree: goal.metafree,
            unfold: UnfoldPermission::from(true),
        });
    }
}

/// Evaluates one source constraint into the semantic pair the machine works on.
///
/// # Errors
///
/// Returns [`SemError`] when lowering or evaluation fails.
fn evaluate(
    nbe: &mut Normalizer,
    constraint: &Constraint,
) -> Result<Pair, SemError>
{
    match *constraint {
        | Constraint::Values(ref lhs, ref rhs) => {
            let lhs = nbe.lower_input(lhs)?;
            let rhs = nbe.lower_input(rhs)?;
            let lhs = eval::eval_value(nbe, SemArena::EMPTY_ENV, lhs)?;
            let rhs = eval::eval_value(nbe, SemArena::EMPTY_ENV, rhs)?;
            Ok(Pair::Value(lhs, rhs))
        },
        | Constraint::Comps(ref lhs, ref rhs) => {
            let lhs = lower_comp(nbe, lhs)?;
            let rhs = lower_comp(nbe, rhs)?;
            Ok(Pair::Comp(lhs, rhs))
        },
    }
}

/// Whether a constraint mentions no hole at all.
///
/// A constraint with no hole has no metavariable either, so the whole equation
/// is a conversion question and the machine hands it to conversion in one call
/// rather than walking it. That is what makes the solver's verdict on a
/// metavariable-free problem **equal** to conversion's verdict rather than
/// merely consistent with it, which is the property the agreement suite pins.
///
/// The scan is syntactic and exact, where the semantic guard word is neither: a
/// guard cannot see a hole beneath a thunk.
fn constraint_is_metafree(constraint: &Constraint) -> MetaFreedom
{
    let holes = match *constraint {
        | Constraint::Values(ref lhs, ref rhs) => {
            let lhs = scan::scan_value(lhs.as_ref(), VariableLevel::from(0), &[]);
            let rhs = scan::scan_value(rhs.as_ref(), VariableLevel::from(0), &[]);
            lhs.holes().len().saturating_add(rhs.holes().len())
        },
        | Constraint::Comps(ref lhs, ref rhs) => {
            let lhs = scan::scan_comp(lhs.as_ref(), VariableLevel::from(0), &[]);
            let rhs = scan::scan_comp(rhs.as_ref(), VariableLevel::from(0), &[]);
            lhs.holes().len().saturating_add(rhs.holes().len())
        },
    };
    MetaFreedom::from(holes == 0)
}

/// Lowers and evaluates one source computation.
///
/// # Errors
///
/// Returns [`SemError`] when lowering or evaluation fails.
pub(crate) fn lower_comp(
    nbe: &mut Normalizer,
    comp: &Comp,
) -> Result<SemCompId, SemError>
{
    let node = nbe
        .syntax_mut()
        .alloc_comp(comp)
        .map_err(|_error| SemError::SyntaxStore)?;
    eval::eval_comp(nbe, SemArena::EMPTY_ENV, node, ForceMode::WeakHead)
}

/// Whether a value pair is settled enough for conversion to answer it outright.
///
/// The guard word must say **rigid and hole-free** on both sides, and rigidity
/// is what makes the hole bit trustworthy. A guard folds the hole bit up from
/// its children, but a thunk's guard is a leaf that marks itself unfoldable
/// rather than descending into the closure it carries, so a hole beneath a
/// thunk is invisible to the hole bit alone. Rigidity is exactly the bit that
/// is false whenever a thunk or an unfolding rule sits anywhere beneath, so
/// requiring both makes the test sound rather than merely cheap.
///
/// # Errors
///
/// Returns [`SemError`] when an id does not resolve.
fn value_pair_is_rigid(
    nbe: &Normalizer,
    lhs: SemValueId,
    rhs: SemValueId,
) -> Result<RigidStatus, SemError>
{
    let lhs = nbe.arena().value(lhs)?.guard();
    let rhs = nbe.arena().value(rhs)?.guard();
    Ok(RigidStatus::from(
        bool::from(settled(lhs)) && bool::from(settled(rhs)),
    ))
}

/// Whether one guard word says its value is rigid and carries no hole.
fn settled(guard: gandr_core_nbe::sem::Guard) -> RigidStatus
{
    RigidStatus::from(bool::from(guard.rigid()) && !bool::from(guard.holes()))
}

/// The computation-sorted twin of [`value_pair_is_rigid`].
///
/// # Errors
///
/// Returns [`SemError`] when an id does not resolve.
fn comp_pair_is_rigid(
    nbe: &Normalizer,
    lhs: SemCompId,
    rhs: SemCompId,
) -> Result<RigidStatus, SemError>
{
    let lhs = nbe.arena().comp(lhs)?.guard();
    let rhs = nbe.arena().comp(rhs)?.guard();
    Ok(RigidStatus::from(
        bool::from(settled(lhs)) && bool::from(settled(rhs)),
    ))
}

/// The hole at the head of a value, when it has one.
///
/// # Errors
///
/// Returns [`SemError`] when the id does not resolve.
fn value_head_hole(
    nbe: &Normalizer,
    id: SemValueId,
) -> Result<Option<HoleId>, SemError>
{
    Ok(match *nbe.arena().value(id)?.node() {
        | SemValueNode::Rigid(Rigid::Hole(hole), _) => Some(HoleId::from(hole)),
        | _ => None,
    })
}

/// The flex head of a value, when its head is a declared metavariable.
///
/// # Errors
///
/// Returns [`SemError`] when the id does not resolve.
fn value_flex(
    nbe: &Normalizer,
    metas: &MetaContext,
    id: SemValueId,
) -> Result<Option<FlexHead>, SemError>
{
    let Some(hole) = value_head_hole(nbe, id)?
    else {
        return Ok(None);
    };
    Ok(bool::from(metas.is_meta(hole)).then_some(FlexHead::Bare(hole)))
}

/// The flex head of a computation, when its head is a declared metavariable.
///
/// Two shapes reach a metavariable at computation sort. A computation hole
/// heads its own neutral directly. A value hole is reached through a `force`,
/// which is how a metavariable standing for a function appears in
/// call-by-push-value.
///
/// # Errors
///
/// Returns [`SemError`] when an id does not resolve.
fn comp_flex(
    nbe: &Normalizer,
    metas: &MetaContext,
    id: SemCompId,
) -> Result<Option<FlexHead>, SemError>
{
    let SemCompNode::Neutral(stuck) = *nbe.arena().comp(id)?.node()
    else {
        return Ok(None);
    };
    let neutral = nbe.arena().neutral(stuck)?;
    let (hole, spine) = match *neutral.head() {
        | NeutralHead::Hole(hole) => (HoleId::from(hole), neutral.spine().to_vec()),
        | NeutralHead::Force(carried) => {
            let Some(hole) = value_head_hole(nbe, carried)?
            else {
                return Ok(None);
            };
            (hole, neutral.spine().to_vec())
        },
        | _ => return Ok(None),
    };
    Ok(match metas.sort(hole) {
        | None => None,
        | Some(MetaSort::Comp) => Some(FlexHead::Computation { meta: hole, spine }),
        | Some(MetaSort::Thunk(grade)) => Some(FlexHead::Thunk {
            meta: hole,
            grade,
            spine,
        }),
        | Some(MetaSort::Value) => Some(FlexHead::Ungraded(hole)),
    })
}

/// Replaces a solved metavariable at a value head by its solution.
///
/// # Errors
///
/// Returns [`SemError`] when lowering or evaluation fails.
fn resolve_value(
    nbe: &mut Normalizer,
    metas: &MetaContext,
    id: SemValueId,
) -> Result<Option<SemValueId>, SemError>
{
    let Some(hole) = value_head_hole(nbe, id)?
    else {
        return Ok(None);
    };
    let Some(solution) = metas.value_solution(hole).map(Rc::clone)
    else {
        return Ok(None);
    };
    let node = nbe.lower_input(solution.as_ref())?;
    let evaluated = eval::eval_value(nbe, SemArena::EMPTY_ENV, node)?;
    Ok(Some(evaluated))
}

/// Replaces a solved metavariable at a computation head by its solution,
/// re-running the spine it wore.
///
/// # Errors
///
/// Returns [`SemError`] when lowering or evaluation fails.
fn resolve_comp(
    nbe: &mut Normalizer,
    metas: &MetaContext,
    id: SemCompId,
) -> Result<Option<SemCompId>, SemError>
{
    let SemCompNode::Neutral(stuck) = *nbe.arena().comp(id)?.node()
    else {
        return Ok(None);
    };
    let neutral = nbe.arena().neutral(stuck)?;
    let spine = neutral.spine().to_vec();
    let base = match *neutral.head() {
        | NeutralHead::Hole(hole) => {
            let Some(solution) = metas.comp_solution(HoleId::from(hole)).map(Rc::clone)
            else {
                return Ok(None);
            };
            lower_comp(nbe, solution.as_ref())?
        },
        | NeutralHead::Force(carried) => {
            let Some(hole) = value_head_hole(nbe, carried)?
            else {
                return Ok(None);
            };
            let Some(solution) = metas.value_solution(hole).map(Rc::clone)
            else {
                return Ok(None);
            };
            let node = nbe.lower_input(solution.as_ref())?;
            let evaluated = eval::eval_value(nbe, SemArena::EMPTY_ENV, node)?;
            eval::force_head(nbe, evaluated, ForceMode::WeakHead)?
        },
        | _ => return Ok(None),
    };
    let rebuilt = eval::rerun_spine(nbe, base, &spine, ForceMode::WeakHead)?;
    Ok(Some(rebuilt))
}

/// Decides one value goal.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
#[expect(
    clippy::too_many_lines,
    reason = "the arm-per-constructor congruence is one decision surface; splitting \
              it would separate the clash verdict from the constructors it reads"
)]
fn step_value(
    machine: &mut Machine,
    nbe: &mut Normalizer,
    metas: &mut MetaContext,
    goal: Goal,
    lhs: SemValueId,
    rhs: SemValueId,
) -> Result<Step, SemError>
{
    if lhs == rhs {
        return Ok(Step::Discharged);
    }
    if bool::from(goal.metafree) {
        // A metavariable-free constraint is a conversion question outright, in
        // both directions, which is what makes the solver's verdict on one
        // equal to conversion's rather than merely consistent with it.
        return Ok(if bool::from(conv::converts_values(nbe, lhs, rhs)?) {
            Step::Discharged
        }
        else {
            Step::Refuse(Refusal::Conversion)
        });
    }
    // Inside a constraint that does carry a metavariable, a settled subterm
    // pair is delegated for its **positive** answer only. A conversion verdict
    // of distinct falls through to the walk instead of refuting, because that
    // is where a canonical form facing a neutral is named as the eta or
    // singleton residue it is rather than reported as a clash the theory does
    // not actually license.
    if bool::from(value_pair_is_rigid(nbe, lhs, rhs)?)
        && bool::from(conv::converts_values(nbe, lhs, rhs)?)
    {
        return Ok(Step::Discharged);
    }
    if let Some(resolved) = resolve_value(nbe, metas, lhs)? {
        machine.push_value(goal, resolved, rhs);
        return Ok(Step::Pushed);
    }
    if let Some(resolved) = resolve_value(nbe, metas, rhs)? {
        machine.push_value(goal, lhs, resolved);
        return Ok(Step::Pushed);
    }
    let lhs_flex = value_flex(nbe, metas, lhs)?;
    let rhs_flex = value_flex(nbe, metas, rhs)?;
    if lhs_flex.is_some() || rhs_flex.is_some() {
        return solve_flex_pair(
            machine,
            nbe,
            metas,
            goal,
            &Sided {
                flex: lhs_flex,
                operand: Operand::Value(lhs),
            },
            &Sided {
                flex: rhs_flex,
                operand: Operand::Value(rhs),
            },
        );
    }
    let lhs_hole = value_head_hole(nbe, lhs)?;
    let rhs_hole = value_head_hole(nbe, rhs)?;
    if lhs_hole.is_some() || rhs_hole.is_some() {
        return Ok(Step::Postpone(PostponeReason::HoleConsistency, Vec::new()));
    }
    let lhs_node = nbe.arena().value(lhs)?.node().clone();
    let rhs_node = nbe.arena().value(rhs)?.node().clone();
    match (&lhs_node, &rhs_node) {
        | (&SemValueNode::Unit, &SemValueNode::Unit) => return Ok(Step::Discharged),
        | (&SemValueNode::Int(left), &SemValueNode::Int(right)) => {
            return Ok(clash_unless(ValueEquality::from(left == right)));
        },
        | (&SemValueNode::Str(ref left), &SemValueNode::Str(ref right)) => {
            return Ok(clash_unless(ValueEquality::from(left == right)));
        },
        | (&SemValueNode::Num(left), &SemValueNode::Num(right)) => {
            return Ok(clash_unless(ValueEquality::from(left == right)));
        },
        | (&SemValueNode::Reified(left), &SemValueNode::Reified(right)) => {
            // A reified stack is opaque to the solver: it carries source syntax
            // verbatim, so nothing inside it decomposes here. Identical ids are
            // one stack; anything else waits for conversion to see it hole-free.
            return Ok(if left == right {
                Step::Discharged
            }
            else {
                Step::Postpone(PostponeReason::OpaqueReifiedStack, Vec::new())
            });
        },
        | (&SemValueNode::Pair(left_fst, left_snd), &SemValueNode::Pair(right_fst, right_snd)) => {
            machine.push_value(goal, left_snd, right_snd);
            machine.push_value(goal, left_fst, right_fst);
            return Ok(Step::Pushed);
        },
        | (
            &SemValueNode::Inj(left_side, left_payload),
            &SemValueNode::Inj(right_side, right_payload),
        ) => {
            if left_side != right_side {
                return Ok(Step::Refute(Refutation::Clash));
            }
            machine.push_value(goal, left_payload, right_payload);
            return Ok(Step::Pushed);
        },
        | (&SemValueNode::Here(left), &SemValueNode::Here(right)) => {
            machine.push_value(goal, left, right);
            return Ok(Step::Pushed);
        },
        | (
            &SemValueNode::Pack {
                witnesses: ref left_witnesses,
                payload: left_payload,
            },
            &SemValueNode::Pack {
                witnesses: ref right_witnesses,
                payload: right_payload,
            },
        ) => {
            // The witnesses are compared, not erased — through the one shared
            // embedded-identity comparison conversion itself uses, so the
            // solver and conversion stay the same relation on the one input
            // neither evaluates. They are types, so no solution can rewrite
            // them: a mismatch is a clash outright, and a match leaves the
            // payloads as the one residual equation. Without this arm a pack
            // pair falls through to the residual case and reads as a clash
            // between *different* constructors, which two packs are not — the
            // one refutation the evidence here does not license.
            if left_witnesses.len() != right_witnesses.len() {
                return Ok(Step::Refute(Refutation::Clash));
            }
            let store = nbe.syntax();
            let aligned = left_witnesses
                .iter()
                .zip(right_witnesses.iter())
                .all(|(left, right)| {
                    bool::from(canonically_equal_value_types(store, *left, *right))
                });
            if !aligned {
                return Ok(Step::Refute(Refutation::Clash));
            }
            machine.push_value(goal, left_payload, right_payload);
            return Ok(Step::Pushed);
        },
        | (
            &SemValueNode::Ctor {
                id: ref left_id,
                tag: left_tag,
                payload: left_payload,
            },
            &SemValueNode::Ctor {
                id: ref right_id,
                tag: right_tag,
                payload: right_payload,
            },
        ) => {
            if left_id != right_id || left_tag != right_tag {
                return Ok(Step::Refute(Refutation::Clash));
            }
            machine.push_value(goal, left_payload, right_payload);
            return Ok(Step::Pushed);
        },
        | (&SemValueNode::List(ref left), &SemValueNode::List(ref right)) => {
            if left.len() != right.len() {
                return Ok(Step::Refute(Refutation::Clash));
            }
            for (left, right) in left.iter().zip(right.iter()).rev() {
                machine.push_value(goal, *left, *right);
            }
            return Ok(Step::Pushed);
        },
        | (&SemValueNode::Record(ref left), &SemValueNode::Record(ref right)) => {
            // Canonical field order, field by field. No width rule and no
            // permutation rule: the map is keyed by label, so a differing label
            // set is a differing record and nothing relates the two.
            if left.len() != right.len() {
                return Ok(Step::Refute(Refutation::Clash));
            }
            for ((left_label, left_field), (right_label, right_field)) in
                left.iter().zip(right.iter()).rev()
            {
                if left_label != right_label {
                    return Ok(Step::Refute(Refutation::Clash));
                }
                machine.push_value(goal, *left_field, *right_field);
            }
            return Ok(Step::Pushed);
        },
        | (
            &SemValueNode::Thunk(left_grade, left_cell),
            &SemValueNode::Thunk(right_grade, right_cell),
        ) => {
            // Grades are compared exactly, as conversion compares them.
            if left_grade != right_grade {
                return Ok(Step::Refute(Refutation::Clash));
            }
            let left = eval::enter_nullary(nbe, left_cell, ForceMode::WeakHead)?;
            let right = eval::enter_nullary(nbe, right_cell, ForceMode::WeakHead)?;
            machine.push_comp(goal, left, right);
            return Ok(Step::Pushed);
        },
        // **Eta for the thunk**, the same clause conversion carries, because
        // the two must decide one relation. A clause added to one consumer and
        // not the other splits definitional equality across them, and a solver
        // that refuted what the checker accepts would be reporting a fact about
        // which component asked rather than about the terms.
        //
        // This is the residue the note above already names: a canonical form
        // facing a neutral is the **eta residue**, left to the walk rather than
        // refuted because a clash is not what the theory licenses there. `U`
        // has one destructor, so forcing both sides is what resolves it.
        //
        // The grade condition is conversion's: `force` requires `1 ≤ r`, so at
        // grade `0` this arm does not exist and the comparison stays
        // structural.
        //
        // A metavariable cannot reach here — every flex path returns above, so
        // this arm can never intercept a solution.
        | (&SemValueNode::Thunk(grade, _), _) | (_, &SemValueNode::Thunk(grade, _))
            if bool::from(Grade::ONE.leq(grade)) =>
        {
            let left = eval::force_head(nbe, lhs, ForceMode::WeakHead)?;
            let right = eval::force_head(nbe, rhs, ForceMode::WeakHead)?;
            machine.push_comp(goal, left, right);
            return Ok(Step::Pushed);
        },
        | (
            &SemValueNode::Rigid(Rigid::Level(left), _),
            &SemValueNode::Rigid(Rigid::Level(right), _),
        ) => {
            return Ok(clash_unless(ValueEquality::from(left == right)));
        },
        | (
            &SemValueNode::Rigid(Rigid::Free(ref left), _),
            &SemValueNode::Rigid(Rigid::Free(ref right), _),
        ) if left == right => {
            return Ok(Step::Discharged);
        },
        | _ => {},
    }
    if bool::from(goal.unfold) {
        let unfolded_lhs = eval::force_value(nbe, lhs, ForceMode::Unfold)?;
        let unfolded_rhs = eval::force_value(nbe, rhs, ForceMode::Unfold)?;
        if unfolded_lhs != lhs || unfolded_rhs != rhs {
            machine.goals.push(Goal {
                pair: Pair::Value(unfolded_lhs, unfolded_rhs),
                source: goal.source,
                metafree: goal.metafree,
                unfold: UnfoldPermission::from(false),
            });
            return Ok(Step::Pushed);
        }
    }
    Ok(residual_value_reason(&lhs_node, &rhs_node))
}

/// Names what a value pair the congruence rules could not decide is waiting on.
///
/// Two canonical forms with different constructors are a clash and nothing can
/// reconcile them. A canonical form facing a neutral is the **residue**: it
/// would be decided by an eta law for a positive former, or by a definitional
/// singleton where the canonical side is the sole inhabitant of its type, and
/// ordinary conversion has neither. Naming those two separately is what keeps
/// them visible as theory boundaries rather than as unexplained gaps.
fn residual_value_reason(
    lhs: &SemValueNode,
    rhs: &SemValueNode,
) -> Step
{
    let canonical = |node: &SemValueNode| !matches!(*node, SemValueNode::Rigid(..));
    match (canonical(lhs), canonical(rhs)) {
        | (true, true) => Step::Refute(Refutation::Clash),
        | (false, false) => Step::Postpone(PostponeReason::HeadMismatch, Vec::new()),
        | (true, false) => Step::Postpone(residual_eta_reason(lhs), Vec::new()),
        | (false, true) => Step::Postpone(residual_eta_reason(rhs), Vec::new()),
    }
}

/// Whether a canonical value facing a neutral is a singleton question or a
/// positive-eta question.
///
/// The sole inhabitant of the unit type, and of the record type with no fields,
/// would be related to any neutral of its type by a definitional singleton
/// rule. Every other canonical former would need eta for that former.
fn residual_eta_reason(canonical: &SemValueNode) -> PostponeReason
{
    match *canonical {
        | SemValueNode::Unit => PostponeReason::DefinitionalSingleton,
        | SemValueNode::Record(ref fields) if fields.is_empty() => {
            PostponeReason::DefinitionalSingleton
        },
        | _ => PostponeReason::PositiveEta,
    }
}

/// Discharges on agreement and refutes a clash otherwise.
fn clash_unless(agrees: ValueEquality) -> Step
{
    if bool::from(agrees) {
        Step::Discharged
    }
    else {
        Step::Refute(Refutation::Clash)
    }
}

/// Decides one computation goal.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn step_comp(
    machine: &mut Machine,
    nbe: &mut Normalizer,
    metas: &mut MetaContext,
    goal: Goal,
    lhs: SemCompId,
    rhs: SemCompId,
) -> Result<Step, SemError>
{
    if lhs == rhs {
        return Ok(Step::Discharged);
    }
    if bool::from(goal.metafree) {
        return Ok(if bool::from(conv::converts_comps(nbe, lhs, rhs)?) {
            Step::Discharged
        }
        else {
            Step::Refuse(Refusal::Conversion)
        });
    }
    if bool::from(comp_pair_is_rigid(nbe, lhs, rhs)?)
        && bool::from(conv::converts_comps(nbe, lhs, rhs)?)
    {
        return Ok(Step::Discharged);
    }
    if let Some(resolved) = resolve_comp(nbe, metas, lhs)? {
        machine.push_comp(goal, resolved, rhs);
        return Ok(Step::Pushed);
    }
    if let Some(resolved) = resolve_comp(nbe, metas, rhs)? {
        machine.push_comp(goal, lhs, resolved);
        return Ok(Step::Pushed);
    }
    // The flex check comes before the eta arms, because a metavariable facing a
    // canonical computation is solved by quoting that computation, and an eta
    // step would only grow its spine on the way to the same answer.
    let lhs_flex = comp_flex(nbe, metas, lhs)?;
    let rhs_flex = comp_flex(nbe, metas, rhs)?;
    if lhs_flex.is_some() || rhs_flex.is_some() {
        return solve_flex_pair(
            machine,
            nbe,
            metas,
            goal,
            &Sided {
                flex: lhs_flex,
                operand: Operand::Comp(lhs),
            },
            &Sided {
                flex: rhs_flex,
                operand: Operand::Comp(rhs),
            },
        );
    }
    let lhs_node = nbe.arena().comp(lhs)?.node().clone();
    let rhs_node = nbe.arena().comp(rhs)?.node().clone();
    match (&lhs_node, &rhs_node) {
        | (&SemCompNode::Return(left), &SemCompNode::Return(right)) => {
            machine.push_value(goal, left, right);
            Ok(Step::Pushed)
        },
        // Function eta, as ordinary conversion decides it: a lambda against
        // anything compares by applying both to one fresh variable.
        | (&SemCompNode::Lambda(_), _) | (_, &SemCompNode::Lambda(_)) => {
            let fresh = fresh_variable(nbe)?;
            let left = eval::apply(nbe, lhs, fresh, ForceMode::WeakHead)?;
            let right = eval::apply(nbe, rhs, fresh, ForceMode::WeakHead)?;
            machine.push_comp(goal, left, right);
            Ok(Step::Pushed)
        },
        // Lazy-pair eta, as ordinary conversion decides it: a lazy pair against
        // anything compares by projecting both.
        | (&SemCompNode::LazyPair(..), _) | (_, &SemCompNode::LazyPair(..)) => {
            let left_fst = eval::project(
                nbe,
                lhs,
                gandr_core_term::syntax::Side::Fst,
                ForceMode::WeakHead,
            )?;
            let right_fst = eval::project(
                nbe,
                rhs,
                gandr_core_term::syntax::Side::Fst,
                ForceMode::WeakHead,
            )?;
            let left_snd = eval::project(
                nbe,
                lhs,
                gandr_core_term::syntax::Side::Snd,
                ForceMode::WeakHead,
            )?;
            let right_snd = eval::project(
                nbe,
                rhs,
                gandr_core_term::syntax::Side::Snd,
                ForceMode::WeakHead,
            )?;
            machine.push_comp(goal, left_snd, right_snd);
            machine.push_comp(goal, left_fst, right_fst);
            Ok(Step::Pushed)
        },
        | (&SemCompNode::Neutral(_), &SemCompNode::Neutral(_)) => {
            step_neutral(machine, nbe, metas, goal, lhs, rhs)
        },
        | _ => Ok(Step::Postpone(PostponeReason::HeadMismatch, Vec::new())),
    }
}

/// Mints one fresh rigid variable at a fresh de Bruijn level.
///
/// # Errors
///
/// Returns [`SemError`] when the arena is exhausted.
fn fresh_variable(nbe: &mut Normalizer) -> Result<SemValueId, SemError>
{
    let level = nbe.fresh_level();
    let node = SemValueNode::Rigid(Rigid::Level(level), ValueUnfold::Rigid);
    eval::value(nbe, node)
}

/// Decides one pair of neutral computations.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn step_neutral(
    machine: &mut Machine,
    nbe: &mut Normalizer,
    metas: &MetaContext,
    goal: Goal,
    lhs: SemCompId,
    rhs: SemCompId,
) -> Result<Step, SemError>
{
    let (lhs_head, lhs_spine) = neutral_parts(nbe, lhs)?;
    let (rhs_head, rhs_spine) = neutral_parts(nbe, rhs)?;
    if matches!(lhs_head, NeutralHead::Hole(_)) || matches!(rhs_head, NeutralHead::Hole(_)) {
        return Ok(Step::Postpone(PostponeReason::HoleConsistency, Vec::new()));
    }
    if lhs_spine.len() != rhs_spine.len() {
        return Ok(Step::Refute(Refutation::Clash));
    }
    match (&lhs_head, &rhs_head) {
        | (&NeutralHead::Force(left), &NeutralHead::Force(right)) => {
            machine.push_value(goal, left, right);
        },
        | (
            &NeutralHead::Project {
                record: left,
                label: ref left_label,
            },
            &NeutralHead::Project {
                record: right,
                label: ref right_label,
            },
        ) => {
            if left_label != right_label {
                return Ok(Step::Refute(Refutation::Clash));
            }
            machine.push_value(goal, left, right);
        },
        | (
            &NeutralHead::Native {
                prim: left_prim,
                args: ref left_args,
            },
            &NeutralHead::Native {
                prim: right_prim,
                args: ref right_args,
            },
        ) => {
            if left_prim != right_prim || left_args.len() != right_args.len() {
                return Ok(Step::Refute(Refutation::Clash));
            }
            for (left, right) in left_args.iter().zip(right_args.iter()).rev() {
                machine.push_value(goal, *left, *right);
            }
        },
        // Every other head is stuck on a scrutinee or quarantined by policy,
        // and deciding it congruently would mean entering its closures. A
        // metavariable is somewhere in the pair or the rigid path above would
        // have taken it, so waiting is the honest answer.
        | _ => {
            let blockers = elimination_blockers(nbe, metas, &lhs_head, &rhs_head)?;
            return Ok(Step::Postpone(PostponeReason::BlockedElimination, blockers));
        },
    }
    for (left, right) in lhs_spine.iter().zip(rhs_spine.iter()).rev() {
        match (*left, *right) {
            | (Elim::Apply(left), Elim::Apply(right)) => machine.push_value(goal, left, right),
            | (Elim::Project(left), Elim::Project(right)) => {
                if left != right {
                    return Ok(Step::Refute(Refutation::Clash));
                }
            },
            | (Elim::Sequence(left), Elim::Sequence(right)) => {
                let fresh = fresh_variable(nbe)?;
                let left = eval::enter_with(nbe, left, &[fresh], ForceMode::WeakHead)?;
                let right = eval::enter_with(nbe, right, &[fresh], ForceMode::WeakHead)?;
                machine.push_comp(goal, left, right);
            },
            | _ => return Ok(Step::Refute(Refutation::Clash)),
        }
    }
    Ok(Step::Pushed)
}

/// The head and spine of a neutral computation.
///
/// # Errors
///
/// Returns [`SemError`] when an id does not resolve.
fn neutral_parts(
    nbe: &Normalizer,
    id: SemCompId,
) -> Result<(NeutralHead, Vec<Elim>), SemError>
{
    let SemCompNode::Neutral(stuck) = *nbe.arena().comp(id)?.node()
    else {
        return Err(SemError::MissingComp(id));
    };
    let neutral = nbe.arena().neutral(stuck)?;
    Ok((neutral.head().clone(), neutral.spine().to_vec()))
}

/// The metavariables a blocked elimination is stuck on.
///
/// # Errors
///
/// Returns [`SemError`] when an id does not resolve.
fn elimination_blockers(
    nbe: &Normalizer,
    metas: &MetaContext,
    lhs: &NeutralHead,
    rhs: &NeutralHead,
) -> Result<Vec<HoleId>, SemError>
{
    let mut blockers = Vec::new();
    for head in [lhs, rhs] {
        let (NeutralHead::Case { scrutinee, .. }
        | NeutralHead::DataCase { scrutinee, .. }
        | NeutralHead::ListCase { scrutinee, .. }
        | NeutralHead::Split { scrutinee, .. }
        | NeutralHead::Walk { scrutinee, .. }
        | NeutralHead::Dup(scrutinee)
        | NeutralHead::Drop(scrutinee)
        | NeutralHead::Perform {
            payload: scrutinee, ..
        }) = *head
        else {
            continue;
        };
        if let Some(hole) = value_head_hole(nbe, scrutinee)?
            && bool::from(metas.is_meta(hole))
            && !blockers.contains(&hole)
        {
            blockers.push(hole);
        }
    }
    Ok(blockers)
}

/// One side of a flex goal: the node, and the metavariable heading it if any.
#[derive(Clone, Debug)]
struct Sided
{
    /// The metavariable at this side's head, when it has one.
    flex: Option<FlexHead>,
    /// The node itself.
    operand: Operand,
}

/// One side of a goal, at the sort it occupies.
#[derive(Clone, Copy, Debug)]
enum Operand
{
    /// A value side.
    Value(SemValueId),
    /// A computation side.
    Comp(SemCompId),
}

/// Solves a goal at least one of whose sides is headed by a metavariable.
///
/// The order is deliberate. Two occurrences of one metavariable go to the
/// intersection rule, because the ordinary path would read the second
/// occurrence as an occurs failure. Otherwise the left side is tried and the
/// right side is the fallback, so a constraint solvable from either direction
/// is solved rather than postponed.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn solve_flex_pair(
    machine: &mut Machine,
    nbe: &mut Normalizer,
    metas: &mut MetaContext,
    goal: Goal,
    lhs: &Sided,
    rhs: &Sided,
) -> Result<Step, SemError>
{
    if let (Some(left), Some(right)) = (lhs.flex.as_ref(), rhs.flex.as_ref())
        && left.meta() == right.meta()
    {
        return solve_same_meta(machine, nbe, metas, goal, left, right);
    }
    let mut blockers = Vec::new();
    for flex in [lhs.flex.as_ref(), rhs.flex.as_ref()].into_iter().flatten() {
        blockers.push(flex.meta());
    }
    let mut first = None;
    for (flex, other) in [
        (lhs.flex.as_ref(), rhs.operand),
        (rhs.flex.as_ref(), lhs.operand),
    ] {
        let Some(flex) = flex
        else {
            continue;
        };
        match solve_flex(machine, nbe, metas, goal, flex, other, &blockers)? {
            | Step::Postpone(reason, _) => {
                if first.is_none() {
                    first = Some(reason);
                }
            },
            | decided => return Ok(decided),
        }
    }
    let reason = match (first, lhs.flex.is_some() && rhs.flex.is_some()) {
        | (Some(PostponeReason::FlexEscape), true) => PostponeReason::FlexFlexIntersection,
        | (Some(reason), _) => reason,
        | (None, _) => PostponeReason::NonPatternSpine,
    };
    Ok(Step::Postpone(reason, blockers))
}

/// Solves one flex side against the other side of its goal.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn solve_flex(
    machine: &mut Machine,
    nbe: &mut Normalizer,
    metas: &mut MetaContext,
    goal: Goal,
    flex: &FlexHead,
    other: Operand,
    blockers: &[HoleId],
) -> Result<Step, SemError>
{
    let meta = flex.meta();
    if let FlexHead::Ungraded(_) = *flex {
        return Ok(Step::Postpone(
            PostponeReason::UndeclaredThunkGrade,
            blockers.to_vec(),
        ));
    }
    let levels = match classify_spine(nbe, flex.spine())? {
        | SpineShape::Pattern(levels) => levels,
        | SpineShape::Projected => return Ok(split_meta(machine, metas, goal, flex)),
        | SpineShape::Blocked(reason) => return Ok(Step::Postpone(reason, blockers.to_vec())),
    };
    // Read the level counter before quoting: every binder readback introduces
    // sits at or above it, and every variable this run opened sits below it.
    let ceiling = nbe.next_level();
    let (occurrences, solution) = match other {
        | Operand::Value(id) => {
            let node = quote_value(nbe, id, QuoteMode::Canonical)?;
            let term = nbe.reify(node)?;
            let occurrences = scan::scan_value(term.as_ref(), ceiling, &levels);
            (occurrences, Solution::Value(term))
        },
        | Operand::Comp(id) => {
            let node = quote_comp(nbe, id, QuoteMode::Canonical)?;
            let term = nbe
                .syntax()
                .comp(node)
                .map_err(|_error| SemError::MissingSyntaxComp(node))?;
            let occurrences = scan::scan_comp(&term, ceiling, &levels);
            (occurrences, Solution::Comp(term))
        },
    };
    // A candidate mentioning another metavariable might still be prunable, so
    // only a candidate with nothing left to prune carries a refutation.
    let prunable = occurrences
        .holes()
        .iter()
        .any(|&hole| hole != meta && bool::from(metas.is_meta(hole)));
    if bool::from(occurrences.mentions(meta)) {
        return Ok(if prunable {
            Step::Postpone(PostponeReason::FlexOccurs, blockers.to_vec())
        }
        else {
            Step::Refute(Refutation::Occurs)
        });
    }
    if bool::from(occurrences.escapes()) {
        return Ok(if prunable || bool::from(occurrences.opaque()) {
            Step::Postpone(PostponeReason::FlexEscape, blockers.to_vec())
        }
        else {
            Step::Refute(Refutation::Escape)
        });
    }
    bind(metas, flex, &levels, solution);
    Ok(Step::Discharged)
}

/// A candidate solution, at the sort its metavariable occupies.
enum Solution
{
    /// A value-sorted candidate.
    Value(Rc<Value>),
    /// A computation-sorted candidate.
    Comp(Comp),
}

/// Binds a metavariable to a candidate, abstracting over its spine.
///
/// The abstraction binders are named by the levels the spine applies, which is
/// exactly how canonical readback already named their occurrences in the
/// candidate. Nothing is renamed, and the result is closed.
fn bind(
    metas: &mut MetaContext,
    flex: &FlexHead,
    levels: &[VariableLevel],
    solution: Solution,
)
{
    let repl = match (flex, solution) {
        // A value-sorted candidate answers a bare value metavariable, which
        // wears no spine, so there is nothing to abstract over.
        | (_, Solution::Value(term)) => HoleRepl::Value(term),
        | (&FlexHead::Thunk { grade, .. }, Solution::Comp(term)) => {
            let body = abstract_over(term, levels);
            HoleRepl::Value(Rc::new(Value::Thunk(grade, Rc::new(body))))
        },
        | (_, Solution::Comp(term)) => HoleRepl::Comp(Rc::new(abstract_over(term, levels))),
    };
    metas.solve(flex.meta(), repl);
}

/// Wraps `body` in one abstraction per spine level, innermost last.
fn abstract_over(
    body: Comp,
    levels: &[VariableLevel],
) -> Comp
{
    let mut built = body;
    for level in levels.iter().rev() {
        built = Comp::Abs(level_name(*level), None, Rc::new(built));
    }
    built
}

/// Inverts a leading projection by splitting the metavariable in two.
///
/// This is the nested half of the fragment, and it is choice-free precisely
/// because ordinary conversion decides lazy-pair eta: every computation of a
/// lazy-pair shape already equals the pair of its own projections, so replacing
/// one metavariable by a pair of fresh ones loses no solutions.
fn split_meta(
    machine: &mut Machine,
    metas: &mut MetaContext,
    goal: Goal,
    flex: &FlexHead,
) -> Step
{
    let fst = metas.fresh(MetaSort::Comp);
    let snd = metas.fresh(MetaSort::Comp);
    let paired = Comp::With(
        Rc::new(Comp::Hole(u32::from(fst))),
        Rc::new(Comp::Hole(u32::from(snd))),
    );
    let repl = match *flex {
        | FlexHead::Thunk { grade, .. } => {
            HoleRepl::Value(Rc::new(Value::Thunk(grade, Rc::new(paired))))
        },
        | _ => HoleRepl::Comp(Rc::new(paired)),
    };
    metas.solve(flex.meta(), repl);
    // The goal itself is re-posed: the next visit resolves the metavariable to
    // the pair just bound, the projection fires, and what remains is the half
    // the projection selected.
    machine.goals.push(goal);
    Step::Pushed
}

/// Solves an equation between two occurrences of one metavariable.
///
/// Miller's intersection rule: the solution may depend only on the positions
/// where the two spines agree, so the metavariable is bound to a fresh one
/// applied to exactly those, and the goal is re-posed against the binding.
///
/// # Errors
///
/// Returns [`SemError`] on arena exhaustion or an unresolvable id.
fn solve_same_meta(
    machine: &mut Machine,
    nbe: &Normalizer,
    metas: &mut MetaContext,
    goal: Goal,
    lhs: &FlexHead,
    rhs: &FlexHead,
) -> Result<Step, SemError>
{
    let left = classify_spine(nbe, lhs.spine())?;
    let right = classify_spine(nbe, rhs.spine())?;
    let (SpineShape::Pattern(left), SpineShape::Pattern(right)) = (left, right)
    else {
        return Ok(Step::Postpone(
            PostponeReason::FlexFlexMismatchedSpines,
            alloc::vec![lhs.meta()],
        ));
    };
    if left.len() != right.len() {
        return Ok(Step::Postpone(
            PostponeReason::FlexFlexMismatchedSpines,
            alloc::vec![lhs.meta()],
        ));
    }
    if left == right {
        return Ok(Step::Discharged);
    }
    let kept = left
        .iter()
        .zip(right.iter())
        .filter_map(|(&left, &right)| (left == right).then_some(left))
        .collect::<Vec<_>>();
    let fresh = metas.fresh(MetaSort::Comp);
    let mut body = Comp::Hole(u32::from(fresh));
    for level in &kept {
        body = Comp::App(Rc::new(body), Rc::new(Value::Var(level_name(*level))));
    }
    let body = abstract_over(body, &left);
    let repl = match *lhs {
        | FlexHead::Thunk { grade, .. } => {
            HoleRepl::Value(Rc::new(Value::Thunk(grade, Rc::new(body))))
        },
        | _ => HoleRepl::Comp(Rc::new(body)),
    };
    metas.solve(lhs.meta(), repl);
    machine.goals.push(goal);
    Ok(Step::Pushed)
}
