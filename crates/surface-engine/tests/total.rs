//! Total-lowering acceptance tests, one module per class:
//!
//! 1. [`tests::recovery_fixtures`] — `incomplete-input.gandr` and
//!    `parser-recovery.gandr` lower **totally** and type to completion (`Done`
//!    or a clean `Error`), with hole leaves where the source elides.
//! 2. [`tests::conversion_table`] — every input-shaped strict `LowerError`
//!    converts to a hole with its documented `HoleNote` (the lower-module doc's
//!    conversion table), pinned on inline sources; recovery is statement-local.
//! 3. [`tests::always_typeable`] — *the always-typeable property*: for every
//!    `current/` fixture and **every line-prefix truncation** of it, parse →
//!    total-lower → type terminates, on **both** the checker and the machine,
//!    in a type or a clean error — never a panic, never a `LowerError`
//!    (exhaustive loop + proptest).
//! 4. [`tests::goals`] — the goals report lists every hole with byte range,
//!    expected type, and local `Γ`; golden-tested (regenerate with
//!    `GANDR_SURFACE_ENGINE_BLESS=1`).

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Acceptance tests for total lowering and the goals report.
#[cfg(test)]
mod tests
{
    use std::fs;
    use std::io::Write as _;
    use std::path::PathBuf;

    use gandr_core_checker::checker;
    use gandr_core_checker::control::Dir;
    use gandr_core_checker::error::TypeError;
    use gandr_core_checker::machine;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Term;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::types::CompType;
    use gandr_core_checker::types::Ty;
    use gandr_core_checker::types::ValueType;
    use gandr_core_incremental::region::Item;
    use gandr_surface_engine::goals::Goal;
    use gandr_surface_engine::goals::goals_report;
    use gandr_surface_engine::lower::Lowered;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::lower::node_kinds;
    use gandr_surface_engine::origin::HoleNote;
    use gandr_surface_engine::prelude_ctx;

    use crate::common::TestText;

    /// Reads a `current/` fixture by stem.
    fn read_current_fixture<'text>(stem: impl Into<TestText<'text>>) -> String
    {
        let stem = stem.into().0;
        let path_buf = repo_root()
            .join("tests/fixtures/current")
            .join(format!("{stem}.gandr"));
        fs::read_to_string(&path_buf)
            .unwrap_or_else(|error| panic!("fixture {path_buf:?} must be readable: {error}"))
    }

    /// The surface-engine crate root.
    fn repo_root() -> PathBuf
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// Every `current/` fixture stem (the always-typeable property's corpus).
    const CURRENT_FIXTURES: [&str; 13] = [
        "anchor-evidence",
        "benchmark-mixed",
        "cst-to-ast-core",
        "duplicate-entity",
        "edit-boundary-mistakes",
        "grammar-facts-core",
        "highlighting-captures",
        "incomplete-input",
        "incremental-base",
        "incremental-edited",
        "parser-recovery",
        "stale-relocation-base",
        "stale-relocation-edited",
    ];

    /// Total-lowers a source that must succeed (every parseable input must).
    fn lower_total<'text>(source: impl Into<TestText<'text>>) -> Lowered
    {
        let source = source.into().0;
        lower_source_total(source.into())
            .unwrap_or_else(|error| panic!("total lowering must succeed: {error}\n{source}"))
    }

    /// Types one lowered item on **both** implementations — against its
    /// ascription when the sorts match, in inference mode otherwise — and
    /// asserts they agree; returns the shared result. Termination without
    /// panic is the totality evidence.
    fn type_item_both(item: &Item) -> Result<Ty, TypeError>
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
        }
    }

    /// Acceptance class 1: the named recovery fixtures lower and type to
    /// completion under total mode.
    mod recovery_fixtures
    {
        use super::*;

        /// `parser-recovery.gandr` (`def answer = 42;` followed by an
        /// unterminated shell pipeline): the def survives intact and the
        /// pipeline is a hole noted `UnsupportedForm` — statement-local
        /// recovery at item granularity.
        #[test]
        fn parser_recovery_lowers_with_a_noted_hole()
        {
            let source = read_current_fixture("parser-recovery");
            let lowered = lower_total(&source);
            assert_eq!(2, lowered.items.len(), "two items: the def and the hole");

            let answer = &lowered.items[0];
            assert_eq!(Some("answer"), answer.name.as_deref());
            assert_eq!(
                Term::Value(Value::Int(42)),
                answer.term,
                "the def before the damage must lower intact"
            );

            let hole_item = &lowered.items[1];
            assert!(
                matches!(hole_item.term, Term::Value(Value::Hole(_))),
                "the shell pipeline must lower to a hole, got {hole_item:?}"
            );

            let goals = goals_report(&lowered, &prelude_ctx());
            assert_eq!(1, goals.len(), "one goal: the shell pipeline");
            assert_eq!(
                Some(HoleNote::UnsupportedForm {
                    kind: node_kinds::PIPELINE,
                }),
                goals[0].note,
                "the note must record what was elided"
            );
            assert!(
                source.get(goals[0].byte_range.clone()).is_some(),
                "the goal's range must lie within the source"
            );

            for item in &lowered.items {
                let _result = type_item_both(item);
            }
        }

        /// `incomplete-input.gandr` (`fn(x) {` … missing `}`): the melder
        /// preserves the function structure and reports the missing closer as
        /// an expected-completion obligation, so the lambda lowers hole-free
        /// and typing completes with the clean directional error an
        /// unannotated λ has in inference mode — `Done` or `Error`, never a
        /// panic, never a `LowerError`. (The plan's "with hole leaves" phrasing
        /// does not apply to this fixture: there is no elided expression
        /// region.)
        #[test]
        fn incomplete_input_lowers_totally_and_types_cleanly()
        {
            let source = read_current_fixture("incomplete-input");
            let lowered = lower_total(&source);
            assert_eq!(1, lowered.items.len(), "one item: the lambda");
            let item = &lowered.items[0];
            assert!(
                matches!(item.term, Term::Comp(Comp::Abs(..))),
                "the lambda structure must be recovered, got {item:?}"
            );
            let result = type_item_both(item);
            assert!(
                matches!(result, Err(TypeError::StuckExpr { .. })),
                "an unannotated λ at item level is cleanly stuck, got {result:?}"
            );
        }
    }

    /// Acceptance class 2: the strict-error → hole conversion table, pinned
    /// on inline sources.
    mod conversion_table
    {
        use super::*;
        /// `MissingCaseArm` ⇒ the missing arm's body is a hole; the present
        /// arm survives.
        #[test]
        fn missing_case_arm_becomes_a_hole_body()
        {
            let source = "case (Inl(1) : Integer + Integer) { Inl(x) => ret x }";
            let term = sole_term(source);
            let Term::Comp(Comp::Case(_, ref arm_fst, ref arm_snd)) = term
            else {
                panic!("expected a case, got {term:?}");
            };
            assert!(
                matches!(*arm_fst.1, Comp::Ret(_)),
                "the present Inl arm must survive, got {arm_fst:?}"
            );
            assert!(
                matches!(*arm_snd.1, Comp::Hole(_)),
                "the missing Inr arm must be a hole, got {arm_snd:?}"
            );
            assert_eq!(notes(source), vec![HoleNote::MissingCaseArm {
                constructor: "Inr"
            }]);
        }
        /// `EmptyBlock` ⇒ the missing tail is a hole (no `ret ()` is
        /// invented).
        #[test]
        fn empty_block_tail_becomes_a_hole()
        {
            let term = sole_term("thunk { ret 1; }");
            let Term::Value(Value::Thunk(_, ref body)) = term
            else {
                panic!("expected a thunk, got {term:?}");
            };
            assert!(
                matches!(**body, Comp::Bind(_, _, ref tail) if matches!(**tail, Comp::Hole(_))),
                "the missing tail must be a hole, got {body:?}"
            );
            assert_eq!(notes("thunk { ret 1; }"), vec![HoleNote::EmptyBlock]);
        }
        /// `InvalidIntegerLiteral` ⇒ a hole at the literal. A bare float now
        /// lowers to `f64` (the value-model contract), so the invalid case is
        /// an i64-overflowing integer numeral (an out-of-range suffixed
        /// literal behaves likewise).
        #[test]
        fn invalid_literal_becomes_a_hole()
        {
            let term = sole_term("def x = 99999999999999999999;");
            assert!(
                matches!(term, Term::Value(Value::Hole(_))),
                "an overflowing integer literal is a hole, got {term:?}"
            );
            assert_eq!(notes("def x = 99999999999999999999;"), vec![
                HoleNote::InvalidIntegerLiteral {
                    text: "99999999999999999999".to_owned()
                }
            ]);
        }
        /// An ERROR between statements binds a hole on the chain and the
        /// rest of the block survives (statement-local recovery inside
        /// blocks).
        #[test]
        fn error_statement_is_statement_local()
        {
            let term = sole_term("thunk { run x <- ret 1; @@@; ret x }");
            let Term::Value(Value::Thunk(_, ref body)) = term
            else {
                panic!("expected a thunk, got {term:?}");
            };
            let Comp::Bind(_, ref binder, ref rest) = **body
            else {
                panic!("expected the bind chain, got {body:?}");
            };
            assert_eq!("x", binder, "the healthy bind survives");
            assert!(
                matches!(
                    **rest,
                    Comp::Bind(ref bound, _, ref tail)
                        if matches!(**bound, Comp::Hole(_))
                            && matches!(**tail, Comp::Ret(_))
                ),
                "the ERROR binds a hole and the tail survives, got {rest:?}"
            );
        }
        /// A failed `let` right-hand side keeps its binder: the continuation
        /// still resolves `x` (at `Unknown`).
        #[test]
        fn failed_let_value_keeps_the_binder()
        {
            let source = "thunk { val x = #!{oops | nope}; ret x }";
            let term = sole_term(source);
            let Term::Value(Value::Thunk(_, ref body)) = term
            else {
                panic!("expected a thunk, got {term:?}");
            };
            assert!(
                matches!(
                    **body,
                    Comp::Bind(ref bound, ref binder, _)
                        if matches!(**bound, Comp::Hole(_)) && binder == "x"
                ),
                "the binder must survive the elided right-hand side, got {body:?}"
            );
            let lowered = lower_total(source);
            let result = type_item_both(&lowered.items[0]);
            assert!(result.is_ok(), "x : Unknown must flow, got {result:?}");
        }
        /// A hole-headed call still lowers (and types) its arguments: the
        /// elision localizes instead of cascading.
        #[test]
        fn failed_call_head_keeps_the_arguments()
        {
            let term = sole_term("#!{oops | nope}(1)");
            assert!(
                matches!(
                    term,
                    Term::Comp(Comp::App(ref head, ref arg))
                        if matches!(**head, Comp::Hole(_)) && matches!(**arg, Value::Int(1))
                ),
                "the argument must survive the elided head, got {term:?}"
            );
        }

        /// The sole item's term of an inline source under total lowering.
        fn sole_term<'text>(source: impl Into<TestText<'text>>) -> Term
        {
            let source = source.into().0;
            let lowered = lower_total(source);
            assert_eq!(1, lowered.items.len(), "one item expected:\n{source}");
            lowered.items.into_iter().next().expect("one item").term
        }
        /// `Unsupported` ⇒ `UnsupportedForm` hole: an out-of-fragment
        /// statement binds a hole and the chain continues (statement-local).
        #[test]
        fn unsupported_statement_becomes_a_hole_and_the_chain_continues()
        {
            let term = sole_term("thunk { leta x = 1; ret 2 }");
            let Term::Value(Value::Thunk(_, ref body)) = term
            else {
                panic!("expected a thunk, got {term:?}");
            };
            let Comp::Bind(ref bound, ref binder, ref rest) = **body
            else {
                panic!("expected the hole-bound chain, got {body:?}");
            };
            assert!(
                matches!(**bound, Comp::Hole(_)),
                "the unsupported statement must bind a hole, got {bound:?}"
            );
            assert_eq!("_", binder, "the hole is discard-bound");
            assert!(
                matches!(**rest, Comp::Ret(_)),
                "the tail after the elision must survive, got {rest:?}"
            );
            assert_eq!(notes("thunk { leta x = 1; ret 2 }"), vec![
                HoleNote::UnsupportedForm {
                    kind: node_kinds::LETA_STATEMENT,
                }
            ]);
        }
        /// `InvalidGrade` ⇒ the whole graded construct is a hole (grades
        /// have no unknown representative).
        #[test]
        fn invalid_grade_elides_the_thunk()
        {
            let term = sole_term("def t = thunk[r] { ret 1 };");
            assert!(
                matches!(term, Term::Value(Value::Hole(_))),
                "a grade-variable thunk is a hole, got {term:?}"
            );
            assert_eq!(notes("def t = thunk[r] { ret 1 };"), vec![
                HoleNote::InvalidGrade {
                    text: "r".to_owned()
                }
            ]);
        }

        /// The notes of all goals of an inline source, in goal order.
        fn notes<'text>(source: impl Into<TestText<'text>>) -> Vec<HoleNote>
        {
            let source = source.into().0;
            let lowered = lower_total(source);
            goals_report(&lowered, &prelude_ctx())
                .into_iter()
                .filter_map(|goal| goal.note)
                .collect()
        }

        /// `DanglingSignature` ⇒ an item whose hole term is the missing
        /// definition, ascribed the signature — the goal carries the
        /// signature as its expected type.
        #[test]
        fn dangling_signature_becomes_a_definition_hole_with_the_signature_as_goal()
        {
            let lowered = lower_total("def f : Integer;");
            assert_eq!(1, lowered.items.len());
            let item = &lowered.items[0];
            assert_eq!(Some("f"), item.name.as_deref());
            assert_eq!(item.ascription, Some(Ty::Value(ValueType::integer())));
            assert!(matches!(item.term, Term::Value(Value::Hole(_))));

            let goals = goals_report(&lowered, &prelude_ctx());
            assert_eq!(1, goals.len());
            assert_eq!(
                goals[0].note,
                Some(HoleNote::MissingDefinition {
                    name: "f".to_owned()
                })
            );
            assert_eq!(
                goals[0].expected,
                Some(Ty::Value(ValueType::integer())),
                "the signature is the hole's goal"
            );
        }

        /// The `Unknown` primitive keyword lowers to the gradual top
        /// [`ValueType::Unknown`] — the consistency hole an ascription names
        /// explicitly — not the rigid `atom("Unknown")` the opaque fallback
        /// previously produced.
        #[test]
        fn unknown_keyword_lowers_to_the_gradual_top_not_an_atom()
        {
            let lowered = lower_total("def u : Unknown;");
            assert_eq!(1, lowered.items.len());
            let item = &lowered.items[0];
            assert_eq!(Some("u"), item.name.as_deref());
            assert_eq!(
                Some(Ty::Value(ValueType::Unknown)),
                item.ascription,
                "`: Unknown` is the gradual top, not a rigid base atom"
            );
            assert_ne!(
                item.ascription,
                Some(Ty::Value(ValueType::atom("Unknown"))),
                "the pre-fix bug lowered `: Unknown` to the rigid atom(\"Unknown\")"
            );
        }

        /// `TypeSortMismatch` ⇒ the type position lowers to `Unknown` (no
        /// note; the `Unknown` is the signal): the ascription survives
        /// partially.
        #[test]
        fn sort_mismatched_type_lowers_to_unknown()
        {
            let lowered = lower_total("def s : F Unit -> F Unit;\ndef s = thunk { ret () };");
            let item = lowered
                .items
                .iter()
                .find(|item| item.name.as_deref() == Some("s"))
                .expect("the s item");
            let Some(Ty::Comp(ref ascription)) = item.ascription
            else {
                panic!(
                    "a partially-unknown comp ascription, got {:?}",
                    item.ascription
                );
            };
            assert_eq!(
                format!("{ascription:?}"),
                "Arrow(Unknown, F(Unit))",
                "the sort-mismatched argument is Unknown; the rest survives"
            );
        }

        /// ERROR items recover at item granularity: the surrounding defs
        /// survive and the damage is one noted hole item.
        #[test]
        fn error_item_is_statement_local()
        {
            let lowered = lower_total("def a = 1;\ndef b = @@@;\ndef c = 2;\n");
            assert_eq!(3, lowered.items.len(), "both healthy defs survive");
            assert_eq!(Some("a"), lowered.items[0].name.as_deref());
            assert_eq!(Term::Value(Value::Int(1)), lowered.items[0].term);
            assert!(
                matches!(lowered.items[1].term, Term::Value(Value::Hole(_))),
                "the damaged region is a hole"
            );
            assert_eq!(Some("c"), lowered.items[2].name.as_deref());
            assert_eq!(Term::Value(Value::Int(2)), lowered.items[2].term);
            let goals = goals_report(&lowered, &prelude_ctx());
            assert_eq!(1, goals.len());
            // The `@@@` region degrouts to three `UnmoldedTok` obligations; the
            // item hole takes the note of the responsible one.
            assert_eq!(Some(HoleNote::UnrecognizedToken), goals[0].note);
        }

        /// A string literal lowers to `Value::Str` with the grammar's escape
        /// sequences decoded, infers the rigid `String` atom on both
        /// implementations, and checks against a `: String` annotation (the
        /// type keyword lowers to the same atom). The value-model ladder's
        /// first scalar rung (the value-model contract).
        #[test]
        fn string_literal_lowers_types_and_checks()
        {
            // Lowering: the delimiting quotes are stripped and `\n` / `\"`
            // are decoded to a newline / a quote.
            let lowered = lower_total("def s = \"hi\\nthere\\\"!\";\n");
            assert_eq!(1, lowered.items.len());
            assert_eq!(Some("s"), lowered.items[0].name.as_deref());
            assert_eq!(
                lowered.items[0].term,
                Term::Value(Value::Str("hi\nthere\"!".to_owned())),
                "the string literal must lower with escapes decoded"
            );

            // Typing: it infers the rigid `String` atom, checker == machine.
            let inferred = type_item_both(&lowered.items[0]);
            assert_eq!(
                inferred,
                Ok(Ty::Value(ValueType::string())),
                "a string literal infers the String atom"
            );

            // A `: String` annotation checks: the type keyword lowers to the
            // same rigid atom the literal synthesizes.
            let annotated = lower_total("def greeting = (\"hello\" : String);\n");
            let result = type_item_both(&annotated.items[0]);
            assert_eq!(
                result,
                Ok(Ty::Value(ValueType::string())),
                "a string literal checks against a String annotation"
            );

            // A backslash before an actual newline is a line continuation
            // (grammar `escape_sequence` = `\` + `\r?\n`): both are elided.
            let continued = lower_total("def cont = \"a\\\nb\";\n");
            assert_eq!(
                continued.items[0].term,
                Term::Value(Value::Str("ab".to_owned())),
                "a line continuation elides the backslash and the newline"
            );
        }

        /// A type-suffixed numeric literal lowers to the `Value::Num` of its
        /// suffix and infers (and checks against) that rigid atom on both
        /// implementations, monomorphically; a bare integer stays `Integer` but
        /// also checks against a sized atom it fits (the Rust `{integer}`
        /// rule); a bare float lowers to `f64`. The value-model
        /// ladder's numeric primitive rung (the value-model contract).
        #[test]
        fn numeric_literals_lower_type_and_check()
        {
            // Lowering: a suffixed literal lowers to the matching `Value::Num`.
            let lowered = lower_total("def port = 8080u32;\n");
            assert_eq!(
                lowered.items[0].term,
                Term::Value(Value::u32(8080)),
                "a u32-suffixed literal must lower to Value::Num(U32)"
            );
            assert_eq!(
                type_item_both(&lowered.items[0]),
                Ok(Ty::Value(ValueType::u32())),
                "a u32 literal infers the u32 atom"
            );

            // A float suffix on integral digits is the float value (`2f32` is
            // `2.0f32`); a bare float defaults to f64 (the value-model contract).
            assert_eq!(
                lower_total("def half = 1.5f32;\n").items[0].term,
                Term::Value(Value::f32(1.5)),
                "1.5f32 must lower to Value::Num(F32)"
            );
            assert_eq!(
                lower_total("def two = 2f64;\n").items[0].term,
                Term::Value(Value::f64(2.0_f64)),
                "2f64 must lower to Value::Num(F64) = 2.0"
            );
            let bare_float = lower_total("def rate = 2.5;\n");
            assert_eq!(
                bare_float.items[0].term,
                Term::Value(Value::f64(2.5_f64)),
                "a bare float literal must lower to f64"
            );
            assert_eq!(
                type_item_both(&bare_float.items[0]),
                Ok(Ty::Value(ValueType::f64())),
                "a bare float infers f64"
            );

            // A `: u32` annotation checks against the suffixed literal's atom
            // (the type keyword lowers to the same rigid atom).
            let annotated = lower_total("def p = (8080u32 : u32);\n");
            assert_eq!(
                type_item_both(&annotated.items[0]),
                Ok(Ty::Value(ValueType::u32())),
                "a u32 literal checks against a : u32 annotation"
            );

            // The Rust `{integer}` rule (the value-model contract): a bare integer literal
            // checks against a sized integer atom it fits.
            let widened = lower_total("def n = (8080 : u32);\n");
            assert_eq!(
                type_item_both(&widened.items[0]),
                Ok(Ty::Value(ValueType::u32())),
                "a bare integer literal checks against u32 when it fits"
            );
        }

        /// Hole identifiers are unique across one lowering.
        #[test]
        fn hole_identifiers_are_unique()
        {
            let lowered = lower_total(
                "def a = 99999999999999999999;\ndef b : Sess;\nthunk { leta x = 1; ret 2 };\n",
            );
            let goals = goals_report(&lowered, &prelude_ctx());
            let mut ids: Vec<u32> = goals.iter().map(|goal| goal.hole).collect();
            assert!(goals.len() >= 3, "several holes expected, got {goals:?}");
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), goals.len(), "hole identifiers must be unique");
        }
    }

    /// Acceptance class 3: the always-typeable property.
    mod always_typeable
    {
        use proptest::prelude::*;

        use super::*;
        /// Exhaustive: every line prefix of every `current/` fixture parses,
        /// lowers totally, and types to `Done` or a clean `Error` on both
        /// implementations.
        #[test]
        fn every_line_prefix_of_every_fixture_is_typeable()
        {
            for stem in CURRENT_FIXTURES {
                let source = read_current_fixture(stem);
                let lines: Vec<&str> = source.split_inclusive('\n').collect();
                for cut in 0 ..= lines.len() {
                    let prefix: String = lines[.. cut].concat();
                    always_typeable(&prefix);
                }
            }
        }

        /// Checks the property for one source: total lowering succeeds and
        /// every item types on both implementations (agreement asserted in
        /// [`type_item_both`]).
        fn always_typeable<'text>(source: impl Into<TestText<'text>>)
        {
            let source = source.into().0;
            let lowered = lower_source_total(source.into()).unwrap_or_else(|error| {
                panic!("total lowering must succeed on every parseable input: {error}\n{source}")
            });
            for item in &lowered.items {
                let _result = type_item_both(item);
            }
        }

        proptest! {
            /// Randomized re-statement of the property (the design asks for a
            /// proptest), additionally cutting *within* the final line at a
            /// char boundary — strictly more prefixes than the exhaustive
            /// line-prefix loop.
            #[test]
            fn random_prefixes_are_typeable(
                fixture in 0_usize .. CURRENT_FIXTURES.len(),
                lines_kept in 0_usize .. 64_usize,
                tail_chars in 0_usize .. 120_usize,
            )
            {
                let source = read_current_fixture(CURRENT_FIXTURES[fixture]);
                let lines: Vec<&str> = source.split_inclusive('\n').collect();
                let cut = lines_kept.min(lines.len());
                let mut prefix: String = lines[.. cut].concat();
                if let Some(line) = lines.get(cut) {
                    prefix.extend(line.chars().take(tail_chars));
                }
                always_typeable(&prefix);
            }
        }
    }

    /// Acceptance class 4: the goals report, golden-tested.
    mod goals
    {
        use super::*;
        /// The parser-recovery fixture's goals report.
        #[test]
        fn parser_recovery_goals_match_golden()
        {
            let source = read_current_fixture("parser-recovery");
            check_golden("parser-recovery", &source);
        }
        /// A constructed source exercising goal shape: a checking-position
        /// hole inside a `def` with a signature (expected type + local `Γ`
        /// from binders), a missing case arm, and a dangling signature.
        #[test]
        fn constructed_goals_match_golden()
        {
            let source = "def f : U[1] (Integer -> F Integer);\n\
                          def f(x: Integer) -> F Integer { leta y = x; ret x }\n\
                          def g : Boolean;\n";
            check_golden("constructed", source);
        }

        /// Compares (or, under `GANDR_SURFACE_ENGINE_BLESS`, rewrites) one
        /// golden goals report.
        fn check_golden<'name, 'source>(
            name: impl Into<TestText<'name>>,
            source: impl Into<TestText<'source>>,
        )
        {
            let name = name.into().0;
            let source = source.into().0;
            let lowered = lower_total(source);
            let rendered = render(&goals_report(&lowered, &prelude_ctx()));
            let path_buf = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"))
                .join(format!("{name}.goals.snap"));
            if std::env::var_os("GANDR_SURFACE_ENGINE_BLESS").is_some() {
                let mut file = fs::File::create(&path_buf)
                    .unwrap_or_else(|error| panic!("blessing {path_buf:?} must open: {error}"));
                file.write_all(rendered.as_bytes())
                    .unwrap_or_else(|error| panic!("blessing {path_buf:?} must write: {error}"));
                return;
            }
            let expected = fs::read_to_string(&path_buf)
                .unwrap_or_else(|error| panic!("golden {path_buf:?} must be committed: {error}"));
            assert_eq!(
                rendered, expected,
                "goals report for {name} drifted (GANDR_SURFACE_ENGINE_BLESS=1 to regenerate)"
            );
        }

        /// A deterministic rendering of a goals report.
        fn render(goals: &[Goal]) -> String
        {
            let mut lines = Vec::new();
            for goal in goals {
                lines.push(format!(
                    "hole {} @ item {} path {:?} bytes {:?}\n  note: {:?}\n  expected: {:?}\n  ctx_local: {:?}\n",
                    goal.hole,
                    goal.item,
                    goal.path,
                    goal.byte_range,
                    goal.note,
                    goal.expected,
                    goal.ctx_local,
                ));
            }
            lines.concat()
        }

        /// An inference-position hole (the bound premise of `Bind` is always
        /// inferred) reports no expected type — there is no goal type to
        /// serve — but does report the binders in scope.
        #[test]
        fn inference_position_hole_reports_local_ctx_without_expectation()
        {
            let lowered = lower_total(
                "def f : U[1] (Integer -> F Integer);\n\
                 def f(x: Integer) -> F Integer { leta y = x; ret x }\n",
            );
            let goals = goals_report(&lowered, &prelude_ctx());
            assert_eq!(1, goals.len(), "one goal (the leta), got {goals:?}");
            let goal = &goals[0];
            assert_eq!(
                Some(HoleNote::UnsupportedForm {
                    kind: node_kinds::LETA_STATEMENT,
                }),
                goal.note
            );
            assert_eq!(
                None, goal.expected,
                "Bind's bound premise is inferred: no goal type at this hole"
            );
            assert_eq!(
                goal.ctx_local.as_deref(),
                Some(&[("x".to_owned(), ValueType::integer())][..]),
                "the binder x : Integer is in scope at the hole; the prelude is implied"
            );
        }

        /// A checking-position hole (a missing block tail under a thunk
        /// signature: rule Bind⇕ sends the ascription's `F Integer` into the
        /// continuation) reports the expectation flowing into it, with the
        /// sequencing binder in scope.
        ///
        /// (A missing *case arm* would be the more natural witness, but the
        /// `def`-function sugar produces annotated binders, and rule Abs⇑
        /// for annotated binders infers its body — so a case under the
        /// sugar never receives a checking direction at Stage 1. Recorded
        /// honestly; the dangling-signature test pins the other checked
        /// shape.)
        #[test]
        fn checking_position_hole_reports_the_expectation()
        {
            let lowered = lower_total("def k : U[1] (F Integer);\ndef k = thunk { ret 1; };\n");
            let goals = goals_report(&lowered, &prelude_ctx());
            assert_eq!(1, goals.len(), "one goal (the missing tail), got {goals:?}");
            let goal = &goals[0];
            assert_eq!(Some(HoleNote::EmptyBlock), goal.note);
            assert_eq!(
                goal.expected,
                Some(Ty::Comp(CompType::returner(ValueType::integer()))),
                "the missing tail checks against the signature's F Integer"
            );
            assert_eq!(
                goal.ctx_local.as_deref(),
                Some(&[("_".to_owned(), ValueType::integer())][..]),
                "the sequencing discard binder is in scope at the hole"
            );
        }
    }
}
