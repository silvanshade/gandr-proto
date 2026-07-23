//! Integration gate: the in-tree prose document classes validate clean.
//!
//! This puts the doc-tool prose classes (research records, workflow docs, and
//! per-crate status) on the `cargo nextest` merge wall — a malformed,
//! mis-cited, or duplicate-label document under `docs/research`,
//! `docs/workflow`, or `crates/*/docs` fails here. The doc-family
//! parse-equals-validate pass is the same discipline `check-docs` runs; this
//! test drives it over the real tree.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the unit and property tests \
                  readable (docs/workflow/rust.md)"
    )
)]

#[cfg(test)]
mod tests
{
    use std::path::PathBuf;

    use gandr_workflow_docs::corpus;

    /// The workspace root, two levels above this crate's manifest directory.
    fn workspace_root() -> PathBuf
    {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// Every prose document under the class roots parses and validates without
    /// a diagnostic, and at least the dogfood research record is present.
    #[test]
    fn prose_document_corpus_is_clean()
    {
        let root = workspace_root();
        let refs = root.join("docs/spec/refs.yml");
        let outcome = corpus::check_docs(&root, &refs);
        assert!(
            outcome.is_ok(),
            "check-docs failed operationally: {:?}",
            outcome.as_ref().err(),
        );
        let Ok(report) = outcome
        else {
            return;
        };
        assert!(
            report.record_count >= 1,
            "expected at least the dogfood research record, found {}",
            report.record_count,
        );
        assert!(
            report.diagnostics.is_empty(),
            "prose document corpus has {} diagnostic(s): {:?}",
            report.diagnostics.len(),
            report.diagnostics,
        );
    }
}
