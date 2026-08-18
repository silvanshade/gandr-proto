//! Test-facing machinery for `gandr-core-checker`, held one tier above it.
//!
//! The checker's own shipping path never links this crate. What lives here is
//! the machinery that only test targets need, extracted so the checker crate
//! carries the checking path and nothing else:
//!
//! - [`strategies`] — the *free* proptest generators over the core CBPV syntax
//!   and types (grades, binder names, leaf and recursive value and computation
//!   types, hole identifiers), shared by this crate's own conformance suite, by
//!   `gandr-core-checker`'s inline property tests, and by
//!   `gandr-surface-engine`'s edit and proptest suites;
//! - the conformance suite itself, which is this crate's `conformance` test
//!   target rather than library code: it pins the *step-for-step* agreement
//!   between the checker's two realizations, the recursive bidirectional
//!   checker and the defunctionalized typing machine.
//!
//! The *type-directed, well-typed* generators stay with the conformance suite
//! that drives them; only the free generators are library surface, because
//! only they have consumers outside that harness.

pub mod strategies;
