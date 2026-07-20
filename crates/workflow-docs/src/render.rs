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

/// Recursively collect term definitions from a block.
fn collect_terms(
    block: &Block,
    terms: &mut BTreeMap<String, String>,
)
{
    match *block {
        | Block::Section(ref section) => {
            for nested in &section.blocks {
                collect_terms(nested, terms);
            }
        },
        | Block::Example(ref example) => {
            for nested in &example.blocks {
                collect_terms(nested, terms);
            }
        },
        | Block::Prose(ref inlines) => collect_inline_terms(inlines, terms),
        | Block::Definition(ref definition) => collect_inline_terms(&definition.body, terms),
        | Block::Judgements(_)
        | Block::Grammar(_)
        | Block::Rule(_)
        | Block::Diagram(_)
        | Block::Code(_)
        | Block::References(_) => {},
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
    let blocks = document
        .blocks
        .iter()
        .map(|block| render_block(block, 2usize, ctx, notes))
        .collect::<Result<String, DocError>>()?;
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

/// Render one block at the given heading level.
fn render_block(
    block: &Block,
    level: usize,
    ctx: &RenderContext<'_>,
    notes: &mut Vec<String>,
) -> Result<String, DocError>
{
    match *block {
        | Block::Section(ref section) => {
            let heading = level.min(6usize);
            let inner = section
                .blocks
                .iter()
                .map(|nested| render_block(nested, level.saturating_add(1), ctx, notes))
                .collect::<Result<String, DocError>>()?;
            Ok(format!(
                "<section>\n<h{heading}>{title}</h{heading}>\n{inner}</section>\n",
                title = escape(&section.title),
            ))
        },
        | Block::Example(ref example) => {
            let heading = level.min(6usize);
            let inner = example
                .blocks
                .iter()
                .map(|nested| render_block(nested, level.saturating_add(1), ctx, notes))
                .collect::<Result<String, DocError>>()?;
            Ok(format!(
                "<div class=\"example\">\n<h{heading}>Example: {title}</h{heading}>\n{inner}</div>\n",
                title = escape(&example.title),
            ))
        },
        | Block::Prose(ref inlines) => {
            Ok(format!("<p>{}</p>\n", render_inlines(inlines, ctx, notes)?))
        },
        | Block::Definition(ref definition) => Ok(format!(
            "<dl>\n<dt>{term}</dt>\n<dd>{body}</dd>\n</dl>\n",
            term = escape(&definition.term),
            body = render_inlines(&definition.body, ctx, notes)?,
        )),
        | Block::Judgements(ref judgements) => {
            let heading = level.min(6usize);
            let forms = judgements
                .forms
                .iter()
                .map(|form| render_math(form, ctx, notes).map(|svg| format!("<div>{svg}</div>\n")))
                .collect::<Result<String, DocError>>()?;
            Ok(format!(
                "<div class=\"judgements\">\n<h{heading}>{title}</h{heading}>\n{forms}</div>\n",
                title = escape(&judgements.title),
            ))
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
            Ok(format!("<dl class=\"grammar\">\n{rows}</dl>\n"))
        },
        | Block::Rule(ref rule) => {
            let premises = rule
                .premises
                .iter()
                .map(|math| math.source.clone())
                .collect::<Vec<String>>()
                .join(" quad ");
            let leaf = typst_leaf::compile_rule(&premises, &rule.conclusion.source, ctx.cache_dir)?;
            Ok(format!(
                "<figure class=\"rule\">\n{svg}\n<figcaption>{name}</figcaption>\n</figure>\n",
                svg = leaf_markup(leaf, "rule", notes),
                name = escape(&rule.name),
            ))
        },
        | Block::Diagram(ref diagram) => {
            let leaf = typst_leaf::compile_diagram(&diagram.source, ctx.cache_dir)?;
            Ok(format!(
                "<figure id=\"{id}\">\n{svg}\n<figcaption>{caption}</figcaption>\n</figure>\n",
                id = escape(&diagram.id),
                svg = leaf_markup(leaf, "diagram", notes),
                caption = escape(&diagram.caption),
            ))
        },
        | Block::Code(ref code) => {
            let output = code
                .expect_output
                .as_ref()
                .map_or_else(String::new, |expected| {
                    format!(
                        "<p class=\"expect\">expected output: <code>{}</code></p>\n",
                        escape(expected),
                    )
                });
            let error = code
                .expect_error
                .as_ref()
                .map_or_else(String::new, |expected| {
                    format!(
                        "<p class=\"expect\">expected error: <code>{}</code></p>\n",
                        escape(expected),
                    )
                });
            Ok(format!(
                "<pre><code class=\"lang-{lang}\">{text}</code></pre>\n{output}{error}",
                lang = escape(&code.language),
                text = escape(&code.text),
            ))
        },
        | Block::References(ref keys) => Ok(render_references(keys)),
    }
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
