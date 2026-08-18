//! Diagnostics-and-goals acceptance tests, one module per
//! class:
//!
//! 1. [`tests::coverage`] — the error corpus drives **every reachable**
//!    [`TypeError`](gandr_core_term::error::TypeError) variant (each shape,
//!    each hint) through the full pipeline (lower → machine), pinning that the
//!    diagnostics surface covers the inventory.
//! 2. [`tests::golden`] — one golden JSON [`Report`] per corpus fixture, plus
//!    the two hole-goal reports (with and without an expected type); regenerate
//!    with `GANDR_SURFACE_ENGINE_BLESS=1`.
//! 3. [`tests::invariants`] — every diagnostic span lies within the source; the
//!    envelope carries [`SCHEMA_VERSION`].
//! 4. [`tests::roundtrip`] — `serde_json` round-trip stability for every
//!    report, plus a hand-written wire-shape pin for the reserved `Other` /
//!    surface-unreachable `EffectRowMismatch` marks the corpus cannot
//!    construct.
//! 5. [`tests::semantic_marks`] — the incremental-pipeline design's marks
//!    surface: each reachable mark kind is covered, the marks oracle holds at
//!    the pipeline boundary (error marks iff ill-typed; well-typed hole-free ⇒
//!    no marks), `is_error` classifies the empty hole alone, every mark span
//!    lies within its source, no surface mark is silently dropped, and no
//!    surface source yields an effect-row or `Other` mark.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Acceptance tests for the diagnostics and goals JSON surface.
#[cfg(test)]
mod tests
{
    #[cfg(feature = "codecs")]
    use std::fs;
    #[cfg(feature = "codecs")]
    use std::io::Write as _;
    #[cfg(feature = "codecs")]
    use std::path::PathBuf;

    use gandr_surface_engine::diag::Diagnostic;
    use gandr_surface_engine::diag::DiagnosticDetail;
    use gandr_surface_engine::diag::MarkDetail;
    use gandr_surface_engine::diag::MarkReport;
    #[cfg(feature = "codecs")]
    use gandr_surface_engine::diag::Report;
    use gandr_surface_engine::diag::SCHEMA_VERSION;
    use gandr_surface_engine::diag::diagnostics;
    use gandr_surface_engine::diag::marks;
    use gandr_surface_engine::diag::report;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::prelude_ctx;

    use crate::common::TestText;

    /// The error corpus: `(name, descriptor, source)`. `descriptor` is the
    /// reachable-variant identity this fixture must drive — the bare kind, or
    /// `kind:detail` for the shape (`ShapeMismatch`) or hint (`StuckExpr`)
    /// refinements — so the table is a literal coverage matrix over the
    /// `TypeError` inventory. Each source goes through the *full* pipeline
    /// (`lower_source_total` → machine), so the goldens also pin lowering and
    /// origin behavior.
    ///
    /// Not represented: the two polarity-guard `ShapeMismatch` descriptions
    /// (`a value type` / `a computation type`) are unreachable by
    /// construction (`error.rs` module doc; a `gandr-core-checker` conformance
    /// meta-test pins this), so there is no surface source that drives them.
    const ERROR_CORPUS: &[(&str, &str, &str)] = &[
        (
            "type-mismatch",
            "TypeMismatch",
            "def x : Unit;\ndef x = 1;\n",
        ),
        ("shape-arrow", "ShapeMismatch:an arrow type", "(ret 1)(2)\n"),
        ("shape-thunk", "ShapeMismatch:a thunk type", "force(1)\n"),
        (
            "shape-returner",
            "ShapeMismatch:a returner type",
            "thunk { run x <- fn(y: Integer) { ret y }; ret x }\n",
        ),
        (
            "shape-sum",
            "ShapeMismatch:a sum type",
            "def c : U[1] (F Integer);\ndef c = thunk { case 1 { Inl(x) => ret x, Inr(y) => ret 0 } };\n",
        ),
        (
            // A motive-less split is check-only (rule Split⇓, dependent-split design), so the
            // product-shape mismatch is driven in *checking* position (the
            // declared `F Integer` answer, like `shape-sum` above); a bare
            // inferred split is stuck-needs-motive instead (see
            // `stuck-split-motive`).
            "shape-prod",
            "ShapeMismatch:a product type",
            "def c : U[1] (F Integer);\ndef c = thunk { val (x, y) = 1; ret x };\n",
        ),
        ("shape-with", "ShapeMismatch:a with-type", "(ret 1).fst\n"),
        (
            "stuck-inject",
            "StuckExpr:annotate this injection",
            "Inl(1)\n",
        ),
        (
            "stuck-binder",
            "StuckExpr:annotate the binder or check against an arrow type",
            "fn(x) { ret x }\n",
        ),
        (
            "stuck-abs-arrow",
            "StuckExpr:an abstraction only checks against an arrow type",
            "def g : U[1] (F Integer);\ndef g = thunk { fn(x) { ret x } };\n",
        ),
        (
            "stuck-case-infer",
            "StuckExpr:case only checks; annotate or supply an expected type",
            "case (Inl(1) : Integer + Integer) { Inl(x) => ret x, Inr(y) => ret 0 }\n",
        ),
        (
            "stuck-with-infer",
            "StuckExpr:a lazy pair only checks against a with-type",
            "co { fst = ret 1, snd = ret 2 }\n",
        ),
        (
            // A motive-less split in inference position is stuck (rule Split⇓ is
            // check-only; a split *infers* only with a dependent motive, rule
            // SplitMotive⇑; dependent-split design) — the lowerer emits motive-less splits, so
            // this fires whenever a `let (x, y) = …` is not in checking position.
            "stuck-split-motive",
            "StuckExpr:a motive-less split only checks; supply a dependent motive (z. M) to infer, or \
             an expected type",
            "thunk { val (x, y) = (1, 2); ret x }\n",
        ),
        ("unbound", "UnboundVariable", "nonesuch\n"),
        (
            "grade-thunk",
            "GradeError",
            "def t : U[ω] (F Integer);\ndef t = thunk[1] { ret 1 };\n",
        ),
        (
            "grade-force",
            "GradeError",
            "def f(z : U[0] (F Integer)) -> F Integer { force z }\n",
        ),
    ];

    /// The hole-goal corpus: `(name, source)` for the with/without-expected
    /// goal reports (D7: "the same report carries hole goals").
    const GOAL_CORPUS: &[(&str, &str)] = &[
        (
            "goals-expected",
            "def k : U[1] (F Integer);\ndef k = thunk { ret 1; };\n",
        ),
        (
            "goals-inferred",
            "def f : U[1] (Integer -> F Integer);\ndef f(x: Integer) -> F Integer { leta y = x; ret x }\n",
        ),
        (
            "goals-user-hole",
            "def k : U[1] (F Integer);\ndef k = thunk { ? };\ndef g : U[1] (F Integer);\ndef g = thunk { ret ?seed };\n",
        ),
    ];

    /// The reachable-variant descriptor of one diagnostic (see
    /// [`ERROR_CORPUS`]).
    fn descriptor(diagnostic: &Diagnostic) -> String
    {
        match diagnostic.detail {
            | DiagnosticDetail::TypeMismatch { .. } => "TypeMismatch".to_owned(),
            | DiagnosticDetail::ShapeMismatch {
                ref expected_shape, ..
            } => format!("ShapeMismatch:{expected_shape}"),
            | DiagnosticDetail::StuckExpr { ref hint } => format!("StuckExpr:{hint}"),
            | DiagnosticDetail::UnboundVariable { .. } => "UnboundVariable".to_owned(),
            | DiagnosticDetail::GradeError { .. } => "GradeError".to_owned(),
            | DiagnosticDetail::Other => "Other".to_owned(),
            | _ => "?".to_owned(),
        }
    }

    /// The golden report path for a corpus name.
    #[cfg(feature = "codecs")]
    fn golden_path<'text>(name: impl Into<TestText<'text>>) -> PathBuf
    {
        let name = name.into().0;
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden"))
            .join(format!("{name}.report.snap"))
    }

    /// The report for one source, pretty-printed JSON.
    #[cfg(feature = "codecs")]
    fn report_json<'text>(source: impl Into<TestText<'text>>) -> String
    {
        let source = source.into().0;
        let lowered = lower_source_total(source.into())
            .unwrap_or_else(|error| panic!("total lowering must succeed: {error}\n{source}"));
        report(&lowered, &prelude_ctx())
            .to_json()
            .unwrap_or_else(|error| panic!("report must serialize: {error}"))
    }

    /// Acceptance class 1: every reachable variant is covered.
    mod coverage
    {
        use super::*;

        /// Each corpus fixture drives a diagnostic whose descriptor matches
        /// the table — the literal coverage matrix.
        #[test]
        fn corpus_covers_each_reachable_variant()
        {
            for &(name, expected, source) in ERROR_CORPUS {
                let lowered = lower_source_total(source.into())
                    .unwrap_or_else(|error| panic!("{name}: total lowering must succeed: {error}"));
                let diags = diagnostics(&lowered, &prelude_ctx());
                let got: Vec<String> = diags.iter().map(descriptor).collect();
                assert!(
                    got.iter().any(|found| found == expected),
                    "{name}: expected a {expected} diagnostic, got {got:?}"
                );
            }
        }
    }

    /// Acceptance class 2: golden reports.
    #[cfg(feature = "codecs")]
    mod golden
    {
        use super::*;
        /// Every error-corpus fixture matches its golden report.
        #[test]
        fn error_corpus_reports_match_goldens()
        {
            for &(name, _expected, source) in ERROR_CORPUS {
                check_golden(name, source);
            }
        }
        /// Both hole-goal fixtures match their golden reports.
        #[test]
        fn goal_corpus_reports_match_goldens()
        {
            for &(name, source) in GOAL_CORPUS {
                check_golden(name, source);
            }
        }

        /// Compares (or, under `GANDR_SURFACE_ENGINE_BLESS`, rewrites) one
        /// golden.
        fn check_golden<'name, 'source>(
            name: impl Into<TestText<'name>>,
            source: impl Into<TestText<'source>>,
        )
        {
            let name = name.into().0;
            let source = source.into().0;
            let rendered = report_json(source);
            let path = golden_path(name);
            if std::env::var_os("GANDR_SURFACE_ENGINE_BLESS").is_some() {
                let mut file = fs::File::create(&path)
                    .unwrap_or_else(|error| panic!("blessing {path:?} must open: {error}"));
                file.write_all(rendered.as_bytes())
                    .unwrap_or_else(|error| panic!("blessing {path:?} must write: {error}"));
                return;
            }
            let expected = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("golden {path:?} must be committed: {error}"));
            assert_eq!(
                rendered, expected,
                "report for {name} drifted (GANDR_SURFACE_ENGINE_BLESS=1 to regenerate)"
            );
        }
    }

    /// Acceptance class 3: report invariants.
    mod invariants
    {
        use super::*;

        /// Every diagnostic span lies within its source, and every envelope
        /// carries the current schema version.
        #[test]
        fn spans_are_in_source_and_schema_is_versioned()
        {
            for &(name, _expected, source) in ERROR_CORPUS {
                let lowered = lower_source_total(source.into())
                    .unwrap_or_else(|error| panic!("{name}: total lowering must succeed: {error}"));
                let built = report(&lowered, &prelude_ctx());
                assert_eq!(
                    SCHEMA_VERSION, built.schema_version,
                    "{name}: the envelope must carry the schema version"
                );
                for diagnostic in &built.diagnostics {
                    if let Some(ref span) = diagnostic.span {
                        assert!(
                            span.start <= span.end && span.end <= source.len(),
                            "{name}: span {span:?} must lie within the source"
                        );
                        assert!(
                            source.get(span.start .. span.end).is_some(),
                            "{name}: span {span:?} must be a valid source slice"
                        );
                    }
                }
                for goal in &built.goals {
                    assert!(
                        goal.span.end <= source.len(),
                        "{name}: goal span {:?} must lie within the source",
                        goal.span
                    );
                }
            }
        }
    }

    /// Acceptance class 4: `serde_json` round-trip stability.
    #[cfg(feature = "codecs")]
    mod roundtrip
    {
        use super::*;

        /// Serialize → deserialize → serialize is the identity on the JSON
        /// string for every corpus report.
        #[test]
        fn reports_round_trip_through_json()
        {
            let sources = ERROR_CORPUS
                .iter()
                .map(|&(_name, _expected, source)| source)
                .chain(GOAL_CORPUS.iter().map(|&(_name, source)| source));
            for source in sources {
                let first = report_json(source);
                let parsed: Report = serde_json::from_str(&first)
                    .unwrap_or_else(|error| panic!("report must deserialize: {error}\n{first}"));
                let again = parsed
                    .to_json()
                    .unwrap_or_else(|error| panic!("report must re-serialize: {error}"));
                assert_eq!(first, again, "round-trip must be stable");
            }
        }

        /// The reserved catch-all `Other` and the surface-unreachable
        /// `EffectRowMismatch` cannot be constructed from the corpus (no
        /// surface source yields them), so their wire shape is pinned by
        /// round-tripping a hand-written JSON image: `Other` is the bare
        /// `{"kind":"Other"}` with no `data` key, the tagged
        /// `EffectRowMismatch` nests under `data`, and both survive
        /// deserialize → serialize → deserialize unchanged. Guards the
        /// serde attributes (`tag` / `content` / `flatten`) against a
        /// regression the corpus goldens cannot reach.
        #[test]
        fn catch_all_and_effect_row_mark_shapes_round_trip()
        {
            let images = [
                r#"{"kind":"Other","is_error":true,"item":0,"span":{"start":0,"end":1}}"#,
                r#"{"kind":"EffectRowMismatch","data":{"expected":"Comp(F(Atom(\"Integer\")))","actual":"Comp(F(Unknown))"},"is_error":true,"item":1,"span":{"start":2,"end":5},"analyzed":"Comp(F(Atom(\"Integer\")))","synthesized":"Comp(Unknown)"}"#,
            ];
            for image in images {
                let parsed: MarkReport = serde_json::from_str(image).unwrap_or_else(|error| {
                    panic!("mark image must deserialize: {error}\n{image}")
                });
                let again = serde_json::to_string(&parsed)
                    .unwrap_or_else(|error| panic!("mark must re-serialize: {error}"));
                let reparsed: MarkReport = serde_json::from_str(&again).unwrap_or_else(|error| {
                    panic!("re-serialized mark must deserialize: {error}\n{again}")
                });
                assert_eq!(
                    parsed, reparsed,
                    "mark round-trip must be stable for {image}"
                );
            }
            // `Other` keeps the bare tagged shape (no `data` key).
            let other: MarkReport =
                serde_json::from_str(images[0]).expect("the Other image must deserialize");
            let other_json = serde_json::to_string(&other).expect("the Other mark must serialize");
            assert!(
                other_json.contains("\"kind\":\"Other\"") && !other_json.contains("\"data\""),
                "Other must serialize as a bare kind tag: {other_json}"
            );
        }
    }

    /// Acceptance class 5: the incremental-pipeline design's semantic marks.
    mod semantic_marks
    {
        use super::*;

        /// Well-typed, hole-free sources — the oracle's negative direction (no
        /// error marks, and no marks at all since there are no holes).
        const WELL_TYPED_CORPUS: &[&str] = &[
            "ret 1\n",
            "1\n",
            "thunk { ret 1 }\n",
            "force (thunk { ret 1 })\n",
        ];

        /// `(name, expected-kind, source)` — one fixture per **reachable** mark
        /// kind. `EffectRowMismatch` and the `Other` catch-all are not yet
        /// reachable from the Stage-1 surface fragment (no surface form lowers
        /// to an effect-row subtype failure), exactly as the two polarity-guard
        /// `ShapeMismatch` descriptions are unreachable for the diagnostics
        /// corpus; they have no fixture here by design.
        const MARK_CORPUS: &[(&str, &str, &str)] = &[
            (
                "type-mismatch",
                "TypeMismatch",
                "def x : Unit;\ndef x = 1;\n",
            ),
            ("shape", "ShapeMismatch", "(ret 1)(2)\n"),
            ("stuck", "Stuck", "Inl(1)\n"),
            ("free", "FreeVariable", "nonesuch\n"),
            (
                "grade-budget",
                "GradeBudget",
                "def t : U[ω] (F Integer);\ndef t = thunk[1] { ret 1 };\n",
            ),
            (
                "thunkability",
                "Thunkability",
                "def f(z : U[0] (F Integer)) -> F Integer { force z }\n",
            ),
            (
                "empty-hole",
                "EmptyHole",
                "def k : U[1] (F Integer);\ndef k = thunk { ? };\n",
            ),
        ];

        /// The kind descriptor of one mark detail (the coverage identity).
        fn descriptor(detail: &MarkDetail) -> TestText<'static>
        {
            match *detail {
                | MarkDetail::EmptyHole { .. } => "EmptyHole".into(),
                | MarkDetail::PatternHole { .. } => "PatternHole".into(),
                | MarkDetail::TypeMismatch { .. } => "TypeMismatch".into(),
                | MarkDetail::EffectRowMismatch { .. } => "EffectRowMismatch".into(),
                | MarkDetail::ShapeMismatch { .. } => "ShapeMismatch".into(),
                | MarkDetail::FreeVariable { .. } => "FreeVariable".into(),
                | MarkDetail::GradeBudget { .. } => "GradeBudget".into(),
                | MarkDetail::Thunkability { .. } => "Thunkability".into(),
                | MarkDetail::Stuck { .. } => "Stuck".into(),
                | MarkDetail::Other => "Other".into(),
            }
        }
        /// Each reachable mark kind is surfaced by its corpus fixture — the
        /// literal coverage matrix over [`MarkDetail`].
        #[test]
        fn corpus_covers_each_reachable_mark_kind()
        {
            for &(name, expected, source) in MARK_CORPUS {
                let got: Vec<TestText<'static>> = marks_of(source)
                    .iter()
                    .map(|mark| descriptor(&mark.detail))
                    .collect();
                assert!(
                    got.contains(&TestText::from(expected)),
                    "{name}: expected a {expected} mark, got {got:?}"
                );
            }
        }
        /// The oracle at the pipeline boundary: an ill-typed item surfaces at
        /// least one error mark; a well-typed item surfaces none.
        #[test]
        fn oracle_error_marks_iff_ill_typed()
        {
            for &(name, _expected, source) in ERROR_CORPUS {
                let surfaced = marks_of(source);
                assert!(
                    surfaced.iter().any(|mark| mark.is_error),
                    "{name}: an ill-typed source must surface an error mark, got {surfaced:?}"
                );
            }
            // Well-typed and hole-free: no marks at all — the strict negative
            // direction (a spurious mark of any kind, error or not, would make
            // the vector non-empty, since the marker emits only on a hole or a
            // failure).
            for &source in WELL_TYPED_CORPUS {
                let surfaced = marks_of(source);
                assert!(
                    surfaced.is_empty(),
                    "well-typed hole-free source must surface no mark: {source:?} got {surfaced:?}"
                );
            }
            // Well-typed but hole-bearing: the accept-side classification — a
            // real non-error mark is surfaced (the empty hole), never an error.
            let with_hole = "def k : U[1] (F Integer);\ndef k = thunk { ? };\n";
            let surfaced = marks_of(with_hole);
            assert!(
                !surfaced.is_empty() && surfaced.iter().all(|mark| !mark.is_error),
                "a well-typed hole-bearing source must surface only non-error marks, got {surfaced:?}"
            );
        }
        /// `is_error` is `false` for exactly the empty-hole mark, across the
        /// whole error + goal corpus.
        #[test]
        fn is_error_classifies_empty_hole_only()
        {
            let sources = ERROR_CORPUS
                .iter()
                .map(|&(_name, _expected, source)| source)
                .chain(GOAL_CORPUS.iter().map(|&(_name, source)| source));
            for source in sources {
                for mark in marks_of(source) {
                    let is_hole = matches!(mark.detail, MarkDetail::EmptyHole { .. });
                    assert_eq!(
                        mark.is_error, !is_hole,
                        "is_error must be false exactly for an empty hole: {mark:?}"
                    );
                }
            }
        }
        /// Every mark span lies within its source (the marks invariant; nodes
        /// without a source identity are dropped, so every surfaced mark has a
        /// valid span).
        #[test]
        fn mark_spans_lie_in_source()
        {
            let sources = ERROR_CORPUS
                .iter()
                .map(|&(_name, _expected, source)| source)
                .chain(GOAL_CORPUS.iter().map(|&(_name, source)| source));
            for source in sources {
                for mark in marks_of(source) {
                    assert!(
                        mark.span.start <= mark.span.end && mark.span.end <= source.len(),
                        "mark span {:?} must lie within the source",
                        mark.span
                    );
                    assert!(
                        source.get(mark.span.start .. mark.span.end).is_some(),
                        "mark span {:?} must be a valid source slice",
                        mark.span
                    );
                }
            }
        }
        /// No surface source yields an `EffectRowMismatch` or `Other` mark —
        /// the two kinds the coverage matrix declares unreachable from
        /// the Stage-1 fragment. A guard so that promoting effects
        /// (`perform` / `handle`) into the surface forces a deliberate
        /// coverage-matrix update rather than silently leaving a kind
        /// uncovered.
        #[test]
        fn no_surface_source_yields_effect_or_other_mark()
        {
            let sources = ERROR_CORPUS
                .iter()
                .map(|&(_name, _expected, source)| source)
                .chain(GOAL_CORPUS.iter().map(|&(_name, source)| source));
            for source in sources {
                for mark in marks_of(source) {
                    assert!(
                        !matches!(
                            mark.detail,
                            MarkDetail::EffectRowMismatch { .. } | MarkDetail::Other
                        ),
                        "unexpected {:?} mark from surface source {source:?}",
                        mark.detail
                    );
                }
            }
        }

        /// The marks for one source, through the full pipeline.
        fn marks_of<'text>(source: impl Into<TestText<'text>>) -> Vec<MarkReport>
        {
            let source = source.into().0;
            let lowered = lower_source_total(source.into())
                .unwrap_or_else(|error| panic!("total lowering must succeed: {error}\n{source}"));
            marks(&lowered, &prelude_ctx())
        }

        /// No silent drop: for every surface program in the corpus, every
        /// marked node in the *raw* marking is `OriginMap`-addressable — i.e.
        /// `marks()` drops nothing (the `Stk`-interior drop path is forward-
        /// compat-dead on surface input). This pins the term → origin coverage
        /// the marks surface depends on, against a future lowering regression
        /// that adds a term child without a matching origin entry.
        /// Drives the marker
        /// directly (mirroring `mark_item`), so it observes drops
        /// `marks()` would otherwise hide.
        #[test]
        fn surface_marks_are_never_dropped()
        {
            use gandr_core_checker::discipline::mark::Marking;
            use gandr_core_checker::discipline::mark::mark_comp;
            use gandr_core_checker::discipline::mark::mark_value;
            use gandr_core_checker::machine::control::Dir;
            use gandr_core_incremental::region::Item;
            use gandr_core_term::ctx::Ctx;
            use gandr_core_term::syntax::Term;
            use gandr_core_term::types::Ty;

            fn marking_of(
                item: &Item,
                base: &Ctx,
            ) -> Marking
            {
                match (&item.term, &item.ascription) {
                    | (&Term::Value(ref value), &Some(Ty::Value(ref expected))) => {
                        mark_value(base.clone(), value.clone(), Dir::Check(expected.clone()))
                    },
                    | (&Term::Value(ref value), _) => {
                        mark_value(base.clone(), value.clone(), Dir::Infer)
                    },
                    | (&Term::Comp(ref comp), &Some(Ty::Comp(ref expected))) => {
                        mark_comp(base.clone(), comp.clone(), Dir::Check(expected.clone()))
                    },
                    | (&Term::Comp(ref comp), _) => {
                        mark_comp(base.clone(), comp.clone(), Dir::Infer)
                    },
                }
            }

            let base = prelude_ctx();
            let sources = ERROR_CORPUS
                .iter()
                .map(|&(_name, _expected, source)| source)
                .chain(GOAL_CORPUS.iter().map(|&(_name, source)| source));
            for source in sources {
                let lowered = lower_source_total(source.into()).unwrap_or_else(|error| {
                    panic!("total lowering must succeed: {error}\n{source}")
                });
                for (item_index, item) in lowered.items.iter().enumerate() {
                    let marking = marking_of(item, &base);
                    let target = u32::try_from(item_index).expect("corpus item index fits u32");
                    for (node_path, node_id) in marking.compatibility_paths() {
                        let Some(facts) = marking.get(*node_id)
                        else {
                            continue;
                        };
                        if facts.marks.is_empty() {
                            continue;
                        }
                        let mut path = vec![target];
                        path.extend_from_slice(node_path);
                        assert!(
                            lowered.origin.get_path(&path).is_some(),
                            "marked node {path:?} has no OriginMap entry (silent drop) in {source:?}"
                        );
                    }
                }
            }
        }
    }
}
