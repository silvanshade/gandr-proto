//! Observable loop contracts: completeness, submit, and encoding.

#[cfg(test)]
mod tests
{
    use gandr_surface_parser::CompletionStatus;
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
                        pair.0 == OutKind::Diag && pair.1.starts_with("[E0001]")
                    }),
                    "the merged verdict stream must show the coded outcome-only refusal: {:?}",
                    block.lines
                );
            },
            | event => panic!("expected a refused submission, got {event:?}"),
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
        let status = run_batch(&input[..], &mut output);
        assert_eq!(i32::from(status), 0_i32, "a value batch completes");
        let text = String::from_utf8(output).expect("transcript is utf-8");
        assert!(text.contains('4'), "the transcript names the value: {text}");
    }

    #[test]
    fn unused_completion_status_name_stays_in_scope()
    {
        let _: fn(CompletionStatus) -> bool = bool::from;
    }
}
