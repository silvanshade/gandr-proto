//! Property tests for the S1 conversion faces over the public API: bounded
//! random value/computation types and terms are generated, and the conversion
//! judgment is checked against its defining laws — reflexivity (a type/term
//! converts with itself), symmetry (order does not matter), and separation (a
//! structural perturbation is detected). Conversion is the derived structural
//! equality computed iteratively, so these laws also witness that the iterative
//! worklist agrees with reflexive equality on every generated input, at any
//! depth, without recursing.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps property tests readable (docs/workflow/rust.md)"
    )
)]

extern crate alloc;

/// Conversion laws over generated types and terms.
#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;
    use alloc::format;

    use gandr_kernel_core::BaseType;
    use gandr_kernel_core::CompType;
    use gandr_kernel_core::Computation;
    use gandr_kernel_core::Convertibility;
    use gandr_kernel_core::DeBruijnIndex;
    use gandr_kernel_core::IntegerLiteral;
    use gandr_kernel_core::Literal;
    use gandr_kernel_core::Magnitude;
    use gandr_kernel_core::Side;
    use gandr_kernel_core::Sign;
    use gandr_kernel_core::Value;
    use gandr_kernel_core::ValueType;
    use gandr_kernel_core::convertible_comp_types;
    use gandr_kernel_core::convertible_computations;
    use gandr_kernel_core::convertible_value_types;
    use gandr_kernel_core::convertible_values;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;
    use proptest::prelude::*;

    /// A small constant or single-variable level.
    fn level_strategy() -> impl Strategy<Value = Level>
    {
        prop_oneof![
            (0_u64 .. 4_u64).prop_map(|value| Level::constant(LevelConstant::from(value))),
            (0_u32 .. 3_u32)
                .prop_map(|index| Level::var(LevelVar::new(LevelVarIndex::from(index)))),
        ]
    }

    /// A base-type atom.
    fn base_type_strategy() -> impl Strategy<Value = BaseType>
    {
        prop_oneof![
            Just(BaseType::Integer),
            Just(BaseType::String),
            Just(BaseType::Numeric),
        ]
    }

    /// A bounded value type (with shallow computation types under thunks).
    fn value_type_strategy() -> impl Strategy<Value = ValueType>
    {
        let leaf = prop_oneof![
            base_type_strategy().prop_map(ValueType::Base),
            Just(ValueType::Unit),
            level_strategy().prop_map(ValueType::Universe),
        ];
        leaf.prop_recursive(4, 48, 3, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(one, two)| ValueType::Product(Box::new(one), Box::new(two))),
                (inner.clone(), inner.clone())
                    .prop_map(|(one, two)| ValueType::Sum(Box::new(one), Box::new(two))),
                (inner.clone(), level_strategy()).prop_map(|(payload, target)| ValueType::Lift {
                    inner: Box::new(payload),
                    target,
                }),
                inner.prop_map(
                    |payload| ValueType::Thunk(Box::new(CompType::Returner(Box::new(payload,))))
                ),
            ]
        })
    }

    /// A bounded computation type built from generated value types.
    fn comp_type_strategy() -> impl Strategy<Value = CompType>
    {
        prop_oneof![
            value_type_strategy().prop_map(|result| CompType::Returner(Box::new(result))),
            (value_type_strategy(), value_type_strategy()).prop_map(|(domain, result)| {
                CompType::Arrow {
                    domain: Box::new(domain),
                    codomain: Box::new(CompType::Returner(Box::new(result))),
                }
            }),
        ]
    }

    /// A small literal.
    fn literal_strategy() -> impl Strategy<Value = Literal>
    {
        prop_oneof![
            (0_u64 .. 1_000_u64).prop_map(|magnitude| {
                Literal::Integer(IntegerLiteral::new(
                    Sign::NonNegative,
                    Magnitude::from_decimal_text(format!("{magnitude}")).unwrap(),
                ))
            }),
            "[a-z]{0,4}"
                .prop_map(|content| Literal::Text(gandr_kernel_core::StringLiteral::new(content))),
        ]
    }

    /// A bounded value term.
    fn value_strategy() -> impl Strategy<Value = Value>
    {
        let leaf = prop_oneof![
            (0_u32 .. 6_u32).prop_map(|index| Value::Variable(DeBruijnIndex::from(index))),
            (0_usize .. 6_usize)
                .prop_map(|index| Value::Constant(gandr_kernel_core::ConstantIndex::from(index))),
            Just(Value::Unit),
            literal_strategy().prop_map(Value::Literal),
        ];
        leaf.prop_recursive(4, 48, 3, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone())
                    .prop_map(|(one, two)| Value::Pair(Box::new(one), Box::new(two))),
                (
                    prop_oneof![Just(Side::Left), Just(Side::Right)],
                    inner.clone()
                )
                    .prop_map(|(side, body)| Value::Injection(side, Box::new(body))),
                (level_strategy(), inner.clone()).prop_map(|(target, body)| Value::Lift {
                    target,
                    body: Box::new(body),
                }),
                inner.prop_map(|body| Value::Thunk(Box::new(Computation::Return(Box::new(body))))),
            ]
        })
    }

    /// A bounded computation term built from generated values.
    fn computation_strategy() -> impl Strategy<Value = Computation>
    {
        prop_oneof![
            value_strategy().prop_map(|body| Computation::Return(Box::new(body))),
            value_strategy().prop_map(|body| Computation::Force(Box::new(body))),
            value_strategy().prop_map(|body| Computation::Lambda(Box::new(Computation::Return(
                Box::new(body)
            )))),
            (value_strategy(), value_strategy()).prop_map(|(scrutinee, branch)| {
                Computation::Case {
                    scrutinee: Box::new(scrutinee),
                    on_left: Box::new(Computation::Return(Box::new(branch.clone()))),
                    on_right: Box::new(Computation::Return(Box::new(branch))),
                }
            }),
        ]
    }

    proptest! {
        #[test]
        fn prop_value_type_conversion_is_reflexive(ty in value_type_strategy())
        {
            prop_assert_eq!(
                convertible_value_types(&ty, &ty),
                Convertibility::Convertible,
                "every value type converts with itself"
            );
        }

        #[test]
        fn prop_value_type_conversion_is_symmetric(
            one in value_type_strategy(),
            two in value_type_strategy(),
        )
        {
            prop_assert_eq!(
                convertible_value_types(&one, &two),
                convertible_value_types(&two, &one),
                "conversion does not depend on argument order"
            );
        }

        #[test]
        fn prop_computation_type_conversion_is_reflexive(ty in comp_type_strategy())
        {
            prop_assert_eq!(
                convertible_comp_types(&ty, &ty),
                Convertibility::Convertible,
                "every computation type converts with itself"
            );
        }

        #[test]
        fn prop_value_conversion_is_reflexive(term in value_strategy())
        {
            prop_assert_eq!(
                convertible_values(&term, &term),
                Convertibility::Convertible,
                "every value converts with itself"
            );
        }

        #[test]
        fn prop_computation_conversion_is_reflexive(term in computation_strategy())
        {
            prop_assert_eq!(
                convertible_computations(&term, &term),
                Convertibility::Convertible,
                "every computation converts with itself"
            );
        }

        #[test]
        fn prop_a_wrapped_type_does_not_convert_with_its_payload(ty in value_type_strategy())
        {
            // Wrapping a type in a thunk changes it, so the two must separate —
            // the iterative walk detects the structural difference.
            let wrapped = ValueType::Thunk(Box::new(CompType::Returner(Box::new(ty.clone()))));
            prop_assert_eq!(
                convertible_value_types(&ty, &wrapped),
                Convertibility::Distinct,
                "a type and its thunk-wrapping are distinct"
            );
        }
    }
}
