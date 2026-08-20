//! Observable TUI launch path.

#[cfg(test)]
mod tests
{
    use gandr_surface_tui::App;
    use gandr_surface_tui::SMOKE_NOTE;
    use gandr_surface_tui::app::AppKey;
    use gandr_surface_tui::run_smoke;

    #[test]
    fn smoke_writes_the_launch_note()
    {
        let mut output = Vec::new();
        run_smoke(&mut output).expect("the smoke face must draw");
        assert_eq!(
            output.as_slice(),
            SMOKE_NOTE.as_bytes(),
            "the launch note is the smoke observable"
        );
    }

    #[test]
    fn an_outcome_refusal_is_visible_in_the_transcript_pane()
    {
        let mut app = App::new();
        app.input = String::from(
            "def fst(a: Type, f: U[ω] (a -> F a), g: U[ω] (a -> F a)) -> F (U(a -> F a)) { ret f }",
        );
        let _continue = app.handle(AppKey::Enter);
        app.input = String::from(
            "def bad(a: Type, f: U[ω] (a -> F a), g: U[ω] (a -> F a)) -> F(Path((U(a -> F a)), fst(a, f, g), g)) { ret here(g) }",
        );
        let _continue = app.handle(AppKey::Enter);
        let block = app
            .transcript
            .last()
            .expect("the refused definition must produce a transcript block");
        assert!(
            block
                .lines
                .iter()
                .any(|line| line.1.contains("type mismatch")),
            "the TUI transcript must retain the merged refusal: {:?}",
            block.lines
        );
    }
}
