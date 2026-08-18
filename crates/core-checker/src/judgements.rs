//! The typing judgements the checker's two faces share.
//!
//! [`checker`] is the direct recursive bidirectional judgement itself, the
//! reference realization the defunctionalized [`machine`] is derived from and
//! held to step-for-step. The other two are the judgements that hang off it and
//! were added as their rungs landed: [`package`] discharges a signature's
//! abstract type components, and [`seal`] mints the nominal atoms opaque
//! ascription needs and keeps the table that makes their freshness checkable.
//!
//! What is *not* here is deliberate. The subsumption judgement lives in
//! [`discipline::subtype`] beside the other discipline the checker enforces,
//! the stack-typing judgement in [`machine::stack`] beside the machine whose
//! reified stacks it types, and the identity motive's value-into-type
//! substitution in `gandr_core_term::identity`, because it is substitution over
//! the substrate rather than a judgement.
//!
//! [`machine`]: crate::machine
//! [`discipline::subtype`]: crate::discipline::subtype
//! [`machine::stack`]: crate::machine::stack

pub mod checker;
pub mod package;
pub mod seal;
