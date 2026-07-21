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
//! # Arena representation (D1(C), gandr-5t3)
//!
//! A node's children are **typed arena ids** ([`ValueId`], [`ComputationId`]),
//! not owned `Box`es: the node lives in the environment's [`TermArena`] and
//! names its children by id. Leaf payloads ([`Literal`], [`DeBruijnIndex`],
//! [`ConstantIndex`], [`Side`]) stay inline. Because children are `Copy` ids,
//! the derived `Clone`/`Drop`/`PartialEq`/`Eq`/`Hash` are **shallow** — no
//! recursion on term depth, so the hand-written iterative `Drop`/`Clone`
//! worklists the owned-tree representation required (gandr-i3i) are retired:
//! arena teardown is a flat `Vec` drop. See [`crate::arena`] for the id
//! discipline and the totality argument.
//!
//! ## The derived-equality caveat (audited, gandr-5t3)
//!
//! Derived `PartialEq`/`Eq`/`Hash` on a node compare its **child ids**, which
//! is **not** structural equality across arbitrarily-shared arenas: the kernel
//! *preserves* sharing but never *creates* it, so two structurally-equal
//! subterms need not share an id. Every use site is audited (STATUS.md): the
//! conversion walk ([`crate::conv`]) compares only inline leaf payloads by
//! derived equality (leaves have no id children); the export writer keys dedup
//! on explicit content keys, never derived node equality; any deep structural
//! comparison is an explicit id-resolving walk.
//!
//! [`TermArena`]: crate::TermArena

use gandr_kernel_strata::Level;

use crate::arena::ComputationId;
use crate::arena::ValueId;
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
/// Children are [`ValueId`]/[`ComputationId`] into the owning [`TermArena`];
/// the derived traits are shallow (the module docs), so no manual
/// `Clone`/`Drop` is needed.
///
/// [`TermArena`]: crate::TermArena
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
    Pair(ValueId, ValueId),
    /// A sum injection, introducing `A + B` on the given side.
    Injection(Side, ValueId),
    /// A thunk, suspending a computation into the value type `U C`.
    Thunk(ComputationId),
    /// An explicit universe lift (`no implicit cumulativity`,
    /// kernel-boundary.md §7): given `body : A` with `A`'s level strictly below
    /// `target`, this value has type `Lift A target`. The lift is written, not
    /// inferred — a bare `body : A` never inhabits `Lift A target` on its own.
    Lift
    {
        /// The target universe level of the lift.
        target: Level,
        /// The value being lifted.
        body: ValueId,
    },
}

/// A computation: the negative fragment of the S1 term vocabulary.
///
/// Computations are the fragment the kernel **types but never evaluates**
/// during conversion (C5). Their eliminators (application, force, bind, case)
/// synthesize; their introductions (lambda, return) check against an expected
/// computation type.
///
/// Children are arena ids; the derived traits are shallow (the module docs).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the kernel's S1 computation vocabulary is closed by design (kernel-boundary.md K1)"
)]
pub enum Computation
{
    /// A lambda `λ. M`, binding one value variable; introduces `A → C`.
    Lambda(ComputationId),
    /// An application `M v` of a computation to a value argument.
    Application(ComputationId, ValueId),
    /// A returner `return v`, introducing `F A`.
    Return(ValueId),
    /// A sequencing bind `x ← M; N`, binding the value `M` returns into `N`.
    Bind(ComputationId, ComputationId),
    /// A force `force v` of a thunk value `v : U C`, running it as `C`.
    Force(ValueId),
    /// A sum elimination `case v { inl ⇒ M | inr ⇒ N }`, binding the injected
    /// value into each branch.
    Case
    {
        /// The scrutinee value (of a sum type).
        scrutinee: ValueId,
        /// The left branch, checked with the left summand bound.
        on_left: ComputationId,
        /// The right branch, checked with the right summand bound.
        on_right: ComputationId,
    },
}
