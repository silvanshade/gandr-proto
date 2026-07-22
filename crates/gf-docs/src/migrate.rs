//! Migration: legacy `XML` components to `.gfd` abstract-syntax text.
//!
//! The legacy parse (`gandr_workflow_docs::parse`) produces the typed model;
//! this module walks it into [`Sexp`] trees rendered with the canonical
//! layout. Lexicon constant naming is the contract shared with the generated
//! lexicon modules: `term_<key>`, `cite_<key>`, `anchor_<id>` with `-` mapped
//! to `_`.

use std::path::Path;

use gandr_workflow_docs::model::Block;
use gandr_workflow_docs::model::CodeRole;
use gandr_workflow_docs::model::Document;
use gandr_workflow_docs::model::Inline;
use gandr_workflow_docs::model::ListItem;
use gandr_workflow_docs::model::Status;
use gandr_workflow_docs::model::TableCell;
use gandr_workflow_docs::parse::parse_document;

use crate::error::GfDocsError;
use crate::sexp::Sexp;

/// Translate one legacy `XML` component file into its `.gfd` text.
///
/// # Errors
///
/// [`GfDocsError::Model`] when the legacy parser rejects the file;
/// [`GfDocsError::Translation`] on a construct outside the `PoC` grammar
/// (missing section id, multi-cite diagram, non-section top-level block).
#[inline]
pub fn translate_file(path: &Path) -> Result<String, GfDocsError>
{
    let xml = std::fs::read_to_string(path)?;
    let parsed = parse_document(path, &xml).map_err(|e| GfDocsError::Model(e.to_string()))?;
    let document = parsed
        .document
        .ok_or_else(|| GfDocsError::Model(format!("{}: no document produced", path.display())))?;
    let tree = translate(&document)?;
    Ok(format!("{}\n", tree.render()))
}

/// Translate a parsed component into its expression tree.
fn translate(document: &Document) -> Result<Sexp, GfDocsError>
{
    let status = status_const(document.status)
        .ok_or_else(|| GfDocsError::Translation("unknown status variant".into()))?;

    let mut sections = Vec::new();
    let mut references = Vec::new();
    for block in &document.blocks {
        match *block {
            | Block::Section(_) => sections.push(block),
            | Block::References(ref cites) => references.extend(cites.iter()),
            | _ => {
                return Err(GfDocsError::Translation(format!(
                    "top-level non-section block: {block:?}"
                )));
            },
        }
    }

    Ok(Sexp::app("MkComponent", vec![
        Sexp::atom(anchor_const(&document.id)),
        Sexp::atom(gf_str(&document.title)),
        Sexp::atom(status),
        anchor_list(&document.grounds),
        anchor_list(&document.derives),
        list_of("Section", &sections, |&b| block_expr(b))?,
        list_of("CiteKey", &references, |c| {
            Ok(Sexp::atom(cite_const(&c.key)))
        })?,
    ]))
}

/// Render one block as a constructor expression.
fn block_expr(block: &Block) -> Result<Sexp, GfDocsError>
{
    Ok(match *block {
        | Block::Section(ref section) => {
            let id = section
                .id
                .as_deref()
                .ok_or_else(|| GfDocsError::Translation("section without id".into()))?;
            Sexp::app("MkSection", vec![
                Sexp::atom(anchor_const(id)),
                Sexp::atom(gf_str(&section.title)),
                list_of("Block", &section.blocks, block_expr)?,
            ])
        },
        | Block::Prose(ref inlines) => Sexp::app("ProseBlock", vec![inline_list(inlines)?]),
        | Block::Judgements(ref j) => Sexp::app("JudgementsBlock", vec![
            Sexp::atom(gf_str(&j.title)),
            list_of("MathRow", &j.forms, |m| {
                Ok(Sexp::app("MkMathRow", vec![Sexp::atom(gf_str(&m.source))]))
            })?,
        ]),
        | Block::Grammar(ref productions) => Sexp::app("GrammarBlock", vec![list_of(
            "Production",
            productions,
            |p| {
                Ok(Sexp::app("MkProduction", vec![
                    Sexp::atom(gf_str(&p.symbol)),
                    Sexp::atom(gf_str(&p.definition)),
                ]))
            },
        )?]),
        | Block::Rule(ref rule) => {
            let id = rule
                .id
                .as_deref()
                .ok_or_else(|| GfDocsError::Translation("rule without id".into()))?;
            Sexp::app("RuleBlock", vec![
                Sexp::atom(anchor_const(id)),
                Sexp::atom(gf_str(&rule.name)),
                list_of("MathRow", &rule.premises, |m| {
                    Ok(Sexp::app("MkMathRow", vec![Sexp::atom(gf_str(&m.source))]))
                })?,
                Sexp::app("MkMathRow", vec![Sexp::atom(gf_str(
                    &rule.conclusion.source,
                ))]),
            ])
        },
        | Block::Definition(ref def) => {
            let id = def
                .id
                .as_deref()
                .ok_or_else(|| GfDocsError::Translation("definition without id".into()))?;
            Sexp::app("DefinitionBlock", vec![
                Sexp::atom(anchor_const(id)),
                Sexp::atom(term_const(&def.term)),
                inline_list(&def.body)?,
            ])
        },
        | Block::Diagram(ref diagram) => {
            let mut cites = diagram.cites.iter();
            let cite = match (cites.next(), cites.next()) {
                | (Some(cite), None) => cite_const(&cite.key),
                | (None, _) => {
                    return Err(GfDocsError::Translation(
                        "diagram without cites (PoC grammar requires exactly one)".into(),
                    ));
                },
                | (Some(_), Some(_)) => {
                    return Err(GfDocsError::Translation(
                        "diagram with multiple cites (PoC grammar takes one)".into(),
                    ));
                },
            };
            Sexp::app("DiagramBlock", vec![
                Sexp::atom(anchor_const(&diagram.id)),
                Sexp::atom(gf_str(&diagram.caption)),
                Sexp::atom(cite),
                Sexp::atom(gf_str(&diagram.source)),
            ])
        },
        | Block::Code(ref code) => match (code.role, code.expect_output.as_ref()) {
            | (CodeRole::Api, _) => Sexp::app("ApiCodeBlock", vec![
                Sexp::atom(gf_str(&code.language)),
                Sexp::atom(gf_str(&code.text)),
            ]),
            | (CodeRole::Example, Some(expect)) => Sexp::app("ExpectCodeBlock", vec![
                Sexp::atom(gf_str(&code.language)),
                Sexp::atom(gf_str(expect)),
                Sexp::atom(gf_str(&code.text)),
            ]),
            | (CodeRole::Example, None) => Sexp::app("PlainCodeBlock", vec![
                Sexp::atom(gf_str(&code.language)),
                Sexp::atom(gf_str(&code.text)),
            ]),
            | _ => {
                return Err(GfDocsError::Translation(
                    "unknown code role (model evolved)".into(),
                ));
            },
        },
        | Block::Example(ref example) => Sexp::app("ExampleBlock", vec![
            Sexp::atom(gf_str(&example.title)),
            list_of("Block", &example.blocks, block_expr)?,
        ]),
        | Block::List(ref list) => {
            let items = list_of("Item", &list.items, item_expr)?;
            if list.ordered {
                Sexp::app("RegisterBlock", vec![items])
            }
            else {
                Sexp::app("PlainRegisterBlock", vec![items])
            }
        },
        | Block::Table(ref table) => Sexp::app("InventoryBlock", vec![
            Sexp::atom(gf_str(&table.caption)),
            row_expr("MkHeaderRow", &table.header)?,
            list_of("Row", &table.rows, |row| row_expr("MkBodyRow", &row.cells))?,
        ]),
        | Block::References(_) => {
            return Err(GfDocsError::Translation(
                "references block outside component root".into(),
            ));
        },
        | _ => {
            return Err(GfDocsError::Translation(
                "unknown block variant (model evolved)".into(),
            ));
        },
    })
}

/// Render one list item.
fn item_expr(item: &ListItem) -> Result<Sexp, GfDocsError>
{
    match item.lead.as_deref() {
        | Some(lead) => Ok(Sexp::app("MkItem", vec![
            Sexp::atom(gf_str(lead)),
            inline_list(&item.body)?,
        ])),
        | None => Ok(Sexp::app("MkPlainItem", vec![inline_list(&item.body)?])),
    }
}

/// Render one table row under the named row constructor.
fn row_expr(
    constructor: &str,
    cells: &[TableCell],
) -> Result<Sexp, GfDocsError>
{
    Ok(Sexp::app(constructor, vec![list_of(
        "Cell",
        cells,
        |cell| Ok(Sexp::app("MkCell", vec![inline_list(&cell.content)?])),
    )?]))
}

/// Render a `[Inline]` list, choosing the glue constructor at boundaries
/// where the next element is punctuation-leading text (so `.` and friends
/// bind to the preceding inline instead of taking a word space).
fn inline_list(inlines: &[Inline]) -> Result<Sexp, GfDocsError>
{
    let mut out = Sexp::atom("BaseInline");
    for (index, item) in inlines.iter().enumerate().rev() {
        let glued = matches!(
            inlines.get(index.saturating_add(1)),
            Some(Inline::Text(text))
                if text
                    .trim_start()
                    .starts_with(|c: char| ".,;:!?)]}\"'".contains(c))
        );
        let cons = if glued {
            "ConsInlineGlued"
        }
        else {
            "ConsInline"
        };
        out = Sexp::app(cons, vec![inline_expr(item)?, out]);
    }
    Ok(out)
}

/// Render one inline element.
fn inline_expr(inline: &Inline) -> Result<Sexp, GfDocsError>
{
    Ok(match *inline {
        | Inline::Text(ref text) => Sexp::app("Txt", vec![Sexp::atom(gf_str(&normalize(text)))]),
        | Inline::TermDef(ref def) => Sexp::app("TermDef", vec![
            Sexp::atom(term_const(&def.key)),
            Sexp::atom(gf_str(&def.text)),
        ]),
        | Inline::TermRef(ref reference) => {
            Sexp::app("TermRef", vec![Sexp::atom(term_const(&reference.key))])
        },
        | Inline::Cite(ref key) => Sexp::app("CiteRef", vec![Sexp::atom(cite_const(&key.key))]),
        | Inline::Ref(ref reference) => {
            Sexp::app("XRef", vec![Sexp::atom(anchor_const(&reference.target))])
        },
        | Inline::Math(ref math) => Sexp::app("MathInline", vec![Sexp::atom(gf_str(&math.source))]),
        | _ => {
            return Err(GfDocsError::Translation(
                "unknown inline variant (model evolved)".into(),
            ));
        },
    })
}

/// Fold a slice into a `Cons`-nested list expression.
fn list_of<T>(
    category: &str,
    items: &[T],
    render: impl Fn(&T) -> Result<Sexp, GfDocsError>,
) -> Result<Sexp, GfDocsError>
{
    let mut out = Sexp::atom(format!("Base{category}"));
    for item in items.iter().rev() {
        out = Sexp::app(format!("Cons{category}"), vec![render(item)?, out]);
    }
    Ok(out)
}

/// Fold id strings into an `[Anchor]` list expression.
fn anchor_list(ids: &[String]) -> Sexp
{
    let mut out = Sexp::atom("BaseAnchor");
    for id in ids.iter().rev() {
        out = Sexp::app("ConsAnchor", vec![Sexp::atom(anchor_const(id)), out]);
    }
    out
}

/// The lexicon constant for a term key.
#[inline]
#[must_use]
pub fn term_const(key: &str) -> String
{
    format!("term_{}", mangle(key))
}

/// The lexicon constant for a cite key.
#[inline]
#[must_use]
pub fn cite_const(key: &str) -> String
{
    format!("cite_{}", mangle(key))
}

/// The lexicon constant for an anchor id.
#[inline]
#[must_use]
pub fn anchor_const(id: &str) -> String
{
    format!("anchor_{}", mangle(id))
}

/// Map a corpus key to a `GF`-safe identifier fragment.
fn mangle(key: &str) -> String
{
    key.replace('-', "_")
}

/// Quote text as a `GF` string literal.
fn gf_str(text: &str) -> String
{
    let mut out = String::with_capacity(text.len().saturating_add(2));
    out.push('"');
    for ch in text.chars() {
        match ch {
            | '"' => out.push_str("\\\""),
            | '\\' => out.push_str("\\\\"),
            | '\n' => out.push_str("\\n"),
            | '\t' => out.push_str("\\t"),
            | '\r' => out.push_str("\\r"),
            | _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

/// Collapse whitespace runs to single spaces (the legacy renderer's
/// normalization, mirrored so equivalence comparisons hold).
fn normalize(text: &str) -> String
{
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The canonical constructor name for a lifecycle status.
fn status_const(status: Status) -> Option<&'static str>
{
    match status {
        | Status::Built => Some("StatusBuilt"),
        | Status::Partial => Some("StatusPartial"),
        | Status::AdoptedUnbuilt => Some("StatusAdoptedUnbuilt"),
        | Status::DesignPass => Some("StatusDesignPass"),
        | Status::Dormant => Some("StatusDormant"),
        | _ => None,
    }
}
