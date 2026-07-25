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

/// Borrowed `XML` source accepted by the canonical formatter.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct XmlSource<'source>(&'source str);

impl<'source> From<&'source str> for XmlSource<'source>
{
    #[inline]
    fn from(source: &'source str) -> Self
    {
        Self(source)
    }
}

/// Whether formatting replaced a file's contents.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileChanged(bool);

impl From<FileChanged> for bool
{
    #[inline]
    fn from(changed: FileChanged) -> Self
    {
        changed.0
    }
}

/// Nesting depth of one canonical `XML` element.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ElementDepth(usize);

impl ElementDepth
{
    /// Return the child-element depth.
    #[must_use]
    fn child(self) -> Self
    {
        Self(self.0.saturating_add(1))
    }

    /// Render canonical two-space indentation.
    #[must_use]
    fn indent(self) -> String
    {
        "  ".repeat(self.0)
    }
}

/// Whether an element has no block-form children.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LeafElement(bool);

impl From<LeafElement> for bool
{
    #[inline]
    fn from(leaf: LeafElement) -> Self
    {
        leaf.0
    }
}

/// Text content escaped for `XML`.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct XmlText<'text>(&'text str);

impl<'text> From<&'text str> for XmlText<'text>
{
    #[inline]
    fn from(text: &'text str) -> Self
    {
        Self(text)
    }
}

/// Attribute content escaped for `XML`.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct XmlAttribute<'text>(&'text str);

impl<'text> From<&'text str> for XmlAttribute<'text>
{
    #[inline]
    fn from(value: &'text str) -> Self
    {
        Self(value)
    }
}

/// Format `XML` text into the canonical form.
///
/// # Errors
/// Returns [`DocError::Xml`] when the input is not well-formed.
#[inline]
pub fn format_xml(
    path: &Path,
    xml: XmlSource<'_>,
) -> Result<String, DocError>
{
    let tree = XmlDocument::parse(xml.0).map_err(|error| DocError::Xml {
        path: path.to_path_buf(),
        detail: error.to_string(),
    })?;
    Ok(write_element(tree.root_element(), ElementDepth::default()))
}

/// Format a file in place, returning whether its contents changed.
///
/// # Errors
/// Returns [`DocError::Io`] on a read or write failure and [`DocError::Xml`]
/// when the file is not well-formed.
#[inline]
pub fn format_file(path: &Path) -> Result<FileChanged, DocError>
{
    let original = std::fs::read_to_string(path).map_err(|source| DocError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let formatted = format_xml(path, original.as_str().into())?;
    if formatted == original {
        return Ok(FileChanged(false));
    }
    std::fs::write(path, &formatted).map_err(|source| DocError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(FileChanged(true))
}

/// Render one element (and its subtree) in canonical form.
fn write_element(
    node: Node<'_, '_>,
    depth: ElementDepth,
) -> String
{
    let indent = depth.indent();
    let name = node.tag_name().name();
    let open = format!("<{name}{}", attributes(node));
    if bool::from(is_leaf(node)) {
        let text = leaf_text(node);
        return if text.is_empty() {
            format!("{indent}{open}/>\n")
        }
        else {
            format!(
                "{indent}{open}>{}</{name}>\n",
                escape_text(text.as_str().into())
            )
        };
    }
    let child_depth = depth.child();
    let children: String = node
        .children()
        .map(|child| render_child(child, child_depth))
        .collect();
    format!("{indent}{open}>\n{children}{indent}</{name}>\n")
}

/// Render one child node within a block-formatted parent.
fn render_child(
    child: Node<'_, '_>,
    child_depth: ElementDepth,
) -> String
{
    let child_indent = child_depth.indent();
    match child.node_type() {
        | NodeType::Element => write_element(child, child_depth),
        | NodeType::Text => {
            let text = child.text().unwrap_or_default().trim();
            if text.is_empty() {
                String::new()
            }
            else {
                format!("{child_indent}{}\n", escape_text(text.into()))
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
fn is_leaf(node: Node<'_, '_>) -> LeafElement
{
    LeafElement(
        !node
            .children()
            .any(|child| matches!(child.node_type(), NodeType::Element | NodeType::Comment)),
    )
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
        .map(|pair| format!(" {}=\"{}\"", pair.0, escape_attr(pair.1.into())))
        .collect::<Vec<String>>()
        .concat()
}

/// Escape the reserved characters of `XML` text content.
fn escape_text(text: XmlText<'_>) -> String
{
    let mut out = String::with_capacity(text.0.len());
    for character in text.0.chars() {
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
fn escape_attr(value: XmlAttribute<'_>) -> String
{
    let mut out = String::with_capacity(value.0.len());
    for character in value.0.chars() {
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
        let once = format_xml(path, input.into())?;
        let twice = format_xml(path, once.as_str().into())?;
        assert_eq!(once, twice, "canonical formatting must be idempotent");
        assert!(
            once.contains(r#"<component id="c" spec-version="1" status="partial" title="T">"#),
            "attributes must be sorted alphabetically, got:\n{once}",
        );
        Ok(())
    }
}
