//! Shared integration-test support: arena-independent **spec** trees describing
//! S1 terms/types, an **iterative** materializer that builds a spec into a
//! [`TermArena`] (recursion is disallowed even in tests, ADR-47), and proptest
//! strategies over the specs.
//!
//! The D1(C) arena restructure means a term/type only exists as ids in an
//! arena. Tests describe structure with the owned spec trees below, then
//! materialize them into whichever arena they need (a bare [`TermArena`] for
//! conversion, or a declaration's content via [`stage_def`]/[`stage_axiom`]).

use gandr_kernel_core::BaseType;
use gandr_kernel_core::CompTypeId;
use gandr_kernel_core::ComputationId;
use gandr_kernel_core::Declaration;
use gandr_kernel_core::Environment;
use gandr_kernel_core::LevelSignature;
use gandr_kernel_core::Literal;
use gandr_kernel_core::Side;
use gandr_kernel_core::TermArena;
use gandr_kernel_core::ValueId;
use gandr_kernel_core::ValueTypeId;
use gandr_kernel_strata::Level;

/// A description of a value type, independent of any arena.
#[derive(Clone, Debug)]
pub enum ValueTypeSpec
{
    Base(BaseType),
    Unit,
    Universe(Level),
    Product(Box<Self>, Box<Self>),
    Sum(Box<Self>, Box<Self>),
    Thunk(Box<CompTypeSpec>),
    Lift(Box<Self>, Level),
}

/// A description of a computation type.
#[derive(Clone, Debug)]
pub enum CompTypeSpec
{
    Returner(Box<ValueTypeSpec>),
    Arrow(Box<ValueTypeSpec>, Box<Self>),
}

/// A description of a value term.
#[derive(Clone, Debug)]
pub enum ValueSpec
{
    Variable(u32),
    Constant(usize),
    Unit,
    Literal(Literal),
    Pair(Box<Self>, Box<Self>),
    Injection(Side, Box<Self>),
    Thunk(Box<ComputationSpec>),
    Lift(Level, Box<Self>),
}

/// A description of a computation term.
#[derive(Clone, Debug)]
pub enum ComputationSpec
{
    Lambda(Box<Self>),
    Application(Box<Self>, Box<ValueSpec>),
    Return(Box<ValueSpec>),
    Bind(Box<Self>, Box<Self>),
    Force(Box<ValueSpec>),
    Case(Box<ValueSpec>, Box<Self>, Box<Self>),
}

// ----- Iterative type materialization (no input recursion) -----

/// The goal of the iterative type materializer.
enum TypeGoal<'spec>
{
    Value(&'spec ValueTypeSpec),
    Comp(&'spec CompTypeSpec),
}

/// The register of the iterative type materializer (a produced arena id).
#[derive(Clone, Copy)]
enum TypeOut
{
    Value(ValueTypeId),
    Comp(CompTypeId),
}

/// A pending continuation of the iterative type materializer.
enum TypeFrame<'spec>
{
    ProductSecond(&'spec ValueTypeSpec),
    ProductBuild(ValueTypeId),
    SumSecond(&'spec ValueTypeSpec),
    SumBuild(ValueTypeId),
    Thunk,
    Lift(Level),
    Returner,
    ArrowCodomain(&'spec CompTypeSpec),
    ArrowBuild(ValueTypeId),
}

/// The produced value-type id (a mis-materialized comp id falls back to unit).
fn expect_value_type(
    arena: &mut TermArena,
    out: TypeOut,
) -> ValueTypeId
{
    match out {
        | TypeOut::Value(id) => id,
        | TypeOut::Comp(_) => arena.value_type_unit(),
    }
}

/// The produced comp-type id (fallback `F Unit`).
fn expect_comp_type(
    arena: &mut TermArena,
    out: TypeOut,
) -> CompTypeId
{
    match out {
        | TypeOut::Comp(id) => id,
        | TypeOut::Value(_) => {
            let unit = arena.value_type_unit();
            arena.comp_type_returner(unit)
        },
    }
}

/// Materialize a value type spec into `arena`, iteratively.
pub fn materialize_value_type(
    arena: &mut TermArena,
    spec: &ValueTypeSpec,
) -> ValueTypeId
{
    let out = materialize_type(arena, TypeGoal::Value(spec));
    expect_value_type(arena, out)
}

/// Materialize a computation type spec into `arena`, iteratively.
pub fn materialize_comp_type(
    arena: &mut TermArena,
    spec: &CompTypeSpec,
) -> CompTypeId
{
    let out = materialize_type(arena, TypeGoal::Comp(spec));
    expect_comp_type(arena, out)
}

/// The shared iterative engine for type materialization.
fn materialize_type<'spec>(
    arena: &mut TermArena,
    root: TypeGoal<'spec>,
) -> TypeOut
{
    let mut frames: Vec<TypeFrame<'spec>> = Vec::new();
    let mut goal = root;
    'expand: loop {
        let mut produced: TypeOut = match goal {
            | TypeGoal::Value(spec) => match spec {
                | ValueTypeSpec::Base(base) => TypeOut::Value(arena.value_type_base(*base)),
                | ValueTypeSpec::Unit => TypeOut::Value(arena.value_type_unit()),
                | ValueTypeSpec::Universe(level) => {
                    TypeOut::Value(arena.value_type_universe(level.clone()))
                },
                | ValueTypeSpec::Product(first, second) => {
                    frames.push(TypeFrame::ProductSecond(second));
                    goal = TypeGoal::Value(first);
                    continue 'expand;
                },
                | ValueTypeSpec::Sum(first, second) => {
                    frames.push(TypeFrame::SumSecond(second));
                    goal = TypeGoal::Value(first);
                    continue 'expand;
                },
                | ValueTypeSpec::Thunk(body) => {
                    frames.push(TypeFrame::Thunk);
                    goal = TypeGoal::Comp(body);
                    continue 'expand;
                },
                | ValueTypeSpec::Lift(inner, target) => {
                    frames.push(TypeFrame::Lift(target.clone()));
                    goal = TypeGoal::Value(inner);
                    continue 'expand;
                },
            },
            | TypeGoal::Comp(spec) => match spec {
                | CompTypeSpec::Returner(result) => {
                    frames.push(TypeFrame::Returner);
                    goal = TypeGoal::Value(result);
                    continue 'expand;
                },
                | CompTypeSpec::Arrow(domain, codomain) => {
                    frames.push(TypeFrame::ArrowCodomain(codomain));
                    goal = TypeGoal::Value(domain);
                    continue 'expand;
                },
            },
        };
        loop {
            let Some(frame) = frames.pop()
            else {
                return produced;
            };
            match frame {
                | TypeFrame::ProductSecond(second) => {
                    let first = expect_value_type(arena, produced);
                    frames.push(TypeFrame::ProductBuild(first));
                    goal = TypeGoal::Value(second);
                    continue 'expand;
                },
                | TypeFrame::ProductBuild(first) => {
                    let second = expect_value_type(arena, produced);
                    produced = TypeOut::Value(arena.value_type_product(first, second));
                },
                | TypeFrame::SumSecond(second) => {
                    let first = expect_value_type(arena, produced);
                    frames.push(TypeFrame::SumBuild(first));
                    goal = TypeGoal::Value(second);
                    continue 'expand;
                },
                | TypeFrame::SumBuild(first) => {
                    let second = expect_value_type(arena, produced);
                    produced = TypeOut::Value(arena.value_type_sum(first, second));
                },
                | TypeFrame::Thunk => {
                    let body = expect_comp_type(arena, produced);
                    produced = TypeOut::Value(arena.value_type_thunk(body));
                },
                | TypeFrame::Lift(target) => {
                    let inner = expect_value_type(arena, produced);
                    produced = TypeOut::Value(arena.value_type_lift(inner, target));
                },
                | TypeFrame::Returner => {
                    let result = expect_value_type(arena, produced);
                    produced = TypeOut::Comp(arena.comp_type_returner(result));
                },
                | TypeFrame::ArrowCodomain(codomain) => {
                    let domain = expect_value_type(arena, produced);
                    frames.push(TypeFrame::ArrowBuild(domain));
                    goal = TypeGoal::Comp(codomain);
                    continue 'expand;
                },
                | TypeFrame::ArrowBuild(domain) => {
                    let codomain = expect_comp_type(arena, produced);
                    produced = TypeOut::Comp(arena.comp_type_arrow(domain, codomain));
                },
            }
        }
    }
}

// ----- Iterative term materialization (no input recursion) -----

/// The goal of the iterative term materializer.
enum TermGoal<'spec>
{
    Value(&'spec ValueSpec),
    Comp(&'spec ComputationSpec),
}

/// The register of the iterative term materializer.
#[derive(Clone, Copy)]
enum TermOut
{
    Value(ValueId),
    Comp(ComputationId),
}

/// A pending continuation of the iterative term materializer.
enum TermFrame<'spec>
{
    PairSecond(&'spec ValueSpec),
    PairBuild(ValueId),
    Injection(Side),
    Thunk,
    Lift(Level),
    Lambda,
    ApplicationArgument(&'spec ValueSpec),
    ApplicationBuild(ComputationId),
    Return,
    Force,
    BindBody(&'spec ComputationSpec),
    BindBuild(ComputationId),
    CaseScrutineeThenLeft
    {
        on_left: &'spec ComputationSpec,
        on_right: &'spec ComputationSpec,
    },
    CaseLeft
    {
        scrutinee: ValueId,
        on_right: &'spec ComputationSpec,
    },
    CaseRight
    {
        scrutinee: ValueId,
        on_left: ComputationId,
    },
}

/// The produced value id (a mis-materialized comp falls back to unit).
fn expect_value(
    arena: &mut TermArena,
    out: TermOut,
) -> ValueId
{
    match out {
        | TermOut::Value(id) => id,
        | TermOut::Comp(_) => arena.value_unit(),
    }
}

/// The produced comp id (fallback `return unit`).
fn expect_comp(
    arena: &mut TermArena,
    out: TermOut,
) -> ComputationId
{
    match out {
        | TermOut::Comp(id) => id,
        | TermOut::Value(_) => {
            let unit = arena.value_unit();
            arena.computation_return(unit)
        },
    }
}

/// Materialize a value spec into `arena`, iteratively.
pub fn materialize_value(
    arena: &mut TermArena,
    spec: &ValueSpec,
) -> ValueId
{
    let out = materialize_term(arena, TermGoal::Value(spec));
    expect_value(arena, out)
}

/// Materialize a computation spec into `arena`, iteratively.
pub fn materialize_computation(
    arena: &mut TermArena,
    spec: &ComputationSpec,
) -> ComputationId
{
    let out = materialize_term(arena, TermGoal::Comp(spec));
    expect_comp(arena, out)
}

/// The shared iterative engine for term materialization.
fn materialize_term<'spec>(
    arena: &mut TermArena,
    root: TermGoal<'spec>,
) -> TermOut
{
    let mut frames: Vec<TermFrame<'spec>> = Vec::new();
    let mut goal = root;
    'expand: loop {
        let mut produced: TermOut = match goal {
            | TermGoal::Value(spec) => match spec {
                | ValueSpec::Variable(index) => TermOut::Value(
                    arena.value_variable(gandr_kernel_core::DeBruijnIndex::from(*index)),
                ),
                | ValueSpec::Constant(index) => TermOut::Value(
                    arena.value_constant(gandr_kernel_core::ConstantIndex::from(*index)),
                ),
                | ValueSpec::Unit => TermOut::Value(arena.value_unit()),
                | ValueSpec::Literal(literal) => {
                    TermOut::Value(arena.value_literal(literal.clone()))
                },
                | ValueSpec::Pair(first, second) => {
                    frames.push(TermFrame::PairSecond(second));
                    goal = TermGoal::Value(first);
                    continue 'expand;
                },
                | ValueSpec::Injection(side, body) => {
                    frames.push(TermFrame::Injection(*side));
                    goal = TermGoal::Value(body);
                    continue 'expand;
                },
                | ValueSpec::Thunk(body) => {
                    frames.push(TermFrame::Thunk);
                    goal = TermGoal::Comp(body);
                    continue 'expand;
                },
                | ValueSpec::Lift(target, body) => {
                    frames.push(TermFrame::Lift(target.clone()));
                    goal = TermGoal::Value(body);
                    continue 'expand;
                },
            },
            | TermGoal::Comp(spec) => match spec {
                | ComputationSpec::Lambda(body) => {
                    frames.push(TermFrame::Lambda);
                    goal = TermGoal::Comp(body);
                    continue 'expand;
                },
                | ComputationSpec::Application(head, argument) => {
                    frames.push(TermFrame::ApplicationArgument(argument));
                    goal = TermGoal::Comp(head);
                    continue 'expand;
                },
                | ComputationSpec::Return(value) => {
                    frames.push(TermFrame::Return);
                    goal = TermGoal::Value(value);
                    continue 'expand;
                },
                | ComputationSpec::Bind(bound, body) => {
                    frames.push(TermFrame::BindBody(body));
                    goal = TermGoal::Comp(bound);
                    continue 'expand;
                },
                | ComputationSpec::Force(value) => {
                    frames.push(TermFrame::Force);
                    goal = TermGoal::Value(value);
                    continue 'expand;
                },
                | ComputationSpec::Case(scrutinee, on_left, on_right) => {
                    frames.push(TermFrame::CaseScrutineeThenLeft { on_left, on_right });
                    goal = TermGoal::Value(scrutinee);
                    continue 'expand;
                },
            },
        };
        loop {
            let Some(frame) = frames.pop()
            else {
                return produced;
            };
            match frame {
                | TermFrame::PairSecond(second) => {
                    let first = expect_value(arena, produced);
                    frames.push(TermFrame::PairBuild(first));
                    goal = TermGoal::Value(second);
                    continue 'expand;
                },
                | TermFrame::PairBuild(first) => {
                    let second = expect_value(arena, produced);
                    produced = TermOut::Value(arena.value_pair(first, second));
                },
                | TermFrame::Injection(side) => {
                    let body = expect_value(arena, produced);
                    produced = TermOut::Value(arena.value_injection(side, body));
                },
                | TermFrame::Thunk => {
                    let body = expect_comp(arena, produced);
                    produced = TermOut::Value(arena.value_thunk(body));
                },
                | TermFrame::Lift(target) => {
                    let body = expect_value(arena, produced);
                    produced = TermOut::Value(arena.value_lift(target, body));
                },
                | TermFrame::Lambda => {
                    let body = expect_comp(arena, produced);
                    produced = TermOut::Comp(arena.computation_lambda(body));
                },
                | TermFrame::ApplicationArgument(argument) => {
                    let head = expect_comp(arena, produced);
                    frames.push(TermFrame::ApplicationBuild(head));
                    goal = TermGoal::Value(argument);
                    continue 'expand;
                },
                | TermFrame::ApplicationBuild(head) => {
                    let argument = expect_value(arena, produced);
                    produced = TermOut::Comp(arena.computation_application(head, argument));
                },
                | TermFrame::Return => {
                    let value = expect_value(arena, produced);
                    produced = TermOut::Comp(arena.computation_return(value));
                },
                | TermFrame::Force => {
                    let value = expect_value(arena, produced);
                    produced = TermOut::Comp(arena.computation_force(value));
                },
                | TermFrame::BindBody(body) => {
                    let bound = expect_comp(arena, produced);
                    frames.push(TermFrame::BindBuild(bound));
                    goal = TermGoal::Comp(body);
                    continue 'expand;
                },
                | TermFrame::BindBuild(bound) => {
                    let body = expect_comp(arena, produced);
                    produced = TermOut::Comp(arena.computation_bind(bound, body));
                },
                | TermFrame::CaseScrutineeThenLeft { on_left, on_right } => {
                    let scrutinee = expect_value(arena, produced);
                    frames.push(TermFrame::CaseLeft {
                        scrutinee,
                        on_right,
                    });
                    goal = TermGoal::Comp(on_left);
                    continue 'expand;
                },
                | TermFrame::CaseLeft {
                    scrutinee,
                    on_right,
                } => {
                    let on_left = expect_comp(arena, produced);
                    frames.push(TermFrame::CaseRight { scrutinee, on_left });
                    goal = TermGoal::Comp(on_right);
                    continue 'expand;
                },
                | TermFrame::CaseRight { scrutinee, on_left } => {
                    let on_right = expect_comp(arena, produced);
                    produced = TermOut::Comp(arena.computation_case(scrutinee, on_left, on_right));
                },
            }
        }
    }
}

// ----- Declaration staging helpers -----

/// Stage a `Def` declaration, materializing its declared type and body into the
/// environment arena.
pub fn stage_def(
    environment: &mut Environment,
    levels: LevelSignature,
    declared: &ValueTypeSpec,
    body: &ValueSpec,
) -> Declaration
{
    let mut builder = environment.stage();
    let declared = materialize_value_type(builder.arena(), declared);
    let body = materialize_value(builder.arena(), body);
    builder.def(levels, declared, body)
}

/// Stage an `Axiom` declaration, materializing its declared type into the
/// arena.
pub fn stage_axiom(
    environment: &mut Environment,
    levels: LevelSignature,
    declared: &ValueTypeSpec,
) -> Declaration
{
    let mut builder = environment.stage();
    let declared = materialize_value_type(builder.arena(), declared);
    builder.axiom(levels, declared)
}

// ----- proptest spec strategies -----

use gandr_kernel_core::IntegerLiteral;
use gandr_kernel_core::Magnitude;
use gandr_kernel_core::NumericLiteral;
use gandr_kernel_core::Sign;
use gandr_kernel_core::StringLiteral;
use gandr_kernel_strata::LevelConstant;
use gandr_kernel_strata::LevelVar;
use gandr_kernel_strata::LevelVarIndex;
use proptest::prelude::*;

/// A small constant or single-variable level (`var + offset`, offset ≤ 3).
pub fn arb_level() -> impl Strategy<Value = Level>
{
    prop_oneof![
        (0_u64 .. 5).prop_map(|value| Level::constant(LevelConstant::from(value))),
        (0_u32 .. 3, 0_u64 .. 4).prop_map(|(index, offset)| {
            let mut level = Level::var(LevelVar::new(LevelVarIndex::from(index)));
            let mut remaining = offset;
            while remaining > 0 {
                level = level.succ().expect("small level offsets never overflow");
                remaining -= 1;
            }
            level
        }),
    ]
}

/// A base-type atom.
pub fn arb_base_type() -> impl Strategy<Value = BaseType>
{
    prop_oneof![
        Just(BaseType::Integer),
        Just(BaseType::String),
        Just(BaseType::Numeric),
    ]
}

/// A small literal (integer, text, or numeric).
pub fn arb_literal() -> impl Strategy<Value = Literal>
{
    let sign = prop_oneof![Just(Sign::NonNegative), Just(Sign::Negative)];
    prop_oneof![
        (sign.clone(), 0_u64 .. 1_000).prop_map(|(sign, magnitude)| {
            Literal::Integer(IntegerLiteral::new(
                sign,
                Magnitude::from_decimal_text(alloc_format(magnitude))
                    .expect("decimal text is a canonical magnitude"),
            ))
        }),
        "[a-z]{0,4}".prop_map(|content| Literal::Text(StringLiteral::new(content))),
        (sign, 0_u64 .. 1_000).prop_map(|(sign, magnitude)| {
            Literal::Numeric(NumericLiteral::new(
                sign,
                Magnitude::from_decimal_text(alloc_format(magnitude))
                    .expect("decimal text is a canonical magnitude"),
                gandr_kernel_core::FractionDigits::none(),
            ))
        }),
    ]
}

/// A `u64`'s decimal text (a tiny helper so strategies stay `no`-format-free).
fn alloc_format(value: u64) -> String
{
    let mut digits = String::new();
    let mut remaining = value;
    if remaining == 0 {
        digits.push('0');
        return digits;
    }
    let mut buffer = Vec::new();
    while remaining > 0 {
        let digit = u8::try_from(remaining % 10).unwrap_or(0);
        buffer.push(b'0' + digit);
        remaining /= 10;
    }
    buffer.reverse();
    for byte in buffer {
        digits.push(char::from(byte));
    }
    digits
}

/// A bounded value-type spec (computation types under thunks and arrows).
pub fn arb_value_type_spec() -> impl Strategy<Value = ValueTypeSpec>
{
    let leaf = prop_oneof![
        arb_base_type().prop_map(ValueTypeSpec::Base),
        Just(ValueTypeSpec::Unit),
        arb_level().prop_map(ValueTypeSpec::Universe),
    ];
    leaf.prop_recursive(3, 24, 3, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone())
                .prop_map(|(one, two)| ValueTypeSpec::Product(Box::new(one), Box::new(two))),
            (inner.clone(), inner.clone())
                .prop_map(|(one, two)| ValueTypeSpec::Sum(Box::new(one), Box::new(two))),
            (inner.clone(), arb_level())
                .prop_map(|(inner, target)| ValueTypeSpec::Lift(Box::new(inner), target)),
            inner.clone().prop_map(|result| {
                ValueTypeSpec::Thunk(Box::new(CompTypeSpec::Returner(Box::new(result))))
            }),
            (inner.clone(), inner).prop_map(|(domain, result)| ValueTypeSpec::Thunk(Box::new(
                CompTypeSpec::Arrow(
                    Box::new(domain),
                    Box::new(CompTypeSpec::Returner(Box::new(result))),
                )
            ))),
        ]
    })
}

/// A bounded computation-type spec.
pub fn arb_comp_type_spec() -> impl Strategy<Value = CompTypeSpec>
{
    prop_oneof![
        arb_value_type_spec().prop_map(|result| CompTypeSpec::Returner(Box::new(result))),
        (arb_value_type_spec(), arb_value_type_spec()).prop_map(|(domain, result)| {
            CompTypeSpec::Arrow(
                Box::new(domain),
                Box::new(CompTypeSpec::Returner(Box::new(result))),
            )
        }),
    ]
}

/// A bounded value spec, exercising every value former and, under thunks, every
/// computation former.
pub fn arb_value_spec() -> impl Strategy<Value = ValueSpec>
{
    let leaf = prop_oneof![
        (0_u32 .. 6).prop_map(ValueSpec::Variable),
        (0_usize .. 6).prop_map(ValueSpec::Constant),
        Just(ValueSpec::Unit),
        arb_literal().prop_map(ValueSpec::Literal),
    ];
    leaf.prop_recursive(3, 40, 3, |inner| {
        let side = prop_oneof![Just(Side::Left), Just(Side::Right)];
        prop_oneof![
            (inner.clone(), inner.clone())
                .prop_map(|(one, two)| ValueSpec::Pair(Box::new(one), Box::new(two))),
            (side, inner.clone())
                .prop_map(|(side, body)| ValueSpec::Injection(side, Box::new(body))),
            (arb_level(), inner.clone())
                .prop_map(|(target, body)| ValueSpec::Lift(target, Box::new(body))),
            inner
                .clone()
                .prop_map(
                    |value| ValueSpec::Thunk(Box::new(ComputationSpec::Return(Box::new(value))))
                ),
            inner
                .clone()
                .prop_map(
                    |value| ValueSpec::Thunk(Box::new(ComputationSpec::Lambda(Box::new(
                        ComputationSpec::Return(Box::new(value))
                    ))))
                ),
            (inner.clone(), inner.clone()).prop_map(|(head, argument)| {
                ValueSpec::Thunk(Box::new(ComputationSpec::Application(
                    Box::new(ComputationSpec::Force(Box::new(head))),
                    Box::new(argument),
                )))
            }),
            (inner.clone(), inner.clone()).prop_map(|(first, second)| {
                ValueSpec::Thunk(Box::new(ComputationSpec::Bind(
                    Box::new(ComputationSpec::Return(Box::new(first))),
                    Box::new(ComputationSpec::Return(Box::new(second))),
                )))
            }),
            (inner.clone(), inner.clone(), inner).prop_map(|(scrutinee, left, right)| {
                ValueSpec::Thunk(Box::new(ComputationSpec::Case(
                    Box::new(scrutinee),
                    Box::new(ComputationSpec::Return(Box::new(left))),
                    Box::new(ComputationSpec::Return(Box::new(right))),
                )))
            }),
        ]
    })
}

/// A bounded computation spec built from generated values.
pub fn arb_computation_spec() -> impl Strategy<Value = ComputationSpec>
{
    prop_oneof![
        arb_value_spec().prop_map(|body| ComputationSpec::Return(Box::new(body))),
        arb_value_spec().prop_map(|body| ComputationSpec::Force(Box::new(body))),
        arb_value_spec().prop_map(|body| ComputationSpec::Lambda(Box::new(
            ComputationSpec::Return(Box::new(body))
        ))),
        (arb_value_spec(), arb_value_spec()).prop_map(|(scrutinee, branch)| {
            ComputationSpec::Case(
                Box::new(scrutinee),
                Box::new(ComputationSpec::Return(Box::new(branch.clone()))),
                Box::new(ComputationSpec::Return(Box::new(branch))),
            )
        }),
    ]
}
