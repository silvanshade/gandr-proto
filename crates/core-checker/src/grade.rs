//! Usage grades for thunks (`type-system.md` §2).
//!
//! Grades track how many times a thunk may be forced. They carry the
//! preordered semiring structure `(R, +, ·, 0, 1, ⊑)` of §2: Stage 1's core
//! rules (§3.3) use only the order — `thunk_r t ⇓ U_s B` requires `s ⊑ r`,
//! `force v` requires `1 ⊑ r` — while `Dup`/`Drop` and grade constraints
//! (Stage 2) additionally use `+` and `·`.
//!
//! Carrier discipline (ADR-28): [`Grade`] is the single concrete carrier
//! across the whole crate — there is no semiring type parameter — and its
//! semiring signature ([`Grade::ZERO`], [`Grade::ONE`], [`Grade::OMEGA`],
//! [`Grade::fin`], [`Grade::leq`], [`Grade::plus`], [`Grade::times`]) is the
//! only interface any *other* module may use; do not construct or match the
//! representation outside this module. The representation is **sealed**
//! (ADR-28): [`Grade`] is a newtype over a crate-private [`Repr`], so this
//! rule is now compiler-enforced — no other module can name, construct, or
//! match a grade's internals. `Debug` is the carrier's *only* rendering
//! surface; it forwards to the sealed representation, so grades still print as
//! `Fin(n)`/`Omega` in diagnostics and UI badges, and there is deliberately
//! **no `Display`** impl, so badge formatting does not widen the sealed
//! surface (`core-ir-contract.md` §4). Swapping in a different semiring is an
//! edit to this module alone.

use crate::boundary::GradeBound;
use crate::boundary::GradeOrderDecision;

/// A usage grade drawn from the default instantiation `ℕ ∪ {ω}`.
///
/// A thunk graded `r` may be forced *at most* `r` times; `ω` is unrestricted.
///
/// The carrier is **representation-sealed** (ADR-28): it is a newtype over the
/// crate-private [`Repr`], so the only way any other module can obtain or
/// observe a grade is through the semiring signature ([`Grade::ZERO`] /
/// [`Grade::ONE`] / [`Grade::OMEGA`] / [`Grade::fin`] / [`Grade::leq`] /
/// [`Grade::plus`] / [`Grade::times`]) plus the forwarding [`Debug`] impl.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct Grade(Repr);

/// The sealed representation of a [`Grade`].
///
/// Crate-private *and* module-private: it is named only inside this module, so
/// the carrier's entire cross-module surface is [`Grade`]'s semiring signature.
/// Changing the semiring is an edit to this enum and the [`Grade`] methods
/// alone.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum Repr
{
    /// A finite usage bound: the thunk may be forced at most this many times.
    Fin(u64),
    /// Unrestricted usage (`ω`).
    Omega,
}

impl core::fmt::Debug for Grade
{
    /// Forwards to the sealed [`Repr`]'s `Debug`, so a grade renders as
    /// `Fin(n)` / `Omega` exactly as before the seal — diagnostics and UI
    /// badges keep that spelling without a `Display` impl widening the carrier
    /// surface (`core-ir-contract.md` §4).
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        core::fmt::Debug::fmt(&self.0, f)
    }
}

impl Grade
{
    /// The grade `0` (exactly unused); the unit of `+` and the annihilator
    /// of `·`.
    pub const ZERO: Self = Self(Repr::Fin(0));
    /// The grade `1` (may be forced once); the unit of the semiring product.
    pub const ONE: Self = Self(Repr::Fin(1));
    /// The unrestricted grade `ω`; the top of `⊑`.
    pub const OMEGA: Self = Self(Repr::Omega);

    /// Builds the finite grade `bound` (the thunk may be forced at most
    /// `bound` times).
    #[inline]
    #[must_use]
    pub fn fin(bound: GradeBound) -> Self
    {
        Self(Repr::Fin(u64::from(bound)))
    }

    /// The grade preorder `self ⊑ other`: `r ⊑ s` iff `r ≤ s` or `s = ω`.
    ///
    /// This is the usage-upper-bound order of `type-system.md` §2.
    ///
    /// # Contract
    /// - ensures: returns `true` iff `self ⊑ other` — numerically `self ≤
    ///   other`, or `other = ω` (`ω` is the top); reflexive and transitive,
    ///   with `ω` above every grade and below only itself.
    /// - panics: none.
    #[inline]
    #[must_use]
    #[cfg_attr(all(not(debug_assertions), panic = "unwind"), no_panic::no_panic)]
    pub fn leq(
        self,
        other: Self,
    ) -> GradeOrderDecision
    {
        match (self.0, other.0) {
            | (_, Repr::Omega) => true,
            | (Repr::Omega, Repr::Fin(_)) => false,
            | (Repr::Fin(lhs), Repr::Fin(rhs)) => lhs <= rhs,
        }
        .into()
    }

    /// The semiring sum `self + other`: alternative/accumulated demand
    /// (`type-system.md` §2).
    ///
    /// On the default carrier this is natural-number addition with `ω`
    /// absorbing. `u64` approximates `ℕ`: a finite sum that overflows is
    /// conservatively `ω` (the clamp is a semiring homomorphism, so the §2
    /// laws hold exactly, and an over-clamped *demand* can only reject more
    /// programs, never accept more).
    ///
    /// # Contract
    /// - ensures: returns the semiring sum — `ω` absorbs (`ω + r = r + ω = ω`),
    ///   otherwise `u64` addition; commutative and associative with unit `0`. A
    ///   finite sum that would overflow `u64` clamps to `ω` (via
    ///   `checked_add`); the clamp is a semiring homomorphism, so the §2 laws
    ///   hold exactly and an over-clamped demand only ever rejects more
    ///   programs.
    /// - panics: none; the overflow clamp replaces any wrap.
    #[inline]
    #[must_use]
    #[cfg_attr(all(not(debug_assertions), panic = "unwind"), no_panic::no_panic)]
    pub fn plus(
        self,
        other: Self,
    ) -> Self
    {
        match (self.0, other.0) {
            | (Repr::Omega, _) | (_, Repr::Omega) => Self::OMEGA,
            | (Repr::Fin(lhs), Repr::Fin(rhs)) => lhs
                .checked_add(rhs)
                .map_or(Self::OMEGA, |bound| Self::fin(GradeBound::from(bound))),
        }
    }

    /// The semiring product `self · other`: nested/sequential demand
    /// (`type-system.md` §2).
    ///
    /// `0` annihilates (in particular `0 · ω ≡ 0`); otherwise `ω` absorbs.
    /// Finite overflow clamps to `ω` exactly as in [`Self::plus`].
    ///
    /// # Contract
    /// - ensures: returns the semiring product — `0` annihilates (including `0
    ///   · ω = ω · 0 = 0`), otherwise `ω` absorbs, otherwise `u64`
    ///   multiplication; associative with unit `1` and distributing over `+`. A
    ///   finite product that would overflow `u64` clamps to `ω` (via
    ///   `checked_mul`), exactly as in `plus`.
    /// - panics: none; the overflow clamp replaces any wrap.
    #[inline]
    #[must_use]
    #[cfg_attr(all(not(debug_assertions), panic = "unwind"), no_panic::no_panic)]
    pub fn times(
        self,
        other: Self,
    ) -> Self
    {
        match (self.0, other.0) {
            | (Repr::Fin(0), _) | (_, Repr::Fin(0)) => Self::ZERO,
            | (Repr::Omega, _) | (_, Repr::Omega) => Self::OMEGA,
            | (Repr::Fin(lhs), Repr::Fin(rhs)) => lhs
                .checked_mul(rhs)
                .map_or(Self::OMEGA, |bound| Self::fin(GradeBound::from(bound))),
        }
    }
}

/// Semiring-law tests for the default carrier, exhaustive over a pool that
/// exercises both units, interior values, the overflow clamp, and `ω`.
#[cfg(test)]
mod tests
{
    use super::*;

    /// `+` is commutative and associative with unit `0`.
    #[test]
    fn plus_is_a_commutative_monoid()
    {
        for r in pool() {
            assert_eq!(Grade::ZERO.plus(r), r, "0 + r must be r (r = {r:?})");
            assert_eq!(r.plus(Grade::ZERO), r, "r + 0 must be r (r = {r:?})");
            for s in pool() {
                assert_eq!(r.plus(s), s.plus(r), "+ must be commutative ({r:?}, {s:?})");
                for t in pool() {
                    assert_eq!(
                        r.plus(s).plus(t),
                        r.plus(s.plus(t)),
                        "+ must be associative ({r:?}, {s:?}, {t:?})"
                    );
                }
            }
        }
    }

    /// `·` is associative with unit `1`, annihilated by `0` (including
    /// `0 · ω = ω · 0 = 0`), and distributes over `+`.
    #[test]
    fn times_is_a_monoid_that_distributes_over_plus()
    {
        assert_eq!(
            Grade::ZERO,
            Grade::ZERO.times(Grade::OMEGA),
            "0 · ω must be 0"
        );
        assert_eq!(
            Grade::ZERO,
            Grade::OMEGA.times(Grade::ZERO),
            "ω · 0 must be 0"
        );
        for r in pool() {
            assert_eq!(Grade::ONE.times(r), r, "1 · r must be r (r = {r:?})");
            assert_eq!(r.times(Grade::ONE), r, "r · 1 must be r (r = {r:?})");
            assert_eq!(
                Grade::ZERO,
                Grade::ZERO.times(r),
                "0 · r must be 0 (r = {r:?})"
            );
            assert_eq!(
                Grade::ZERO,
                r.times(Grade::ZERO),
                "r · 0 must be 0 (r = {r:?})"
            );
            for s in pool() {
                for t in pool() {
                    assert_eq!(
                        r.times(s).times(t),
                        r.times(s.times(t)),
                        "· must be associative ({r:?}, {s:?}, {t:?})"
                    );
                    assert_eq!(
                        r.times(s.plus(t)),
                        r.times(s).plus(r.times(t)),
                        "· must distribute over + on the left ({r:?}, {s:?}, {t:?})"
                    );
                    assert_eq!(
                        s.plus(t).times(r),
                        s.times(r).plus(t.times(r)),
                        "· must distribute over + on the right ({r:?}, {s:?}, {t:?})"
                    );
                }
            }
        }
    }

    /// `⊑` is reflexive and transitive with top `ω`, and `+`/`·` are
    /// monotone in both arguments (`type-system.md` §2).
    #[test]
    fn order_is_a_preorder_and_the_operations_are_monotone()
    {
        for r in pool() {
            assert!(bool::from(r.leq(r)), "⊑ must be reflexive (r = {r:?})");
            assert!(
                bool::from(r.leq(Grade::OMEGA)),
                "ω must be the top of ⊑ (r = {r:?})"
            );
            for s in pool() {
                for t in pool() {
                    if bool::from(r.leq(s)) && bool::from(s.leq(t)) {
                        assert!(
                            bool::from(r.leq(t)),
                            "⊑ must be transitive ({r:?}, {s:?}, {t:?})"
                        );
                    }
                    if bool::from(r.leq(s)) {
                        assert!(
                            bool::from(r.plus(t).leq(s.plus(t))),
                            "+ must be monotone ({r:?}, {s:?}, {t:?})"
                        );
                        assert!(
                            bool::from(r.times(t).leq(s.times(t))),
                            "· must be monotone ({r:?}, {s:?}, {t:?})"
                        );
                    }
                }
            }
        }
    }

    /// The overflow clamp goes to `ω` (never wraps, never panics).
    #[test]
    fn finite_overflow_clamps_to_omega()
    {
        assert_eq!(
            Grade::OMEGA,
            Grade::fin(GradeBound::from(u64::MAX)).plus(Grade::ONE),
            "a + overflowing u64 must clamp to ω"
        );
        assert_eq!(
            Grade::OMEGA,
            Grade::fin(GradeBound::from(u64::MAX)).times(Grade::fin(GradeBound::from(2))),
            "a · overflowing u64 must clamp to ω"
        );
        assert_eq!(
            Grade::fin(GradeBound::from(u64::MAX)),
            Grade::fin(GradeBound::from(u64::MAX)).plus(Grade::ZERO),
            "a sum at the boundary without overflow must stay finite"
        );
    }
    /// The test pool: units, small interior values, the clamp boundary, `ω`.
    fn pool() -> [Grade; 7]
    {
        [
            Grade::ZERO,
            Grade::ONE,
            Grade::fin(GradeBound::from(2)),
            Grade::fin(GradeBound::from(3)),
            Grade::fin(GradeBound::from(u64::MAX - 1)),
            Grade::fin(GradeBound::from(u64::MAX)),
            Grade::OMEGA,
        ]
    }
}
