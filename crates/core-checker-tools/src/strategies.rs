//! Shared proptest generators for the gandr core syntax and types.
//!
//! These are the *free* (input-free / random / grammar-directed) generators —
//! grades, binder names, leaf and recursive value/computation types, and hole
//! identifiers. Being grammar-directed rather than type-directed, they produce
//! mostly ill-typed terms, which is what the agreement properties want.
//!
//! Three consumers share them: this crate's own conformance suite,
//! `gandr-core-checker`'s inline property tests, and `gandr-surface-engine`'s
//! edit and totality suites. That spread is why they are an ordinary library
//! module of a crate one tier above the checker rather than a feature-gated
//! module inside it.
//!
//! The *type-directed, well-typed* generators stay beside the conformance
//! suite that drives them: they are coupled to that harness and have no second
//! consumer to justify promoting them here.

use gandr_core_term::boundary::GenerationDepth;
use gandr_core_term::grade::Grade;
use gandr_core_term::types::CompType;
use gandr_core_term::types::ValueType;
use proptest::prelude::*;

/// A small pool of binder names (deliberately overlapping, to exercise
/// shadowing).
///
/// Includes the base-scope names `i`/`s` so that a generated binder can
/// *shadow* a base hypothesis (e.g. `λi:Str. …` rebinds `i` away from its
/// base `Int`), exercising the innermost-wins lookup discipline.
#[inline]
pub fn binder_name() -> impl Strategy<Value = String>
{
    prop_oneof![
        Just("x".to_owned()),
        Just("y".to_owned()),
        Just("z".to_owned()),
        Just("i".to_owned()),
        Just("s".to_owned()),
    ]
}

/// Arbitrary value types up to a depth.
#[inline]
/// # Termination
/// - reason: property strategies mirror finite grammar productions.
/// - measure: generation depth fuel.
/// - boundedness: finite `u32` fuel reaches leaf strategies at zero.
/// - input recursion: none.
pub fn arb_value_type<D>(depth: D) -> BoxedStrategy<ValueType>
where
    D: Into<GenerationDepth>,
{
    let (strategy, _) = type_strategies(depth.into());
    strategy
}

/// Arbitrary computation types up to a depth (including the A2.2 `Unknown`).
#[inline]
/// # Termination
/// - reason: property strategies mirror finite grammar productions.
/// - measure: generation depth fuel.
/// - boundedness: finite `u32` fuel reaches leaf strategies at zero.
/// - input recursion: none.
pub fn arb_comp_type<D>(depth: D) -> BoxedStrategy<CompType>
where
    D: Into<GenerationDepth>,
{
    let (_, strategy) = type_strategies(depth.into());
    strategy
}

/// Build value/computation type strategies by folding depth layers on the heap.
#[inline]
fn type_strategies(depth: GenerationDepth) -> (BoxedStrategy<ValueType>, BoxedStrategy<CompType>)
{
    let mut value = leaf_value_type().boxed();
    let mut comp = prop_oneof![
        leaf_value_type().prop_map(CompType::returner),
        Just(CompType::Unknown),
    ]
    .boxed();
    for _ in 0 .. u32::from(depth) {
        let sub_value = value.clone();
        let sub_comp = comp.clone();
        value = prop_oneof![
            leaf_value_type(),
            (sub_value.clone(), sub_value.clone()).prop_map(|(fst, snd)| ValueType::prod(fst, snd)),
            (sub_value.clone(), sub_value.clone()).prop_map(|(lhs, rhs)| ValueType::sum(lhs, rhs)),
            sub_value.clone().prop_map(ValueType::list),
            proptest::collection::btree_map(record_label(), sub_value.clone(), 0 ..= 3)
                .prop_map(ValueType::record),
            (any_grade(), sub_comp.clone()).prop_map(|(grade, body)| ValueType::thunk(grade, body)),
            (sub_comp.clone(), sub_comp.clone()).prop_map(|(b, c)| ValueType::stk(b, c)),
        ]
        .boxed();
        comp = prop_oneof![
            sub_value.clone().prop_map(CompType::returner),
            Just(CompType::Unknown),
            (sub_value, sub_comp.clone()).prop_map(|(arg, res)| CompType::arrow(arg, res)),
            (sub_comp.clone(), sub_comp).prop_map(|(fst, snd)| CompType::with(fst, snd)),
        ]
        .boxed();
    }
    (value, comp)
}

/// Leaf value types: the atoms of the base scope, the literal-realizable
/// `Integer` atom, unit, and the A2.2 `Unknown` (whose depth-0 realizer is
/// the hole, a universal check-mode inhabitant).
#[inline]
pub fn leaf_value_type() -> impl Strategy<Value = ValueType>
{
    prop_oneof![
        Just(ValueType::Unit),
        Just(int()),
        Just(txt()),
        Just(integer()),
        Just(string()),
        numeric_atom(),
        Just(ValueType::Unknown),
    ]
}

/// The `Int` atom.
#[inline]
#[must_use]
pub fn int() -> ValueType
{
    ValueType::atom("Int")
}

/// The `Str` atom.
#[inline]
#[must_use]
pub fn txt() -> ValueType
{
    ValueType::atom("Str")
}

/// The `Integer` atom: the type of integer literals (A2.1 literals
/// extension). Realized by `Value::Int` directly, so unlike `Int`/`Str` it
/// needs no base-scope realizer.
#[inline]
#[must_use]
pub fn integer() -> ValueType
{
    ValueType::integer()
}

/// The `String` atom: the type of string literals (value-model ladder, ADR-38).
///
/// Realized by `Value::Str` directly, the string analogue of [`integer`];
/// distinct from the arbitrary base-scope [`txt`] atom `Str`.
#[inline]
#[must_use]
pub fn string() -> ValueType
{
    ValueType::string()
}

/// A numeric primitive atom, chosen uniformly (value-model ladder, ADR-39).
///
/// One of `u32`/`u64`/`i32`/`i64`/`f32`/`f64`. Each is realized directly by a
/// suitable suffixed `Value::Num` literal (the numeric analogue of [`integer`]
/// / [`string`]).
#[inline]
pub fn numeric_atom() -> impl Strategy<Value = ValueType>
{
    prop_oneof![
        Just(ValueType::u32()),
        Just(ValueType::u64()),
        Just(ValueType::i32()),
        Just(ValueType::i64()),
        Just(ValueType::f32()),
        Just(ValueType::f64()),
    ]
}

/// A small pool of record field labels (value-model ladder, ADR-45).
///
/// Deliberately tiny (three labels) so that independently generated record
/// types overlap on labels — exercising width subtyping (`{a, b} <: {a}`) and
/// depth subtyping (a shared label's field types relating) rather than always
/// being disjoint.
#[inline]
pub fn record_label() -> impl Strategy<Value = String>
{
    prop_oneof![
        Just("a".to_owned()),
        Just("b".to_owned()),
        Just("c".to_owned()),
    ]
}

/// Strategies for grades.
#[inline]
pub fn any_grade() -> impl Strategy<Value = Grade>
{
    prop_oneof![
        Just(Grade::ZERO),
        Just(Grade::ONE),
        Just(Grade::fin(gandr_core_term::boundary::GradeBound::from(
            3_u64
        ))),
        Just(Grade::OMEGA),
    ]
}

/// An arbitrary hole identifier (typing must ignore it; drawing more than
/// one value pins that).
#[inline]
pub fn hole_id() -> impl Strategy<Value = u32>
{
    0_u32 .. 4_u32
}
