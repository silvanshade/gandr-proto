//! **Fusion by completion** — phase **L2** of the sequent-machines kernel
//! (`../../../docs/gandr/spec/proposal-sequent-kernel.md` §7 + the VDC
//! addendum; decision K5; ADRs 65, 68, and 69).
//!
//! This crate is the third, purely-additive phase of the sequent kernel. It
//! reifies the fusion engine of §7 over the polarized command IL: an oriented
//! **cell store**, **overlap enumeration** at cut seams, budgeted
//! **Knuth–Bendix / Squier completion**, and **replayable tracelet 3-cell
//! certificates**. It touches no frozen-core code and consumes the public
//! [`gandr_core_sequent`] IL vocabulary (the
//! [`gandr_core_sequent::il::Polarity`] orientation and, through [`bridge`],
//! the [`gandr_core_sequent::il::CommandArena`]) read-only, and elaborates the
//! reserved `rule` faces of [`gandr_theory_levitation`].
//!
//! # The layers
//!
//! - [`pattern`] — the **command-pattern IL**: the §2.1 cut grammar with
//!   pattern metavariables, the §7.1 operation frame `f(p̄; c)` and return-side
//!   constructor frame `K⁻(c)`, positions, and the reduction order.
//! - [`subst`] — substitutions, one-sided **matching** (cell application) and
//!   two-sided **unification** (overlaps), both iterative (ADR-47).
//! - [`cell`] — the [`cell::Cell`] (`lhs ~> rhs`, orientation, provenance,
//!   derived [`cell::CellMeta`] with the VDC variance/linearity and
//!   `invertible` flag) and the content-addressed [`cell::CellStore`].
//! - [`elaborate`] — surface `rule` faces
//!   ([`gandr_theory_levitation::CellFace`]) into cells (§7.1), the ADR-54
//!   acceptance target.
//! - [`rewrite`] — direct command-level rewriting with a budget and the
//!   η-polarity discipline (§5).
//! - [`overlap`] — the multi-sum overlap enumerator (§7.3.2): confluence
//!   critical pairs and composition (fusion) overlaps.
//! - [`completion`] — budgeted Knuth–Bendix / Squier completion with
//!   decline-and-report (§7.3.3).
//! - [`tracelet`] — replayable 3-cell certificates and the derived fused cell
//!   (§7.2, §7.3.4), with replay-equivalence as the identity criterion (ADR-69
//!   D1).
//! - [`compose`] — two-mode certificate composition (§4.3, ADR-69 D3):
//!   [`compose::compose_invertible`] (the unconditional coherence lane) and
//!   [`compose::compose_directed`] (gated by variable-flow acyclicity across
//!   the composed seam, declining with the cycle as diagnostic).
//! - [`bridge`] — reification of the frozen fragment into the L0 command arena.
//!
//! # The L2 gate
//!
//! The `fused ≡ two-step` differential ([`tracelet::derive_fused`] +
//! [`tracelet::Tracelet::replay`], property-tested over generated ground
//! configurations), certificate replay, and the η-polarity pathological pins
//! are the phase-L2 gate rows (§9). The `compose_invertible` /
//! `compose_directed` operations and the acyclicity gate (VDC addendum §A)
//! now live in [`compose`], reading the **live**
//! [`cell::CellVariance`] (with `Mixed` reachable) and the
//! [`cell::CellMeta::invertible`] flag [`cell::CellMeta::derive`] computes at
//! construction.
//!
//! # Draft-ADR candidate
//!
//! The **cell-visible fragment boundary** — that fusion cells range over a
//! symbolic command-pattern IL (datatype constructors/operations plus the two
//! §7.1 frames), with the frozen [`gandr_core_sequent::il`] node types reached
//! only by the [`bridge`] reification of the frozen-representable subset — is a
//! design decision this crate takes without minting an ADR (per the lane's
//! no-ADR constraint). If the L3 promotion decision (§9) makes command syntax
//! canonical, the relationship between this symbolic pattern IL and the arena
//! command IL should be pinned by an ADR. Recorded here as a candidate, not a
//! decision.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        reason = "the standard test-allow set keeps the unit and property tests readable (docs/workflow/rust.md)"
    )
)]

extern crate alloc;

pub mod boundary;
pub mod bridge;
pub mod cell;
pub mod completion;
pub mod compose;
pub mod elaborate;
pub mod overlap;
pub mod pattern;
pub mod rewrite;
pub mod subst;
pub mod tracelet;

pub use crate::boundary::CellCount;
pub use crate::boundary::CellInvertibility;
pub use crate::boundary::CellLinearity;
pub use crate::boundary::CellStoreEmptyStatus;
pub use crate::boundary::CompletionCellBudget;
pub use crate::boundary::CompletionStatus;
pub use crate::boundary::CompletionStepBudget;
pub use crate::boundary::DeclinedFaceIndex;
pub use crate::boundary::GroundPatternStatus;
pub use crate::boundary::NormalizationBudget;
pub use crate::boundary::PatternSize;
pub use crate::boundary::PositionRootStatus;
pub use crate::boundary::PositionStep;
pub use crate::boundary::SubstitutionBindingCount;
pub use crate::boundary::SubstitutionDecision;
pub use crate::boundary::SubstitutionEmptyStatus;
pub use crate::boundary::TraceletEquivalence;
pub use crate::boundary::TraceletReplay;
pub use crate::boundary::VarianceFlowRole;
pub use crate::cell::Cell;
pub use crate::cell::CellId;
pub use crate::cell::CellMeta;
pub use crate::cell::CellProvenance;
pub use crate::cell::CellStore;
pub use crate::cell::CellVarMeta;
pub use crate::cell::CellVariance;
pub use crate::cell::EtaKind;
pub use crate::cell::Orientation;
pub use crate::cell::frame_defining_cell;
pub use crate::completion::CompletionBudget;
pub use crate::completion::CompletionOutcome;
pub use crate::completion::DeclineReason;
pub use crate::completion::complete;
pub use crate::compose::CompositionObstruction;
pub use crate::compose::compose_directed;
pub use crate::compose::compose_invertible;
pub use crate::elaborate::ElaborateError;
pub use crate::elaborate::elaborate_data_desc;
pub use crate::elaborate::elaborate_rule;
pub use crate::overlap::Overlap;
pub use crate::overlap::OverlapKind;
pub use crate::overlap::enumerate_overlaps;
pub use crate::pattern::Cat;
pub use crate::pattern::CmdPat;
pub use crate::pattern::ConsPat;
pub use crate::pattern::MetaVar;
pub use crate::pattern::Pos;
pub use crate::pattern::ProdPat;
pub use crate::pattern::Sym;
pub use crate::rewrite::CellApp;
pub use crate::rewrite::Normalization;
pub use crate::rewrite::normalize;
pub use crate::subst::Subst;
pub use crate::tracelet::Tracelet;
pub use crate::tracelet::confluence_tracelet;
pub use crate::tracelet::derive_fused;
pub use crate::tracelet::replay_equivalent;
