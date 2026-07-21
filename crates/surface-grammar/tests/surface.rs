//! Integration-test aggregator for the checked PBG grammar core.
//!
//! `autotests = false` funnels the crate's integration tests through this one
//! binary. At the F1 rung it wires the two parser-free grammar unit-golden
//! suites: the PBG construction contracts (`pbg`) and the walk-index /
//! comparison-table / reachability contracts (`walk`).
//!
//! The parser-driven surface-acceptance contracts (which need the melder push
//! machine) and the tree-sitter parity / node-types drift gates are parked for
//! later rungs and return here as they land — see `docs/STATUS.md`.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps grammar contract tests readable (docs/workflow/rust.md)"
    )
)]

extern crate alloc;

#[path = "pbg.rs"]
mod pbg;

#[path = "walk.rs"]
mod walk;
