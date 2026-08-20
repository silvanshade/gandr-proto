//! Serde types for the LSP subset this face serves.
//!
//! The wrappers keep crate-defined signatures off bare primitives. JSON
//! images stay the protocol's numbers and strings.

use alloc::string::String;
use alloc::vec::Vec;

use gandr_surface_render_remote::diagnostic::DiagnosticCode;
use serde::Deserialize;
use serde::Serialize;

/// A zero-based LSP line.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct LineNumber(u32);

impl From<u32> for LineNumber
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<LineNumber> for u32
{
    #[inline]
    fn from(value: LineNumber) -> Self
    {
        value.0
    }
}

/// A zero-based character offset in the negotiated encoding.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct CharacterOffset(u32);

impl From<u32> for CharacterOffset
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<CharacterOffset> for u32
{
    #[inline]
    fn from(value: CharacterOffset) -> Self
    {
        value.0
    }
}

/// One unsigned integer in the semantic-token stream.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct TokenUnit(u32);

impl From<u32> for TokenUnit
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<TokenUnit> for u32
{
    #[inline]
    fn from(value: TokenUnit) -> Self
    {
        value.0
    }
}

/// An LSP position in the negotiated encoding.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position
{
    /// Zero-based line.
    pub line: LineNumber,
    /// Zero-based character offset on that line.
    pub character: CharacterOffset,
}

impl Position
{
    /// A position from its line and character.
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn new(
        line: LineNumber,
        character: CharacterOffset,
    ) -> Self
    {
        Self { line, character }
    }
}

/// A half-open range of positions.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Range
{
    /// Inclusive start.
    pub start: Position,
    /// Exclusive end.
    pub end: Position,
}

impl Range
{
    /// A range from its endpoints.
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn new(
        start: Position,
        end: Position,
    ) -> Self
    {
        Self { start, end }
    }
}

/// A document URI as the editor spelled it.
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct DocumentUri(String);

impl AsRef<str> for DocumentUri
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0.as_str()
    }
}

impl From<String> for DocumentUri
{
    #[inline]
    fn from(value: String) -> Self
    {
        Self(value)
    }
}

/// Text document item on `didOpen`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem
{
    /// Document identity.
    pub uri: DocumentUri,
    /// Full buffer text.
    pub text: String,
}

/// Versioned document identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[repr(transparent)]
pub struct VersionedTextDocumentIdentifier
{
    /// Document identity.
    pub uri: DocumentUri,
}

/// Text document identity without a version.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(transparent)]
pub struct TextDocumentIdentifier
{
    /// Document identity.
    pub uri: DocumentUri,
}

/// One full-document change (this face advertises full sync only).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(transparent)]
pub struct TextDocumentContentChangeEvent
{
    /// Replacement text for the whole buffer under full sync.
    pub text: String,
}

/// `textDocument/didOpen` params.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[repr(transparent)]
pub struct DidOpenTextDocumentParams
{
    /// The opened document.
    pub text_document: TextDocumentItem,
}

/// `textDocument/didChange` params.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DidChangeTextDocumentParams
{
    /// The changed document.
    pub text_document: VersionedTextDocumentIdentifier,
    /// Content changes; this face uses the last full-text change.
    pub content_changes: Vec<TextDocumentContentChangeEvent>,
}

/// `textDocument/didClose` params.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[repr(transparent)]
pub struct DidCloseTextDocumentParams
{
    /// The closed document.
    pub text_document: TextDocumentIdentifier,
}

/// `textDocument/hover` and `textDocument/completion` params.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentPositionParams
{
    /// The document.
    pub text_document: TextDocumentIdentifier,
    /// The cursor.
    pub position: Position,
}

/// `textDocument/semanticTokens/full` params.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[repr(transparent)]
pub struct SemanticTokensParams
{
    /// The document.
    pub text_document: TextDocumentIdentifier,
}

/// Semantic-token payload.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[repr(transparent)]
pub struct SemanticTokens
{
    /// Delta-encoded integer stream.
    pub data: Vec<TokenUnit>,
}

/// Hover contents as a plain Markdown string.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(transparent)]
pub struct Hover
{
    /// Rendered hover body.
    pub contents: MarkupContent,
}

/// A Markup content block.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MarkupContent
{
    /// Markup kind; this face always sends Markdown.
    pub kind: String,
    /// Markup body.
    pub value: String,
}

impl MarkupContent
{
    /// Markdown markup from `value`.
    ///
    /// # Contract
    /// - ensures: `kind` is `markdown` and `value` is stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn markdown(value: String) -> Self
    {
        Self {
            kind: String::from("markdown"),
            value,
        }
    }
}

/// One completion item.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompletionItem
{
    /// Inserted label.
    pub label: String,
    /// Optional type or kind note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// A publishDiagnostics notification payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PublishDiagnosticsParams
{
    /// Document identity.
    pub uri: DocumentUri,
    /// Current diagnostics for that document.
    pub diagnostics: Vec<Diagnostic>,
}

/// One editor diagnostic.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic
{
    /// Source range.
    pub range: Range,
    /// Stable diagnostic identifier.
    pub code: DiagnosticCode,
    /// Severity: 1 error, 2 warning.
    pub severity: DiagnosticSeverity,
    /// Human message.
    pub message: String,
}

/// LSP diagnostic severity.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub struct DiagnosticSeverity(u32);

impl DiagnosticSeverity
{
    /// Error.
    pub const ERROR: Self = Self(1);
    /// Warning.
    pub const WARNING: Self = Self(2);
}

/// Initialize params, only the fields this face reads.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[repr(transparent)]
pub struct InitializeParams
{
    /// Client capabilities, when sent.
    #[serde(default)]
    pub capabilities: ClientCapabilities,
}

/// Client capability bag, only the fields this face reads.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[repr(transparent)]
pub struct ClientCapabilities
{
    /// General capabilities, when sent.
    #[serde(default)]
    pub general: Option<GeneralClientCapabilities>,
}

/// General client capabilities.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[repr(transparent)]
pub struct GeneralClientCapabilities
{
    /// Position encodings the client accepts, most preferred first.
    #[serde(default)]
    pub position_encodings: Option<Vec<String>>,
}
