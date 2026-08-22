//! Shared types for Rust AST-backed workspace gates.
//!
//! The binary crate keeps command-line parsing thin while analyzer modules own
//! the domain-specific `syn` traversal for contracts, CI workflow contracts,
//! and graph-boundary checks.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use std::path::PathBuf;

/// Convert a value into an explicitly selected semantic boundary type.
#[inline]
#[must_use]
pub fn semantic_value<T, Value>(value: Value) -> T
where
    Value: Into<T>,
{
    value.into()
}

/// Define a named transparent borrowed-string boundary for gate-local domains.
#[macro_export]
macro_rules! semantic_str {
    ($vis:vis struct $name:ident) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        $vis struct $name<'text>(pub &'text str);

        impl<'text> From<&'text str> for $name<'text> {
            #[inline]
            fn from(value: &'text str) -> Self {
                Self(value)
            }
        }

        impl<'text> From<&'text alloc::string::String> for $name<'text> {
            #[inline]
            fn from(value: &'text alloc::string::String) -> Self {
                Self(value.as_str())
            }
        }

        impl<'text> From<$name<'text>> for &'text str {
            #[inline]
            fn from(value: $name<'text>) -> Self {
                value.0
            }
        }

        impl AsRef<str> for $name<'_> {
            #[inline]
            fn as_ref(&self) -> &str {
                self.0
            }
        }

        impl core::fmt::Display for $name<'_> {
            #[inline]
            fn fmt(
                &self,
                f: &mut core::fmt::Formatter<'_>,
            ) -> core::fmt::Result {
                f.write_str(self.0)
            }
        }
    };
}

/// Define a named transparent optional borrowed-string boundary.
#[macro_export]
macro_rules! semantic_optional_str {
    ($vis:vis struct $name:ident) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        $vis struct $name<'text>(pub Option<&'text str>);

        impl<'text> From<Option<&'text str>> for $name<'text> {
            #[inline]
            fn from(value: Option<&'text str>) -> Self {
                Self(value)
            }
        }


        impl<'text> From<$name<'text>> for Option<&'text str> {
            #[inline]
            fn from(value: $name<'text>) -> Self {
                value.0
            }
        }
    };
}

/// Define a named transparent borrowed-bytes boundary.
#[macro_export]
macro_rules! semantic_bytes {
    ($vis:vis struct $name:ident) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        $vis struct $name<'bytes>(pub &'bytes [u8]);

        impl<'bytes> From<&'bytes [u8]> for $name<'bytes> {
            #[inline]
            fn from(value: &'bytes [u8]) -> Self {
                Self(value)
            }
        }

        impl<'bytes, const N: usize> From<&'bytes [u8; N]> for $name<'bytes> {
            #[inline]
            fn from(value: &'bytes [u8; N]) -> Self {
                Self(value.as_slice())
            }
        }

        impl<'bytes> From<&'bytes alloc::vec::Vec<u8>> for $name<'bytes> {
            #[inline]
            fn from(value: &'bytes alloc::vec::Vec<u8>) -> Self {
                Self(value.as_slice())
            }
        }

        impl<'bytes> From<$name<'bytes>> for &'bytes [u8] {
            #[inline]
            fn from(value: $name<'bytes>) -> Self {
                value.0
            }
        }
    };
}

/// Define a named transparent optional-copy boundary for scalar gate-local
/// domains.
#[macro_export]
macro_rules! semantic_optional_copy {
    ($vis:vis struct $name:ident($inner:ty)) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        $vis struct $name(pub Option<$inner>);

        impl From<Option<$inner>> for $name {
            #[inline]
            fn from(value: Option<$inner>) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Option<$inner> {
            #[inline]
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

/// Define a named transparent copy boundary for scalar gate-local domains.
#[macro_export]
macro_rules! semantic_copy {
    ($vis:vis struct $name:ident($inner:ty)) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        $vis struct $name(pub $inner);

        impl From<$inner> for $name {
            #[inline]
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $inner {
            #[inline]
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

crate::semantic_str!(pub struct SText);
crate::semantic_str!(pub struct ValueText);
crate::semantic_optional_str!(pub struct OptionalValueText);

/// Contract documentation, CI workflow, and nextest-witness gate
/// implementation.
pub mod contracts;
/// Per-production-file coverage policy and ratchet implementation.
pub mod coverage;
/// Bounded parser-only facade shared by CLI parsing and AFL fuzzing.
pub mod parser_facade;
#[cfg(feature = "fuzzing")]
pub mod fuzzing
{
    //! Feature-gated AFL fuzzing facade for pure gate parsers.
    //!
    //! This module preserves the public AFL target API while delegating all
    //! parser-only work to [`crate::parser_facade`].

    pub use crate::parser_facade::exercise_fuzz_input;
}
/// Documentation manifest, reference, page-balance, and rumdl gates.
pub mod docs;
/// Embedded-syntax raw-string policy gate for Rust tests.
pub mod embedded_syntax;
/// Graph dependency-boundary gate implementation.
pub mod graph_boundary;
/// Maintenance range planning and watermark publication.
pub mod maintenance;
/// Contained mutation campaign planning, execution, and publication.
pub mod mutants;
/// Repository dependency-graph policy.
pub mod project;
/// Source-level soundness-oracle policy.
pub mod source_policy;
/// Shared process and filesystem support used by gate domains.
pub mod support;
pub mod test_registration;
/// Deterministic merge and push workflow orchestration.
pub mod workflow;

/// Result type returned by gate analyzers.
pub type GateResult = Result<Vec<Finding>, GateError>;

/// A stable diagnostic emitted by a gate when a contract is violated.
///
/// # Contract
/// - ensures: display renders fields in `kind`, `package`, `path`,
///   `declaration`, then `detail` order, using a single `key=value` fragment
///   per field.
/// - provides: a comparable diagnostic row for shell wrappers and regression
///   tests.
/// - panics: none.
/// - intension: display streams fields directly and normalizes each field
///   through the declared stable whitespace projection.
///
/// # Adequacy
/// - hypothesis: L3 only — field-order, key spelling, newline, carriage-return,
///   and tab decisions are killed by one finding containing every unstable
///   whitespace byte and an exact rendered string observation.
/// - witness: `gandr_workflow_gates::tests::finding_display_preserves_field_order_and_normalizes_whitespace`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding
{
    /// Stable finding kind, such as `missing-contract` or `forbidden-boundary`.
    pub kind: String,
    /// Workspace package that owns the declaration, or an empty string when not
    /// applicable.
    pub package: String,
    /// Source path associated with the finding, rendered as analyzer-provided
    /// text.
    pub path: String,
    /// Declaration or symbol associated with the finding, or an empty string.
    pub declaration: String,
    /// Human-readable stable detail for the violation.
    pub detail: String,
}

impl Finding
{
    /// Build a stable finding from analyzer-provided fields.
    #[inline]
    #[must_use]
    pub fn new<Kind, Package, Path, Declaration, Detail>(
        kind: Kind,
        package: Package,
        path: Path,
        declaration: Declaration,
        detail: Detail,
    ) -> Self
    where
        Kind: Into<String>,
        Package: Into<String>,
        Path: Into<String>,
        Declaration: Into<String>,
        Detail: Into<String>,
    {
        Self {
            kind: kind.into(),
            package: package.into(),
            path: path.into(),
            declaration: declaration.into(),
            detail: detail.into(),
        }
    }
}

impl fmt::Display for Finding
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str("kind=")?;
        write_stable_value(f, &self.kind)?;
        f.write_str(" package=")?;
        write_stable_value(f, &self.package)?;
        f.write_str(" path=")?;
        write_stable_value(f, &self.path)?;
        f.write_str(" declaration=")?;
        write_stable_value(f, &self.declaration)?;
        f.write_str(" detail=")?;
        write_stable_value(f, &self.detail)
    }
}

/// Typed operational error returned by gate analyzers and CLI parsing.
///
/// # Contract
/// - ensures: display renders usage and operational details exactly after their
///   stable prefixes, and renders source-backed variants with their stable
///   prefix, stable payload label, and normalized wrapped-source display.
/// - provides: typed usage, I/O, Rust parse, JSON, and operational failure
///   surfaces.
/// - fails: semantic gate violations are not represented here; they are
///   returned as [`Finding`] values by successful analyzer runs.
/// - panics: none.
/// - intension: error display streams directly and normalizes path/source
///   payloads through the declared stable whitespace projection.
///
/// # Adequacy
/// - hypothesis: L3 only — usage/operational exact strings, source-backed
///   stable prefixes, path/source-name payload normalization, and
///   source-exposure decisions are killed by constructing every variant and
///   observing display and [`core::error::Error::source`].
/// - witness: `gandr_workflow_gates::tests::gate_error_display_and_sources_are_exact`
#[derive(Debug)]
pub enum GateError
{
    /// Command-line usage failed validation.
    Usage
    {
        /// Stable usage failure detail.
        detail: String,
    },
    /// Filesystem I/O failed for an input path.
    Io
    {
        /// Path being read or inspected.
        path: PathBuf,
        /// Original I/O error.
        source: std::io::Error,
    },
    /// Rust source parsing failed for an input path.
    RustParse
    {
        /// Rust source path being parsed.
        path: PathBuf,
        /// Original `syn` parser error.
        source: syn::Error,
    },
    /// JSON parsing failed for a fixture or process output.
    Json
    {
        /// JSON source path or logical source name.
        source_name: String,
        /// Original JSON parser error.
        source: serde_json::Error,
    },
    /// A gate operation failed without a richer typed source.
    Operational
    {
        /// Stable operational failure detail.
        detail: String,
    },
}

impl GateError
{
    /// Build a usage error with a stable detail string.
    #[inline]
    #[must_use]
    pub fn usage<Detail>(detail: Detail) -> Self
    where
        Detail: Into<String>,
    {
        Self::Usage {
            detail: detail.into(),
        }
    }

    /// Build an operational error with a stable detail string.
    #[inline]
    #[must_use]
    pub fn operational<Detail>(detail: Detail) -> Self
    where
        Detail: Into<String>,
    {
        Self::Operational {
            detail: detail.into(),
        }
    }
}

impl fmt::Display for GateError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::Usage { ref detail } => {
                f.write_str("usage error: ")?;
                write_stable_value(f, detail)
            },
            | Self::Io {
                ref path,
                ref source,
            } => {
                f.write_str("io error: path=")?;
                write_stable_display(f, &path.display())?;
                f.write_str(" detail=")?;
                write_stable_display(f, source)
            },
            | Self::RustParse {
                ref path,
                ref source,
            } => {
                f.write_str("rust parse error: path=")?;
                write_stable_display(f, &path.display())?;
                f.write_str(" detail=")?;
                write_stable_display(f, source)
            },
            | Self::Json {
                ref source_name,
                ref source,
            } => {
                f.write_str("json error: source=")?;
                write_stable_value(f, source_name)?;
                f.write_str(" detail=")?;
                write_stable_display(f, source)
            },
            | Self::Operational { ref detail } => {
                f.write_str("operational error: ")?;
                write_stable_value(f, detail)
            },
        }
    }
}

impl core::error::Error for GateError
{
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)>
    {
        match *self {
            | Self::Io { ref source, .. } => Some(source),
            | Self::RustParse { ref source, .. } => Some(source),
            | Self::Json { ref source, .. } => Some(source),
            | Self::Usage { .. } | Self::Operational { .. } => None,
        }
    }
}

/// Write a deterministic single-line display value without an intermediate
/// allocation.
///
/// # Contract
/// - ensures: writes the display output with newline, carriage-return, and tab
///   characters replaced by spaces and all other characters unchanged.
/// - provides: normalization for display-only values that do not expose `&str`.
/// - fails: returns the formatter error produced by the destination formatter.
/// - panics: none.
/// - intension: streams through [`fmt::Write`] without allocating an
///   intermediate `String`.
///
/// # Errors
/// Returns the destination formatter error when writing fails.
///
/// # Adequacy
/// - hypothesis: L3 only — newline, carriage-return, tab, ordinary-character,
///   and destination-failure paths are killed by exact rendered diagnostics and
///   an exact propagated formatter-error observation.
/// - witness: `gandr_workflow_gates::tests::gate_error_display_and_sources_are_exact`
/// - witness: `gandr_workflow_gates::tests::stable_formatters_propagate_destination_failures`
fn write_stable_display<Value>(
    formatter: &mut fmt::Formatter<'_>,
    value: &Value,
) -> fmt::Result
where
    Value: fmt::Display + ?Sized,
{
    let mut stable_formatter = StableFormatter { formatter };
    fmt::write(&mut stable_formatter, format_args!("{value}"))
}

/// Formatting adapter that normalizes strings before forwarding them.
#[repr(transparent)]
struct StableFormatter<'formatter, 'output>
{
    /// Destination formatter.
    formatter: &'formatter mut fmt::Formatter<'output>,
}

impl fmt::Write for StableFormatter<'_, '_>
{
    fn write_str(
        &mut self,
        s: &str,
    ) -> fmt::Result
    {
        write_stable_value(self.formatter, s)
    }
}

/// Write a deterministic single-line field value.
///
/// # Contract
/// - ensures: writes newline, carriage-return, and tab characters as plain
///   spaces while preserving every other character exactly.
/// - provides: the stable whitespace projection used by finding and error
///   display.
/// - fails: returns the formatter error produced by the destination formatter.
/// - panics: none.
/// - intension: scans `value` once in character order and streams each
///   projected character to the destination formatter.
///
/// # Errors
/// Returns the destination formatter error when writing fails.
///
/// # Adequacy
/// - hypothesis: L3 only — the three normalized whitespace cases, one preserved
///   non-whitespace case, and destination-failure propagation are killed by
///   exact rendered diagnostic and formatter-error observations.
/// - witness: `gandr_workflow_gates::tests::finding_display_preserves_field_order_and_normalizes_whitespace`
/// - witness: `gandr_workflow_gates::tests::stable_formatters_propagate_destination_failures`
fn write_stable_value<'semantic, Value>(
    formatter: &mut fmt::Formatter<'_>,
    value: Value,
) -> fmt::Result
where
    Value: Into<ValueText<'semantic>>,
{
    let value = value.into().0;
    for character in value.chars() {
        match character {
            | '\n' | '\r' | '\t' => formatter.write_str(" ")?,
            | stable_character => write!(formatter, "{stable_character}")?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests
{
    //! Unit witnesses for shared gate finding and error contracts.

    use core::error::Error;
    use core::fmt::Write as _;

    use super::*;

    /// Finding display keeps field order stable and normalizes record-splitting
    /// whitespace.
    #[test]
    fn finding_display_preserves_field_order_and_normalizes_whitespace()
    {
        let finding = Finding::new(
            "missing\ncontract",
            "pkg\rname",
            "src\tlib.rs",
            "fn target",
            "line\none\rtwo\tthree",
        );

        assert_eq!(
            "kind=missing contract package=pkg name path=src lib.rs declaration=fn target detail=line one two three",
            finding.to_string(),
        );
    }

    /// Gate error display and source exposure are exact for every variant.
    #[test]
    fn gate_error_display_and_sources_are_exact() -> Result<(), Box<dyn Error>>
    {
        let usage = GateError::usage("bad\nargument");
        assert_eq!("usage error: bad argument", usage.to_string());
        assert!(Error::source(&usage).is_none());

        let io = GateError::Io {
            path: PathBuf::from("src\tlib.rs"),
            source: std::io::Error::other("disk\nfull"),
        };
        assert_eq!("io error: path=src lib.rs detail=disk full", io.to_string(),);
        assert!(Error::source(&io).is_some());

        let Err(rust_parse_source) = syn::parse_file("fn {")
        else {
            return Err(Box::new(std::io::Error::other(
                "invalid Rust fixture parsed successfully",
            )));
        };
        let rust_parse = GateError::RustParse {
            path: PathBuf::from("bad\rsource.rs"),
            source: rust_parse_source,
        };
        assert!(
            rust_parse
                .to_string()
                .starts_with("rust parse error: path=bad source.rs detail="),
            "rust parse display should carry the stable prefix and normalized path",
        );
        assert!(Error::source(&rust_parse).is_some());

        let Err(json_source) = serde_json::from_str::<serde_json::Value>("{")
        else {
            return Err(Box::new(std::io::Error::other(
                "invalid JSON fixture parsed successfully",
            )));
        };
        let json = GateError::Json {
            source_name: String::from("fixture\tjson"),
            source: json_source,
        };
        assert!(
            json.to_string()
                .starts_with("json error: source=fixture json detail="),
            "json display should carry the stable prefix and normalized source name",
        );
        assert!(Error::source(&json).is_some());

        let operational = GateError::operational("missing\ttool");
        assert_eq!("operational error: missing tool", operational.to_string());
        assert!(Error::source(&operational).is_none());
        Ok(())
    }

    /// Stable formatting helpers propagate destination formatter errors
    /// exactly.
    #[test]
    fn stable_formatters_propagate_destination_failures()
    {
        let mut value_writer = FailingWriter;
        assert!(write!(&mut value_writer, "{}", StableValue("value")).is_err());

        let mut display_writer = FailingWriter;
        assert!(
            write!(
                &mut display_writer,
                "{}",
                StableDisplay(std::io::Error::other("display"))
            )
            .is_err()
        );
    }

    /// Writer that rejects every string write with `fmt::Error`.
    struct FailingWriter;

    impl fmt::Write for FailingWriter
    {
        fn write_str(
            &mut self,
            _s: &str,
        ) -> fmt::Result
        {
            Err(fmt::Error)
        }
    }

    /// Display adapter that exposes `write_stable_value` to `write!`.
    #[repr(transparent)]
    struct StableValue<'value>(&'value str);

    impl fmt::Display for StableValue<'_>
    {
        fn fmt(
            &self,
            f: &mut fmt::Formatter<'_>,
        ) -> fmt::Result
        {
            write_stable_value(f, self.0)
        }
    }

    /// Display adapter that exposes `write_stable_display` to `write!`.
    #[repr(transparent)]
    struct StableDisplay<Value>(Value);

    impl<Value> fmt::Display for StableDisplay<Value>
    where
        Value: fmt::Display,
    {
        fn fmt(
            &self,
            f: &mut fmt::Formatter<'_>,
        ) -> fmt::Result
        {
            write_stable_display(f, &self.0)
        }
    }
}
