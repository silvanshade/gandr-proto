//! Observable TUI launch path.

#[cfg(test)]
mod tests
{
    use gandr_surface_render_remote::present::HlRole;
    use gandr_surface_syntax::SourceSlice;
    use gandr_surface_tui::App;
    use gandr_surface_tui::SMOKE_NOTE;
    use gandr_surface_tui::app::AppKey;
    use gandr_surface_tui::run_smoke;
    use gandr_surface_tui::theme::style_of;
    use gandr_surface_tui::view::draw;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

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

    /// Draw one frame and read it back a cell at a time.
    ///
    /// Each row is a list of `(symbol, foreground colour)` pairs, addressed
    /// by column. A row is deliberately NOT flattened into a `String` first:
    /// the pane borders are multi-byte glyphs, so a byte offset into the
    /// joined text is not the column the cell was painted at, and a witness
    /// built on that offset reads the colours of neighbouring cells.
    fn drawn_rows(app: &App) -> Vec<Vec<(String, ratatui::style::Color)>>
    {
        let mut terminal =
            Terminal::new(TestBackend::new(120, 24)).expect("the test backend must open");
        terminal
            .draw(|frame| draw(frame, app))
            .expect("the frame must draw");
        let buffer = terminal.backend().buffer().clone();
        let area = buffer.area;
        (0 .. area.height)
            .map(|row| {
                (0 .. area.width)
                    .map(|column| {
                        let cell = &buffer[(column, row)];
                        (String::from(cell.symbol()), cell.fg)
                    })
                    .collect()
            })
            .collect()
    }

    /// The colours of the cells spelling `word`, at the first column where it
    /// appears in `rows`.
    fn painted_word(
        rows: &[Vec<(String, ratatui::style::Color)>],
        word: SourceSlice<'_>,
    ) -> Option<Vec<ratatui::style::Color>>
    {
        let symbols: Vec<String> = word.as_ref().chars().map(String::from).collect();
        rows.iter().find_map(|row| {
            row.windows(symbols.len()).find_map(|window| {
                window
                    .iter()
                    .zip(symbols.iter())
                    .all(|pair| pair.0.0 == *pair.1)
                    .then(|| window.iter().map(|&(_, colour)| colour).collect())
            })
        })
    }

    /// A submitted definition is drawn with its keyword in the keyword
    /// colour.
    ///
    /// This is the terminal end of the highlighter wiring, and it is stated
    /// against the role map rather than against "some colour appeared", so a
    /// wrong role fails it as loudly as no role at all. Before the encoder
    /// asked the highlighter, every transcript block carried an empty span
    /// set, `paint_source` took its plain-text early return, and this whole
    /// path was unreachable from any shipping caller.
    #[test]
    fn a_submitted_keyword_is_painted_in_the_keyword_colour()
    {
        let mut app = App::new();
        app.input = String::from("def one() -> F Integer { ret 1 }");
        let stop = app.handle(AppKey::Enter);
        assert!(stop.is_none(), "submitting does not stop the app");
        let block = app
            .transcript
            .last()
            .expect("a complete definition submits");
        assert!(
            !block.source_hl.is_empty(),
            "the encoder supplies classified spans: {block:?}"
        );

        let keyword = style_of(HlRole::Keyword).fg.expect("keywords are coloured");
        let rows = drawn_rows(&app);
        let painted = painted_word(&rows, SourceSlice::from("def"))
            .expect("the submitted source is drawn in the transcript pane");
        assert!(
            painted.iter().all(|&colour| colour == keyword),
            "the def keyword is painted in the keyword colour: {painted:?} against {keyword:?}"
        );
    }

    /// The painted frame carries more than one foreground colour.
    ///
    /// The companion to the witness above, and the one that is sensitive to
    /// the whole wiring rather than to one role: with no spans every cell in
    /// the frame is drawn at the terminal default, so this assertion can see
    /// the difference the ablation makes.
    #[test]
    fn the_painted_frame_is_not_uniformly_default()
    {
        let mut app = App::new();
        app.input = String::from("def one() -> F Integer { ret 1 }");
        let _stop = app.handle(AppKey::Enter);
        let rows = drawn_rows(&app);
        let mut colours: Vec<ratatui::style::Color> = rows
            .iter()
            .flat_map(|row| row.iter().map(|&(_, colour)| colour))
            .collect();
        colours.sort_by_key(|colour| format!("{colour:?}"));
        colours.dedup();
        assert!(
            colours.len() > 1,
            "a classified frame is painted in more than one colour: {colours:?}"
        );
    }
}
