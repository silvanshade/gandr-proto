//! The metacontext: which holes are metavariables, at what sort, and what the
//! solver has bound them to.
//!
//! # A metavariable is a hole the caller nominates
//!
//! The core syntax already carries holes ([`Value::Hole`], [`Comp::Hole`]), and
//! the solver adds no former of its own. A hole is a **metavariable** exactly
//! while this context declares it; every other hole stays what the checker
//! already makes it, a gradual wildcard the solver never binds.
//!
//! That choice is load-bearing rather than economical. A new former would reach
//! the checker, the typing machine, the marking pass, the kernel bridge, and
//! every conformance generator, and the step-for-step agreement between the
//! checker and the machine would have to be re-established rather than
//! inherited. Nominating a hole reaches none of them.
//!
//! # Metavariables are closed
//!
//! A metavariable stands for a **closed** term. Dependence on the surrounding
//! context travels through its spine instead: an elaborator that wants a
//! metavariable able to mention the locals `x` and `y` creates one and applies
//! it to them. Two properties follow, and the certificate rests on both:
//! substituting a solution into a caller's term can capture nothing, and a
//! solution means the same thing wherever it is substituted.
//!
//! # The sorts, and why one of them carries a grade
//!
//! [`MetaSort::Value`] and [`MetaSort::Comp`] are the two syntactic sorts a
//! hole occupies. [`MetaSort::Thunk`] is a value-sorted metavariable whose
//! solutions are thunks, and it carries the grade the creator minted it at.
//!
//! The grade is not decoration. Conversion compares two thunks by their grades
//! before their bodies, so a solver that invented a grade would emit solutions
//! the ordinary checker refutes — and inventing one is exactly the guess the
//! fragment discipline forbids. The creator always knows the grade, because it
//! wrote the type the metavariable stands at. A value-sorted metavariable met
//! under a `force` with no declared grade is a named postponement instead.
//!
//! [`Value::Hole`]: crate::syntax::Value::Hole
//! [`Comp::Hole`]: crate::syntax::Comp::Hole

use alloc::collections::BTreeMap;
use alloc::rc::Rc;

use crate::boundary::HoleId;
use crate::boundary::MetaStatus;
use crate::grade::Grade;
use crate::subst::HoleRepl;
use crate::subst::HoleSubstitution;
use crate::subst::subst_holes_comp;
use crate::subst::subst_holes_value;
use crate::syntax::Comp;
use crate::syntax::Value;

/// The sort a metavariable occupies, and the shape its solutions take.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MetaSort
{
    /// A value-sorted metavariable, solved by a closed value.
    Value,
    /// A value-sorted metavariable whose solutions are thunks at this grade —
    /// the shape a metavariable standing for a function takes in
    /// call-by-push-value, where it is reached through a `force`.
    Thunk(Grade),
    /// A computation-sorted metavariable, solved by a closed computation.
    Comp,
}

/// The metavariables of one unification problem, with their solutions.
///
/// # Contract
/// - requires: no hole occurring anywhere in the problem has an identity at or
///   above the watermark the context was built with, so a minted metavariable
///   collides with nothing the caller already wrote.
/// - ensures: [`Self::fresh`] returns an identity distinct from every earlier
///   one and from every declared metavariable; solving is monotone, so a bound
///   metavariable never changes its binding.
/// - provides: the metavariable vocabulary the solver machine works against.
/// - panics: none.
#[derive(Clone, Debug)]
pub struct MetaContext
{
    /// The declared and minted metavariables, with their sorts.
    sorts: BTreeMap<HoleId, MetaSort>,
    /// The solutions bound so far, possibly mentioning other metavariables.
    solutions: HoleSubstitution,
    /// The next identity [`Self::fresh`] will mint.
    watermark: HoleId,
}

impl MetaContext
{
    /// A metacontext that mints identities at or above `watermark`.
    ///
    /// # Contract
    /// - requires: no hole in any constraint the solver will be given has an
    ///   identity at or above `watermark`.
    /// - ensures: the context declares nothing and has solved nothing.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(watermark: HoleId) -> Self
    {
        Self {
            sorts: BTreeMap::new(),
            solutions: HoleSubstitution::new(),
            watermark,
        }
    }

    /// Declares the existing hole `meta` to be a metavariable of `sort`.
    ///
    /// # Contract
    /// - ensures: `meta` is a metavariable of `sort` from here on, and the mint
    ///   watermark is at least one past it, so a later [`Self::fresh`] cannot
    ///   collide with it.
    /// - panics: none.
    #[inline]
    pub fn declare(
        &mut self,
        meta: HoleId,
        sort: MetaSort,
    )
    {
        self.sorts.insert(meta, sort);
        let next = HoleId::from(u32::from(meta).saturating_add(1));
        if u32::from(next) > u32::from(self.watermark) {
            self.watermark = next;
        }
    }

    /// Mints a fresh metavariable of `sort`.
    ///
    /// Meta splitting is what needs this: solving a projected metavariable by
    /// the lazy-pair eta law replaces it with a pair of fresh ones.
    ///
    /// # Contract
    /// - ensures: the result differs from every identity this context has ever
    ///   declared or minted, and is a metavariable of `sort`.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — freshness against a declared identity and against a
    ///   previously minted one, separated by declaring an identity above the
    ///   constructor watermark and then minting twice.
    /// - witness: `unify::tests::a_minted_metavariable_avoids_declared_identities`
    #[inline]
    pub fn fresh(
        &mut self,
        sort: MetaSort,
    ) -> HoleId
    {
        let minted = self.watermark;
        self.watermark = HoleId::from(u32::from(minted).saturating_add(1));
        self.sorts.insert(minted, sort);
        minted
    }

    /// Whether `hole` is a metavariable of this context.
    #[inline]
    #[must_use]
    pub fn is_meta(
        &self,
        hole: HoleId,
    ) -> MetaStatus
    {
        MetaStatus::from(self.sorts.contains_key(&hole))
    }

    /// The sort of `meta`, or `None` when it is not a metavariable here.
    #[inline]
    #[must_use]
    pub fn sort(
        &self,
        meta: HoleId,
    ) -> Option<MetaSort>
    {
        self.sorts.get(&meta).copied()
    }

    /// The value-sorted solution of `meta`, when it has one.
    #[inline]
    #[must_use]
    pub(crate) fn value_solution(
        &self,
        meta: HoleId,
    ) -> Option<&Rc<Value>>
    {
        self.solutions.value(meta)
    }

    /// The computation-sorted solution of `meta`, when it has one.
    #[inline]
    #[must_use]
    pub(crate) fn comp_solution(
        &self,
        meta: HoleId,
    ) -> Option<&Rc<Comp>>
    {
        self.solutions.comp(meta)
    }

    /// Binds `meta` to `repl`.
    ///
    /// # Contract
    /// - requires: `repl` is closed, mentions `meta` nowhere, and sits at the
    ///   sort `meta` occupies — the three conditions the solver checks before
    ///   it calls this.
    /// - ensures: `meta` reads as solved from here on.
    /// - panics: none.
    #[inline]
    pub(crate) fn solve(
        &mut self,
        meta: HoleId,
        repl: HoleRepl,
    )
    {
        self.solutions.bind(meta, repl);
    }

    /// The solutions with every solution-inside-a-solution resolved.
    ///
    /// Meta splitting binds one metavariable to a term mentioning two fresh
    /// ones, and those may be solved afterwards, so the raw bindings are not
    /// closed under themselves. A certificate has to hand its consumer bindings
    /// that need no further work, because the consumer substitutes once and
    /// then asks the ordinary conversion engine.
    ///
    /// # Contract
    /// - ensures: no returned solution mentions a metavariable this context has
    ///   solved; the result binds exactly the same metavariables as the raw
    ///   solutions do.
    /// - provides: the substitution a certificate carries.
    /// - panics: none.
    /// - intension: the fixpoint runs at most one round per binding, plus one.
    ///   A round is one substitution pass over every binding, and the longest
    ///   chain a binding set can hold is as long as the set itself, which the
    ///   occurs check already makes acyclic. The bound is stated so the loop is
    ///   bounded by construction rather than by an invariant.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a two-step chain resolves in full, an independent
    ///   pair of bindings is untouched, and an unsolved metavariable inside a
    ///   solution survives, separated pointwise.
    /// - witness: `unify::tests::zonking_resolves_a_chained_solution`
    /// - witness: `unify::tests::zonking_leaves_an_unsolved_metavariable_alone`
    #[must_use]
    pub(crate) fn zonked(&self) -> HoleSubstitution
    {
        let mut current = self.solutions.clone();
        let mut rounds = u32::try_from(self.solutions.entries().count())
            .unwrap_or(u32::MAX)
            .saturating_add(1);
        while rounds > 0 {
            rounds = rounds.saturating_sub(1);
            let mut next = HoleSubstitution::new();
            let mut changed = false;
            for (meta, repl) in current.entries() {
                let rebuilt = match *repl {
                    | HoleRepl::Value(ref term) => {
                        let (substituted, _residual) = subst_holes_value(term.as_ref(), &current);
                        HoleRepl::Value(Rc::new(substituted))
                    },
                    | HoleRepl::Comp(ref term) => {
                        let (substituted, _residual) = subst_holes_comp(term.as_ref(), &current);
                        HoleRepl::Comp(Rc::new(substituted))
                    },
                };
                changed = changed || rebuilt != *repl;
                next.bind(meta, rebuilt);
            }
            current = next;
            if !changed {
                break;
            }
        }
        current
    }
}
