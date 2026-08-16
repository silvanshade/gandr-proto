//! **Predictable-fragment unification**: the checker's solver-machine service.
//!
//! A unification problem is a list of equations between core terms, some of
//! whose holes the caller has nominated as metavariables. The service answers
//! with a [`Certificate`]: a most general substitution, a residual of equations
//! it declined to decide, and a claim the caller can re-check by substituting
//! and asking the ordinary conversion engine.
//!
//! # The reliability contract
//!
//! **Most general or nothing.** A solver that sometimes returns a solution less
//! general than the problem admits is user-visible unreliability even when
//! every answer it gives is correct, because the difference shows up later and
//! somewhere else. So every binding here is forced by the equation it came
//! from, and where the most general answer would need a choice, the equation is
//! postponed with a name instead.
//!
//! **Postpone, never guess.** [`PostponeReason`] has one variant per place the
//! solver stops. A caller reading one knows what would have to change; a test
//! reading one pins the boundary rather than the absence of an answer.
//!
//! **The theory is the checker's theory.** Every rule the solver applies is a
//! rule [`crate::nbe::conv`] already decides, which is what makes the
//! substitute-and-re-check evidence meaningful. Where a rule is not in
//! conversion, it is not in the fragment either, whatever the type would
//! license.
//!
//! # The fragment, precisely
//!
//! Inside:
//!
//! - **Miller patterns.** A metavariable applied to a spine of distinct
//!   variables the solver opened. The solution abstracts over exactly those.
//! - **Function eta and lazy-pair eta**, both decided by ordinary conversion. A
//!   lambda against anything compares by applying both to one fresh variable; a
//!   lazy pair against anything compares by projecting both.
//! - **Meta splitting**, the nested half: a metavariable whose spine leads with
//!   a projection is replaced by a pair of fresh ones. Lazy-pair eta is what
//!   makes that most general rather than a choice.
//! - **`Return` congruence** and **same-constructor congruence** over the
//!   positive structure: pairs, injections, lists, records, data constructors,
//!   and thunks. Record fields are compared in canonical label order, with no
//!   width rule and no permutation rule, and thunk grades are compared exactly.
//! - **Rigid-rigid decomposition** of neutral computations sharing a head, over
//!   the head and the spine.
//! - **Miller's intersection rule** for two occurrences of one metavariable,
//!   and ordinary flex-flex where one spine covers the other.
//!
//! Outside, each with a named reason:
//!
//! - **Definitional singletons** and **eta for the positive formers**,
//!   including surjective pairing. Ordinary conversion decides neither: a
//!   neutral at unit type does not convert with `unit`, and a positive pair
//!   rebuilt by `split` does not convert with its own neutral. Solving them
//!   would produce certificates the ordinary checker refutes. They become
//!   available when conversion becomes type-directed, and not before.
//! - **Anything discharged only by hole consistency.** Conversion relates an
//!   undeclared hole to every value in both directions, and that relation is
//!   not transitive, so a discharge resting on it composes with nothing.
//! - The pattern boundaries: a non-variable or repeated spine argument, a
//!   positive constructor in a spine, a sequencing continuation in a spine, a
//!   projection behind an application, a flex occurrence or flex escape that
//!   pruning might still remove, a flex-flex pair needing a spine intersection,
//!   and a blocked elimination.
//!
//! Refuted, where the evidence is complete: a clash between two canonical forms
//! with different constructors, an occurs with nothing left to prune, and an
//! escape with nothing left to prune.
//!
//! # Metavariables are nominated holes, and they are closed
//!
//! The solver adds no syntactic former. A metavariable is an existing
//! [`Value::Hole`] or [`Comp::Hole`] that a [`MetaContext`] declares, so the
//! checker, the typing machine, the marking pass, and the conformance
//! generators see nothing new and their step-for-step agreement is inherited
//! rather than re-established.
//!
//! Every metavariable stands for a **closed** term, with context dependence
//! travelling through its spine. That is what lets a solution be substituted
//! into a caller's term without capture and mean the same thing wherever it
//! lands.
//!
//! # Shape of the service
//!
//! [`Solver`] holds the metacontext and the pending constraints;
//! [`Solver::run`] drives the machine and returns the certificate. Running
//! again after pushing more constraints resumes from the bindings already made,
//! which is how a residual is retried once its blockers are bound.
//!
//! [`Value::Hole`]: crate::syntax::Value::Hole
//! [`Comp::Hole`]: crate::syntax::Comp::Hole

pub mod certify;
pub mod frag;
pub mod meta;
mod scan;
pub mod solve;

use alloc::rc::Rc;
use alloc::vec::Vec;

use crate::boundary::ConstraintCount;
use crate::boundary::SolverBudget;
use crate::nbe::Normalizer;
use crate::nbe::sem::SemError;
use crate::syntax::Comp;
use crate::syntax::Value;
pub use crate::unify::certify::Certificate;
pub use crate::unify::certify::Postponed;
pub use crate::unify::certify::Replay;
pub use crate::unify::certify::Verdict;
pub use crate::unify::frag::PostponeReason;
pub use crate::unify::frag::Refutation;
pub use crate::unify::meta::MetaContext;
pub use crate::unify::meta::MetaSort;

/// The default number of steps one run may spend.
///
/// The bound exists so the machine is total on untrusted input rather than
/// total by an argument about its goal stack. No constraint inside the fragment
/// approaches it: a run spends one step per node the two sides share plus one
/// per binding, so the budget is exhausted only by terms far larger than an
/// elaborator produces, or by a pathological definitional environment the
/// normalizer's own fuel already guards against.
const DEFAULT_BUDGET: u32 = 0x0001_0000;

/// One equation to solve, at the sort its two sides occupy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Constraint
{
    /// Two values that must become definitionally equal.
    Values(Rc<Value>, Rc<Value>),
    /// Two computations that must become definitionally equal.
    Comps(Rc<Comp>, Rc<Comp>),
}

/// The solver-machine service: a metacontext, the equations posed against it,
/// and the run that answers them.
///
/// # Posing a constraint in a context
///
/// A constraint is posed **closed over its context**: the locals it mentions
/// are bound by abstractions wrapping both sides, and a metavariable that may
/// depend on one of them is applied to it. The solver opens those abstractions
/// itself, and a variable it opened is what a pattern spine is made of.
///
/// A local left as a free name instead is indistinguishable from a global
/// definition, which a closed metavariable's solution is free to mention. So a
/// spine argument that is a free name is not a pattern argument, and the
/// equation postpones as [`PostponeReason::NonPatternSpine`] rather than
/// solving. That is a fact about what the solver can tell apart, and the
/// closure discipline is what tells it.
///
/// # Contract
/// - requires: every metavariable occurring in a pushed constraint is declared
///   in the metacontext, and every local variable a metavariable may depend on
///   reaches it through that metavariable's spine, under an abstraction the
///   constraint itself carries, rather than as a free name.
/// - ensures: [`Self::run`] leaves the normalizer's semantic arena exactly as
///   it found it, and leaves the metacontext holding every binding the run
///   made, so a second run resumes rather than restarts.
/// - provides: predictable-fragment unification as checkable evidence.
/// - panics: none.
#[derive(Clone, Debug)]
pub struct Solver
{
    /// The metavariables and their bindings.
    metas: MetaContext,
    /// The equations posed, in the order they were pushed.
    constraints: Vec<Constraint>,
    /// The step budget one run may spend.
    budget: SolverBudget,
}

impl Solver
{
    /// A solver over `metas`, with the default step budget.
    #[inline]
    #[must_use]
    pub fn new(metas: MetaContext) -> Self
    {
        Self {
            metas,
            constraints: Vec::new(),
            budget: SolverBudget::from(DEFAULT_BUDGET),
        }
    }

    /// Sets the step budget one run may spend.
    #[inline]
    #[must_use]
    pub fn with_budget(
        mut self,
        budget: SolverBudget,
    ) -> Self
    {
        self.budget = budget;
        self
    }

    /// Poses one more equation.
    #[inline]
    pub fn push(
        &mut self,
        constraint: Constraint,
    )
    {
        self.constraints.push(constraint);
    }

    /// The equations posed so far, which is what a certificate replays against.
    #[inline]
    #[must_use]
    pub fn constraints(&self) -> &[Constraint]
    {
        &self.constraints
    }

    /// How many equations have been posed.
    #[inline]
    #[must_use]
    pub fn constraint_count(&self) -> ConstraintCount
    {
        ConstraintCount::from(self.constraints.len())
    }

    /// The metacontext, with whatever the last run bound.
    #[inline]
    #[must_use]
    pub fn metas(&self) -> &MetaContext
    {
        &self.metas
    }

    /// Runs the machine over every posed equation.
    ///
    /// # Contract
    /// - requires: `nbe` carries the definitional environment the constraints
    ///   are stated against.
    /// - ensures: the certificate binds only metavariables declared in the
    ///   metacontext, each to a closed most general solution; the arena is
    ///   truncated back to where the run found it.
    /// - provides: the answer, as evidence.
    /// - fails: [`SemError`] when the semantic arena is exhausted or an id does
    ///   not resolve.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns [`SemError`] on arena exhaustion or an unresolvable id.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — the run returns a certificate and the witnesses
    ///   validate it by replay rather than against a predicted solution, so a
    ///   mutant that forges a binding cannot forge the replay.
    /// - witness: `unify::tests::a_pattern_solution_replays_through_ordinary_conversion`
    /// - witness: `unify::tests::running_twice_resumes_from_the_bindings_already_made`
    #[inline]
    pub fn run(
        &mut self,
        nbe: &mut Normalizer,
    ) -> Result<Certificate, SemError>
    {
        solve::solve(nbe, &mut self.metas, &self.constraints, self.budget)
    }
}

#[cfg(test)]
mod tests
{
    use alloc::borrow::ToOwned as _;
    use alloc::collections::BTreeMap;
    use alloc::vec;

    use proptest::prelude::*;

    use super::*;
    use crate::boundary::FieldName;
    use crate::boundary::HoleId;
    use crate::boundary::IntegerLiteral;
    use crate::boundary::NameRef;
    use crate::boundary::SolverSteps;
    use crate::grade::Grade;
    use crate::nbe::quote::level_name;
    use crate::subst::HoleRepl;
    use crate::subst::HoleSubstitution;
    use crate::syntax::Side;
    use crate::types::ValueType;
    use crate::unify::scan;

    // ── fixtures ────────────────────────────────────────────────────────────

    /// A hole in value position.
    fn value_hole<H>(id: H) -> Rc<Value>
    where
        H: Into<HoleId>,
    {
        Rc::new(Value::Hole(u32::from(id.into())))
    }

    /// A source variable.
    fn var<'source, N>(name: N) -> Rc<Value>
    where
        N: Into<NameRef<'source>>,
    {
        Rc::new(Value::var(name.into()))
    }

    /// An integer literal.
    fn int<L>(literal: L) -> Rc<Value>
    where
        L: Into<IntegerLiteral>,
    {
        Rc::new(Value::Int(i64::from(literal.into())))
    }

    /// A thunk at the unit grade.
    fn thunk(body: Comp) -> Rc<Value>
    {
        Rc::new(Value::Thunk(Grade::ONE, Rc::new(body)))
    }

    /// A record literal.
    fn record(fields: &[(FieldName<'_>, Rc<Value>)]) -> Rc<Value>
    {
        let fields = fields
            .iter()
            .map(|&(label, ref field)| (label.as_ref().to_owned(), Rc::clone(field)))
            .collect::<BTreeMap<_, _>>();
        Rc::new(Value::Record(fields))
    }

    /// A value-sorted equation.
    fn values(
        lhs: Rc<Value>,
        rhs: Rc<Value>,
    ) -> Constraint
    {
        Constraint::Values(lhs, rhs)
    }

    /// An atom as one package elimination minted it.
    fn seal(serial: crate::boundary::TypeSerial) -> crate::types::SealId
    {
        crate::types::SealId::new(
            serial,
            crate::boundary::SealDeclarationName::from("module"),
            crate::boundary::SealComponentName::from("component"),
        )
    }

    /// A one-component packed module `pack ⟨Integer⟩ payload`.
    fn packed(payload: Value) -> Rc<Value>
    {
        Rc::new(Value::pack([ValueType::integer()], payload))
    }

    /// A computation-sorted equation.
    fn comps(
        lhs: Comp,
        rhs: Comp,
    ) -> Constraint
    {
        Constraint::Comps(Rc::new(lhs), Rc::new(rhs))
    }

    /// A metacontext declaring each identity at the sort beside it.
    fn context(declared: &[(HoleId, MetaSort)]) -> MetaContext
    {
        let mut metas = MetaContext::new(HoleId::from(0));
        for &(id, sort) in declared {
            metas.declare(id, sort);
        }
        metas
    }

    /// Runs one problem to a certificate.
    fn run(
        metas: MetaContext,
        constraints: Vec<Constraint>,
    ) -> (Normalizer, Solver, Certificate)
    {
        let mut nbe = Normalizer::new();
        let mut solver = Solver::new(metas);
        for constraint in constraints {
            solver.push(constraint);
        }
        let certificate = solver.run(&mut nbe).expect("the solver run must not fail");
        (nbe, solver, certificate)
    }

    /// Runs one problem and replays its certificate through ordinary
    /// conversion, which is the only evidence a solved verdict rests on.
    fn run_and_replay(
        metas: MetaContext,
        constraints: Vec<Constraint>,
    ) -> (Certificate, Replay)
    {
        let (mut nbe, solver, certificate) = run(metas, constraints);
        let replay = certificate.validate(&mut nbe, solver.constraints());
        (certificate, replay)
    }

    /// The single postponement a problem produced.
    fn sole_residual(certificate: &Certificate) -> &Postponed
    {
        assert_eq!(certificate.residual().len(), 1, "expected one postponement");
        certificate
            .residual()
            .first()
            .expect("the length was just asserted")
    }

    // ── what ordinary conversion decides, which is where the fragment ends ──

    #[test]
    fn conversion_decides_function_eta()
    {
        let mut nbe = Normalizer::new();
        let expanded = thunk(Comp::lam(
            "x",
            Comp::app(
                Comp::force(Value::var(NameRef::from("f"))),
                Value::var(NameRef::from("x")),
            ),
        ));
        let bare = thunk(Comp::force(Value::var(NameRef::from("f"))));
        assert!(bool::from(nbe.converts(&expanded, &bare)));
    }

    #[test]
    fn conversion_decides_lazy_pair_eta()
    {
        let mut nbe = Normalizer::new();
        let force_f = || Comp::force(Value::var(NameRef::from("f")));
        let paired = thunk(Comp::With(
            Rc::new(Comp::Prj(Side::Fst, Rc::new(force_f()))),
            Rc::new(Comp::Prj(Side::Snd, Rc::new(force_f()))),
        ));
        let bare = thunk(force_f());
        assert!(bool::from(nbe.converts(&paired, &bare)));
    }

    #[test]
    fn conversion_refutes_the_unit_singleton()
    {
        let mut nbe = Normalizer::new();
        assert!(!bool::from(nbe.converts(&var("x"), &Rc::new(Value::Unit))));
    }

    #[test]
    fn conversion_refutes_surjective_pairing()
    {
        let mut nbe = Normalizer::new();
        let rebuilt = thunk(Comp::Split {
            scrut: Rc::new(Value::var(NameRef::from("p"))),
            fst_name: "a".to_owned(),
            snd_name: "b".to_owned(),
            motive: None,
            body: Rc::new(Comp::ret(Value::Pair(
                Rc::new(Value::var(NameRef::from("a"))),
                Rc::new(Value::var(NameRef::from("b"))),
            ))),
        });
        let bare = thunk(Comp::ret(Value::var(NameRef::from("p"))));
        assert!(!bool::from(nbe.converts(&rebuilt, &bare)));
    }

    // ── solved rule classes, each replayed through ordinary conversion ──────

    #[test]
    fn a_bare_value_metavariable_solution_replays()
    {
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(value_hole(0), int(3)),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert_eq!(
            certificate
                .value_solution(HoleId::from(0))
                .map(AsRef::as_ref),
            Some(&Value::Int(3))
        );
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn a_pattern_solution_replays_through_ordinary_conversion()
    {
        // Under one binder the solver opens a level, so `?a x` is a Miller
        // pattern and the identity is the forced solution.
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam(
                        "x",
                        Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                    ),
                    Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        let solution = certificate
            .comp_solution(HoleId::from(0))
            .expect("the pattern must be solved");
        let binder = level_name(crate::boundary::VariableLevel::from(0));
        assert_eq!(
            solution.as_ref(),
            &Comp::Abs(
                binder.clone(),
                None,
                Rc::new(Comp::ret(Value::var(NameRef::from(binder.as_str()))))
            )
        );
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn a_pattern_solution_is_the_most_general_one()
    {
        // The equation does not use the spine variable, so the most general
        // solution ignores it. A solution that mentioned the variable anyway
        // would still replay green, which is exactly why generality needs its
        // own witness rather than resting on the replay.
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam(
                        "x",
                        Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                    ),
                    Comp::lam("x", Comp::ret(Value::Int(1))),
                ),
            ]);
        let binder = level_name(crate::boundary::VariableLevel::from(0));
        assert_eq!(
            certificate
                .comp_solution(HoleId::from(0_u32))
                .map(AsRef::as_ref),
            Some(&Comp::Abs(binder, None, Rc::new(Comp::ret(Value::Int(1)))))
        );
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn an_intersection_solution_drops_every_position_the_spines_disagree_on()
    {
        // `?a x y` against `?a y x` agrees nowhere, so the most general
        // solution takes both arguments and uses neither: the body is a bare
        // fresh metavariable under two abstractions, with no application left.
        let (certificate, _replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam(
                        "x",
                        Comp::lam(
                            "y",
                            Comp::app(
                                Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                                Value::var(NameRef::from("y")),
                            ),
                        ),
                    ),
                    Comp::lam(
                        "x",
                        Comp::lam(
                            "y",
                            Comp::app(
                                Comp::app(Comp::Hole(0), Value::var(NameRef::from("y"))),
                                Value::var(NameRef::from("x")),
                            ),
                        ),
                    ),
                ),
            ]);
        let solution = certificate
            .comp_solution(HoleId::from(0_u32))
            .expect("the intersection must bind the metavariable");
        let Comp::Abs(_, _, ref outer) = *solution.as_ref()
        else {
            panic!("the solution must abstract over the first spine position");
        };
        let Comp::Abs(_, _, ref inner) = *outer.as_ref()
        else {
            panic!("the solution must abstract over the second spine position");
        };
        assert!(matches!(*inner.as_ref(), Comp::Hole(_)));
    }

    #[test]
    fn a_thunk_metavariable_under_force_replays_at_its_declared_grade()
    {
        let (certificate, replay) = run_and_replay(
            context(&[(HoleId::from(0_u32), MetaSort::Thunk(Grade::ONE))]),
            vec![comps(
                Comp::lam(
                    "x",
                    Comp::app(Comp::force(Value::Hole(0)), Value::var(NameRef::from("x"))),
                ),
                Comp::lam(
                    "x",
                    Comp::ret(Value::Pair(
                        Rc::new(Value::var(NameRef::from("x"))),
                        Rc::new(Value::Int(1)),
                    )),
                ),
            )],
        );
        assert_eq!(certificate.verdict(), Verdict::Solved);
        let solution = certificate
            .value_solution(HoleId::from(0))
            .expect("the pattern must be solved");
        assert!(matches!(*solution.as_ref(), Value::Thunk(grade, _) if grade == Grade::ONE));
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn function_eta_is_a_solved_rule_and_replays()
    {
        // A lambda against a neutral: ordinary conversion relates them by
        // applying both to one fresh variable, so the solver may too.
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam(
                        "x",
                        Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                    ),
                    Comp::force(Value::var(NameRef::from("f"))),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn lazy_pair_eta_and_meta_splitting_replay()
    {
        // A lazy pair against a bare metavariable: eta projects both sides, the
        // projected metavariable splits, and both halves are then patterns.
        let force_f = || Comp::force(Value::var(NameRef::from("f")));
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::With(
                        Rc::new(Comp::Prj(Side::Fst, Rc::new(force_f()))),
                        Rc::new(Comp::Prj(Side::Snd, Rc::new(force_f()))),
                    ),
                    Comp::Hole(0),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert!(matches!(
            certificate
                .comp_solution(HoleId::from(0))
                .map(AsRef::as_ref),
            Some(&Comp::With(..))
        ));
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn meta_splitting_leaves_the_unconstrained_half_open_and_replays_vacuously()
    {
        // Only the first projection is constrained, so the second half stays an
        // unsolved metavariable inside the solution. The replay converts, and
        // says so as vacuous rather than validated, because a surviving hole is
        // exactly what conversion relates to everything.
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::Prj(Side::Fst, Rc::new(Comp::Hole(0))),
                    Comp::ret(Value::Int(1)),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert_eq!(replay, Replay::Vacuous);
    }

    #[test]
    fn return_congruence_solves_and_replays()
    {
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                comps(Comp::ret(Value::Hole(0)), Comp::ret(Value::Int(5))),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert_eq!(
            certificate
                .value_solution(HoleId::from(0))
                .map(AsRef::as_ref),
            Some(&Value::Int(5))
        );
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn positive_congruence_solves_componentwise_and_replays()
    {
        let (certificate, replay) = run_and_replay(
            context(&[
                (HoleId::from(0_u32), MetaSort::Value),
                (HoleId::from(1_u32), MetaSort::Value),
            ]),
            vec![values(
                Rc::new(Value::Pair(
                    value_hole(0),
                    Rc::new(Value::Inj(Side::Fst, value_hole(1))),
                )),
                Rc::new(Value::Pair(int(1), Rc::new(Value::Inj(Side::Fst, int(2))))),
            )],
        );
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert_eq!(
            certificate
                .value_solution(HoleId::from(1))
                .map(AsRef::as_ref),
            Some(&Value::Int(2))
        );
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn record_congruence_is_canonical_and_replays()
    {
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(
                    record(&[
                        (FieldName::from("beta"), int(2)),
                        (FieldName::from("alpha"), value_hole(0)),
                    ]),
                    record(&[
                        (FieldName::from("alpha"), int(1)),
                        (FieldName::from("beta"), int(2)),
                    ]),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert_eq!(
            certificate
                .value_solution(HoleId::from(0))
                .map(AsRef::as_ref),
            Some(&Value::Int(1))
        );
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn a_record_of_different_width_is_refuted_with_no_width_rule()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(
                    record(&[
                        (FieldName::from("alpha"), value_hole(0)),
                        (FieldName::from("beta"), int(2)),
                    ]),
                    record(&[(FieldName::from("alpha"), int(1))]),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Refuted(Refutation::Clash));
    }

    #[test]
    fn a_thunk_grade_mismatch_is_refuted()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(
                    Rc::new(Value::Thunk(Grade::ONE, Rc::new(Comp::ret(Value::Hole(0))))),
                    Rc::new(Value::Thunk(Grade::ZERO, Rc::new(Comp::ret(Value::Int(1))))),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Refuted(Refutation::Clash));
    }

    #[test]
    fn a_rigid_spine_decomposes_and_replays()
    {
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                comps(
                    Comp::app(Comp::force(Value::var(NameRef::from("f"))), Value::Hole(0)),
                    Comp::app(Comp::force(Value::var(NameRef::from("f"))), Value::Int(7)),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert_eq!(
            certificate
                .value_solution(HoleId::from(0))
                .map(AsRef::as_ref),
            Some(&Value::Int(7))
        );
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn same_metavariable_spines_intersect_to_their_agreeing_positions()
    {
        // `?a x y` against `?a y x` may depend on neither position, so the
        // intersection rule binds it to a constant abstraction.
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam(
                        "x",
                        Comp::lam(
                            "y",
                            Comp::app(
                                Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                                Value::var(NameRef::from("y")),
                            ),
                        ),
                    ),
                    Comp::lam(
                        "x",
                        Comp::lam(
                            "y",
                            Comp::app(
                                Comp::app(Comp::Hole(0), Value::var(NameRef::from("y"))),
                                Value::var(NameRef::from("x")),
                            ),
                        ),
                    ),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        let solution = certificate
            .comp_solution(HoleId::from(0))
            .expect("the intersection must bind the metavariable");
        assert!(matches!(*solution.as_ref(), Comp::Abs(..)));
        assert_eq!(replay, Replay::Vacuous);
    }

    #[test]
    fn flex_flex_solves_from_the_covering_side()
    {
        // `?a x` against `?b x y`: the second spine covers the first, so the
        // second metavariable is the one that can be bound.
        let (certificate, replay) = run_and_replay(
            context(&[
                (HoleId::from(0_u32), MetaSort::Comp),
                (HoleId::from(1_u32), MetaSort::Comp),
            ]),
            vec![comps(
                Comp::lam(
                    "x",
                    Comp::lam(
                        "y",
                        Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                    ),
                ),
                Comp::lam(
                    "x",
                    Comp::lam(
                        "y",
                        Comp::app(
                            Comp::app(Comp::Hole(1), Value::var(NameRef::from("x"))),
                            Value::var(NameRef::from("y")),
                        ),
                    ),
                ),
            )],
        );
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert!(certificate.comp_solution(HoleId::from(1)).is_some());
        assert_eq!(replay, Replay::Vacuous);
    }

    #[test]
    fn running_twice_resumes_from_the_bindings_already_made()
    {
        let mut nbe = Normalizer::new();
        let mut solver = Solver::new(context(&[(HoleId::from(0_u32), MetaSort::Value)]));
        solver.push(values(value_hole(0), int(3)));
        let first = solver.run(&mut nbe).expect("the first run must not fail");
        assert_eq!(first.verdict(), Verdict::Solved);
        solver.push(values(value_hole(0), int(3)));
        let second = solver.run(&mut nbe).expect("the second run must not fail");
        assert_eq!(second.verdict(), Verdict::Solved);
        assert_eq!(
            second.value_solution(HoleId::from(0)).map(AsRef::as_ref),
            Some(&Value::Int(3))
        );
    }

    #[test]
    fn a_second_run_refutes_a_constraint_the_first_binding_contradicts()
    {
        let mut nbe = Normalizer::new();
        let mut solver = Solver::new(context(&[(HoleId::from(0_u32), MetaSort::Value)]));
        solver.push(values(value_hole(0), int(3)));
        solver.push(values(value_hole(0), int(4)));
        let certificate = solver.run(&mut nbe).expect("the run must not fail");
        assert_eq!(certificate.verdict(), Verdict::Refuted(Refutation::Clash));
    }

    #[test]
    fn a_pack_congruence_solves_through_the_payload_and_replays()
    {
        // Both sides carry the same interned witness, so the id fast-path
        // answers the witness comparison and the payload is the one residual
        // equation.
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(packed(Value::Hole(0)), packed(Value::Int(3))),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert_eq!(
            certificate
                .value_solution(HoleId::from(0))
                .map(AsRef::as_ref),
            Some(&Value::Int(3))
        );
        assert_eq!(replay, Replay::Validated);
    }

    #[test]
    fn alpha_variant_witnesses_and_convertible_payloads_solve_and_replay()
    {
        // The witnesses differ only in how they spell the abstract component —
        // distinct interned types, one canonical key — and the payloads are
        // α-distinct spellings of one thunk. Neither difference is one a
        // substitution repairs; both are what the comparison is defined to see
        // through.
        let signature = |label: &str| ValueType::Package {
            grade: Grade::ONE,
            abstracts: vec![label.to_owned()],
            payload: Rc::new(ValueType::Thunk(
                Grade::ONE,
                Rc::new(crate::types::CompType::returner(ValueType::atom(label))),
            )),
        };
        let packed_identity = |witness: ValueType, binder: &str| {
            Rc::new(Value::pack(
                [witness],
                Value::Thunk(
                    Grade::ONE,
                    Rc::new(Comp::lam(
                        binder,
                        Comp::ret(Value::var(NameRef::from(binder))),
                    )),
                ),
            ))
        };
        let (certificate, replay) =
            run_and_replay(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(
                    Rc::new(Value::Pair(
                        packed_identity(signature("left"), "x"),
                        value_hole(0),
                    )),
                    Rc::new(Value::Pair(
                        packed_identity(signature("right"), "y"),
                        int(7),
                    )),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Solved);
        assert_eq!(
            certificate
                .value_solution(HoleId::from(0))
                .map(AsRef::as_ref),
            Some(&Value::Int(7))
        );
        assert_eq!(replay, Replay::Validated);
    }

    // ── refutations ─────────────────────────────────────────────────────────

    #[test]
    fn a_rigid_clash_is_refuted()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(
                    Rc::new(Value::Pair(value_hole(0), int(1))),
                    Rc::new(Value::Pair(int(2), int(9))),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Refuted(Refutation::Clash));
    }

    #[test]
    fn a_rigid_occurrence_is_refuted()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(value_hole(0), Rc::new(Value::Pair(value_hole(0), int(1)))),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Refuted(Refutation::Occurs));
    }

    #[test]
    fn a_rigid_escape_is_refuted()
    {
        // The metavariable is closed and wears no spine, so it can never
        // produce the variable the binder opened.
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam("x", Comp::Hole(0)),
                    Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                ),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Refuted(Refutation::Escape));
    }

    #[test]
    fn a_pack_witness_mismatch_is_a_clash_no_solution_can_justify()
    {
        // The witnesses are types and no substitution rewrites a type, so the
        // mismatch decides the pair even with a metavariable still open — the
        // solver's clash and conversion's refusal are the one verdict.
        let left = packed(Value::Hole(0));
        let right = Rc::new(Value::pack([ValueType::Unit], Value::Hole(0)));
        let (mut nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(Rc::clone(&left), Rc::clone(&right)),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Refuted(Refutation::Clash));
        assert!(!bool::from(nbe.converts(&left, &right)));
    }

    #[test]
    fn matching_witnesses_with_mismatched_payloads_fail_in_solver_and_conversion()
    {
        // The open metavariable elsewhere keeps the solver on the walking
        // path, so the payload clash is the solver's own verdict, reached
        // through the pack congruence — and conversion refuses the same pair.
        let left = Rc::new(Value::Pair(packed(Value::Int(1)), value_hole(0)));
        let right = Rc::new(Value::Pair(packed(Value::Int(2)), int(9)));
        let (mut nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(Rc::clone(&left), Rc::clone(&right)),
            ]);
        assert_eq!(certificate.verdict(), Verdict::Refuted(Refutation::Clash));
        assert!(!bool::from(nbe.converts(&left, &right)));
    }

    #[test]
    fn a_refuting_certificate_has_nothing_to_replay()
    {
        let (mut nbe, solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(
                    Rc::new(Value::Pair(value_hole(0), int(1))),
                    Rc::new(Value::Pair(int(2), int(9))),
                ),
            ]);
        assert_eq!(
            certificate.validate(&mut nbe, solver.constraints()),
            Replay::Unproven
        );
    }

    // ── the postponement boundaries, one witness each ───────────────────────

    #[test]
    fn a_non_pattern_spine_argument_postpones()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam(
                        "x",
                        Comp::app(Comp::Hole(0), Value::var(NameRef::from("g"))),
                    ),
                    Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::NonPatternSpine
        );
    }

    #[test]
    fn a_repeated_spine_variable_postpones()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam(
                        "x",
                        Comp::app(
                            Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                            Value::var(NameRef::from("x")),
                        ),
                    ),
                    Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::RepeatedSpineVariable
        );
    }

    #[test]
    fn a_constructor_in_a_spine_postpones()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam("x", Comp::app(Comp::Hole(0), Value::Int(1))),
                    Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::ConstructorInSpine
        );
    }

    #[test]
    fn a_sequencing_continuation_in_a_spine_postpones()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::bind(
                        Comp::Hole(0),
                        "r",
                        Comp::ret(Value::var(NameRef::from("r"))),
                    ),
                    Comp::ret(Value::Int(1)),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::SequencedSpine
        );
    }

    #[test]
    fn a_projection_behind_an_application_postpones()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam(
                        "x",
                        Comp::Prj(
                            Side::Fst,
                            Rc::new(Comp::app(Comp::Hole(0), Value::var(NameRef::from("x")))),
                        ),
                    ),
                    Comp::lam("x", Comp::ret(Value::var(NameRef::from("x")))),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::ProjectionAfterApplication
        );
    }

    #[test]
    fn a_value_metavariable_under_force_with_no_declared_grade_postpones()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                comps(Comp::force(Value::Hole(0)), Comp::ret(Value::Int(1))),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::UndeclaredThunkGrade
        );
    }

    #[test]
    fn a_prunable_occurrence_postpones_rather_than_refuting()
    {
        let (_nbe, _solver, certificate) = run(
            context(&[
                (HoleId::from(0_u32), MetaSort::Value),
                (HoleId::from(1_u32), MetaSort::Value),
            ]),
            vec![values(
                value_hole(0),
                Rc::new(Value::Pair(value_hole(0), value_hole(1))),
            )],
        );
        let residual = sole_residual(&certificate);
        assert_eq!(residual.reason(), PostponeReason::FlexOccurs);
        assert_eq!(residual.blockers(), &[HoleId::from(0)]);
    }

    #[test]
    fn a_prunable_escape_postpones_rather_than_refuting()
    {
        let (_nbe, _solver, certificate) = run(
            context(&[
                (HoleId::from(0_u32), MetaSort::Comp),
                (HoleId::from(1_u32), MetaSort::Value),
            ]),
            vec![comps(
                Comp::lam("x", Comp::Hole(0)),
                Comp::lam(
                    "x",
                    Comp::ret(Value::Pair(
                        Rc::new(Value::var(NameRef::from("x"))),
                        value_hole(1),
                    )),
                ),
            )],
        );
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::FlexEscape
        );
    }

    #[test]
    fn a_flex_flex_pair_needing_an_intersection_postpones()
    {
        let (_nbe, _solver, certificate) = run(
            context(&[
                (HoleId::from(0_u32), MetaSort::Comp),
                (HoleId::from(1_u32), MetaSort::Comp),
            ]),
            vec![comps(
                Comp::lam(
                    "x",
                    Comp::lam(
                        "y",
                        Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                    ),
                ),
                Comp::lam(
                    "x",
                    Comp::lam(
                        "y",
                        Comp::app(Comp::Hole(1), Value::var(NameRef::from("y"))),
                    ),
                ),
            )],
        );
        let residual = sole_residual(&certificate);
        assert_eq!(residual.reason(), PostponeReason::FlexFlexIntersection);
        assert_eq!(residual.blockers(), &[HoleId::from(0), HoleId::from(1)]);
    }

    #[test]
    fn one_metavariable_under_spines_of_different_length_postpones()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Comp)]), vec![
                comps(
                    Comp::lam(
                        "x",
                        Comp::lam(
                            "y",
                            Comp::app(
                                Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                                Value::var(NameRef::from("y")),
                            ),
                        ),
                    ),
                    Comp::lam(
                        "x",
                        Comp::lam(
                            "y",
                            Comp::app(Comp::Hole(0), Value::var(NameRef::from("x"))),
                        ),
                    ),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::FlexFlexMismatchedSpines
        );
    }

    #[test]
    fn an_elimination_blocked_on_a_metavariable_postpones_and_names_it()
    {
        let arm = |name: &str| {
            (
                name.to_owned(),
                Rc::new(Comp::ret(Value::var(NameRef::from(name)))),
            )
        };
        let case = |scrutinee: Rc<Value>| Comp::Case(scrutinee, arm("l"), arm("r"));
        let (_nbe, _solver, certificate) = run(
            context(&[
                (HoleId::from(0_u32), MetaSort::Value),
                (HoleId::from(1_u32), MetaSort::Value),
            ]),
            vec![comps(case(value_hole(0)), case(value_hole(1)))],
        );
        let residual = sole_residual(&certificate);
        assert_eq!(residual.reason(), PostponeReason::BlockedElimination);
        assert_eq!(residual.blockers(), &[HoleId::from(0), HoleId::from(1)]);
    }

    #[test]
    fn two_reified_stacks_carrying_a_metavariable_postpone()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(
                    Rc::new(Value::Stk(Rc::new(crate::syntax::Stack::Arg(
                        value_hole(0),
                        Rc::new(crate::syntax::Stack::Empty),
                    )))),
                    Rc::new(Value::Stk(Rc::new(crate::syntax::Stack::Arg(
                        int(1),
                        Rc::new(crate::syntax::Stack::Arg(
                            int(2),
                            Rc::new(crate::syntax::Stack::Empty),
                        )),
                    )))),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::OpaqueReifiedStack
        );
    }

    #[test]
    fn a_canonical_value_facing_a_neutral_postpones_as_positive_eta()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(Rc::new(Value::Pair(value_hole(0), int(1))), var("p")),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::PositiveEta
        );
    }

    #[test]
    fn the_unit_value_facing_a_neutral_postpones_as_a_definitional_singleton()
    {
        // A type-directed conversion would relate these, because the unit type
        // has one inhabitant. Ordinary conversion does not, so the solver may
        // neither solve the equation nor refute it, and names the theory it
        // would need instead.
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                values(
                    Rc::new(Value::Pair(Rc::new(Value::Unit), value_hole(0))),
                    Rc::new(Value::Pair(var("p"), int(1))),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::DefinitionalSingleton
        );
    }

    #[test]
    fn a_returner_facing_a_rigid_neutral_postpones_as_a_head_mismatch()
    {
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(0_u32), MetaSort::Value)]), vec![
                comps(
                    Comp::ret(Value::Hole(0)),
                    Comp::force(Value::var(NameRef::from("f"))),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::HeadMismatch
        );
    }

    #[test]
    fn an_undeclared_hole_postpones_rather_than_riding_hole_consistency()
    {
        // Conversion relates an undeclared hole to every value, and that
        // relation is not transitive, so a discharge resting on it would
        // compose with nothing. The solver refuses the discharge instead.
        let (_nbe, _solver, certificate) =
            run(context(&[(HoleId::from(1_u32), MetaSort::Value)]), vec![
                values(
                    Rc::new(Value::Pair(value_hole(0), value_hole(1))),
                    Rc::new(Value::Pair(int(1), int(2))),
                ),
            ]);
        assert_eq!(
            sole_residual(&certificate).reason(),
            PostponeReason::HoleConsistency
        );
    }

    #[test]
    fn an_exhausted_budget_postpones_everything_left()
    {
        let mut nbe = Normalizer::new();
        let mut solver = Solver::new(context(&[(HoleId::from(0_u32), MetaSort::Value)]))
            .with_budget(crate::boundary::SolverBudget::from(0));
        solver.push(values(value_hole(0), int(3)));
        solver.push(values(value_hole(0), int(3)));
        let certificate = solver.run(&mut nbe).expect("the run must not fail");
        assert_eq!(certificate.verdict(), Verdict::Postponed);
        assert_eq!(certificate.residual().len(), 2);
        for postponed in certificate.residual() {
            assert_eq!(postponed.reason(), PostponeReason::BudgetExhausted);
        }
    }

    // ── the negative certificates: what the residue would cost if solved ────

    #[test]
    fn a_hand_built_singleton_eta_certificate_is_refuted_by_replay()
    {
        // The unit type has one inhabitant, so a type-directed conversion would
        // accept `?a := unit` for `?a = x`. Ordinary conversion does not, and
        // the replay is what says so — which is why the solver may not emit it.
        let mut bindings = HoleSubstitution::new();
        bindings.bind(HoleId::from(0), HoleRepl::Value(Rc::new(Value::Unit)));
        let certificate =
            Certificate::new(Verdict::Solved, bindings, Vec::new(), SolverSteps::from(0));
        let constraints = vec![values(value_hole(0), var("x"))];
        let mut nbe = Normalizer::new();
        assert_eq!(
            certificate.validate(&mut nbe, &constraints),
            Replay::Refuted
        );
    }

    #[test]
    fn a_hand_built_surjective_pairing_certificate_is_refuted_by_replay()
    {
        let mut bindings = HoleSubstitution::new();
        bindings.bind(
            HoleId::from(0),
            HoleRepl::Value(Rc::new(Value::Pair(var("a"), var("b")))),
        );
        let certificate =
            Certificate::new(Verdict::Solved, bindings, Vec::new(), SolverSteps::from(0));
        let constraints = vec![values(value_hole(0), var("p"))];
        let mut nbe = Normalizer::new();
        assert_eq!(
            certificate.validate(&mut nbe, &constraints),
            Replay::Refuted
        );
    }

    // ── the metacontext and the substitution it carries ─────────────────────

    #[test]
    fn a_minted_metavariable_avoids_declared_identities()
    {
        let mut metas = MetaContext::new(HoleId::from(0));
        metas.declare(HoleId::from(7), MetaSort::Value);
        let first = metas.fresh(MetaSort::Comp);
        let second = metas.fresh(MetaSort::Comp);
        assert_eq!(first, HoleId::from(8));
        assert_eq!(second, HoleId::from(9));
        assert!(bool::from(metas.is_meta(first)));
    }

    #[test]
    fn zonking_resolves_a_chained_solution()
    {
        let mut metas = MetaContext::new(HoleId::from(0));
        metas.declare(HoleId::from(0), MetaSort::Value);
        metas.declare(HoleId::from(1), MetaSort::Value);
        metas.solve(
            HoleId::from(0),
            HoleRepl::Value(Rc::new(Value::Pair(value_hole(1), int(1)))),
        );
        metas.solve(HoleId::from(1), HoleRepl::Value(int(2)));
        let zonked = metas.zonked();
        assert_eq!(
            zonked.value(HoleId::from(0)).map(AsRef::as_ref),
            Some(&Value::Pair(int(2), int(1)))
        );
    }

    #[test]
    fn zonking_leaves_an_unsolved_metavariable_alone()
    {
        let mut metas = MetaContext::new(HoleId::from(0));
        metas.declare(HoleId::from(0), MetaSort::Value);
        metas.declare(HoleId::from(1), MetaSort::Value);
        metas.solve(
            HoleId::from(0),
            HoleRepl::Value(Rc::new(Value::Pair(value_hole(1), int(1)))),
        );
        let zonked = metas.zonked();
        assert_eq!(
            zonked.value(HoleId::from(0)).map(AsRef::as_ref),
            Some(&Value::Pair(value_hole(1), int(1)))
        );
    }

    #[test]
    fn substituting_a_solution_reports_a_hole_free_result()
    {
        let mut bindings = HoleSubstitution::new();
        bindings.bind(HoleId::from(0), HoleRepl::Value(int(3)));
        let (term, holes) =
            crate::subst::subst_holes_value(&Value::Pair(value_hole(0), int(1)), &bindings);
        assert_eq!(term, Value::Pair(int(3), int(1)));
        assert!(!bool::from(holes));
    }

    #[test]
    fn substituting_leaves_an_unsolved_hole_and_reports_it()
    {
        let bindings = HoleSubstitution::new();
        let (term, holes) =
            crate::subst::subst_holes_value(&Value::Pair(value_hole(0), int(1)), &bindings);
        assert_eq!(term, Value::Pair(value_hole(0), int(1)));
        assert!(bool::from(holes));
    }

    #[test]
    fn substituting_a_computation_solution_beta_reduces_on_replay()
    {
        let mut bindings = HoleSubstitution::new();
        bindings.bind(
            HoleId::from(0),
            HoleRepl::Comp(Rc::new(Comp::lam(
                "z",
                Comp::ret(Value::var(NameRef::from("z"))),
            ))),
        );
        let (term, holes) =
            crate::subst::subst_holes_comp(&Comp::app(Comp::Hole(0), Value::Int(4)), &bindings);
        assert!(!bool::from(holes));
        let mut nbe = Normalizer::new();
        assert!(bool::from(
            nbe.converts(&thunk(term), &thunk(Comp::ret(Value::Int(4))))
        ));
    }

    #[test]
    fn a_substitution_reaches_an_unpack_body_only_past_a_different_binder()
    {
        let unpack = |scrut: Value, body: Comp| {
            Value::Thunk(
                Grade::ONE,
                Rc::new(Comp::unpack(
                    scrut,
                    ValueType::integer(),
                    [seal(crate::boundary::TypeSerial::from(0))],
                    "m",
                    body,
                )),
            )
        };
        // The module binder differs: the substitution reaches the scrutinee
        // and the body.
        let reached = crate::subst::subst_value(
            &unpack(
                Value::var(NameRef::from("x")),
                Comp::ret(Value::var(NameRef::from("x"))),
            ),
            "x",
            &Value::Int(3),
        );
        assert_eq!(reached, unpack(Value::Int(3), Comp::ret(Value::Int(3))));
        // The module binder rebinds the name: the body is blocked, and the
        // scrutinee — outside the binder's scope — is not.
        let blocked = crate::subst::subst_value(
            &unpack(
                Value::var(NameRef::from("m")),
                Comp::ret(Value::var(NameRef::from("m"))),
            ),
            "m",
            &Value::Int(3),
        );
        assert_eq!(
            blocked,
            unpack(Value::Int(3), Comp::ret(Value::var(NameRef::from("m"))))
        );
    }

    // ── the occurrence scan ─────────────────────────────────────────────────

    #[test]
    fn a_scan_allows_the_spine_levels_and_flags_the_others()
    {
        let allowed = crate::boundary::VariableLevel::from(0);
        let ceiling = crate::boundary::VariableLevel::from(2);
        let inside = Value::var(NameRef::from(level_name(allowed).as_str()));
        let outside = Value::var(NameRef::from(
            level_name(crate::boundary::VariableLevel::from(1)).as_str(),
        ));
        assert!(!bool::from(
            scan::scan_value(&inside, ceiling, &[allowed]).escapes()
        ));
        assert!(bool::from(
            scan::scan_value(&outside, ceiling, &[allowed]).escapes()
        ));
    }

    #[test]
    fn a_scan_ignores_readback_binders_and_source_names()
    {
        let ceiling = crate::boundary::VariableLevel::from(1);
        let binder = Value::var(NameRef::from(
            level_name(crate::boundary::VariableLevel::from(5)).as_str(),
        ));
        let source = Value::var(NameRef::from("x"));
        assert!(!bool::from(
            scan::scan_value(&binder, ceiling, &[]).escapes()
        ));
        assert!(!bool::from(
            scan::scan_value(&source, ceiling, &[]).escapes()
        ));
    }

    #[test]
    fn a_scan_reaches_through_an_application_spine()
    {
        let ceiling = crate::boundary::VariableLevel::from(2);
        let term = Comp::app(
            Comp::Hole(3),
            Value::var(NameRef::from(
                level_name(crate::boundary::VariableLevel::from(1)).as_str(),
            )),
        );
        let found = scan::scan_comp(&term, ceiling, &[]);
        assert!(bool::from(found.escapes()));
        assert!(bool::from(found.mentions(HoleId::from(3))));
    }

    #[test]
    fn a_scan_reaches_through_a_pack_and_an_unpack()
    {
        let ceiling = crate::boundary::VariableLevel::from(2);
        let escaped = level_name(crate::boundary::VariableLevel::from(1));
        // The pack's payload is the one term child: the hole inside it is
        // found, and a disallowed level inside it is flagged.
        let found = scan::scan_value(&packed(Value::Hole(0)), ceiling, &[]);
        assert!(bool::from(found.mentions(HoleId::from(0))));
        let found = scan::scan_value(
            &packed(Value::var(NameRef::from(escaped.as_str()))),
            ceiling,
            &[],
        );
        assert!(bool::from(found.escapes()));
        // The unpack's term children are the scrutinee and the body: the hole
        // in the scrutinee is found and the body's disallowed level is
        // flagged, past the signature, the atoms, and the module binder.
        let term = Comp::unpack(
            Value::Hole(0),
            ValueType::integer(),
            [seal(crate::boundary::TypeSerial::from(0))],
            "m",
            Comp::ret(Value::var(NameRef::from(escaped.as_str()))),
        );
        let found = scan::scan_comp(&term, ceiling, &[]);
        assert!(bool::from(found.mentions(HoleId::from(0))));
        assert!(bool::from(found.escapes()));
    }

    // ── the agreement property ──────────────────────────────────────────────

    /// Values over a small grammar, deep enough to exercise every congruence
    /// rule and shallow enough to keep the pair generator's collision rate up.
    fn arb_value() -> impl Strategy<Value = Rc<Value>>
    {
        let leaf = prop_oneof![
            Just(Value::Unit),
            (0_i64 .. 3).prop_map(Value::Int),
            prop_oneof![Just("x"), Just("y"), Just("f")]
                .prop_map(|name| Value::var(NameRef::from(name))),
        ];
        leaf.prop_recursive(3, 24, 2, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(fst, snd)| Value::Pair(Rc::new(fst), Rc::new(snd))),
                (any::<bool>(), inner.clone()).prop_map(|(left, payload)| Value::Inj(
                    if left { Side::Fst } else { Side::Snd },
                    Rc::new(payload)
                )),
                inner
                    .clone()
                    .prop_map(|body| Value::Thunk(Grade::ONE, Rc::new(Comp::ret(body)))),
                (inner.clone(), inner).prop_map(|(alpha, beta)| {
                    let mut fields = BTreeMap::new();
                    fields.insert("alpha".to_owned(), Rc::new(alpha));
                    fields.insert("beta".to_owned(), Rc::new(beta));
                    Value::Record(fields)
                }),
            ]
        })
        .prop_map(Rc::new)
    }

    /// Pairs biased towards agreement, because two independently generated
    /// terms almost never convert and the equal case is where a mutant hides.
    fn arb_pair() -> impl Strategy<Value = (Rc<Value>, Rc<Value>)>
    {
        prop_oneof![
            arb_value().prop_map(|value| (Rc::clone(&value), value)),
            (arb_value(), arb_value()),
        ]
    }

    proptest! {
        /// The solver decides a metavariable-free problem exactly as ordinary
        /// conversion decides it — the agreement the whole design rests on.
        #[test]
        fn a_metavariable_free_problem_agrees_with_conversion((lhs, rhs) in arb_pair())
        {
            let mut nbe = Normalizer::new();
            let expected = bool::from(nbe.converts(&lhs, &rhs));
            let mut solver = Solver::new(MetaContext::new(HoleId::from(0)));
            solver.push(Constraint::Values(Rc::clone(&lhs), Rc::clone(&rhs)));
            let certificate = solver.run(&mut nbe).expect("the run must not fail");
            let solved = matches!(certificate.verdict(), Verdict::Solved);
            prop_assert_eq!(solved, expected);
            prop_assert!(certificate.residual().is_empty());
        }

        /// Whatever the solver claims to have solved, ordinary conversion
        /// accepts after substitution. A refuted replay here is a defect.
        #[test]
        fn every_solved_verdict_replays_without_refutation((lhs, rhs) in arb_pair())
        {
            let mut nbe = Normalizer::new();
            let mut solver = Solver::new(context(&[(HoleId::from(0_u32), MetaSort::Value)]));
            solver.push(Constraint::Values(
                Rc::new(Value::Pair(value_hole(0), Rc::clone(&lhs))),
                Rc::new(Value::Pair(Rc::clone(&rhs), Rc::clone(&rhs))),
            ));
            let certificate = solver.run(&mut nbe).expect("the run must not fail");
            let replay = certificate.validate(&mut nbe, solver.constraints());
            prop_assert_ne!(replay, Replay::Refuted);
        }
    }
}
