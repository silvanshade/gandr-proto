//! The two-zone typing context `Γ; Σ` (`type-system.md` §"Notation and judgment
//! forms"; the effects and control record's one-shot-linearity
//! section; ADR-33 D5).
//!
//! The intuitionistic zone `Γ : x ↦ A` is kept as a binding *stack*: binding
//! pushes, unbinding pops, and lookup scans from the most recent binding, so
//! shadowing behaves correctly. Both the recursive checker and the typing
//! machine drive the same operations in the same order, which is part of the
//! step-for-step agreement contract.
//!
//! The linear zone `Σ` ([`Sigma`]) carries the §2.4 one-shot/linear
//! obligations a captured stack owns. It is the **frozen `Ctx` shape**
//! committed now (ADR-33 D5 — an expensive retrofit later) but **vacuous in
//! v0**: every `Σ`-obligation source (session endpoints, held capabilities,
//! acquired channels) is a deferred `+feature` (contract §9), so no v0 typing
//! rule binds or consumes a linear obligation and `Σ` stays empty through every
//! run (a conformance meta-invariant pins this). The linear discipline itself
//! is real and **directly unit-tested** (see the `tests` module) so it is not
//! "green because vacuous"; when the first `+sessions` / `+sharing` / `+worlds`
//! obligation-source lands, the discipline is already in force.
//!
//! Error-path note: on a *successful* run both implementations return `Γ` to
//! its initial value (every bind is matched by an unbind). On the *error* path
//! they diverge — the recursive checker unwinds `Γ` along the host call stack,
//! while the machine leaves `Γ` at the failure point and surfaces it through
//! `gandr_core_machine::FailureState` (the machine module's error-path
//! `Γ` contract). The conformance suite therefore compares
//! [`crate::error::TypeError`] values, not `Γ`.

use crate::boundary::ContextEmptyStatus;
use crate::boundary::ContextLength;
use crate::boundary::NameRef;
use crate::defs::Definitions;
use crate::types::ValueType;

/// The linear context zone `Σ` (the effects and control record's one-shot
/// linearity section; ADR-33 D5):
/// the one-shot/linear obligations a captured stack owns.
///
/// `Σ` is **structurally distinct** from the intuitionistic `Γ`: its hypotheses
/// admit **no weakening** (a bound obligation must be consumed before its scope
/// closes — a leftover live obligation is a leak) and **no contraction** (an
/// obligation is consumed *exactly once* — [`Self::consume`] is single-shot).
/// It is independent of the `U_r` grade carrier — grades are a coeffect on `U`,
/// `Σ` is resource linearity (ADR-33 D5; contract §4).
///
/// In v0 the zone is **vacuous**: no typing rule populates it (its obligation
/// sources are deferred `+feature`s), so a reified stack captures no
/// obligations and `resume` / `discard` / duplication are unrestricted. The
/// operations below are the committed substrate the §2.4 discipline will ride;
/// their laws are pinned by direct unit tests now (so the discipline is not
/// merely vacuously satisfied), and the runtime unwind (async-drop) is A5.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct Sigma
{
    /// The live (bound, not-yet-consumed) linear obligations; the innermost is
    /// last, matching `Γ`'s stack discipline.
    live: Vec<(String, ValueType)>,
}

impl Sigma
{
    /// Creates an empty linear zone.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Whether the zone has no live obligation — every bound obligation has
    /// been consumed (no leak).
    ///
    /// # Contract
    /// - ensures: returns `true` iff no obligation is live.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> ContextEmptyStatus
    {
        self.live.is_empty().into()
    }

    /// The number of live (unconsumed) obligations.
    ///
    /// # Contract
    /// - ensures: returns the count of obligations bound but not yet consumed.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn len(&self) -> ContextLength
    {
        self.live.len().into()
    }

    /// Binds a linear obligation `name : ty` (**no weakening**: it must be
    /// consumed before its scope closes, else it leaks).
    ///
    /// # Contract
    /// - ensures: `name : ty` becomes the innermost live obligation; the zone
    ///   is no longer empty.
    /// - panics: none.
    #[inline]
    pub fn bind(
        &mut self,
        name: String,
        ty: ValueType,
    )
    {
        self.live.push((name, ty));
    }

    /// Consumes the obligation `name` **exactly once** (**no contraction**):
    /// removes the innermost live obligation of that name and returns its type.
    ///
    /// # Contract
    /// - ensures: on the first call for a live `name`, removes that innermost
    ///   obligation and returns `Some(ty)`; a second consume of the same
    ///   obligation, or a consume of an unbound `name`, returns `None` (linear
    ///   use is single-shot).
    /// - panics: none.
    #[inline]
    pub fn consume<'source, N>(
        &mut self,
        name: N,
    ) -> Option<ValueType>
    where
        N: Into<NameRef<'source>>,
    {
        let name = name.into();
        let index = self.live.iter().rposition(|entry| {
            let (ref entry_name, _) = *entry;
            entry_name == name.as_ref()
        })?;
        Some(self.live.remove(index).1)
    }
}

/// The two-zone typing context `Γ; Σ`: a stack of intuitionistic value-type
/// hypotheses ([`Self::bindings`]) and the linear zone [`Sigma`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Ctx
{
    /// The intuitionistic binding stack `Γ`; the innermost binding is last.
    entries: Vec<(String, ValueType)>,
    /// The linear zone `Σ` (vacuous in v0).
    sigma: Sigma,
    /// The **definitional environment**: which names carry an unfolding rule,
    /// at what height, and how transparently.
    ///
    /// It lives on the context rather than beside it because conversion is
    /// reached from three implementations through one relation, and a
    /// definitional equality that varied by *which site asked* would not be a
    /// definitional equality. Carrying it here is what makes "the same
    /// environment everywhere" structural instead of a convention.
    ///
    /// Empty is the valid initial state and is exactly the pre-unfolding
    /// behaviour, so a context built anywhere without one behaves as it always
    /// did.
    defs: Definitions,
}

impl Ctx
{
    /// Creates an empty context.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// The definitional environment this context carries.
    #[inline]
    #[must_use]
    pub fn definitions(&self) -> &Definitions
    {
        &self.defs
    }

    /// The definitional environment, mutably — how a caller populates it.
    #[inline]
    pub fn definitions_mut(&mut self) -> &mut Definitions
    {
        &mut self.defs
    }

    /// Returns a context carrying `defs` (builder style).
    #[inline]
    #[must_use]
    pub fn with_definitions(
        mut self,
        defs: Definitions,
    ) -> Self
    {
        self.defs = defs;
        self
    }

    /// Returns a context extended with `name : ty` (builder style).
    #[inline]
    #[must_use]
    pub fn with<'source, N>(
        mut self,
        name: N,
        ty: ValueType,
    ) -> Self
    where
        N: Into<NameRef<'source>>,
    {
        let name = name.into();
        self.bind(name.as_ref().to_owned(), ty);
        self
    }

    /// Pushes the hypothesis `name : ty`.
    ///
    /// # Contract
    /// - ensures: `name : ty` becomes the innermost (most recent) hypothesis,
    ///   shadowing any prior binding of the same name until a matching
    ///   `unbind`.
    /// - panics: none.
    #[inline]
    pub fn bind(
        &mut self,
        name: String,
        ty: ValueType,
    )
    {
        self.entries.push((name, ty));
    }

    /// Pops the most recent hypothesis.
    ///
    /// Both implementations bind and unbind in strict stack discipline, so the
    /// innermost binding is always the one to remove. An unbind with no
    /// matching bind would silently corrupt that discipline, masking a
    /// machine/checker imbalance; the debug assertion turns it into a panic
    /// under `cfg(debug_assertions)`.
    ///
    /// # Contract
    /// - requires: the context is non-empty — there is a matching prior `bind`
    ///   (callers maintain strict bind/unbind stack discipline).
    /// - ensures: pops the innermost hypothesis, restoring the context to its
    ///   state before the matching `bind`.
    /// - panics: a `debug_assert!` fires under `cfg(debug_assertions)` if the
    ///   context is empty (a bind/unbind imbalance); in release builds the
    ///   assertion is compiled out and the underlying `Vec::pop` is a no-op.
    #[inline]
    pub fn unbind(&mut self)
    {
        debug_assert!(
            !self.entries.is_empty(),
            "Ctx::unbind on an empty context: bind/unbind imbalance"
        );
        self.entries.pop();
    }

    /// The binding stack, innermost binding last (outermost-to-innermost
    /// iteration order).
    ///
    /// Read-only inspection for consumers that render `Γ` — the pipeline's
    /// goals report (A2.2) slices off its base-context prefix here to report
    /// the bindings local to a hole.
    #[inline]
    #[must_use]
    pub fn bindings(&self) -> &[(String, ValueType)]
    {
        &self.entries
    }

    /// Looks up the innermost hypothesis for `name` (rule Var, §"Core rules").
    ///
    /// # Contract
    /// - ensures: returns the innermost (most recently bound, shadowing)
    ///   hypothesis for `name`, or `None` when `name` is unbound.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn lookup<'source, N>(
        &self,
        name: N,
    ) -> Option<&ValueType>
    where
        N: Into<NameRef<'source>>,
    {
        let name = name.into();
        self.entries
            .iter()
            .rev()
            .find(|&entry| {
                let (ref entry_name, _) = *entry;
                entry_name == name.as_ref()
            })
            .map(|entry| {
                let (_, ref entry_ty) = *entry;
                entry_ty
            })
    }

    /// The linear zone `Σ`.
    ///
    /// Read-only inspection: vacuous in v0 (no typing rule populates it), so a
    /// conformance meta-invariant asserts it stays empty across every run.
    #[inline]
    #[must_use]
    pub fn sigma(&self) -> &Sigma
    {
        &self.sigma
    }
}

/// Linear-discipline tests for the `Σ` zone: the one-shot /
/// linear laws, exercised directly over [`Sigma`] so the discipline is pinned
/// *now*, not merely vacuously satisfied because no v0 rule populates `Σ`.
#[cfg(test)]
mod tests
{
    use super::*;

    /// A fresh zone is empty; binding makes an obligation live; consuming it
    /// returns its type and restores emptiness (consumed *exactly once*).
    #[test]
    fn bind_then_consume_is_single_shot()
    {
        let mut sigma = Sigma::new();
        assert!(bool::from(sigma.is_empty()), "a fresh Σ is empty");
        sigma.bind("k".to_owned(), ValueType::Unit);
        assert_eq!(
            1,
            usize::from(sigma.len()),
            "binding makes one obligation live"
        );
        assert!(
            !bool::from(sigma.is_empty()),
            "a bound obligation is a live leak until consumed"
        );
        assert_eq!(
            Some(ValueType::Unit),
            sigma.consume(NameRef::from("k")),
            "the first consume yields the obligation's type"
        );
        assert!(
            bool::from(sigma.is_empty()),
            "consuming the sole obligation empties Σ"
        );
    }

    /// No contraction: a second consume of the same obligation fails (linear
    /// use is single-shot), as does consuming an unbound name.
    #[test]
    fn double_consume_and_absent_consume_fail()
    {
        let mut sigma = Sigma::new();
        sigma.bind("k".to_owned(), ValueType::Unit);
        assert_eq!(Some(ValueType::Unit), sigma.consume(NameRef::from("k")));
        assert_eq!(
            None,
            sigma.consume(NameRef::from("k")),
            "no contraction: an obligation cannot be consumed twice"
        );
        assert_eq!(
            None,
            sigma.consume(NameRef::from("absent")),
            "consuming an unbound obligation fails"
        );
    }

    /// No weakening: a scope that binds an obligation without consuming it
    /// leaves `Σ` non-empty — the leak is detectable (the discipline that, once
    /// a `Σ`-obligation source lands, makes discarding a captured stack without
    /// `resume` a linearity error).
    #[test]
    fn unconsumed_obligation_is_a_detectable_leak()
    {
        let mut sigma = Sigma::new();
        sigma.bind("k".to_owned(), ValueType::Unit);
        assert!(
            !bool::from(sigma.is_empty()),
            "an unconsumed obligation leaves Σ non-empty (no weakening)"
        );
    }

    /// Consume removes the *innermost* obligation under shadowing, matching
    /// `Γ`'s stack discipline.
    #[test]
    fn consume_removes_the_innermost_under_shadowing()
    {
        let mut sigma = Sigma::new();
        sigma.bind("k".to_owned(), ValueType::Unit);
        sigma.bind("k".to_owned(), ValueType::integer());
        assert_eq!(
            sigma.consume(NameRef::from("k")),
            Some(ValueType::integer()),
            "consume removes the innermost obligation first"
        );
        assert_eq!(
            Some(ValueType::Unit),
            sigma.consume(NameRef::from("k")),
            "the outer obligation remains until its own consume"
        );
        assert!(bool::from(sigma.is_empty()));
    }
}
