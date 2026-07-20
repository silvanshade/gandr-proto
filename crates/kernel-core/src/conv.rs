//! Definitional equality (conversion) for the S1 fragment — the C5-quarantined
//! conversion record (kernel-boundary.md §6 C5).
//!
//! # The S1 conversion algorithm, precisely
//!
//! The checker's conversion runs at a **mode switch** — when a synthesizing
//! term is used where a type is expected, its synthesized type must convert
//! against the expected type. At S1 the definitional equality this needs is
//! **type conversion only**, and it is exactly:
//!
//! * α-structural equality — terms are nameless (de Bruijn), so α-equivalence
//!   is syntactic identity, and types carry no binders;
//! * with `gandr_kernel_strata::Level` **canonical equality** at
//!   [`crate::types::ValueType::Universe`] and
//!   [`crate::types::ValueType::Lift`] (the strata `Level` type's derived
//!   equality **is** the ADR-78 level-equality oracle);
//! * structurally through `Product`, `Sum`, `Thunk`, `Arrow`, and `Returner`.
//!
//! **No β law fires and no computation is evaluated.** At S1 no value-type or
//! computation-type former is indexed by a value term, so type conversion
//! never descends into a term — the C5 quarantine ("the kernel types
//! computations but never evaluates them during conversion") holds
//! **vacuously** at S1, because there is nothing to evaluate. This coincidence
//! with plain structural equality is not permanent: once a term-indexed type
//! former lands (an `El`/description code carrying a value into a type, S2+),
//! values will appear in types and the β laws of the value fragment will become
//! load-bearing here.
//!
//! # The value-fragment conversion (present, quarantined)
//!
//! [`convertible_values`] / [`convertible_computations`] provide the
//! α-structural equality of the value/computation term fragment — the fragment
//! C5 permits conversion to compare. At S1 they carry no β law (the value
//! fragment forces none — the only value embedding a computation is a thunk,
//! which suspends rather than reduces) and are **not invoked by the checker**
//! (types hold no terms); they are the seed the term-indexed extensions will
//! grow. They never evaluate a computation — a `Thunk`/`Force` pair is compared
//! structurally, never run.
//!
//! # Totality
//!
//! Every face is **iterative** over an explicit heap worklist, not recursive:
//! the relation coincides with the derived structural equality, but the derived
//! `PartialEq` recurses on type/term depth and would overflow the stack on an
//! adversarial-depth input. The worklist keeps conversion total (the kernel
//! never diverges — docs/workflow/rust.md).

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::error::ComputationTypeMismatch;
use crate::error::KernelError;
use crate::error::ValueTypeMismatch;
use crate::term::Computation;
use crate::term::Value;
use crate::types::CompType;
use crate::types::ValueType;

/// Whether two types or two terms are convertible (definitionally equal).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    clippy::exhaustive_enums,
    reason = "conversion is a two-valued judgment by design"
)]
pub enum Convertibility
{
    /// The two are definitionally equal.
    Convertible,
    /// The two are distinct.
    Distinct,
}

/// A pending type-conversion obligation: a same-polarity pair still to compare.
enum TypeGoal<'types>
{
    /// A pair of value types.
    Value(&'types ValueType, &'types ValueType),
    /// A pair of computation types.
    Comp(&'types CompType, &'types CompType),
}

/// A pending term-conversion obligation: a same-polarity pair still to compare.
enum TermGoal<'terms>
{
    /// A pair of values.
    Value(&'terms Value, &'terms Value),
    /// A pair of computations.
    Comp(&'terms Computation, &'terms Computation),
}

/// Decide convertibility of two value types.
///
/// # Contract
/// - requires: nothing (any two value types are comparable).
/// - ensures: [`Convertibility::Convertible`] exactly when the two are equal up
///   to the S1 definitional equality (α-structural with canonical levels); the
///   relation is reflexive, symmetric, and transitive.
/// - provides: the type equality the checker invokes at a value mode switch.
/// - fails: never — a negative answer is [`Convertibility::Distinct`], not a
///   failure.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — a boundary-biased differential against the crate's own
///   generator confirms it agrees with reflexive equality and separates
///   distinct types on every generated pair; the L3 residue is the level-node
///   comparison (canonical `Level` equality, not syntactic), pinned by a lift
///   whose targets differ only by canonical form.
/// - witness: `conv::tests::value_type_conversion_is_reflexive`
/// - witness: `conv::tests::value_type_conversion_separates_distinct_types`
/// - witness: `conversion::tests::prop_value_type_conversion_is_reflexive`
#[inline]
#[must_use]
pub fn convertible_value_types(
    left: &ValueType,
    right: &ValueType,
) -> Convertibility
{
    converge_types(TypeGoal::Value(left, right))
}

/// Decide convertibility of two computation types.
///
/// # Contract
/// - requires: nothing.
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
///   children (domain value type, codomain computation type), pinned by a pair
///   differing in exactly one child.
/// - witness: `conv::tests::computation_type_conversion_is_reflexive`
/// - witness: `conversion::tests::prop_computation_type_conversion_is_reflexive`
#[inline]
#[must_use]
pub fn convertible_comp_types(
    left: &CompType,
    right: &CompType,
) -> Convertibility
{
    converge_types(TypeGoal::Comp(left, right))
}

/// Decide α-structural convertibility of two values (the C5 value fragment).
///
/// # Contract
/// - requires: nothing.
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
    left: &Value,
    right: &Value,
) -> Convertibility
{
    converge_terms(TermGoal::Value(left, right))
}

/// Decide α-structural convertibility of two computations (the C5 fragment).
///
/// # Contract
/// - requires: nothing.
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
///   bodies do, with no name comparison), pinned by a lambda-body pair.
/// - witness: `conv::tests::computation_conversion_is_alpha_structural`
/// - witness: `conversion::tests::prop_computation_conversion_is_reflexive`
#[inline]
#[must_use]
pub fn convertible_computations(
    left: &Computation,
    right: &Computation,
) -> Convertibility
{
    converge_terms(TermGoal::Comp(left, right))
}

/// Convert a synthesized value type against an expected one, building the
/// mismatch error from the two roots on divergence.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(())` exactly when [`convertible_value_types`] converges.
/// - provides: the checker's value mode-switch step.
/// - fails: [`KernelError::ValueTypeMismatch`] carrying the two root types.
/// - panics: none.
///
/// # Errors
/// [`KernelError::ValueTypeMismatch`].
#[inline]
pub fn convert_value_type(
    expected: &ValueType,
    actual: &ValueType,
) -> Result<(), KernelError>
{
    match convertible_value_types(expected, actual) {
        | Convertibility::Convertible => Ok(()),
        | Convertibility::Distinct => Err(KernelError::ValueTypeMismatch(Box::new(
            ValueTypeMismatch::new(expected.clone(), actual.clone()),
        ))),
    }
}

/// Convert a synthesized computation type against an expected one, building the
/// mismatch error from the two roots on divergence.
///
/// # Contract
/// - requires: nothing.
/// - ensures: `Ok(())` exactly when [`convertible_comp_types`] converges.
/// - provides: the checker's computation mode-switch step.
/// - fails: [`KernelError::ComputationTypeMismatch`] carrying the two roots.
/// - panics: none.
///
/// # Errors
/// [`KernelError::ComputationTypeMismatch`].
#[inline]
pub fn convert_comp_type(
    expected: &CompType,
    actual: &CompType,
) -> Result<(), KernelError>
{
    match convertible_comp_types(expected, actual) {
        | Convertibility::Convertible => Ok(()),
        | Convertibility::Distinct => Err(KernelError::ComputationTypeMismatch(Box::new(
            ComputationTypeMismatch::new(expected.clone(), actual.clone()),
        ))),
    }
}

/// Run the type-conversion worklist to a verdict.
///
/// # Contract
/// - requires: `initial` is a same-polarity pair.
/// - ensures: [`Convertibility::Convertible`] exactly when every reachable
///   sub-pair matches structurally with canonical-level equality at level
///   nodes; the walk is iterative over a heap stack, so it is total on any
///   depth.
/// - provides: the shared engine of the type-conversion faces.
/// - fails: never (the verdict is the return value).
/// - panics: none.
#[inline]
fn converge_types(initial: TypeGoal<'_>) -> Convertibility
{
    let mut stack: Vec<TypeGoal<'_>> = Vec::new();
    stack.push(initial);
    while let Some(goal) = stack.pop() {
        match goal {
            | TypeGoal::Value(left, right) => match (left, right) {
                | (&ValueType::Base(ref one), &ValueType::Base(ref other)) => {
                    if one != other {
                        return Convertibility::Distinct;
                    }
                },
                | (&ValueType::Unit, &ValueType::Unit) => {},
                | (
                    &ValueType::Product(ref one_first, ref one_second),
                    &ValueType::Product(ref other_first, ref other_second),
                ) => {
                    stack.push(TypeGoal::Value(one_first, other_first));
                    stack.push(TypeGoal::Value(one_second, other_second));
                },
                | (
                    &ValueType::Sum(ref one_left, ref one_right),
                    &ValueType::Sum(ref other_left, ref other_right),
                ) => {
                    stack.push(TypeGoal::Value(one_left, other_left));
                    stack.push(TypeGoal::Value(one_right, other_right));
                },
                | (&ValueType::Thunk(ref one_body), &ValueType::Thunk(ref other_body)) => {
                    stack.push(TypeGoal::Comp(one_body, other_body));
                },
                | (&ValueType::Universe(ref one_level), &ValueType::Universe(ref other_level)) => {
                    if one_level != other_level {
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
                    stack.push(TypeGoal::Value(one_inner, other_inner));
                },
                | _ => return Convertibility::Distinct,
            },
            | TypeGoal::Comp(left, right) => match (left, right) {
                | (&CompType::Returner(ref one), &CompType::Returner(ref other)) => {
                    stack.push(TypeGoal::Value(one, other));
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
                    stack.push(TypeGoal::Value(one_domain, other_domain));
                    stack.push(TypeGoal::Comp(one_codomain, other_codomain));
                },
                | _ => return Convertibility::Distinct,
            },
        }
    }
    Convertibility::Convertible
}

/// Run the term-conversion (α-equivalence) worklist to a verdict.
///
/// # Contract
/// - requires: `initial` is a same-polarity pair.
/// - ensures: [`Convertibility::Convertible`] exactly when the two terms are
///   α-equal — syntactic identity under de Bruijn, with canonical literal
///   equality at leaves and injection-side sensitivity; nothing is evaluated.
/// - provides: the shared engine of the term-conversion faces.
/// - fails: never.
/// - panics: none.
#[inline]
fn converge_terms(initial: TermGoal<'_>) -> Convertibility
{
    let mut stack: Vec<TermGoal<'_>> = Vec::new();
    stack.push(initial);
    while let Some(goal) = stack.pop() {
        match goal {
            | TermGoal::Value(left, right) => {
                match (left, right) {
                    | (&Value::Variable(_), &Value::Variable(_))
                    | (&Value::Constant(_), &Value::Constant(_))
                    | (&Value::Unit, &Value::Unit)
                    | (&Value::Literal(_), &Value::Literal(_)) => {
                        // Leaf values convert by direct (non-recursive) equality
                        // on the whole node.
                        if left != right {
                            return Convertibility::Distinct;
                        }
                    },
                    | (
                        &Value::Pair(ref one_first, ref one_second),
                        &Value::Pair(ref other_first, ref other_second),
                    ) => {
                        stack.push(TermGoal::Value(one_first, other_first));
                        stack.push(TermGoal::Value(one_second, other_second));
                    },
                    | (
                        &Value::Injection(one_side, ref one_body),
                        &Value::Injection(other_side, ref other_body),
                    ) => {
                        if one_side != other_side {
                            return Convertibility::Distinct;
                        }
                        stack.push(TermGoal::Value(one_body, other_body));
                    },
                    | (&Value::Thunk(ref one_body), &Value::Thunk(ref other_body)) => {
                        stack.push(TermGoal::Comp(one_body, other_body));
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
                            return Convertibility::Distinct;
                        }
                        stack.push(TermGoal::Value(one_body, other_body));
                    },
                    | _ => return Convertibility::Distinct,
                }
            },
            | TermGoal::Comp(left, right) => match (left, right) {
                | (&Computation::Lambda(ref one_body), &Computation::Lambda(ref other_body)) => {
                    stack.push(TermGoal::Comp(one_body, other_body));
                },
                | (
                    &Computation::Application(ref one_head, ref one_arg),
                    &Computation::Application(ref other_head, ref other_arg),
                ) => {
                    stack.push(TermGoal::Comp(one_head, other_head));
                    stack.push(TermGoal::Value(one_arg, other_arg));
                },
                | (&Computation::Return(ref one), &Computation::Return(ref other))
                | (&Computation::Force(ref one), &Computation::Force(ref other)) => {
                    stack.push(TermGoal::Value(one, other));
                },
                | (
                    &Computation::Bind(ref one_bound, ref one_body),
                    &Computation::Bind(ref other_bound, ref other_body),
                ) => {
                    stack.push(TermGoal::Comp(one_bound, other_bound));
                    stack.push(TermGoal::Comp(one_body, other_body));
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
                    stack.push(TermGoal::Value(one_scrutinee, other_scrutinee));
                    stack.push(TermGoal::Comp(one_left, other_left));
                    stack.push(TermGoal::Comp(one_right, other_right));
                },
                | _ => return Convertibility::Distinct,
            },
        }
    }
    Convertibility::Convertible
}

#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;
    use alloc::string::String;

    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelConstant;

    use super::Convertibility;
    use super::convertible_comp_types;
    use super::convertible_computations;
    use super::convertible_value_types;
    use super::convertible_values;
    use crate::base::BaseType;
    use crate::base::FractionDigits;
    use crate::base::IntegerLiteral;
    use crate::base::Literal;
    use crate::base::Magnitude;
    use crate::base::Sign;
    use crate::term::Computation;
    use crate::term::DeBruijnIndex;
    use crate::term::Side;
    use crate::term::Value;
    use crate::types::CompType;
    use crate::types::ValueType;

    /// The universe former at constant level `value`.
    fn universe(value: u64) -> ValueType
    {
        ValueType::Universe(Level::constant(LevelConstant::from(value)))
    }

    #[test]
    fn value_type_conversion_is_reflexive()
    {
        let ty = ValueType::Product(
            Box::new(ValueType::Base(BaseType::Integer)),
            Box::new(ValueType::Thunk(Box::new(CompType::Returner(Box::new(
                ValueType::Unit,
            ))))),
        );
        assert_eq!(
            Convertibility::Convertible,
            convertible_value_types(&ty, &ty),
            "a value type converts with itself"
        );
    }

    #[test]
    fn value_type_conversion_separates_distinct_types()
    {
        let integer = ValueType::Base(BaseType::Integer);
        let string = ValueType::Base(BaseType::String);
        assert_eq!(
            Convertibility::Distinct,
            convertible_value_types(&integer, &string),
            "distinct base atoms do not convert"
        );
        assert_eq!(
            Convertibility::Distinct,
            convertible_value_types(&universe(0), &universe(1)),
            "universes at distinct levels do not convert"
        );
    }

    #[test]
    fn computation_type_conversion_is_reflexive()
    {
        let ty = CompType::Arrow {
            domain: Box::new(ValueType::Unit),
            codomain: Box::new(CompType::Returner(Box::new(ValueType::Base(
                BaseType::Numeric,
            )))),
        };
        assert_eq!(
            Convertibility::Convertible,
            convertible_comp_types(&ty, &ty),
            "a computation type converts with itself"
        );
    }

    #[test]
    fn value_conversion_ignores_literal_padding()
    {
        let padded = Value::Literal(Literal::Integer(IntegerLiteral::new(
            Sign::NonNegative,
            Magnitude::from_decimal_text(String::from("007")).unwrap(),
        )));
        let bare = Value::Literal(Literal::Integer(IntegerLiteral::new(
            Sign::NonNegative,
            Magnitude::from_decimal_text(String::from("7")).unwrap(),
        )));
        assert_eq!(
            Convertibility::Convertible,
            convertible_values(&padded, &bare),
            "literals convert up to canonical value, not spelling"
        );
    }

    #[test]
    fn value_conversion_distinguishes_injection_sides()
    {
        let left = Value::Injection(Side::Left, Box::new(Value::Unit));
        let right = Value::Injection(Side::Right, Box::new(Value::Unit));
        assert_eq!(
            Convertibility::Distinct,
            convertible_values(&left, &right),
            "the two injections of a sum are distinct"
        );
    }

    #[test]
    fn value_conversion_is_variable_sensitive()
    {
        let zero = Value::Variable(DeBruijnIndex::from(0_u32));
        let one = Value::Variable(DeBruijnIndex::from(1_u32));
        assert_eq!(
            Convertibility::Distinct,
            convertible_values(&zero, &one),
            "distinct de Bruijn variables are distinct"
        );
    }

    #[test]
    fn computation_conversion_is_alpha_structural()
    {
        // Two lambdas over structurally identical bodies convert with no name
        // comparison (de Bruijn α is syntactic).
        let identity = Computation::Lambda(Box::new(Computation::Return(Box::new(
            Value::Variable(DeBruijnIndex::from(0_u32)),
        ))));
        let same = Computation::Lambda(Box::new(Computation::Return(Box::new(Value::Variable(
            DeBruijnIndex::from(0_u32),
        )))));
        assert_eq!(
            Convertibility::Convertible,
            convertible_computations(&identity, &same),
            "α-equal lambdas convert"
        );
        let numeric = Value::Literal(Literal::Numeric(crate::base::NumericLiteral::new(
            Sign::NonNegative,
            Magnitude::zero(),
            FractionDigits::none(),
        )));
        let different = Computation::Lambda(Box::new(Computation::Return(Box::new(numeric))));
        assert_eq!(
            Convertibility::Distinct,
            convertible_computations(&identity, &different),
            "lambdas with distinct bodies do not convert"
        );
    }
}
