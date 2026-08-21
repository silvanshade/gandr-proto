//! **The identity relations on derivations**: when two derivations that fire
//! the same cells in different orders are the same derivation, and what
//! structure decides it.
//!
//! Deep inference is the setting the relations come from. Adjacent independent
//! steps permute; the permutations quotient a derivation to a canonical form;
//! and the atom-occurrence flow of a derivation is a projection through that
//! quotient rather than a finer invariant. Each module is one of those, and
//! none of them is the semantic oracle: replay
//! ([`gandr_theory_coherent_resolutions::tracelet::replay_equivalent`]) is.
//!
//! [`causal`] and [`normal_form`] are mutually dependent and the crate boundary
//! encloses them: an event's canonical key digests its causal past, and the
//! causal order is read back to schedule the normal form.
//!
//! # The modules
//!
//! - [`shift`] — the earned shift-equivalence witness: two adjacent
//!   applications at disjoint positions with trivial overlap are one composite
//!   transformation, granted per pair against the decided guard and carrying
//!   the convexity conjunct's discharge as a certificate rather than a
//!   recomputed sweep. It is this crate's single independence relation.
//! - [`causal`] — the finite event partial order of a recorded derivation: its
//!   events, the dependence edges the guard decides, the causal precedence
//!   order, the layering, and the exchange witness carrying one
//!   sequentialization to another as licensed adjacent transpositions.
//! - [`mod@causal_web`] — the conflict-free two-colour analysis surface over
//!   one canonical event order: green precedence, white independence, licensed
//!   slice-chain refinement witnesses, and named refusals at the
//!   edge-strengthening and open h↓ frontiers.
//! - [`normal_form`] — the certificate normal form: unique primitive
//!   factorization by content address, integer-graded multiplicities, and a
//!   causal canonical schedule, whose equality is a decidable **sound
//!   under-approximation** of replay-equality. Normal-form-equal implies
//!   replay-equal; the converse is never claimed.
//! - [`flow`] — the atom-occurrence flow projection over certificate legs. It
//!   witnesses the shift quotient rather than certificate identity, is sound
//!   exactly on the left-connected-over-acyclic-target discharge, and refuses
//!   rather than re-deriving on a carrier admitting multi-output or
//!   disconnected left-hand sides. Consumed by nothing.
//! - [`footprint`] — a prototype polarized independence test beside the shift
//!   guard: a transition's match image split into rewritten,
//!   matched-but-preserved and framed addresses, with independence defined from
//!   the split. It measures where a polarized reading would license
//!   commutations the guard refuses, and it never replaces the guard. Consumed
//!   by nothing.

extern crate alloc;

pub mod causal;
pub mod causal_web;
pub mod flow;
pub mod footprint;
pub mod normal_form;
pub mod shift;

pub use crate::causal::DerivationEvent;
pub use crate::causal::EventKey;
pub use crate::causal::EventOrder;
pub use crate::causal::ExchangeObstruction;
pub use crate::causal::ExchangeWitness;
pub use crate::causal::KeyCollision;
pub use crate::causal::Transposition;
pub use crate::causal_web::CausalWeb;
pub use crate::causal_web::DependenceBits;
pub use crate::causal_web::HomomorphismFrontier;
pub use crate::causal_web::RefinementCounterexample;
pub use crate::causal_web::RefinementVerdict;
pub use crate::causal_web::SliceChain;
pub use crate::causal_web::SliceStep;
pub use crate::causal_web::SliceStepCount;
pub use crate::causal_web::WebIndependence;
pub use crate::causal_web::WebPrecedence;
pub use crate::causal_web::WebRelation;
pub use crate::causal_web::WebVertex;
pub use crate::causal_web::WebVertexCount;
pub use crate::causal_web::causal_web;
pub use crate::causal_web::refines;
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
pub use crate::normal_form::normalize_certified;
pub use crate::normal_form::prim_address;
pub use crate::normal_form::tracelets_nf_equal;
pub use crate::shift::ShiftEquivalence;
pub use crate::shift::ShiftObstruction;
pub use crate::shift::derive_shift_equivalence;
