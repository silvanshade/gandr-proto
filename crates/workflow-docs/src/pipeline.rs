//! The render pipeline: `readExpr` → `checkExpr` → linearize → post-pass.
//!
//! Post-pass duties (proposal §3.6): `HTML`-escape raw-payload containers (the
//! tree stores raw text; escaping is presentation), splice compiled
//! math/diagram `SVG` leaves (the typst lane, reused from `workflow-docs`),
//! enrich the references list from the bibliography, and renumber headings by
//! container depth (heading level is context-sensitive presentation the
//! context-free linearization cannot express).
//!
//! The page shell (gandr-4l9) wraps the linearized body in the design-
//! language landmarks, inlines the compile-time stylesheet, and lifts the
//! page title from the component's rendered `<h1>`.

use std::path::Path;

use gandr_workflow_grammatical_framework::rt::ExprText;
use gandr_workflow_grammatical_framework::rt::GfRuntime;
use gandr_workflow_grammatical_framework::sexp::Sexp;

use crate::bibliography::Bibliography;
use crate::error::GfDocsError;
use crate::metrics;
use crate::model::CiteKey;
use crate::references::render_references;
use crate::typst_leaf;
use crate::typst_leaf::Leaf;

/// The post-pass context: the bibliography for references enrichment and the
/// cache directory for compiled math/diagram leaves.
pub struct PostContext<'shared>
{
    /// The corpus bibliography (`refs.yml`), used to enrich the references
    /// list from bare keyed rows to full bibliography rows.
    pub bibliography: &'shared Bibliography,
    /// The content-hash cache directory for compiled typst leaves.
    pub cache_dir: &'shared Path,
}

impl<'shared> PostContext<'shared>
{
    /// Bundle the bibliography and cache directory for the post-pass.
    #[inline]
    #[must_use]
    pub const fn new(
        bibliography: &'shared Bibliography,
        cache_dir: &'shared Path,
    ) -> Self
    {
        Self {
            bibliography,
            cache_dir,
        }
    }
}

/// Borrowed rendered markup consumed by one post-processing pass.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct RenderedMarkup<'markup>(&'markup str);

impl<'markup> From<&'markup str> for RenderedMarkup<'markup>
{
    #[inline]
    fn from(markup: &'markup str) -> Self
    {
        Self(markup)
    }
}

/// Borrowed page title consumed by the document shell.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTitle<'title>(&'title str);

impl<'title> From<&'title str> for PageTitle<'title>
{
    #[inline]
    fn from(title: &'title str) -> Self
    {
        Self(title)
    }
}

/// Borrowed component id consumed by an index row.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct IndexComponentId<'id>(&'id str);

impl<'id> From<&'id str> for IndexComponentId<'id>
{
    #[inline]
    fn from(id: &'id str) -> Self
    {
        Self(id)
    }
}

/// Borrowed component title consumed by an index row.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct IndexComponentTitle<'title>(&'title str);

impl<'title> From<&'title str> for IndexComponentTitle<'title>
{
    #[inline]
    fn from(title: &'title str) -> Self
    {
        Self(title)
    }
}

/// Borrowed component status consumed by an index row.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct IndexComponentStatus<'status>(&'status str);

impl<'status> From<&'status str> for IndexComponentStatus<'status>
{
    #[inline]
    fn from(status: &'status str) -> Self
    {
        Self(status)
    }
}

/// One table-of-contents nesting depth.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct TocDepth(u32);

impl From<u32> for TocDepth
{
    #[inline]
    fn from(depth: u32) -> Self
    {
        Self(depth)
    }
}

impl From<TocDepth> for u32
{
    #[inline]
    fn from(depth: TocDepth) -> Self
    {
        depth.0
    }
}

impl TocDepth
{
    /// Advance to the next heading depth.
    #[inline]
    fn next(self) -> Self
    {
        Self(self.0.saturating_add(1))
    }
}

/// One regular expression selecting an escapable payload.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct PayloadPattern<'pattern>(&'pattern str);

impl<'pattern> From<&'pattern str> for PayloadPattern<'pattern>
{
    #[inline]
    fn from(pattern: &'pattern str) -> Self
    {
        Self(pattern)
    }
}

/// Entity-escaped payload text awaiting compilation.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct EscapedPayload<'text>(&'text str);

impl<'text> From<&'text str> for EscapedPayload<'text>
{
    #[inline]
    fn from(text: &'text str) -> Self
    {
        Self(text)
    }
}

/// Text escaped into one generated `HTML` attribute.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct AttributeText<'text>(&'text str);

impl<'text> From<&'text str> for AttributeText<'text>
{
    #[inline]
    fn from(text: &'text str) -> Self
    {
        Self(text)
    }
}

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
    gfd: &ExprText,
    context: &PostContext<'_>,
) -> Result<String, GfDocsError>
where
    R: GfRuntime + ?Sized,
{
    let html = runtime.check_and_linearize(gfd)?;
    let html = strip_glue_markers(html.as_str().into());
    let html = escape_payloads(html.as_str().into());
    let html = splice_leaves(html.as_str().into(), context.cache_dir)?;
    let html = enrich_references(html.as_str().into(), context.bibliography);
    Ok(renumber_headings(html.as_str().into()))
}

/// Validate one `.gfd` document and render the full standalone `HTML` page.
///
/// # Contract
/// - requires: nothing beyond [`build_body`]; validation happens at the
///   runtime's `checkExpr` lane inside it.
/// - ensures: the returned page carries the design-language stylesheet inline,
///   a `<title>` lifted from the component's rendered `<h1>` (falling back to
///   `fallback_title` when none is found), the `<main class="page"><article>`
///   landmarks, and the section table of contents (from `toc`, empty for the
///   corpus index). The title text is reused from the body verbatim — the
///   grammar emits the component title as a plain `String` leaf, so it contains
///   no markup.
/// # Errors
/// Whatever the runtime lane rejects ([`GfDocsError::Pgf`] on validation).
#[inline]
pub fn build_page<R>(
    runtime: &R,
    gfd: &ExprText,
    fallback_title: PageTitle<'_>,
    context: &PostContext<'_>,
    toc: &[TocEntry],
) -> Result<String, GfDocsError>
where
    R: GfRuntime + ?Sized,
{
    let body = build_body(runtime, gfd, context)?;
    let title = extract_h1(body.as_str().into()).unwrap_or(fallback_title);
    Ok(shell_with_toc(title, body.as_str().into(), toc))
}

/// One component-listing row for the corpus index page.
pub struct IndexEntry
{
    /// The component id (its page filename stem).
    pub id: String,
    /// The component title (already `HTML`-escaped at the translation
    /// boundary, as every text-destined string is).
    pub title: String,
    /// The status display string.
    pub status: String,
}

impl IndexEntry
{
    /// Build one index row.
    #[inline]
    #[must_use]
    pub fn new(
        id: IndexComponentId<'_>,
        title: IndexComponentTitle<'_>,
        status: IndexComponentStatus<'_>,
    ) -> Self
    {
        Self {
            id: id.0.to_owned(),
            title: title.0.to_owned(),
            status: status.0.to_owned(),
        }
    }
}

/// Render the corpus index page (the component listing) in the design shell.
///
/// The listing is corpus machinery, not a component: the spec-index
/// component's own page carries the About/Conventions prose, this page lists
/// every component with its status (the legacy `index.html` contract).
#[inline]
#[must_use]
pub fn render_index(entries: &[IndexEntry]) -> String
{
    let mut items = String::new();
    for entry in entries {
        items.push_str("<li><a href=\"");
        items.push_str(&entry.id);
        items.push_str(".html\">");
        items.push_str(&entry.title);
        items.push_str("</a> <span class=\"status\">");
        items.push_str(&entry.status);
        items.push_str("</span></li>\n");
    }
    let body = format!("<h1>gandr specification corpus</h1>\n<ul class=\"index\">\n{items}</ul>\n");
    shell("gandr specification corpus".into(), body.as_str().into())
}

/// One table-of-contents row: a section's `HTML` id, its title, and its
/// nesting depth (1 for top-level sections).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TocEntry
{
    /// The section's anchor id (the link target).
    pub id: String,
    /// The section title (the link text).
    pub title: String,
    /// The nesting depth (1 for top-level sections).
    pub depth: u32,
}

/// Extract the table of contents from a component's tree: every `MkSection`
/// in reading order, anchor constant resolved to its `HTML` id through the
/// views.
///
/// # Errors
/// [`GfDocsError::Parse`] when the tree departs from the expected shapes or
/// an anchor constant has no lexicon id.
#[inline]
pub fn toc_entries(
    tree: &Sexp,
    views: &metrics::LexiconViews,
) -> Result<Vec<TocEntry>, GfDocsError>
{
    let mut entries = Vec::new();
    collect_toc(tree, TocDepth::from(1), views, &mut entries)?;
    Ok(entries)
}

/// Walk for `MkSection` nodes, recording (id, title, depth).
///
/// The traversal is an explicit work stack (the house pattern): each pop
/// visits one node and pushes its strict children in reverse, so the walk
/// stays document-ordered depth-first without call recursion.
///
/// # Termination
/// - reason: an explicit work stack visits each tree node once.
/// - measure: pending stack entries; every pop removes one entry and pushes
///   only strict children of the popped node.
/// - boundedness: the source tree is finite.
/// - input recursion: none.
fn collect_toc(
    tree: &Sexp,
    depth: TocDepth,
    views: &metrics::LexiconViews,
    entries: &mut Vec<TocEntry>,
) -> Result<(), GfDocsError>
{
    let mut stack = vec![(tree, depth)];
    while let Some((node, depth)) = stack.pop() {
        let Sexp::App { ref head, ref args } = *node
        else {
            continue;
        };
        let next_depth = if head == "MkSection" {
            let [ref anchor, ref title, ref _status, ref _blocks] = *args.as_slice()
            else {
                return Err(GfDocsError::Parse("MkSection arity is not four".into()));
            };
            let Sexp::Atom(ref anchor_name) = *anchor
            else {
                return Err(GfDocsError::Parse("section anchor is not an atom".into()));
            };
            let Sexp::Atom(ref title_atom) = *title
            else {
                return Err(GfDocsError::Parse(
                    "section title is not a string literal".into(),
                ));
            };
            let title = gandr_workflow_grammatical_framework::sexp::unquote((title_atom).into())
                .ok_or_else(|| {
                    GfDocsError::Parse("section title is not a string literal".into())
                })?;
            let id = views
                .anchor_ids()
                .get(anchor_name)
                .ok_or_else(|| GfDocsError::Parse(format!("{anchor_name}: no lexicon id")))?
                .clone();
            entries.push(TocEntry {
                id,
                title,
                depth: u32::from(depth),
            });
            depth.next()
        }
        else {
            depth
        };
        for arg in args.iter().rev() {
            stack.push((arg, next_depth));
        }
    }
    Ok(())
}

/// Wrap a rendered body in the full standalone `HTML` page shell: the
/// design-language stylesheet inline, the viewport meta, and the
/// `<main class="page"><article>` landmarks.
fn shell(
    title: PageTitle<'_>,
    body: RenderedMarkup<'_>,
) -> String
{
    shell_with_toc(title, body, &[])
}

/// Wrap a rendered body in the full standalone `HTML` page shell with the
/// section table of contents in the left rail (empty when there are no
/// entries — the corpus index stays bare).
fn shell_with_toc(
    title: PageTitle<'_>,
    body: RenderedMarkup<'_>,
    toc: &[TocEntry],
) -> String
{
    let title = title.0;
    let body = body.0;
    let mut nav = String::new();
    if !toc.is_empty() {
        use core::fmt::Write as _;
        nav.push_str("<nav class=\"toc\"><div class=\"toc-heading\">Contents</div><ol>");
        for entry in toc {
            let _res = write!(
                nav,
                "<li class=\"toc-depth-{}\"><a href=\"#{}\">{}</a></li>",
                entry.depth, entry.id, entry.title
            );
        }
        nav.push_str("</ol></nav>");
    }
    // The scroll-spy runs after `main` is parsed — emitted before it, the
    // section query matches nothing and the spy silently never fires.
    let spy = if nav.is_empty() { "" } else { SCROLL_SPY };
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<style>\n{STYLESHEET}</style>\n</head>\n<body>\n{nav}<main class=\"page\">\n<article>\n{body}\n</article>\n</main>\n{spy}</body>\n</html>\n"
    )
}

/// The scroll-spy: mark the `ToC` row of the section nearest the viewport
/// top. Progressive enhancement — the `ToC` links work without it. This is
/// the one sanctioned `JavaScript` exception to the design language's no-`JS`
/// rule (owner decision, gandr-aaq, 2026-07-23; recorded in the design-
/// language proposal).
const SCROLL_SPY: &str = "<script>(() => {\n\
  const rows = [...document.querySelectorAll('nav.toc li')];\n\
  const byId = new Map(rows.map((li) => [li.querySelector('a').hash.slice(1), li]));\n\
  const visible = new Set();\n\
  const sections = [...document.querySelectorAll('section[id]')];\n\
  let current = sections[0];\n\
  const mark = () => {\n\
    const line = 0.25 * window.innerHeight;\n\
    current = sections.find((section) => visible.has(section.id))\n\
      ?? sections.findLast((section) => section.getBoundingClientRect().top <= line)\n\
      ?? sections[0];\n\
    rows.forEach((li) => li.classList.toggle('current', li === byId.get(current && current.id)));\n\
  };\n\
  mark();\n\
  const spy = new IntersectionObserver((entries) => {\n\
    entries.forEach((entry) => visible[entry.isIntersecting ? 'add' : 'delete'](entry.target.id));\n\
    mark();\n\
  }, { rootMargin: '-10% 0px -75% 0px' });\n\
  sections.forEach((section) => spy.observe(section));\n\
  window.addEventListener('scroll', mark, { passive: true });\n\
})();</script>\n";

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
fn extract_h1(html: RenderedMarkup<'_>) -> Option<PageTitle<'_>>
{
    let (_, after_open) = html.0.split_once("<h1>")?;
    after_open
        .split_once("</h1>")
        .map(|(title, _)| PageTitle::from(title))
}

/// Remove the zero-width glue markers (U+200B) the linearization inserts at
/// bind points, together with their single surrounding join spaces.
fn strip_glue_markers(html: RenderedMarkup<'_>) -> String
{
    html.0
        .replace(" \u{200B} ", "")
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
fn escape_payloads(html: RenderedMarkup<'_>) -> String
{
    let mut out = html.0.to_owned();
    for pattern in [
        r"(?s)(<pre><code[^>]*>)(.*?)(</code></pre>)",
        r"(?s)(<dt>)(.*?)(</dt>)",
        r#"(?s)(<dd class="mono">)(.*?)(</dd>)"#,
        r#"(?s)(<div class="diagram-slot"[^>]*>)(.*?)(</div>)"#,
        r#"(?s)(<span class="math[^>]*>)(.*?)(</span>)"#,
    ] {
        out = escape_scoped(out.as_str().into(), pattern.into());
    }
    out
}

/// Escape `& < >` in the middle capture group of `pattern` throughout `html`.
fn escape_scoped(
    html: RenderedMarkup<'_>,
    pattern: PayloadPattern<'_>,
) -> String
{
    let html = html.0;
    let Ok(pattern) = regex::Regex::new(pattern.0)
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

/// Reverse the payload escaping (`&lt;` `&gt;` then `&amp;`, in that order)
/// for a raw-payload container's text content.
fn unescape_entities(text: EscapedPayload<'_>) -> String
{
    text.0
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Escape text for an attribute value (`& < > "`).
fn attr_escape(text: AttributeText<'_>) -> String
{
    let text = text.0;
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            | '&' => out.push_str("&amp;"),
            | '<' => out.push_str("&lt;"),
            | '>' => out.push_str("&gt;"),
            | '"' => out.push_str("&quot;"),
            | other => out.push(other),
        }
    }
    out
}

/// Splice compiled `SVG` leaves into the math spans and diagram slots.
///
/// Runs after [`escape_payloads`]: the spans hold entity-escaped typst source,
/// which this pass unescapes for the compiler and replaces with the compiled
/// `SVG` (or the placeholder on a missing/failed compile, matching the legacy
/// renderer's fallback contract).
fn splice_leaves(
    html: RenderedMarkup<'_>,
    cache_dir: &Path,
) -> Result<String, GfDocsError>
{
    let html = splice_math(html, cache_dir)?;
    splice_diagrams(html.as_str().into(), cache_dir)
}

/// Splice inline and display math spans.
fn splice_math(
    html: RenderedMarkup<'_>,
    cache_dir: &Path,
) -> Result<String, GfDocsError>
{
    let html = html.0;
    let Ok(pattern) = regex::Regex::new(r#"(?s)<span class="(math(?: math-block)?)">(.*?)</span>"#)
    else {
        return Ok(html.to_owned());
    };
    let mut out = String::with_capacity(html.len());
    let mut last = 0usize;
    for caps in pattern.captures_iter(html) {
        let (Some(whole), Some(kind), Some(source)) = (caps.get(0), caps.get(1), caps.get(2))
        else {
            continue;
        };
        if let Some(text) = html.get(last .. whole.start()) {
            out.push_str(text);
        }
        last = whole.end();
        let source = unescape_entities(source.as_str().into());
        let leaf = typst_leaf::compile_math(source.as_str().into(), cache_dir)
            .map_err(|e| GfDocsError::Model(e.to_string()))?;
        out.push_str("<span class=\"");
        out.push_str(kind.as_str());
        out.push_str("\" role=\"math\" aria-label=\"");
        out.push_str(&attr_escape(source.as_str().into()));
        out.push_str("\">");
        out.push_str(&leaf_markup(leaf));
        out.push_str("</span>");
    }
    if let Some(text) = html.get(last ..) {
        out.push_str(text);
    }
    Ok(out)
}

/// Splice diagram slots (fletcher commutative diagrams).
fn splice_diagrams(
    html: RenderedMarkup<'_>,
    cache_dir: &Path,
) -> Result<String, GfDocsError>
{
    let html = html.0;
    let Ok(pattern) = regex::Regex::new(r#"(?s)(<div class="diagram-slot"[^>]*>)(.*?)(</div>)"#)
    else {
        return Ok(html.to_owned());
    };
    let mut out = String::with_capacity(html.len());
    let mut last = 0usize;
    for caps in pattern.captures_iter(html) {
        let (Some(whole), Some(open), Some(source), Some(close)) =
            (caps.get(0), caps.get(1), caps.get(2), caps.get(3))
        else {
            continue;
        };
        if let Some(text) = html.get(last .. whole.start()) {
            out.push_str(text);
        }
        last = whole.end();
        let source = unescape_entities(source.as_str().into());
        let leaf = typst_leaf::compile_diagram(source.as_str().into(), cache_dir)
            .map_err(|e| GfDocsError::Model(e.to_string()))?;
        out.push_str(open.as_str());
        out.push_str(&leaf_markup(leaf));
        out.push_str(close.as_str());
    }
    if let Some(text) = html.get(last ..) {
        out.push_str(text);
    }
    Ok(out)
}

/// Render one compiled leaf, mirroring the legacy placeholder contract.
fn leaf_markup(leaf: Leaf) -> String
{
    match leaf {
        | Leaf::Svg(svg) => svg,
        | _ => String::from("<span class=\"fallback\">[math placeholder]</span>"),
    }
}

/// Enrich the references list: the linearization emits bare keyed rows; this
/// pass substitutes the legacy renderer's full bibliography rows (the reused
/// [`render_references`]) for the same keys in the same order.
fn enrich_references(
    html: RenderedMarkup<'_>,
    bibliography: &Bibliography,
) -> String
{
    let html = html.0;
    let Some((before, rest)) = html.split_once("<h2>References</h2><ul class=\"refs\">")
    else {
        return html.to_owned();
    };
    let Some((rows, after)) = rest.split_once("</ul>")
    else {
        return html.to_owned();
    };
    let keys = rows
        .split("<li id=\"ref-")
        .skip(1)
        .filter_map(|row| row.split_once('\"').map(|(key, _)| CiteKey::from(key)))
        .collect::<Vec<_>>();
    let rendered = render_references(&keys, bibliography);
    format!("{before}{rendered}{after}")
}

/// Renumber headings by container depth.
///
/// The context-free linearization emits fixed levels (`<h2>` sections, `<h3>`
/// payload headings); the legacy renderer's contract is level 2 plus the
/// number of enclosing section/example containers. This scanner is the
/// pipeline's depth pass over the rendered markup: section and example
/// containers push depth, their own headings take the pre-increment level,
/// and every other heading takes the level of its enclosing depth. Closing
/// heading tags mirror the level of their opener.
///
/// # Contract
/// - requires: `html` is the post-splice body (escaped payloads carry no raw
///   `<`; `SVG` leaves contain none of the tracked tag names).
/// - ensures: every `<h2>`/`<h3>` pair is rewritten to `<hN>`/`</hN>` with N =
///   2 + the number of enclosing section/example containers; all other markup
///   passes through byte-identically.
/// - panics: none.
fn renumber_headings(html: RenderedMarkup<'_>) -> String
{
    let html = html.0;
    let Ok(tag) = regex::Regex::new(r"<(/?)(section|div|h2|h3)([^>]*)>")
    else {
        return html.to_owned();
    };
    let mut out = String::with_capacity(html.len());
    let mut last = 0usize;
    let mut depth = 0usize;
    let mut pending: Option<usize> = None;
    let mut open_heading: Option<usize> = None;
    let mut divs: Vec<bool> = Vec::new();
    for caps in tag.captures_iter(html) {
        let (Some(whole), Some(slash), Some(name)) = (caps.get(0), caps.get(1), caps.get(2))
        else {
            continue;
        };
        let attrs = caps.get(3).map_or("", |m| m.as_str());
        if let Some(text) = html.get(last .. whole.start()) {
            out.push_str(text);
        }
        last = whole.end();
        let closing = !slash.as_str().is_empty();
        match (closing, name.as_str()) {
            | (false, "section") => {
                pending = Some(depth.saturating_add(2));
                depth = depth.saturating_add(1);
                out.push_str(whole.as_str());
            },
            | (true, "section") => {
                depth = depth.saturating_sub(1);
                out.push_str(whole.as_str());
            },
            | (false, "div") => {
                let example = attrs.contains("class=\"example\"");
                divs.push(example);
                if example {
                    pending = Some(depth.saturating_add(2));
                    depth = depth.saturating_add(1);
                }
                out.push_str(whole.as_str());
            },
            | (true, "div") => {
                if divs.pop() == Some(true) {
                    depth = depth.saturating_sub(1);
                }
                out.push_str(whole.as_str());
            },
            | (false, "h2" | "h3") => {
                let level = pending.take().unwrap_or_else(|| depth.saturating_add(2));
                open_heading = Some(level);
                out.push_str("<h");
                out.push_str(&level.to_string());
                out.push_str(attrs);
                out.push('>');
            },
            | (true, "h2" | "h3") => {
                let level = open_heading
                    .take()
                    .unwrap_or_else(|| depth.saturating_add(2));
                out.push_str("</h");
                out.push_str(&level.to_string());
                out.push_str(attrs);
                out.push('>');
            },
            | _ => out.push_str(whole.as_str()),
        }
    }
    if let Some(text) = html.get(last ..) {
        out.push_str(text);
    }
    out
}
