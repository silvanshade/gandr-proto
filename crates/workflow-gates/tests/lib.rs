//! Explicit single integration target for the `gandr-workflow-gates` crate.
//!
//! Cargo auto-discovery is disabled for this crate so every integration suite
//! is compiled exactly once through this target while preserving the historical
//! module paths used by contract witnesses.

extern crate alloc;

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
