//! Stable diagnostic codes and localizable message arguments.

use core::fmt;

/// One localizable registry template.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct DiagnosticTemplate(&'static str);

/// The private primitive image used only by display and codecs.
#[derive(Clone, Copy)]
#[repr(transparent)]
struct DiagnosticCodeText(&'static str);

impl fmt::Display for DiagnosticTemplate
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(self.0)
    }
}

/// Declares the complete stable code registry.
///
/// A code literal occurs exactly once, in its registry row. Codes are never
/// reused; a retired row remains reserved.
macro_rules! diagnostic_registry {
    ($( $variant:ident => ($code:literal, $template:literal) ),+ $(,)?) => {
        /// A stable, rustc-style diagnostic identifier.
        #[expect(
            clippy::doc_markdown,
            reason = "registry templates use serialized argument names"
        )]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum DiagnosticCode
        {
            $(
                #[doc = $template]
                $variant,
            )+
        }

        /// Every allocated diagnostic code, in allocation order.
        pub const DIAGNOSTIC_CODES: &[DiagnosticCode] = &[
            $(DiagnosticCode::$variant,)+
        ];

        impl DiagnosticCode
        {
            /// The stable wire spelling used by display and codecs.
            #[inline]
            const fn text(self) -> DiagnosticCodeText
            {
                match self {
                    $(Self::$variant => DiagnosticCodeText($code),)+
                }
            }

            /// The localizable message template registered for this code.
            #[inline]
            #[must_use]
            pub const fn template(self) -> DiagnosticTemplate
            {
                match self {
                    $(Self::$variant => DiagnosticTemplate($template),)+
                }
            }

        }
    };
}

diagnostic_registry! {
    TypeMismatch => ("E0001", "type mismatch: expected {expected}, actual {actual}"),
    ShapeMismatch => ("E0002", "type shape mismatch: expected {expected_shape}, actual {actual}"),
    StuckExpression => ("E0003", "no typing rule applies to term {expression}: {hint}"),
    UnboundVariable => ("E0004", "variable is unbound: {name}"),
    GradeOrder => ("E0005", "grade order requirement failed: {lower} ⊑ {upper} does not hold"),
    UnknownAttribute => ("E0006", "unknown attribute `{name}`{suggestion}"),
    DuplicateAttribute => ("E0007", "duplicate attribute `{name}` (single-valued)"),
    MissingAttributePayload => ("E0008", "attribute `{name}` requires a payload"),
    NonValueAttributePayload => ("E0009", "attribute `{name}` payload must be a value, not a computation"),
    ShadowedName => ("W0010", "`{path}` is a prelude or host name; this declaration takes it, and the policy allows it"),
    Other => ("E0011", "{message}"),
    ParseRepair => ("W0012", "parse repaired: {class}"),
}
impl core::str::FromStr for DiagnosticCode
{
    type Err = ();

    #[inline]
    fn from_str(code: &str) -> Result<Self, Self::Err>
    {
        match code {
            | "E0001" => Ok(Self::TypeMismatch),
            | "E0002" => Ok(Self::ShapeMismatch),
            | "E0003" => Ok(Self::StuckExpression),
            | "E0004" => Ok(Self::UnboundVariable),
            | "E0005" => Ok(Self::GradeOrder),
            | "E0006" => Ok(Self::UnknownAttribute),
            | "E0007" => Ok(Self::DuplicateAttribute),
            | "E0008" => Ok(Self::MissingAttributePayload),
            | "E0009" => Ok(Self::NonValueAttributePayload),
            | "W0010" => Ok(Self::ShadowedName),
            | "E0011" => Ok(Self::Other),
            | "W0012" => Ok(Self::ParseRepair),
            | _ => Err(()),
        }
    }
}

impl fmt::Display for DiagnosticCode
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(self.text().0)
    }
}

#[cfg(feature = "codecs")]
impl serde::Serialize for DiagnosticCode
{
    #[inline]
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.text().0)
    }
}

#[cfg(feature = "codecs")]
impl<'de> serde::Deserialize<'de> for DiagnosticCode
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let code = <String as serde::Deserialize>::deserialize(deserializer)?;
        code.parse::<Self>().map_err(|()| {
            serde::de::Error::custom(format_args!("unknown diagnostic code `{code}`"))
        })
    }
}

/// A diagnostic's template identity and named arguments.
///
/// The variant is the template. Its fields are arguments; rendered prose is a
/// projection rather than stored source data.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "codecs", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "codecs",
    serde(tag = "message_template", content = "message_arguments")
)]
pub enum DiagnosticMessage
{
    /// Type mismatch arguments.
    TypeMismatch
    {
        /// Expected type.
        expected: String,
        /// Actual type.
        actual: String,
    },
    /// Type-shape mismatch arguments.
    ShapeMismatch
    {
        /// Expected shape.
        expected_shape: String,
        /// Actual type.
        actual: String,
    },
    /// Stuck-expression arguments.
    StuckExpression
    {
        /// Offending expression.
        expression: String,
        /// Progress hint.
        hint: String,
    },
    /// Unbound-variable arguments.
    UnboundVariable
    {
        /// Missing name.
        name: String,
    },
    /// Grade-order arguments.
    GradeOrder
    {
        /// Lower grade.
        lower: String,
        /// Upper grade.
        upper: String,
    },
    /// Unknown-attribute arguments.
    UnknownAttribute
    {
        /// Unknown name.
        name: String,
        /// Nearest registry name.
        suggestion: Option<String>,
    },
    /// Duplicate-attribute arguments.
    DuplicateAttribute
    {
        /// Duplicated name.
        name: String,
    },
    /// Missing-payload arguments.
    MissingAttributePayload
    {
        /// Attribute name.
        name: String,
    },
    /// Non-value-payload arguments.
    NonValueAttributePayload
    {
        /// Attribute name.
        name: String,
    },
    /// Shadowed-name arguments.
    ShadowedName
    {
        /// Shadowed path.
        path: String,
    },
    /// Parse-repair arguments.
    ParseRepair
    {
        /// Recovery obligation class.
        class: String,
    },
    /// Forward-compatible message text.
    Other
    {
        /// Rendered message.
        message: String,
    },
}

impl DiagnosticMessage
{
    /// The stable registry code for this template.
    #[inline]
    #[must_use]
    pub const fn code(&self) -> DiagnosticCode
    {
        match *self {
            | Self::TypeMismatch { .. } => DiagnosticCode::TypeMismatch,
            | Self::ShapeMismatch { .. } => DiagnosticCode::ShapeMismatch,
            | Self::StuckExpression { .. } => DiagnosticCode::StuckExpression,
            | Self::UnboundVariable { .. } => DiagnosticCode::UnboundVariable,
            | Self::GradeOrder { .. } => DiagnosticCode::GradeOrder,
            | Self::UnknownAttribute { .. } => DiagnosticCode::UnknownAttribute,
            | Self::DuplicateAttribute { .. } => DiagnosticCode::DuplicateAttribute,
            | Self::MissingAttributePayload { .. } => DiagnosticCode::MissingAttributePayload,
            | Self::NonValueAttributePayload { .. } => DiagnosticCode::NonValueAttributePayload,
            | Self::ShadowedName { .. } => DiagnosticCode::ShadowedName,
            | Self::ParseRepair { .. } => DiagnosticCode::ParseRepair,
            | Self::Other { .. } => DiagnosticCode::Other,
        }
    }

    /// The localizable template registered for this message.
    #[inline]
    #[must_use]
    pub const fn template(&self) -> DiagnosticTemplate
    {
        self.code().template()
    }
}

impl fmt::Display for DiagnosticMessage
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::TypeMismatch {
                ref expected,
                ref actual,
            } => write!(f, "type mismatch: expected {expected}, actual {actual}"),
            | Self::ShapeMismatch {
                ref expected_shape,
                ref actual,
            } => write!(
                f,
                "type shape mismatch: expected {expected_shape}, actual {actual}"
            ),
            | Self::StuckExpression {
                ref expression,
                ref hint,
            } => write!(f, "no typing rule applies to term {expression}: {hint}"),
            | Self::UnboundVariable { ref name } => write!(f, "variable is unbound: {name}"),
            | Self::GradeOrder {
                ref lower,
                ref upper,
            } => write!(
                f,
                "grade order requirement failed: {lower} ⊑ {upper} does not hold"
            ),
            | Self::UnknownAttribute {
                ref name,
                ref suggestion,
            } => match suggestion.as_ref() {
                | Some(candidate) => {
                    write!(f, "unknown attribute `{name}`; did you mean `{candidate}`?")
                },
                | None => write!(f, "unknown attribute `{name}`"),
            },
            | Self::DuplicateAttribute { ref name } => {
                write!(f, "duplicate attribute `{name}` (single-valued)")
            },
            | Self::MissingAttributePayload { ref name } => {
                write!(f, "attribute `{name}` requires a payload")
            },
            | Self::NonValueAttributePayload { ref name } => {
                write!(
                    f,
                    "attribute `{name}` payload must be a value, not a computation"
                )
            },
            | Self::ShadowedName { ref path } => write!(
                f,
                "`{path}` is a prelude or host name; this declaration takes it, and the policy allows it"
            ),
            | Self::ParseRepair { ref class } => write!(f, "parse repaired: {class}"),
            | Self::Other { ref message } => f.write_str(message),
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::DIAGNOSTIC_CODES;
    use super::DiagnosticCode;
    use super::DiagnosticMessage;

    #[test]
    fn registry_codes_are_dense_unique_and_round_trip()
    {
        for (index, code) in DIAGNOSTIC_CODES.iter().copied().enumerate() {
            assert!(!DIAGNOSTIC_CODES[.. index].contains(&code));
            let spelling = code.to_string();
            let digits = spelling
                .strip_prefix(['E', 'W'])
                .expect("registry codes begin with their severity class");
            assert_eq!(digits.parse::<usize>(), Ok(index + 1));
            assert_eq!(spelling.parse::<DiagnosticCode>(), Ok(code));
        }
    }

    #[test]
    fn one_message_kind_has_one_code_independent_of_arguments()
    {
        let first = DiagnosticMessage::UnboundVariable {
            name: String::from("first"),
        };
        let second = DiagnosticMessage::UnboundVariable {
            name: String::from("second"),
        };

        assert_eq!(first.code(), DiagnosticCode::UnboundVariable);
        assert_eq!(second.code(), DiagnosticCode::UnboundVariable);
        assert_eq!(first.template().to_string(), "variable is unbound: {name}");
        assert_eq!(first.to_string(), "variable is unbound: first");
        assert_eq!(second.to_string(), "variable is unbound: second");
    }

    #[cfg(feature = "codecs")]
    #[test]
    fn code_wire_image_is_its_stable_spelling()
    {
        let json = serde_json::to_string(&DiagnosticCode::TypeMismatch).unwrap();
        assert_eq!(json, r#""E0001""#);
        let decoded = serde_json::from_str::<DiagnosticCode>(&json).unwrap();
        assert_eq!(decoded, DiagnosticCode::TypeMismatch);
    }
}
