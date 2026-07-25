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

extern crate alloc;

mod contracts;

mod pbg;

mod walk;
