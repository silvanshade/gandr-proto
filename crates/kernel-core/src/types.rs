//! The S1 type language: [`ValueType`] (positive) and [`CompType`]
//! (negative), the closed vocabulary of value and computation types the
//! polarized CBPV core admits at S1.
//!
//! The vocabulary is **closed** (kernel-boundary.md K1): no hole, no
//! metavariable, no mark, no effect-row constructor exists to be represented.
//! Every type former embeds only other types and (at [`ValueType::Universe`]
//! and [`ValueType::Lift`]) a `gandr_kernel_strata::Level` in canonical form —
//! **no type former is indexed by a value term** at S1. That fact is
//! load-bearing for [`crate::conv`]: type conversion never descends into a
//! term, so the C5 conversion-versus-effects quarantine holds vacuously (there
//! is nothing to evaluate).
//!
//! Excluded at S1, deliberately unrepresentable (coordinator staging call,
//! gandr-wvd.2; kernel-boundary.md §7): effect rows (`F` is pure — there is no
//! effect-row constructor), the `Sigma` dependent pair (a value variable
//! cannot occur in a type at S1, so `Product` is the **non-dependent** product
//! and its dependent form waits for S2), description codes, `Path`/identity
//! types, and `List`/`Record`/`With`.
//!
//! # Totality of destruction and duplication
//!
//! [`ValueType`] and [`CompType`] are mutually recursive owned trees whose
//! depth scales with untrusted input (export decode can build an arbitrarily
//! deep type from adversarial bytes, gandr-i3i), and the checker synthesizes
//! deep types that ride inside a [`crate::error::KernelError`]. The derived
//! `Drop` glue and a derived `Clone` would both recurse on that depth and
//! overflow the stack, so both are **iterative** over an explicit heap worklist
//! (mirroring [`crate::term`]): `Clone` is a hand-written two-stack
//! goal/produced walk and `Drop` extracts each node's children by
//! placeholder-swap. `PartialEq`/`Eq`/ `Hash` stay **derived** (recursive)
//! because the production comparator is [`crate::conv`] (iterative); the
//! derived relations are test-only — an iterative rewrite is a tracked
//! residuals candidate, not a live hazard.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::mem;

use gandr_kernel_strata::Level;

use crate::base::BaseType;

/// A value type: the positive fragment of the S1 type vocabulary.
///
/// The type of a value. Value types classify [`crate::term::Value`]s and are
/// themselves classified by universes (the checker's value-type formation).
///
/// `Clone` and `Drop` are hand-written to stay total on adversarial depth (the
/// module docs); `PartialEq`/`Eq`/`Hash` stay derived and recursive by design.
#[derive(Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the kernel's S1 value-type vocabulary is closed by design (kernel-boundary.md K1)"
)]
pub enum ValueType
{
    /// A rigid base-type atom (`Integer`, `String`, `Numeric`).
    Base(BaseType),
    /// The unit type, with the single value [`crate::term::Value::Unit`].
    Unit,
    /// The non-dependent product `A × B` (the dependent `Sigma` is S2).
    Product(Box<Self>, Box<Self>),
    /// The sum `A + B`, with left and right injections.
    Sum(Box<Self>, Box<Self>),
    /// The thunk type `U C` of a computation type `C` (ungraded at S1 —
    /// grades are erased upstream and reserved only in the format plane).
    Thunk(Box<CompType>),
    /// The universe former `U_l` at a canonical level `l`. Its own level is
    /// `l + 1`, so `U_l : U_m` iff `l < m` (the ADR-78 universe rule, decided
    /// through `gandr_kernel_strata::Level::lt`).
    Universe(Level),
    /// An explicit lift of a value type into a strictly higher universe
    /// (`no implicit cumulativity`, kernel-boundary.md §4 / §7): the inner
    /// type, relocated to `target`, valid when the inner type's level is
    /// strictly below `target`.
    Lift
    {
        /// The value type being lifted.
        inner: Box<Self>,
        /// The target universe level (the lifted type's level).
        target: Level,
    },
}

/// A computation type: the negative fragment of the S1 type vocabulary.
///
/// The type of a computation. `F` is **pure** — S1 has no effect rows at all
/// (kernel-boundary.md §7), so the returner carries only its result type.
///
/// `Clone` and `Drop` are hand-written to stay total on adversarial depth (the
/// module docs); `PartialEq`/`Eq`/`Hash` stay derived and recursive by design.
#[derive(Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the kernel's S1 computation-type vocabulary is closed by design (kernel-boundary.md K1)"
)]
pub enum CompType
{
    /// The returner `F A` of a value type `A`: the type of a computation that
    /// returns a value of type `A` (pure — no effect row at S1).
    Returner(Box<ValueType>),
    /// The function type `A → C` from a value type to a computation type.
    /// Non-dependent at S1: no type former embeds a value term, so the
    /// codomain cannot mention the argument.
    Arrow
    {
        /// The value-type domain.
        domain: Box<ValueType>,
        /// The computation-type codomain.
        codomain: Box<Self>,
    },
}

/// The allocation-free placeholder a worklist swap leaves in a drained value-
/// type slot (the unit type is a leaf, so its own drop is `O(1)`).
#[inline]
const fn value_type_placeholder() -> ValueType
{
    ValueType::Unit
}

/// The placeholder a worklist swap leaves in a drained computation-type slot: a
/// returner of unit. Computation types have no leaf variant, so this costs one
/// transient allocation, reclaimed when the draining node falls.
#[inline]
fn comp_type_placeholder() -> CompType
{
    CompType::Returner(Box::new(value_type_placeholder()))
}

/// A cloned type node, tagged by polarity — the produced register of the
/// iterative clone walk.
enum TypeNode
{
    /// A cloned value type.
    Value(ValueType),
    /// A cloned computation type.
    Comp(CompType),
}

/// A borrowed source type still to be cloned, tagged by polarity — the goal of
/// the iterative clone walk.
enum TypeGoal<'src>
{
    /// A value type to clone.
    Value(&'src ValueType),
    /// A computation type to clone.
    Comp(&'src CompType),
}

/// A pending clone frame: a parent whose already-cloned children are held and
/// whose remaining source children are borrowed (mirrors [`crate::term`]'s
/// term-clone frames).
enum TypeFrame<'src>
{
    /// A product: first child cloning; the second child's source is held.
    ProductFirst(&'src ValueType),
    /// A product: first child cloned; the second is cloning.
    ProductSecond(ValueType),
    /// A sum: first child cloning; the second child's source is held.
    SumFirst(&'src ValueType),
    /// A sum: first child cloned; the second is cloning.
    SumSecond(ValueType),
    /// A thunk: the body computation type is cloning.
    Thunk,
    /// A lift: the target level is held; the inner value type is cloning.
    Lift(Level),
    /// A returner: the result value type is cloning.
    Returner,
    /// An arrow: the domain is cloning; the codomain's source is held.
    ArrowDomain(&'src CompType),
    /// An arrow: the domain is cloned; the codomain is cloning.
    ArrowCodomain(ValueType),
}

/// Project the value type out of a produced type node.
///
/// # Contract
/// - requires: `node` was produced for a value-type slot (polarity matches by
///   construction, as the walk expands a value-type source into a value node).
/// - ensures: the produced value type.
/// - provides: the total polarity projection the value-type frames need.
/// - fails: never — a computation node is unreachable; the fallback is the unit
///   type, and a mis-clone is caught by the deep-clone depth witnesses.
/// - panics: none.
#[inline]
fn expect_value_type(node: TypeNode) -> ValueType
{
    match node {
        | TypeNode::Value(value_type) => value_type,
        | TypeNode::Comp(_) => value_type_placeholder(),
    }
}

/// Project the computation type out of a produced type node.
///
/// # Contract
/// - requires: `node` was produced for a computation-type slot (polarity
///   matches by construction, as in [`expect_value_type`]).
/// - ensures: the produced computation type.
/// - provides: the total polarity projection the computation-type frames need.
/// - fails: never — a value node is unreachable; the fallback is the unit
///   returner, and a mis-clone is caught by the deep-clone depth witnesses.
/// - panics: none.
#[inline]
fn expect_comp_type(node: TypeNode) -> CompType
{
    match node {
        | TypeNode::Comp(comp_type) => comp_type,
        | TypeNode::Value(_) => comp_type_placeholder(),
    }
}

/// Deep-clone a type iteratively over a goal register and a frame stack.
///
/// # Contract
/// - requires: nothing.
/// - ensures: a structurally identical owned copy of `root`; the walk is
///   iterative over an explicit heap frame stack, so it is total on any depth.
/// - provides: the shared engine behind the `Clone` impls of [`ValueType`] and
///   [`CompType`].
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2/L3 — the deep-clone witnesses build a ~1M-deep type, clone
///   it in a small-stack thread, and assert the clone's iterative depth equals
///   the source's; the L3 residues are the per-former assembly arms.
/// - witness: `hardening::tests::deep_value_type_clone_is_total`
/// - witness: `hardening::tests::deep_comp_type_clone_is_total`
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of the borrowed source node; every binding is a shared reference by intent"
)]
fn clone_type(root: TypeGoal<'_>) -> TypeNode
{
    let mut frames: Vec<TypeFrame<'_>> = Vec::new();
    let mut goal = root;
    'expand: loop {
        let mut produced = match goal {
            | TypeGoal::Value(value_type) => match value_type {
                | ValueType::Base(base) => TypeNode::Value(ValueType::Base(*base)),
                | ValueType::Unit => TypeNode::Value(ValueType::Unit),
                | ValueType::Universe(level) => TypeNode::Value(ValueType::Universe(level.clone())),
                | ValueType::Product(first, second) => {
                    frames.push(TypeFrame::ProductFirst(second));
                    goal = TypeGoal::Value(first);
                    continue 'expand;
                },
                | ValueType::Sum(first, second) => {
                    frames.push(TypeFrame::SumFirst(second));
                    goal = TypeGoal::Value(first);
                    continue 'expand;
                },
                | ValueType::Thunk(body) => {
                    frames.push(TypeFrame::Thunk);
                    goal = TypeGoal::Comp(body);
                    continue 'expand;
                },
                | ValueType::Lift { inner, target } => {
                    frames.push(TypeFrame::Lift(target.clone()));
                    goal = TypeGoal::Value(inner);
                    continue 'expand;
                },
            },
            | TypeGoal::Comp(comp_type) => match comp_type {
                | CompType::Returner(result) => {
                    frames.push(TypeFrame::Returner);
                    goal = TypeGoal::Value(result);
                    continue 'expand;
                },
                | CompType::Arrow { domain, codomain } => {
                    frames.push(TypeFrame::ArrowDomain(codomain));
                    goal = TypeGoal::Value(domain);
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
                | TypeFrame::ProductFirst(second) => {
                    frames.push(TypeFrame::ProductSecond(expect_value_type(produced)));
                    goal = TypeGoal::Value(second);
                    continue 'expand;
                },
                | TypeFrame::ProductSecond(first) => {
                    let second = expect_value_type(produced);
                    produced =
                        TypeNode::Value(ValueType::Product(Box::new(first), Box::new(second)));
                },
                | TypeFrame::SumFirst(second) => {
                    frames.push(TypeFrame::SumSecond(expect_value_type(produced)));
                    goal = TypeGoal::Value(second);
                    continue 'expand;
                },
                | TypeFrame::SumSecond(first) => {
                    let second = expect_value_type(produced);
                    produced = TypeNode::Value(ValueType::Sum(Box::new(first), Box::new(second)));
                },
                | TypeFrame::Thunk => {
                    let body = expect_comp_type(produced);
                    produced = TypeNode::Value(ValueType::Thunk(Box::new(body)));
                },
                | TypeFrame::Lift(target) => {
                    let inner = expect_value_type(produced);
                    produced = TypeNode::Value(ValueType::Lift {
                        inner: Box::new(inner),
                        target,
                    });
                },
                | TypeFrame::Returner => {
                    let result = expect_value_type(produced);
                    produced = TypeNode::Comp(CompType::Returner(Box::new(result)));
                },
                | TypeFrame::ArrowDomain(codomain) => {
                    frames.push(TypeFrame::ArrowCodomain(expect_value_type(produced)));
                    goal = TypeGoal::Comp(codomain);
                    continue 'expand;
                },
                | TypeFrame::ArrowCodomain(domain) => {
                    let codomain = expect_comp_type(produced);
                    produced = TypeNode::Comp(CompType::Arrow {
                        domain: Box::new(domain),
                        codomain: Box::new(codomain),
                    });
                },
            }
        }
    }
}

impl Clone for ValueType
{
    #[inline]
    fn clone(&self) -> Self
    {
        expect_value_type(clone_type(TypeGoal::Value(self)))
    }
}

impl Clone for CompType
{
    #[inline]
    fn clone(&self) -> Self
    {
        expect_comp_type(clone_type(TypeGoal::Comp(self)))
    }
}

/// Extract a value type's immediate children into the worklists by placeholder-
/// swap, leaving `value_type` holding only leaves.
///
/// # Contract
/// - requires: nothing.
/// - ensures: each boxed child of `value_type` is moved onto the matching
///   worklist and replaced by a placeholder leaf, so its own drop is `O(1)`.
/// - provides: one step of the iterative type deallocator.
/// - fails: never.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of the mutably-borrowed node to swap its child boxes in place; every binding is a mutable reference by intent"
)]
fn drain_value_type_children(
    value_type: &mut ValueType,
    value_types: &mut Vec<ValueType>,
    comp_types: &mut Vec<CompType>,
)
{
    match value_type {
        | ValueType::Base(_) | ValueType::Unit | ValueType::Universe(_) => {},
        | ValueType::Product(first, second) | ValueType::Sum(first, second) => {
            value_types.push(mem::replace(first.as_mut(), value_type_placeholder()));
            value_types.push(mem::replace(second.as_mut(), value_type_placeholder()));
        },
        | ValueType::Thunk(body) => {
            comp_types.push(mem::replace(body.as_mut(), comp_type_placeholder()));
        },
        | ValueType::Lift { inner, .. } => {
            value_types.push(mem::replace(inner.as_mut(), value_type_placeholder()));
        },
    }
}

/// Extract a computation type's immediate children into the worklists by
/// placeholder-swap, leaving `comp_type` holding only leaves.
///
/// # Contract
/// - requires: nothing.
/// - ensures: each boxed child of `comp_type` is moved onto the matching
///   worklist and replaced by a placeholder, so its own drop is `O(1)`.
/// - provides: one step of the iterative type deallocator.
/// - fails: never.
/// - panics: none.
#[expect(
    clippy::pattern_type_mismatch,
    reason = "ergonomic matching of the mutably-borrowed node to swap its child boxes in place; every binding is a mutable reference by intent"
)]
fn drain_comp_type_children(
    comp_type: &mut CompType,
    value_types: &mut Vec<ValueType>,
    comp_types: &mut Vec<CompType>,
)
{
    match comp_type {
        | CompType::Returner(result) => {
            value_types.push(mem::replace(result.as_mut(), value_type_placeholder()));
        },
        | CompType::Arrow { domain, codomain } => {
            value_types.push(mem::replace(domain.as_mut(), value_type_placeholder()));
            comp_types.push(mem::replace(codomain.as_mut(), comp_type_placeholder()));
        },
    }
}

/// Drain the type worklists to empty, extracting each popped node's children
/// before it falls.
///
/// # Contract
/// - requires: `value_types`/`comp_types` hold owned subtrees whose children
///   are still live.
/// - ensures: every node in both worklists — and transitively every descendant
///   — is dropped; each popped node's children are extracted before it falls,
///   so its own re-entrant drop is `O(1)`. The loop is iterative, so
///   deallocation is total on any depth.
/// - provides: the shared engine behind the `Drop` impls of [`ValueType`] and
///   [`CompType`].
/// - fails: never.
/// - panics: none.
fn drain_types(
    mut value_types: Vec<ValueType>,
    mut comp_types: Vec<CompType>,
)
{
    loop {
        if let Some(mut value_type) = value_types.pop() {
            drain_value_type_children(&mut value_type, &mut value_types, &mut comp_types);
            continue;
        }
        if let Some(mut comp_type) = comp_types.pop() {
            drain_comp_type_children(&mut comp_type, &mut value_types, &mut comp_types);
            continue;
        }
        break;
    }
}

impl Drop for ValueType
{
    /// Deallocate a value-type tree iteratively (gandr-i3i): extract this
    /// node's children into a heap worklist and drain it, so recursive drop
    /// glue only ever sees placeholder leaves.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the deep-drop witness drops a ~1M-deep type chain in
    ///   a small-stack thread where recursive glue would overflow; the indirect
    ///   witness covers a deep type reached through a
    ///   [`crate::error::KernelError`] payload.
    /// - witness: `hardening::tests::deep_value_type_drop_is_total`
    /// - witness: `hardening::tests::deep_error_payload_drop_is_total`
    #[inline]
    fn drop(&mut self)
    {
        let mut value_types: Vec<Self> = Vec::new();
        let mut comp_types: Vec<CompType> = Vec::new();
        drain_value_type_children(self, &mut value_types, &mut comp_types);
        drain_types(value_types, comp_types);
    }
}

impl Drop for CompType
{
    /// Deallocate a computation-type tree iteratively (gandr-i3i), mirroring
    /// [`ValueType`]'s worklist deallocator.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the deep-drop witness drops a ~1M-deep
    ///   computation-type chain in a small-stack thread where recursive glue
    ///   would overflow.
    /// - witness: `hardening::tests::deep_comp_type_drop_is_total`
    #[inline]
    fn drop(&mut self)
    {
        let mut value_types: Vec<ValueType> = Vec::new();
        let mut comp_types: Vec<Self> = Vec::new();
        drain_comp_type_children(self, &mut value_types, &mut comp_types);
        drain_types(value_types, comp_types);
    }
}
