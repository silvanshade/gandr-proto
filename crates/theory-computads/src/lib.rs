//! **Fusion by completion** — phase **L2** of the sequent-machines kernel
//! (the sequent-kernel design's §7, with the VDC addendum), lifted over its
//! cell alphabet (the executed metatheory-roadmap meta-spike-01).
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
//! - [`alphabet`] — the [`alphabet::CellAlphabet`] trait: the pattern grammar,
//!   ordered-map substitution, and seam/overlap vocabulary the engines quantify
//!   over (the executed meta-spike-01 — the crate was intentionally monomorphic
//!   as a review tripwire; the lift is a design change, documented there).
//! - [`sequent`] — the [`sequent::SequentAlphabet`], the first
//!   [`alphabet::CellAlphabet`] inhabitant: the sequent-kernel command-pattern
//!   alphabet with its orientation/provenance tags, live variance metadata, and
//!   η-polarity firing discipline.
//! - [`pattern`] — the **command-pattern IL**: the §2.1 cut grammar with
//!   pattern metavariables, the §7.1 operation frame `f(p̄; c)` and return-side
//!   constructor frame `K⁻(c)`, positions, and the reduction order.
//! - [`subst`] — substitutions, one-sided **matching** (cell application) and
//!   two-sided **unification** (overlaps), both iterative (ADR-47).
//! - [`cell`] — the [`cell::Cell`] (`lhs ~> rhs`, orientation, provenance,
//!   derived metadata) and the structurally deduplicated, insertion-ordered
//!   [`cell::CellStore`], generic over the alphabet.
//! - [`elaborate`] — a whole [`gandr_theory_levitation::SignDesc`] into cells
//!   (§7.1), the ADR-54 acceptance target: surface `rule` faces
//!   ([`gandr_theory_levitation::RuleFace`]) become cells, and the declared
//!   `op` members' [`gandr_theory_levitation::BridgeArity`]s decide which of
//!   them the single-continuation grammar admits.
//! - [`linearity`] — the **cell-admission linearity boundary**: cell patterns
//!   are linear (owner ruling, 2026-08-01), so a description whose rule copies
//!   a hole is refused with a diagnostic naming the copy and the respelling.
//!   The refusal runs where cells are *admitted*, never where patterns are
//!   *constructed*.
//! - [`rewrite`] — direct command-level rewriting with a budget and the
//!   alphabet's firing discipline (the sequent η-polarity gate, §5).
//! - [`overlap`] — the multi-sum overlap enumerator (§7.3.2): confluence
//!   critical pairs and composition (fusion) overlaps, emitting seam data.
//! - [`completion`] — budgeted Knuth–Bendix / Squier completion with
//!   decline-and-report (§7.3.3).
//! - [`tracelet`] — replayable 3-cell certificates and the derived fused cell
//!   (§7.2, §7.3.4), with replay-equivalence as the identity criterion (ADR-69
//!   D1), indexed-store [`cell::CellId`] references, and observable
//!   [`tracelet::ReplayTrace`] step evidence.
//! - [`shift`] — the **earned shift-equivalence witness**: two adjacent
//!   applications at disjoint positions with trivial overlap are one composite
//!   transformation, granted per pair against the decided guard
//!   (`spec:implementation/circuit-terms.md`, `circuit-terms-spike-07`) and
//!   carrying the convexity conjunct's discharge as a certificate rather than a
//!   recomputed sweep.
//! - [`instantiate`] — the **circuit rule application site**, where that
//!   identification is well-posed. A circuit rule is a schema whose body
//!   applies rewrite-*sorted ports*, so a two-redex body names no cell pair to
//!   ask the guard about and [`gandr_theory_levitation::elaborate_body`] defers
//!   the question rather than answering it. Here each port is instantiated by a
//!   stored cell, the two positions are read from the block's own occurrence
//!   record rather than fabricated, and [`shift::derive_shift_equivalence`]
//!   decides the pair — with the composite replayed under both
//!   sequentializations before the identification is handed back.
//! - [`causal`] — the **finite event partial order** of a recorded derivation:
//!   its events, the dependence edges the [`shift`] guard decides, the causal
//!   precedence order, the layering, and the **exchange witness** carrying one
//!   sequentialization to another as licensed adjacent transpositions. The
//!   canonical schedule is one projection of it, so what used to be an argument
//!   about shift-invariance is a value a caller can check. The owner's
//!   2026-08-16 ruling binds that projection: it is the lexicographic sort by
//!   causal depth and a content-derived [`causal::EventKey`] that reads neither
//!   arrival order nor store-local indices, with union-find component
//!   pre-grouping declined, and its injectivity **enforced** rather than
//!   assumed.
//! - [`normal_form`] — the **tracelet normal form**: a canonical form on
//!   certificate data (unique primitive factorization by content address,
//!   integer-graded multiplicities, and a causal canonical schedule) whose
//!   equality is a decidable **sound under-approximation** of replay-equality.
//!   NF-equal implies replay-equal; the converse is never claimed, and replay
//!   ([`tracelet::replay_equivalent`]) remains the semantic oracle. It reuses
//!   the [`shift`] guard as the crate's single independence relation and
//!   *checks* its own canonicalization by replaying it.
//! - [`flow`] — the **atom-occurrence flow projection**: a prototype beside the
//!   shift guard, consumed by nothing. It instantiates the atomic-flow
//!   definition over tracelet legs and witnesses the **shift quotient** rather
//!   than certificate identity — [`tracelet::replay_equivalent`] does not read
//!   the recorded paths, so no relation that reads them can coincide with it.
//!   Sound exactly on the left-connected-over-acyclic-target discharge, and
//!   **refused** rather than re-derived on a carrier admitting multi-output or
//!   disconnected left-hand sides. A positive answer *forces* equal peaks,
//!   equal joins and two successful replays rather than checking for them
//!   afterwards, so the boundary is carried by construction.
//! - [`footprint`] — a **prototype** polarized independence test beside the
//!   shift guard, consumed by nothing: a transition's match image split into
//!   rewritten, matched-but-preserved, and framed addresses, with independence
//!   defined from the split rather than from the alphabet. It exists to measure
//!   where a polarized reading licenses commutations the guard's
//!   incomparable-positions conjunct refuses, and it never replaces the guard.
//! - [`compose`] — two-mode certificate composition (§4.3, ADR-69 D3):
//!   [`compose::compose_invertible`] (the unconditional coherence lane) and
//!   [`compose::compose_directed`] (gated by variable-flow acyclicity across
//!   the composed seam, declining with the cycle as diagnostic).
//! - [`pathway`] — **static pathway queries**: which compressed derivations can
//!   end in a target cell, grown backwards from the target by composition and
//!   compressed to normal form, evaluating nothing. Its target-occurs-only-last
//!   condition is decided as an order property rather than over an equivalence
//!   class, because the rearrangements of a derivation are exactly the linear
//!   extensions of its [`causal`] order. The positive verdict is named for what
//!   it is worth: independence is granted only where the [`shift`] guard can
//!   discharge it, so a refutation is sound while an acceptance is
//!   guard-relative. An obstruction that describes the candidate drops it; a
//!   kill signal reaches the caller.
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
//! [`sequent::CellVariance`] (with `Mixed` reachable) and the
//! [`sequent::CellMeta::invertible`] flag [`sequent::CellMeta::derive`]
//! computes at construction.
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

extern crate alloc;

pub mod alphabet;
pub mod boundary;
pub mod bridge;
pub mod causal;
pub mod cell;
pub mod completion;
pub mod compose;
pub mod elaborate;
pub mod flow;
pub mod footprint;
pub mod instantiate;
pub mod linearity;
pub mod normal_form;
pub mod order;
pub mod overlap;
pub mod pathway;
pub mod pattern;
pub mod rewrite;
pub mod sequent;
pub mod shift;
pub mod subst;
pub mod tracelet;
pub mod transport;

pub use gandr_storage_artifact::TransportStepId;

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
pub use crate::causal::DerivationEvent;
pub use crate::causal::EventKey;
pub use crate::causal::EventOrder;
pub use crate::causal::ExchangeObstruction;
pub use crate::causal::ExchangeWitness;
pub use crate::causal::KeyCollision;
pub use crate::causal::Transposition;
pub use crate::cell::Cell;
pub use crate::cell::CellId;
pub use crate::cell::CellStore;
pub use crate::completion::CompletionBudget;
pub use crate::completion::CompletionOutcome;
pub use crate::completion::DeclineReason;
pub use crate::completion::complete;
pub use crate::compose::CompositionObstruction;
pub use crate::compose::compose_directed;
pub use crate::compose::compose_invertible;
pub use crate::elaborate::DescElaboration;
pub use crate::elaborate::ElaborateError;
pub use crate::elaborate::OpElaborateError;
pub use crate::elaborate::OpFrame;
pub use crate::elaborate::elaborate_data_desc;
pub use crate::elaborate::elaborate_rule;
pub use crate::flow::Flow;
pub use crate::flow::FlowEnd;
pub use crate::flow::FlowObstruction;
pub use crate::flow::FlowThread;
pub use crate::flow::TraceletFlow;
pub use crate::flow::flows_equal;
pub use crate::flow::legs_flow_equal;
pub use crate::flow::project_flow;
pub use crate::flow::tracelet_flow;
pub use crate::flow::tracelets_flow_equal;
pub use crate::footprint::FootprintIndependence;
pub use crate::footprint::FootprintObstruction;
pub use crate::footprint::MatchFootprint;
pub use crate::footprint::footprint_independence;
pub use crate::footprint::match_footprint;
pub use crate::instantiate::CircuitShift;
pub use crate::instantiate::CircuitShiftObstruction;
pub use crate::instantiate::RewriteBinding;
pub use crate::instantiate::instantiate_two_redex_rule;
pub use crate::linearity::NonLinearPattern;
pub use crate::linearity::admit_linear_cell;
pub use crate::linearity::copied_hole;
pub use crate::normal_form::CausalPast;
pub use crate::normal_form::CellAddress;
pub use crate::normal_form::NormalFormObstruction;
pub use crate::normal_form::PrimCert;
pub use crate::normal_form::PrimId;
pub use crate::normal_form::ReplayPlan;
pub use crate::normal_form::ReplayWitness;
pub use crate::normal_form::TraceletNf;
pub use crate::normal_form::causal_past_address;
pub use crate::normal_form::cell_address;
pub use crate::normal_form::certified_nf_equal;
pub use crate::normal_form::event_order;
pub use crate::normal_form::nf_equal;
pub use crate::normal_form::nf_equal_across_stores;
// NOTE: `normal_form::normalize` is deliberately NOT re-exported here —
// `rewrite::normalize` (budgeted term normalization) already owns that name at
// the crate root, and the two are different operations on different objects.
// The normal form's entry point is spelled `normal_form::normalize`.
// `normalize_certified` carries no such collision and is re-exported below.
pub use crate::normal_form::normalize_certified;
pub use crate::normal_form::prim_address;
pub use crate::normal_form::tracelets_nf_equal;
pub use crate::order::path_order_cmp;
pub use crate::order::reduction_cmp;
pub use crate::overlap::Overlap;
pub use crate::overlap::OverlapKind;
pub use crate::overlap::OverlapSupport;
pub use crate::overlap::enumerate_overlaps;
pub use crate::overlap::overlaps_between;
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
pub use crate::shift::ShiftEquivalence;
pub use crate::shift::ShiftObstruction;
pub use crate::shift::derive_shift_equivalence;
pub use crate::subst::Subst;
pub use crate::tracelet::ReplayPath;
pub use crate::tracelet::ReplayPathOutcome;
pub use crate::tracelet::ReplayStep;
pub use crate::tracelet::ReplayTrace;
pub use crate::tracelet::Tracelet;
pub use crate::tracelet::confluence_tracelet;
pub use crate::tracelet::derive_fused;
pub use crate::tracelet::replay_equivalent;
pub use crate::transport::CanonicalStepEncoding;
pub use crate::transport::TransportStepIndex;
pub use crate::transport::TransportStepObstruction;
pub use crate::transport::transport_step_id;
pub use crate::transport::transport_step_index;
