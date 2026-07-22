//! No-JavaScript static `HTML` rendering of a validated corpus.
//!
//! Every page reads with no client-side script. Math and diagram leaves are
//! compiled to inline `SVG` by [`crate::typst_leaf`]; when the typst tool is
//! unavailable the leaf renders as a source placeholder and a note is recorded.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use std::path::Path;

use crate::DocError;
use crate::bibliography::Bibliography;
use crate::model::Block;
use crate::model::CiteKey;
use crate::model::Document;
use crate::model::Inline;
use crate::model::Math;
use crate::model::MathDisplay;
use crate::typst_leaf;
use crate::typst_leaf::Leaf;

/// Inline stylesheet shared by every generated page (no external assets).
const STYLESHEET: &str = r#":root{--fg:#1a1a1a;--bg:#fdfdfc;--muted:#666;--accent:#3b5;--line:#ddd;--code:#f4f4f2}
@media(prefers-color-scheme:dark){:root{--fg:#e8e8e6;--bg:#16171a;--muted:#9a9a9a;--accent:#6d9;--line:#333;--code:#22242a}}
body{color:var(--fg);background:var(--bg);font-family:system-ui,sans-serif;line-height:1.55;max-width:48rem;margin:2rem auto;padding:0 1rem}
h1,h2,h3,h4{line-height:1.2}code,pre{background:var(--code);border-radius:4px}
code{padding:.1em .3em}pre{padding:.7em 1em;overflow-x:auto}
.status{font-size:.75rem;text-transform:uppercase;letter-spacing:.05em;border:1px solid var(--line);border-radius:1em;padding:.1em .6em;color:var(--muted)}
.math{display:inline-block;vertical-align:middle}.math svg,figure svg{max-width:100%;height:auto}
figure{margin:1.2em 0;text-align:center}figcaption{color:var(--muted);font-size:.9rem}
.rule{border:none}.term{border-bottom:1px dotted var(--accent);text-decoration:none;color:inherit}
.cite{color:var(--accent);text-decoration:none}.mono{font-family:ui-monospace,monospace}
.fallback{border:1px dashed var(--line);padding:.2em .4em;color:var(--muted)}
.xref{color:var(--accent)}table{border-collapse:collapse;margin:0 auto}
th,td{border:1px solid var(--line);padding:.3em .6em;text-align:left;vertical-align:top}
figure.table{overflow-x:auto}
dl.grammar dt{font-family:ui-monospace,monospace;font-weight:bold}
.expect{color:var(--muted);font-size:.85rem;margin:.2em 0}
.refs{list-style:none;padding-left:0}.refs li{margin:.8em 0}
.ref-key{font-family:ui-monospace,monospace;white-space:nowrap}.ref-locator{color:var(--accent);white-space:nowrap}"#;

/// Rendering context: leaf cache, corpus term map, anchor map, and
/// bibliography.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct RenderContext<'ctx>
{
    /// Directory for compiled leaf assets.
    pub cache_dir: &'ctx Path,
    /// Corpus-wide map from term key to displayed definition text.
    pub terms: &'ctx BTreeMap<String, String>,
    /// Corpus-wide map from declared anchor id to its owning page and title.
    pub anchors: &'ctx BTreeMap<String, AnchorTarget>,
    /// Typed bibliography used to materialize per-component reference entries.
    pub references: &'ctx Bibliography,
}

impl<'ctx> RenderContext<'ctx>
{
    /// Build a rendering context.
    #[inline]
    #[must_use]
    pub const fn new(
        cache_dir: &'ctx Path,
        terms: &'ctx BTreeMap<String, String>,
        anchors: &'ctx BTreeMap<String, AnchorTarget>,
        references: &'ctx Bibliography,
    ) -> Self
    {
        Self {
            cache_dir,
            terms,
            anchors,
            references,
        }
    }
}

/// The resolution of one corpus anchor id: its owning component page and its
/// human-readable title (used as the link text of an inline `ref`).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AnchorTarget
{
    /// Identifier of the component whose page carries the anchor.
    pub component: String,
    /// Human-readable link text (component title, section title, rule name,
    /// definition term, or diagram caption).
    pub title: String,
}

/// Collect the corpus term map (key to displayed text) for cross-references.
#[inline]
#[must_use]
pub fn term_map(documents: &[Document]) -> BTreeMap<String, String>
{
    let mut terms = BTreeMap::new();
    for document in documents {
        for block in &document.blocks {
            collect_terms(block, &mut terms);
        }
    }
    terms
}

/// Collect term definitions from a block in document order.
///
/// # Contract
///
/// - requires: `block` is a finite, structurally well-formed block tree.
/// - ensures: inserts every nested term definition only when its key is absent,
///   so the earliest definition remains authoritative.
/// - provides: the updated corpus term map in `terms`.
/// - panics: none.
/// - intension: [`Block::walk`] supplies depth-first document order without
///   input-scaled native-stack recursion.
///
/// # Adequacy
///
/// - hypothesis: L3 pointwise — duplicate nested definitions distinguish
///   traversal order through the exact retained display text.
/// - witness: `tests::term_map_prefers_first_nested_definition`.
fn collect_terms(
    block: &Block,
    terms: &mut BTreeMap<String, String>,
)
{
    for block in block.walk() {
        match *block {
            | Block::Prose(ref inlines) => collect_inline_terms(inlines, terms),
            | Block::Definition(ref definition) => {
                collect_inline_terms(&definition.body, terms);
            },
            | Block::List(ref list) => {
                for item in &list.items {
                    collect_inline_terms(&item.body, terms);
                }
            },
            | Block::Table(ref table) => {
                for cell in table
                    .header
                    .iter()
                    .chain(table.rows.iter().flat_map(|row| row.cells.iter()))
                {
                    collect_inline_terms(&cell.content, terms);
                }
            },
            | Block::Section(_)
            | Block::Judgements(_)
            | Block::Grammar(_)
            | Block::Rule(_)
            | Block::Diagram(_)
            | Block::Code(_)
            | Block::Example(_)
            | Block::References(_) => {},
        }
    }
}

/// Collect the corpus anchor map (declared id to owning page and title).
///
/// # Contract
///
/// - requires: `documents` hold finite, structurally well-formed block trees.
/// - ensures: maps every component id and every declared section, rule,
///   definition, and diagram id to its owning component and human-readable
///   title, keeping the first declaration when ids collide (the validator
///   rejects collisions separately).
/// - provides: the link-resolution input of inline `ref` rendering.
/// - panics: none.
///
/// # Adequacy
///
/// - hypothesis: L3 pointwise — one anchor of each declaring block kind
///   distinguishes the per-kind component and title projections.
/// - witness: `tests::anchor_map_collects_each_declaring_kind`.
#[inline]
#[must_use]
pub fn anchor_map(documents: &[Document]) -> BTreeMap<String, AnchorTarget>
{
    let mut anchors = BTreeMap::new();
    for document in documents {
        anchors
            .entry(document.id.clone())
            .or_insert_with(|| AnchorTarget {
                component: document.id.clone(),
                title: document.title.clone(),
            });
        for block in &document.blocks {
            for block in block.walk() {
                let id_title = match *block {
                    | Block::Section(ref section) => {
                        section.id.as_ref().map(|id| (id, section.title.clone()))
                    },
                    | Block::Rule(ref rule) => rule.id.as_ref().map(|id| (id, rule.name.clone())),
                    | Block::Definition(ref definition) => definition
                        .id
                        .as_ref()
                        .map(|id| (id, definition.term.clone())),
                    | Block::Diagram(ref diagram) => Some((&diagram.id, diagram.caption.clone())),
                    | Block::Prose(_)
                    | Block::Judgements(_)
                    | Block::Grammar(_)
                    | Block::Code(_)
                    | Block::Example(_)
                    | Block::List(_)
                    | Block::Table(_)
                    | Block::References(_) => None,
                };
                if let Some((id, title)) = id_title {
                    anchors.entry(id.clone()).or_insert_with(|| AnchorTarget {
                        component: document.id.clone(),
                        title,
                    });
                }
            }
        }
    }
    anchors
}

/// Collect term definitions from an inline sequence.
fn collect_inline_terms(
    inlines: &[Inline],
    terms: &mut BTreeMap<String, String>,
)
{
    for inline in inlines {
        if let Inline::TermDef(ref term_def) = *inline {
            terms
                .entry(term_def.key.clone())
                .or_insert_with(|| term_def.text.clone());
        }
    }
}

/// Render the corpus index page listing every component and its status.
#[inline]
#[must_use]
pub fn render_index(documents: &[Document]) -> String
{
    let items: String = documents
        .iter()
        .map(|document| {
            format!(
                "<li><a href=\"{id}.html\">{title}</a> <span class=\"status\">{status}</span></li>\n",
                id = escape(&document.id),
                title = escape(&document.title),
                status = document.status.as_str(),
            )
        })
        .collect::<Vec<String>>()
        .concat();
    let body = format!("<h1>gandr specification corpus</h1>\n<ul class=\"index\">\n{items}</ul>\n");
    page(&escape("gandr specification corpus"), &body)
}

/// Render one component to a full `HTML` page.
///
/// # Errors
/// Returns [`DocError::Io`] when a leaf asset cannot be written.
#[inline]
pub fn render_document(
    document: &Document,
    ctx: &RenderContext<'_>,
    notes: &mut Vec<String>,
) -> Result<String, DocError>
{
    let provenance = if document.grounds.is_empty() && document.derives.is_empty() {
        String::new()
    }
    else {
        render_provenance(document)
    };
    let blocks = render_blocks(&document.blocks, 2usize, ctx, notes)?;
    let body = format!(
        "<p><a href=\"index.html\">&larr; corpus</a></p>\n<h1>{title}</h1>\n\
         <p class=\"status\">{status}</p>\n{provenance}{blocks}",
        title = escape(&document.title),
        status = document.status.as_str(),
    );
    Ok(page(&escape(&document.title), &body))
}

/// Render the provenance edges of a component.
fn render_provenance(document: &Document) -> String
{
    let grounds = if document.grounds.is_empty() {
        String::new()
    }
    else {
        format!("grounds: {} ", escape(&document.grounds.join(", ")))
    };
    let derives = if document.derives.is_empty() {
        String::new()
    }
    else {
        format!("derives: {}", escape(&document.derives.join(", ")))
    };
    format!("<p class=\"mono\">{grounds}{derives}</p>\n")
}

/// One pending action in the iterative block renderer.
enum RenderTask<'block>
{
    /// Render one block at the supplied heading level.
    Block
    {
        /// Block whose opening markup or leaf content is next.
        block: &'block Block,
        /// Heading level inherited from the containing document block.
        level: usize,
    },
    /// Emit the closing markup for a container after all of its children.
    Close(&'static str),
}

/// Render a block sequence at the given initial heading level.
///
/// # Errors
///
/// Returns [`DocError`] when a math or diagram leaf cannot be rendered.
///
/// # Contract
///
/// - requires: `blocks` is a finite, structurally well-formed block forest.
/// - ensures: emits every block exactly once in depth-first document order and
///   closes each section/example after all of its children.
/// - provides: concatenated `HTML` for the supplied block forest.
/// - fails: propagates [`DocError`] from fallible math or diagram leaf
///   rendering.
/// - panics: none.
/// - intension: an explicit enter/close worklist bounds native stack use
///   independently of document nesting depth.
///
/// # Adequacy
///
/// - hypothesis: L3 pointwise — the exact markup distinguishes sibling order,
///   container nesting, heading depth, and escaped leaf content.
/// - witness: `nested_blocks_render_in_document_order`.
fn render_blocks(
    blocks: &[Block],
    level: usize,
    ctx: &RenderContext<'_>,
    notes: &mut Vec<String>,
) -> Result<String, DocError>
{
    let mut pending = blocks
        .iter()
        .rev()
        .map(|block| RenderTask::Block { block, level })
        .collect::<Vec<_>>();
    let mut output = String::new();

    while let Some(task) = pending.pop() {
        match task {
            | RenderTask::Close(markup) => output.push_str(markup),
            | RenderTask::Block { block, level } => match *block {
                | Block::Section(ref section) => {
                    let heading = level.min(6usize).to_string();
                    match section.id {
                        | Some(ref id) => {
                            output.push_str("<section id=\"");
                            output.push_str(&escape(id));
                            output.push_str("\">\n<h");
                        },
                        | None => output.push_str("<section>\n<h"),
                    }
                    output.push_str(&heading);
                    output.push('>');
                    output.push_str(&escape(&section.title));
                    output.push_str("</h");
                    output.push_str(&heading);
                    output.push_str(">\n");
                    pending.push(RenderTask::Close("</section>\n"));
                    let child_level = level.saturating_add(1usize);
                    pending.extend(section.blocks.iter().rev().map(|block| RenderTask::Block {
                        block,
                        level: child_level,
                    }));
                },
                | Block::Example(ref example) => {
                    let heading = level.min(6usize).to_string();
                    output.push_str("<div class=\"example\">\n<h");
                    output.push_str(&heading);
                    output.push_str(">Example: ");
                    output.push_str(&escape(&example.title));
                    output.push_str("</h");
                    output.push_str(&heading);
                    output.push_str(">\n");
                    pending.push(RenderTask::Close("</div>\n"));
                    let child_level = level.saturating_add(1usize);
                    pending.extend(example.blocks.iter().rev().map(|block| RenderTask::Block {
                        block,
                        level: child_level,
                    }));
                },
                | Block::Prose(ref inlines) => {
                    let body = render_inlines(inlines, ctx, notes)?;
                    output.push_str("<p>");
                    output.push_str(&body);
                    output.push_str("</p>\n");
                },
                | Block::Definition(ref definition) => {
                    let body = render_inlines(&definition.body, ctx, notes)?;
                    match definition.id {
                        | Some(ref id) => {
                            output.push_str("<dl id=\"");
                            output.push_str(&escape(id));
                            output.push_str("\">\n<dt>");
                        },
                        | None => output.push_str("<dl>\n<dt>"),
                    }
                    output.push_str(&escape(&definition.term));
                    output.push_str("</dt>\n<dd>");
                    output.push_str(&body);
                    output.push_str("</dd>\n</dl>\n");
                },
                | Block::Judgements(ref judgements) => {
                    let heading = level.min(6usize).to_string();
                    let forms = judgements
                        .forms
                        .iter()
                        .map(|form| {
                            render_math(form, ctx, notes).map(|svg| format!("<div>{svg}</div>\n"))
                        })
                        .collect::<Result<String, DocError>>()?;
                    output.push_str("<div class=\"judgements\">\n<h");
                    output.push_str(&heading);
                    output.push('>');
                    output.push_str(&escape(&judgements.title));
                    output.push_str("</h");
                    output.push_str(&heading);
                    output.push_str(">\n");
                    output.push_str(&forms);
                    output.push_str("</div>\n");
                },
                | Block::Grammar(ref productions) => {
                    let rows: String = productions
                        .iter()
                        .map(|production| {
                            format!(
                                "<dt>{symbol}</dt>\n<dd class=\"mono\">::= {definition}</dd>\n",
                                symbol = escape(&production.symbol),
                                definition = escape(&production.definition),
                            )
                        })
                        .collect::<Vec<String>>()
                        .concat();
                    output.push_str("<dl class=\"grammar\">\n");
                    output.push_str(&rows);
                    output.push_str("</dl>\n");
                },
                | Block::Rule(ref rule) => {
                    let premises = rule
                        .premises
                        .iter()
                        .map(|math| math.source.clone())
                        .collect::<Vec<String>>()
                        .join(" quad ");
                    let leaf = typst_leaf::compile_rule(
                        &premises,
                        &rule.conclusion.source,
                        ctx.cache_dir,
                    )?;
                    match rule.id {
                        | Some(ref id) => {
                            output.push_str("<figure class=\"rule\" id=\"");
                            output.push_str(&escape(id));
                            output.push_str("\">\n");
                        },
                        | None => output.push_str("<figure class=\"rule\">\n"),
                    }
                    output.push_str(&leaf_markup(leaf, "rule", notes));
                    output.push_str("\n<figcaption>");
                    output.push_str(&escape(&rule.name));
                    output.push_str("</figcaption>\n</figure>\n");
                },
                | Block::Diagram(ref diagram) => {
                    let leaf = typst_leaf::compile_diagram(&diagram.source, ctx.cache_dir)?;
                    output.push_str("<figure id=\"");
                    output.push_str(&escape(&diagram.id));
                    output.push_str("\">\n");
                    output.push_str(&leaf_markup(leaf, "diagram", notes));
                    output.push_str("\n<figcaption>");
                    output.push_str(&escape(&diagram.caption));
                    output.push_str("</figcaption>\n</figure>\n");
                },
                | Block::Code(ref code) => {
                    let expected_output =
                        code.expect_output
                            .as_ref()
                            .map_or_else(String::new, |expected| {
                                format!(
                                    "<p class=\"expect\">expected output: \
                                     <code>{}</code></p>\n",
                                    escape(expected),
                                )
                            });
                    let expected_error =
                        code.expect_error
                            .as_ref()
                            .map_or_else(String::new, |expected| {
                                format!(
                                    "<p class=\"expect\">expected error: \
                                     <code>{}</code></p>\n",
                                    escape(expected),
                                )
                            });
                    output.push_str("<pre><code class=\"lang-");
                    output.push_str(&escape(&code.language));
                    output.push_str("\">");
                    output.push_str(&escape(&code.text));
                    output.push_str("</code></pre>\n");
                    output.push_str(&expected_output);
                    output.push_str(&expected_error);
                },
                | Block::List(ref list) => {
                    let tag = if list.ordered { "ol" } else { "ul" };
                    output.push('<');
                    output.push_str(tag);
                    output.push_str(">\n");
                    for item in &list.items {
                        let body = render_inlines(&item.body, ctx, notes)?;
                        output.push_str("<li>");
                        if let Some(ref lead) = item.lead {
                            output.push_str("<strong>");
                            output.push_str(&escape(lead));
                            output.push_str("</strong> — ");
                        }
                        output.push_str(&body);
                        output.push_str("</li>\n");
                    }
                    output.push_str("</");
                    output.push_str(tag);
                    output.push_str(">\n");
                },
                | Block::Table(ref table) => {
                    output.push_str("<figure class=\"table\">\n<table>\n");
                    output.push_str("<thead>\n<tr>");
                    for cell in &table.header {
                        let body = render_inlines(&cell.content, ctx, notes)?;
                        output.push_str("<th>");
                        output.push_str(&body);
                        output.push_str("</th>");
                    }
                    output.push_str("</tr>\n</thead>\n<tbody>\n");
                    for row in &table.rows {
                        output.push_str("<tr>");
                        for cell in &row.cells {
                            let body = render_inlines(&cell.content, ctx, notes)?;
                            output.push_str("<td>");
                            output.push_str(&body);
                            output.push_str("</td>");
                        }
                        output.push_str("</tr>\n");
                    }
                    output.push_str("</tbody>\n</table>\n");
                    if !table.caption.is_empty() {
                        output.push_str("<figcaption>");
                        output.push_str(&escape(&table.caption));
                        output.push_str("</figcaption>\n");
                    }
                    output.push_str("</figure>\n");
                },
                | Block::References(ref keys) => {
                    output.push_str(&render_references(keys, ctx.references));
                },
            },
        }
    }

    Ok(output)
}

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
fn render_references(
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

/// Render an inline sequence to `HTML`.
fn render_inlines(
    inlines: &[Inline],
    ctx: &RenderContext<'_>,
    notes: &mut Vec<String>,
) -> Result<String, DocError>
{
    inlines
        .iter()
        .map(|inline| render_inline(inline, ctx, notes))
        .collect()
}

/// Render one inline element to `HTML`.
fn render_inline(
    inline: &Inline,
    ctx: &RenderContext<'_>,
    notes: &mut Vec<String>,
) -> Result<String, DocError>
{
    match *inline {
        | Inline::Text(ref text) => Ok(escape(text)),
        | Inline::TermDef(ref term_def) => Ok(format!(
            "<dfn id=\"term-{key}\">{text}</dfn>",
            key = escape(&term_def.key),
            text = escape(&term_def.text),
        )),
        | Inline::TermRef(ref term_ref) => {
            let label = ctx
                .terms
                .get(&term_ref.key)
                .cloned()
                .unwrap_or_else(|| term_ref.key.clone());
            Ok(format!(
                "<a class=\"term\" href=\"#term-{key}\">{label}</a>",
                key = escape(&term_ref.key),
                label = escape(&label),
            ))
        },
        | Inline::Cite(ref cite) => Ok(format!(
            "<sup><a class=\"cite\" href=\"#ref-{key}\">[{key}]</a></sup>",
            key = escape(&cite.key),
        )),
        | Inline::Ref(ref anchor) => Ok(render_anchor_ref(anchor, ctx)),
        | Inline::Math(ref math) => render_math(math, ctx, notes),
    }
}

/// Render an inline anchor reference as a titled cross-page link.
///
/// A target that is itself a component links to that component's page; any
/// other anchor links to its owning component's page at the anchor fragment.
/// An unresolvable target (rejected by the validator, but rendered
/// defensively) falls back to a same-page fragment labeled by the raw id.
fn render_anchor_ref(
    anchor: &crate::model::AnchorRef,
    ctx: &RenderContext<'_>,
) -> String
{
    match ctx.anchors.get(&anchor.target) {
        | Some(target) => {
            let href = if target.component == anchor.target {
                format!("{}.html", target.component)
            }
            else {
                format!("{}.html#{}", target.component, anchor.target)
            };
            format!(
                "<a class=\"xref\" href=\"{href}\">{title}</a>",
                href = escape(&href),
                title = escape(&target.title),
            )
        },
        | None => format!(
            "<a class=\"xref\" href=\"#{target}\">{target}</a>",
            target = escape(&anchor.target),
        ),
    }
}

/// Render a math leaf to inline `SVG` (or a placeholder), wrapped for
/// accessibility.
fn render_math(
    math: &Math,
    ctx: &RenderContext<'_>,
    notes: &mut Vec<String>,
) -> Result<String, DocError>
{
    let leaf = typst_leaf::compile_math(&math.source, ctx.cache_dir)?;
    let kind = match math.display {
        | MathDisplay::Inline => "math",
        | MathDisplay::Block => "math math-block",
    };
    Ok(format!(
        "<span class=\"{kind}\" role=\"math\" aria-label=\"{label}\">{svg}</span>",
        label = escape(&math.source),
        svg = leaf_markup(leaf, "math", notes),
    ))
}

/// Turn a compiled leaf into inline markup, recording a note on a placeholder.
fn leaf_markup(
    leaf: Leaf,
    kind: &str,
    notes: &mut Vec<String>,
) -> String
{
    match leaf {
        | Leaf::Svg(svg) => svg,
        | Leaf::Missing(reason) => {
            let note = format!("{kind}: {reason}");
            if !notes.contains(&note) {
                notes.push(note);
            }
            format!("<span class=\"fallback\">[{kind} placeholder]</span>")
        },
    }
}

/// Wrap page body in the full `HTML` document skeleton.
fn page(
    title: &str,
    body: &str,
) -> String
{
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta \
         name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>{title}</title>\n\
         <style>\n{STYLESHEET}\n</style>\n</head>\n<body>\n{body}</body>\n</html>\n"
    )
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

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use std::path::Path;

    use super::AnchorTarget;
    use super::RenderContext;
    use super::anchor_map;
    use super::render_blocks;
    use super::render_references;
    use super::term_map;
    use crate::bibliography::Bibliography;
    use crate::model::AnchorRef;
    use crate::model::Block;
    use crate::model::CiteKey;
    use crate::model::Definition;
    use crate::model::Document;
    use crate::model::Example;
    use crate::model::Inline;
    use crate::model::List;
    use crate::model::ListItem;
    use crate::model::Math;
    use crate::model::MathDisplay;
    use crate::model::Rule;
    use crate::model::Section;
    use crate::model::Status;
    use crate::model::Table;
    use crate::model::TableCell;
    use crate::model::TableRow;
    use crate::model::TermDef;

    /// The first nested definition remains authoritative in document order.
    #[test]
    fn term_map_prefers_first_nested_definition()
    {
        let definition = |text: &str| {
            Block::Definition(Definition {
                id: None,
                term: "term".to_owned(),
                body: alloc::vec![Inline::TermDef(TermDef {
                    key: "shared".to_owned(),
                    text: text.to_owned(),
                })],
            })
        };
        let document = Document {
            id: "component".to_owned(),
            spec_version: "1".to_owned(),
            title: "Component".to_owned(),
            status: Status::Partial,
            grounds: Vec::new(),
            derives: Vec::new(),
            blocks: alloc::vec![Block::Section(Section {
                id: None,
                status: None,
                title: "Section".to_owned(),
                blocks: alloc::vec![
                    definition("first"),
                    Block::Example(Example {
                        title: "Example".to_owned(),
                        blocks: alloc::vec![definition("second")],
                    }),
                ],
            })],
            source_path: "mem:component.xml".to_owned(),
        };

        let terms = term_map(&[document]);

        assert_eq!(terms.get("shared").map(String::as_str), Some("first"));
    }
    /// Nested containers render with balanced markup in document order.
    #[test]
    fn nested_blocks_render_in_document_order() -> Result<(), crate::DocError>
    {
        let blocks = alloc::vec![Block::Section(Section {
            id: None,
            status: None,
            title: "Outer".to_owned(),
            blocks: alloc::vec![
                Block::Example(Example {
                    title: "Inner".to_owned(),
                    blocks: alloc::vec![Block::Prose(alloc::vec![Inline::Text(
                        "body & more".to_owned(),
                    )])],
                }),
                Block::Definition(Definition {
                    id: None,
                    term: "Last".to_owned(),
                    body: alloc::vec![Inline::Text("tail".to_owned())],
                }),
            ],
        })];
        let terms = BTreeMap::new();
        let anchors = BTreeMap::new();
        let bibliography = Bibliography::default();
        let context = RenderContext::new(Path::new("unused"), &terms, &anchors, &bibliography);
        let mut notes = Vec::new();

        let rendered = render_blocks(&blocks, 2usize, &context, &mut notes)?;

        assert_eq!(
            rendered,
            concat!(
                "<section>\n",
                "<h2>Outer</h2>\n",
                "<div class=\"example\">\n",
                "<h3>Example: Inner</h3>\n",
                "<p>body &amp; more</p>\n",
                "</div>\n",
                "<dl>\n",
                "<dt>Last</dt>\n",
                "<dd>tail</dd>\n",
                "</dl>\n",
                "</section>\n",
            ),
        );
        assert!(notes.is_empty());
        Ok(())
    }

    /// The anchor map records one entry per declaring block kind with the
    /// owning component and the human-readable title projection.
    #[test]
    fn anchor_map_collects_each_declaring_kind()
    {
        let math = || Math {
            source: "x".to_owned(),
            display: MathDisplay::Block,
        };
        let document = Document {
            id: "comp".to_owned(),
            spec_version: "1".to_owned(),
            title: "Component Title".to_owned(),
            status: Status::Partial,
            grounds: Vec::new(),
            derives: Vec::new(),
            blocks: alloc::vec![Block::Section(Section {
                id: Some("sec".to_owned()),
                status: None,
                title: "Section Title".to_owned(),
                blocks: alloc::vec![
                    Block::Rule(Rule {
                        id: Some("rul".to_owned()),
                        name: "Rule Name".to_owned(),
                        premises: Vec::new(),
                        conclusion: math(),
                    }),
                    Block::Definition(Definition {
                        id: Some("def".to_owned()),
                        term: "Term".to_owned(),
                        body: Vec::new(),
                    }),
                ],
            })],
            source_path: "mem:comp.xml".to_owned(),
        };

        let anchors = anchor_map(&[document]);

        let expect = |id: &str, title: &str| {
            assert_eq!(
                anchors.get(id),
                Some(&AnchorTarget {
                    component: "comp".to_owned(),
                    title: title.to_owned(),
                }),
                "anchor '{id}'",
            );
        };
        expect("comp", "Component Title");
        expect("sec", "Section Title");
        expect("rul", "Rule Name");
        expect("def", "Term");
    }

    /// List, table, and anchor-ref payload blocks render their exact markup.
    #[test]
    fn payload_blocks_render_expected_markup() -> Result<(), crate::DocError>
    {
        let blocks = alloc::vec![
            Block::List(List {
                ordered: false,
                items: alloc::vec![ListItem {
                    lead: Some("K".to_owned()),
                    body: alloc::vec![Inline::Text("v".to_owned())],
                }],
            }),
            Block::Table(Table {
                caption: "Cap".to_owned(),
                header: alloc::vec![TableCell {
                    content: alloc::vec![Inline::Text("H".to_owned())],
                }],
                rows: alloc::vec![TableRow {
                    cells: alloc::vec![
                        TableCell {
                            content: alloc::vec![Inline::Ref(AnchorRef {
                                target: "tgt".to_owned(),
                            })],
                        },
                        TableCell {
                            content: alloc::vec![Inline::Text("x".to_owned())],
                        },
                    ],
                }],
            }),
        ];
        let terms = BTreeMap::new();
        let mut anchors = BTreeMap::new();
        anchors.insert("tgt".to_owned(), AnchorTarget {
            component: "other".to_owned(),
            title: "Target".to_owned(),
        });
        let bibliography = Bibliography::default();
        let context = RenderContext::new(Path::new("unused"), &terms, &anchors, &bibliography);
        let mut notes = Vec::new();

        let rendered = render_blocks(&blocks, 2usize, &context, &mut notes)?;

        assert_eq!(
            rendered,
            concat!(
                "<ul>\n",
                "<li><strong>K</strong> — v</li>\n",
                "</ul>\n",
                "<figure class=\"table\">\n",
                "<table>\n",
                "<thead>\n",
                "<tr><th>H</th></tr>\n",
                "</thead>\n",
                "<tbody>\n",
                "<tr><td><a class=\"xref\" href=\"other.html#tgt\">Target</a></td><td>x</td></tr>\n",
                "</tbody>\n",
                "</table>\n",
                "<figcaption>Cap</figcaption>\n",
                "</figure>\n",
            ),
        );
        assert!(notes.is_empty());
        Ok(())
    }

    /// Reference rows materialize metadata and preferred outbound links.
    #[test]
    fn reference_rows_materialize_metadata_and_links() -> Result<(), crate::DocError>
    {
        let bibliography = Bibliography::parse_source(
            r#"D:
  type: article
  title: "Typed & Linked"
  author: Ada & Bob
  date: 2024
  parent:
    type: proceedings
    title: Venue
  serial-number:
    doi: 10.1000/example
A:
  type: article
  title: Archive
  serial-number:
    arxiv: "2402.00002"
U:
  type: thesis
  title: Repository Copy
  organization: Example University
  url: https://example.test/thesis
M:
  type: misc
  title: Metadata Minimum
"#,
        )?;
        let keys = alloc::vec![
            CiteKey {
                key: "D".to_owned(),
            },
            CiteKey {
                key: "A".to_owned(),
            },
            CiteKey {
                key: "U".to_owned(),
            },
            CiteKey {
                key: "M".to_owned(),
            },
        ];

        let rendered = render_references(&keys, &bibliography);

        assert_eq!(
            rendered,
            concat!(
                "<h2>References</h2>\n",
                "<ul class=\"refs\">\n",
                "<li id=\"ref-D\"><span class=\"ref-key\">[D]</span> Ada &amp; Bob. ",
                "<cite>Typed &amp; Linked</cite>. Venue, 2024. ",
                "<a class=\"ref-locator\" href=\"https://doi.org/10.1000/example\">",
                "doi:10.1000/example</a>.</li>\n",
                "<li id=\"ref-A\"><span class=\"ref-key\">[A]</span> ",
                "<cite>Archive</cite>. ",
                "<a class=\"ref-locator\" href=\"https://arxiv.org/abs/2402.00002\">",
                "arXiv:2402.00002</a>.</li>\n",
                "<li id=\"ref-U\"><span class=\"ref-key\">[U]</span> ",
                "<cite>Repository Copy</cite>. Example University. ",
                "<a class=\"ref-locator\" href=\"https://example.test/thesis\">",
                "source</a>.</li>\n",
                "<li id=\"ref-M\"><span class=\"ref-key\">[M]</span> ",
                "<cite>Metadata Minimum</cite>.</li>\n",
                "</ul>\n",
            ),
        );
        Ok(())
    }
}
