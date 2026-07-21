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
dl.grammar dt{font-family:ui-monospace,monospace;font-weight:bold}
.expect{color:var(--muted);font-size:.85rem;margin:.2em 0}"#;

/// Rendering context: the leaf cache directory and the corpus term map.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct RenderContext<'ctx>
{
    /// Directory for compiled leaf assets.
    pub cache_dir: &'ctx Path,
    /// Corpus-wide map from term key to displayed definition text.
    pub terms: &'ctx BTreeMap<String, String>,
}

impl<'ctx> RenderContext<'ctx>
{
    /// Build a rendering context.
    #[inline]
    #[must_use]
    pub const fn new(
        cache_dir: &'ctx Path,
        terms: &'ctx BTreeMap<String, String>,
    ) -> Self
    {
        Self { cache_dir, terms }
    }
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
                    output.push_str("<section>\n<h");
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
                    output.push_str("<dl>\n<dt>");
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
                    output.push_str("<figure class=\"rule\">\n");
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
                | Block::References(ref keys) => output.push_str(&render_references(keys)),
            },
        }
    }

    Ok(output)
}

/// Render a per-component references list.
fn render_references(keys: &[CiteKey]) -> String
{
    let rows: String = keys
        .iter()
        .map(|cite| {
            format!(
                "<li id=\"ref-{key}\">[{key}]</li>\n",
                key = escape(&cite.key)
            )
        })
        .collect::<Vec<String>>()
        .concat();
    format!("<h2>References</h2>\n<ul class=\"refs\">\n{rows}</ul>\n")
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
        | Inline::Math(ref math) => render_math(math, ctx, notes),
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

    use super::RenderContext;
    use super::render_blocks;
    use super::term_map;
    use crate::model::Block;
    use crate::model::Definition;
    use crate::model::Document;
    use crate::model::Example;
    use crate::model::Inline;
    use crate::model::Section;
    use crate::model::Status;
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
        let context = RenderContext::new(Path::new("unused"), &terms);
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
}
