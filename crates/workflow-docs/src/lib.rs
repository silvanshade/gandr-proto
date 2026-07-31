//! The gandr documentation tool: the prose document classes.
//!
//! Documents in the three authored classes ([`doc`]) are `XML`, and parsing
//! _is_ validation — banner presence, status presence, label define-once,
//! label and citation resolution, and the per-class schema are all enforced by
//! the one pass. The shared machinery — the Hayagriva bibliography
//! ([`bibliography`]) and canonical `XML` formatting ([`mod@format`]) — lives
//! here with it.

extern crate alloc;

use core::fmt;
use std::path::PathBuf;

/// Typed Hayagriva bibliography read for citation resolution.
pub mod bibliography;
/// Corpus discovery and the document-class `check`/`fmt` orchestration.
pub mod corpus;
/// The prose document classes.
pub mod doc;
/// Canonical `XML` formatting (idempotent), used as the doc-tool formatter.
pub mod format;
/// Shared vocabulary types.
pub mod model;

/// Stable machine-readable diagnostic category.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DiagnosticCode(&'static str);

impl From<&'static str> for DiagnosticCode
{
    #[inline]
    fn from(code: &'static str) -> Self
    {
        Self(code)
    }
}

impl AsRef<str> for DiagnosticCode
{
    #[inline]
    fn as_ref(&self) -> &'static str
    {
        self.0
    }
}

impl PartialEq<&str> for DiagnosticCode
{
    #[inline]
    fn eq(
        &self,
        other: &&str,
    ) -> bool
    {
        self.0 == *other
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
        f.write_str(self.0)
    }
}

/// Non-fatal specification violation reported by the parse-validate pass.
///
/// A run that produces one or more diagnostics fails the check; the diagnostics
/// are the machine-stable explanation.
///
/// # Contract
/// - ensures: [`fmt::Display`] renders `location: code: message` on one line.
/// - provides: a comparable, sortable violation row for the reporter.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Diagnostic
{
    /// Stable diagnostic kind, such as `duplicate-id` or `unresolved-cite`.
    pub code: DiagnosticCode,
    /// Human-readable, single-line explanation of the violation.
    pub message: String,
    /// Source location: the file path and, where known, the element or
    /// attribute at fault.
    pub location: String,
}

impl Diagnostic
{
    /// Build a diagnostic from a stable code, a location, and a message.
    #[inline]
    #[must_use]
    pub fn new<Location, Message>(
        code: DiagnosticCode,
        location: Location,
        message: Message,
    ) -> Self
    where
        Location: Into<String>,
        Message: Into<String>,
    {
        Self {
            code,
            message: message.into(),
            location: location.into(),
        }
    }
}

impl fmt::Display for Diagnostic
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        write!(f, "{}: {}: {}", self.location, self.code, self.message)
    }
}

/// Typed operational error returned by the doc tool.
///
/// Semantic specification violations are not errors; they are returned as
/// [`Diagnostic`] values by a successful run. This type carries only
/// operational failures such as filesystem, `XML`, or `YAML` problems.
#[derive(Debug)]
pub enum DocError
{
    /// A filesystem operation failed for a path.
    Io
    {
        /// Path being read, written, or inspected.
        path: PathBuf,
        /// Underlying input/output error.
        source: std::io::Error,
    },
    /// An `XML` document was not well formed.
    Xml
    {
        /// Path of the offending document.
        path: PathBuf,
        /// Stable detail describing the malformation.
        detail: String,
    },
    /// A `YAML` document (the references file) was not well formed.
    Yaml
    {
        /// Path of the offending document.
        path: PathBuf,
        /// Stable detail describing the malformation.
        detail: String,
    },
    /// Command-line usage was invalid.
    Usage
    {
        /// Stable usage detail.
        detail: String,
    },
}

impl DocError
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
}

impl fmt::Display for DocError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::Io {
                ref path,
                ref source,
            } => write!(f, "io error: path={} detail={source}", path.display()),
            | Self::Xml {
                ref path,
                ref detail,
            } => write!(f, "xml error: path={} detail={detail}", path.display()),
            | Self::Yaml {
                ref path,
                ref detail,
            } => write!(f, "yaml error: path={} detail={detail}", path.display()),
            | Self::Usage { ref detail } => write!(f, "usage error: {detail}"),
        }
    }
}

impl core::error::Error for DocError
{
    #[inline]
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)>
    {
        match *self {
            | Self::Io { ref source, .. } => Some(source),
            | Self::Xml { .. } | Self::Yaml { .. } | Self::Usage { .. } => None,
        }
    }
}
