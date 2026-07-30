//! The `Send`-safe presentation seam, graduated from `gandr-tui::present`.
//!
//! These are the plain-data types the pipeline worker hands a renderer, plus
//! the byte-offset ↔ (row, column) projection the byte-span surface requires.
//!
//! The pipeline's semantic values (`Ty`, `Eval`, `Lowered`, …) are `Rc`-based
//! and must not leave the worker thread; everything here is plain data
//! (strings, ranges, enums), so it crosses a channel — or, once serialized, a
//! socket. `gandr-tui::present` re-exports these types unchanged; the render
//! bus ([`crate::wire`]) carries them as frame bodies. This is the seam the
//! inspection-protocol proposal calls "the in-process worker→present seam
//! promoted to a wire protocol"
//! (`docs/gandr/spec/proposal-inspection-protocol.md` §1).

use core::fmt;
use core::ops::Range;

/// Whether a marker denotes an error rather than the empty-hole tint.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct MarkIsError(bool);

impl From<bool> for MarkIsError
{
    #[inline]
    fn from(is_error: bool) -> Self
    {
        Self(is_error)
    }
}

impl From<MarkIsError> for bool
{
    #[inline]
    fn from(is_error: MarkIsError) -> Self
    {
        is_error.0
    }
}

/// Source text addressed by render byte offsets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct SourceText<'source>(&'source str);

impl<'source> From<&'source str> for SourceText<'source>
{
    #[inline]
    fn from(text: &'source str) -> Self
    {
        Self(text)
    }
}

impl<'source> From<SourceText<'source>> for &'source str
{
    #[inline]
    fn from(text: SourceText<'source>) -> Self
    {
        text.0
    }
}

/// A zero-based source row in the editor projection.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct PositionRow(usize);

impl From<usize> for PositionRow
{
    #[inline]
    fn from(row: usize) -> Self
    {
        Self(row)
    }
}

impl From<PositionRow> for usize
{
    #[inline]
    fn from(row: PositionRow) -> Self
    {
        row.0
    }
}

/// A zero-based character column within a source row.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct PositionColumn(usize);

impl From<usize> for PositionColumn
{
    #[inline]
    fn from(column: usize) -> Self
    {
        Self(column)
    }
}

impl From<PositionColumn> for usize
{
    #[inline]
    fn from(column: PositionColumn) -> Self
    {
        column.0
    }
}

/// A byte offset into a source document.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codecs", serde(transparent))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct ByteOffset(usize);

impl From<usize> for ByteOffset
{
    #[inline]
    fn from(byte: usize) -> Self
    {
        Self(byte)
    }
}

impl From<ByteOffset> for usize
{
    #[inline]
    fn from(byte: ByteOffset) -> Self
    {
        byte.0
    }
}

/// A byte-to-position projection failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct PosOfByteError
{
    /// The byte offset that landed inside a UTF-8 code point.
    byte: ByteOffset,
}

impl PosOfByteError
{
    /// The failure for a byte offset that lands inside a UTF-8 code point.
    ///
    /// # Contract
    /// - ensures: stores `byte` verbatim for [`Self::byte`].
    /// - panics: none.
    #[inline]
    #[must_use]
    fn interior_utf8_offset(byte: ByteOffset) -> Self
    {
        Self { byte }
    }

    /// The rejected byte offset.
    ///
    /// # Contract
    /// - ensures: returns the exact offset passed to [`pos_of_byte`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn byte(self) -> ByteOffset
    {
        self.byte
    }
}

impl fmt::Display for PosOfByteError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        write!(
            f,
            "byte offset {} is inside a UTF-8 code point",
            usize::from(self.byte)
        )
    }
}

impl core::error::Error for PosOfByteError
{
}

/// A semantic highlight role (the capture names of the grammar's
/// `highlights.scm`, folded to the classes a theme styles).
///
/// Graduated from `gandr-tui::highlight` so both the in-process TUI renderer
/// and a bus client project the same role vocabulary.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HlRole
{
    /// Language keywords.
    Keyword,
    /// Operators (term and shell redirection).
    Operator,
    /// A defined function name.
    FunctionDef,
    /// A called function or command name.
    FunctionCall,
    /// A defined value/binder name.
    VariableDef,
    /// A parameter binder.
    VariableParam,
    /// A record/co-record member.
    Member,
    /// A plain variable reference.
    Variable,
    /// A constructor.
    Constructor,
    /// A type identifier.
    Type,
    /// A builtin/primitive type.
    TypeBuiltin,
    /// A type variable.
    TypeVariable,
    /// A numeric literal (including grades and file descriptors).
    Number,
    /// A boolean literal.
    Boolean,
    /// A character literal.
    Character,
    /// A string literal.
    StringLit,
    /// An escape sequence inside a literal.
    Escape,
    /// A comment.
    Comment,
    /// A typed hole (`?` / `?name`).
    Hole,
    /// A label (session field, world, hole name).
    Label,
    /// A shell word / path.
    Path,
    /// A directive (shebang).
    Directive,
    /// Anything not otherwise classified.
    Other,
}

/// A syntax-highlight span: a byte range of the source classified by role.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HlSpan
{
    /// The classified byte range.
    pub range: Range<ByteOffset>,
    /// The highlight role.
    pub role: HlRole,
}

impl HlSpan
{
    /// A highlight span from its parts (the struct is `non_exhaustive`;
    /// external constructors — the highlighter, tests — come through here).
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        range: Range<ByteOffset>,
        role: HlRole,
    ) -> Self
    {
        Self { range, role }
    }
}

/// A Hazel-style mark span: a byte range decorated by the total marker.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkSpan
{
    /// The marked byte range.
    pub range: Range<ByteOffset>,
    /// Whether the mark is an error (`false` only for an empty hole).
    pub is_error: MarkIsError,
    /// The one-line mark description (already rendered).
    pub message: String,
}

impl MarkSpan
{
    /// A mark span from its parts (the struct is `non_exhaustive`; external
    /// constructors come through here).
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        range: Range<ByteOffset>,
        is_error: MarkIsError,
        message: String,
    ) -> Self
    {
        Self {
            range,
            is_error,
            message,
        }
    }
}

/// One diagnostic card: a fail-fast typing diagnostic with its partial
/// derivation.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagCard
{
    /// The primary message.
    pub message: String,
    /// The source byte span, when resolvable.
    pub span: Option<Range<ByteOffset>>,
    /// The offending expression, when recorded.
    pub expr: Option<String>,
    /// The elaboration note (synthesized-node provenance), when present.
    pub elaboration: Option<String>,
    /// The partial derivation, outermost first ("while checking …").
    pub chain: Vec<String>,
}

impl DiagCard
{
    /// A diagnostic card from its parts (the struct is `non_exhaustive`;
    /// external constructors come through here).
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        message: String,
        span: Option<Range<ByteOffset>>,
        expr: Option<String>,
        elaboration: Option<String>,
        chain: Vec<String>,
    ) -> Self
    {
        Self {
            message,
            span,
            expr,
            elaboration,
            chain,
        }
    }
}

/// One goal card: a hole with its expected type and local context.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoalCard
{
    /// The display label (`?0` or `?name`).
    pub label: String,
    /// What was elided (the hole note), rendered.
    pub note: String,
    /// The expected type at the hole, rendered, when typing reached it.
    pub expected: Option<String>,
    /// Local bindings beyond the session context (`name : type`), rendered,
    /// innermost shadowing applied.
    pub ctx: Vec<String>,
    /// The hole's byte range in the source.
    pub range: Range<ByteOffset>,
}

impl GoalCard
{
    /// A goal card from its parts (the struct is `non_exhaustive`; external
    /// constructors — tests, future renderers — come through here).
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        label: String,
        note: String,
        expected: Option<String>,
        ctx: Vec<String>,
        range: Range<ByteOffset>,
    ) -> Self
    {
        Self {
            label,
            note,
            expected,
            ctx,
            range,
        }
    }
}

/// The type information under the cursor.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CursorTy
{
    /// The synthesized type at the enclosing term, rendered.
    pub synthesized: Option<String>,
    /// The analyzed (checked-against) type at the enclosing term, rendered.
    pub analyzed: Option<String>,
}

impl CursorTy
{
    /// Cursor type info from its parts (the struct is `non_exhaustive`;
    /// external constructors come through here).
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        synthesized: Option<String>,
        analyzed: Option<String>,
    ) -> Self
    {
        Self {
            synthesized,
            analyzed,
        }
    }
}

/// One completion candidate.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Candidate
{
    /// The insertable name.
    pub name: String,
    /// The rendered type (or a kind note for keywords), shown dim.
    pub detail: String,
}

impl Candidate
{
    /// A candidate from its parts (the struct is `non_exhaustive`; external
    /// constructors come through here).
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        name: String,
        detail: String,
    ) -> Self
    {
        Self { name, detail }
    }
}

/// The per-preview presentation frame: everything the view needs from one
/// pure checking pass over the current buffer.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PreviewFrame
{
    /// The input generation this frame was computed for (stale frames are
    /// dropped by the app).
    pub generation: u64,
    /// Whether the buffer parses without `ERROR`/`MISSING` nodes (the
    /// submission gate; holes do NOT make a buffer incomplete).
    pub parse_complete: bool,
    /// Whether the depth guard rejected the buffer.
    pub too_deep: bool,
    /// Syntax-highlight spans over the buffer.
    pub highlights: Vec<HlSpan>,
    /// Mark spans (error underlines + hole tints).
    pub marks: Vec<MarkSpan>,
    /// Fail-fast diagnostics.
    pub diags: Vec<DiagCard>,
    /// Hole goals, in source order.
    pub goals: Vec<GoalCard>,
    /// Type under the cursor byte the preview was requested with.
    pub cursor: Option<CursorTy>,
    /// Completion candidates (session bindings; prelude and user definitions).
    pub bindings: Vec<Candidate>,
}

/// The kind of one rendered transcript line (styled by the theme at draw
/// time so theme switches restyle history).
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutKind
{
    /// An echoed source line.
    Source,
    /// A type line (`name : T` or `: T`).
    Type,
    /// An evaluated value line (`= v`).
    Value,
    /// A runtime-blame line (`! …`).
    Blame,
    /// A stuck-evaluation line (`· no value …`).
    Stuck,
    /// A diagnostic line.
    Diag,
    /// A goal line.
    Goal,
    /// An informational line (banner, meta-command output).
    Info,
}

/// One rendered block in the transcript: an echoed submission and its
/// rendered results.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TranscriptBlock
{
    /// The submitted source (echoed with highlighting).
    pub source: String,
    /// Highlight spans over [`Self::source`].
    pub source_hl: Vec<HlSpan>,
    /// The rendered result lines, in presentation order.
    pub lines: Vec<(OutKind, String)>,
}

impl TranscriptBlock
{
    /// A block from its parts (the struct is `non_exhaustive`; external
    /// constructors come through here).
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        source: String,
        source_hl: Vec<HlSpan>,
        lines: Vec<(OutKind, String)>,
    ) -> Self
    {
        Self {
            source,
            source_hl,
            lines,
        }
    }
}

/// The (row, column) position of a byte offset, both zero-based; the column
/// counts characters (the editor-widget convention), not bytes.
#[cfg_attr(feature = "codecs", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Pos
{
    /// The zero-based row.
    pub row: PositionRow,
    /// The zero-based character column within the row.
    pub col: PositionColumn,
}

impl Pos
{
    /// A position from its row and column (the struct is `non_exhaustive`;
    /// external constructors come through here).
    ///
    /// # Contract
    /// - ensures: fields are stored verbatim.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        row: PositionRow,
        col: PositionColumn,
    ) -> Self
    {
        Self { row, col }
    }
}

/// Converts a byte offset into a [`Pos`] over `text`.
///
/// # Contract
/// - ensures: returns the position of a valid UTF-8 boundary; an offset past
///   the end still clamps to the final position; newline bytes belong to the
///   line they terminate.
/// - fails: returns [`PosOfByteError`] when `byte` is inside a UTF-8 sequence.
/// - panics: none.
///
/// # Errors
///
/// Returns [`PosOfByteError`] when `byte` is inside a UTF-8 code point.
#[inline]
pub fn pos_of_byte(
    text: SourceText<'_>,
    byte: ByteOffset,
) -> Result<Pos, PosOfByteError>
{
    if byte.0 < text.0.len() && !text.0.is_char_boundary(byte.0) {
        return Err(PosOfByteError::interior_utf8_offset(byte));
    }

    let mut row = 0_usize;
    let mut col = 0_usize;
    for (offset, ch) in text.0.char_indices() {
        if offset >= byte.0 {
            return Ok(Pos {
                row: PositionRow(row),
                col: PositionColumn(col),
            });
        }
        if ch == '\n' {
            row = row.saturating_add(1);
            col = 0;
        }
        else {
            col = col.saturating_add(1);
        }
    }
    Ok(Pos {
        row: PositionRow(row),
        col: PositionColumn(col),
    })
}

/// Converts a [`Pos`] into a byte offset over `text`.
///
/// # Contract
/// - ensures: total — a position past the end of its row clamps to the row end;
///   a row past the end clamps to `text.len()`.
/// - panics: none.
#[inline]
#[must_use]
pub fn byte_of_pos(
    text: SourceText<'_>,
    pos: Pos,
) -> ByteOffset
{
    let mut row = 0_usize;
    let mut col = 0_usize;
    for (offset, ch) in text.0.char_indices() {
        if row == pos.row.0 && col == pos.col.0 {
            return ByteOffset(offset);
        }
        if ch == '\n' {
            if row == pos.row.0 {
                // The requested column overshoots this row: clamp to its end.
                return ByteOffset(offset);
            }
            row = row.saturating_add(1);
            col = 0;
        }
        else {
            col = col.saturating_add(1);
        }
    }
    ByteOffset(text.0.len())
}

#[cfg(test)]
mod tests
{
    use super::ByteOffset;
    use super::Candidate;
    use super::CursorTy;
    use super::DiagCard;
    use super::GoalCard;
    use super::HlRole;
    use super::HlSpan;
    use super::MarkIsError;
    use super::MarkSpan;
    use super::OutKind;
    use super::Pos;
    use super::PositionColumn;
    use super::PositionRow;
    use super::SourceText;
    use super::TranscriptBlock;
    use super::byte_of_pos;
    use super::pos_of_byte;

    #[test]
    fn constructors_store_fields_verbatim()
    {
        // Distinct field values so a field-swap in any constructor is caught.
        let hl = HlSpan::new(ByteOffset::from(3) .. ByteOffset::from(7), HlRole::Keyword);
        assert_eq!(ByteOffset::from(3) .. ByteOffset::from(7), hl.range);
        assert_eq!(HlRole::Keyword, hl.role);

        let mark = MarkSpan::new(
            ByteOffset::from(2) .. ByteOffset::from(5),
            MarkIsError::from(true),
            "unbound".to_owned(),
        );
        assert_eq!(ByteOffset::from(2) .. ByteOffset::from(5), mark.range);
        assert!(bool::from(mark.is_error));
        assert_eq!("unbound", mark.message);

        let diag = DiagCard::new(
            "type mismatch".to_owned(),
            Some(ByteOffset::from(1) .. ByteOffset::from(4)),
            Some("f x".to_owned()),
            Some("elaborated here".to_owned()),
            vec!["while checking f".to_owned(), "in application".to_owned()],
        );
        assert_eq!("type mismatch", diag.message);
        assert_eq!(Some(ByteOffset::from(1) .. ByteOffset::from(4)), diag.span);
        assert_eq!(Some("f x"), diag.expr.as_deref());
        assert_eq!(Some("elaborated here"), diag.elaboration.as_deref());
        assert_eq!(diag.chain, vec![
            "while checking f".to_owned(),
            "in application".to_owned(),
        ]);

        let goal = GoalCard::new(
            "?0".to_owned(),
            "hole note".to_owned(),
            Some("Nat".to_owned()),
            vec!["x : Nat".to_owned()],
            ByteOffset::from(10) .. ByteOffset::from(12),
        );
        assert_eq!("?0", goal.label);
        assert_eq!("hole note", goal.note);
        assert_eq!(Some("Nat"), goal.expected.as_deref());
        assert_eq!(goal.ctx, vec!["x : Nat".to_owned()]);
        assert_eq!(ByteOffset::from(10) .. ByteOffset::from(12), goal.range);

        let cursor = CursorTy::new(Some("Nat".to_owned()), Some("Int".to_owned()));
        assert_eq!(Some("Nat"), cursor.synthesized.as_deref());
        assert_eq!(Some("Int"), cursor.analyzed.as_deref());

        let cand = Candidate::new("map".to_owned(), "(a -> b) -> [a] -> [b]".to_owned());
        assert_eq!("map", cand.name);
        assert_eq!("(a -> b) -> [a] -> [b]", cand.detail);

        let block = TranscriptBlock::new(
            "def x = 1".to_owned(),
            vec![HlSpan::new(
                ByteOffset::from(0) .. ByteOffset::from(3),
                HlRole::Keyword,
            )],
            vec![(OutKind::Value, "= 1".to_owned())],
        );
        assert_eq!("def x = 1", block.source);
        assert_eq!(block.source_hl, vec![HlSpan::new(
            ByteOffset::from(0) .. ByteOffset::from(3),
            HlRole::Keyword,
        )]);
        assert_eq!(block.lines, vec![(OutKind::Value, "= 1".to_owned())]);

        // Pos::new keeps row and col unswapped.
        let pos = Pos::new(PositionRow::from(2), PositionColumn::from(5));
        assert_eq!(2, usize::from(pos.row));
        assert_eq!(5, usize::from(pos.col));
    }

    #[cfg(feature = "codecs")]
    #[test]
    fn byte_ranges_keep_the_existing_json_shape()
    {
        let hl = HlSpan::new(ByteOffset::from(3) .. ByteOffset::from(7), HlRole::Keyword);
        let hl_json = serde_json::to_value(&hl).expect("serialize highlight");
        assert_eq!(
            serde_json::json!({
                "range": {"start": 3_usize, "end": 7_usize},
                "role": "Keyword",
            }),
            hl_json
        );
        let decoded_hl: HlSpan = serde_json::from_value(hl_json).expect("deserialize highlight");
        assert_eq!(hl, decoded_hl);

        let mark = MarkSpan::new(
            ByteOffset::from(2) .. ByteOffset::from(5),
            MarkIsError::from(true),
            "unbound".to_owned(),
        );
        let mark_json = serde_json::to_value(&mark).expect("serialize mark");
        assert_eq!(
            serde_json::json!({
                "range": {"start": 2_usize, "end": 5_usize},
                "is_error": true,
                "message": "unbound",
            }),
            mark_json
        );
        let decoded_mark: MarkSpan = serde_json::from_value(mark_json).expect("deserialize mark");
        assert_eq!(mark, decoded_mark);

        let diag = DiagCard::new(
            "type mismatch".to_owned(),
            Some(ByteOffset::from(1) .. ByteOffset::from(4)),
            Some("f x".to_owned()),
            Some("elaborated here".to_owned()),
            vec!["while checking f".to_owned(), "in application".to_owned()],
        );
        let diag_json = serde_json::to_value(&diag).expect("serialize diagnostic");
        assert_eq!(
            serde_json::json!({
                "message": "type mismatch",
                "span": {"start": 1_usize, "end": 4_usize},
                "expr": "f x",
                "elaboration": "elaborated here",
                "chain": ["while checking f", "in application"],
            }),
            diag_json
        );
        let decoded_diag: DiagCard =
            serde_json::from_value(diag_json).expect("deserialize diagnostic");
        assert_eq!(diag, decoded_diag);

        let goal = GoalCard::new(
            "?0".to_owned(),
            "hole note".to_owned(),
            Some("Nat".to_owned()),
            vec!["x : Nat".to_owned()],
            ByteOffset::from(10) .. ByteOffset::from(12),
        );
        let goal_json = serde_json::to_value(&goal).expect("serialize goal");
        assert_eq!(
            serde_json::json!({
                "label": "?0",
                "note": "hole note",
                "expected": "Nat",
                "ctx": ["x : Nat"],
                "range": {"start": 10_usize, "end": 12_usize},
            }),
            goal_json
        );
        let decoded_goal: GoalCard = serde_json::from_value(goal_json).expect("deserialize goal");
        assert_eq!(goal, decoded_goal);
    }

    #[test]
    fn positions_round_trip_on_ascii()
    {
        let text = "def x = 1;\ndef y = 2;";
        for (byte, _) in text.char_indices() {
            let pos = pos_of_byte(SourceText::from(text), ByteOffset::from(byte))
                .expect("ASCII offsets are UTF-8 boundaries");
            assert_eq!(
                ByteOffset::from(byte),
                byte_of_pos(SourceText::from(text), pos),
                "byte {byte} round-trips"
            );
        }
    }

    #[test]
    fn positions_count_characters_not_bytes()
    {
        let text = "é×é\nz";
        // Bytes: é=2, ×=2, é=2, \n=1, z=1.
        assert_eq!(
            Ok(Pos {
                row: PositionRow::from(0),
                col: PositionColumn::from(0),
            }),
            pos_of_byte(SourceText::from(text), ByteOffset::from(0))
        );
        assert_eq!(
            Ok(Pos {
                row: PositionRow::from(0),
                col: PositionColumn::from(1),
            }),
            pos_of_byte(SourceText::from(text), ByteOffset::from(2))
        );
        assert_eq!(
            Ok(Pos {
                row: PositionRow::from(0),
                col: PositionColumn::from(2),
            }),
            pos_of_byte(SourceText::from(text), ByteOffset::from(4))
        );
        assert_eq!(
            Ok(Pos {
                row: PositionRow::from(1),
                col: PositionColumn::from(0),
            }),
            pos_of_byte(SourceText::from(text), ByteOffset::from(7))
        );
        assert_eq!(
            ByteOffset::from(7),
            byte_of_pos(SourceText::from(text), Pos {
                row: PositionRow::from(1),
                col: PositionColumn::from(0),
            })
        );
    }

    #[test]
    fn pos_of_byte_rejects_interior_multibyte_offsets()
    {
        let err = pos_of_byte(SourceText::from("é"), ByteOffset::from(1))
            .expect_err("byte 1 is inside the two-byte é code point");
        assert_eq!(ByteOffset::from(1), err.byte());
    }

    #[test]
    fn out_of_range_positions_clamp()
    {
        let text = "ab\ncd";
        assert_eq!(
            Ok(Pos {
                row: PositionRow::from(1),
                col: PositionColumn::from(2),
            }),
            pos_of_byte(SourceText::from(text), ByteOffset::from(999))
        );
        assert_eq!(
            ByteOffset::from(2),
            byte_of_pos(SourceText::from(text), Pos {
                row: PositionRow::from(0),
                col: PositionColumn::from(99),
            })
        );
        assert_eq!(
            ByteOffset::from(5),
            byte_of_pos(SourceText::from(text), Pos {
                row: PositionRow::from(9),
                col: PositionColumn::from(0),
            })
        );
    }
}
