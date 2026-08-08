//! The A2.3 differential gate: incremental validated resume ≡ from-scratch
//! re-typing (`incremental-pipeline.md` §§4-6).
//!
//! The theorem `gandr_core_checker::checkpoint::resume` must satisfy: for
//! **every** edit, the incrementally-resumed per-item typing equals the typing
//! a full from-scratch re-type of the edited program produces. Adoption
//! (reusing a validated checkpoint) skips work; this gate proves the skips
//! never change the answer.
//!
//! The front end is a `ToySurface` — an in-tree test double implementing the
//! parser-agnostic [`ItemSource`] seam. It lowers a compact statement model
//! (integer / string literals, name references, and integer-ascribed
//! references) to the same core [`Item`]s a real parser would, so the gate
//! exercises the checkpoint engine without a parser (the surface lane supplies
//! the real one). The four classes below mirror the incremental loop's cases:
//! adoption, invalidation, structural edits, and property-generated edits.
//!
//! [`ItemSource`]: gandr_core_checker::region::ItemSource
//! [`Item`]: gandr_core_checker::region::Item

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code"
    )
)]

/// The differential gate for `gandr_core_checker::checkpoint`.
#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;

    use gandr_core_checker::checkpoint::ItemTyping;
    use gandr_core_checker::checkpoint::Resume;
    use gandr_core_checker::checkpoint::checkpoint_program;
    use gandr_core_checker::checkpoint::resume;
    use gandr_core_checker::region::Item;
    use gandr_core_checker::region::ItemSource;
    use gandr_core_checker::region::Program;
    use gandr_core_checker::syntax::Term;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::types::ValueType;

    /// One definition body the toy surface can lower — enough scalar and
    /// reference shapes to drive body edits, type changes, and downstream
    /// errors without a parser.
    #[derive(Clone, Debug)]
    enum Body
    {
        /// An integer literal `n` (infers the rigid `Integer` atom).
        Int(i64),
        /// A string literal `s` (infers the rigid `String` atom) — the
        /// type-changing counterpart of [`Body::Int`].
        Str(String),
        /// A bare reference to a name — reads it from the ambient context, so
        /// its footprint is `{name}` and its type moves with the referent's.
        Ref(String),
        /// An integer-ascribed reference `(name : Integer)` — types iff `name`
        /// is `Integer`, so a change that retypes `name` to a non-integer makes
        /// this item ill-typed downstream.
        CheckInteger(String),
    }

    /// One top-level statement: an optional definition name plus its body.
    #[derive(Clone, Debug)]
    struct Stmt
    {
        /// The defined name (`def name = …`); `None` for a bare expression.
        name: Option<String>,
        /// The statement body.
        body: Body,
    }

    /// The in-tree test double for the parser-agnostic [`ItemSource`] seam,
    /// standing in for the surface lane's real lowering front end so this gate
    /// runs without one. Its revision is a statement slice, and it has no
    /// input-independent failure residue, so its error is
    /// [`core::convert::Infallible`].
    struct ToySurface;

    impl ItemSource for ToySurface
    {
        type Error = core::convert::Infallible;
        type Revision = [Stmt];

        fn items(
            &self,
            revision: &[Stmt],
        ) -> Result<Program, Self::Error>
        {
            Ok(revision.iter().map(lower_stmt).collect())
        }
    }

    /// The toy front end's items for `revision`, with its uninhabited failure
    /// residue discharged by a total match rather than an unwrap.
    fn items_of(revision: &[Stmt]) -> Program
    {
        match ToySurface.items(revision) {
            | Ok(program) => program,
            | Err(never) => match never {},
        }
    }

    /// Lowers one statement to a core [`Item`] — the toy analogue of the
    /// surface lane's CST → core lowering.
    fn lower_stmt(stmt: &Stmt) -> Item
    {
        let term = match stmt.body {
            | Body::Int(literal) => Term::Value(Value::int(literal)),
            | Body::Str(ref literal) => Term::Value(Value::string(literal)),
            | Body::Ref(ref name) => Term::Value(Value::var(name)),
            | Body::CheckInteger(ref name) => Term::Value(Value::Annot(
                Rc::new(Value::var(name)),
                Rc::new(ValueType::integer()),
            )),
        };
        Item::new(stmt.name.clone(), None, term)
    }

    /// Builds a `def name = body` statement.
    fn def(
        name: impl Into<String>,
        body: Body,
    ) -> Stmt
    {
        Stmt {
            name: Some(name.into()),
            body,
        }
    }

    /// Runs the gate: checkpoints `base`, resumes onto `edited`, and asserts
    /// the resumed typings equal a from-scratch re-type of `edited`.
    /// Returns the resume so a caller can additionally assert *which* items
    /// were adopted.
    fn gate(
        base: &[Stmt],
        edited: &[Stmt],
    ) -> Resume
    {
        let base_program = items_of(base);
        let edited_program = items_of(edited);
        let checkpoints = checkpoint_program(&base_program);
        let resumed = resume(&checkpoints, &edited_program);
        assert_eq!(
            resumed.typings,
            from_scratch(&edited_program),
            "incremental resume must equal from-scratch re-typing"
        );
        assert_eq!(
            resumed.typings.len(),
            resumed.adopted.len(),
            "one adoption flag per typing"
        );
        resumed
    }

    /// The per-item typings a from-scratch re-type of `program` produces.
    fn from_scratch(program: &Program) -> Vec<ItemTyping>
    {
        checkpoint_program(program)
            .items
            .into_iter()
            .map(|checkpoint| checkpoint.typing)
            .collect()
    }

    /// The reuse the trail-aware footprint buys.
    mod adoption
    {
        use super::*;

        /// A body edit that keeps the definition's *type* leaves its dependent
        /// adoptable: `target`'s literal changes but its type does not, so the
        /// dependent `d = target` is **adopted**, not re-typed. This is the
        /// §4/§5 refinement: reuse keyed on whether the binding changed, not on
        /// whether an upstream item was edited.
        #[test]
        fn body_edit_adopts_the_type_stable_dependent()
        {
            let base = [
                def("target", Body::Int(1)),
                def("d", Body::Ref("target".to_owned())),
            ];
            let edited = [
                def("target", Body::Int(2)),
                def("d", Body::Ref("target".to_owned())),
            ];
            let resumed = gate(&base, &edited);

            assert_eq!(2, resumed.adopted.len(), "two items");
            assert!(
                !resumed.adopted[0],
                "the edited definition `target` is re-typed"
            );
            assert!(
                resumed.adopted[1],
                "the type-stable dependent `d = target` is adopted, not re-typed"
            );
        }

        /// Inserting a definition re-types only the insert; the untouched
        /// neighbours (which do not read it) are adopted.
        #[test]
        fn insertion_adopts_untouched_neighbours()
        {
            let base = [def("a", Body::Int(1)), def("c", Body::Int(3))];
            let edited = [
                def("a", Body::Int(1)),
                def("b", Body::Int(2)),
                def("c", Body::Int(3)),
            ];
            let resumed = gate(&base, &edited);

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
            let source = [
                def("a", Body::Int(1)),
                def("b", Body::Ref("a".to_owned())),
                def("c", Body::Ref("b".to_owned())),
            ];
            let resumed = gate(&source, &source);

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
        /// footprint of `y` reads `x`, whose binding changed (Integer ⇒
        /// String), so its checkpoint is invalidated — and the resumed
        /// typing still equals from-scratch.
        #[test]
        fn type_change_retypes_the_dependent()
        {
            let base = [def("x", Body::Int(1)), def("y", Body::Ref("x".to_owned()))];
            let edited = [
                def("x", Body::Str("hi".to_owned())),
                def("y", Body::Ref("x".to_owned())),
            ];
            let resumed = gate(&base, &edited);

            assert!(!resumed.adopted[0], "the edited `x` is re-typed");
            assert!(
                !resumed.adopted[1],
                "the dependent `y` reads the changed binding `x`, so it is re-typed"
            );
            match (&resumed.typings[0], &resumed.typings[1]) {
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
        /// adopting a stale success): `(x : Integer)` types while `x` is
        /// `Integer`, and errors once `x` becomes `String`.
        #[test]
        fn downstream_error_surfaces()
        {
            let base = [
                def("x", Body::Int(1)),
                def("y", Body::CheckInteger("x".to_owned())),
            ];
            let edited = [
                def("x", Body::Str("hi".to_owned())),
                def("y", Body::CheckInteger("x".to_owned())),
            ];
            let resumed = gate(&base, &edited);

            assert!(
                !resumed.adopted[1],
                "the dependent `y` is re-typed against the changed `x`"
            );
            assert!(
                matches!(resumed.typings[1], ItemTyping::TypeError { .. }),
                "the ascription `(x : Integer)` fails once `x` is a String: {:?}",
                resumed.typings[1]
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
            let base = [
                def("a", Body::Int(1)),
                def("b", Body::Int(2)),
                def("c", Body::Int(3)),
            ];
            let edited = [def("a", Body::Int(1)), def("c", Body::Int(3))];
            let resumed = gate(&base, &edited);

            assert_eq!(resumed.adopted, vec![true, true], "both survivors reused");
        }

        /// Renaming a definition is a delete-plus-insert: the renamed item is
        /// fresh, and a definition it does not read is adopted. The gate holds
        /// regardless.
        #[test]
        fn rename_matches_from_scratch()
        {
            let base = [def("foo", Body::Int(1)), def("keep", Body::Int(9))];
            let edited = [def("bar", Body::Int(1)), def("keep", Body::Int(9))];
            let resumed = gate(&base, &edited);

            assert!(resumed.adopted[1], "`keep` is adopted across the rename");
        }
    }

    /// The gate over property-generated random edits.
    mod property
    {
        use proptest::prelude::*;

        use super::*;

        /// An edit applied to a statement list.
        #[derive(Clone, Debug)]
        enum Edit
        {
            /// Replace the statement at the index with a fresh one.
            Replace(usize, Stmt),
            /// Insert a fresh statement before the index.
            Insert(usize, Stmt),
            /// Delete the statement at the index.
            Delete(usize),
        }

        /// A program of one to six statements.
        fn program() -> impl Strategy<Value = Vec<Stmt>>
        {
            proptest::collection::vec(statement(), 1_usize .. 7_usize)
        }

        /// One statement: `def d{index} = {body}`. The name pool (`d0..d5`)
        /// deliberately allows repeats and forward references, exercising the
        /// engine's conservative handling of both.
        fn statement() -> impl Strategy<Value = Stmt>
        {
            (0_usize .. 6_usize, body()).prop_map(|(index, body)| Stmt {
                name: Some(format!("d{index}")),
                body,
            })
        }

        /// One definition body: an integer literal, a name reference, an
        /// integer-ascribed reference, or a string literal (so edits can change
        /// a definition's type).
        fn body() -> impl Strategy<Value = Body>
        {
            prop_oneof![
                (0_i64 .. 20_i64).prop_map(Body::Int),
                (0_usize .. 6_usize).prop_map(|index| Body::Ref(format!("d{index}"))),
                (0_usize .. 6_usize).prop_map(|index| Body::CheckInteger(format!("d{index}"))),
                "[a-z]{0,3}".prop_map(Body::Str),
            ]
        }

        /// A program paired with a random edit sized to it.
        fn program_and_edit() -> impl Strategy<Value = (Vec<Stmt>, Edit)>
        {
            program().prop_flat_map(|statements| {
                let length = statements.len();
                let replace =
                    (0_usize .. length, statement()).prop_map(|(at, s)| Edit::Replace(at, s));
                let insert =
                    (0_usize ..= length, statement()).prop_map(|(at, s)| Edit::Insert(at, s));
                let delete = (0_usize .. length).prop_map(Edit::Delete);
                (Just(statements), prop_oneof![replace, insert, delete])
            })
        }

        /// Applies `edit` to a clone of `statements`, returning the edited
        /// list.
        fn apply_edit(
            statements: &[Stmt],
            edit: &Edit,
        ) -> Vec<Stmt>
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
                let base = items_of(&statements);
                let edited_statements = apply_edit(&statements, &concrete);
                let edited = items_of(&edited_statements);

                let checkpoints = checkpoint_program(&base);
                let resumed = resume(&checkpoints, &edited);

                let expected = from_scratch(&edited);
                prop_assert_eq!(resumed.typings, expected);
            }
        }
    }
}
