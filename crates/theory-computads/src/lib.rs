//! **The elaborator seam**: where a levitation description becomes cells, where
//! a circuit rule is instantiated against the cell store, and where the cell
//! grammar is reified back into the frozen command arena.
//!
//! This is the production-facing face of the cell-rewriting stack. The
//! machinery it drives lives below it —
//! [`gandr_theory_cell_complexes`] defines what a cell is,
//! [`gandr_theory_coherent_resolutions`] fires and completes them,
//! [`gandr_theory_deep_inference`] decides when two derivations are one, and
//! `gandr_theory_decomposition_spaces` composes and transports the
//! certificates. A consumer that only wants descriptions turned into cells
//! consumes this crate and nothing else: the vocabulary its signatures mention
//! is re-exported here.
//!
//! It consumes the public [`gandr_core_sequent`] command IL read-only and
//! elaborates the reserved `rule` faces of [`gandr_theory_levitation`].
//!
//! # The modules
//!
//! - [`elaborate`] — a whole [`gandr_theory_levitation::SignDesc`] into cells:
//!   surface `rule` faces become cells, and the declared operations' bridge
//!   arities decide which of them the single-continuation grammar admits.
//! - [`instantiate`] — the **circuit rule application site**, where the
//!   shift-equivalence question about a two-redex rule body becomes well-posed.
//!   A circuit rule is a schema whose body applies rewrite-sorted ports, so the
//!   body names no cell pair to ask the guard about and levitation defers the
//!   question rather than answering it. Here each port is instantiated by a
//!   stored cell, the two positions are read from the block's own occurrence
//!   record rather than fabricated, and the guard decides the pair — with the
//!   composite replayed under both sequentializations before the identification
//!   is handed back. **The matcher is supplied at this site**, which is why no
//!   crate below depends on `gandr-theory-circuit-algebras`.
//! - [`bridge`] — reification of the frozen fragment into the L0 command arena.
//!
//! # Draft decision-record candidate
//!
//! The **cell-visible fragment boundary** — that fusion cells range over a
//! symbolic command-pattern intermediate language, with the frozen
//! [`gandr_core_sequent::il`] node types reached only by the [`bridge`]
//! reification of the frozen-representable subset — is a design decision this
//! stack takes without a decision record, per the lane's no-record constraint.
//! If a later stage makes command syntax canonical, the relationship between
//! the symbolic pattern language and the arena command IL needs pinning.
//! Recorded here as a candidate, not a decision.

extern crate alloc;

pub mod bridge;
pub mod elaborate;
pub mod instantiate;

pub use gandr_theory_cell_complexes::alphabet::CellAlphabet;
pub use gandr_theory_cell_complexes::boundary::DeclinedCircuitIndex;
pub use gandr_theory_cell_complexes::boundary::DeclinedFaceIndex;
pub use gandr_theory_cell_complexes::boundary::DeclinedOpIndex;
pub use gandr_theory_cell_complexes::boundary::OperationInputCount;
pub use gandr_theory_cell_complexes::boundary::PositionStep;
pub use gandr_theory_cell_complexes::cell::Cell;
pub use gandr_theory_cell_complexes::cell::CellId;
pub use gandr_theory_cell_complexes::cell::CellStore;
pub use gandr_theory_cell_complexes::pattern::Cat;
pub use gandr_theory_cell_complexes::pattern::ConsPat;
pub use gandr_theory_cell_complexes::pattern::MetaVar;
pub use gandr_theory_cell_complexes::pattern::Pos;
pub use gandr_theory_cell_complexes::pattern::ProdPat;
pub use gandr_theory_cell_complexes::pattern::collect_cmd_metavars;
pub use gandr_theory_cell_complexes::sequent::SequentAlphabet;
pub use gandr_theory_cell_complexes::subst::Subst;

pub use crate::elaborate::DescElaboration;
pub use crate::elaborate::ElaborateError;
pub use crate::elaborate::EtaElaborateError;
pub use crate::elaborate::OpElaborateError;
pub use crate::elaborate::OpFrame;
pub use crate::elaborate::elaborate_data_desc;
pub use crate::elaborate::elaborate_rule;
pub use crate::instantiate::CellInstantiationError;
pub use crate::instantiate::CircuitShift;
pub use crate::instantiate::CircuitShiftObstruction;
pub use crate::instantiate::RewriteBinding;
pub use crate::instantiate::instantiate_cell;
pub use crate::instantiate::instantiate_position;
pub use crate::instantiate::instantiate_two_redex_rule;
