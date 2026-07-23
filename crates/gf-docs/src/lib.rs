//! `GF`-native documentation pipeline (gandr-5n6).
//!
//! The corpus is authored as `GF` abstract-syntax trees (`.gfd`, read by the
//! runtime's expression reader), validated at the mandatory `checkExpr` lane,
//! and rendered by linearization. The legacy `XML` pipeline is reused only as
//! the migration parser ([`migrate`]); the `GF`/PGF runtime is reached through
//! the [`GfRuntime`] trait in `gandr-workflow-grammatical-framework` so the
//! `PyO3` backend can be swapped for a C FFI or pure-Rust backend without
//! touching the pipeline.

pub mod error;
pub mod lexicon;
pub mod migrate;
pub mod pipeline;

pub use error::GfDocsError;
