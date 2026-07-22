//! Typed model of the component vocabulary: the normative schema.
//!
//! Every element of the custom `XML` vocabulary has a Rust type here. Parsing
//! (see [`crate::parse`]) is the only constructor path, so a value of
//! [`Document`] is a structurally well-formed component; corpus-level semantic
//! checks live in [`crate::validate`].
//!
//! [`Document`]: crate::model::Document

use alloc::string::String;
use alloc::vec::Vec;

/// Lifecycle status carried by every component (and optionally by a section).
///
/// The five values are the single document class of decision `gandr-fcw.8`
/// (component-owned status); a missing status fails validation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Status
{
    /// Implemented and matches the specification.
    Built,
    /// Partially implemented.
    Partial,
    /// Decided and specified, not yet implemented.
    AdoptedUnbuilt,
    /// A design pass exists; not yet adopted.
    DesignPass,
    /// Retired or parked; retained for provenance.
    Dormant,
}

impl Status
{
    /// Parse a status from its canonical attribute spelling.
    #[inline]
    #[must_use]
    pub fn parse(text: &str) -> Option<Self>
    {
        match text {
            | "built" => Some(Self::Built),
            | "partial" => Some(Self::Partial),
            | "adopted-unbuilt" => Some(Self::AdoptedUnbuilt),
            | "design-pass" => Some(Self::DesignPass),
            | "dormant" => Some(Self::Dormant),
            | _ => None,
        }
    }

    /// Return the canonical attribute spelling of the status.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str
    {
        match self {
            | Self::Built => "built",
            | Self::Partial => "partial",
            | Self::AdoptedUnbuilt => "adopted-unbuilt",
            | Self::DesignPass => "design-pass",
            | Self::Dormant => "dormant",
        }
    }

    /// Return every canonical status spelling, for diagnostics.
    #[inline]
    #[must_use]
    pub const fn spellings() -> [&'static str; 5]
    {
        [
            "built",
            "partial",
            "adopted-unbuilt",
            "design-pass",
            "dormant",
        ]
    }
}

/// A specification component: the versioned root element of one file.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Document
{
    /// Corpus-unique component identifier.
    pub id: String,
    /// Root schema version (the versioned root element).
    pub spec_version: String,
    /// Human-readable component title.
    pub title: String,
    /// Required lifecycle status.
    pub status: Status,
    /// Provenance edges: component identifiers this component is grounded in.
    pub grounds: Vec<String>,
    /// Provenance edges: component identifiers this component derives from.
    pub derives: Vec<String>,
    /// Ordered top-level blocks.
    pub blocks: Vec<Block>,
    /// Source path this component was parsed from (for diagnostics).
    pub source_path: String,
}

/// A block-level element of the vocabulary.
///
/// The taxonomy is the seeded block set of decision `gandr-fcw.8` — section,
/// prose, judgements, grammar, rule, definition, diagram, code, example, and
/// references — extended by the `gandr-fid.3` payload blocks (list, table)
/// that mirror the prose-substrate shapes so the two vocabularies author
/// identically.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Block
{
    /// A titled, optionally identified and status-attributed grouping of
    /// nested blocks.
    Section(Section),
    /// A paragraph of inline content.
    Prose(Vec<Inline>),
    /// A titled collection of judgement-form math leaves.
    Judgements(Judgements),
    /// A grammar block of productions.
    Grammar(Vec<Production>),
    /// A single inference rule (premises over a conclusion).
    Rule(Rule),
    /// A titled definition with inline body content.
    Definition(Definition),
    /// A commutative (or other) diagram compiled from a typst leaf.
    Diagram(Diagram),
    /// A code block: a checked-example candidate or a normative API surface.
    Code(CodeBlock),
    /// A worked example: a titled grouping of nested blocks.
    Example(Example),
    /// A flat (non-nested) ordered or unordered list of inline items.
    List(List),
    /// A captioned header-plus-body table of inline cells.
    Table(Table),
    /// A per-component bibliography: the cite keys it declares.
    References(Vec<CiteKey>),
}

impl Block
{
    /// Visit this block and every nested block in document order.
    ///
    /// # Contract
    ///
    /// - ensures: yields the root followed by every descendant exactly once in
    ///   depth-first document order.
    /// - provides: a borrowing iterator over the complete block tree.
    /// - panics: none.
    /// - intension: reverse child scheduling on an explicit LIFO worklist
    ///   preserves source order while bounding native stack use independently
    ///   of tree depth.
    ///
    /// # Adequacy
    ///
    /// - hypothesis: L3 pointwise — the exact projected visit stream
    ///   distinguishes missing, duplicated, or reordered root/container/leaf
    ///   nodes.
    /// - witness: `block_walk_preserves_document_order`.
    pub(crate) fn walk(&self) -> impl Iterator<Item = &Self>
    {
        let mut pending = alloc::vec![self];
        core::iter::from_fn(move || {
            let block = pending.pop()?;
            match *block {
                | Self::Section(ref section) => pending.extend(section.blocks.iter().rev()),
                | Self::Example(ref example) => pending.extend(example.blocks.iter().rev()),
                | Self::Prose(_)
                | Self::Judgements(_)
                | Self::Grammar(_)
                | Self::Rule(_)
                | Self::Definition(_)
                | Self::Diagram(_)
                | Self::Code(_)
                | Self::List(_)
                | Self::Table(_)
                | Self::References(_) => {},
            }
            Some(block)
        })
    }
}

/// A titled grouping of nested blocks.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Section
{
    /// Optional corpus-unique identifier.
    pub id: Option<String>,
    /// Optional section-level status override.
    pub status: Option<Status>,
    /// Section title.
    pub title: String,
    /// Nested blocks.
    pub blocks: Vec<Block>,
}

/// A titled collection of judgement-form math leaves.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Judgements
{
    /// Title of the judgement collection.
    pub title: String,
    /// Ordered judgement-form math leaves.
    pub forms: Vec<Math>,
}

/// A single grammar production.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Production
{
    /// Nonterminal symbol being defined.
    pub symbol: String,
    /// Right-hand side text of the production.
    pub definition: String,
}

/// A single inference rule.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Rule
{
    /// Optional corpus-unique identifier.
    pub id: Option<String>,
    /// Rule name (rendered as the rule label).
    pub name: String,
    /// Premise math leaves (the numerator of the rule tree).
    pub premises: Vec<Math>,
    /// Conclusion math leaf (the denominator of the rule tree).
    pub conclusion: Math,
}

/// A titled definition with inline body content.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Definition
{
    /// Optional corpus-unique identifier.
    pub id: Option<String>,
    /// The term being defined (rendered as the definition title).
    pub term: String,
    /// Inline body content.
    pub body: Vec<Inline>,
}

/// A diagram compiled from an opaque typst leaf.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Diagram
{
    /// Corpus-unique diagram identifier.
    pub id: String,
    /// Caption metadata.
    pub caption: String,
    /// Opaque typst leaf source (the only unvalidated region).
    pub source: String,
    /// Cite keys the diagram references.
    pub cites: Vec<CiteKey>,
}

/// A code block: a checked-example candidate or a normative API surface.
///
/// The `expect_output` and `expect_error` fields are validated for shape only
/// and never executed; execution gating is deferred until the interpreter runs.
/// An [`CodeRole::Api`] block is a normative signature listing and never an
/// execution candidate, so it excludes the expectation attributes.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CodeBlock
{
    /// Source language label.
    pub language: String,
    /// The block's role: checked-example candidate (default) or API surface.
    pub role: CodeRole,
    /// Verbatim code text.
    pub text: String,
    /// Optional anticipated standard output.
    pub expect_output: Option<String>,
    /// Optional anticipated error.
    pub expect_error: Option<String>,
}

/// The role of a code block.
///
/// The `gandr-fid.0` two-register weave makes normative API surfaces (typed
/// signatures, data layouts) first-class payload; the role keeps them
/// distinguishable from checked-example candidates so execution gating never
/// picks them up.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum CodeRole
{
    /// A checked-example candidate (execution gating deferred). The default.
    #[default]
    Example,
    /// A normative API-surface listing; never executed.
    Api,
}

impl CodeRole
{
    /// Parse a role from its canonical attribute spelling.
    #[inline]
    #[must_use]
    pub fn parse(text: &str) -> Option<Self>
    {
        match text {
            | "example" => Some(Self::Example),
            | "api" => Some(Self::Api),
            | _ => None,
        }
    }

    /// Return the canonical attribute spelling of the role.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> &'static str
    {
        match self {
            | Self::Example => "example",
            | Self::Api => "api",
        }
    }
}

/// A worked example: a titled grouping of nested blocks.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Example
{
    /// Example title.
    pub title: String,
    /// Nested blocks.
    pub blocks: Vec<Block>,
}

/// A flat ordered or unordered list of inline items.
///
/// The shape mirrors the prose substrate's list so the two vocabularies
/// author identically; component list items carry the full component inline
/// set (terms, cites, refs, math).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct List
{
    /// Whether the list is a numbered (ordered) sequence.
    pub ordered: bool,
    /// The list items in document order.
    pub items: Vec<ListItem>,
}

/// A single list item: an optional bold lead-in plus inline body content.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ListItem
{
    /// Optional bold lead-in naming the item.
    pub lead: Option<String>,
    /// Inline body content.
    pub body: Vec<Inline>,
}

/// A captioned header-plus-body table of inline cells.
///
/// The shape mirrors the prose substrate's table; cells carry the full
/// component inline set, so dictionary and decision tables keep their term
/// links and citations.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Table
{
    /// Table caption metadata.
    pub caption: String,
    /// The header row.
    pub header: Vec<TableCell>,
    /// The body rows in document order.
    pub rows: Vec<TableRow>,
}

/// A single table body row.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct TableRow
{
    /// The cells of this row in column order.
    pub cells: Vec<TableCell>,
}

/// A single table cell of inline content.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct TableCell
{
    /// Inline cell content.
    pub content: Vec<Inline>,
}

/// An inline element within prose or a definition body.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Inline
{
    /// Literal text.
    Text(String),
    /// An inline term definition (define-once corpus-wide).
    TermDef(TermDef),
    /// A reference to a defined term.
    TermRef(TermRef),
    /// A cite-key reference.
    Cite(CiteKey),
    /// A cross-reference to a corpus anchor (a component, section, rule,
    /// definition, or diagram id).
    Ref(AnchorRef),
    /// An inline math leaf.
    Math(Math),
}

/// An inline term definition (define-once corpus-wide).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct TermDef
{
    /// Term key, unique across the corpus.
    pub key: String,
    /// Displayed definition text.
    pub text: String,
}

/// A reference to a term defined elsewhere in the corpus.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct TermRef
{
    /// Referenced term key.
    pub key: String,
}

/// A cross-reference to a corpus anchor, resolved against declared ids.
///
/// The target is any corpus-unique identifier (component, section, rule,
/// definition, or diagram); the validator rejects an unresolvable target.
/// This is the section-granular link the `gandr-fid.0` weave relies on in
/// place of prose-flattened anchors.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AnchorRef
{
    /// Referenced anchor identifier.
    pub target: String,
}

/// A cite-key reference resolved against the references file.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct CiteKey
{
    /// Bibliography key, resolved against the hayagriva references file.
    pub key: String,
}

/// A math leaf carrying typst source.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Math
{
    /// Opaque typst math source (the house prelude is prepended by the tool).
    pub source: String,
    /// Whether the leaf renders inline or as a display block.
    pub display: MathDisplay,
}

/// Whether a math leaf renders inline with text or as a display block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MathDisplay
{
    /// Rendered inline with surrounding text.
    Inline,
    /// Rendered as a centered display block.
    Block,
}

#[cfg(test)]
mod tests
{
    use super::Block;
    use super::Definition;
    use super::Example;
    use super::Section;

    /// Nested traversal visits each root, container, and leaf exactly once.
    #[test]
    fn block_walk_preserves_document_order()
    {
        let definition = |term: &str| {
            Block::Definition(Definition {
                id: None,
                term: term.to_owned(),
                body: Vec::new(),
            })
        };
        let root = Block::Section(Section {
            id: None,
            status: None,
            title: "root".to_owned(),
            blocks: alloc::vec![
                definition("first"),
                Block::Example(Example {
                    title: "nested".to_owned(),
                    blocks: alloc::vec![definition("second")],
                }),
                definition("third"),
            ],
        });

        let visits = root
            .walk()
            .map(|block| match *block {
                | Block::Section(ref section) => ("section", section.title.as_str()),
                | Block::Prose(_) => ("prose", ""),
                | Block::Judgements(ref judgements) => ("judgements", judgements.title.as_str()),
                | Block::Grammar(_) => ("grammar", ""),
                | Block::Rule(ref rule) => ("rule", rule.name.as_str()),
                | Block::Definition(ref definition) => ("definition", definition.term.as_str()),
                | Block::Diagram(ref diagram) => ("diagram", diagram.id.as_str()),
                | Block::Code(ref code) => ("code", code.language.as_str()),
                | Block::Example(ref example) => ("example", example.title.as_str()),
                | Block::List(_) => ("list", ""),
                | Block::Table(ref table) => ("table", table.caption.as_str()),
                | Block::References(_) => ("references", ""),
            })
            .collect::<Vec<_>>();

        assert_eq!(visits, [
            ("section", "root"),
            ("definition", "first"),
            ("example", "nested"),
            ("definition", "second"),
            ("definition", "third"),
        ],);
    }
}
