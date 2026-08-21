//! The small conversion-decision seam between the untrusted engine and the
//! certified kernel.
//!
//! This crate owns no term vocabulary, storage, wire representation, or replay
//! policy. Its generic identifier parameter keeps those concerns in the
//! consumer that owns the arena. A trace is a session artifact: the enum is a
//! shared vocabulary, not a serialization contract.
//!
//! The sink is deliberately statically dispatched. [`NullSink`] is an empty
//! monomorphized implementation, so a conversion path instantiated with it has
//! no recording state or dynamic dispatch to pay for.

#![no_std]

/// One decision made by a conversion strategy.
///
/// The identifier type is owned by the consumer. In the normalizer it names a
/// semantic or closure node; in the kernel it names an arena node. The values
/// are intentionally not interpreted here.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConversionDecision<Id>
{
    /// A definition was unfolded to continue the comparison.
    Unfold
    {
        /// The consumer's identifier for the definition head.
        constant: Id,
    },
    /// A definition was left for a later decision point.
    Postpone
    {
        /// The consumer's identifier for the definition head.
        constant: Id,
    },
    /// A suspended computation or thunk was forced for the comparison.
    Force
    {
        /// The consumer's identifier for the suspended computation.
        thunk: Id,
    },
    /// The comparison closed on two already-shared nodes.
    ComparedShared
    {
        /// The left consumer-owned node identifier.
        left: Id,
        /// The right consumer-owned node identifier.
        right: Id,
    },
}

/// A statically dispatched receiver of conversion decisions.
pub trait TraceSink<Id>
{
    /// Record one decision without imposing a storage policy on the caller.
    fn record(
        &mut self,
        decision: ConversionDecision<Id>,
    );
}

/// The default sink: recording is compiled away at the call site.
pub struct NullSink;

impl<Id> TraceSink<Id> for NullSink
{
    #[inline(always)]
    fn record(
        &mut self,
        _: ConversionDecision<Id>,
    )
    {
    }
}
