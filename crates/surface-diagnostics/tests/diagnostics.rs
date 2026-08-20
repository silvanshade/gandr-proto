#[cfg(test)]
mod tests
{
    use std::path::Path;

    use gandr_surface_diagnostics::render_submission;
    use gandr_surface_engine::session::Session;
    use gandr_surface_syntax::SourceSlice;

    /// A type mismatch remains a located terminal report after the session
    /// merges report diagnostics with outcome-level refusals.
    ///
    /// # Contract
    /// - requires: the source is accepted by the session's lowering seam.
    /// - ensures: the report names the source path, expected type, and actual
    ///   type.
    /// - provides: a golden rendering that catches facade drift in layout and
    ///   labels.
    /// - panics: only when the checked fixture or its expected report is
    ///   malformed.
    #[test]
    fn a_type_mismatch_renders_as_a_located_report()
    {
        const SOURCE: &str = "def x : Unit;\ndef x = 1;\n";
        let mut session = Session::new();
        let submission = session
            .submit(SOURCE)
            .expect("the mismatch fixture must lower into a report");
        let reports = render_submission(
            SourceSlice::from(SOURCE),
            Some(Path::new("mismatch.gandr")),
            &submission,
        );
        assert_eq!(1, reports.len(), "one mismatch should produce one report");
        let rendered = &reports[0];
        assert!(
            rendered.contains("mismatch.gandr"),
            "the report must name its source"
        );
        assert!(
            rendered.contains("expected"),
            "the report must label the expectation"
        );
        assert!(
            rendered.contains("found"),
            "the report must label the actual type"
        );
        assert_eq!(
            include_str!("golden/type-mismatch.txt"),
            rendered,
            "the terminal layout is a public surface"
        );
    }
}
