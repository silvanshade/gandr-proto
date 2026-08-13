//! The differential gate over **real surface source**: incremental validated
//! resume ≡ from-scratch re-typing (`incremental-pipeline.md` §"Checkpoints and
//! the reuse rule" through §"Derivation merging and identity stability").
//!
//! `gandr-core-incremental` gates its engine against an in-tree item-source
//! double; this is the same theorem driven through *this* crate's front end —
//! the melder push machine and the CST → core lowering, crossed at
//! [`LoweringItemSource`] — and against the surface [`prelude_ctx`] rather than
//! the engine's empty default base. For **every** edit, the incrementally
//! resumed per-item typing equals the typing a full from-scratch re-type of the
//! edited program produces. Adoption (reusing a validated checkpoint) skips
//! work; this gate proves the skips never change the answer.
//!
//! Four classes:
//!
//! 1. [`tests::adoption`] — the reuse the trail-aware footprint buys: a
//!    body-only edit adopts its *type-stable* dependent (the model
//!    `incremental-base` ⇄ `incremental-edited` shape), an insertion adopts its
//!    untouched neighbours, and a no-op edit adopts everything.
//! 2. [`tests::invalidation`] — the re-typing a real dependency change forces:
//!    a type-changing edit re-types every downstream reader, and a downstream
//!    type error surfaces exactly as from-scratch.
//! 3. [`tests::structure`] — item-list edits (delete, rename) match from
//!    scratch.
//! 4. [`tests::property`] — the gate over property-generated random edits
//!    (replace / insert / delete) on chained-definition programs: `resume_with`
//!    equals `checkpoint_with` unconditionally.
//!
//! [`LoweringItemSource`]: gandr_surface_engine::item_source::LoweringItemSource
//! [`prelude_ctx`]: gandr_surface_engine::prelude_ctx

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// The differential gate for `gandr_core_incremental::checkpoint`, driven
/// through this crate's item seam against the surface prelude.
#[cfg(test)]
mod tests
{
    use gandr_core_incremental::checkpoint::ItemTyping;
    use gandr_core_incremental::checkpoint::Resume;
    use gandr_core_incremental::checkpoint::checkpoint_with;
    use gandr_core_incremental::checkpoint::resume_with;
    use gandr_core_incremental::region::ItemSource as _;
    use gandr_core_incremental::region::Program;
    use gandr_surface_engine::item_source::LoweringItemSource;
    use gandr_surface_engine::item_source::SourceRevision;
    use gandr_surface_engine::prelude_ctx;

    use crate::common::TestText;

    /// Runs the gate: checkpoints `base`, resumes onto `edited`, and asserts
    /// the resumed typings equal a from-scratch re-type of `edited`.
    /// Returns the resume so a caller can additionally assert *which* items
    /// were adopted.
    fn gate<'base, 'edited>(
        base_source: impl Into<TestText<'base>>,
        edited_source: impl Into<TestText<'edited>>,
    ) -> Resume
    {
        let base_source = base_source.into().0;
        let edited_source = edited_source.into().0;
        let base = checkpoint_with(&seam_program(base_source), &prelude_ctx());
        let edited = seam_program(edited_source);
        let resumed = resume_with(&base, &edited, &prelude_ctx());
        let resumed_typings: Vec<ItemTyping> = resumed.typings().cloned().collect();
        assert_eq!(
            resumed_typings,
            from_scratch(edited_source),
            "incremental resume must equal from-scratch re-typing\n base:   {base_source:?}\n \
             edited: {edited_source:?}"
        );
        assert_eq!(
            resumed.typings().len(),
            resumed.adopted.len(),
            "one adoption flag per typing"
        );
        resumed
    }

    /// The per-item typings a from-scratch re-type of `source` produces.
    fn from_scratch<'text>(source: impl Into<TestText<'text>>) -> Vec<ItemTyping>
    {
        let source = source.into().0;
        checkpoint_with(&seam_program(source), &prelude_ctx())
            .items
            .into_iter()
            .map(|checkpoint| checkpoint.typing)
            .collect()
    }

    /// Crosses `source` through the melder-and-lowering front end at the item
    /// seam (lowering is total, so out-of-fragment regions become holes).
    fn seam_program<'text>(source: impl Into<TestText<'text>>) -> Program
    {
        let source = source.into().0;
        LoweringItemSource
            .items(&SourceRevision::from(source))
            .expect("total lowering fails only when the parser is unavailable")
    }

    /// The reuse the trail-aware footprint buys.
    mod adoption
    {
        use super::*;

        /// The model edit (`incremental-base` ⇄ `incremental-edited`): the
        /// literal inside `target` changes, but `target`'s *type* does not — so
        /// the dependent `print(target)` is **adopted**, not re-typed. This is
        /// the §"Checkpoints and the reuse rule" / §"The edit loop" refinement:
        /// reuse keyed on whether the binding changed, not on whether an
        /// upstream item was edited.
        #[test]
        fn body_edit_adopts_the_type_stable_dependent()
        {
            let base = "def target(x: Integer) -> F Integer {\n  ret (x + 1)\n}\nprint(target)\n";
            let edited = "def target(x: Integer) -> F Integer {\n  ret (x + 2)\n}\nprint(target)\n";
            let resumed = gate(base, edited);

            assert_eq!(2, resumed.adopted.len(), "two items");
            assert!(
                !resumed.adopted[0],
                "the edited definition `target` is re-typed"
            );
            assert!(
                resumed.adopted[1],
                "the type-stable dependent `print(target)` is adopted, not re-typed"
            );
        }

        /// Inserting a definition re-types only the insert; the untouched
        /// neighbours (which do not read it) are adopted.
        #[test]
        fn insertion_adopts_untouched_neighbours()
        {
            let base = "def a = 1;\ndef c = 3;\n";
            let edited = "def a = 1;\ndef b = 2;\ndef c = 3;\n";
            let resumed = gate(base, edited);

            assert_eq!(
                resumed.adopted,
                vec![true, false, true],
                "only `b` is fresh"
            );
            assert_eq!(
                2,
                usize::from(resumed.adopted_count()),
                "`a` and `c` reused"
            );
        }

        /// A no-op edit (identical source) adopts every item.
        #[test]
        fn noop_edit_adopts_everything()
        {
            let source = "def a = 1;\ndef b = a;\ndef c = b;\n";
            let resumed = gate(source, source);

            assert!(
                resumed.adopted.iter().all(|&adopted| adopted),
                "an identity edit reuses everything: {:?}",
                resumed.adopted
            );
        }
    }

    /// The re-typing a real dependency change forces.
    mod invalidation
    {
        use super::*;

        /// A type-changing edit to `x` re-types the downstream reader `y`: the
        /// footprint of `y` reads `x`, whose binding changed, so its checkpoint
        /// is invalidated — and the resumed typing still equals from-scratch.
        #[test]
        fn type_change_retypes_the_dependent()
        {
            let base = "def x = 1;\ndef y = x;\n";
            let edited = "def x = \"hi\";\ndef y = x;\n";
            let resumed = gate(base, edited);

            assert!(!resumed.adopted[0], "the edited `x` is re-typed");
            assert!(
                !resumed.adopted[1],
                "the dependent `y` reads the changed binding `x`, so it is re-typed"
            );
            // The dependency really did change type (Integer ⇒ String), so the
            // downstream reader's type moved with it — the gate confirms the
            // incremental path tracked that move.
            let typings: Vec<&ItemTyping> = resumed.typings().collect();
            match (typings[0], typings[1]) {
                | (
                    &ItemTyping::Definition {
                        name: ref x_name, ..
                    },
                    &ItemTyping::Definition {
                        name: ref y_name, ..
                    },
                ) => {
                    assert_eq!("x", x_name);
                    assert_eq!("y", y_name);
                },
                | other => panic!("expected two definitions, got {other:?}"),
            }
        }

        /// An edit that makes a downstream item ill-typed surfaces the error
        /// exactly as from-scratch would (the resume never masks a new error by
        /// adopting a stale success).
        #[test]
        fn downstream_error_surfaces()
        {
            // `y` reads `x` in an integer addition; retyping `x` to a string
            // makes `x + 1` ill-typed downstream.
            let base = "def x = 1;\ndef y = x + 1;\n";
            let edited = "def x = \"hi\";\ndef y = x + 1;\n";
            let resumed = gate(base, edited);

            assert!(
                !resumed.adopted[1],
                "the dependent `y` is re-typed against the changed `x`"
            );
        }
    }

    /// Item-list edits.
    mod structure
    {
        use super::*;

        /// Deleting a definition matches from-scratch, and the survivors that
        /// do not read it are adopted.
        #[test]
        fn deletion_matches_from_scratch()
        {
            let base = "def a = 1;\ndef b = 2;\ndef c = 3;\n";
            let edited = "def a = 1;\ndef c = 3;\n";
            let resumed = gate(base, edited);

            assert_eq!(resumed.adopted, vec![true, true], "both survivors reused");
        }

        /// Renaming a definition is a delete-plus-insert: the renamed item is
        /// fresh, and a definition it does not read is adopted. The gate holds
        /// regardless.
        #[test]
        fn rename_matches_from_scratch()
        {
            let base = "def foo = 1;\ndef keep = 9;\n";
            let edited = "def bar = 1;\ndef keep = 9;\n";
            let resumed = gate(base, edited);

            // `keep` does not read `foo`/`bar`, so it is adopted; `bar` is a
            // fresh insertion.
            assert!(resumed.adopted[1], "`keep` is adopted across the rename");
        }
    }

    /// The gate over property-generated random edits.
    mod property
    {
        use gandr_core_incremental::checkpoint::checkpoint_with;
        use gandr_core_incremental::checkpoint::resume_with;
        use gandr_surface_engine::prelude_ctx;

        use super::seam_program;
        use crate::common::TestCount;
        use crate::proptest_crate::collection::vec;
        use crate::proptest_crate::prelude::*;

        /// A program paired with a random edit sized to it.
        fn program_and_edit() -> impl Strategy<Value = (Vec<String>, Edit)>
        {
            program().prop_flat_map(|statements| {
                let length = statements.len();
                (Just(statements), edit(length))
            })
        }

        /// A program of one to six statements.
        fn program() -> impl Strategy<Value = Vec<String>>
        {
            vec(statement(), 1_usize .. 7_usize)
        }

        /// A random edit over a statement list of `len` statements.
        fn edit(len: impl Into<TestCount>) -> impl Strategy<Value = Edit>
        {
            let len = len.into().0;
            let replace = (0_usize .. len, statement()).prop_map(|(at, s)| Edit::Replace(at, s));
            let insert = (0_usize ..= len, statement()).prop_map(|(at, s)| Edit::Insert(at, s));
            let delete = (0_usize .. len).prop_map(Edit::Delete);
            prop_oneof![replace, insert, delete]
        }

        /// One statement: `def d{index} = {body};`. The name pool (`d0..d5`)
        /// deliberately allows repeats and forward references, exercising the
        /// engine's conservative handling of both.
        fn statement() -> impl Strategy<Value = String>
        {
            (0_usize .. 6_usize, body()).prop_map(|(index, body)| format!("def d{index} = {body};"))
        }

        /// One definition body: an integer literal, a reference to a name in
        /// the pool, or an integer addition of a reference and a
        /// literal.
        fn body() -> impl Strategy<Value = String>
        {
            prop_oneof![
                (0_u64 .. 20_u64).prop_map(|literal| literal.to_string()),
                (0_usize .. 6_usize).prop_map(|index| format!("d{index}")),
                (0_usize .. 6_usize, 0_u64 .. 20_u64)
                    .prop_map(|(index, literal)| format!("d{index} + {literal}")),
            ]
        }

        /// An edit applied to a statement list: replace a body, insert a
        /// statement, or delete one.
        #[derive(Clone, Debug)]
        enum Edit
        {
            /// Replace the statement at the index with a fresh one.
            Replace(usize, String),
            /// Insert a fresh statement before the index.
            Insert(usize, String),
            /// Delete the statement at the index.
            Delete(usize),
        }

        /// Applies `edit` to a clone of `statements`, returning the edited
        /// list.
        fn apply_edit(
            statements: &[String],
            edit: &Edit,
        ) -> Vec<String>
        {
            let mut edited = statements.to_vec();
            match *edit {
                | Edit::Replace(at, ref statement) => {
                    if let Some(slot) = edited.get_mut(at) {
                        slot.clone_from(statement);
                    }
                },
                | Edit::Insert(at, ref statement) => {
                    edited.insert(at.min(edited.len()), statement.clone());
                },
                | Edit::Delete(at) => {
                    if at < edited.len() {
                        let _removed = edited.remove(at);
                    }
                },
            }
            edited
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(400))]

            /// The gate over arbitrary programs and edits: the incrementally
            /// resumed typings always equal a from-scratch re-type of the edited
            /// program — even when the edit introduces type errors, holes, name
            /// collisions, or dangling references.
            #[test]
            fn incremental_equals_from_scratch((statements, concrete) in program_and_edit()) {
                let base_source = statements.join("\n");
                let edited_source = apply_edit(&statements, &concrete).join("\n");

                let base = checkpoint_with(&seam_program(base_source.as_str()), &prelude_ctx());
                let edited = seam_program(edited_source.as_str());
                let resumed = resume_with(&base, &edited, &prelude_ctx());

                let expected: Vec<_> = checkpoint_with(&edited, &prelude_ctx())
                    .items
                    .into_iter()
                    .map(|checkpoint| checkpoint.typing)
                    .collect();
                let actual: Vec<_> = resumed.typings().cloned().collect();
                prop_assert_eq!(
                    actual,
                    expected,
                    "resume != from-scratch\n base:   {:?}\n edited: {:?}",
                    base_source,
                    edited_source
                );
            }
        }
    }
}
