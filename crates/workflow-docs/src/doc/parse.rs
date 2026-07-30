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
use core::str::FromStr as _;
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
use crate::model::Status;

/// Outcome of parsing one file: an optional model and any structural
/// diagnostics.
#[derive(Clone, Debug)]
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

/// Borrowed `XML` source for one prose document.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct DocumentXml<'source>(&'source str);

impl<'source> From<&'source str> for DocumentXml<'source>
{
    #[inline]
    fn from(source: &'source str) -> Self
    {
        Self(source)
    }
}

/// Shared source location and diagnostic sink for one parse.
struct ParseContext<'parse>
{
    /// Path rendered in diagnostics.
    path: &'parse str,
    /// Accumulated structural diagnostics.
    diagnostics: &'parse mut Vec<Diagnostic>,
}

/// One `XML` attribute name.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct AttributeName<'name>(&'name str);

impl<'name> From<&'name str> for AttributeName<'name>
{
    #[inline]
    fn from(name: &'name str) -> Self
    {
        Self(name)
    }
}

/// One `XML` element name.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ElementName<'name>(&'name str);

impl<'name> From<&'name str> for ElementName<'name>
{
    #[inline]
    fn from(name: &'name str) -> Self
    {
        Self(name)
    }
}

/// Raw lifecycle-status text at the parsing seam.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct StatusText<'text>(&'text str);

impl<'text> From<&'text str> for StatusText<'text>
{
    #[inline]
    fn from(text: &'text str) -> Self
    {
        Self(text)
    }
}

/// Raw prose text awaiting whitespace normalization.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ProseText<'text>(&'text str);

impl<'text> From<&'text str> for ProseText<'text>
{
    #[inline]
    fn from(text: &'text str) -> Self
    {
        Self(text)
    }
}

/// Parse one prose-document file into a model plus structural diagnostics.
///
/// # Errors
/// Returns [`DocError::Xml`] when the input is not well-formed `XML`.
#[inline]
pub fn parse_doc_document(
    path: &Path,
    xml: DocumentXml<'_>,
) -> Result<DocParsed, DocError>
{
    let tree = XmlDocument::parse(xml.0).map_err(|error| DocError::Xml {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    let path_text = path.display().to_string();
    let mut diagnostics = Vec::new();
    let document = {
        let mut context = ParseContext {
            path: path_text.as_str(),
            diagnostics: &mut diagnostics,
        };
        parse_record(tree.root_element(), &mut context)
    };
    Ok(DocParsed {
        document,
        diagnostics,
    })
}

/// Parse the class-tagged root element.
fn parse_record(
    root: Node<'_, '_>,
    context: &mut ParseContext<'_>,
) -> Option<DocRecord>
{
    let name = root.tag_name().name();
    let Ok(class) = DocClass::from_str(name)
    else {
        context.diagnostics.push(Diagnostic::new(
            "unknown-root".into(),
            element_location(context, ElementName::from(name)),
            format!(
                "root element must be one of [{}], found <{name}>",
                DocClass::ALL.map(|class| class.to_string()).join(", ")
            ),
        ));
        return None;
    };
    let root_name = class.as_ref();
    let id = required_attribute(
        root,
        AttributeName::from("id"),
        ElementName::from(root_name),
        context,
    )?;
    let title = required_attribute(
        root,
        AttributeName::from("title"),
        ElementName::from(root_name),
        context,
    )?;
    let status = required_status(root, ElementName::from(root_name), context)?;
    let crate_scope = match class {
        | DocClass::CrateStatus => required_attribute(
            root,
            AttributeName::from("crate"),
            ElementName::from(root_name),
            context,
        ),
        | DocClass::ResearchRecord | DocClass::WorkflowDoc => None,
    };
    let (banner, block_nodes) = split_banner(root, context);
    let blocks = parse_blocks(block_nodes, context);
    Some(DocRecord {
        class,
        id,
        title,
        status,
        crate_scope,
        banner,
        blocks,
        source_path: context.path.to_owned(),
    })
}

/// Split the leading `<banner>` from the block children, flagging a missing or
/// misplaced banner.
fn split_banner<'tree, 'input>(
    root: Node<'tree, 'input>,
    context: &mut ParseContext<'_>,
) -> (Banner, Vec<Node<'tree, 'input>>)
{
    let children: Vec<Node<'tree, 'input>> = root.children().filter(Node::is_element).collect();
    let mut banner = Banner::default();
    let mut seen_banner = false;
    let mut block_nodes = Vec::new();
    for (index, child) in children.iter().enumerate() {
        if child.tag_name().name() == "banner" {
            if index == 0 && !seen_banner {
                banner = parse_banner(*child, context);
                seen_banner = true;
            }
            else {
                context.diagnostics.push(Diagnostic::new(
                    "misplaced-banner".into(),
                    element_location(context, ElementName::from("banner")),
                    "the <banner> must be the first and only banner child".to_owned(),
                ));
            }
        }
        else {
            block_nodes.push(*child);
        }
    }
    if !seen_banner {
        context.diagnostics.push(Diagnostic::new(
            "missing-banner".into(),
            element_location(context, ElementName::from("banner")),
            "a prose document must open with a <banner> element".to_owned(),
        ));
    }
    (banner, block_nodes)
}

/// Parse a banner element into its orientation line and free notes.
fn parse_banner(
    node: Node<'_, '_>,
    context: &mut ParseContext<'_>,
) -> Banner
{
    let mut banner = Banner::default();
    for child in node.children().filter(Node::is_element) {
        match child.tag_name().name() {
            | "read-when" => {
                if banner.read_when.is_some() {
                    context.diagnostics.push(Diagnostic::new(
                        "duplicate-read-when".into(),
                        element_location(context, ElementName::from("read-when")),
                        "a banner carries at most one <read-when> line".to_owned(),
                    ));
                }
                else {
                    banner.read_when = Some(parse_inlines(child, context));
                }
            },
            | "note" => banner.notes.push(parse_inlines(child, context)),
            | other => context.diagnostics.push(Diagnostic::new(
                "unexpected-child".into(),
                element_location(context, ElementName::from(other)),
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
    context: &mut ParseContext<'_>,
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
                let title = required_attribute(
                    node,
                    AttributeName::from("title"),
                    ElementName::from("section"),
                    context,
                )
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
            else if let Some(block) = parse_leaf_block(node, context) {
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
    context: &mut ParseContext<'_>,
) -> Option<DocBlock>
{
    let name = node.tag_name().name();
    match name {
        | "prose" => Some(DocBlock::Prose(parse_inlines(node, context))),
        | "list" => Some(DocBlock::List(parse_list(node, context))),
        | "table" => Some(DocBlock::Table(parse_table(node, context))),
        | "code" => Some(DocBlock::Code(parse_code(node))),
        | other => {
            context.diagnostics.push(Diagnostic::new(
                "unknown-block".into(),
                element_location(context, ElementName::from(other)),
                format!("unknown block element <{other}>"),
            ));
            None
        },
    }
}

/// Parse a list block into its items.
fn parse_list(
    node: Node<'_, '_>,
    context: &mut ParseContext<'_>,
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
                body: parse_inlines(child, context),
            });
        }
        else {
            context.diagnostics.push(Diagnostic::new(
                "unexpected-child".into(),
                element_location(context, ElementName::from(child.tag_name().name())),
                "list accepts only <item> children".to_owned(),
            ));
        }
    }
    DocList { ordered, items }
}

/// Parse a table block into its header row and body rows.
fn parse_table(
    node: Node<'_, '_>,
    context: &mut ParseContext<'_>,
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
                    context.diagnostics.push(Diagnostic::new(
                        "duplicate-header".into(),
                        element_location(context, ElementName::from("header")),
                        "a table carries at most one <header> row".to_owned(),
                    ));
                }
                else {
                    header = Some(parse_cells(child, context));
                }
            },
            | "row" => rows.push(DocRow::from(parse_cells(child, context))),
            | other => context.diagnostics.push(Diagnostic::new(
                "unexpected-child".into(),
                element_location(context, ElementName::from(other)),
                "table accepts only <header> and <row> children".to_owned(),
            )),
        }
    }
    let header = header.unwrap_or_else(|| {
        context.diagnostics.push(Diagnostic::new(
            "missing-header".into(),
            element_location(context, ElementName::from("table")),
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
    context: &mut ParseContext<'_>,
) -> Vec<DocCell>
{
    let mut cells = Vec::new();
    for child in node.children().filter(Node::is_element) {
        if child.tag_name().name() == "cell" {
            cells.push(DocCell::from(parse_inlines(child, context)));
        }
        else {
            context.diagnostics.push(Diagnostic::new(
                "unexpected-child".into(),
                element_location(context, ElementName::from(child.tag_name().name())),
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
    context: &mut ParseContext<'_>,
) -> Vec<DocInline>
{
    let mut inlines = Vec::new();
    for child in parent.children() {
        if child.is_text() {
            let text = collapse_whitespace(ProseText::from(child.text().unwrap_or_default()));
            if !text.is_empty() {
                inlines.push(DocInline::Text(text));
            }
        }
        else if child.is_element()
            && let Some(inline) = parse_inline(child, context)
        {
            inlines.push(inline);
        }
    }
    inlines
}

/// Parse one inline element.
fn parse_inline(
    node: Node<'_, '_>,
    context: &mut ParseContext<'_>,
) -> Option<DocInline>
{
    match node.tag_name().name() {
        | "inline-code" => Some(DocInline::InlineCode(element_text(node))),
        | "label" => {
            let key = required_attribute(
                node,
                AttributeName::from("key"),
                ElementName::from("label"),
                context,
            )?;
            Some(DocInline::Label(Label {
                key,
                text: element_text(node),
            }))
        },
        | "ref" => {
            let key = required_attribute(
                node,
                AttributeName::from("key"),
                ElementName::from("ref"),
                context,
            )?;
            Some(DocInline::Ref(LabelRef::from(key)))
        },
        | "cite" => {
            let key = required_attribute(
                node,
                AttributeName::from("key"),
                ElementName::from("cite"),
                context,
            )?;
            Some(DocInline::Cite(CiteKey::from(key)))
        },
        | other => {
            context.diagnostics.push(Diagnostic::new(
                "unknown-inline".into(),
                element_location(context, ElementName::from(other)),
                format!("unknown inline element <{other}>"),
            ));
            None
        },
    }
}

// ── shared leaf helpers (the component parser's discipline, kept for the
// classes when the component parser retired with the XML corpus) ────────────

/// Read a required attribute, pushing a diagnostic when it is absent.
fn required_attribute(
    node: Node<'_, '_>,
    name: AttributeName<'_>,
    element: ElementName<'_>,
    context: &mut ParseContext<'_>,
) -> Option<String>
{
    match node.attribute(name.0) {
        | Some(value) => Some(value.to_owned()),
        | None => {
            context.diagnostics.push(Diagnostic::new(
                "missing-attribute".into(),
                element_location(context, element),
                format!("<{}> is missing required attribute {}", element.0, name.0),
            ));
            None
        },
    }
}

/// Read the required status attribute, pushing a diagnostic when absent or
/// malformed.
fn required_status(
    node: Node<'_, '_>,
    element: ElementName<'_>,
    context: &mut ParseContext<'_>,
) -> Option<Status>
{
    let raw = required_attribute(node, "status".into(), element, context)?;
    parse_status_value(raw.as_str().into(), element, context)
}

/// Parse a status value, pushing a diagnostic when it is not canonical.
fn parse_status_value(
    raw: StatusText<'_>,
    element: ElementName<'_>,
    context: &mut ParseContext<'_>,
) -> Option<Status>
{
    match Status::from_str(raw.0) {
        | Ok(status) => Some(status),
        | Err(_) => {
            let allowed = Status::ALL.map(|status| status.to_string());
            context.diagnostics.push(Diagnostic::new(
                "invalid-status".into(),
                element_location(context, element),
                format!("status '{}' is not one of [{}]", raw.0, allowed.join(", ")),
            ));
            None
        },
    }
}

/// Concatenate the descendant text of an element, trimming outer whitespace.
pub(crate) fn element_text(node: Node<'_, '_>) -> String
{
    let mut text = String::new();
    for descendant in node.descendants().filter(Node::is_text) {
        text.push_str(descendant.text().unwrap_or_default());
    }
    text.trim().to_owned()
}

/// Collapse internal whitespace runs to single spaces, preserving boundary
/// spacing.
fn collapse_whitespace(input: ProseText<'_>) -> String
{
    let core: String = input.0.split_whitespace().collect::<Vec<&str>>().join(" ");
    if core.is_empty() {
        return core;
    }
    let lead = input.0.starts_with(char::is_whitespace);
    let trail = input.0.ends_with(char::is_whitespace);
    let mut out = String::new();
    if lead {
        out.push(' ');
    }
    out.push_str(&core);
    if trail {
        out.push(' ');
    }
    out
}

/// Build a diagnostic location string from a path and element name.
fn element_location(
    context: &ParseContext<'_>,
    element: ElementName<'_>,
) -> String
{
    format!("{}:<{}>", context.path, element.0)
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
        let parsed = parse_doc_document(std::path::Path::new("mem:r.xml"), xml.into())?;

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
        let parsed = parse_doc_document(std::path::Path::new("mem:w.xml"), xml.into())?;
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
