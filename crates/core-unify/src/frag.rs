//! The fragment boundary, named.
//!
//! A solver that is unreliable in a way nobody can predict is worse than one
//! that solves less, so every place this solver stops carries a name, and the
//! name is part of the answer. A caller reading [`PostponeReason`] knows what
//! would have to change for the constraint to become solvable, and a test
//! reading it can pin the boundary rather than pin the absence of a solution.
//!
//! # What is inside the fragment
//!
//! - **Miller patterns.** A metavariable applied to a spine of distinct
//!   variables the solver itself opened.
//! - **The negative eta laws.** Ordinary conversion decides function eta and
//!   lazy-pair eta, so the solver may use both. Lazy-pair eta is also what
//!   makes **meta splitting** most general: a projected metavariable is
//!   replaced by a pair of fresh ones with no choice taken.
//! - **Same-constructor congruence** over the positive structure, with record
//!   fields compared in canonical order, no width rule, no permutation rule,
//!   and thunk grades compared exactly. A packed module decomposes by the same
//!   congruence: its witness types are compared by α-identity of their syntax —
//!   exactly the comparison ordinary conversion states — and its payload is the
//!   one residual equation.
//! - **Rigid-rigid same-head spine decomposition**, and `Return` congruence.
//!
//! # What is outside it, and why
//!
//! Everything in [`PostponeReason`]. Two of those entries are not fragment
//! boundaries but **theory** boundaries, and they are the ones worth reading
//! twice.
//!
//! [`PostponeReason::DefinitionalSingleton`] and
//! [`PostponeReason::PositiveEta`] are outside because ordinary conversion does
//! not decide them: a neutral at unit type does not convert with `unit`, and a
//! positive pair rebuilt by `split` does not convert with its own neutral. A
//! solver that solved them would emit certificates the ordinary checker
//! refutes, which is the one failure the self-certifying design exists to make
//! impossible. They become solvable when conversion becomes type-directed, and
//! not before.
//!
//! [`PostponeReason::HoleConsistency`] is outside for a different reason.
//! Conversion relates an undeclared hole to every value in both directions, and
//! that relation is **not transitive**, so an equation discharged through it
//! composes with nothing. Treating such a discharge as a solved constraint
//! would let a chain of individually-accepted steps produce a conclusion no
//! single conversion supports.

use alloc::vec::Vec;

use gandr_core_nbe::Normalizer;
use gandr_core_nbe::sem::Elim;
use gandr_core_nbe::sem::Rigid;
use gandr_core_nbe::sem::SemError;
use gandr_core_nbe::sem::SemValueNode;
use gandr_core_term::boundary::VariableLevel;

/// Why the solver stopped on a constraint without deciding it.
///
/// Postponement is never a failure claim. It says the constraint is outside the
/// predictable fragment as posed, and it may well become solvable once other
/// metavariables are bound — which is what the blockers a postponement reports
/// are for.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum PostponeReason
{
    /// A spine argument is not a variable the solver opened, so inverting the
    /// spine would need a choice.
    NonPatternSpine,
    /// A spine repeats a variable, so the solution is undetermined at the
    /// repeated position.
    RepeatedSpineVariable,
    /// A spine argument is a positive constructor or a packed module — an
    /// inert canonical form either way. Inverting it would define the solution
    /// only on that form's image and leave the rest to a guess.
    ConstructorInSpine,
    /// A spine carries a sequencing continuation, which is not an eliminator
    /// the pattern discipline can invert.
    SequencedSpine,
    /// A spine projects after applying. Meta splitting inverts a projection
    /// that leads the spine; a projection behind an application would need the
    /// split to be pushed under the abstraction.
    ProjectionAfterApplication,
    /// A value-sorted metavariable was reached through a `force` without a
    /// declared thunk grade, and conversion compares thunk grades exactly, so
    /// building the solution would mean inventing one.
    UndeclaredThunkGrade,
    /// The metavariable occurs in the candidate solution, but so does another
    /// metavariable, so pruning might still remove the occurrence. Only an
    /// occurrence with nothing left to prune is a refutation.
    FlexOccurs,
    /// The candidate solution mentions a variable outside the spine, but
    /// another metavariable is present and pruning might still remove it.
    FlexEscape,
    /// Two distinct metavariables face each other and neither spine covers the
    /// other, so the most general answer needs a spine intersection.
    FlexFlexIntersection,
    /// Two occurrences of one metavariable face each other under spines the
    /// intersection rule cannot align.
    FlexFlexMismatchedSpines,
    /// An eliminator is stuck on a metavariable, so the equation cannot be
    /// decomposed until that metavariable is bound.
    BlockedElimination,
    /// Two reified stacks face each other. A reified stack carries source
    /// syntax verbatim and is opaque to the solver, so nothing inside it
    /// decomposes; conversion decides it once no metavariable is in the way.
    OpaqueReifiedStack,
    /// The two heads disagree, neither of them is a metavariable, and neither
    /// can reduce further, but a metavariable somewhere inside means the solver
    /// defers rather than refuting.
    HeadMismatch,
    /// Deciding the constraint would need a definitional singleton rule, which
    /// ordinary conversion does not have.
    DefinitionalSingleton,
    /// Deciding the constraint would need eta for a positive former, which
    /// ordinary conversion does not have.
    PositiveEta,
    /// The constraint would be discharged only by conversion's hole
    /// consistency, and that relation is not transitive, so the discharge
    /// cannot be composed with anything.
    HoleConsistency,
    /// The step budget ran out before the constraint was reached.
    BudgetExhausted,
}

/// Why no substitution can satisfy a constraint.
///
/// A refutation is the strongest claim the solver makes, so each of the three
/// is available only where the evidence is complete: a clash needs two
/// canonical heads, and an occurs or escape needs a candidate solution with no
/// other metavariable left to prune.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Refutation
{
    /// Two canonical forms with different constructors, which no substitution
    /// can bring together.
    Clash,
    /// A metavariable occurs in its own candidate solution, with no other
    /// metavariable present to prune the occurrence away.
    Occurs,
    /// A candidate solution mentions a variable outside the metavariable's
    /// spine, with no other metavariable present to prune it away. A closed
    /// metavariable can never produce that variable.
    Escape,
}

/// Why the current conversion relation declined to relate a metavariable-free
/// constraint.
///
/// A refusal is weaker than a refutation: a fuller environment or a different
/// conversion budget may relate the same terms.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum Refusal
{
    /// Ordinary conversion reports distinct under the current environment.
    Conversion,
}

/// What a metavariable's spine is, for the pattern discipline.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SpineShape
{
    /// A Miller pattern: distinct opened variables, in spine order.
    Pattern(Vec<VariableLevel>),
    /// A leading projection, which meta splitting inverts.
    Projected,
    /// Outside the fragment, for this reason.
    Blocked(PostponeReason),
}

/// Classifies a metavariable's spine.
///
/// # Contract
/// - ensures: an empty spine is the empty pattern; a spine whose first
///   eliminator is a projection is [`SpineShape::Projected`] whatever follows
///   it, because splitting fires before the rest is read; otherwise the result
///   is a pattern exactly when every eliminator applies a distinct variable the
///   solver opened, and a named block otherwise. The first blocking eliminator
///   in spine order chooses the reason.
/// - provides: the fragment test one flex solve turns on.
/// - fails: [`SemError`] when a spine argument does not resolve in the arena.
/// - panics: none.
///
/// # Errors
///
/// Returns [`SemError`] when a spine argument does not resolve.
///
/// # Adequacy
/// - hypothesis: L3 — one witness per outcome, separated pointwise: the empty
///   spine, a two-variable pattern, a repeated variable, a constructor
///   argument, a non-variable neutral argument, a sequencing continuation, a
///   leading projection, and a projection behind an application.
/// - witness: `unify::tests::a_pattern_solution_replays_through_ordinary_conversion`
/// - witness: `unify::tests::a_repeated_spine_variable_postpones`
/// - witness: `unify::tests::a_constructor_in_a_spine_postpones`
/// - witness: `unify::tests::a_sequencing_continuation_in_a_spine_postpones`
/// - witness: `unify::tests::a_projection_behind_an_application_postpones`
/// - witness: `unify::tests::lazy_pair_eta_and_meta_splitting_replay`
pub(crate) fn classify_spine(
    nbe: &Normalizer,
    spine: &[Elim],
) -> Result<SpineShape, SemError>
{
    if let Some(&Elim::Project(_)) = spine.first() {
        return Ok(SpineShape::Projected);
    }
    let mut levels = Vec::with_capacity(spine.len());
    for elim in spine {
        let argument = match *elim {
            | Elim::Apply(argument) => argument,
            | Elim::Project(_) => {
                return Ok(SpineShape::Blocked(
                    PostponeReason::ProjectionAfterApplication,
                ));
            },
            | Elim::Sequence(_) => {
                return Ok(SpineShape::Blocked(PostponeReason::SequencedSpine));
            },
        };
        let level = match *nbe.arena().value(argument)?.node() {
            | SemValueNode::Rigid(Rigid::Level(level), _) => level,
            | SemValueNode::Unit
            | SemValueNode::Int(_)
            | SemValueNode::Str(_)
            | SemValueNode::Num(_)
            | SemValueNode::Pair(..)
            | SemValueNode::Inj(..)
            | SemValueNode::List(_)
            | SemValueNode::Record(_)
            | SemValueNode::Here(_)
            | SemValueNode::Ctor { .. }
            | SemValueNode::Pack { .. } => {
                return Ok(SpineShape::Blocked(PostponeReason::ConstructorInSpine));
            },
            // A free or hole-headed rigid, a thunk, a reified stack — and any
            // variant the `#[non_exhaustive]` semantic domain gains later, which
            // reaches this arm rather than failing the match to compile. None of
            // them is a distinct bound variable, so none is a pattern spine, and
            // postponing loses completeness rather than soundness.
            | _ => {
                return Ok(SpineShape::Blocked(PostponeReason::NonPatternSpine));
            },
        };
        if levels.contains(&level) {
            return Ok(SpineShape::Blocked(PostponeReason::RepeatedSpineVariable));
        }
        levels.push(level);
    }
    Ok(SpineShape::Pattern(levels))
}
