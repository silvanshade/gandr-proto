//! The per-component references list: full bibliography rows for the cite
//! keys a component declares (the stable-anchor row contract, reused by the
//! render post-pass's enrichment step).

use alloc::string::String;

use crate::bibliography::Bibliography;
use crate::model::CiteKey;

/// Render a per-component references list.
///
/// # Contract
/// - requires: `keys` belongs to the validated bibliography supplied with the
///   render context.
/// - ensures: preserves each `ref-KEY` anchor while materializing author,
///   title, venue, date, and the preferred resolvable locator when present.
/// - provides: one references section in the supplied key order.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — exact rows for DOI, arXiv, URL, and missing
///   optional fields distinguish every rendering branch.
/// - witness: `render::tests::reference_rows_materialize_metadata_and_links`
pub fn render_references(
    keys: &[CiteKey],
    bibliography: &Bibliography,
) -> String
{
    let mut rows = String::new();
    for cite in keys {
        rows.push_str(&render_reference(cite, bibliography));
    }
    format!("<h2>References</h2>\n<ul class=\"refs\">\n{rows}</ul>\n")
}

/// Render one stable-anchor bibliography row.
fn render_reference(
    cite: &CiteKey,
    bibliography: &Bibliography,
) -> String
{
    let key = escape(&cite.key);
    let Some(reference) = bibliography.get(cite)
    else {
        return format!("<li id=\"ref-{key}\"><span class=\"ref-key\">[{key}]</span></li>\n");
    };
    let mut row = format!("<li id=\"ref-{key}\"><span class=\"ref-key\">[{key}]</span> ");
    if let Some(author) = reference.author() {
        row.push_str(&escape(author));
        row.push_str(". ");
    }
    row.push_str("<cite>");
    row.push_str(&escape(reference.title()));
    row.push_str("</cite>.");
    match (reference.venue(), reference.date()) {
        | (Some(venue), Some(date)) => {
            row.push(' ');
            row.push_str(&escape(venue));
            row.push_str(", ");
            row.push_str(&escape(date));
            row.push('.');
        },
        | (Some(venue), None) => {
            row.push(' ');
            row.push_str(&escape(venue));
            row.push('.');
        },
        | (None, Some(date)) => {
            row.push(' ');
            row.push_str(&escape(date));
            row.push('.');
        },
        | (None, None) => {},
    }
    if let Some(locator) = reference.locator() {
        let href = locator.href();
        let label = locator.label();
        row.push_str(" <a class=\"ref-locator\" href=\"");
        row.push_str(&escape(&href));
        row.push_str("\">");
        row.push_str(&escape(&label));
        row.push_str("</a>.");
    }
    row.push_str("</li>\n");
    row
}

/// Escape `HTML` text content.
fn escape(text: &str) -> String
{
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            | '&' => out.push_str("&amp;"),
            | '<' => out.push_str("&lt;"),
            | '>' => out.push_str("&gt;"),
            | '"' => out.push_str("&quot;"),
            | '\'' => out.push_str("&#39;"),
            | other => out.push(other),
        }
    }
    out
}
