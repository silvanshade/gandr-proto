//! The per-declaration level context ([`LevelContext`]) and the universe
//! strict comparison the checker decides against it.
//!
//! ADR-78 discipline: a declaration is generalized over its **own** prenex
//! level parameters and carries its **own** declared landmark poset; the
//! kernel checks it against that context and **no global level state**
//! (kernel-boundary.md §3/§4). The context pins:
//!
//! * `params` — the prenex level-parameter count. A `Level` is well-scoped iff
//!   every variable index is below it ([`LevelContext::check_level_scope`]).
//! * `poset` — the admitted landmark poset. Admission (Bezem–Coquand
//!   loop-checking) is a soundness gate: an inconsistent constraint set would
//!   re-admit `U_l : U_l` and is rejected at [`LevelContext::admit`].
//!
//! The universe rule `U_l : U_m` iff `l < m` (ADR-78) is decided by
//! [`LevelContext::check_universe_below`]. With **no** landmark constraints
//! declared — the S1 v0 default — this is exactly
//! `gandr_kernel_strata::Level`'s free-fragment `lt` oracle (the coordinator's
//! "one call into `Level::lt`"); with constraints declared, it is landmark
//! entailment under the poset's hypotheses (`entails_lt`), which strata
//! guarantees agrees with the free oracle on the empty poset. Routing through
//! entailment when hypotheses exist makes the birth-declared landmark feature
//! load-bearing without a later boundary change, and degenerates to the free
//! oracle otherwise.

use alloc::boxed::Box;
use alloc::vec::Vec;

use gandr_kernel_strata::AdmissionOutcome;
use gandr_kernel_strata::Entailment;
use gandr_kernel_strata::LandmarkConstraint;
use gandr_kernel_strata::LandmarkPoset;
use gandr_kernel_strata::Level;
use gandr_kernel_strata::LevelVar;

use crate::error::KernelError;
use crate::error::LevelOrderRefutation;
use crate::error::UniverseViolation;

/// A prenex level-parameter count: the number of level variables a
/// declaration is generalized over.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LevelParamCount(u32);

impl From<u32> for LevelParamCount
{
    #[inline]
    fn from(count: u32) -> Self
    {
        Self(count)
    }
}

impl From<LevelParamCount> for u32
{
    #[inline]
    fn from(count: LevelParamCount) -> Self
    {
        count.0
    }
}

/// The admitted level context of one declaration: its prenex parameter count
/// and its consistency-checked landmark poset.
#[derive(Clone, Debug)]
pub struct LevelContext
{
    /// The prenex level-parameter count.
    params: LevelParamCount,
    /// The admitted landmark poset (empty when no constraints are declared).
    poset: LandmarkPoset,
}

impl LevelContext
{
    /// Build a level context from a declaration's prenex parameters and
    /// declared landmark constraints, admitting the constraints.
    ///
    /// # Contract
    /// - requires: each constraint is a well-formed (variable-only)
    ///   `gandr_kernel_strata::LandmarkConstraint`.
    /// - ensures: `Ok(context)` carrying an admitted poset when every
    ///   constraint variable is within `params` and the constraint set has a
    ///   model; the poset then decides landmark-aware universe comparisons.
    /// - provides: the per-declaration level context (ADR-78: no global state).
    /// - fails: [`KernelError::LevelVariableOutOfScope`] when a constraint
    ///   mentions a variable at or above `params`;
    ///   [`KernelError::InconsistentLevelConstraints`] when the constraints
    ///   loop (no model — the consistency-bearing gate);
    ///   [`KernelError::LevelOracleFault`] on a strata fault the theory
    ///   excludes.
    /// - panics: none.
    ///
    /// # Errors
    /// [`KernelError::LevelVariableOutOfScope`],
    /// [`KernelError::InconsistentLevelConstraints`],
    /// [`KernelError::LevelOracleFault`].
    ///
    /// # Adequacy
    /// - hypothesis: L1/L2 — admission returns the strata dichotomy as evidence
    ///   (a consistency witness or a loop witness), so the two admission arms
    ///   are pinned by the empty-context and the self-successor-loop goldens
    ///   against strata's own validators; the L3 residue is the scope-boundary
    ///   check, pinned by an at-`params` constraint variable.
    /// - witness: `levels::tests::empty_context_admits`
    /// - witness: `levels::tests::looping_constraints_are_rejected`
    /// - witness: `levels::tests::out_of_scope_constraint_variable_is_rejected`
    #[inline]
    pub fn admit(
        params: LevelParamCount,
        constraints: Vec<LandmarkConstraint>,
    ) -> Result<Self, KernelError>
    {
        for constraint in &constraints {
            for variable in constraint_variables(constraint) {
                check_scope(params, variable)?;
            }
        }
        let outcome = LandmarkPoset::admit(constraints).map_err(KernelError::LevelOracleFault)?;
        match outcome {
            | AdmissionOutcome::Admitted(poset) => Ok(Self { params, poset }),
            | AdmissionOutcome::Loop(witness) => {
                Err(KernelError::InconsistentLevelConstraints(Box::new(witness)))
            },
        }
    }

    /// The prenex level-parameter count.
    #[inline]
    #[must_use]
    pub const fn params(&self) -> LevelParamCount
    {
        self.params
    }

    /// Check that every variable of `level` is within the prenex parameters.
    ///
    /// # Contract
    /// - requires: `level` is a canonical `gandr_kernel_strata::Level`
    ///   (guaranteed by that type's construction).
    /// - ensures: `Ok(())` exactly when every variable index is strictly below
    ///   the parameter count.
    /// - provides: the K1 scope check for a level embedded in a type
    ///   (`Universe`, `Lift`) or a declared constraint.
    /// - fails: [`KernelError::LevelVariableOutOfScope`] naming the first
    ///   out-of-scope variable, in ascending order.
    /// - panics: none.
    ///
    /// # Errors
    /// [`KernelError::LevelVariableOutOfScope`].
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the boundary is `index < params`, pinned by an
    ///   at-`params` variable (rejected) and an in-scope variable (accepted).
    /// - witness: `levels::tests::level_scope_boundary_is_exact`
    #[inline]
    pub fn check_level_scope(
        &self,
        level: &Level,
    ) -> Result<(), KernelError>
    {
        for (variable, _offset) in level.atoms() {
            check_scope(self.params, variable)?;
        }
        Ok(())
    }

    /// Decide the strict universe order `lower < upper` (the rule `U_lower :
    /// U_upper`), the free oracle degenerating from landmark entailment.
    ///
    /// # Contract
    /// - requires: `lower` and `upper` are well-scoped in this context (callers
    ///   apply [`Self::check_level_scope`] to embedded levels first).
    /// - ensures: `Ok(())` exactly when `lower < upper` holds — under the
    ///   free-fragment oracle (`gandr_kernel_strata::Level::lt`) when no
    ///   landmark constraints are declared, and under landmark entailment
    ///   (`entails_lt`) otherwise, which agrees with the free oracle on the
    ///   empty poset.
    /// - provides: the ADR-78 universe rule and the strictness check of an
    ///   explicit lift.
    /// - fails: [`KernelError::UniverseViolation`] carrying the strata
    ///   refutation when the order does not hold;
    ///   [`KernelError::LevelOracleFault`] on a strata fault the theory
    ///   excludes.
    /// - panics: none.
    ///
    /// # Errors
    /// [`KernelError::UniverseViolation`], [`KernelError::LevelOracleFault`].
    ///
    /// # Adequacy
    /// - hypothesis: L1/L2 — the decision returns strata evidence either way (a
    ///   witness or a refutation), so a mutant deciding wrongly must forge
    ///   coherent evidence; agreement of the empty-poset path with `Level::lt`
    ///   is the strata slice-2 gate. The L3 residue is irreflexivity (`l < l`
    ///   refused) and the successor boundary (`l < l+1` accepted), pinned by
    ///   unit goldens.
    /// - witness: `levels::tests::universe_rule_is_the_free_lt_on_the_empty_context`
    /// - witness: `levels::tests::universe_rule_is_irreflexive`
    /// - witness: `levels::tests::landmark_hypothesis_decides_the_universe_rule`
    #[inline]
    pub fn check_universe_below(
        &self,
        lower: &Level,
        upper: &Level,
    ) -> Result<(), KernelError>
    {
        if self.poset.constraints().is_empty() {
            return match lower.lt_with_evidence(upper) {
                | Ok(_witness) => Ok(()),
                | Err(refutation) => Err(KernelError::UniverseViolation(Box::new(
                    UniverseViolation::new(
                        lower.clone(),
                        upper.clone(),
                        LevelOrderRefutation::Free(refutation),
                    ),
                ))),
            };
        }
        let entailment = self
            .poset
            .entails_lt_with_evidence(lower, upper)
            .map_err(KernelError::LevelOracleFault)?;
        match entailment {
            | Entailment::Holds(_witness) => Ok(()),
            | Entailment::Refuted(countermodel) => Err(KernelError::UniverseViolation(Box::new(
                UniverseViolation::new(
                    lower.clone(),
                    upper.clone(),
                    LevelOrderRefutation::Landmark(Box::new(countermodel)),
                ),
            ))),
        }
    }
}

/// The variables a landmark constraint mentions, in ascending order across
/// both sides.
#[inline]
fn constraint_variables(constraint: &LandmarkConstraint) -> impl Iterator<Item = LevelVar> + '_
{
    let left = constraint
        .left()
        .atoms()
        .map(|(variable, _offset)| variable);
    let right = constraint
        .right()
        .atoms()
        .map(|(variable, _offset)| variable);
    left.chain(right)
}

/// Reject a level variable at or above the prenex parameter count.
#[inline]
fn check_scope(
    params: LevelParamCount,
    variable: LevelVar,
) -> Result<(), KernelError>
{
    if u32::from(variable.index()) < u32::from(params) {
        Ok(())
    }
    else {
        Err(KernelError::LevelVariableOutOfScope { variable })
    }
}

#[cfg(test)]
mod tests
{
    use alloc::vec;

    use gandr_kernel_strata::LandmarkConstraint;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;

    use super::LevelContext;
    use super::LevelParamCount;
    use crate::error::KernelError;

    /// A level variable by index.
    fn var(index: LevelVarIndex) -> LevelVar
    {
        LevelVar::new(index)
    }

    /// The level of the variable `index`.
    fn level_var(index: LevelVarIndex) -> Level
    {
        Level::var(var(index))
    }

    /// The constant level `value`.
    fn constant(value: LevelConstant) -> Level
    {
        Level::constant(value)
    }

    /// An empty context over `params` parameters.
    fn empty_context(params: LevelParamCount) -> LevelContext
    {
        LevelContext::admit(params, vec![]).unwrap()
    }

    #[test]
    fn empty_context_admits()
    {
        let context = empty_context(0_u32.into());
        assert_eq!(u32::from(context.params()), 0, "no prenex parameters");
    }

    #[test]
    fn universe_rule_is_the_free_lt_on_the_empty_context()
    {
        let context = empty_context(0_u32.into());
        assert_eq!(
            Ok(()),
            context.check_universe_below(&constant(0_u64.into()), &constant(1_u64.into())),
            "U_0 : U_1 holds"
        );
        assert!(
            matches!(
                context.check_universe_below(&constant(1_u64.into()), &constant(0_u64.into())),
                Err(KernelError::UniverseViolation(_))
            ),
            "U_1 : U_0 fails with a refutation"
        );
    }

    #[test]
    fn universe_rule_is_irreflexive()
    {
        let context = empty_context(1_u32.into());
        let composite = level_var(0_u32.into()).max(&constant(3_u64.into()));
        assert!(
            matches!(
                context.check_universe_below(&composite, &composite),
                Err(KernelError::UniverseViolation(_))
            ),
            "l < l must be refused (irreflexivity is the consistency invariant)"
        );
    }

    #[test]
    fn landmark_hypothesis_decides_the_universe_rule()
    {
        // Under x+1 ≤ y the free oracle cannot see x < y, but entailment can.
        let constraints = vec![
            LandmarkConstraint::leq(succ(&level_var(0_u32.into())), level_var(1_u32.into()))
                .unwrap(),
        ];
        let context = LevelContext::admit(LevelParamCount::from(2_u32), constraints).unwrap();
        assert_eq!(
            Ok(()),
            context.check_universe_below(&level_var(0_u32.into()), &level_var(1_u32.into()),),
            "x < y follows from x+1 ≤ y under the landmark poset"
        );
    }

    /// The successor of a level.
    fn succ(level: &Level) -> Level
    {
        level.succ().unwrap()
    }

    #[test]
    fn out_of_scope_constraint_variable_is_rejected()
    {
        let constraints = vec![
            LandmarkConstraint::leq(level_var(0_u32.into()), level_var(5_u32.into())).unwrap(),
        ];
        assert_eq!(
            LevelContext::admit(LevelParamCount::from(2_u32), constraints).unwrap_err(),
            KernelError::LevelVariableOutOfScope {
                variable: var(5_u32.into()),
            },
            "a constraint variable at or above the parameter count is out of scope"
        );
    }

    #[test]
    fn looping_constraints_are_rejected()
    {
        // x = x+1 has no model in ℕ; admission must loop.
        let constraints = vec![
            LandmarkConstraint::equal(level_var(0_u32.into()), succ(&level_var(0_u32.into())))
                .unwrap(),
        ];
        assert!(
            matches!(
                LevelContext::admit(LevelParamCount::from(1_u32), constraints),
                Err(KernelError::InconsistentLevelConstraints(_))
            ),
            "an unsatisfiable level constraint set is rejected"
        );
    }

    #[test]
    fn level_scope_boundary_is_exact()
    {
        let context = empty_context(2_u32.into());
        assert_eq!(
            Ok(()),
            context.check_level_scope(&level_var(1_u32.into())),
            "variable 1 is in scope under 2 parameters"
        );
        assert_eq!(
            context.check_level_scope(&level_var(2_u32.into())),
            Err(KernelError::LevelVariableOutOfScope {
                variable: var(2_u32.into()),
            }),
            "variable 2 is out of scope under 2 parameters"
        );
    }
}
