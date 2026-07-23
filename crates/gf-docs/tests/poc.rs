//! `PoC` acceptance tests (gandr-wrs): migration shape, the `checkExpr` lanes,
//! and the end-to-end render.
//!
//! The runtime lanes skip cleanly when the `PoC` environment is absent
//! (no compiled PGF or no pgf-enabled Python), so the suite stays green on a
//! bare checkout; the mise `docs:gfd:poc` task provisions the environment and
//! exercises them for real.

use core::error::Error;
use std::path::PathBuf;

use gandr_gf_docs::migrate::translate_file;
use gandr_gf_docs::pipeline::build_body;
use gandr_gf_docs::pipeline::build_page;
use gandr_gf_docs::pipeline::copy_fonts;
use gandr_gf_docs::rt::GfRuntime as _;
use gandr_gf_docs::rt::PyPgf;

/// Shared result type for the `PoC` integration witnesses.
type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The repo root (the crate manifest dir's grandparent).
fn repo_root() -> PathBuf
{
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The migration source of truth for the `PoC`.
fn xml_path() -> PathBuf
{
    repo_root().join("docs/spec/component-vocabulary.xml")
}

/// The compiled grammar the `PoC` environment produces.
fn pgf_path() -> PathBuf
{
    repo_root().join("target/gf-docs/GandrDocsLex.pgf")
}

/// Load the runtime, or `None` when the `PoC` environment is unprovisioned.
fn runtime() -> Option<PyPgf>
{
    let pgf = pgf_path();
    if !pgf.exists() {
        return None;
    }
    PyPgf::load(&pgf.to_string_lossy(), "GandrDocsLexHtml").ok()
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// The migration emits every constructor the conversion target exercises.
    #[test]
    fn migration_emits_all_block_constructors() -> TestResult
    {
        let gfd = translate_file(&xml_path())?;
        let flat = gfd.split_whitespace().collect::<Vec<_>>().join(" ");
        for constructor in [
            "MkComponent anchor_component_vocabulary",
            "StatusPartial",
            "ConsAnchor anchor_spec_index",
            "MkSection anchor_cv_blocks",
            "DefinitionBlock anchor_cv_def_prose term_prose",
            "GrammarBlock",
            "MkProduction \"status\"",
            "JudgementsBlock \"Judgement forms\"",
            "RuleBlock anchor_cv_rule_app \"T-App\"",
            "InventoryBlock \"The payload blocks and links of the two-register weave\"",
            "MkHeaderRow",
            "MkBodyRow",
            "RegisterBlock",
            "MkItem \"first\"",
            "ConsInlineGlued",
            "ApiCodeBlock \"rust\"",
            "DiagramBlock anchor_cv_diagram",
            "ExampleBlock \"A code block anticipating output\"",
            "ExpectCodeBlock \"gandr\" \"6\"",
            "ConsCiteKey cite_P_2",
            "ConsCiteKey cite_A_1a",
            "TermDef term_component",
            "XRef anchor_cv_examples",
            "MathInline \"tilde(mu)\"",
        ] {
            assert!(
                flat.contains(constructor),
                "missing constructor text: {constructor}"
            );
        }
        Ok(())
    }

    /// A valid migrated document passes the mandatory `checkExpr` lane.
    #[test]
    fn valid_document_passes_check_lane() -> TestResult
    {
        let Some(runtime) = runtime()
        else {
            eprintln!("skip: PoC environment unprovisioned");
            return Ok(());
        };
        let gfd = translate_file(&xml_path())?;
        runtime.check(&gfd)?;
        Ok(())
    }

    /// A dangling term constant is rejected at the `checkExpr` lane.
    #[test]
    fn dangling_term_fails_check_lane() -> TestResult
    {
        let Some(runtime) = runtime()
        else {
            eprintln!("skip: PoC environment unprovisioned");
            return Ok(());
        };
        let gfd =
            translate_file(&xml_path())?.replace("TermRef term_status", "TermRef term_missing");
        let result = runtime.check(&gfd);
        let Err(error) = result
        else {
            return Err("a dangling term constant must be rejected".into());
        };
        let message = error.to_string();
        assert!(
            message.contains("Unknown function \"term_missing\""),
            "unexpected rejection: {message}"
        );
        Ok(())
    }

    /// The render carries clean anchors, escaped code, and no glue markers.
    #[test]
    fn render_contains_clean_anchors_and_escaped_code() -> TestResult
    {
        let Some(runtime) = runtime()
        else {
            eprintln!("skip: PoC environment unprovisioned");
            return Ok(());
        };
        let gfd = translate_file(&xml_path())?;
        let body = build_body(&runtime, &gfd)?;
        for expected in [
            "id=\"cv-overview\"",
            "href=\"#term-component\"",
            "id=\"term-component\"",
            "class=\"status-chip status-partial\"",
            "Vec&lt;Inline&gt;",
            "'&lt;component&gt;' status blocks* '&lt;/component&gt;'",
            "class=\"lang-rust api\"",
            "</sup>.",
            "Example: A code block anticipating output",
            "expected output: 6",
            "<figcaption>The payload blocks and links of the two-register weave</figcaption>",
        ] {
            assert!(
                body.contains(expected),
                "missing rendered fragment: {expected}"
            );
        }
        assert!(
            !body.contains('\u{200B}'),
            "glue markers survive the post-pass"
        );
        Ok(())
    }

    /// The page shell (gandr-4l9) carries its observable contract: the lifted
    /// `<title>`, the inlined stylesheet, the `<main>` landmark, and the
    /// vendored fonts beside the page.
    #[test]
    fn page_shell_carries_design_language_contract() -> TestResult
    {
        let Some(runtime) = runtime()
        else {
            eprintln!("skip: PoC environment unprovisioned");
            return Ok(());
        };
        let gfd = translate_file(&xml_path())?;
        let page = build_page(&runtime, &gfd, "component-vocabulary")?;
        for expected in [
            "<title>The component vocabulary</title>",
            "<meta name=\"viewport\"",
            "<main class=\"page\">",
            "<article>",
            "<style>",
            "--paper: #fffff8",
            "grid-template-columns: minmax(0, 62ch) minmax(0, 30ch)",
            "fonts/etbookot-roman-webfont.woff2",
        ] {
            assert!(
                page.contains(expected),
                "missing shell fragment: {expected}"
            );
        }
        let out_dir = repo_root().join("target/gf-docs");
        std::fs::create_dir_all(&out_dir)?;
        copy_fonts(&out_dir)?;
        for font in [
            "etbookot-roman-webfont.woff2",
            "etbookot-italic-webfont.woff2",
            "etbookot-bold-webfont.woff2",
            "LICENSE.et-book",
        ] {
            assert!(
                out_dir.join("fonts").join(font).is_file(),
                "missing copied font: {font}"
            );
        }
        Ok(())
    }
}
