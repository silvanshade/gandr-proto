//! Explicit single integration target for the `gandr-workflow-gates` crate.
//!
//! Cargo auto-discovery is disabled for this crate so every integration suite
//! is compiled exactly once through this target while preserving the historical
//! module paths used by contract witnesses.

// Crate-local lint-wall overrides, parked for triage (gandr-0ze): see the
// matching block in `src/lib.rs`. Remove entries as their sites are
// remediated.
#![allow(
    clippy::as_conversions,
    clippy::derive_partial_eq_without_eq,
    clippy::explicit_auto_deref,
    clippy::field_scoped_visibility_modifiers,
    clippy::if_then_some_else_none,
    clippy::indexing_slicing,
    clippy::needless_borrows_for_generic_args,
    clippy::pattern_type_mismatch,
    reason = "ported crate predates the current lint wall; parked for triage (gandr-0ze)"
)]

/// Contract-documentation gate integration witnesses.
#[cfg(test)]
#[path = "contracts.rs"]
mod contracts;

/// Graph-boundary gate integration witnesses.
#[cfg(test)]
#[path = "graph_boundary.rs"]
mod graph_boundary;

/// CI workflow contract integration witnesses.
#[cfg(test)]
#[path = "ci_contracts.rs"]
mod ci_contracts;

/// Unified CLI tooling integration witnesses.
#[cfg(test)]
#[path = "tooling.rs"]
mod tooling;

/// Legacy Nushell regression parity ledger.
#[cfg(test)]
#[path = "legacy_parity.rs"]
mod legacy_parity;
