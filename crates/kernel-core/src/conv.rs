//! Definitional equality (conversion) for the S1 fragment — the C5-quarantined
//! conversion record (C5).
//!
//! # The S1 conversion algorithm, precisely
//!
//! The checker's conversion runs at a **mode switch** — when a synthesizing
//! term is used where a type is expected, its synthesized type must convert
//! against the expected type. At S1 the definitional equality this needs is
//! **type conversion only**, and it is exactly:
//!
//! * an **id-equality early-out** — two nodes named by the *same id in the same
//!   arena* are the same node, hence structurally equal (trivially sound, the
//!   Lean `is_eqp`-first / smalltt ptr-eq posture, impl-models §2.1); it admits
//!   **no table into the TCB** (C3 held) because it decides only *reflexive*
//!   pairs and defers every distinct-id pair to the structural walk;
//! * α-structural equality — terms are nameless (de Bruijn), so α-equivalence
//!   is syntactic identity, and types carry no binders;
//! * with `gandr_kernel_strata::Level` **canonical equality** at
//!   [`crate::types::ValueType::Universe`] and
//!   [`crate::types::ValueType::Lift`] (the strata `Level` type's derived
//!   equality **is** the level-equality oracle);
//! * structurally through `Product`, `Sum`, `Thunk`, `Arrow`, and `Returner`,
//!   resolving each child through the arena's checked `get`.
//!
//! **No β law fires and no computation is evaluated.** At S1 no value-type or
//! computation-type former is indexed by a value term, so type conversion
//! never descends into a term — the C5 quarantine holds **vacuously** at S1.
//! Once a term-indexed type former lands (an `El`/description code carrying a
//! value into a type, S2+), values will appear in types and the β laws of the
//! value fragment will become load-bearing here.
//!
//! # Fail-closed on a dangling id
//!
//! A child id always resolves under the minting invariant ([`crate::arena`]).
//! Were one to dangle, the checked `get` yields `None` and the walk answers
//! [`Convertibility::Distinct`] — rejecting is the safe (fail-closed) verdict.
//!
//! # Totality
//!
//! Every face is **iterative** over an explicit heap worklist of `Copy` id
//! pairs (no lifetimes), never recursive: the derived structural equality would
//! recurse on depth and overflow the stack on an adversarial-depth input; the
//! worklist keeps conversion total (docs/workflow/rust.md).

use alloc::vec::Vec;

use gandr_kernel_conversion_trace::ConversionDecision;
use gandr_kernel_conversion_trace::NullSink;
use gandr_kernel_conversion_trace::TraceSink;

use crate::arena::CompTypeId;
use crate::arena::ComputationId;
use crate::arena::TermArena;
use crate::arena::ValueId;
use crate::arena::ValueTypeId;
use crate::error::ComputationTypeMismatch;
use crate::error::KernelError;
use crate::error::ValueTypeMismatch;
use crate::term::Computation;
use crate::term::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// The kernel-local identity carried by one conversion decision.
///
/// The decision vocabulary is shared with the untrusted engine; only the
/// identities are consumer-owned, so the seam carries no term representation
/// or wire format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceId
{
    /// A value node in the kernel arena.
    Value(ValueId),
    /// A computation node in the kernel arena.
    Comp(ComputationId),
    /// A value-type node in the kernel arena.
    ValueType(ValueTypeId),
    /// A computation-type node in the kernel arena.
    CompType(CompTypeId),
}

/// Whether a kernel-side decision replay accepted the trace and verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayStatus
{
    /// The kernel reproduced the decision grain and claimed verdict.
    Green,
    /// The kernel's decision grain differed from the supplied trace.
    TraceMismatch,
    /// The kernel's verdict differed from the supplied verdict.
    VerdictMismatch,
}

/// Whether two types or two terms are convertible (definitionally equal).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Convertibility
{
    /// The two are definitionally equal.
    Convertible,
    /// The two are distinct.
    Distinct,
}

/// A pending type-conversion obligation: a same-polarity id pair still to
/// compare.
#[derive(Clone, Copy)]
enum TypeGoal
{
    /// A pair of value types.
    Value(ValueTypeId, ValueTypeId),
    /// A pair of computation types.
    Comp(CompTypeId, CompTypeId),
}

/// A pending term-conversion obligation: a same-polarity id pair still to
/// compare.
#[derive(Clone, Copy)]
enum TermGoal
{
    /// A pair of values.
    Value(ValueId, ValueId),
    /// A pair of computations.
    Comp(ComputationId, ComputationId),
}

/// Decide convertibility of two value types in one arena.
///
/// # Contract
/// - requires: `left` and `right` resolve in `arena`.
/// - ensures: [`Convertibility::Convertible`] exactly when the two are equal up
///   to the S1 definitional equality (id-equality then α-structural with
///   canonical levels); the relation is reflexive, symmetric, and transitive.
/// - provides: the type equality the checker invokes at a value mode switch.
/// - fails: never — a negative answer is [`Convertibility::Distinct`], not a
///   failure (and a dangling id fails closed to `Distinct`).
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — a boundary-biased differential against the crate's own
///   generator confirms it agrees with reflexive equality and separates
///   distinct types on every generated pair; the L3 residues are the
///   id-equality early-out (a shared id converts without a structural walk) and
///   the level-node comparison (canonical `Level` equality, not syntactic).
/// - witness: `conv::tests::value_type_conversion_is_reflexive`
/// - witness: `conv::tests::value_type_conversion_separates_distinct_types`
/// - witness: `conversion::tests::prop_value_type_conversion_is_reflexive`
#[inline]
#[must_use]
pub fn convertible_value_types(
    arena: &TermArena,
    left: ValueTypeId,
    right: ValueTypeId,
) -> Convertibility
{
    converge_types(arena, TypeGoal::Value(left, right))
}

/// Decide convertibility of two computation types in one arena.
///
/// # Contract
/// - requires: `left` and `right` resolve in `arena`.
/// - ensures: [`Convertibility::Convertible`] exactly when the two are equal up
///   to the S1 definitional equality; reflexive, symmetric, transitive.
/// - provides: the type equality the checker invokes at a computation mode
///   switch.
/// - fails: never — a negative answer is [`Convertibility::Distinct`].
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — the differential confirms agreement with reflexive
///   equality on generated computation types; the L3 residue is the arrow's two
///   children (domain value type, codomain computation type).
/// - witness: `conv::tests::computation_type_conversion_is_reflexive`
/// - witness: `conversion::tests::prop_computation_type_conversion_is_reflexive`
#[inline]
#[must_use]
pub fn convertible_comp_types(
    arena: &TermArena,
    left: CompTypeId,
    right: CompTypeId,
) -> Convertibility
{
    converge_types(arena, TypeGoal::Comp(left, right))
}

/// Decide α-structural convertibility of two values (the C5 value fragment).
///
/// # Contract
/// - requires: `left` and `right` resolve in `arena`.
/// - ensures: [`Convertibility::Convertible`] exactly when the two values are
///   α-equal (syntactic identity under de Bruijn, with canonical literal
///   equality at leaves); no β law fires and no thunked computation is run.
/// - provides: the value-fragment definitional equality C5 permits (unused by
///   the S1 checker; the seed for term-indexed extensions).
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2/L3 — reflexivity on generated values plus the leaf
///   residues: canonical literal equality (a padded and a bare literal convert)
///   and injection-side sensitivity (`inl v` and `inr v` do not).
/// - witness: `conv::tests::value_conversion_ignores_literal_padding`
/// - witness: `conv::tests::value_conversion_distinguishes_injection_sides`
/// - witness: `conversion::tests::prop_value_conversion_is_reflexive`
#[inline]
#[must_use]
pub fn convertible_values(
    arena: &TermArena,
    left: ValueId,
    right: ValueId,
) -> Convertibility
{
    let mut sink = NullSink;
    convertible_values_with_sink(arena, left, right, &mut sink)
}

/// Decide value convertibility while emitting decision-grain events to `sink`.
///
/// # Contract
/// - ensures: returns the same verdict as [`convertible_values`].
/// - provides: the kernel consumer of the shared conversion trace vocabulary.
/// - fails: never.
/// - panics: none.
#[inline]
#[must_use]
pub fn convertible_values_with_sink<S>(
    arena: &TermArena,
    left: ValueId,
    right: ValueId,
    sink: &mut S,
) -> Convertibility
where
    S: TraceSink<TraceId>,
{
    converge_terms(arena, TermGoal::Value(left, right), sink)
}

/// Decide α-structural convertibility of two computations (the C5 fragment).
///
/// # Contract
/// - requires: `left` and `right` resolve in `arena`.
/// - ensures: [`Convertibility::Convertible`] exactly when the two computations
///   are α-equal (syntactic identity under de Bruijn); no β law fires and
///   nothing is evaluated.
/// - provides: the computation-fragment structural equality C5 permits (unused
///   by the S1 checker; the seed for term-indexed extensions).
/// - fails: never.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — reflexivity on generated computations; the L3 residue is
///   the binder-blindness of de Bruijn (two lambdas convert exactly when their
///   bodies do, with no name comparison).
/// - witness: `conv::tests::computation_conversion_is_alpha_structural`
/// - witness: `conversion::tests::prop_computation_conversion_is_reflexive`
#[inline]
#[must_use]
pub fn convertible_computations(
    arena: &TermArena,
    left: ComputationId,
    right: ComputationId,
) -> Convertibility
{
    let mut sink = NullSink;
    convertible_computations_with_sink(arena, left, right, &mut sink)
}

/// Decide computation convertibility while emitting decision-grain events.
///
/// # Contract
/// - ensures: returns the same verdict as [`convertible_computations`].
/// - provides: the kernel consumer of the shared conversion trace vocabulary.
/// - fails: never.
/// - panics: none.
#[inline]
#[must_use]
pub fn convertible_computations_with_sink<S>(
    arena: &TermArena,
    left: ComputationId,
    right: ComputationId,
    sink: &mut S,
) -> Convertibility
where
    S: TraceSink<TraceId>,
{
    converge_terms(arena, TermGoal::Comp(left, right), sink)
}
/// Convert a synthesized value type against an expected one, building the
/// mismatch error (a self-contained arena snapshot of both roots) on
/// divergence.
///
/// # Contract
/// - requires: `expected` and `actual` resolve in `arena`.
/// - ensures: `Ok(())` exactly when [`convertible_value_types`] converges.
/// - provides: the checker's value mode-switch step.
/// - fails: [`KernelError::ValueTypeMismatch`] carrying a snapshot of the two
///   roots (self-contained, so it survives the arena truncation on rejection).
/// - panics: none.
///
/// # Errors
/// [`KernelError::ValueTypeMismatch`].
#[inline]
pub fn convert_value_type(
    arena: &TermArena,
    expected: ValueTypeId,
    actual: ValueTypeId,
) -> Result<(), KernelError>
{
    match convertible_value_types(arena, expected, actual) {
        | Convertibility::Convertible => Ok(()),
        | Convertibility::Distinct => {
            let (snapshot, expected, actual) = arena.snapshot_value_types(expected, actual);
            Err(KernelError::ValueTypeMismatch(alloc::boxed::Box::new(
                ValueTypeMismatch::new(snapshot, expected, actual),
            )))
        },
    }
}

/// Convert a synthesized computation type against an expected one, building the
/// mismatch error (a self-contained arena snapshot) on divergence.
///
/// # Contract
/// - requires: `expected` and `actual` resolve in `arena`.
/// - ensures: `Ok(())` exactly when [`convertible_comp_types`] converges.
/// - provides: the checker's computation mode-switch step.
/// - fails: [`KernelError::ComputationTypeMismatch`] carrying a snapshot of the
///   two roots.
/// - panics: none.
///
/// # Errors
/// [`KernelError::ComputationTypeMismatch`].
#[inline]
pub fn convert_comp_type(
    arena: &TermArena,
    expected: CompTypeId,
    actual: CompTypeId,
) -> Result<(), KernelError>
{
    match convertible_comp_types(arena, expected, actual) {
        | Convertibility::Convertible => Ok(()),
        | Convertibility::Distinct => {
            let (snapshot, expected, actual) = arena.snapshot_comp_types(expected, actual);
            Err(KernelError::ComputationTypeMismatch(
                alloc::boxed::Box::new(ComputationTypeMismatch::new(snapshot, expected, actual)),
            ))
        },
    }
}

/// Run the type-conversion worklist to a verdict.
///
/// # Contract
/// - requires: `initial` is a same-polarity id pair resolving in `arena`.
/// - ensures: [`Convertibility::Convertible`] exactly when every reachable
///   sub-pair is id-equal or matches structurally with canonical-level equality
///   at level nodes; the walk is iterative over a heap stack, so it is total on
///   any depth.
/// - provides: the shared engine of the type-conversion faces.
/// - fails: never (the verdict is the return value; a dangling id ⇒
///   `Distinct`).
/// - panics: none.
#[inline]
fn converge_types(
    arena: &TermArena,
    initial: TypeGoal,
) -> Convertibility
{
    let mut stack: Vec<TypeGoal> = Vec::new();
    stack.push(initial);
    while let Some(goal) = stack.pop() {
        match goal {
            | TypeGoal::Value(left, right) => {
                if left == right {
                    continue;
                }
                let (Some(left), Some(right)) = (arena.value_type(left), arena.value_type(right))
                else {
                    return Convertibility::Distinct;
                };
                match (left, right) {
                    | (&ValueType::Base(ref one), &ValueType::Base(ref other)) => {
                        if one != other {
                            return Convertibility::Distinct;
                        }
                    },
                    | (&ValueType::Unit, &ValueType::Unit) => {},
                    | (
                        &ValueType::Product(ref one_first, ref one_second),
                        &ValueType::Product(ref other_first, ref other_second),
                    )
                    | (
                        &ValueType::Sum(ref one_first, ref one_second),
                        &ValueType::Sum(ref other_first, ref other_second),
                    ) => {
                        stack.push(TypeGoal::Value(*one_first, *other_first));
                        stack.push(TypeGoal::Value(*one_second, *other_second));
                    },
                    | (&ValueType::Thunk(ref one_body), &ValueType::Thunk(ref other_body)) => {
                        stack.push(TypeGoal::Comp(*one_body, *other_body));
                    },
                    | (
                        &ValueType::Universe(ref one_level),
                        &ValueType::Universe(ref other_level),
                    ) => {
                        if one_level != other_level {
                            return Convertibility::Distinct;
                        }
                    },
                    // Sealed abstract types: **nominal, and this is the whole
                    // rule**. Two atoms convert exactly when they name the same
                    // abstract-type declaration; there is deliberately no arm
                    // that relates an atom to a structural type, so opacity is
                    // a consequence of the closed match rather than a fact the
                    // producer asserts. Distinct sealings of structurally
                    // identical implementations therefore stay distinct types.
                    | (&ValueType::Abstract(one_atom), &ValueType::Abstract(other_atom)) => {
                        if one_atom != other_atom {
                            return Convertibility::Distinct;
                        }
                    },
                    | (
                        &ValueType::Lift {
                            inner: ref one_inner,
                            target: ref one_target,
                        },
                        &ValueType::Lift {
                            inner: ref other_inner,
                            target: ref other_target,
                        },
                    ) => {
                        if one_target != other_target {
                            return Convertibility::Distinct;
                        }
                        stack.push(TypeGoal::Value(*one_inner, *other_inner));
                    },
                    | _ => return Convertibility::Distinct,
                }
            },
            | TypeGoal::Comp(left, right) => {
                if left == right {
                    continue;
                }
                let (Some(left), Some(right)) = (arena.comp_type(left), arena.comp_type(right))
                else {
                    return Convertibility::Distinct;
                };
                match (left, right) {
                    | (&CompType::Returner(ref one), &CompType::Returner(ref other)) => {
                        stack.push(TypeGoal::Value(*one, *other));
                    },
                    | (
                        &CompType::Arrow {
                            domain: ref one_domain,
                            codomain: ref one_codomain,
                        },
                        &CompType::Arrow {
                            domain: ref other_domain,
                            codomain: ref other_codomain,
                        },
                    ) => {
                        stack.push(TypeGoal::Value(*one_domain, *other_domain));
                        stack.push(TypeGoal::Comp(*one_codomain, *other_codomain));
                    },
                    | _ => return Convertibility::Distinct,
                }
            },
        }
    }
    Convertibility::Convertible
}

/// Run the term-conversion (α-equivalence) worklist to a verdict.
///
/// # Contract
/// - requires: `initial` is a same-polarity id pair resolving in `arena`.
/// - ensures: [`Convertibility::Convertible`] exactly when the two terms are
///   α-equal — id-equality then syntactic identity under de Bruijn, with
///   canonical literal equality at leaves and injection-side sensitivity;
///   nothing is evaluated.
/// - provides: the shared engine of the term-conversion faces.
/// - fails: never (a dangling id ⇒ `Distinct`).
/// - panics: none.
#[inline]
fn converge_terms<S>(
    arena: &TermArena,
    initial: TermGoal,
    sink: &mut S,
) -> Convertibility
where
    S: TraceSink<TraceId>,
{
    let mut stack: Vec<TermGoal> = Vec::new();
    stack.push(initial);
    while let Some(goal) = stack.pop() {
        match goal {
            | TermGoal::Value(left_id, right_id) => {
                if left_id == right_id {
                    sink.record(ConversionDecision::ComparedShared {
                        left: TraceId::Value(left_id),
                        right: TraceId::Value(right_id),
                    });
                    continue;
                }
                let (Some(left), Some(right)) = (arena.value(left_id), arena.value(right_id))
                else {
                    sink.record(ConversionDecision::Postpone {
                        constant: TraceId::Value(left_id),
                    });
                    return Convertibility::Distinct;
                };
                match (left, right) {
                    | (&Value::Variable(_), &Value::Variable(_))
                    | (&Value::Constant(_), &Value::Constant(_))
                    | (&Value::Unit, &Value::Unit)
                    | (&Value::Literal(_), &Value::Literal(_)) => {
                        // Leaf values convert by direct equality on the inline
                        // payload (leaves have no id children — the derived-
                        // equality caveat does not apply here).
                        if left != right {
                            sink.record(ConversionDecision::Postpone {
                                constant: TraceId::Value(left_id),
                            });
                            return Convertibility::Distinct;
                        }
                    },
                    | (
                        &Value::Pair(ref one_first, ref one_second),
                        &Value::Pair(ref other_first, ref other_second),
                    ) => {
                        stack.push(TermGoal::Value(*one_first, *other_first));
                        stack.push(TermGoal::Value(*one_second, *other_second));
                    },
                    | (
                        &Value::Injection(ref one_side, ref one_body),
                        &Value::Injection(ref other_side, ref other_body),
                    ) => {
                        if one_side != other_side {
                            sink.record(ConversionDecision::Postpone {
                                constant: TraceId::Value(left_id),
                            });
                            return Convertibility::Distinct;
                        }
                        stack.push(TermGoal::Value(*one_body, *other_body));
                    },
                    | (&Value::Thunk(ref one_body), &Value::Thunk(ref other_body)) => {
                        sink.record(ConversionDecision::Force {
                            thunk: TraceId::Comp(*one_body),
                        });
                        sink.record(ConversionDecision::Force {
                            thunk: TraceId::Comp(*other_body),
                        });
                        stack.push(TermGoal::Comp(*one_body, *other_body));
                    },
                    | (
                        &Value::Lift {
                            target: ref one_target,
                            body: ref one_body,
                        },
                        &Value::Lift {
                            target: ref other_target,
                            body: ref other_body,
                        },
                    ) => {
                        if one_target != other_target {
                            sink.record(ConversionDecision::Postpone {
                                constant: TraceId::Value(left_id),
                            });
                            return Convertibility::Distinct;
                        }
                        stack.push(TermGoal::Value(*one_body, *other_body));
                    },
                    | _ => {
                        sink.record(ConversionDecision::Postpone {
                            constant: TraceId::Value(left_id),
                        });
                        return Convertibility::Distinct;
                    },
                }
            },
            | TermGoal::Comp(left_id, right_id) => {
                if left_id == right_id {
                    sink.record(ConversionDecision::ComparedShared {
                        left: TraceId::Comp(left_id),
                        right: TraceId::Comp(right_id),
                    });
                    continue;
                }
                let (Some(left), Some(right)) =
                    (arena.computation(left_id), arena.computation(right_id))
                else {
                    sink.record(ConversionDecision::Postpone {
                        constant: TraceId::Comp(left_id),
                    });
                    return Convertibility::Distinct;
                };
                match (left, right) {
                    | (
                        &Computation::Lambda(ref one_body),
                        &Computation::Lambda(ref other_body),
                    ) => {
                        stack.push(TermGoal::Comp(*one_body, *other_body));
                    },
                    | (
                        &Computation::Application(ref one_head, ref one_arg),
                        &Computation::Application(ref other_head, ref other_arg),
                    ) => {
                        stack.push(TermGoal::Comp(*one_head, *other_head));
                        stack.push(TermGoal::Value(*one_arg, *other_arg));
                    },
                    | (&Computation::Return(ref one), &Computation::Return(ref other)) => {
                        stack.push(TermGoal::Value(*one, *other));
                    },
                    | (&Computation::Force(ref one), &Computation::Force(ref other)) => {
                        sink.record(ConversionDecision::Force {
                            thunk: TraceId::Value(*one),
                        });
                        sink.record(ConversionDecision::Force {
                            thunk: TraceId::Value(*other),
                        });
                        stack.push(TermGoal::Value(*one, *other));
                    },
                    | (
                        &Computation::Bind(ref one_bound, ref one_body),
                        &Computation::Bind(ref other_bound, ref other_body),
                    ) => {
                        stack.push(TermGoal::Comp(*one_bound, *other_bound));
                        stack.push(TermGoal::Comp(*one_body, *other_body));
                    },
                    | (
                        &Computation::Case {
                            scrutinee: ref one_scrutinee,
                            on_left: ref one_left,
                            on_right: ref one_right,
                        },
                        &Computation::Case {
                            scrutinee: ref other_scrutinee,
                            on_left: ref other_left,
                            on_right: ref other_right,
                        },
                    ) => {
                        stack.push(TermGoal::Value(*one_scrutinee, *other_scrutinee));
                        stack.push(TermGoal::Comp(*one_left, *other_left));
                        stack.push(TermGoal::Comp(*one_right, *other_right));
                    },
                    | _ => {
                        sink.record(ConversionDecision::Postpone {
                            constant: TraceId::Comp(left_id),
                        });
                        return Convertibility::Distinct;
                    },
                }
            },
        }
    }
    Convertibility::Convertible
}

/// The allocation-backed sink used only while replaying a session trace.
#[repr(transparent)]
struct RecordingSink
{
    /// Decisions emitted by the replayed worklist.
    decisions: Vec<ConversionDecision<TraceId>>,
}

impl RecordingSink
{
    /// Creates an empty recording sink.
    fn new() -> Self
    {
        Self {
            decisions: Vec::new(),
        }
    }
}

impl TraceSink<TraceId> for RecordingSink
{
    #[inline]
    fn record(
        &mut self,
        decision: ConversionDecision<TraceId>,
    )
    {
        self.decisions.push(decision);
    }
}

/// Whether two decision kinds agree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DecisionMatch
{
    /// The decision kinds agree.
    Same,
    /// The decision kinds differ.
    Different,
}
/// Compares two traces by decision kind, ignoring consumer-owned identities.
fn same_decision_kind<Id>(
    expected: &ConversionDecision<Id>,
    actual: &ConversionDecision<TraceId>,
) -> DecisionMatch
{
    match (expected, actual) {
        | (&ConversionDecision::Unfold { .. }, &ConversionDecision::Unfold { .. })
        | (&ConversionDecision::Postpone { .. }, &ConversionDecision::Postpone { .. })
        | (&ConversionDecision::Force { .. }, &ConversionDecision::Force { .. })
        | (
            &ConversionDecision::ComparedShared { .. },
            &ConversionDecision::ComparedShared { .. },
        ) => DecisionMatch::Same,
        | _ => DecisionMatch::Different,
    }
}

/// Replays a term-conversion worklist and checks its decision grain and
/// verdict.
fn replay_terms<Id>(
    arena: &TermArena,
    initial: TermGoal,
    trace: &[ConversionDecision<Id>],
    claimed: Convertibility,
) -> ReplayStatus
{
    let mut sink = RecordingSink::new();
    let actual = converge_terms(arena, initial, &mut sink);
    let trace_matches = trace.len() == sink.decisions.len()
        && trace
            .iter()
            .zip(sink.decisions.iter())
            .all(|(expected, actual)| same_decision_kind(expected, actual) == DecisionMatch::Same);
    if !trace_matches {
        return ReplayStatus::TraceMismatch;
    }
    if actual != claimed {
        return ReplayStatus::VerdictMismatch;
    }
    ReplayStatus::Green
}

/// Re-executes kernel value conversion and compares its decision grain and
/// verdict with a supplied session trace.
///
/// Trace identities are intentionally compared by decision kind. The engine
/// and kernel own different arenas, and this session seam has no wire format or
/// identity translation layer.
#[inline]
#[must_use]
pub fn replay_values<Id>(
    arena: &TermArena,
    left: ValueId,
    right: ValueId,
    trace: &[ConversionDecision<Id>],
    claimed: Convertibility,
) -> ReplayStatus
{
    replay_terms(arena, TermGoal::Value(left, right), trace, claimed)
}

/// Re-executes kernel computation conversion and compares its decision grain
/// and verdict with a supplied session trace.
#[inline]
#[must_use]
pub fn replay_computations<Id>(
    arena: &TermArena,
    left: ComputationId,
    right: ComputationId,
    trace: &[ConversionDecision<Id>],
    claimed: Convertibility,
) -> ReplayStatus
{
    replay_terms(arena, TermGoal::Comp(left, right), trace, claimed)
}

#[cfg(test)]
mod tests
{
    use alloc::string::String;

    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;

    use super::Convertibility;
    use super::convertible_comp_types;
    use super::convertible_computations;
    use super::convertible_value_types;
    use super::convertible_values;
    use crate::arena::TermArena;
    use crate::base::BaseType;
    use crate::base::FractionDigits;
    use crate::base::IntegerLiteral;
    use crate::base::Literal;
    use crate::base::Magnitude;
    use crate::base::Sign;
    use crate::term::ConstantIndex;
    use crate::term::DeBruijnIndex;
    use crate::term::Side;

    #[test]
    fn value_type_conversion_is_reflexive()
    {
        // Product(Integer, Thunk(F Unit)) — reflexivity on distinct ids by
        // structural walk (the two arguments are the same root id, so the
        // top-level early-out also fires; children below it are walked).
        let mut arena = TermArena::new();
        let integer = arena.value_type_base(BaseType::Integer);
        let unit = arena.value_type_unit();
        let returner = arena.comp_type_returner(unit);
        let thunk = arena.value_type_thunk(returner);
        let product = arena.value_type_product(integer, thunk);
        assert_eq!(
            Convertibility::Convertible,
            convertible_value_types(&arena, product, product),
            "a value type converts with itself"
        );
    }

    #[test]
    fn value_type_conversion_separates_distinct_types()
    {
        let mut arena = TermArena::new();
        let integer = arena.value_type_base(BaseType::Integer);
        let string = arena.value_type_base(BaseType::String);
        assert_eq!(
            Convertibility::Distinct,
            convertible_value_types(&arena, integer, string),
            "distinct base atoms do not convert"
        );
        let universe0 = arena.value_type_universe(Level::constant(LevelConstant::from(0_u64)));
        let universe1 = arena.value_type_universe(Level::constant(LevelConstant::from(1_u64)));
        assert_eq!(
            Convertibility::Distinct,
            convertible_value_types(&arena, universe0, universe1),
            "universes at distinct levels do not convert"
        );
    }

    /// Two atoms minted by two sealings do not convert, however alike their
    /// implementations were.
    ///
    /// This is generativity at the conversion wall: nothing about how a sealed
    /// module was built reaches here, so two sealings of the *same*
    /// implementation still yield types the kernel keeps apart.
    #[test]
    fn distinct_sealed_atoms_do_not_convert()
    {
        let mut arena = TermArena::new();
        let first = arena.value_type_abstract(ConstantIndex::from(0_usize));
        let second = arena.value_type_abstract(ConstantIndex::from(1_usize));
        assert_eq!(
            Convertibility::Distinct,
            convertible_value_types(&arena, first, second),
            "atoms naming different declarations do not convert (generativity)"
        );
    }

    /// One atom converts with itself through two separately-minted nodes — the
    /// rule is the *position*, not the arena id.
    #[test]
    fn one_sealed_atom_converts_with_itself()
    {
        let mut arena = TermArena::new();
        let one = arena.value_type_abstract(ConstantIndex::from(3_usize));
        let other = arena.value_type_abstract(ConstantIndex::from(3_usize));
        assert_ne!(one, other, "the two nodes are distinct arena ids");
        assert_eq!(
            Convertibility::Convertible,
            convertible_value_types(&arena, one, other),
            "two references to one atom convert"
        );
    }

    /// **The abstraction-leak refutation.** An atom does not convert with the
    /// type a sealed module might have implemented it by, in either direction.
    ///
    /// There is no unfolding rule to try, so this is not a check that failed to
    /// find a representation — it is that the conversion vocabulary has no arm
    /// relating an atom to a structural type at all. A client that guesses the
    /// representation gets a type error, not a proof.
    #[test]
    fn a_sealed_atom_never_converts_with_a_representation()
    {
        let mut arena = TermArena::new();
        let atom = arena.value_type_abstract(ConstantIndex::from(0_usize));
        let integer = arena.value_type_base(BaseType::Integer);
        let unit = arena.value_type_unit();
        for representation in [integer, unit] {
            assert_eq!(
                Convertibility::Distinct,
                convertible_value_types(&arena, atom, representation),
                "an atom does not convert with a candidate representation"
            );
            assert_eq!(
                Convertibility::Distinct,
                convertible_value_types(&arena, representation, atom),
                "nor does a candidate representation convert with the atom"
            );
        }
    }

    #[test]
    fn distinct_ids_for_equal_structure_still_convert()
    {
        // Two separately-minted Unit-typed nodes have distinct ids; the walk
        // resolves both and agrees structurally (the kernel preserves sharing,
        // never creates it, so equal structure need not share an id).
        let mut arena = TermArena::new();
        let one = arena.value_type_unit();
        let other = arena.value_type_unit();
        assert_ne!(one, other, "distinct mints have distinct ids");
        assert_eq!(
            Convertibility::Convertible,
            convertible_value_types(&arena, one, other),
            "structurally equal but differently-allocated types convert"
        );
    }

    #[test]
    fn computation_type_conversion_is_reflexive()
    {
        let mut arena = TermArena::new();
        let unit = arena.value_type_unit();
        let numeric = arena.value_type_base(BaseType::Numeric);
        let returner = arena.comp_type_returner(numeric);
        let arrow = arena.comp_type_arrow(unit, returner);
        assert_eq!(
            Convertibility::Convertible,
            convertible_comp_types(&arena, arrow, arrow),
            "a computation type converts with itself"
        );
    }

    #[test]
    fn value_conversion_ignores_literal_padding()
    {
        let mut arena = TermArena::new();
        let padded = arena.value_literal(Literal::Integer(IntegerLiteral::new(
            Sign::NonNegative,
            Magnitude::from_decimal_text(String::from("007")).unwrap(),
        )));
        let bare = arena.value_literal(Literal::Integer(IntegerLiteral::new(
            Sign::NonNegative,
            Magnitude::from_decimal_text(String::from("7")).unwrap(),
        )));
        assert_eq!(
            Convertibility::Convertible,
            convertible_values(&arena, padded, bare),
            "literals convert up to canonical value, not spelling"
        );
    }

    #[test]
    fn value_conversion_distinguishes_injection_sides()
    {
        let mut arena = TermArena::new();
        let unit_left = arena.value_unit();
        let unit_right = arena.value_unit();
        let left = arena.value_injection(Side::Left, unit_left);
        let right = arena.value_injection(Side::Right, unit_right);
        assert_eq!(
            Convertibility::Distinct,
            convertible_values(&arena, left, right),
            "the two injections of a sum are distinct"
        );
    }

    #[test]
    fn value_conversion_is_variable_sensitive()
    {
        let mut arena = TermArena::new();
        let zero = arena.value_variable(DeBruijnIndex::from(0_u32));
        let one = arena.value_variable(DeBruijnIndex::from(1_u32));
        assert_eq!(
            Convertibility::Distinct,
            convertible_values(&arena, zero, one),
            "distinct de Bruijn variables are distinct"
        );
    }

    #[test]
    fn computation_conversion_is_alpha_structural()
    {
        // Two lambdas over structurally identical bodies convert with no name
        // comparison (de Bruijn α is syntactic).
        let mut arena = TermArena::new();
        let variable_one = arena.value_variable(DeBruijnIndex::from(0_u32));
        let return_one = arena.computation_return(variable_one);
        let identity = arena.computation_lambda(return_one);
        let variable_two = arena.value_variable(DeBruijnIndex::from(0_u32));
        let return_two = arena.computation_return(variable_two);
        let same = arena.computation_lambda(return_two);
        assert_eq!(
            Convertibility::Convertible,
            convertible_computations(&arena, identity, same),
            "α-equal lambdas convert"
        );
        let numeric = arena.value_literal(Literal::Numeric(crate::base::NumericLiteral::new(
            Sign::NonNegative,
            Magnitude::zero(),
            FractionDigits::none(),
        )));
        let return_numeric = arena.computation_return(numeric);
        let different = arena.computation_lambda(return_numeric);
        assert_eq!(
            Convertibility::Distinct,
            convertible_computations(&arena, identity, different),
            "lambdas with distinct bodies do not convert"
        );
    }
    #[test]
    fn conversion_trace_replays_decisions_and_verdict()
    {
        let mut arena = TermArena::new();
        let value = arena.value_unit();
        let mut sink = super::RecordingSink::new();
        let verdict = super::convertible_values_with_sink(&arena, value, value, &mut sink);
        assert_eq!(Convertibility::Convertible, verdict);
        assert_eq!(
            super::ReplayStatus::Green,
            super::replay_values(&arena, value, value, &sink.decisions, verdict),
        );
        assert_eq!(
            super::ReplayStatus::VerdictMismatch,
            super::replay_values(
                &arena,
                value,
                value,
                &sink.decisions,
                Convertibility::Distinct,
            ),
        );
        assert_eq!(verdict, super::convertible_values(&arena, value, value));
    }
}
