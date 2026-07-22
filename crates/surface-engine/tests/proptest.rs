//! Grammar-directed property tests for the pipeline's totality and
//! source-level checker ≡ machine agreement.
//!
//! These generate grammar-shaped source — well-formed bindings and signatures,
//! variable references, shell blocks, and deliberately broken fragments — from
//! the shared `gandr_core_checker::strategies` generators (the
//! `proptest-strategies` feature) and assert two behavioural properties over
//! it:
//!
//! 1. `lower::lower_source_total` is **total**: it yields a `Lowered` for every
//!    input and never panics (the "no parse wall" guarantee).
//! 2. The recursive checker and the typing machine **agree** on every lowered
//!    item and neither panics — the source-level extension of the in-crate
//!    conformance property (the no-panic / totality behavioural leg).

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]
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

/// Grammar-directed totality and agreement properties.
#[cfg(test)]
mod tests
{
    use gandr_core_checker::checker;
    use gandr_core_checker::control::Dir;
    use gandr_core_checker::error::TypeError;
    use gandr_core_checker::machine;
    use gandr_core_checker::strategies::binder_name;
    use gandr_core_checker::syntax::Term;
    use gandr_core_checker::types::Ty;
    use gandr_surface_engine::lower::LoweredItem;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::prelude_ctx;

    use crate::proptest_crate::collection::vec;
    use crate::proptest_crate::prelude::*;

    /// A small program: zero to five grammar-directed statements.
    fn grammar_directed_source() -> impl Strategy<Value = String>
    {
        vec(statement(), 0_usize .. 6_usize).prop_map(|statements| statements.join("\n"))
    }

    /// One grammar-directed statement: well-formed bindings and signatures,
    /// a variable reference, a shell block, and a deliberately broken fragment
    /// (which total lowering must recover into a hole).
    fn statement() -> impl Strategy<Value = String>
    {
        prop_oneof![
            (binder_name(), 0_u64 .. 100_u64)
                .prop_map(|(name, literal)| format!("def {name} = {literal};")),
            binder_name().prop_map(|name| format!("def {name} : Integer;")),
            (binder_name(), binder_name()).prop_map(|(lhs, rhs)| format!("def {lhs} = {rhs};")),
            Just("#!{ echo hi }".to_owned()),
            binder_name().prop_map(|name| format!("def {name} = ;")),
        ]
    }

    /// Types one lowered item on **both** implementations — against its
    /// ascription when the sorts match, in inference mode otherwise — and
    /// asserts they agree. Termination without a panic is the totality
    /// evidence.
    fn type_item_both(item: &LoweredItem) -> Result<Ty, TypeError>
    {
        match (&item.term, &item.ascription) {
            | (&Term::Value(ref value), &Some(Ty::Value(ref expected))) => {
                let dir = Dir::Check(expected.clone());
                let (rec, _) = checker::run_value(prelude_ctx(), value.clone(), dir.clone());
                let (mach, _) = machine::run_value(prelude_ctx(), value.clone(), dir);
                assert_eq!(rec, mach, "checker and machine must agree on {item:?}");
                mach
            },
            | (&Term::Value(ref value), _) => {
                let (rec, _) = checker::run_value(prelude_ctx(), value.clone(), Dir::Infer);
                let (mach, _) = machine::run_value(prelude_ctx(), value.clone(), Dir::Infer);
                assert_eq!(rec, mach, "checker and machine must agree on {item:?}");
                mach
            },
            | (&Term::Comp(ref comp), &Some(Ty::Comp(ref expected))) => {
                let dir = Dir::Check(expected.clone());
                let (rec, _) = checker::run_comp(prelude_ctx(), comp.clone(), dir.clone());
                let (mach, _) = machine::run_comp(prelude_ctx(), comp.clone(), dir);
                assert_eq!(rec, mach, "checker and machine must agree on {item:?}");
                mach
            },
            | (&Term::Comp(ref comp), _) => {
                let (rec, _) = checker::run_comp(prelude_ctx(), comp.clone(), Dir::Infer);
                let (mach, _) = machine::run_comp(prelude_ctx(), comp.clone(), Dir::Infer);
                assert_eq!(rec, mach, "checker and machine must agree on {item:?}");
                mach
            },
            | _ => panic!("unknown term sort in {item:?}"),
        }
    }

    proptest! {
        /// Total lowering never panics, returns a `Lowered` for every generated
        /// source, and the checker and machine agree on every lowered item.
        #[test]
        fn total_lowering_and_checking_never_panics(source in grammar_directed_source())
        {
            let lowered = lower_source_total((&source).into())
                .expect("total lowering yields a Lowered for any source");
            for item in &lowered.items {
                let _typed = type_item_both(item);
            }
        }
    }
}
