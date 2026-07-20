//! The `L-run ∘ 𝓕 ≡ run` differential comparison (`proposal-sequent-kernel.md`
//! §9, phase **L1** gate).
//!
//! The CEK machine (`gandr_core_checker::eval::run` and friends) is the
//! **external oracle** (ADR-71 adequacy discipline): a distinct,
//! independently-maintained implementation of the same operational semantics,
//! so agreement between it and the L machine is evidence for the L machine's
//! correctness (the two share no step code — the CEK drives `Comp`, the L
//! machine drives the focused command IL). The **adequacy hypothesis** the
//! differential rests on: for a checked core computation `t`, `run(t)` and
//! `L-run(𝓕(t))` denote the same observable outcome; a disagreement is a
//! genuine defect in one machine (a finding for the orchestrator), never a
//! tolerated divergence.
//!
//! # What is compared, and the one principled granularity choice
//!
//! Comparison is on the [`Eval`] outcome — the outcome KIND (value / blame /
//! stuck), the stuck reason, and the returned value — through [`canonical`],
//! which maps both sides to a [`CanonOutcome`]:
//!
//! - a returned **first-order value** (unit / integer / string / numeric atom /
//!   hole / neutral variable, and pairs / injections / lists / records nested
//!   over them) is compared EXACTLY — this is the observable fragment the
//!   corpus `expect-last-value` directives check;
//! - a returned **thunk** or **reified stack**, a bare **function** / **lazy
//!   pair** terminal, and a **partial native** are compared at KIND granularity
//!   (a thunk vs a thunk, a function vs a function, …). Their exact structural
//!   readback needs *un-focusing* the IL back to source syntax (the inverse of
//!   `𝓕`), which is a listed residual seam, not a semantic divergence: the
//!   machines still agree that the outcome IS a thunk / function / etc. A
//!   thunk's **grade** is compared exactly (carried end-to-end); only its body
//!   stays opaque pending the un-focusing readback.
//!
//! Both sides pass through the same [`canonical`], so the comparison is
//! symmetric and cannot hide a disagreement in the compared (first-order)
//! fragment.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::eval::Blame;
use gandr_core_checker::eval::Eval;
use gandr_core_checker::eval::StuckReason;
use gandr_core_checker::grade::Grade;
use gandr_core_checker::prim::NativePrim;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::HoleId;
use gandr_core_checker::syntax::NumLit;
use gandr_core_checker::syntax::Side;
use gandr_core_checker::syntax::Value;

use crate::boundary::DifferentialAgreement;

/// A canonicalized evaluation outcome — the comparison target both the CEK
/// oracle and the L machine map onto (see the module docs for the granularity).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CanonOutcome
{
    /// A returned value `ret v`.
    Ret(CanonValue),
    /// A bare function terminal (`λx. t`) — compared opaquely (un-focusing
    /// residual).
    Function,
    /// A bare lazy-pair terminal (`⟨t, u⟩`) — compared opaquely.
    LazyPair,
    /// A partial native (a curried builtin awaiting arguments) — compared by
    /// its primitive and accumulated-argument count.
    Native(NativePrim, usize),
    /// Any other computation terminal — compared opaquely.
    OtherComp,
    /// A defined runtime halt.
    Blame(Blame),
    /// An undefined stuck configuration.
    Stuck(StuckReason),
}

/// A canonicalized value — first-order structure exact, higher-order parts
/// opaque (see the module docs).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CanonValue
{
    /// The unit value.
    Unit,
    /// An integer literal.
    Int(i64),
    /// A string literal.
    Str(String),
    /// A typed numeric literal.
    Num(NumLit),
    /// A typed hole.
    Hole(HoleId),
    /// A free / neutral variable.
    Var(String),
    /// An eager pair.
    Pair(Box<Self>, Box<Self>),
    /// A sum injection.
    Inj(Side, Box<Self>),
    /// A list.
    List(Vec<Self>),
    /// A record.
    Record(Vec<(String, Self)>),
    /// A thunk — its **grade** compared exactly (carried end-to-end), its
    /// body opaque (the un-focusing readback residual).
    Thunk(Grade),
    /// A reified stack — compared opaquely.
    Stk,
    /// A future value former — compared opaquely (both machines map it here).
    Other,
}

/// Whether the CEK oracle and the L machine agree on an outcome (their
/// canonicalization are equal).
///
/// # Contract
/// - ensures: `true` iff `canonical(oracle) == canonical(machine)`.
/// - panics: none.
#[inline]
#[must_use]
pub fn agree(
    oracle: &Eval,
    machine: &Eval,
) -> DifferentialAgreement
{
    (canonical(oracle) == canonical(machine)).into()
}

/// Canonicalizes an [`Eval`] outcome for the differential (see the module
/// docs).
///
/// # Contract
/// - ensures: total; a value terminal maps by its computation shape, a first-
///   order returned value maps exactly, higher-order parts map opaquely.
/// - panics: none.
#[inline]
#[must_use]
pub fn canonical(eval: &Eval) -> CanonOutcome
{
    match *eval {
        | Eval::Blame(blame) => CanonOutcome::Blame(blame),
        | Eval::Stuck(ref reason) => CanonOutcome::Stuck(reason.clone()),
        | Eval::Value(ref comp) => canonical_terminal(comp),
        // A future `Eval` variant maps opaquely (both machines map it here).
        | _ => CanonOutcome::OtherComp,
    }
}

/// Canonicalizes a terminal computation whnf.
fn canonical_terminal(comp: &Comp) -> CanonOutcome
{
    match *comp {
        | Comp::Ret(ref value) => CanonOutcome::Ret(canonical_value(value)),
        | Comp::Abs(..) => CanonOutcome::Function,
        | Comp::With(..) => CanonOutcome::LazyPair,
        | Comp::Native { prim, ref args } => CanonOutcome::Native(prim, args.len()),
        | _ => CanonOutcome::OtherComp,
    }
}

/// Canonicalizes a value, stripping annotations and mapping higher-order parts
/// opaquely.
///
/// # Termination
/// - reason: an explicit frame stack descends through finite source-value
///   children until scalar or opaque higher-order leaves.
/// - measure: the frame stack shrinks or receives strict child values after
///   their parent frame.
/// - boundedness: lowered core values are finite `Rc` trees in the
///   generated/corpus inputs.
/// - input recursion: none; caller-supplied values are canonicalized by the
///   explicit frame stack.
fn canonical_value(value: &Value) -> CanonValue
{
    enum Frame<'value>
    {
        Enter(&'value Value),
        Pair,
        Inj(Side),
        List(usize),
        Record(Vec<String>),
    }

    let mut frames = vec![Frame::Enter(value)];
    let mut values = Vec::new();
    while let Some(frame) = frames.pop() {
        match frame {
            | Frame::Enter(node) => match *node {
                | Value::Unit => values.push(CanonValue::Unit),
                | Value::Int(literal) => values.push(CanonValue::Int(literal)),
                | Value::Str(ref literal) => values.push(CanonValue::Str(literal.clone())),
                | Value::Num(literal) => values.push(CanonValue::Num(literal)),
                | Value::Hole(hole) => values.push(CanonValue::Hole(hole)),
                | Value::Var(ref name) => values.push(CanonValue::Var(name.clone())),
                | Value::Pair(ref fst, ref snd) => {
                    frames.push(Frame::Pair);
                    frames.push(Frame::Enter(snd.as_ref()));
                    frames.push(Frame::Enter(fst.as_ref()));
                },
                | Value::Inj(side, ref payload) => {
                    frames.push(Frame::Inj(side));
                    frames.push(Frame::Enter(payload.as_ref()));
                },
                | Value::List(ref elements) => {
                    frames.push(Frame::List(elements.len()));
                    for element in elements.iter().rev() {
                        frames.push(Frame::Enter(element.as_ref()));
                    }
                },
                | Value::Record(ref fields) => {
                    let labels = fields.keys().cloned().collect::<Vec<_>>();
                    frames.push(Frame::Record(labels));
                    for (_, field) in fields.iter().rev() {
                        frames.push(Frame::Enter(field.as_ref()));
                    }
                },
                | Value::Annot(ref inner, _) => frames.push(Frame::Enter(inner.as_ref())),
                | Value::Thunk(grade, _) => values.push(CanonValue::Thunk(grade)),
                | Value::Stk(_) => values.push(CanonValue::Stk),
                | _ => values.push(CanonValue::Other),
            },
            | Frame::Pair => {
                let snd = pop_canon(&mut values);
                let fst = pop_canon(&mut values);
                values.push(CanonValue::Pair(Box::new(fst), Box::new(snd)));
            },
            | Frame::Inj(side) => {
                let payload = pop_canon(&mut values);
                values.push(CanonValue::Inj(side, Box::new(payload)));
            },
            | Frame::List(len) => {
                debug_assert!(
                    len <= values.len(),
                    "list frame length exceeds the canonicalized value stack"
                );
                let start = values.len().saturating_sub(len);
                let elements = values.split_off(start);
                values.push(CanonValue::List(elements));
            },
            | Frame::Record(labels) => {
                debug_assert!(
                    labels.len() <= values.len(),
                    "record frame field count exceeds the canonicalized value stack"
                );
                let start = values.len().saturating_sub(labels.len());
                let fields = labels.into_iter().zip(values.split_off(start)).collect();
                values.push(CanonValue::Record(fields));
            },
        }
    }
    pop_canon(&mut values)
}

/// Pops a canonicalized value off the result stack.
///
/// # Contract
/// - requires: the worklist discipline pushed a value for the current build
///   frame.
/// - ensures: returns that value.
/// - panics: none in release builds (a `debug_assert!` guards the stack
///   discipline in test / debug builds; a desync falls back to the absorbing
///   [`CanonValue::Other`] — never reached, the differential oracle would fail
///   first).
fn pop_canon(values: &mut Vec<CanonValue>) -> CanonValue
{
    let value = values.pop();
    debug_assert!(value.is_some(), "canonicalization result stack underflowed");
    value.unwrap_or(CanonValue::Other)
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::eval::Eval;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Value;

    use super::*;

    /// Equal first-order returns canonicalize equal.
    #[test]
    fn first_order_returns_compare_exactly()
    {
        let left = Eval::Value(Comp::ret(Value::pair(Value::int(1), Value::int(2))));
        let right = Eval::Value(Comp::ret(Value::pair(Value::int(1), Value::int(2))));
        assert!(
            bool::from(agree(&left, &right)),
            "identical first-order returns agree"
        );

        let different = Eval::Value(Comp::ret(Value::pair(Value::int(1), Value::int(3))));
        assert!(
            !bool::from(agree(&left, &different)),
            "a differing first-order value is caught exactly"
        );
    }

    /// Thunks compare by grade with an opaque body (the un-focusing residual):
    /// same grade + different bodies agree, a differing grade does not, and a
    /// thunk vs an integer does not.
    #[test]
    fn thunks_compare_by_grade_with_opaque_body()
    {
        let thunk_a = Eval::Value(Comp::ret(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(1)),
        )));
        let thunk_b = Eval::Value(Comp::ret(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(2)),
        )));
        assert!(
            bool::from(agree(&thunk_a, &thunk_b)),
            "same-grade thunks agree though their bodies differ"
        );

        let thunk_omega = Eval::Value(Comp::ret(Value::thunk(
            Grade::OMEGA,
            Comp::ret(Value::int(1)),
        )));
        assert!(
            !bool::from(agree(&thunk_a, &thunk_omega)),
            "a differing grade is caught"
        );

        let integer = Eval::Value(Comp::ret(Value::int(1)));
        assert!(
            !bool::from(agree(&thunk_a, &integer)),
            "a thunk and an integer do not agree"
        );
    }
}
