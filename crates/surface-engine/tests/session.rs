//! REPL session engine tests: the read → lower → type →
//! eval → result slice — literal evaluation, cross-line definition carry-over,
//! the fixed-table operator evaluation, the annotation story for check-only
//! forms, and the decline-eval-on-holes validator.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// REPL session engine tests.
#[cfg(test)]
mod tests
{
    use gandr_core_checker::outcome::Blame;
    use gandr_core_checker::outcome::Eval;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::types::CompType;
    use gandr_core_checker::types::Ty;
    use gandr_core_checker::types::ValueType;
    use gandr_core_incremental::boundary::AdoptionDecision;
    use gandr_core_incremental::stream::SynthesisEvent;
    use gandr_surface_engine::namespace::DottedName;
    use gandr_surface_engine::namespace::NamePath;
    use gandr_surface_engine::session::ItemOutcome;
    use gandr_surface_engine::session::Session;
    use gandr_surface_engine::session::Submission;

    use crate::common::TestDecision;
    use crate::common::TestText;
    /// Later items in a single whole-file submission type against definitions
    /// that earlier items bound, so the report and outcomes agree.
    #[test]
    fn whole_file_submit_carries_definitions_forward()
    {
        let mut session = Session::new();
        let submission = session
            .submit("def greeting = \"the value zone\"; ret greeting")
            .expect("lowering must not fail");
        assert_clean_submission(&submission);
        let Some((first, rest)) = submission.outcomes.split_first()
        else {
            panic!("expected definition outcome for `greeting`, got []");
        };
        let Some((second, rest)) = rest.split_first()
        else {
            panic!(
                "expected expression outcome after `greeting`, got {:?}",
                submission.outcomes
            );
        };
        assert!(
            rest.is_empty(),
            "expected exactly two outcomes, got {:?}",
            submission.outcomes
        );
        match *first {
            | ItemOutcome::Definition {
                ref name,
                ref ty,
                bound,
            } => {
                assert_eq!("greeting", name, "the first item binds `greeting`");
                assert_eq!(ty, &Ty::Value(ValueType::string()), "`greeting : String`");
                assert!(bound, "`greeting` enters scope for the later item");
            },
            | ref outcome => panic!("first item should define `greeting`, got {outcome:?}"),
        }
        match *second {
            | ItemOutcome::Expression {
                ty: ref ret_ty,
                ref value,
            } => {
                assert_eq!(
                    ret_ty,
                    &Ty::Comp(CompType::returner(ValueType::string())),
                    "`ret greeting : F String`"
                );
                assert_eq!(
                    value,
                    &Eval::Value(Comp::ret(Value::string("the value zone"))),
                    "`ret greeting` evaluates through the same submission"
                );
            },
            | ref outcome => panic!("second item should return `greeting`, got {outcome:?}"),
        }
    }

    /// Appending a submission through the session checkpoint state yields the
    /// same item outcomes as typing and evaluating the accumulated source from
    /// scratch in one submission.
    #[test]
    fn checkpointed_session_matches_from_scratch()
    {
        let mut checkpointed = Session::new();
        let definitions = checkpointed
            .submit("def x = 40; def y = x + 2")
            .expect("definition lowering must not fail");
        assert_clean_submission(&definitions);
        let expression = checkpointed
            .submit("ret y")
            .expect("expression lowering must not fail");
        assert_clean_submission(&expression);

        let mut resumed_outcomes = definitions.outcomes;
        resumed_outcomes.extend(expression.outcomes);

        let mut from_scratch = Session::new();
        let full = from_scratch
            .submit("def x = 40; def y = x + 2; ret y")
            .expect("from-scratch lowering must not fail");
        assert_clean_submission(&full);
        assert_eq!(
            resumed_outcomes, full.outcomes,
            "checkpointed append must preserve the from-scratch outcomes"
        );
    }

    /// Successful submissions publish owned whole-program streams with stable
    /// boundaries, global order, and per-item adoption decisions.
    #[test]
    fn successful_submissions_publish_whole_program_synthesis()
    {
        let mut session = Session::new();
        assert!(
            session.latest_synthesis_stream().is_none(),
            "a fresh session has no successful synthesis"
        );

        let _first_submission = session
            .submit("def retained = 40")
            .expect("first definition lowering must succeed");
        let first_stream = session
            .latest_synthesis_stream()
            .expect("first successful submission publishes a stream");

        let _second_submission = session
            .submit("ret retained")
            .expect("appended expression lowering must succeed");
        let first_events = first_stream.collect::<Vec<SynthesisEvent>>();
        let &[
            SynthesisEvent::Started {
                item_count: first_count,
            },
            SynthesisEvent::Item {
                index: first_index,
                typing: ref first_typing,
                adopted: first_adopted,
            },
            SynthesisEvent::Completed,
        ] = first_events.as_slice()
        else {
            panic!("first stream must contain Started, one Item, Completed: {first_events:?}");
        };
        assert_eq!(
            first_count, 1,
            "first stream covers the complete one-item program"
        );
        assert_eq!(first_index, 0, "first item has global source index zero");
        assert_eq!(
            first_adopted,
            AdoptionDecision::from(false),
            "the first submission recomputes its only item"
        );

        let second_events = session
            .latest_synthesis_stream()
            .expect("second successful submission replaces the latest stream")
            .collect::<Vec<SynthesisEvent>>();
        let &[
            SynthesisEvent::Started {
                item_count: second_count,
            },
            SynthesisEvent::Item {
                index: retained_index,
                typing: ref retained_typing,
                adopted: retained_adopted,
            },
            SynthesisEvent::Item {
                index: appended_index,
                adopted: appended_adopted,
                ..
            },
            SynthesisEvent::Completed,
        ] = second_events.as_slice()
        else {
            panic!(
                "second stream must contain Started, two ordered Items, Completed: \
                 {second_events:?}"
            );
        };
        assert_eq!(
            second_count, 2,
            "second stream covers both accumulated program items"
        );
        assert_eq!(
            retained_index, 0,
            "retained definition remains globally first"
        );
        assert_eq!(
            retained_typing, first_typing,
            "adoption preserves the retained definition's validated typing"
        );
        assert_eq!(
            retained_adopted,
            AdoptionDecision::from(true),
            "the unchanged prior definition is adopted"
        );
        assert_eq!(
            appended_index, 1,
            "new expression follows in global source order"
        );
        assert_eq!(
            appended_adopted,
            AdoptionDecision::from(false),
            "the newly appended expression is recomputed"
        );
    }
    /// A submission owns its outcomes after the session advances, so callers
    /// can drain an earlier result without observing later session mutations.
    #[test]
    fn submission_owns_outcomes_after_session_advances()
    {
        let mut session = Session::new();
        let first = session
            .submit("def retained = 40")
            .expect("definition lowering must not fail");
        let second = session
            .submit("ret retained")
            .expect("expression lowering must not fail");

        assert_eq!(
            1,
            first.outcomes.len(),
            "the first submission remains intact"
        );
        assert_eq!(
            1,
            second.outcomes.len(),
            "the second submission is independent"
        );
        assert!(
            matches!(
                first.outcomes.first(),
                Some(ItemOutcome::Definition { name, bound: true, .. })
                    if name == "retained"
            ),
            "the first submission retains its owned definition outcome"
        );
    }

    /// One-part function sugar pushes the declared `F Integer` result type into
    /// a check-only `case` body, then the definition evaluates normally.
    #[test]
    fn one_part_case_bodied_function_checks_and_evaluates()
    {
        let mut session = Session::new();
        let definition = session
            .submit(
                "def bump(v: Integer + String) -> F Integer { \
                    case v { Inl(x) => ret (x + 1), Inr(s) => ret 0 } \
                }",
            )
            .expect("lowering must not fail");
        assert_clean_submission(&definition);
        assert!(
            matches!(
                definition.outcomes.as_slice(),
                [ItemOutcome::Definition { name, bound: true, .. }] if name.as_str() == "bump"
            ),
            "`bump` should bind as a callable definition: {:?}",
            definition.outcomes
        );

        match sole_outcome(&mut session, "bump((Inl(41) : Integer + String))") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(ValueType::integer())),
                    "`bump(...) : F Integer`"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(42))),
                    "the `Inl` case branch evaluates"
                );
            },
            | outcome => panic!("`bump(...)` should evaluate, got {outcome:?}"),
        }
    }

    /// A checked sum stored by one submission remains consumable after the
    /// L machine erases its runtime annotation and the session re-embeds it.
    #[test]
    fn erased_sum_definition_is_consumed_by_a_later_case()
    {
        let mut session = Session::new();
        let definition = session
            .submit("def chosen = (Inl(41) : Integer + String)")
            .expect("lowering must not fail");
        assert_clean_submission(&definition);

        match sole_outcome(
            &mut session,
            "(case chosen { Inl(x) => ret x, Inr(s) => ret 0 } : F Integer)",
        ) {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    Ty::Comp(CompType::returner(ValueType::integer())),
                    ty,
                    "the later case retains the stored definition's checked sum type"
                );
                assert_eq!(
                    Eval::Value(Comp::ret(Value::int(41))),
                    value,
                    "the later case consumes the erased Inl payload"
                );
            },
            | outcome => panic!("the later case should evaluate, got {outcome:?}"),
        }
    }

    /// Asserts that a submission has no type diagnostics and no hole goals.
    fn assert_clean_submission(submission: &Submission)
    {
        assert!(
            submission.report.diagnostics.is_empty(),
            "expected no diagnostics, got {:?}",
            submission.report.diagnostics
        );
        assert!(
            submission.report.goals.is_empty(),
            "expected no goals, got {:?}",
            submission.report.goals
        );
    }
    /// An integer literal types to `Integer` and evaluates to itself.
    #[test]
    fn integer_literal_types_and_evaluates()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "42") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(ty, Ty::Value(ValueType::integer()), "`42 : Integer`");
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(42))),
                    "`42` evaluates to itself"
                );
            },
            | _ => panic!("`42` should be an expression outcome"),
        }
    }
    /// String and suffixed numeric literals carry their rigid atom types.
    #[test]
    fn scalar_literals_carry_their_types()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "\"hi\"") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(ty, Ty::Value(ValueType::string()), "a string literal");
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::string("hi"))),
                    "the string value"
                );
            },
            | _ => panic!("a string literal should evaluate"),
        }
        match sole_outcome(&mut session, "8080u32") {
            | ItemOutcome::Expression { ty, .. } => {
                assert_eq!(
                    ty,
                    Ty::Value(ValueType::u32()),
                    "a suffixed numeric literal"
                );
            },
            | _ => panic!("a numeric literal should evaluate"),
        }
    }
    /// A value-typed `def` is checkpointed and carried to later lines, where it
    /// both types and evaluates.
    #[test]
    fn definitions_carry_across_lines()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "def y = 5") {
            | ItemOutcome::Definition { name, ty, bound } => {
                assert_eq!("y", name, "the definition binds `y`");
                assert_eq!(ty, Ty::Value(ValueType::integer()), "`y : Integer`");
                assert!(bound, "a value-typed definition enters scope");
            },
            | _ => panic!("`def y = 5` should be a definition outcome"),
        }
        match sole_outcome(&mut session, "y") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Value(ValueType::integer()),
                    "`y : Integer` on a later line"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(5))),
                    "`y` evaluates through the session prelude"
                );
            },
            | _ => panic!("`y` should evaluate via the carried definition"),
        }
        match sole_outcome(&mut session, "(y, y)") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::pair(Value::int(5), Value::int(5)))),
                    "`(y, y)` substitutes the definition twice"
                );
            },
            | _ => panic!("`(y, y)` should evaluate"),
        }
    }
    /// A nullary function definition binds a callable thunk, and `step()`
    /// forces and applies it without the old `Unit`-argument workaround.
    #[test]
    fn nullary_function_call_evaluates()
    {
        let mut session = Session::new();
        let definition = session
            .submit("def step() -> F Integer { ret 1 }")
            .expect("lowering must not fail");
        assert_clean_submission(&definition);
        assert!(
            matches!(
                definition.outcomes.as_slice(),
                [ItemOutcome::Definition { name, bound: true, .. }] if name.as_str() == "step"
            ),
            "`step` should bind as a callable definition: {:?}",
            definition.outcomes
        );

        match sole_outcome(&mut session, "step()") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(ValueType::integer())),
                    "`step() : F Integer`"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(1))),
                    "`step()` evaluates the nullary body"
                );
            },
            | outcome => panic!("`step()` should evaluate, got {outcome:?}"),
        }
    }
    /// A prelude arithmetic operator evaluates through the native prelude and
    /// uses the gradual arithmetic type (`F Unknown`).
    #[test]
    fn arithmetic_operators_type_check_and_evaluate()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "1 + 2") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(ValueType::Unknown)),
                    "`1 + 2` has the gradual arithmetic type"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(3))),
                    "`1 + 2` evaluates to `ret 3`"
                );
            },
            | _ => panic!("`1 + 2` should type-check and evaluate"),
        }
    }
    /// List concat accepts list operands and returns a list with the gradual
    /// element type.
    #[test]
    fn list_concat_type_checks_and_evaluates()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "[1] ++ [2]") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(ValueType::list(ValueType::Unknown))),
                    "`[1] ++ [2]` has gradual list type"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::list(vec![Value::int(1), Value::int(2)]))),
                    "`[1] ++ [2]` evaluates to the concatenated list"
                );
            },
            | _ => panic!("`[1] ++ [2]` should type-check and evaluate"),
        }
    }
    /// A definition whose body uses an operator carries through the session
    /// prelude and remains usable by later expressions.
    #[test]
    fn operator_definition_carries_across_lines()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "def z = 1 + 2") {
            | ItemOutcome::Definition { ty, bound, .. } => {
                assert_eq!(
                    Ty::Value(ValueType::Unknown),
                    ty,
                    "`z` is usable at the gradual arithmetic type"
                );
                assert!(bound, "`z` enters the typing scope");
            },
            | _ => panic!("`def z = 1 + 2` should be a definition"),
        }
        match sole_outcome(&mut session, "z") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(3))),
                    "an operator-backed definition evaluates through the session prelude"
                );
            },
            | _ => panic!("`z` should evaluate"),
        }
    }
    /// A module-qualified native builtin types (its declared type, applied) and
    /// evaluates through the prelude binding-environment: `prim.id(5)` is
    /// `F Integer` and
    /// reduces to `5`; `prim.const(7, 9)` reduces to its first argument
    /// `7`. The end-to-end seam — module-select elaboration (`prim.id` ⇒
    /// `Var("prim.id")`), the native node, and the eval prelude — is the
    /// same native-prelude path the gradual-arithmetic operators use.
    #[test]
    fn module_builtins_type_and_evaluate()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "prim.id(5)") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(ValueType::integer())),
                    "`prim.id(5) : F Integer`"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(5))),
                    "`prim.id(5)` evaluates to 5 through the prelude"
                );
            },
            | _ => panic!("`prim.id(5)` should be an expression outcome"),
        }
        match sole_outcome(&mut session, "prim.const(7, 9)") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(7))),
                    "`prim.const(7, 9)` returns its first argument (currying through the native node)"
                );
            },
            | _ => panic!("`prim.const(7, 9)` should evaluate"),
        }
    }
    /// An unbound variable is a typing error (carried in the report).
    #[test]
    fn an_unbound_variable_is_a_type_error()
    {
        let mut session = Session::new();
        assert!(
            matches!(
                sole_outcome(&mut session, "foo"),
                ItemOutcome::TypeError { .. }
            ),
            "`foo` is unbound"
        );
    }
    /// A list in inference position is stuck; an annotated list checks and
    /// evaluates (the check-only-forms story, list-former design D3).
    #[test]
    fn lists_need_an_annotation_then_evaluate()
    {
        let mut session = Session::new();
        assert!(
            matches!(
                sole_outcome(&mut session, "[1, 2, 3]"),
                ItemOutcome::TypeError { .. }
            ),
            "an unannotated list cannot infer its element type"
        );
        match sole_outcome(&mut session, "([1, 2, 3] : List(Integer))") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Value(ValueType::list(ValueType::integer())),
                    "`: List(Integer)`"
                );
                // The terminal carries the surface annotation (`ret (… : A)`);
                // the renderer strips it (the CLI test checks the `[1, 2, 3]`
                // surface). Here we only require a terminal value.
                assert!(
                    matches!(value, Eval::Value(_)),
                    "the annotated list evaluates to a terminal value"
                );
            },
            | _ => panic!("an annotated list should evaluate"),
        }
    }
    /// `list.each` maps a pure closure over a list through the module prelude:
    /// `list.each(thunk { fn(x) { x + 1 } }, [1, 2, 3])` types to
    /// `F (List ?)` and evaluates to `[2, 3, 4]` — the
    /// closure argument, the list argument, and the native-combinator seam
    /// end to end.
    #[test]
    fn list_each_maps_a_closure_over_a_list()
    {
        let mut session = Session::new();
        match sole_outcome(
            &mut session,
            "list.each(thunk { fn(x) { x + 1 } }, [1, 2, 3])",
        ) {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(ty, list_returner(), "`list.each(...) : F (List ?)`");
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::list(vec![
                        Value::int(2),
                        Value::int(3),
                        Value::int(4),
                    ]))),
                    "`list.each` increments each element"
                );
            },
            | _ => panic!("`list.each(...)` should type-check and evaluate"),
        }
    }
    /// `list.reduce` left-folds with a binary closure `fn(acc, x) { … }`.
    #[test]
    fn list_reduce_folds_a_list()
    {
        let mut session = Session::new();
        match sole_outcome(
            &mut session,
            "list.reduce(thunk { fn(acc, x) { acc + x } }, 0, [1, 2, 3])",
        ) {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(ValueType::Unknown)),
                    "`list.reduce(...) : F ?`"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(6))),
                    "`list.reduce((+), 0, [1,2,3])` is 6"
                );
            },
            | _ => panic!("`list.reduce(...)` should type-check and evaluate"),
        }
    }
    /// `record.get` returns an `Optional` cell and `record.insert` extends a
    /// record — the dynamic-label access the static projection cannot express.
    #[test]
    fn record_get_and_insert_evaluate()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "record.get(#{a = 1, b = 2}, \"a\")") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(ValueType::sum(
                        ValueType::Unknown,
                        ValueType::Unit
                    ))),
                    "`record.get(...) : F (? + 1)`"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::inj1(Value::int(1)))),
                    "`record.get` returns `Some 1` after the L machine erases the checked sum annotation"
                );
            },
            | _ => panic!("`record.get(...)` should evaluate"),
        }
        match sole_outcome(&mut session, "record.insert(#{a = 1}, \"b\", 2)") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::record([
                        ("a".to_owned(), Value::int(1)),
                        ("b".to_owned(), Value::int(2)),
                    ]))),
                    "`record.insert` extends the record"
                );
            },
            | _ => panic!("`record.insert(...)` should evaluate"),
        }
    }
    /// Functional record update `#{ r | ℓ = v }` rebuilds a **fresh** record
    /// (value-semantics MVP, `proposal-value-semantics-mvp.md` §3.1): field
    /// replacement overrides in place, field extension widens, and the base
    /// binding observes no change (the `record-update state-visibility
    /// invariant` state-visibility red line).
    #[test]
    fn record_update_rebuilds_a_fresh_record()
    {
        let mut session = Session::new();
        // Establish the base record binding.
        let _base = sole_outcome(&mut session, "def r = #{ x = 1, y = 2 };");
        // Field replacement: `x` overridden, `y` preserved.
        match sole_outcome(&mut session, "#{ r | x = 9 }") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::record([
                        ("x".to_owned(), Value::int(9)),
                        ("y".to_owned(), Value::int(2)),
                    ]))),
                    "field replacement rebuilds a fresh record"
                );
            },
            | _ => panic!("`#{{ r | x = 9 }}` should evaluate"),
        }
        // Field extension: a new label widens the closed record.
        match sole_outcome(&mut session, "#{ r | z = 3 }") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::record([
                        ("x".to_owned(), Value::int(1)),
                        ("y".to_owned(), Value::int(2)),
                        ("z".to_owned(), Value::int(3)),
                    ]))),
                    "field extension widens the record"
                );
            },
            | _ => panic!("`#{{ r | z = 3 }}` should evaluate"),
        }
        // Multiple overrides in one update, last-wins on a repeated label.
        match sole_outcome(&mut session, "#{ r | x = 7, y = 8 }") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::record([
                        ("x".to_owned(), Value::int(7)),
                        ("y".to_owned(), Value::int(8)),
                    ]))),
                    "multiple overrides replace multiple fields"
                );
            },
            | _ => panic!("`#{{ r | x = 7, y = 8 }}` should evaluate"),
        }
        // The red line: the original binding still denotes the un-updated record.
        match sole_outcome(&mut session, "r") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::record([
                        ("x".to_owned(), Value::int(1)),
                        ("y".to_owned(), Value::int(2)),
                    ]))),
                    "the base binding observes no update (record-update state-visibility invariant red line)"
                );
            },
            | _ => panic!("`r` should evaluate"),
        }
    }
    /// The list functional-update builtins each return a **fresh** list
    /// (value-semantics MVP, `proposal-value-semantics-mvp.md` §3.2): index
    /// replacement, length-changing edits, growth, and predicate-guarded map.
    #[test]
    fn list_functional_update_builtins_evaluate()
    {
        let mut session = Session::new();
        let cases: &[(&str, Vec<i64>)] = &[
            ("list.set([10, 20, 30], 1, 99)", vec![10, 99, 30]),
            (
                "list.update_at([10, 20, 30], 0, thunk { fn(x) { x + 1 } })",
                vec![11, 20, 30],
            ),
            ("list.insert_at([1, 2, 3], 1, 99)", vec![1, 99, 2, 3]),
            ("list.insert_at([1, 2], 2, 9)", vec![1, 2, 9]),
            ("list.remove_at([1, 2, 3], 1)", vec![1, 3]),
            ("list.push([1, 2], 3)", vec![1, 2, 3]),
            ("list.append([1, 2], [3, 4])", vec![1, 2, 3, 4]),
            ("list.concat([[1, 2], [3, 4]])", vec![1, 2, 3, 4]),
            (
                "list.update_where(thunk { fn(x) { x > 1 } }, thunk { fn(x) { x * 10 } }, [1, 2, 3])",
                vec![1, 20, 30],
            ),
        ];
        for &(source, ref expected) in cases {
            let elements: Vec<Value> = expected.iter().copied().map(Value::int).collect();
            match sole_outcome(&mut session, source) {
                | ItemOutcome::Expression { value, .. } => {
                    assert_eq!(
                        value,
                        Eval::Value(Comp::ret(Value::list(elements))),
                        "`{source}` returns a fresh list"
                    );
                },
                | other => panic!("`{source}` should evaluate; got {other:?}"),
            }
        }
    }
    /// An out-of-bounds list-update index is a defined `Blame::Hole`, not a
    /// panic (the gradual-hole discipline; `proposal-value-semantics-mvp.md`
    /// §3.2).
    #[test]
    fn out_of_bounds_list_update_blames()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "list.set([1, 2, 3], 9, 0)") {
            | ItemOutcome::Expression { value, .. } => {
                assert!(
                    matches!(value, Eval::Blame(_)),
                    "an out-of-bounds `set` blames rather than panics; got {value:?}"
                );
            },
            | other => panic!("`list.set(...)` should evaluate; got {other:?}"),
        }
    }
    /// String builtins are exposed through module-qualified prelude names.
    #[test]
    fn string_builtin_type_and_evaluate()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "string.escape(\"a+b.txt\")") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(ValueType::string())),
                    "`string.escape(...) : F String`",
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::string(r"a\+b\.txt"))),
                    "regex metacharacters are escaped for literal matching",
                );
            },
            | _ => panic!("`string.escape(...)` should evaluate"),
        }
    }
    /// Regex builtins are exposed through module-qualified prelude names.
    #[cfg(feature = "regex")]
    #[test]
    fn regex_builtin_type_and_evaluate()
    {
        let mut session = Session::new();
        match sole_outcome(
            &mut session,
            "regex.extract(\"^(?<marker><{7})(?<label>.*)\", \"<<<<<<< HEAD\")",
        ) {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::record([
                        ("label".to_owned(), Value::string(" HEAD")),
                        ("marker".to_owned(), Value::string("<<<<<<<")),
                    ]))),
                    "`regex.extract` returns named captures as a record",
                );
            },
            | _ => panic!("`regex.extract(...)` should evaluate"),
        }
    }
    /// Regex failure modes evaluate to the established gradual-hole blame.
    #[cfg(feature = "regex")]
    #[test]
    fn regex_extract_failures_are_gradual_blame()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "regex.extract(\"(\", \"anything\")") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(
                    Eval::Blame(Blame::Hole),
                    value,
                    "invalid regex blames a hole"
                );
            },
            | _ => panic!("invalid `regex.extract(...)` should still type-check"),
        }
        match sole_outcome(&mut session, "regex.extract(\"needle\", \"haystack\")") {
            | ItemOutcome::Expression { value, .. } => {
                assert_eq!(Eval::Blame(Blame::Hole), value, "no match blames a hole");
            },
            | _ => panic!("no-match `regex.extract(...)` should still type-check"),
        }
    }
    /// A computation-sorted ascription gives the check-only computations an
    /// expected type outside a `def` signature: `(t : B)` elaborates to
    /// `force ((thunk t) : U_ω B)`, so an `if`
    /// in bare expression position types and evaluates.
    #[test]
    fn computation_ascription_types_and_evaluates_check_only_forms()
    {
        let mut session = Session::new();
        match sole_outcome(
            &mut session,
            "(if true { ret 1 } else { ret 2 } : F Integer)",
        ) {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(ValueType::integer())),
                    "the ascription synthesizes the expected computation type",
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::int(1))),
                    "the `true` arm evaluates through the thunk-annotation encoding",
                );
            },
            | other => panic!("the ascribed `if` should evaluate; got {other:?}"),
        }
    }
    /// An `extern` block submitted on one line carries to a later line, where a
    /// foreign call `m.op(args)` type-checks cleanly (its effect row is honest)
    /// and — with no handler installed — evaluates to
    /// `Blame(PerformNoHandler)`: the capability-denied outcome
    /// (proposal-ffi.md §3.1/§3.2), least authority made visible.
    #[test]
    fn extern_declaration_carries_across_lines_and_a_foreign_call_blames_without_a_handler()
    {
        let mut session = Session::new();
        // The `extern` block is a declaration: it yields no runnable item.
        let declared = session
            .submit("extern \"c\" from \"m\" {\n  def cos(x: f64) -> f64;\n}")
            .expect("lowering must not fail");
        assert!(
            declared.outcomes.is_empty(),
            "an extern block contributes no runnable item"
        );
        assert!(
            declared.report.diagnostics.is_empty() && declared.report.goals.is_empty(),
            "declaring a foreign module is clean"
        );
        // The later call sees the module and elaborates to a perform.
        match sole_outcome(&mut session, "m.cos(2.0f64)") {
            | ItemOutcome::Expression { ty, value } => {
                match ty {
                    | Ty::Comp(CompType::F(payload, row)) => {
                        assert_eq!(*payload, ValueType::f64(), "the reply is f64");
                        assert!(
                            bool::from(row.contains("m".into())),
                            "the effect row records the foreign reach ⟨m⟩ — purity is not lost"
                        );
                    },
                    | other => panic!("the foreign call should type as `F^⟨m⟩ f64`, got {other:?}"),
                }
                assert_eq!(
                    Eval::Blame(Blame::PerformNoHandler),
                    value,
                    "with no FFI handler installed, the foreign call blames (capability denied)"
                );
            },
            | other => panic!("the foreign call should be an expression outcome, got {other:?}"),
        }
    }

    /// Import aliases persist as namespace bindings across submissions, and a
    /// later collision is declined without replacing the original binding.
    #[test]
    fn import_namespace_carries_across_lines_and_resolves_source_declarations()
    {
        let mut session = Session::new();
        let parse_path = NamePath::from(DottedName::from("parse"));
        let list_path = NamePath::from(DottedName::from("list_ext"));
        let missing_path = NamePath::from(DottedName::from("missing"));

        let parse = session
            .submit("import \"file:///lib/parse.gandr\" as parse ;")
            .expect("the first import must lower");
        assert!(
            parse.outcomes.is_empty()
                && parse.report.diagnostics.is_empty()
                && parse.report.goals.is_empty(),
            "an import is a clean declaration rather than a runnable item"
        );

        let list = session
            .submit("import \"file:///lib/list.gandr\" as list_ext ;")
            .expect("the second import must lower against the prior namespace");
        assert!(
            list.outcomes.is_empty()
                && list.report.diagnostics.is_empty()
                && list.report.goals.is_empty(),
            "a distinct later alias extends the persistent namespace cleanly"
        );

        assert_eq!(
            session
                .resolve_import(&parse_path)
                .map(|declaration| declaration.uri.as_str()),
            Some("file:///lib/parse.gandr"),
            "the first line's alias resolves to its retained declaration"
        );
        assert_eq!(
            session
                .resolve_import(&list_path)
                .map(|declaration| declaration.uri.as_str()),
            Some("file:///lib/list.gandr"),
            "the second line's index resolves against the cumulative declaration table"
        );
        assert_eq!(
            session.resolve_import(&missing_path),
            None,
            "an unbound path does not resolve"
        );

        let duplicate = session
            .submit("import \"file:///lib/other.gandr\" as parse ;")
            .expect("total session lowering turns the namespace rejection into a hole");
        assert_eq!(
            duplicate.outcomes,
            Vec::from([ItemOutcome::Holey]),
            "a later line cannot silently replace an existing import alias"
        );
        assert_eq!(
            session
                .resolve_import(&parse_path)
                .map(|declaration| declaration.uri.as_str()),
            Some("file:///lib/parse.gandr"),
            "the rejected collision leaves the original declaration reachable"
        );
    }

    /// Submits `source`, asserts it lowered to exactly one item, and returns
    /// that item's outcome.
    fn sole_outcome<'text>(
        session: &mut Session,
        source: impl Into<TestText<'text>>,
    ) -> ItemOutcome
    {
        let source = source.into().0;
        let submission = session
            .submit(source)
            .expect("infrastructure lowering must not fail");
        assert_eq!(
            1,
            submission.outcomes.len(),
            "expected exactly one item for `{source}`"
        );
        submission
            .outcomes
            .into_iter()
            .next()
            .expect("one outcome was just asserted")
    }
    /// `list.any` returns a boolean; `list.sort` returns an ordered list — both
    /// through the module prelude.
    #[test]
    fn list_any_and_sort_evaluate()
    {
        let mut session = Session::new();
        match sole_outcome(
            &mut session,
            "list.any(thunk { fn(x) { x > 2 } }, [1, 2, 3])",
        ) {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(boolean_type())),
                    "`list.any(...) : F Bool`"
                );
                assert_eq!(value, Eval::Value(Comp::ret(boolean_value(true))));
            },
            | _ => panic!("`list.any(...)` should evaluate"),
        }
        match sole_outcome(&mut session, "list.sort([3, 1, 2])") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(ty, list_returner());
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::list(vec![
                        Value::int(1),
                        Value::int(2),
                        Value::int(3),
                    ]))),
                    "`list.sort` orders the list"
                );
            },
            | _ => panic!("`list.sort(...)` should evaluate"),
        }
    }

    /// Builds the boolean payload returned after the L machine erases checked
    /// annotations.
    fn boolean_value(is_true: impl Into<TestDecision>) -> Value
    {
        if is_true.into().0 {
            Value::inj1(Value::Unit)
        }
        else {
            Value::inj2(Value::Unit)
        }
    }
    /// A comparison operator returns a boolean payload and carries the
    /// contracted `Unknown -> Unknown -> F Boolean` type.
    #[test]
    fn comparison_operators_type_check_and_evaluate()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "1 < 2") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(boolean_type())),
                    "`1 < 2` has boolean result type"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(boolean_value(true))),
                    "`1 < 2` evaluates to true"
                );
            },
            | _ => panic!("`1 < 2` should type-check and evaluate"),
        }
    }
    /// A boolean operator accepts checked booleans and returns a boolean
    /// payload.
    #[test]
    fn boolean_operators_type_check_and_evaluate()
    {
        let mut session = Session::new();
        match sole_outcome(&mut session, "true && false") {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(boolean_type())),
                    "`true && false` has boolean result type"
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(boolean_value(false))),
                    "`true && false` evaluates to false"
                );
            },
            | _ => panic!("`true && false` should type-check and evaluate"),
        }
    }
    /// Conflict-marker scanning can use direct string predicates from the
    /// prelude.
    #[test]
    fn string_contains_scans_conflict_marker_text()
    {
        let mut session = Session::new();
        match sole_outcome(
            &mut session,
            "string.contains(\"<<<<<<< HEAD\", \"<<<<<<<\")",
        ) {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(
                    ty,
                    Ty::Comp(CompType::returner(boolean_type())),
                    "`string.contains(...) : F Bool`",
                );
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(boolean_value(true))),
                    "the marker prefix is found",
                );
            },
            | _ => panic!("`string.contains(...)` should evaluate"),
        }
    }

    /// The `1 + 1` boolean carrier used by bool literals and native predicates.
    fn boolean_type() -> ValueType
    {
        ValueType::sum(ValueType::Unit, ValueType::Unit)
    }

    /// An unknown member of a **prelude** namespace is declined at the
    /// recognition boundary, and total mode recovers the decline as a hole.
    ///
    /// The hole is what total mode does with a refusal — it is not evidence
    /// that record projection is unavailable. Record projection is a working
    /// path, and a selection off an *ungoverned* target takes it. What this
    /// pins is that `prim.bogus` is refused rather than guessed: `prim` is a
    /// namespace whose members the scope knows exactly, so an absent one is an
    /// error rather than an open field access.
    #[test]
    fn an_unknown_prelude_member_is_declined_as_a_hole()
    {
        let mut session = Session::new();
        let submission = session
            .submit("prim.bogus")
            .expect("lowering must not fail");
        assert!(
            !submission.report.goals.is_empty(),
            "`prim.bogus` lowers to a hole goal"
        );
        assert!(
            submission
                .outcomes
                .iter()
                .all(|outcome| matches!(*outcome, ItemOutcome::Holey)),
            "an unknown prelude member is declined (a hole), not a value"
        );
    }

    /// A user module's hidden and absent components are declined in **total**
    /// mode too, and the decline recovers as a hole rather than as a record
    /// projection.
    ///
    /// This is the total-mode half of the module stratum's governance, and a
    /// strict-mode refusal test cannot reach it: strict lowering returns the
    /// error, while total lowering has to choose what to put in its place.
    /// Putting a `RecordProj` there is the silent failure this kills —
    /// `Facts.hidden` would become a field access on a record that matching
    /// had already removed the field from, and the mistake would surface much
    /// later against a record type that never mentions the signature that hid
    /// it.
    ///
    /// Both reasons a component can be missing are exercised, because they
    /// reach the scope differently: `hidden` was defined and then dropped by
    /// matching, while `never` was never written at all. The exported
    /// component is checked first, so the two declines are known to be about
    /// the component rather than about the module.
    #[test]
    fn a_hidden_or_absent_user_module_component_is_declined_as_a_hole()
    {
        const DECLARATION: &str =
            "module Facts : #{ total: Integer } { def hidden = 1; def total = 2; }";

        // One session throughout, so the positive control and the two declines
        // are known to be reading the *same* registered module. Re-declaring
        // per case would let a silently holey declaration pass this test for
        // the wrong reason: an unregistered `Facts` is ungoverned, and its
        // selection would reach a hole by the record path instead.
        let mut session = Session::new();
        let declaration = session
            .submit(DECLARATION)
            .expect("the module declaration lowers");
        assert!(
            declaration
                .outcomes
                .iter()
                .all(|outcome| !matches!(*outcome, ItemOutcome::Holey)),
            "the declaration itself lowers cleanly, so `Facts` is registered"
        );
        let exported = session
            .submit("Facts.total")
            .expect("lowering must not fail");
        assert!(
            exported
                .outcomes
                .iter()
                .all(|outcome| !matches!(*outcome, ItemOutcome::Holey)),
            "the exported component is reached through its path, not declined"
        );

        for selection in ["Facts.hidden", "Facts.never"] {
            let submission = session.submit(selection).expect("lowering must not fail");
            assert!(
                submission
                    .outcomes
                    .iter()
                    .all(|outcome| matches!(*outcome, ItemOutcome::Holey)),
                "`{selection}` is declined as a hole, never projected off the module record"
            );
            assert!(
                !submission.report.goals.is_empty(),
                "`{selection}` leaves a hole goal"
            );
        }
    }
    /// `list.where` filters by a pure predicate closure.
    #[test]
    fn list_where_filters_by_a_predicate()
    {
        let mut session = Session::new();
        match sole_outcome(
            &mut session,
            "list.where(thunk { fn(x) { x > 1 } }, [1, 2, 3])",
        ) {
            | ItemOutcome::Expression { ty, value } => {
                assert_eq!(ty, list_returner());
                assert_eq!(
                    value,
                    Eval::Value(Comp::ret(Value::list(vec![Value::int(2), Value::int(3)]))),
                    "`list.where(x > 1)` keeps `[2, 3]`"
                );
            },
            | _ => panic!("`list.where(...)` should type-check and evaluate"),
        }
    }

    /// The gradual list-returning combinator type `F (List ?)`.
    fn list_returner() -> Ty
    {
        Ty::Comp(CompType::returner(ValueType::list(ValueType::Unknown)))
    }

    /// An item carrying holes is reported as a goal and not evaluated (the
    /// parse-completeness validator).
    #[test]
    fn holes_decline_evaluation()
    {
        let mut session = Session::new();
        let submission = session.submit("{ }").expect("lowering must not fail");
        assert!(
            !submission.report.goals.is_empty(),
            "an empty block lowers to a hole goal"
        );
        assert!(
            submission
                .outcomes
                .iter()
                .all(|outcome| matches!(*outcome, ItemOutcome::Holey)),
            "a holey item is not evaluated"
        );
    }
}
