//! The dynamic-graph test target.
//!
//! `differential` is the zero-drift gate: every incremental verdict is compared
//! against the batch acyclicity answer over the same edge set.
//! `probe` measures where the graph-theoretic maintenance stops agreeing with
//! the offset-carrying one.

#[cfg(test)]
mod differential;
#[cfg(test)]
mod probe;
#[cfg(test)]
mod support;
