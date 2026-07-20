//! The S1 term language: [`Value`] (positive) and [`Computation`]
//! (negative), the closed vocabulary of fully-elaborated CBPV terms the kernel
//! checks at S1, plus the [`DeBruijnIndex`] and [`ConstantIndex`] reference
//! forms and the injection [`Side`].
//!
//! Terms are **nameless** (de Bruijn): a bound value variable is a
//! [`DeBruijnIndex`] counting binders outward from its use site, so
//! α-equivalence is syntactic identity and no name capture is representable.
//! A [`Value::Constant`] references a prior declaration in the append-only
//! environment by its admission position — the reference form the choke-point
//! audit walks (kernel-boundary.md §3); it is the one term the coordinator's
//! terse S1 stock did not name but the environment and its `#print axioms`
//! analogue structurally require.
//!
//! The vocabulary is **closed** (K1): no hole, no metavariable, no mark, no
//! annotation, no `dup`/`drop`, no effect/handler, no control operator, no
//! native, no datatype constructor exists to be represented.
//!
//! # Totality of destruction and duplication
//!
//! [`Value`] and [`Computation`] are mutually recursive owned trees whose depth
//! scales with untrusted input (export decode can build an arbitrarily deep
//! term from adversarial bytes, gandr-i3i). The derived `Drop` glue and a
//! derived `Clone` would both recurse on that depth and overflow the stack — a
//! divergence the kernel forbids (docs/workflow/rust.md, "the checker and the
//! machine are total"). Both are therefore **iterative** over an explicit heap
//! worklist (the [`crate::conv`] / [`crate::export`] precedent): `Clone` is a
//! hand-written two-stack goal/produced walk, and `Drop` extracts each node's
//! children into a worklist by placeholder-swap so the compiler's own drop glue
//! only ever sees leaves. `PartialEq`/`Eq`/`Hash` remain **derived** (hence
//! recursive) because the production comparator is [`crate::conv`] (iterative);
//! the derived relations are exercised only by tests on bounded fixtures — an
//! iterative rewrite is a tracked residuals candidate, not a live hazard.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::mem;

use gandr_kernel_strata::Level;

use crate::base::Literal;

/// A value variable, as a de Bruijn index counting binders outward: `0` is
/// the nearest enclosing binder.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeBruijnIndex(u32);

impl From<u32> for DeBruijnIndex
{
    #[inline]
    fn from(index: u32) -> Self
    {
        Self(index)
    }
}

impl From<DeBruijnIndex> for u32
{
    #[inline]
    fn from(index: DeBruijnIndex) -> Self
    {
        index.0
    }
}

/// A reference to a prior declaration in the append-only environment, by its
/// admission position (0 is the first admitted declaration).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConstantIndex(usize);

impl From<usize> for ConstantIndex
{
    #[inline]
    fn from(index: usize) -> Self
    {
        Self(index)
    }
}

impl From<ConstantIndex> for usize
{
    #[inline]
    fn from(index: ConstantIndex) -> Self
    {
        index.0
    }
}

/// The side of a sum injection.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "a sum has exactly two injections by design"
)]
pub enum Side
{
    /// The left injection, into the left summand of `A + B`.
    Left,
    /// The right injection, into the right summand of `A + B`.
    Right,
}

/// A value: the positive fragment of the S1 term vocabulary.
///
/// Values are the total, thunkable half of the polarity split — the fragment
/// conversion is permitted to compare (C5). No value constructor introduces a
/// computation effect; the only value that embeds a computation is
/// [`Self::Thunk`], and a thunk suspends rather than runs it.
///
/// `Clone` and `Drop` are hand-written to stay total on adversarial depth (the
/// module docs); `PartialEq`/`Eq`/`Hash` stay derived and recursive by design.
#[derive(Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the kernel's S1 value vocabulary is closed by design (kernel-boundary.md K1)"
)]
pub enum Value
{
    /// A bound value variable.
    Variable(DeBruijnIndex),
    /// A reference to a prior environment declaration.
    Constant(ConstantIndex),
    /// The unique inhabitant of the unit type.
    Unit,
    /// A base-type literal (integer, string, or numeric).
    Literal(Literal),
    /// A pair, introducing the product `A × B`.
    Pair(Box<Self>, Box<Self>),
    /// A sum injection, introducing `A + B` on the given side.
    Injection(Side, Box<Self>),
    /// A thunk, suspending a computation into the value type `U C`.
    Thunk(Box<Computation>),
    /// An explicit universe lift (`no implicit cumulativity`,
    /// kernel-boundary.md §7): given `body : A` with `A`'s level strictly below
    /// `target`, this value has type `Lift A target`. The lift is written, not
    /// inferred — a bare `body : A` never inhabits `Lift A target` on its own.
    Lift
    {
        /// The target universe level of the lift.
        target: Level,
        /// The value being lifted.
        body: Box<Self>,
    },
}

/// A computation: the negative fragment of the S1 term vocabulary.
///
/// Computations are the fragment the kernel **types but never evaluates**
/// during conversion (C5). Their eliminators (application, force, bind, case)
/// synthesize; their introductions (lambda, return) check against an expected
/// computation type.
///
/// `Clone` and `Drop` are hand-written to stay total on adversarial depth (the
/// module docs); `PartialEq`/`Eq`/`Hash` stay derived and recursive by design.
#[derive(Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the kernel's S1 computation vocabulary is closed by design (kernel-boundary.md K1)"
)]
pub enum Computation
{
    /// A lambda `λ. M`, binding one value variable; introduces `A → C`.
    Lambda(Box<Self>),
    /// An application `M v` of a computation to a value argument.
    Application(Box<Self>, Box<Value>),
    /// A returner `return v`, introducing `F A`.
    Return(Box<Value>),
    /// A sequencing bind `x ← M; N`, binding the value `M` returns into `N`.
    Bind(Box<Self>, Box<Self>),
    /// A force `force v` of a thunk value `v : U C`, running it as `C`.
    Force(Box<Value>),
    /// A sum elimination `case v { inl ⇒ M | inr ⇒ N }`, binding the injected
    /// value into each branch.
    Case
    {
        /// The scrutinee value (of a sum type).
        scrutinee: Box<Value>,
        /// The left branch, checked with the left summand bound.
        on_left: Box<Self>,
        /// The right branch, checked with the right summand bound.
        on_right: Box<Self>,
    },
}

/// The allocation-free placeholder a worklist swap leaves in a drained value
/// slot (the unit value is a leaf, so its own drop is `O(1)`).
#[inline]
const fn value_placeholder() -> Value
{
    Value::Unit
}

/// The placeholder a worklist swap leaves in a drained computation slot: a
/// trivial returner of unit. Computations have no leaf variant, so this costs
/// one transient allocation, reclaimed when the draining node falls.
#[inline]
fn comp_placeholder() -> Computation
{
    Computation::Return(Box::new(value_placeholder()))
}

/// A cloned term node, tagged by its polarity (mirrors the export reader's
/// `TermNode`): the produced register of the iterative clone walk.
enum TermNode
{
    /// A cloned value.
    Value(Value),
    /// A cloned computation.
    Comp(Computation),
}

/// A borrowed source term still to be cloned, tagged by polarity — the goal of
/// the iterative clone walk.
enum TermGoal<'src>
{
    /// A value to clone.
    Value(&'src Value),
    /// A computation to clone.
    Comp(&'src Computation),
}

/// A pending clone frame: a parent whose already-cloned children are held and
/// whose remaining source children are borrowed. Mirrors the export reader's
/// `TermFrame`, but carries the source of the not-yet-cloned children (the walk
/// reads a borrowed tree, not a byte stream).
enum TermFrame<'src>
{
    /// A pair: first child cloning; the second child's source is held.
    PairFirst(&'src Value),
    /// A pair: first child cloned; the second is cloning.
    PairSecond(Value),
    /// An injection: the side is held; the body is cloning.
    Injection(Side),
    /// A value thunk: the body computation is cloning.
    Thunk,
    /// A value lift: the target level is held; the body is cloning.
    Lift(Level),
    /// A lambda: the body computation is cloning.
    Lambda,
    /// An application: the head is cloning; the argument's source is held.
    ApplicationHead(&'src Value),
    /// An application: the head is cloned; the argument is cloning.
    ApplicationArgument(Computation),
    /// A returner: the value is cloning.
    Return,
    /// A force: the value is cloning.
    Force,
    /// A bind: the bound computation is cloning; the body's source is held.
    BindBound(&'src Computation),
    /// A bind: the bound computation is cloned; the body is cloning.
    BindBody(Computation),
    /// A case: the scrutinee is cloning; both branch sources are held.
    CaseScrutinee
    {
        /// The left branch's source.
        on_left: &'src Computation,
        /// The right branch's source.
        on_right: &'src Computation,
    },
    /// A case: the scrutinee is cloned; the left branch is cloning; the right
    /// branch's source is held.
    CaseLeft
    {
        /// The cloned scrutinee.
        scrutinee: Value,
        /// The right branch's source.
        on_right: &'src Computation,
    },
    /// A case: the scrutinee and left branch are cloned; the right is cloning.
    CaseRight
    {
        /// The cloned scrutinee.
        scrutinee: Value,
        /// The cloned left branch.
        on_left: Computation,
    },
}

/// Project the value out of a produced term node.
///
/// # Contract
/// - requires: `node` was produced for a value slot — the clone walk expands a
///   value source into a value node, so the polarity matches by construction.
/// - ensures: the produced value.
/// - provides: the total, infallible polarity projection the value frames need.
/// - fails: never — a computation node is unreachable here; the fallback is the
///   unit value, and a mis-clone is caught by the deep-clone depth witnesses.
/// - panics: none.
#[inline]
fn expect_value(node: TermNode) -> Value
{
    match node {
        | TermNode::Value(value) => value,
        | TermNode::Comp(_) => value_placeholder(),
    }
}

/// Project the computation out of a produced term node.
///
/// # Contract
/// - requires: `node` was produced for a computation slot (polarity matches by
///   construction, as in [`expect_value`]).
/// - ensures: the produced computation.
/// - provides: the total polarity projection the computation frames need.
/// - fails: never — a value node is unreachable here; the fallback is the unit
///   returner, and a mis-clone is caught by the deep-clone depth witnesses.
/// - panics: none.
#[inline]
fn expect_comp(node: TermNode) -> Computation
{
    match node {
        | TermNode::Comp(computation) => computation,
        | TermNode::Value(_) => comp_placeholder(),
    }
}

/// Deep-clone a term iteratively over a goal register and a frame stack.
///
/// # Contract
/// - requires: nothing — any borrowed value/computation tree is cloneable.
/// - ensures: a structurally identical owned copy of `root`; the walk is
///   iterative over an explicit heap frame stack, so it is total on any depth
///   (no derived recursion on term depth).
/// - provides: the shared engine behind the `Clone` impls of [`Value`] and
///   [`Computation`].
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2/L3 — the deep-clone witnesses build a ~1M-deep chain, clone
///   it in a small-stack thread, and assert the clone's iterative depth equals
///   the source's (a shallow or wrong clone would diverge on depth); the L3
///   residues are the per-former assembly arms.
/// - witness: `hardening::tests::deep_value_clone_is_total`
/// - witness: `hardening::tests::deep_computation_clone_is_total`
#[expect(
    clippy::too_many_lines,
    reason = "the flat iterative clone enumerates every value and computation former in one goal/frame loop; splitting it would obscure the single reduction machine"
)]
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of the borrowed source node; every binding is a shared reference by intent"
)]
fn clone_term(root: TermGoal<'_>) -> TermNode
{
    let mut frames: Vec<TermFrame<'_>> = Vec::new();
    let mut goal = root;
    'expand: loop {
        let mut produced = match goal {
            | TermGoal::Value(value) => match value {
                | Value::Variable(index) => TermNode::Value(Value::Variable(*index)),
                | Value::Constant(index) => TermNode::Value(Value::Constant(*index)),
                | Value::Unit => TermNode::Value(Value::Unit),
                | Value::Literal(literal) => TermNode::Value(Value::Literal(literal.clone())),
                | Value::Pair(first, second) => {
                    frames.push(TermFrame::PairFirst(second));
                    goal = TermGoal::Value(first);
                    continue 'expand;
                },
                | Value::Injection(side, body) => {
                    frames.push(TermFrame::Injection(*side));
                    goal = TermGoal::Value(body);
                    continue 'expand;
                },
                | Value::Thunk(body) => {
                    frames.push(TermFrame::Thunk);
                    goal = TermGoal::Comp(body);
                    continue 'expand;
                },
                | Value::Lift { target, body } => {
                    frames.push(TermFrame::Lift(target.clone()));
                    goal = TermGoal::Value(body);
                    continue 'expand;
                },
            },
            | TermGoal::Comp(computation) => match computation {
                | Computation::Lambda(body) => {
                    frames.push(TermFrame::Lambda);
                    goal = TermGoal::Comp(body);
                    continue 'expand;
                },
                | Computation::Application(head, argument) => {
                    frames.push(TermFrame::ApplicationHead(argument));
                    goal = TermGoal::Comp(head);
                    continue 'expand;
                },
                | Computation::Return(value) => {
                    frames.push(TermFrame::Return);
                    goal = TermGoal::Value(value);
                    continue 'expand;
                },
                | Computation::Force(value) => {
                    frames.push(TermFrame::Force);
                    goal = TermGoal::Value(value);
                    continue 'expand;
                },
                | Computation::Bind(bound, body) => {
                    frames.push(TermFrame::BindBound(body));
                    goal = TermGoal::Comp(bound);
                    continue 'expand;
                },
                | Computation::Case {
                    scrutinee,
                    on_left,
                    on_right,
                } => {
                    frames.push(TermFrame::CaseScrutinee { on_left, on_right });
                    goal = TermGoal::Value(scrutinee);
                    continue 'expand;
                },
            },
        };
        loop {
            let Some(frame) = frames.pop()
            else {
                return produced;
            };
            match frame {
                | TermFrame::PairFirst(second) => {
                    frames.push(TermFrame::PairSecond(expect_value(produced)));
                    goal = TermGoal::Value(second);
                    continue 'expand;
                },
                | TermFrame::PairSecond(first) => {
                    let second = expect_value(produced);
                    produced = TermNode::Value(Value::Pair(Box::new(first), Box::new(second)));
                },
                | TermFrame::Injection(side) => {
                    let body = expect_value(produced);
                    produced = TermNode::Value(Value::Injection(side, Box::new(body)));
                },
                | TermFrame::Thunk => {
                    let body = expect_comp(produced);
                    produced = TermNode::Value(Value::Thunk(Box::new(body)));
                },
                | TermFrame::Lift(target) => {
                    let body = expect_value(produced);
                    produced = TermNode::Value(Value::Lift {
                        target,
                        body: Box::new(body),
                    });
                },
                | TermFrame::Lambda => {
                    let body = expect_comp(produced);
                    produced = TermNode::Comp(Computation::Lambda(Box::new(body)));
                },
                | TermFrame::ApplicationHead(argument) => {
                    frames.push(TermFrame::ApplicationArgument(expect_comp(produced)));
                    goal = TermGoal::Value(argument);
                    continue 'expand;
                },
                | TermFrame::ApplicationArgument(head) => {
                    let argument = expect_value(produced);
                    produced = TermNode::Comp(Computation::Application(
                        Box::new(head),
                        Box::new(argument),
                    ));
                },
                | TermFrame::Return => {
                    let value = expect_value(produced);
                    produced = TermNode::Comp(Computation::Return(Box::new(value)));
                },
                | TermFrame::Force => {
                    let value = expect_value(produced);
                    produced = TermNode::Comp(Computation::Force(Box::new(value)));
                },
                | TermFrame::BindBound(body) => {
                    frames.push(TermFrame::BindBody(expect_comp(produced)));
                    goal = TermGoal::Comp(body);
                    continue 'expand;
                },
                | TermFrame::BindBody(bound) => {
                    let body = expect_comp(produced);
                    produced = TermNode::Comp(Computation::Bind(Box::new(bound), Box::new(body)));
                },
                | TermFrame::CaseScrutinee { on_left, on_right } => {
                    frames.push(TermFrame::CaseLeft {
                        scrutinee: expect_value(produced),
                        on_right,
                    });
                    goal = TermGoal::Comp(on_left);
                    continue 'expand;
                },
                | TermFrame::CaseLeft {
                    scrutinee,
                    on_right,
                } => {
                    frames.push(TermFrame::CaseRight {
                        scrutinee,
                        on_left: expect_comp(produced),
                    });
                    goal = TermGoal::Comp(on_right);
                    continue 'expand;
                },
                | TermFrame::CaseRight { scrutinee, on_left } => {
                    let on_right = expect_comp(produced);
                    produced = TermNode::Comp(Computation::Case {
                        scrutinee: Box::new(scrutinee),
                        on_left: Box::new(on_left),
                        on_right: Box::new(on_right),
                    });
                },
            }
        }
    }
}

impl Clone for Value
{
    #[inline]
    fn clone(&self) -> Self
    {
        expect_value(clone_term(TermGoal::Value(self)))
    }
}

impl Clone for Computation
{
    #[inline]
    fn clone(&self) -> Self
    {
        expect_comp(clone_term(TermGoal::Comp(self)))
    }
}

/// Extract a value node's immediate children into the worklists by
/// placeholder-swap, leaving `value` holding only leaves.
///
/// # Contract
/// - requires: nothing.
/// - ensures: each boxed child of `value` is moved onto the matching worklist
///   and replaced in place by a placeholder leaf, so `value`'s own subsequent
///   drop is `O(1)`.
/// - provides: one step of the iterative term deallocator.
/// - fails: never.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of the mutably-borrowed node to swap its child boxes in place; every binding is a mutable reference by intent"
)]
fn drain_value_children(
    value: &mut Value,
    values: &mut Vec<Value>,
    computations: &mut Vec<Computation>,
)
{
    match value {
        | Value::Variable(_) | Value::Constant(_) | Value::Unit | Value::Literal(_) => {},
        | Value::Pair(first, second) => {
            values.push(mem::replace(first.as_mut(), value_placeholder()));
            values.push(mem::replace(second.as_mut(), value_placeholder()));
        },
        | Value::Injection(_side, body) => {
            values.push(mem::replace(body.as_mut(), value_placeholder()));
        },
        | Value::Thunk(body) => {
            computations.push(mem::replace(body.as_mut(), comp_placeholder()));
        },
        | Value::Lift { body, .. } => {
            values.push(mem::replace(body.as_mut(), value_placeholder()));
        },
    }
}

/// Extract a computation node's immediate children into the worklists by
/// placeholder-swap, leaving `computation` holding only leaves.
///
/// # Contract
/// - requires: nothing.
/// - ensures: each boxed child of `computation` is moved onto the matching
///   worklist and replaced by a placeholder, so its own drop is `O(1)`.
/// - provides: one step of the iterative term deallocator.
/// - fails: never.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of the mutably-borrowed node to swap its child boxes in place; every binding is a mutable reference by intent"
)]
fn drain_comp_children(
    computation: &mut Computation,
    values: &mut Vec<Value>,
    computations: &mut Vec<Computation>,
)
{
    match computation {
        | Computation::Lambda(body) => {
            computations.push(mem::replace(body.as_mut(), comp_placeholder()));
        },
        | Computation::Application(head, argument) => {
            computations.push(mem::replace(head.as_mut(), comp_placeholder()));
            values.push(mem::replace(argument.as_mut(), value_placeholder()));
        },
        | Computation::Return(value) | Computation::Force(value) => {
            values.push(mem::replace(value.as_mut(), value_placeholder()));
        },
        | Computation::Bind(bound, body) => {
            computations.push(mem::replace(bound.as_mut(), comp_placeholder()));
            computations.push(mem::replace(body.as_mut(), comp_placeholder()));
        },
        | Computation::Case {
            scrutinee,
            on_left,
            on_right,
        } => {
            values.push(mem::replace(scrutinee.as_mut(), value_placeholder()));
            computations.push(mem::replace(on_left.as_mut(), comp_placeholder()));
            computations.push(mem::replace(on_right.as_mut(), comp_placeholder()));
        },
    }
}

/// Drain the term worklists to empty, extracting each popped node's children
/// before it falls.
///
/// # Contract
/// - requires: `values`/`computations` hold owned subtrees whose children are
///   still live (not yet placeholder-swapped).
/// - ensures: every node in both worklists — and transitively every descendant
///   — is dropped; each popped node's children are extracted before the node
///   falls, so its own re-entrant drop sees only placeholders and is `O(1)`.
///   The loop is iterative, so deallocation is total on any depth.
/// - provides: the shared engine behind the `Drop` impls of [`Value`] and
///   [`Computation`].
/// - fails: never.
/// - panics: none.
fn drain_terms(
    mut values: Vec<Value>,
    mut computations: Vec<Computation>,
)
{
    loop {
        if let Some(mut value) = values.pop() {
            drain_value_children(&mut value, &mut values, &mut computations);
            continue;
        }
        if let Some(mut computation) = computations.pop() {
            drain_comp_children(&mut computation, &mut values, &mut computations);
            continue;
        }
        break;
    }
}

impl Drop for Value
{
    /// Deallocate a value tree iteratively (gandr-i3i): extract this node's
    /// children into a heap worklist and drain it, so the compiler's recursive
    /// drop glue only ever sees placeholder leaves.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the deep-drop witnesses drop a ~1M-deep chain in a
    ///   small-stack thread where recursive glue would overflow; the indirect
    ///   witnesses cover a term reached through a [`crate::error::KernelError`]
    ///   payload and through a decoded declaration.
    /// - witness: `hardening::tests::deep_value_drop_is_total`
    /// - witness: `hardening::tests::deep_decoded_declaration_drop_is_total`
    #[inline]
    fn drop(&mut self)
    {
        let mut values: Vec<Self> = Vec::new();
        let mut computations: Vec<Computation> = Vec::new();
        drain_value_children(self, &mut values, &mut computations);
        drain_terms(values, computations);
    }
}

impl Drop for Computation
{
    /// Deallocate a computation tree iteratively (gandr-i3i), mirroring
    /// [`Value`]'s worklist deallocator.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the deep-drop witness drops a ~1M-deep computation
    ///   chain in a small-stack thread where recursive glue would overflow.
    /// - witness: `hardening::tests::deep_computation_drop_is_total`
    #[inline]
    fn drop(&mut self)
    {
        let mut values: Vec<Value> = Vec::new();
        let mut computations: Vec<Self> = Vec::new();
        drain_comp_children(self, &mut values, &mut computations);
        drain_terms(values, computations);
    }
}
