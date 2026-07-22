//! The render pipeline: `readExpr` → `checkExpr` → linearize → post-pass.
//!
//! Post-pass duties (proposal §3.6): `HTML`-escape code block payloads (the
//! tree stores raw text; escaping is presentation) and — in the next `PoC` step
//! — splice compiled math/diagram `SVG` leaves. Math spans currently carry
//! their typst source visibly pending the splice lane.

use crate::error::GfDocsError;
use crate::rt::GfRuntime;

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
    Ok(escape_code_payloads(&strip_glue_markers(&html)))
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

/// `HTML`-escape the text content of `<pre><code>` blocks.
///
/// The linearization emits code payloads raw (trees hold unescaped text);
/// code payloads never contain `</code></pre>`, so this scoped escape is safe
/// for the `PoC` (a payload-token protocol replaces it in production).
fn escape_code_payloads(html: &str) -> String
{
    let Ok(pattern) = regex::Regex::new(r"(?s)(<pre><code[^>]*>)(.*?)(</code></pre>)")
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
