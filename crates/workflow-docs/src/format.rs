//! Canonical `XML` formatting for the component vocabulary.
//!
//! Formatting is a round-trip through the read-only `XML` tree: attributes are
//! sorted, indentation is two spaces per level, leaf elements keep their
//! (trimmed) text inline, and elements with child elements are block-indented.
//! The transform is idempotent, which is what the `treefmt` check mode
//! requires. Well-formedness failures are [`DocError::Xml`].

use alloc::string::String;
use alloc::vec::Vec;
use std::path::Path;

use roxmltree::Document as XmlDocument;
use roxmltree::Node;
use roxmltree::NodeType;

use crate::DocError;

/// Format `XML` text into the canonical form.
///
/// # Errors
/// Returns [`DocError::Xml`] when the input is not well-formed.
#[inline]
pub fn format_xml(
    path: &Path,
    xml: &str,
) -> Result<String, DocError>
{
    let tree = XmlDocument::parse(xml).map_err(|error| DocError::Xml {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    Ok(write_element(tree.root_element(), 0usize))
}

/// Format a file in place, returning whether its contents changed.
///
/// # Errors
/// Returns [`DocError::Io`] on a read or write failure and [`DocError::Xml`]
/// when the file is not well-formed.
#[inline]
pub fn format_file(path: &Path) -> Result<bool, DocError>
{
    let original = std::fs::read_to_string(path).map_err(|source| DocError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let formatted = format_xml(path, &original)?;
    if formatted == original {
        return Ok(false);
    }
    std::fs::write(path, &formatted).map_err(|source| DocError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(true)
}

/// Render one element (and its subtree) in canonical form.
fn write_element(
    node: Node<'_, '_>,
    depth: usize,
) -> String
{
    let indent = "  ".repeat(depth);
    let name = node.tag_name().name();
    let open = format!("<{name}{}", attributes(node));
    if is_leaf(node) {
        let text = leaf_text(node);
        return if text.is_empty() {
            format!("{indent}{open}/>\n")
        }
        else {
            format!("{indent}{open}>{}</{name}>\n", escape_text(&text))
        };
    }
    let child_depth = depth.saturating_add(1);
    let child_indent = "  ".repeat(child_depth);
    let children: String = node
        .children()
        .map(|child| render_child(child, child_depth, &child_indent))
        .collect();
    format!("{indent}{open}>\n{children}{indent}</{name}>\n")
}

/// Render one child node within a block-formatted parent.
fn render_child(
    child: Node<'_, '_>,
    child_depth: usize,
    child_indent: &str,
) -> String
{
    match child.node_type() {
        | NodeType::Element => write_element(child, child_depth),
        | NodeType::Text => {
            let text = child.text().unwrap_or_default().trim();
            if text.is_empty() {
                String::new()
            }
            else {
                format!("{child_indent}{}\n", escape_text(text))
            }
        },
        | NodeType::Comment => {
            let comment = child.text().unwrap_or_default().trim();
            format!("{child_indent}<!-- {comment} -->\n")
        },
        | NodeType::Root | NodeType::PI => String::new(),
    }
}

/// An element is a leaf when it has no element or comment children.
fn is_leaf(node: Node<'_, '_>) -> bool
{
    !node
        .children()
        .any(|child| matches!(child.node_type(), NodeType::Element | NodeType::Comment))
}

/// Concatenate and trim the text of a leaf element.
fn leaf_text(node: Node<'_, '_>) -> String
{
    let mut text = String::new();
    for child in node.children().filter(Node::is_text) {
        text.push_str(child.text().unwrap_or_default());
    }
    text.trim().to_owned()
}

/// Render sorted attribute pairs, each prefixed with a single space.
fn attributes(node: Node<'_, '_>) -> String
{
    let mut pairs: Vec<(&str, &str)> = node
        .attributes()
        .map(|attribute| (attribute.name(), attribute.value()))
        .collect();
    pairs.sort_by(|left, right| left.0.cmp(right.0));
    pairs
        .iter()
        .map(|pair| format!(" {}=\"{}\"", pair.0, escape_attr(pair.1)))
        .collect::<Vec<String>>()
        .concat()
}

/// Escape the reserved characters of `XML` text content.
fn escape_text(text: &str) -> String
{
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            | '&' => out.push_str("&amp;"),
            | '<' => out.push_str("&lt;"),
            | '>' => out.push_str("&gt;"),
            | other => out.push(other),
        }
    }
    out
}

/// Escape the reserved characters of an `XML` attribute value.
fn escape_attr(value: &str) -> String
{
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            | '&' => out.push_str("&amp;"),
            | '<' => out.push_str("&lt;"),
            | '>' => out.push_str("&gt;"),
            | '"' => out.push_str("&quot;"),
            | other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests
{
    //! Canonical-formatting witnesses: attribute sorting and idempotency.

    use std::path::Path;

    use super::format_xml;

    /// Formatting sorts attributes and normalizes indentation, and a second
    /// pass is a fixed point.
    #[test]
    fn formatting_is_idempotent_and_sorts_attributes() -> Result<(), crate::DocError>
    {
        let path = Path::new("mem:demo.xml");
        let input = r#"<component status="partial" id="c" title="T" spec-version="1">
        <prose>hello <term key="x"/> world</prose></component>"#;
        let once = format_xml(path, input)?;
        let twice = format_xml(path, &once)?;
        assert_eq!(once, twice, "canonical formatting must be idempotent");
        assert!(
            once.contains(r#"<component id="c" spec-version="1" status="partial" title="T">"#),
            "attributes must be sorted alphabetically, got:\n{once}",
        );
        Ok(())
    }
}
