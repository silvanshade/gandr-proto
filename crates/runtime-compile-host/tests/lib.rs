//! Consolidated integration-test binary for `gandr-runtime-compile-host`.
//!
//! The suite splits by what it needs. `lowering`, `rendering` and `contract`
//! need nothing but this workspace and run everywhere, merge wall included.
//! `bridge` needs the linked host, so it exists only under the `full` feature
//! that links it; there is no absent-host case to report, because a build that
//! reached this binary already resolved every entry.

extern crate alloc;

#[cfg(test)]
#[cfg(feature = "full")]
mod bridge;
#[cfg(test)]
mod contract;
#[cfg(test)]
mod lowering;
#[cfg(test)]
mod rendering;

#[cfg(test)]
mod programs;
