//! The rigid base-type atoms and the S1 literal stock ([`BaseType`],
//! [`Literal`], and the normalized literal payloads).
//!
//! The base-type atoms are **rigid and closed** (kernel-boundary.md K1): the
//! S1 v0 stock is exactly the three the literal forms name — an integer atom,
//! a string atom, and a numeric atom (coordinator staging call, gandr-wvd.2).
//! Literal payloads are held in a **canonical, structurally comparable** form
//! (leading/trailing zeros normalized, zero sign pinned) so that syntactic
//! equality of two literals coincides with their intended value equality. At
//! S1 the payloads are inert to type checking — no value-type or
//! computation-type former is indexed by a value term, so the checker never
//! inspects a literal beyond its atom — but they are compared structurally by
//! the forward-compatible term conversion ([`crate::conv`]) and normalized
//! here so that comparison is meaningful.

use alloc::string::String;

/// A rigid base-type atom: one of the S1 v0 stock of primitive value types
/// (kernel-boundary.md §7 "literals over the base types").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BaseType
{
    /// The integer atom — the type of [`IntegerLiteral`]s.
    Integer,
    /// The string atom — the type of [`StringLiteral`]s.
    String,
    /// The numeric atom — the type of [`NumericLiteral`]s.
    Numeric,
}

/// The sign of a numeric literal. Zero is [`Self::NonNegative`], so the
/// canonical form of a literal never carries a negative zero.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Sign
{
    /// A negative magnitude (never the zero magnitude).
    Negative,
    /// A non-negative magnitude (zero included).
    NonNegative,
}

/// A canonical unsigned integer magnitude: ASCII decimal digits with no
/// leading zeros, the all-zero magnitude collapsed to a single `0`.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Magnitude(String);

impl Magnitude
{
    /// The canonical zero magnitude.
    #[inline]
    #[must_use]
    pub fn zero() -> Self
    {
        Self(String::from("0"))
    }

    /// Build a canonical magnitude from decimal digit text.
    ///
    /// # Contract
    /// - requires: `text` is intended as an unsigned decimal integer.
    /// - ensures: `Some(magnitude)` with leading zeros stripped and the
    ///   all-zero case collapsed to `0`, when every character is an ASCII
    ///   digit; the result compares equal exactly to magnitudes of the same
    ///   integer value.
    /// - provides: the canonical, structurally comparable magnitude of an
    ///   integer or numeric literal.
    /// - fails: returns `None` when `text` holds a non-digit character — a
    ///   malformed magnitude is not representable (K1 at construction).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn from_decimal_text(mut text: String) -> Option<Self>
    {
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        match text.find(|character: char| character != '0') {
            | Some(index) => {
                text.replace_range(.. index, "");
                Some(Self(text))
            },
            | None => Some(Self::zero()),
        }
    }

    /// The canonical decimal digits, as owned text — the export writer's
    /// serialization source (kernel-boundary.md §5 E1).
    #[inline]
    #[must_use]
    pub(crate) fn to_digits(&self) -> String
    {
        self.0.clone()
    }
}

/// A canonical fractional-digit sequence: ASCII decimal digits with no
/// trailing zeros (the empty sequence denotes an integral value).
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FractionDigits(String);

impl FractionDigits
{
    /// The canonical empty fraction (an integral numeric value).
    #[inline]
    #[must_use]
    pub fn none() -> Self
    {
        Self(String::new())
    }

    /// Build a canonical fraction from decimal digit text.
    ///
    /// # Contract
    /// - requires: `text` is intended as the digits after a decimal point.
    /// - ensures: `Some(fraction)` with trailing zeros stripped, when every
    ///   character is an ASCII digit; the result compares equal exactly to
    ///   fractions of the same value.
    /// - provides: the canonical, structurally comparable fractional part of a
    ///   numeric literal.
    /// - fails: returns `None` on a non-digit character (K1 at construction).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn from_decimal_text(mut text: String) -> Option<Self>
    {
        if !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        let trimmed_len = text.trim_end_matches('0').len();
        text.truncate(trimmed_len);
        Some(Self(text))
    }

    /// The canonical fractional digits, as owned text (empty for an integral
    /// value) — the export writer's serialization source (kernel-boundary.md
    /// §5 E1).
    #[inline]
    #[must_use]
    pub(crate) fn to_digits(&self) -> String
    {
        self.0.clone()
    }
}

/// A canonical integer literal: a [`Sign`] and a [`Magnitude`], typed at
/// [`BaseType::Integer`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IntegerLiteral
{
    /// The sign (never [`Sign::Negative`] on the zero magnitude).
    sign: Sign,
    /// The unsigned magnitude, canonicalized.
    magnitude: Magnitude,
}

impl IntegerLiteral
{
    /// Build a canonical integer literal, pinning a negative zero to
    /// non-negative.
    ///
    /// # Contract
    /// - requires: `magnitude` is already canonical (built through
    ///   [`Magnitude::from_decimal_text`]).
    /// - ensures: the sign of the zero magnitude is forced to
    ///   [`Sign::NonNegative`], so two literals compare equal exactly when they
    ///   denote the same integer.
    /// - provides: the canonical integer literal.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        sign: Sign,
        magnitude: Magnitude,
    ) -> Self
    {
        let sign = if magnitude == Magnitude::zero() {
            Sign::NonNegative
        }
        else {
            sign
        };
        Self { sign, magnitude }
    }

    /// The literal's sign.
    #[inline]
    #[must_use]
    pub const fn sign(&self) -> Sign
    {
        self.sign
    }

    /// The literal's canonical magnitude.
    #[inline]
    #[must_use]
    pub const fn magnitude(&self) -> &Magnitude
    {
        &self.magnitude
    }
}

/// A canonical string literal, typed at [`BaseType::String`]. The payload is
/// held verbatim — a string carries no normalization beyond its own bytes.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct StringLiteral(String);

impl StringLiteral
{
    /// Wrap string content as a literal.
    #[inline]
    #[must_use]
    pub fn new(content: String) -> Self
    {
        Self(content)
    }

    /// The string content, as owned text — the export writer's serialization
    /// source (kernel-boundary.md §5 E1).
    #[inline]
    #[must_use]
    pub(crate) fn to_content(&self) -> String
    {
        self.0.clone()
    }
}

/// A canonical numeric literal: a [`Sign`], an integral [`Magnitude`], and a
/// [`FractionDigits`] fractional part, typed at [`BaseType::Numeric`].
///
/// The canonical form pins a negative zero to non-negative, so two literals
/// compare equal exactly when they denote the same numeric value under the
/// documented decimal reading.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NumericLiteral
{
    /// The sign (never [`Sign::Negative`] on the zero value).
    sign: Sign,
    /// The integral part, canonicalized.
    integer_part: Magnitude,
    /// The fractional part, canonicalized (empty for an integral value).
    fraction: FractionDigits,
}

impl NumericLiteral
{
    /// Build a canonical numeric literal, pinning a negative zero to
    /// non-negative.
    ///
    /// # Contract
    /// - requires: `integer_part` and `fraction` are already canonical.
    /// - ensures: the sign of the zero value (zero integral part and empty
    ///   fraction) is forced to [`Sign::NonNegative`]; two literals compare
    ///   equal exactly when they denote the same numeric value.
    /// - provides: the canonical numeric literal.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        sign: Sign,
        integer_part: Magnitude,
        fraction: FractionDigits,
    ) -> Self
    {
        let is_zero = integer_part == Magnitude::zero() && fraction == FractionDigits::none();
        let sign = if is_zero { Sign::NonNegative } else { sign };
        Self {
            sign,
            integer_part,
            fraction,
        }
    }

    /// The literal's sign.
    #[inline]
    #[must_use]
    pub const fn sign(&self) -> Sign
    {
        self.sign
    }

    /// The literal's integral part.
    #[inline]
    #[must_use]
    pub const fn integer_part(&self) -> &Magnitude
    {
        &self.integer_part
    }

    /// The literal's fractional part.
    #[inline]
    #[must_use]
    pub const fn fraction(&self) -> &FractionDigits
    {
        &self.fraction
    }
}

/// An S1 literal value: one of the three rigid base-type atoms' canonical
/// payloads.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Literal
{
    /// An integer literal, typed at [`BaseType::Integer`].
    Integer(IntegerLiteral),
    /// A string literal, typed at [`BaseType::String`].
    Text(StringLiteral),
    /// A numeric literal, typed at [`BaseType::Numeric`].
    Numeric(NumericLiteral),
}

impl Literal
{
    /// The rigid base-type atom this literal inhabits.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: [`BaseType::Integer`] for an integer literal,
    ///   [`BaseType::String`] for a string literal, [`BaseType::Numeric`] for a
    ///   numeric literal.
    /// - provides: the literal's type atom for the synthesizing checker rule.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn base_type(&self) -> BaseType
    {
        match *self {
            | Self::Integer(_) => BaseType::Integer,
            | Self::Text(_) => BaseType::String,
            | Self::Numeric(_) => BaseType::Numeric,
        }
    }
}

#[cfg(test)]
mod tests
{
    use alloc::string::String;

    use super::FractionDigits;
    use super::IntegerLiteral;
    use super::Literal;
    use super::Magnitude;
    use super::NumericLiteral;
    use super::Sign;
    use super::StringLiteral;

    #[test]
    fn magnitude_strips_leading_zeros()
    {
        let padded = Magnitude::from_decimal_text(String::from("007")).unwrap();
        let bare = Magnitude::from_decimal_text(String::from("7")).unwrap();
        assert_eq!(padded, bare, "leading zeros must canonicalize away");
    }

    #[test]
    fn all_zero_magnitude_collapses_to_single_zero()
    {
        let magnitude = Magnitude::from_decimal_text(String::from("000")).unwrap();
        assert_eq!(
            magnitude,
            Magnitude::zero(),
            "an all-zero magnitude is zero"
        );
    }

    #[test]
    fn magnitude_rejects_non_digits()
    {
        assert_eq!(
            None,
            Magnitude::from_decimal_text(String::from("1a2")),
            "a non-digit character makes the magnitude unrepresentable"
        );
        assert_eq!(
            None,
            Magnitude::from_decimal_text(String::new()),
            "an empty magnitude is not a decimal integer"
        );
    }

    #[test]
    fn fraction_strips_trailing_zeros()
    {
        let padded = FractionDigits::from_decimal_text(String::from("50")).unwrap();
        let bare = FractionDigits::from_decimal_text(String::from("5")).unwrap();
        assert_eq!(padded, bare, "trailing zeros must canonicalize away");
        let none = FractionDigits::from_decimal_text(String::from("00")).unwrap();
        assert_eq!(none, FractionDigits::none(), "all-zero fraction is empty");
    }

    #[test]
    fn negative_zero_integer_is_pinned_non_negative()
    {
        let negative = IntegerLiteral::new(Sign::Negative, Magnitude::zero());
        let positive = IntegerLiteral::new(Sign::NonNegative, Magnitude::zero());
        assert_eq!(negative, positive, "a negative zero canonicalizes away");
        assert_eq!(Sign::NonNegative, negative.sign(), "zero is non-negative");
    }

    #[test]
    fn numeric_value_equality_ignores_representation_padding()
    {
        let long = NumericLiteral::new(
            Sign::NonNegative,
            Magnitude::from_decimal_text(String::from("01")).unwrap(),
            FractionDigits::from_decimal_text(String::from("50")).unwrap(),
        );
        let short = NumericLiteral::new(
            Sign::NonNegative,
            Magnitude::from_decimal_text(String::from("1")).unwrap(),
            FractionDigits::from_decimal_text(String::from("5")).unwrap(),
        );
        assert_eq!(long, short, "1.50 and 01.5 denote the same numeric value");
    }

    #[test]
    fn negative_zero_numeric_is_pinned_non_negative()
    {
        let negative =
            NumericLiteral::new(Sign::Negative, Magnitude::zero(), FractionDigits::none());
        assert_eq!(
            Sign::NonNegative,
            negative.sign(),
            "numeric zero is non-negative"
        );
    }

    #[test]
    fn literals_report_their_base_type_atom()
    {
        use super::BaseType;
        let integer = Literal::Integer(IntegerLiteral::new(Sign::NonNegative, Magnitude::zero()));
        let text = Literal::Text(StringLiteral::new(String::from("hi")));
        let numeric = Literal::Numeric(NumericLiteral::new(
            Sign::NonNegative,
            Magnitude::zero(),
            FractionDigits::none(),
        ));
        assert_eq!(BaseType::Integer, integer.base_type(), "integer atom");
        assert_eq!(BaseType::String, text.base_type(), "string atom");
        assert_eq!(BaseType::Numeric, numeric.base_type(), "numeric atom");
    }
}
