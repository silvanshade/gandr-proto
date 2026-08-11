//! Expression-lowering coverage (`src/lower.rs`): every syntax-directed
//! handler's happy path, its out-of-fragment / malformed strict errors, and
//! the total-mode hole recovery, driven through `lower_source` (strict) and
//! `lower_source_total` (total).
//!
//! The vehicle is a single `def d = <expr>;` item, whose lowered `term` is the
//! expression's core form; an exact `matches!` guard pins the elaboration.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

#[cfg(test)]
mod tests
{
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Side;
    use gandr_core_checker::syntax::Term;
    use gandr_core_checker::syntax::Value;
    use gandr_core_incremental::region::Item;
    use gandr_surface_engine::goals::goals_report;
    use gandr_surface_engine::lower::LowerError;
    use gandr_surface_engine::lower::lower_source;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::lower::node_kinds;
    use gandr_surface_engine::origin::HoleNote;
    use gandr_surface_engine::prelude_ctx;

    use crate::common::TestText;
    #[test]
    fn every_expression_former_lowers_to_its_core_computation()
    {
        // Each computation-sorted former lowers to its named core node.
        assert!(
            matches!(
                strict_term(
                    "case (Inl(1) : Integer + Integer) { Inl(x) => ret x, Inr(y) => ret y }"
                ),
                Term::Comp(Comp::Case(_, _, _))
            ),
            "a two-arm sum case is `Comp::Case`"
        );
        assert!(
            matches!(
                strict_term("case xs { Nil => ret 0, Cons(h, t) => ret h }"),
                Term::Comp(Comp::ListCase { .. })
            ),
            "a Nil/Cons case is `Comp::ListCase`"
        );
        assert!(
            matches!(
                strict_term("if true { ret 1 } else { ret 2 }"),
                Term::Comp(Comp::Case(_, _, _))
            ),
            "`if`/`else` desugars to a boolean `Comp::Case`"
        );
        // `else if` chains: the alternative is itself an `if` computation.
        assert!(
            matches!(
                strict_term("if true { ret 1 } else if false { ret 2 } else { ret 3 }"),
                Term::Comp(Comp::Case(_, _, ref alt)) if matches!(*alt.1, Comp::Case(_, _, _))
            ),
            "an `else if` nests a second boolean case in the alternative"
        );
        assert!(
            matches!(
                strict_term("co { fst = ret 1, snd = ret 2 }"),
                Term::Comp(Comp::With(_, _))
            ),
            "co fst/snd is the lazy product `Comp::With`"
        );
        assert!(
            matches!(strict_term("t.fst"), Term::Comp(Comp::Prj(Side::Fst, _))),
            "`.fst` is the first projection"
        );
        assert!(
            matches!(strict_term("t.snd"), Term::Comp(Comp::Prj(Side::Snd, _))),
            "`.snd` is the second projection"
        );
        assert!(
            matches!(
                strict_term("r.field"),
                Term::Comp(Comp::RecordProj { ref label, .. }) if label == "field"
            ),
            "a non-structural field is a record projection"
        );
        assert!(
            matches!(strict_term("#{ r | a = 1 }"), Term::Comp(Comp::App(_, _))),
            "a record update elaborates to a native application spine"
        );
        assert!(
            matches!(
                strict_term("1 + 2"),
                Term::Comp(Comp::App(ref outer, _)) if matches!(**outer, Comp::App(_, _))
            ),
            "a binary operator is a two-argument application"
        );
        assert!(
            matches!(strict_term("-1"), Term::Comp(Comp::App(_, _))),
            "unary negation is a one-argument application"
        );
        assert!(
            matches!(strict_term("force t"), Term::Comp(Comp::Force(_))),
            "`force` is `Comp::Force`"
        );
        assert!(
            matches!(strict_term("ret 1"), Term::Comp(Comp::Ret(_))),
            "`ret` is `Comp::Ret`"
        );
    }
    #[test]
    fn every_value_former_lowers_to_its_core_value()
    {
        assert!(
            matches!(strict_term("(1, 2)"), Term::Value(Value::Pair(_, _))),
            "a tuple is a `Value::Pair`"
        );
        // n-ary tuples right-nest.
        assert!(
            matches!(
                strict_term("(1, 2, 3)"),
                Term::Value(Value::Pair(_, ref snd)) if matches!(**snd, Value::Pair(_, _))
            ),
            "a 3-tuple right-nests pairs"
        );
        assert!(
            matches!(strict_term("[1, 2, 3]"), Term::Value(Value::List(ref xs)) if xs.len() == 3),
            "a list literal is a `Value::List`"
        );
        assert!(
            matches!(strict_term("[]"), Term::Value(Value::List(ref xs)) if xs.is_empty()),
            "the empty list literal is an empty `Value::List`"
        );
        assert!(
            matches!(strict_term("Inl(1)"), Term::Value(Value::Inj(Side::Fst, _))),
            "`Inl` injects on the first side"
        );
        assert!(
            matches!(strict_term("Inr(1)"), Term::Value(Value::Inj(Side::Snd, _))),
            "`Inr` injects on the second side"
        );
        assert!(
            matches!(
                strict_term("#{ a = 1, b = 2 }"),
                Term::Value(Value::Record(ref fields)) if fields.len() == 2
            ),
            "a record literal is a `Value::Record`"
        );
        assert!(
            matches!(
                strict_term("#{}"),
                Term::Value(Value::Record(ref fields)) if fields.is_empty()
            ),
            "the empty record literal is an empty `Value::Record`"
        );
        // `true`/`false` are annotated injections into `1 + 1`.
        assert!(
            matches!(
                strict_term("true"),
                Term::Value(Value::Annot(ref inner, _)) if matches!(**inner, Value::Inj(Side::Fst, _))
            ),
            "`true` is `inj1 ()` annotated `Boolean`"
        );
        assert!(
            matches!(
                strict_term("false"),
                Term::Value(Value::Annot(ref inner, _)) if matches!(**inner, Value::Inj(Side::Snd, _))
            ),
            "`false` is `inj2 ()` annotated `Boolean`"
        );
        assert!(
            matches!(strict_term("()"), Term::Value(Value::Unit)),
            "the unit literal is `Value::Unit`"
        );
        assert!(
            matches!(strict_term("x"), Term::Value(Value::Var(ref name)) if name == "x"),
            "a bare identifier is a `Value::Var`"
        );
    }
    #[test]
    fn string_escapes_decode_to_their_control_characters()
    {
        // Every recognized escape, one unrecognized `\q` (drops the backslash),
        // and a `\`-newline line continuation (elides both).
        let term = strict_term("\"a\\nb\\tc\\\\d\\\"e\\0f\\'g\\rh\\qi\"");
        let Term::Value(Value::Str(decoded)) = term
        else {
            panic!("a string literal must lower to `Value::Str`, got {term:?}");
        };
        assert_eq!("a\nb\tc\\d\"e\0f'g\rhqi", decoded);

        // A backslash before an actual newline is a continuation: both are dropped.
        let cont = strict_term("\"line1\\\nline2\"");
        assert_eq!(cont, Term::Value(Value::Str("line1line2".to_owned())));
    }
    #[test]
    fn a_module_namespace_is_not_a_projectable_record()
    {
        // A known member selects to the flat qualified variable (module-select,
        // the module-selection contract): `list.each` ⇒ `Value::Var("list.each")`.
        assert!(
            matches!(
                strict_term("list.each"),
                Term::Value(Value::Var(ref name)) if name == "list.each"
            ),
            "a known module member selects to the qualified variable"
        );
        // A bare selection of an unknown member from a known module is declined
        // (the module-selection contract): a module namespace is not a record value.
        let error = strict_error("list.nonesuch");
        assert!(
            matches!(error, LowerError::Unsupported {
                kind: node_kinds::PROJECTION_EXPRESSION,
                ..
            }),
            "an unknown module member is an unsupported projection, got {error:?}"
        );
    }
    #[test]
    fn block_statements_desugar_onto_the_bind_chain_spine()
    {
        // `let x = v;` with an identifier binds through `Bind(Ret v, x, …)`.
        assert!(
            matches!(
                thunk_body("thunk { val x = 1; ret x }"),
                Comp::Bind(_, ref binder, _) if binder == "x"
            ),
            "a `val` identifier binder names the bind"
        );
        // A wildcard `val _ = v;` discards.
        assert!(
            matches!(
                thunk_body("thunk { val _ = 1; ret 2 }"),
                Comp::Bind(_, ref binder, _) if binder == "_"
            ),
            "a `val _` wildcard discards"
        );
        // A monadic bind `run x <- source;` binds the source computation directly.
        assert!(
            matches!(
                thunk_body("thunk { run x <- ret 1; ret x }"),
                Comp::Bind(_, ref binder, _) if binder == "x"
            ),
            "a `<-` bind names the source result"
        );
        // A non-tail expression statement sequences with a discard.
        assert!(
            matches!(
                thunk_body("thunk { ret 1; ret 2 }"),
                Comp::Bind(_, ref binder, ref rest)
                    if binder == "_" && matches!(**rest, Comp::Ret(_))
            ),
            "an expression statement discard-binds and the tail follows"
        );
        // A tuple pattern `let (a, b) = t;` splits the product.
        assert!(
            matches!(
                thunk_body("thunk { val (a, b) = p; ret a }"),
                Comp::Split { ref fst_name, ref snd_name, .. } if fst_name == "a" && snd_name == "b"
            ),
            "a 2-tuple pattern is a single split"
        );
        // A 3-tuple pattern right-nests splits through a fresh scrutinee.
        assert!(
            matches!(
                thunk_body("thunk { val (a, b, c) = p; ret a }"),
                Comp::Split { fst_name: ref fst, body: ref inner, .. }
                    if fst == "a" && matches!(**inner, Comp::Split { ref fst_name, ref snd_name, .. } if fst_name == "b" && snd_name == "c")
            ),
            "a 3-tuple pattern nests a second split"
        );
    }

    /// Unwraps the sole thunk item's body computation.
    fn thunk_body<'text>(block_source: impl Into<TestText<'text>>) -> Comp
    {
        let block_source = block_source.into().0;
        let Term::Value(Value::Thunk(_, body)) = strict_term(block_source)
        else {
            panic!("expected a thunk value for `{block_source}`");
        };
        (*body).clone()
    }
    #[test]
    fn shell_blocks_lower_to_exec_perform_sequences()
    {
        // A single simple command lowers to `bind (perform Exec::exec …) x. ret x`.
        let Term::Comp(Comp::Bind(ref sole_bound, ref sole_binder, ref sole_tail)) =
            strict_term("#!{ echo hi }")
        else {
            panic!("a shell command must lower to a bind-of-perform");
        };
        assert!(
            matches!(**sole_bound, Comp::Perform(ref sig, ref op, _) if sig.name().as_ref() == "Exec" && op == "exec"),
            "the command is an `Exec::exec` perform"
        );
        assert_ne!(
            sole_binder, "_",
            "the sole command's result is bound, not discarded"
        );
        assert!(
            matches!(**sole_tail, Comp::Ret(_)),
            "the block returns the result"
        );

        // Two commands sequence with a discard-bind between them.
        let Term::Comp(Comp::Bind(_, ref seq_binder, ref seq_rest)) =
            strict_term("#!{ echo hi; ls }")
        else {
            panic!("a two-command block must sequence");
        };
        assert_eq!(
            "_", seq_binder,
            "the non-final command's result is discarded"
        );
        assert!(
            matches!(**seq_rest, Comp::Bind(_, _, _)),
            "the second command follows in the chain"
        );

        // An empty shell block returns unit (no command to run).
        assert!(
            matches!(strict_term("#!{ }"), Term::Comp(Comp::Ret(ref v)) if matches!(**v, Value::Unit)),
            "an empty shell block returns unit"
        );

        // Quoted arguments decode to their string payloads.
        for source in ["#!{ echo \"dq\" }", "#!{ echo 'sq' }", "#!{ echo a b c }"] {
            assert!(
                matches!(
                    strict_term(source),
                    Term::Comp(Comp::Bind(ref bound, _, _))
                        if matches!(**bound, Comp::Perform(_, _, _))
                ),
                "`{source}` lowers to an exec perform"
            );
        }

        // A double-quoted argument carrying an escape sequence has child nodes, so
        // it is out of the no-interpolation fragment.
        assert!(matches!(
            strict_error("#!{ echo \"a\\tb\" }"),
            LowerError::Unsupported {
                kind: node_kinds::DOUBLE_QUOTED_STRING,
                ..
            }
        ));
    }

    /// The sole item's term of a strictly-lowered `def d = …;` source.
    fn strict_term<'text>(expr_source: impl Into<TestText<'text>>) -> Term
    {
        let expr_source = expr_source.into().0;
        let source = format!("def d = {expr_source};");
        let lowered = lower_source((&source).into())
            .unwrap_or_else(|error| panic!("strict lowering of `{expr_source}` failed: {error}"));
        assert_eq!(
            1,
            lowered.items.len(),
            "one item expected for `{expr_source}`"
        );
        lowered.items.into_iter().next().expect("one item").term
    }
    #[test]
    fn def_function_sugar_thunks_a_curried_abstraction()
    {
        use gandr_core_checker::grade::Grade;
        use gandr_core_checker::syntax::Comp;
        use gandr_core_checker::types::CompType;
        use gandr_core_checker::types::Ty;
        use gandr_core_checker::types::ValueType;

        // A fully-annotated `def f(x: A) -> B { … }` derives the ascription
        // `U_ω (A → B)` and thunks a curried abstraction; because the ascription
        // records the parameter type, the `Abs` binder annotation is elided.
        let annotated = sole_item("def f(x: Integer) -> F Integer { ret x }");
        assert_eq!(Some("f"), annotated.name.as_deref());
        assert_eq!(
            annotated.ascription,
            Some(Ty::Value(ValueType::thunk(
                Grade::OMEGA,
                CompType::arrow(
                    ValueType::atom("Integer"),
                    CompType::returner(ValueType::atom("Integer"))
                )
            )))
        );
        assert!(
            matches!(
                annotated.term,
                Term::Value(Value::Thunk(_, ref body))
                    if matches!(**body, Comp::Abs(ref name, None, _) if name == "x")
            ),
            "the derived-ascription abstraction elides its binder annotation: {:?}",
            annotated.term
        );

        // Without a result type the sugar derives no ascription, and the `Abs`
        // keeps its parameter annotation.
        let unascribed = sole_item("def f(x: Integer) { ret x }");
        assert_eq!(
            None, unascribed.ascription,
            "no result type ⇒ no ascription"
        );
        assert!(
            matches!(
                unascribed.term,
                Term::Value(Value::Thunk(_, ref body))
                    if matches!(**body, Comp::Abs(_, Some(_), _))
            ),
            "an unascribed def-function keeps its parameter annotation: {:?}",
            unascribed.term
        );

        // A nullary `def f() { … }` thunks the body directly (binds nothing).
        let nullary = sole_item("def f() { ret 1 }");
        assert!(
            matches!(
                nullary.term,
                Term::Value(Value::Thunk(_, ref body)) if matches!(**body, Comp::Ret(_))
            ),
            "a nullary def-function thunks its body with no abstraction: {:?}",
            nullary.term
        );

        // A multi-parameter annotated def-function nests the derived arrows.
        let curried = sole_item("def f(x: Integer, y: Integer) -> F Integer { ret x }");
        assert_eq!(
            curried.ascription,
            Some(Ty::Value(ValueType::thunk(
                Grade::OMEGA,
                CompType::arrow(
                    ValueType::atom("Integer"),
                    CompType::arrow(
                        ValueType::atom("Integer"),
                        CompType::returner(ValueType::atom("Integer"))
                    )
                )
            ))),
            "each parameter contributes a curried arrow"
        );
    }
    #[test]
    fn a_top_level_value_with_a_hoisted_computation_wraps_into_a_bind()
    {
        use gandr_core_checker::syntax::Comp;

        // A value item that embeds a computation (a call in a tuple slot) leaves
        // a hoist; `finalize_term` coerces the value through `Ret` and wraps the
        // hoist into a leading `Bind`.
        let item = sole_item("def d = (f(1), 2);");
        assert!(
            matches!(
                item.term,
                Term::Comp(Comp::Bind(_, ref binder, ref tail))
                    if binder.starts_with('%') && matches!(**tail, Comp::Ret(_))
            ),
            "the hoisted call becomes a leading bind over a `Ret` of the tuple: {:?}",
            item.term
        );
    }

    /// The sole item of a strictly-lowered whole program.
    fn sole_item<'text>(program: impl Into<TestText<'text>>) -> Item
    {
        let program = program.into().0;
        let lowered = lower_source(program.into())
            .unwrap_or_else(|error| panic!("strict lowering of `{program}` failed: {error}"));
        assert_eq!(1, lowered.items.len(), "one item expected for `{program}`");
        lowered.items.into_iter().next().expect("one item")
    }
    #[test]
    fn out_of_fragment_arm_shapes_are_unsupported()
    {
        // A catch-all wildcard arm is out of the binary-sum fragment.
        assert!(matches!(
            strict_error("case v { _ => ret 0 }"),
            LowerError::Unsupported {
                kind: node_kinds::WILDCARD,
                ..
            }
        ));
        // A non-`Inl`/`Inr` constructor arm.
        assert!(matches!(
            strict_error("case v { Foo(x) => ret x, Inr(y) => ret y }"),
            LowerError::Unsupported {
                kind: node_kinds::CONSTRUCTOR,
                ..
            }
        ));
        // A binary-sum arm binds exactly one argument; two is out of fragment.
        assert!(matches!(
            strict_error("case v { Inl(x, y) => ret x, Inr(y) => ret y }"),
            LowerError::Unsupported {
                kind: node_kinds::CONSTRUCTOR_PATTERN,
                ..
            }
        ));
        // A duplicate arm for one constructor.
        assert!(matches!(
            strict_error("case v { Inl(x) => ret x, Inl(z) => ret z, Inr(y) => ret y }"),
            LowerError::Unsupported {
                kind: node_kinds::ARM,
                ..
            }
        ));
        // `Cons` binds exactly two arguments; one is out of fragment.
        assert!(matches!(
            strict_error("case xs { Nil => ret 0, Cons(h) => ret h }"),
            LowerError::Unsupported {
                kind: node_kinds::CONSTRUCTOR_PATTERN,
                ..
            }
        ));
        // Total mode absorbs a duplicate arm by keeping the first.
        let Term::Comp(Comp::Case(_, ref inl, _)) = total_term(
            "case (Inl(1) : Integer + Integer) { Inl(x) => ret x, Inl(z) => ret z, Inr(y) => ret y }",
        )
        else {
            panic!("expected a total sum case");
        };
        assert!(
            matches!(*inl.1, Comp::Ret(ref v) if matches!(**v, Value::Var(ref n) if n == "x")),
            "total mode keeps the first Inl arm"
        );
    }
    #[test]
    fn co_field_errors_are_strict_and_holed()
    {
        // A missing lazy-product field.
        assert!(matches!(
            strict_error("co { fst = ret 1 }"),
            LowerError::Unsupported {
                kind: node_kinds::CO_EXPRESSION,
                ..
            }
        ));
        // An out-of-fragment field name (only `fst` / `snd` are covered).
        assert!(matches!(
            strict_error("co { foo = ret 1, snd = ret 2 }"),
            LowerError::Unsupported {
                kind: node_kinds::CO_FIELD,
                ..
            }
        ));
        // A duplicate `fst`.
        assert!(matches!(
            strict_error("co { fst = ret 1, fst = ret 2, snd = ret 3 }"),
            LowerError::Unsupported {
                kind: node_kinds::CO_FIELD,
                ..
            }
        ));
        // Total mode fills a missing field with a hole.
        let Term::Comp(Comp::With(ref fst, ref snd)) = total_term("co { fst = ret 1 }")
        else {
            panic!("expected a total `Comp::With`");
        };
        assert!(matches!(**fst, Comp::Ret(_)), "the present fst survives");
        assert!(matches!(**snd, Comp::Hole(_)), "the missing snd holes");
    }
    #[test]
    fn out_of_fragment_statement_patterns_are_unsupported()
    {
        // A constructor pattern in a `let` is out of the covered fragment.
        assert!(matches!(
            strict_error("thunk { val Foo(x) = v; ret 1 }"),
            LowerError::Unsupported {
                kind: node_kinds::CONSTRUCTOR_PATTERN,
                ..
            }
        ));
        // An empty block has no tail computation.
        assert!(matches!(
            strict_error("thunk { }"),
            LowerError::EmptyBlock { .. }
        ));
        // Total mode holes the missing tail rather than inventing `ret ()`.
        assert_eq!(total_notes("thunk { }"), vec![HoleNote::EmptyBlock]);
    }
    #[test]
    fn out_of_fragment_shell_constructs_are_unsupported_strictly_and_holed_totally()
    {
        // Block-level control operators and still-unsupported in-command
        // decorations remain out of the simple-command fragment. Safe
        // `$(...)` host escapes are covered by shell lowering shape tests.
        let cases: &[(&str, &str)] = &[
            ("#!{ ls | grep x }", "pipeline"),
            ("#!{ a && b }", "and_expression"),
            ("#!{ a || b }", "or_expression"),
            ("#!{ echo $HOME }", "variable_expansion"),
            ("#!{ echo hi > out.txt }", "redirection"),
            ("#!{ ! false }", "negation"),
        ];
        for &(source, kind) in cases {
            let error = strict_error(source);
            assert!(
                matches!(error, LowerError::Unsupported { kind: k, .. } if k == kind),
                "`{source}` must be Unsupported `{kind}`, got {error:?}"
            );
            // Total mode never errs on any of these — the block holes.
            let full = format!("def d = {source};");
            assert!(
                lower_source_total((&full).into()).is_ok(),
                "total lowering absorbs `{source}`"
            );
        }
        // A command-local environment assignment (`FOO=1 cmd`) molds as one
        // `environment_assignment` tile, so it reaches the
        // lowerer's decoration rejection and NAMES the construct, rather than
        // collapsing into an unlocalized syntax obligation on the split `=`.
        assert!(
            matches!(strict_error("#!{ FOO=1 echo }"), LowerError::Unsupported {
                kind: node_kinds::ENVIRONMENT_ASSIGNMENT,
                ..
            }),
            "an environment assignment is out of fragment, named as such"
        );
        assert!(
            lower_source_total("def d = #!{ FOO=1 echo };".into()).is_ok(),
            "total lowering absorbs the environment assignment"
        );
        // A `"`-quoted value binds into the SAME assignment token (grammar.js's
        // `choice(pattern_shell_word, /"…"/)` value), so it is one assignment
        // rather than an assignment plus a stray string.
        assert!(
            matches!(
                strict_error("#!{ FOO=\"a b\" echo }"),
                LowerError::Unsupported {
                    kind: node_kinds::ENVIRONMENT_ASSIGNMENT,
                    ..
                }
            ),
            "a quoted assignment value stays inside the assignment"
        );
        // A word whose name part is NOT identifier-shaped is an ordinary shell
        // word, not an assignment: `--color=auto` is one argument (and a bare `=`
        // is a plain word), so neither raises the assignment rejection.
        assert!(
            lower_source("def d = #!{ ls --color=auto };".into()).is_ok(),
            "`--color=auto` is one ordinary shell word, not an assignment"
        );
    }

    /// The first `LowerError` of a strictly-lowered `def d = …;` source.
    fn strict_error<'text>(expr_source: impl Into<TestText<'text>>) -> LowerError
    {
        let expr_source = expr_source.into().0;
        let source = format!("def d = {expr_source};");
        lower_source((&source).into())
            .expect_err(&format!("strict lowering of `{expr_source}` must fail"))
    }
    #[test]
    fn injection_and_record_errors_are_reported()
    {
        // An injection takes exactly one payload.
        assert!(matches!(
            strict_error("Inl(1, 2)"),
            LowerError::Unsupported {
                kind: node_kinds::CALL_EXPRESSION,
                ..
            }
        ));
        // A user constructor is out of fragment.
        assert!(matches!(strict_error("Foo(1)"), LowerError::Unsupported {
            kind: node_kinds::CONSTRUCTOR,
            ..
        }));
        // A duplicate record label is a strict error; total keeps the last.
        assert!(matches!(
            strict_error("#{ a = 1, a = 2 }"),
            LowerError::Unsupported {
                kind: node_kinds::RECORD_FIELD,
                ..
            }
        ));
        let Term::Value(Value::Record(fields)) = total_term("#{ a = 1, a = 2 }")
        else {
            panic!("expected a total record");
        };
        assert_eq!(1, fields.len(), "a duplicate label collapses to one field");
    }

    /// The sole item's term of a totally-lowered `def d = …;` source.
    fn total_term<'text>(expr_source: impl Into<TestText<'text>>) -> Term
    {
        let expr_source = expr_source.into().0;
        let source = format!("def d = {expr_source};");
        let lowered = lower_source_total((&source).into()).expect("total lowering never errs");
        assert_eq!(
            1,
            lowered.items.len(),
            "one item expected for `{expr_source}`"
        );
        lowered.items.into_iter().next().expect("one item").term
    }
    #[test]
    fn missing_case_arms_are_strict_errors_and_total_holes()
    {
        // Sum case: each missing arm names its constructor.
        assert!(matches!(
            strict_error("case (Inl(1) : Integer + Integer) { Inl(x) => ret x }"),
            LowerError::MissingCaseArm {
                constructor: "Inr",
                ..
            }
        ));
        assert!(matches!(
            strict_error("case (Inl(1) : Integer + Integer) { Inr(y) => ret y }"),
            LowerError::MissingCaseArm {
                constructor: "Inl",
                ..
            }
        ));
        // The missing arm's body becomes a hole in total mode; the present one
        // survives.
        let Term::Comp(Comp::Case(_, ref inl, ref inr)) =
            total_term("case (Inl(1) : Integer + Integer) { Inl(x) => ret x }")
        else {
            panic!("expected a total sum case");
        };
        assert!(
            matches!(*inl.1, Comp::Ret(_)),
            "the present Inl arm survives"
        );
        assert!(matches!(*inr.1, Comp::Hole(_)), "the missing Inr arm holes");

        // List case: each missing arm likewise.
        assert!(matches!(
            strict_error("case xs { Nil => ret 0 }"),
            LowerError::MissingCaseArm {
                constructor: "Cons",
                ..
            }
        ));
        assert!(matches!(
            strict_error("case xs { Cons(h, t) => ret h }"),
            LowerError::MissingCaseArm {
                constructor: "Nil",
                ..
            }
        ));
        assert_eq!(total_notes("case xs { Nil => ret 0 }"), vec![
            HoleNote::MissingCaseArm {
                constructor: "Cons"
            }
        ]);
    }
    #[test]
    fn an_unsupported_statement_holes_and_the_chain_continues_in_total_mode()
    {
        // `leta` is out of fragment: the statement binds a hole and the tail
        // survives (statement-local recovery).
        assert_eq!(total_notes("thunk { leta x = 1; ret x }"), vec![
            HoleNote::UnsupportedForm {
                kind: node_kinds::LETA_STATEMENT,
            }
        ]);
        // Strict mode fails fast on the same input.
        assert!(matches!(
            strict_error("thunk { leta x = 1; ret x }"),
            LowerError::Unsupported {
                kind: node_kinds::LETA_STATEMENT,
                ..
            }
        ));
    }

    /// The hole notes of a totally-lowered `def d = …;` source, in goal order.
    fn total_notes<'text>(expr_source: impl Into<TestText<'text>>) -> Vec<HoleNote>
    {
        let expr_source = expr_source.into().0;
        let source = format!("def d = {expr_source};");
        let lowered = lower_source_total((&source).into()).expect("total lowering never errs");
        goals_report(&lowered, &prelude_ctx())
            .into_iter()
            .filter_map(|goal| goal.note)
            .collect()
    }

    #[test]
    fn a_whole_file_parse_failure_is_a_single_hole_item_in_total_mode()
    {
        // Source that does not parse as items at all yields an `ERROR` root:
        // strict mode reports a `Syntax` error, total mode recovers the whole file
        // to one unnamed hole item.
        for garbage in ["}{", "@@@", ")("] {
            assert!(
                matches!(lower_source(garbage.into()), Err(LowerError::Syntax { .. })),
                "`{garbage}` is a strict syntax error"
            );
            let total = lower_source_total(garbage.into()).expect("total lowering never errs");
            assert_eq!(1, total.items.len(), "`{garbage}` recovers to one item");
            let item = &total.items[0];
            assert!(item.name.is_none(), "the whole-file hole item is unnamed");
            assert!(
                matches!(item.term, Term::Value(Value::Hole(_))),
                "the whole-file recovery is a value hole: {:?}",
                item.term
            );
        }
    }
}
