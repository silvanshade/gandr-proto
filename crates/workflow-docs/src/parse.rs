//! Parse-equals-validate pass: `XML` text to a [`Document`] with structural
//! diagnostics.
//!
//! Well-formedness failures are operational ([`DocError::Xml`]); structural
//! violations (wrong root, missing required attribute, unknown element,
//! malformed status) are [`Diagnostic`] values. A file that yields any
//! structural diagnostic produces no [`Document`], so the run fails on it.

use alloc::string::String;
use alloc::vec::Vec;
use std::path::Path;

use roxmltree::Document as XmlDocument;
use roxmltree::Node;

use crate::Diagnostic;
use crate::DocError;
use crate::model::Block;
use crate::model::CiteKey;
use crate::model::CodeBlock;
use crate::model::Definition;
use crate::model::Diagram;
use crate::model::Document;
use crate::model::Example;
use crate::model::Inline;
use crate::model::Judgements;
use crate::model::Math;
use crate::model::MathDisplay;
use crate::model::Production;
use crate::model::Rule;
use crate::model::Section;
use crate::model::Status;

/// Outcome of parsing one file: an optional model and any structural
/// diagnostics.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Parsed
{
    /// Parsed component, absent when a structural diagnostic prevented it.
    pub document: Option<Document>,
    /// Structural diagnostics discovered while parsing.
    pub diagnostics: Vec<Diagnostic>,
}

/// Parse one component file into a model plus structural diagnostics.
///
/// # Errors
/// Returns [`DocError::Xml`] when the input is not well-formed `XML`.
#[inline]
pub fn parse_document(
    path: &Path,
    xml: &str,
) -> Result<Parsed, DocError>
{
    let tree = XmlDocument::parse(xml).map_err(|error| DocError::Xml {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    let path_text = path.display().to_string();
    let mut diagnostics = Vec::new();
    let document = parse_component(tree.root_element(), &path_text, &mut diagnostics);
    Ok(Parsed {
        document,
        diagnostics,
    })
}

/// Parse the root component element.
fn parse_component(
    root: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Document>
{
    let location = element_location(path_text, "component");
    if root.tag_name().name() != "component" {
        diagnostics.push(Diagnostic::new(
            "unknown-root",
            location,
            format!(
                "root element must be <component>, found <{}>",
                root.tag_name().name()
            ),
        ));
        return None;
    }
    let id = required_attribute(root, "id", path_text, "component", diagnostics)?;
    let spec_version =
        required_attribute(root, "spec-version", path_text, "component", diagnostics)?;
    let title = required_attribute(root, "title", path_text, "component", diagnostics)?;
    let status = required_status(root, path_text, "component", diagnostics)?;
    let blocks = parse_blocks(root, path_text, diagnostics);
    Some(Document {
        id,
        spec_version,
        title,
        status,
        grounds: token_list(root.attribute("grounds")),
        derives: token_list(root.attribute("derives")),
        blocks,
        source_path: path_text.to_owned(),
    })
}

/// Parse the block children of an element in document order.
fn parse_blocks(
    parent: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Block>
{
    let mut blocks = Vec::new();
    for child in parent.children().filter(Node::is_element) {
        if let Some(block) = parse_block(child, path_text, diagnostics) {
            blocks.push(block);
        }
    }
    blocks
}

/// Parse one block element.
fn parse_block(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Block>
{
    let name = node.tag_name().name();
    match name {
        | "section" => Some(Block::Section(parse_section(node, path_text, diagnostics))),
        | "prose" => Some(Block::Prose(parse_inlines(node, path_text, diagnostics))),
        | "judgements" => Some(Block::Judgements(parse_judgements(
            node,
            path_text,
            diagnostics,
        ))),
        | "grammar" => Some(Block::Grammar(parse_grammar(node, path_text, diagnostics))),
        | "rule" => parse_rule(node, path_text, diagnostics).map(Block::Rule),
        | "definition" => Some(Block::Definition(parse_definition(
            node,
            path_text,
            diagnostics,
        ))),
        | "diagram" => parse_diagram(node, path_text, diagnostics).map(Block::Diagram),
        | "code" => Some(Block::Code(parse_code(node, path_text, diagnostics))),
        | "example" => Some(Block::Example(parse_example(node, path_text, diagnostics))),
        | "references" => Some(Block::References(parse_references(node))),
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

/// Parse a section block.
fn parse_section(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Section
{
    let title = node
        .attribute("title")
        .map_or_else(String::new, str::to_owned);
    Section {
        id: node.attribute("id").map(str::to_owned),
        status: optional_status(node, path_text, "section", diagnostics),
        title,
        blocks: parse_blocks(node, path_text, diagnostics),
    }
}

/// Parse a judgements block.
fn parse_judgements(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Judgements
{
    let title = node
        .attribute("title")
        .map_or_else(String::new, str::to_owned);
    let mut forms = Vec::new();
    for child in node.children().filter(Node::is_element) {
        if child.tag_name().name() == "math" {
            forms.push(Math {
                source: element_text(child),
                display: MathDisplay::Block,
            });
        }
        else {
            diagnostics.push(Diagnostic::new(
                "unexpected-child",
                element_location(path_text, child.tag_name().name()),
                "judgements accepts only <math> children".to_owned(),
            ));
        }
    }
    Judgements { title, forms }
}

/// Parse a grammar block.
fn parse_grammar(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Production>
{
    let mut productions = Vec::new();
    for child in node.children().filter(Node::is_element) {
        if child.tag_name().name() == "production" {
            let symbol = child
                .attribute("symbol")
                .map_or_else(String::new, str::to_owned);
            productions.push(Production {
                symbol,
                definition: element_text(child),
            });
        }
        else {
            diagnostics.push(Diagnostic::new(
                "unexpected-child",
                element_location(path_text, child.tag_name().name()),
                "grammar accepts only <production> children".to_owned(),
            ));
        }
    }
    productions
}

/// Parse a rule block.
fn parse_rule(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Rule>
{
    let name = required_attribute(node, "name", path_text, "rule", diagnostics)?;
    let mut premises = Vec::new();
    let mut conclusion: Option<Math> = None;
    for child in node.children().filter(Node::is_element) {
        match child.tag_name().name() {
            | "premise" => premises.push(Math {
                source: element_text(child),
                display: MathDisplay::Block,
            }),
            | "conclusion" => {
                conclusion = Some(Math {
                    source: element_text(child),
                    display: MathDisplay::Block,
                });
            },
            | other => diagnostics.push(Diagnostic::new(
                "unexpected-child",
                element_location(path_text, other),
                "rule accepts only <premise> and <conclusion> children".to_owned(),
            )),
        }
    }
    let Some(conclusion) = conclusion
    else {
        diagnostics.push(Diagnostic::new(
            "missing-conclusion",
            element_location(path_text, "rule"),
            format!("rule {name} has no <conclusion>"),
        ));
        return None;
    };
    Some(Rule {
        id: node.attribute("id").map(str::to_owned),
        name,
        premises,
        conclusion,
    })
}

/// Parse a definition block.
fn parse_definition(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Definition
{
    let term = node
        .attribute("term")
        .map_or_else(String::new, str::to_owned);
    Definition {
        id: node.attribute("id").map(str::to_owned),
        term,
        body: parse_inlines(node, path_text, diagnostics),
    }
}

/// Parse a diagram block.
fn parse_diagram(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Diagram>
{
    let id = required_attribute(node, "id", path_text, "diagram", diagnostics)?;
    let caption = node
        .attribute("caption")
        .map_or_else(String::new, str::to_owned);
    Some(Diagram {
        id,
        caption,
        source: element_text(node),
        cites: cite_list(node.attribute("cites")),
    })
}

/// Parse a code block.
fn parse_code(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> CodeBlock
{
    let language = node
        .attribute("language")
        .map_or_else(String::new, str::to_owned);
    let expect_output = node.attribute("expect-output").map(str::to_owned);
    let expect_error = node.attribute("expect-error").map(str::to_owned);
    if let (Some(_), Some(_)) = (expect_output.as_ref(), expect_error.as_ref()) {
        diagnostics.push(Diagnostic::new(
            "conflicting-expectation",
            element_location(path_text, "code"),
            "code declares both expect-output and expect-error".to_owned(),
        ));
    }
    CodeBlock {
        language,
        text: element_text(node),
        expect_output,
        expect_error,
    }
}

/// Parse an example block.
fn parse_example(
    node: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Example
{
    let title = node
        .attribute("title")
        .map_or_else(String::new, str::to_owned);
    Example {
        title,
        blocks: parse_blocks(node, path_text, diagnostics),
    }
}

/// Parse a references block into declared cite keys.
fn parse_references(node: Node<'_, '_>) -> Vec<CiteKey>
{
    let mut keys = Vec::new();
    for child in node.children().filter(Node::is_element) {
        if child.tag_name().name() == "cite"
            && let Some(key) = child.attribute("key")
        {
            keys.push(CiteKey {
                key: key.to_owned(),
            });
        }
    }
    keys
}

/// Parse the inline children of an element in document order.
fn parse_inlines(
    parent: Node<'_, '_>,
    path_text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Inline>
{
    let mut inlines = Vec::new();
    for child in parent.children() {
        if child.is_text() {
            let text = collapse_whitespace(child.text().unwrap_or_default());
            if !text.is_empty() {
                inlines.push(Inline::Text(text));
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
) -> Option<Inline>
{
    match node.tag_name().name() {
        | "term-def" => {
            let key = required_attribute(node, "key", path_text, "term-def", diagnostics)?;
            Some(Inline::TermDef(crate::model::TermDef {
                key,
                text: element_text(node),
            }))
        },
        | "term" => {
            let key = required_attribute(node, "key", path_text, "term", diagnostics)?;
            Some(Inline::TermRef(crate::model::TermRef { key }))
        },
        | "cite" => {
            let key = required_attribute(node, "key", path_text, "cite", diagnostics)?;
            Some(Inline::Cite(CiteKey { key }))
        },
        | "math" => Some(Inline::Math(Math {
            source: element_text(node),
            display: MathDisplay::Inline,
        })),
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

/// Read a required attribute, pushing a diagnostic when it is absent.
fn required_attribute(
    node: Node<'_, '_>,
    name: &str,
    path_text: &str,
    element: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String>
{
    match node.attribute(name) {
        | Some(value) => Some(value.to_owned()),
        | None => {
            diagnostics.push(Diagnostic::new(
                "missing-attribute",
                element_location(path_text, element),
                format!("<{element}> is missing required attribute {name}"),
            ));
            None
        },
    }
}

/// Read the required status attribute, pushing a diagnostic when absent or
/// malformed.
fn required_status(
    node: Node<'_, '_>,
    path_text: &str,
    element: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Status>
{
    let raw = required_attribute(node, "status", path_text, element, diagnostics)?;
    parse_status_value(&raw, path_text, element, diagnostics)
}

/// Read an optional status attribute, pushing a diagnostic when malformed.
fn optional_status(
    node: Node<'_, '_>,
    path_text: &str,
    element: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Status>
{
    let raw = node.attribute("status")?;
    parse_status_value(raw, path_text, element, diagnostics)
}

/// Parse a status value, pushing a diagnostic when it is not canonical.
fn parse_status_value(
    raw: &str,
    path_text: &str,
    element: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Status>
{
    match Status::parse(raw) {
        | Some(status) => Some(status),
        | None => {
            let allowed = Status::spellings().to_vec();
            diagnostics.push(Diagnostic::new(
                "invalid-status",
                element_location(path_text, element),
                format!("status '{raw}' is not one of [{}]", allowed.join(", ")),
            ));
            None
        },
    }
}

/// Concatenate the descendant text of an element, trimming outer whitespace.
fn element_text(node: Node<'_, '_>) -> String
{
    let mut text = String::new();
    for descendant in node.descendants().filter(Node::is_text) {
        text.push_str(descendant.text().unwrap_or_default());
    }
    text.trim().to_owned()
}

/// Collapse internal whitespace runs to single spaces, preserving boundary
/// spacing.
fn collapse_whitespace(input: &str) -> String
{
    let core: String = input.split_whitespace().collect::<Vec<&str>>().join(" ");
    if core.is_empty() {
        return core;
    }
    let lead = input.starts_with(char::is_whitespace);
    let trail = input.ends_with(char::is_whitespace);
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

/// Split a whitespace-delimited attribute into owned tokens.
fn token_list(value: Option<&str>) -> Vec<String>
{
    value.map_or_else(Vec::new, |text| {
        text.split_whitespace().map(str::to_owned).collect()
    })
}

/// Split a whitespace-delimited cite attribute into cite keys.
fn cite_list(value: Option<&str>) -> Vec<CiteKey>
{
    value.map_or_else(Vec::new, |text| {
        text.split_whitespace()
            .map(|key| CiteKey {
                key: key.to_owned(),
            })
            .collect()
    })
}

/// Build a diagnostic location string from a path and element name.
fn element_location(
    path_text: &str,
    element: &str,
) -> String
{
    format!("{path_text}:<{element}>")
}
