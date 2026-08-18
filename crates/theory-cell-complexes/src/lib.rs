//! The **cell-shape substrate**: what a rewriting cell is, over what pattern
//! grammar, and under which alphabet.
//!
//! Everything above this crate — critical pairs and completion, the identity
//! relations, the certificate algebra, the elaborator — is stated over the
//! vocabulary defined here and adds no vocabulary of its own at this level.
//! The crate depends on no other workspace crate but
//! [`gandr_core_sequent`], and on that only for the polarity tag the sequent
//! alphabet carries.
//!
//! # The modules
//!
//! - [`boundary`] — the nominal wrapper vocabulary: every count, index, budget,
//!   depth and verdict that crosses a public signature in this crate or above
//!   it, so no engine passes a bare primitive.
//! - [`pattern`] — the command-pattern intermediate language: the cut grammar
//!   with pattern metavariables, the operation and return-side constructor
//!   frames, positions, and the erased node view the orders are taken over.
//! - [`subst`] — substitutions, one-sided **matching** for cell application and
//!   two-sided **unification** for overlaps, both iterative.
//! - [`order`] — the **reduction order** used to orient a divergent pair: a
//!   hole-occurrence-guarded size comparison with a lexicographic path order
//!   deciding the ties.
//! - [`alphabet`] — the [`alphabet::CellAlphabet`] trait every engine above
//!   quantifies over: the pattern grammar, the ordered-map substitution, and
//!   the seam and overlap vocabulary.
//! - [`sequent`] — [`sequent::SequentAlphabet`], the trait's one inhabitant in
//!   this tree: the sequent-kernel command-pattern alphabet with its
//!   orientation and provenance tags, live variance metadata, and η-polarity
//!   firing discipline.
//! - [`cell`] — the [`cell::Cell`] (`lhs ~> rhs`, with orientation, provenance
//!   and derived metadata) and the structurally deduplicated, insertion-ordered
//!   [`cell::CellStore`], generic over the alphabet.
//! - [`linearity`] — the cell-admission linearity boundary: cell patterns are
//!   linear, so a rule that copies a hole is refused where cells are *admitted*
//!   rather than where patterns are *constructed*.

extern crate alloc;

pub mod alphabet;
pub mod boundary;
pub mod cell;
pub mod linearity;
pub mod order;
pub mod pattern;
pub mod sequent;
pub mod subst;

pub use crate::alphabet::CellAlphabet;
pub use crate::alphabet::ConvexityDischarge;
pub use crate::alphabet::PositionOrder;
pub use crate::alphabet::SeamRole;
pub use crate::alphabet::path_order;
pub use crate::boundary::CausalDepth;
pub use crate::boundary::CellCount;
pub use crate::boundary::CellInvertibility;
pub use crate::boundary::CellLinearity;
pub use crate::boundary::CellStoreEmptyStatus;
pub use crate::boundary::CertificateIndex;
pub use crate::boundary::CompletionCellBudget;
pub use crate::boundary::CompletionStatus;
pub use crate::boundary::CompletionStepBudget;
pub use crate::boundary::ConstructorCount;
pub use crate::boundary::DeclinedCircuitIndex;
pub use crate::boundary::DeclinedFaceIndex;
pub use crate::boundary::DeclinedOpIndex;
pub use crate::boundary::EventConcurrency;
pub use crate::boundary::EventCount;
pub use crate::boundary::EventDependence;
pub use crate::boundary::EventIndex;
pub use crate::boundary::EventPrecedence;
pub use crate::boundary::FiringPermission;
pub use crate::boundary::FlowEquality;
pub use crate::boundary::FlowPortIndex;
pub use crate::boundary::FlowVertexIndex;
pub use crate::boundary::GroundPatternStatus;
pub use crate::boundary::NormalFormEquality;
pub use crate::boundary::NormalizationBudget;
pub use crate::boundary::OperationInputCount;
pub use crate::boundary::PatternSize;
pub use crate::boundary::PeakOccurrenceIndex;
pub use crate::boundary::PositionRootStatus;
pub use crate::boundary::PositionStep;
pub use crate::boundary::PrimMultiplicity;
pub use crate::boundary::RedexOccurrenceCount;
pub use crate::boundary::ReplayLevel;
pub use crate::boundary::SchedulePosition;
pub use crate::boundary::ShiftReplay;
pub use crate::boundary::StepIndependence;
pub use crate::boundary::SubstitutionBindingCount;
pub use crate::boundary::SubstitutionDecision;
pub use crate::boundary::SubstitutionEmptyStatus;
pub use crate::boundary::TraceletEquivalence;
pub use crate::boundary::TraceletReplay;
pub use crate::boundary::TranspositionCount;
pub use crate::boundary::VarianceFlowRole;
pub use crate::cell::Cell;
pub use crate::cell::CellId;
pub use crate::cell::CellStore;
pub use crate::linearity::NonLinearPattern;
pub use crate::linearity::admit_linear_cell;
pub use crate::linearity::copied_hole;
pub use crate::order::path_order_cmp;
pub use crate::order::reduction_cmp;
pub use crate::pattern::Cat;
pub use crate::pattern::CmdPat;
pub use crate::pattern::ConsPat;
pub use crate::pattern::MetaVar;
pub use crate::pattern::Pos;
pub use crate::pattern::ProdPat;
pub use crate::pattern::Sym;
pub use crate::sequent::CellContractumUse;
pub use crate::sequent::CellMeta;
pub use crate::sequent::CellProvenance;
pub use crate::sequent::CellVarMeta;
pub use crate::sequent::CellVariance;
pub use crate::sequent::EtaKind;
pub use crate::sequent::HoleName;
pub use crate::sequent::Orientation;
pub use crate::sequent::SequentAlphabet;
pub use crate::sequent::StepGrowth;
pub use crate::sequent::frame_defining_cell;
pub use crate::subst::Subst;
