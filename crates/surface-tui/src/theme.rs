//! Style a highlight role.
//!
//! The grouping matches the language-server face's `token_of_role` map onto
//! `TOKEN_TYPES`. This crate styles [`HlRole`] directly and does not speak
//! the LSP integer legend.

use gandr_surface_render_remote::present::HlRole;
use ratatui::style::Color;
use ratatui::style::Style;

/// Style for `role`.
///
/// # Contract
/// - ensures: every role the language-server face maps to a token type gets a
///   colour; [`HlRole::Other`] is the terminal default.
/// - panics: none.
#[inline]
#[must_use]
pub fn style_of(role: HlRole) -> Style
{
    let color = match role {
        | HlRole::Keyword | HlRole::Boolean | HlRole::Hole | HlRole::Directive => Color::Magenta,
        | HlRole::Operator | HlRole::Label => Color::Cyan,
        | HlRole::FunctionDef | HlRole::FunctionCall | HlRole::Constructor => Color::Blue,
        | HlRole::VariableDef | HlRole::Variable | HlRole::Other => Color::Reset,
        | HlRole::VariableParam | HlRole::Member | HlRole::Number => Color::Yellow,
        | HlRole::Type | HlRole::TypeBuiltin | HlRole::TypeVariable => Color::Green,
        | HlRole::StringLit | HlRole::Character | HlRole::Escape | HlRole::Path => Color::Red,
        | HlRole::Comment => Color::DarkGray,
    };
    Style::default().fg(color)
}

#[cfg(test)]
mod tests
{
    use gandr_surface_render_remote::present::HlRole;
    use ratatui::style::Color;

    use super::style_of;

    #[test]
    fn other_is_the_terminal_default()
    {
        assert_eq!(style_of(HlRole::Other).fg, Some(Color::Reset));
    }

    #[test]
    fn keyword_and_boolean_share_the_keyword_colour()
    {
        assert_eq!(style_of(HlRole::Keyword), style_of(HlRole::Boolean));
    }
}
