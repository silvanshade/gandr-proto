//! The semantic marking layer's agreement suite, driven by the shared free
//! generators in `gandr-core-checker-tools`.
//!
//! It lives in an integration target rather than beside `mark.rs` because the
//! generators sit one tier above this crate: an inline `cfg(test)` module is a
//! distinct crate instance from the library the generator crate links, so the
//! types would not unify. Every item it touches is public API.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "the recursive proptest generator helpers form a cycle with no linear call order"
    )
)]

use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;

use gandr_core_checker::discipline::boundary::GenerationDepth;
use gandr_core_checker::discipline::boundary::PathPrefixMut;
use gandr_core_checker::discipline::boundary::PathSetMut;
use gandr_core_checker::discipline::grade::Grade;
use gandr_core_checker::discipline::mark::Mark;
use gandr_core_checker::discipline::mark::MarkNodeId;
use gandr_core_checker::discipline::mark::Marking;
use gandr_core_checker::discipline::mark::mark_comp;
use gandr_core_checker::discipline::mark::mark_value;
use gandr_core_checker::effect::EffectOp;
use gandr_core_checker::effect::EffectRow;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::judgements::checker;
use gandr_core_checker::machine::control::Dir;
use gandr_core_checker::prim::NativePrim;
use gandr_core_checker::term::ctx::Ctx;
use gandr_core_checker::term::intern::TypeInterner;
use gandr_core_checker::term::intern::type_hash;
use gandr_core_checker::term::syntax::Comp;
use gandr_core_checker::term::syntax::OpClause;
use gandr_core_checker::term::syntax::Side;
use gandr_core_checker::term::syntax::Stack;
use gandr_core_checker::term::syntax::Value;
use gandr_core_checker::term::types::CompType;
use gandr_core_checker::term::types::Ty;
use gandr_core_checker::term::types::ValueType;
use gandr_core_checker_tools::strategies::any_grade;
use gandr_core_checker_tools::strategies::arb_comp_type;
use gandr_core_checker_tools::strategies::arb_value_type;
use gandr_core_checker_tools::strategies::binder_name;
use gandr_core_checker_tools::strategies::hole_id;
use gandr_core_checker_tools::strategies::int;
use gandr_core_checker_tools::strategies::record_label;
use gandr_core_checker_tools::strategies::txt;
use proptest::prelude::*;

/// The mark recovery pass types a native builtin (ADR-42) as the same
/// declared-type axiom as the checker / typing machine and emits NO error
/// mark — the one typing face the directed `mod native` checker ≡ machine
/// tests do not reach (the generators never construct a `Native`), so this
/// closes the lock-step coverage gap for `Comp::Native`.
#[test]
fn native_marks_clean_with_its_declared_type()
{
    let marking = mark_comp(Ctx::new(), Comp::native(NativePrim::Id), Dir::Infer);
    assert!(
        !bool::from(marking.has_errors()),
        "a well-typed native emits no recovery mark"
    );
    assert_eq!(
        *marking.root_type(),
        Ty::Comp(NativePrim::Id.declared_type()),
        "the mark pass infers I : Integer → F Integer"
    );
    // A source-facing higher-order combinator marks through the same axiom
    // path (its declared type, no child descended, no mark) — closing the
    // lock-step gap for the added prims.
    let each = mark_comp(Ctx::new(), Comp::native(NativePrim::Each), Dir::Infer);
    assert!(
        !bool::from(each.has_errors()),
        "a well-typed combinator emits no recovery mark"
    );
    assert_eq!(
        *each.root_type(),
        Ty::Comp(NativePrim::Each.declared_type())
    );
}

/// One handler operation clause. The body uses a leaf computation so the
/// generator graph stays acyclic; handler shape coverage is exercised by
/// [`arb_comp`] and curated tests.
fn arb_op_clause<D>(depth: D) -> BoxedStrategy<OpClause>
where
    D: Into<GenerationDepth>,
{
    let _depth = depth.into();
    (op_name(), binder_name(), binder_name(), leaf_comp())
        .prop_map(|(op, payload, resume, body)| OpClause::new(&op, &payload, &resume, body))
        .boxed()
}

/// Arbitrary computations up to a depth, built by proptest's explicit
/// recursive strategy combinator rather than Rust call recursion.
fn arb_comp<D>(depth: D) -> BoxedStrategy<Comp>
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    leaf_comp()
        .prop_recursive(depth, 64, 4, move |inner| {
            let value = arb_value(depth.saturating_sub(1));
            prop_oneof![
                (binder_name(), inner.clone()).prop_map(|(name, body)| Comp::lam(&name, body)),
                (binder_name(), arb_value_type(1), inner.clone())
                    .prop_map(|(name, ty, body)| Comp::lam_ann(&name, ty, body)),
                (inner.clone(), value.clone()).prop_map(|(head, arg)| Comp::app(head, arg)),
                (inner.clone(), binder_name(), inner.clone())
                    .prop_map(|(bound, name, cont)| Comp::bind(bound, &name, cont)),
                (
                    value.clone(),
                    binder_name(),
                    inner.clone(),
                    binder_name(),
                    inner.clone(),
                )
                    .prop_map(|(scrut, fst_name, fst_body, snd_name, snd_body)| {
                        Comp::case(scrut, &fst_name, fst_body, &snd_name, snd_body)
                    }),
                (value.clone(), binder_name(), binder_name(), inner.clone())
                    .prop_map(|(scrut, fst, snd, body)| Comp::split(scrut, &fst, &snd, body)),
                (inner.clone(), inner.clone()).prop_map(|(fst, snd)| Comp::with(fst, snd)),
                (side(), inner.clone()).prop_map(|(side, target)| match side {
                    | Side::Fst => Comp::prj1(target),
                    | Side::Snd => Comp::prj2(target),
                }),
                (value.clone(), record_label())
                    .prop_map(|(record, label)| Comp::record_proj(record, &label)),
                value.clone().prop_map(Comp::dup),
                value.clone().prop_map(Comp::drop),
                (op_name(), value.clone()).prop_map(|(op, arg)| Comp::perform(ask_sig(), &op, arg)),
                (
                    inner.clone(),
                    binder_name(),
                    inner.clone(),
                    prop::collection::vec(arb_op_clause(0), 0 ..= 2),
                )
                    .prop_map(|(scrut, ret_var, ret_body, ops)| {
                        Comp::handle(ask_sig(), scrut, &ret_var, ret_body, ops)
                    }),
                (value, inner.clone()).prop_map(|(stack, comp)| Comp::resume(stack, comp)),
                inner.clone().prop_map(Comp::reset),
                (binder_name(), inner).prop_map(|(k, body)| Comp::shift(&k, body)),
            ]
        })
        .boxed()
}

/// Leaf computations: returners, forces, and holes.
fn leaf_comp() -> impl Strategy<Value = Comp>
{
    prop_oneof![
        leaf_value().prop_map(Comp::ret),
        leaf_value().prop_map(Comp::force),
        hole_id().prop_map(Comp::hole),
    ]
}

/// Arbitrary values up to a depth, built by proptest's explicit recursive
/// strategy combinator rather than Rust call recursion. Thunk and stack
/// payloads use leaf subterms here; curated tests cover deep control/effect
/// payloads through those forms.
fn arb_value<D>(depth: D) -> BoxedStrategy<Value>
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    leaf_value()
        .prop_recursive(depth, 64, 4, |inner| {
            prop_oneof![
                (inner.clone(), inner.clone()).prop_map(|(fst, snd)| Value::pair(fst, snd)),
                (side(), inner.clone()).prop_map(|(side, payload)| match side {
                    | Side::Fst => Value::inj1(payload),
                    | Side::Snd => Value::inj2(payload),
                }),
                prop::collection::btree_map(record_label(), inner.clone(), 0 ..= 3)
                    .prop_map(Value::record),
                (any_grade(), leaf_comp()).prop_map(|(grade, body)| Value::thunk(grade, body)),
                (inner, arb_value_type(1)).prop_map(|(value, ty)| Value::annot(value, ty)),
                arb_stack(2).prop_map(Value::stk),
            ]
        })
        .boxed()
}

/// Leaf values: variables (some bound by [`base_ctx`], some free), unit,
/// literals, and holes.
fn leaf_value() -> impl Strategy<Value = Value>
{
    prop_oneof![
        binder_name().prop_map(|name| Value::var(&name)),
        Just(Value::Unit),
        any::<i64>().prop_map(Value::int),
        Just(Value::string("hello world")),
        any::<u32>().prop_map(Value::u32),
        any::<i64>().prop_map(Value::i64),
        any::<f64>().prop_map(Value::f64),
        hole_id().prop_map(Value::hole),
    ]
}

/// A side generator.
fn side() -> impl Strategy<Value = Side>
{
    prop_oneof![Just(Side::Fst), Just(Side::Snd)]
}

/// A small pool of operation names (some in `Ask`, one not) to exercise
/// both resolved and unresolved `perform` / handler clauses.
fn op_name() -> impl Strategy<Value = String>
{
    prop_oneof![Just("ask".to_owned()), Just("nope".to_owned()),]
}

/// Arbitrary reified stacks up to a depth, built by proptest's explicit
/// recursive strategy combinator rather than Rust call recursion.
fn arb_stack<D>(depth: D) -> BoxedStrategy<Stack>
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    Just(Stack::empty())
        .prop_recursive(depth, 32, 3, |inner| {
            prop_oneof![
                (leaf_value(), inner.clone()).prop_map(|(value, rest)| Stack::arg(value, rest)),
                (binder_name(), leaf_comp(), inner.clone())
                    .prop_map(|(name, cont, rest)| Stack::bind(&name, cont, rest)),
                (side(), inner).prop_map(|(side, rest)| match side {
                    | Side::Fst => Stack::prj1(rest),
                    | Side::Snd => Stack::prj2(rest),
                }),
            ]
        })
        .boxed()
}

/// A computation or value node whose descendants are addressable by
/// `origin::resolve`.
enum EnumTerm<'term>
{
    /// A value term.
    Value(&'term Value),
    /// A computation term.
    Comp(&'term Comp),
}

/// Pending node-path enumeration item.
struct EnumItem<'term>
{
    /// The term to enumerate.
    term: EnumTerm<'term>,
    /// Absolute path of that term from the oracle root.
    path: Vec<u32>,
}

/// Appends the node paths of a computation to `out` (mirroring
/// `origin::step_comp`).
fn enumerate_comp(
    comp: &Comp,
    prefix: PathPrefixMut<'_>,
    out: PathSetMut<'_>,
)
{
    enumerate_from(EnumTerm::Comp(comp), prefix, out);
}

/// Appends the `origin::resolve`-addressable node paths rooted at `term`.
fn enumerate_from(
    term: EnumTerm<'_>,
    mut prefix: PathPrefixMut<'_>,
    mut out: PathSetMut<'_>,
)
{
    let mut pending = alloc::vec![EnumItem {
        term,
        path: prefix.as_mut().clone()
    }];
    let out = out.as_mut();

    while let Some(item) = pending.pop() {
        out.push(item.path.clone());
        match item.term {
            | EnumTerm::Comp(comp) => match *comp {
                | Comp::Abs(_, _, ref body)
                | Comp::Prj(_, ref body)
                | Comp::Reset(ref body)
                | Comp::Shift(_, ref body) => {
                    let mut path = item.path;
                    path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(body),
                        path,
                    });
                },
                | Comp::App(ref head, ref arg) => {
                    let mut arg_path = item.path.clone();
                    arg_path.push(1);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(arg),
                        path: arg_path,
                    });
                    let mut head_path = item.path;
                    head_path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(head),
                        path: head_path,
                    });
                },
                | Comp::Ret(ref payload) | Comp::Force(ref payload) => {
                    let mut path = item.path;
                    path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(payload),
                        path,
                    });
                },
                // A record projection's record value is its single value
                // child `0` (ADR-45), matching the checker / machine /
                // origin order.
                | Comp::RecordProj { ref record, .. } => {
                    let mut path = item.path;
                    path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(record),
                        path,
                    });
                },
                | Comp::Bind(ref bound, _, ref cont) => {
                    let mut cont_path = item.path.clone();
                    cont_path.push(1);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(cont),
                        path: cont_path,
                    });
                    let mut bound_path = item.path;
                    bound_path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(bound),
                        path: bound_path,
                    });
                },
                | Comp::Case(ref scrut, (_, ref fst), (_, ref snd)) => {
                    let mut snd_path = item.path.clone();
                    snd_path.push(2);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(snd),
                        path: snd_path,
                    });
                    let mut fst_path = item.path.clone();
                    fst_path.push(1);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(fst),
                        path: fst_path,
                    });
                    let mut scrut_path = item.path;
                    scrut_path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(scrut),
                        path: scrut_path,
                    });
                },
                // A split's term children are the scrutinee (0) and the
                // body (1); the `p`/`q` binders and the motive (a
                // computation type, ADR-82) are attributes, not term
                // children.
                | Comp::Split {
                    ref scrut,
                    ref body,
                    ..
                }
                // An unpack's term children are the same pair: the
                // scrutinee (0) and the body (1). Its ascribed signature,
                // minted atoms and module binder are attributes, exactly as
                // a split's motive and binders are.
                | Comp::Unpack {
                    ref scrut,
                    ref body,
                    ..
                } => {
                    let mut body_path = item.path.clone();
                    body_path.push(1);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(body),
                        path: body_path,
                    });
                    let mut scrut_path = item.path;
                    scrut_path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(scrut),
                        path: scrut_path,
                    });
                },
                | Comp::With(ref fst, ref snd) => {
                    let mut snd_path = item.path.clone();
                    snd_path.push(1);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(snd),
                        path: snd_path,
                    });
                    let mut fst_path = item.path;
                    fst_path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(fst),
                        path: fst_path,
                    });
                },
                | Comp::Dup(ref value)
                | Comp::Drop(ref value)
                | Comp::Perform(_, _, ref value) => {
                    let mut path = item.path;
                    path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(value),
                        path,
                    });
                },
                | Comp::Handle {
                    ref scrutinee,
                    ref ret,
                    ref ops,
                    ..
                } => {
                    for (index, clause) in ops.iter().enumerate().rev() {
                        let mut path = item.path.clone();
                        path.push(u32::try_from(index).unwrap().saturating_add(2));
                        pending.push(EnumItem {
                            term: EnumTerm::Comp(&clause.body),
                            path,
                        });
                    }
                    let mut ret_path = item.path.clone();
                    ret_path.push(1);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(&ret.1),
                        path: ret_path,
                    });
                    let mut scrut_path = item.path;
                    scrut_path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(scrutinee),
                        path: scrut_path,
                    });
                },
                | Comp::Resume(ref stack, ref comp) => {
                    let mut comp_path = item.path.clone();
                    comp_path.push(1);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(comp),
                        path: comp_path,
                    });
                    let mut stack_path = item.path;
                    stack_path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(stack),
                        path: stack_path,
                    });
                },
                // A list-case's children: scrutinee (0), `nil` body (1),
                // `cons` body (2); the `head`/`tail` binders are
                // attributes (ADR-40), exactly as `origin::step_comp`.
                | Comp::ListCase {
                    ref scrut,
                    ref nil,
                    ref cons,
                    ..
                } => {
                    let mut cons_path = item.path.clone();
                    cons_path.push(2);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(cons),
                        path: cons_path,
                    });
                    let mut nil_path = item.path.clone();
                    nil_path.push(1);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(nil),
                        path: nil_path,
                    });
                    let mut scrut_path = item.path;
                    scrut_path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(scrut),
                        path: scrut_path,
                    });
                },
                // `Hole` is a leaf.
                | _ => {},
            },
            | EnumTerm::Value(value) => match *value {
                | Value::Pair(ref fst, ref snd) => {
                    let mut snd_path = item.path.clone();
                    snd_path.push(1);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(snd),
                        path: snd_path,
                    });
                    let mut fst_path = item.path;
                    fst_path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(fst),
                        path: fst_path,
                    });
                },
                // A pack joins these: its term child is its payload (0),
                // its witness types being attributes as an ascription's
                // type is.
                | Value::Inj(_, ref payload)
                | Value::Annot(ref payload, _)
                | Value::Pack { ref payload, .. } => {
                    let mut path = item.path;
                    path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Value(payload),
                        path,
                    });
                },
                | Value::Thunk(_, ref body) => {
                    let mut path = item.path;
                    path.push(0);
                    pending.push(EnumItem {
                        term: EnumTerm::Comp(body),
                        path,
                    });
                },
                // A list literal's elements are its value children
                // `0, 1, …` (ADR-40), exactly as `origin::step_value`.
                | Value::List(ref elements) => {
                    for (index, element) in elements.iter().enumerate().rev() {
                        let mut path = item.path.clone();
                        path.push(u32::try_from(index).unwrap_or(u32::MAX));
                        pending.push(EnumItem {
                            term: EnumTerm::Value(element),
                            path,
                        });
                    }
                },
                // A record literal's field values are its value children
                // `0, 1, …` in canonical (sorted-label) order (ADR-45),
                // matching the checker / machine / origin field order.
                | Value::Record(ref fields) => {
                    for (index, field) in fields.values().enumerate().rev() {
                        let mut path = item.path.clone();
                        path.push(u32::try_from(index).unwrap_or(u32::MAX));
                        pending.push(EnumItem {
                            term: EnumTerm::Value(field),
                            path,
                        });
                    }
                },
                // `Var`/`Unit`/`Int`/`Str`/`Num`/`Hole`/`Stk`/`Here`/`Ctor`
                // are leaves under `origin::resolve` (the stack interior
                // is not addressable).
                | _ => {},
            },
        }
    }
}

/// The error marks of a marking, for assertion messages.
fn error_marks(marking: &Marking) -> Vec<Mark>
{
    marking
        .marks()
        .map(|(_, mark)| mark.clone())
        .filter(|mark| bool::from(mark.is_error()))
        .collect()
}

proptest! {
    /// The value oracle over arbitrary terms and directions.
    #[test]
    fn value_marking_agrees_with_checker(
        value in arb_value(3),
        check in prop::option::of(arb_value_type(2)),
    ) {
        let dir = check.map_or(Dir::Infer, Dir::Check);
        oracle_value(&base_ctx(), &value, &dir);
    }
    /// The computation oracle over arbitrary terms and directions.
    #[test]
    fn comp_marking_agrees_with_checker(
        comp in arb_comp(3),
        check in prop::option::of(arb_comp_type(2)),
    ) {
        let dir = check.map_or(Dir::Infer, Dir::Check);
        oracle_comp(&base_ctx(), &comp, &dir);
    }
}

/// Curated well-typed computations, one per common shape, asserting the
/// checker accepts and the marker agrees (no error marks, equal root type).
#[test]
fn curated_well_typed_terms_agree()
{
    let f_unit = CompType::returner(ValueType::Unit);
    let f_int = CompType::returner(ValueType::integer());

    // ret unit ⇓ F Unit
    oracle_comp(
        &Ctx::new(),
        &Comp::ret(Value::Unit),
        &Dir::Check(f_unit.clone()),
    );
    // λx:Int. ret x ⇑
    oracle_comp(
        &Ctx::new(),
        &Comp::lam_ann("x", ValueType::integer(), Comp::ret(Value::var("x"))),
        &Dir::Infer,
    );
    // thunk_ω (ret unit) ⇑
    oracle_value(
        &Ctx::new(),
        &Value::thunk(Grade::OMEGA, Comp::ret(Value::Unit)),
        &Dir::Infer,
    );
    // force (thunk_ω (ret unit)) ⇑
    oracle_comp(
        &Ctx::new(),
        &Comp::force(Value::thunk(Grade::OMEGA, Comp::ret(Value::Unit))),
        &Dir::Infer,
    );
    // split (unit, 0) as (a, b) in ret a ⇑
    oracle_comp(
        &Ctx::new(),
        &Comp::split(
            Value::pair(Value::Unit, Value::int(0)),
            "a",
            "b",
            Comp::ret(Value::var("a")),
        ),
        &Dir::Infer,
    );
    // case (inj1 unit : Unit + Unit) of { inj1 a => ret unit | inj2 b => ret unit }
    // ⇓ F Unit
    oracle_comp(
        &Ctx::new(),
        &Comp::case(
            Value::annot(
                Value::inj1(Value::Unit),
                ValueType::sum(ValueType::Unit, ValueType::Unit),
            ),
            "a",
            Comp::ret(Value::Unit),
            "b",
            Comp::ret(Value::Unit),
        ),
        &Dir::Check(f_unit.clone()),
    );
    // handle (perform Ask.ask unit) { ret x => ret x | ask p k => resume k (ret 5)
    // } ⇓ F Int
    let handler = Comp::handle(
        ask_sig(),
        Comp::perform(ask_sig(), "ask", Value::Unit),
        "x",
        Comp::ret(Value::var("x")),
        vec![OpClause::new(
            "ask",
            "p",
            "k",
            Comp::resume(Value::var("k"), Comp::ret(Value::int(5))),
        )],
    );
    oracle_comp(&Ctx::new(), &handler, &Dir::Check(f_int));
    // reset (shift k. ret unit) ⇓ F Unit
    oracle_comp(
        &Ctx::new(),
        &Comp::reset(Comp::shift("k", Comp::ret(Value::Unit))),
        &Dir::Check(f_unit),
    );
}

/// **The package rung against the oracle.** A well-typed pack and a
/// well-typed consumer carry no error mark and synthesize the checker's own
/// type; an abstraction leak and a grade-zero opening force one.
///
/// Without this the oracle would hold for packages only vacuously: the free
/// generators produce no package term, so the property tests above quantify
/// over a set the new formers are not in.
#[test]
fn package_terms_agree_with_the_checker()
{
    let component = ValueType::atom("t");
    let signature = ValueType::package(
        Grade::OMEGA,
        ["t"],
        ValueType::thunk(
            Grade::OMEGA,
            CompType::F(
                Rc::new(ValueType::record([("seed".to_owned(), component)])),
                EffectRow::EMPTY,
            ),
        ),
    );
    let implementation = Value::pack(
        [ValueType::integer()],
        Value::thunk(
            Grade::OMEGA,
            Comp::ret(Value::record([("seed".to_owned(), Value::int(7_i64))])),
        ),
    );
    oracle_value(&Ctx::new(), &implementation, &Dir::Check(signature.clone()));
    // A pack in inference position: the checker is stuck, so the marker
    // must carry an error mark.
    oracle_value(&Ctx::new(), &implementation, &Dir::Infer);

    let mut ctx = Ctx::new();
    ctx.bind("p".to_owned(), signature.clone());
    let atom = gandr_core_checker::term::types::SealId::new(0_u64, "counter", "t");
    // The consumer keeps the seed abstract: it binds it and returns unit,
    // which needs nothing of its type.
    let opaque_use = Comp::bind(
        Comp::force(Value::var("m")),
        "r",
        Comp::bind(
            Comp::record_proj(Value::var("r"), "seed"),
            "s",
            Comp::ret(Value::Unit),
        ),
    );
    let well_typed = Comp::unpack(
        Value::var("p"),
        signature.clone(),
        [atom.clone()],
        "m",
        opaque_use,
    );
    oracle_comp(
        &ctx,
        &well_typed,
        &Dir::Check(CompType::returner(ValueType::Unit)),
    );

    // The leak: the seed used as an `Integer`. The checker rejects, so the
    // marker must record at least one error mark.
    let leak = Comp::bind(
        Comp::force(Value::var("m")),
        "r",
        Comp::bind(
            Comp::record_proj(Value::var("r"), "seed"),
            "s",
            Comp::ret(Value::var("s")),
        ),
    );
    oracle_comp(
        &ctx,
        &Comp::unpack(Value::var("p"), signature, [atom.clone()], "m", leak),
        &Dir::Check(CompType::returner(ValueType::integer())),
    );

    // The grade-zero opening: refused for its grade, and marked for it.
    let closed = ValueType::package(
        Grade::ZERO,
        ["t"],
        ValueType::thunk(
            Grade::ZERO,
            CompType::F(Rc::new(ValueType::Unit), EffectRow::EMPTY),
        ),
    );
    let mut closed_ctx = Ctx::new();
    closed_ctx.bind("q".to_owned(), closed.clone());
    oracle_comp(
        &closed_ctx,
        &Comp::unpack(Value::var("q"), closed, [atom], "m", Comp::ret(Value::Unit)),
        &Dir::Check(CompType::returner(ValueType::Unit)),
    );
}

/// The shared oracle for a computation (as [`oracle_value`]).
fn oracle_comp(
    ctx: &Ctx,
    comp: &Comp,
    dir: &Dir<CompType>,
)
{
    let (checker_result, _) = checker::run_comp(ctx.clone(), comp.clone(), dir.clone());
    let marking = mark_comp(ctx.clone(), comp.clone(), dir.clone());
    let mut paths = Vec::new();
    enumerate_comp(
        comp,
        PathPrefixMut::from(&mut Vec::new()),
        PathSetMut::from(&mut paths),
    );
    for path in &paths {
        assert!(
            marking.get_compat_path(path).is_some(),
            "node {path:?} undecorated in computation {comp:?}"
        );
    }
    match checker_result {
        | Ok(ty) => {
            assert!(
                !bool::from(marking.has_errors()),
                "checker accepted computation {comp:?} but marker produced error marks \
                 {marks:?}",
                marks = error_marks(&marking)
            );
            assert_eq!(
                marking.root_type(),
                &ty,
                "root type disagreement for accepted computation {comp:?}"
            );
        },
        | Err(_) => assert!(
            bool::from(marking.has_errors()),
            "checker rejected computation {comp:?} but marker produced no error mark"
        ),
    }
}

/// Asserts the checker genuinely ACCEPTS `(comp, dir)` — so the accept side
/// of the oracle is actually exercised, not silently skipped on a
/// mis-constructed term — then runs the full oracle.
fn accept_comp(
    ctx: &Ctx,
    comp: &Comp,
    dir: &Dir<CompType>,
)
{
    let (result, _) = checker::run_comp(ctx.clone(), comp.clone(), dir.clone());
    assert!(
        result.is_ok(),
        "expected the checker to accept {comp:?}, got {result:?}"
    );
    oracle_comp(ctx, comp, dir);
}

/// The base typing context the oracle runs under: the strategy pool's
/// base atoms `i : Int`, `s : Str`, plus a with-typed and arrow-typed
/// thunk so the inference-only `prj`/`force`/`app` forms can sometimes
/// type.
fn base_ctx() -> Ctx
{
    Ctx::new()
        .with("i", int())
        .with("s", txt())
        .with(
            "w",
            ValueType::thunk(
                Grade::OMEGA,
                CompType::with(CompType::returner(int()), CompType::returner(txt())),
            ),
        )
        .with(
            "f",
            ValueType::thunk(
                Grade::OMEGA,
                CompType::arrow(int(), CompType::returner(int())),
            ),
        )
}

/// Directed well-typed ACCEPT cases for the novel effect / control / grade
/// / stack forms the free generators almost never produce well-typed.
/// This is the accept side of the oracle — the direction that guards
/// against false-positive marks and root-type disagreement — pinned on
/// exactly the forms (dup, reify, prj, with, effectful check targets,
/// non-empty residual rows) whose accept paths the random generators do
/// not reach.
#[test]
fn curated_well_typed_novel_forms_agree()
{
    let f_int = CompType::returner(ValueType::integer());
    let f_unit = CompType::returner(ValueType::Unit);
    let f_str = CompType::returner(txt());

    // dup (thunk_ω (ret unit)) ⇓ F (U_1 (F Unit) × U_1 (F Unit))
    let dup_target = CompType::returner(ValueType::prod(
        ValueType::thunk(Grade::ONE, f_unit.clone()),
        ValueType::thunk(Grade::ONE, f_unit.clone()),
    ));
    accept_comp(
        &Ctx::new(),
        &Comp::dup(Value::thunk(Grade::OMEGA, Comp::ret(Value::Unit))),
        &Dir::Check(dup_target),
    );

    // stk (unit :: ε) ⇓ Stk(Unit → F Unit, F Unit)   (argument frame)
    accept_value(
        &Ctx::new(),
        &Value::stk(Stack::arg(Value::Unit, Stack::empty())),
        &Dir::Check(ValueType::stk(
            CompType::arrow(ValueType::Unit, f_unit.clone()),
            f_unit.clone(),
        )),
    );

    // stk ((x. ret x) :: ε) ⇓ Stk(F Int, F Int)   (bind frame)
    accept_value(
        &Ctx::new(),
        &Value::stk(Stack::bind("x", Comp::ret(Value::var("x")), Stack::empty())),
        &Dir::Check(ValueType::stk(f_int.clone(), f_int.clone())),
    );

    // stk (prj1 :: ε) ⇓ Stk(F Int & F Str, F Int)   (projection frame)
    accept_value(
        &Ctx::new(),
        &Value::stk(Stack::prj1(Stack::empty())),
        &Dir::Check(ValueType::stk(
            CompType::with(f_int.clone(), f_str),
            f_int.clone(),
        )),
    );

    // prj1 (force w) ⇑ F Int   (w : U_ω (F Int & F Str) in base_ctx — the
    // only way to *infer* a with-typed projection target)
    accept_comp(
        &base_ctx(),
        &Comp::prj1(Comp::force(Value::var("w"))),
        &Dir::Infer,
    );

    // ⟨ret 0, ret unit⟩ ⇓ F Int & F Unit   (with-introduction)
    accept_comp(
        &Ctx::new(),
        &Comp::with(Comp::ret(Value::int(0)), Comp::ret(Value::Unit)),
        &Dir::Check(CompType::with(f_int, f_unit)),
    );

    // [0, 1, 2] ⇓ List Integer   (list literal, the check-only intro; ADR-40)
    accept_value(
        &Ctx::new(),
        &Value::list(vec![Value::int(0), Value::int(1), Value::int(2)]),
        &Dir::Check(ValueType::list(ValueType::integer())),
    );

    // [] ⇓ List Unit   (the empty list — the inhabitant that *cannot* infer
    // its element type, so the whole former is check-only)
    accept_value(
        &Ctx::new(),
        &Value::list(vec![]),
        &Dir::Check(ValueType::list(ValueType::Unit)),
    );

    // case ([unit] : List Unit) { Nil ⇒ ret unit | Cons(h, t) ⇒ ret h } ⇓ F
    // Unit   (the list eliminator, binding head : Unit and tail : List Unit;
    // the scrutinee is annotated so it infers `List Unit`)
    accept_comp(
        &Ctx::new(),
        &Comp::list_case(
            Value::annot(
                Value::list(vec![Value::Unit]),
                ValueType::list(ValueType::Unit),
            ),
            Comp::ret(Value::Unit),
            "h",
            "t",
            Comp::ret(Value::var("h")),
        ),
        &Dir::Check(CompType::returner(ValueType::Unit)),
    );

    // perform Ask.ask unit ⇓ F^⟨Ask⟩ Int   (an effectful CHECK target — the
    // row-subset subsumption leg the free generators never reach on accept)
    accept_comp(
        &Ctx::new(),
        &Comp::perform(ask_sig(), "ask", Value::Unit),
        &Dir::Check(CompType::returner_eff(
            ValueType::integer(),
            EffectRow::singleton(ask_sig()),
        )),
    );

    // A handler leaving a NON-EMPTY residual row: handle Ask in a scrutinee
    // that performs both Other and Ask, against the answer F^⟨Other⟩ Int —
    // exercises rule_handle's residual-row finish (ε_t ∖ E ⊆ ε, ε = ⟨Other⟩).
    let other = EffectSig::new(
        gandr_core_checker::discipline::boundary::EffectSignatureName::from("Other"),
        vec![EffectOp::new(
            gandr_core_checker::discipline::boundary::OperationName::from("op"),
            ValueType::Unit,
            ValueType::Unit,
        )],
    );
    let answer = CompType::returner_eff(ValueType::integer(), EffectRow::singleton(other.clone()));
    let scrutinee = Comp::bind(
        Comp::perform(other, "op", Value::Unit),
        "_dropped",
        Comp::perform(ask_sig(), "ask", Value::Unit),
    );
    let handler = Comp::handle(ask_sig(), scrutinee, "x", Comp::ret(Value::var("x")), vec![
        OpClause::new(
            "ask",
            "p",
            "k",
            Comp::resume(Value::var("k"), Comp::ret(Value::int(5))),
        ),
    ]);
    accept_comp(&Ctx::new(), &handler, &Dir::Check(answer));
}

/// The reified-stack interior (the bonus decoration) is decorated
/// under the stack node's path, typed with full `Γ; answer` fidelity.
#[test]
fn reified_stack_interior_is_decorated()
{
    // stk ((x. ret x) :: ε) ⇓ Stk(F Int, F Int): the bind continuation is a
    // bonus interior node at the stack node's path, frame index `[0]`.
    let stk_ty = ValueType::stk(
        CompType::returner(ValueType::integer()),
        CompType::returner(ValueType::integer()),
    );
    let value = Value::stk(Stack::bind("x", Comp::ret(Value::var("x")), Stack::empty()));
    accept_value(&Ctx::new(), &value, &Dir::Check(stk_ty.clone()));
    let marking = mark_value(Ctx::new(), value, Dir::Check(stk_ty));
    assert!(
        marking.get_compat_path([].as_slice()).is_some(),
        "the stk node is decorated"
    );
    let cont = marking
        .get_compat_path([0].as_slice())
        .expect("the bind continuation is a bonus interior node under the stack path");
    assert!(
        !bool::from(cont.has_error()),
        "the well-typed bind continuation carries no error mark"
    );
}

/// Asserts the checker genuinely ACCEPTS `(value, dir)`, then runs the
/// oracle (as [`accept_comp`]).
fn accept_value(
    ctx: &Ctx,
    value: &Value,
    dir: &Dir<ValueType>,
)
{
    let (result, _) = checker::run_value(ctx.clone(), value.clone(), dir.clone());
    assert!(
        result.is_ok(),
        "expected the checker to accept {value:?}, got {result:?}"
    );
    oracle_value(ctx, value, dir);
}

/// The shared oracle for a value: totality (every addressable node
/// decorated) plus checker agreement (accept ⟺ no error mark ∧ root type
/// equal; reject ⟹ some error mark).
fn oracle_value(
    ctx: &Ctx,
    value: &Value,
    dir: &Dir<ValueType>,
)
{
    let (checker_result, _) = checker::run_value(ctx.clone(), value.clone(), dir.clone());
    let marking = mark_value(ctx.clone(), value.clone(), dir.clone());
    let mut paths = Vec::new();
    enumerate_value(
        value,
        PathPrefixMut::from(&mut Vec::new()),
        PathSetMut::from(&mut paths),
    );
    for path in &paths {
        assert!(
            marking.get_compat_path(path).is_some(),
            "node {path:?} undecorated in value {value:?}"
        );
    }
    match checker_result {
        | Ok(ty) => {
            assert!(
                !bool::from(marking.has_errors()),
                "checker accepted value {value:?} but marker produced error marks \
                 {marks:?}",
                marks = error_marks(&marking)
            );
            assert_eq!(
                marking.root_type(),
                &ty,
                "root type disagreement for accepted value {value:?}"
            );
        },
        | Err(_) => assert!(
            bool::from(marking.has_errors()),
            "checker rejected value {value:?} but marker produced no error mark"
        ),
    }
}

/// Appends the `origin::resolve`-addressable node paths of a value to
/// `out`, rooted at `prefix` (mirroring `origin::step_value`).
fn enumerate_value(
    value: &Value,
    prefix: PathPrefixMut<'_>,
    out: PathSetMut<'_>,
)
{
    enumerate_from(EnumTerm::Value(value), prefix, out);
}

/// A type-mismatch boundary: `unit` checked against `Int` marks the unit
/// node (path `[0]`), not the enclosing `ret`.
#[test]
fn type_mismatch_marks_the_inconsistent_node()
{
    let marking = mark_comp(
        Ctx::new(),
        Comp::ret(Value::Unit),
        Dir::Check(CompType::returner(ValueType::integer())),
    );
    let facts = marking
        .get_compat_path([0].as_slice())
        .expect("the unit node is decorated");
    assert!(
        facts
            .marks
            .iter()
            .any(|mark| matches!(mark, Mark::TypeMismatch(_))),
        "expected a TypeMismatch on the unit node, got {marks:?}",
        marks = facts.marks
    );
    assert!(bool::from(marking.has_errors()));
}

/// A free variable marks the variable node.
#[test]
fn free_variable_marks_the_variable()
{
    let marking = mark_value(Ctx::new(), Value::var("nope"), Dir::Infer);
    let facts = marking
        .get_compat_path([].as_slice())
        .expect("the root is decorated");
    assert!(
        facts
            .marks
            .iter()
            .any(|mark| matches!(mark, Mark::FreeVariable { .. })),
        "expected a FreeVariable mark, got {marks:?}",
        marks = facts.marks
    );
}

/// Applying a non-arrow marks a shape mismatch on the application node.
#[test]
fn shape_mismatch_on_non_arrow_application()
{
    let marking = mark_comp(
        Ctx::new(),
        Comp::app(Comp::ret(Value::Unit), Value::Unit),
        Dir::Infer,
    );
    let facts = marking
        .get_compat_path([].as_slice())
        .expect("the root is decorated");
    assert!(
        facts
            .marks
            .iter()
            .any(|mark| matches!(mark, Mark::ShapeMismatch { .. })),
        "expected a ShapeMismatch on the application, got {marks:?}",
        marks = facts.marks
    );
}

/// A thunk graded `0` checked against `U_1` exceeds its grade budget.
#[test]
fn grade_budget_mark_on_undersized_thunk()
{
    let marking = mark_value(
        Ctx::new(),
        Value::thunk(Grade::ZERO, Comp::ret(Value::Unit)),
        Dir::Check(ValueType::thunk(
            Grade::ONE,
            CompType::returner(ValueType::Unit),
        )),
    );
    let facts = marking
        .get_compat_path([].as_slice())
        .expect("the root is decorated");
    assert!(
        facts
            .marks
            .iter()
            .any(|mark| matches!(mark, Mark::GradeBudget { .. })),
        "expected a GradeBudget mark, got {marks:?}",
        marks = facts.marks
    );
}

/// Forcing a thunk graded `0` is a thunkability failure (`1 ⊑ 0` fails).
#[test]
fn thunkability_mark_on_unforceable_thunk()
{
    let marking = mark_comp(
        Ctx::new(),
        Comp::force(Value::thunk(Grade::ZERO, Comp::ret(Value::Unit))),
        Dir::Infer,
    );
    let facts = marking
        .get_compat_path([].as_slice())
        .expect("the root is decorated");
    assert!(
        facts
            .marks
            .iter()
            .any(|mark| matches!(mark, Mark::Thunkability { .. })),
        "expected a Thunkability mark, got {marks:?}",
        marks = facts.marks
    );
}

/// An effectful returner checked against a pure one is an effect-row
/// mismatch (the row leg fails while the payload agrees).
#[test]
fn effect_row_mismatch_on_unhandled_effect()
{
    let marking = mark_comp(
        Ctx::new(),
        Comp::perform(ask_sig(), "ask", Value::Unit),
        Dir::Check(CompType::returner(ValueType::integer())),
    );
    let facts = marking
        .get_compat_path([].as_slice())
        .expect("the root is decorated");
    assert!(
        facts
            .marks
            .iter()
            .any(|mark| matches!(mark, Mark::EffectRowMismatch(_))),
        "expected an EffectRowMismatch mark, got {marks:?}",
        marks = facts.marks
    );
}

/// A bare injection in inference mode is stuck (no rule applies); the empty
/// hole that recovers it is not an error mark.
#[test]
fn stuck_injection_and_holes_are_distinguished()
{
    let stuck = mark_value(Ctx::new(), Value::inj1(Value::Unit), Dir::Infer);
    assert!(
        bool::from(stuck.has_errors()),
        "a bare inj in inference mode is stuck"
    );
    let hole = mark_value(Ctx::new(), Value::hole(0), Dir::Infer);
    assert!(
        !bool::from(hole.has_errors()),
        "an empty hole is complete-but-incomplete, not an error"
    );
    let facts = hole
        .get_compat_path([].as_slice())
        .expect("the hole is decorated");
    assert!(
        facts
            .marks
            .iter()
            .any(|mark| matches!(mark, Mark::EmptyHole(_))),
        "the hole carries an EmptyHole mark"
    );
}

/// A pattern-position hole is complete-but-incomplete exactly as an
/// expression-position one is, and the two remain distinguishable — the
/// property a live match analysis rests on, since an unfinished *test*
/// admits a reading an unfinished *value* does not.
#[test]
fn pattern_holes_are_incomplete_not_erroneous()
{
    let pattern = Mark::PatternHole(7);
    assert!(
        !bool::from(pattern.is_error()),
        "a pattern hole leaves the program well-typed and merely unfinished"
    );
    assert_ne!(
        pattern,
        Mark::EmptyHole(7),
        "the pattern position is part of the mark's identity, not a detail"
    );
    assert!(
        !bool::from(Mark::EmptyHole(7).is_error()),
        "the expression hole's classification is unchanged"
    );
    assert!(
        bool::from(Mark::Stuck { hint: "" }.is_error()),
        "every other kind stays an error mark"
    );
}

/// Compatibility paths are only a snapshot: two paths that point at the
/// same shared node resolve to the same stable node identity.
#[test]
fn shared_node_paths_resolve_to_one_stable_id()
{
    let shared = Rc::new(Value::Unit);
    let value = Value::Pair(Rc::clone(&shared), Rc::clone(&shared));
    let marking = mark_value(Ctx::new(), value, Dir::Infer);

    let mut child_ids = marking
        .compatibility_paths()
        .filter_map(|(path, &id)| match *path {
            | [0 | 1] => Some(id),
            | _ => None,
        });
    let first = child_ids.next();
    let second = child_ids.next();

    assert!(
        first.is_some() && first == second && child_ids.next().is_none(),
        "shared child paths should resolve to exactly one stable id"
    );
}

/// Stable-id APIs expose stable identities; path access is explicitly named
/// as compatibility lookup.
#[test]
fn iter_and_errors_expose_stable_ids_with_explicit_path_compatibility()
{
    let marking = mark_value(Ctx::new(), Value::var("missing"), Dir::Infer);

    assert!(
        marking
            .iter()
            .all(|(id, _)| matches!(id, MarkNodeId::Value(_))),
        "Marking::iter should expose stable node ids"
    );
    assert!(
        marking
            .errors()
            .all(|(id, _)| matches!(id, MarkNodeId::Value(_))),
        "Marking::errors should expose stable node ids"
    );
    assert!(
        marking
            .compatibility_paths()
            .any(|(path, &id)| path.is_empty() && marking.get(id).is_some()),
        "legacy path access should be an explicit compatibility boundary"
    );
}

/// The interner gives O(1) canonical equality: equal types (including rows
/// built in different orders) intern to the same id; distinct types differ.
#[test]
fn interner_is_canonical_over_rows_and_grades()
{
    let mut interner = TypeInterner::new();
    let int_ty = Ty::Value(ValueType::integer());
    let first = interner.intern(&int_ty);
    let again = interner.intern(&ValueType::integer().into_ty());
    assert_eq!(first, again, "equal types intern to the same id");
    assert_eq!(interner.resolve(first), Some(&int_ty));

    // Two rows built by different union orders are canonical-equal, so the
    // returners over them intern identically.
    let ask = EffectRow::singleton(ask_sig());
    let put = EffectRow::singleton(EffectSig::new(
        gandr_core_checker::discipline::boundary::EffectSignatureName::from("Put"),
        vec![EffectOp::new(
            gandr_core_checker::discipline::boundary::OperationName::from("put"),
            ValueType::Unit,
            ValueType::Unit,
        )],
    ));
    let forward = ask.union(&put);
    let backward = put.union(&ask);
    let lhs = Ty::Comp(CompType::returner_eff(ValueType::Unit, forward));
    let rhs = Ty::Comp(CompType::returner_eff(ValueType::Unit, backward));
    assert_eq!(type_hash(&lhs), type_hash(&rhs), "row order is canonical");
    assert_eq!(
        interner.intern(&lhs),
        interner.intern(&rhs),
        "row-equivalent returners intern to the same id"
    );

    let unit_ty = Ty::Value(ValueType::Unit);
    assert_ne!(
        interner.intern(&int_ty),
        interner.intern(&unit_ty),
        "distinct types get distinct ids"
    );
}

/// The fixed single-operation effect signature `Ask = { ask : Unit ↠ Int }`
/// the effect generators and curated handler use.
fn ask_sig() -> EffectSig
{
    EffectSig::new(
        gandr_core_checker::discipline::boundary::EffectSignatureName::from("Ask"),
        vec![EffectOp::new(
            gandr_core_checker::discipline::boundary::OperationName::from("ask"),
            ValueType::Unit,
            ValueType::integer(),
        )],
    )
}

/// The content hash is deterministic and consistent with equality.
#[test]
fn type_hash_is_deterministic_and_eq_consistent()
{
    let ty = Ty::Comp(CompType::arrow(
        ValueType::integer(),
        CompType::returner(ValueType::Unit),
    ));
    assert_eq!(type_hash(&ty), type_hash(&ty.clone()));
}

/// A small helper to lift a value type into a [`Ty`] for the interner test.
trait IntoTy
{
    /// Wraps `self` as a [`Ty`].
    fn into_ty(self) -> Ty;
}

impl IntoTy for ValueType
{
    fn into_ty(self) -> Ty
    {
        Ty::Value(self)
    }
}
