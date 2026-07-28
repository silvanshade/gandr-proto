//! The S1 checker's kernel-native golden corpus and a totality property.
//!
//! These fixtures are **kernel-native** (C5): they are authored directly
//! against the S1 term language (as arena-independent specs materialized into
//! the environment arena), not lowered from the untrusted core-checker. Each
//! positive golden is a declaration the choke point must admit; each negative
//! golden a declaration it must reject with a named error. The property
//! confirms the choke point is **total** — an arbitrary body is always accepted
//! or rejected, never a panic or a divergence.

/// The kernel-native golden corpus and totality property.
#[cfg(test)]
mod tests
{
    use gandr_kernel_core::BaseType;
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::IntegerLiteral;
    use gandr_kernel_core::KernelError;
    use gandr_kernel_core::LevelParamCount;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::Literal;
    use gandr_kernel_core::Magnitude;
    use gandr_kernel_core::Side;
    use gandr_kernel_core::Sign;
    use gandr_kernel_core::StringLiteral;
    use gandr_kernel_strata::LandmarkConstraint;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;
    use proptest::prelude::*;

    use crate::common;
    use crate::common::CompTypeSpec;
    use crate::common::ComputationSpec;
    use crate::common::ValueSpec;
    use crate::common::ValueTypeSpec;

    /// `U (Unit -> F Unit)` — the type of a thunked unit-to-unit function.
    fn unit_endo_thunk_type() -> ValueTypeSpec
    {
        ValueTypeSpec::Thunk(Box::new(CompTypeSpec::Arrow(
            Box::new(ValueTypeSpec::Unit),
            Box::new(CompTypeSpec::Returner(Box::new(ValueTypeSpec::Unit))),
        )))
    }

    /// Admit a `Def` that must check.
    fn admit_def(
        declared: &ValueTypeSpec,
        body: &ValueSpec,
    )
    {
        let mut environment = Environment::new();
        let declaration = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            declared,
            body,
        );
        environment
            .add_decl(declaration)
            .expect("the golden declaration must check");
    }

    /// Admit an `Axiom` that must check.
    fn admit_axiom(
        levels: LevelSignature,
        declared: &ValueTypeSpec,
    )
    {
        let mut environment = Environment::new();
        let declaration = common::stage_axiom(&mut environment, levels, declared);
        environment
            .add_decl(declaration)
            .expect("the golden axiom must check");
    }

    #[test]
    fn golden_unit_definition()
    {
        admit_def(&ValueTypeSpec::Unit, &ValueSpec::Unit);
    }

    #[test]
    fn golden_integer_literal_definition()
    {
        let literal = ValueSpec::Literal(Literal::Integer(IntegerLiteral::new(
            Sign::NonNegative,
            Magnitude::from_decimal_text(String::from("42")).unwrap(),
        )));
        admit_def(&ValueTypeSpec::Base(BaseType::Integer), &literal);
    }

    #[test]
    fn golden_pair_definition()
    {
        let pair = ValueSpec::Pair(
            Box::new(ValueSpec::Unit),
            Box::new(ValueSpec::Literal(Literal::Text(StringLiteral::new(
                String::from("x"),
            )))),
        );
        admit_def(
            &ValueTypeSpec::Product(
                Box::new(ValueTypeSpec::Unit),
                Box::new(ValueTypeSpec::Base(BaseType::String)),
            ),
            &pair,
        );
    }

    #[test]
    fn golden_sum_injection_definition()
    {
        admit_def(
            &ValueTypeSpec::Sum(
                Box::new(ValueTypeSpec::Unit),
                Box::new(ValueTypeSpec::Base(BaseType::Integer)),
            ),
            &ValueSpec::Injection(Side::Left, Box::new(ValueSpec::Unit)),
        );
    }

    #[test]
    fn golden_identity_thunk_definition()
    {
        let body = ValueSpec::Thunk(Box::new(ComputationSpec::Lambda(Box::new(
            ComputationSpec::Return(Box::new(ValueSpec::Variable(0))),
        ))));
        admit_def(&unit_endo_thunk_type(), &body);
    }

    #[test]
    fn golden_case_computation_definition()
    {
        let inner = ComputationSpec::Case(
            Box::new(ValueSpec::Variable(0)),
            Box::new(ComputationSpec::Return(Box::new(ValueSpec::Unit))),
            Box::new(ComputationSpec::Return(Box::new(ValueSpec::Unit))),
        );
        let body = ValueSpec::Thunk(Box::new(ComputationSpec::Lambda(Box::new(inner))));
        let declared = ValueTypeSpec::Thunk(Box::new(CompTypeSpec::Arrow(
            Box::new(ValueTypeSpec::Sum(
                Box::new(ValueTypeSpec::Unit),
                Box::new(ValueTypeSpec::Unit),
            )),
            Box::new(CompTypeSpec::Returner(Box::new(ValueTypeSpec::Unit))),
        )));
        admit_def(&declared, &body);
    }

    #[test]
    fn golden_bind_computation_definition()
    {
        let inner = ComputationSpec::Bind(
            Box::new(ComputationSpec::Return(Box::new(ValueSpec::Unit))),
            Box::new(ComputationSpec::Return(Box::new(ValueSpec::Variable(0)))),
        );
        let body = ValueSpec::Thunk(Box::new(inner));
        let declared = ValueTypeSpec::Thunk(Box::new(CompTypeSpec::Returner(Box::new(
            ValueTypeSpec::Unit,
        ))));
        admit_def(&declared, &body);
    }

    #[test]
    fn golden_universe_axiom()
    {
        admit_axiom(
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Universe(Level::constant(LevelConstant::from(0_u64))),
        );
    }

    #[test]
    fn golden_lift_axiom()
    {
        admit_axiom(
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Lift(
                Box::new(ValueTypeSpec::Unit),
                Level::constant(LevelConstant::from(2_u64)),
            ),
        );
    }

    #[test]
    fn golden_prenex_level_parameter_axiom()
    {
        let variable = Level::var(LevelVar::new(LevelVarIndex::from(0_u32)));
        admit_axiom(
            LevelSignature::new(LevelParamCount::from(1_u32), Vec::new()),
            &ValueTypeSpec::Universe(variable),
        );
    }

    #[test]
    fn golden_landmark_constrained_axiom()
    {
        let lower = Level::var(LevelVar::new(LevelVarIndex::from(0_u32)));
        let upper = Level::var(LevelVar::new(LevelVarIndex::from(1_u32)));
        let constraint = LandmarkConstraint::leq(lower.clone(), upper).unwrap();
        admit_axiom(
            LevelSignature::new(LevelParamCount::from(2_u32), vec![constraint]),
            &ValueTypeSpec::Universe(lower),
        );
    }

    #[test]
    fn negative_ill_typed_body_is_rejected()
    {
        let mut environment = Environment::new();
        let declaration = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Base(BaseType::Integer),
            &ValueSpec::Unit,
        );
        assert!(
            matches!(
                environment.add_decl(declaration),
                Err(KernelError::ValueTypeMismatch(_))
            ),
            "unit does not have integer type"
        );
    }

    #[test]
    fn negative_out_of_scope_level_variable_is_rejected()
    {
        let mut environment = Environment::new();
        let variable = LevelVar::new(LevelVarIndex::from(0_u32));
        let declaration = common::stage_axiom(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Universe(Level::var(variable)),
        );
        assert_eq!(
            environment.add_decl(declaration),
            Err(KernelError::LevelVariableOutOfScope { variable }),
            "a level variable with no prenex parameter is out of scope"
        );
    }

    #[test]
    fn negative_inconsistent_landmark_poset_is_rejected()
    {
        let mut environment = Environment::new();
        let variable = Level::var(LevelVar::new(LevelVarIndex::from(0_u32)));
        let constraint =
            LandmarkConstraint::equal(variable.clone(), variable.succ().unwrap()).unwrap();
        let declaration = common::stage_axiom(
            &mut environment,
            LevelSignature::new(LevelParamCount::from(1_u32), vec![constraint]),
            &ValueTypeSpec::Unit,
        );
        assert!(
            matches!(
                environment.add_decl(declaration),
                Err(KernelError::InconsistentLevelConstraints(_))
            ),
            "x = x+1 has no model, so the level context is rejected"
        );
    }

    #[test]
    fn the_bypass_records_an_unchecked_admission_in_the_audit()
    {
        // A declaration the checker would reject enters through the bypass and
        // taints every dependent's audit.
        let mut environment = Environment::new();
        let bypassed_decl = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Base(BaseType::Integer),
            &ValueSpec::Unit, // ill-typed, but the bypass does not check
        );
        let bypassed = environment.add_decl_unchecked(bypassed_decl);
        let dependent_decl = common::stage_def(
            &mut environment,
            LevelSignature::monomorphic(),
            &ValueTypeSpec::Base(BaseType::Integer),
            &ValueSpec::Constant(usize::from(bypassed.position())),
        );
        let dependent = environment.add_decl(dependent_decl).unwrap();
        let report = environment.audit(dependent);
        assert_eq!(
            report.unchecked_admissions(),
            &[bypassed.position()],
            "the dependent rests on the unchecked admission"
        );
        assert!(report.axioms().is_empty(), "no axiom is involved");
    }

    proptest! {
        #[test]
        fn prop_the_choke_point_is_total(
            declared in common::arb_value_type_spec(),
            body in common::arb_value_spec(),
        )
        {
            // On any input the choke point returns a verdict — it never panics
            // or diverges. Re-checking the same declaration in a fresh
            // environment yields the identical verdict (checking is
            // deterministic and stateless beyond the environment).
            let mut first_env = Environment::new();
            let first_decl =
                common::stage_def(&mut first_env, LevelSignature::monomorphic(), &declared, &body);
            let first = first_env.add_decl(first_decl).is_ok();
            let mut second_env = Environment::new();
            let second_decl =
                common::stage_def(&mut second_env, LevelSignature::monomorphic(), &declared, &body);
            let second = second_env.add_decl(second_decl).is_ok();
            prop_assert_eq!(first, second, "checking is deterministic");
        }
    }
}
