//! The certificate, and the replay that validates it.
//!
//! # A solution is evidence, not a verdict
//!
//! The solver answers with a substitution and a claim, and the claim is
//! checkable by machinery that knows nothing about unification: substitute the
//! bindings into the original constraints and ask the ordinary conversion
//! engine whether the two sides are now equal. Nothing opaque enters the
//! trusted base, because the trusted base re-derives the answer.
//!
//! [`Certificate::validate`] performs exactly that replay, and it calls
//! [`conv::converts_values`] and [`conv::converts_comps`] directly. It never
//! compares readback normal forms. The distinction is the whole point: readback
//! agreement is a different relation from conversion, decided by different
//! code, and evidence that the solver agrees with readback is not evidence that
//! the checker will accept the solution.
//!
//! # Why a replay can be inconclusive
//!
//! Conversion relates a hole to every value in both directions. So a replay
//! whose substituted terms still carry a hole may be passing on that wildcard
//! rather than on the solution, and reporting it as a validation would be a
//! false positive. [`Replay::Vacuous`] is that case, named rather than hidden,
//! and it is distinct from [`Replay::Validated`], which the validator reports
//! only when every replayed term is hole-free.

use alloc::rc::Rc;
use alloc::vec::Vec;

use gandr_core_nbe::Normalizer;
use gandr_core_nbe::conv;
use gandr_core_nbe::sem::SemError;
use gandr_core_term::boundary::ConstraintIndex;
use gandr_core_term::boundary::HoleId;
use gandr_core_term::boundary::HoleOccurrence;
use gandr_core_term::boundary::SolverSteps;
use gandr_core_term::boundary::VariableLevel;
use gandr_core_term::subst::HoleRepl;
use gandr_core_term::subst::HoleSubstitution;
use gandr_core_term::subst::subst_holes_comp;
use gandr_core_term::subst::subst_holes_value;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Value;

use crate::Constraint;
use crate::frag::PostponeReason;
use crate::frag::Refutation;
use crate::scan;
use crate::solve::lower_comp;

/// What the solver concluded about a problem as a whole.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Verdict
{
    /// Every constraint was discharged, and the bindings say how.
    Solved,
    /// At least one constraint is outside the fragment as posed and waits in
    /// the residual. The bindings are still most general as far as they go.
    Postponed,
    /// No substitution satisfies the constraints.
    Refuted(Refutation),
}

/// One constraint the solver did not decide, with the reason and the
/// metavariables whose binding would let it be retried.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Postponed
{
    /// Which constraint, by its position in the problem.
    source: ConstraintIndex,
    /// Why the solver stopped.
    reason: PostponeReason,
    /// The metavariables at the two heads, when the stop has any.
    blockers: Vec<HoleId>,
}

impl Postponed
{
    /// Builds one postponement record.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        source: ConstraintIndex,
        reason: PostponeReason,
        blockers: Vec<HoleId>,
    ) -> Self
    {
        Self {
            source,
            reason,
            blockers,
        }
    }

    /// Which constraint was postponed.
    #[inline]
    #[must_use]
    pub fn source(&self) -> ConstraintIndex
    {
        self.source
    }

    /// Why the solver stopped on it.
    #[inline]
    #[must_use]
    pub fn reason(&self) -> PostponeReason
    {
        self.reason
    }

    /// The metavariables whose binding would let the constraint be retried.
    ///
    /// These are the heads the constraint is stuck on, which is where a
    /// blocked equation resumes. A constraint may also be waiting on a
    /// metavariable further inside; the heads are what the solver reads without
    /// a traversal, and they are what a worklist wakes on.
    #[inline]
    #[must_use]
    pub fn blockers(&self) -> &[HoleId]
    {
        &self.blockers
    }
}

/// What a replay found.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Replay
{
    /// Every discharged constraint converts after substitution, and every
    /// replayed term is hole-free, so the conversion verdicts are conclusive.
    Validated,
    /// Every discharged constraint converts after substitution, but a replayed
    /// term still carries a hole, so hole consistency may be carrying a
    /// verdict rather than the solution.
    Vacuous,
    /// A discharged constraint does not convert after substitution. For a
    /// [`Verdict::Solved`] certificate this is a defect in the solver, and it
    /// is exactly what the replay exists to catch.
    Refuted,
    /// The certificate claims no solution, so there is nothing to replay.
    Unproven,
}

/// The solver's answer: a substitution, a residual, and a claim about both.
#[derive(Clone, Debug)]
pub struct Certificate
{
    /// What the solver concluded.
    verdict: Verdict,
    /// The bindings, with every solution-inside-a-solution already resolved.
    bindings: HoleSubstitution,
    /// The constraints left undecided.
    residual: Vec<Postponed>,
    /// Steps spent, the machine's declared intensional projection.
    steps: SolverSteps,
    /// Whether any binding still carries a hole, which a replay cannot see by
    /// substituting, because substitution does not descend into what it
    /// inserts.
    bindings_carry_holes: HoleOccurrence,
}

impl Certificate
{
    /// Builds a certificate from a finished run.
    #[must_use]
    pub(crate) fn new(
        verdict: Verdict,
        bindings: HoleSubstitution,
        residual: Vec<Postponed>,
        steps: SolverSteps,
    ) -> Self
    {
        let bindings_carry_holes = HoleOccurrence::from(bindings.entries().any(|(_meta, repl)| {
            let holes = match *repl {
                | HoleRepl::Value(ref term) => {
                    scan::scan_value(term.as_ref(), VariableLevel::from(0), &[])
                        .holes()
                        .len()
                },
                | HoleRepl::Comp(ref term) => {
                    scan::scan_comp(term.as_ref(), VariableLevel::from(0), &[])
                        .holes()
                        .len()
                },
            };
            holes > 0
        }));
        Self {
            verdict,
            bindings,
            residual,
            steps,
            bindings_carry_holes,
        }
    }

    /// What the solver concluded.
    #[inline]
    #[must_use]
    pub fn verdict(&self) -> Verdict
    {
        self.verdict
    }

    /// The constraints the solver did not decide.
    #[inline]
    #[must_use]
    pub fn residual(&self) -> &[Postponed]
    {
        &self.residual
    }

    /// The steps the machine spent, its one declared intensional projection.
    #[inline]
    #[must_use]
    pub fn steps(&self) -> SolverSteps
    {
        self.steps
    }

    /// The metavariables this certificate binds, in canonical order.
    #[inline]
    pub fn solved(&self) -> impl Iterator<Item = HoleId>
    {
        self.bindings.entries().map(|(meta, _repl)| meta)
    }

    /// The value-sorted solution of `meta`, when this certificate binds one.
    #[inline]
    #[must_use]
    pub fn value_solution(
        &self,
        meta: HoleId,
    ) -> Option<&Rc<Value>>
    {
        self.bindings.value(meta)
    }

    /// The computation-sorted solution of `meta`, when this certificate binds
    /// one.
    #[inline]
    #[must_use]
    pub fn comp_solution(
        &self,
        meta: HoleId,
    ) -> Option<&Rc<Comp>>
    {
        self.bindings.comp(meta)
    }

    /// Applies the certificate's bindings to a value.
    ///
    /// # Contract
    /// - ensures: returns `term` with every metavariable this certificate binds
    ///   replaced by its solution, and whether the result still carries a hole.
    /// - provides: the substitution half of the evidence, for a caller that
    ///   wants the substituted term rather than the replay verdict.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn substitute_value(
        &self,
        term: &Value,
    ) -> (Value, HoleOccurrence)
    {
        subst_holes_value(term, &self.bindings)
    }

    /// Applies the certificate's bindings to a computation.
    ///
    /// # Contract
    /// - ensures: as [`Self::substitute_value`], at computation sort.
    /// - provides: the substituted computation.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn substitute_comp(
        &self,
        term: &Comp,
    ) -> (Comp, HoleOccurrence)
    {
        subst_holes_comp(term, &self.bindings)
    }

    /// Replays the certificate against the ordinary conversion engine.
    ///
    /// Every constraint the certificate discharged is substituted and handed to
    /// [`conv::converts_values`] or [`conv::converts_comps`]. Postponed
    /// constraints are skipped, because the certificate makes no claim about
    /// them.
    ///
    /// # Contract
    /// - requires: `constraints` is the problem this certificate was produced
    ///   from, in the same order, so a residual index names the same constraint
    ///   it named at solve time.
    /// - ensures: [`Replay::Validated`] exactly when every discharged
    ///   constraint converts after substitution and no replayed term or binding
    ///   carries a hole; [`Replay::Vacuous`] when they all convert but a hole
    ///   survives somewhere, so hole consistency may be carrying a verdict;
    ///   [`Replay::Refuted`] as soon as one discharged constraint does not
    ///   convert; [`Replay::Unproven`] for a certificate claiming no solution.
    ///   An arena error is absorbed into [`Replay::Refuted`], which is the
    ///   fail-closed direction and matches how conversion absorbs one.
    /// - ensures: `nbe`'s semantic arena is left at the population it was
    ///   handed, whichever verdict is returned — each constraint replays inside
    ///   a watermark [`Self::replay_one`] truncates back to before it returns.
    ///   The syntax store and its interner grow, identically on every verdict,
    ///   because they are content-keyed caches a definitional environment names
    ///   by handle; nothing truncation drops can be named through them.
    /// - provides: the self-certifying half of the service — a validator small
    ///   enough to read, resting entirely on machinery the solver does not own.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — the validator is the evidence checker, so it takes
    ///   the L3 rigor: the four verdicts are separated pointwise by a solved
    ///   certificate, a certificate whose substituted term keeps a hole, a
    ///   hand-built certificate whose claimed solution ordinary conversion
    ///   refutes, and a refuting certificate.
    /// - witness: `unify::tests::a_pattern_solution_replays_through_ordinary_conversion`
    /// - witness: `unify::tests::meta_splitting_leaves_the_unconstrained_half_open_and_replays_vacuously`
    /// - witness: `unify::tests::a_hand_built_singleton_eta_certificate_is_refuted_by_replay`
    /// - witness: `unify::tests::a_refuting_certificate_has_nothing_to_replay`
    /// - hypothesis: L0 for the arena-restoration clause — the entry watermark
    ///   has exactly one consumer, so removing the rollback or retargeting it
    ///   at a later watermark leaves the mark unused and fails to compile —
    ///   plus L3 on the reachable leg, the arena population observed either
    ///   side of a validating replay. The refuting leg carries no separate
    ///   witness: no input reaches an error inside the replay, whose only
    ///   failures are arena exhaustion and an unresolvable id, so the rollback
    ///   being unconditional in source is what carries it.
    /// - witness: `unify::tests::replaying_a_certificate_restores_the_callers_semantic_arena`
    #[cfg_attr(
        dylint_lib = "non_local_effect_before_unhandled_error",
        expect(
            non_local_effect_before_unhandled_error,
            reason = "the flagged effect is replay_one's own rollback: it truncates the caller's \
                      semantic arena back to the watermark it took at entry, unconditionally and \
                      before returning, and it hands out no arena id, so the arena a caller can \
                      still name is exactly the one it passed in; witnessed by \
                      tests::replaying_a_certificate_restores_the_callers_semantic_arena"
        )
    )]
    #[inline]
    #[must_use]
    pub fn validate(
        &self,
        nbe: &mut Normalizer,
        constraints: &[Constraint],
    ) -> Replay
    {
        if matches!(self.verdict, Verdict::Refuted(_)) {
            return Replay::Unproven;
        }
        let mut vacuous = bool::from(self.bindings_carry_holes);
        for (index, constraint) in constraints.iter().enumerate() {
            let index = ConstraintIndex::from(index);
            if self
                .residual
                .iter()
                .any(|postponed| postponed.source == index)
            {
                continue;
            }
            let (converts, holes) = match self.replay_one(nbe, constraint) {
                | Ok(outcome) => outcome,
                | Err(_error) => return Replay::Refuted,
            };
            if !bool::from(converts) {
                return Replay::Refuted;
            }
            vacuous = vacuous || bool::from(holes);
        }
        if vacuous {
            Replay::Vacuous
        }
        else {
            Replay::Validated
        }
    }

    /// Substitutes one constraint and asks ordinary conversion about it.
    ///
    /// # Errors
    ///
    /// Returns [`SemError`] when lowering or evaluation fails.
    fn replay_one(
        &self,
        nbe: &mut Normalizer,
        constraint: &Constraint,
    ) -> Result<(gandr_core_term::boundary::ValueEquality, HoleOccurrence), SemError>
    {
        let mark = nbe.watermark();
        let outcome = self.replay_checked(nbe, constraint);
        nbe.truncate_to(mark);
        outcome
    }

    /// The fallible core of [`Self::replay_one`], inside the arena watermark.
    ///
    /// # Errors
    ///
    /// Returns [`SemError`] when lowering or evaluation fails.
    fn replay_checked(
        &self,
        nbe: &mut Normalizer,
        constraint: &Constraint,
    ) -> Result<(gandr_core_term::boundary::ValueEquality, HoleOccurrence), SemError>
    {
        match *constraint {
            | Constraint::Values(ref lhs, ref rhs) => {
                let (lhs, lhs_holes) = self.substitute_value(lhs.as_ref());
                let (rhs, rhs_holes) = self.substitute_value(rhs.as_ref());
                let lhs = nbe.lower_input(&lhs)?;
                let rhs = nbe.lower_input(&rhs)?;
                let lhs = gandr_core_nbe::eval::eval_value(
                    nbe,
                    gandr_core_nbe::sem::SemArena::EMPTY_ENV,
                    lhs,
                )?;
                let rhs = gandr_core_nbe::eval::eval_value(
                    nbe,
                    gandr_core_nbe::sem::SemArena::EMPTY_ENV,
                    rhs,
                )?;
                let converts = conv::converts_values(nbe, lhs, rhs)?;
                Ok((
                    converts,
                    HoleOccurrence::from(bool::from(lhs_holes) || bool::from(rhs_holes)),
                ))
            },
            | Constraint::Comps(ref lhs, ref rhs) => {
                let (lhs, lhs_holes) = self.substitute_comp(lhs.as_ref());
                let (rhs, rhs_holes) = self.substitute_comp(rhs.as_ref());
                let lhs = lower_comp(nbe, &lhs)?;
                let rhs = lower_comp(nbe, &rhs)?;
                let converts = conv::converts_comps(nbe, lhs, rhs)?;
                Ok((
                    converts,
                    HoleOccurrence::from(bool::from(lhs_holes) || bool::from(rhs_holes)),
                ))
            },
        }
    }
}
