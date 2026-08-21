//! **Coherent resolution of a cell rewriting system**: firing a cell, finding
//! the critical pairs, joining them with a certificate, and completing the
//! system until the pairs are exhausted or the budget declines.
//!
//! The four modules are one construction read in order. A cell fires
//! ([`rewrite`]); two cells that can fire at overlapping positions form a
//! critical branching ([`overlap`]); a branching that converges is witnessed by
//! a replayable coherence cell ([`tracelet`]); and orienting the branchings
//! that diverge until none remains is completion ([`completion`]).
//!
//! [`overlap`] and [`tracelet`] are mutually dependent and the crate boundary
//! encloses them rather than cutting between: a certificate carries the
//! branching it joins, and the support index memoizes independence over
//! certificates as well as cells.
//!
//! Everything here is generic over
//! [`gandr_theory_cell_complexes::alphabet::CellAlphabet`] and adds no cell
//! vocabulary of its own.
//!
//! # The modules
//!
//! - [`rewrite`] — one cell applied at one position, and budgeted
//!   normalization, both under the alphabet's own firing discipline.
//! - [`overlap`] — the multi-sum overlap enumerator: confluence critical pairs
//!   and composition overlaps, with the seam data each carries, plus the
//!   memoized support relation the layers above query for independence.
//! - [`tracelet`] — replayable coherence certificates and the derived fused
//!   cell, with replay-equivalence as the identity criterion and observable
//!   step evidence.
//! - [`completion`] — budgeted Knuth–Bendix and Squier completion that declines
//!   with a report rather than diverging.

extern crate alloc;

pub mod completion;
pub mod overlap;
pub mod rewrite;
pub mod tracelet;

pub use gandr_theory_cell_complexes::boundary::CertificateIndex;

pub use crate::completion::CompletionBudget;
pub use crate::completion::CompletionOutcome;
pub use crate::completion::DeclineReason;
pub use crate::completion::SuppliedOverlapError;
pub use crate::completion::complete;
pub use crate::completion::complete_with_overlap_source;
pub use crate::overlap::Overlap;
pub use crate::overlap::OverlapKind;
pub use crate::overlap::OverlapSupport;
pub use crate::overlap::enumerate_overlaps;
pub use crate::overlap::overlaps_between;
pub use crate::rewrite::CellApp;
pub use crate::rewrite::Normalization;
pub use crate::rewrite::normalize;
pub use crate::tracelet::ReplayPath;
pub use crate::tracelet::ReplayPathOutcome;
pub use crate::tracelet::ReplayStep;
pub use crate::tracelet::ReplayTrace;
pub use crate::tracelet::Tracelet;
pub use crate::tracelet::confluence_tracelet;
pub use crate::tracelet::derive_fused;
pub use crate::tracelet::replay_equivalent;
