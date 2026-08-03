//! 2-cell **faces** — a rewrite `lhs ==> rhs` as a pair of open terms in the
//! free structure over a signature (proposal-levitation.md §4.1, V3; VDC
//! addendum §A).
//!
//! A [`RuleFace`] stores the two [`FreeTerm`]s untyped, with host-side
//! well-formedness ([`crate::check_desc`]) standing in for stage-1 typing;
//! nothing about the encoding changes when checking moves into the checker
//! (proposal §4.1). Each face carries derived [`RuleVarMeta`] per pattern
//! variable — the VDC delta — whose [`Variance`] is the constant
//! [`Variance::Producer`] at stage 0.

use crate::boundary::RuleVariableLinearity;
use crate::code::Name;
use crate::desc::SurfaceSpan;

/// A term in the **free monad** `D⋆(V)` over the description functor with
/// variables `V` (proposal §4.1): a variable, a constructor application, or a
/// (reserved) operation application.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FreeTerm
{
    /// A pattern **variable** `x`.
    Var(Name),
    /// A **constructor** application `C(t₀, …, tₙ)` (`n = 0` is a nullary
    /// constructor).
    Ctor(Name, Box<[Self]>),
    /// A reserved **operation** application `f(t₀, …, tₙ)` (an `op` member).
    Op(Name, Box<[Self]>),
}

impl FreeTerm
{
    /// A variable term.
    #[inline]
    #[must_use]
    pub fn var<N>(name: N) -> Self
    where
        N: Into<Name>,
    {
        Self::Var(name.into())
    }

    /// A constructor application.
    #[inline]
    #[must_use]
    pub fn ctor<N, A>(
        name: N,
        args: A,
    ) -> Self
    where
        N: Into<Name>,
        A: Into<Box<[Self]>>,
    {
        Self::Ctor(name.into(), args.into())
    }

    /// An operation application.
    #[inline]
    #[must_use]
    pub fn op<N, A>(
        name: N,
        args: A,
    ) -> Self
    where
        N: Into<Name>,
        A: Into<Box<[Self]>>,
    {
        Self::Op(name.into(), args.into())
    }

    /// Return this term's free variables (its [`FreeTerm::Var`] leaves), in
    /// left-to-right order with repeats.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: every variable leaf is recorded once per occurrence,
    ///   preserving left-to-right order (so linearity can be judged by
    ///   counting).
    /// - fails: never.
    #[inline]
    #[must_use]
    pub fn collect_vars(&self) -> Vec<Name>
    {
        let mut vars = Vec::new();
        let mut stack = vec![self];
        while let Some(term) = stack.pop() {
            match *term {
                | Self::Var(ref name) => vars.push(name.clone()),
                | Self::Ctor(_, ref args) | Self::Op(_, ref args) => {
                    for arg in args.iter().rev() {
                        stack.push(arg);
                    }
                },
            }
        }
        vars
    }
}

/// The **variance** role a cell pattern variable occupies (VDC addendum §A;
/// `proposal-vdc-reflection.md` §4.2).
///
/// At stage 0 the derived variance is the constant [`Variance::Producer`] —
/// consumer-argument operations, where a variable would first occupy a
/// [`Variance::Consumer`] (observation-side) position, do not exist yet. The
/// variant ships now so the sequent-era refinement (rules in `codata` blocks
/// departing from the constant) is an update, not a migration.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum Variance
{
    /// A producer-side (introduction) position — the stage-0 constant.
    #[default]
    Producer,
    /// A consumer-side (observation) position — reserved; first reached by
    /// rules in `codata` blocks once consumer-argument operations exist.
    Consumer,
}

/// Derived **metadata for one cell pattern variable** (VDC addendum §A): its
/// name, variance role, and linearity (whether it occurs exactly once across
/// the face's left-hand side).
///
/// The metadata is *derived* from the faces by [`crate::check_desc`]; any
/// surface attempt to *declare* it is declined (the declined-declaration
/// golden, addendum §A/§C).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuleVarMeta
{
    /// The pattern variable's name.
    pub var: Name,
    /// The variance role (the stage-0 constant [`Variance::Producer`]).
    pub variance: Variance,
    /// Whether the variable occurs exactly once in the left-hand side (linear).
    pub linear: RuleVariableLinearity,
}

impl RuleVarMeta
{
    /// Metadata for a variable with an explicit variance and linearity.
    #[inline]
    #[must_use]
    pub fn new<N>(
        var: N,
        variance: Variance,
        linear: RuleVariableLinearity,
    ) -> Self
    where
        N: Into<Name>,
    {
        Self {
            var: var.into(),
            variance,
            linear,
        }
    }
}

/// A **2-cell face** `lhs ==> rhs` — the surface-to-cell intermediary the
/// polygraph cell store elaborates through (proposal §4.1).
///
/// The two [`FreeTerm`]s are open terms in `D⋆(V)`; `vars` is the derived
/// [`RuleVarMeta`] per left-hand-side pattern variable (VDC delta);
/// `provenance` is the surface span the rule was read from.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuleFace
{
    /// The rewrite's left-hand side.
    pub lhs: FreeTerm,
    /// The rewrite's right-hand side.
    pub rhs: FreeTerm,
    /// Derived per-variable metadata (the VDC field; see [`RuleVarMeta`]).
    pub vars: Box<[RuleVarMeta]>,
    /// The surface span this face was read from.
    pub provenance: SurfaceSpan,
}

impl RuleFace
{
    /// A face over two terms with derived variable metadata and a provenance
    /// span.
    #[inline]
    #[must_use]
    pub fn new<V>(
        lhs: FreeTerm,
        rhs: FreeTerm,
        vars: V,
        provenance: SurfaceSpan,
    ) -> Self
    where
        V: Into<Box<[RuleVarMeta]>>,
    {
        Self {
            lhs,
            rhs,
            vars: vars.into(),
            provenance,
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn free_variables_are_collected_in_order_with_repeats()
    {
        // `id(Zero) ==> Zero` has no variables; `f(x, g(x)) ==> x` has `x` thrice.
        let term = FreeTerm::op("f", [
            FreeTerm::var("x"),
            FreeTerm::ctor("g", [FreeTerm::var("x")]),
        ]);
        let vars = term.collect_vars();
        assert_eq!(
            vars,
            vec!["x".into(), "x".into()],
            "each occurrence is counted"
        );

        let ground = FreeTerm::ctor("Zero", Vec::new());
        let none = ground.collect_vars();
        assert!(none.is_empty(), "a ground term has no variables");
    }

    #[test]
    fn variance_defaults_to_producer()
    {
        assert_eq!(
            Variance::Producer,
            Variance::default(),
            "the stage-0 constant variance is Producer"
        );
    }
}
