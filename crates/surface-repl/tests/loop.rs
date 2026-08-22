//! Observable loop contracts: completeness, submit, and encoding.

#[cfg(test)]
mod tests
{
    use gandr_surface_diagnostics::RenderStyle;
    use gandr_surface_parser::CompletionStatus;
    use gandr_surface_render_remote::present::HlRole;
    use gandr_surface_render_remote::present::OutKind;
    use gandr_surface_repl::LoopEvent;
    use gandr_surface_repl::SessionLoop;
    use gandr_surface_repl::completeness;
    use gandr_surface_repl::run_batch;
    use gandr_surface_syntax::SourceSlice;

    #[test]
    fn an_open_form_is_incomplete()
    {
        let status = completeness(SourceSlice::from("(")).expect("grammar must assemble");
        assert!(!bool::from(status), "an open form still expects tokens");
    }

    #[test]
    fn a_bare_atom_is_complete()
    {
        let status = completeness(SourceSlice::from("42")).expect("grammar must assemble");
        assert!(bool::from(status), "a bare atom is parse-complete");
    }

    #[test]
    fn a_hole_is_complete()
    {
        let status = completeness(SourceSlice::from("?")).expect("grammar must assemble");
        assert!(
            bool::from(status),
            "a hole is typeable, so it is not incomplete"
        );
    }

    #[test]
    fn an_open_form_continues()
    {
        let mut session = SessionLoop::new();
        let event = session
            .offer(SourceSlice::from("("))
            .expect("completeness must run");
        assert_eq!(event, LoopEvent::Continue, "incomplete source is kept");
    }

    #[test]
    fn a_complete_atom_submits()
    {
        let mut session = SessionLoop::new();
        match session
            .offer(SourceSlice::from("42"))
            .expect("submit must run")
        {
            | LoopEvent::Submitted(block) => {
                assert_eq!(block.source, "42");
                assert!(
                    block
                        .lines
                        .iter()
                        .any(|pair| pair.0 == OutKind::Value && pair.1.contains('4')),
                    "a value line must mention the result: {:?}",
                    block.lines
                );
            },
            | event => panic!("expected a submission, got {event:?}"),
        }
    }

    #[test]
    fn a_definition_is_visible_on_the_next_line()
    {
        let mut session = SessionLoop::new();
        match session
            .offer(SourceSlice::from("def y = 5;"))
            .expect("definition must submit")
        {
            | LoopEvent::Submitted(block) => {
                assert!(
                    block
                        .lines
                        .iter()
                        .any(|pair| pair.0 == OutKind::Type && pair.1.contains('y')),
                    "a definition encodes as a type line: {:?}",
                    block.lines
                );
            },
            | event => panic!("expected a definition submission, got {event:?}"),
        }
        match session
            .offer(SourceSlice::from("y"))
            .expect("use must submit")
        {
            | LoopEvent::Submitted(block) => {
                assert!(
                    block.lines.iter().any(|pair| pair.0 == OutKind::Value),
                    "the later line must evaluate: {:?}",
                    block.lines
                );
            },
            | event => panic!("expected a use submission, got {event:?}"),
        }
    }

    #[test]
    fn a_hole_encodes_as_a_goal_line()
    {
        let mut session = SessionLoop::new();
        match session
            .offer(SourceSlice::from("?"))
            .expect("a hole must submit")
        {
            | LoopEvent::Submitted(block) => {
                assert!(
                    block
                        .lines
                        .iter()
                        .any(|pair| pair.0 == OutKind::Goal || pair.0 == OutKind::Info),
                    "a hole must surface as a goal or an info line: {:?}",
                    block.lines
                );
            },
            | event => panic!("expected a hole submission, got {event:?}"),
        }
    }

    /// The REPL exposes an outcome-only refusal without inventing a source
    /// span, while retaining the stable diagnostic code and operands.
    ///
    /// # Contract
    /// - ensures: an outcome-only type refusal is an unlocated diagnostic block
    ///   containing the code and both semantic operand labels.
    /// - provides: a regression witness for the facade routing seam.
    /// - panics: none beyond an unmet observable contract.
    #[test]
    fn an_outcome_only_refusal_is_visible_in_the_repl()
    {
        let mut session = SessionLoop::new();
        let _definition = session
            .offer(SourceSlice::from(
                r#"def fst(a: Type, f: U[ω] (a -> F a), g: U[ω] (a -> F a)) -> F (U(a -> F a)) {
  ret f
}"#,
            ))
            .expect("the definition must submit");
        match session
            .offer(SourceSlice::from(
                r#"def bad(a: Type, f: U[ω] (a -> F a), g: U[ω] (a -> F a)) -> F(Path((U(a -> F a)), fst(a, f, g), g)) {
  ret here(g)
}"#,
            ))
            .expect("the refused law must submit")
        {
            | LoopEvent::Submitted(block) => {
                assert!(
                    block.lines.iter().any(|pair| {
                        pair.0 == OutKind::Diag
                            && pair.1.contains("error[E0001]")
                            && pair.1.contains("expected")
                            && pair.1.contains("found")
                            && !pair.1.contains('━')
                    }),
                    "the merged verdict stream must show an honest unlocated refusal: {:?}",
                    block.lines
                );
            },
            | event => panic!("expected a refused submission, got {event:?}"),
        }
    }

    #[test]
    fn styled_session_diagnostics_reach_the_repl_transcript()
    {
        let mut session = SessionLoop::with_render_style(RenderStyle::Styled);
        match session
            .offer(SourceSlice::from("force(1)"))
            .expect("the styled refusal must submit")
        {
            | LoopEvent::Submitted(block) => {
                assert!(
                    block.lines.iter().any(|pair| {
                        pair.0 == OutKind::Diag
                            && pair.1.contains("\u{1b}[")
                            && pair.1.contains("while checking the forced value")
                    }),
                    "the selected styled facade output must reach the transcript: {:?}",
                    block.lines
                );
            },
            | event => panic!("expected a styled refusal, got {event:?}"),
        }
    }

    #[test]
    fn quit_stops_the_loop()
    {
        let mut session = SessionLoop::new();
        let event = session
            .offer(SourceSlice::from(":q"))
            .expect("meta-commands do not lower");
        assert_eq!(event, LoopEvent::Quit);
    }

    #[test]
    fn piped_value_prints_a_transcript()
    {
        let input = b"42\n";
        let mut output = Vec::new();
        let status = run_batch(&input[..], &mut output, RenderStyle::Plain);
        assert_eq!(i32::from(status), 0_i32, "a value batch completes");
        let text = String::from_utf8(output).expect("transcript is utf-8");
        assert!(text.contains('4'), "the transcript names the value: {text}");
    }

    /// The transcript block the shipping loop produces carries classified
    /// spans. Before the highlighter was wired, every block was built with an
    /// empty span vector unconditionally, so the terminal painter's styled
    /// path could not be reached from any production caller — a correct
    /// mechanism whose trigger condition was never met.
    ///
    /// This is the separating witness for that wiring: it goes red when the
    /// encoder stops asking the highlighter, and it names the mechanism it
    /// asserts, because an empty span set is exactly the ablated behaviour.
    #[test]
    fn a_submission_carries_highlight_spans()
    {
        let mut session = SessionLoop::new();
        match session
            .offer(SourceSlice::from("def one() -> F Integer { ret 1 }"))
            .expect("submit must run")
        {
            | LoopEvent::Submitted(block) => {
                assert!(
                    !block.source_hl.is_empty(),
                    "the echoed source arrives classified: {block:?}"
                );
                assert!(
                    block
                        .source_hl
                        .iter()
                        .any(|span| span.role == HlRole::Keyword),
                    "the def keyword is classified as a keyword: {:?}",
                    block.source_hl
                );
            },
            | other => panic!("a complete def submits: {other:?}"),
        }
    }

    /// The spans reaching the transcript are sorted and pairwise disjoint.
    ///
    /// The producing crate states this in its contract and witnesses it
    /// nowhere, and a witness written inside the producer's own picture could
    /// not refute the producer. This asserts it from the consuming side: a
    /// violation is a silently dropped span in the terminal painter and an
    /// LSP protocol violation on the sibling face.
    #[test]
    fn transcript_spans_are_sorted_and_disjoint()
    {
        let mut session = SessionLoop::new();
        match session
            .offer(SourceSlice::from("def one() -> F Integer { ret 1 }"))
            .expect("submit must run")
        {
            | LoopEvent::Submitted(block) => {
                assert!(!block.source_hl.is_empty(), "the fixture classifies");
                let mut cursor = 0_usize;
                for span in &block.source_hl {
                    assert!(
                        usize::from(span.range.start) >= cursor,
                        "sorted and disjoint at {span:?}, cursor {cursor}: {:?}",
                        block.source_hl
                    );
                    cursor = usize::from(span.range.end);
                }
            },
            | other => panic!("a complete def submits: {other:?}"),
        }
    }

    /// A buffer the validator never called complete is reported at end of
    /// input rather than dropped.
    ///
    /// The batch face used to return its success status with the pending
    /// buffer still held, so unparseable input produced no output at all and
    /// exited zero — a failure with no report, wearing the shape of success.
    #[test]
    fn an_incomplete_buffer_is_submitted_at_end_of_input()
    {
        let mut session = SessionLoop::new();
        let event = session
            .offer(SourceSlice::from("("))
            .expect("completeness must run");
        assert_eq!(event, LoopEvent::Continue, "an open form is kept");
        match session.finish().expect("finishing must run") {
            | Some(LoopEvent::Submitted(block)) => {
                assert_eq!(block.source, "(", "the pending buffer is what is submitted");
            },
            | other => panic!("a pending buffer is submitted at end of input: {other:?}"),
        }
    }

    #[test]
    fn finishing_an_empty_loop_yields_nothing()
    {
        let mut session = SessionLoop::new();
        assert!(
            session.finish().expect("finishing must run").is_none(),
            "nothing pending, nothing reported"
        );
    }

    #[test]
    fn finishing_twice_reports_once()
    {
        let mut session = SessionLoop::new();
        drop(session.offer(SourceSlice::from("(")).expect("completeness"));
        assert!(
            session.finish().expect("finishing must run").is_some(),
            "the first close reports the buffer"
        );
        assert!(
            session.finish().expect("finishing must run").is_none(),
            "the buffer is cleared, so the second close reports nothing"
        );
    }

    /// The end-to-end form of the same claim, through the shipping batch
    /// face: input the validator refuses leaves a report on the transcript
    /// instead of leaving silence at a successful exit.
    #[test]
    fn an_unparseable_pipe_reports_rather_than_going_quiet()
    {
        let input = b"@@@ !! nonsense\n";
        let mut output = Vec::new();
        let status = run_batch(&input[..], &mut output, RenderStyle::Plain);
        let text = String::from_utf8(output).expect("transcript is utf-8");
        assert!(
            !text.is_empty(),
            "unparseable input is reported, not swallowed (status {})",
            i32::from(status)
        );
    }

    #[test]
    fn unused_completion_status_name_stays_in_scope()
    {
        let _: fn(CompletionStatus) -> bool = bool::from;
    }
}
