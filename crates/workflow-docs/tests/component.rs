//! The component-vocabulary corpus witness: constructor coverage, the
//! `checkExpr` lanes, and the end-to-end render of the committed `.gfd`.
//!
//! The runtime lanes skip cleanly when the `GF` environment is absent
//! (no compiled PGF or no pgf-enabled Python), so the suite stays green on a
//! bare checkout; the mise corpus arc provisions the environment and
//! exercises them for real.

use core::error::Error;
use std::path::PathBuf;

use gandr_workflow_docs::bibliography;
use gandr_workflow_docs::pipeline::PostContext;
use gandr_workflow_docs::pipeline::build_body;
use gandr_workflow_docs::pipeline::build_page;
use gandr_workflow_docs::pipeline::copy_fonts;
use gandr_workflow_docs::typst_leaf;
use gandr_workflow_grammatical_framework::rt::ExprText;
use gandr_workflow_grammatical_framework::rt::GfRuntime as _;
use gandr_workflow_grammatical_framework::rt::LanguageName;
use gandr_workflow_grammatical_framework::rt::PyPgf;

use crate::common::repo_root;

/// Shared result type for the corpus witnesses.
type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The committed component-vocabulary corpus file.
fn gfd_path() -> PathBuf
{
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/component-vocabulary.gfd")
}

/// Read the committed component-vocabulary document.
fn gfd() -> TestResult<String>
{
    Ok(std::fs::read_to_string(gfd_path())?)
}

/// The compiled grammar the corpus environment produces.
fn pgf_path() -> PathBuf
{
    repo_root().join("target/gf/GandrDocsLex.pgf")
}

/// Load the runtime, or `None` when the `GF` environment is unprovisioned.
fn runtime() -> Option<PyPgf>
{
    let pgf = pgf_path();
    if !pgf.exists() {
        return None;
    }
    PyPgf::new(&pgf, &LanguageName::new("GandrDocsLexHtml")).ok()
}

/// The post-pass context against the repo's real bibliography and the shared
/// leaf cache, or `None` when `refs.yml` is unavailable.
fn context() -> Option<(bibliography::Bibliography, PathBuf)>
{
    let bibliography = bibliography::load(&repo_root().join("docs/spec/refs.yml")).ok()?;
    let cache_dir = typst_leaf::default_cache_dir(&repo_root().join("target/gf"));
    Some((bibliography, cache_dir))
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// The committed component carries every block constructor of the
    /// conversion target's inventory.
    #[test]
    fn component_carries_all_block_constructors() -> TestResult
    {
        let flat = gfd()?.split_whitespace().collect::<Vec<_>>().join(" ");
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
            "RegisterBlock OrderedList",
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
            "CodeInline \"def rec\"",
        ] {
            assert!(
                flat.contains(constructor),
                "missing constructor text: {constructor}"
            );
        }
        Ok(())
    }

    /// The committed component passes the mandatory `checkExpr` lane.
    #[test]
    fn valid_document_passes_check_lane() -> TestResult
    {
        let Some(runtime) = runtime()
        else {
            eprintln!("skip: GF environment unprovisioned");
            return Ok(());
        };
        runtime.check(&ExprText::new(gfd()?))?;
        Ok(())
    }

    /// A dangling term constant is rejected at the `checkExpr` lane.
    #[test]
    fn dangling_term_fails_check_lane() -> TestResult
    {
        let Some(runtime) = runtime()
        else {
            eprintln!("skip: GF environment unprovisioned");
            return Ok(());
        };
        let gfd = gfd()?.replace("TermRef term_status", "TermRef term_missing");
        let result = runtime.check(&ExprText::new(gfd));
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
        let (Some(runtime), Some((bibliography, cache_dir))) = (runtime(), context())
        else {
            eprintln!("skip: GF environment unprovisioned");
            return Ok(());
        };
        let context = PostContext::new(&bibliography, &cache_dir);
        let body = build_body(&runtime, &ExprText::new(gfd()?), &context)?;
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
        let (Some(runtime), Some((bibliography, cache_dir))) = (runtime(), context())
        else {
            eprintln!("skip: GF environment unprovisioned");
            return Ok(());
        };
        let context = PostContext::new(&bibliography, &cache_dir);
        let page = build_page(
            &runtime,
            &ExprText::new(gfd()?),
            "component-vocabulary".into(),
            &context,
            &[],
        )?;
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
        let out_dir = repo_root().join("target/gf");
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
