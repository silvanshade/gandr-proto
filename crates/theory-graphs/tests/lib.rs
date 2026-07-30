//! Consolidated integration-test binary for `gandr-theory-graphs`.

extern crate alloc;

#[cfg(test)]
mod algorithms;
#[cfg(test)]
mod determinism;
#[cfg(test)]
mod partition_refine;
#[cfg(test)]
mod prec;
#[cfg(test)]
mod walk;
