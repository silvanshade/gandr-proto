//! Entailment under an admitted landmark poset (TCS 913 Corollary 3.4):
//! `left ≤ right` (or `left < right`) under the poset's order hypotheses,
//! decided by the minimal-model computation and returning checkable
//! evidence either way — an [`EntailmentWitness`] (a replayable forward
//! derivation of every goal atom) or an [`EntailmentCountermodel`] (the
//! minimal model itself, which satisfies the whole query system and the
//! query body while refuting a goal atom).
//!
//! # The query encoding (one documented, deterministic translation)
//!
//! A query `left ≤ right` asks, per Theorem 2.2, whether each atom of
//! `left` follows from the atoms of `right` — with two adjustments, both
//! definitional and shared verbatim by the oracle and the validators:
//!
//! * **Constants ride the pinned bottom generator `⊥`** (the paper's algebra is
//!   constant-free): a constant part `c` becomes the atom `⊥ + c`, and one
//!   bottom clause `x → ⊥` per in-scope variable orders `⊥` below everything
//!   (see [`crate::poset`] — the module docs record why this is sound,
//!   conservative, and loop-immune exactly for variable-only declared
//!   constraints). Under this encoding the derived maximum of `⊥` is the right
//!   side's value at the zero valuation, which is what makes empty-poset
//!   agreement with the slice-1 oracle exact.
//! * **Everything shifts up by one** (the paper's Lemma 2.1 device, used
//!   uniformly): the system takes minimum shift `1`, every in-scope variable is
//!   seeded at `0` (the lemma's `V` body atoms), the right side seeds at
//!   `offset + 1` (`⊥` at `c_right + 1`), and each goal atom asks one above its
//!   offset. This makes queries over variables missing from the body sound
//!   without a case split.
//!
//! The goal list is `⊥ + c_left + 1 (+1 when strict)` first, then one goal
//! per left atom in ascending variable order at `offset + 1 (+1 when
//! strict)`; strictness is the same left-shift the slice-1 oracle uses.
//!
//! Divergence cannot occur here: an admitted poset has no loop
//! (Corollary 3.5), bottom clauses cannot close a cycle, and upward shifts
//! preserve gains, so the query system's minimal models are finite. The
//! engine still guards the computation and surfaces the impossible case as
//! [`PosetError::UnexpectedDivergence`] rather than trusting it.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use core::ops::Not;

use crate::horn;
use crate::horn::ClauseIndex;
use crate::horn::ClauseSystem;
use crate::horn::FiringLogStep;
use crate::horn::HVar;
use crate::horn::HornAtom;
use crate::horn::HornClause;
use crate::horn::HornOffset;
use crate::horn::HornShift;
use crate::horn::ModelValue;
use crate::horn::SaturationBound;
use crate::level::Level;
use crate::level::LevelVar;
use crate::order::Strictness;
use crate::poset::Derivation;
use crate::poset::LandmarkPoset;
use crate::poset::PosetError;
use crate::poset::PosetEvidenceError;
use crate::poset::replay_derivation;

/// Whether an entailment dichotomy is the positive branch.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntailmentHolds(bool);

impl From<bool> for EntailmentHolds
{
    #[inline]
    fn from(holds: bool) -> Self
    {
        Self(holds)
    }
}

impl From<EntailmentHolds> for bool
{
    #[inline]
    fn from(holds: EntailmentHolds) -> Self
    {
        holds.0
    }
}

impl Not for EntailmentHolds
{
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output
    {
        Self::from(!bool::from(self))
    }
}

/// The length of the derivation-log prefix needed to cover a query.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WitnessPrefixLength(usize);

impl From<usize> for WitnessPrefixLength
{
    #[inline]
    fn from(length: usize) -> Self
    {
        Self(length)
    }
}

impl From<WitnessPrefixLength> for usize
{
    #[inline]
    fn from(length: WitnessPrefixLength) -> Self
    {
        length.0
    }
}

/// The positive entailment evidence: a replayable forward derivation
/// covering every goal atom of the claim from the query seeds.
///
/// Constructed only by the oracle (fields are private); inspected through
/// the accessors and replayed by [`validate_entailment_witness`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntailmentWitness
{
    /// Whether this witnesses the strict order.
    strict: Strictness,
    /// The replayable derivation.
    derivation: Derivation,
}

impl EntailmentWitness
{
    /// Whether this witnesses the strict order (`left < right`).
    #[inline]
    #[must_use]
    pub const fn strict(&self) -> Strictness
    {
        self.strict
    }

    /// The replayable derivation.
    #[inline]
    #[must_use]
    pub const fn derivation(&self) -> &Derivation
    {
        &self.derivation
    }
}

/// The negative entailment evidence: a model of the whole query system
/// that covers the query seeds while refuting one recorded goal atom —
/// which is exactly what non-derivability means, checkably.
///
/// Constructed only by the oracle (fields are private); inspected through
/// the accessors and checked by [`validate_entailment_countermodel`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntailmentCountermodel
{
    /// Whether this refutes the strict order.
    strict: Strictness,
    /// The model's value at the bottom generator.
    bottom: ModelValue,
    /// The model's value at each in-scope variable.
    values: BTreeMap<LevelVar, ModelValue>,
    /// The refuted goal's variable; `None` names the constant (bottom)
    /// goal.
    refuted_variable: Option<LevelVar>,
    /// The refuted goal's (encoded, shifted) offset.
    refuted_offset: HornOffset,
}

impl EntailmentCountermodel
{
    /// Whether this refutes the strict order (`left < right`).
    #[inline]
    #[must_use]
    pub const fn strict(&self) -> Strictness
    {
        self.strict
    }

    /// The model's value at the bottom generator.
    #[inline]
    #[must_use]
    pub const fn bottom_value(&self) -> ModelValue
    {
        self.bottom
    }

    /// The model's value at `variable`, when in scope.
    #[inline]
    #[must_use]
    pub fn value_of(
        &self,
        variable: LevelVar,
    ) -> Option<ModelValue>
    {
        self.values.get(&variable).copied()
    }

    /// The assignments in ascending variable order.
    #[inline]
    pub fn assignments(&self) -> impl Iterator<Item = (LevelVar, ModelValue)> + '_
    {
        self.values
            .iter()
            .map(|(&variable, &value)| (variable, value))
    }

    /// The refuted goal's variable; `None` names the constant (bottom)
    /// goal.
    #[inline]
    #[must_use]
    pub const fn refuted_variable(&self) -> Option<LevelVar>
    {
        self.refuted_variable
    }

    /// The refuted goal's encoded (shifted) offset.
    #[inline]
    #[must_use]
    pub const fn refuted_offset(&self) -> HornOffset
    {
        self.refuted_offset
    }
}

/// The entailment dichotomy, as data: under an admitted poset a query
/// either holds with a derivation or is refuted by a countermodel.
#[derive(Clone, Debug)]
#[expect(
    clippy::exhaustive_enums,
    reason = "the entailment dichotomy is closed by theorem (TCS 913 Corollary 3.4 with Theorem 3.2)"
)]
pub enum Entailment
{
    /// The claim holds; the witness derivation replays.
    Holds(EntailmentWitness),
    /// The claim fails; the countermodel checks.
    Refuted(EntailmentCountermodel),
}

impl Entailment
{
    /// Whether this is the positive case.
    #[inline]
    #[must_use]
    pub fn holds(&self) -> EntailmentHolds
    {
        EntailmentHolds::from(matches!(*self, Self::Holds(_)))
    }
}

/// The deterministic query encoding shared by the oracle and both
/// validators (see the module docs).
pub struct QueryEncoding
{
    /// The query clause system's base clauses: the poset's compiled
    /// clauses, then one bottom clause per in-scope variable, ascending.
    pub clauses: Vec<HornClause>,
    /// The seeds: every in-scope variable at `0`, the right side's atoms
    /// at `offset + 1`, the bottom generator at `c_right + 1`.
    pub seed: BTreeMap<HVar, HornOffset>,
    /// The goals: the bottom (constant) goal first, then one per left
    /// atom, ascending; offsets already shifted.
    pub goals: Vec<HornAtom>,
}

/// The shared decision path of both public faces.
fn decide(
    poset: &LandmarkPoset,
    left: &Level,
    right: &Level,
    strict: Strictness,
) -> Result<Entailment, PosetError>
{
    let encoding = encode_query(poset, left, right, strict)?;
    let system = ClauseSystem {
        clauses: encoding.clauses,
        min_shift: HornShift::from(1_u128),
    };
    let maxgain = system.maxgain();
    let max_seed = encoding
        .seed
        .values()
        .copied()
        .max()
        .unwrap_or(HornOffset::ZERO);
    let scope_count = u128::try_from(encoding.seed.len()).map_err(|_error| PosetError::Overflow)?;
    let headroom = scope_count
        .checked_mul(u128::from(maxgain))
        .ok_or(PosetError::Overflow)?;
    let snap_bound = u128::from(max_seed)
        .checked_add(headroom)
        .map(SaturationBound::from)
        .ok_or(PosetError::Overflow)?;
    let seed_model: BTreeMap<HVar, ModelValue> = encoding
        .seed
        .iter()
        .map(|(&variable, &value)| (variable, ModelValue::Finite(value)))
        .collect();
    let outcome =
        horn::saturate(&system, seed_model, snap_bound).map_err(|_error| PosetError::Overflow)?;
    if outcome.diverged {
        return Err(PosetError::UnexpectedDivergence);
    }
    let uncovered = encoding.goals.iter().copied().find(|goal| {
        !outcome
            .values
            .get(&goal.variable())
            .is_some_and(|value| bool::from(value.covers(goal.offset())))
    });
    if let Some(goal) = uncovered {
        let goal_var = goal.variable();
        let goal_offset = goal.offset();
        let bottom = outcome
            .values
            .get(&HVar::Bottom)
            .copied()
            .unwrap_or(ModelValue::Finite(HornOffset::ZERO));
        let values: BTreeMap<LevelVar, ModelValue> = outcome
            .values
            .iter()
            .filter_map(|(&variable, &value)| match variable {
                | HVar::Var(level_var) => Some((level_var, value)),
                | HVar::Bottom => None,
            })
            .collect();
        let refuted_variable = match goal_var {
            | HVar::Bottom => None,
            | HVar::Var(level_var) => Some(level_var),
        };
        return Ok(Entailment::Refuted(EntailmentCountermodel {
            strict,
            bottom,
            values,
            refuted_variable,
            refuted_offset: goal_offset,
        }));
    }
    let cutoff = witness_cutoff(
        &system.clauses,
        &encoding.seed,
        &encoding.goals,
        &outcome.log,
    )
    .ok_or(PosetError::EvidenceIncomplete)?;
    let trimmed: Vec<FiringLogStep> = outcome.log.into_iter().take(usize::from(cutoff)).collect();
    Ok(Entailment::Holds(EntailmentWitness {
        strict,
        derivation: Derivation::from_log(trimmed),
    }))
}

impl LandmarkPoset
{
    /// Decide `left ≤ right` under this poset's hypotheses, returning
    /// checkable evidence either way.
    ///
    /// # Contract
    /// - requires: nothing beyond canonical inputs (guaranteed by
    ///   construction); queries may mention any variables, declared or not.
    /// - ensures: [`Entailment::Holds`] with a witness passing
    ///   [`validate_entailment_witness`] exactly when every goal atom of the
    ///   encoded claim is derivable (Theorem 2.2's characterization of
    ///   provability from the hypotheses); [`Entailment::Refuted`] with a
    ///   countermodel passing [`validate_entailment_countermodel`] otherwise.
    ///   With no declared constraints this agrees with [`Level::leq`] on every
    ///   input (the slice-2 acceptance gate).
    /// - provides: the kernel's hypothesis-aware level order (design record §4,
    ///   slice 2).
    /// - fails: [`PosetError::Overflow`] past the representable range;
    ///   [`PosetError::UnexpectedDivergence`] /
    ///   [`PosetError::EvidenceIncomplete`] only under a defect the theory
    ///   excludes (surfaced rather than trusted).
    /// - panics: none.
    /// - intension: the countermodel names the first uncovered goal in encoding
    ///   order (the constant goal first, then ascending variables); the witness
    ///   derivation is the engine's firing log truncated at the step that
    ///   covers the last outstanding goal.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the empty-poset property differential against the
    ///   slice-1 oracle (exact agreement, both strictness modes) plus evidence
    ///   validation on every decided query; the L3 residues are the hypothesis
    ///   paths (landmark order used, strict-vs-nonstrict split, the §5.3
    ///   non-total-order refusal), each pinned by a unit golden.
    /// - witness: `poset_differential` (the integration suite)
    /// - witness: `entail::tests::landmark_order_is_entailed`
    /// - witness: `entail::tests::strictness_needs_a_strict_hypothesis`
    /// - witness: `entail::tests::non_total_order_is_refused_with_a_countermodel`
    ///
    /// # Errors
    /// [`PosetError::Overflow`], [`PosetError::UnexpectedDivergence`],
    /// [`PosetError::EvidenceIncomplete`].
    #[inline]
    pub fn entails_leq_with_evidence(
        &self,
        left: &Level,
        right: &Level,
    ) -> Result<Entailment, PosetError>
    {
        decide(self, left, right, Strictness::NON_STRICT)
    }

    /// Decide the strict order `left < right` under this poset's
    /// hypotheses, returning checkable evidence either way.
    ///
    /// # Contract
    /// - requires: nothing beyond canonical inputs.
    /// - ensures: agrees with `left.succ()` entailed below `right` whenever the
    ///   successor is representable, and is total even where it is not (the
    ///   shift happens in the wide encoding); on an admitted poset `left <
    ///   left` is always refuted — admission's consistency certificate is what
    ///   keeps the universe rule's irreflexivity (ADR-78) under hypotheses.
    /// - provides: the hypothesis-aware consistency-bearing comparison.
    /// - fails, panics, intension: as [`Self::entails_leq_with_evidence`].
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the empty-poset differential plus the `lt ≡ leq ∘
    ///   succ` law under a fixed poset; the L3 residue is hypothesis-strictness
    ///   (`x < y` needs `x+1 ≤ y` declared, not `x ≤ y`), pinned by the
    ///   strictness golden.
    /// - witness: `poset_differential` (the integration suite)
    /// - witness: `entail::tests::strictness_needs_a_strict_hypothesis`
    ///
    /// # Errors
    /// As [`Self::entails_leq_with_evidence`].
    #[inline]
    pub fn entails_lt_with_evidence(
        &self,
        left: &Level,
        right: &Level,
    ) -> Result<Entailment, PosetError>
    {
        decide(self, left, right, Strictness::STRICT)
    }

    /// The boolean face of [`Self::entails_leq_with_evidence`].
    ///
    /// # Contract
    /// - ensures: `Ok(true)` iff the evidence face returns
    ///   [`Entailment::Holds`].
    ///
    /// # Errors
    /// As [`Self::entails_leq_with_evidence`].
    #[inline]
    pub fn entails_leq(
        &self,
        left: &Level,
        right: &Level,
    ) -> Result<EntailmentHolds, PosetError>
    {
        let evidence = self.entails_leq_with_evidence(left, right)?;
        Ok(evidence.holds())
    }

    /// The boolean face of [`Self::entails_lt_with_evidence`].
    ///
    /// # Contract
    /// - ensures: `Ok(true)` iff the evidence face returns
    ///   [`Entailment::Holds`].
    ///
    /// # Errors
    /// As [`Self::entails_lt_with_evidence`].
    #[inline]
    pub fn entails_lt(
        &self,
        left: &Level,
        right: &Level,
    ) -> Result<EntailmentHolds, PosetError>
    {
        let evidence = self.entails_lt_with_evidence(left, right)?;
        Ok(evidence.holds())
    }
}

/// Check an entailment witness against its claim by replaying its
/// derivation over the recomputed query encoding.
///
/// # Contract
/// - requires: `witness` claims `left ≤ right` under `poset` (`left < right`
///   when its strict flag holds); the claim direction is read from the witness.
///   The encoding is definitional and shared with the oracle — the replay is
///   the trusted step.
/// - ensures: `Ok(())` exactly when the derivation replays from the query seeds
///   (every step a known clause at an admissible shift with its body available)
///   to cover every goal atom of the claim.
/// - provides: the trusted half of the positive entailment evidence.
/// - fails: the first [`PosetEvidenceError`] encountered — replay errors in
///   step order, then uncovered goals in encoding order.
/// - panics: none.
///
/// # Errors
/// The replay errors and [`PosetEvidenceError::TargetUncovered`];
/// [`PosetEvidenceError::Overflow`] on adversarial arithmetic.
///
/// # Adequacy
/// - hypothesis: L3 — each rejection arm pinned by a hand-perturbed witness
///   asserting the exact variant, acceptance pinned by the property that every
///   oracle witness validates.
/// - witness: `entail::tests::perturbed_witness_arms_are_rejected`
/// - witness: `poset_differential` (evidence-validation properties)
#[inline]
pub fn validate_entailment_witness(
    poset: &LandmarkPoset,
    left: &Level,
    right: &Level,
    witness: &EntailmentWitness,
) -> Result<(), PosetEvidenceError>
{
    let encoding = encode_query(poset, left, right, witness.strict())
        .map_err(|_error| PosetEvidenceError::Overflow)?;
    let maxima = replay_derivation(
        &encoding.clauses,
        HornShift::from(1_u128),
        encoding.seed,
        witness.derivation(),
    )?;
    for &goal in &encoding.goals {
        let covered = maxima
            .get(&goal.variable())
            .is_some_and(|&maximum| goal.offset() <= maximum);
        if !covered {
            let uncovered = match goal.variable() {
                | HVar::Bottom => None,
                | HVar::Var(level_var) => Some(level_var),
            };
            return Err(PosetEvidenceError::TargetUncovered {
                variable: uncovered,
            });
        }
    }
    Ok(())
}

/// Check an entailment countermodel against its claim.
///
/// The model must satisfy the whole (recomputed) query system, cover the
/// query seeds, and refute its recorded goal — which soundly witnesses
/// non-derivability.
///
/// # Contract
/// - requires: `countermodel` claims `left ≤ right` fails under `poset`
///   (strictly, when its strict flag holds); the claim direction is read from
///   the countermodel.
/// - ensures: `Ok(())` exactly when the model assigns every in-scope variable,
///   covers every seed atom, satisfies every clause family of the query system
///   (the Lemma 3.1 check per base clause), and its recorded refuted goal is a
///   goal of the claim that the model does not cover.
/// - provides: the trusted half of the negative entailment evidence,
///   independent of the minimal-model computation (it only checks
///   satisfaction).
/// - fails: the first [`PosetEvidenceError`] encountered — missing values, then
///   seeds, then clauses in index order, then the refuted goal.
/// - panics: none.
///
/// # Errors
/// [`PosetEvidenceError::MissingValue`],
/// [`PosetEvidenceError::SeedUnsatisfied`],
/// [`PosetEvidenceError::ClauseUnsatisfied`],
/// [`PosetEvidenceError::NotRefuting`], and
/// [`PosetEvidenceError::Overflow`].
///
/// # Adequacy
/// - hypothesis: L3 — each rejection arm pinned by a hand-perturbed
///   countermodel asserting the exact variant, acceptance pinned by the
///   property that every oracle countermodel validates.
/// - witness: `entail::tests::perturbed_countermodel_arms_are_rejected`
/// - witness: `poset_differential` (evidence-validation properties)
#[inline]
pub fn validate_entailment_countermodel(
    poset: &LandmarkPoset,
    left: &Level,
    right: &Level,
    countermodel: &EntailmentCountermodel,
) -> Result<(), PosetEvidenceError>
{
    let encoding = encode_query(poset, left, right, countermodel.strict())
        .map_err(|_error| PosetEvidenceError::Overflow)?;
    let mut model: BTreeMap<HVar, ModelValue> = BTreeMap::new();
    let _bottom_slot = model.insert(HVar::Bottom, countermodel.bottom_value());
    for &variable in encoding.seed.keys() {
        let HVar::Var(level_var) = variable
        else {
            continue;
        };
        let value = countermodel
            .value_of(level_var)
            .ok_or(PosetEvidenceError::MissingValue {
                variable: level_var,
            })?;
        let _variable_slot = model.insert(variable, value);
    }
    for (&variable, &seeded) in &encoding.seed {
        let covered = model
            .get(&variable)
            .is_some_and(|value| bool::from(value.covers(seeded)));
        if !covered {
            let uncovered = match variable {
                | HVar::Bottom => None,
                | HVar::Var(level_var) => Some(level_var),
            };
            return Err(PosetEvidenceError::SeedUnsatisfied {
                variable: uncovered,
            });
        }
    }
    for (index, clause) in encoding.clauses.iter().enumerate() {
        let maybe_firing = horn::fire(clause, &model, HornShift::from(1_u128))
            .map_err(|_error| PosetEvidenceError::Overflow)?;
        let Some(firing) = maybe_firing
        else {
            continue;
        };
        let head = clause.head();
        let head_var = head.variable();
        let satisfied = match (model.get(&head_var), firing.value) {
            | (Some(&ModelValue::Infinite), _) => true,
            | (Some(&ModelValue::Finite(existing)), ModelValue::Finite(forced)) => {
                forced <= existing
            },
            | (Some(&ModelValue::Finite(_)), ModelValue::Infinite) | (None, _) => false,
        };
        if !satisfied {
            return Err(PosetEvidenceError::ClauseUnsatisfied {
                clause: ClauseIndex::from(index),
            });
        }
    }
    let goal = HornAtom::new(
        countermodel
            .refuted_variable()
            .map_or(HVar::Bottom, HVar::Var),
        countermodel.refuted_offset(),
    );
    if !encoding.goals.contains(&goal) {
        return Err(PosetEvidenceError::NotRefuting);
    }
    let refuted = !model
        .get(&goal.variable())
        .is_some_and(|value| bool::from(value.covers(goal.offset())));
    if refuted {
        Ok(())
    }
    else {
        Err(PosetEvidenceError::NotRefuting)
    }
}

/// Build the query encoding for `left ≤ right` (strict when `strict`).
pub fn encode_query(
    poset: &LandmarkPoset,
    left: &Level,
    right: &Level,
    strict: Strictness,
) -> Result<QueryEncoding, PosetError>
{
    let goal_shift = HornOffset::ONE
        .checked_add(HornOffset::from(u128::from(bool::from(strict))))
        .map_err(|_error| PosetError::Overflow)?;
    let mut scope: BTreeSet<LevelVar> = poset.variables().clone();
    scope.extend(left.atoms().map(|(variable, _offset)| variable));
    scope.extend(right.atoms().map(|(variable, _offset)| variable));
    let mut clauses = poset.compiled_clauses().to_vec();
    for &variable in &scope {
        if let Some(clause) = HornClause::new(
            &[HornAtom::new(HVar::Var(variable), HornOffset::ZERO)],
            HornAtom::new(HVar::Bottom, HornOffset::ZERO),
        ) {
            clauses.push(clause);
        }
    }
    let mut seed: BTreeMap<HVar, HornOffset> = scope
        .iter()
        .map(|&variable| (HVar::Var(variable), HornOffset::ZERO))
        .collect();
    for (variable, offset) in right.atoms() {
        let seeded = HornOffset::from(offset)
            .checked_add(HornOffset::ONE)
            .map_err(|_error| PosetError::Overflow)?;
        let _previous = seed.insert(HVar::Var(variable), seeded);
    }
    let bottom_seed = HornOffset::from(right.constant_part())
        .checked_add(HornOffset::ONE)
        .map_err(|_error| PosetError::Overflow)?;
    let _previous = seed.insert(HVar::Bottom, bottom_seed);
    let mut goals = Vec::new();
    let bottom_goal = HornOffset::from(left.constant_part())
        .checked_add(goal_shift)
        .map_err(|_error| PosetError::Overflow)?;
    goals.push(HornAtom::new(HVar::Bottom, bottom_goal));
    for (variable, offset) in left.atoms() {
        let goal = HornOffset::from(offset)
            .checked_add(goal_shift)
            .map_err(|_error| PosetError::Overflow)?;
        goals.push(HornAtom::new(HVar::Var(variable), goal));
    }
    Ok(QueryEncoding {
        clauses,
        seed,
        goals,
    })
}

/// The shortest log prefix whose replay covers every goal, or `None` when
/// even the full log does not (excluded when the saturated model covers
/// the goals, since the log replays to that model).
fn witness_cutoff(
    clauses: &[HornClause],
    seed: &BTreeMap<HVar, HornOffset>,
    goals: &[HornAtom],
    log: &[FiringLogStep],
) -> Option<WitnessPrefixLength>
{
    let mut maxima = seed.clone();
    let covered = |state: &BTreeMap<HVar, HornOffset>, goal: HornAtom| {
        state
            .get(&goal.variable())
            .is_some_and(|&maximum| goal.offset() <= maximum)
    };
    let mut remaining: Vec<HornAtom> = goals
        .iter()
        .copied()
        .filter(|&goal| !covered(&maxima, goal))
        .collect();
    if remaining.is_empty() {
        return Some(WitnessPrefixLength::from(0_usize));
    }
    for (position, &step) in log.iter().enumerate() {
        let clause = clauses.get(usize::from(step.clause()))?;
        let head = clause.head();
        let concluded = head.offset().checked_add_shift(step.shift()).ok()?;
        let maximum = maxima.entry(head.variable()).or_insert(concluded);
        *maximum = (*maximum).max(concluded);
        remaining.retain(|&goal| !covered(&maxima, goal));
        if remaining.is_empty() {
            return position.checked_add(1_usize).map(WitnessPrefixLength::from);
        }
    }
    None
}

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::Entailment;
    use super::EntailmentCountermodel;
    use super::EntailmentWitness;
    use super::validate_entailment_countermodel;
    use super::validate_entailment_witness;
    use crate::horn::ClauseIndex;
    use crate::horn::FiringLogStep;
    use crate::horn::HornOffset;
    use crate::horn::HornShift;
    use crate::horn::ModelValue;
    use crate::level::Level;
    use crate::level::LevelConstant;
    use crate::level::LevelOffset;
    use crate::level::LevelVar;
    use crate::level::LevelVarIndex;
    use crate::order::Strictness;
    use crate::poset::AdmissionOutcome;
    use crate::poset::Derivation;
    use crate::poset::LandmarkConstraint;
    use crate::poset::LandmarkPoset;
    use crate::poset::PosetEvidenceError;

    #[test]
    fn constants_cross_the_bottom_encoding()
    {
        // The encoding-soundness pins: the constant-vs-atom bound that a
        // passively pinned bottom would lose.
        let poset = empty_poset();
        let _witness = holds(
            &poset,
            &Level::constant(LevelConstant::from(3_u64)),
            &var_plus(0, 3),
        );
        let _countermodel = refuted(
            &poset,
            &Level::constant(LevelConstant::from(4_u64)),
            &var_plus(0, 3),
        );
        let _reflexive = holds(
            &poset,
            &Level::constant(LevelConstant::from(5_u64)),
            &Level::constant(LevelConstant::from(5_u64)),
        );
    }

    #[test]
    fn landmark_order_is_entailed()
    {
        let poset = leq_poset();
        let witness = holds(&poset, &var_plus(0, 0), &var_plus(1, 0));
        assert!(
            !witness.derivation().steps().is_empty(),
            "x ≤ y under the hypothesis needs a genuine derivation step"
        );
        // Beyond slice 1: the free oracle refutes exactly this query.
        assert!(
            !bool::from(var_plus(0, 0).leq(&var_plus(1, 0))),
            "the free fragment cannot see the hypothesis"
        );
    }

    #[test]
    fn shifted_landmark_order_is_entailed()
    {
        let poset = leq_poset();
        let _witness = holds(&poset, &var_plus(0, 3), &var_plus(1, 3));
        let countermodel = refuted(&poset, &var_plus(1, 0), &var_plus(0, 0));
        assert_eq!(
            countermodel.refuted_variable(),
            Some(var(1)),
            "the reverse direction is refuted at its variable goal"
        );
    }

    #[test]
    fn strictness_needs_a_strict_hypothesis()
    {
        let weak = leq_poset();
        let strict_claim = weak
            .entails_lt_with_evidence(&var_plus(0, 0), &var_plus(1, 0))
            .unwrap();
        assert!(
            !bool::from(strict_claim.holds()),
            "x ≤ y does not entail x < y"
        );
        let strong = admitted(vec![
            LandmarkConstraint::leq(var_plus(0, 1), var_plus(1, 0)).unwrap(),
        ]);
        let strict_holds = strong
            .entails_lt_with_evidence(&var_plus(0, 0), &var_plus(1, 0))
            .unwrap();
        assert!(bool::from(strict_holds.holds()), "x+1 ≤ y entails x < y");
    }

    #[test]
    fn non_total_order_is_refused_with_a_countermodel()
    {
        // TCS 913 §5.3: x ∨ y = x+1 does not entail y = x+1 (levels are
        // not totally ordered), though y ≤ x+1 does follow.
        let poset = admitted(vec![
            LandmarkConstraint::equal(var_plus(0, 0).max(&var_plus(1, 0)), var_plus(0, 1)).unwrap(),
        ]);
        let _forward = holds(&poset, &var_plus(1, 0), &var_plus(0, 1));
        let _countermodel = refuted(&poset, &var_plus(0, 1), &var_plus(1, 0));
    }

    #[test]
    fn hypotheses_reach_constant_goals()
    {
        // Under x ≤ y, the value of max-at-zero flows through the
        // hypothesis: 3 ≤ y+3 needs no landmark, but 1 ≤ y follows only
        // if something seeds y — it does not, so it is refuted; while
        // 0 ≤ y holds trivially.
        let poset = leq_poset();
        let _trivial = holds(&poset, &Level::zero(), &var_plus(1, 0));
        let _refused = refuted(
            &poset,
            &Level::constant(LevelConstant::from(1_u64)),
            &var_plus(0, 0),
        );
    }

    #[test]
    fn perturbed_witness_arms_are_rejected()
    {
        let poset = leq_poset();
        let left = var_plus(0, 0);
        let right = var_plus(1, 0);
        let genuine = holds(&poset, &left, &right);

        let unknown = EntailmentWitness {
            strict: Strictness::NON_STRICT,
            derivation: Derivation::from_log(vec![FiringLogStep::new(
                ClauseIndex::from(99_usize),
                HornShift::from(1_u128),
            )]),
        };
        assert_eq!(
            Err(PosetEvidenceError::UnknownClause {
                clause: ClauseIndex::from(99_usize),
            }),
            validate_entailment_witness(&poset, &left, &right, &unknown),
            "an out-of-range clause index is rejected"
        );

        let below_minimum = EntailmentWitness {
            strict: Strictness::NON_STRICT,
            derivation: Derivation::from_log(vec![FiringLogStep::new(
                ClauseIndex::from(0_usize),
                HornShift::from(0_u128),
            )]),
        };
        assert_eq!(
            Err(PosetEvidenceError::ShiftBelowMinimum {
                clause: ClauseIndex::from(0_usize),
            }),
            validate_entailment_witness(&poset, &left, &right, &below_minimum),
            "a step below the query system's minimum shift is rejected"
        );

        let starved = EntailmentWitness {
            strict: Strictness::NON_STRICT,
            derivation: Derivation::from_log(vec![FiringLogStep::new(
                ClauseIndex::from(0_usize),
                HornShift::from(9_u128),
            )]),
        };
        assert_eq!(
            Err(PosetEvidenceError::BodyUnavailable {
                clause: ClauseIndex::from(0_usize),
            }),
            validate_entailment_witness(&poset, &left, &right, &starved),
            "a step firing above its available body is rejected"
        );

        let empty = EntailmentWitness {
            strict: Strictness::NON_STRICT,
            derivation: Derivation::from_log(vec![]),
        };
        assert_eq!(
            validate_entailment_witness(&poset, &left, &right, &empty),
            Err(PosetEvidenceError::TargetUncovered {
                variable: Some(var(0)),
            }),
            "an empty derivation leaves the variable goal uncovered"
        );

        drop(genuine);
    }

    #[test]
    fn perturbed_countermodel_arms_are_rejected()
    {
        let poset = leq_poset();
        let left = var_plus(1, 0);
        let right = var_plus(0, 0);
        let genuine = refuted(&poset, &left, &right);

        let mut missing = genuine.clone();
        let _removed = missing.values.remove(&var(1));
        assert_eq!(
            validate_entailment_countermodel(&poset, &left, &right, &missing),
            Err(PosetEvidenceError::MissingValue { variable: var(1) }),
            "an unassigned in-scope variable is rejected"
        );

        let mut starving = genuine.clone();
        let _starved_slot = starving
            .values
            .insert(var(0), ModelValue::Finite(HornOffset::from(0_u128)));
        assert_eq!(
            validate_entailment_countermodel(&poset, &left, &right, &starving),
            Err(PosetEvidenceError::SeedUnsatisfied {
                variable: Some(var(0)),
            }),
            "a model below the query seeds is rejected"
        );

        // x ≤ y compiles to the clause {y} → x (the query system's clause
        // 0), so raising y's value while x stays low violates it.
        let mut violating = genuine.clone();
        let _violated_slot = violating
            .values
            .insert(var(1), ModelValue::Finite(HornOffset::from(9_u128)));
        assert_eq!(
            Err(PosetEvidenceError::ClauseUnsatisfied {
                clause: ClauseIndex::from(0_usize),
            }),
            validate_entailment_countermodel(&poset, &left, &right, &violating),
            "a model violating a hypothesis clause is rejected"
        );

        let mut misdirected = genuine.clone();
        misdirected.refuted_variable = Some(var(0));
        assert_eq!(
            Err(PosetEvidenceError::NotRefuting),
            validate_entailment_countermodel(&poset, &left, &right, &misdirected),
            "a recorded goal outside the claim's goals is rejected"
        );

        let mut satisfied = genuine;
        let mut saturated_values: BTreeMap<LevelVar, ModelValue> = BTreeMap::new();
        let _p0 = saturated_values.insert(var(0), ModelValue::Infinite);
        let _p1 = saturated_values.insert(var(1), ModelValue::Infinite);
        satisfied.values = saturated_values;
        satisfied.bottom = ModelValue::Infinite;
        assert_eq!(
            Err(PosetEvidenceError::NotRefuting),
            validate_entailment_countermodel(&poset, &left, &right, &satisfied),
            "a model that covers the recorded goal refutes nothing"
        );

        // Overflow needs a clause with a positive head offset: under
        // x+1 ≤ y the clause {y} → x+1 fires at shift u128::MAX when the
        // model drives y that high, and 1 + u128::MAX overflows.
        let strong = admitted(vec![
            LandmarkConstraint::leq(var_plus(0, 1), var_plus(1, 0)).unwrap(),
        ]);
        let strong_cm = refuted(&strong, &var_plus(1, 0), &var_plus(0, 0));
        let mut overflowing = strong_cm;
        let _overflow_slot = overflowing
            .values
            .insert(var(1), ModelValue::Finite(HornOffset::from(u128::MAX)));
        let outcome = validate_entailment_countermodel(
            &strong,
            &var_plus(1, 0),
            &var_plus(0, 0),
            &overflowing,
        );
        assert_eq!(
            Err(PosetEvidenceError::Overflow),
            outcome,
            "adversarial magnitudes are rejected, never wrapped"
        );
    }

    #[test]
    fn queries_mentioning_undeclared_variables_work()
    {
        let poset = leq_poset();
        let _witness = holds(&poset, &var_plus(7, 0), &var_plus(7, 2));
        let _countermodel = refuted(&poset, &var_plus(7, 0), &var_plus(8, 0));
    }

    #[test]
    fn lt_equals_succ_leq_under_hypotheses()
    {
        let poset = leq_poset();
        let pairs = [
            (var_plus(0, 0), var_plus(1, 0)),
            (var_plus(0, 0), var_plus(1, 2)),
            (var_plus(0, 2), var_plus(1, 1)),
            (Level::constant(LevelConstant::from(2_u64)), var_plus(1, 3)),
        ];
        for (left, right) in pairs {
            let via_lt = poset.entails_lt(&left, &right).unwrap();
            let via_succ = poset.entails_leq(&left.succ().unwrap(), &right).unwrap();
            assert_eq!(
                via_lt, via_succ,
                "lt must equal leq ∘ succ ({left:?}, {right:?})"
            );
        }
    }

    #[test]
    fn entailment_is_irreflexive_for_lt_on_admitted_posets()
    {
        let poset = leq_poset();
        let composite = var_plus(0, 2).max(&var_plus(1, 5));
        assert!(
            !bool::from(poset.entails_lt(&composite, &composite).unwrap()),
            "lt stays irreflexive under admitted hypotheses (ADR-78)"
        );
    }

    use support::admitted;
    use support::empty_poset;
    use support::holds;
    use support::leq_poset;
    use support::refuted;
    use support::var;
    use support::var_plus;

    mod support
    {
        use super::*;

        /// The empty poset (the slice-1 degeneration).
        pub(super) fn empty_poset() -> LandmarkPoset
        {
            admitted(vec![])
        }

        /// A poset declaring `x ≤ y` over variables `0, 1`.
        pub(super) fn leq_poset() -> LandmarkPoset
        {
            admitted(vec![
                LandmarkConstraint::leq(var_plus(0, 0), var_plus(1, 0)).unwrap(),
            ])
        }

        /// Admit a constraint set that must admit.
        pub(super) fn admitted(constraints: Vec<LandmarkConstraint>) -> LandmarkPoset
        {
            match LandmarkPoset::admit(constraints).unwrap() {
                | AdmissionOutcome::Admitted(poset) => poset,
                | AdmissionOutcome::Loop(_witness) => panic!("expected admission"),
            }
        }

        /// `var + n` built through the public constructors.
        pub(super) fn var_plus(
            index: impl Into<LevelVarIndex>,
            offset: impl Into<LevelOffset>,
        ) -> Level
        {
            let offset = offset.into();
            let mut level = Level::var(var(index));
            for _step in 0_u64 .. u64::from(offset) {
                level = level.succ().unwrap();
            }
            level
        }

        /// A test variable by index.
        pub(super) fn var(index: impl Into<LevelVarIndex>) -> LevelVar
        {
            LevelVar::new(index.into())
        }

        /// Decide and unwrap the positive case, validating the witness.
        pub(super) fn holds(
            poset: &LandmarkPoset,
            left: &Level,
            right: &Level,
        ) -> EntailmentWitness
        {
            match poset.entails_leq_with_evidence(left, right).unwrap() {
                | Entailment::Holds(witness) => {
                    assert_eq!(
                        Ok(()),
                        validate_entailment_witness(poset, left, right, &witness),
                        "the witness must validate"
                    );
                    witness
                },
                | Entailment::Refuted(_countermodel) => panic!("expected entailment"),
            }
        }

        /// Decide and unwrap the negative case, validating the countermodel.
        pub(super) fn refuted(
            poset: &LandmarkPoset,
            left: &Level,
            right: &Level,
        ) -> EntailmentCountermodel
        {
            match poset.entails_leq_with_evidence(left, right).unwrap() {
                | Entailment::Refuted(countermodel) => {
                    assert_eq!(
                        Ok(()),
                        validate_entailment_countermodel(poset, left, right, &countermodel),
                        "the countermodel must validate"
                    );
                    countermodel
                },
                | Entailment::Holds(_witness) => panic!("expected refusal"),
            }
        }
    }
}
