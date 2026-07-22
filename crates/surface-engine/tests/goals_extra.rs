//! Goals-report coverage (`src/goals.rs`): the sort-directed `initial_state`
//! entry and the hole-observation passes, driven through `goals_report`.

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
        clippy::indexing_slicing,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the coverage tables readable (docs/workflow/rust.md)"
    )
)]

#[cfg(test)]
mod tests
{
    use gandr_surface_engine::goals::goals_report;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::prelude_ctx;

    #[test]
    fn a_computation_signature_folds_into_its_def_and_yields_one_hole_goal()
    {
        // A `def`-signature ascribes a computation type and folds into the matching
        // `def`, driving the `Comp`-term / `Comp`-ascription `initial_state`
        // entry. NOTE: the hole is forced in INFERENCE position, so its goal
        // records no expected type — this test asserts the ascription, the goal
        // count, and the owning item, nothing more.
        let source = "def f : F Integer; def f = force ?;";
        let lowered = lower_source_total(source.into()).expect("total lowering never errs");
        // The matched item carries the computation ascription.
        assert!(
            lowered
                .items
                .iter()
                .any(|item| item.name.as_deref() == Some("f")
                    && matches!(
                        item.ascription,
                        Some(gandr_core_checker::types::Ty::Comp(_))
                    )),
            "the signature ascribes a computation type: {:?}",
            lowered
                .items
                .iter()
                .map(|item| (&item.name, &item.ascription))
                .collect::<Vec<_>>()
        );
        let goals = goals_report(&lowered, &prelude_ctx());
        assert_eq!(1, goals.len(), "one hole goal for the `force ?` body");
        // The signature attaches to the matching `def`, so both fold into one
        // item; the hole is forced in inference position (no expected type), but
        // the item was driven through the `Check`-mode computation entry.
        assert_eq!(0, goals[0].item, "the goal belongs to the sole `f` item");
    }

    #[test]
    fn a_computation_signature_on_a_hole_free_body_types_through_the_check_entry()
    {
        // A hole-free computation body under a computation signature exercises the
        // `Comp`-term / `Some(Comp)`-ascription `initial_state` entry with no goal
        // to record.
        let source = "def g : F Integer; def g = ret 1;";
        let lowered = lower_source_total(source.into()).expect("total lowering never errs");
        let goals = goals_report(&lowered, &prelude_ctx());
        assert!(
            goals.is_empty(),
            "a hole-free body reports no goals: {goals:?}"
        );
    }
}
