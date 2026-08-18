//! **The algebra of certificates**: composing them, decomposing them, and
//! minting a durable identity for one step of one.
//!
//! A decomposition space is the structure that answers, of an arrow, in how
//! many ways it factors as a composite. That is what this crate computes over
//! derivation certificates: [`compose`] is the composition operation,
//! [`pathway`] is the backward decomposition query — which compressed
//! derivations can end in a given cell — and [`transport`] mints the
//! content-addressed step identity a certificate needs before it may leave the
//! process it was built in.
//!
//! The certificates themselves are
//! [`gandr_theory_coherent_resolutions::tracelet`]'s, and the normal form the
//! queries answer in is
//! [`gandr_theory_deep_inference::normal_form`]'s. This crate adds the algebra
//! over them and no new certificate vocabulary.
//!
//! # The modules
//!
//! - [`compose`] — two-mode certificate composition:
//!   [`compose::compose_invertible`], the unconditional coherence lane, and
//!   [`compose::compose_directed`], gated by variable-flow acyclicity across
//!   the composed seam and declining with the cycle as its diagnostic.
//! - [`pathway`] — static pathway queries: which compressed derivations can end
//!   in a target cell, grown backwards from the target and compressed to normal
//!   form, evaluating nothing. Its target-occurs-only-last condition is decided
//!   as an order property, because the rearrangements of a derivation are
//!   exactly the linear extensions of its causal order. A refutation is sound;
//!   an acceptance is relative to what the shift guard can discharge.
//! - [`transport`] — the canonical step encoding and the durable step identity
//!   it frames: the one boundary at which a process-local content address
//!   becomes something that may be persisted or transmitted.

extern crate alloc;

pub mod compose;
pub mod pathway;
pub mod transport;

pub use gandr_storage_artifact::TransportStepId;

pub use crate::compose::CompositionObstruction;
pub use crate::compose::compose_directed;
pub use crate::compose::compose_invertible;
pub use crate::transport::CanonicalStepEncoding;
pub use crate::transport::TransportStepIndex;
pub use crate::transport::TransportStepObstruction;
pub use crate::transport::transport_step_id;
pub use crate::transport::transport_step_index;
