//! Consolidated integration-test binary for `gandr-runtime-compile-host`.
//!
//! The suite splits by what it needs. `lowering`, `rendering` and `contract`
//! need nothing but this workspace and run everywhere, merge wall included.
//! `bridge` needs a built compilation host and reports its absence rather than
//! failing, because the host's toolchain is discovered rather than pinned.

extern crate alloc;

#[cfg(test)]
mod bridge;
#[cfg(test)]
mod contract;
#[cfg(test)]
mod lowering;
#[cfg(test)]
mod rendering;

#[cfg(test)]
mod programs;
