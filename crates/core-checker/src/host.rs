//! The host-effect seam (ADR-35 D4) — the boundary at which the machine offers
//! an effect operation *no source-level handler claims* to an ambient host
//! interpreter.
//!
//! This is a **preserved boundary**: it is expressed over the public
//! [`Value`] / [`EffectSig`] surface and the operation *name* only — never a
//! machine continuation frame — so it stays representation-independent across
//! the evaluator that realizes it. That representation-independence is what
//! lets the seam survive the port from the CEK oracle ([`crate::eval`]) to its
//! successor: the two machines present the *identical* seam to the host, and
//! the L machine realization in `gandr_core_sequent` is the durable driver the
//! runtime-host binds against (the CEK drivers retire with the CEK).
//!
//! The three types here are the seam's whole public vocabulary: a [`HostOp`]
//! the machine hands out, a [`HostReply`] the host hands back, and the
//! [`HostHandler`] that mediates. The CEK drivers that offer them
//! ([`crate::eval::run_with_host`] / [`crate::eval::run_state_with_host`], via
//! [`crate::eval::State::pending_host_op`] /
//! [`crate::eval::State::resume_host`]) stay with the CEK and die with it; this
//! module is the seam's durable home precisely because the boundary outlives
//! any one machine.
//!
//! It is its own module (coordinator decision D1): [`crate::boundary`] is the
//! newtype vocabulary, [`crate::effect`] is the effect-row algebra, and the
//! host seam is a distinct ADR-35 D4 boundary concern.

use alloc::string::String;

use crate::boundary::OperationName;
use crate::effect::EffectSig;
use crate::syntax::Value;

/// The host's reply to an intercepted effect operation — the host-effect seam
/// (ADR-35 D4).
///
/// A [`HostHandler`] either resumes the operation with a value or declines it,
/// leaving the machine to take its ordinary step (which blames
/// [`crate::eval::Blame::PerformNoHandler`] on an unclaimed `perform`).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum HostReply
{
    /// Resume the intercepted operation with this reply value — the machine
    /// continues as if an always-resume ambient handler resumed the
    /// continuation (see [`HostHandler`] for the exact intercept-set
    /// invariant).
    Resume(
        /// The value delivered to the performing continuation.
        Value,
    ),
    /// Decline the operation — the machine takes its ordinary step, blaming
    /// [`crate::eval::Blame::PerformNoHandler`] when no in-term handler catches
    /// it.
    Unhandled,
}

/// A host-side interpreter for the effect operations no source-level handler
/// claims — the host-effect seam (ADR-35 D4; the earliest bootstrap gate).
///
/// **Invariant.** The host intercepts EXACTLY the operations that would
/// otherwise [`crate::eval::Blame::PerformNoHandler`] — those no source-level
/// handler claims across the structural prefix (see
/// [`crate::eval::State::pending_host_op`]). This equals an identity-return,
/// always-resume ambient handler ONLY in the absence of a `reset` and of
/// intervening non-matching handlers: a `perform` cut off from its in-term
/// handler by an intervening [`crate::eval::Cont::Reset`] or a non-matching
/// [`crate::eval::Cont::Handle`] is host-interceptable here even though that
/// handler encloses it in a from-the-outside reading (the v0 structural
/// single-handler scope, pinned by `host_seam_intercepts_across_a_reset` and
/// `host_seam_intercepts_across_an_intervening_handler`).
///
/// [`crate::eval::run_state_with_host`] offers the host every such `perform`
/// (see [`crate::eval::State::pending_host_op`]) and either resumes
/// ([`HostReply::Resume`]) or declines ([`HostReply::Unhandled`]). The handler
/// sees only the public [`Value`] / [`EffectSig`] surface and the operation
/// *name* — never a continuation frame — so the seam stays
/// representation-independent across machine/arena changes.
///
/// Any `FnMut(&EffectSig, &str, &Value) -> HostReply` is a `HostHandler` via
/// the blanket impl below, so a closure suffices for the common case.
pub trait HostHandler
{
    /// Handles the operation `op` of signature `sig` performed with `payload`,
    /// returning the host's [`HostReply`].
    ///
    /// # Contract
    /// - ensures: [`HostReply::Resume`] to continue the machine with a reply,
    ///   or [`HostReply::Unhandled`] to let it take its ordinary (blaming)
    ///   step.
    fn handle<'source, O>(
        &mut self,
        sig: &EffectSig,
        op: O,
        payload: &Value,
    ) -> HostReply
    where
        O: Into<OperationName<'source>>;
}

impl<F> HostHandler for F
where
    F: FnMut(&EffectSig, &str, &Value) -> HostReply,
{
    #[inline]
    fn handle<'source, O>(
        &mut self,
        sig: &EffectSig,
        op: O,
        payload: &Value,
    ) -> HostReply
    where
        O: Into<OperationName<'source>>,
    {
        let op = op.into();
        self(sig, op.as_ref(), payload)
    }
}

/// A host-interceptable effect operation carried out of the machine as an
/// **owned** payload (ADR-35 D4; see [`crate::eval::State::pending_host_op`]).
///
/// Owned by design: under the coming arena the payload is a node with no
/// `&Value` to borrow, so the seam hands the host a self-contained operation
/// over the public [`Value`] API rather than a borrow into machine internals.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct HostOp
{
    /// The effect signature `E` the operation belongs to.
    pub sig: EffectSig,
    /// The operation's name (an operation of `sig`).
    pub op: String,
    /// The performed payload, with type annotations stripped.
    pub payload: Value,
}

impl HostOp
{
    /// Assembles a host-interceptable operation over the public surface.
    ///
    /// [`HostOp`] is `#[non_exhaustive]`, so this constructor is how a machine
    /// *outside* this crate — the L machine realization in `gandr_core_sequent`
    /// — builds the offer it hands to a [`HostHandler`], the same triple the
    /// CEK packages in [`crate::eval::State::pending_host_op`]. This is
    /// what keeps the seam a single shared boundary across the two machines
    /// rather than two parallel vocabularies.
    ///
    /// # Contract
    /// - ensures: a [`HostOp`] carrying `sig`, the owned operation name of
    ///   `op`, and `payload` verbatim (the caller has already read the payload
    ///   back over the public [`Value`] surface, annotation-stripped).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        sig: EffectSig,
        op: OperationName<'_>,
        payload: Value,
    ) -> Self
    {
        Self {
            sig,
            op: String::from(op.as_ref()),
            payload,
        }
    }
}
