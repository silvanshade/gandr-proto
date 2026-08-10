//! Typed-IL well-formedness checks (the sequent-machines design's §2.3).
//!
//! The IL's own discipline is the two-sided sequent (`Γ ⊢ p : A | Δ`,
//! `Γ | c : A ⊢ Δ`, `Γ ⊢ s ⊢ Δ`); the user-facing gate stays the frozen-core
//! bidirectional checker, so this is the **debug-assertion face** the focusing
//! translation and (later) the completion engine lean on — a rewrite must
//! preserve IL well-formedness. Full two-sided *type* checking needs the source
//! types the L0 command carrier deliberately does not retain; what is decidable
//! from the command alone, and what this module verifies, is the structural
//! spine of well-formedness:
//!
//! 1. **Reference integrity** — every child id resolves in the arena.
//! 2. **Scope** — every variable / covariable reference is bound by an
//!    enclosing binder, and the free sets are returned so the caller can assert
//!    the top-level invariant (a `𝓕`-produced command has *no* free
//!    covariables, `★` being the only top continuation).
//! 3. **Focus** — a producer in argument position (`K(p̄; …)`, `D(p̄; …)`,
//!    `prim(p̄; …)`) is a substitutable value `𝔭`, never a context capture
//!    `μα.s`.
//! 4. **Arity** — a node carries the argument counts **its own head declares**
//!    (§2.1). A constructor and a destructor frame carry the producer count of
//!    [`CtorTag::producer_arity`] / [`DtorTag::producer_arity`]; a constructor,
//!    a destructor frame, and a `prim` command carry the consumer count of
//!    [`CtorTag::consumer_arity`] / [`DtorTag::consumer_arity`] /
//!    [`PrimOp::consumer_arity`]. This walk reads those declarations and holds
//!    no arity constant of its own, so a head admitting several return
//!    continuations is admitted here without a change (the
//!    intermediate-language half of the multi-output axis:
//!    `docs/gandr/spec/implementation/circuit-terms.md`, the execution ladder's
//!    `circuit-terms-rung-06`). A [`CommandNode::Jump`] is the one
//!    argument-carrying command whose head is a definition name rather than a
//!    tag, so neither count is decidable from the command alone and its
//!    arguments are walked without being counted.
//! 5. **Polarity** — a cut's `ε` is consistent with the producer's polarity (a
//!    codata `cocase` is a negative cut; a literal / constructor / thunk /
//!    boxed covalue is a positive cut; a variable or `μ` capture is
//!    unconstrained here).
//!
//! The walk is recursive over the command tree the translation builds (a tree,
//! not a shared DAG — content-addressed sharing is a phase-L2 concern),
//! with a depth guard so a malformed hand-built cyclic input
//! reports rather than recurses without bound.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::boundary::NameRef;

use crate::boundary::CheckDepth;
use crate::boundary::ConsumerArity;
use crate::boundary::WellformedDecision;
use crate::il::CommandArena;
use crate::il::CommandId;
use crate::il::CommandNode;
use crate::il::ConsumerId;
use crate::il::ConsumerNode;
use crate::il::CtorTag;
use crate::il::DtorTag;
use crate::il::Polarity;
use crate::il::PrimOp;
use crate::il::ProducerId;
use crate::il::ProducerNode;

/// The recursion-depth ceiling for the well-formedness walk — far above any
/// real focused term, present only so a malformed cyclic hand-built input
/// reports [`CheckError::DepthExceeded`] rather than overflowing the stack.
const DEPTH_LIMIT: u32 = 0x0004_0000;

/// The free variables and covariables of a (sub)command.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Frees
{
    /// The free term variables.
    pub vars: BTreeSet<String>,
    /// The free covariables.
    pub covars: BTreeSet<String>,
}

impl Frees
{
    /// The empty free set.
    fn empty() -> Self
    {
        Self::default()
    }

    /// Merges another free set into this one.
    fn merge(
        mut self,
        other: Self,
    ) -> Self
    {
        self.vars.extend(other.vars);
        self.covars.extend(other.covars);
        self
    }
}

/// The head a consumer-arity violation was found at — the tag whose
/// declaration the node's consumer list `c̄` failed to meet.
///
/// Naming the head is what makes the diagnostic actionable: the count the walk
/// held the node to came from this tag, not from the walk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConsumerArityHead
{
    /// A positive constructor `K(p̄; c̄)` ([`ProducerNode::Ctor`]).
    Ctor(CtorTag),
    /// A destructor frame `D(p̄; c̄)` ([`ConsumerNode::Dtor`]).
    Dtor(DtorTag),
    /// A native `prim(p̄; c̄)` command ([`CommandNode::Prim`]).
    Prim(PrimOp),
}

impl core::fmt::Display for ConsumerArityHead
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::Ctor(_) => f.write_str("constructor"),
            | Self::Dtor(_) => f.write_str("destructor frame"),
            | Self::Prim(_) => f.write_str("prim command"),
        }
    }
}

/// A well-formedness violation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckError
{
    /// A producer id did not resolve in the arena.
    DanglingProducer(ProducerId),
    /// A consumer id did not resolve in the arena.
    DanglingConsumer(ConsumerId),
    /// A command id did not resolve in the arena.
    DanglingCommand(CommandId),
    /// A constructor's producer-argument count did not match its tag's arity.
    CtorArity
    {
        /// The offending tag.
        tag: CtorTag,
        /// The arity the tag declares.
        expected: usize,
        /// The argument count found.
        found: usize,
    },
    /// A destructor's producer-argument count did not match its tag's arity.
    DtorArity
    {
        /// The offending tag.
        tag: DtorTag,
        /// The arity the tag declares.
        expected: usize,
        /// The argument count found.
        found: usize,
    },
    /// A node's consumer list `c̄` did not match the arity its head declares.
    ConsumerArity
    {
        /// The head whose declaration was violated.
        head: ConsumerArityHead,
        /// The consumer arity the head declares.
        expected: ConsumerArity,
        /// The consumer-child count found.
        found: ConsumerArity,
    },
    /// A producer in argument position was a non-value (a `μ` context capture).
    NonValueArgument(ProducerId),
    /// A cut's polarity was inconsistent with its producer's polarity.
    PolarityMismatch
    {
        /// The producer half of the cut.
        producer: ProducerId,
        /// The recorded (inconsistent) polarity.
        polarity: Polarity,
    },
    /// The walk exceeded [`DEPTH_LIMIT`] — a sign of a malformed cyclic input.
    DepthExceeded,
}

impl core::fmt::Display for CheckError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::DanglingProducer(id) => {
                write!(f, "producer id {} does not resolve", u32::from(id.index()))
            },
            | Self::DanglingConsumer(id) => {
                write!(f, "consumer id {} does not resolve", u32::from(id.index()))
            },
            | Self::DanglingCommand(id) => {
                write!(f, "command id {} does not resolve", u32::from(id.index()))
            },
            | Self::CtorArity {
                expected, found, ..
            } => write!(
                f,
                "constructor arity mismatch: expected {expected} argument(s), found {found}"
            ),
            | Self::DtorArity {
                expected, found, ..
            } => write!(
                f,
                "destructor arity mismatch: expected {expected} argument(s), found {found}"
            ),
            | Self::ConsumerArity {
                ref head,
                expected,
                found,
            } => write!(
                f,
                "consumer arity mismatch: the {head} tag declares {expected} consumer child(ren), \
                 found {found}"
            ),
            | Self::NonValueArgument(id) => write!(
                f,
                "producer id {} in argument position is not a value",
                u32::from(id.index())
            ),
            | Self::PolarityMismatch { producer, .. } => write!(
                f,
                "cut polarity is inconsistent with producer id {}",
                u32::from(producer.index())
            ),
            | Self::DepthExceeded => f.write_str("well-formedness walk exceeded the depth limit"),
        }
    }
}

impl core::error::Error for CheckError
{
}

/// Whether a command tree is well-formed (the boolean face of [`wellformed`]).
///
/// # Contract
/// - ensures: `true` iff [`wellformed`] returns `Ok`.
#[inline]
#[must_use]
pub fn is_wellformed(
    arena: &CommandArena,
    root: CommandId,
) -> WellformedDecision
{
    wellformed(arena, root).is_ok().into()
}

/// Checks a command tree for typed-IL well-formedness, returning its free sets.
///
/// # Contract
/// - ensures: `Ok(frees)` iff the reachable command tree satisfies reference
///   integrity, scope, focus, arity, and polarity consistency; `frees` are the
///   command's free variables and covariables.
///
/// # Errors
/// The first [`CheckError`] the walk encounters.
#[inline]
pub fn wellformed(
    arena: &CommandArena,
    root: CommandId,
) -> Result<Frees, CheckError>
{
    let checker = Checker { arena };
    let empty = BTreeSet::new();
    checker.command(root, &empty, &empty, CheckDepth::ROOT)
}

/// The well-formedness walk, borrowing the arena for the duration.
#[repr(transparent)]
struct Checker<'arena>
{
    /// The arena being checked.
    arena: &'arena CommandArena,
}

/// One well-formedness walk target — the sort of node a work entry checks.
#[derive(Clone)]
enum CheckTarget
{
    /// A command node.
    Command(CommandId),
    /// A producer node.
    Producer(ProducerId),
    /// A consumer node.
    Consumer(ConsumerId),
}

/// One work-stack entry of the well-formedness walk: a target plus the scope
/// and descent depth in force at its position.
#[derive(Clone)]
struct CheckWork
{
    /// The node to check.
    target: CheckTarget,
    /// The variables in scope at the target.
    bound_vars: BTreeSet<String>,
    /// The covariables in scope at the target.
    bound_covars: BTreeSet<String>,
    /// The remaining descent budget.
    depth: CheckDepth,
}

impl CheckWork
{
    /// Builds a work entry, snapshotting the ambient scope sets.
    fn new(
        target: CheckTarget,
        bound_vars: &BTreeSet<String>,
        bound_covars: &BTreeSet<String>,
        depth: CheckDepth,
    ) -> Self
    {
        Self {
            target,
            bound_vars: bound_vars.clone(),
            bound_covars: bound_covars.clone(),
            depth,
        }
    }
}

impl Checker<'_>
{
    /// Checks a command, returning its free sets under the ambient scope.
    ///
    /// # Termination
    /// - reason: an explicit work stack follows command-IL child edges until a
    ///   leaf, dangling id, or depth error.
    /// - measure: the finite work stack shrinks or receives strict child ids at
    ///   the next [`CheckDepth`].
    /// - boundedness: [`DEPTH_LIMIT`] caps malformed cyclic arenas before
    ///   host-stack exhaustion.
    /// - input recursion: none; caller-supplied arena ids are traversed by the
    ///   explicit work stack.
    fn command(
        &self,
        id: CommandId,
        bound_vars: &BTreeSet<String>,
        bound_covars: &BTreeSet<String>,
        depth: CheckDepth,
    ) -> Result<Frees, CheckError>
    {
        self.walk(vec![CheckWork::new(
            CheckTarget::Command(id),
            bound_vars,
            bound_covars,
            depth,
        )])
    }

    /// Schedules the consumer and producer arguments of a node onto the work
    /// stack (innermost-last, so children pop in source order), checking each
    /// producer's value-ness before it is scheduled.
    ///
    /// # Contract
    /// - requires: `ps` / `cs` are the node's argument id slices.
    /// - ensures: every argument is queued under the same scope and depth; a
    ///   non-value producer argument fails with
    ///   [`CheckError::NonValueArgument`].
    /// - fails: [`CheckError::NonValueArgument`], or a dangling id from the
    ///   value-ness probe.
    fn push_arguments(
        &self,
        stack: &mut Vec<CheckWork>,
        ps: &[ProducerId],
        cs: &[ConsumerId],
        bound_vars: &BTreeSet<String>,
        bound_covars: &BTreeSet<String>,
        depth: CheckDepth,
    ) -> Result<(), CheckError>
    {
        for &consumer in cs.iter().rev() {
            stack.push(CheckWork::new(
                CheckTarget::Consumer(consumer),
                bound_vars,
                bound_covars,
                depth,
            ));
        }
        for &producer in ps.iter().rev() {
            self.check_value_argument(producer)?;
            stack.push(CheckWork::new(
                CheckTarget::Producer(producer),
                bound_vars,
                bound_covars,
                depth,
            ));
        }
        Ok(())
    }

    /// Schedules a command target under the given scope and depth.
    fn push_command(
        stack: &mut Vec<CheckWork>,
        id: CommandId,
        bound_vars: &BTreeSet<String>,
        bound_covars: &BTreeSet<String>,
        depth: CheckDepth,
    )
    {
        stack.push(CheckWork::new(
            CheckTarget::Command(id),
            bound_vars,
            bound_covars,
            depth,
        ));
    }

    /// Drains the well-formedness work stack, unioning the free sets of every
    /// reachable node.
    ///
    /// # Termination
    /// - reason: each pop checks one node and schedules only strict child ids.
    /// - measure: the finite work stack shrinks or receives strict child ids at
    ///   the next [`CheckDepth`].
    /// - boundedness: [`DEPTH_LIMIT`] caps malformed cyclic arenas before
    ///   host-stack exhaustion.
    /// - input recursion: none; the explicit work stack traverses
    ///   caller-supplied arena ids.
    fn walk(
        &self,
        mut stack: Vec<CheckWork>,
    ) -> Result<Frees, CheckError>
    {
        let mut frees = Frees::empty();
        while let Some(work) = stack.pop() {
            let depth = descend(work.depth)?;
            match work.target {
                | CheckTarget::Command(id) => {
                    let node = self
                        .arena
                        .command(id)
                        .ok_or(CheckError::DanglingCommand(id))?;
                    match *node {
                        | CommandNode::Cut {
                            pol,
                            producer,
                            consumer,
                        } => {
                            self.check_polarity(producer, pol)?;
                            stack.push(CheckWork::new(
                                CheckTarget::Consumer(consumer),
                                &work.bound_vars,
                                &work.bound_covars,
                                depth,
                            ));
                            stack.push(CheckWork::new(
                                CheckTarget::Producer(producer),
                                &work.bound_vars,
                                &work.bound_covars,
                                depth,
                            ));
                        },
                        | CommandNode::Prim { op, ref ps, ref cs } => {
                            let expected = op.consumer_arity();
                            if let Some(found) = consumer_arity_mismatch(expected, cs) {
                                return Err(CheckError::ConsumerArity {
                                    head: ConsumerArityHead::Prim(op),
                                    expected,
                                    found,
                                });
                            }
                            self.push_arguments(
                                &mut stack,
                                ps,
                                cs,
                                &work.bound_vars,
                                &work.bound_covars,
                                depth,
                            )?;
                        },
                        // A jump's head is a definition name, not a tag: both
                        // its counts belong to the named definition's
                        // signature, which the L0 command carrier does not
                        // retain, so nothing is decidable from the command
                        // alone and the arguments are walked uncounted.
                        | CommandNode::Jump { ref ps, ref cs, .. } => {
                            self.push_arguments(
                                &mut stack,
                                ps,
                                cs,
                                &work.bound_vars,
                                &work.bound_covars,
                                depth,
                            )?;
                        },
                    }
                },
                | CheckTarget::Producer(id) => {
                    let node = self
                        .arena
                        .producer(id)
                        .ok_or(CheckError::DanglingProducer(id))?;
                    match *node {
                        | ProducerNode::Var(ref name) => {
                            frees = frees.merge(free_name(
                                NameRef::from(name.as_str()),
                                &work.bound_vars,
                                FreeNameKind::Var,
                            ));
                        },
                        | ProducerNode::Lit(_) => {},
                        | ProducerNode::Mu(ref covar, body) => {
                            let inner = extend(&work.bound_covars, core::slice::from_ref(covar));
                            Self::push_command(&mut stack, body, &work.bound_vars, &inner, depth);
                        },
                        | ProducerNode::Ctor {
                            ref tag,
                            ref ps,
                            ref cs,
                        } => {
                            let expected = tag.producer_arity();
                            if ps.len() != usize::from(expected) {
                                return Err(CheckError::CtorArity {
                                    tag: tag.clone(),
                                    expected: usize::from(expected),
                                    found: ps.len(),
                                });
                            }
                            let expected = tag.consumer_arity();
                            if let Some(found) = consumer_arity_mismatch(expected, cs) {
                                return Err(CheckError::ConsumerArity {
                                    head: ConsumerArityHead::Ctor(tag.clone()),
                                    expected,
                                    found,
                                });
                            }
                            self.push_arguments(
                                &mut stack,
                                ps,
                                cs,
                                &work.bound_vars,
                                &work.bound_covars,
                                depth,
                            )?;
                        },
                        | ProducerNode::Cocase { ref arms } => {
                            for arm in arms.iter().rev() {
                                let inner_vars = extend(&work.bound_vars, &arm.binders);
                                let inner_covars = extend(&work.bound_covars, &arm.cobinders);
                                Self::push_command(
                                    &mut stack,
                                    arm.body,
                                    &inner_vars,
                                    &inner_covars,
                                    depth,
                                );
                            }
                        },
                        | ProducerNode::Thunk {
                            ref cobinder, body, ..
                        } => {
                            let inner = extend(&work.bound_covars, core::slice::from_ref(cobinder));
                            Self::push_command(&mut stack, body, &work.bound_vars, &inner, depth);
                        },
                        | ProducerNode::Boxed(consumer) => {
                            stack.push(CheckWork::new(
                                CheckTarget::Consumer(consumer),
                                &work.bound_vars,
                                &work.bound_covars,
                                depth,
                            ));
                        },
                        | ProducerNode::Shift { ref binder, body } => {
                            let inner = extend(&work.bound_vars, core::slice::from_ref(binder));
                            Self::push_command(&mut stack, body, &inner, &work.bound_covars, depth);
                        },
                    }
                },
                | CheckTarget::Consumer(id) => {
                    let node = self
                        .arena
                        .consumer(id)
                        .ok_or(CheckError::DanglingConsumer(id))?;
                    match *node {
                        | ConsumerNode::CoVar(ref name) => {
                            frees = frees.merge(free_name(
                                NameRef::from(name.as_str()),
                                &work.bound_covars,
                                FreeNameKind::Covar,
                            ));
                        },
                        | ConsumerNode::MuTilde(ref binder, body) => {
                            let inner = extend(&work.bound_vars, core::slice::from_ref(binder));
                            Self::push_command(&mut stack, body, &inner, &work.bound_covars, depth);
                        },
                        | ConsumerNode::Dtor {
                            ref tag,
                            ref ps,
                            ref cs,
                        } => {
                            let expected = tag.producer_arity();
                            if ps.len() != usize::from(expected) {
                                return Err(CheckError::DtorArity {
                                    tag: tag.clone(),
                                    expected: usize::from(expected),
                                    found: ps.len(),
                                });
                            }
                            let expected = tag.consumer_arity();
                            if let Some(found) = consumer_arity_mismatch(expected, cs) {
                                return Err(CheckError::ConsumerArity {
                                    head: ConsumerArityHead::Dtor(tag.clone()),
                                    expected,
                                    found,
                                });
                            }
                            self.push_arguments(
                                &mut stack,
                                ps,
                                cs,
                                &work.bound_vars,
                                &work.bound_covars,
                                depth,
                            )?;
                        },
                        | ConsumerNode::Case { ref arms } => {
                            for arm in arms.iter().rev() {
                                let inner_vars = extend(&work.bound_vars, &arm.binders);
                                let inner_covars = extend(&work.bound_covars, &arm.cobinders);
                                Self::push_command(
                                    &mut stack,
                                    arm.body,
                                    &inner_vars,
                                    &inner_covars,
                                    depth,
                                );
                            }
                        },
                        | ConsumerNode::Top => {},
                        | ConsumerNode::Handler(ref handler) => {
                            for clause in handler.ops.iter().rev() {
                                let op_scope = extend(&work.bound_vars, &[
                                    clause.payload.clone(),
                                    clause.resume.clone(),
                                ]);
                                Self::push_command(
                                    &mut stack,
                                    clause.body,
                                    &op_scope,
                                    &work.bound_covars,
                                    depth,
                                );
                            }
                            let ret_scope = extend(
                                &work.bound_vars,
                                core::slice::from_ref(&handler.ret_binder),
                            );
                            Self::push_command(
                                &mut stack,
                                handler.ret_body,
                                &ret_scope,
                                &work.bound_covars,
                                depth,
                            );
                            stack.push(CheckWork::new(
                                CheckTarget::Consumer(handler.continuation),
                                &work.bound_vars,
                                &work.bound_covars,
                                depth,
                            ));
                        },
                        | ConsumerNode::Prompt(inner) => {
                            stack.push(CheckWork::new(
                                CheckTarget::Consumer(inner),
                                &work.bound_vars,
                                &work.bound_covars,
                                depth,
                            ));
                        },
                    }
                },
            }
        }
        Ok(frees)
    }

    /// Asserts a producer in argument position is a substitutable value (not a
    /// `μ` context capture).
    fn check_value_argument(
        &self,
        id: ProducerId,
    ) -> Result<(), CheckError>
    {
        let node = self
            .arena
            .producer(id)
            .ok_or(CheckError::DanglingProducer(id))?;
        if matches!(*node, ProducerNode::Mu(_, _) | ProducerNode::Shift { .. }) {
            return Err(CheckError::NonValueArgument(id));
        }
        Ok(())
    }

    /// Asserts a cut's polarity is consistent with its producer's polarity.
    fn check_polarity(
        &self,
        producer: ProducerId,
        polarity: Polarity,
    ) -> Result<(), CheckError>
    {
        let node = self
            .arena
            .producer(producer)
            .ok_or(CheckError::DanglingProducer(producer))?;
        let consistent = match *node {
            | ProducerNode::Cocase { .. } => polarity == Polarity::Negative,
            | ProducerNode::Lit(_)
            | ProducerNode::Ctor { .. }
            | ProducerNode::Thunk { .. }
            | ProducerNode::Boxed(_) => polarity == Polarity::Positive,
            // A variable, a context capture, or a `shift` capture may meet
            // either polarity (the focusing emits the shift cut positive).
            | ProducerNode::Var(_) | ProducerNode::Mu(..) | ProducerNode::Shift { .. } => true,
        };
        if consistent {
            Ok(())
        }
        else {
            Err(CheckError::PolarityMismatch { producer, polarity })
        }
    }
}

/// The consumer-child count a node actually carries, when it disagrees with
/// the arity its head declares.
///
/// The one place the walk compares a consumer list against a declaration, so
/// every consumer-carrying head is held to the same rule: **exactly** the
/// declared count, never at least it. Reporting the found count rather than a
/// decision keeps the offending head's tag out of the happy path, where cloning
/// it would cost an allocation on every well-formed node.
///
/// # Contract
/// - requires: `declared` is the arity `cs`'s own head declares
///   ([`CtorTag::consumer_arity`], [`DtorTag::consumer_arity`], or
///   [`PrimOp::consumer_arity`]).
/// - ensures: `None` exactly when `cs.len()` equals `declared`.
/// - provides: the offending count, for the [`CheckError::ConsumerArity`] the
///   caller raises against its own head.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — the decision surface is one equality, so the boundary
///   inputs are a list one short of the declaration, one long, and exactly
///   equal, at a head declaring zero and at a head declaring one; the pair of
///   declarations is what separates reading the declaration from a constant.
/// - witness: `check::tests::consumer_arity_follows_the_head_not_a_constant`
/// - witness: `check::tests::constructor_consumer_arity_is_checked`
/// - witness: `check::tests::destructor_consumer_arity_is_checked`
/// - witness: `check::tests::prim_consumer_arity_is_checked`
fn consumer_arity_mismatch(
    declared: ConsumerArity,
    cs: &[ConsumerId],
) -> Option<ConsumerArity>
{
    let found = ConsumerArity::from(cs.len());
    (found != declared).then_some(found)
}

/// Increments the depth, reporting [`CheckError::DepthExceeded`] past the
/// ceiling.
fn descend(depth: CheckDepth) -> Result<CheckDepth, CheckError>
{
    let next = u32::from(depth).wrapping_add(1_u32);
    if next > DEPTH_LIMIT {
        return Err(CheckError::DepthExceeded);
    }
    Ok(next.into())
}

/// Builds the scope extended by `names`.
fn extend(
    base: &BTreeSet<String>,
    names: &[String],
) -> BTreeSet<String>
{
    let mut out = base.clone();
    for name in names {
        let _inserted = out.insert(name.clone());
    }
    out
}

/// The free-set component a reference lands in.
#[derive(Clone, Copy)]
enum FreeNameKind
{
    /// A term-level variable (`x`).
    Var,
    /// A covariable (`α`).
    Covar,
}

/// The free set of a single reference: empty if bound, else the singleton,
/// placed in the variable or covariable component per `is_covar`.
fn free_name(
    name: NameRef<'_>,
    bound: &BTreeSet<String>,
    kind: FreeNameKind,
) -> Frees
{
    let mut frees = Frees::empty();
    if !bound.contains(name.as_ref()) {
        let _inserted = match kind {
            | FreeNameKind::Covar => frees.covars.insert(String::from(name.as_ref())),
            | FreeNameKind::Var => frees.vars.insert(String::from(name.as_ref())),
        };
    }
    frees
}

#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;
    use alloc::string::String;

    use gandr_core_checker::prim::NativePrim;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Value;

    use super::*;
    use crate::il::CommandNode;
    use crate::il::ConsumerNode;
    use crate::il::Lit;
    use crate::il::ProducerNode;

    /// A well-formed `⟨() | ★⟩` passes with empty free sets.
    #[test]
    fn terminal_cut_is_wellformed()
    {
        let mut arena = CommandArena::new();
        let producer = arena
            .alloc_producer(ProducerNode::Lit(Lit::Unit))
            .expect("room");
        let consumer = arena.alloc_consumer(ConsumerNode::Top).expect("room");
        let root = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer,
                consumer,
            })
            .expect("room");
        let frees = wellformed(&arena, root).expect("well-formed");
        assert!(frees.vars.is_empty(), "no free variables");
        assert!(frees.covars.is_empty(), "no free covariables");
    }

    /// A free variable is reported in the free set; a bound one is not.
    #[test]
    fn scope_tracks_binders()
    {
        let mut arena = CommandArena::new();
        let free = arena
            .alloc_producer(ProducerNode::Var(String::from("x")))
            .expect("room");
        let top = arena.alloc_consumer(ConsumerNode::Top).expect("room");
        let free_cut = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer: free,
                consumer: top,
            })
            .expect("room");
        let frees = wellformed(&arena, free_cut).expect("well-formed");
        assert!(frees.vars.contains("x"), "x is free");

        // ⟨x | μ̃x. ⟨x | ★⟩⟩ binds x on the consumer side.
        let bound_producer = arena
            .alloc_producer(ProducerNode::Var(String::from("x")))
            .expect("room");
        let inner_top = arena.alloc_consumer(ConsumerNode::Top).expect("room");
        let inner_cut = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer: bound_producer,
                consumer: inner_top,
            })
            .expect("room");
        let mu = arena
            .alloc_consumer(ConsumerNode::MuTilde(String::from("x"), inner_cut))
            .expect("room");
        let outer_producer = arena
            .alloc_producer(ProducerNode::Lit(Lit::Unit))
            .expect("room");
        let outer_cut = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer: outer_producer,
                consumer: mu,
            })
            .expect("room");
        let bound_frees = wellformed(&arena, outer_cut).expect("well-formed");
        assert!(!bound_frees.vars.contains("x"), "the μ̃ binder discharges x");
    }

    /// A dangling command id is reported.
    #[test]
    fn dangling_reference_is_rejected()
    {
        let arena = CommandArena::new();
        let phantom = CommandId::new(7);
        assert!(
            matches!(
                wellformed(&arena, phantom),
                Err(CheckError::DanglingCommand(_))
            ),
            "an unresolved command id is dangling"
        );
    }

    /// A `μ` context capture in argument position is rejected as a non-value.
    #[test]
    fn non_value_argument_is_rejected()
    {
        let mut arena = CommandArena::new();
        let inner_producer = arena
            .alloc_producer(ProducerNode::Lit(Lit::Unit))
            .expect("room");
        let inner_top = arena.alloc_consumer(ConsumerNode::Top).expect("room");
        let inner_cut = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer: inner_producer,
                consumer: inner_top,
            })
            .expect("room");
        let mu = arena
            .alloc_producer(ProducerNode::Mu(String::from("%a"), inner_cut))
            .expect("room");
        // Pair(μα.s ; unit) — the μ argument is a non-value.
        let unit = arena
            .alloc_producer(ProducerNode::Lit(Lit::Unit))
            .expect("room");
        let ctor = arena
            .alloc_producer(ProducerNode::Ctor {
                tag: CtorTag::Pair,
                ps: Box::from([mu, unit]),
                cs: Box::from([]),
            })
            .expect("room");
        let top = arena.alloc_consumer(ConsumerNode::Top).expect("room");
        let cut = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer: ctor,
                consumer: top,
            })
            .expect("room");
        assert!(
            matches!(
                wellformed(&arena, cut),
                Err(CheckError::NonValueArgument(_))
            ),
            "a μ capture in argument position is a non-value"
        );
    }

    /// A mislabeled polarity (a positive cut over a codata object) is rejected.
    #[test]
    fn polarity_mismatch_is_rejected()
    {
        let mut arena = CommandArena::new();
        let cocase = arena
            .alloc_producer(ProducerNode::Cocase {
                arms: Box::from([]),
            })
            .expect("room");
        let top = arena.alloc_consumer(ConsumerNode::Top).expect("room");
        let cut = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer: cocase,
                consumer: top,
            })
            .expect("room");
        assert!(
            matches!(
                wellformed(&arena, cut),
                Err(CheckError::PolarityMismatch { .. })
            ),
            "a codata object meets a negative cut, not a positive one"
        );
    }

    /// A constructor whose argument count contradicts its tag is rejected.
    #[test]
    fn constructor_arity_is_checked()
    {
        let mut arena = CommandArena::new();
        let only = arena
            .alloc_producer(ProducerNode::Lit(Lit::Unit))
            .expect("room");
        let ctor = arena
            .alloc_producer(ProducerNode::Ctor {
                tag: CtorTag::Pair,
                ps: Box::from([only]),
                cs: Box::from([]),
            })
            .expect("room");
        let top = arena.alloc_consumer(ConsumerNode::Top).expect("room");
        let cut = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer: ctor,
                consumer: top,
            })
            .expect("room");
        assert!(
            matches!(wellformed(&arena, cut), Err(CheckError::CtorArity { .. })),
            "a pair needs two arguments"
        );
    }

    /// Builds `⟨() |+ c⟩` and checks it, so a consumer under test is reached by
    /// the walk in its ordinary position.
    fn check_against_unit(
        arena: &mut CommandArena,
        consumer: ConsumerNode,
    ) -> Result<Frees, CheckError>
    {
        let producer = arena
            .alloc_producer(ProducerNode::Lit(Lit::Unit))
            .expect("room");
        let consumer = arena.alloc_consumer(consumer).expect("room");
        let root = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer,
                consumer,
            })
            .expect("room");
        wellformed(arena, root)
    }

    /// Builds `⟨p |+ ★⟩` for a producer under test and checks it.
    fn check_producer_against_top(
        arena: &mut CommandArena,
        producer: ProducerNode,
    ) -> Result<Frees, CheckError>
    {
        let producer = arena.alloc_producer(producer).expect("room");
        let consumer = arena.alloc_consumer(ConsumerNode::Top).expect("room");
        let root = arena
            .alloc_command(CommandNode::Cut {
                pol: Polarity::Positive,
                producer,
                consumer,
            })
            .expect("room");
        wellformed(arena, root)
    }

    /// Allocates a consumer list of the given length out of distinct terminal
    /// consumers, so a consumer-arity test varies only the count.
    fn tops(
        arena: &mut CommandArena,
        count: ConsumerArity,
    ) -> Box<[ConsumerId]>
    {
        core::iter::repeat_with(|| arena.alloc_consumer(ConsumerNode::Top).expect("room"))
            .take(usize::from(count))
            .collect()
    }

    /// A constructor is held to the consumer arity its tag declares — zero —
    /// so the multi-consumer constructor the walk used to admit silently is
    /// refused, and the diagnostic names the head and both counts.
    #[test]
    fn constructor_consumer_arity_is_checked()
    {
        let mut arena = CommandArena::new();
        let cs = tops(&mut arena, ConsumerArity::from(1_usize));
        let outcome = check_producer_against_top(&mut arena, ProducerNode::Ctor {
            tag: CtorTag::Nil,
            ps: Box::from([]),
            cs,
        });
        assert_eq!(
            Err(CheckError::ConsumerArity {
                head: ConsumerArityHead::Ctor(CtorTag::Nil),
                expected: ConsumerArity::from(0_usize),
                found: ConsumerArity::from(1_usize),
            }),
            outcome,
            "a frozen-core constructor declares no consumer children"
        );
    }

    /// A destructor frame is held to the consumer arity its tag declares — one
    /// — in both directions: a frame with no return continuation and a frame
    /// with two are each refused, with the exact counts reported.
    #[test]
    fn destructor_consumer_arity_is_checked()
    {
        let mut arena = CommandArena::new();
        let outcome = check_against_unit(&mut arena, ConsumerNode::Dtor {
            tag: DtorTag::Force,
            ps: Box::from([]),
            cs: Box::from([]),
        });
        assert_eq!(
            Err(CheckError::ConsumerArity {
                head: ConsumerArityHead::Dtor(DtorTag::Force),
                expected: ConsumerArity::from(1_usize),
                found: ConsumerArity::from(0_usize),
            }),
            outcome,
            "a destructor frame without its return continuation is refused"
        );

        let cs = tops(&mut arena, ConsumerArity::from(2_usize));
        let outcome = check_against_unit(&mut arena, ConsumerNode::Dtor {
            tag: DtorTag::Force,
            ps: Box::from([]),
            cs,
        });
        assert_eq!(
            Err(CheckError::ConsumerArity {
                head: ConsumerArityHead::Dtor(DtorTag::Force),
                expected: ConsumerArity::from(1_usize),
                found: ConsumerArity::from(2_usize),
            }),
            outcome,
            "a destructor frame with two return continuations is refused"
        );
    }

    /// A `prim` command is held to the consumer arity its head declares — one —
    /// which the walk did not police at all before the declaration existed.
    #[test]
    fn prim_consumer_arity_is_checked()
    {
        let mut arena = CommandArena::new();
        let unit = arena
            .alloc_producer(ProducerNode::Lit(Lit::Unit))
            .expect("room");
        let root = arena
            .alloc_command(CommandNode::Prim {
                op: PrimOp::Dup,
                ps: Box::from([unit]),
                cs: Box::from([]),
            })
            .expect("room");
        assert_eq!(
            Err(CheckError::ConsumerArity {
                head: ConsumerArityHead::Prim(PrimOp::Dup),
                expected: ConsumerArity::from(1_usize),
                found: ConsumerArity::from(0_usize),
            }),
            wellformed(&arena, root),
            "a prim command without its return continuation is refused"
        );

        let cs = tops(&mut arena, ConsumerArity::from(2_usize));
        let root = arena
            .alloc_command(CommandNode::Prim {
                op: PrimOp::Drop,
                ps: Box::from([unit]),
                cs,
            })
            .expect("room");
        assert_eq!(
            Err(CheckError::ConsumerArity {
                head: ConsumerArityHead::Prim(PrimOp::Drop),
                expected: ConsumerArity::from(1_usize),
                found: ConsumerArity::from(2_usize),
            }),
            wellformed(&arena, root),
            "a multi-consumer prim command is refused while its head declares one"
        );
    }

    /// The count comes from the head, not from the walk: **one** consumer child
    /// is admitted at a destructor frame and refused at a constructor, and
    /// **zero** is admitted at a constructor and refused at a destructor frame.
    /// Replacing the declaration lookup with either constant therefore fails
    /// half of this test.
    #[test]
    fn consumer_arity_follows_the_head_not_a_constant()
    {
        let mut arena = CommandArena::new();
        let cs = tops(&mut arena, ConsumerArity::from(1_usize));
        let admitted = check_against_unit(&mut arena, ConsumerNode::Dtor {
            tag: DtorTag::Force,
            ps: Box::from([]),
            cs,
        });
        assert_eq!(
            Ok(Frees::empty()),
            admitted,
            "one consumer child meets the destructor frame's declaration"
        );

        let admitted = check_producer_against_top(&mut arena, ProducerNode::Ctor {
            tag: CtorTag::Nil,
            ps: Box::from([]),
            cs: Box::from([]),
        });
        assert_eq!(
            Ok(Frees::empty()),
            admitted,
            "no consumer child meets the constructor's declaration"
        );

        let cs = tops(&mut arena, ConsumerArity::from(1_usize));
        let refused = check_producer_against_top(&mut arena, ProducerNode::Ctor {
            tag: CtorTag::Nil,
            ps: Box::from([]),
            cs,
        });
        assert!(
            matches!(refused, Err(CheckError::ConsumerArity { .. })),
            "the same one consumer child violates the constructor's declaration"
        );

        let refused = check_against_unit(&mut arena, ConsumerNode::Dtor {
            tag: DtorTag::Force,
            ps: Box::from([]),
            cs: Box::from([]),
        });
        assert!(
            matches!(refused, Err(CheckError::ConsumerArity { .. })),
            "the same empty consumer list violates the destructor frame's declaration"
        );
    }

    /// The declaration agrees with the builder: every `prim` command the
    /// focusing translation emits carries exactly the consumer count its head
    /// declares, so the declaration is true of `𝓕` and not merely
    /// self-consistent.
    #[test]
    fn focused_prim_commands_meet_the_declaration()
    {
        let comps = [
            Comp::dup(Value::Unit),
            Comp::drop(Value::Unit),
            Comp::native(NativePrim::Add),
        ];
        for comp in &comps {
            let focused = crate::focus::focus_comp(comp).expect("focuses");
            let command = focused
                .arena()
                .command(focused.root())
                .expect("the root resolves");
            let CommandNode::Prim { op, ref cs, .. } = *command
            else {
                panic!("focusing a structural or native op emits a prim command: {command:?}")
            };
            assert_eq!(
                usize::from(op.consumer_arity()),
                cs.len(),
                "𝓕 builds the consumer count {op:?} declares"
            );
        }
    }
}
