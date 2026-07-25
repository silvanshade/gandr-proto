//! A self-contained **order-maintenance** structure for the gandr incremental
//! pipeline (milestone A2; design: `docs/gandr/spec/incremental-pipeline.md`
//! §7, the Porter order-maintenance disposition).
//!
//! The *order-maintenance problem* is to maintain a collection of elements
//! under a total order, supporting insertion of a new element immediately
//! before or after an existing one, deletion, and — the headline operation —
//! **comparison of any two elements in O(1)**. It is the order structure the
//! pipeline's incremental story is missing: Porter, Kirisame, Wei, Panchekha &
//! Omar's *Incremental Bidirectional Typing via Order Maintenance*
//! (arXiv:2504.08946) uses pre/post-order timestamp intervals over exactly such
//! a structure to test term containment in O(1) and drive the dirty-step
//! priority queue. The corpus cites the underlying data structure at
//! `incremental-pipeline.md` §7 reference `[6]` (Bender, Cole, Demaine,
//! Farach-Colton & Zito, *Two Simplified Algorithms for Maintaining Order in a
//! List*, 2002), itself a refinement of the list-labeling scheme of Itai,
//! Konheim & Rodeh (1981) and the order-maintenance problem of Dietz & Sleator
//! (1987).
//!
//! # What this crate is, and is not
//!
//! This crate is **only** the order structure plus the interval-containment
//! query built on it ([`Interval`], the pre/post-order use). It is a generic,
//! payload-carrying total order ([`OrderMaintenance`]) over opaque, stable,
//! generation-checked handles ([`Pos`]). Deliberately out of scope (each a
//! separate brick that *consumes* this one):
//!
//! - the lowest-enclosing-**binder** lookup and binding pointers;
//! - the per-node dual-type + boolean-mark + dirty-bit layout for the marked
//!   CBPV core, which additionally needs the semantic-marking layer (Zhao et
//!   al. POPL'24);
//! - any future binding of order points to the merkle CST's reproducible
//!   `OriginEntry` identity. The retired tree-sitter node-address seam is gone;
//!   whether an OM-over-CST adapter still pays for itself waits on the dirty-
//!   frontier consumer.
//!
//! # Algorithm and complexity
//!
//! The implementation is single-level **list-labeling**: every element carries
//! an integer `label` in the universe `[0, 2^62)`, strictly increasing in list
//! order, so [`OrderMaintenance::cmp`] is one `u64` comparison — **O(1)**. An
//! insertion takes the midpoint label of the gap between its neighbours when a
//! gap exists; when the gap is exhausted it **relabels** the smallest
//! power-of-two-aligned window of labels around the insertion point that is at
//! most half full, redistributing those labels (and the new one) evenly. The
//! density cap (at most `2^61` live elements) guarantees the whole-universe
//! window is always sparse enough, so a relabel always succeeds and the
//! structure is **total** — capacity exhaustion surfaces as
//! [`OrderError::CapacityExhausted`], never a panic. Construction itself is
//! fallible: [`OrderMaintenance::new`] returns
//! [`OrderError::StructureIdExhausted`] instead of wrapping the process-wide
//! structure-id counter. Insertion is **O(log² n) amortized**; see
//! `docs/OPTIMIZATION.md` for the two-level O(1)-amortized refinement and the
//! byte-range resync that are deliberately deferred.
//!
//! Handles are generation- and structure-checked: removing an element and
//! reusing its slot bumps a generation counter, and every structure carries a
//! process-unique identity, so a stale handle (to a removed element) or a
//! foreign handle (from a different structure) is detected (queries return
//! `None`, insertions return [`OrderError::UnknownPosition`]) rather than
//! silently aliasing an unrelated element. A slot whose generation counter is
//! exhausted is permanently retired instead of wrapping to an old generation.

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
