//! Consolidated integration-test binary for `gandr-core-checker`.
//!
//! It holds the suites that need the shared free generators, which live one
//! tier above this crate in `gandr-core-checker-tools`. An inline `cfg(test)`
//! module could not use them: that build is a distinct crate instance from the
//! library the generator crate links against, so the two spellings of a type
//! would not unify.

extern crate alloc;

#[cfg(test)]
mod interning;
#[cfg(test)]
mod marking;
