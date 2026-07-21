//! Integration-test aggregator for the checked PBG grammar core.
//!
//! `autotests = false` funnels the crate's integration tests through this one
//! binary. It wires the parser-free grammar unit-golden suites — the PBG
//! construction contracts (`pbg`) and the walk-index / comparison-table /
//! reachability contracts (`walk`) — and, at rung F2, the parser-driven
//! surface-acceptance `contracts` suite, restored over the melder push machine
//! (`gandr-surface-parser`, a dev-only dependency — the deliberate cycle-break
//! direction).
//!
//! The tree-sitter parity / node-types drift gates remain parked for F6 and
//! return here as they land — see `docs/STATUS.md`.

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

// Included as `surface_contracts` (not `contracts`) so the outer module name
// differs from the file's inner `#[cfg(test)] mod contracts`, avoiding the
// module-inception lint while keeping the tests inside a test module.
#[path = "contracts.rs"]
mod surface_contracts;

#[path = "pbg.rs"]
mod pbg;

#[path = "walk.rs"]
mod walk;
