//! The `𝓕`-outcome canonicalization (the sequent-machines design's §9, the
//! **L1** gate).
//!
//! The differential's **external oracle** was a distinct,
//! independently-maintained implementation of the same operational semantics
//! (the ADR-71 adequacy discipline): agreement between it and the L machine was
//! evidence for the L machine's correctness, the two sharing no step code —
//! the oracle drove `Comp`, the L machine drives the focused command IL. The
//! **adequacy hypothesis**: for a checked core computation `t`, the two denote
//! the same observable outcome. The oracle has retired; this module's
//! [`canonical`] / [`agree`] now compare the L machine's outcome against the
//! frozen outcome snapshots (`tests/differential.rs`,
//! `tests/corpus_differential.rs`) that captured the final oracle-agreeing run
//! — the same canonicalization, its
//! reference now a fixture rather than a live second machine.
//!
//! # What is compared
//!
//! Comparison is on the [`Eval`] outcome — the outcome KIND (value / blame /
//! stuck), the stuck reason, and the returned value — through [`canonical`],
//! which maps both sides to a [`CanonOutcome`]:
//!
//! - a returned **first-order value** (unit / integer / string / numeric atom /
//!   hole / neutral variable, and pairs / injections / lists / records nested
//!   over them) is compared EXACTLY — the observable fragment the corpus
//!   `expect-last-value` directives check;
//! - a returned **thunk**, a bare **function** / **lazy pair** terminal, and a
//!   **partial native** are now compared **structurally** through the
//!   un-focusing readback `𝓕⁻¹` ([`crate::unfocus`]): both machines read the
//!   terminal back to an exact source term, which [`normalize_comp`] carries to
//!   its **commuting normal form** (re-focusing then un-focusing, which erases
//!   the annotations / effect-signature operation lists / motives / nominal ids
//!   `𝓕` drops and commutes the administrative redexes `𝓕` commutes) so the
//!   CEK's un-commuted source and the L machine's commuted un-focusing
//!   converge. A thunk's grade is compared exactly, as before, and now so is
//!   its body.
//! - a returned **reified stack** stays at KIND granularity
//!   ([`CanonValue::Stk`] opaque): a captured continuation crossing into value
//!   position has a runtime frame representation that diverges from the CEK's
//!   α-renamed side-table continuation, an un-reconcilable readback residual
//!   (§7a). The machines still agree that the outcome IS a reified stack.
//!
//! Both sides pass through the same [`canonical`], so the comparison is
//! symmetric and cannot hide a disagreement in the compared fragment.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_core_checker::grade::Grade;
use gandr_core_checker::outcome::Blame;
use gandr_core_checker::outcome::Eval;
use gandr_core_checker::outcome::StuckReason;
use gandr_core_checker::prim::NativePrim;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::HoleId;
use gandr_core_checker::syntax::NumLit;
use gandr_core_checker::syntax::Side;
use gandr_core_checker::syntax::Stack;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::DataId;

use crate::boundary::DifferentialAgreement;

/// A canonicalized evaluation outcome — the comparison target the L machine's
/// outcome maps onto (as the retired CEK oracle's did; see the module docs for
/// the granularity).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonOutcome
{
    /// A returned value `ret v`.
    Ret(CanonValue),
    /// A bare function terminal (`λx. t`) — compared **structurally** through
    /// the un-focusing readback `𝓕⁻¹`, its body normalized past the data `𝓕`
    /// erases (see [`normalize_comp`]).
    Function(Box<Comp>),
    /// A bare lazy-pair terminal (`⟨t, u⟩`) — compared structurally through
    /// `𝓕⁻¹` (see [`normalize_comp`]).
    LazyPair(Box<Comp>),
    /// A partial native (a curried builtin awaiting arguments) — compared by
    /// its primitive and its accumulated arguments (read back exactly through
    /// `𝓕⁻¹` and normalized).
    Native(NativePrim, Vec<Value>),
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
    /// A thunk — its **grade** and its **body** both compared exactly (the body
    /// un-focused through `𝓕⁻¹` and normalized past the data `𝓕` erases; see
    /// [`normalize_comp`]).
    Thunk(Grade, Box<Comp>),
    /// A reified stack — compared opaquely (the k-in-value residual: the
    /// runtime frame representation diverges from the CEK's α-renamed
    /// side-table continuation, so its content stays coarse).
    Stk,
    /// A future value former — compared opaquely (both machines map it here).
    Other,
}

/// Whether two evaluation outcomes agree (they canonicalize equally) — the L
/// machine's outcome against a recorded snapshot, or against the retired
/// oracle.
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
    }
}

/// Canonicalizes a terminal computation whnf. A codata / native terminal is now
/// compared structurally through the un-focusing readback `𝓕⁻¹` (both machines
/// read it back to an exact source term), normalized past the data `𝓕` erases.
fn canonical_terminal(comp: &Comp) -> CanonOutcome
{
    match *comp {
        | Comp::Ret(ref value) => CanonOutcome::Ret(canonical_value(value)),
        | Comp::Abs(..) => CanonOutcome::Function(Box::new(normalize_comp(comp))),
        | Comp::With(..) => CanonOutcome::LazyPair(Box::new(normalize_comp(comp))),
        | Comp::Native { prim, ref args } => {
            CanonOutcome::Native(prim, args.iter().map(|arg| normalize_value(arg)).collect())
        },
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
                | Value::Thunk(grade, ref body) => {
                    values.push(CanonValue::Thunk(grade, Box::new(normalize_comp(body))));
                },
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

/// Normalizes a source computation to the **commuting normal form** `𝓕⁻¹ ∘ 𝓕`,
/// so a CEK-side readback (the un-commuted source, retaining annotations /
/// effect-signature operation lists / motives / nominal ids) and an L-side
/// un-focusing readback (already commuted, with those erased) compare
/// structurally equal.
///
/// `𝓕` is the canonical form: it commutes administrative redexes — `App` of a
/// `Bind` / `Case` threads the eliminator through the binder, `t v` becomes an
/// `ap`-frame spine — and erases the four things `𝓕⁻¹` cannot recover (type
/// annotations, an effect signature's operation list, a `Walk` / `Split`
/// motive, a declared-data nominal id). Re-focusing then un-focusing therefore
/// lands both readbacks on the same term. It is idempotent on an
/// already-normalized computation, and falls back to the raw term only when the
/// readback declines (a reified stack in value position — the k-in-value
/// residual).
#[must_use]
fn normalize_comp(comp: &Comp) -> Comp
{
    crate::focus::focus_comp(comp)
        .ok()
        .and_then(|focused| crate::unfocus::unfocus_comp(&focused))
        .unwrap_or_else(|| comp.clone())
}

/// Normalizes a source value past the data `𝓕` erases (see [`normalize_comp`]).
///
/// # Termination
/// - reason: descends a finite source value, each node visited once.
/// - measure: the source value node the recursion descends into.
/// - boundedness: source values are finite `Rc` trees.
/// - input recursion: sub-values and thunk bodies flow into recursion; each
///   descends into a strictly smaller node.
#[cfg_attr(
    dylint_lib = "gandr_workflow_dylint",
    allow(
        unknown_lints,
        recursive_function_needs_termination,
        reason = "descends a finite source value; termination is proved above"
    )
)]
#[must_use]
fn normalize_value(value: &Value) -> Value
{
    match *value {
        | Value::Pair(ref fst, ref snd) => {
            Value::Pair(Rc::new(normalize_value(fst)), Rc::new(normalize_value(snd)))
        },
        | Value::Inj(side, ref payload) => Value::Inj(side, Rc::new(normalize_value(payload))),
        | Value::List(ref elements) => Value::List(
            elements
                .iter()
                .map(|element| Rc::new(normalize_value(element)))
                .collect(),
        ),
        | Value::Record(ref fields) => Value::Record(
            fields
                .iter()
                .map(|(label, field)| (label.clone(), Rc::new(normalize_value(field))))
                .collect(),
        ),
        | Value::Thunk(grade, ref body) => Value::Thunk(grade, Rc::new(normalize_comp(body))),
        // Strip a type annotation — `𝓕` focuses through it.
        | Value::Annot(ref inner, _) => normalize_value(inner),
        | Value::Here(ref witness) => Value::Here(Rc::new(normalize_value(witness))),
        | Value::Ctor {
            tag, ref payload, ..
        } => Value::Ctor {
            // The nominal id is render-only (`𝓕` erases it).
            id: canonical_data_id(),
            tag,
            payload: Rc::new(normalize_value(payload)),
        },
        // Drop a reified stack's content — the k-in-value residual stays coarse.
        | Value::Stk(_) => Value::stk(Stack::Empty),
        // Scalars, holes, variables, and any future value former pass through.
        | _ => value.clone(),
    }
}

/// A canonical render-only declared-data id (`𝓕` erases the nominal id;
/// ADR-80).
#[must_use]
fn canonical_data_id() -> DataId
{
    DataId::new(0_u64, "")
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::outcome::Eval;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::types::ValueType;

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

    /// Thunks compare **structurally** through the un-focusing readback: same
    /// grade + same body agree, same grade + DIFFERENT bodies now DISAGREE (the
    /// intentional-difference probe — the retired kind-granularity arm would
    /// have called these equal), a differing grade disagrees, and a thunk vs an
    /// integer disagrees. A type annotation on the body is erased by `𝓕`, so it
    /// does not perturb agreement.
    #[test]
    fn thunks_compare_structurally_through_readback()
    {
        let thunk_a = Eval::Value(Comp::ret(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(1)),
        )));
        let thunk_a_again = Eval::Value(Comp::ret(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(1)),
        )));
        assert!(
            bool::from(agree(&thunk_a, &thunk_a_again)),
            "same-grade, same-body thunks agree"
        );

        // The intentional-difference probe: two structurally different bodies
        // the old kind-granularity comparison called equal are now DISTINGUISHED.
        let thunk_b = Eval::Value(Comp::ret(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(2)),
        )));
        assert!(
            !bool::from(agree(&thunk_a, &thunk_b)),
            "same-grade thunks with different bodies are now distinguished"
        );

        // An annotation on the body is `𝓕`-erased, so it is invisible to the
        // comparison — the readback normalizes it away on both sides.
        let thunk_annotated = Eval::Value(Comp::ret(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::annot(Value::int(1), ValueType::integer())),
        )));
        assert!(
            bool::from(agree(&thunk_a, &thunk_annotated)),
            "a `𝓕`-erased annotation does not perturb thunk agreement"
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
