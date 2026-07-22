//! A2.1 acceptance tests (`A2-PLAN.md` §A2.1), one module per class:
//!
//! 1. [`tests::fixture_roundtrip`] — `cst-to-ast-core.gandr` lowers and
//!    `square` type-checks against its recorded ascription with
//!    `prelude_ctx()`, via both the checker and the machine.
//! 2. [`tests::golden_lowering`] — committed snapshots for every in-fragment
//!    `current/` fixture (regenerate with `GANDR_SURFACE_ENGINE_BLESS=1`).
//! 3. [`tests::totality`] — every manifest fixture (excluding the
//!    `unsupported/` category) lowers to `Lowered` or a structured
//!    `LowerError`, never a panic; plus a truncation proptest.
//! 4. [`tests::origin_invariants`] — every origin path resolves, byte ranges
//!    nest along paths, elaborated nodes carry `ElabKind`.
//! 5. Core conformance staying green is `cargo test -p gandr-core-checker`
//!    (gate).
//! 6. Workspace gates are `treefmt --ci` plus build, clippy, and test gates.
//!
//! [`tests::lowering_shapes`] additionally pins the lowering fragment's
//! corners (statements, patterns, errors) on inline sources.

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
        reason = "the standard test-allow set keeps acceptance tests readable (docs/workflow/rust.md)"
    )
)]

/// Acceptance tests for the `gandr-surface-engine` public API.
#[cfg(test)]
mod tests
{
    use std::fs;
    use std::io::Write as _;
    use std::path::PathBuf;

    use gandr_core_checker::checker;
    use gandr_core_checker::control::Dir;
    use gandr_core_checker::ctx::Ctx;
    use gandr_core_checker::effect::EffectRow;
    use gandr_core_checker::error::TypeError;
    use gandr_core_checker::grade::Grade;
    use gandr_core_checker::machine;
    use gandr_core_checker::syntax::Term;
    use gandr_core_checker::types::CompType;
    use gandr_core_checker::types::Ty;
    use gandr_core_checker::types::ValueType;
    use gandr_surface_engine::boundary::SourceRange;
    use gandr_surface_engine::goals::goals_report;
    use gandr_surface_engine::host;
    use gandr_surface_engine::lower::LowerError;
    use gandr_surface_engine::lower::Lowered;
    use gandr_surface_engine::lower::LoweredItem;
    use gandr_surface_engine::lower::lower_source;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::lower::node_kinds;
    use gandr_surface_engine::origin::ElabKind;
    use gandr_surface_engine::origin::HoleNote;
    use gandr_surface_engine::origin::TermRef;
    use gandr_surface_engine::origin::resolve;
    use gandr_surface_engine::prelude_ctx;

    use crate::TestCount;
    use crate::TestDecision;
    use crate::TestOwnedPath;
    use crate::TestPath;
    use crate::TestPathComponent;
    use crate::TestText;

    /// The `current/` fixtures inside the A2.1 covered fragment: these must
    /// lower without error and have committed golden snapshots.
    const IN_FRAGMENT_FIXTURES: [&str; 8] = [
        "anchor-evidence",
        "cst-to-ast-core",
        "duplicate-entity",
        "edit-boundary-mistakes",
        "incremental-base",
        "incremental-edited",
        "stale-relocation-base",
        "stale-relocation-edited",
    ];

    /// Lowers a `current/` fixture that must be in-fragment.
    fn lower_current_fixture<'text>(stem: impl Into<TestText<'text>>) -> Lowered
    {
        let stem = stem.into().0;
        let source = read_current_fixture(stem);
        lower_source((&source).into())
            .unwrap_or_else(|error| panic!("fixture {stem} must lower without error: {error}"))
    }

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

    /// Every local current-fixture path exercised by totality.
    fn manifest_source_paths() -> Vec<String>
    {
        let root = repo_root();
        let fixture_dir = root.join("tests/fixtures/current");
        let mut paths: Vec<String> = fs::read_dir(&fixture_dir)
            .unwrap_or_else(|error| {
                panic!("fixture directory {fixture_dir:?} must be readable: {error}")
            })
            .map(|entry| {
                entry.unwrap_or_else(|error| {
                    panic!("every fixture entry in {fixture_dir:?} must be readable: {error}")
                })
            })
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "gandr")
            })
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap_or_else(|error| {
                        panic!("fixture {path:?} must be under crate root {root:?}: {error}")
                    })
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        paths.sort();
        assert!(
            !paths.is_empty(),
            "the current fixture directory must not be empty"
        );
        paths
    }

    /// Acceptance class 1: the `cst-to-ast-core.gandr` round trip — lower,
    /// then type-check `square` against its recorded ascription with
    /// `prelude_ctx()`, via both implementations, agreeing on the type.
    mod fixture_roundtrip
    {
        use super::*;
        /// `square` lowers with the sugar-derived ascription and type-checks
        /// on both the recursive checker and the machine (run to `Done`).
        #[test]
        fn square_type_checks_against_its_ascription()
        {
            let lowered = lower_current_fixture("cst-to-ast-core");
            let item = lowered
                .items
                .iter()
                .find(|item| item.name.as_deref() == Some("square"))
                .expect("the square item must be present");

            let expected = square_ty();
            assert_eq!(
                item.ascription,
                Some(Ty::Value(expected.clone())),
                "the def sugar must record U_ω (Integer → F Integer)"
            );
            let Term::Value(ref value) = item.term
            else {
                panic!("the def sugar must produce a thunk value");
            };

            let checked = checker::check_value(prelude_ctx(), value.clone(), expected.clone());
            assert_eq!(
                checked,
                Ok(expected.clone()),
                "the recursive checker must accept square"
            );

            let (machine_result, _trace) =
                machine::run_value(prelude_ctx(), value.clone(), Dir::Check(expected.clone()));
            assert_eq!(
                machine_result,
                Ok(Ty::Value(expected)),
                "the machine must run square to Done with the same type"
            );
        }

        /// The recorded ascription for `square`: `U_ω (Integer → F Integer)`.
        fn square_ty() -> ValueType
        {
            ValueType::thunk(
                Grade::OMEGA,
                CompType::arrow(
                    ValueType::integer(),
                    CompType::returner(ValueType::integer()),
                ),
            )
        }

        /// The fixture's second item (the trailing lambda) lowers to an
        /// unannotated abstraction over the `if`-as-`case` sugar.
        #[test]
        fn trailing_lambda_lowers_to_case_sugar()
        {
            let lowered = lower_current_fixture("cst-to-ast-core");
            assert_eq!(2, lowered.items.len(), "the fixture has two items");
            let item = lowered.items.last().expect("two items");
            assert_eq!(None, item.name, "the trailing item is unnamed");
            assert!(
                matches!(item.term, Term::Comp(_)),
                "a lambda lowers to a computation"
            );
        }
    }

    /// Acceptance class 2: golden lowering snapshots for every in-fragment
    /// `current/` fixture. Regenerate with `GANDR_SURFACE_ENGINE_BLESS=1`.
    mod golden_lowering
    {
        use super::*;
        /// Every in-fragment fixture matches its committed snapshot.
        #[test]
        fn in_fragment_fixtures_match_their_snapshots()
        {
            for stem in IN_FRAGMENT_FIXTURES {
                check_golden(stem);
            }
        }

        /// Compares (or, under `GANDR_SURFACE_ENGINE_BLESS`, rewrites) one
        /// fixture's snapshot.
        fn check_golden<'text>(stem: impl Into<TestText<'text>>)
        {
            let stem = stem.into().0;
            let snapshot = lower_current_fixture(stem).debug_snapshot();
            let path = golden_path(stem);
            if std::env::var_os("GANDR_SURFACE_ENGINE_BLESS").is_some() {
                let mut file = fs::File::create(&path)
                    .unwrap_or_else(|error| panic!("blessing {path:?} must open: {error}"));
                file.write_all(snapshot.as_bytes())
                    .unwrap_or_else(|error| panic!("blessing {path:?} must write: {error}"));
                return;
            }
            let expected = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("golden {path:?} must be committed: {error}"));
            assert_eq!(
                snapshot, expected,
                "lowering of {stem} drifted from its golden snapshot \
                 (GANDR_SURFACE_ENGINE_BLESS=1 to regenerate)"
            );
        }

        /// The committed snapshot path for a fixture stem.
        fn golden_path<'text>(stem: impl Into<TestText<'text>>) -> PathBuf
        {
            let stem = stem.into().0;
            PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"))
                .join(format!("{stem}.snap"))
        }
    }

    /// Acceptance class 3: no-panic totality over the contract corpus.
    mod totality
    {
        use proptest::prelude::*;

        use super::*;

        /// Every manifest fixture outside the `unsupported/` category parses
        /// and lowers to `Lowered` or a structured `LowerError` (returning
        /// at all is the no-panic evidence).
        #[test]
        fn corpus_lowers_or_errors_without_panic()
        {
            let mut seen = 0_u32;
            for relative in manifest_source_paths() {
                if relative.contains("/unsupported/") {
                    continue;
                }
                let path_buf = repo_root().join(&relative);
                let source = fs::read_to_string(&path_buf)
                    .unwrap_or_else(|error| panic!("fixture {path_buf:?} must read: {error}"));
                // Returning at all — `Lowered` or a structured error — is
                // the no-panic evidence.
                let _outcome = lower_source((&source).into());
                seen += 1;
            }
            assert!(
                seen >= 10,
                "the manifest must cover the corpus (saw {seen})"
            );
        }

        /// A predicate over the expected structured error of one fixture.
        type ErrorPredicate = fn(&LowerError) -> bool;

        /// Known out-of-fragment fixtures fail with *structured* errors.
        #[test]
        fn out_of_fragment_fixtures_yield_structured_errors()
        {
            let cases: [(&str, ErrorPredicate); 4] = [
                ("parser-recovery", |error| {
                    matches!(*error, LowerError::Syntax { .. })
                }),
                ("incomplete-input", |error| {
                    matches!(*error, LowerError::Syntax { .. })
                }),
                // This fixture's shell block is `FOO=bar echo … | grep … > …`.
                // The env-assignment now molds as one `environment_assignment`
                // tile, so the parse no longer raises a syntax
                // obligation on the split `=` and lowering reaches the OUTERMOST
                // out-of-fragment construct — the A8 pipeline — naming it, rather
                // than reporting an unlocalized `Syntax` error for the whole block.
                ("grammar-facts-core", |error| {
                    matches!(*error, LowerError::Unsupported {
                        kind: node_kinds::PIPELINE,
                        ..
                    })
                }),
                // `List(Integer)` is now in-fragment (the surface contract), so this fixture
                // lowers its `map_square` and first-errors on the A8 shell
                // pipeline inside the shell block.
                ("benchmark-mixed", |error| {
                    matches!(*error, LowerError::Unsupported {
                        kind: node_kinds::PIPELINE,
                        ..
                    })
                }),
            ];
            for (stem, matches_expected) in cases {
                let source = read_current_fixture(stem);
                let error = lower_source((&source).into())
                    .err()
                    .unwrap_or_else(|| panic!("{stem} must be out of fragment"));
                assert!(
                    matches_expected(&error),
                    "{stem} must fail with the expected structured error, got: {error}"
                );
            }

            // A *non*-`List` type application is still out of fragment (only
            // `List(A)` lowers; the surface contract), yielding a structured
            // `type_application` error — the coverage `benchmark-mixed` no longer
            // provides now that its `List(Integer)` is in-fragment.
            let non_list = lower_source("def g = (() : Foo(Integer));\n".into())
                .expect_err("a non-List type application is out of fragment");
            assert!(
                matches!(non_list, LowerError::Unsupported {
                    kind: node_kinds::TYPE_APPLICATION,
                    ..
                }),
                "a non-List type application must fail with a structured type_application error, \
                 got: {non_list}"
            );
        }

        proptest! {
            /// Byte-truncations of one fixture never panic the lowerer (full
            /// truncation-typeability is A2.2's gate).
            #[test]
            fn truncations_never_panic(cut in 0_usize .. 100_usize)
            {
                let source = read_current_fixture("cst-to-ast-core");
                let bounded = cut.min(source.len());
                // The fixture is ASCII, so every byte index is a char
                // boundary; `get` keeps this total regardless. Returning is
                // the no-panic evidence.
                if let Some(prefix) = source.get(.. bounded) {
                    let _outcome = lower_source(prefix.into());
                }
            }
        }
    }

    /// Acceptance class 4: origin-map invariants over the in-fragment
    /// fixtures.
    mod origin_invariants
    {
        use super::*;

        /// Every recorded term path resolves against its item's term; every
        /// prefix is also recorded; byte ranges nest along the path and lie
        /// within the source.
        #[test]
        fn paths_resolve_and_ranges_nest()
        {
            for stem in IN_FRAGMENT_FIXTURES {
                let source = read_current_fixture(stem);
                let lowered = lower_source((&source).into())
                    .unwrap_or_else(|error| panic!("{stem} must lower: {error}"));
                assert!(
                    !lowered.origin.is_empty(),
                    "{stem}: the origin map must not be empty"
                );
                for (path, _id, entry) in lowered.origin.iter_paths() {
                    let Some((&item_index, term_path)) = path.split_first()
                    else {
                        panic!("{stem}: paths must start with an item index");
                    };
                    let item = lowered
                        .items
                        .get(usize::try_from(item_index).expect("item index fits usize"))
                        .unwrap_or_else(|| panic!("{stem}: item {item_index} must exist"));
                    assert!(
                        resolve(&item.term, term_path).is_some(),
                        "{stem}: path {path:?} must resolve in the lowered term"
                    );
                    assert!(
                        entry.byte_range.end <= source.len(),
                        "{stem}: {path:?} range must lie within the source"
                    );
                    // Ranges nest along the path: every proper prefix is
                    // recorded and contains this entry's range.
                    for cut in 1 .. path.len() {
                        let prefix = path.get(.. cut).expect("prefix within path");
                        let ancestor = lowered.origin.get_path(prefix).unwrap_or_else(|| {
                            panic!("{stem}: prefix {prefix:?} must be recorded")
                        });
                        assert!(
                            ancestor.byte_range.start <= entry.byte_range.start
                                && entry.byte_range.end <= ancestor.byte_range.end,
                            "{stem}: range of {path:?} must nest inside {prefix:?}"
                        );
                    }
                }
            }
        }

        /// The elaborations the plan names are recorded where expected in
        /// `cst-to-ast-core`: the `def` sugar, operator elaboration, the
        /// `if` desugaring, the boolean-literal injection, and the hoist.
        #[test]
        fn elaborated_nodes_carry_elab_kinds()
        {
            let lowered = lower_current_fixture("cst-to-ast-core");
            let kinds: Vec<ElabKind> = lowered
                .origin
                .iter()
                .filter_map(|(_path, entry)| entry.elaboration)
                .collect();
            for expected in [
                ElabKind::DefFunctionSugar,
                ElabKind::OperatorElab,
                ElabKind::IfSugar,
                ElabKind::BoolLiteral,
                ElabKind::BindHoist,
            ] {
                assert!(
                    kinds.contains(&expected),
                    "cst-to-ast-core must record {expected:?} (got {kinds:?})"
                );
            }
            let root = lowered.origin.get_path(&[0]).expect("item 0 root entry");
            assert_eq!(
                Some(ElabKind::DefFunctionSugar),
                root.elaboration,
                "the square item root is the def sugar's thunk"
            );
        }

        /// Checked-module lowering records its own elaboration tag on the
        /// synthesized record and sequencing envelope, rather than looking like
        /// an ordinary record literal or user-written `let`.
        #[test]
        fn module_declaration_nodes_carry_elab_kind()
        {
            let lowered =
                lower_source("module M { def x = 1; }".into()).expect("module source must lower");
            let tagged_paths: Vec<Vec<u32>> = lowered
                .origin
                .iter_paths()
                .filter(|&(_, _, item_2)| item_2.elaboration == Some(ElabKind::ModuleDeclaration))
                .map(|(path, ..)| Vec::<u32>::from(path.clone()))
                .collect();
            assert_eq!(
                tagged_paths,
                vec![vec![0], vec![0, 1], vec![0, 1, 0]],
                "module lowering must tag exactly the bind, terminal ret, and synthesized record"
            );
            for path in tagged_paths {
                let term_path = path.get(1 ..).expect("path has item prefix");
                let item = lowered.items.first().expect("module item");
                assert!(
                    resolve(&item.term, term_path).is_some(),
                    "module declaration tag path {path:?} must resolve"
                );
            }
        }

        /// A checked-module ascription resolves declared datatypes through the
        /// same nominal resolver as ordinary signatures, including type
        /// arguments nested under the module record.
        #[test]
        fn module_ascription_resolves_declared_data_types()
        {
            let lowered = lower_source(
                "data Maybe(a) { None, Some(x: a) }\n\
             module M : #{ value: Maybe(Integer) } {\n\
               def value = (Some(1) : Maybe(Integer));\n\
             }"
                .into(),
            )
            .expect("declared-data module source must lower");
            let module = lowered
                .items
                .iter()
                .find(|item| item.name.as_deref() == Some("M"))
                .expect("module item");
            let Some(&Ty::Value(ValueType::Record(ref fields))) = module.ascription.as_ref()
            else {
                panic!(
                    "module carries its record ascription: {:?}",
                    module.ascription
                );
            };
            let field = fields.get("value").expect("the value signature field");
            assert!(
                matches!(*field.as_ref(), ValueType::Data { .. }),
                "the value field remains nominal data, got {field:?}"
            );
        }

        /// Computation-signed module members mirror the full
        /// `Force(Annot(Thunk(body), Uω B))` origin topology.
        #[test]
        fn computation_signed_module_member_origin_mirrors_ascription_encoding()
        {
            let lowered = lower_source("module M { def x : F Integer; def x = ret 1; }".into())
                .expect("module source must lower");
            let item = lowered.items.first().expect("module item");
            for term_path in [&[0_u32][..], &[0, 0][..], &[0, 0, 0][..], &[0, 0, 0, 0][..]] {
                assert!(
                    resolve(&item.term, term_path).is_some(),
                    "ascription path {term_path:?} must resolve"
                );
            }
            for full_path in [vec![0, 0], vec![0, 0, 0], vec![0, 0, 0, 0]] {
                let entry = lowered
                    .origin
                    .get_path(&full_path)
                    .unwrap_or_else(|| panic!("origin path {full_path:?} recorded"));
                assert_eq!(
                    Some(ElabKind::CompAscription),
                    entry.elaboration,
                    "generated ascription layer {full_path:?} must be tagged"
                );
            }
            assert!(
                lowered.origin.get_path(&[0, 0, 0, 0, 0]).is_some(),
                "the original body remains below the synthesized thunk"
            );
        }
    }

    /// Fragment-corner pins on inline sources: statement forms, patterns,
    /// coercions, the prelude, and the structured error catalogue.
    mod lowering_shapes
    {
        use alloc::rc::Rc;

        use gandr_core_checker::syntax::Comp;
        use gandr_core_checker::syntax::Value;

        use super::*;
        /// `run x <- t;` is `Bind`; `t;` is `Bind` on `_`; the tail closes
        /// the chain.
        #[test]
        fn bind_and_sequencing_statements()
        {
            let term = sole_term("thunk { run x <- ret 1; ret x; ret x }");
            let Term::Value(Value::Thunk(_, ref body)) = term
            else {
                panic!("expected a thunk, got {term:?}");
            };
            let Comp::Bind(_, ref first_binder, ref rest) = **body
            else {
                panic!("expected a bind chain, got {body:?}");
            };
            assert_eq!("x", first_binder);
            let Comp::Bind(_, ref discard, _) = **rest
            else {
                panic!("expected sequencing sugar, got {rest:?}");
            };
            assert_eq!("_", discard, "`t;` binds the discard name");
        }
        /// A user-written hole `?`/`?name` lowers in *strict* mode (it is a
        /// legitimate axiom, not a total-mode recovery artifact) and takes the
        /// sort of its consuming position.
        #[test]
        fn user_holes_lower_in_strict_mode()
        {
            // Computation position: the thunk body `{ ? }` is a computation
            // hole, *not* `Ret` of a value hole — holes are sort-polymorphic.
            let comp = sole_term("thunk { ? }");
            let Term::Value(Value::Thunk(_, ref comp_body)) = comp
            else {
                panic!("expected a thunk, got {comp:?}");
            };
            assert!(
                matches!(**comp_body, Comp::Hole(_)),
                "a `?` in computation position is a Comp::Hole, got {comp_body:?}"
            );

            // Value position: `ret ?seed` returns a value hole.
            let value = sole_term("thunk { ret ?seed }");
            let Term::Value(Value::Thunk(_, ref value_body)) = value
            else {
                panic!("expected a thunk, got {value:?}");
            };
            let Comp::Ret(ref returned) = **value_body
            else {
                panic!("expected a ret, got {value_body:?}");
            };
            assert!(
                matches!(**returned, Value::Hole(_)),
                "a `?` in value position is a Value::Hole, got {returned:?}"
            );
        }
        /// A list literal lowers to `Value::List`; a `List(A)` annotation to
        /// `ValueType::List`; and a `case` with `Nil`/`Cons` arms to
        /// `Comp::ListCase` (the surface contract), dispatched away from the
        /// sum case by the constructor names.
        #[test]
        fn list_literal_and_list_case_lower()
        {
            use gandr_core_checker::types::ValueType;

            // `[1, 2, 3] : List(Integer)` — the literal lowers to a flat
            // `Value::List` under a `List` annotation that lowers to
            // `ValueType::List`.
            let annotated = sole_term("def xs = ([1, 2, 3] : List(Integer));");
            let Term::Value(Value::Annot(ref inner, ref ty)) = annotated
            else {
                panic!("expected an annotation, got {annotated:?}");
            };
            let Value::List(ref elements) = **inner
            else {
                panic!("expected a list literal, got {inner:?}");
            };
            assert_eq!(3, elements.len(), "three elements");
            assert_eq!(**ty, ValueType::list(ValueType::integer()));

            // The empty list `[]` lowers to an empty `Value::List`.
            let empty = sole_term("def e = [];");
            assert!(
                matches!(empty, Term::Value(Value::List(ref empty_elements)) if empty_elements.is_empty()),
                "expected an empty list, got {empty:?}"
            );

            // `case [] { Nil => …, Cons(h, t) => … }` lowers to `Comp::ListCase`
            // with the `head`/`tail` binders.
            let list_case = sole_term("thunk { case [] { Nil => ret 0, Cons(h, t) => ret h } }");
            let Term::Value(Value::Thunk(_, ref body)) = list_case
            else {
                panic!("expected a thunk, got {list_case:?}");
            };
            let Comp::ListCase {
                ref head, ref tail, ..
            } = **body
            else {
                panic!("expected a ListCase, got {body:?}");
            };
            assert_eq!("h", head);
            assert_eq!("t", tail);
        }
        /// `let (x, y) = v;` is `Split`; a ternary pattern nests through a
        /// fresh scrutinee.
        #[test]
        fn tuple_let_is_split()
        {
            let term = sole_term("thunk { val (x, y) = (1, 2); ret x }");
            let Term::Value(Value::Thunk(_, ref body)) = term
            else {
                panic!("expected a thunk, got {term:?}");
            };
            assert!(
                matches!(**body, Comp::Split { ref fst_name, ref snd_name, .. } if fst_name == "x" && snd_name == "y"),
                "a tuple let must lower to Split, got {body:?}"
            );

            let nested = sole_term("thunk { val (x, y, z) = (1, 2, 3); ret y }");
            let Term::Value(Value::Thunk(_, ref nested_body)) = nested
            else {
                panic!("expected a thunk, got {nested:?}");
            };
            let Comp::Split {
                fst_name: ref first,
                snd_name: ref fresh,
                body: ref inner,
                ..
            } = **nested_body
            else {
                panic!("expected a split, got {nested_body:?}");
            };
            assert_eq!("x", first);
            assert!(fresh.starts_with('%'), "the nested scrutinee is fresh");
            assert!(
                matches!(**inner, Comp::Split { ref fst_name, ref snd_name, .. } if fst_name == "y" && snd_name == "z"),
                "the inner split binds the remaining elements, got {inner:?}"
            );
        }
        /// `co { fst = …, snd = … }` is `With` (normalized fst-then-snd) and
        /// `t.fst` is `Prj` with the force sugar on a variable target.
        #[test]
        fn lazy_products_and_projections()
        {
            let term = sole_term("co { snd = ret 2, fst = ret 1 }");
            let Term::Comp(Comp::With(ref fst, _)) = term
            else {
                panic!("expected a lazy pair, got {term:?}");
            };
            assert!(
                matches!(**fst, Comp::Ret(ref payload) if **payload == Value::Int(1)),
                "fields must normalize fst-then-snd, got {fst:?}"
            );

            let projection = sole_term("h.fst");
            assert!(
                matches!(
                    projection,
                    Term::Comp(Comp::Prj(_, ref target))
                        if matches!(**target, Comp::Force(_))
                ),
                "a variable projection target takes the force sugar, got {projection:?}"
            );
        }
        /// A case over annotated injections type-checks end to end through
        /// `prelude_ctx()` (checker and machine agreeing).
        #[test]
        fn case_round_trips_through_both_implementations()
        {
            let source = "case (Inl(1) : Integer + Boolean) { Inl(x) => ret x, Inr(y) => ret 0 }";
            let term = sole_term(source);
            let Term::Comp(ref comp) = term
            else {
                panic!("a case lowers to a computation, got {term:?}");
            };
            let expected = CompType::returner(ValueType::integer());
            let checked = checker::check_comp(Ctx::new(), comp.clone(), expected.clone());
            assert_eq!(checked, Ok(expected.clone()), "the checker must accept");
            let (machine_result, _trace) =
                machine::run_comp(Ctx::new(), comp.clone(), Dir::Check(expected.clone()));
            assert_eq!(
                machine_result,
                Ok(Ty::Comp(expected)),
                "the machine must agree"
            );
        }
        /// A value in computation position takes the `Ret` coercion.
        #[test]
        fn value_tail_takes_ret_coercion()
        {
            let term = sole_term("fn(x) { x }");
            assert!(
                matches!(
                    term,
                    Term::Comp(Comp::Abs(_, _, ref body))
                        if matches!(**body, Comp::Ret(_))
                ),
                "a value block tail must be wrapped in Ret, got {term:?}"
            );
        }
        /// A user module value selected with `M.field` uses ordinary record
        /// projection; only registered builtin/host module names are special.
        #[test]
        fn user_module_field_selection_is_record_projection()
        {
            let lowered = lower_ok("module M {}\ndef use_field = M.field;");
            let item = lowered
                .items
                .iter()
                .find(|item| item.name.as_deref() == Some("use_field"))
                .expect("projection item");
            assert!(
                matches!(
                    item.term,
                    Term::Comp(Comp::RecordProj { ref record, ref label })
                        if label == "field"
                            && matches!(**record, Value::Var(ref target) if target == "M")
                ),
                "M.field must project field from Var(M): {:?}",
                item.term
            );

            let nested = sole_term("M.inner.field");
            let Term::Comp(Comp::Bind(ref inner, ref tmp, ref outer)) = nested
            else {
                panic!("nested projection must hoist the inner projection: {nested:?}");
            };
            assert_eq!(
                "%tmp0", tmp,
                "the outer projection consumes the inner result once"
            );
            let Comp::RecordProj {
                record: ref inner_target,
                label: ref inner_label,
            } = **inner
            else {
                panic!("inner projection must be a record projection: {inner:?}");
            };
            assert_eq!("inner", inner_label);
            assert!(
                matches!(**inner_target, Value::Var(ref target) if target == "M"),
                "inner projection target must be Var(M): {inner_target:?}"
            );
            let Comp::RecordProj {
                record: ref outer_target,
                label: ref outer_label,
            } = **outer
            else {
                panic!("outer projection must be a record projection: {outer:?}");
            };
            assert_eq!("field", outer_label);
            assert!(
                matches!(**outer_target, Value::Var(ref target) if target == tmp),
                "outer projection target must be the hoisted temporary: {outer_target:?}"
            );
            let mut ctx = Ctx::new();
            ctx.bind(
                "M".to_owned(),
                ValueType::record([(
                    "inner".to_owned(),
                    ValueType::record([("field".to_owned(), ValueType::integer())]),
                )]),
            );
            let Term::Comp(nested_comp) = nested
            else {
                panic!("nested projection remains a computation");
            };
            assert_eq!(
                checker::infer_comp(ctx, nested_comp),
                Ok(CompType::returner(ValueType::integer())),
                "nested user-module projection must infer the inner field type"
            );
        }

        /// The single item's term of an inline source.
        fn sole_term<'text>(source: impl Into<TestText<'text>>) -> Term
        {
            let source = source.into().0;
            let lowered = lower_ok(source);
            assert_eq!(1, lowered.items.len(), "one item expected:\n{source}");
            lowered.items.into_iter().next().expect("one item").term
        }
        /// Empty checked modules lower to ordinary named record items, with an
        /// optional record-shaped ascription preserved on the item.
        #[test]
        fn modules_lower_to_named_record_items()
        {
            let bare = sole_item("module M {}");
            assert_eq!(Some("M"), bare.name.as_deref());
            assert_eq!(None, bare.ascription);
            assert!(
                matches!(bare.term, Term::Value(Value::Record(ref fields)) if fields.is_empty()),
                "a bare empty module is an empty record value: {:?}",
                bare.term
            );

            let ascribed = sole_item("module M : #{} {}");
            assert_eq!(Some("M"), ascribed.name.as_deref());
            assert!(
                matches!(
                    ascribed.ascription,
                    Some(Ty::Value(ValueType::Record(ref fields))) if fields.is_empty()
                ),
                "the module record ascription must be preserved: {:?}",
                ascribed.ascription
            );
            assert!(
                matches!(
                    ascribed.term,
                    Term::Value(Value::Annot(ref payload, ref ty))
                        if matches!(**payload, Value::Record(ref fields) if fields.is_empty())
                            && matches!(**ty, ValueType::Record(ref fields) if fields.is_empty())
                ),
                "an ascribed empty module is an annotated empty record value: {:?}",
                ascribed.term
            );
        }
        /// Nonempty checked modules are computations that return the record, so
        /// the surface record ascription is checked as the return payload.
        #[test]
        fn nonempty_module_ascription_checks_returned_record()
        {
            let item =
                sole_item("module M : #{ x: Integer, y: Integer } { def x = 1; def y = 2; }");
            let expected_record = ValueType::record([
                ("x".to_owned(), ValueType::integer()),
                ("y".to_owned(), ValueType::integer()),
            ]);
            assert_eq!(
                item.ascription,
                Some(Ty::Value(expected_record.clone())),
                "the surface module record ascription is preserved as metadata"
            );
            let Term::Comp(ref comp) = item.term
            else {
                panic!("a nonempty module lowers to a computation: {:?}", item.term);
            };
            assert_eq!(
                checker::infer_comp(Ctx::new(), comp.clone()),
                Ok(CompType::returner(expected_record)),
                "the checker must accept a matching two-member module record ascription"
            );

            let missing = sole_item(
                "module M : #{ x: Integer, y: Integer, z: Integer } { \
                 def x = 1; def y = 2; }",
            );
            let Term::Comp(ref missing_comp) = missing.term
            else {
                panic!(
                    "a nonempty missing-field module lowers to a computation: {:?}",
                    missing.term
                );
            };
            assert!(
                checker::infer_comp(Ctx::new(), missing_comp.clone()).is_err(),
                "a missing module record field must be checked by the terminal record annotation"
            );

            let wrong =
                sole_item("module M : #{ x: Integer, y: String } { def x = 1; def y = 2; }");
            let Term::Comp(ref wrong_comp) = wrong.term
            else {
                panic!(
                    "a nonempty wrong-field module lowers to a computation: {:?}",
                    wrong.term
                );
            };
            assert!(
                checker::infer_comp(Ctx::new(), wrong_comp.clone()).is_err(),
                "a wrong module record field type must be checked by the terminal record annotation"
            );

            let effectful = lower_ok(
                "extern \"c\" from \"sensor\" { def read(channel: i32) -> i64; }\n\
                 module M : #{ reading: i64, ok: Integer } { \
                 def reading = sensor.read(0i32); def ok = 1; }",
            );
            let effect_item = effectful
                .items
                .iter()
                .find(|candidate| candidate.name.as_deref() == Some("M"))
                .expect("effectful module item");
            let Term::Comp(ref effect_comp) = effect_item.term
            else {
                panic!(
                    "an effectful nonempty module lowers to a computation: {:?}",
                    effect_item.term
                );
            };
            assert!(
                checker::infer_comp(Ctx::new(), effect_comp.clone()).is_ok(),
                "a module record ascription constrains the returned record, not the eager-member effect row"
            );
        }
        /// Member definitions become source-ordered binds and one final record
        /// whose fields point at the generated member binders.
        #[test]
        fn module_members_bind_in_source_order_and_return_record()
        {
            let item = sole_item("module M { def first = 1; def second = 2; }");
            let Term::Comp(Comp::Bind(ref first_bound, ref first_binder, ref first_rest)) =
                item.term
            else {
                panic!("members must sequence in a bind chain: {:?}", item.term);
            };
            assert_eq!("first", first_binder);
            assert!(
                matches!(**first_bound, Comp::Ret(ref value) if matches!(**value, Value::Int(1)))
            );

            let Comp::Bind(ref second_bound, ref second_binder, ref second_rest) = **first_rest
            else {
                panic!("second member must be the next bind: {first_rest:?}");
            };
            assert_eq!("second", second_binder);
            assert!(
                matches!(**second_bound, Comp::Ret(ref value) if matches!(**value, Value::Int(2)))
            );

            let Comp::Ret(ref record) = **second_rest
            else {
                panic!("module chain must return the final record: {second_rest:?}");
            };
            let Value::Record(ref fields) = **record
            else {
                panic!("module result must be a record: {record:?}");
            };
            assert!(matches!(
                fields.get("first").map(|value| &**value),
                Some(Value::Var(name)) if name == "first"
            ));
            assert!(matches!(
                fields.get("second").map(|value| &**value),
                Some(Value::Var(name)) if name == "second"
            ));
        }
        /// Explicit member signatures attach to their definition and override a
        /// function member's sugar-derived ascription.
        #[test]
        fn member_signature_attaches_and_wins_over_derived_function_type()
        {
            let item = sole_item(
                "module M { def f : U[1] (Integer -> F Integer); \
                 def f(x: Integer) -> F Integer { ret x } }",
            );
            let Term::Comp(Comp::Bind(ref bound, ref binder, _)) = item.term
            else {
                panic!(
                    "function member must bind before record return: {:?}",
                    item.term
                );
            };
            assert_eq!("f", binder);
            let Comp::Ret(ref value) = **bound
            else {
                panic!("function member binding must return the thunk value: {bound:?}");
            };
            let Value::Annot(_, ref ty) = **value
            else {
                panic!("explicit signature must annotate the member value: {value:?}");
            };
            assert_eq!(
                **ty,
                ValueType::thunk(
                    Grade::ONE,
                    CompType::arrow(
                        ValueType::integer(),
                        CompType::returner(ValueType::integer()),
                    ),
                ),
                "the explicit grade-one signature wins over the derived omega signature"
            );
        }

        /// The single item of an inline source.
        fn sole_item<'text>(source: impl Into<TestText<'text>>) -> LoweredItem
        {
            let source = source.into().0;
            let lowered = lower_ok(source);
            assert_eq!(1, lowered.items.len(), "one item expected:\n{source}");
            lowered.items.into_iter().next().expect("one item")
        }
        /// An explicit signature is recorded as the matching def's
        /// ascription and wins over the sugar-derived one.
        #[test]
        fn signatures_attach_to_their_defs()
        {
            let lowered = lower_ok(
                "def answer : Integer;\ndef answer = 42;\n\
                 def f : U[1] (Integer -> F Integer);\ndef f(x: Integer) -> F Integer { ret x }",
            );
            let answer = lowered
                .items
                .iter()
                .find(|item| item.name.as_deref() == Some("answer"))
                .expect("answer item");
            assert_eq!(
                answer.ascription,
                Some(Ty::Value(ValueType::integer())),
                "the signature must attach"
            );
            let f_item = lowered
                .items
                .iter()
                .find(|item| item.name.as_deref() == Some("f"))
                .expect("f item");
            assert_eq!(
                f_item.ascription,
                Some(Ty::Value(ValueType::thunk(
                    Grade::ONE,
                    CompType::arrow(
                        ValueType::integer(),
                        CompType::returner(ValueType::integer()),
                    ),
                ))),
                "an explicit signature must win over the derived ascription"
            );
        }

        /// Lowers an inline source that must succeed.
        fn lower_ok<'text>(source: impl Into<TestText<'text>>) -> Lowered
        {
            let source = source.into().0;
            lower_source(source.into())
                .unwrap_or_else(|error| panic!("source must lower: {error}\n{source}"))
        }

        /// Asserts a value path has both a resolving term node and origin node.
        fn assert_value_path<'path, 'text>(
            lowered: &Lowered,
            term_path: impl Into<TestPath<'path>>,
            label: impl Into<TestText<'text>>,
            matches_value: impl FnOnce(&Value) -> TestDecision,
        )
        {
            let term_path = term_path.into().0;
            let label = label.into().0;
            let origin_path = item0_origin_path(term_path);
            assert!(
                lowered.origin.get_path(&*origin_path).is_some(),
                "origin path for {label} ({origin_path:?}) must exist"
            );
            match resolve(&lowered.items[0].term, term_path) {
                | Some(TermRef::Value(value)) if bool::from(matches_value(value)) => {},
                | other => panic!("{label} path {term_path:?} resolved to {other:?}"),
            }
        }

        /// Converts a term-child path for item 0 to an origin-map path.
        fn item0_origin_path<'path>(term_path: impl Into<TestPath<'path>>) -> TestOwnedPath
        {
            let term_path = term_path.into().0;
            let mut path = Vec::with_capacity(term_path.len().saturating_add(1));
            path.push(0);
            path.extend_from_slice(term_path);
            TestOwnedPath(path)
        }

        /// Asserts a bound computation is exactly `ret (literal : String)`.
        fn assert_ret_annotated_string<'text>(
            comp: &Comp,
            expected: impl Into<TestText<'text>>,
        )
        {
            let expected = expected.into().0;
            let Comp::Ret(ref value) = *comp
            else {
                panic!("host escape bound computation must be Ret, got {comp:?}");
            };
            assert_eq!(
                value.as_ref(),
                &Value::annot(Value::string(expected), ValueType::string())
            );
        }
        /// Across commands, host escapes nest in source order before each
        /// command's `Exec::exec`, and payload origins resolve for each
        /// fragment.
        #[test]
        fn shell_host_escapes_sequence_across_commands_with_origins()
        {
            let source = r#"#!{ printf '%s%s' $("a1") $("a2"); echo $("b1") $("b2"); }"#;
            let lowered = lower_ok(source);
            let Term::Comp(ref comp) = lowered.items[0].term
            else {
                panic!("shell block must lower to a computation");
            };
            let Comp::Bind(ref first_command, ref discard, ref rest) = *comp
            else {
                panic!("two shell commands must sequence, got {comp:?}");
            };
            assert_eq!("_", discard);
            let Comp::Bind(ref first_bound, ref first_binder, ref first_rest) = **first_command
            else {
                panic!("first command must start with first escape");
            };
            assert_eq!("%tmp0", first_binder);
            assert_ret_annotated_string(first_bound.as_ref(), "a1");
            let Comp::Bind(ref second_bound, ref second_binder, ref first_exec) = **first_rest
            else {
                panic!("first command must hoist second escape before exec");
            };
            assert_eq!("%tmp1", second_binder);
            assert_ret_annotated_string(second_bound.as_ref(), "a2");
            let first_payload = exec_payload(first_exec.as_ref());
            assert_eq!(
                record_field(first_payload, host::FIELD_PROGRAM),
                &Value::string("printf")
            );
            let first_args = argv(first_payload);
            assert_eq!(3, first_args.len());
            assert_eq!(first_args[0].as_ref(), &Value::string("%s%s"));
            assert_annotated_var_arg(first_args[1].as_ref(), "%tmp0");
            assert_annotated_var_arg(first_args[2].as_ref(), "%tmp1");

            let Comp::Bind(ref final_exec_chain, ref reply, ref returned) = **rest
            else {
                panic!("last shell command must preserve its reply");
            };
            assert_eq!("%tmp4", reply);
            assert!(matches!(returned.as_ref(), Comp::Ret(_)));
            let Comp::Bind(ref third_bound, ref third_binder, ref second_rest) = **final_exec_chain
            else {
                panic!("second command must start with third escape");
            };
            assert_eq!("%tmp2", third_binder);
            assert_ret_annotated_string(third_bound.as_ref(), "b1");
            let Comp::Bind(ref fourth_bound, ref fourth_binder, ref second_exec) = **second_rest
            else {
                panic!("second command must hoist fourth escape before exec");
            };
            assert_eq!("%tmp3", fourth_binder);
            assert_ret_annotated_string(fourth_bound.as_ref(), "b2");
            let second_payload = exec_payload(second_exec.as_ref());
            assert_eq!(
                record_field(second_payload, host::FIELD_PROGRAM),
                &Value::string("echo")
            );
            let second_args = argv(second_payload);
            assert_eq!(2, second_args.len());
            assert_annotated_var_arg(second_args[0].as_ref(), "%tmp2");
            assert_annotated_var_arg(second_args[1].as_ref(), "%tmp3");

            let first_payload_path = [0, 1, 1, 0];
            assert_exec_payload_child_paths(&lowered, &first_payload_path, "printf", 3);
            assert_exact_origin_path(source, &lowered, &[0, 1, 1, 0, 2], "printf");
            assert_exact_origin_path(source, &lowered, &[0, 1, 1, 0, 0, 0], "'%s%s'");
            assert_exact_origin_path(source, &lowered, &[0, 1, 1, 0, 0, 1], r#"$("a1")"#);
            assert_exact_origin_path(source, &lowered, &[0, 1, 1, 0, 0, 2], r#"$("a2")"#);

            let second_payload_path = [1, 0, 1, 1, 0];
            assert_exec_payload_child_paths(&lowered, &second_payload_path, "echo", 2);
            assert_exact_origin_path(source, &lowered, &[1, 0, 1, 1, 0, 2], "echo");
            assert_exact_origin_path(source, &lowered, &[1, 0, 1, 1, 0, 0, 0], r#"$("b1")"#);
            assert_exact_origin_path(source, &lowered, &[1, 0, 1, 1, 0, 0, 1], r#"$("b2")"#);
        }

        /// Asserts one exact origin path resolves and points at `needle`.
        fn assert_exact_origin_path<'source, 'path, 'needle>(
            source: impl Into<TestText<'source>>,
            lowered: &Lowered,
            term_path: impl Into<TestPath<'path>>,
            needle: impl Into<TestText<'needle>>,
        )
        {
            let source = source.into().0;
            let term_path = term_path.into().0;
            let needle = needle.into().0;
            let origin_path = item0_origin_path(term_path);
            let entry = lowered
                .origin
                .get_path(&*origin_path)
                .unwrap_or_else(|| panic!("origin path {origin_path:?} must exist"));
            assert_eq!(
                entry.byte_range,
                source_range(source, needle),
                "origin path {origin_path:?} must point exactly at {needle:?}"
            );
            assert!(
                resolve(&lowered.items[0].term, term_path).is_some(),
                "term path {term_path:?} must resolve"
            );
        }
        /// Host escapes with non-expression interiors are malformed in strict
        /// mode and become local holes in total mode.
        #[test]
        fn shell_host_escape_multi_interior_is_strict_error_and_total_hole()
        {
            for (source, host_escape) in [
                (r#"#!{ printf '%s' $(1 2); }"#, "$(1 2)"),
                (r#"#!{ printf '%s' $(1 { "x" }); }"#, r#"$(1 { "x" })"#),
                (r#"#!{ printf '%s' $({ "x" } 1); }"#, r#"$({ "x" } 1)"#),
                (r#"#!{ printf '%s' $(1 + 2 3); }"#, "$(1 + 2 3)"),
            ] {
                let expected_range = source_range(source, host_escape);
                assert!(
                    matches!(
                        lower_err(source),
                        LowerError::MalformedNode { byte_range, .. } if byte_range == expected_range
                    ),
                    "malformed host escape {host_escape:?} must be a local strict error"
                );
                let lowered = lower_source_total(source.into())
                    .unwrap_or_else(|error| panic!("total lowering must recover: {error}"));
                assert!(
                    lowered.origin.iter().any(|(_id, entry)| {
                        entry.byte_range == expected_range
                            && matches!(entry.note, Some(HoleNote::MalformedNode { .. }))
                    }),
                    "total lowering must record a local malformed-node hole for {host_escape:?}"
                );
            }
        }

        /// Byte range for a source needle that must occur exactly once.
        fn source_range<'source, 'needle>(
            source: impl Into<TestText<'source>>,
            needle: impl Into<TestText<'needle>>,
        ) -> SourceRange
        {
            let source = source.into().0;
            let needle = needle.into().0;
            let mut match_indices = source.match_indices(needle);
            let Some((start, _)) = match_indices.next()
            else {
                panic!("{needle:?} must appear in source");
            };
            assert!(
                match_indices.next().is_none(),
                "{needle:?} must be unique in source"
            );
            SourceRange(start .. start.saturating_add(needle.len()))
        }
        /// Dangling member signatures reject in strict mode and become a record
        /// field hole with the usual missing-definition note in total mode.
        #[test]
        fn dangling_member_signature_is_strict_error_and_total_hole()
        {
            let error = lower_err("module M { def missing : Integer; }");
            assert!(
                matches!(error, LowerError::DanglingSignature { ref name, .. } if name == "missing"),
                "dangling member signatures must be structured errors: {error:?}"
            );

            let lowered = lower_source_total("module M { def missing : Integer; }".into())
                .expect("total lowering must recover a dangling member signature");
            let goals = goals_report(&lowered, &prelude_ctx());
            assert!(
                goals.iter().any(|goal| matches!(
                    goal.note,
                    Some(HoleNote::MissingDefinition { ref name })
                        if name == "missing"
                )),
                "total recovery must expose a missing-definition hole: {goals:?}"
            );
        }
        /// Duplicate module member definitions are rejected instead of silently
        /// overwriting a record field.
        #[test]
        fn duplicate_module_member_definition_is_rejected()
        {
            let error = lower_err("module M { def x = 1; def x = 2; }");
            assert!(
                matches!(error, LowerError::DuplicateModuleMember { ref name, .. } if name == "x"),
                "duplicate member definitions must be rejected structurally: {error:?}"
            );

            let recovered = lower_source_total(
                "module M { def x = 1; def x : Integer; def x = 2; def y = 3; }".into(),
            )
            .expect("total mode must recover a duplicate member");
            let goals = goals_report(&recovered, &prelude_ctx());
            let duplicate_notes = goals
                .iter()
                .filter(|goal| {
                    matches!(
                        goal.note,
                        Some(HoleNote::UnsupportedForm { kind })
                            if kind == node_kinds::MODULE_DECLARATION
                    )
                })
                .count();
            assert_eq!(
                1, duplicate_notes,
                "total recovery must expose exactly one duplicate-site note: {goals:?}"
            );
            assert!(
                goals
                    .iter()
                    .all(|goal| !matches!(goal.note, Some(HoleNote::MissingDefinition { .. }))),
                "the signature before the duplicate is consumed with it, not reported dangling: {goals:?}"
            );

            let item = recovered.items.first().expect("module item");
            let Term::Comp(Comp::Bind(_, ref first_binder, ref first_rest)) = item.term
            else {
                panic!(
                    "the first member must remain the first bind: {:?}",
                    item.term
                );
            };
            assert_eq!("x", first_binder);
            let Comp::Bind(ref duplicate_bound, ref duplicate_binder, ref after_duplicate) =
                **first_rest
            else {
                panic!("the duplicate site must become a discard-bound hole: {first_rest:?}");
            };
            assert_eq!(
                "_", duplicate_binder,
                "the duplicate site is sequenced but does not define a field"
            );
            assert!(
                matches!(**duplicate_bound, Comp::Hole(_)),
                "the duplicate site must recover as a computation hole: {duplicate_bound:?}"
            );
            let Comp::Bind(_, ref later_binder, ref final_ret) = **after_duplicate
            else {
                panic!("the later member must remain after the duplicate: {after_duplicate:?}");
            };
            assert_eq!("y", later_binder, "later members keep source order");
            let Comp::Ret(ref record) = **final_ret
            else {
                panic!("the recovered module must still return a record: {final_ret:?}");
            };
            let Value::Record(ref fields) = **record
            else {
                panic!("the recovered module result must be a record: {record:?}");
            };
            assert!(
                matches!(fields.get("x").map(|value| &**value), Some(Value::Var(name)) if name == "x"),
                "a signature before the duplicate must not leave a dangling hole field that overwrites the first definition: {fields:?}"
            );
        }
        /// A malformed member recovers locally in total mode: the damaged
        /// member becomes a hole-bound field and later members still
        /// lower.
        #[test]
        fn malformed_module_member_recovers_in_total_mode()
        {
            let source = "module M { def broken = ; def ok = 2; }";
            let damaged_range = SourceRange(11 .. 25);
            let error = lower_err(source);
            assert!(
                matches!(
                    error,
                    LowerError::MalformedNode { kind, ref byte_range }
                        if kind == node_kinds::DEF_VALUE
                            && *byte_range == damaged_range
                ),
                "strict malformed member must report the damaged span, got {error:?}"
            );

            let lowered = lower_source_total(source.into())
                .expect("total lowering must recover a malformed module member");
            let item = lowered.items.first().expect("module item");
            assert_eq!(Some("M"), item.name.as_deref());
            let Term::Comp(Comp::Bind(ref broken_bound, ref broken_binder, ref rest)) = item.term
            else {
                panic!(
                    "damaged member must still bind a recovery hole: {:?}",
                    item.term
                );
            };
            assert_eq!("broken", broken_binder);
            assert!(
                matches!(**broken_bound, Comp::Ret(ref value) if matches!(**value, Value::Hole(_))),
                "the malformed member must bind a value hole: {broken_bound:?}"
            );
            let hole_entry = lowered
                .origin
                .get_path(&[0, 0, 0])
                .expect("malformed member value-hole origin");
            assert_eq!(
                hole_entry.byte_range, damaged_range,
                "the total-mode hole keeps the strict damaged span"
            );
            assert_eq!(
                Some(HoleNote::MalformedNode {
                    kind: node_kinds::DEF_VALUE,
                }),
                hole_entry.note,
                "the total-mode hole keeps the structured malformed-node note"
            );
            assert!(
                matches!(**rest, Comp::Bind(_, ref ok_binder, _) if ok_binder == "ok"),
                "later well-formed members must survive in order: {rest:?}"
            );
        }
        /// A host escape adjacent to another fragment is one lexical shell
        /// word. With no landed typed concat operation, the lowerer
        /// rejects it rather than splitting it into multiple argv
        /// elements.
        #[test]
        fn shell_host_escape_mixed_word_is_local_lower_error()
        {
            for source in [
                r#"#!{ printf '%s' pre$("computed")post; }"#,
                r#"#!{ printf '%s' $("a")$("b"); }"#,
                r#"#!{ $("printf") x; }"#,
                r#"#!{ pre$("fix") x; }"#,
            ] {
                let error = lower_err(source);
                assert!(
                    matches!(error, LowerError::Unsupported {
                        kind: node_kinds::HOST_ESCAPE,
                        ..
                    }),
                    "mixed host-escape word must be a local Unsupported error: {error:?}"
                );
            }
        }
        /// Shell control stays unsupported at shell-command level.
        #[test]
        fn shell_control_and_or_remain_shell_unsupported()
        {
            for (source, expected_kind) in [
                (r#"#!{ true && false; }"#, "and_expression"),
                (r#"#!{ true || false; }"#, "or_expression"),
            ] {
                assert!(
                    matches!(
                        lower_err(source),
                        LowerError::Unsupported { kind, .. } if kind == expected_kind
                    ),
                    "shell control {source} must remain Unsupported({expected_kind})"
                );
            }
        }
        /// The structured error catalogue: each in-fragment misuse yields
        /// its dedicated `LowerError` constructor.
        #[test]
        fn structured_error_catalogue()
        {
            assert!(
                matches!(
                    lower_err("case (Inl(1) : Integer + Integer) { Inl(x) => ret x }"),
                    LowerError::MissingCaseArm {
                        constructor: "Inr",
                        ..
                    }
                ),
                "a missing arm is MissingCaseArm"
            );
            assert!(
                matches!(
                    lower_err("def f : Integer;"),
                    LowerError::DanglingSignature { .. }
                ),
                "an unmatched signature is DanglingSignature"
            );
            assert!(
                matches!(lower_err("thunk { ret 1; }"), LowerError::EmptyBlock { .. }),
                "a tail-less block is EmptyBlock"
            );
            assert!(
                matches!(
                    lower_err("def x = 99999999999999999999;"),
                    LowerError::InvalidIntegerLiteral { .. }
                ),
                "an i64-overflowing integer literal is InvalidIntegerLiteral (a bare \
                 float now lowers to f64, the surface contract)"
            );
            assert!(
                matches!(
                    lower_err("def x = 4294967296u32;"),
                    LowerError::InvalidIntegerLiteral { .. }
                ),
                "a u32 literal out of range is InvalidIntegerLiteral (the surface contract)"
            );
            assert!(
                matches!(
                    lower_err("def x = 1e400;"),
                    LowerError::InvalidIntegerLiteral { .. }
                ),
                "a bare float that overflows f64 to a non-finite value is rejected (the surface contract)"
            );
            assert!(
                matches!(
                    lower_err("def x = 1e400f64;"),
                    LowerError::InvalidIntegerLiteral { .. }
                ),
                "a suffixed float that overflows f64 is rejected (the surface contract)"
            );
            assert!(
                matches!(
                    lower_err("def t = thunk[r] { ret 1 };"),
                    LowerError::InvalidGrade { .. }
                ),
                "a grade variable is InvalidGrade (Stage 2)"
            );
            assert!(
                matches!(
                    lower_err("def s : F Unit -> F Unit;"),
                    LowerError::TypeSortMismatch { .. }
                ),
                "a computation type in argument position is TypeSortMismatch"
            );
            assert!(
                matches!(
                    lower_err("thunk { leta x = 1; ret x }"),
                    LowerError::Unsupported {
                        kind: node_kinds::LETA_STATEMENT,
                        ..
                    }
                ),
                "an out-of-fragment statement is Unsupported with its kind"
            );
        }

        /// Lowers an inline source that must fail, returning the error.
        fn lower_err<'text>(source: impl Into<TestText<'text>>) -> LowerError
        {
            let source = source.into().0;
            lower_source(source.into())
                .err()
                .unwrap_or_else(|| panic!("source must not lower:\n{source}"))
        }

        /// Reads a local engine fixture by path relative to
        /// `tests/fixtures/corpus`.
        fn read_corpus_example<'text>(relative: impl Into<TestText<'text>>) -> String
        {
            let relative = relative.into().0;
            fs::read_to_string(repo_root().join("tests/fixtures/corpus").join(relative))
                .unwrap_or_else(|error| panic!("corpus example {relative} must read: {error}"))
        }
        /// Boolean operators inside `$(...)` belong to gandr, not shell
        /// control, so they reach ordinary String checking.
        #[test]
        fn shell_host_escape_boolean_ops_are_typed_not_shell_control()
        {
            let boolean_ty = ValueType::sum(ValueType::Unit, ValueType::Unit);
            assert_host_escape_type_mismatch(
                r#"#!{ printf '%s' $(true && false); }"#,
                boolean_ty.clone(),
            );
            assert_host_escape_type_mismatch(r#"#!{ printf '%s' $(true || false); }"#, boolean_ty);
        }
        /// The real corpus files exercise the PBG-folded host-escape CST shape
        /// (no tree-sitter `expression` field) rather than only the inline
        /// wrapper shape.
        #[test]
        fn shell_host_escape_corpus_examples_lower_real_cst_shape()
        {
            let model = read_corpus_example("model/13-shell-blocks.gandr");
            let Term::Comp(_) = sole_term(&model)
            else {
                panic!("the model shell example must lower to a computation");
            };

            let pathological =
                read_corpus_example("pathological/shell-host-escape-non-string.gandr");
            assert_host_escape_type_mismatch(&pathological, ValueType::integer());
        }
        /// Non-`String` host escapes are rejected by ordinary type checking,
        /// not lowered by numeric coercion or a shell fallback.
        #[test]
        fn shell_host_escape_non_string_is_typed_error()
        {
            assert_host_escape_type_mismatch(r#"#!{ printf '%s' $(1); }"#, ValueType::integer());
            assert_host_escape_type_mismatch(r#"#!{ printf '%s' $(1e4); }"#, ValueType::f64());
            assert_host_escape_type_mismatch(r#"#!{ printf '%s' $(1e4f64); }"#, ValueType::f64());
        }

        /// Asserts a host escape reaches ordinary String type checking.
        fn assert_host_escape_type_mismatch<'text>(
            source: impl Into<TestText<'text>>,
            actual_ty: ValueType,
        )
        {
            let source = source.into().0;
            let comp = shell_comp(source);
            let result = checker::infer_comp(prelude_ctx(), comp);
            assert!(
                matches!(
                    &result,
                    Err(TypeError::TypeMismatch { expected, actual })
                        if *expected == Ty::Value(ValueType::string())
                            && *actual == Ty::Value(actual_ty)
                ),
                "host escape must be rejected as an ordinary String type mismatch: {result:?}"
            );
        }
        /// A string containing spaces remains one argv element: no word
        /// splitting, interpolation, or shell reparse is introduced.
        #[test]
        fn shell_host_escape_with_spaces_is_one_argument()
        {
            let comp = shell_comp(r#"#!{ printf '%s' $("a b"); }"#);
            let command = single_shell_exec(&comp);
            let Comp::Bind(_, ref binder, ref exec) = *command
            else {
                panic!("host escape with spaces must cross a bind before Exec");
            };
            assert_eq!("%tmp0", binder);
            let args = argv(exec_payload(exec.as_ref()));
            assert_eq!(2, args.len(), "format plus one computed argv value");
            assert_eq!(args[0].as_ref(), &Value::string("%s"));
            assert_annotated_var_arg(args[1].as_ref(), "%tmp0");
        }

        /// The single shell block computation of an inline source.
        fn shell_comp<'text>(source: impl Into<TestText<'text>>) -> Comp
        {
            let source = source.into().0;
            let term = sole_term(source);
            let Term::Comp(comp) = term
            else {
                panic!("a shell block lowers to a computation, got {term:?}");
            };
            comp
        }

        /// The inferred computation type for one `Exec::exec` shell command.
        fn exec_comp_ty() -> CompType
        {
            CompType::returner_eff(exec_reply_ty(), EffectRow::singleton(host::exec()))
        }

        /// The shell block's `Exec::exec` reply type.
        fn exec_reply_ty() -> ValueType
        {
            ValueType::record([
                (host::FIELD_STDOUT.to_owned(), ValueType::string()),
                (host::FIELD_STDERR.to_owned(), ValueType::string()),
                (host::FIELD_EXIT_CODE.to_owned(), ValueType::integer()),
            ])
        }

        /// The bound command computation of a one-command shell block.
        fn single_shell_exec(comp: &Comp) -> &Comp
        {
            let Comp::Bind(ref exec, _, ref ret) = *comp
            else {
                panic!("one shell command lowers to a final bind, got {comp:?}");
            };
            assert!(
                matches!(ret.as_ref(), Comp::Ret(_)),
                "the final shell command's reply must remain the block result"
            );
            exec.as_ref()
        }
        /// Literal words stay literal while a standalone `$(...)` contributes
        /// one checked `String` argv element.
        #[test]
        fn shell_host_escape_standalone_dynamic_argv_shape()
        {
            let comp = shell_comp(r#"#!{ printf '%s' $("computed"); }"#);
            assert_eq!(
                checker::infer_comp(Ctx::new(), comp.clone()),
                Ok(exec_comp_ty()),
                "a string-valued host escape must type-check through Exec::exec"
            );
            let command = single_shell_exec(&comp);
            let Comp::Bind(_, ref binder, ref exec) = *command
            else {
                panic!("host escape must cross a bind before Exec, got {command:?}");
            };
            assert_eq!("%tmp0", binder);
            let payload = exec_payload(exec.as_ref());
            assert_eq!(
                record_field(payload, host::FIELD_PROGRAM),
                &Value::string("printf")
            );
            assert_eq!(
                record_field(payload, host::FIELD_MODE),
                &Value::string(host::MODE_CAPTURED)
            );
            let args = argv(payload);
            assert_eq!(2, args.len(), "format plus one computed argv value");
            assert_eq!(args[0].as_ref(), &Value::string("%s"));
            assert_annotated_var_arg(args[1].as_ref(), "%tmp0");
        }
        /// Multiple standalone computed argv elements evaluate left-to-right
        /// before the containing `Exec::exec`.
        #[test]
        fn shell_host_escapes_lower_left_to_right()
        {
            let comp = shell_comp(r#"#!{ printf '%s%s' $("a") $("b"); }"#);
            let command = single_shell_exec(&comp);
            let Comp::Bind(ref first_bound, ref first_binder, ref rest) = *command
            else {
                panic!("first host escape must be hoisted, got {command:?}");
            };
            assert_eq!("%tmp0", first_binder);
            assert_ret_annotated_string(first_bound.as_ref(), "a");
            let Comp::Bind(ref second_bound, ref second_binder, ref exec) = **rest
            else {
                panic!("second host escape must be hoisted after the first, got {rest:?}");
            };
            assert_eq!("%tmp1", second_binder);
            assert_ret_annotated_string(second_bound.as_ref(), "b");
            let args = argv(exec_payload(exec.as_ref()));
            assert_eq!(3, args.len(), "format plus two computed argv values");
            assert_eq!(args[0].as_ref(), &Value::string("%s%s"));
            assert_annotated_var_arg(args[1].as_ref(), "%tmp0");
            assert_annotated_var_arg(args[2].as_ref(), "%tmp1");
        }

        /// The payload of an `Exec::exec` perform.
        fn exec_payload(comp: &Comp) -> &Value
        {
            let Comp::Perform(ref sig, ref op, ref payload) = *comp
            else {
                panic!("expected Exec::exec perform, got {comp:?}");
            };
            assert_eq!(host::EXEC, sig.name().as_ref(), "signature must be Exec");
            assert_eq!(host::EXEC_RUN, op.as_str(), "operation must be exec");
            payload.as_ref()
        }
        /// A computation-valued host expression is hoisted once before its own
        /// shell argv bind, then that argv bind is the value used by Exec.
        #[test]
        fn shell_host_escape_string_computation_precedes_own_exec_once()
        {
            let comp = shell_comp(r#"#!{ printf '%s' $({ "made" }); }"#);
            let command = single_shell_exec(&comp);
            let Comp::Bind(ref computed, ref computed_binder, ref rest) = *command
            else {
                panic!("computed host expression must be hoisted before argv bind");
            };
            assert_eq!("%tmp0", computed_binder);
            assert_ret_string(computed.as_ref(), "made");

            let Comp::Bind(ref argv_bound, ref argv_binder, ref exec) = **rest
            else {
                panic!("argv bind must follow computed host expression");
            };
            assert_eq!("%tmp1", argv_binder);
            let Comp::Ret(ref argv_value) = **argv_bound
            else {
                panic!("argv bind must return an annotated variable");
            };
            assert_eq!(
                argv_value.as_ref(),
                &Value::annot(Value::var("%tmp0"), ValueType::string())
            );

            let args = argv(exec_payload(exec.as_ref()));
            assert_eq!(2, args.len(), "format plus computed argv value");
            assert_eq!(args[0].as_ref(), &Value::string("%s"));
            assert_annotated_var_arg(args[1].as_ref(), "%tmp1");
        }

        /// The argv list from an `Exec::exec` payload.
        fn argv(payload: &Value) -> &[Rc<Value>]
        {
            let Value::List(ref args) = *record_field(payload, host::FIELD_ARGS)
            else {
                panic!("args field must be a list: {payload:?}");
            };
            args
        }

        /// A field from an `Exec::exec` payload record.
        fn record_field<'value, 'text>(
            value: &'value Value,
            field: impl Into<TestText<'text>>,
        ) -> &'value Value
        {
            let field = field.into().0;
            let Value::Record(ref fields) = *value
            else {
                panic!("payload must be a record, got {value:?}");
            };
            fields.get(field).map_or_else(
                || panic!("payload missing {field:?}: {value:?}"),
                Rc::as_ref,
            )
        }

        /// Asserts a bound computation is exactly `ret "literal"`.
        fn assert_ret_string<'text>(
            comp: &Comp,
            expected: impl Into<TestText<'text>>,
        )
        {
            let expected = expected.into().0;
            let Comp::Ret(ref value) = *comp
            else {
                panic!("computed host expression must be Ret, got {comp:?}");
            };
            assert_eq!(value.as_ref(), &Value::string(expected));
        }

        /// Asserts a host-escape argv element is an annotated variable.
        fn assert_annotated_var_arg<'text>(
            value: &Value,
            expected: impl Into<TestText<'text>>,
        )
        {
            let expected = expected.into().0;
            let Value::Annot(ref inner, ref ty) = *value
            else {
                panic!("host escape must be String-annotated, got {value:?}");
            };
            assert_eq!(ty.as_ref(), &ValueType::string());
            assert_eq!(inner.as_ref(), &Value::var(expected));
        }

        /// Asserts `Exec::exec` payload children are in canonical record order.
        fn assert_exec_payload_child_paths<'path, 'text>(
            lowered: &Lowered,
            payload_path: impl Into<TestPath<'path>>,
            program: impl Into<TestText<'text>>,
            argc: impl Into<TestCount>,
        )
        {
            let payload_path = payload_path.into().0;
            let program = program.into().0;
            let argc = argc.into().0;
            assert_value_path(lowered, &child_path(payload_path, 0), "args", |value| {
                matches!(value, Value::List(elements) if elements.len() == argc).into()
            });
            assert_value_path(lowered, &child_path(payload_path, 1), "mode", |value| {
                (value == &Value::string(host::MODE_CAPTURED)).into()
            });
            assert_value_path(lowered, &child_path(payload_path, 2), "program", |value| {
                (value == &Value::string(program)).into()
            });
        }

        /// Appends a single path component.
        fn child_path<'path>(
            path: impl Into<TestPath<'path>>,
            child: impl Into<TestPathComponent>,
        ) -> TestOwnedPath
        {
            let path = path.into().0;
            let child = child.into().0;
            let mut child_path = path.to_vec();
            child_path.push(child);
            TestOwnedPath(child_path)
        }

        /// One value or computation node in an iterative variable search.
        #[derive(Clone, Copy)]
        enum SearchNode<'term>
        {
            Value(&'term Value),
            Comp(&'term Comp),
        }

        /// Whether a finite value or computation tree contains a variable.
        fn contains_var<'term, 'text>(
            root: SearchNode<'term>,
            name: impl Into<TestText<'text>>,
        ) -> TestDecision
        {
            let name = name.into().0;
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                match node {
                    | SearchNode::Value(node) => match *node {
                        | Value::Var(ref candidate) if candidate == name => {
                            return true.into();
                        },
                        | Value::Pair(ref left, ref right) => {
                            pending.push(SearchNode::Value(right));
                            pending.push(SearchNode::Value(left));
                        },
                        | Value::Inj(_, ref payload) | Value::Annot(ref payload, _) => {
                            pending.push(SearchNode::Value(payload));
                        },
                        | Value::List(ref elements) => {
                            pending.extend(elements.iter().map(|value| SearchNode::Value(value)));
                        },
                        | Value::Record(ref fields) => {
                            pending.extend(fields.values().map(|value| SearchNode::Value(value)));
                        },
                        | Value::Thunk(_, ref body) => {
                            pending.push(SearchNode::Comp(body));
                        },
                        | _ => {},
                    },
                    | SearchNode::Comp(node) => match *node {
                        | Comp::App(ref function, ref argument) => {
                            pending.push(SearchNode::Value(argument));
                            pending.push(SearchNode::Comp(function));
                        },
                        | Comp::Ret(ref value) | Comp::Force(ref value) => {
                            pending.push(SearchNode::Value(value));
                        },
                        | Comp::Bind(ref bound, _, ref rest) => {
                            pending.push(SearchNode::Comp(rest));
                            pending.push(SearchNode::Comp(bound));
                        },
                        | Comp::Case(ref scrutinee, (_, ref left), (_, ref right)) => {
                            pending.push(SearchNode::Comp(right));
                            pending.push(SearchNode::Comp(left));
                            pending.push(SearchNode::Value(scrutinee));
                        },
                        | Comp::RecordProj { ref record, .. } => {
                            pending.push(SearchNode::Value(record));
                        },
                        | _ => {},
                    },
                }
            }
            false.into()
        }
        /// Computed members are bound once, left-to-right; later member
        /// computations can refer to earlier binders, and the final record does
        /// not duplicate either computation.
        #[test]
        fn computed_module_members_sequence_once_and_scope_left_to_right()
        {
            let item = sole_item("module M { def first = 1 + 2; def second = first + 3; }");
            let Term::Comp(Comp::Bind(ref first_bound, ref first_binder, ref first_rest)) =
                item.term
            else {
                panic!("computed members must sequence in binds: {:?}", item.term);
            };
            assert_eq!("first", first_binder);
            assert!(
                matches!(**first_bound, Comp::App(_, _)),
                "first is computed once"
            );

            let Comp::Bind(ref second_bound, ref second_binder, ref second_rest) = **first_rest
            else {
                panic!("second computed member must be the next bind: {first_rest:?}");
            };
            assert_eq!("second", second_binder);
            assert!(
                matches!(**second_bound, Comp::App(_, _)),
                "second is computed once"
            );
            assert!(
                bool::from(comp_contains_var(second_bound, "first")),
                "the later member computation must see the earlier binder"
            );

            let Comp::Ret(ref record) = **second_rest
            else {
                panic!("module chain must return the final record: {second_rest:?}");
            };
            let Value::Record(ref fields) = **record
            else {
                panic!("module result must be a record: {record:?}");
            };
            assert!(
                fields
                    .values()
                    .all(|value| matches!(**value, Value::Var(_))),
                "the final record reuses member binders instead of duplicating computations: {fields:?}"
            );
        }

        /// Whether a computation contains a variable occurrence.
        fn comp_contains_var<'text>(
            comp: &Comp,
            name: impl Into<TestText<'text>>,
        ) -> TestDecision
        {
            contains_var(SearchNode::Comp(comp), name)
        }

        /// The prelude context types every operator the table elaborates to.
        #[test]
        fn prelude_covers_the_operator_table()
        {
            let ctx = prelude_ctx();
            for name in [
                "add", "sub", "mul", "eq", "ne", "lt", "le", "gt", "ge", "concat", "and", "or",
                "neg",
            ] {
                assert!(
                    matches!(ctx.lookup(name), Some(&ValueType::Thunk(grade, _)) if grade == Grade::OMEGA),
                    "prelude operator {name} must be an ω-graded thunk"
                );
            }
        }
    }
    /// Acceptance coverage for the first recursion-surface resolver rung.
    mod recursion_surface
    {
        use super::*;

        #[test]
        fn marked_self_reference_reaches_recursive_definition_lowering() -> Result<(), String>
        {
            let error = lowering_error("def rec f(n: Integer) -> F Integer { ret f[<](n) }")?;
            assert!(
                matches!(
                    error,
                    LowerError::Unsupported { kind, .. } if kind == node_kinds::DEF_REC
                ),
                "the marked self-call must satisfy scope validation and reach the \
                 not-yet-lowered recursive definition boundary, got {error:?}"
            );
            Ok(())
        }

        #[test]
        fn unmarked_self_reference_is_a_hard_error_with_a_marked_suggestion() -> Result<(), String>
        {
            let error = lowering_error("def rec f(n: Integer) -> F Integer { ret f(n) }")?;
            assert!(matches!(
                error,
                LowerError::UnmarkedRecursiveReference {
                    ref name,
                    ref suggestion,
                    ..
                } if name == "f" && suggestion == "f[<](…)"
            ));
            Ok(())
        }

        #[test]
        fn qualified_outer_reference_escapes_the_recursive_binding() -> Result<(), String>
        {
            let error = lowering_error("def rec f(n: Integer) -> F Integer { ret (outer.f)(n) }")?;
            assert!(
                matches!(
                    error,
                    LowerError::Unsupported { kind, .. } if kind == node_kinds::DEF_REC
                ),
                "the qualified call must pass recursive-name resolution and reach the \
                 not-yet-lowered recursive definition boundary, got {error:?}"
            );
            Ok(())
        }

        #[test]
        fn marked_mutual_group_reaches_recursive_group_lowering() -> Result<(), String>
        {
            let error = lowering_error(concat!(
                "rec { ",
                "def even(n: Integer) -> F Integer { ret odd[<](n) } ",
                "def odd(n: Integer) -> F Integer { ret even[<](n) }",
                " }",
            ))?;
            assert!(
                matches!(
                    error,
                    LowerError::Unsupported { kind, .. } if kind == node_kinds::REC_BLOCK
                ),
                "the marked group must satisfy scope validation and reach the \
                 not-yet-lowered recursive-group boundary, got {error:?}"
            );
            Ok(())
        }

        #[test]
        fn unmarked_mutual_peer_is_a_hard_error_with_a_marked_suggestion() -> Result<(), String>
        {
            let error = lowering_error(concat!(
                "rec { ",
                "def even(n: Integer) -> F Integer { ret odd(n) } ",
                "def odd(n: Integer) -> F Integer { ret even[<](n) }",
                " }",
            ))?;
            assert!(matches!(
                error,
                LowerError::UnmarkedRecursiveReference {
                    ref name,
                    ref suggestion,
                    ..
                } if name == "odd" && suggestion == "odd[<](…)"
            ));
            Ok(())
        }

        #[test]
        fn reserved_marker_residents_have_named_decline_messages() -> Result<(), String>
        {
            let cases = [
                (
                    "def rec f(n: Integer) -> F Integer { ret f[n <](n) }",
                    "reserved for named measures",
                ),
                (
                    "def rec f(n: Integer) -> F Integer { ret f[n = 1](n) }",
                    "reserved for explicit instantiation",
                ),
                (
                    "def rec f(n: Integer) -> F Integer { ret f[size = 1](n) }",
                    "reserved for explicit sizes",
                ),
                (
                    "def rec f(n: Integer) -> F Integer { ret f[cost = 1](n) }",
                    "reserved for cost bounds",
                ),
                (
                    "def rec f(n: Integer) -> F Integer { ret f[tail](n) }",
                    "reserved for tail-call assertions",
                ),
            ];
            for (source, expected) in cases {
                let error = lowering_error(source)?;
                assert!(
                    error.to_string().contains(expected),
                    "reserved resident must name its decline class: {error}"
                );
            }
            Ok(())
        }

        #[test]
        fn total_lowering_recovers_scope_errors_as_item_local_holes() -> Result<(), String>
        {
            let lowered = lower_source_total(
                "def ok = 1; def rec f(n: Integer) -> F Integer { ret f(n) }".into(),
            )
            .map_err(|error| format!("total lowering failed: {error}"))?;
            let names = lowered
                .items
                .iter()
                .filter_map(|item| item.name.as_deref())
                .collect::<Vec<_>>();
            assert_eq!(
                vec!["ok", "f"],
                names,
                "a recursive scope error must not consume its successful sibling item"
            );
            assert!(
                lowered.origin.iter().any(|(_id, entry)| {
                    matches!(
                        entry.note,
                        Some(HoleNote::UnsupportedForm { kind })
                            if kind == node_kinds::IDENTIFIER
                    )
                }),
                "the failed recursive item must carry an item-local unmarked-reference hole"
            );
            Ok(())
        }

        #[test]
        fn total_lowering_recovers_mutual_scope_errors_as_item_local_holes() -> Result<(), String>
        {
            let lowered = lower_source_total(
                concat!(
                    "rec { ",
                    "def even(n: Integer) -> F Integer { ret odd(n) } ",
                    "def odd(n: Integer) -> F Integer { ret even[<](n) }",
                    " } ",
                    "def ok = 1;",
                )
                .into(),
            )
            .map_err(|error| format!("total lowering failed: {error}"))?;
            assert_eq!(
                2,
                lowered.items.len(),
                "a failed recursive group and its successful sibling each yield one item"
            );
            assert!(
                lowered
                    .items
                    .iter()
                    .any(|item| item.name.as_deref() == Some("ok")),
                "the successful sibling after a failed recursive group must survive"
            );
            assert!(
                lowered.origin.iter().any(|(_id, entry)| {
                    matches!(
                        entry.note,
                        Some(HoleNote::UnsupportedForm { kind })
                            if kind == node_kinds::IDENTIFIER
                    )
                }),
                "the failed recursive group must carry an item-local unmarked-reference hole"
            );
            Ok(())
        }

        fn lowering_error(source: &str) -> Result<LowerError, String>
        {
            lower_source(source.into())
                .err()
                .ok_or_else(|| "strict lowering must decline the source".to_owned())
        }
    }
}
