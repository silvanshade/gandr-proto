//! The S1 type language: [`ValueType`] (positive) and [`CompType`]
//! (negative), the closed vocabulary of value and computation types the
//! polarized CBPV core admits at S1.
//!
//! The vocabulary is **closed** (K1): no hole, no
//! metavariable, no mark, no effect-row constructor exists to be represented.
//! Every type former embeds only other types and (at [`ValueType::Universe`]
//! and [`ValueType::Lift`]) a `gandr_kernel_strata::Level` in canonical form —
//! **no type former is indexed by a value term** at S1. That fact is
//! load-bearing for [`crate::conv`]: type conversion never descends into a
//! term, so the C5 conversion-versus-effects quarantine holds vacuously (there
//! is nothing to evaluate).
//!
//! Excluded at S1, deliberately unrepresentable: effect rows (`F` is pure —
//! there is no effect-row constructor), the `Sigma` dependent pair (a value
//! variable cannot occur in a type at S1, so `Product` is the **non-dependent**
//! product and its dependent form waits for S2), description codes,
//! `Path`/identity types, and `List`/`Record`/`With`.
//!
//! # Arena representation (D1(C))
//!
//! A type node's children are **typed arena ids** ([`ValueTypeId`],
//! [`CompTypeId`]) into the owning [`TermArena`], not owned `Box`es; the
//! embedded `Level` at [`ValueType::Universe`]/[`ValueType::Lift`] stays
//! inline. Children are `Copy`, so the derived
//! `Clone`/`Drop`/`PartialEq`/`Eq`/`Hash` are **shallow** and the hand-written
//! iterative worklists the owned-tree representation required are
//! retired — arena teardown is a flat `Vec` drop. The derived-equality caveat
//! (child-id, not structural, equality) and its audit are in [`crate::term`].
//!
//! [`TermArena`]: crate::TermArena

use gandr_kernel_strata::Level;

use crate::arena::CompTypeId;
use crate::arena::ValueTypeId;
use crate::base::BaseType;

/// A value type: the positive fragment of the S1 type vocabulary.
///
/// The type of a value. Value types classify [`crate::term::Value`]s and are
/// themselves classified by universes (the checker's value-type formation).
///
/// Children are arena ids; the derived traits are shallow (the module docs).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ValueType
{
    /// A rigid base-type atom (`Integer`, `String`, `Numeric`).
    Base(BaseType),
    /// The unit type, with the single value [`crate::term::Value::Unit`].
    Unit,
    /// The non-dependent product `A × B` (the dependent `Sigma` is S2).
    Product(ValueTypeId, ValueTypeId),
    /// The sum `A + B`, with left and right injections.
    Sum(ValueTypeId, ValueTypeId),
    /// The thunk type `U C` of a computation type `C` (ungraded at S1 —
    /// grades are erased upstream and reserved only in the format plane).
    Thunk(CompTypeId),
    /// The universe former `U_l` at a canonical level `l`. Its own level is
    /// `l + 1`, so `U_l : U_m` iff `l < m` (the universe rule, decided
    /// through `gandr_kernel_strata::Level::lt`).
    Universe(Level),
    /// An explicit lift of a value type into a strictly higher universe
    /// (`no implicit cumulativity`): the inner
    /// type, relocated to `target`, valid when the inner type's level is
    /// strictly below `target`.
    Lift
    {
        /// The value type being lifted.
        inner: ValueTypeId,
        /// The target universe level (the lifted type's level).
        target: Level,
    },
}

/// A computation type: the negative fragment of the S1 type vocabulary.
///
/// The type of a computation. `F` is **pure** — S1 has no effect rows at all
/// so the returner carries only its result type.
///
/// Children are arena ids; the derived traits are shallow (the module docs).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CompType
{
    /// The returner `F A` of a value type `A`: the type of a computation that
    /// returns a value of type `A` (pure — no effect row at S1).
    Returner(ValueTypeId),
    /// The function type `A → C` from a value type to a computation type.
    /// Non-dependent at S1: no type former embeds a value term, so the
    /// codomain cannot mention the argument.
    Arrow
    {
        /// The value-type domain.
        domain: ValueTypeId,
        /// The computation-type codomain.
        codomain: CompTypeId,
    },
}
