//! The render pipeline: `readExpr` → `checkExpr` → linearize → post-pass.
//!
//! Post-pass duties (proposal §3.6): `HTML`-escape code block payloads (the
//! tree stores raw text; escaping is presentation) and — in the next `PoC` step
//! — splice compiled math/diagram `SVG` leaves. Math spans currently carry
//! their typst source visibly pending the splice lane.
//!
//! The page shell (gandr-4l9) wraps the linearized body in the design-
//! language landmarks, inlines the compile-time stylesheet, and lifts the
//! page title from the component's rendered `<h1>`.

use std::path::Path;

use crate::error::GfDocsError;
use crate::rt::GfRuntime;

/// The design-language stylesheet (gandr-4l9), embedded at compile time so
/// every rendered page is self-contained.
const STYLESHEET: &str = include_str!("../assets/gandr-docs.css");

/// The vendored ET Book fonts directory (the `ETBookOT` set, MIT license).
const FONTS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts");

/// Validate one `.gfd` document and render its `HTML` body.
///
/// # Errors
/// Whatever the runtime lane rejects ([`GfDocsError::Pgf`] on validation).
#[inline]
pub fn build_body<R>(
    runtime: &R,
    gfd: &str,
) -> Result<String, GfDocsError>
where
    R: GfRuntime + ?Sized,
{
    let html = runtime.check_and_linearize(gfd)?;
    Ok(escape_payloads(&strip_glue_markers(&html)))
}

/// Validate one `.gfd` document and render the full standalone `HTML` page.
///
/// # Contract
/// - requires: nothing beyond [`build_body`]; validation happens at the
///   runtime's `checkExpr` lane inside it.
/// - ensures: the returned page carries the design-language stylesheet inline,
///   a `<title>` lifted from the component's rendered `<h1>` (falling back to
///   `fallback_title` when none is found), and the `<main
///   class="page"><article>` landmarks. The title text is reused from the body
///   verbatim — the grammar emits the component title as a plain `String` leaf,
///   so it contains no markup.
/// # Errors
/// Whatever the runtime lane rejects ([`GfDocsError::Pgf`] on validation).
#[inline]
pub fn build_page<R>(
    runtime: &R,
    gfd: &str,
    fallback_title: &str,
) -> Result<String, GfDocsError>
where
    R: GfRuntime + ?Sized,
{
    let body = build_body(runtime, gfd)?;
    let title = extract_h1(&body).unwrap_or(fallback_title);
    Ok(format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<style>\n{STYLESHEET}</style>\n</head>\n<body>\n<main class=\"page\">\n<article>\n{body}\n</article>\n</main>\n</body>\n</html>\n"
    ))
}

/// Copy the vendored ET Book fonts next to the rendered page(s).
///
/// # Contract
/// - ensures: `<out_dir>/fonts/` exists and holds byte-copies of every file in
///   the crate's `assets/fonts/` directory (idempotent overwrite); the inlined
///   stylesheet references them as `fonts/<name>`.
/// # Errors
/// [`GfDocsError::Io`] on any filesystem failure.
#[inline]
pub fn copy_fonts(out_dir: &Path) -> Result<(), GfDocsError>
{
    let target = out_dir.join("fonts");
    std::fs::create_dir_all(&target)?;
    for entry in std::fs::read_dir(FONTS_DIR)? {
        let entry = entry?;
        std::fs::copy(entry.path(), target.join(entry.file_name()))?;
    }
    Ok(())
}

/// Lift the text of the first `<h1>` out of a rendered body, if present.
///
/// One `h1` per page exists by grammatical construction (`MkComponent` is the
/// sole `Component` linearization), so the first match is the component title.
fn extract_h1(html: &str) -> Option<&str>
{
    let (_, after_open) = html.split_once("<h1>")?;
    after_open.split_once("</h1>").map(|(title, _)| title)
}

/// Remove the zero-width glue markers (U+200B) the linearization inserts at
/// bind points, together with their single surrounding join spaces.
fn strip_glue_markers(html: &str) -> String
{
    html.replace(" \u{200B} ", "")
        .replace(" \u{200B}", "")
        .replace("\u{200B} ", "")
        .replace('\u{200B}', "")
}

/// `HTML`-escape the text content of the raw-payload containers.
///
/// The linearization emits `String`-leaf payloads raw (trees hold unescaped
/// text); every container whose content is exactly one `String` leaf is safe
/// to escape as a scoped unit: `<pre><code>` code payloads, `<dt>`/`<dd>`
/// grammar productions, `.diagram-slot` typst sources, and `.math` spans.
/// (Prose `Txt` leaves interleave with constructor-emitted tags and cannot be
/// escaped in the post-pass; raw `<` in prose is an authoring error — see
/// `docs/workflow/gfd.md`.)
fn escape_payloads(html: &str) -> String
{
    let mut out = html.to_owned();
    for pattern in [
        r"(?s)(<pre><code[^>]*>)(.*?)(</code></pre>)",
        r"(?s)(<dt>)(.*?)(</dt>)",
        r#"(?s)(<dd class="mono">)(.*?)(</dd>)"#,
        r#"(?s)(<div class="diagram-slot"[^>]*>)(.*?)(</div>)"#,
        r#"(?s)(<span class="math[^>]*>)(.*?)(</span>)"#,
    ] {
        out = escape_scoped(&out, pattern);
    }
    out
}

/// Escape `& < >` in the middle capture group of `pattern` throughout `html`.
fn escape_scoped(
    html: &str,
    pattern: &str,
) -> String
{
    let Ok(pattern) = regex::Regex::new(pattern)
    else {
        return html.to_owned();
    };
    pattern
        .replace_all(html, |caps: &regex::Captures<'_>| {
            let full = caps.get(0).map_or(0, |m| m.as_str().len());
            let mut out = String::with_capacity(full.saturating_add(8));
            if let Some(open) = caps.get(1) {
                out.push_str(open.as_str());
            }
            if let Some(payload) = caps.get(2) {
                for ch in payload.as_str().chars() {
                    match ch {
                        | '&' => out.push_str("&amp;"),
                        | '<' => out.push_str("&lt;"),
                        | '>' => out.push_str("&gt;"),
                        | _ => out.push(ch),
                    }
                }
            }
            if let Some(close) = caps.get(3) {
                out.push_str(close.as_str());
            }
            out
        })
        .into_owned()
}
