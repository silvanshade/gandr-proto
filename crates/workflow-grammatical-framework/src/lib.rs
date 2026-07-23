//! `GF`/`PGF` interop for the gandr documentation pipeline.
//!
//! This crate is the physical form of the proposal's internalization seam
//! (proposal-docs-gf-pipeline.md §4, internalizing-gf.md): everything outside
//! talks to the [`rt::GfRuntime`] trait only, so the `PyO3` backend can be
//! swapped for a C FFI or pure-Rust backend without touching the pipeline.
//! The `Python`/pyo3 dependency is quarantined here — it is the only crate in
//! the workspace that links the interpreter.
//!
//! The [`sexp`] module is the B′ `.gfd` surface: the canonical builder
//! (printer) and the reader, so the corpus's trees round-trip through Rust
//! without a `GF` runtime in the loop.

pub mod error;
pub mod rt;
pub mod sexp;

pub use error::GfError;
