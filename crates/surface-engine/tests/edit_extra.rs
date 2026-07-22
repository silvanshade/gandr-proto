//! Diff + apply coverage (`src/edit.rs`): the list-, record-, record-
//! projection-, and computation-hole diff arms, plus the structural `apply`
//! rebuild that reproduces the new program. When the shape is stable the diff
//! descends to the changed leaf and `apply` rebuilds element/field-wise; a
//! shape change (list length, record label set, projection label) replaces
//! wholesale; a hole appearing or vanishing is a `FillHole` / `EraseToHole`.

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
        reason = "the standard test-allow set keeps the coverage tables readable (docs/workflow/rust.md)"
    )
)]

#[cfg(test)]
mod tests
{
    use gandr_surface_engine::edit::Action;
    use gandr_surface_engine::edit::apply;
    use gandr_surface_engine::edit::diff;
    use gandr_surface_engine::lower::Lowered;
    use gandr_surface_engine::lower::lower_source_total;

    use crate::TestText;
    #[test]
    fn a_vanishing_or_appearing_computation_hole_is_erase_or_fill()
    {
        // `co { fst = ret 1 }` holes its missing `snd`; adding the `snd` fills
        // the hole, dropping it erases to a hole.
        let with_snd = "def d = co { fst = ret 1, snd = ret 2 };";
        let missing_snd = "def d = co { fst = ret 1 };";

        let fill = actions(missing_snd, with_snd);
        assert!(
            matches!(fill.as_slice(), [Action::FillHole { path, .. }] if path.as_slice() == [0, 1]),
            "adding the missing `snd` fills the hole: {fill:?}"
        );

        let erase = actions(with_snd, missing_snd);
        assert!(
            matches!(erase.as_slice(), [Action::EraseToHole { path, .. }] if path.as_slice() == [0, 1]),
            "dropping the `snd` erases it to a hole: {erase:?}"
        );

        // Two identical hole-bearing programs diff to nothing (the hole/hole
        // no-op arm; the self-diff-empty invariant).
        assert!(
            actions(missing_snd, missing_snd).is_empty(),
            "a self-diff over a hole-bearing term is empty"
        );
    }

    /// The actions of the diff between two totally-lowered sources.
    fn actions<'old, 'new>(
        old: impl Into<TestText<'old>>,
        new: impl Into<TestText<'new>>,
    ) -> Vec<Action>
    {
        let old = old.into().0;
        let new = new.into().0;
        diff(&lowered(old), &lowered(new)).actions
    }
    #[test]
    fn same_shape_containers_descend_and_apply_rebuilds_them()
    {
        // Same-length list, same-label record, and a list-case body all keep
        // their shape, so the diff is a single leaf `SetInt` and `apply`
        // rebuilds the container element/field-wise to reproduce the new term.
        let list = diff_roundtrips("def d = [1, 2];", "def d = [1, 9];");
        assert!(
            matches!(list.as_slice(), [Action::SetInt { path, from: 2, to: 9 }] if path.as_slice() == [0, 1]),
            "a same-length list descends to the changed element: {list:?}"
        );

        let record = diff_roundtrips("def d = #{ a = 1 };", "def d = #{ a = 9 };");
        assert!(
            matches!(record.as_slice(), [Action::SetInt { path, from: 1, to: 9 }] if path.as_slice() == [0, 0]),
            "a same-label record descends to the changed field: {record:?}"
        );

        let list_case = diff_roundtrips(
            "def d = case xs { Nil => ret 0, Cons(h, t) => ret h };",
            "def d = case xs { Nil => ret 9, Cons(h, t) => ret h };",
        );
        assert!(
            matches!(list_case.as_slice(), [Action::SetInt {
                from: 0,
                to: 9,
                ..
            }]),
            "a list-case nil body descends to its changed leaf: {list_case:?}"
        );
    }
    #[test]
    fn shape_changes_replace_wholesale_and_apply_installs_the_subtree()
    {
        // A list length change, a record label rename / resize, and a record-
        // projection label change all fail to align, so each is a wholesale
        // `Replace`; `apply` installs the replacement subtree.
        for (old, new) in [
            ("def d = [1, 2];", "def d = [1, 2, 3];"),
            ("def d = #{ a = 1 };", "def d = #{ b = 1 };"),
            ("def d = #{ a = 1, b = 2 };", "def d = #{ a = 1 };"),
            ("def d = r.foo;", "def d = r.bar;"),
        ] {
            let acts = diff_roundtrips(old, new);
            assert!(
                matches!(acts.as_slice(), [Action::Replace { path, .. }] if path.as_slice() == [0]),
                "`{old}` -> `{new}` is a wholesale replace: {acts:?}"
            );
        }
    }

    /// Asserts `apply(old, diff(old, new))` reproduces `new` exactly, and
    /// returns the diff's actions for further inspection.
    fn diff_roundtrips<'old, 'new>(
        old: impl Into<TestText<'old>>,
        new: impl Into<TestText<'new>>,
    ) -> Vec<Action>
    {
        let old = old.into().0;
        let new = new.into().0;
        let old_lowered = lowered(old);
        let new_lowered = lowered(new);
        let script = diff(&old_lowered, &new_lowered);
        assert_eq!(
            apply(&old_lowered.items, &script),
            new_lowered.items,
            "apply(old, diff(old, new)) must reproduce new for `{old}` -> `{new}`"
        );
        script.actions
    }

    fn lowered<'text>(source: impl Into<TestText<'text>>) -> Lowered
    {
        let source = source.into().0;
        lower_source_total(source.into()).expect("total lowering never errs")
    }
}
