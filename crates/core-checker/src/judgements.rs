//! The typing judgements the core's two realizations share.
//!
//! [`checker`] is the direct recursive bidirectional judgement itself, the
//! reference realization the defunctionalized machine in `gandr-core-machine`
//! is derived from and held to step-for-step. Around it sit the vocabulary and
//! the judgements both realizations spend:
//!
//! - [`control`] — the direction the judgement runs in and the `Descend` /
//!   `Return` event log the two realizations are compared through. It is not
//!   the machine's: the recursive checker emits the events and the machine
//!   passes through the registers, so the vocabulary is shared by construction;
//! - [`stack`] — the stack-typing judgement a reified stack needs, with the
//!   component projections that destructure a consumed type;
//! - [`package`] — discharging a signature's abstract type components;
//! - [`seal`] — minting the nominal atoms opaque ascription needs, and the
//!   table that makes their freshness checkable.
//!
//! What is *not* here is deliberate. The subsumption judgement lives in
//! [`discipline::subtype`] beside the other discipline the checker enforces,
//! and the identity motive's value-into-type substitution in
//! `gandr_core_term::identity`, because it is substitution over the substrate
//! rather than a judgement.
//!
//! [`discipline::subtype`]: crate::discipline::subtype

pub mod checker;
pub mod control;
pub mod package;
pub mod seal;
pub mod stack;
