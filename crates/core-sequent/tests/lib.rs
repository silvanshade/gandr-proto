//! Consolidated integration-test binary for `gandr-core-sequent`.

extern crate alloc;

#[cfg(test)]
mod common;

#[cfg(test)]
mod conformance_soundness;
#[cfg(test)]
mod corpus_differential;
#[cfg(test)]
mod corpus_totality;
#[cfg(test)]
mod csl_fibration;
#[cfg(test)]
mod differential;
#[cfg(test)]
mod focus_properties;
#[cfg(test)]
mod kernel_corpus_partition;
#[cfg(test)]
mod kernel_export_gate;
#[cfg(test)]
mod packages;
