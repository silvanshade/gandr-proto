//! Typed model of the component vocabulary: the normative schema.
//!
//! Every element of the custom `XML` vocabulary has a Rust type here. Parsing
//! (see [`crate::parse`]) is the only constructor path, so a value of
//! [`Document`] is a structurally well-formed component; corpus-level semantic
//! checks live in [`crate::validate`].

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
/// The taxonomy is the seeded block set of decision `gandr-fcw.8`: section,
/// prose, judgements, grammar, rule, definition, diagram, code, example, and
/// references.
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
    /// A code block that structurally anticipates a checked example.
    Code(CodeBlock),
    /// A worked example: a titled grouping of nested blocks.
    Example(Example),
    /// A per-component bibliography: the cite keys it declares.
    References(Vec<CiteKey>),
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

/// A code block that structurally anticipates a checked example.
///
/// The `expect_output` and `expect_error` fields are validated for shape only
/// and never executed; execution gating is deferred until the interpreter runs.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CodeBlock
{
    /// Source language label.
    pub language: String,
    /// Verbatim code text.
    pub text: String,
    /// Optional anticipated standard output.
    pub expect_output: Option<String>,
    /// Optional anticipated error.
    pub expect_error: Option<String>,
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
