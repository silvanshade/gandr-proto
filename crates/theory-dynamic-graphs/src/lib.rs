//! **Dynamic graph maintenance**: invariants of a directed graph kept current
//! under edge insertion, rather than recomputed from scratch after each one.
//!
//! Two structures live here, and they answer the same question at two
//! strengths.
//!
//! [`AcyclicityMaintenance`] maintains a topological order of the admitted
//! edges. An insertion the order already witnesses costs one comparison; an
//! insertion that violates the order is repaired by relocating exactly the
//! affected region; an insertion that closes a cycle is refused and carries the
//! cycle as a [`gandr_theory_graphs::CycleWitness`] — the same witness type the
//! batch [`gandr_theory_graphs::cycle_witness`] returns, so an incremental
//! verdict and a batch verdict are directly comparable rather than translated.
//!
//! [`PotentialMaintenance`] maintains a valuation satisfying offset-carrying
//! constraints `value(target) >= value(source) + offset`. It refuses exactly
//! the constraint sets no valuation satisfies — the positive-weight cycles.
//!
//! # Why both, and what separates them
//!
//! Acyclicity is a **sound but incomplete** approximation of offset
//! satisfiability, and the gap is a property of the offsets rather than of the
//! algorithms. Every cycle is unsatisfiable when every offset is strictly
//! positive, so on that regime the two structures agree and the cheaper
//! graph-theoretic one is exact. Admit a single zero offset and the agreement
//! breaks: a cycle whose offsets sum to zero forces its nodes to share one
//! value and is perfectly satisfiable, while the topological order cannot
//! represent it at all and refuses.
//!
//! So the boundary between the two is not a matter of taste. It is the point at
//! which the edges start carrying algebra the order cannot hold, and a consumer
//! whose edges carry offsets needs the valuation rather than the order.
//!
//! The named ideas and their primary references are in this crate's
//! `README.md`.

extern crate alloc;

pub mod maintenance;
pub mod potential;
mod slot;

pub use crate::maintenance::AcyclicityMaintenance;
pub use crate::maintenance::AdmittedEdgeCount;
pub use crate::maintenance::EdgeVerdict;
pub use crate::maintenance::InsertionCount;
pub use crate::maintenance::MaintenanceError;
pub use crate::maintenance::MaintenanceTelemetry;
pub use crate::maintenance::RefusalCount;
pub use crate::maintenance::RelocationCount;
pub use crate::maintenance::RepairCount;
pub use crate::maintenance::TopologicalOrderStatus;
pub use crate::maintenance::VisitCount;
pub use crate::potential::AdmittedConstraintCount;
pub use crate::potential::ConstraintVerdict;
pub use crate::potential::FeasibilityStatus;
pub use crate::potential::Offset;
pub use crate::potential::Potential;
pub use crate::potential::PotentialError;
pub use crate::potential::PotentialMaintenance;
pub use crate::potential::PotentialTelemetry;
pub use crate::potential::RaiseCount;
pub use crate::potential::RefutationCount;
pub use crate::potential::RelaxationCount;
