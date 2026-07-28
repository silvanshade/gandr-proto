//! Property tests for the S1 conversion faces over the public API: bounded
//! random value/computation types and terms are generated (as arena-independent
//! specs), materialized into a [`TermArena`], and the conversion judgment is
//! checked against its defining laws — reflexivity, symmetry, and separation.
//! Conversion is the id-equality-then-structural walk computed iteratively, so
//! these laws also witness that the iterative worklist agrees with reflexive
//! equality on every generated input, at any depth, without recursing.

/// Conversion laws over generated types and terms.
#[cfg(test)]
mod tests
{
    use gandr_kernel_core::Convertibility;
    use gandr_kernel_core::TermArena;
    use gandr_kernel_core::convertible_comp_types;
    use gandr_kernel_core::convertible_computations;
    use gandr_kernel_core::convertible_value_types;
    use gandr_kernel_core::convertible_values;
    use proptest::prelude::*;

    use crate::common;

    proptest! {
        #[test]
        fn prop_value_type_conversion_is_reflexive(spec in common::arb_value_type_spec())
        {
            let mut arena = TermArena::new();
            let ty = common::materialize_value_type(&mut arena, &spec);
            prop_assert_eq!(
                convertible_value_types(&arena, ty, ty),
                Convertibility::Convertible,
                "every value type converts with itself"
            );
        }

        #[test]
        fn prop_value_type_conversion_is_symmetric(
            one in common::arb_value_type_spec(),
            two in common::arb_value_type_spec(),
        )
        {
            let mut arena = TermArena::new();
            let one = common::materialize_value_type(&mut arena, &one);
            let two = common::materialize_value_type(&mut arena, &two);
            prop_assert_eq!(
                convertible_value_types(&arena, one, two),
                convertible_value_types(&arena, two, one),
                "conversion does not depend on argument order"
            );
        }

        #[test]
        fn prop_computation_type_conversion_is_reflexive(spec in common::arb_comp_type_spec())
        {
            let mut arena = TermArena::new();
            let ty = common::materialize_comp_type(&mut arena, &spec);
            prop_assert_eq!(
                convertible_comp_types(&arena, ty, ty),
                Convertibility::Convertible,
                "every computation type converts with itself"
            );
        }

        #[test]
        fn prop_value_conversion_is_reflexive(spec in common::arb_value_spec())
        {
            let mut arena = TermArena::new();
            let term = common::materialize_value(&mut arena, &spec);
            prop_assert_eq!(
                convertible_values(&arena, term, term),
                Convertibility::Convertible,
                "every value converts with itself"
            );
        }

        #[test]
        fn prop_computation_conversion_is_reflexive(spec in common::arb_computation_spec())
        {
            let mut arena = TermArena::new();
            let term = common::materialize_computation(&mut arena, &spec);
            prop_assert_eq!(
                convertible_computations(&arena, term, term),
                Convertibility::Convertible,
                "every computation converts with itself"
            );
        }

        #[test]
        fn prop_a_wrapped_type_does_not_convert_with_its_payload(
            spec in common::arb_value_type_spec(),
        )
        {
            // Wrapping a type in a thunk changes it, so the two must separate —
            // the iterative walk detects the structural difference.
            let mut arena = TermArena::new();
            let ty = common::materialize_value_type(&mut arena, &spec);
            let returner = arena.comp_type_returner(ty);
            let wrapped = arena.value_type_thunk(returner);
            prop_assert_eq!(
                convertible_value_types(&arena, ty, wrapped),
                Convertibility::Distinct,
                "a type and its thunk-wrapping are distinct"
            );
        }
    }
}
