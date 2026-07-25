//! Typed model of the prose document classes: the normative schema for the
//! research-record, workflow-doc, and crate-status families.
//!
//! These classes share one minimal block/inline substrate and differ only by
//! their root element, their banner obligation, and the block set each class
//! admits (see [`crate::doc::validate`]). Parsing (see [`crate::doc::parse`])
//! is the only constructor path, so a value of [`DocRecord`] is a structurally
//! well-formed document; the class-schema and cross-reference checks live in
//! [`crate::doc::validate`].
//!
//! The five lifecycle spellings are reused verbatim from the component model
//! ([`crate::model::Status`]); a document's machine status is one of them and a
//! missing status fails validation, exactly as for a component.
//!
//! [`DocRecord`]: crate::doc::model::DocRecord

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use crate::model::CiteKey;
use crate::model::Status;

/// The prose document class a file belongs to, fixed by its root element.
///
/// The class selects the banner obligation and the admitted block set; the
/// substrate below the root is shared.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum DocClass
{
    /// A `docs/research/` design study or staging record (`<research-record>`).
    ResearchRecord,
    /// A `docs/workflow/` process or convention document (`<workflow-doc>`).
    WorkflowDoc,
    /// A per-crate `docs/STATUS.xml` lean-tier status narrative
    /// (`<crate-status>`).
    CrateStatus,
}

impl DocClass
{
    /// Every document class, in root-element declaration order.
    pub const ALL: [Self; 3] = [Self::ResearchRecord, Self::WorkflowDoc, Self::CrateStatus];
}

impl AsRef<str> for DocClass
{
    #[inline]
    fn as_ref(&self) -> &'static str
    {
        match *self {
            | Self::ResearchRecord => "research-record",
            | Self::WorkflowDoc => "workflow-doc",
            | Self::CrateStatus => "crate-status",
        }
    }
}

impl fmt::Display for DocClass
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(self.as_ref())
    }
}

impl FromStr for DocClass
{
    type Err = UnknownDocClass;

    #[inline]
    fn from_str(root: &str) -> Result<Self, Self::Err>
    {
        match root {
            | "research-record" => Ok(Self::ResearchRecord),
            | "workflow-doc" => Ok(Self::WorkflowDoc),
            | "crate-status" => Ok(Self::CrateStatus),
            | _ => Err(UnknownDocClass),
        }
    }
}

/// Rejection returned when a root element does not name a document class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct UnknownDocClass;

/// A prose document: the class-tagged root element of one file.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DocRecord
{
    /// The document class fixed by the root element.
    pub class: DocClass,
    /// Corpus-unique document identifier.
    pub id: String,
    /// Human-readable document title.
    pub title: String,
    /// Required lifecycle status (shared with the component vocabulary).
    pub status: Status,
    /// Crate scope, required for and only meaningful to `crate-status`.
    pub crate_scope: Option<String>,
    /// The leading banner (read-when / provenance / scope metadata).
    pub banner: Banner,
    /// Ordered top-level blocks.
    pub blocks: Vec<DocBlock>,
    /// Source path this document was parsed from (for diagnostics).
    pub source_path: String,
}

/// The leading metadata banner every prose document opens with.
///
/// The banner is the structured form of the Markdown lead blockquote: an
/// optional `read-when` orientation line (required for workflow docs) plus a
/// sequence of free-prose notes (status phrase, scope, provenance).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct Banner
{
    /// The `read-when` orientation line, when present.
    pub read_when: Option<Vec<DocInline>>,
    /// Free-prose banner notes in document order.
    pub notes: Vec<Vec<DocInline>>,
}

impl Banner
{
    /// Whether the banner carries no orientation line and no notes.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> BannerEmpty
    {
        BannerEmpty(self.read_when.is_none() && self.notes.is_empty())
    }
}

/// Whether a document banner carries no content.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BannerEmpty(bool);

impl From<BannerEmpty> for bool
{
    #[inline]
    fn from(value: BannerEmpty) -> Self
    {
        value.0
    }
}

/// A block-level element of the prose substrate.
///
/// The taxonomy is the minimal shared set the three classes need: a nesting
/// section, a prose paragraph, a flat list, a table, and a code listing. It
/// omits the component vocabulary's math-bearing blocks (judgements, grammar,
/// rule, diagram) by design — prose documents carry no typeset leaves.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DocBlock
{
    /// A titled, optionally identified and dated grouping of nested blocks.
    Section(DocSection),
    /// A paragraph of inline content.
    Prose(Vec<DocInline>),
    /// A flat (non-nested) ordered or unordered list.
    List(DocList),
    /// A header-plus-body table.
    Table(DocTable),
    /// A verbatim code listing.
    Code(DocCode),
}

impl DocBlock
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
    ///   of section-nesting depth (only sections nest).
    ///
    /// # Adequacy
    ///
    /// - hypothesis: L3 pointwise — the exact projected visit stream
    ///   distinguishes missing, duplicated, or reordered root/section/leaf
    ///   nodes.
    /// - witness: `crate::doc::validate::tests::nested_sections_validate_in_order`.
    pub(crate) fn walk(&self) -> impl Iterator<Item = &Self>
    {
        let mut pending = alloc::vec![self];
        core::iter::from_fn(move || {
            let block = pending.pop()?;
            match *block {
                | Self::Section(ref section) => pending.extend(section.blocks.iter().rev()),
                | Self::Prose(_) | Self::List(_) | Self::Table(_) | Self::Code(_) => {},
            }
            Some(block)
        })
    }
}

/// A titled grouping of nested blocks.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DocSection
{
    /// Optional corpus-unique identifier.
    pub id: Option<String>,
    /// Optional date stamp (the dated-section shape of a changelog entry).
    pub date: Option<String>,
    /// Section title.
    pub title: String,
    /// Nested blocks.
    pub blocks: Vec<DocBlock>,
}

/// A flat ordered or unordered list.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DocList
{
    /// Whether the list is a numbered (ordered) sequence.
    pub ordered: bool,
    /// The list items in document order.
    pub items: Vec<DocItem>,
}

/// A single list item: an optional bold lead-in plus inline body content.
///
/// The optional `lead` models the rule-statement convention of the workflow
/// docs (a bold lead-in naming the rule, then its prose body).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DocItem
{
    /// Optional bold lead-in naming the item (the rule-statement head).
    pub lead: Option<String>,
    /// Inline body content.
    pub body: Vec<DocInline>,
}

/// A header-plus-body table.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DocTable
{
    /// Table caption metadata.
    pub caption: String,
    /// The header row.
    pub header: Vec<DocCell>,
    /// The body rows in document order.
    pub rows: Vec<DocRow>,
}

/// A single table body row.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DocRow(Vec<DocCell>);

impl AsRef<[DocCell]> for DocRow
{
    #[inline]
    fn as_ref(&self) -> &[DocCell]
    {
        &self.0
    }
}

impl From<Vec<DocCell>> for DocRow
{
    #[inline]
    fn from(cells: Vec<DocCell>) -> Self
    {
        Self(cells)
    }
}

/// A single table cell of inline content.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DocCell(Vec<DocInline>);

impl AsRef<[DocInline]> for DocCell
{
    #[inline]
    fn as_ref(&self) -> &[DocInline]
    {
        &self.0
    }
}

impl From<Vec<DocInline>> for DocCell
{
    #[inline]
    fn from(content: Vec<DocInline>) -> Self
    {
        Self(content)
    }
}

/// A verbatim code listing.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DocCode
{
    /// Source language label.
    pub language: String,
    /// Verbatim code text.
    pub text: String,
}

/// An inline element within prose, a list item, a table cell, or a banner line.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DocInline
{
    /// Literal text.
    Text(String),
    /// An inline code span (a backtick-style identifier or path).
    InlineCode(String),
    /// A coined-label anchor definition (define-once within the document).
    Label(Label),
    /// A reference to a coined label defined in the same document.
    Ref(LabelRef),
    /// A register-key citation, resolved against the references file.
    Cite(CiteKey),
}

/// A coined-label anchor definition (define-once within the document).
///
/// Labels are the research-record recommendation/hazard/question anchors (`R1`,
/// `HZ-1`, `O1`): a keyed anchor coined once and referenced elsewhere by key.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct Label
{
    /// Label key, unique within the document.
    pub key: String,
    /// Displayed anchor text.
    pub text: String,
}

/// A reference to a coined label defined elsewhere in the same document.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct LabelRef(String);

impl AsRef<str> for LabelRef
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        &self.0
    }
}

impl From<String> for LabelRef
{
    #[inline]
    fn from(key: String) -> Self
    {
        Self(key)
    }
}

impl From<&str> for LabelRef
{
    #[inline]
    fn from(key: &str) -> Self
    {
        Self(key.to_owned())
    }
}
