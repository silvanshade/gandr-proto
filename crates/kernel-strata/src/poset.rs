//! The landmark poset (kernel-boundary design record §4, slice 2): a fixed,
//! declared set of order constraints over level variables, admitted into the
//! kernel by Bezem–Coquand loop-checking (TCS 913, 2022, Corollary 3.5) with
//! checkable evidence either way — a [`ConsistencyWitness`] (an explicit
//! `ℕ`-homomorphism, validated by evaluating every constraint under it) on
//! admission, or a [`LoopWitness`] (a replayable pumping derivation) on
//! refusal. Exactly one of the two exists (the paper's dichotomy), which is
//! what makes admission consistency-bearing: an admitted poset has an `ℕ`
//! model, so `U_l : U_l` stays underivable under its hypotheses.
//!
//! # The constraint language is variable-only (recorded API restriction)
//!
//! Declared constraint sides must be **variable-only levels**: canonical
//! constant part `0` and at least one atom. The paper's algebra is
//! deliberately constant-free (§5.1), and gandr carries query constants
//! across that gap with a pinned bottom generator `⊥` (a constant `c`
//! becomes the atom `⊥ + c`), ordered below every in-scope variable by
//! bottom clauses `x → ⊥` added at query time (see [`crate::entail`]).
//! That encoding is sound and conservative **because** constraints are
//! variable-only:
//!
//! * soundness — `⊥ ≤ x` proves `⊥+k ≤ x+k` by the monotone endomorphism, which
//!   is exactly the bottom clauses' shift family; any semilattice model of the
//!   constraints extends over the freely adjoined bottom;
//! * conservativity — bottom atoms are derivation dead-ends (no declared clause
//!   has `⊥` in its body, because no declared constraint may mention a
//!   constant), so a derivation of a `⊥`-free goal never needs them and
//!   restricts to the declared system;
//! * loop-immunity — for the same reason `⊥` cannot participate in a cycle, so
//!   admission is decided on the declared constraints alone.
//!
//! Allowing constants in declared constraints (say `α ≥ 5`) would put `⊥`
//! in clause bodies and break all three arguments at once; if that is ever
//! wanted it needs its own design pass, not a lifted restriction.
//!
//! # Determinism of compiled clauses (evidence index vocabulary)
//!
//! Evidence refers to clauses by index into one documented, deterministic
//! compilation (TCS 913 §2, `S_{s=t}`), so the indices mean the same thing
//! to the oracle, the validators, and a human reviewer:
//!
//! * constraints compile in declaration order;
//! * `left ≤ right` compiles to `atoms(right) → a` for each atom `a` of `left`
//!   in ascending variable order (the equation `left ∨ right = right`'s
//!   non-trivial family);
//! * `left = right` compiles to `atoms(left) → b` for each atom `b` of `right`
//!   in ascending variable order, then `atoms(right) → a` for each atom `a` of
//!   `left`;
//! * self-subsumed clauses (conclusion dominated by a same-variable body atom)
//!   are omitted — every model satisfies them vacuously;
//! * query systems append one bottom clause `x → ⊥` per in-scope variable in
//!   ascending order after the constraint clauses ([`crate::entail`]).

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use core::error::Error;
use core::fmt;

use crate::horn;
use crate::horn::ClauseIndex;
use crate::horn::ClauseSystem;
use crate::horn::DerivationRoundLimit;
use crate::horn::FiringLogStep;
use crate::horn::HVar;
use crate::horn::HornAtom;
use crate::horn::HornClause;
use crate::horn::HornOffset;
use crate::horn::HornShift;
use crate::horn::MaxGain;
use crate::horn::ModelValue;
use crate::horn::SaturationBound;
use crate::level::Level;
use crate::level::LevelConstant;
use crate::level::LevelVar;

/// A natural value in a consistency homomorphism certificate.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ConsistencyValue(u128);

impl From<u128> for ConsistencyValue
{
    #[inline]
    fn from(value: u128) -> Self
    {
        Self(value)
    }
}

impl From<ConsistencyValue> for u128
{
    #[inline]
    fn from(value: ConsistencyValue) -> Self
    {
        value.0
    }
}

/// The declared relation of a [`LandmarkConstraint`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintRelation
{
    /// `left ≤ right`.
    Leq,
    /// `left = right`.
    Eq,
}

/// Failures of poset construction and the decision procedures.
///
/// The vocabulary is closed and total: every operation either succeeds,
/// returns its negative evidence, or surfaces one of these — the kernel
/// never panics on poset data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PosetError
{
    /// A declared constraint side mentions a constant (nonzero canonical
    /// constant part, or no atoms at all) — outside the variable-only
    /// restriction this module's docs record.
    ConstantInConstraint,
    /// Arithmetic stepped past the representable range. Saturating instead
    /// would silently identify distinct judgments — a soundness bug — so
    /// the overflow surfaces as a typed error.
    Overflow,
    /// A query's model computation diverged on an admitted poset. Excluded
    /// by admission (Corollary 3.5: an admitted poset has no loop, and a
    /// divergent component implies one), so this is reachable only under a
    /// defect in admission itself; it is surfaced rather than trusted.
    UnexpectedDivergence,
    /// Evidence extraction hit its round limit before covering its targets.
    /// Excluded by the theory (the targets are derivable whenever this path
    /// runs — Corollary 3.5's claim); surfaced rather than trusted.
    EvidenceIncomplete,
}

impl fmt::Display for PosetError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::ConstantInConstraint => {
                f.write_str("a declared constraint side mentions a constant")
            },
            | Self::Overflow => {
                f.write_str("poset arithmetic stepped past the representable range")
            },
            | Self::UnexpectedDivergence => {
                f.write_str("a query diverged on an admitted poset (excluded by admission)")
            },
            | Self::EvidenceIncomplete => {
                f.write_str("evidence extraction missed its targets (excluded by the theory)")
            },
        }
    }
}

impl Error for PosetError
{
}

/// One declared landmark constraint: `left REL right` over variable-only
/// levels.
///
/// Constructed only through [`Self::leq`] / [`Self::equal`], which enforce the
/// variable-only restriction, so an ill-formed constraint is
/// unrepresentable (K1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LandmarkConstraint
{
    /// The left side (variable-only).
    left: Level,
    /// The declared relation.
    relation: ConstraintRelation,
    /// The right side (variable-only).
    right: Level,
}

impl LandmarkConstraint
{
    /// Declare `left ≤ right`.
    ///
    /// # Contract
    /// - requires: both sides variable-only — canonical constant part `0` and
    ///   at least one atom (the module docs record why).
    /// - ensures: a well-formed constraint carrying the sides verbatim.
    /// - fails: [`PosetError::ConstantInConstraint`] otherwise.
    /// - panics: none.
    ///
    /// # Errors
    /// [`PosetError::ConstantInConstraint`] — a side mentions a constant.
    #[inline]
    pub fn leq(
        left: Level,
        right: Level,
    ) -> Result<Self, PosetError>
    {
        Self::new(left, ConstraintRelation::Leq, right)
    }

    /// Declare `left = right`.
    ///
    /// # Contract
    /// - requires: both sides variable-only — canonical constant part `0` and
    ///   at least one atom (the module docs record why).
    /// - ensures: a well-formed constraint carrying the sides verbatim.
    /// - fails: [`PosetError::ConstantInConstraint`] otherwise.
    /// - panics: none.
    ///
    /// # Errors
    /// [`PosetError::ConstantInConstraint`] — a side mentions a constant.
    #[inline]
    pub fn equal(
        left: Level,
        right: Level,
    ) -> Result<Self, PosetError>
    {
        Self::new(left, ConstraintRelation::Eq, right)
    }

    /// The shared checked constructor.
    fn new(
        left: Level,
        relation: ConstraintRelation,
        right: Level,
    ) -> Result<Self, PosetError>
    {
        let variable_only = |side: &Level| {
            side.constant_part() == LevelConstant::ZERO && side.atoms().next().is_some()
        };
        if variable_only(&left) && variable_only(&right) {
            Ok(Self {
                left,
                relation,
                right,
            })
        }
        else {
            Err(PosetError::ConstantInConstraint)
        }
    }

    /// The left side.
    #[inline]
    #[must_use]
    pub const fn left(&self) -> &Level
    {
        &self.left
    }

    /// The declared relation.
    #[inline]
    #[must_use]
    pub const fn relation(&self) -> ConstraintRelation
    {
        self.relation
    }

    /// The right side.
    #[inline]
    #[must_use]
    pub const fn right(&self) -> &Level
    {
        &self.right
    }

    /// The variables the constraint mentions, in ascending order.
    fn variables(&self) -> impl Iterator<Item = LevelVar> + '_
    {
        let left = self.left.atoms().map(|(variable, _offset)| variable);
        let right = self.right.atoms().map(|(variable, _offset)| variable);
        left.chain(right)
    }
}

/// One step of a replayable forward derivation: the base clause (by index
/// into the documented compilation) and the upward shift it fires at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DerivationStep
{
    /// The base clause index.
    clause: ClauseIndex,
    /// The upward shift the instance fires at.
    shift: HornShift,
}

impl DerivationStep
{
    /// Build a replayable derivation step.
    #[inline]
    #[must_use]
    pub const fn new(
        clause: ClauseIndex,
        shift: HornShift,
    ) -> Self
    {
        Self { clause, shift }
    }

    /// The base clause index.
    #[inline]
    #[must_use]
    pub const fn clause(&self) -> ClauseIndex
    {
        self.clause
    }

    /// The upward shift the instance fires at.
    #[inline]
    #[must_use]
    pub const fn shift(&self) -> HornShift
    {
        self.shift
    }
}

/// A replayable forward derivation (`⊢_H`, TCS 913 §2).
///
/// A derivation is a sequence of clause-instance applications. Predecessor
/// clauses stay implicit — an atom is available whenever the derived
/// maximum of its variable reaches its offset (the downward-closed model
/// reading).
///
/// Derivations are constructed only by the oracle (fields are private);
/// they are inspected through the accessors and replayed by the validators.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Derivation
{
    /// The steps, in application order.
    steps: Vec<DerivationStep>,
}

impl Derivation
{
    /// The steps, in application order.
    #[inline]
    #[must_use]
    pub fn steps(&self) -> &[DerivationStep]
    {
        &self.steps
    }

    /// Wrap an engine firing log.
    pub(crate) fn from_log(log: Vec<FiringLogStep>) -> Self
    {
        let steps = log
            .into_iter()
            .map(|step| DerivationStep {
                clause: step.clause(),
                shift: step.shift(),
            })
            .collect();
        Self { steps }
    }
}

/// The positive admission evidence (TCS 913 Corollary 3.5, case 2).
///
/// An explicit homomorphism into `ℕ` — one value per declared variable —
/// under which every declared constraint holds. Its existence is what
/// admission certifies; [`validate_consistency`] checks it by direct
/// evaluation.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsistencyWitness
{
    /// The homomorphism's value at each declared variable.
    values: BTreeMap<LevelVar, ConsistencyValue>,
}

impl ConsistencyWitness
{
    /// The homomorphism's value at `variable`, when declared.
    #[inline]
    #[must_use]
    pub fn value_of(
        &self,
        variable: LevelVar,
    ) -> Option<ConsistencyValue>
    {
        self.values.get(&variable).copied()
    }

    /// The assignments in ascending variable order.
    #[inline]
    pub fn assignments(&self) -> impl Iterator<Item = (LevelVar, ConsistencyValue)> + '_
    {
        self.values
            .iter()
            .map(|(&variable, &value)| (variable, value))
    }
}

/// The negative admission evidence (TCS 913 Corollary 3.5, case 1).
///
/// A nonempty variable set `W` and shift `n` such that the derivation
/// replays, from exactly the atoms `{w+n | w ∈ W}`, to cover every
/// `w+n+1` — a pumping certificate, so `∨_{w∈W} w+n` is a loop and no
/// homomorphism into `ℕ` exists.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopWitness
{
    /// The looping variable set `W`.
    members: BTreeSet<LevelVar>,
    /// The shift `n` at which the pumping is exhibited.
    shift: HornShift,
    /// The replayable pumping derivation.
    derivation: Derivation,
}

impl LoopWitness
{
    /// The looping variable set `W`, in ascending order.
    #[inline]
    #[must_use]
    pub const fn members(&self) -> &BTreeSet<LevelVar>
    {
        &self.members
    }

    /// The shift `n` at which the pumping is exhibited.
    #[inline]
    #[must_use]
    pub const fn shift(&self) -> HornShift
    {
        self.shift
    }

    /// The replayable pumping derivation.
    #[inline]
    #[must_use]
    pub const fn derivation(&self) -> &Derivation
    {
        &self.derivation
    }
}

/// Rejection vocabulary of the slice-2 evidence validators: exactly why a
/// piece of poset or entailment evidence fails to check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PosetEvidenceError
{
    /// A loop witness with an empty member set.
    EmptyLoop,
    /// A derivation step names a clause index outside the compiled system.
    UnknownClause
    {
        /// The offending step's clause index.
        clause: ClauseIndex,
    },
    /// A derivation step fires below the system's minimum shift.
    ShiftBelowMinimum
    {
        /// The offending step's clause index.
        clause: ClauseIndex,
    },
    /// A derivation step's body is not available at its shift.
    BodyUnavailable
    {
        /// The offending step's clause index.
        clause: ClauseIndex,
    },
    /// A required target atom is not covered after replay. `None` names the
    /// bottom (constant) goal, `Some` a variable goal.
    TargetUncovered
    {
        /// The uncovered target's variable.
        variable: Option<LevelVar>,
    },
    /// A consistency witness misses a declared variable.
    MissingAssignment
    {
        /// The unassigned variable.
        variable: LevelVar,
    },
    /// A declared constraint fails under the consistency witness.
    ConstraintViolated
    {
        /// The failing constraint's declaration index.
        index: usize,
    },
    /// A countermodel misses an in-scope variable.
    MissingValue
    {
        /// The unassigned variable.
        variable: LevelVar,
    },
    /// A countermodel fails to cover a query seed atom. `None` names the
    /// bottom seed, `Some` a variable seed.
    SeedUnsatisfied
    {
        /// The uncovered seed's variable.
        variable: Option<LevelVar>,
    },
    /// A countermodel violates a clause of the query system.
    ClauseUnsatisfied
    {
        /// The violated clause's index.
        clause: ClauseIndex,
    },
    /// The recorded refuted goal is not a goal of the claim, or the
    /// countermodel does not actually refute it.
    NotRefuting,
    /// Evaluating the (adversarial) evidence overflowed; the evidence is
    /// rejected rather than wrapped.
    Overflow,
}

impl fmt::Display for PosetEvidenceError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::EmptyLoop => f.write_str("the loop witness has no members"),
            | Self::UnknownClause { clause } => {
                write!(f, "step names clause {clause} outside the compiled system")
            },
            | Self::ShiftBelowMinimum { clause } => {
                write!(f, "step on clause {clause} fires below the minimum shift")
            },
            | Self::BodyUnavailable { clause } => {
                write!(f, "step on clause {clause} fires without its body")
            },
            | Self::TargetUncovered { variable } => match variable {
                | Some(variable) => {
                    write!(
                        f,
                        "the target over variable {} is not covered",
                        variable.index()
                    )
                },
                | None => f.write_str("the constant (bottom) target is not covered"),
            },
            | Self::MissingAssignment { variable } => {
                write!(
                    f,
                    "the witness assigns no value to variable {}",
                    variable.index()
                )
            },
            | Self::ConstraintViolated { index } => {
                write!(f, "declared constraint {index} fails under the witness")
            },
            | Self::MissingValue { variable } => {
                write!(
                    f,
                    "the countermodel assigns no value to variable {}",
                    variable.index()
                )
            },
            | Self::SeedUnsatisfied { variable } => match variable {
                | Some(variable) => {
                    write!(
                        f,
                        "the countermodel misses the seed over variable {}",
                        variable.index()
                    )
                },
                | None => f.write_str("the countermodel misses the bottom seed"),
            },
            | Self::ClauseUnsatisfied { clause } => {
                write!(f, "the countermodel violates clause {clause}")
            },
            | Self::NotRefuting => f.write_str("the recorded goal is not refuted"),
            | Self::Overflow => f.write_str("evaluating the evidence overflowed"),
        }
    }
}

impl Error for PosetEvidenceError
{
}

/// The Corollary 3.5 dichotomy, as data: a declared constraint set either
/// admits (with its consistency certificate inside the poset) or loops
/// (with the pumping witness). Exactly one case holds.
#[derive(Clone, Debug)]
pub enum AdmissionOutcome
{
    /// The constraints admit; the poset carries its consistency witness.
    Admitted(LandmarkPoset),
    /// The constraints loop; no homomorphism into `ℕ` exists.
    Loop(LoopWitness),
}

/// An admitted landmark poset.
///
/// It carries the declared constraints, their compiled clause system
/// (fixed at admission and reused by every query — the design record's
/// fixed-poset simplification), and the consistency certificate admission
/// produced.
#[derive(Clone, Debug)]
pub struct LandmarkPoset
{
    /// The declared constraints, in declaration order.
    constraints: Vec<LandmarkConstraint>,
    /// The declared variables, ascending.
    variables: BTreeSet<LevelVar>,
    /// The compiled base clauses (the documented deterministic compilation).
    clauses: Vec<HornClause>,
    /// The consistency certificate.
    consistency: ConsistencyWitness,
}

impl LandmarkPoset
{
    /// Admit a declared constraint set by loop-checking (TCS 913
    /// Corollary 3.5), returning the dichotomy as evidence-carrying data.
    ///
    /// # Contract
    /// - requires: nothing beyond well-formed constraints (guaranteed by
    ///   construction).
    /// - ensures: [`AdmissionOutcome::Admitted`] with a poset whose
    ///   [`ConsistencyWitness`] passes [`validate_consistency`] exactly when
    ///   the constraints have no loop; [`AdmissionOutcome::Loop`] with a
    ///   witness passing [`validate_loop_witness`] otherwise.
    /// - provides: the kernel's landmark-poset admission choke point (design
    ///   record §4; the empty set admits with an empty certificate, and queries
    ///   under it agree exactly with the slice-1 oracle).
    /// - fails: [`PosetError::Overflow`] on arithmetic past the representable
    ///   range; [`PosetError::EvidenceIncomplete`] if loop evidence extraction
    ///   misses its round limit (excluded by the theory; surfaced rather than
    ///   trusted).
    /// - panics: none.
    /// - intension: the seed is the constant `m` (the maximal body offset) over
    ///   the declared variables, the small-model snap bound is `m + |V| ·
    ///   Maxgain` (Corollary 4.2), and the loop shift is `m` when every
    ///   variable loops, `max(g(V∖W)) + Maxgain` otherwise — the paper's
    ///   constructions verbatim.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the paper's worked example pins both dichotomy arms
    ///   end-to-end (exact certificate on admission, exact member set/shift on
    ///   refusal), and every produced witness must pass its independent
    ///   validator, so a deciding mutant must also forge coherent evidence; the
    ///   L3 residue is the `W = V` shift choice, pinned by the loop goldens.
    /// - witness: `poset::tests::paper_example_admits_with_a_validating_homomorphism`
    /// - witness: `poset::tests::paper_loop_variant_returns_a_validating_loop_witness`
    /// - witness: `poset::tests::self_successor_equality_loops`
    /// - witness: `poset::tests::empty_constraint_set_admits_with_an_empty_certificate`
    ///
    /// # Errors
    /// [`PosetError::Overflow`] and [`PosetError::EvidenceIncomplete`] (the
    /// latter excluded by the theory).
    #[inline]
    pub fn admit(constraints: Vec<LandmarkConstraint>) -> Result<AdmissionOutcome, PosetError>
    {
        let variables: BTreeSet<LevelVar> = constraints
            .iter()
            .flat_map(LandmarkConstraint::variables)
            .collect();
        let clauses = compile(&constraints);
        let system = ClauseSystem {
            clauses,
            min_shift: HornShift::from(0_u128),
        };
        let maxgain = system.maxgain();
        let seed_value = system.max_body_offset();
        let variable_count =
            u128::try_from(variables.len()).map_err(|_error| PosetError::Overflow)?;
        let headroom = variable_count
            .checked_mul(u128::from(maxgain))
            .ok_or(PosetError::Overflow)?;
        let snap_bound = u128::from(seed_value)
            .checked_add(headroom)
            .map(SaturationBound::from)
            .ok_or(PosetError::Overflow)?;
        let seed: BTreeMap<HVar, ModelValue> = variables
            .iter()
            .map(|&variable| (HVar::Var(variable), ModelValue::Finite(seed_value)))
            .collect();
        let outcome =
            horn::saturate(&system, seed, snap_bound).map_err(|_error| PosetError::Overflow)?;
        let members: BTreeSet<LevelVar> = variables
            .iter()
            .copied()
            .filter(|&variable| {
                outcome
                    .values
                    .get(&HVar::Var(variable))
                    .copied()
                    .is_some_and(|value| bool::from(value.is_infinite()))
            })
            .collect();
        if members.is_empty() {
            let ceiling = outcome
                .values
                .values()
                .filter_map(|value| value.as_finite())
                .max()
                .unwrap_or(HornOffset::ZERO);
            let mut assigned_values = BTreeMap::new();
            for &variable in &variables {
                let level = outcome
                    .values
                    .get(&HVar::Var(variable))
                    .copied()
                    .and_then(ModelValue::as_finite)
                    .unwrap_or(HornOffset::ZERO);
                let assigned = u128::from(ceiling)
                    .checked_sub(u128::from(level))
                    .ok_or(PosetError::Overflow)?;
                let _previous = assigned_values.insert(variable, ConsistencyValue::from(assigned));
            }
            let consistency = ConsistencyWitness {
                values: assigned_values,
            };
            let poset = Self {
                constraints,
                variables,
                clauses: system.clauses,
                consistency,
            };
            return Ok(AdmissionOutcome::Admitted(poset));
        }
        let shift = if members.len() == variables.len() {
            seed_value.shift_from_zero()
        }
        else {
            let finite_ceiling = outcome
                .values
                .values()
                .filter_map(|value| value.as_finite())
                .max()
                .unwrap_or(HornOffset::ZERO);
            let finite_shift = u128::from(finite_ceiling)
                .checked_add(u128::from(maxgain))
                .ok_or(PosetError::Overflow)?;
            HornOffset::from(finite_shift).shift_from_zero()
        };
        let witness = extract_loop_witness(&system, &members, shift, maxgain)?;
        Ok(AdmissionOutcome::Loop(witness))
    }

    /// The declared constraints, in declaration order.
    #[inline]
    #[must_use]
    pub fn constraints(&self) -> &[LandmarkConstraint]
    {
        &self.constraints
    }

    /// The declared variables, in ascending order.
    #[inline]
    #[must_use]
    pub const fn variables(&self) -> &BTreeSet<LevelVar>
    {
        &self.variables
    }

    /// The consistency certificate admission produced.
    #[inline]
    #[must_use]
    pub const fn consistency(&self) -> &ConsistencyWitness
    {
        &self.consistency
    }

    /// Crate-internal access to the compiled clause list.
    pub(crate) fn compiled_clauses(&self) -> &[HornClause]
    {
        &self.clauses
    }
}

/// Extract the replayable pumping derivation behind a loop refusal: from
/// the atoms `{w+shift | w ∈ members}` over the clauses that stay inside
/// `members` (the paper's `S̄_C|W`), derive every `w+shift+1`.
fn extract_loop_witness(
    system: &ClauseSystem,
    members: &BTreeSet<LevelVar>,
    shift: HornShift,
    maxgain: MaxGain,
) -> Result<LoopWitness, PosetError>
{
    let inside = |variable: HVar| match variable {
        | HVar::Var(level_var) => members.contains(&level_var),
        | HVar::Bottom => false,
    };
    let mut filtered = Vec::new();
    let mut indices = Vec::new();
    for (index, clause) in system.clauses.iter().enumerate() {
        let head_inside = inside(clause.head().variable());
        let body_inside = clause.body().iter().all(|atom| inside(atom.variable()));
        if head_inside && body_inside {
            filtered.push(clause.clone());
            indices.push(index);
        }
    }
    let member_count = u128::try_from(members.len()).map_err(|_error| PosetError::Overflow)?;
    let clause_count = u128::try_from(filtered.len()).map_err(|_error| PosetError::Overflow)?;
    // A generous syntactic round limit: pumping raises every member within
    // one clause-graph traversal per unit of gain deficit, so the product
    // of the total gain budget and the traversal length bounds the rounds
    // with headroom. Exceeding it is theory-refuting and surfaces as
    // `EvidenceIncomplete` rather than being trusted or looping forever.
    let gain_budget = member_count
        .checked_mul(u128::from(maxgain))
        .and_then(|product| product.checked_add(2_u128))
        .ok_or(PosetError::Overflow)?;
    let traversal = clause_count
        .checked_add(member_count)
        .and_then(|sum| sum.checked_add(2_u128))
        .ok_or(PosetError::Overflow)?;
    let round_limit = gain_budget
        .checked_mul(traversal)
        .ok_or(PosetError::Overflow)?;
    let filtered_system = ClauseSystem {
        clauses: filtered,
        min_shift: HornShift::from(0_u128),
    };
    let seed: BTreeMap<HVar, ModelValue> = members
        .iter()
        .map(|&variable| {
            (
                HVar::Var(variable),
                ModelValue::Finite(shift.offset_from_zero()),
            )
        })
        .collect();
    let target_offset = shift
        .offset_from_zero()
        .checked_add(HornOffset::ONE)
        .map_err(|_error| PosetError::Overflow)?;
    let targets: Vec<HornAtom> = members
        .iter()
        .map(|&variable| HornAtom::new(HVar::Var(variable), target_offset))
        .collect();
    let log = horn::derive_targets(
        &filtered_system,
        seed,
        &targets,
        DerivationRoundLimit::from(round_limit),
    )
    .map_err(|_error| PosetError::Overflow)?;
    let log = log.ok_or(PosetError::EvidenceIncomplete)?;
    let remapped: Vec<FiringLogStep> = log
        .into_iter()
        .filter_map(|step| {
            indices
                .get(usize::from(step.clause()))
                .map(|&original| FiringLogStep::new(ClauseIndex::from(original), step.shift()))
        })
        .collect();
    Ok(LoopWitness {
        members: members.clone(),
        shift,
        derivation: Derivation::from_log(remapped),
    })
}

/// Check a loop witness against its declared constraints by replaying its
/// pumping derivation.
///
/// # Contract
/// - requires: `witness` claims the constraints loop; the constraint list must
///   be the declared one (the clause compilation is definitional, so validator
///   and oracle share it — the check, not the encoding, is the trusted step).
/// - ensures: `Ok(())` exactly when the member set is nonempty and the
///   derivation replays from exactly `{w + shift | w ∈ members}` to cover every
///   `w + shift + 1` — which exhibits `∨ members + shift` as a loop, so no
///   homomorphism into `ℕ` can exist.
/// - provides: the trusted half of the refusal evidence.
/// - fails: the first [`PosetEvidenceError`] encountered — replay errors in
///   step order, then uncovered targets in ascending variable order.
/// - panics: none.
///
/// # Errors
/// [`PosetEvidenceError::EmptyLoop`], the replay errors, and
/// [`PosetEvidenceError::TargetUncovered`].
///
/// # Adequacy
/// - hypothesis: L3 — rejection arms pinned by hand-perturbed witnesses,
///   acceptance pinned by the loop goldens (paper variant, self-successor).
/// - witness: `poset::tests::perturbed_loop_witness_arms_are_rejected`
/// - witness: `poset::tests::paper_loop_variant_returns_a_validating_loop_witness`
#[inline]
pub fn validate_loop_witness(
    constraints: &[LandmarkConstraint],
    witness: &LoopWitness,
) -> Result<(), PosetEvidenceError>
{
    if witness.members().is_empty() {
        return Err(PosetEvidenceError::EmptyLoop);
    }
    let clauses = compile(constraints);
    let seed: BTreeMap<HVar, HornOffset> = witness
        .members()
        .iter()
        .map(|&variable| (HVar::Var(variable), witness.shift().offset_from_zero()))
        .collect();
    let maxima = replay_derivation(
        &clauses,
        HornShift::from(0_u128),
        seed,
        witness.derivation(),
    )?;
    let target = witness
        .shift()
        .offset_from_zero()
        .checked_add(HornOffset::ONE)
        .map_err(|_error| PosetEvidenceError::Overflow)?;
    for &variable in witness.members() {
        let covered = maxima
            .get(&HVar::Var(variable))
            .is_some_and(|&maximum| target <= maximum);
        if !covered {
            return Err(PosetEvidenceError::TargetUncovered {
                variable: Some(variable),
            });
        }
    }
    Ok(())
}

/// Compile a declared constraint list into its base clauses — the
/// documented deterministic enumeration in this module's docs.
pub fn compile(constraints: &[LandmarkConstraint]) -> Vec<HornClause>
{
    let mut clauses = Vec::new();
    for constraint in constraints {
        let left_atoms = level_atoms(constraint.left());
        let right_atoms = level_atoms(constraint.right());
        match constraint.relation() {
            | ConstraintRelation::Leq => {
                push_family(&mut clauses, &right_atoms, &left_atoms);
            },
            | ConstraintRelation::Eq => {
                push_family(&mut clauses, &left_atoms, &right_atoms);
                push_family(&mut clauses, &right_atoms, &left_atoms);
            },
        }
    }
    clauses
}

/// A variable-only level's atoms as Horn atoms, ascending.
fn level_atoms(level: &Level) -> Vec<HornAtom>
{
    level
        .atoms()
        .map(|(variable, offset)| HornAtom::new(HVar::Var(variable), HornOffset::from(offset)))
        .collect()
}

/// Append the clause family `body → head` for each head atom, omitting
/// self-subsumed clauses.
fn push_family(
    clauses: &mut Vec<HornClause>,
    body: &[HornAtom],
    heads: &[HornAtom],
)
{
    for &head in heads {
        if let Some(clause) = HornClause::new(body, head)
            && !bool::from(clause.is_trivial())
        {
            clauses.push(clause);
        }
    }
}

/// Replay a derivation over a compiled clause list from seeded atom
/// availability, returning the per-variable maxima it reaches.
///
/// # Contract
/// - requires: `seed` maps each initially available variable to its maximal
///   seeded offset (absent variables have no atoms).
/// - ensures: `Ok(maxima)` exactly when every step names a known clause, fires
///   at or above `min_shift`, and finds its whole (shifted) body available at
///   application time; the maxima then bound exactly the atoms the derivation
///   establishes.
/// - provides: the shared replay engine of [`validate_loop_witness`] and the
///   entailment witness validator — trust concentrates here.
/// - fails: the first [`PosetEvidenceError`] encountered, in step order;
///   adversarial arithmetic overflow is rejected as
///   [`PosetEvidenceError::Overflow`], never wrapped.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 — each rejection arm is pinned by a hand-perturbed
///   derivation asserting the exact variant, and acceptance is pinned by the
///   property that every oracle-produced derivation replays.
/// - witness: `poset::tests::perturbed_loop_witness_arms_are_rejected`
/// - witness: `entail::tests::perturbed_witness_arms_are_rejected`
/// - witness: `poset_differential` (evidence-validation properties)
pub fn replay_derivation(
    clauses: &[HornClause],
    min_shift: HornShift,
    seed: BTreeMap<HVar, HornOffset>,
    derivation: &Derivation,
) -> Result<BTreeMap<HVar, HornOffset>, PosetEvidenceError>
{
    let mut maxima = seed;
    for step in derivation.steps() {
        let Some(clause) = clauses.get(usize::from(step.clause()))
        else {
            return Err(PosetEvidenceError::UnknownClause {
                clause: step.clause(),
            });
        };
        if step.shift() < min_shift {
            return Err(PosetEvidenceError::ShiftBelowMinimum {
                clause: step.clause(),
            });
        }
        for &atom in clause.body() {
            let needed = atom
                .offset()
                .checked_add_shift(step.shift())
                .map_err(|_error| PosetEvidenceError::Overflow)?;
            let available = maxima
                .get(&atom.variable())
                .is_some_and(|&maximum| needed <= maximum);
            if !available {
                return Err(PosetEvidenceError::BodyUnavailable {
                    clause: step.clause(),
                });
            }
        }
        let head = clause.head();
        let concluded = head
            .offset()
            .checked_add_shift(step.shift())
            .map_err(|_error| PosetEvidenceError::Overflow)?;
        let maximum = maxima.entry(head.variable()).or_insert(concluded);
        *maximum = (*maximum).max(concluded);
    }
    Ok(maxima)
}

/// Check a consistency witness against its declared constraints by direct
/// evaluation into `ℕ`.
///
/// # Contract
/// - requires: `witness` claims the constraints admit; the constraint list must
///   be the declared one (order matters only for error indices).
/// - ensures: `Ok(())` exactly when the witness assigns every mentioned
///   variable and every constraint holds under the assignment, evaluating a
///   side as `max(h(x) + offset, …)` — the homomorphism reading.
/// - provides: the trusted half of the admission evidence — a validator
///   independent of the whole Horn machinery (it only evaluates), against which
///   the decision procedure is self-incriminating.
/// - fails: the first [`PosetEvidenceError`] encountered, walking constraints
///   in declaration order.
/// - panics: none.
///
/// # Errors
/// [`PosetEvidenceError::MissingAssignment`],
/// [`PosetEvidenceError::ConstraintViolated`], and
/// [`PosetEvidenceError::Overflow`].
///
/// # Adequacy
/// - hypothesis: L3 — each rejection arm pinned by a hand-perturbed witness,
///   acceptance pinned by the property that every admission certificate
///   validates (and by the paper golden's exact certificate).
/// - witness: `poset::tests::perturbed_consistency_witness_is_rejected`
/// - witness: `poset::tests::paper_example_admits_with_a_validating_homomorphism`
#[inline]
pub fn validate_consistency(
    constraints: &[LandmarkConstraint],
    witness: &ConsistencyWitness,
) -> Result<(), PosetEvidenceError>
{
    for (index, constraint) in constraints.iter().enumerate() {
        let left = evaluate_side(constraint.left(), witness)?;
        let right = evaluate_side(constraint.right(), witness)?;
        let holds = match constraint.relation() {
            | ConstraintRelation::Leq => left <= right,
            | ConstraintRelation::Eq => left == right,
        };
        if !holds {
            return Err(PosetEvidenceError::ConstraintViolated { index });
        }
    }
    Ok(())
}

/// Evaluate a variable-only side under a consistency witness.
fn evaluate_side(
    side: &Level,
    witness: &ConsistencyWitness,
) -> Result<ConsistencyValue, PosetEvidenceError>
{
    let mut value = u128::from(side.constant_part());
    for (variable, offset) in side.atoms() {
        let base = witness
            .value_of(variable)
            .ok_or(PosetEvidenceError::MissingAssignment { variable })?;
        let component = u128::from(base)
            .checked_add(u128::from(offset))
            .ok_or(PosetEvidenceError::Overflow)?;
        value = value.max(component);
    }
    Ok(ConsistencyValue::from(value))
}

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::collections::BTreeSet;
    use alloc::vec;
    use alloc::vec::Vec;

    use super::AdmissionOutcome;
    use super::ConsistencyValue;
    use super::ConsistencyWitness;
    use super::Derivation;
    use super::DerivationStep;
    use super::LandmarkConstraint;
    use super::LandmarkPoset;
    use super::LoopWitness;
    use super::PosetError;
    use super::PosetEvidenceError;
    use super::validate_consistency;
    use super::validate_loop_witness;
    use crate::horn::ClauseIndex;
    use crate::horn::HornShift;
    use crate::level::Level;
    use crate::level::LevelConstant;
    use crate::level::LevelOffset;
    use crate::level::LevelVar;
    use crate::level::LevelVarIndex;

    #[test]
    fn paper_loop_variant_returns_a_validating_loop_witness()
    {
        let mut constraints = paper_constraints();
        constraints.push(LandmarkConstraint::leq(var_plus(0, 0), var_plus(4, 0)).unwrap());
        let witness = looped(&constraints);
        let expected: BTreeSet<LevelVar> = (0_u32 .. 5_u32).map(var).collect();
        assert_eq!(
            witness.members(),
            &expected,
            "the loop closes over every variable (W = abcde)"
        );
        assert_eq!(
            2_u128,
            u128::from(witness.shift()),
            "W = V pins the shift to the maximal body offset m = 2"
        );
        assert_eq!(
            Ok(()),
            validate_loop_witness(&constraints, &witness),
            "the pumping derivation replays"
        );
    }

    #[test]
    fn paper_example_admits_with_a_validating_homomorphism()
    {
        let poset = admitted(paper_constraints());
        assert_eq!(
            Ok(()),
            validate_consistency(poset.constraints(), poset.consistency()),
            "the certificate validates by direct evaluation"
        );
        // Admission seeds the constant m = 2, reaching the least model
        // a2 b3 c6 d5 e3, so h(v) = 6 − g(v) is pinned exactly.
        let expected: BTreeMap<LevelVar, ConsistencyValue> = [
            (var(0), ConsistencyValue::from(4_u128)),
            (var(1), ConsistencyValue::from(3_u128)),
            (var(2), ConsistencyValue::from(0_u128)),
            (var(3), ConsistencyValue::from(1_u128)),
            (var(4), ConsistencyValue::from(3_u128)),
        ]
        .into_iter()
        .collect();
        let actual: BTreeMap<LevelVar, ConsistencyValue> =
            poset.consistency().assignments().collect();
        assert_eq!(actual, expected, "the certificate is the paper's h");
    }

    #[test]
    fn transitive_chain_admits()
    {
        let constraints = vec![
            LandmarkConstraint::leq(var_plus(0, 0), var_plus(1, 0)).unwrap(),
            LandmarkConstraint::leq(var_plus(1, 0), var_plus(2, 0)).unwrap(),
        ];
        let poset = admitted(constraints);
        assert_eq!(
            Ok(()),
            validate_consistency(poset.constraints(), poset.consistency()),
            "a transitive chain admits with a validating certificate"
        );
    }

    #[test]
    fn equality_constraints_validate_under_the_certificate()
    {
        // Pins the `Eq` arm of the consistency check (the paper goldens
        // are ≤-only).
        let constraints = vec![
            LandmarkConstraint::leq(var_plus(0, 0), var_plus(1, 0)).unwrap(),
            LandmarkConstraint::equal(var_plus(1, 0), var_plus(2, 0)).unwrap(),
        ];
        let poset = admitted(constraints);
        assert_eq!(
            Ok(()),
            validate_consistency(poset.constraints(), poset.consistency()),
            "an equality constraint validates under the certificate"
        );
    }

    #[test]
    fn self_successor_equality_loops()
    {
        let constraints = vec![LandmarkConstraint::equal(var_plus(0, 0), var_plus(0, 1)).unwrap()];
        let witness = looped(&constraints);
        let expected: BTreeSet<LevelVar> = core::iter::once(var(0)).collect();
        assert_eq!(witness.members(), &expected, "x = x+1 loops on x");
        assert_eq!(
            Ok(()),
            validate_loop_witness(&constraints, &witness),
            "the one-step pumping derivation replays"
        );
    }

    #[test]
    fn perturbed_loop_witness_arms_are_rejected()
    {
        let constraints = vec![LandmarkConstraint::equal(var_plus(0, 0), var_plus(0, 1)).unwrap()];
        let genuine = looped(&constraints);

        let empty = LoopWitness {
            members: BTreeSet::new(),
            shift: genuine.shift(),
            derivation: genuine.derivation().clone(),
        };
        assert_eq!(
            Err(PosetEvidenceError::EmptyLoop),
            validate_loop_witness(&constraints, &empty),
            "an empty member set is rejected"
        );

        let unknown = LoopWitness {
            members: genuine.members().clone(),
            shift: genuine.shift(),
            derivation: Derivation {
                steps: vec![DerivationStep::new(
                    ClauseIndex::from(7_usize),
                    HornShift::from(0_u128),
                )],
            },
        };
        assert_eq!(
            Err(PosetEvidenceError::UnknownClause {
                clause: ClauseIndex::from(7_usize),
            }),
            validate_loop_witness(&constraints, &unknown),
            "an out-of-range clause index is rejected"
        );

        let unavailable = LoopWitness {
            members: genuine.members().clone(),
            shift: genuine.shift(),
            derivation: Derivation {
                steps: vec![DerivationStep::new(
                    ClauseIndex::from(0_usize),
                    HornShift::from(5_u128),
                )],
            },
        };
        assert_eq!(
            Err(PosetEvidenceError::BodyUnavailable {
                clause: ClauseIndex::from(0_usize),
            }),
            validate_loop_witness(&constraints, &unavailable),
            "a step firing above its available body is rejected"
        );

        let uncovered = LoopWitness {
            members: genuine.members().clone(),
            shift: genuine.shift(),
            derivation: Derivation { steps: vec![] },
        };
        assert_eq!(
            validate_loop_witness(&constraints, &uncovered),
            Err(PosetEvidenceError::TargetUncovered {
                variable: Some(var(0)),
            }),
            "an empty derivation covers no pumping target"
        );
    }

    #[test]
    fn partial_loop_pins_the_shift_choice()
    {
        // W ⊂ V: `x = x+1` loops on x while nothing ever raises y, so the
        // witness shift takes the Corollary 3.5 `max(g(V∖W)) + Maxgain`
        // branch (the paper goldens all have W = V).
        let constraints = vec![
            LandmarkConstraint::equal(var_plus(0, 0), var_plus(0, 1)).unwrap(),
            LandmarkConstraint::leq(var_plus(0, 0), var_plus(1, 0)).unwrap(),
        ];
        let witness = looped(&constraints);
        let expected: BTreeSet<LevelVar> = core::iter::once(var(0)).collect();
        assert_eq!(witness.members(), &expected, "only x loops");
        assert_eq!(
            1_u128,
            u128::from(witness.shift()),
            "the shift is max(g(V∖W)) + Maxgain = 0 + 1"
        );
        assert_eq!(
            Ok(()),
            validate_loop_witness(&constraints, &witness),
            "the partial-loop pumping derivation replays"
        );
    }

    #[test]
    fn constants_in_constraints_are_rejected()
    {
        let undominated = Level::constant(LevelConstant::from(5_u64)).max(&var_plus(0, 3));
        assert_eq!(
            PosetError::ConstantInConstraint,
            LandmarkConstraint::leq(undominated, var_plus(1, 0)).unwrap_err(),
            "an undominated constant part is rejected"
        );
        assert_eq!(
            PosetError::ConstantInConstraint,
            LandmarkConstraint::leq(Level::constant(LevelConstant::from(3_u64)), var_plus(0, 0))
                .unwrap_err(),
            "a pure constant side is rejected"
        );
        assert_eq!(
            PosetError::ConstantInConstraint,
            LandmarkConstraint::leq(Level::zero(), var_plus(0, 0)).unwrap_err(),
            "the zero side is rejected (zero is the constant 0)"
        );
        let dominated = Level::constant(LevelConstant::from(3_u64)).max(&var_plus(0, 3));
        assert!(
            LandmarkConstraint::leq(dominated, var_plus(0, 4)).is_ok(),
            "a dominated (canonicalized-away) constant is variable-only"
        );
    }

    #[test]
    fn perturbed_consistency_witness_is_rejected()
    {
        let constraints = vec![LandmarkConstraint::leq(var_plus(0, 1), var_plus(1, 0)).unwrap()];
        let missing = ConsistencyWitness {
            values: core::iter::once((var(0), ConsistencyValue::from(0_u128))).collect(),
        };
        assert_eq!(
            validate_consistency(&constraints, &missing),
            Err(PosetEvidenceError::MissingAssignment { variable: var(1) }),
            "an unassigned variable is rejected"
        );
        let violating = ConsistencyWitness {
            values: [
                (var(0), ConsistencyValue::from(0_u128)),
                (var(1), ConsistencyValue::from(0_u128)),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(
            Err(PosetEvidenceError::ConstraintViolated { index: 0_usize }),
            validate_consistency(&constraints, &violating),
            "x+1 ≤ y fails at h(x) = h(y) = 0"
        );
    }

    #[test]
    fn empty_constraint_set_admits_with_an_empty_certificate()
    {
        let poset = admitted(vec![]);
        assert!(poset.variables().is_empty(), "no variables are declared");
        assert_eq!(
            0_usize,
            poset.consistency().assignments().count(),
            "the certificate is empty"
        );
        assert_eq!(
            Ok(()),
            validate_consistency(poset.constraints(), poset.consistency()),
            "the empty certificate validates"
        );
    }

    #[test]
    fn error_displays_are_stable()
    {
        use alloc::string::ToString as _;

        let poset_errors = [
            (
                PosetError::ConstantInConstraint,
                "a declared constraint side mentions a constant",
            ),
            (
                PosetError::Overflow,
                "poset arithmetic stepped past the representable range",
            ),
            (
                PosetError::UnexpectedDivergence,
                "a query diverged on an admitted poset (excluded by admission)",
            ),
            (
                PosetError::EvidenceIncomplete,
                "evidence extraction missed its targets (excluded by the theory)",
            ),
        ];
        for (error, expected) in poset_errors {
            assert_eq!(error.to_string(), expected, "{error:?} must render stably");
        }

        let evidence_errors = [
            (
                PosetEvidenceError::EmptyLoop,
                "the loop witness has no members",
            ),
            (
                PosetEvidenceError::UnknownClause {
                    clause: ClauseIndex::from(7_usize),
                },
                "step names clause 7 outside the compiled system",
            ),
            (
                PosetEvidenceError::ShiftBelowMinimum {
                    clause: ClauseIndex::from(3_usize),
                },
                "step on clause 3 fires below the minimum shift",
            ),
            (
                PosetEvidenceError::BodyUnavailable {
                    clause: ClauseIndex::from(2_usize),
                },
                "step on clause 2 fires without its body",
            ),
            (
                PosetEvidenceError::TargetUncovered {
                    variable: Some(var(4)),
                },
                "the target over variable 4 is not covered",
            ),
            (
                PosetEvidenceError::TargetUncovered { variable: None },
                "the constant (bottom) target is not covered",
            ),
            (
                PosetEvidenceError::MissingAssignment { variable: var(1) },
                "the witness assigns no value to variable 1",
            ),
            (
                PosetEvidenceError::ConstraintViolated { index: 0_usize },
                "declared constraint 0 fails under the witness",
            ),
            (
                PosetEvidenceError::MissingValue { variable: var(2) },
                "the countermodel assigns no value to variable 2",
            ),
            (
                PosetEvidenceError::SeedUnsatisfied {
                    variable: Some(var(5)),
                },
                "the countermodel misses the seed over variable 5",
            ),
            (
                PosetEvidenceError::SeedUnsatisfied { variable: None },
                "the countermodel misses the bottom seed",
            ),
            (
                PosetEvidenceError::ClauseUnsatisfied {
                    clause: ClauseIndex::from(9_usize),
                },
                "the countermodel violates clause 9",
            ),
            (
                PosetEvidenceError::NotRefuting,
                "the recorded goal is not refuted",
            ),
            (
                PosetEvidenceError::Overflow,
                "evaluating the evidence overflowed",
            ),
        ];
        for (error, expected) in evidence_errors {
            assert_eq!(error.to_string(), expected, "{error:?} must render stably");
        }
    }

    use support::admitted;
    use support::looped;
    use support::paper_constraints;
    use support::var;
    use support::var_plus;

    mod support
    {
        use super::*;

        /// The paper's §5.2 example as declared constraints over `a`–`e` =
        /// variables `0`–`4`: `b+1 ≤ a∨b`, `c+3 ≤ b`, `d ≤ c+1`, `e ≤ b∨d+2`,
        /// compiling to exactly the paper's clause set.
        pub(super) fn paper_constraints() -> Vec<LandmarkConstraint>
        {
            vec![
                LandmarkConstraint::leq(var_plus(1, 1), var_plus(0, 0).max(&var_plus(1, 0)))
                    .unwrap(),
                LandmarkConstraint::leq(var_plus(2, 3), var_plus(1, 0)).unwrap(),
                LandmarkConstraint::leq(var_plus(3, 0), var_plus(2, 1)).unwrap(),
                LandmarkConstraint::leq(var_plus(4, 0), var_plus(1, 0).max(&var_plus(3, 2)))
                    .unwrap(),
            ]
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

        /// Admit a constraint set that must admit.
        pub(super) fn admitted(constraints: Vec<LandmarkConstraint>) -> LandmarkPoset
        {
            match LandmarkPoset::admit(constraints).unwrap() {
                | AdmissionOutcome::Admitted(poset) => poset,
                | AdmissionOutcome::Loop(witness) => {
                    panic!("expected admission, got loop over {:?}", witness.members())
                },
            }
        }

        /// Admit a constraint set that must loop.
        pub(super) fn looped(constraints: &[LandmarkConstraint]) -> LoopWitness
        {
            match LandmarkPoset::admit(constraints.to_vec()).unwrap() {
                | AdmissionOutcome::Loop(witness) => witness,
                | AdmissionOutcome::Admitted(_poset) => panic!("expected a loop, got admission"),
            }
        }

        /// A test variable by index.
        pub(super) fn var(index: impl Into<LevelVarIndex>) -> LevelVar
        {
            LevelVar::new(index.into())
        }
    }
}
