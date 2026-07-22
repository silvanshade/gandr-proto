//! Tests for the edit-action reconstruction layer (`edit`; `wyrd-kekv`,
//! `wyrd-el81`).
//!
//! Six classes:
//!
//! 1. [`tests::localized_edits`] — the precise cases: a literal edit is one
//!    `SetInt`, an inserted definition is one `InsertItem` leaving its
//!    neighbours untouched, a filled hole is one `FillHole`, a grade bump is
//!    one `SetGrade`, a binder rename composes a `Rebind` with the occurrence's
//!    `SetVar`.
//! 2. [`tests::attribute_edits`] — the in-place attribute actions across both
//!    sorts: injection *and* projection `SetSide`, value- and binder-annotation
//!    `SetAnnotation` (both `None` ⇄ `Some` directions), `Rebind` on every
//!    binder slot of `Case` *and* `Split` (`Fst`, `Snd`, both at once), and the
//!    `With` structural descent — each composing with edits inside the node.
//! 3. [`tests::coarse_fallback`] — the honest residual: a node whose
//!    constructor changed becomes one coarse `Replace` (value- *and*
//!    computation-sorted); a deleted item becomes one `DeleteItem`.
//! 4. [`tests::localization`] — [`localize`] / [`edit_locus`] find the smallest
//!    enclosing core term, and the diff's changed paths sit within that locus.
//! 5. [`tests::application`] — `apply` is the diff's adjoint (the soundness
//!    direction), and a self-diff is empty.
//! 6. [`tests::oracle`] — the property: `apply(old, diff(old, new))` equals
//!    `new` up to hole identifiers, for arbitrary lowered program pairs (the
//!    generator spans both sorts, the attribute-bearing constructors, and the
//!    multi-child descent constructors `Case` / `With` / `Prj` / `Split`) and
//!    as an identity on equal programs.

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
        clippy::match_same_arms,
        clippy::pattern_type_mismatch,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set, plus ergonomic-pattern and \
                  identical-arm relaxations that keep the recursive \
                  equality-up-to-holes helpers readable (docs/WORKFLOW.md \
                  §Rust coding conventions)"
    )
)]

/// Tests for the `gandr_surface_engine::edit` public API.
#[cfg(test)]
mod tests
{
    use gandr_core_checker::boundary::GradeBound;
    use gandr_core_checker::grade::Grade;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Side;
    use gandr_core_checker::syntax::Term;
    use gandr_core_checker::syntax::Value;
    use gandr_surface_engine::edit::Action;
    use gandr_surface_engine::edit::AnnSlot;
    use gandr_surface_engine::edit::BinderSlot;
    use gandr_surface_engine::edit::SourceEdit;
    use gandr_surface_engine::edit::Subtree;
    use gandr_surface_engine::edit::apply;
    use gandr_surface_engine::edit::diff;
    use gandr_surface_engine::edit::edit_locus;
    use gandr_surface_engine::edit::localize;
    use gandr_surface_engine::lower::Lowered;
    use gandr_surface_engine::lower::LoweredItem;
    use gandr_surface_engine::lower::lower_source_total;

    use crate::TestDecision;
    use crate::TestInteger;
    use crate::TestPath;
    use crate::TestText;

    /// Lowers an inline source in total mode (every input lowers).
    fn lower<'text>(source: impl Into<TestText<'text>>) -> Lowered
    {
        let source = source.into().0;
        lower_source_total(source.into()).expect("total lowering yields a Lowered")
    }

    /// One corresponding value or computation pair in an iterative equality
    /// comparison.
    #[derive(Clone, Copy)]
    enum EqNode<'term>
    {
        Value(&'term Value, &'term Value),
        Comp(&'term Comp, &'term Comp),
    }

    /// Structural equality of two terms up to hole identifiers (typing ignores
    /// them, so the diff treats holes at corresponding positions as equal; the
    /// oracle must too).
    fn term_eq_mod_holes(
        left: &Term,
        right: &Term,
    ) -> TestDecision
    {
        match (left, right) {
            | (Term::Value(left_value), Term::Value(right_value)) => {
                nodes_eq_mod_holes(EqNode::Value(left_value, right_value))
            },
            | (Term::Comp(left_comp), Term::Comp(right_comp)) => {
                nodes_eq_mod_holes(EqNode::Comp(left_comp, right_comp))
            },
            | _ => false.into(),
        }
    }

    /// Iteratively compares finite core trees, ignoring corresponding hole IDs.
    fn nodes_eq_mod_holes(root: EqNode<'_>) -> TestDecision
    {
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            match node {
                | EqNode::Value(Value::Hole(_), Value::Hole(_))
                | EqNode::Value(Value::Unit, Value::Unit)
                | EqNode::Comp(Comp::Hole(_), Comp::Hole(_)) => {},
                | EqNode::Value(Value::Var(left), Value::Var(right)) if left == right => {},
                | EqNode::Value(Value::Int(left), Value::Int(right)) if left == right => {},
                | EqNode::Value(Value::Str(left), Value::Str(right)) if left == right => {},
                | EqNode::Value(Value::Num(left), Value::Num(right)) if left == right => {},
                | EqNode::Value(Value::Stk(left), Value::Stk(right)) if left == right => {},
                | EqNode::Value(
                    Value::Pair(left_fst, left_snd),
                    Value::Pair(right_fst, right_snd),
                ) => {
                    pending.push(EqNode::Value(left_snd, right_snd));
                    pending.push(EqNode::Value(left_fst, right_fst));
                },
                | EqNode::Value(
                    Value::Inj(left_side, left_payload),
                    Value::Inj(right_side, right_payload),
                ) if left_side == right_side => {
                    pending.push(EqNode::Value(left_payload, right_payload));
                },
                | EqNode::Value(
                    Value::Annot(left_inner, left_ty),
                    Value::Annot(right_inner, right_ty),
                ) if left_ty == right_ty => {
                    pending.push(EqNode::Value(left_inner, right_inner));
                },
                | EqNode::Value(
                    Value::Thunk(left_grade, left_body),
                    Value::Thunk(right_grade, right_body),
                ) if left_grade == right_grade => {
                    pending.push(EqNode::Comp(left_body, right_body));
                },
                | EqNode::Value(Value::List(left), Value::List(right))
                    if left.len() == right.len() =>
                {
                    pending.extend(
                        left.iter()
                            .zip(right)
                            .map(|(left, right)| EqNode::Value(left, right)),
                    );
                },
                | EqNode::Value(Value::Record(left), Value::Record(right))
                    if left.keys().eq(right.keys()) =>
                {
                    pending.extend(
                        left.values()
                            .zip(right.values())
                            .map(|(left, right)| EqNode::Value(left, right)),
                    );
                },
                | EqNode::Comp(
                    Comp::Abs(left_name, left_ann, left_body),
                    Comp::Abs(right_name, right_ann, right_body),
                ) if left_name == right_name && left_ann == right_ann => {
                    pending.push(EqNode::Comp(left_body, right_body));
                },
                | EqNode::Comp(
                    Comp::App(left_head, left_arg),
                    Comp::App(right_head, right_arg),
                ) => {
                    pending.push(EqNode::Value(left_arg, right_arg));
                    pending.push(EqNode::Comp(left_head, right_head));
                },
                | EqNode::Comp(Comp::Ret(left), Comp::Ret(right))
                | EqNode::Comp(Comp::Force(left), Comp::Force(right))
                | EqNode::Comp(Comp::Dup(left), Comp::Dup(right))
                | EqNode::Comp(Comp::Drop(left), Comp::Drop(right)) => {
                    pending.push(EqNode::Value(left, right));
                },
                | EqNode::Comp(
                    Comp::Bind(left_bound, left_name, left_rest),
                    Comp::Bind(right_bound, right_name, right_rest),
                ) if left_name == right_name => {
                    pending.push(EqNode::Comp(left_rest, right_rest));
                    pending.push(EqNode::Comp(left_bound, right_bound));
                },
                | EqNode::Comp(
                    Comp::Case(left_scrutinee, left_arm1, left_arm2),
                    Comp::Case(right_scrutinee, right_arm1, right_arm2),
                ) if left_arm1.0 == right_arm1.0 && left_arm2.0 == right_arm2.0 => {
                    pending.push(EqNode::Comp(&left_arm2.1, &right_arm2.1));
                    pending.push(EqNode::Comp(&left_arm1.1, &right_arm1.1));
                    pending.push(EqNode::Value(left_scrutinee, right_scrutinee));
                },
                | EqNode::Comp(
                    Comp::Split {
                        scrut: left_scrutinee,
                        fst_name: left_first,
                        snd_name: left_second,
                        body: left_body,
                        ..
                    },
                    Comp::Split {
                        scrut: right_scrutinee,
                        fst_name: right_first,
                        snd_name: right_second,
                        body: right_body,
                        ..
                    },
                ) if left_first == right_first && left_second == right_second => {
                    // Lowering emits only motive-less splits, so the motive is
                    // intentionally outside this hole-modulo comparison.
                    pending.push(EqNode::Comp(left_body, right_body));
                    pending.push(EqNode::Value(left_scrutinee, right_scrutinee));
                },
                | EqNode::Comp(
                    Comp::With(left_first, left_second),
                    Comp::With(right_first, right_second),
                ) => {
                    pending.push(EqNode::Comp(left_second, right_second));
                    pending.push(EqNode::Comp(left_first, right_first));
                },
                | EqNode::Comp(
                    Comp::Prj(left_side, left_target),
                    Comp::Prj(right_side, right_target),
                ) if left_side == right_side => {
                    pending.push(EqNode::Comp(left_target, right_target));
                },
                | EqNode::Comp(
                    Comp::RecordProj {
                        record: left_record,
                        label: left_label,
                    },
                    Comp::RecordProj {
                        record: right_record,
                        label: right_label,
                    },
                ) if left_label == right_label => {
                    pending.push(EqNode::Value(left_record, right_record));
                },
                | EqNode::Comp(
                    Comp::Perform(left_sig, left_op, left_arg),
                    Comp::Perform(right_sig, right_op, right_arg),
                ) if left_sig == right_sig && left_op == right_op => {
                    pending.push(EqNode::Value(left_arg, right_arg));
                },
                | EqNode::Comp(
                    Comp::Handle {
                        sig: left_sig,
                        scrutinee: left_scrutinee,
                        ret: left_ret,
                        ops: left_ops,
                    },
                    Comp::Handle {
                        sig: right_sig,
                        scrutinee: right_scrutinee,
                        ret: right_ret,
                        ops: right_ops,
                    },
                ) if left_sig == right_sig
                    && left_ret.0 == right_ret.0
                    && left_ops.len() == right_ops.len() =>
                {
                    for (left, right) in left_ops.iter().zip(right_ops) {
                        if left.op != right.op
                            || left.payload != right.payload
                            || left.resume != right.resume
                        {
                            return false.into();
                        }
                        pending.push(EqNode::Comp(&left.body, &right.body));
                    }
                    pending.push(EqNode::Comp(&left_ret.1, &right_ret.1));
                    pending.push(EqNode::Comp(left_scrutinee, right_scrutinee));
                },
                | EqNode::Comp(
                    Comp::Resume(left_stack, left_body),
                    Comp::Resume(right_stack, right_body),
                ) => {
                    pending.push(EqNode::Comp(left_body, right_body));
                    pending.push(EqNode::Value(left_stack, right_stack));
                },
                | EqNode::Comp(Comp::Reset(left), Comp::Reset(right)) => {
                    pending.push(EqNode::Comp(left, right));
                },
                | EqNode::Comp(
                    Comp::Shift(left_binder, left_body),
                    Comp::Shift(right_binder, right_body),
                ) if left_binder == right_binder => {
                    pending.push(EqNode::Comp(left_body, right_body));
                },
                | EqNode::Comp(
                    Comp::ListCase {
                        scrut: left_scrutinee,
                        nil: left_nil,
                        head: left_head,
                        tail: left_tail,
                        cons: left_cons,
                    },
                    Comp::ListCase {
                        scrut: right_scrutinee,
                        nil: right_nil,
                        head: right_head,
                        tail: right_tail,
                        cons: right_cons,
                    },
                ) if left_head == right_head && left_tail == right_tail => {
                    pending.push(EqNode::Comp(left_cons, right_cons));
                    pending.push(EqNode::Comp(left_nil, right_nil));
                    pending.push(EqNode::Value(left_scrutinee, right_scrutinee));
                },
                | _ => return false.into(),
            }
        }
        true.into()
    }

    /// Item-list equality up to hole identifiers: names and ascriptions
    /// exactly, terms up to holes.
    fn items_eq_mod_holes(
        left: &[LoweredItem],
        right: &[LoweredItem],
    ) -> TestDecision
    {
        (left.len() == right.len()
            && left.iter().zip(right).all(|(left_item, right_item)| {
                left_item.name == right_item.name
                    && left_item.ascription == right_item.ascription
                    && bool::from(term_eq_mod_holes(&left_item.term, &right_item.term))
            }))
        .into()
    }

    /// The model edit (`incremental-base` ⇄ `incremental-edited`): the literal
    /// `1` becomes `2` inside one definition.
    mod localized_edits
    {
        use super::*;

        const BASE: &str =
            "def target(x: Integer) -> F Integer {\n  ret (x + 1)\n}\nprint(target)\n";
        const EDITED: &str =
            "def target(x: Integer) -> F Integer {\n  ret (x + 2)\n}\nprint(target)\n";

        /// A literal edit inside one definition is exactly one `SetInt` at the
        /// literal's path, and `apply` reproduces the edited program.
        #[test]
        fn literal_edit_is_one_set_int()
        {
            let base = lower(BASE);
            let edited = lower(EDITED);
            let script = diff(&base, &edited);

            assert_eq!(1, script.actions.len(), "exactly one action: {script:?}");
            match script.actions.first().expect("one action") {
                | Action::SetInt { path, from, to } => {
                    assert_eq!(*path, vec![0, 0, 0, 0, 1].into(), "the literal's term path");
                    assert_eq!((1, 2), (*from, *to), "1 became 2");
                },
                | other => panic!("expected a SetInt, got {other:?}"),
            }
            assert!(
                items_eq_mod_holes(&apply(&base.items, &script), &edited.items),
                "apply reproduces the edited program"
            );
        }

        /// Inserting a definition before existing ones (the `stale-relocation`
        /// shape: a prepended `def` plus a literal edit in the next) is one
        /// `InsertItem` plus one `SetInt`, touching no later item.
        #[test]
        fn item_insertion_leaves_neighbours_untouched()
        {
            let base = lower(
                "def alpha(x: Integer) -> F Integer {\n  ret (x + 1)\n}\n\
                 def target(y: Integer) -> F Integer {\n  ret (y * y)\n}\n\
                 def omega(z: Integer) -> F Integer {\n  ret (z - 1)\n}\n",
            );
            let edited = lower(
                "def inserted(n: Integer) -> F Integer {\n  ret n\n}\n\
                 def alpha(x: Integer) -> F Integer {\n  ret (x + 2)\n}\n\
                 def target(y: Integer) -> F Integer {\n  ret (y * y)\n}\n\
                 def omega(z: Integer) -> F Integer {\n  ret (z - 1)\n}\n",
            );
            let script = diff(&base, &edited);

            let inserts: Vec<&Action> = script
                .actions
                .iter()
                .filter(|action| matches!(action, Action::InsertItem { .. }))
                .collect();
            assert_eq!(1, inserts.len(), "exactly one insertion: {script:?}");
            match inserts.first().expect("one insert") {
                | Action::InsertItem { at, item } => {
                    assert_eq!(0, *at, "inserted at the front of the new list");
                    assert_eq!(Some("inserted"), item.name.as_deref());
                },
                | other => panic!("expected an InsertItem, got {other:?}"),
            }
            assert!(
                script
                    .actions
                    .iter()
                    .all(|action| !matches!(action, Action::DeleteItem { .. })),
                "no deletions"
            );
            // Every path-addressed action lands in old item 0 (`alpha`); items 1
            // and 2 (`target`, `omega`) are byte-shifted but structurally
            // unchanged, so the diff never mentions them.
            for action in &script.actions {
                if let Some(path) = action.path() {
                    assert_eq!(Some(&0), path.first(), "only alpha's term is touched");
                }
            }
            assert!(
                items_eq_mod_holes(&apply(&base.items, &script), &edited.items),
                "apply reproduces the edited program"
            );
        }

        /// Filling a user hole is one `FillHole`; erasing back is one
        /// `EraseToHole`.
        #[test]
        fn hole_fill_and_erase()
        {
            let with_hole = lower("def answer = ?;\n");
            let filled = lower("def answer = 42;\n");

            let fill = diff(&with_hole, &filled);
            assert_eq!(1, fill.actions.len(), "one action: {fill:?}");
            assert!(
                matches!(
                    fill.actions.first(),
                    Some(Action::FillHole {
                        to: Subtree::Value(Value::Int(42)),
                        ..
                    })
                ),
                "filling a hole is a FillHole, got {fill:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&with_hole.items, &fill),
                &filled.items
            ));

            let erase = diff(&filled, &with_hole);
            assert!(
                matches!(erase.actions.first(), Some(Action::EraseToHole { .. })),
                "erasing to a hole is an EraseToHole, got {erase:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&filled.items, &erase),
                &with_hole.items
            ));
        }

        /// A grade bump on a thunk is one `SetGrade`.
        #[test]
        fn grade_bump_is_one_set_grade()
        {
            let base = lower("def t = thunk[2] { ret 1 };\n");
            let edited = lower("def t = thunk[3] { ret 1 };\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one action: {script:?}");
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetGrade { from, to, .. })
                        if *from == Grade::fin(GradeBound::from(2)) && *to == Grade::fin(GradeBound::from(3))
                ),
                "a grade change is a SetGrade, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Renaming a binder composes a `Rebind` of the binder with a `SetVar`
        /// of its occurrence (the binder is an attribute, the occurrence a leaf
        /// child — both reconstructed).
        #[test]
        fn binder_rename_composes_rebind_and_setvar()
        {
            let base = lower("fn(x) { x }\n");
            let edited = lower("fn(y) { y }\n");
            let script = diff(&base, &edited);
            assert!(
                script
                    .actions
                    .iter()
                    .any(|action| matches!(action, Action::Rebind { from, to, .. } if from == "x" && to == "y")),
                "the binder is renamed: {script:?}"
            );
            assert!(
                script
                    .actions
                    .iter()
                    .any(|action| matches!(action, Action::SetVar { from, to, .. } if from == "x" && to == "y")),
                "the occurrence is renamed: {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }
    }

    /// The attribute actions: side / annotation / binder-rename, each
    /// composing with edits inside the node.
    mod attribute_edits
    {
        use super::*;

        /// Flipping an injection's side is one `SetSide`.
        #[test]
        fn injection_side_flip_is_one_set_side()
        {
            let base = lower("def v = Inl(1);\n");
            let edited = lower("def v = Inr(1);\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one action: {script:?}");
            assert!(
                matches!(script.actions.first(), Some(Action::SetSide { .. })),
                "a side flip is a SetSide, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Retyping a value ascription is one `SetAnnotation` on the value
        /// slot; the annotated term is untouched.
        #[test]
        fn value_ascription_change_is_one_set_annotation()
        {
            let base = lower("def v = (1 : Integer);\n");
            let edited = lower("def v = (1 : Unit);\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one action: {script:?}");
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetAnnotation {
                        slot: AnnSlot::Value,
                        ..
                    })
                ),
                "an ascription change is a SetAnnotation/Value, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Adding a binder annotation is one `SetAnnotation` on the abs-binder
        /// slot (the `None` ⇄ `Some` edit).
        #[test]
        fn binder_annotation_added_is_one_set_annotation()
        {
            let base = lower("fn(x) { x }\n");
            let edited = lower("fn(x: Integer) { x }\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one action: {script:?}");
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetAnnotation {
                        slot: AnnSlot::AbsBinder,
                        from: None,
                        to: Some(_),
                        ..
                    })
                ),
                "adding a binder annotation is SetAnnotation/AbsBinder None→Some, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Renaming a `case` arm binder is a `Rebind` on the matching slot,
        /// composed with the occurrence's `SetVar`.
        #[test]
        fn case_arm_rename_targets_the_right_slot()
        {
            let base =
                lower("case (Inl(1) : Integer + Integer) { Inl(a) => ret a, Inr(b) => ret 0 }\n");
            let edited =
                lower("case (Inl(1) : Integer + Integer) { Inl(c) => ret c, Inr(b) => ret 0 }\n");
            let script = diff(&base, &edited);
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::Rebind { slot: BinderSlot::Fst, from, to, .. } if from == "a" && to == "c"
                )),
                "the first arm's binder is rebound on the Fst slot: {script:?}"
            );
            assert!(
                script
                    .actions
                    .iter()
                    .all(|action| !matches!(action, Action::Rebind {
                        slot: BinderSlot::Snd,
                        ..
                    })),
                "the untouched second arm is not rebound: {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Renaming the *second* `case` arm binder is a `Rebind` on the `Snd`
        /// slot (the computation-binder dual of the first-arm test), composed
        /// with its occurrence's `SetVar`, leaving the first arm untouched.
        #[test]
        fn case_second_arm_rename_targets_the_snd_slot()
        {
            let base =
                lower("case (Inl(1) : Integer + Integer) { Inl(a) => ret a, Inr(b) => ret b }\n");
            let edited =
                lower("case (Inl(1) : Integer + Integer) { Inl(a) => ret a, Inr(d) => ret d }\n");
            let script = diff(&base, &edited);
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::Rebind { slot: BinderSlot::Snd, from, to, .. } if from == "b" && to == "d"
                )),
                "the second arm's binder is rebound on the Snd slot: {script:?}"
            );
            assert!(
                script
                    .actions
                    .iter()
                    .all(|action| !matches!(action, Action::Rebind {
                        slot: BinderSlot::Fst,
                        ..
                    })),
                "the untouched first arm is not rebound: {script:?}"
            );
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::SetVar { from, to, .. } if from == "b" && to == "d"
                )),
                "the second arm's occurrence is renamed: {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Renaming *both* `case` arm binders at once emits a `Rebind` on each
        /// slot, each composed with its occurrence's `SetVar` — the two slots
        /// are independent.
        #[test]
        fn case_both_arms_renamed_at_once()
        {
            let base =
                lower("case (Inl(1) : Integer + Integer) { Inl(a) => ret a, Inr(b) => ret b }\n");
            let edited =
                lower("case (Inl(1) : Integer + Integer) { Inl(c) => ret c, Inr(d) => ret d }\n");
            let script = diff(&base, &edited);
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::Rebind { slot: BinderSlot::Fst, from, to, .. } if from == "a" && to == "c"
                )),
                "the first arm is rebound on the Fst slot: {script:?}"
            );
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::Rebind { slot: BinderSlot::Snd, from, to, .. } if from == "b" && to == "d"
                )),
                "the second arm is rebound on the Snd slot: {script:?}"
            );
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::SetVar { from, to, .. } if from == "a" && to == "c"
                )),
                "the first arm's occurrence is renamed: {script:?}"
            );
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::SetVar { from, to, .. } if from == "b" && to == "d"
                )),
                "the second arm's occurrence is renamed: {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// An edit inside a `case` *scrutinee* descends through the
        /// `Comp::Case` to the scrutinee child (child 0) — one
        /// `SetInt`, not a coarse replacement — pinning the scrutinee
        /// descent the arm-rename tests (which hold the scrutinee
        /// constant) leave unexercised.
        #[test]
        fn case_scrutinee_edit_descends_to_one_set_int()
        {
            let base =
                lower("case (Inl(1) : Integer + Integer) { Inl(a) => ret a, Inr(b) => ret 0 }\n");
            let edited =
                lower("case (Inl(2) : Integer + Integer) { Inl(a) => ret a, Inr(b) => ret 0 }\n");
            let script = diff(&base, &edited);
            assert_eq!(
                1,
                script.actions.len(),
                "one action (scrutinee descent): {script:?}"
            );
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetInt { from: 1, to: 2, .. })
                ),
                "the scrutinee edit descends to one SetInt, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Renaming the *first* binder of a tuple `let` (`Comp::Split`) is a
        /// `Rebind` on the `Fst` slot composed with its occurrence's `SetVar`,
        /// leaving the second binder untouched.
        #[test]
        fn split_first_binder_rename_targets_the_fst_slot()
        {
            let base = lower("thunk { val (a, b) = (1, 2); ret a }\n");
            let edited = lower("thunk { val (x, b) = (1, 2); ret x }\n");
            let script = diff(&base, &edited);
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::Rebind { slot: BinderSlot::Fst, from, to, .. } if from == "a" && to == "x"
                )),
                "the first Split binder is rebound on the Fst slot: {script:?}"
            );
            assert!(
                script
                    .actions
                    .iter()
                    .all(|action| !matches!(action, Action::Rebind {
                        slot: BinderSlot::Snd,
                        ..
                    })),
                "the untouched second binder is not rebound: {script:?}"
            );
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::SetVar { from, to, .. } if from == "a" && to == "x"
                )),
                "the bound occurrence is renamed: {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Renaming the *second* binder of a tuple `let` (`Comp::Split`) is a
        /// `Rebind` on the `Snd` slot composed with its occurrence's `SetVar`,
        /// leaving the first binder untouched.
        #[test]
        fn split_second_binder_rename_targets_the_snd_slot()
        {
            let base = lower("thunk { val (a, b) = (1, 2); ret b }\n");
            let edited = lower("thunk { val (a, d) = (1, 2); ret d }\n");
            let script = diff(&base, &edited);
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::Rebind { slot: BinderSlot::Snd, from, to, .. } if from == "b" && to == "d"
                )),
                "the second Split binder is rebound on the Snd slot: {script:?}"
            );
            assert!(
                script
                    .actions
                    .iter()
                    .all(|action| !matches!(action, Action::Rebind {
                        slot: BinderSlot::Fst,
                        ..
                    })),
                "the untouched first binder is not rebound: {script:?}"
            );
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::SetVar { from, to, .. } if from == "b" && to == "d"
                )),
                "the bound occurrence is renamed: {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Renaming *both* binders of a tuple `let` (`Comp::Split`) at once
        /// emits a `Rebind` on each slot, each composed with its
        /// occurrence's `SetVar` — the `Split` analogue of the
        /// both-arms `case` test.
        #[test]
        fn split_both_binders_renamed_at_once()
        {
            let base = lower("thunk { val (a, b) = (1, 2); ret (a, b) }\n");
            let edited = lower("thunk { val (x, y) = (1, 2); ret (x, y) }\n");
            let script = diff(&base, &edited);
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::Rebind { slot: BinderSlot::Fst, from, to, .. } if from == "a" && to == "x"
                )),
                "the first binder is rebound on the Fst slot: {script:?}"
            );
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::Rebind { slot: BinderSlot::Snd, from, to, .. } if from == "b" && to == "y"
                )),
                "the second binder is rebound on the Snd slot: {script:?}"
            );
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::SetVar { from, to, .. } if from == "a" && to == "x"
                )),
                "the first occurrence is renamed: {script:?}"
            );
            assert!(
                script.actions.iter().any(|action| matches!(
                    action,
                    Action::SetVar { from, to, .. } if from == "b" && to == "y"
                )),
                "the second occurrence is renamed: {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// An edit inside a tuple `let` *scrutinee* descends through the
        /// `Comp::Split` to the scrutinee child (child 0) — one `SetInt`, not a
        /// coarse replacement — pinning the scrutinee descent the rename tests
        /// (which hold the scrutinee constant) leave unexercised.
        #[test]
        fn split_scrutinee_edit_descends_to_one_set_int()
        {
            let base = lower("thunk { val (a, b) = (1, 2); ret a }\n");
            let edited = lower("thunk { val (a, b) = (9, 2); ret a }\n");
            let script = diff(&base, &edited);
            assert_eq!(
                1,
                script.actions.len(),
                "one action (scrutinee descent): {script:?}"
            );
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetInt { from: 1, to: 9, .. })
                ),
                "the scrutinee edit descends to one SetInt, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Flipping a *projection's* side (`Comp::Prj`) is one `SetSide`, the
        /// computation-sorted dual of the injection side flip; the projection
        /// target is untouched.
        #[test]
        fn projection_side_flip_is_one_set_side()
        {
            let base = lower("(ret 1).fst\n");
            let edited = lower("(ret 1).snd\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one action: {script:?}");
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetSide {
                        from: Side::Fst,
                        to: Side::Snd,
                        ..
                    })
                ),
                "a projection side flip is a SetSide Fst→Snd, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// An edit inside a projection *target* (the side held constant)
        /// descends through the `Comp::Prj` to the target child — one `SetInt`,
        /// not a coarse replacement — pinning the target descent that the
        /// side-flip test (which holds the target constant) leaves unexercised.
        #[test]
        fn projection_target_edit_descends_to_one_set_int()
        {
            let base = lower("(ret 1).fst\n");
            let edited = lower("(ret 9).fst\n");
            let script = diff(&base, &edited);
            assert_eq!(
                1,
                script.actions.len(),
                "one action (target descent): {script:?}"
            );
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetInt { from: 1, to: 9, .. })
                ),
                "the target edit descends to one SetInt, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// An edit inside one field of a lazy product (`Comp::With`) descends
        /// to that field — one `SetInt`, not a coarse replacement — exercising
        /// `With` structural descent (two computation children).
        #[test]
        fn with_field_edit_is_one_set_int()
        {
            let base = lower("co { fst = ret 1, snd = ret 2 }\n");
            let edited = lower("co { fst = ret 9, snd = ret 2 }\n");
            let script = diff(&base, &edited);
            assert_eq!(
                1,
                script.actions.len(),
                "one action (descent, not replace): {script:?}"
            );
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetInt { from: 1, to: 9, .. })
                ),
                "the edited field descends to one SetInt, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// An edit inside the *second* field of a lazy product (`Comp::With`)
        /// descends to that field — one `SetInt` — pinning the `snd`-child
        /// descent the first-field test leaves unexercised (a coarsening of
        /// either With child to a sound `Replace` would otherwise survive).
        #[test]
        fn with_second_field_edit_is_one_set_int()
        {
            let base = lower("co { fst = ret 1, snd = ret 2 }\n");
            let edited = lower("co { fst = ret 1, snd = ret 9 }\n");
            let script = diff(&base, &edited);
            assert_eq!(
                1,
                script.actions.len(),
                "one action (snd-field descent): {script:?}"
            );
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetInt { from: 2, to: 9, .. })
                ),
                "the edited second field descends to one SetInt, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Dropping a binder annotation is one `SetAnnotation` on the
        /// abs-binder slot (the `Some` ⇄ `None` edit, the dual of
        /// adding one).
        #[test]
        fn binder_annotation_dropped_is_one_set_annotation()
        {
            let base = lower("fn(x: Integer) { x }\n");
            let edited = lower("fn(x) { x }\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one action: {script:?}");
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetAnnotation {
                        slot: AnnSlot::AbsBinder,
                        from: Some(_),
                        to: None,
                        ..
                    })
                ),
                "dropping a binder annotation is SetAnnotation/AbsBinder Some→None, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// An attribute change and a deeper child edit at once: a grade bump on
        /// a thunk plus a literal edit in its body compose into two actions at
        /// nested paths, both reconstructed.
        #[test]
        fn attribute_and_nested_child_edit_compose()
        {
            let base = lower("def t = thunk[2] { ret 1 };\n");
            let edited = lower("def t = thunk[3] { ret 2 };\n");
            let script = diff(&base, &edited);
            assert!(
                script
                    .actions
                    .iter()
                    .any(|action| matches!(action, Action::SetGrade { .. })),
                "the grade bump is present: {script:?}"
            );
            assert!(
                script
                    .actions
                    .iter()
                    .any(|action| matches!(action, Action::SetInt { from: 1, to: 2, .. })),
                "the nested literal edit is present: {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// Changing an item's attached signature is one `SetItemAscription`.
        #[test]
        fn item_ascription_change_is_one_set_item_ascription()
        {
            let base = lower("def f : Integer;\ndef f = 1;\n");
            let edited = lower("def f : Unit;\ndef f = 1;\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one action: {script:?}");
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::SetItemAscription { at: 0, .. })
                ),
                "the ascription change is a SetItemAscription, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }
    }

    /// The honest residual: unalignable nodes become coarse replacements.
    mod coarse_fallback
    {
        use super::*;

        /// A node whose constructor changed (an eager pair becomes an
        /// injection) is one coarse `Replace`, not a descent.
        #[test]
        fn constructor_change_is_one_replace()
        {
            let base = lower("def v = (1, 2);\n");
            let edited = lower("def v = Inl(1);\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one coarse action: {script:?}");
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::Replace {
                        to: Subtree::Value(Value::Inj(..)),
                        ..
                    })
                ),
                "a constructor change is a Replace, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// A *computation* whose constructor changed (a lazy product becomes a
        /// projection) is one coarse `Replace` carrying a `Subtree::Comp` — the
        /// computation-sorted dual of the value constructor-change case, with
        /// no sort flip (both terms are computations).
        #[test]
        fn comp_constructor_change_is_one_replace()
        {
            let base = lower("co { fst = ret 1, snd = ret 2 }\n");
            let edited = lower("(ret 1).fst\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one coarse action: {script:?}");
            assert!(
                matches!(
                    script.actions.first(),
                    Some(Action::Replace {
                        to: Subtree::Comp(Comp::Prj(..)),
                        ..
                    })
                ),
                "a computation constructor change is a Replace/Comp, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }

        /// An item whose root term changes **sort** (a value definition becomes
        /// a computation one) is one coarse `Replace` carrying the other sort —
        /// `apply` must install it across the sort boundary, not silently keep
        /// the old value (regression for the cross-sort root-replace bug).
        #[test]
        fn cross_sort_root_replace_is_reconstructed()
        {
            // `def f = 1;` is value-rooted; `def f = g(1);` is computation-rooted
            // (a call). The items match by name `f`, so the diff replaces the
            // root with a `Subtree::Comp`.
            let base = lower("def f = 1;\n");
            let edited = lower("def f = g(1);\n");
            let forward = diff(&base, &edited);
            assert!(
                matches!(
                    forward.actions.first(),
                    Some(Action::Replace {
                        to: Subtree::Comp(_),
                        ..
                    })
                ),
                "a value→computation root change is a Replace/Comp, got {forward:?}"
            );
            assert!(
                items_eq_mod_holes(&apply(&base.items, &forward), &edited.items),
                "apply installs the computation across the sort boundary"
            );
            // And the reverse (computation → value).
            let backward = diff(&edited, &base);
            assert!(
                matches!(
                    backward.actions.first(),
                    Some(Action::Replace {
                        to: Subtree::Value(_),
                        ..
                    })
                ),
                "a computation→value root change is a Replace/Value, got {backward:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&edited.items, &backward),
                &base.items
            ));
        }

        /// Removing a trailing definition is one `DeleteItem`.
        #[test]
        fn item_deletion_is_one_delete()
        {
            let base = lower("def a = 1;\ndef b = 2;\n");
            let edited = lower("def a = 1;\n");
            let script = diff(&base, &edited);
            assert_eq!(1, script.actions.len(), "one action: {script:?}");
            assert!(
                matches!(script.actions.first(), Some(Action::DeleteItem { at: 1 })),
                "deleting the second item is DeleteItem at old index 1, got {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }
    }

    /// The byte-range localizer and its tie to the structural diff.
    mod localization
    {
        use super::*;

        const BASE: &str =
            "def target(x: Integer) -> F Integer {\n  ret (x + 1)\n}\nprint(target)\n";

        /// [`localize`] returns the smallest enclosing core term: the literal's
        /// own path for the literal's byte range, and a strictly shorter
        /// enclosing path for a wider range.
        #[test]
        fn localize_finds_smallest_enclosing_term()
        {
            let base = lower(BASE);
            let literal = base
                .origin
                .get_path(&[0, 0, 0, 0, 1])
                .expect("the literal has an origin entry");

            let exact = localize(
                &base.origin,
                literal.byte_range.start.into(),
                literal.byte_range.end.into(),
            )
            .expect("a locus for the literal's range");
            assert_eq!(
                exact,
                vec![0, 0, 0, 0, 1].into(),
                "the literal localizes to itself"
            );

            // A range spanning the whole first definition localizes to a strict
            // ancestor of the literal: a shorter path that is its prefix (the
            // nesting `localize` exploits). The deepest node sharing the item's
            // full byte span is the locus.
            let item = base
                .origin
                .get_path(&[0])
                .expect("the first item has an origin entry");
            let wide = localize(
                &base.origin,
                item.byte_range.start.into(),
                item.byte_range.end.into(),
            )
            .expect("a locus for the item range");
            assert!(
                wide.len() < exact.len(),
                "the item localizes higher: {wide:?}"
            );
            assert!(
                exact.starts_with(&wide),
                "the literal locus {exact:?} nests inside the item locus {wide:?}"
            );
        }

        /// [`edit_locus`] over a tree-sitter edit's *old* coordinates agrees
        /// with [`localize`], and every changed path of the structural diff
        /// sits within (has as a prefix) the locus.
        #[test]
        fn edit_locus_contains_the_diff()
        {
            let base = lower(BASE);
            let edited =
                lower("def target(x: Integer) -> F Integer {\n  ret (x + 2)\n}\nprint(target)\n");
            let literal = base
                .origin
                .get_path(&[0, 0, 0, 0, 1])
                .expect("literal origin");

            // The edit that turns `1` into `2`: a one-byte replacement at the
            // literal (same length, so `old_end == new_end`).
            let edit = SourceEdit::new(
                literal.byte_range.start.into(),
                literal.byte_range.end.into(),
                literal.byte_range.end.into(),
            );
            let locus = edit_locus(&base.origin, &edit).expect("a locus for the edit");
            assert_eq!(locus, vec![0, 0, 0, 0, 1].into());

            for action in &diff(&base, &edited).actions {
                if let Some(path) = action.path() {
                    assert!(
                        path.starts_with(&locus),
                        "changed path {path:?} must sit within the locus {locus:?}"
                    );
                }
            }
        }

        /// A *multi-point* contiguous edit — changing both the operator and the
        /// literal of `x + 1` to `x * 2` — localizes to their common-ancestor
        /// term, and BOTH induced actions (a `SetVar` for the operator, a
        /// `SetInt` for the literal) sit within it. This is the
        /// non-tautological containment case: the locus is a strict
        /// ancestor of two distinct, non-prefix-related changed paths.
        #[test]
        fn multi_point_edit_localizes_to_the_common_ancestor()
        {
            let base = lower(BASE);
            let edited =
                lower("def target(x: Integer) -> F Integer {\n  ret (x * 2)\n}\nprint(target)\n");

            // The operator application `x + 1` is the smallest term spanning both
            // the operator and the literal; use its byte span as the edit range.
            let app = base
                .origin
                .get_path(&[0, 0, 0, 0])
                .expect("the operator application has an origin entry");
            let locus = localize(
                &base.origin,
                app.byte_range.start.into(),
                app.byte_range.end.into(),
            )
            .expect("a locus for the operator-application range");

            let script = diff(&base, &edited);
            let changed: Vec<_> = script.actions.iter().filter_map(Action::path).collect();
            assert!(
                changed.len() >= 2,
                "the edit changes at least two nodes: {script:?}"
            );
            for path in &changed {
                assert!(
                    path.starts_with(&locus),
                    "changed path {path:?} must sit within the common-ancestor locus {locus:?}"
                );
            }
            // The two distinct changes really are non-prefix-related siblings
            // under the locus, not one nested inside the other.
            assert!(
                script
                    .actions
                    .iter()
                    .any(|action| matches!(action, Action::SetVar { from, to, .. } if from == "add" && to == "mul")),
                "the operator elaboration changes add→mul: {script:?}"
            );
            assert!(
                script
                    .actions
                    .iter()
                    .any(|action| matches!(action, Action::SetInt { from: 1, to: 2, .. })),
                "the literal changes 1→2: {script:?}"
            );
            assert!(items_eq_mod_holes(
                &apply(&base.items, &script),
                &edited.items
            ));
        }
    }

    /// `apply` as the diff's adjoint.
    mod application
    {
        use super::*;

        /// A self-diff is empty, and applying an empty script is the identity.
        #[test]
        fn self_diff_is_empty_and_apply_is_identity()
        {
            let program =
                lower("def square(n: Integer) -> F Integer {\n  ret (n * n)\n}\nsquare\n");
            let script = diff(&program, &program);
            assert!(
                script.actions.is_empty(),
                "a self-diff has no actions: {script:?}"
            );
            assert!(
                items_eq_mod_holes(&apply(&program.items, &script), &program.items),
                "applying an empty script is the identity"
            );
        }
    }

    /// The soundness oracle: `apply(old, diff(old, new))` ≡ `new` (mod holes).
    mod oracle
    {
        use gandr_core_checker::strategies::binder_name;
        use proptest::collection::vec;
        use proptest::prelude::*;

        use super::*;

        /// A small program: zero to four grammar-directed items.
        fn program_source() -> impl Strategy<Value = String>
        {
            vec(item_source(), 0_usize .. 5_usize).prop_map(|items| items.join("\n"))
        }

        /// One grammar-directed top-level item, spanning both term sorts, the
        /// attribute-bearing constructors, and the multi-child descent
        /// constructors so the oracle exercises more than value-rooted descent:
        /// a literal binding, a signature, an alias, a function (thunk-valued),
        /// an ascribed value, an injection, a graded thunk, a
        /// computation-rooted call (the cross-sort root case), a `case`
        /// (binary-sum descent), a lazy product (`co` ⇒ `With`
        /// descent), a projection (`(ret n).fst` ⇒ `Prj` descent, with
        /// a random side so two bare items can flip it), a tuple `let`
        /// (`Split` descent through two binders), a hole binding, and a
        /// deliberately broken fragment (recovered to a hole).
        fn item_source() -> impl Strategy<Value = String>
        {
            prop_oneof![
                (binder_name(), 0_u64 .. 50_u64)
                    .prop_map(|(name, literal)| format!("def {name} = {literal};")),
                // String literals (value-model ladder, ADR-38): a small pool so
                // two items can match (a self-diff stays empty) or differ (a
                // wholesale `Replace`), exercising `diff_value`'s `Str` arm.
                (binder_name(), prop_oneof![
                    Just("hi"),
                    Just("bye"),
                    Just(""),
                    Just("a b")
                ],)
                    .prop_map(|(name, text)| format!("def {name} = \"{text}\";")),
                (binder_name(), prop_oneof![Just("x"), Just("yz")])
                    .prop_map(|(name, text)| format!("def {name} = (\"{text}\" : String);")),
                // Typed numeric literals (value-model ladder, ADR-39): a small
                // pool spanning all six suffixes plus a bare float, so two items
                // can match (self-diff stays empty) or differ (a wholesale
                // `Replace`), exercising `diff_value`'s `Num` arm.
                (binder_name(), prop_oneof![
                    Just("8080u32"),
                    Just("100u64"),
                    Just("255i32"),
                    Just("9i64"),
                    Just("1.5f32"),
                    Just("2.0f64"),
                    Just("3.14")
                ],)
                    .prop_map(|(name, literal)| format!("def {name} = {literal};")),
                binder_name().prop_map(|name| format!("def {name} : Integer;")),
                (binder_name(), binder_name()).prop_map(|(lhs, rhs)| format!("def {lhs} = {rhs};")),
                (binder_name(), binder_name(), 0_u64 .. 9_u64).prop_map(
                    |(name, param, literal)| format!(
                        "def {name}({param}: Integer) -> F Integer {{ ret ({param} + {literal}) }}"
                    )
                ),
                (binder_name(), 0_u64 .. 9_u64)
                    .prop_map(|(name, literal)| format!("def {name} = ({literal} : Integer);")),
                (binder_name(), 0_u64 .. 9_u64)
                    .prop_map(|(name, literal)| format!("def {name} = Inl({literal});")),
                (binder_name(), 0_u64 .. 4_u64, 0_u64 .. 9_u64).prop_map(
                    |(name, grade, literal)| format!(
                        "def {name} = thunk[{grade}] {{ ret {literal} }};"
                    )
                ),
                // Computation-rooted item (a trailing call): exercises
                // `diff_comp` at the root and the cross-sort root replace.
                (binder_name(), binder_name()).prop_map(|(callee, arg)| format!("{callee}({arg})")),
                // A `case` def: exercises `Case` descent (a scrutinee plus two
                // arm bodies, each with its own binder) through the oracle.
                (binder_name(), binder_name(), 0_u64 .. 9_u64).prop_map(|(name, arm, literal)| {
                    format!(
                        "def {name} = case (Inl({literal}) : Integer + Integer) \
                         {{ Inl({arm}) => ret {arm}, Inr(other) => ret {literal} }};"
                    )
                }),
                // A lazy product `co { fst = …, snd = … }` (bare, the proven
                // form): exercises `With` descent — two computation children —
                // through `diff_comp` / `rebuild_comp`.
                (0_u64 .. 9_u64, 0_u64 .. 9_u64).prop_map(|(left, right)| {
                    format!("co {{ fst = ret {left}, snd = ret {right} }}")
                }),
                // A projection `(ret n).fst` / `.snd` (bare, the proven form):
                // exercises `Prj` descent, and — when two bare items align on
                // opposite sides — the `SetSide` rebuild on a computation.
                (0_u64 .. 9_u64, any::<bool>()).prop_map(|(literal, snd)| {
                    let field = if snd { "snd" } else { "fst" };
                    format!("(ret {literal}).{field}")
                }),
                // A tuple `val` `thunk { val (p, q) = (a, b); ret p }`:
                // exercises `Split` descent (two binders, a scrutinee, a body)
                // through the thunk body.
                (
                    binder_name(),
                    binder_name(),
                    binder_name(),
                    0_u64 .. 9_u64,
                    0_u64 .. 9_u64,
                )
                    .prop_map(|(name, fst_binder, snd_binder, left, right)| {
                        format!(
                            "def {name} = thunk {{ let ({fst_binder}, {snd_binder}) = \
                             ({left}, {right}); ret {fst_binder} }};"
                        )
                    }),
                // A list literal `def n = [a, b, …];` (ADR-40): exercises
                // `Value::List` n-ary element descent — a same-length pair diffs
                // element-wise, a length change replaces wholesale, and a
                // self-diff is empty — through the oracle.
                (binder_name(), vec(0_u64 .. 9_u64, 0_usize .. 4_usize)).prop_map(
                    |(name, elements)| {
                        let items = elements
                            .iter()
                            .map(u64::to_string)
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("def {name} = [{items}];")
                    }
                ),
                // A list-case `def n = case [k] { Nil => …, Cons(h, rest) => … }`
                // (ADR-40): exercises `Comp::ListCase` descent (scrutinee, nil
                // body, cons body) and the `head` `Rebind` (Fst) through the
                // oracle.
                (binder_name(), binder_name(), 0_u64 .. 9_u64).prop_map(|(name, head, literal)| {
                    format!(
                        "def {name} = case [{literal}] \
                         {{ Nil => ret {literal}, Cons({head}, rest) => ret {head} }};"
                    )
                }),
                // A record literal `def n = #{a = x, b = y};` (ADR-45):
                // exercises `Value::Record` n-ary field descent — a same-label-set
                // field change diffs field-wise, a label-set change replaces
                // wholesale, and a self-diff is empty — through the oracle (the
                // latent-bug-prone diff face, per the string rung's self-diff bug).
                (binder_name(), 0_u64 .. 9_u64, 0_u64 .. 9_u64)
                    .prop_map(|(name, x, y)| { format!("def {name} = #{{a = {x}, b = {y}}};") }),
                // A record projection `def n = #{a = x}.a;` (ADR-45): exercises
                // `Comp::RecordProj` descent (the record value child, the stable
                // projected label).
                (binder_name(), 0_u64 .. 9_u64)
                    .prop_map(|(name, x)| format!("def {name} = #{{a = {x}}}.a;")),
                binder_name().prop_map(|name| format!("def {name} = ?;")),
                binder_name().prop_map(|name| format!("def {name} = ;")),
            ]
        }

        proptest! {
            /// For arbitrary lowered program pairs, `apply(old, diff(old, new))`
            /// reproduces `new` up to hole identifiers — soundness is total even
            /// where localization falls back to coarse replacement.
            #[test]
            fn apply_of_diff_reproduces_new(
                old_source in program_source(),
                new_source in program_source(),
            )
            {
                let old = lower(&old_source);
                let new = lower(&new_source);
                let script = diff(&old, &new);
                let rebuilt = apply(&old.items, &script);
                prop_assert!(
                    items_eq_mod_holes(&rebuilt, &new.items),
                    "apply∘diff must reproduce new\nold: {old_source}\nnew: {new_source}\nscript: {script:?}"
                );
            }

            /// A self-diff is always empty and applies as the identity.
            #[test]
            fn self_diff_is_identity(source in program_source())
            {
                let program = lower(&source);
                let script = diff(&program, &program);
                prop_assert!(script.actions.is_empty(), "self-diff must be empty: {script:?}");
                prop_assert!(
                    items_eq_mod_holes(&apply(&program.items, &script), &program.items),
                    "applying an empty script is the identity"
                );
            }
        }
    }

    /// The A3 effect/control constructors and the grade structural ops localize
    /// edits into their children rather than coarse-replacing, and `apply`
    /// stays the diff's adjoint over them (`wyrd-q1mz`). The surface does not
    /// yet lower to these forms, so the fixtures build the core terms directly,
    /// overwriting the term of one lowered item; `diff`/`apply` read only the
    /// item terms, never the origin map, so the stale origin is immaterial.
    /// (`Value::Stk` reified stacks are machine-constructed, not a surface
    /// form; the diff treats them as opaque — sound coarse replacement —
    /// and descent into a reified stack is deferred.)
    mod effect_control
    {
        use alloc::rc::Rc;

        use gandr_core_checker::effect::EffectOp;
        use gandr_core_checker::effect::EffectSig;
        use gandr_core_checker::syntax::OpClause;
        use gandr_core_checker::syntax::Stack;
        use gandr_core_checker::types::ValueType;
        use gandr_surface_engine::edit::EditScript;
        use gandr_surface_engine::origin::TermRef;
        use gandr_surface_engine::origin::resolve;
        use proptest::prelude::*;

        use super::*;
        /// Editing the computation fed to `resume` localizes (child 1).
        #[test]
        fn resume_computation_edit_localizes()
        {
            let script = diff_sound(
                &item(Comp::resume(Value::var("s"), ret(1))),
                &item(Comp::resume(Value::var("s"), ret(2))),
            );
            assert!(
                script.actions.iter().any(|action| {
                    // child index 1 = the fed computation; the SetInt sits one
                    // level deeper (the ret's value), so path[1] is the resume
                    // child index, pinning the diff side of the child order.
                    matches!(action, Action::SetInt { from, to, path }
                        if *from == 1 && *to == 2 && path.get(1) == Some(&1))
                }),
                "the fed computation localizes to a SetInt under child 1: {script:?}"
            );
            assert!(!has_replace(&script), "resume is not replaced: {script:?}");
        }
        /// A reified stack is opaque to the diff: an unchanged stack yields an
        /// empty self-diff (no spurious replace), and a changed stack is a
        /// coarse replace — both round-trip.
        #[test]
        fn reified_stack_is_opaque_but_sound()
        {
            let empty = item(Comp::resume(Value::stk(Stack::empty()), ret(1)));
            let identity = diff(&empty, &empty);
            assert!(
                identity.actions.is_empty(),
                "self-diff over a stack is empty: {identity:?}"
            );

            let pushed = item(Comp::resume(
                Value::stk(Stack::arg(Value::int(0), Stack::empty())),
                ret(1),
            ));
            let script = diff_sound(&empty, &pushed);
            assert!(
                script
                    .actions
                    .iter()
                    .any(|action| matches!(action, Action::Replace { .. })),
                "a changed stack is a Replace: {script:?}"
            );
        }

        /// A single-item program whose term is the synthetic computation
        /// `comp`.
        fn item(comp: Comp) -> Lowered
        {
            let mut program = lower("def t = thunk[2] { ret 1 };\n");
            program.items[0].term = Term::Comp(comp);
            program
        }
        /// Editing a handler's scrutinee localizes (child 0).
        #[test]
        fn handle_scrutinee_edit_localizes()
        {
            let script = diff_sound(
                &item(handler(ret(1), ret(9), ret(8))),
                &item(handler(ret(2), ret(9), ret(8))),
            );
            assert!(
                has_set_int(&script, 1, 2),
                "scrutinee edit localizes: {script:?}"
            );
            assert!(
                !has_replace(&script),
                "the handler is not replaced: {script:?}"
            );
        }
        /// Editing a handler's return body localizes (child 1).
        #[test]
        fn handle_return_body_edit_localizes()
        {
            let script = diff_sound(
                &item(handler(ret(1), ret(9), ret(8))),
                &item(handler(ret(1), ret(7), ret(8))),
            );
            assert!(
                has_set_int(&script, 9, 7),
                "return-body edit localizes: {script:?}"
            );
            assert!(
                !has_replace(&script),
                "the handler is not replaced: {script:?}"
            );
        }

        /// A handler over `state_sig` with the given scrutinee, return body,
        /// and single-clause body.
        fn handler(
            scrutinee: Comp,
            return_body: Comp,
            clause_body: Comp,
        ) -> Comp
        {
            Comp::handle(state_sig(), scrutinee, "x", return_body, vec![
                OpClause::new("get", "p", "k", clause_body),
            ])
        }
        /// Changing a `perform`'s operation name is a coarse replace.
        #[test]
        fn perform_op_change_is_replace()
        {
            let script = diff_sound(
                &item(Comp::perform(state_sig(), "get", Value::int(1))),
                &item(Comp::perform(state_sig(), "set", Value::int(1))),
            );
            assert!(
                matches!(script.actions.first(), Some(Action::Replace { .. })),
                "an op-name change is a Replace: {script:?}"
            );
        }
        /// A handler whose return binder changed has a different skeleton, so
        /// it is a coarse replace (the clause binders are not in
        /// `BinderSlot`).
        #[test]
        fn handle_skeleton_change_is_replace()
        {
            let old = item(Comp::handle(state_sig(), ret(1), "x", ret(9), vec![
                OpClause::new("get", "p", "k", ret(8)),
            ]));
            let new = item(Comp::handle(state_sig(), ret(1), "y", ret(9), vec![
                OpClause::new("get", "p", "k", ret(8)),
            ]));
            let script = diff_sound(&old, &new);
            assert!(
                matches!(script.actions.first(), Some(Action::Replace { .. })),
                "a return-binder change is a Replace: {script:?}"
            );
        }
        /// `origin::resolve` (hence `step_comp`) follows the SAME child-index
        /// convention `diff`/`rebuild` use, for every new constructor — the
        /// three-way agreement their doc comments assert, here enforced. (A
        /// child-index swap kept consistent between `diff` and `rebuild` would
        /// round-trip yet desync from `step_comp`; this is the test that
        /// bites.)
        #[test]
        fn step_comp_child_order_matches_diff_and_rebuild()
        {
            // Dup / Drop / Perform: a single value child at 0.
            assert_eq!(
                Some(TestInteger(5)),
                resolved_sentinel(&Term::Comp(Comp::dup(Value::int(5))), &[0])
            );
            assert_eq!(
                Some(TestInteger(6)),
                resolved_sentinel(&Term::Comp(Comp::drop(Value::int(6))), &[0])
            );
            assert_eq!(
                Some(TestInteger(42)),
                resolved_sentinel(
                    &Term::Comp(Comp::perform(state_sig(), "get", Value::int(42))),
                    &[0]
                )
            );
            // Resume: value child 0, computation child 1 (not swapped).
            let resume = Term::Comp(Comp::resume(Value::int(1), ret(2)));
            assert_eq!(Some(TestInteger(1)), resolved_sentinel(&resume, &[0]));
            assert_eq!(Some(TestInteger(2)), resolved_sentinel(&resume, &[1]));
            // Reset / Shift: a single computation child at 0.
            assert_eq!(
                Some(TestInteger(3)),
                resolved_sentinel(&Term::Comp(Comp::reset(ret(3))), &[0])
            );
            assert_eq!(
                Some(TestInteger(4)),
                resolved_sentinel(&Term::Comp(Comp::shift("k", ret(4))), &[0])
            );
            // Handle: scrutinee 0, return body 1, clause bodies 2 + k.
            let handle = Term::Comp(handler2(ret(7), ret(8), ret(9), ret(10)));
            assert_eq!(Some(TestInteger(7)), resolved_sentinel(&handle, &[0]));
            assert_eq!(Some(TestInteger(8)), resolved_sentinel(&handle, &[1]));
            assert_eq!(Some(TestInteger(9)), resolved_sentinel(&handle, &[2]));
            assert_eq!(Some(TestInteger(10)), resolved_sentinel(&handle, &[3]));
        }

        /// The one-operation signature `State { get : Unit ⇒ Integer }`.
        fn state_sig() -> EffectSig
        {
            EffectSig::new("State".into(), vec![EffectOp::new(
                "get".into(),
                ValueType::Unit,
                ValueType::integer(),
            )])
        }
        /// Renaming a `shift` continuation binder is a `Rebind`, and a body
        /// edit localizes — the binder is an attribute composing with
        /// body edits.
        #[test]
        fn shift_binder_rebind_and_body_edit_compose()
        {
            let script = diff_sound(
                &item(Comp::shift("k", ret(1))),
                &item(Comp::shift("j", ret(2))),
            );
            assert!(
                script.actions.iter().any(|action| {
                    matches!(action, Action::Rebind { from, to, .. } if from == "k" && to == "j")
                }),
                "the continuation binder is a Rebind: {script:?}"
            );
            assert!(
                has_set_int(&script, 1, 2),
                "the body edit localizes: {script:?}"
            );
            assert!(!has_replace(&script), "shift is not replaced: {script:?}");
        }

        /// The integer sentinel a resolved child carries — `n` for a
        /// `Value::Int`, or `ret n` for a `Comp::Ret(Int)` — used to pin which
        /// child a path lands on.
        fn resolved_sentinel<'path>(
            term: &Term,
            path: impl Into<TestPath<'path>>,
        ) -> Option<TestInteger>
        {
            let path = path.into().0;
            match resolve(term, path) {
                | Some(TermRef::Value(Value::Int(literal))) => Some((*literal).into()),
                | Some(TermRef::Comp(Comp::Ret(value))) => match &**value {
                    | Value::Int(literal) => Some((*literal).into()),
                    | _ => None,
                },
                | _ => None,
            }
        }
        /// Every handler-skeleton dimension forces a coarse replace (and still
        /// round-trips): a signature change, a clause-count change, and an
        /// operation-, payload-binder-, or resume-binder rename. The resume-
        /// and payload-binder cases are load-bearing — there is no
        /// compensating per-binder action, so dropping a skeleton
        /// conjunct would be a silent soundness regression (the diff
        /// would localize and lose the rename).
        #[test]
        fn handle_skeleton_dimensions_are_replace()
        {
            let base = handler(ret(1), ret(2), ret(3));
            let dimensions = [
                // signature change (same clause op name, different signature).
                Comp::handle(
                    EffectSig::new("Store".into(), vec![EffectOp::new(
                        "get".into(),
                        ValueType::Unit,
                        ValueType::integer(),
                    )]),
                    ret(1),
                    "x",
                    ret(2),
                    vec![OpClause::new("get", "p", "k", ret(3))],
                ),
                // clause-count change (one clause vs two).
                handler2(ret(1), ret(2), ret(3), ret(4)),
                // operation-name rename.
                Comp::handle(state_sig(), ret(1), "x", ret(2), vec![OpClause::new(
                    "put",
                    "p",
                    "k",
                    ret(3),
                )]),
                // payload-binder rename.
                Comp::handle(state_sig(), ret(1), "x", ret(2), vec![OpClause::new(
                    "get",
                    "q",
                    "k",
                    ret(3),
                )]),
                // resume-binder rename.
                Comp::handle(state_sig(), ret(1), "x", ret(2), vec![OpClause::new(
                    "get",
                    "p",
                    "j",
                    ret(3),
                )]),
            ];
            for changed in dimensions {
                let script = diff_sound(&item(base.clone()), &item(changed));
                assert!(
                    matches!(script.actions.first(), Some(Action::Replace { .. })),
                    "a skeleton change is a Replace: {script:?}"
                );
            }
        }

        /// A returner `ret n` (the simplest computation with an editable leaf).
        fn ret(literal: impl Into<TestInteger>) -> Comp
        {
            let literal = literal.into().0;
            Comp::Ret(Rc::new(Value::int(literal)))
        }
        /// Editing a handler's operation-clause body localizes (child 2).
        #[test]
        fn handle_clause_body_edit_localizes()
        {
            let script = diff_sound(
                &item(handler(ret(1), ret(9), ret(8))),
                &item(handler(ret(1), ret(9), ret(5))),
            );
            assert!(
                has_set_int(&script, 8, 5),
                "clause-body edit localizes: {script:?}"
            );
            assert!(
                !has_replace(&script),
                "the handler is not replaced: {script:?}"
            );
        }
        /// Editing the reified-stack value of `resume` localizes to a `SetVar`
        /// (child 0).
        #[test]
        fn resume_stack_edit_localizes()
        {
            let script = diff_sound(
                &item(Comp::resume(Value::var("s"), ret(1))),
                &item(Comp::resume(Value::var("t"), ret(1))),
            );
            assert!(
                script.actions.iter().any(|action| {
                    // child index 0 = the reified-stack value, pinning the diff
                    // side of the Resume child order against `step_comp`.
                    matches!(action, Action::SetVar { from, to, path }
                        if from == "s" && to == "t" && path.get(1) == Some(&0))
                }),
                "the stack value localizes to a SetVar at child 0: {script:?}"
            );
            assert!(!has_replace(&script), "resume is not replaced: {script:?}");
        }
        /// A change between different control constructors is a coarse replace.
        #[test]
        fn cross_constructor_change_is_replace()
        {
            let script = diff_sound(&item(Comp::reset(ret(1))), &item(ret(1)));
            assert!(
                matches!(script.actions.first(), Some(Action::Replace { .. })),
                "reset ⇒ ret is a Replace: {script:?}"
            );
        }

        /// Diffs `old`→`new`, asserts `apply(old, diff)` reconstructs `new`
        /// (the soundness adjoint, total regardless of localization),
        /// and returns the script for variant inspection.
        fn diff_sound(
            old: &Lowered,
            new: &Lowered,
        ) -> EditScript
        {
            let script = diff(old, new);
            assert!(
                items_eq_mod_holes(&apply(&old.items, &script), &new.items),
                "apply∘diff must reproduce new; script: {script:?}"
            );
            script
        }
        /// Changing a `perform`'s signature is a coarse replace.
        #[test]
        fn perform_signature_change_is_replace()
        {
            let script = diff_sound(
                &item(Comp::perform(state_sig(), "get", Value::int(1))),
                &item(Comp::perform(reader_sig(), "get", Value::int(1))),
            );
            assert!(
                matches!(script.actions.first(), Some(Action::Replace { .. })),
                "a signature change is a Replace: {script:?}"
            );
        }

        /// A structurally distinct signature (different name and operation).
        fn reader_sig() -> EffectSig
        {
            EffectSig::new("Reader".into(), vec![EffectOp::new(
                "ask".into(),
                ValueType::Unit,
                ValueType::integer(),
            )])
        }
        /// Editing a `perform` payload localizes to a `SetInt`, not a replace.
        #[test]
        fn perform_payload_edit_localizes()
        {
            let script = diff_sound(
                &item(Comp::perform(state_sig(), "get", Value::int(1))),
                &item(Comp::perform(state_sig(), "get", Value::int(2))),
            );
            assert!(
                has_set_int(&script, 1, 2),
                "payload edit is a SetInt: {script:?}"
            );
            assert!(
                !has_replace(&script),
                "the operation is not replaced: {script:?}"
            );
        }
        /// The grade structural ops `dup`/`drop` localize edits into their
        /// value.
        #[test]
        fn grade_op_value_edits_localize()
        {
            let dup = diff_sound(
                &item(Comp::dup(Value::int(1))),
                &item(Comp::dup(Value::int(2))),
            );
            assert!(
                bool::from(has_set_int(&dup, 1, 2)) && !has_replace(&dup),
                "dup localizes: {dup:?}"
            );
            let drop = diff_sound(
                &item(Comp::drop(Value::int(3))),
                &item(Comp::drop(Value::int(4))),
            );
            assert!(
                bool::from(has_set_int(&drop, 3, 4)) && !has_replace(&drop),
                "drop localizes: {drop:?}"
            );
        }

        /// Whether any action is a `SetInt from→to`.
        fn has_set_int(
            script: &EditScript,
            from: impl Into<TestInteger>,
            to: impl Into<TestInteger>,
        ) -> TestDecision
        {
            let from = from.into().0;
            let to = to.into().0;
            script
                .actions
                .iter()
                .any(|action| {
                    matches!(action, Action::SetInt { from: f, to: t, .. } if *f == from && *t == to)
                })
                .into()
        }
        /// Editing a `reset` body localizes (child 0).
        #[test]
        fn reset_body_edit_localizes()
        {
            let script = diff_sound(&item(Comp::reset(ret(1))), &item(Comp::reset(ret(2))));
            assert!(
                has_set_int(&script, 1, 2),
                "reset body localizes: {script:?}"
            );
            assert!(!has_replace(&script), "reset is not replaced: {script:?}");
        }
        /// Editing the second operation clause of a multi-clause handler
        /// localizes to its body (child index 3 = clause 1) — exercising the
        /// `handle_clause_child(k) = k + 2` convention for `k >= 1`.
        #[test]
        fn handle_second_clause_body_edit_localizes()
        {
            let script = diff_sound(
                &item(handler2(ret(1), ret(2), ret(3), ret(4))),
                &item(handler2(ret(1), ret(2), ret(3), ret(7))),
            );
            assert!(
                has_set_int(&script, 4, 7),
                "the put-clause body localizes: {script:?}"
            );
            assert!(
                !has_replace(&script),
                "the handler is not replaced: {script:?}"
            );
        }

        /// Whether any action coarse-replaces a subtree.
        fn has_replace(script: &EditScript) -> TestDecision
        {
            script
                .actions
                .iter()
                .any(|action| matches!(action, Action::Replace { .. }))
                .into()
        }

        /// A two-clause handler over [`state2_sig`].
        fn handler2(
            scrutinee: Comp,
            return_body: Comp,
            get_body: Comp,
            put_body: Comp,
        ) -> Comp
        {
            Comp::handle(state2_sig(), scrutinee, "x", return_body, vec![
                OpClause::new("get", "p", "k", get_body),
                OpClause::new("put", "p", "k", put_body),
            ])
        }

        /// A two-operation signature `State { get, put }` (a signature sorts
        /// its operations by name, so `get` precedes `put`).
        fn state2_sig() -> EffectSig
        {
            EffectSig::new("State".into(), vec![
                EffectOp::new("get".into(), ValueType::Unit, ValueType::integer()),
                EffectOp::new("put".into(), ValueType::integer(), ValueType::Unit),
            ])
        }

        /// A depth-bounded strategy over the A3 effect/control + grade
        /// constructors, built directly (the surface does not lower to them),
        /// for the randomized soundness oracle.
        fn comp_strategy() -> impl Strategy<Value = Comp>
        {
            const PROP_RECURSION_SIZE: u32 = 24;

            let leaf = prop_oneof![
                (0_i64 .. 6).prop_map(ret),
                (0_i64 .. 6).prop_map(|literal| Comp::dup(Value::int(literal))),
                (0_i64 .. 6).prop_map(|literal| Comp::drop(Value::int(literal))),
                (0_i64 .. 6, any::<bool>()).prop_map(|(literal, alt)| {
                    Comp::perform(
                        state_sig(),
                        if alt { "get" } else { "set" },
                        Value::int(literal),
                    )
                }),
            ];
            leaf.prop_recursive(3, PROP_RECURSION_SIZE, 3, |inner| {
                prop_oneof![
                    inner.clone().prop_map(Comp::reset),
                    (prop::sample::select(vec!["j", "k"]), inner.clone())
                        .prop_map(|(binder, body)| Comp::shift(binder, body)),
                    (0_i64 .. 6, inner.clone())
                        .prop_map(|(literal, body)| Comp::resume(Value::int(literal), body)),
                    (inner.clone(), inner.clone(), inner).prop_map(
                        |(scrutinee, return_body, clause)| handler(scrutinee, return_body, clause)
                    ),
                ]
            })
        }

        proptest! {
            /// `apply ∘ diff` reproduces a randomized single-clause handler body
            /// edit — three `ret` literals over a fixed State-handler skeleton,
            /// the multi-child `Handle` descent path. Soundness only (a coarse
            /// replace would also round-trip); localization is pinned by the
            /// deterministic tests above.
            #[test]
            fn handle_body_edits_round_trip(
                old in (0_i64 .. 20, 0_i64 .. 20, 0_i64 .. 20),
                new in (0_i64 .. 20, 0_i64 .. 20, 0_i64 .. 20),
            )
            {
                let old_program = item(handler(ret(old.0), ret(old.1), ret(old.2)));
                let new_program = item(handler(ret(new.0), ret(new.1), ret(new.2)));
                let script = diff(&old_program, &new_program);
                prop_assert!(
                    items_eq_mod_holes(&apply(&old_program.items, &script), &new_program.items),
                    "handler body edit must round-trip; script: {script:?}"
                );
            }

            /// `apply ∘ diff` reproduces an INDEPENDENT pair of randomized
            /// effect/control core terms — nesting and the cross-constructor
            /// coarse-replace path the surface-driven oracle cannot reach.
            #[test]
            fn effect_control_pairs_round_trip(
                old in comp_strategy(),
                new in comp_strategy(),
            )
            {
                let old_program = item(old);
                let new_program = item(new);
                let script = diff(&old_program, &new_program);
                prop_assert!(
                    items_eq_mod_holes(&apply(&old_program.items, &script), &new_program.items),
                    "round-trip over random effect/control terms; script: {script:?}"
                );
            }
        }
    }
}
