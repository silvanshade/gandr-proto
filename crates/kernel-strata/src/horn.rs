//! The crate-internal Horn machinery of Bezem–Coquand loop-checking
//! (TCS 913, 2022): compiled clause systems over level variables extended
//! with the pinned bottom generator, the Lemma 3.1 uniform-shift firing
//! rule, the Theorem 3.2 least-model saturation, and the bounded
//! instrumented forward derivation that extracts checkable evidence.
//!
//! The module is private, so everything here except the re-exported
//! [`ModelValue`] stays crate-internal: the public vocabulary (constraints,
//! posets, evidence) lives in [`crate::poset`] and [`crate::entail`] and
//! compiles down to these clause systems through one documented,
//! deterministic enumeration, so the clause indices carried by evidence
//! stay meaningful across the oracle/validator boundary.
//!
//! Paper correspondence (page numbers refer to the TCS 913 note):
//!
//! * A clause `body → head` stands for the infinite family of its upward shifts
//!   `body+k → head+k` with `k ≥ min_shift` (the finitely represented `S̄_C` of
//!   §2; `min_shift = 1` realizes the `S̄_{C⁺} ∪ T` system of the Lemma 2.1
//!   query transformation). Predecessor clauses are implicit: a model is stored
//!   as its per-variable maximum, which **is** the downward-closed atom set of
//!   §3.
//! * [`fire`] is Lemma 3.1: one min/compare decides what the whole shift family
//!   forces from a model.
//! * [`saturate`] is Theorem 3.2 by monotone rounds, with the Corollary 4.2
//!   small-model property turning "a finite value passed the bound" into "this
//!   component is infinite".
//! * [`derive_targets`] is the `⊢_H` forward reasoning of §2, instrumented with
//!   a step log so admission and entailment can hand back replayable
//!   derivations instead of asking for trust.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;

use crate::level::LevelConstant;
use crate::level::LevelError;
use crate::level::LevelOffset;
use crate::level::LevelVar;

/// A Horn atom offset in the Bezem--Coquand shifted clause system.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HornOffset(u128);

impl HornOffset
{
    /// The zero atom offset.
    pub const ZERO: Self = Self(0_u128);

    /// The unit atom offset.
    pub const ONE: Self = Self(1_u128);

    /// Checked addition in the atom-offset domain.
    ///
    /// # Errors
    /// [`LevelError::Overflow`] — the sum passed the `u128` range.
    #[inline]
    pub fn checked_add(
        self,
        rhs: Self,
    ) -> Result<Self, LevelError>
    {
        self.0
            .checked_add(rhs.0)
            .map(Self)
            .ok_or(LevelError::Overflow)
    }

    /// Checked addition of a clause-family shift.
    ///
    /// # Errors
    /// [`LevelError::Overflow`] — the shifted sum passed the `u128` range.
    #[inline]
    pub fn checked_add_shift(
        self,
        shift: HornShift,
    ) -> Result<Self, LevelError>
    {
        self.0
            .checked_add(u128::from(shift))
            .map(Self)
            .ok_or(LevelError::Overflow)
    }

    /// Saturating subtraction in the atom-offset domain.
    #[inline]
    #[must_use]
    pub fn saturating_sub(
        self,
        rhs: Self,
    ) -> Self
    {
        Self(self.0.saturating_sub(rhs.0))
    }

    /// The upward shift that carries a zero-offset atom to this offset.
    ///
    /// This is the explicit cross-domain projection for the theorem's
    /// "start at zero and shift upward" cases; it replaces the old blanket
    /// conversion between atom offsets and clause-family shifts.
    #[inline]
    #[must_use]
    pub const fn shift_from_zero(self) -> HornShift
    {
        HornShift(self.0)
    }
}

impl From<u128> for HornOffset
{
    #[inline]
    fn from(offset: u128) -> Self
    {
        Self(offset)
    }
}

impl From<HornOffset> for u128
{
    #[inline]
    fn from(offset: HornOffset) -> Self
    {
        offset.0
    }
}

impl From<LevelOffset> for HornOffset
{
    #[inline]
    fn from(offset: LevelOffset) -> Self
    {
        Self(u128::from(offset))
    }
}

impl From<LevelConstant> for HornOffset
{
    #[inline]
    fn from(constant: LevelConstant) -> Self
    {
        Self(u128::from(constant))
    }
}

/// An upward shift applied to a represented Horn clause family.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HornShift(u128);

impl From<u128> for HornShift
{
    #[inline]
    fn from(shift: u128) -> Self
    {
        Self(shift)
    }
}

impl From<HornShift> for u128
{
    #[inline]
    fn from(shift: HornShift) -> Self
    {
        shift.0
    }
}

impl HornShift
{
    /// The atom offset reached by applying this shift to a zero-offset atom.
    ///
    /// This is the explicit cross-domain projection for loop-witness seeds and
    /// targets, where the math has already fixed the starting offset at zero.
    #[inline]
    #[must_use]
    pub const fn offset_from_zero(self) -> HornOffset
    {
        HornOffset(self.0)
    }
}

/// A base-clause index in the documented compilation order.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ClauseIndex(usize);

impl From<usize> for ClauseIndex
{
    #[inline]
    fn from(index: usize) -> Self
    {
        Self(index)
    }
}

impl From<ClauseIndex> for usize
{
    #[inline]
    fn from(index: ClauseIndex) -> Self
    {
        index.0
    }
}

impl fmt::Display for ClauseIndex
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        usize::from(*self).fmt(f)
    }
}

/// The maximum gain of a shifted Horn clause system.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct MaxGain(u128);

impl From<u128> for MaxGain
{
    #[inline]
    fn from(gain: u128) -> Self
    {
        Self(gain)
    }
}

impl From<MaxGain> for u128
{
    #[inline]
    fn from(gain: MaxGain) -> Self
    {
        gain.0
    }
}

/// A small-model saturation bound.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SaturationBound(u128);

impl From<u128> for SaturationBound
{
    #[inline]
    fn from(bound: u128) -> Self
    {
        Self(bound)
    }
}

impl From<SaturationBound> for u128
{
    #[inline]
    fn from(bound: SaturationBound) -> Self
    {
        bound.0
    }
}

/// A finite round limit for derivation extraction.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DerivationRoundLimit(u128);

impl From<u128> for DerivationRoundLimit
{
    #[inline]
    fn from(limit: u128) -> Self
    {
        Self(limit)
    }
}

impl From<DerivationRoundLimit> for u128
{
    #[inline]
    fn from(limit: DerivationRoundLimit) -> Self
    {
        limit.0
    }
}

/// Whether a model value is infinite.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelIsInfinite(bool);

impl From<bool> for ModelIsInfinite
{
    #[inline]
    fn from(is_infinite: bool) -> Self
    {
        Self(is_infinite)
    }
}

impl From<ModelIsInfinite> for bool
{
    #[inline]
    fn from(is_infinite: ModelIsInfinite) -> Self
    {
        is_infinite.0
    }
}

/// Whether a stored model value covers a requested Horn atom.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AtomCoverage(bool);

impl From<bool> for AtomCoverage
{
    #[inline]
    fn from(covered: bool) -> Self
    {
        Self(covered)
    }
}

impl From<AtomCoverage> for bool
{
    #[inline]
    fn from(covered: AtomCoverage) -> Self
    {
        covered.0
    }
}

/// Whether a clause is self-subsumed.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClauseTriviality(bool);

impl From<bool> for ClauseTriviality
{
    #[inline]
    fn from(trivial: bool) -> Self
    {
        Self(trivial)
    }
}

impl From<ClauseTriviality> for bool
{
    #[inline]
    fn from(trivial: ClauseTriviality) -> Self
    {
        trivial.0
    }
}

/// A represented Horn atom `variable + offset`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HornAtom
{
    /// The atom's variable.
    variable: HVar,
    /// The atom's offset.
    offset: HornOffset,
}

impl HornAtom
{
    /// Build a Horn atom from semantic parts.
    #[inline]
    #[must_use]
    pub const fn new(
        variable: HVar,
        offset: HornOffset,
    ) -> Self
    {
        Self { variable, offset }
    }

    /// The atom's variable.
    #[inline]
    #[must_use]
    pub const fn variable(self) -> HVar
    {
        self.variable
    }

    /// The atom's offset.
    #[inline]
    #[must_use]
    pub const fn offset(self) -> HornOffset
    {
        self.offset
    }
}

/// One finite firing recorded in a derivation log.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FiringLogStep
{
    /// The fired base clause.
    clause: ClauseIndex,
    /// The upward shift used by the fired instance.
    shift: HornShift,
}

impl FiringLogStep
{
    /// Build a finite firing-log step.
    #[inline]
    #[must_use]
    pub const fn new(
        clause: ClauseIndex,
        shift: HornShift,
    ) -> Self
    {
        Self { clause, shift }
    }

    /// The fired base clause.
    #[inline]
    #[must_use]
    pub const fn clause(self) -> ClauseIndex
    {
        self.clause
    }

    /// The upward shift used by the fired instance.
    #[inline]
    #[must_use]
    pub const fn shift(self) -> HornShift
    {
        self.shift
    }
}

/// A Horn-system variable: the pinned bottom generator `⊥` or an ordinary
/// level variable.
///
/// `⊥` carries constants across the paper's constant-free algebra (a level
/// constant `c` becomes the atom `⊥ + c`) and is deliberately
/// unrepresentable in user levels — it exists only inside compiled systems,
/// so no user input can collide with it (kernel-boundary.md K1). `⊥` orders
/// before every variable, which keeps clause and seed enumeration
/// deterministic.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HVar
{
    /// The pinned bottom generator.
    Bottom,
    /// An ordinary level variable.
    Var(LevelVar),
}

/// A value of the model domain `ℕ^∞` (TCS 913 §3): the downward-closed atom
/// set `{v+k | k ≤ f(v)}` of a variable, represented by its maximum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelValue
{
    /// A finite maximum.
    Finite(HornOffset),
    /// The infinite value: every atom over the variable holds.
    Infinite,
}

impl ModelValue
{
    /// Whether this is the infinite value.
    #[inline]
    #[must_use]
    pub const fn is_infinite(self) -> ModelIsInfinite
    {
        ModelIsInfinite(matches!(self, Self::Infinite))
    }

    /// The finite maximum, when there is one.
    #[inline]
    #[must_use]
    pub const fn as_finite(self) -> Option<HornOffset>
    {
        match self {
            | Self::Finite(value) => Some(value),
            | Self::Infinite => None,
        }
    }

    /// Whether this value covers the atom offset `offset` (the atom holds in
    /// the downward-closed set the value stands for).
    #[inline]
    #[must_use]
    pub(crate) fn covers(
        self,
        offset: HornOffset,
    ) -> AtomCoverage
    {
        AtomCoverage(match self {
            | Self::Finite(value) => offset <= value,
            | Self::Infinite => true,
        })
    }
}

/// One base Horn clause `body → head`, standing for the family of its
/// upward shifts at or above the owning system's minimum shift.
///
/// The body is nonempty and canonical: sorted by variable with one
/// (maximal) offset per variable — a body atom `x + 2` beside `x + 5` is
/// redundant, since the downward-closed model semantics makes the higher
/// atom imply the lower.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HornClause
{
    /// The body atoms, sorted by variable, one maximal offset each.
    body: Vec<HornAtom>,
    /// The conclusion atom.
    head: HornAtom,
}

impl HornClause
{
    /// Build a canonical clause from raw body atoms and a head.
    ///
    /// # Contract
    /// - requires: nothing; any raw atom bag is admissible.
    /// - ensures: `Some` exactly when the body is nonempty, holding the
    ///   canonicalized body (sorted, deduplicated by maximal offset).
    /// - provides: the only constructor — a non-canonical clause is
    ///   unrepresentable.
    /// - fails: `None` on an empty body (the paper's clauses have nonempty
    ///   bodies; an empty-bodied clause would assert an unconditional atom,
    ///   which no constraint compiles to).
    /// - panics: none.
    pub fn new(
        raw_body: &[HornAtom],
        head: HornAtom,
    ) -> Option<Self>
    {
        if raw_body.is_empty() {
            return None;
        }
        let mut merged: BTreeMap<HVar, HornOffset> = BTreeMap::new();
        for &atom in raw_body {
            let variable = atom.variable();
            let offset = atom.offset();
            let maximum = merged.entry(variable).or_insert(offset);
            *maximum = (*maximum).max(offset);
        }
        let body = merged
            .into_iter()
            .map(|(variable, offset)| HornAtom::new(variable, offset))
            .collect();
        Some(Self { body, head })
    }

    /// Whether the clause is self-subsumed: some body atom over the head
    /// variable already dominates the conclusion, so every model satisfies
    /// the whole shift family vacuously. Compilation omits such clauses.
    pub fn is_trivial(&self) -> ClauseTriviality
    {
        let head = self.head;
        ClauseTriviality::from(
            self.body
                .iter()
                .any(|&atom| atom.variable() == head.variable() && atom.offset() >= head.offset()),
        )
    }

    /// The body atoms, sorted by variable.
    pub fn body(&self) -> &[HornAtom]
    {
        &self.body
    }

    /// The conclusion atom.
    pub const fn head(&self) -> HornAtom
    {
        self.head
    }

    /// The clause's gain floored at zero: conclusion offset minus the
    /// minimal body offset (TCS 913 §3). The system-wide maximum of these
    /// is `Maxgain`, the engine's growth modulus.
    fn gain(&self) -> MaxGain
    {
        let min_body = self
            .body
            .iter()
            .map(|atom| atom.offset())
            .min()
            .unwrap_or(HornOffset::ZERO);
        MaxGain::from(u128::from(self.head.offset().saturating_sub(min_body)))
    }
}

/// A compiled clause system: base clauses plus the minimum admissible
/// upward shift (`0` for the admission system `S̄_C`, `1` for the query
/// system `S̄_{C⁺} ∪ T` of the Lemma 2.1 transformation).
#[derive(Clone, Debug)]
pub struct ClauseSystem
{
    /// The base clauses, in the documented deterministic compilation order.
    pub clauses: Vec<HornClause>,
    /// The minimum admissible upward shift.
    pub min_shift: HornShift,
}

impl ClauseSystem
{
    /// `Maxgain` (TCS 913 §3): the smallest bound `≥ 0` on every clause's
    /// gain. Shift-invariant, so it is computed once on the base clauses.
    pub fn maxgain(&self) -> MaxGain
    {
        self.clauses
            .iter()
            .map(HornClause::gain)
            .max()
            .unwrap_or_else(|| MaxGain::from(0_u128))
    }

    /// The maximal body offset over all base clauses (the paper's `m` in
    /// Corollary 3.5, seeding the loop check).
    pub fn max_body_offset(&self) -> HornOffset
    {
        self.clauses
            .iter()
            .flat_map(|clause| clause.body().iter().map(|atom| atom.offset()))
            .max()
            .unwrap_or(HornOffset::ZERO)
    }
}

/// The strongest conclusion a clause family forces from a model, per
/// [`fire`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Firing
{
    /// The forced value of the head variable.
    pub value: ModelValue,
    /// The shift the strongest instance fires at; `None` exactly when every
    /// body variable is infinite (the conclusion is infinite with no single
    /// witnessing instance).
    pub shift: Option<HornShift>,
}

/// Theorem 3.2 by monotone rounds: the least model of `system` above
/// `seed`, snapping finite values above `snap_bound` to the infinite value.
///
/// # Contract
/// - requires: `snap_bound` is a Corollary 4.2 small-model bound for `seed` —
///   at least `max_finite(seed) + |V| · Maxgain` — so that a value driven past
///   it can only belong to an infinite component of the true least model.
/// - ensures: the returned values are the least model of the system's shift
///   families above `seed` (with predecessor clauses implicit in the
///   downward-closed representation); `diverged` holds exactly when some
///   component is infinite; the log replays every finite-shift update in order.
/// - provides: the shared model engine of admission (Corollary 3.5) and
///   entailment (Corollary 3.4).
/// - fails: [`LevelError::Overflow`] only through [`fire`] on adversarial
///   inputs; engine-internal values stay at most `snap_bound + Maxgain`.
/// - panics: none.
/// - termination: values are monotone per variable, bounded by `snap_bound`
///   while finite, and cross into the infinite value at most once per variable,
///   so every round without progress is final and only finitely many rounds
///   make progress.
///
/// # Adequacy
/// - hypothesis: L2 — the paper's §5.2 worked example pins the exact fixpoint
///   and its loop variant pins the divergence set, and the empty-poset
///   differential pins the degenerate case against the slice-1 oracle; the L3
///   residue is the snap decision, exercised by the loop goldens (a mutant
///   snapping early or late moves the divergence set).
/// - witness: `horn::tests::paper_example_reaches_the_published_fixpoint`
/// - witness: `horn::tests::paper_loop_variant_diverges_everywhere`
/// - witness: `poset_differential` (the integration suite, empty-poset
///   agreement)
pub fn saturate(
    system: &ClauseSystem,
    seed: BTreeMap<HVar, ModelValue>,
    snap_bound: SaturationBound,
) -> Result<Saturation, LevelError>
{
    let mut values = seed;
    let mut log = Vec::new();
    let mut diverged = false;
    loop {
        let mut progress = false;
        for (index, clause) in system.clauses.iter().enumerate() {
            let Some(firing) = fire(clause, &values, system.min_shift)?
            else {
                continue;
            };
            let head = clause.head();
            let head_var = head.variable();
            let current = values.get(&head_var).copied();
            let (candidate, snapped) = match firing.value {
                | ModelValue::Infinite => (ModelValue::Infinite, false),
                | ModelValue::Finite(value) if u128::from(value) > u128::from(snap_bound) => {
                    (ModelValue::Infinite, true)
                },
                | finite @ ModelValue::Finite(_) => (finite, false),
            };
            let improved = match (current, candidate) {
                | (Some(ModelValue::Infinite), _) => false,
                | (None, _) | (Some(ModelValue::Finite(_)), ModelValue::Infinite) => true,
                | (Some(ModelValue::Finite(existing)), ModelValue::Finite(update)) => {
                    update > existing
                },
            };
            if improved {
                if snapped || bool::from(candidate.is_infinite()) {
                    diverged = true;
                }
                if let Some(shift) = firing.shift {
                    log.push(FiringLogStep::new(ClauseIndex::from(index), shift));
                }
                let _previous = values.insert(head_var, candidate);
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }
    Ok(Saturation {
        values,
        log,
        diverged,
    })
}

/// The result of [`saturate`]: the least model, the finite-shift firing
/// log, and whether any component left the finite range.
#[derive(Clone, Debug)]
pub struct Saturation
{
    /// The least model above the seed.
    pub values: BTreeMap<HVar, ModelValue>,
    /// Every applied finite-shift firing, in application order, as
    /// `(clause index, shift)`. Infinite-conclusion firings (all body
    /// variables infinite) are not logged — they have no single replayable
    /// instance and occur only after `diverged` is already set.
    pub log: Vec<FiringLogStep>,
    /// Whether some component was driven past the small-model bound (its
    /// true least-model value is infinite, by Corollary 4.2).
    pub diverged: bool,
}

/// Instrumented `⊢_H` forward reasoning (TCS 913 §2): starting from exactly
/// the `seed` atoms, fire clauses until every target atom is derived,
/// logging each applied step.
///
/// # Contract
/// - requires: a finite `seed` (the `⊢_H` body; variables absent from it have
///   no atoms until derived).
/// - ensures: `Ok(Some(log))` exactly when every target was covered within
///   `round_limit` rounds, with the log replaying to a state covering all
///   targets from the seed; `Ok(None)` when the limit passed first (the caller
///   treats this as the theory-refuting case and surfaces it as a typed error
///   rather than trusting it).
/// - provides: the evidence extractor behind loop witnesses — the Corollary 3.5
///   claim `S̄_C ⊢_H W+n → w+n+1` made replayable.
/// - fails: [`LevelError::Overflow`] when a derived value passes the `u128`
///   range (bounded in practice by `round_limit · Maxgain` above the seed).
/// - panics: none.
/// - termination: at most `round_limit` rounds, each over finitely many
///   clauses; early exit on coverage.
///
/// # Adequacy
/// - hypothesis: L2 — the loop goldens (paper §5.2 variant, self-successor
///   equality) pin extraction end-to-end: the produced log must replay through
///   the independent witness validator, so an extraction mutant is caught at
///   validation; the L3 residue is the early-exit boundary (coverage checked
///   after every step, pinned by the one-step golden).
/// - witness: `horn::tests::derive_targets_extracts_a_checkable_log`
/// - witness: `poset::tests::paper_loop_variant_returns_a_validating_loop_witness`
/// - witness: `poset::tests::self_successor_equality_loops`
pub fn derive_targets(
    system: &ClauseSystem,
    seed: BTreeMap<HVar, ModelValue>,
    targets: &[HornAtom],
    round_limit: DerivationRoundLimit,
) -> Result<Option<Vec<FiringLogStep>>, LevelError>
{
    let mut values = seed;
    let mut log = Vec::new();
    if bool::from(covered(&values, targets)) {
        return Ok(Some(log));
    }
    let mut round = 0_u128;
    while round < u128::from(round_limit) {
        let mut progress = false;
        for (index, clause) in system.clauses.iter().enumerate() {
            let Some(firing) = fire(clause, &values, system.min_shift)?
            else {
                continue;
            };
            let Some(shift) = firing.shift
            else {
                continue;
            };
            let head = clause.head();
            let head_var = head.variable();
            let current = values.get(&head_var).copied();
            let improved = match (current, firing.value) {
                | (Some(ModelValue::Infinite), _) => false,
                | (None, _) | (Some(ModelValue::Finite(_)), ModelValue::Infinite) => true,
                | (Some(ModelValue::Finite(existing)), ModelValue::Finite(update)) => {
                    update > existing
                },
            };
            if improved {
                let _previous = values.insert(head_var, firing.value);
                log.push(FiringLogStep::new(ClauseIndex::from(index), shift));
                progress = true;
                if bool::from(covered(&values, targets)) {
                    return Ok(Some(log));
                }
            }
        }
        if !progress {
            break;
        }
        round = round.checked_add(1_u128).ok_or(LevelError::Overflow)?;
    }
    Ok(None)
}

/// Whether every target atom holds in `values`.
pub fn covered(
    values: &BTreeMap<HVar, ModelValue>,
    targets: &[HornAtom],
) -> AtomCoverage
{
    AtomCoverage::from(targets.iter().all(|&target| {
        values
            .get(&target.variable())
            .is_some_and(|value| bool::from(value.covers(target.offset())))
    }))
}

/// The Lemma 3.1 firing rule: what the whole shift family of `clause`
/// (shifts `≥ min_shift`) forces from `model`.
///
/// # Contract
/// - requires: nothing; `model` may be partial (an absent variable has no atoms
///   — the `⊢_H` reading) and may hold adversarial values (the validator
///   reading).
/// - ensures: `Ok(None)` when no admissible instance's body is satisfied (an
///   absent body variable, a finite maximum below a body offset, or a maximal
///   admissible shift below `min_shift`); `Ok(Some(firing))` with the maximal
///   forced conclusion otherwise — `head + k₀` at the maximal admissible shift
///   `k₀ = min(model(xᵢ) − kᵢ)` over finite body variables, or the infinite
///   value when every body variable is infinite.
/// - provides: the single firing rule shared by saturation, derivation, and the
///   countermodel validator.
/// - fails: [`LevelError::Overflow`] when `head + k₀` passes the `u128` range
///   (unreachable for engine-bounded models, reachable for adversarial
///   validator inputs — rejected, never wrapped).
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — each guard arm is pinned by an exact unit case, and the
///   rule as a whole is exercised by the paper-example golden and the
///   differential property suite (a firing mutant shifts the computed fixpoint
///   away from the published one or breaks slice-1 agreement).
/// - witness: `horn::tests::firing_requires_the_minimum_shift`
/// - witness: `horn::tests::firing_skips_absent_and_dominated_bodies`
/// - witness: `horn::tests::infinite_bodies_force_infinite_conclusions`
/// - witness: `horn::tests::paper_example_reaches_the_published_fixpoint`
pub fn fire(
    clause: &HornClause,
    model: &BTreeMap<HVar, ModelValue>,
    min_shift: HornShift,
) -> Result<Option<Firing>, LevelError>
{
    let mut max_shift: Option<HornShift> = None;
    let mut all_infinite = true;
    for &atom in clause.body() {
        match model.get(&atom.variable()) {
            | None => return Ok(None),
            | Some(&ModelValue::Infinite) => {},
            | Some(&ModelValue::Finite(maximum)) => {
                all_infinite = false;
                let Some(cap) = u128::from(maximum).checked_sub(u128::from(atom.offset()))
                else {
                    return Ok(None);
                };
                let cap = HornShift::from(cap);
                max_shift = Some(max_shift.map_or(cap, |current| current.min(cap)));
            },
        }
    }
    if all_infinite {
        return Ok(Some(Firing {
            value: ModelValue::Infinite,
            shift: None,
        }));
    }
    let Some(shift) = max_shift
    else {
        return Ok(None);
    };
    if shift < min_shift {
        return Ok(None);
    }
    let head = clause.head();
    let value = head.offset().checked_add_shift(shift)?;
    Ok(Some(Firing {
        value: ModelValue::Finite(value),
        shift: Some(shift),
    }))
}

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::ClauseIndex;
    use super::ClauseSystem;
    use super::DerivationRoundLimit;
    use super::FiringLogStep;
    use super::HVar;
    use super::HornAtom;
    use super::HornClause;
    use super::HornOffset;
    use super::HornShift;
    use super::ModelValue;
    use super::SaturationBound;
    use super::derive_targets;
    use super::fire;
    use super::saturate;
    use crate::level::LevelVar;
    use crate::level::LevelVarIndex;

    #[test]
    fn paper_example_reaches_the_published_fixpoint()
    {
        let system = ClauseSystem {
            clauses: paper_clauses(),
            min_shift: HornShift::from(0_u128),
        };
        assert_eq!(
            super::MaxGain::from(3_u128),
            system.maxgain(),
            "the paper reports Maxgain = 3"
        );
        let bound = SaturationBound::from(15_u128);
        let outcome = saturate(&system, zero_seed(), bound).unwrap();
        assert!(!outcome.diverged, "the base example has a finite model");
        let expected: BTreeMap<HVar, ModelValue> = [
            (v(0), ModelValue::Finite(HornOffset::from(0_u128))),
            (v(1), ModelValue::Finite(HornOffset::from(1_u128))),
            (v(2), ModelValue::Finite(HornOffset::from(4_u128))),
            (v(3), ModelValue::Finite(HornOffset::from(3_u128))),
            (v(4), ModelValue::Finite(HornOffset::from(1_u128))),
        ]
        .into_iter()
        .collect();
        assert_eq!(
            outcome.values, expected,
            "the minimal model above zero is 01431 in order abcde (TCS 913 §5.2)"
        );
    }

    #[test]
    fn paper_loop_variant_diverges_everywhere()
    {
        let mut clauses = paper_clauses();
        clauses.push(
            HornClause::new(
                &[HornAtom::new(v(4), HornOffset::from(0_u128))],
                HornAtom::new(v(0), HornOffset::from(0_u128)),
            )
            .unwrap(),
        );
        let system = ClauseSystem {
            clauses,
            min_shift: HornShift::from(0_u128),
        };
        let bound = SaturationBound::from(15_u128);
        let outcome = saturate(&system, zero_seed(), bound).unwrap();
        assert!(outcome.diverged, "adding e → a creates a loop");
        for index in 0_u32 .. 5_u32 {
            assert_eq!(
                Some(&ModelValue::Infinite),
                outcome.values.get(&v(index)),
                "every variable diverges in the loop variant (W = abcde)"
            );
        }
    }

    /// The paper's §5.2 base clauses
    /// `{a,b → b+1; b → c+3; c+1 → d; b,d+2 → e}`.
    fn paper_clauses() -> Vec<HornClause>
    {
        vec![
            HornClause::new(
                &[
                    HornAtom::new(v(0), HornOffset::from(0_u128)),
                    HornAtom::new(v(1), HornOffset::from(0_u128)),
                ],
                HornAtom::new(v(1), HornOffset::from(1_u128)),
            )
            .unwrap(),
            HornClause::new(
                &[HornAtom::new(v(1), HornOffset::from(0_u128))],
                HornAtom::new(v(2), HornOffset::from(3_u128)),
            )
            .unwrap(),
            HornClause::new(
                &[HornAtom::new(v(2), HornOffset::from(1_u128))],
                HornAtom::new(v(3), HornOffset::from(0_u128)),
            )
            .unwrap(),
            HornClause::new(
                &[
                    HornAtom::new(v(1), HornOffset::from(0_u128)),
                    HornAtom::new(v(3), HornOffset::from(2_u128)),
                ],
                HornAtom::new(v(4), HornOffset::from(0_u128)),
            )
            .unwrap(),
        ]
    }

    /// A total zero seed over the paper's five variables.
    fn zero_seed() -> BTreeMap<HVar, ModelValue>
    {
        (0_u32 .. 5_u32)
            .map(|index| (v(index), ModelValue::Finite(HornOffset::from(0_u128))))
            .collect()
    }

    #[test]
    fn firing_requires_the_minimum_shift()
    {
        let clause = HornClause::new(
            &[HornAtom::new(v(0), HornOffset::from(0_u128))],
            HornAtom::new(v(1), HornOffset::from(0_u128)),
        )
        .unwrap();
        let model: BTreeMap<HVar, ModelValue> =
            core::iter::once((v(0), ModelValue::Finite(HornOffset::from(0_u128)))).collect();
        let at_zero = fire(&clause, &model, HornShift::from(0_u128)).unwrap();
        assert!(at_zero.is_some(), "shift 0 is admissible at min_shift 0");
        let at_one = fire(&clause, &model, HornShift::from(1_u128)).unwrap();
        assert!(at_one.is_none(), "shift 0 is inadmissible at min_shift 1");
    }

    #[test]
    fn firing_skips_absent_and_dominated_bodies()
    {
        let clause = HornClause::new(
            &[HornAtom::new(v(0), HornOffset::from(2_u128))],
            HornAtom::new(v(1), HornOffset::from(0_u128)),
        )
        .unwrap();
        let absent: BTreeMap<HVar, ModelValue> = BTreeMap::new();
        assert!(
            fire(&clause, &absent, HornShift::from(0_u128))
                .unwrap()
                .is_none(),
            "an absent body variable blocks every instance"
        );
        let below: BTreeMap<HVar, ModelValue> =
            core::iter::once((v(0), ModelValue::Finite(HornOffset::from(1_u128)))).collect();
        assert!(
            fire(&clause, &below, HornShift::from(0_u128))
                .unwrap()
                .is_none(),
            "a maximum below the body offset blocks every instance"
        );
    }

    #[test]
    fn infinite_bodies_force_infinite_conclusions()
    {
        let clause = HornClause::new(
            &[HornAtom::new(v(0), HornOffset::from(3_u128))],
            HornAtom::new(v(1), HornOffset::from(0_u128)),
        )
        .unwrap();
        let model: BTreeMap<HVar, ModelValue> =
            core::iter::once((v(0), ModelValue::Infinite)).collect();
        let firing = fire(&clause, &model, HornShift::from(0_u128))
            .unwrap()
            .unwrap();
        assert_eq!(
            ModelValue::Infinite,
            firing.value,
            "an all-infinite body forces an infinite conclusion"
        );
        assert_eq!(None, firing.shift, "no single instance witnesses it");
    }

    #[test]
    fn mixed_finite_bodies_bound_the_shift()
    {
        let clause = HornClause::new(
            &[
                HornAtom::new(v(0), HornOffset::from(0_u128)),
                HornAtom::new(v(1), HornOffset::from(2_u128)),
            ],
            HornAtom::new(v(2), HornOffset::from(1_u128)),
        )
        .unwrap();
        let model: BTreeMap<HVar, ModelValue> = [
            (v(0), ModelValue::Infinite),
            (v(1), ModelValue::Finite(HornOffset::from(5_u128))),
        ]
        .into_iter()
        .collect();
        let firing = fire(&clause, &model, HornShift::from(0_u128))
            .unwrap()
            .unwrap();
        assert_eq!(
            ModelValue::Finite(HornOffset::from(4_u128)),
            firing.value,
            "the finite body variable bounds the shift: k₀ = 5 − 2 = 3, head 1 + 3 = 4"
        );
        assert_eq!(Some(HornShift::from(3_u128)), firing.shift);
    }

    #[test]
    fn derive_targets_extracts_a_checkable_log()
    {
        // The self-loop `x → x+1`: from `x+0`, deriving `x+3` takes three
        // logged pumping steps.
        let clauses = vec![
            HornClause::new(
                &[HornAtom::new(v(0), HornOffset::from(0_u128))],
                HornAtom::new(v(0), HornOffset::from(1_u128)),
            )
            .unwrap(),
        ];
        let system = ClauseSystem {
            clauses,
            min_shift: HornShift::from(0_u128),
        };
        let seed: BTreeMap<HVar, ModelValue> =
            core::iter::once((v(0), ModelValue::Finite(HornOffset::from(0_u128)))).collect();
        let targets = [HornAtom::new(v(0), HornOffset::from(3_u128))];
        let log = derive_targets(&system, seed, &targets, DerivationRoundLimit::from(16_u128))
            .unwrap()
            .expect("the target is derivable");
        assert_eq!(
            log,
            vec![
                FiringLogStep::new(ClauseIndex::from(0_usize), HornShift::from(0_u128)),
                FiringLogStep::new(ClauseIndex::from(0_usize), HornShift::from(1_u128)),
                FiringLogStep::new(ClauseIndex::from(0_usize), HornShift::from(2_u128)),
            ],
            "each pumping step fires the sole clause at the next shift"
        );
    }

    #[test]
    fn derive_targets_reports_underivable_targets()
    {
        let clauses = vec![
            HornClause::new(
                &[HornAtom::new(v(0), HornOffset::from(0_u128))],
                HornAtom::new(v(1), HornOffset::from(0_u128)),
            )
            .unwrap(),
        ];
        let system = ClauseSystem {
            clauses,
            min_shift: HornShift::from(0_u128),
        };
        let seed: BTreeMap<HVar, ModelValue> =
            core::iter::once((v(0), ModelValue::Finite(HornOffset::from(0_u128)))).collect();
        let targets = [HornAtom::new(v(1), HornOffset::from(5_u128))];
        let outcome =
            derive_targets(&system, seed, &targets, DerivationRoundLimit::from(16_u128)).unwrap();
        assert_eq!(
            None, outcome,
            "a fixpoint below the target reports non-derivability"
        );
    }

    #[test]
    fn trivial_clauses_are_recognized()
    {
        let subsumed = HornClause::new(
            &[HornAtom::new(v(0), HornOffset::from(1_u128))],
            HornAtom::new(v(0), HornOffset::from(0_u128)),
        )
        .unwrap();
        assert!(
            bool::from(subsumed.is_trivial()),
            "x+1 → x is self-subsumed"
        );
        let productive = HornClause::new(
            &[HornAtom::new(v(0), HornOffset::from(0_u128))],
            HornAtom::new(v(0), HornOffset::from(1_u128)),
        )
        .unwrap();
        assert!(
            !bool::from(productive.is_trivial()),
            "x → x+1 is productive"
        );
    }

    #[test]
    fn clause_bodies_canonicalize()
    {
        let clause = HornClause::new(
            &[
                HornAtom::new(v(0), HornOffset::from(2_u128)),
                HornAtom::new(v(0), HornOffset::from(5_u128)),
                HornAtom::new(v(1), HornOffset::from(0_u128)),
            ],
            HornAtom::new(v(2), HornOffset::from(0_u128)),
        )
        .unwrap();
        assert_eq!(
            clause.body(),
            &[
                HornAtom::new(v(0), HornOffset::from(5_u128)),
                HornAtom::new(v(1), HornOffset::from(0_u128)),
            ],
            "duplicate body variables keep the maximal offset"
        );
        assert!(
            HornClause::new(&[], HornAtom::new(v(0), HornOffset::from(0_u128))).is_none(),
            "an empty body is unrepresentable"
        );
    }

    /// The paper's §5.2 variables `a`–`e` as level variables `0`–`4`.
    fn v(index: impl Into<LevelVarIndex>) -> HVar
    {
        HVar::Var(LevelVar::new(index.into()))
    }
}
