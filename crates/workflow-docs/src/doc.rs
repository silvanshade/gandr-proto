//! The prose document classes: research records, workflow docs, and the
//! per-crate lean-tier status narrative.
//!
//! These classes grow the doc tool beyond the spec corpus to the surrounding
//! Markdown tail (`gandr-712`). They share one minimal block/inline substrate
//! ([`model`]) parsed by a single parse-equals-validate pass ([`parse`]) and
//! checked against a small per-class schema ([`validate`]).
//!
//! The canonical `XML` formatter ([`crate::format`]) is class-agnostic, so
//! these classes need no new formatting code — only the `treefmt` `docs-xml`
//! glob is widened to their roots.

/// Typed model of the prose document substrate (the normative schema).
pub mod model;
/// Parse-equals-validate pass: `XML` text to [`model::DocRecord`] with
/// structural diagnostics.
///
/// [`model::DocRecord`]: crate::doc::model::DocRecord
pub mod parse;
/// Corpus-level class-schema and cross-reference validation.
pub mod validate;
