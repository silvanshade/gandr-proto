// Bindings-first doctrine (docs/workflow/gfd.md §"The bindings-first
// doctrine"): every `GF`-touching lane in this crate rides the runtime bindings
// in `gandr-workflow-grammatical-framework`; never re-implement what the
// runtime provides (the retired `.gfd` reader is the cautionary example).

//! `GF`-native documentation pipeline.
//!
//! The spec corpus is authored as `GF` abstract-syntax trees(`.gfd`, read by
//! the runtime's expression reader), validated at the mandatory `checkExpr`
//! lane, and rendered by linearization; the `GF`/PGF runtime is reached
//! through the `GfRuntime` trait in `gandr-workflow-grammatical-framework` so
//! the `PyO3` backend can be swapped for a C FFI or pure-Rust backend without
//! touching the pipeline. The prose document classes ([`doc`]) and the shared
//! documentation machinery — the Hayagriva bibliography ([`bibliography`]),
//! the typst leaf compiler ([`typst_leaf`]), canonical `XML` formatting
//! ([`mod@format`]), and the references renderer ([`references`]) — live here
//! the one documentation tool.

extern crate alloc;

use core::fmt;
use std::path::PathBuf;

/// The domain application grammar (the gandr-739 lane's generator).
pub mod appgrammar;
/// Typed Hayagriva bibliography shared by validation and rendering.
pub mod bibliography;
/// Corpus discovery and the document-class `check`/`fmt` orchestration.
pub mod corpus;
/// The prose document classes, a minimal family alongside the `GF` corpus.
pub mod doc;
/// The crate error vocabulary for the `GF` lanes.
pub mod error;
/// Canonical `XML` formatting (idempotent), used as the doc-tool formatter.
pub mod format;
/// Lexicon generation (the corpus-wide `GF` term/cite/anchor modules).
pub mod lexicon;
/// Prose-pacing metrics over the document trees (the gandr-aaq arc).
pub mod metrics;
/// Shared vocabulary types.
pub mod model;
/// The render pipeline: read, validate, post-pass, page.
pub mod pipeline;
/// The per-component references renderer.
pub mod references;
/// Math and diagram leaf compilation to `SVG`.
pub mod typst_leaf;

pub use error::GfDocsError;

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
/// operational failures such as filesystem, `XML`, `YAML`, or typst-tool
/// problems.
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
    /// The typst leaf-compilation tool failed or was unavailable.
    Typst
    {
        /// Stable detail describing the failure.
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
    /// Build a typst-tool error with a stable detail string.
    #[inline]
    #[must_use]
    pub fn typst<Detail>(detail: Detail) -> Self
    where
        Detail: Into<String>,
    {
        Self::Typst {
            detail: detail.into(),
        }
    }

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
            | Self::Typst { ref detail } => write!(f, "typst error: {detail}"),
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
            | Self::Xml { .. } | Self::Yaml { .. } | Self::Typst { .. } | Self::Usage { .. } => {
                None
            },
        }
    }
}
