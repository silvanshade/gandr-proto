//! The migration dual-run gate (gandr-5n6): the legacy renderer and the `GF`
//! pipeline both build every component, and per-section normalized text must
//! match.
//!
//! Named intentional deltas (proposal §7's presentation moves, all
//! attribute-level or additive): section-status chips and component status
//! styling are stripped before comparison, compiled `SVG` leaves are elided
//! on both sides, registry-scoped hrefs (cross-component term/anchor pages,
//! gandr-38l) differ while their link text does not, and the legacy
//! concatenation artifact — source line-break whitespace surfacing as a space
//! before `.,;:!?` — is normalized away (the `GF` glue lane binds punctuation
//! to its left neighbour by design).
//!
//! The runtime lane skips cleanly when the `GF` environment is absent, so the
//! suite stays green on a bare checkout; the mise corpus arc exercises it.

use core::error::Error;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

use gandr_gf_docs::pipeline::PostContext;
use gandr_gf_docs::pipeline::build_body;
use gandr_gf_docs::rt::PyPgf;
use gandr_workflow_docs::bibliography;
use gandr_workflow_docs::corpus;
use gandr_workflow_docs::typst_leaf;

/// Shared result type for the migration witnesses.
type TestResult<T = ()> = Result<T, Box<dyn Error>>;

/// The repo root (the crate manifest dir's grandparent).
fn repo_root() -> PathBuf
{
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Load the runtime, or `None` when the `GF` environment is unprovisioned.
fn runtime() -> Option<PyPgf>
{
    let pgf = repo_root().join("target/gf-docs/GandrDocsLex.pgf");
    if !pgf.exists() {
        return None;
    }
    PyPgf::load(&pgf.to_string_lossy(), "GandrDocsLexHtml").ok()
}

/// The `.gfd` corpus files, sorted.
fn gfd_files() -> Vec<PathBuf>
{
    let mut files = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .read_dir()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "gfd"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// Extract the `<body>` content of a full page.
fn body_of(page: &str) -> &str
{
    page.split_once("<body>")
        .and_then(|(_, rest)| rest.split_once("</body>"))
        .map_or(page, |(body, _)| body)
}

/// Segment rendered markup by section anchor, then normalize each segment to
/// its text content. The preamble (back-link, title, provenance) segments
/// under the empty key.
fn segments(html: &str) -> BTreeMap<String, String>
{
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    let mut key = String::new();
    let mut rest = html;
    loop {
        match rest.split_once("<section id=\"") {
            | Some((before, after)) => {
                map.entry(core::mem::take(&mut key))
                    .or_default()
                    .push_str(before);
                let (id, tail) = after.split_once('"').unwrap_or((after, ""));
                key = id.to_owned();
                rest = tail.split_once('>').map_or(tail, |(_, body)| body);
            },
            | None => {
                map.entry(key).or_default().push_str(rest);
                break;
            },
        }
    }
    map.into_iter()
        .map(|(id, chunk)| (id, normalize(&chunk)))
        .collect()
}

/// Normalize one markup chunk to comparable text: elide `SVG` leaves and
/// status styling, strip tags to boundary spaces, decode entities, absorb the
/// presentation deltas (`::= ` is CSS-generated content in the new pages, the
/// `Definition (term).` scaffolding is new presentation), collapse whitespace.
fn normalize(chunk: &str) -> String
{
    let mut text = chunk.to_owned();
    for (pattern, replacement) in [
        (r"(?s)<figcaption>.*?</figcaption>", ""),
        (r#"(?s)(<div class="rule"[^>]*>)<h3>[^<]*</h3>"#, "$1"),
        (r"(?s)<svg.*?</svg>", ""),
        (r#"(?s)<p class="status[^"]*">[^<]*</p>"#, ""),
        (r#"(?s)<span class="status-chip[^"]*">[^<]*</span>"#, ""),
        (r"(?s)<[^>]+>", " "),
        (r"Definition\s*\(([^)]*)\)\s*\.\s*", "$1"),
        (r"::= ", ""),
    ] {
        if let Ok(pattern) = regex::Regex::new(pattern) {
            text = pattern.replace_all(&text, replacement).into_owned();
        }
    }
    for (entity, ch) in [
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&amp;", "&"),
    ] {
        text = text.replace(entity, ch);
    }
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Ok(spaced) = regex::Regex::new(r#" ([.,;:!?\)\]\}"'])"#) {
        return spaced.replace_all(&collapsed, "$1").into_owned();
    }
    collapsed
}

/// The first differing window between two normalized texts, for diagnostics.
fn first_divergence(
    expected: &str,
    actual: &str,
) -> String
{
    let split = expected
        .chars()
        .zip(actual.chars())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| expected.len().min(actual.len()));
    let start = split.saturating_sub(80);
    format!(
        "expected: …{}\nactual:   …{}",
        expected
            .get(start .. split.saturating_add(80))
            .unwrap_or(expected),
        actual
            .get(start .. split.saturating_add(80))
            .unwrap_or(actual),
    )
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// Every component renders through both pipelines with matching
    /// per-section text (the migration gate of proposal §7).
    #[test]
    fn dual_run_section_text_matches() -> TestResult
    {
        let Some(runtime) = runtime()
        else {
            eprintln!("skip: GF environment unprovisioned");
            return Ok(());
        };
        let root = repo_root();
        let spec = root.join("docs/spec");
        let bibliography = bibliography::load(&spec.join("refs.yml"))?;
        let old_dir = root.join("target/gf-docs/dual-old");
        let report = corpus::build(&spec, &spec.join("refs.yml"), &old_dir)?;
        assert!(
            report.check.diagnostics.is_empty(),
            "legacy corpus must validate: {:?}",
            report.check.diagnostics
        );
        let cache_dir = typst_leaf::default_cache_dir(&root.join("target/gf-docs"));
        let context = PostContext::new(&bibliography, &cache_dir);
        let files = gfd_files();
        assert_eq!(files.len(), 10, "the corpus is ten components");
        for gfd_path in files {
            let id = gfd_path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_owned();
            let old_page = std::fs::read_to_string(old_dir.join(format!("{id}.html")))?;
            let gfd = std::fs::read_to_string(&gfd_path)?;
            let new_body = build_body(&runtime, &gfd, &context)?;
            let old_segments = segments(body_of(&old_page));
            let new_segments = segments(&new_body);
            assert_eq!(
                old_segments.keys().collect::<Vec<_>>(),
                new_segments.keys().collect::<Vec<_>>(),
                "{id}: section anchor sets diverge"
            );
            for (anchor, old_text) in &old_segments {
                let Some(new_text) = new_segments.get(anchor)
                else {
                    continue;
                };
                assert_eq!(
                    old_text,
                    new_text,
                    "{id} section '{anchor}' diverges\n{}",
                    first_divergence(old_text, new_text),
                );
            }
        }
        Ok(())
    }

    /// The lexicon generated from the legacy `XML` scan agrees with the
    /// committed modules (the bootstrap invariant; the `.gfd` scan succeeds
    /// it at retirement).
    #[test]
    fn generated_lexicon_is_fresh() -> TestResult
    {
        let root = repo_root();
        let lexicon = gandr_gf_docs::lexicon::generate(
            &root.join("docs/spec"),
            &root.join("docs/spec/refs.yml"),
        )?;
        let grammar = Path::new(env!("CARGO_MANIFEST_DIR")).join("grammar");
        for (name, rendered) in [
            ("GandrDocsLex.gf", lexicon.render_abstract()),
            ("GandrDocsLexHtml.gf", lexicon.render_concrete()),
        ] {
            let committed = std::fs::read_to_string(grammar.join(name))?;
            assert_eq!(committed, rendered, "{name} is stale; run the lexicon lane");
        }
        Ok(())
    }
}
