//! UTF-8 byte-offset to LSP position mapping.
//!
//! Pipeline spans are UTF-8 byte offsets. LSP positions count code units of
//! the negotiated encoding per line: UTF-16 by default, UTF-8 when a client
//! offers it.

use alloc::vec::Vec;

use gandr_surface_render_remote::present::ByteOffset;
use gandr_surface_render_remote::present::SourceText;

use crate::boundary::EncodingWidth;
use crate::protocol::CharacterOffset;
use crate::protocol::LineNumber;
use crate::protocol::Position;

/// The negotiated position encoding for `Position.character` units.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PositionEncoding
{
    /// UTF-8 code units (bytes).
    Utf8,
    /// UTF-16 code units.
    Utf16,
}

/// A line-start index over one document text.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LineIndex
{
    /// Byte offset of each line start, including offset 0 for line 0.
    starts: Vec<ByteOffset>,
}

impl LineIndex
{
    /// Build a line index from `text`.
    ///
    /// # Contract
    /// - ensures: line 0 starts at byte 0; every byte after `\n` starts a line,
    ///   including a trailing empty line when `text` ends in `\n`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(text: SourceText<'_>) -> Self
    {
        let source: &str = text.into();
        let mut starts = Vec::from([ByteOffset::from(0)]);
        for (offset, ch) in source.char_indices() {
            if ch == '\n' {
                let next = offset.saturating_add(ch.len_utf8());
                starts.push(ByteOffset::from(next));
            }
        }
        Self { starts }
    }

    /// Start and end byte offsets of `line`.
    ///
    /// # Contract
    /// - ensures: returns the line's byte span when the line exists.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn line_bytes(
        &self,
        text: SourceText<'_>,
        line: LineNumber,
    ) -> Option<core::ops::Range<ByteOffset>>
    {
        let source: &str = text.into();
        let index = usize::try_from(u32::from(line)).ok()?;
        let start = *self.starts.get(index)?;
        let end = self
            .starts
            .get(index.saturating_add(1))
            .copied()
            .unwrap_or_else(|| ByteOffset::from(source.len()));
        Some(start .. end)
    }
}

/// Converts a byte offset into an LSP position over `text`.
///
/// # Contract
/// - requires: `index` was built from `text`.
/// - ensures: total — an offset past the end clamps to the end; an offset
///   inside a UTF-8 sequence snaps to its character's start; the character
///   column counts code units of `encoding` from the line start.
/// - panics: none.
#[inline]
#[must_use]
pub fn position_of_byte(
    text: SourceText<'_>,
    index: &LineIndex,
    byte: ByteOffset,
    encoding: PositionEncoding,
) -> Position
{
    let snapped = snap_to_char(text, byte);
    let snapped_usize = usize::from(snapped);
    let line_slot = index
        .starts
        .partition_point(|start| usize::from(*start) <= snapped_usize)
        .saturating_sub(1);
    let line_start = index
        .starts
        .get(line_slot)
        .copied()
        .unwrap_or_else(|| ByteOffset::from(0));
    let Ok(line) = u32::try_from(line_slot)
    else {
        return Position::new(LineNumber::from(0_u32), CharacterOffset::from(0_u32));
    };
    let character = units_between(text, line_start, snapped, encoding);
    Position::new(LineNumber::from(line), character)
}

/// Converts an LSP position into a byte offset over `text`.
///
/// # Contract
/// - requires: `index` was built from `text`.
/// - ensures: total — a line past the end clamps to the text end; a character
///   past the line's content clamps to the line end.
/// - panics: none.
#[inline]
#[must_use]
pub fn byte_of_position(
    text: SourceText<'_>,
    index: &LineIndex,
    position: Position,
    encoding: PositionEncoding,
) -> ByteOffset
{
    let source: &str = text.into();
    let Some(range) = index.line_bytes(text, position.line)
    else {
        return ByteOffset::from(source.len());
    };
    let start = usize::from(range.start);
    let end = usize::from(range.end);
    let content_end = match source.get(start .. end) {
        | Some(slice) => match slice.find('\n') {
            | Some(relative) => ByteOffset::from(start.saturating_add(relative)),
            | None => range.end,
        },
        | None => range.end,
    };
    advance_units(text, range.start, content_end, position.character, encoding)
}

/// Snap `offset` down to a character boundary.
fn snap_to_char(
    text: SourceText<'_>,
    offset: ByteOffset,
) -> ByteOffset
{
    let source: &str = text.into();
    let mut candidate = usize::from(offset).min(source.len());
    if candidate >= source.len() {
        return ByteOffset::from(source.len());
    }
    if source.is_char_boundary(candidate) {
        return ByteOffset::from(candidate);
    }
    while candidate > 0 && !source.is_char_boundary(candidate) {
        candidate = candidate.saturating_sub(1);
    }
    ByteOffset::from(candidate)
}

/// Count encoding units on the half-open byte range `[start, end)`.
fn units_between(
    text: SourceText<'_>,
    start: ByteOffset,
    end: ByteOffset,
    encoding: PositionEncoding,
) -> CharacterOffset
{
    let source: &str = text.into();
    let Some(slice) = source.get(usize::from(start) .. usize::from(end))
    else {
        return CharacterOffset::from(0_u32);
    };
    let mut units = 0_u32;
    for ch in slice.chars() {
        units = units.saturating_add(u32::from(unit_width(
            crate::boundary::SourceChar::from(ch),
            encoding,
        )));
    }
    CharacterOffset::from(units)
}

/// Advance `target` encoding units from `start`, stopping at `limit`.
fn advance_units(
    text: SourceText<'_>,
    start: ByteOffset,
    limit: ByteOffset,
    target: CharacterOffset,
    encoding: PositionEncoding,
) -> ByteOffset
{
    let source: &str = text.into();
    let start_usize = usize::from(start);
    let limit_usize = usize::from(limit);
    let Some(slice) = source.get(start_usize .. limit_usize)
    else {
        return ByteOffset::from(start_usize.min(source.len()));
    };
    let mut units = 0_u32;
    let want = u32::from(target);
    let mut offset = start_usize;
    for ch in slice.chars() {
        if units >= want {
            break;
        }
        units = units.saturating_add(u32::from(unit_width(
            crate::boundary::SourceChar::from(ch),
            encoding,
        )));
        offset = offset.saturating_add(ch.len_utf8());
    }
    ByteOffset::from(offset)
}

/// Width of `ch` in the negotiated encoding.
fn unit_width(
    ch: crate::boundary::SourceChar,
    encoding: PositionEncoding,
) -> EncodingWidth
{
    let width = match encoding {
        | PositionEncoding::Utf8 => ch.0.len_utf8(),
        | PositionEncoding::Utf16 => ch.0.len_utf16(),
    };
    EncodingWidth::from(u32::try_from(width).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests
{
    use gandr_surface_render_remote::present::ByteOffset;
    use gandr_surface_render_remote::present::SourceText;

    use super::LineIndex;
    use super::PositionEncoding;
    use super::byte_of_position;
    use super::position_of_byte;
    use crate::protocol::CharacterOffset;
    use crate::protocol::LineNumber;
    use crate::protocol::Position;

    #[test]
    fn utf16_counts_an_astral_character_as_two_units()
    {
        let text = "a😀b";
        let source = SourceText::from(text);
        let index = LineIndex::new(source);
        let grin = text.find('😀').expect("the grin is in the fixture");
        let after = grin.saturating_add('😀'.len_utf8());
        let pos = position_of_byte(
            source,
            &index,
            ByteOffset::from(after),
            PositionEncoding::Utf16,
        );
        assert_eq!(LineNumber::from(0_u32), pos.line);
        assert_eq!(CharacterOffset::from(3_u32), pos.character);
        let back = byte_of_position(source, &index, pos, PositionEncoding::Utf16);
        assert_eq!(ByteOffset::from(after), back);
    }

    #[test]
    fn a_line_past_the_end_clamps()
    {
        let text = "ab";
        let source = SourceText::from(text);
        let index = LineIndex::new(source);
        let pos = Position::new(LineNumber::from(4_u32), CharacterOffset::from(0_u32));
        assert_eq!(
            ByteOffset::from(text.len()),
            byte_of_position(source, &index, pos, PositionEncoding::Utf8)
        );
    }
}
