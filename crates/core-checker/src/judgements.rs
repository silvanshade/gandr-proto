//! The typing judgements the checker's two faces share.
//!
//! [`checker`] is the direct recursive bidirectional judgement itself, the
//! reference realization the defunctionalized [`machine`] is derived from and
//! held to step-for-step. The other three are the judgements that hang off it
//! and were added as their rungs landed: [`identity`] instantiates a `Walk`
//! motive by substituting a value into a type, [`package`] discharges a
//! signature's abstract type components, and [`seal`] mints the nominal atoms
//! opaque ascription needs and keeps the table that makes their freshness
//! checkable.
//!
//! What is *not* here is deliberate. The subsumption judgement lives in
//! [`discipline::subtype`] beside the other disciplines the checker enforces,
//! and the stack-typing judgement in [`machine::stack`] beside the machine
//! whose reified stacks it types.
//!
//! [`machine`]: crate::machine
//! [`discipline::subtype`]: crate::discipline::subtype
//! [`machine::stack`]: crate::machine::stack

pub mod checker;
pub mod identity;
pub mod package;
pub mod seal;
