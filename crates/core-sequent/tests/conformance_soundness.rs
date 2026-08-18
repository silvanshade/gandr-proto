//! The conformance suite's **observable-outcome** soundness rows, re-homed on
//! the L machine (B1 phase-3 stage F; coordinator decision D2).
//!
//! The conformance suite pinned the runtime outcome of a large family of
//! programs — declared-data and split β-reduction, the native-builtin
//! substrate, the higher-order combinators, the string / path / regex builtins,
//! and the deep-handler assert runner — against a CEK oracle that has since
//! retired. Decision D2 re-points those observable-outcome rows at the L
//! machine so their soundness survives that retirement.
//!
//! # Why they live here
//!
//! Because the L machine is this crate's, and a row asserting an observable
//! outcome belongs beside the machine that produces it.
//!
//! **The obstruction that originally forced the move has dissolved, and the
//! placement is now a choice rather than a consequence.** When the rows moved,
//! the suite was an internal `#[cfg(test)]` module of `gandr-core-checker`, and
//! that crate's unit-test target is a distinct compilation of itself from the
//! one a dev-dependency links — so the two spellings of a term type would not
//! unify and no program could be handed across. The suite now lives in
//! `gandr-core-checker-tools`, and the term vocabulary it shares with the L
//! machine comes from `gandr-core-term`, a plain dependency of both, so the
//! types unify and the argument no longer applies. What survives it is the
//! ordinary reason above.
//!
//! Each program is rebuilt through the frozen public `gandr_core_checker` API
//! and run on the L machine via [`gandr_core_sequent::machine`]; every asserted
//! outcome is preserved verbatim from the conformance row it mirrors. The
//! native reductions themselves are shared `gandr_core_term::prim` code
//! (machine independent), so these rows pin that substrate's behaviour where
//! the CEK conformance rows used to.
//!
//! # What stays behind in `gandr_core_checker::conformance`
//!
//! - the **typing** rows (`checker ≡ typing machine` via `agree_comp`) — they
//!   never touched the evaluator and are unaffected by the CEK's retirement;
//! - the CEK-internal **differentials** with no L analogue — the recursive
//!   reference `eval_comp ≡ run_comp` agreement and the per-step
//!   subject-reduction harness (`State` / `step` / `Outcome`) — which retire
//!   *with* the CEK;
//! - `rigid_returner_evaluates_to_a_typed_value`, the generator-driven
//!   operational soundness oracle: its `rigid_value_type` / `rigid_check`
//!   generators are private to the conformance suite, so it retires with the
//!   CEK. The L machine's operational soundness over that fragment is covered
//!   by the corpus outcome-snapshot sweep (every corpus program realizes a
//!   defined outcome on L) and the property differential
//!   (`tests/differential.rs`).

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::machine;
    use gandr_core_term::boundary::EffectSignatureName;
    use gandr_core_term::boundary::OperationName;
    use gandr_core_term::effect::EffectOp;
    use gandr_core_term::effect::EffectSig;
    use gandr_core_term::grade::Grade;
    use gandr_core_term::outcome::Blame;
    use gandr_core_term::outcome::Eval;
    use gandr_core_term::outcome::StuckReason;
    use gandr_core_term::prim::NativePrim;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::OpClause;
    use gandr_core_term::syntax::SplitMotive;
    use gandr_core_term::syntax::Value;
    use gandr_core_term::types::CompType;
    use gandr_core_term::types::DataId;
    use gandr_core_term::types::ValueType;

    // ─────────────────────────────── declared data + split (β on the machine)

    /// The declared-data β-rule selects the arm at the constructor's tag and
    /// binds the field-tuple payload; `Some(3)` matched by its arm returns `3`.
    #[test]
    fn data_case_selects_arm_returns_three()
    {
        let maybe = DataId::new(0, "Maybe");
        let some3 = Value::ctor(maybe, 1, Value::int(3));
        let data_case = Comp::data_case(some3, vec![
            ("_none".to_owned(), Comp::ret(Value::int(0))),
            ("x".to_owned(), Comp::ret(Value::var("x"))),
        ]);
        assert_eq!(
            machine::run_comp(&data_case),
            Eval::Value(Comp::ret(Value::int(3)))
        );
    }

    /// A declared-data `case` on a non-constructor scrutinee is an undefined
    /// stuck (an ill-typed elimination), never a panic.
    #[test]
    fn data_case_on_non_ctor_is_stuck()
    {
        let data_case = Comp::data_case(Value::int(5), vec![(
            "x".to_owned(),
            Comp::ret(Value::var("x")),
        )]);
        assert_eq!(
            Eval::Stuck(StuckReason::DataCasedNonCtor),
            machine::run_comp(&data_case)
        );
    }

    /// `split (3, here 3) as (x, q) in ret x` reduces to `ret 3` — split-β is
    /// type-erased, so the runtime ignores the Σ dependency (the typing face
    /// stays in `gandr_core_checker::conformance`).
    #[test]
    fn sigma_split_evaluates_to_first_component()
    {
        let pair = Value::pair(Value::int(3), Value::here(Value::int(3)));
        let split = Comp::split(pair, "x", "q", Comp::ret(Value::var("x")));
        assert_eq!(
            machine::run_comp(&split),
            Eval::Value(Comp::ret(Value::int(3)))
        );
    }

    /// A motive-bearing split over a product reduces identically to a plain one
    /// (the motive is inert at runtime): `split (3, 4) as (x, y) [z. …] in
    /// ret (here (x, y))` evaluates to `ret (here (3, 4))`.
    #[test]
    fn dependent_split_over_prod_evaluates_type_erased()
    {
        let scrut = Value::pair(Value::int(3), Value::int(4));
        let int_prod = ValueType::prod(ValueType::integer(), ValueType::integer());
        let motive = SplitMotive::new(
            "z",
            CompType::returner(ValueType::path(int_prod, Value::var("z"), Value::var("z"))),
        );
        let split = Comp::split_motive(
            scrut,
            "x",
            "y",
            motive,
            Comp::ret(Value::here(Value::pair(Value::var("x"), Value::var("y")))),
        );
        assert_eq!(
            machine::run_comp(&split),
            Eval::Value(Comp::ret(Value::here(Value::pair(
                Value::int(3),
                Value::int(4)
            )))),
            "split-β is type-erased: the motive is inert"
        );
    }

    /// A constant-motive split over a Σ-typed pair reduces exactly as a product
    /// one: `split (3, here 3) as (x, q) [z. F Integer] in ret x` → `ret 3`.
    #[test]
    fn motive_split_over_sigma_evaluates_to_first_component()
    {
        let scrut = Value::pair(Value::int(3), Value::here(Value::int(3)));
        let split = Comp::split_motive(
            scrut,
            "x",
            "q",
            SplitMotive::new("z", CompType::returner(ValueType::integer())),
            Comp::ret(Value::var("x")),
        );
        assert_eq!(
            machine::run_comp(&split),
            Eval::Value(Comp::ret(Value::int(3)))
        );
    }

    // ─────────────────────────────────────────────── native-builtin substrate

    /// `I 5` evaluates to `ret 5` (no prelude — the native is directly in
    /// focus).
    #[test]
    fn identity_applied_returns_its_argument()
    {
        let comp = Comp::app(Comp::native(NativePrim::Id), Value::int(5));
        assert_eq!(
            machine::run_comp(&comp),
            Eval::Value(Comp::ret(Value::int(5)))
        );
    }

    /// `K 7 9` evaluates to `ret 7` — currying: the native stays a terminal
    /// awaiting its second argument, then reduces.
    #[test]
    fn const_applied_returns_its_first_argument()
    {
        let comp = Comp::app(
            Comp::app(Comp::native(NativePrim::Const), Value::int(7)),
            Value::int(9),
        );
        assert_eq!(
            machine::run_comp(&comp),
            Eval::Value(Comp::ret(Value::int(7)))
        );
    }

    /// Native integer addition reduces: `1 + 2 -> ret 3`.
    #[test]
    fn native_integer_addition_returns_sum()
    {
        let term = native_binary(NativePrim::Add, Value::int(1), Value::int(2));
        assert_eq!(
            machine::run_comp(&term),
            Eval::Value(Comp::ret(Value::int(3)))
        );
    }

    /// Mismatched numeric tags and checked integer overflow both lower to the
    /// gradual hole computation, which the machine reports as defined blame.
    #[test]
    fn native_numeric_mismatch_and_overflow_blame_via_hole()
    {
        let mixed_tags = native_binary(NativePrim::Add, Value::u32(1_u32), Value::u64(2_u64));
        assert_eq!(Eval::Blame(Blame::Hole), machine::run_comp(&mixed_tags));

        let overflow = native_binary(NativePrim::Add, Value::int(i64::MAX), Value::int(1));
        assert_eq!(Eval::Blame(Blame::Hole), machine::run_comp(&overflow));
    }

    /// Same-tag suffixed numeric addition preserves the numeric tag:
    /// `1u32 + 2u32 -> ret 3u32`.
    #[test]
    fn native_u32_addition_returns_u32_sum()
    {
        let term = native_binary(NativePrim::Add, Value::u32(1_u32), Value::u32(2_u32));
        assert_eq!(
            machine::run_comp(&term),
            Eval::Value(Comp::ret(Value::u32(3_u32)))
        );
    }

    /// Native comparison returns an annotated canonical boolean: `1 < 2 ->
    /// true`.
    #[test]
    fn native_integer_less_than_returns_true()
    {
        let term = native_binary(NativePrim::Lt, Value::int(1), Value::int(2));
        assert_eq!(
            machine::run_comp(&term),
            Eval::Value(Comp::ret(boolean((true).into())))
        );
    }

    /// Native boolean conjunction: `true && false -> ret false`.
    #[test]
    fn native_boolean_and_returns_false()
    {
        let term = native_binary(
            NativePrim::And,
            boolean((true).into()),
            boolean((false).into()),
        );
        assert_eq!(
            machine::run_comp(&term),
            Eval::Value(Comp::ret(boolean((false).into())))
        );
    }

    /// `[1] ++ [2] -> ret [1, 2]`.
    #[test]
    fn native_list_concat_returns_concatenated_list()
    {
        let term = native_binary(
            NativePrim::ListConcat,
            Value::list(vec![Value::int(1)]),
            Value::list(vec![Value::int(2)]),
        );
        assert_eq!(
            machine::run_comp(&term),
            Eval::Value(Comp::ret(Value::list(vec![Value::int(1), Value::int(2)])))
        );
    }

    /// The prelude binding-environment resolves a forced free name: `id` bound
    /// to a thunk wrapping `Native{Id}` resolves, so `force(id) 5`
    /// evaluates to `ret 5` — the seam the REPL's operator / module
    /// preludes ride (ADR-42).
    #[test]
    fn a_forced_prelude_name_resolves_to_its_builtin()
    {
        let bindings = vec![(
            "id".to_owned(),
            Value::thunk(Grade::OMEGA, Comp::native(NativePrim::Id)),
        )];
        let comp = Comp::app(Comp::force(Value::var("id")), Value::int(5));
        assert_eq!(
            machine::run_comp_with_prelude(&comp, &bindings),
            Eval::Value(Comp::ret(Value::int(5)))
        );
    }

    /// A forced name absent from the prelude is the ordinary `ForcedNonThunk`
    /// stuck — the prelude widens resolution without masking a
    /// genuinely-unbound name.
    #[test]
    fn an_unbound_forced_name_is_still_stuck()
    {
        let comp = Comp::app(Comp::force(Value::var("nope")), Value::int(5));
        assert_eq!(
            Eval::Stuck(StuckReason::ForcedNonThunk),
            machine::run_comp_with_prelude(&comp, &[])
        );
    }

    // ──────────────────────────────────── higher-order + string / path builtins

    /// Regex extraction returns named captures as a manifest record.
    #[test]
    #[cfg(feature = "gandr_feat_regex")]
    fn regex_extract_returns_named_capture_record()
    {
        assert_eq!(
            run(NativePrim::RegexExtract, vec![
                Value::string(r"^(?<marker><{7}|={7}|>{7}|\|{7})(?<label>$|[ \t].*)"),
                Value::string("<<<<<<< HEAD"),
            ]),
            Eval::Value(Comp::ret(Value::record([
                ("label".to_owned(), Value::string(" HEAD")),
                ("marker".to_owned(), Value::string("<<<<<<<")),
            ]))),
        );
    }

    /// Invalid patterns and no-match searches degrade to gradual-hole blame.
    #[test]
    #[cfg(feature = "gandr_feat_regex")]
    fn regex_extract_failures_blame_a_hole()
    {
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::RegexExtract, vec![
                Value::string("("),
                Value::string("anything")
            ]),
            "an invalid regex is a gradual failure",
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::RegexExtract, vec![
                Value::string("needle"),
                Value::string("haystack")
            ]),
            "no match is a gradual failure",
        );
    }

    /// Regex escaping and conflict-marker string scanning are direct natives.
    #[test]
    fn string_builtins_escape_and_scan_conflict_markers()
    {
        assert_eq!(
            run(NativePrim::StringEscape, vec![Value::string("a+b.txt")]),
            Eval::Value(Comp::ret(Value::string(r"a\+b\.txt"))),
        );
        assert_eq!(
            run(NativePrim::StringContains, vec![
                Value::string("<<<<<<< HEAD"),
                Value::string("<<<<<<<")
            ]),
            Eval::Value(Comp::ret(boolean((true).into()))),
        );
    }

    /// `string.eq` decides string equality on the `1 + 1` carrier; a non-string
    /// argument degrades to gradual-hole blame like every wrong-shape native.
    #[test]
    fn string_eq_decides_equality_and_blames_non_strings()
    {
        assert_eq!(
            run(NativePrim::StringEq, vec![
                Value::string("agda"),
                Value::string("agda")
            ]),
            Eval::Value(Comp::ret(boolean((true).into()))),
        );
        assert_eq!(
            run(NativePrim::StringEq, vec![
                Value::string("agda"),
                Value::string("agdA")
            ]),
            Eval::Value(Comp::ret(boolean((false).into()))),
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::StringEq, vec![
                Value::string("agda"),
                Value::int(1)
            ]),
            "a non-string operand is a gradual failure",
        );
    }

    /// Pure UTF-8 path helpers cover the gate-script path joins and filename
    /// checks.
    #[test]
    fn path_builtins_join_and_read_components()
    {
        assert_eq!(
            run(NativePrim::PathJoin, vec![
                Value::string("docs"),
                Value::string("guide.md")
            ]),
            Eval::Value(Comp::ret(Value::string("docs/guide.md"))),
        );
        assert_eq!(
            run(NativePrim::PathBasename, vec![Value::string(
                "docs/guide.md"
            )]),
            Eval::Value(Comp::ret(Value::string("guide.md"))),
        );
        assert_eq!(
            run(NativePrim::PathExtension, vec![Value::string(
                "docs/guide.md"
            )]),
            Eval::Value(Comp::ret(Value::string("md"))),
        );
    }

    // ─────────────── band-01-rung-07: division, negation, read-side builtins

    /// Truncating integer division rounds toward zero (`div 7 2 -> 3`, `div
    /// -7 2 -> -3`); same-tag sized atoms divide within their own tag.
    #[test]
    fn native_integer_division_truncates_toward_zero()
    {
        assert_eq!(
            machine::run_comp(&native_binary(
                NativePrim::Div,
                Value::int(7),
                Value::int(2)
            )),
            Eval::Value(Comp::ret(Value::int(3)))
        );
        assert_eq!(
            machine::run_comp(&native_binary(
                NativePrim::Div,
                Value::int(-7),
                Value::int(2)
            )),
            Eval::Value(Comp::ret(Value::int(-3))),
            "truncation rounds toward zero"
        );
        assert_eq!(
            machine::run_comp(&native_binary(
                NativePrim::Div,
                Value::u32(7_u32),
                Value::u32(2_u32)
            )),
            Eval::Value(Comp::ret(Value::u32(3_u32))),
            "same-tag sized atoms keep their tag"
        );
    }

    /// A zero divisor, the overflowing `MIN / -1`, mixed numeric tags, and —
    /// by the rung-07 scope ruling — float operands all degrade to the
    /// gradual hole.
    #[test]
    fn native_division_failures_blame_via_hole()
    {
        assert_eq!(
            Eval::Blame(Blame::Hole),
            machine::run_comp(&native_binary(
                NativePrim::Div,
                Value::int(1),
                Value::int(0)
            )),
            "a zero divisor is a gradual failure"
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            machine::run_comp(&native_binary(
                NativePrim::Div,
                Value::int(i64::MIN),
                Value::int(-1)
            )),
            "the MIN / -1 overflow is a gradual failure"
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            machine::run_comp(&native_binary(
                NativePrim::Div,
                Value::int(1),
                Value::u32(1_u32)
            )),
            "mixed numeric tags are a gradual failure"
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            machine::run_comp(&native_binary(
                NativePrim::Div,
                Value::f32(1.5_f32),
                Value::f32(0.5_f32)
            )),
            "float operands stay out of this primitive (the rung-07 ruling)"
        );
    }

    /// Truncating remainder keeps the dividend's sign (`mod -7 2 -> -1`) and
    /// refuses a zero divisor exactly as division does.
    #[test]
    fn native_integer_remainder_keeps_the_dividend_sign()
    {
        assert_eq!(
            machine::run_comp(&native_binary(
                NativePrim::Mod,
                Value::int(7),
                Value::int(2)
            )),
            Eval::Value(Comp::ret(Value::int(1)))
        );
        assert_eq!(
            machine::run_comp(&native_binary(
                NativePrim::Mod,
                Value::int(-7),
                Value::int(2)
            )),
            Eval::Value(Comp::ret(Value::int(-1))),
            "the remainder takes the dividend's sign"
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            machine::run_comp(&native_binary(
                NativePrim::Mod,
                Value::int(1),
                Value::int(0)
            )),
            "a zero divisor is a gradual failure"
        );
    }

    /// Boolean negation flips the canonical carrier both ways; a non-boolean
    /// argument degrades to the gradual hole.
    #[test]
    fn native_boolean_not_negates_and_blames_non_booleans()
    {
        assert_eq!(
            run(NativePrim::Not, vec![boolean((true).into())]),
            Eval::Value(Comp::ret(boolean((false).into())))
        );
        assert_eq!(
            run(NativePrim::Not, vec![boolean((false).into())]),
            Eval::Value(Comp::ret(boolean((true).into())))
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::Not, vec![Value::int(1)]),
            "a non-boolean is a gradual failure"
        );
    }

    /// `list.length` counts elements (`0` for the empty list); a non-list
    /// subject degrades to the gradual hole.
    #[test]
    fn native_list_length_counts_elements()
    {
        assert_eq!(
            run(NativePrim::ListLength, vec![ints((&[1, 2, 3]).into())]),
            Eval::Value(Comp::ret(Value::int(3)))
        );
        assert_eq!(
            run(NativePrim::ListLength, vec![ints((&[]).into())]),
            Eval::Value(Comp::ret(Value::int(0))),
            "the empty list has length zero"
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::ListLength, vec![Value::int(1)]),
            "a non-list subject is a gradual failure"
        );
    }

    /// `list.get` reads an `Optional`: in range `Some`, out of range or a
    /// negative index `None`; a non-list subject or non-integer index
    /// degrades to the gradual hole.
    #[test]
    fn native_list_at_reads_an_optional()
    {
        let list = ints((&[10, 20, 30]).into());
        assert_eq!(
            run(NativePrim::ListAt, vec![list.clone(), Value::int(1)]),
            Eval::Value(Comp::ret(some(Value::int(20))))
        );
        assert_eq!(
            run(NativePrim::ListAt, vec![list.clone(), Value::int(3)]),
            Eval::Value(Comp::ret(none())),
            "an out-of-range index is None, not a hole"
        );
        assert_eq!(
            run(NativePrim::ListAt, vec![list.clone(), Value::int(-1)]),
            Eval::Value(Comp::ret(none())),
            "a negative index names no position: None"
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::ListAt, vec![Value::int(1), Value::int(0)]),
            "a non-list subject is a gradual failure"
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::ListAt, vec![list, Value::string("1")]),
            "a non-integer index is a gradual failure"
        );
    }

    /// `string.append` concatenates; `string.length` counts Unicode scalar
    /// values (`"héllo"` is 5 scalars, 6 bytes). Wrong shapes degrade to the
    /// gradual hole.
    #[test]
    fn native_string_append_and_length_build_and_measure()
    {
        assert_eq!(
            run(NativePrim::StringAppend, vec![
                Value::string("types are "),
                Value::string("machines")
            ]),
            Eval::Value(Comp::ret(Value::string("types are machines")))
        );
        assert_eq!(
            run(NativePrim::StringLength, vec![Value::string("héllo")]),
            Eval::Value(Comp::ret(Value::int(5))),
            "length counts scalar values, not bytes"
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::StringAppend, vec![
                Value::string("a"),
                Value::int(1)
            ]),
            "a non-string operand is a gradual failure"
        );
    }

    /// `each inc [1,2,3] -> [2,3,4]`; the empty and singleton lists are the
    /// unroll's base and one-step cases.
    #[test]
    fn each_maps_a_pure_closure_over_a_list()
    {
        let inc = || unary_op(NativePrim::Add, Value::int(1));
        assert_eq!(
            run(NativePrim::Each, vec![inc(), ints((&[1, 2, 3]).into())]),
            Eval::Value(Comp::ret(ints((&[2, 3, 4]).into())))
        );
        assert_eq!(
            run(NativePrim::Each, vec![inc(), ints((&[]).into())]),
            Eval::Value(Comp::ret(ints((&[]).into()))),
            "the empty list is mapped to the empty list"
        );
        assert_eq!(
            run(NativePrim::Each, vec![inc(), ints((&[9]).into())]),
            Eval::Value(Comp::ret(ints((&[10]).into()))),
            "a singleton exercises the one-step unroll"
        );
    }

    /// `each (λx. each inc x) [[1,2],[3]] -> [[2,3],[4]]` — a combinator inside
    /// a combinator's closure.
    #[test]
    fn combinators_nest_inside_closures()
    {
        let inc = unary_op(NativePrim::Add, Value::int(1));
        let map_inc = closure1(Comp::app(
            Comp::app(Comp::native(NativePrim::Each), inc),
            Value::var("x"),
        ));
        let nested = Value::list(vec![ints((&[1, 2]).into()), ints((&[3]).into())]);
        assert_eq!(
            run(NativePrim::Each, vec![map_inc, nested]),
            Eval::Value(Comp::ret(Value::list(vec![
                ints((&[2, 3]).into()),
                ints((&[4]).into())
            ])))
        );
    }

    /// `where (λx. x > 1) [1,2,3] -> [2,3]` — the filter keeps order.
    #[test]
    fn where_filters_by_a_pure_predicate()
    {
        let gt1 = || unary_op(NativePrim::Gt, Value::int(1));
        assert_eq!(
            run(NativePrim::Where, vec![gt1(), ints((&[1, 2, 3]).into())]),
            Eval::Value(Comp::ret(ints((&[2, 3]).into())))
        );
        assert_eq!(
            run(NativePrim::Where, vec![gt1(), ints((&[]).into())]),
            Eval::Value(Comp::ret(ints((&[]).into()))),
            "the empty filter is the empty list"
        );
        assert_eq!(
            run(NativePrim::Where, vec![gt1(), ints((&[2, 1, 3]).into())]),
            Eval::Value(Comp::ret(ints((&[2, 3]).into())))
        );
    }

    /// `reduce (+) 0 [1,2,3] -> 6` (a left fold); the empty list returns the
    /// seed unchanged.
    #[test]
    fn reduce_left_folds_a_list()
    {
        let add = || {
            closure2(Comp::app(
                Comp::app(Comp::native(NativePrim::Add), Value::var("acc")),
                Value::var("x"),
            ))
        };
        assert_eq!(
            run(NativePrim::Reduce, vec![
                add(),
                Value::int(0),
                ints((&[1, 2, 3]).into())
            ]),
            Eval::Value(Comp::ret(Value::int(6)))
        );
        assert_eq!(
            run(NativePrim::Reduce, vec![
                add(),
                Value::int(42),
                ints((&[]).into())
            ]),
            Eval::Value(Comp::ret(Value::int(42))),
            "the empty fold is the seed"
        );
    }

    /// `any (λx. x > 2)` short-circuits to `true` on the first witness and is
    /// `false` over the empty list.
    #[test]
    fn any_short_circuits_to_true()
    {
        let gt2 = || unary_op(NativePrim::Gt, Value::int(2));
        assert_eq!(
            run(NativePrim::Any, vec![gt2(), ints((&[1, 2, 3]).into())]),
            Eval::Value(Comp::ret(boolean((true).into())))
        );
        assert_eq!(
            run(NativePrim::Any, vec![gt2(), ints((&[1, 2]).into())]),
            Eval::Value(Comp::ret(boolean((false).into())))
        );
        assert_eq!(
            run(NativePrim::Any, vec![gt2(), ints((&[]).into())]),
            Eval::Value(Comp::ret(boolean((false).into()))),
            "`any` over the empty list is false"
        );
    }

    /// `all (λx. x > 0)` short-circuits to `false` on the first failure and is
    /// `true` over the empty list.
    #[test]
    fn all_short_circuits_to_false()
    {
        let gt0 = || unary_op(NativePrim::Gt, Value::int(0));
        assert_eq!(
            run(NativePrim::All, vec![gt0(), ints((&[1, 2, 3]).into())]),
            Eval::Value(Comp::ret(boolean((true).into())))
        );
        assert_eq!(
            run(NativePrim::All, vec![gt0(), ints((&[1, 0, 3]).into())]),
            Eval::Value(Comp::ret(boolean((false).into())))
        );
        assert_eq!(
            run(NativePrim::All, vec![gt0(), ints((&[]).into())]),
            Eval::Value(Comp::ret(boolean((true).into()))),
            "`all` over the empty list is true"
        );
    }

    /// `flatten [[1,2],[3]] -> [1,2,3]`; a non-list element blames a hole.
    #[test]
    fn flatten_concatenates_manifest_sublists()
    {
        let nested = Value::list(vec![ints((&[1, 2]).into()), ints((&[3]).into())]);
        assert_eq!(
            run(NativePrim::Flatten, vec![nested]),
            Eval::Value(Comp::ret(ints((&[1, 2, 3]).into())))
        );
        assert_eq!(
            run(NativePrim::Flatten, vec![Value::list(vec![])]),
            Eval::Value(Comp::ret(ints((&[]).into()))),
            "flattening the empty list is the empty list"
        );
        let ragged = Value::list(vec![ints((&[1]).into()), Value::int(5)]);
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::Flatten, vec![ragged]),
            "a non-list element is a wrong-shape hole"
        );
    }

    /// `uniq [1,2,1,3,2] -> [1,2,3]` — first occurrence wins.
    #[test]
    fn uniq_drops_later_structural_duplicates()
    {
        assert_eq!(
            run(NativePrim::Uniq, vec![ints((&[1, 2, 1, 3, 2]).into())]),
            Eval::Value(Comp::ret(ints((&[1, 2, 3]).into())))
        );
    }

    /// `sort` orders homogeneous integer / string lists; a heterogeneous or
    /// non-orderable (float) list blames a hole.
    #[test]
    fn sort_orders_homogeneous_orderable_atoms()
    {
        assert_eq!(
            run(NativePrim::Sort, vec![ints((&[3, 1, 2]).into())]),
            Eval::Value(Comp::ret(ints((&[1, 2, 3]).into())))
        );
        let strings = Value::list(vec![
            Value::string("b"),
            Value::string("a"),
            Value::string("c"),
        ]);
        let sorted = Value::list(vec![
            Value::string("a"),
            Value::string("b"),
            Value::string("c"),
        ]);
        assert_eq!(
            run(NativePrim::Sort, vec![strings]),
            Eval::Value(Comp::ret(sorted))
        );
        let mixed = Value::list(vec![Value::int(1), Value::string("a")]);
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::Sort, vec![mixed]),
            "a heterogeneous list has no shared order"
        );
        let floats = Value::list(vec![Value::f64(1.5_f64), Value::f64(0.5_f64)]);
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::Sort, vec![floats]),
            "floats are excluded (no total order over NaN)"
        );
        assert_eq!(
            run(NativePrim::Sort, vec![Value::list(vec![Value::f64(
                1.5_f64
            )])]),
            Eval::Value(Comp::ret(Value::list(vec![Value::f64(1.5_f64)]))),
            "a singleton short-circuits the order check"
        );
    }

    /// `sort` orders the sized-integer atoms (`u32`/`u64`/`i32`/`i64`), not
    /// only bare integer literals and strings.
    #[test]
    fn sort_orders_sized_integer_atoms()
    {
        assert_eq!(
            run(NativePrim::Sort, vec![Value::list(vec![
                Value::u32(3_u32),
                Value::u32(1_u32),
                Value::u32(2_u32),
            ])]),
            Eval::Value(Comp::ret(Value::list(vec![
                Value::u32(1_u32),
                Value::u32(2_u32),
                Value::u32(3_u32),
            ])))
        );
        assert_eq!(
            run(NativePrim::Sort, vec![Value::list(vec![
                Value::i64(2_i64),
                Value::i64(-5_i64),
                Value::i64(0_i64),
            ])]),
            Eval::Value(Comp::ret(Value::list(vec![
                Value::i64(-5_i64),
                Value::i64(0_i64),
                Value::i64(2_i64),
            ])))
        );
    }

    /// `get` on a manifest record returns `Some` for a present field and `None`
    /// for an absent one.
    #[test]
    fn get_returns_an_optional_cell()
    {
        let record = Value::record([
            ("a".to_owned(), Value::int(1)),
            ("b".to_owned(), Value::int(2)),
        ]);
        assert_eq!(
            run(NativePrim::Get, vec![record.clone(), Value::string("a")]),
            Eval::Value(Comp::ret(some(Value::int(1))))
        );
        assert_eq!(
            run(NativePrim::Get, vec![record, Value::string("z")]),
            Eval::Value(Comp::ret(none())),
            "an absent field is None"
        );
    }

    /// `insert` extends a record with a new field and overrides an existing
    /// one.
    #[test]
    fn insert_extends_and_overrides_a_record()
    {
        let record = Value::record([("a".to_owned(), Value::int(1))]);
        assert_eq!(
            run(NativePrim::Insert, vec![
                record.clone(),
                Value::string("b"),
                Value::int(2)
            ]),
            Eval::Value(Comp::ret(Value::record([
                ("a".to_owned(), Value::int(1)),
                ("b".to_owned(), Value::int(2)),
            ])))
        );
        assert_eq!(
            run(NativePrim::Insert, vec![
                record,
                Value::string("a"),
                Value::int(9)
            ]),
            Eval::Value(Comp::ret(Value::record([("a".to_owned(), Value::int(9))]))),
            "inserting an existing label overrides it"
        );
    }

    /// A wrong-shape (non-manifest-list / non-record) argument reduces to a
    /// defined blame, not a panic — the gradual-hole discipline of the base
    /// natives.
    #[test]
    fn a_wrong_shape_argument_blames_a_hole()
    {
        let inc = unary_op(NativePrim::Add, Value::int(1));
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::Each, vec![inc, Value::int(5)]),
            "`each` over a non-list is a wrong-shape hole"
        );
        assert_eq!(
            Eval::Blame(Blame::Hole),
            run(NativePrim::Get, vec![Value::int(5), Value::string("a")]),
            "`get` on a non-record is a wrong-shape hole"
        );
    }

    /// A length-64 filter must unroll linearly (a regression guard on the
    /// bound-once `where` unroll).
    #[test]
    fn where_over_a_long_list_is_linear()
    {
        const LONG_LIST_THRESHOLD: i64 = 31;
        let input: Vec<i64> = (0 .. 64).collect();
        let expected: Vec<i64> = (0 .. 64)
            .filter(|&value| value > LONG_LIST_THRESHOLD)
            .collect();
        let gt31 = unary_op(NativePrim::Gt, Value::int(LONG_LIST_THRESHOLD));
        assert_eq!(
            run(NativePrim::Where, vec![gt31, ints((&input).into())]),
            Eval::Value(Comp::ret(ints((&expected).into())))
        );
    }

    /// `reduce` / `any` / `all` build their unrolls with an explicit loop; this
    /// exercises that loop past the n ≤ 3 the other rows use. The L machine is
    /// environment/store-based, so the CEK substitution evaluator's
    /// O(list-length) `subst_comp` stack ceiling (which capped this at n =
    /// 64) no longer binds, but the scale is kept as the pinned regression
    /// witness.
    #[test]
    fn fold_and_quantifiers_are_correct_at_moderate_scale()
    {
        let count: i64 = 64;
        let input: Vec<i64> = (1 ..= count).collect();
        let add = closure2(Comp::app(
            Comp::app(Comp::native(NativePrim::Add), Value::var("acc")),
            Value::var("x"),
        ));
        let expected_sum: i64 = input.iter().copied().sum();
        assert_eq!(
            run(NativePrim::Reduce, vec![
                add,
                Value::int(0),
                ints((&input).into())
            ]),
            Eval::Value(Comp::ret(Value::int(expected_sum))),
            "reduce sums 1..=64 via the iterative left-fold builder"
        );
        assert_eq!(
            run(NativePrim::All, vec![
                unary_op(NativePrim::Gt, Value::int(0)),
                ints((&input).into())
            ]),
            Eval::Value(Comp::ret(boolean((true).into()))),
            "`all (> 0)` holds across the long list"
        );
        assert_eq!(
            run(NativePrim::Any, vec![
                unary_op(NativePrim::Gt, Value::int(count)),
                ints((&input).into())
            ]),
            Eval::Value(Comp::ret(boolean((false).into()))),
            "`any (> max)` finds no witness"
        );
    }

    // ─────────────────────────────────────── deep-handler assert / mock runner

    /// The deep handler resumes past each passing assertion and reports overall
    /// success (a combinator result feeds the assertion; both true → passes).
    #[test]
    fn the_assert_handler_runs_combinator_assertions()
    {
        let all_positive = Comp::app(
            Comp::app(
                Comp::native(NativePrim::All),
                unary_op(NativePrim::Gt, Value::int(0)),
            ),
            ints((&[1, 2, 3]).into()),
        );
        let any_big = Comp::app(
            Comp::app(
                Comp::native(NativePrim::Any),
                unary_op(NativePrim::Gt, Value::int(2)),
            ),
            ints((&[1, 2, 3]).into()),
        );
        let body = Comp::bind(
            all_positive,
            "$b1",
            Comp::bind(
                assert_that(Value::var("$b1")),
                "$u1",
                Comp::bind(
                    any_big,
                    "$b2",
                    Comp::bind(
                        assert_that(Value::var("$b2")),
                        "$u2",
                        Comp::ret(Value::Unit),
                    ),
                ),
            ),
        );
        assert_eq!(
            run_assertions(body),
            Eval::Value(Comp::ret(boolean((true).into()))),
            "both assertions pass, so the runner reports success"
        );
    }

    /// A failing assertion short-circuits the runner to `false` — the handler
    /// declines to resume.
    #[test]
    fn the_assert_handler_reports_a_failure()
    {
        let all_big = Comp::app(
            Comp::app(
                Comp::native(NativePrim::All),
                unary_op(NativePrim::Gt, Value::int(5)),
            ),
            ints((&[1, 2, 3]).into()),
        );
        let body = Comp::bind(
            all_big,
            "$b",
            Comp::bind(assert_that(Value::var("$b")), "$u", Comp::ret(Value::Unit)),
        );
        assert_eq!(
            run_assertions(body),
            Eval::Value(Comp::ret(boolean((false).into()))),
            "a failed assertion short-circuits to failure"
        );
    }

    /// A mock `Exec` handler stands in for a PATH-shadow fake: a deep handler
    /// returns canned output instead of running a real command.
    #[test]
    fn a_mock_exec_handler_replaces_a_real_command()
    {
        let exec_sig = EffectSig::new(EffectSignatureName::from("Exec"), vec![EffectOp::new(
            OperationName::from("exec"),
            ValueType::string(),
            ValueType::string(),
        )]);
        let body = Comp::bind(
            Comp::perform(exec_sig.clone(), "exec", Value::string("ls")),
            "$out",
            Comp::ret(Value::var("$out")),
        );
        let clause = OpClause::new(
            "exec",
            "$cmd",
            "$k",
            Comp::resume(Value::var("$k"), Comp::ret(Value::string("mocked-output"))),
        );
        let handler = Comp::handle(exec_sig, body, "$x", Comp::ret(Value::var("$x")), vec![
            clause,
        ]);
        assert_eq!(
            machine::run_comp(&handler),
            Eval::Value(Comp::ret(Value::string("mocked-output"))),
            "the mock handler returns canned output for the command"
        );
    }

    // ─────────────────────────────────────────────────────────────── helpers

    /// Applies a saturated combinator directly (the natives are in focus) and
    /// runs it on the L machine.
    fn run(
        prim: NativePrim,
        args: Vec<Value>,
    ) -> Eval
    {
        let mut comp = Comp::native(prim);
        for arg in args {
            comp = Comp::app(comp, arg);
        }
        machine::run_comp(&comp)
    }

    /// `Comp::app(Comp::app(Comp::native(prim), lhs), rhs)` — a saturated
    /// binary native application.
    fn native_binary(
        prim: NativePrim,
        lhs: Value,
        rhs: Value,
    ) -> Comp
    {
        Comp::app(Comp::app(Comp::native(prim), lhs), rhs)
    }

    /// The unary closure `thunk_ω (λx. x ⊕ operand)` over a native binary
    /// primitive — the shape `each` / `where` / `any` / `all` consume.
    fn unary_op(
        prim: NativePrim,
        operand: Value,
    ) -> Value
    {
        closure1(Comp::app(
            Comp::app(Comp::native(prim), Value::var("x")),
            operand,
        ))
    }

    /// A pure unary closure `thunk_ω (λx. body)`.
    fn closure1(body: Comp) -> Value
    {
        Value::thunk(Grade::OMEGA, Comp::lam("x", body))
    }

    /// A pure binary closure `thunk_ω (λacc. λx. body)` — the shape `reduce`
    /// consumes.
    fn closure2(body: Comp) -> Value
    {
        Value::thunk(Grade::OMEGA, Comp::lam("acc", Comp::lam("x", body)))
    }

    /// Borrowed integers for one checked-core list fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct IntegerFixtureValues<'values>(&'values [i64]);

    impl<'values, const LENGTH: usize> From<&'values [i64; LENGTH]> for IntegerFixtureValues<'values>
    {
        fn from(values: &'values [i64; LENGTH]) -> Self
        {
            Self(values)
        }
    }

    impl<'values> From<&'values Vec<i64>> for IntegerFixtureValues<'values>
    {
        fn from(values: &'values Vec<i64>) -> Self
        {
            Self(values.as_slice())
        }
    }

    /// A list value from bare integers.
    fn ints(values: IntegerFixtureValues<'_>) -> Value
    {
        Value::list(values.0.iter().copied().map(Value::int).collect())
    }

    // The `Optional` / boolean carriers `get` and the comparison / boolean
    // natives build are annotated sums (`v : ? + 1` / `v : 1 + 1`) in the
    // source. The static focusing translation `𝓕` erases type annotations, so
    // the L machine's readback delivers the **bare** injection — the observable
    // value with its (runtime-irrelevant) type ascription dropped. These
    // constructors therefore build the annotation-erased form the L outcome
    // actually carries, so the ported rows assert L's own stable outcome
    // exactly (the annotated CEK form compared equal to it under the
    // differential's `canonical`, which strips the same annotation on both
    // sides).

    /// The `Optional` value `get` returns for a present field (`Some v = inj1
    /// v`).
    fn some(value: Value) -> Value
    {
        Value::inj1(value)
    }

    /// The `Optional` value `get` returns for an absent field (`None = inj2
    /// ()`).
    fn none() -> Value
    {
        Value::inj2(Value::Unit)
    }

    /// One canonical boolean fixture input.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct BooleanFixture(bool);

    impl From<bool> for BooleanFixture
    {
        fn from(value: bool) -> Self
        {
            Self(value)
        }
    }

    /// The canonical boolean (`true = inj1 ()`, `false = inj2 ()`).
    fn boolean(value: BooleanFixture) -> Value
    {
        if value.0 {
            Value::inj1(Value::Unit)
        }
        else {
            Value::inj2(Value::Unit)
        }
    }

    /// `perform Assert.assert p` on a bool `p`.
    fn assert_that(condition: Value) -> Comp
    {
        Comp::perform(assert_sig(), "assert", condition)
    }

    /// The `Assert` effect: `assert : Bool ↠ 1`.
    fn assert_sig() -> EffectSig
    {
        EffectSig::new(EffectSignatureName::from("Assert"), vec![EffectOp::new(
            OperationName::from("assert"),
            ValueType::sum(ValueType::Unit, ValueType::Unit),
            ValueType::Unit,
        )])
    }

    /// Wraps a body performing `Assert.assert` calls in a deep handler that
    /// resumes on `true` and short-circuits to `false` on the first failed
    /// assertion — a minimal handler-based test runner (answer `Bool`).
    fn run_assertions(body: Comp) -> Eval
    {
        let clause = OpClause::new(
            "assert",
            "$p",
            "$k",
            Comp::case(
                Value::var("$p"),
                "$ok",
                Comp::resume(Value::var("$k"), Comp::ret(Value::Unit)),
                "$bad",
                Comp::ret(boolean((false).into())),
            ),
        );
        let handler = Comp::handle(
            assert_sig(),
            body,
            "$done",
            Comp::ret(boolean((true).into())),
            vec![clause],
        );
        machine::run_comp(&handler)
    }
}
