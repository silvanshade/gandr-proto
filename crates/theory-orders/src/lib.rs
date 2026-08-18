//! A self-contained **order-maintenance** structure: a total order over
//! payload-carrying elements, supporting insertion beside an existing element,
//! deletion, and **comparison of any two elements in O(1)**.
//!
//! [`OrderMaintenance`] is the structure and [`Interval`] the pre/post-order
//! containment query built on it. Everything a consumer layers on top — binder
//! lookup, per-node mark and dirty-bit layout, syntax-tree resync — has its own
//! invariants and lives in the consumer.
//!
//! Comparison is one integer comparison because every element carries a label
//! strictly increasing in list order. An insertion takes the midpoint label of
//! its neighbours' gap, or relabels the smallest power-of-two-aligned window
//! that is at most half full when the gap is exhausted; the density cap keeps
//! that window sparse enough for the relabel to succeed, so insertion is
//! O(log^2 n) amortized and the structure is total. Capacity exhaustion is
//! [`OrderError::CapacityExhausted`] rather than a panic, and construction is
//! fallible rather than wrapping the process-wide structure-id counter.
//!
//! Handles are generation- and structure-checked: a stale handle to a removed
//! element, or a foreign handle from another structure, is detected rather than
//! silently aliasing an unrelated element.
//!
//! The named ideas and their primary references are in this crate's
//! `README.md`.

extern crate alloc;

pub mod interval;
pub mod order;

pub use crate::interval::Interval;
pub use crate::order::HandleMembership;
pub use crate::order::IntervalContainment;
pub use crate::order::LiveLen;
pub use crate::order::OrderError;
pub use crate::order::OrderIsEmpty;
pub use crate::order::OrderMaintenance;
pub use crate::order::Pos;
