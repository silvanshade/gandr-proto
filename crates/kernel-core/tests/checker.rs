//! The S1 checker's kernel-native golden corpus and a totality property.
//!
//! These fixtures are **kernel-native** (C5): they are authored directly
//! against the S1 term language, not lowered from the untrusted core-checker
//! (whose bridge is B2.3). Each positive golden is a declaration the choke
//! point must admit; each negative golden a declaration it must reject with a
//! named error. The property confirms the choke point is **total** — an
//! arbitrary (well- or ill-formed) body is always accepted or rejected, never a
//! panic or a divergence.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps unit and property tests readable (docs/workflow/rust.md)"
    )
)]

extern crate alloc;

/// The kernel-native golden corpus and totality property.
#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;
    use alloc::string::String;
    use alloc::vec;

    use gandr_kernel_core::BaseType;
    use gandr_kernel_core::CompType;
    use gandr_kernel_core::Computation;
    use gandr_kernel_core::DeBruijnIndex;
    use gandr_kernel_core::Declaration;
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
    use gandr_kernel_core::Value;
    use gandr_kernel_core::ValueType;
    use gandr_kernel_strata::LandmarkConstraint;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;
    use proptest::prelude::*;

    /// `U (Unit -> F Unit)` — the type of a thunked unit-to-unit function.
    fn unit_endo_thunk_type() -> ValueType
    {
        ValueType::Thunk(Box::new(CompType::Arrow {
            domain: Box::new(ValueType::Unit),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        }))
    }

    /// Admit a declaration that must check, returning the fresh environment.
    fn admit(declaration: Declaration) -> Environment
    {
        let mut environment = Environment::new();
        environment
            .add_decl(declaration)
            .expect("the golden declaration must check");
        environment
    }

    #[test]
    fn golden_unit_definition()
    {
        let _environment = admit(Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Unit,
            Value::Unit,
        ));
    }

    #[test]
    fn golden_integer_literal_definition()
    {
        let literal = Value::Literal(Literal::Integer(IntegerLiteral::new(
            Sign::NonNegative,
            Magnitude::from_decimal_text(String::from("42")).unwrap(),
        )));
        let _environment = admit(Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Base(BaseType::Integer),
            literal,
        ));
    }

    #[test]
    fn golden_pair_definition()
    {
        let pair = Value::Pair(
            Box::new(Value::Unit),
            Box::new(Value::Literal(Literal::Text(StringLiteral::new(
                String::from("x"),
            )))),
        );
        let _environment = admit(Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Product(
                Box::new(ValueType::Unit),
                Box::new(ValueType::Base(BaseType::String)),
            ),
            pair,
        ));
    }

    #[test]
    fn golden_sum_injection_definition()
    {
        // An injection Def: inl unit : Unit + Integer (checks against the sum).
        let _environment = admit(Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Sum(
                Box::new(ValueType::Unit),
                Box::new(ValueType::Base(BaseType::Integer)),
            ),
            Value::Injection(Side::Left, Box::new(Value::Unit)),
        ));
    }

    #[test]
    fn golden_identity_thunk_definition()
    {
        let body = Value::Thunk(Box::new(Computation::Lambda(Box::new(
            Computation::Return(Box::new(Value::Variable(DeBruijnIndex::from(0_u32)))),
        ))));
        let _environment = admit(Declaration::def(
            LevelSignature::monomorphic(),
            unit_endo_thunk_type(),
            body,
        ));
    }

    #[test]
    fn golden_case_computation_definition()
    {
        // U (Sum Unit Unit -> F Unit) with body thunk (lambda. case v0 {
        // inl => return unit | inr => return unit }) — case in check mode.
        let inner = Computation::Case {
            scrutinee: Box::new(Value::Variable(DeBruijnIndex::from(0_u32))),
            on_left: Box::new(Computation::Return(Box::new(Value::Unit))),
            on_right: Box::new(Computation::Return(Box::new(Value::Unit))),
        };
        let body = Value::Thunk(Box::new(Computation::Lambda(Box::new(inner))));
        let declared = ValueType::Thunk(Box::new(CompType::Arrow {
            domain: Box::new(ValueType::Sum(
                Box::new(ValueType::Unit),
                Box::new(ValueType::Unit),
            )),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Unit))),
        }));
        let _environment = admit(Declaration::def(
            LevelSignature::monomorphic(),
            declared,
            body,
        ));
    }

    #[test]
    fn golden_bind_computation_definition()
    {
        // U (F Unit) with body thunk (bind x <- return unit; return x) — the
        // bound computation synthesizes F Unit, binding Unit at v0.
        let inner = Computation::Bind(
            Box::new(Computation::Return(Box::new(Value::Unit))),
            Box::new(Computation::Return(Box::new(Value::Variable(
                DeBruijnIndex::from(0_u32),
            )))),
        );
        let body = Value::Thunk(Box::new(inner));
        let declared = ValueType::Thunk(Box::new(CompType::Returner(Box::new(ValueType::Unit))));
        let _environment = admit(Declaration::def(
            LevelSignature::monomorphic(),
            declared,
            body,
        ));
    }

    #[test]
    fn golden_universe_axiom()
    {
        // An axiom whose declared type is the universe U_0 (well-formed at 1).
        let _environment = admit(Declaration::axiom(
            LevelSignature::monomorphic(),
            ValueType::Universe(Level::constant(LevelConstant::from(0_u64))),
        ));
    }

    #[test]
    fn golden_lift_axiom()
    {
        // An axiom of the lifted type Lift Unit -> level 2.
        let _environment = admit(Declaration::axiom(
            LevelSignature::monomorphic(),
            ValueType::Lift {
                inner: Box::new(ValueType::Unit),
                target: Level::constant(LevelConstant::from(2_u64)),
            },
        ));
    }

    #[test]
    fn golden_prenex_level_parameter_axiom()
    {
        // An axiom generalized over one level parameter, declared type U_(x0).
        let variable = Level::var(LevelVar::new(LevelVarIndex::from(0_u32)));
        let _environment = admit(Declaration::axiom(
            LevelSignature::new(LevelParamCount::from(1_u32), vec![]),
            ValueType::Universe(variable),
        ));
    }

    #[test]
    fn golden_landmark_constrained_axiom()
    {
        // Two level parameters with the landmark x0 <= x1; the poset admits, so
        // the declaration checks.
        let lower = Level::var(LevelVar::new(LevelVarIndex::from(0_u32)));
        let upper = Level::var(LevelVar::new(LevelVarIndex::from(1_u32)));
        let constraint = LandmarkConstraint::leq(lower.clone(), upper).unwrap();
        let _environment = admit(Declaration::axiom(
            LevelSignature::new(LevelParamCount::from(2_u32), vec![constraint]),
            ValueType::Universe(lower),
        ));
    }

    #[test]
    fn negative_ill_typed_body_is_rejected()
    {
        let mut environment = Environment::new();
        let outcome = environment.add_decl(Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Base(BaseType::Integer),
            Value::Unit,
        ));
        assert!(
            matches!(outcome, Err(KernelError::ValueTypeMismatch(_))),
            "unit does not have integer type"
        );
    }

    #[test]
    fn negative_out_of_scope_level_variable_is_rejected()
    {
        let mut environment = Environment::new();
        let variable = LevelVar::new(LevelVarIndex::from(0_u32));
        let outcome = environment.add_decl(Declaration::axiom(
            LevelSignature::monomorphic(),
            ValueType::Universe(Level::var(variable)),
        ));
        assert_eq!(
            outcome,
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
        let outcome = environment.add_decl(Declaration::axiom(
            LevelSignature::new(LevelParamCount::from(1_u32), vec![constraint]),
            ValueType::Unit,
        ));
        assert!(
            matches!(outcome, Err(KernelError::InconsistentLevelConstraints(_))),
            "x = x+1 has no model, so the level context is rejected"
        );
    }

    #[test]
    fn the_bypass_records_an_unchecked_admission_in_the_audit()
    {
        // A declaration the checker would reject enters through the bypass and
        // taints every dependent's audit.
        let mut environment = Environment::new();
        let bypassed = environment.add_decl_unchecked(Declaration::def(
            LevelSignature::monomorphic(),
            ValueType::Base(BaseType::Integer),
            Value::Unit, // ill-typed, but the bypass does not check
        ));
        let dependent = environment
            .add_decl(Declaration::def(
                LevelSignature::monomorphic(),
                ValueType::Base(BaseType::Integer),
                Value::Constant(bypassed.position()),
            ))
            .unwrap();
        let report = environment.audit(dependent);
        assert_eq!(
            report.unchecked_admissions(),
            &[bypassed.position()],
            "the dependent rests on the unchecked admission"
        );
        assert!(report.axioms().is_empty(), "no axiom is involved");
    }

    /// A bounded, possibly ill-typed value body.
    fn arbitrary_value() -> impl Strategy<Value = Value>
    {
        let leaf = prop_oneof![
            (0_u32 .. 4_u32).prop_map(|index| Value::Variable(DeBruijnIndex::from(index))),
            Just(Value::Unit),
        ];
        leaf.prop_recursive(4, 40, 3, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(one, two)| Value::Pair(Box::new(one), Box::new(two))),
                (
                    prop_oneof![Just(Side::Left), Just(Side::Right)],
                    inner.clone()
                )
                    .prop_map(|(side, body)| Value::Injection(side, Box::new(body))),
                inner.prop_map(|body| Value::Thunk(Box::new(Computation::Return(Box::new(body))))),
            ]
        })
    }

    /// A bounded declared value type.
    fn arbitrary_type() -> impl Strategy<Value = ValueType>
    {
        let leaf = prop_oneof![
            Just(ValueType::Unit),
            Just(ValueType::Base(BaseType::Integer)),
        ];
        leaf.prop_recursive(4, 40, 3, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(one, two)| ValueType::Product(Box::new(one), Box::new(two))),
                (inner.clone(), inner)
                    .prop_map(|(one, two)| ValueType::Sum(Box::new(one), Box::new(two))),
            ]
        })
    }

    proptest! {
        #[test]
        fn prop_the_choke_point_is_total(
            declared in arbitrary_type(),
            body in arbitrary_value(),
        )
        {
            // On any input the choke point returns a verdict — it never panics
            // or diverges. Re-checking the same declaration in a fresh
            // environment yields the identical verdict (checking is
            // deterministic and stateless beyond the environment).
            let first = Environment::new()
                .add_decl(Declaration::def(
                    LevelSignature::monomorphic(),
                    declared.clone(),
                    body.clone(),
                ))
                .is_ok();
            let second = Environment::new()
                .add_decl(Declaration::def(LevelSignature::monomorphic(), declared, body))
                .is_ok();
            prop_assert_eq!(first, second, "checking is deterministic");
        }
    }
}
