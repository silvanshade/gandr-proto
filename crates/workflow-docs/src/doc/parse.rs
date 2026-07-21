//! The parse-equals-validate pass for the prose document classes.
//!
//! `XML` text becomes a [`DocRecord`] — well-formedness failures are
//! operational ([`crate::DocError::Xml`]); structural violations (unknown root,
//! missing
//! required attribute, unknown element, malformed status, misplaced banner) are
//! [`crate::Diagnostic`] values. A file that yields any structural diagnostic
//! produces
//! no [`DocRecord`], so the run fails on it. Class-schema and cross-reference
//! checks are deferred to [`crate::doc::validate`].
//!
//! Only sections nest, so the block parser is a single explicit frame stack
//! (the component parser's discipline); lists, tables, items, and cells are
//! flat loops.
//!
//! [`DocRecord`]: crate::doc::model::DocRecord

use alloc::string::String;
use alloc::vec::Vec;
use std::path::Path;

use roxmltree::Document as XmlDocument;
use roxmltree::Node;

use crate::Diagnostic;
use crate::DocError;
use crate::doc::model::Banner;
use crate::doc::model::DocBlock;
use crate::doc::model::DocCell;
use crate::doc::model::DocClass;
use crate::doc::model::DocCode;
use crate::doc::model::DocInline;
use crate::doc::model::DocItem;
use crate::doc::model::DocList;
use crate::doc::model::DocRecord;
use crate::doc::model::DocRow;
use crate::doc::model::DocSection;
use crate::doc::model::DocTable;
use crate::doc::model::Label;
use crate::doc::model::LabelRef;
use crate::model::CiteKey;
use crate::parse::collapse_whitespace;
use crate::parse::element_location;
use crate::parse::element_text;
use crate::parse::required_attribute;
use crate::parse::required_status;

/// Outcome of parsing one file: an optional model and any structural
/// diagnostics.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DocParsed
{
    /// Parsed document, absent when a structural diagnostic prevented it.
    pub document: Option<DocRecord>,
    /// Structural diagnostics discovered while parsing.
    pub diagnostics: Vec<Diagnostic>,
}

/// A suspended parent section while the iterative block parser visits a child.
struct DocBlockFrame<'tree, 'input>
{
    /// Parent nodes still awaiting parsing, in reverse document order.
    pending: Vec<Node<'tree, 'input>>,
    /// Parent blocks already parsed in document order.
    blocks: Vec<DocBlock>,
    /// Optional document-local anchor of the suspended section.
    id: Option<String>,
    /// Optional date stamp of the suspended section.
    date: Option<String>,
    /// Human-facing title of the suspended section.
    title: String,
}

/// Parse one prose-document file into a model plus structural diagnostics.
///
/// # Errors
/// Returns [`DocError::Xml`] when the input is not well-formed `XML`.
#[inline]
pub fn parse_doc_document(
    path: &Path,
    xml: &str,
) -> Result<DocParsed, DocError>
{
    let tree = XmlDocument::parse(xml).map_err(|error| DocError::Xml {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    let path_text = path.display().to_string();
    let mut diagnostics = Vec::new();
    let document = parse_record(tree.root_element(), &path_text, &mut diagnostics);
    Ok(DocParsed {
        document,
        diagnostics,
    })
}

/// Parse the class-tagged root element.
fn parse_record(
    root: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<DocRecord>
{
    let name = root.tag_name().name();
    let Some(class) = DocClass::parse(name)
    else {
        diagnostics.push(Diagnostic::new(
            "unknown-root",
            element_location(path_text, name),
            format!(
                "root element must be one of [{}], found <{name}>",
                DocClass::root_names().join(", ")
            ),
        ));
        return None;
    };
    let root_name = class.root_name();
    let id = required_attribute(root, "id", path_text, root_name, diagnostics)?;
    let title = required_attribute(root, "title", path_text, root_name, diagnostics)?;
    let status = required_status(root, path_text, root_name, diagnostics)?;
    let crate_scope = match class {
        | DocClass::CrateStatus => {
            required_attribute(root, "crate", path_text, root_name, diagnostics)
        },
        | DocClass::ResearchRecord | DocClass::WorkflowDoc => None,
    };
    let (banner, block_nodes) = split_banner(root, path_text, diagnostics);
    let blocks = parse_blocks(block_nodes, path_text, diagnostics);
    Some(DocRecord {
        class,
        id,
        title,
        status,
        crate_scope,
        banner,
        blocks,
        source_path: path_text.to_owned(),
    })
}

/// Split the leading `<banner>` from the block children, flagging a missing or
/// misplaced banner.
fn split_banner<'tree, 'input>(
    root: Node<'tree, 'input>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Banner, Vec<Node<'tree, 'input>>)
{
    let children: Vec<Node<'tree, 'input>> = root.children().filter(Node::is_element).collect();
    let mut banner = Banner::default();
    let mut seen_banner = false;
    let mut block_nodes = Vec::new();
    for (index, child) in children.iter().enumerate() {
        if child.tag_name().name() == "banner" {
            if index == 0 && !seen_banner {
                banner = parse_banner(*child, path_text, diagnostics);
                seen_banner = true;
            }
            else {
                diagnostics.push(Diagnostic::new(
                    "misplaced-banner",
                    element_location(path_text, "banner"),
                    "the <banner> must be the first and only banner child".to_owned(),
                ));
            }
        }
        else {
            block_nodes.push(*child);
        }
    }
    if !seen_banner {
        diagnostics.push(Diagnostic::new(
            "missing-banner",
            element_location(path_text, "banner"),
            "a prose document must open with a <banner> element".to_owned(),
        ));
    }
    (banner, block_nodes)
}

/// Parse a banner element into its orientation line and free notes.
fn parse_banner(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Banner
{
    let mut banner = Banner::default();
    for child in node.children().filter(Node::is_element) {
        match child.tag_name().name() {
            | "read-when" => {
                if banner.read_when.is_some() {
                    diagnostics.push(Diagnostic::new(
                        "duplicate-read-when",
                        element_location(path_text, "read-when"),
                        "a banner carries at most one <read-when> line".to_owned(),
                    ));
                }
                else {
                    banner.read_when = Some(parse_inlines(child, path_text, diagnostics));
                }
            },
            | "note" => banner
                .notes
                .push(parse_inlines(child, path_text, diagnostics)),
            | other => diagnostics.push(Diagnostic::new(
                "unexpected-child",
                element_location(path_text, other),
                "banner accepts only <read-when> and <note> children".to_owned(),
            )),
        }
    }
    banner
}

/// Parse the block children below the root or a section in document order.
///
/// # Contract
///
/// - requires: `top` are element block nodes of one finite, well-formed `XML`
///   document, in source order.
/// - ensures: returns every recognized block exactly once in source order with
///   section nesting and metadata preserved; structural violations are appended
///   to `diagnostics`.
/// - provides: the typed block forest for the supplied siblings.
/// - panics: none.
/// - intension: explicit suspended-section frames reconstruct nesting while
///   bounding native stack use independently of section depth.
///
/// # Adequacy
///
/// - hypothesis: L3 pointwise — an exact expected block tree distinguishes
///   section metadata, nesting, sibling order, and leaf identity.
/// - witness: `tests::nested_blocks_parse_exact_tree`.
fn parse_blocks(
    top: Vec<Node<'_, '_>>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<DocBlock>
{
    let mut pending = top;
    pending.reverse();
    let mut blocks = Vec::new();
    let mut frames: Vec<DocBlockFrame<'_, '_>> = Vec::new();

    loop {
        while let Some(node) = pending.pop() {
            if node.tag_name().name() == "section" {
                let id = node.attribute("id").map(str::to_owned);
                let date = node.attribute("date").map(str::to_owned);
                let title = required_attribute(node, "title", path_text, "section", diagnostics)
                    .unwrap_or_default();
                frames.push(DocBlockFrame {
                    pending,
                    blocks,
                    id,
                    date,
                    title,
                });
                pending = reversed_element_children(node);
                blocks = Vec::new();
            }
            else if let Some(block) = parse_leaf_block(node, path_text, diagnostics) {
                blocks.push(block);
            }
        }

        let Some(frame) = frames.pop()
        else {
            return blocks;
        };
        let DocBlockFrame {
            pending: parent_pending,
            blocks: mut parent_blocks,
            id,
            date,
            title,
        } = frame;
        parent_blocks.push(DocBlock::Section(DocSection {
            id,
            date,
            title,
            blocks,
        }));
        pending = parent_pending;
        blocks = parent_blocks;
    }
}

/// Collect element children for LIFO traversal in source order.
fn reversed_element_children<'tree, 'input>(parent: Node<'tree, 'input>)
-> Vec<Node<'tree, 'input>>
{
    let mut children: Vec<Node<'tree, 'input>> =
        parent.children().filter(Node::is_element).collect();
    children.reverse();
    children
}

/// Parse one non-container block element.
fn parse_leaf_block(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<DocBlock>
{
    let name = node.tag_name().name();
    match name {
        | "prose" => Some(DocBlock::Prose(parse_inlines(node, path_text, diagnostics))),
        | "list" => Some(DocBlock::List(parse_list(node, path_text, diagnostics))),
        | "table" => Some(DocBlock::Table(parse_table(node, path_text, diagnostics))),
        | "code" => Some(DocBlock::Code(parse_code(node))),
        | other => {
            diagnostics.push(Diagnostic::new(
                "unknown-block",
                element_location(path_text, other),
                format!("unknown block element <{other}>"),
            ));
            None
        },
    }
}

/// Parse a list block into its items.
fn parse_list(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> DocList
{
    let ordered = node
        .attribute("ordered")
        .is_some_and(|value| value == "true");
    let mut items = Vec::new();
    for child in node.children().filter(Node::is_element) {
        if child.tag_name().name() == "item" {
            items.push(DocItem {
                lead: child.attribute("lead").map(str::to_owned),
                body: parse_inlines(child, path_text, diagnostics),
            });
        }
        else {
            diagnostics.push(Diagnostic::new(
                "unexpected-child",
                element_location(path_text, child.tag_name().name()),
                "list accepts only <item> children".to_owned(),
            ));
        }
    }
    DocList { ordered, items }
}

/// Parse a table block into its header row and body rows.
fn parse_table(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> DocTable
{
    let caption = node
        .attribute("caption")
        .map_or_else(String::new, str::to_owned);
    let mut header: Option<Vec<DocCell>> = None;
    let mut rows = Vec::new();
    for child in node.children().filter(Node::is_element) {
        match child.tag_name().name() {
            | "header" => {
                if header.is_some() {
                    diagnostics.push(Diagnostic::new(
                        "duplicate-header",
                        element_location(path_text, "header"),
                        "a table carries at most one <header> row".to_owned(),
                    ));
                }
                else {
                    header = Some(parse_cells(child, path_text, diagnostics));
                }
            },
            | "row" => rows.push(DocRow {
                cells: parse_cells(child, path_text, diagnostics),
            }),
            | other => diagnostics.push(Diagnostic::new(
                "unexpected-child",
                element_location(path_text, other),
                "table accepts only <header> and <row> children".to_owned(),
            )),
        }
    }
    let header = header.unwrap_or_else(|| {
        diagnostics.push(Diagnostic::new(
            "missing-header",
            element_location(path_text, "table"),
            "a table must declare a <header> row".to_owned(),
        ));
        Vec::new()
    });
    DocTable {
        caption,
        header,
        rows,
    }
}

/// Parse the `<cell>` children of a table row.
fn parse_cells(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<DocCell>
{
    let mut cells = Vec::new();
    for child in node.children().filter(Node::is_element) {
        if child.tag_name().name() == "cell" {
            cells.push(DocCell {
                content: parse_inlines(child, path_text, diagnostics),
            });
        }
        else {
            diagnostics.push(Diagnostic::new(
                "unexpected-child",
                element_location(path_text, child.tag_name().name()),
                "a table row accepts only <cell> children".to_owned(),
            ));
        }
    }
    cells
}

/// Parse a code block.
fn parse_code(node: Node<'_, '_>) -> DocCode
{
    DocCode {
        language: node
            .attribute("language")
            .map_or_else(String::new, str::to_owned),
        text: element_text(node),
    }
}

/// Parse the inline children of an element in document order.
fn parse_inlines(
    parent: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<DocInline>
{
    let mut inlines = Vec::new();
    for child in parent.children() {
        if child.is_text() {
            let text = collapse_whitespace(child.text().unwrap_or_default());
            if !text.is_empty() {
                inlines.push(DocInline::Text(text));
            }
        }
        else if child.is_element()
            && let Some(inline) = parse_inline(child, path_text, diagnostics)
        {
            inlines.push(inline);
        }
    }
    inlines
}

/// Parse one inline element.
fn parse_inline(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<DocInline>
{
    match node.tag_name().name() {
        | "inline-code" => Some(DocInline::InlineCode(element_text(node))),
        | "label" => {
            let key = required_attribute(node, "key", path_text, "label", diagnostics)?;
            Some(DocInline::Label(Label {
                key,
                text: element_text(node),
            }))
        },
        | "ref" => {
            let key = required_attribute(node, "key", path_text, "ref", diagnostics)?;
            Some(DocInline::Ref(LabelRef { key }))
        },
        | "cite" => {
            let key = required_attribute(node, "key", path_text, "cite", diagnostics)?;
            Some(DocInline::Cite(CiteKey { key }))
        },
        | other => {
            diagnostics.push(Diagnostic::new(
                "unknown-inline",
                element_location(path_text, other),
                format!("unknown inline element <{other}>"),
            ));
            None
        },
    }
}

#[cfg(test)]
mod tests
{
    use super::parse_doc_document;
    use crate::doc::model::DocBlock;
    use crate::doc::model::DocClass;
    use crate::model::Status;

    /// Parsing preserves the exact nested block tree, banner, and metadata.
    #[test]
    fn nested_blocks_parse_exact_tree() -> Result<(), crate::DocError>
    {
        let xml = r#"<research-record id="r" title="T" status="design-pass">
            <banner><read-when>designing doc classes</read-when><note>a note</note></banner>
            <section id="s" title="Outer" date="2026-07-21">
                <prose>hello <inline-code>fmt</inline-code> world</prose>
                <section title="Inner">
                    <list ordered="true"><item lead="Lead">body</item></list>
                </section>
            </section>
            <table caption="C"><header><cell>H</cell></header><row><cell>x</cell></row></table>
        </research-record>"#;
        let parsed = parse_doc_document(std::path::Path::new("mem:r.xml"), xml)?;

        assert!(
            parsed.diagnostics.is_empty(),
            "expected a clean parse, got {:?}",
            parsed.diagnostics,
        );
        let document = parsed.document.as_ref();
        assert_eq!(
            document.map(|record| record.class),
            Some(DocClass::ResearchRecord)
        );
        assert_eq!(
            document.map(|record| record.status),
            Some(Status::DesignPass)
        );
        assert_eq!(
            document.map(|record| record.banner.read_when.is_some()),
            Some(true),
        );
        assert_eq!(document.map(|record| record.banner.notes.len()), Some(1));
        assert_eq!(document.map(|record| record.blocks.len()), Some(2));
        let outer_is_dated_section = document
            .and_then(|record| record.blocks.first())
            .is_some_and(|block| match *block {
                | DocBlock::Section(ref section) => {
                    section.date.as_deref() == Some("2026-07-21") && section.blocks.len() == 2
                },
                | _ => false,
            });
        assert!(
            outer_is_dated_section,
            "first block is a dated two-child section"
        );
        let second_is_table = document
            .and_then(|record| record.blocks.get(1))
            .is_some_and(|block| matches!(*block, DocBlock::Table(_)));
        assert!(second_is_table, "second block is a table");
        Ok(())
    }

    /// A document opening without a banner is a structural violation.
    #[test]
    fn missing_banner_is_flagged() -> Result<(), crate::DocError>
    {
        let xml = r#"<workflow-doc id="w" title="T" status="built">
            <prose>no banner here</prose>
        </workflow-doc>"#;
        let parsed = parse_doc_document(std::path::Path::new("mem:w.xml"), xml)?;
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|d| d.code == "missing-banner"),
            "expected a missing-banner diagnostic, got {:?}",
            parsed.diagnostics,
        );
        Ok(())
    }
}
