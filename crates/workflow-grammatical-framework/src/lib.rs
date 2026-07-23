// Bindings-first doctrine (docs/workflow/gfd.md §"The bindings-first
// doctrine"): never re-implement functionality the `GF`/PGF runtime already
// provides — reading, parsing, validating, linearizing, and introspection all
// ride these bindings. The one blessed exception is the B′ printer in `sexp`
// (the `GF` toolchain ships no formatting/canonical-layout tooling); the `.gfd`
// reader that once shadowed `readExpr` was removed (owner directive,
// 2026-07-23).

//! `GF`/`PGF` interop for the gandr documentation pipeline.
//!
//! This crate is the physical form of the proposal's internalization seam
//! (proposal-docs-gf-pipeline.md §4, internalizing-gf.md): everything outside
//! talks to the [`rt::GfRuntime`] trait only, so the `PyO3` backend can be
//! swapped for a C FFI or pure-Rust backend without touching the pipeline.
//! The `Python`/pyo3 dependency is quarantined here — it is the only crate in
//! the workspace that links the interpreter.
//!
//! The [`sexp`] module is the B′ `.gfd` surface: the crate's expression tree
//! type and its canonical printer (the `fmt` lane's engine); trees are read by
//! the runtime, never by a house parser.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the unit and property tests \
                  readable (docs/workflow/rust.md)"
    )
)]

pub mod error;
pub mod rt;
pub mod sexp;

extern crate alloc;

pub use error::GfError;
