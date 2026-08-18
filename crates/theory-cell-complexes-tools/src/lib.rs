//! **The second inhabitant** of
//! [`gandr_theory_cell_complexes::alphabet::CellAlphabet`], and the adversarial
//! variants built over it.
//!
//! The workspace ships exactly one production alphabet, so every measured
//! property of the engines above the substrate would otherwise be a property of
//! that one alphabet rather than of the trait. This crate is what keeps the
//! alphabet-generic claims honest: it is a **test-facing** crate, a
//! dev-dependency of the crates whose engines quantify over the trait, and no
//! production crate links it.
//!
//! # The modules
//!
//! - [`toy`] — a single-sorted first-order term language (`Zero` / `Succ` /
//!   `Add` with metavariables) inhabiting the trait from outside the crates
//!   that define it, which is exactly the path a future shape-layer or directed
//!   rule-layer alphabet takes. It is also the only alphabet in the tree whose
//!   terms nest commands, so it is where anything about two applications in one
//!   term can be exercised at all.
//! - [`adversarial`] — five wrappers that delegate everything to [`toy`] except
//!   the one law each is built to break: an alphabet that calls every position
//!   pair incomparable, one that withholds the convexity discharge, one whose
//!   splice disturbs a sibling it was not asked about, one that gives two
//!   distinct cells the same address, and the delegating wrapper the other four
//!   are expressed through.

pub mod adversarial;
pub mod toy;
