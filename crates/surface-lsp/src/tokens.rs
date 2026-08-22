//! Semantic tokens: highlight spans re-encoded as the LSP integer stream.
//!
//! The token content is the grammar's highlight classification
//! ([`gandr_surface_render_remote::present::HlRole`]) — the same spans a TUI
//! styles. This module maps roles onto the LSP standard token-type registry
//! and performs the spec's delta encoding: five unsigned integers per token
//! (`deltaLine`, `deltaStartChar`, `length`, a `tokenTypes` legend index, and
//! a `tokenModifiers` bitset), with multi-line spans split per line.

use alloc::vec::Vec;

use gandr_surface_render_remote::present::ByteOffset;
use gandr_surface_render_remote::present::HlRole;
use gandr_surface_render_remote::present::HlSpan;
use gandr_surface_render_remote::present::SourceText;

use crate::position::LineIndex;
use crate::position::PositionEncoding;
use crate::position::position_of_byte;
use crate::protocol::CharacterOffset;
use crate::protocol::LineNumber;
use crate::protocol::TokenUnit;

/// The advertised `tokenTypes` legend, in wire-index order.
pub const TOKEN_TYPES: [&str; 14] = [
    "keyword",
    "operator",
    "function",
    "variable",
    "parameter",
    "property",
    "enumMember",
    "type",
    "typeParameter",
    "number",
    "string",
    "comment",
    "macro",
    "label",
];

/// The advertised `tokenModifiers` legend, in bit order.
pub const TOKEN_MODIFIERS: [&str; 2] = ["declaration", "defaultLibrary"];

/// Legend index of a standard token type.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TokenTypeIndex(u32);

impl From<TokenTypeIndex> for TokenUnit
{
    #[inline]
    fn from(value: TokenTypeIndex) -> Self
    {
        Self::from(value.0)
    }
}

/// Modifier bitset over [`TOKEN_MODIFIERS`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct TokenModifierSet(u32);

impl From<TokenModifierSet> for TokenUnit
{
    #[inline]
    fn from(value: TokenModifierSet) -> Self
    {
        Self::from(value.0)
    }
}

impl TokenModifierSet
{
    /// The `declaration` modifier bit (legend index 0).
    const DECLARATION: Self = Self(1);
    /// The `defaultLibrary` modifier bit (legend index 1).
    const DEFAULT_LIBRARY: Self = Self(2);
}

/// Maps a highlight role to its legend index and modifier bitset.
///
/// # Contract
/// - ensures: every returned index is in bounds of [`TOKEN_TYPES`] and every
///   bitset only sets [`TOKEN_MODIFIERS`] bits; [`HlRole::Other`] emits
///   nothing.
/// - panics: none.
#[must_use]
fn token_of_role(role: HlRole) -> Option<(TokenTypeIndex, TokenModifierSet)>
{
    match role {
        | HlRole::Keyword | HlRole::Boolean => Some((TokenTypeIndex(0), TokenModifierSet(0))),
        | HlRole::Operator => Some((TokenTypeIndex(1), TokenModifierSet(0))),
        | HlRole::FunctionDef => Some((TokenTypeIndex(2), TokenModifierSet::DECLARATION)),
        | HlRole::FunctionCall => Some((TokenTypeIndex(2), TokenModifierSet(0))),
        | HlRole::VariableDef => Some((TokenTypeIndex(3), TokenModifierSet::DECLARATION)),
        | HlRole::Variable => Some((TokenTypeIndex(3), TokenModifierSet(0))),
        | HlRole::VariableParam => Some((TokenTypeIndex(4), TokenModifierSet(0))),
        | HlRole::Member => Some((TokenTypeIndex(5), TokenModifierSet(0))),
        | HlRole::Constructor => Some((TokenTypeIndex(6), TokenModifierSet(0))),
        | HlRole::Type => Some((TokenTypeIndex(7), TokenModifierSet(0))),
        | HlRole::TypeBuiltin => Some((TokenTypeIndex(7), TokenModifierSet::DEFAULT_LIBRARY)),
        | HlRole::TypeVariable => Some((TokenTypeIndex(8), TokenModifierSet(0))),
        | HlRole::Number => Some((TokenTypeIndex(9), TokenModifierSet(0))),
        | HlRole::StringLit | HlRole::Character | HlRole::Escape | HlRole::Path => {
            Some((TokenTypeIndex(10), TokenModifierSet(0)))
        },
        | HlRole::Comment => Some((TokenTypeIndex(11), TokenModifierSet(0))),
        | HlRole::Hole | HlRole::Directive => Some((TokenTypeIndex(12), TokenModifierSet(0))),
        | HlRole::Label => Some((TokenTypeIndex(13), TokenModifierSet(0))),
        | HlRole::Other => None,
    }
}

/// One token after multi-line splitting, before delta encoding.
struct FlatToken
{
    /// Line of the token start.
    line: LineNumber,
    /// Character offset of the token start.
    start: CharacterOffset,
    /// Length in encoding units.
    length: TokenUnit,
    /// Legend index.
    type_index: TokenTypeIndex,
    /// Modifier bitset.
    modifiers: TokenModifierSet,
}

/// Encodes highlight spans as the LSP semantic-token integer stream.
///
/// # Contract
/// - requires: `index` was built from `text`; `spans` are sorted and disjoint
///   with ranges inside `text`.
/// - ensures: tokens appear in position order with non-negative deltas; spans
///   crossing line boundaries are split into one token per line with trailing
///   line terminators excluded; lengths and columns count `encoding` code
///   units; unmappable or empty pieces are skipped.
/// - panics: none.
#[inline]
#[must_use]
pub fn encode(
    text: SourceText<'_>,
    index: &LineIndex,
    encoding: PositionEncoding,
    spans: &[HlSpan],
) -> Vec<TokenUnit>
{
    let mut flats: Vec<FlatToken> = Vec::new();
    for span in spans {
        let Some(class) = token_of_role(span.role)
        else {
            continue;
        };
        push_split_tokens(
            text,
            index,
            encoding,
            span.range.start,
            span.range.end,
            class,
            &mut flats,
        );
    }
    flats.sort_by(|left, right| {
        u32::from(left.line)
            .cmp(&u32::from(right.line))
            .then(u32::from(left.start).cmp(&u32::from(right.start)))
    });
    delta_encode(&flats)
}

/// Splits one byte span into per-line tokens, excluding line terminators.
fn push_split_tokens(
    source: SourceText<'_>,
    index: &LineIndex,
    encoding: PositionEncoding,
    start: ByteOffset,
    end: ByteOffset,
    class: (TokenTypeIndex, TokenModifierSet),
    out: &mut Vec<FlatToken>,
)
{
    let (type_index, modifiers) = class;
    let start_pos = position_of_byte(source, index, start, encoding);
    let end_pos = position_of_byte(source, index, end, encoding);
    let start_line = u32::from(start_pos.line);
    let end_line = u32::from(end_pos.line);
    let mut line = start_line;
    while line <= end_line {
        let line_number = LineNumber::from(line);
        let start_char = if line == start_line {
            start_pos.character
        }
        else {
            CharacterOffset::from(0_u32)
        };
        let end_char = if line == end_line {
            end_pos.character
        }
        else {
            line_content_units(source, index, line_number, encoding)
        };
        let start_units = u32::from(start_char);
        let end_units = u32::from(end_char);
        if let Some(length) = end_units.checked_sub(start_units)
            && length > 0
        {
            out.push(FlatToken {
                line: line_number,
                start: start_char,
                length: TokenUnit::from(length),
                type_index,
                modifiers,
            });
        }
        let Some(next) = line.checked_add(1)
        else {
            break;
        };
        line = next;
    }
}

/// Code units on `line` before its terminator.
fn line_content_units(
    source: SourceText<'_>,
    index: &LineIndex,
    line: LineNumber,
    encoding: PositionEncoding,
) -> CharacterOffset
{
    let Some(range) = index.line_bytes(source, line)
    else {
        return CharacterOffset::from(0_u32);
    };
    let content_end = line_content_end(source, range);
    position_of_byte(source, index, content_end, encoding).character
}

/// Byte offset of the first line terminator in `range`, or the range end.
fn line_content_end(
    source: SourceText<'_>,
    range: core::ops::Range<ByteOffset>,
) -> ByteOffset
{
    let text: &str = source.into();
    let start = usize::from(range.start);
    let end = usize::from(range.end);
    let Some(slice) = text.get(start .. end)
    else {
        return range.end;
    };
    match slice.find('\n') {
        | Some(relative) => ByteOffset::from(start.saturating_add(relative)),
        | None => range.end,
    }
}

/// Delta-encodes sorted flat tokens into the five-integer LSP stream.
fn delta_encode(tokens: &[FlatToken]) -> Vec<TokenUnit>
{
    let mut data = Vec::new();
    let mut prev_line = 0_u32;
    let mut prev_start = 0_u32;
    for token in tokens {
        let line = u32::from(token.line);
        let start = u32::from(token.start);
        let Some(delta_line) = line.checked_sub(prev_line)
        else {
            continue;
        };
        let delta_start = if delta_line == 0 {
            match start.checked_sub(prev_start) {
                | Some(delta) => delta,
                | None => continue,
            }
        }
        else {
            start
        };
        data.push(TokenUnit::from(delta_line));
        data.push(TokenUnit::from(delta_start));
        data.push(token.length);
        data.push(TokenUnit::from(token.type_index));
        data.push(TokenUnit::from(token.modifiers));
        prev_line = line;
        prev_start = start;
    }
    data
}

#[cfg(test)]
mod tests
{
    use gandr_surface_render_remote::present::ByteOffset;
    use gandr_surface_render_remote::present::HlRole;
    use gandr_surface_render_remote::present::HlSpan;
    use gandr_surface_render_remote::present::SourceText;

    use super::TOKEN_MODIFIERS;
    use super::TOKEN_TYPES;
    use super::TokenModifierSet;
    use super::encode;
    use super::token_of_role;
    use crate::position::LineIndex;
    use crate::position::PositionEncoding;
    use crate::protocol::TokenUnit;

    #[test]
    fn every_classified_role_maps_inside_the_legend()
    {
        let roles = [
            HlRole::Keyword,
            HlRole::Operator,
            HlRole::FunctionDef,
            HlRole::FunctionCall,
            HlRole::VariableDef,
            HlRole::VariableParam,
            HlRole::Member,
            HlRole::Variable,
            HlRole::Constructor,
            HlRole::Type,
            HlRole::TypeBuiltin,
            HlRole::TypeVariable,
            HlRole::Number,
            HlRole::Boolean,
            HlRole::Character,
            HlRole::StringLit,
            HlRole::Escape,
            HlRole::Comment,
            HlRole::Hole,
            HlRole::Label,
            HlRole::Path,
            HlRole::Directive,
        ];
        for role in roles {
            let (index, modifiers) =
                token_of_role(role).unwrap_or_else(|| panic!("{role:?} must map"));
            assert!(
                usize::try_from(index.0).is_ok_and(|slot| slot < TOKEN_TYPES.len()),
                "{role:?} index {} is outside the legend",
                index.0
            );
            assert!(
                modifiers.0 < 4,
                "{role:?} modifiers {:#b} set an unknown bit",
                modifiers.0
            );
        }
        assert_eq!(None, token_of_role(HlRole::Other));
    }

    /// The LSP standard token type and modifiers each highlight role *means*,
    /// stated independently of [`TOKEN_TYPES`]'s index order and of the numeric
    /// literals [`token_of_role`] returns.
    ///
    /// This is deliberately a second statement of the same contract rather than
    /// a derivation from either side. The legend is a wire contract: a client
    /// indexes into it by integer, so the pair (index a role emits, string that
    /// index names) is the thing a client observes, and neither
    /// [`TOKEN_TYPES`] nor [`token_of_role`] states it alone.
    ///
    /// Exhaustiveness is compiler-enforced: adding an [`HlRole`] variant fails
    /// to compile here until its LSP meaning is stated.
    fn lsp_meaning_of_role(role: HlRole) -> Option<(LegendName, &'static [ModifierName])>
    {
        const DECLARED: &[ModifierName] = &[ModifierName("declaration")];
        const BUILTIN: &[ModifierName] = &[ModifierName("defaultLibrary")];
        const PLAIN: &[ModifierName] = &[];
        match role {
            | HlRole::Keyword | HlRole::Boolean => Some((LegendName("keyword"), PLAIN)),
            | HlRole::Operator => Some((LegendName("operator"), PLAIN)),
            | HlRole::FunctionDef => Some((LegendName("function"), DECLARED)),
            | HlRole::FunctionCall => Some((LegendName("function"), PLAIN)),
            | HlRole::VariableDef => Some((LegendName("variable"), DECLARED)),
            | HlRole::Variable => Some((LegendName("variable"), PLAIN)),
            | HlRole::VariableParam => Some((LegendName("parameter"), PLAIN)),
            | HlRole::Member => Some((LegendName("property"), PLAIN)),
            | HlRole::Constructor => Some((LegendName("enumMember"), PLAIN)),
            | HlRole::Type => Some((LegendName("type"), PLAIN)),
            | HlRole::TypeBuiltin => Some((LegendName("type"), BUILTIN)),
            | HlRole::TypeVariable => Some((LegendName("typeParameter"), PLAIN)),
            | HlRole::Number => Some((LegendName("number"), PLAIN)),
            | HlRole::StringLit | HlRole::Character | HlRole::Escape | HlRole::Path => {
                Some((LegendName("string"), PLAIN))
            },
            | HlRole::Comment => Some((LegendName("comment"), PLAIN)),
            | HlRole::Hole | HlRole::Directive => Some((LegendName("macro"), PLAIN)),
            | HlRole::Label => Some((LegendName("label"), PLAIN)),
            | HlRole::Other => None,
        }
    }

    /// An LSP standard token-type name as it appears in the advertised legend.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct LegendName(&'static str);

    /// A modifier name as it appears in [`TOKEN_MODIFIERS`].
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ModifierName(&'static str);

    /// Modifier names a bitset selects out of [`TOKEN_MODIFIERS`], in bit
    /// order.
    fn modifier_names(bits: TokenModifierSet) -> Vec<ModifierName>
    {
        let mut names = Vec::new();
        for (bit, name) in TOKEN_MODIFIERS.iter().enumerate() {
            let Ok(shift) = u32::try_from(bit)
            else {
                continue;
            };
            if bits.0 & (1_u32 << shift) != 0 {
                names.push(ModifierName(name));
            }
        }
        names
    }

    /// The index a role emits must name that role's meaning in the legend.
    ///
    /// Ablation (2026-08-22): swapping the `number` and `string` entries of
    /// [`TOKEN_TYPES`] leaves every other witness in this crate, the
    /// `capabilities` integration witness, and the driver's smoke witness green
    /// while the server advertises every numeric literal as a string. The
    /// ablation was verified to reach the shipping artifact — `gandr lsp
    /// --capabilities` printed the permuted legend — so the green was a gap in
    /// the witness set rather than an ineffective ablation. This test is the
    /// assertion that gap owed; it goes red under exactly that permutation.
    #[test]
    fn the_legend_index_a_role_emits_names_what_that_role_means()
    {
        let roles = [
            HlRole::Keyword,
            HlRole::Operator,
            HlRole::FunctionDef,
            HlRole::FunctionCall,
            HlRole::VariableDef,
            HlRole::VariableParam,
            HlRole::Member,
            HlRole::Variable,
            HlRole::Constructor,
            HlRole::Type,
            HlRole::TypeBuiltin,
            HlRole::TypeVariable,
            HlRole::Number,
            HlRole::Boolean,
            HlRole::Character,
            HlRole::StringLit,
            HlRole::Escape,
            HlRole::Comment,
            HlRole::Hole,
            HlRole::Label,
            HlRole::Path,
            HlRole::Directive,
        ];
        for role in roles {
            let Some((expected_type, expected_modifiers)) = lsp_meaning_of_role(role)
            else {
                panic!("{role:?} is classified, so it must have an LSP meaning");
            };
            let (index, modifiers) =
                token_of_role(role).unwrap_or_else(|| panic!("{role:?} must map"));
            let slot = usize::try_from(index.0).unwrap_or(usize::MAX);
            let named = LegendName(
                TOKEN_TYPES
                    .get(slot)
                    .copied()
                    .unwrap_or_else(|| panic!("{role:?} index {} is outside the legend", index.0)),
            );
            assert_eq!(
                expected_type, named,
                "{role:?} emits legend index {}, which names {} rather than {}: a client \
                 would render it as {}",
                index.0, named.0, expected_type.0, named.0
            );
            assert_eq!(
                expected_modifiers.to_vec(),
                modifier_names(modifiers),
                "{role:?} modifier bitset {:#b} does not name {expected_modifiers:?}",
                modifiers.0
            );
        }
        assert_eq!(None, lsp_meaning_of_role(HlRole::Other));
        assert_eq!(None, token_of_role(HlRole::Other));
    }

    #[test]
    fn a_one_line_keyword_encodes_as_five_integers()
    {
        let text = "def x";
        let index = LineIndex::new(SourceText::from(text));
        let spans = [HlSpan::new(
            ByteOffset::from(0) .. ByteOffset::from(3),
            HlRole::Keyword,
        )];
        let data = encode(
            SourceText::from(text),
            &index,
            PositionEncoding::Utf16,
            &spans,
        );
        assert_eq!(
            vec![
                TokenUnit::from(0_u32),
                TokenUnit::from(0_u32),
                TokenUnit::from(3_u32),
                TokenUnit::from(0_u32),
                TokenUnit::from(0_u32),
            ],
            data
        );
    }

    #[test]
    fn a_multiline_span_splits_and_drops_the_terminator()
    {
        let text = "ab\ncd";
        let index = LineIndex::new(SourceText::from(text));
        let spans = [HlSpan::new(
            ByteOffset::from(0) .. ByteOffset::from(5),
            HlRole::Comment,
        )];
        let data = encode(
            SourceText::from(text),
            &index,
            PositionEncoding::Utf16,
            &spans,
        );
        assert_eq!(
            vec![
                TokenUnit::from(0_u32),
                TokenUnit::from(0_u32),
                TokenUnit::from(2_u32),
                TokenUnit::from(11_u32),
                TokenUnit::from(0_u32),
                TokenUnit::from(1_u32),
                TokenUnit::from(0_u32),
                TokenUnit::from(2_u32),
                TokenUnit::from(11_u32),
                TokenUnit::from(0_u32),
            ],
            data
        );
    }
}
