//! A safe first-order engine for structurally recursive gandr algorithms.
//!
//! Caller-controlled syntax, type, and proof trees must not consume the native
//! call stack. This crate provides the small execution mechanism shared by
//! defunctionalized machines: a machine turns a request into either a return or
//! one child request plus an explicit continuation frame; [`run`] drives those
//! steps over a heap-backed frame stack.
//!
//! The engine deliberately owns no language semantics and performs no dynamic
//! dispatch. Each consumer defines closed request, frame, and output enums, so
//! its recursive control flow remains inspectable and exhaustively checked.

#![no_std]
#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        reason = "the standard test-allow set keeps unit and property tests readable (docs/workflow/rust.md)"
    )
)]

extern crate alloc;

use alloc::vec::Vec;

/// One machine transition: descend into a child or return a completed value.
///
/// # Contract
/// - requires: `Descend` carries the frame that resumes exactly the parent of
///   `request`;
/// - ensures: exposes every recursive edge as first-order data consumed by
///   [`run`];
/// - provides: an exhaustively typed alternative to native-stack recursion;
/// - panics: none.
#[expect(
    clippy::exhaustive_enums,
    reason = "the transition protocol is closed by design: Descend and Return are the complete vocabulary, and downstream machines construct both variants directly"
)]
pub enum Step<Request, Frame, Output>
{
    /// Evaluate one child and resume its parent through `frame`.
    Descend
    {
        /// Child request evaluated next.
        request: Request,
        /// Parent continuation retained until the child returns.
        frame: Frame,
    },
    /// Return one completed machine value.
    Return(Output),
}

/// Result of one machine transition.
pub type StepResult<M> = Result<
    Step<<M as Machine>::Request, <M as Machine>::Frame, <M as Machine>::Output>,
    <M as Machine>::Error,
>;

/// A defunctionalized recursive algorithm driven by [`run`].
///
/// # Contract
/// - requires: every `Descend` step makes progress according to the consumer's
///   documented well-founded measure;
/// - ensures: `begin` evaluates a request and `resume` consumes exactly one
///   frame/value pair;
/// - provides: consumer-owned request/frame semantics behind a shared safe
///   execution loop;
/// - fails: methods return the consumer's typed error;
/// - panics: none.
pub trait Machine
{
    /// One recursive request.
    type Request;
    /// One suspended parent continuation.
    type Frame;
    /// One completed request value.
    type Output;
    /// Typed machine failure.
    type Error;

    /// Starts or descends into one request.
    ///
    /// # Errors
    /// Returns the machine's typed failure when the request cannot advance.
    fn begin(
        &mut self,
        request: Self::Request,
    ) -> StepResult<Self>;

    /// Resumes one parent after its child returned.
    ///
    /// # Errors
    /// Returns the machine's typed failure when the frame cannot consume the
    /// returned child value.
    fn resume(
        &mut self,
        frame: Self::Frame,
        output: Self::Output,
    ) -> StepResult<Self>;
}

/// Runs a defunctionalized machine to completion without native recursion.
///
/// # Contract
/// - requires: `initial` and every emitted frame satisfy [`Machine`]'s
///   contract, and the machine eventually returns after finitely many steps;
/// - ensures: evaluates depth-first in the same call/return order as the
///   represented recursive algorithm while native call depth stays constant;
/// - provides: a heap-bounded continuation stack whose maximum length is the
///   represented algorithm's logical recursion depth;
/// - fails: returns the first typed error emitted by `machine`;
/// - panics: none.
///
/// # Errors
/// Returns the first [`Machine::Error`] produced while beginning or resuming a
/// step.
///
/// # Adequacy
/// - hypothesis: L3 algorithm — the loop must preserve recursive call/return
///   order and remain stack-safe for caller-controlled depth;
/// - witness: `tests::deep_countdown_uses_the_frame_stack` and
///   `tests::branching_machine_resumes_children_in_order`;
/// - mutation trigger: swap `frames.push(frame)` and `machine.begin(request)`,
///   discard a frame, or resume before a child returns;
/// - future programs: lowering, normalization, and proof traversal machines
///   instantiate the same request/frame protocol.
#[inline]
pub fn run<M>(
    machine: &mut M,
    initial: M::Request,
) -> Result<M::Output, M::Error>
where
    M: Machine,
{
    let mut frames = Vec::new();
    let mut step = machine.begin(initial)?;
    loop {
        step = match step {
            | Step::Descend { request, frame } => {
                frames.push(frame);
                machine.begin(request)?
            },
            | Step::Return(output) => match frames.pop() {
                | Some(frame) => machine.resume(frame, output)?,
                | None => return Ok(output),
            },
        };
    }
}

#[cfg(test)]
mod tests
{
    use core::convert::Infallible;

    use super::Machine;
    use super::Step;
    use super::run;

    /// Logical input depth for the test machines.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct Depth(usize);

    impl Depth
    {
        /// Zero depth.
        const ZERO: Self = Self(0);

        /// Returns the proper predecessor, if any.
        fn predecessor(self) -> Option<Self>
        {
            self.0.checked_sub(1).map(Self)
        }
    }

    impl From<usize> for Depth
    {
        fn from(value: usize) -> Self
        {
            Self(value)
        }
    }

    /// A countdown with one continuation frame per input layer.
    struct Countdown;

    impl Machine for Countdown
    {
        type Request = Depth;
        type Frame = ();
        type Output = Depth;
        type Error = Infallible;

        fn begin(
            &mut self,
            request: Self::Request,
        ) -> Result<Step<Self::Request, Self::Frame, Self::Output>, Self::Error>
        {
            Ok(request.predecessor().map_or_else(
                || Step::Return(Depth::ZERO),
                |request| Step::Descend { request, frame: () },
            ))
        }

        fn resume(
            &mut self,
            _frame: Self::Frame,
            output: Self::Output,
        ) -> Result<Step<Self::Request, Self::Frame, Self::Output>, Self::Error>
        {
            Ok(Step::Return(output))
        }
    }

    /// Completed leaf count for a branching traversal.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct LeafCount(usize);

    impl LeafCount
    {
        /// One leaf.
        const ONE: Self = Self(1);

        /// Combines disjoint subtree counts.
        fn combine(
            self,
            other: Self,
        ) -> Self
        {
            Self(self.0.saturating_add(other.0))
        }
    }

    impl From<usize> for LeafCount
    {
        fn from(value: usize) -> Self
        {
            Self(value)
        }
    }

    /// Continuation state for a full binary-tree traversal.
    enum BranchFrame
    {
        /// Visit the right subtree after the left completes.
        Left(Depth),
        /// Combine the retained left count after the right completes.
        Right(LeafCount),
    }

    /// A depth-first full binary-tree leaf counter.
    struct Branching;

    impl Machine for Branching
    {
        type Request = Depth;
        type Frame = BranchFrame;
        type Output = LeafCount;
        type Error = Infallible;

        fn begin(
            &mut self,
            request: Self::Request,
        ) -> Result<Step<Self::Request, Self::Frame, Self::Output>, Self::Error>
        {
            Ok(request.predecessor().map_or_else(
                || Step::Return(LeafCount::ONE),
                |child| Step::Descend {
                    request: child,
                    frame: BranchFrame::Left(child),
                },
            ))
        }

        fn resume(
            &mut self,
            frame: Self::Frame,
            output: Self::Output,
        ) -> Result<Step<Self::Request, Self::Frame, Self::Output>, Self::Error>
        {
            Ok(match frame {
                | BranchFrame::Left(right) => Step::Descend {
                    request: right,
                    frame: BranchFrame::Right(output),
                },
                | BranchFrame::Right(left) => Step::Return(left.combine(output)),
            })
        }
    }

    /// A caller-controlled depth far beyond the native stack remains safe.
    #[test]
    fn deep_countdown_uses_the_frame_stack()
    {
        /// Recursion depth that would overflow a native call stack.
        const DEEP_DEPTH: usize = 100_000;
        let output = run(&mut Countdown, Depth::from(DEEP_DEPTH)).expect("infallible machine");
        assert_eq!(Depth::ZERO, output);
    }

    /// Parent frames preserve left-to-right return order for branching calls.
    #[test]
    fn branching_machine_resumes_children_in_order()
    {
        let output = run(&mut Branching, Depth::from(10)).expect("infallible machine");
        assert_eq!(LeafCount::from(1_024), output);
    }
}
