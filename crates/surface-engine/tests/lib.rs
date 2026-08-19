//! Consolidated integration-test binary for `gandr-surface-engine`.

extern crate alloc;
extern crate proptest as proptest_crate;

#[cfg(test)]
mod common;

#[cfg(test)]
mod acceptance;
#[cfg(test)]
mod attributes;
#[cfg(test)]
mod circuit;
#[cfg(test)]
mod circuit_desc;
#[cfg(test)]
mod circuit_embed;
#[cfg(test)]
mod circuit_ports;
#[cfg(test)]
mod desc_cells;
#[cfg(test)]
mod desc_elab;
#[cfg(test)]
mod diag;
#[cfg(test)]
mod diag_attr;
#[cfg(test)]
mod diag_frames;
#[cfg(test)]
mod diag_obligations;
#[cfg(test)]
mod edit;
#[cfg(test)]
mod edit_extra;
#[cfg(test)]
mod export;
#[cfg(test)]
mod goals_extra;
#[cfg(test)]
mod incremental;
#[cfg(test)]
mod kernel;
#[cfg(test)]
mod live_match;
#[cfg(test)]
mod matrix;
#[cfg(test)]
mod modules;
#[cfg(test)]
mod namespace;
#[cfg(test)]
mod origin_identity;
#[cfg(test)]
mod packages;
#[cfg(test)]
mod pattern_holes;
#[cfg(test)]
mod proptest;
#[cfg(test)]
mod recognition;
#[cfg(test)]
mod run;
#[cfg(test)]
mod seam;
#[cfg(test)]
mod session;
#[cfg(test)]
mod share;
#[cfg(test)]
mod surface;
#[cfg(test)]
mod total;
#[cfg(test)]
mod types;
