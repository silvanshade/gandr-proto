//! Static specification-documentation tool for the gandr component vocabulary.
//!
//! The crate is the normative schema of a custom `XML` component vocabulary
//! (decision record `gandr-fcw.8`): a typed Rust model, a parse-equals-validate
//! pass, canonical `XML` formatting, and a no-JavaScript static `HTML` build.
//! Math and diagram leaves are compiled to `SVG` by shelling out to the pinned
//! typst command-line tool (see [`typst_leaf`]).
//!
//! The public surface is deliberately small: the typed model in [`model`], the
//! [`Diagnostic`] and [`DocError`] reporting types here, and the orchestration
//! entry points in [`corpus`] ([`corpus::check`], [`corpus::build`],
//! [`corpus::format_paths`]).

extern crate alloc;

use alloc::string::String;
use core::fmt;
use std::path::PathBuf;

/// Corpus discovery and the `check`, `build`, and `fmt` orchestration.
pub mod corpus;
/// Canonical `XML` formatting (idempotent), used as the doc-tool formatter.
pub mod format;
/// Typed model of the component vocabulary (the normative schema).
pub mod model;
/// Parse-equals-validate pass: `XML` text to [`model::Document`] with
/// structural diagnostics.
pub mod parse;
/// No-JavaScript static `HTML` rendering of a validated corpus.
pub mod render;
/// Math and diagram leaf compilation to `SVG` via the pinned typst tool.
pub mod typst_leaf;
/// Corpus-level cross-file validation.
///
/// Checks identifier uniqueness, define-once, and term, cite, and provenance
/// resolution.
pub mod validate;

/// Non-fatal specification violation reported by the parse-validate pass.
///
/// A run that produces one or more diagnostics fails the check; the diagnostics
/// are the machine-stable explanation.
///
/// # Contract
/// - ensures: [`fmt::Display`] renders `location: code: message` on one line.
/// - provides: a comparable, sortable violation row for the reporter.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct Diagnostic
{
    /// Stable diagnostic kind, such as `duplicate-id` or `unresolved-cite`.
    pub code: &'static str,
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
        code: &'static str,
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
#[non_exhaustive]
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
