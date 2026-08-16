//! The parser recovery obligations, end to end: the parse's own buffer, the
//! lowering that carries it, the report rows projected from it, and the
//! render-bus cards a producer hands a renderer.
//!
//! The classes this suite exercises:
//!
//! 1. [`tests::authority`] — the parse is the only obligation authority: the
//!    lowering carries the melder's buffer verbatim, and a clean source carries
//!    nothing.
//! 2. [`tests::rows`] — the report rows preserve class and exact byte span, in
//!    source order rather than the parse's severity order, deterministically.
//! 3. [`tests::recovery`] — a malformed declaration followed by a valid one
//!    reports the repair and keeps the later declaration, with the diagnostic,
//!    goal, mark, and attribute surfaces unchanged.
//! 4. [`tests::bus`] — the render-bus projection carries the rows and
//!    advertises the capability it actually backs.
//! 5. [`tests::codec`] — the report JSON and the render frame both round-trip
//!    the rows, empty and non-empty.

/// Recovery-obligation acceptance tests.
#[cfg(test)]
mod tests
{
    use gandr_surface_engine::diag::ObligationReport;
    use gandr_surface_engine::diag::SCHEMA_VERSION;
    use gandr_surface_engine::diag::Span;
    use gandr_surface_engine::diag::obligation_class;
    use gandr_surface_engine::diag::obligations;
    use gandr_surface_engine::diag::report;
    use gandr_surface_engine::lower::Lowered;
    use gandr_surface_engine::lower::lower_source_total;
    use gandr_surface_engine::prelude_ctx;
    use gandr_surface_engine::remote::obligation_cards;
    use gandr_surface_engine::remote::server_caps;
    use gandr_surface_grammar::built_in;
    use gandr_surface_parser::Oblig;
    use gandr_surface_parser::parse;
    use gandr_surface_render_remote::present::ByteOffset;
    use gandr_surface_render_remote::present::ObligationClass;
    use gandr_surface_syntax::SourceSlice;

    use crate::common::TestText;

    /// A malformed declaration (the stray `~` the melder cannot mold) followed
    /// by a well-formed one — the recovery-continuation shape.
    const MALFORMED_THEN_VALID: &str = "def bad = 1 ~ 2;\ndef good = 2;\n";

    /// A source whose obligations disagree about order: the most severe
    /// obligation (the unmolded `~`) is the *last* in the source, behind two
    /// lower-severity ghost-tile repairs.
    const SEVERITY_AND_SOURCE_ORDER_DISAGREE: &str = "def k = thunk { ret 1 ;\ndef b = 1 ~ 2;\n";

    /// A source that parses clean.
    const CLEAN: &str = "def good = 2;\n";

    /// Total-lowers a source, which is total over every input.
    fn lower<'text>(source: impl Into<TestText<'text>>) -> Lowered
    {
        let source = source.into().0;
        lower_source_total(source.into())
            .unwrap_or_else(|error| panic!("total lowering must succeed: {error}\n{source}"))
    }

    /// The source text a report row spans.
    fn spanned<'text>(
        source: impl Into<TestText<'text>>,
        span: &Span,
    ) -> TestText<'text>
    {
        TestText(
            source
                .into()
                .0
                .get(span.start .. span.end)
                .unwrap_or_else(|| panic!("row span {span:?} must lie within the source")),
        )
    }

    /// Class 1: the parse is the only obligation authority.
    mod authority
    {
        use super::*;

        #[test]
        fn lowered_carries_the_parse_obligations_verbatim()
        {
            // The melder decides what a source's obligations are; lowering
            // carries that buffer and re-derives nothing, so the engine's slice
            // is element-for-element the parser's own.
            let pbg = built_in().expect("the built-in grammar is checked");
            for source in [MALFORMED_THEN_VALID, SEVERITY_AND_SOURCE_ORDER_DISAGREE] {
                let parsed = parse(&pbg, SourceSlice::from(source)).expect("the parse is total");
                let lowered = lower(source);
                assert_eq!(
                    parsed.obligations(),
                    lowered.obligations(),
                    "the lowering carries the parse's obligations verbatim for {source:?}"
                );
                assert!(
                    !lowered.obligations().is_empty(),
                    "{source:?} is a recovering source and must carry obligations"
                );
            }
        }

        #[test]
        fn a_clean_source_carries_no_obligations()
        {
            let pbg = built_in().expect("the built-in grammar is checked");
            let parsed = parse(&pbg, SourceSlice::from(CLEAN)).expect("the parse is total");
            assert!(
                bool::from(parsed.is_clean()),
                "the fixture must parse clean for this to witness anything"
            );
            assert!(lower(CLEAN).obligations().is_empty());
        }

        #[test]
        fn every_parser_class_maps_to_its_own_name_and_rank()
        {
            // The single crossing between the parser taxonomy and the published
            // vocabulary: each class keeps its own name, and the severity ladder
            // survives the crossing.
            let expected = [
                (Oblig::MissingMeld, ObligationClass::MissingMeld),
                (Oblig::MissingTile, ObligationClass::MissingTile),
                (Oblig::IncompleteTile, ObligationClass::IncompleteTile),
                (Oblig::UnmoldedTok, ObligationClass::UnmoldedTok),
                (Oblig::InconMeld, ObligationClass::InconMeld),
                (Oblig::ExtraMeld, ObligationClass::ExtraMeld),
                (Oblig::ReservedKeyword, ObligationClass::ReservedKeyword),
                (Oblig::AmbiguousPrec, ObligationClass::AmbiguousPrec),
            ];
            assert_eq!(
                Oblig::all().len(),
                expected.len(),
                "the table must cover every parser class"
            );
            for (class, name) in expected {
                assert_eq!(name, obligation_class(class), "{class:?} keeps its name");
            }
            for pair in expected.windows(2) {
                if let &[(lower_class, lower_name), (higher_class, higher_name)] = pair {
                    assert!(
                        lower_class < higher_class && lower_name < higher_name,
                        "the ladder survives the crossing at {lower_class:?} < {higher_class:?}"
                    );
                }
            }
        }
    }

    /// Class 2: the report rows.
    mod rows
    {
        use super::*;

        #[test]
        fn rows_carry_the_class_and_the_exact_span()
        {
            // One repair, one row: the class the melder assigned and the bytes
            // it held responsible — here the stray token itself.
            let lowered = lower(MALFORMED_THEN_VALID);
            let rows = obligations(&lowered);
            assert_eq!(
                vec![ObligationReport {
                    class: ObligationClass::UnmoldedTok,
                    span: Span { start: 12, end: 13 },
                }],
                rows
            );
            assert_eq!(TestText("~"), spanned(MALFORMED_THEN_VALID, &rows[0].span));
        }

        #[test]
        fn a_clean_source_reports_no_obligations()
        {
            let lowered = lower(CLEAN);
            assert!(obligations(&lowered).is_empty());
            assert!(report(&lowered, &prelude_ctx()).obligations.is_empty());
        }

        #[test]
        fn rows_are_in_source_order_not_severity_order()
        {
            // The parse buffers by severity (its minimization order), which puts
            // the last obligation in the source first. The rows are read in
            // source order, so the projection sorts rather than inherits.
            let lowered = lower(SEVERITY_AND_SOURCE_ORDER_DISAGREE);
            let carried = lowered.obligations();
            assert_eq!(
                Oblig::UnmoldedTok,
                carried[0].class,
                "the parse buffers the most severe obligation first"
            );
            let rows = obligations(&lowered);
            assert_eq!(
                vec![
                    ObligationClass::MissingTile,
                    ObligationClass::MissingTile,
                    ObligationClass::UnmoldedTok,
                ],
                rows.iter().map(|row| row.class).collect::<Vec<_>>()
            );
            let starts: Vec<usize> = rows.iter().map(|row| row.span.start).collect();
            let mut sorted = starts.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, starts, "the rows ascend by span start");
            assert_eq!(
                TestText("~"),
                spanned(SEVERITY_AND_SOURCE_ORDER_DISAGREE, &rows[2].span)
            );
        }

        #[test]
        fn rows_are_deterministic_across_lowerings()
        {
            for source in [
                MALFORMED_THEN_VALID,
                SEVERITY_AND_SOURCE_ORDER_DISAGREE,
                CLEAN,
            ] {
                assert_eq!(
                    obligations(&lower(source)),
                    obligations(&lower(source)),
                    "{source:?} produces the same rows on every lowering"
                );
            }
        }
    }

    /// Class 3: recovery keeps everything else intact.
    mod recovery
    {
        use super::*;

        #[test]
        fn recovery_then_a_valid_declaration_reports_the_repair_and_keeps_both()
        {
            let lowered = lower(MALFORMED_THEN_VALID);
            let names: Vec<Option<&str>> = lowered
                .items
                .iter()
                .map(|item| item.name.as_deref())
                .collect();
            assert_eq!(
                vec![Some("bad"), Some("good")],
                names,
                "the malformed declaration is holed in place and the later one lowers intact"
            );

            let built = report(&lowered, &prelude_ctx());
            assert_eq!(SCHEMA_VERSION, built.schema_version);
            assert_eq!(
                vec![ObligationReport {
                    class: ObligationClass::UnmoldedTok,
                    span: Span { start: 12, end: 13 },
                }],
                built.obligations,
                "the repair is reported once, at the responsible bytes"
            );

            // The other surfaces are what they were before obligations landed:
            // the recovery hole is a goal, its node is marked, nothing fails to
            // type, and no attribute is claimed.
            assert!(
                built.diagnostics.is_empty(),
                "a holed declaration types, so nothing is diagnosed: {:?}",
                built.diagnostics
            );
            assert_eq!(1, built.goals.len(), "the recovery hole is the one goal");
            assert_eq!(1, built.marks.len(), "the hole is the one marked node");
            assert!(built.attributes.is_empty());
        }

        #[test]
        fn a_clean_source_keeps_every_surface_empty()
        {
            let built = report(&lower(CLEAN), &prelude_ctx());
            assert!(built.obligations.is_empty());
            assert!(built.diagnostics.is_empty());
            assert!(built.goals.is_empty());
            assert!(built.marks.is_empty());
            assert!(built.attributes.is_empty());
        }
    }

    /// Class 4: the render-bus projection.
    mod bus
    {
        use super::*;

        #[test]
        fn cards_preserve_the_report_rows()
        {
            for source in [MALFORMED_THEN_VALID, SEVERITY_AND_SOURCE_ORDER_DISAGREE] {
                let built = report(&lower(source), &prelude_ctx());
                let cards = obligation_cards(&built);
                assert_eq!(
                    built.obligations.len(),
                    cards.len(),
                    "{source:?}: one card per row"
                );
                for (row, card) in built.obligations.iter().zip(&cards) {
                    assert_eq!(row.class, card.class);
                    assert_eq!(ByteOffset::from(row.span.start), card.range.start);
                    assert_eq!(ByteOffset::from(row.span.end), card.range.end);
                }
            }
        }

        #[test]
        fn a_clean_source_produces_no_cards()
        {
            let built = report(&lower(CLEAN), &prelude_ctx());
            assert!(obligation_cards(&built).is_empty());
        }

        #[test]
        fn advertised_capabilities_match_the_live_path()
        {
            // The capability is backed: the same crate that advertises rows
            // produces them. It is a claim about the path, not about a
            // document, so a clean source leaves it set with an empty row set.
            let caps = server_caps();
            assert!(bool::from(caps.obligations));
            assert!(
                !bool::from(caps.deltas),
                "delta streaming has no producer here"
            );
            assert!(
                !bool::from(caps.sessions),
                "session badges have no producer here"
            );

            let recovering = report(&lower(MALFORMED_THEN_VALID), &prelude_ctx());
            assert!(
                !obligation_cards(&recovering).is_empty(),
                "the advertised row path must actually produce rows"
            );
            let clean = report(&lower(CLEAN), &prelude_ctx());
            assert!(obligation_cards(&clean).is_empty());
            assert!(bool::from(server_caps().obligations));
        }
    }

    /// Class 5: the serialized surfaces.
    #[cfg(feature = "codecs")]
    mod codec
    {
        use gandr_surface_render_remote::wire::DocVersion;
        use gandr_surface_render_remote::wire::FrameBody;
        use gandr_surface_render_remote::wire::MachineView;
        use gandr_surface_render_remote::wire::RenderFrame;
        use gandr_surface_render_remote::wire::ReportView;

        use super::*;

        #[test]
        fn report_json_carries_the_rows_and_round_trips()
        {
            let built = report(&lower(SEVERITY_AND_SOURCE_ORDER_DISAGREE), &prelude_ctx());
            let json = serde_json::to_value(&built).expect("the report serializes");
            assert_eq!(
                serde_json::json!([
                    {"class": "MissingTile", "span": {"start": 6_usize, "end": 7_usize}},
                    {"class": "MissingTile", "span": {"start": 22_usize, "end": 23_usize}},
                    {"class": "UnmoldedTok", "span": {"start": 34_usize, "end": 35_usize}},
                ]),
                json["obligations"],
                "the rows are class-and-span data, with no opaque payload"
            );
            let back: gandr_surface_engine::diag::Report =
                serde_json::from_value(json).expect("the report deserializes");
            assert_eq!(built, back, "the round trip preserves the whole report");
        }

        #[test]
        fn an_empty_row_set_serializes_as_an_empty_array()
        {
            let built = report(&lower(CLEAN), &prelude_ctx());
            let json = serde_json::to_value(&built).expect("the report serializes");
            assert_eq!(serde_json::json!([]), json["obligations"]);
            let back: gandr_surface_engine::diag::Report =
                serde_json::from_value(json).expect("the report deserializes");
            assert_eq!(built, back);
        }

        #[test]
        fn a_render_frame_round_trips_the_produced_cards()
        {
            // End to end: a source, its report, its cards, a bus frame, the
            // wire, and back — with the rows still exactly what the parse said.
            let built = report(&lower(SEVERITY_AND_SOURCE_ORDER_DISAGREE), &prelude_ctx());
            let cards = obligation_cards(&built);
            let frame = RenderFrame::frame(
                "file:///a.gandr".to_owned(),
                DocVersion::from(1_i32),
                ReportView {
                    obligations: cards.clone(),
                    ..ReportView::default()
                },
                MachineView::default(),
            );
            let json = serde_json::to_string(&frame).expect("the frame serializes");
            let back: RenderFrame = serde_json::from_str(&json).expect("the frame deserializes");
            assert_eq!(frame, back);
            let FrameBody::Frame { ref report, .. } = *back.body()
            else {
                panic!("the body is a frame body");
            };
            assert_eq!(cards, report.obligations);
            assert_eq!(
                built
                    .obligations
                    .iter()
                    .map(|row| row.class)
                    .collect::<Vec<_>>(),
                report
                    .obligations
                    .iter()
                    .map(|card| card.class)
                    .collect::<Vec<_>>(),
                "the classes survive the wire"
            );
        }
    }
}
