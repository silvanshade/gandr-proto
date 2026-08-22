//! Consolidated integration-test binary for `gandr-surface-corpus`: the corpus
//! walker suites (ADR-52 / ADR-84) funnel through this one binary.

#[cfg(test)]
mod corpus;

#[cfg(test)]
mod cat_shape_model;

#[cfg(test)]
mod flagship_probe;
