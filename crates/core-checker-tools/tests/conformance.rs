#![allow(
    unknown_lints,
    non_topologically_sorted_functions,
    reason = "the type-directed generator roots are mutually recursive through value and computation strategies"
)]

//! Conformance suite: the recursive checker and the typing machine must agree
//! *step for step* (ADR-9; the roadmap's Stage 1).
//!
//! Two kinds of evidence:
//!
//! - example-based tests over the core-CBPV worked-example subset, including
//!   the literal machine trace of Example 1;
//! - property tests over generated terms — type-directed generators for
//!   well-typed terms (agreement *and* success) and free generators for
//!   arbitrary, mostly ill-typed terms (agreement on the error and on the trace
//!   prefix).
//!
//! # Example ledger
//!
//! Which worked examples the suite pins, and at what stage:
//!
//! | Example                                  | Stage           | Tested?                                                            |
//! | ---------------------------------------- | --------------- | ----------------------------------------------------------------- |
//! | 1 — identity (`λx:A. ret x`)             | 1               | yes — [`examples::example_1_identity`] (+ literal trace), and per-step depth [`examples::example_1_depth_per_step`] |
//! | 2 — composition                          | 1               | yes — [`examples::example_2_composition`]                         |
//! | 3a — case on an annotated injection      | 1               | yes — [`examples::example_3a_case_on_annotated_injection`]        |
//! | 4 — checked thunk of unannotated identity | 1 (core subset) | yes (core) — [`examples::example_4_checked_thunk_of_unannotated_identity`]; the full intersection/overload version is Stage 3 (not yet in scope) |
//!
//! Beyond the worked examples, rule coverage lives in [`positive`] (one per
//! introduction/elimination rule), failure modes in [`negative`] (one per
//! [`gandr_core_checker::error::TypeError`] constructor reachable in core
//! CBPV), the `type-system.md` §"Subtyping decomposition" rows in
//! [`subtype_rows`] (one positive and one negative per row), and the A2.2 hole
//! rules in [`holes`] — the hole axioms, every matched-type elimination/checked
//! introduction, the consistency rows of `Unknown`, and the recorded
//! non-transitivity witness. The generated-term properties follow at the end
//! of the module.
//!
//! **Holes have no worked example in `examples.md`** (the document predates
//! A2.2); their evidence is the [`holes`] module plus the generators, which
//! gain `Value::Hole`/`Comp::Hole` (free and type-directed: a hole is a
//! universal check-mode inhabitant) and `Unknown` (in the type pool) in
//! ADR-9 lockstep with the checker and machine extensions.

use alloc::rc::Rc;

use gandr_core_checker::boundary::CoherenceDecision;
use gandr_core_checker::boundary::GenerationDepth;
use gandr_core_checker::boundary::I64Slice;
use gandr_core_checker::boundary::IntegerLiteral;
use gandr_core_checker::boundary::NumericLiteralName;
use gandr_core_checker::boundary::Staticness;
use gandr_core_checker::checker;
use gandr_core_checker::control::Dir;
use gandr_core_checker::control::Trace;
use gandr_core_checker::ctx::Ctx;
use gandr_core_checker::effect::EffectOp;
use gandr_core_checker::effect::EffectRow;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::error::TypeError;
use gandr_core_checker::grade::Grade;
use gandr_core_checker::machine;
use gandr_core_checker::subtype::comp_subtype;
use gandr_core_checker::subtype::value_subtype;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::OpClause;
use gandr_core_checker::syntax::SplitMotive;
use gandr_core_checker::syntax::Stack;
use gandr_core_checker::syntax::Value;
use gandr_core_checker::types::CompType;
use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;
use gandr_core_checker_tools::strategies::any_grade;
use gandr_core_checker_tools::strategies::arb_comp_type;
use gandr_core_checker_tools::strategies::arb_value_type;
use gandr_core_checker_tools::strategies::binder_name;
use gandr_core_checker_tools::strategies::hole_id;
use gandr_core_checker_tools::strategies::int;
use gandr_core_checker_tools::strategies::integer;
use gandr_core_checker_tools::strategies::leaf_value_type;
use gandr_core_checker_tools::strategies::numeric_atom;
use gandr_core_checker_tools::strategies::record_label;
use gandr_core_checker_tools::strategies::string;
use gandr_core_checker_tools::strategies::txt;
use proptest::prelude::*;
use proptest::strategy::NewTree;
use proptest::strategy::Union;
use proptest::strategy::ValueTree;

/// A generator scope: the names and types the well-typed generators may use.
type Scope = Vec<(String, ValueType)>;

/// One implementation's run: the result paired with its trace.
type Run = (Result<Ty, TypeError>, Trace);

/// Paired runs of the recursive checker and the typing machine.
type PairedRuns = (Run, Run);

/// The scope corresponding to [`base_ctx`].
///
/// `i`/`s` are the *shadowable* realizers: [`binder_name`] can draw their names
/// and rebind them, exercising base-scope shadowing. `int_base`/`str_base` are
/// *reserve* realizers under names [`binder_name`] never draws, so that even
/// when `i`/`s` are shadowed away from their atoms, every atom stays realizable
/// (the type-directed generators rely on this to avoid stranding an atom leaf).
fn base_scope() -> Scope
{
    vec![
        ("i".to_owned(), int()),
        ("s".to_owned(), txt()),
        ("int_base".to_owned(), int()),
        ("str_base".to_owned(), txt()),
    ]
}

/// The `IO` effect signature `{ print : Integer ↠ 1 }`: a one-operation
/// signature, for exercising row union (`⟨State⟩ ∪ ⟨IO⟩`) and effect leaks.
fn io_sig() -> EffectSig
{
    EffectSig::new(
        gandr_core_checker::boundary::EffectSignatureName::from("IO"),
        vec![EffectOp::new(
            gandr_core_checker::boundary::OperationName::from("print"),
            integer(),
            ValueType::Unit,
        )],
    )
}

/// Asserts step-for-step agreement on a computation and returns the shared
/// result.
fn agree_comp(
    ctx: &Ctx,
    comp: &Comp,
    dir: &Dir<CompType>,
) -> Result<Ty, TypeError>
{
    let ((rec_result, rec_trace), (mach_result, mach_trace)) = both_comp(ctx, comp, dir);
    assert_eq!(
        rec_trace, mach_trace,
        "checker and machine traces must agree step for step"
    );
    assert_eq!(
        rec_result, mach_result,
        "checker and machine results must agree"
    );
    mach_result
}

/// Runs both implementations on a computation, returning
/// `((recursive result, recursive trace), (machine result, machine trace))`.
fn both_comp(
    ctx: &Ctx,
    comp: &Comp,
    dir: &Dir<CompType>,
) -> PairedRuns
{
    let (rec_result, rec_trace) = checker::run_comp(ctx.clone(), comp.clone(), dir.clone());
    let (mach_result, mach_trace) = machine::run_comp(ctx.clone(), comp.clone(), dir.clone());
    ((rec_result, rec_trace), (mach_result, mach_trace))
}

/// Asserts step-for-step agreement on a value and returns the shared result.
fn agree_value(
    ctx: &Ctx,
    value: &Value,
    dir: &Dir<ValueType>,
) -> Result<Ty, TypeError>
{
    let ((rec_result, rec_trace), (mach_result, mach_trace)) = both_value(ctx, value, dir);
    assert_eq!(
        rec_trace, mach_trace,
        "checker and machine traces must agree step for step"
    );
    assert_eq!(
        rec_result, mach_result,
        "checker and machine results must agree"
    );
    mach_result
}

/// Runs both implementations on a value, as [`both_comp`].
fn both_value(
    ctx: &Ctx,
    value: &Value,
    dir: &Dir<ValueType>,
) -> PairedRuns
{
    let (rec_result, rec_trace) = checker::run_value(ctx.clone(), value.clone(), dir.clone());
    let (mach_result, mach_trace) = machine::run_value(ctx.clone(), value.clone(), dir.clone());
    ((rec_result, rec_trace), (mach_result, mach_trace))
}

/// Whether a value type is *static* (`Unknown`-free). On static types
/// consistent subtyping coincides with the old structural subtyping and is
/// transitive; with `Unknown` it deliberately is not (see
/// [`gandr_core_checker::subtype`]
/// and [`holes::consistency_is_not_transitive`]).
/// # Termination
/// - reason: the helper drains an explicit finite type worklist.
/// - measure: pending type nodes in the worklist.
/// - boundedness: type inputs are finite Rust values.
/// - input recursion: none.
fn value_type_is_static(ty: &ValueType) -> Staticness
{
    type_is_static_from(StaticGoal::Value(ty))
}

/// Whether a computation type is *static* (`Unknown`-free); see
/// [`value_type_is_static`].
/// # Termination
/// - reason: the helper drains an explicit finite type worklist.
/// - measure: pending type nodes in the worklist.
/// - boundedness: type inputs are finite Rust values.
/// - input recursion: none.
fn comp_type_is_static(ty: &CompType) -> Staticness
{
    type_is_static_from(StaticGoal::Comp(ty))
}

#[derive(Clone, Copy)]
enum StaticGoal<'ty>
{
    Value(&'ty ValueType),
    Comp(&'ty CompType),
}

fn type_is_static_from(root: StaticGoal<'_>) -> Staticness
{
    let mut pending = vec![root];
    while let Some(goal) = pending.pop() {
        match goal {
            | StaticGoal::Value(ty) => match *ty {
                // The code universe carries no children, so it is trivially
                // static (ADR-81), joining the leaf atoms and unit.
                // A sealed atom joins them for the same reason and one more:
                // it has no children at all, so there is nowhere for an
                // `Unknown` to hide inside one.
                | ValueType::Atom(_)
                | ValueType::Unit
                | ValueType::Universe
                | ValueType::Sealed(_) => {},
                // Products, sums, and dependent pairs decompose the same way:
                // static iff both components are. The dependent pair's binder
                // is a name, carrying no type-level `Unknown` (ADR-81).
                | ValueType::Prod(ref fst, ref snd)
                | ValueType::Sum(ref fst, ref snd)
                | ValueType::Sigma {
                    ref fst, ref snd, ..
                } => {
                    pending.push(StaticGoal::Value(fst));
                    pending.push(StaticGoal::Value(snd));
                },
                | ValueType::List(ref elem) => pending.push(StaticGoal::Value(elem)),
                | ValueType::Record(ref fields) => {
                    pending.extend(fields.values().map(|field| StaticGoal::Value(field)));
                },
                | ValueType::Thunk(_, ref body) => pending.push(StaticGoal::Comp(body)),
                | ValueType::Stk(ref consumes, ref delivers) => {
                    pending.push(StaticGoal::Comp(consumes));
                    pending.push(StaticGoal::Comp(delivers));
                },
                // The identity type is `Unknown`-free iff its carrier is
                // (ADR-76); the endpoints are values, which carry no type-level
                // `Unknown`.
                | ValueType::Path {
                    ty: ref carrier, ..
                } => pending.push(StaticGoal::Value(carrier)),
                // A declared-data handle is `Unknown`-free iff every type
                // argument is (ADR-80); the nominal `id` carries no type-level
                // `Unknown`.
                | ValueType::Data { ref args, .. } => {
                    pending.extend(args.iter().map(|arg| StaticGoal::Value(arg)));
                },
                // A package is `Unknown`-free iff its payload is; the binder
                // labels are names, carrying no type-level `Unknown`, exactly
                // as a dependent pair's binder does not.
                | ValueType::Package { ref payload, .. } => {
                    pending.push(StaticGoal::Value(payload));
                },
                | ValueType::Unknown => return false.into(),
            },
            | StaticGoal::Comp(ty) => match *ty {
                | CompType::F(ref of, _) => pending.push(StaticGoal::Value(of)),
                | CompType::Arrow(ref arg, ref res) => {
                    pending.push(StaticGoal::Value(arg));
                    pending.push(StaticGoal::Comp(res));
                },
                | CompType::With(ref fst, ref snd) => {
                    pending.push(StaticGoal::Comp(fst));
                    pending.push(StaticGoal::Comp(snd));
                },
                | CompType::Unknown => return false.into(),
            },
        }
    }
    true.into()
}

/// Supertypes of `ty` under [`value_subtype`], for exercising *nontrivial*
/// subsumption: the type itself, grade-widened thunks (`U_r B <: U_s B` when
/// `s ⊑ r`), and `Unknown` (consistent with everything — A2.2).
fn value_supertype(ty: &ValueType) -> BoxedStrategy<ValueType>
{
    let mut options: Vec<BoxedStrategy<ValueType>> =
        vec![Just(ty.clone()).boxed(), Just(ValueType::Unknown).boxed()];
    if let ValueType::Thunk(grade, ref body) = *ty {
        for widened in [
            Grade::ZERO,
            Grade::ONE,
            Grade::fin(gandr_core_checker::boundary::GradeBound::from(2_u64)),
            Grade::fin(gandr_core_checker::boundary::GradeBound::from(3_u64)),
            Grade::OMEGA,
        ] {
            if bool::from(widened.leq(grade)) && widened != grade {
                options.push(Just(ValueType::Thunk(widened, Rc::clone(body))).boxed());
            }
        }
    }
    Union::new(options).boxed()
}

/// Declared-data conformance (ADR-80): the frozen-core `Ctor` / `DataCase`
/// forms are exercised by the checker ≡ typing machine differential (via
/// [`agree_value`] / [`agree_comp`]), so the new forms stay lock-step exactly
/// as the sum family they sit beside. Their β-reduction outcomes are pinned on
/// the L machine in `gandr_core_sequent::conformance_soundness` (the retired
/// reference evaluator's frozen outcome snapshots).
mod declared_data
{
    use gandr_core_checker::types::DataId;

    use super::*;

    /// A nullary declared-data constructor checks against its data type, and
    /// the recursive checker and the typing machine agree step for step
    /// (rule Ctor⇓).
    #[test]
    fn ctor_checks_against_its_data_type()
    {
        let color = DataId::new(0, "Color");
        let red = Value::ctor(color.clone(), 0, Value::Unit);
        let data_ty = ValueType::data(color, Vec::new());
        let result = agree_value(&Ctx::new(), &red, &Dir::Check(data_ty.clone()));
        assert_eq!(result, Ok(Ty::Value(data_ty)));
    }

    /// A one-field constructor's payload checks against its data type with
    /// empty (value-erased) arguments fitting an ascribed instantiation
    /// `Maybe(a)`.
    #[test]
    fn ctor_fits_ascribed_instantiation()
    {
        let maybe = DataId::new(0, "Maybe");
        let some = Value::ctor(maybe.clone(), 1, Value::int(3));
        let ascribed = ValueType::data(maybe, vec![ValueType::atom("a")]);
        let result = agree_value(&Ctx::new(), &some, &Dir::Check(ascribed.clone()));
        assert_eq!(result, Ok(Ty::Value(ascribed)));
    }

    /// Generativity: a `Fahrenheit` value does not check against the `Celsius`
    /// data type — the nominal ids differ — and both passes reject identically.
    #[test]
    fn ctor_nominal_distinctness_is_rejected()
    {
        const WRONG_TEMPERATURE_READING: i64 = 20;
        let celsius = DataId::new(0, "Celsius");
        let fahrenheit = DataId::new(1, "Fahrenheit");
        let wrong = Value::ctor(fahrenheit, 0, Value::int(WRONG_TEMPERATURE_READING));
        let expected_ty = ValueType::data(celsius, Vec::new());
        let result = agree_value(&Ctx::new(), &wrong, &Dir::Check(expected_ty));
        assert!(
            matches!(result, Err(TypeError::TypeMismatch { .. })),
            "a Fahrenheit is not a Celsius (ADR-80 generativity)"
        );
    }

    /// A declared-data `case` over a data-typed scrutinee checks against the
    /// expected answer, checker ≡ machine, each arm's payload binder at
    /// `Unknown` (rule `DataCase`⇓).
    #[test]
    fn data_case_checks_and_agrees()
    {
        let maybe = DataId::new(0, "Maybe");
        let mut ctx = Ctx::new();
        ctx.bind("s".to_owned(), ValueType::data(maybe, Vec::new()));
        let data_case = Comp::data_case(Value::var("s"), vec![
            ("_none".to_owned(), Comp::ret(Value::int(0))),
            ("x".to_owned(), Comp::ret(Value::int(1))),
        ]);
        let expected = CompType::returner(ValueType::integer());
        let result = agree_comp(&ctx, &data_case, &Dir::Check(expected.clone()));
        assert_eq!(result, Ok(Ty::Comp(expected)));
    }

    /// The absurd match `case v {}` over an uninhabited datatype checks against
    /// any answer vacuously, checker ≡ machine.
    #[test]
    fn data_case_empty_absurd_agrees()
    {
        let void = DataId::new(0, "Void");
        let mut ctx = Ctx::new();
        ctx.bind("v".to_owned(), ValueType::data(void, Vec::new()));
        let data_case = Comp::data_case(Value::var("v"), Vec::new());
        let expected = CompType::returner(ValueType::Unit);
        let result = agree_comp(&ctx, &data_case, &Dir::Check(expected.clone()));
        assert_eq!(result, Ok(Ty::Comp(expected)));
    }

    // The declared-data β-rule and its non-`Ctor` stuck are pinned on the L
    // machine in `gandr_core_sequent::conformance_soundness` (the retired
    // reference evaluator's frozen outcome snapshots).
}

/// Levitation stage-1 conformance (ADR-81; the levitation design's §6): the
/// frozen-core dependent pair `Σ` (feature 2) and the code universe `Universe`
/// (feature 1). The `Σ` intro (a pair checked against a `Σ`) and elimination (a
/// `split` over a `Σ`-typed scrutinee) forms are exercised by the checker ≡
/// typing machine differential (via [`agree_value`] / [`agree_comp`]), exactly
/// as the declared-data forms beside them; their runtime β is pinned on the L
/// machine in `gandr_core_sequent::conformance_soundness`. The genuinely
/// *dependent*
/// tail is the load-bearing witness: `Σ(x : Integer). Path Integer x x` is a
/// family whose tail mentions the bound value, so a well-typed second component
/// depends on the first.
mod levitation
{
    use gandr_core_checker::subtype::value_subtype;

    use super::*;
    /// A pair whose second component is a reflexivity proof *of its first*
    /// checks against the dependent pair, checker ≡ machine (rule Sigma⇓): the
    /// tail `Path Integer x x` is instantiated at the actual first component
    /// `3`, so `here(3) : Path Integer 3 3` fits `B[3/x]`.
    #[test]
    fn sigma_intro_dependent_pair_checks_and_agrees()
    {
        let sigma = refl_sigma();
        let pair = Value::pair(Value::int(3), Value::here(Value::int(3)));
        let result = agree_value(&Ctx::new(), &pair, &Dir::Check(sigma.clone()));
        assert_eq!(result, Ok(Ty::Value(sigma)));
    }

    /// The dependency is REAL: a pair whose reflexivity proof is of a
    /// *different* value fails, because the tail was instantiated at the
    /// first component — `here(5) : Path Integer 5 5` does not fit `B[3/x]
    /// = Path Integer 3 3`. Both passes reject identically.
    #[test]
    fn sigma_intro_mismatched_tail_is_rejected()
    {
        let pair = Value::pair(Value::int(3), Value::here(Value::int(5)));
        let result = agree_value(&Ctx::new(), &pair, &Dir::Check(refl_sigma()));
        assert!(
            matches!(result, Err(TypeError::TypeMismatch { .. })),
            "the tail Path Integer 3 3 rejects a proof about 5 (ADR-81 dependency)"
        );
    }

    /// Eliminating a `Σ`-typed scrutinee binds the first binder at the head and
    /// the second at the substituted tail (rule Sigma⇕), checker ≡ machine: the
    /// body `ret x` returns the first component at `Integer` (the answer keeps
    /// the binders out of scope — the non-dependent-motive discipline).
    #[test]
    fn sigma_elim_split_checks_and_agrees()
    {
        let mut ctx = Ctx::new();
        ctx.bind("p".to_owned(), refl_sigma());
        let split = Comp::split(Value::var("p"), "x", "q", Comp::ret(Value::var("x")));
        let expected = CompType::returner(ValueType::integer());
        let result = agree_comp(&ctx, &split, &Dir::Check(expected.clone()));
        assert_eq!(result, Ok(Ty::Comp(expected)));
    }

    // The Σ-typed split's β-rule (`split (3, here 3) as (x, q) in ret x` ↦
    // `ret 3`) is pinned on the L machine in
    // `gandr_core_sequent::conformance_soundness`.

    /// The code universe is its own subtype and nothing else's (ADR-81 feature
    /// 1; no cumulativity per ADR-78): `Universe <: Universe`, but `Universe`
    /// relates to no other former in either direction.
    #[test]
    fn universe_subtyping_is_reflexive_only()
    {
        assert!(bool::from(value_subtype(
            &ValueType::Universe,
            &ValueType::Universe
        )));
        assert!(!bool::from(value_subtype(
            &ValueType::Universe,
            &ValueType::Unit
        )));
        assert!(!bool::from(value_subtype(
            &ValueType::Unit,
            &ValueType::Universe
        )));
    }

    /// The dependent pair is invariant up to α-renaming of the binder (ADR-81
    /// feature 2): `Σ(x:Int). Path Int x x` and `Σ(y:Int). Path Int y y` relate
    /// (the bound name is not observable), but a `Σ` whose tail differs does
    /// not.
    #[test]
    fn sigma_subtyping_is_invariant_up_to_alpha()
    {
        let sigma_x = refl_sigma();
        let sigma_y = ValueType::sigma(
            ValueType::integer(),
            "y",
            ValueType::path(ValueType::integer(), Value::var("y"), Value::var("y")),
        );
        assert!(
            bool::from(value_subtype(&sigma_x, &sigma_y)),
            "α-equivalent Σ types relate"
        );
        assert!(
            bool::from(value_subtype(&sigma_y, &sigma_x)),
            "…and symmetrically"
        );
        // A Σ whose head differs does not relate (invariant head).
        let sigma_str = ValueType::sigma(
            ValueType::string(),
            "x",
            ValueType::path(ValueType::string(), Value::var("x"), Value::var("x")),
        );
        assert!(
            !bool::from(value_subtype(&sigma_x, &sigma_str)),
            "invariant in the head"
        );
    }

    /// `Σ(x : Integer). Path Integer x x` — a dependent pair whose tail
    /// mentions the bound value `x`.
    fn refl_sigma() -> ValueType
    {
        ValueType::sigma(
            ValueType::integer(),
            "x",
            ValueType::path(ValueType::integer(), Value::var("x"), Value::var("x")),
        )
    }
}

/// **The package rung.** Packing, unpacking, and the two properties the rung
/// exists for: a client meets abstract types rather than the representation,
/// and a package is not a graded thunk however alike the two look.
mod packages
{
    use gandr_core_checker::boundary::NameRef;
    use gandr_core_checker::boundary::TypeSerial;
    use gandr_core_checker::package;
    use gandr_core_checker::types::SealId;

    use super::*;

    /// `U_grade (F payload)` — the thunked module returner a package carries.
    fn returner_thunk(
        grade: Grade,
        payload: ValueType,
    ) -> ValueType
    {
        ValueType::thunk(grade, CompType::F(Rc::new(payload), EffectRow::EMPTY))
    }

    /// The worked signature: a counter abstracting its state type `t`, with a
    /// seed of that type and a reader from it to `Integer`.
    ///
    /// `Package_grade ⟨t⟩ U_grade (F #{ read: U_ω (t → F Integer), seed: t })`.
    fn counter(grade: Grade) -> ValueType
    {
        ValueType::package(
            grade,
            ["t"],
            returner_thunk(
                grade,
                ValueType::record([
                    (
                        "read".to_owned(),
                        ValueType::thunk(
                            Grade::OMEGA,
                            CompType::arrow(
                                ValueType::atom("t"),
                                CompType::returner(ValueType::integer()),
                            ),
                        ),
                    ),
                    ("seed".to_owned(), ValueType::atom("t")),
                ]),
            ),
        )
    }

    /// A counter whose state is an integer: the seed is a literal and the
    /// reader is the identity.
    fn integer_counter() -> Value
    {
        Value::pack(
            [ValueType::integer()],
            Value::thunk(
                Grade::OMEGA,
                Comp::ret(Value::record([
                    (
                        "read".to_owned(),
                        Value::thunk(
                            Grade::OMEGA,
                            Comp::lam_ann("n", ValueType::integer(), Comp::ret(Value::var("n"))),
                        ),
                    ),
                    ("seed".to_owned(), Value::int(7_i64)),
                ])),
            ),
        )
    }

    /// A counter whose state is a *string*: a structurally different
    /// representation behind the same signature, which is what makes the
    /// abstraction worth having.
    fn string_counter() -> Value
    {
        Value::pack(
            [ValueType::string()],
            Value::thunk(
                Grade::OMEGA,
                Comp::ret(Value::record([
                    (
                        "read".to_owned(),
                        Value::thunk(
                            Grade::OMEGA,
                            Comp::lam_ann("s", ValueType::string(), Comp::ret(Value::int(7_i64))),
                        ),
                    ),
                    ("seed".to_owned(), Value::string("seven")),
                ])),
            ),
        )
    }

    /// The atom one unpack mints for the counter's single component.
    fn atom(serial: TypeSerial) -> SealId
    {
        SealId::new(serial, "counter", "t")
    }

    /// The consumer body: force the module, project its reader and its seed,
    /// and apply the one to the other — the only route the signature offers,
    /// and the only route that type-checks.
    fn read_the_seed(module: NameRef<'_>) -> Comp
    {
        let module = <&str>::from(module);
        Comp::bind(
            Comp::force(Value::var(module)),
            "r",
            Comp::bind(
                Comp::record_proj(Value::var("r"), "read"),
                "f",
                Comp::bind(
                    Comp::record_proj(Value::var("r"), "seed"),
                    "s",
                    Comp::app(Comp::force(Value::var("f")), Value::var("s")),
                ),
            ),
        )
    }

    /// Packing checks against the signature, checker and machine agreeing step
    /// for step: the payload is checked at the *witness* type, so the packer's
    /// representation is checked exactly once.
    #[test]
    fn packing_checks_against_its_signature_and_agrees()
    {
        let signature = counter(Grade::OMEGA);
        for implementation in [integer_counter(), string_counter()] {
            let result = agree_value(&Ctx::new(), &implementation, &Dir::Check(signature.clone()));
            assert_eq!(
                result,
                Ok(Ty::Value(signature.clone())),
                "either representation packs at the same signature"
            );
        }
    }

    /// A pack whose payload does not match the signature at the supplied
    /// witness is rejected — the representation is checked, not assumed.
    #[test]
    fn packing_a_mismatched_payload_is_rejected()
    {
        let mismatched = Value::pack(
            [ValueType::integer()],
            Value::thunk(
                Grade::OMEGA,
                Comp::ret(Value::record([
                    (
                        "read".to_owned(),
                        Value::thunk(
                            Grade::OMEGA,
                            Comp::lam_ann("n", ValueType::integer(), Comp::ret(Value::var("n"))),
                        ),
                    ),
                    ("seed".to_owned(), Value::string("not an integer")),
                ])),
            ),
        );
        let result = agree_value(&Ctx::new(), &mismatched, &Dir::Check(counter(Grade::OMEGA)));
        assert!(
            matches!(result, Err(TypeError::TypeMismatch { .. })),
            "the seed is checked at the witness type Integer, so a string seed fails"
        );
    }

    /// A pack in inference position is stuck: the abstract components live only
    /// in the signature, so there is nothing to infer them from.
    #[test]
    fn packing_never_infers()
    {
        let result = agree_value(&Ctx::new(), &integer_counter(), &Dir::Infer);
        assert!(
            matches!(
                result,
                Err(TypeError::StuckExpr {
                    hint: gandr_core_checker::error::text::ANNOTATE_PACK,
                    ..
                })
            ),
            "a package type is never guessed from a payload"
        );
    }

    /// A witness count that is not the signature's arity is refused where it is
    /// written.
    #[test]
    fn packing_with_the_wrong_arity_is_refused()
    {
        let extra = Value::pack(
            [ValueType::integer(), ValueType::string()],
            Value::thunk(Grade::OMEGA, Comp::ret(Value::Unit)),
        );
        let result = agree_value(&Ctx::new(), &extra, &Dir::Check(counter(Grade::OMEGA)));
        assert!(
            matches!(
                result,
                Err(TypeError::StuckExpr {
                    hint: gandr_core_checker::error::text::PACK_ARITY_MISMATCH,
                    ..
                })
            ),
            "one witness per abstract type component, no more and no fewer"
        );
    }

    /// **The application consumer.** Unpacking binds the module at abstract
    /// types and the body reaches its `Integer` answer through the signature's
    /// own operation — checker and machine agreeing step for step.
    #[test]
    fn unpacking_lets_a_consumer_use_the_module_through_its_signature()
    {
        let signature = counter(Grade::OMEGA);
        let mut ctx = Ctx::new();
        ctx.bind("p".to_owned(), signature.clone());
        let unpack = Comp::unpack(
            Value::var("p"),
            signature,
            [atom(TypeSerial::from(0_u64))],
            "m",
            read_the_seed(NameRef::from("m")),
        );
        let expected = CompType::returner(ValueType::integer());
        let result = agree_comp(&ctx, &unpack, &Dir::Check(expected.clone()));
        assert_eq!(
            result,
            Ok(Ty::Comp(expected)),
            "the consumer reaches its answer through the signature's reader"
        );
    }

    /// **The abstraction.** The same consumer body, with the seed used as an
    /// `Integer` directly, is rejected: inside the unpack the seed has the
    /// minted atom's type and relates to no representation at all.
    #[test]
    fn a_consumer_cannot_reach_the_representation()
    {
        let signature = counter(Grade::OMEGA);
        let mut ctx = Ctx::new();
        ctx.bind("p".to_owned(), signature.clone());
        let leak = Comp::bind(
            Comp::force(Value::var("m")),
            "r",
            Comp::bind(
                Comp::record_proj(Value::var("r"), "seed"),
                "s",
                Comp::ret(Value::var("s")),
            ),
        );
        let unpack = Comp::unpack(
            Value::var("p"),
            signature,
            [atom(TypeSerial::from(0_u64))],
            "m",
            leak,
        );
        let result = agree_comp(
            &ctx,
            &unpack,
            &Dir::Check(CompType::returner(ValueType::integer())),
        );
        assert!(
            matches!(result, Err(TypeError::TypeMismatch { .. })),
            "the seed is abstract inside the unpack, whatever the packer supplied"
        );
    }

    /// **Atom freshness per unpack.** Two eliminations mint two atoms, and the
    /// abstract types they bind do not interchange: the inner body cannot feed
    /// the outer module's reader with the inner module's seed.
    #[test]
    fn two_unpacks_do_not_interchange_their_abstract_types()
    {
        let signature = counter(Grade::OMEGA);
        let mut ctx = Ctx::new();
        ctx.bind("p".to_owned(), signature.clone());
        // outer: bind `f` to the outer module's reader; inner: bind `s` to the
        // inner module's seed; then apply the one to the other.
        let inner = Comp::bind(
            Comp::force(Value::var("n")),
            "r2",
            Comp::bind(
                Comp::record_proj(Value::var("r2"), "seed"),
                "s",
                Comp::app(Comp::force(Value::var("f")), Value::var("s")),
            ),
        );
        let outer = Comp::bind(
            Comp::force(Value::var("m")),
            "r1",
            Comp::bind(
                Comp::record_proj(Value::var("r1"), "read"),
                "f",
                Comp::unpack(
                    Value::var("p"),
                    signature.clone(),
                    [atom(TypeSerial::from(1_u64))],
                    "n",
                    inner,
                ),
            ),
        );
        let crossed = Comp::unpack(
            Value::var("p"),
            signature.clone(),
            [atom(TypeSerial::from(0_u64))],
            "m",
            outer,
        );
        let expected = CompType::returner(ValueType::integer());
        let result = agree_comp(&ctx, &crossed, &Dir::Check(expected.clone()));
        assert!(
            matches!(result, Err(TypeError::TypeMismatch { .. })),
            "two unpacks mint two atoms, so their abstract types are not interchangeable"
        );
        // The same body, kept inside one unpack, is well typed — so the failure
        // above is the freshness and not a defect in the term.
        let straight = Comp::unpack(
            Value::var("p"),
            signature,
            [atom(TypeSerial::from(0_u64))],
            "m",
            read_the_seed(NameRef::from("m")),
        );
        assert_eq!(
            agree_comp(&ctx, &straight, &Dir::Check(expected.clone())),
            Ok(Ty::Comp(expected)),
            "one unpack's own reader and seed do meet"
        );
    }

    /// An unpack in inference position is stuck, which is the avoidance fence:
    /// the answer comes from outside, so a minted atom can never escape into
    /// it.
    #[test]
    fn unpacking_never_infers()
    {
        let signature = counter(Grade::OMEGA);
        let mut ctx = Ctx::new();
        ctx.bind("p".to_owned(), signature.clone());
        let unpack = Comp::unpack(
            Value::var("p"),
            signature,
            [atom(TypeSerial::from(0_u64))],
            "m",
            read_the_seed(NameRef::from("m")),
        );
        let result = agree_comp(&ctx, &unpack, &Dir::Infer);
        assert!(
            matches!(
                result,
                Err(TypeError::StuckExpr {
                    hint: gandr_core_checker::error::text::UNPACK_NEEDS_CHECK,
                    ..
                })
            ),
            "the expectation is what keeps a minted atom inside its scope"
        );
    }

    /// **The grade leg, and what the Q4 ruling buys.** A `Package_0` may be
    /// carried and never opened: unpacking demands `1 ⊑ r`, exactly as forcing
    /// a thunk does.
    #[test]
    fn a_grade_zero_package_cannot_be_unpacked()
    {
        let signature = counter(Grade::ZERO);
        let mut ctx = Ctx::new();
        ctx.bind("p".to_owned(), signature.clone());
        let unpack = Comp::unpack(
            Value::var("p"),
            signature,
            [atom(TypeSerial::from(0_u64))],
            "m",
            read_the_seed(NameRef::from("m")),
        );
        let result = agree_comp(
            &ctx,
            &unpack,
            &Dir::Check(CompType::returner(ValueType::integer())),
        );
        assert_eq!(
            result,
            Err(TypeError::GradeError {
                lower: Grade::ONE,
                upper: Grade::ZERO,
            }),
            "a grade-zero package is transportable and unopenable"
        );
    }

    /// **The representation is distinct from graded composition, in both
    /// directions.** Forcing a package is a shape mismatch, and unpacking a
    /// thunk is one too; neither former's eliminator reaches the other.
    #[test]
    fn a_package_is_not_a_thunk_at_either_eliminator()
    {
        let mut ctx = Ctx::new();
        ctx.bind("p".to_owned(), counter(Grade::OMEGA));
        ctx.bind(
            "t".to_owned(),
            returner_thunk(Grade::OMEGA, ValueType::integer()),
        );
        let forced = agree_comp(
            &ctx,
            &Comp::force(Value::var("p")),
            &Dir::Check(CompType::returner(ValueType::integer())),
        );
        assert!(
            matches!(
                forced,
                Err(TypeError::ShapeMismatch {
                    expected: gandr_core_checker::error::text::SHAPE_THUNK,
                    ..
                })
            ),
            "force refuses a package, so no client opens one without minting"
        );
        let unpacked = Comp::unpack(
            Value::var("t"),
            returner_thunk(Grade::OMEGA, ValueType::integer()),
            [atom(TypeSerial::from(0_u64))],
            "m",
            Comp::ret(Value::int(0_i64)),
        );
        let result = agree_comp(
            &ctx,
            &unpacked,
            &Dir::Check(CompType::returner(ValueType::integer())),
        );
        assert!(
            matches!(
                result,
                Err(TypeError::ShapeMismatch {
                    expected: gandr_core_checker::error::text::SHAPE_PACKAGE,
                    ..
                })
            ),
            "unpack refuses a thunk, so the two formers stay apart"
        );
    }

    /// Subtyping relates a package to a package and to nothing else, so no
    /// coercion to or from a thunk exists in either direction.
    #[test]
    fn no_subtyping_bridges_a_package_and_a_thunk()
    {
        let package = counter(Grade::OMEGA);
        let ValueType::Package { ref payload, .. } = package
        else {
            panic!("the counter signature is a package");
        };
        let thunk = payload.as_ref().clone();
        assert!(
            !bool::from(value_subtype(&package, &thunk)),
            "a package is not a subtype of the thunk it carries"
        );
        assert!(
            !bool::from(value_subtype(&thunk, &package)),
            "nor is that thunk a subtype of the package"
        );
    }

    /// Package subtyping is invariant in the payload up to α-renaming of the
    /// binders and contravariant in the grade — the `Thunk` leg, on a former
    /// that shares no representation with it.
    #[test]
    fn package_subtyping_is_alpha_invariant_and_grade_contravariant()
    {
        let spelled_t = counter(Grade::OMEGA);
        let spelled_u = ValueType::package(
            Grade::OMEGA,
            ["u"],
            returner_thunk(
                Grade::OMEGA,
                ValueType::record([
                    (
                        "read".to_owned(),
                        ValueType::thunk(
                            Grade::OMEGA,
                            CompType::arrow(
                                ValueType::atom("u"),
                                CompType::returner(ValueType::integer()),
                            ),
                        ),
                    ),
                    ("seed".to_owned(), ValueType::atom("u")),
                ]),
            ),
        );
        assert!(
            bool::from(value_subtype(&spelled_t, &spelled_u)),
            "two spellings of one signature relate"
        );
        assert!(
            bool::from(value_subtype(&spelled_u, &spelled_t)),
            "and they relate in both directions, the payload being invariant"
        );
        let once = counter(Grade::ONE);
        assert!(
            bool::from(value_subtype(&spelled_t, &once)),
            "a package openable ω times is usable where one opening is expected"
        );
        assert!(
            !bool::from(value_subtype(&once, &spelled_t)),
            "the reverse fails: the grade leg is contravariant"
        );
        let two_components = ValueType::package(
            Grade::OMEGA,
            ["t", "u"],
            returner_thunk(Grade::OMEGA, ValueType::atom("t")),
        );
        assert!(
            !bool::from(value_subtype(&spelled_t, &two_components)),
            "packages of different arity relate to nothing"
        );
    }

    /// **Malformed payloads are rejected rather than normalized.** A package
    /// whose payload thunk is graded other than the package itself relates to
    /// nothing: the grade normalization applies only where the two already
    /// agree, so the comparator is a backstop behind the typing rules rather
    /// than a repair that hides the defect.
    #[test]
    fn a_malformed_payload_grade_relates_to_nothing()
    {
        let well_formed = counter(Grade::OMEGA);
        let malformed = ValueType::package(
            Grade::OMEGA,
            ["t"],
            // The payload thunk is graded ONE where the package is graded ω.
            returner_thunk(
                Grade::ONE,
                ValueType::record([
                    (
                        "read".to_owned(),
                        ValueType::thunk(
                            Grade::OMEGA,
                            CompType::arrow(
                                ValueType::atom("t"),
                                CompType::returner(ValueType::integer()),
                            ),
                        ),
                    ),
                    ("seed".to_owned(), ValueType::atom("t")),
                ]),
            ),
        );
        assert!(
            !bool::from(value_subtype(&malformed, &well_formed)),
            "a payload graded other than its package is not silently normalized"
        );
        assert!(
            !bool::from(value_subtype(&well_formed, &malformed)),
            "nor in the other direction"
        );
        assert!(
            bool::from(value_subtype(&malformed, &malformed)),
            "it still relates to itself, so the refusal is about the disagreement"
        );
        // And the typing rules refuse it ahead of subtyping, which is where the
        // check belongs; the comparator above is the backstop.
        let mut ctx = Ctx::new();
        ctx.bind("p".to_owned(), malformed.clone());
        let result = agree_comp(
            &ctx,
            &Comp::unpack(
                Value::var("p"),
                malformed,
                [atom(TypeSerial::from(0_u64))],
                "m",
                Comp::ret(Value::Unit),
            ),
            &Dir::Check(CompType::returner(ValueType::Unit)),
        );
        assert!(
            matches!(
                result,
                Err(TypeError::ShapeMismatch {
                    expected: gandr_core_checker::error::text::SHAPE_PACKAGE_PAYLOAD,
                    ..
                })
            ),
            "well-formedness is decided at the rule, before any subtyping question"
        );
    }

    /// The renaming subtyping performs cannot be captured by a source-level
    /// atom, because the canonical binders carry a character no identifier
    /// does.
    #[test]
    fn alignment_cannot_be_captured_by_a_source_atom()
    {
        let hostile = ValueType::package(
            Grade::OMEGA,
            ["t"],
            returner_thunk(
                Grade::OMEGA,
                ValueType::prod(
                    ValueType::atom("t"),
                    ValueType::atom(
                        package::canonical_binder(
                            gandr_core_checker::boundary::PackageArity::from(0_usize),
                        )
                        .as_str(),
                    ),
                ),
            ),
        );
        assert!(
            bool::from(value_subtype(&hostile, &hostile)),
            "a payload naming a canonical binder still relates to itself"
        );
        let benign = ValueType::package(
            Grade::OMEGA,
            ["t"],
            returner_thunk(
                Grade::OMEGA,
                ValueType::prod(ValueType::atom("t"), ValueType::integer()),
            ),
        );
        assert!(
            !bool::from(value_subtype(&hostile, &benign)),
            "and it does not relate to a payload that differs there"
        );
    }
}

/// The dependent split motive (ADR-82): the motive-bearing split *infers*
/// (rule `SplitMotive`⇑), the motive-less split is *check-only* (rule Split⇓),
/// and the binder-escape hazard reported by ADR-81 is closed at the rule. The
/// checker ≡ typing machine differential (via [`agree_comp`]) holds for the new
/// forms exactly as for the `Σ` family they sit beside; the split-β evaluation
/// is pinned on the L machine in `gandr_core_sequent::conformance_soundness`.
mod split_motive
{
    use super::*;
    /// (a) A **dependent-motive** split over a `Prod` scrutinee infers, checker
    /// ≡ machine (rule `SplitMotive`⇑): `split (3, 4) as (x, y) [z. F (Path
    /// (Int×Int) z z)] in ret (here (x, y))` infers `F (Path (Int×Int) (3, 4)
    /// (3, 4))` — the answer is `M[v/z]`, substituted from the outer motive and
    /// the scrutinee, so no binder occurs in it. The split-β evaluation
    /// (`↦ ret (here (3, 4))`) is pinned on the L machine in
    /// `gandr_core_sequent::conformance_soundness`.
    #[test]
    fn dependent_split_over_prod_infers_and_agrees()
    {
        let scrut = Value::pair(Value::int(3), Value::int(4));
        let motive = SplitMotive::new(
            "z",
            CompType::returner(ValueType::path(
                int_prod(),
                Value::var("z"),
                Value::var("z"),
            )),
        );
        let split = Comp::split_motive(
            scrut.clone(),
            "x",
            "y",
            motive,
            Comp::ret(Value::here(Value::pair(Value::var("x"), Value::var("y")))),
        );
        let answer = CompType::returner(ValueType::path(int_prod(), scrut.clone(), scrut));
        assert_eq!(
            agree_comp(&Ctx::new(), &split, &Dir::Infer),
            Ok(Ty::Comp(answer)),
            "the delivered answer is the substituted M[v/z], binder-free"
        );
    }
    /// (a) A **motive-bearing** split over a `Σ`-typed scrutinee infers,
    /// checker ≡ machine (rule `SplitMotive`⇑): the constant motive `(z). F
    /// Integer` supplies the answer, the `Σ` tail's dependency is discharged at
    /// the binder, and `split ((3, here 3) : Σ(x:Int). Path Int x x) as (x, q)
    /// [z. F Integer] in ret x` infers `F Integer`. Its `↦ ret 3` evaluation is
    /// pinned on the L machine in `gandr_core_sequent::conformance_soundness`.
    #[test]
    fn motive_split_over_sigma_infers_and_agrees()
    {
        let scrut = Value::annot(
            Value::pair(Value::int(3), Value::here(Value::int(3))),
            refl_sigma(),
        );
        let split = Comp::split_motive(
            scrut,
            "x",
            "q",
            SplitMotive::new("z", CompType::returner(ValueType::integer())),
            Comp::ret(Value::var("x")),
        );
        let answer = CompType::returner(ValueType::integer());
        assert_eq!(
            agree_comp(&Ctx::new(), &split, &Dir::Infer),
            Ok(Ty::Comp(answer))
        );
    }

    /// (b) The **regression witness** (ADR-82 D5b): a split in inference
    /// position whose body's `Path`-bearing answer type mentions a split binder
    /// — `split (3, 4) as (x, y) in ret (here x)`, whose body infers `F (Path
    /// Integer x x)` with the binder `x` escaping — was **wrongly accepted** by
    /// the frozen Split⇕ (which returned the body's type verbatim). It is now
    /// **stuck** with the needs-motive hint, firing at rule entry, checker ≡
    /// machine.
    #[test]
    fn inference_position_binder_escape_is_now_stuck()
    {
        let split = Comp::split(
            Value::pair(Value::int(3), Value::int(4)),
            "x",
            "y",
            Comp::ret(Value::here(Value::var("x"))),
        );
        let result = agree_comp(&Ctx::new(), &split, &Dir::Infer);
        assert!(
            matches!(
                result,
                Err(TypeError::StuckExpr { hint, .. })
                    if hint == gandr_core_checker::error::text::SPLIT_NEEDS_MOTIVE
            ),
            "a motive-less split cannot infer — the binder-escape hazard is \
             closed at the rule (ADR-82); got {result:?}"
        );
    }

    /// (c) The **repaired twin** (ADR-82 D5c): the escaping `here x` is
    /// replaced by `here (x, y)` — the reconstructed *whole scrutinee* —
    /// under a motive `(z). F (Path (Int×Int) z z)` that can express it.
    /// The split now infers the **substituted** answer `M[v/z] = F (Path
    /// (Int×Int) (3, 4) (3, 4))`, which is binder-free by construction (the
    /// scrutinee is substituted from outside), checker ≡ machine.
    #[test]
    fn motive_repairs_the_escape_at_the_substituted_answer()
    {
        let scrut = Value::pair(Value::int(3), Value::int(4));
        let motive = SplitMotive::new(
            "z",
            CompType::returner(ValueType::path(
                int_prod(),
                Value::var("z"),
                Value::var("z"),
            )),
        );
        let split = Comp::split_motive(
            scrut.clone(),
            "x",
            "y",
            motive,
            Comp::ret(Value::here(Value::pair(Value::var("x"), Value::var("y")))),
        );
        let answer = CompType::returner(ValueType::path(int_prod(), scrut.clone(), scrut));
        assert_eq!(
            agree_comp(&Ctx::new(), &split, &Dir::Infer),
            Ok(Ty::Comp(answer)),
            "the delivered answer is exactly M[v/z]"
        );
    }

    /// The product `Integer × Integer`.
    fn int_prod() -> ValueType
    {
        ValueType::prod(ValueType::integer(), ValueType::integer())
    }

    /// (d) A **motive-less checking** split over a `Prod` stays green (rule
    /// Split⇓): `split (1, 2) as (x, y) in ret x` checked against `F Integer`
    /// binds both components and delivers the expectation verbatim, checker ≡
    /// machine.
    #[test]
    fn motive_less_checking_split_over_prod_stays_green()
    {
        let split = Comp::split(
            Value::pair(Value::int(1), Value::int(2)),
            "x",
            "y",
            Comp::ret(Value::var("x")),
        );
        let expected = CompType::returner(ValueType::integer());
        assert_eq!(
            agree_comp(&Ctx::new(), &split, &Dir::Check(expected.clone())),
            Ok(Ty::Comp(expected))
        );
    }

    /// (d) A **motive-less checking** split over a `Σ`-typed scrutinee stays
    /// green (rule Split⇓): the `Σ` tail's dependency is discharged at the
    /// binder, and the body delivers the expectation verbatim, checker ≡
    /// machine.
    #[test]
    fn motive_less_checking_split_over_sigma_stays_green()
    {
        let mut ctx = Ctx::new();
        ctx.bind("p".to_owned(), refl_sigma());
        let split = Comp::split(Value::var("p"), "x", "q", Comp::ret(Value::var("x")));
        let expected = CompType::returner(ValueType::integer());
        assert_eq!(
            agree_comp(&ctx, &split, &Dir::Check(expected.clone())),
            Ok(Ty::Comp(expected))
        );
    }

    /// `Σ(x : Integer). Path Integer x x` — the dependent pair reused from the
    /// levitation differentials (its tail mentions the bound value `x`).
    fn refl_sigma() -> ValueType
    {
        ValueType::sigma(
            ValueType::integer(),
            "x",
            ValueType::path(ValueType::integer(), Value::var("x"), Value::var("x")),
        )
    }
}

/// Example-based tests over the core-CBPV worked examples.
mod examples
{
    use gandr_core_checker::control::Control;

    use super::*;

    /// Example 1 (identity): `λx:A. ret x ⇑ A → F A`, with the literal
    /// machine trace given in the worked example.
    #[test]
    fn example_1_identity()
    {
        let atom_a = ValueType::atom("A");
        let identity = Comp::lam_ann("x", atom_a.clone(), Comp::ret(Value::var("x")));
        let expected_ty = CompType::arrow(atom_a.clone(), CompType::returner(atom_a.clone()));

        let result = agree_comp(&Ctx::new(), &identity, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(expected_ty.clone())),
            "identity must infer A → F A"
        );

        let (_, trace) = machine::run_comp(Ctx::new(), identity.clone(), Dir::Infer);
        let expected_trace = vec![
            Control::DescendComp {
                comp: identity,
                dir: Dir::Infer,
            },
            Control::DescendComp {
                comp: Comp::ret(Value::var("x")),
                dir: Dir::Infer,
            },
            Control::DescendValue {
                value: Value::var("x"),
                dir: Dir::Infer,
            },
            Control::Return {
                ty: Ty::Value(atom_a.clone()),
            },
            Control::Return {
                ty: Ty::Comp(CompType::returner(atom_a)),
            },
            Control::Return {
                ty: Ty::Comp(expected_ty),
            },
        ];
        assert_eq!(
            trace, expected_trace,
            "machine trace must match the worked example"
        );
    }

    /// Example 2 (composition): the inferred type is
    /// `U_ω(B → F C) → U_ω(A → F B) → A → F C`.
    #[test]
    fn example_2_composition()
    {
        let atom_a = ValueType::atom("A");
        let atom_b = ValueType::atom("B");
        let atom_c = ValueType::atom("C");
        let f_ty = ValueType::thunk(
            Grade::OMEGA,
            CompType::arrow(atom_b.clone(), CompType::returner(atom_c.clone())),
        );
        let g_ty = ValueType::thunk(
            Grade::OMEGA,
            CompType::arrow(atom_a.clone(), CompType::returner(atom_b)),
        );
        let body = Comp::bind(
            Comp::app(Comp::force(Value::var("g")), Value::var("x")),
            "y",
            Comp::app(Comp::force(Value::var("f")), Value::var("y")),
        );
        let compose = Comp::lam_ann(
            "f",
            f_ty.clone(),
            Comp::lam_ann("g", g_ty.clone(), Comp::lam_ann("x", atom_a.clone(), body)),
        );
        let expected_ty = CompType::arrow(
            f_ty,
            CompType::arrow(g_ty, CompType::arrow(atom_a, CompType::returner(atom_c))),
        );

        let result = agree_comp(&Ctx::new(), &compose, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(expected_ty)),
            "composition must infer U(B → F C) → U(A → F B) → A → F C"
        );
    }

    /// Example 3a (tagged sum with case): the case expression checks against
    /// `F Int`; the annotated scrutinee infers `Int + Str`.
    #[test]
    fn example_3a_case_on_annotated_injection()
    {
        let scrut = Value::annot(Value::inj1(Value::var("i")), ValueType::sum(int(), txt()));
        let case = Comp::case(
            scrut,
            "x",
            Comp::ret(Value::var("x")),
            "y",
            Comp::ret(Value::var("i")),
        );
        let result = agree_comp(&base_ctx(), &case, &Dir::Check(CompType::returner(int())));
        assert_eq!(
            result,
            Ok(Ty::Comp(CompType::returner(int()))),
            "the case expression must check against F Int"
        );
    }

    /// Example 4, core subset: the same unannotated body checks under a
    /// thunk expectation (the full intersection version is Stage 3).
    #[test]
    fn example_4_checked_thunk_of_unannotated_identity()
    {
        let overload = Value::thunk(Grade::OMEGA, Comp::lam("x", Comp::ret(Value::var("x"))));
        let expected = ValueType::thunk(
            Grade::OMEGA,
            CompType::arrow(int(), CompType::returner(int())),
        );
        let result = agree_value(&Ctx::new(), &overload, &Dir::Check(expected.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(expected)),
            "thunk_ω (λx. ret x) must check against U_ω (Int → F Int)"
        );
    }

    /// (a8) Single-stepping Example 1 (`λx:A. ret x ⇑ A → F A`), asserting the
    /// stack [`machine::State::depth`] at each visited state. The frame stack
    /// grows `Abs`, then `Ret`, holds while the leaf `x` returns, then unwinds
    /// `Ret`, then `Abs` — so the depths are `0,1,2,2,1,0`, one per control of
    /// the worked trace.
    #[test]
    fn example_1_depth_per_step()
    {
        let atom_a = ValueType::atom("A");
        let identity = Comp::lam_ann("x", atom_a, Comp::ret(Value::var("x")));
        let mut state = machine::State::new_comp(Ctx::new(), identity, Dir::Infer);
        let mut depths = vec![state.depth()];
        // Loops while the machine steps; `Done` (expected) or an `Error` (which
        // would leave `depths` short and fail the assertion) ends it.
        while let machine::Outcome::Step(next) = machine::step(state) {
            state = next;
            depths.push(state.depth());
        }
        assert_eq!(
            depths,
            vec![
                gandr_core_checker::boundary::StackDepth::from(0_usize),
                gandr_core_checker::boundary::StackDepth::from(1),
                gandr_core_checker::boundary::StackDepth::from(2),
                gandr_core_checker::boundary::StackDepth::from(2),
                gandr_core_checker::boundary::StackDepth::from(1),
                gandr_core_checker::boundary::StackDepth::from(0),
            ],
            "the stack depth must follow the worked Example 1 derivation"
        );
    }
}

/// Positive rule-coverage tests beyond the worked examples.
mod positive
{
    use super::*;

    /// With⇓ and Prj⇑: a lazy pair checks componentwise, and projections
    /// infer from a forced thunk.
    #[test]
    fn with_checks_and_prj_infers()
    {
        let lazy = Comp::with(Comp::ret(Value::var("i")), Comp::ret(Value::var("s")));
        let with_ty = CompType::with(CompType::returner(int()), CompType::returner(txt()));
        let checked = agree_comp(&base_ctx(), &lazy, &Dir::Check(with_ty.clone()));
        assert_eq!(
            checked,
            Ok(Ty::Comp(with_ty.clone())),
            "⟨ret i, ret s⟩ must check"
        );

        let ctx = base_ctx().with("h", ValueType::thunk(Grade::ONE, with_ty));
        let prj = Comp::prj2(Comp::force(Value::var("h")));
        let inferred = agree_comp(&ctx, &prj, &Dir::Infer);
        assert_eq!(
            inferred,
            Ok(Ty::Comp(CompType::returner(txt()))),
            "prj2 must infer F Str"
        );
    }

    /// `SplitMotive`⇑ (ADR-82): a **motive-bearing** split infers — the
    /// constant motive `(z). F Str` supplies the answer, and the body `ret
    /// y` types under both binders (a motive-less split can no longer
    /// infer, rule Split⇓, so the inference position now takes the motive).
    #[test]
    fn split_infers_through_pair()
    {
        let split = Comp::split_motive(
            Value::pair(Value::var("i"), Value::var("s")),
            "x",
            "y",
            SplitMotive::new("z", CompType::returner(txt())),
            Comp::ret(Value::var("y")),
        );
        let result = agree_comp(&base_ctx(), &split, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(CompType::returner(txt()))),
            "a motive-bearing split must infer F Str"
        );
    }

    /// Subsumption on graded thunks: `U_ω B <: U_1 B` because `1 ⊑ ω`.
    #[test]
    fn thunk_grade_subsumption()
    {
        let ctx = base_ctx().with(
            "u",
            ValueType::thunk(Grade::OMEGA, CompType::returner(int())),
        );
        let expected = ValueType::thunk(Grade::ONE, CompType::returner(int()));
        let result = agree_value(&ctx, &Value::var("u"), &Dir::Check(expected.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(expected)),
            "U_ω (F Int) must check against U_1 (F Int)"
        );
    }

    /// Rule Dup (`type-system.md` §"Grades"): `dup (thunk_2 t)` splits the
    /// budget `2` into `1 + 1`, checking against `F (U_1 B × U_1 B)` since `1 +
    /// 1 ⊑ 2`. The result echoes the expectation (the inlined Sub rule
    /// discharges the shared body).
    #[test]
    fn dup_conserves_grade()
    {
        let thunk = Value::thunk(
            Grade::fin(gandr_core_checker::boundary::GradeBound::from(2)),
            Comp::ret(Value::int(1)),
        );
        let half = ValueType::thunk(Grade::ONE, CompType::returner(integer()));
        let expected = CompType::returner(ValueType::prod(half.clone(), half));
        let result = agree_comp(
            &Ctx::new(),
            &Comp::dup(thunk),
            &Dir::Check(expected.clone()),
        );
        assert_eq!(
            result,
            Ok(Ty::Comp(expected)),
            "dup must split U_2 into U_1 × U_1 (1 + 1 ⊑ 2)"
        );
    }

    /// Rule Drop (`type-system.md` §"Grades"): `drop (thunk_2 t)` discards the
    /// budget and infers `F 1`, independent of the thunk's grade and body.
    #[test]
    fn drop_discards_thunk_budget()
    {
        let thunk = Value::thunk(
            Grade::fin(gandr_core_checker::boundary::GradeBound::from(2)),
            Comp::ret(Value::int(1)),
        );
        let result = agree_comp(&Ctx::new(), &Comp::drop(thunk), &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(CompType::returner(ValueType::Unit))),
            "drop must infer F 1"
        );
    }

    /// Rule Dup with slack and subsumption: `dup (thunk_ω t)` checks against
    /// `F (U_1 (F Int) × U_1 ?)` — the conservation is *strict* (`1 + 1 ⊏ ω`)
    /// and the two declared factor bodies differ (`F Int` vs the hole type
    /// `Unknown`), each a supertype of the inferred `F Int`, so the inlined
    /// Sub rule discharges them independently against the expectation.
    #[test]
    fn dup_splits_with_slack_and_subsumed_bodies()
    {
        let thunk = Value::thunk(Grade::OMEGA, Comp::ret(Value::int(1)));
        let left = ValueType::thunk(Grade::ONE, CompType::returner(integer()));
        let right = ValueType::thunk(Grade::ONE, CompType::Unknown);
        let expected = CompType::returner(ValueType::prod(left, right));
        let result = agree_comp(
            &Ctx::new(),
            &Comp::dup(thunk),
            &Dir::Check(expected.clone()),
        );
        assert_eq!(
            result,
            Ok(Ty::Comp(expected)),
            "dup must split U_ω with slack (1 + 1 ⊏ ω) and subsume distinct factor bodies"
        );
    }

    /// Variable shadowing follows stack discipline in both implementations.
    #[test]
    fn shadowed_binders_resolve_innermost()
    {
        let shadowing = Comp::lam_ann(
            "x",
            int(),
            Comp::lam_ann("x", txt(), Comp::ret(Value::var("x"))),
        );
        let result = agree_comp(&Ctx::new(), &shadowing, &Dir::Infer);
        let expected = CompType::arrow(int(), CompType::arrow(txt(), CompType::returner(txt())));
        assert_eq!(result, Ok(Ty::Comp(expected)), "the inner binder must win");
    }

    /// Int⇑/Int⇓ (A2.1 literals extension): a literal infers the rigid
    /// `Integer` atom and checks against it by subsumption.
    #[test]
    fn integer_literal_infers_and_checks()
    {
        const INFERRED_INTEGER_LITERAL: i64 = 42;
        let inferred = agree_value(
            &Ctx::new(),
            &Value::int(INFERRED_INTEGER_LITERAL),
            &Dir::Infer,
        );
        assert_eq!(
            inferred,
            Ok(Ty::Value(integer())),
            "an integer literal must infer the Integer atom"
        );

        let checked = agree_value(&Ctx::new(), &Value::int(-7), &Dir::Check(integer()));
        assert_eq!(
            checked,
            Ok(Ty::Value(integer())),
            "an integer literal must check against the Integer atom"
        );
    }

    /// Str⇑/Str⇓ (value-model ladder, ADR-38): a literal infers the rigid
    /// `String` atom and checks against it by subsumption, exactly as Int.
    #[test]
    fn string_literal_infers_and_checks()
    {
        let inferred = agree_value(&Ctx::new(), &Value::string("hello"), &Dir::Infer);
        assert_eq!(
            inferred,
            Ok(Ty::Value(string())),
            "a string literal must infer the String atom"
        );

        let checked = agree_value(&Ctx::new(), &Value::string(""), &Dir::Check(string()));
        assert_eq!(
            checked,
            Ok(Ty::Value(string())),
            "a string literal must check against the String atom"
        );

        // A string against the Integer atom is a mismatch (distinct rigid
        // atoms), produced identically by checker and machine.
        let mismatch = agree_value(&Ctx::new(), &Value::string("x"), &Dir::Check(integer()));
        assert!(
            mismatch.is_err(),
            "a string literal must not check against the Integer atom"
        );
    }

    /// Num⇑/Num⇓ (value-model ladder, ADR-39 D1/D3/D5): a *suffixed* numeric
    /// literal infers and checks against the rigid atom of its tag,
    /// monomorphically — it does not widen to a wider numeric atom, and
    /// distinct numeric atoms are mutually incomparable. Checker ≡ machine
    /// throughout.
    #[test]
    fn numeric_literals_infer_and_check()
    {
        // Each suffixed literal infers exactly its atom.
        for (value, atom) in [
            (Value::u32(8080), ValueType::u32()),
            (Value::u64(100), ValueType::u64()),
            (Value::i32(255_i32), ValueType::i32()),
            (Value::i64(9), ValueType::i64()),
            (Value::f32(1.5), ValueType::f32()),
            (Value::f64(2.0_f64), ValueType::f64()),
        ] {
            assert_eq!(
                agree_value(&Ctx::new(), &value, &Dir::Infer),
                Ok(Ty::Value(atom.clone())),
                "a {atom:?} literal must infer its own atom"
            );
            assert_eq!(
                agree_value(&Ctx::new(), &value, &Dir::Check(atom.clone())),
                Ok(Ty::Value(atom.clone())),
                "a {atom:?} literal must check against its own atom"
            );
        }

        // No implicit widening between concrete numeric atoms (D5): a `u32`
        // literal does not check against `u64`, nor `f32` against `f64`.
        assert!(
            agree_value(&Ctx::new(), &Value::u32(1), &Dir::Check(ValueType::u64())).is_err(),
            "a u32 literal must not widen to u64 (no implicit widening)"
        );
        assert!(
            agree_value(&Ctx::new(), &Value::f32(1.0), &Dir::Check(ValueType::f64())).is_err(),
            "an f32 literal must not widen to f64 (no implicit widening)"
        );
    }

    /// The Rust `{integer}` literal rule (ADR-39 D4): an *unsuffixed* integer
    /// literal infers `Integer` but, in checking mode, also checks against any
    /// integer numeric atom it is representable in — while a *variable* of type
    /// `Integer` never widens (that stays plain atom subtyping). Checker ≡
    /// machine throughout.
    #[test]
    fn integer_literal_widens_in_checking_mode()
    {
        // Infers Integer (frozen A2.1 default).
        assert_eq!(
            agree_value(&Ctx::new(), &Value::int(42), &Dir::Infer),
            Ok(Ty::Value(integer())),
            "a bare integer literal still infers Integer"
        );

        // Checks against every integer atom it fits.
        for atom in [
            ValueType::u32(),
            ValueType::u64(),
            ValueType::i32(),
            ValueType::i64(),
        ] {
            assert_eq!(
                agree_value(&Ctx::new(), &Value::int(42), &Dir::Check(atom.clone())),
                Ok(Ty::Value(atom.clone())),
                "the literal 42 must check against {atom:?}"
            );
        }
        // …and still against Integer itself (the ordinary subsumption fallback).
        assert_eq!(
            agree_value(&Ctx::new(), &Value::int(42), &Dir::Check(integer())),
            Ok(Ty::Value(integer())),
            "42 must still check against Integer"
        );

        // Out-of-range / wrong-domain checks fail (identically on both passes):
        // a negative literal does not fit an unsigned atom, an over-large one
        // does not fit i32, and an integer literal never checks at a float atom.
        assert!(
            agree_value(&Ctx::new(), &Value::int(-1), &Dir::Check(ValueType::u32())).is_err(),
            "-1 must not check against u32"
        );
        assert!(
            agree_value(
                &Ctx::new(),
                &Value::int(i64::from(i32::MAX) + 1),
                &Dir::Check(ValueType::i32())
            )
            .is_err(),
            "i32::MAX + 1 must not check against i32"
        );
        assert!(
            agree_value(&Ctx::new(), &Value::int(1), &Dir::Check(ValueType::f64())).is_err(),
            "an integer literal must not check against a float atom"
        );
        // Exact unsigned upper boundary: u32::MAX fits, one past does not.
        assert_eq!(
            agree_value(
                &Ctx::new(),
                &Value::int(i64::from(u32::MAX)),
                &Dir::Check(ValueType::u32())
            ),
            Ok(Ty::Value(ValueType::u32())),
            "u32::MAX must check against u32"
        );
        assert!(
            agree_value(
                &Ctx::new(),
                &Value::int(i64::from(u32::MAX) + 1),
                &Dir::Check(ValueType::u32())
            )
            .is_err(),
            "u32::MAX + 1 must not check against u32"
        );

        // A *variable* of type Integer does NOT widen — D4 is literal-only.
        let ctx = Ctx::new().with("n", integer());
        assert!(
            agree_value(&ctx, &Value::var("n"), &Dir::Check(ValueType::u32())).is_err(),
            "an Integer-typed variable must not widen to u32"
        );
    }

    /// The float `Value::Num` carriers store IEEE-754 bits (ADR-39 D2), so
    /// structural equality is reflexive even for `NaN` — the invariant the
    /// incremental edit engine's self-diff-empty guarantee rests on (a
    /// native-`f64` `NumLit` would break it, since `NaN != NaN`). `±0.0` have
    /// distinct bit patterns and so are distinct literals. This locks the
    /// bit-storage representation choice against a future refactor.
    #[test]
    fn float_literals_compare_by_bits()
    {
        assert_eq!(
            Value::f64(f64::NAN),
            Value::f64(f64::NAN),
            "an f64 NaN literal must equal itself (bitwise Eq)"
        );
        assert_eq!(
            Value::f32(f32::NAN),
            Value::f32(f32::NAN),
            "an f32 NaN literal must equal itself (bitwise Eq)"
        );
        assert_ne!(
            Value::f64(0.0_f64),
            Value::f64(-0.0_f64),
            "+0.0 and -0.0 are distinct bit patterns, hence distinct literals"
        );
    }

    /// Bind⇕ in checking mode: the expected type flows into the continuation.
    #[test]
    fn bind_propagates_checking_direction()
    {
        let bound = Comp::bind(Comp::ret(Value::var("i")), "x", Comp::ret(Value::var("x")));
        let result = agree_comp(&base_ctx(), &bound, &Dir::Check(CompType::returner(int())));
        assert_eq!(
            result,
            Ok(Ty::Comp(CompType::returner(int()))),
            "ret i >>= x. ret x must check against F Int"
        );
    }
}

/// Negative tests: each failure mode is produced identically by both
/// implementations.
mod negative
{
    use gandr_core_checker::error::text;
    use gandr_core_checker::syntax::Term;

    use super::*;

    /// An unbound variable fails with `UnboundVariable`.
    #[test]
    fn unbound_variable()
    {
        let result = agree_value(&Ctx::new(), &Value::var("ghost"), &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::UnboundVariable {
                name: "ghost".to_owned()
            }),
            "an unbound variable must be reported"
        );
    }

    /// Injections do not infer (`typing-machine.md` §"The step function": stuck
    /// with a hint).
    #[test]
    fn injection_does_not_infer()
    {
        let inj = Value::inj1(Value::var("i"));
        let result = agree_value(&base_ctx(), &inj, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Value(inj),
                hint: text::ANNOTATE_INJECTION,
            }),
            "an injection in inference mode must be stuck with a hint"
        );
    }

    /// An unannotated abstraction does not infer.
    #[test]
    fn unannotated_abstraction_does_not_infer()
    {
        let lam = Comp::lam("x", Comp::ret(Value::var("x")));
        let result = agree_comp(&Ctx::new(), &lam, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(lam),
                hint: text::ANNOTATE_BINDER
            }),
            "an unannotated λ in inference mode must be stuck"
        );
    }

    /// Case is check-only (rule Case⇓).
    #[test]
    fn case_does_not_infer()
    {
        let scrut = Value::annot(Value::inj1(Value::var("i")), ValueType::sum(int(), txt()));
        let case = Comp::case(
            scrut,
            "x",
            Comp::ret(Value::var("x")),
            "y",
            Comp::ret(Value::var("i")),
        );
        let result = agree_comp(&base_ctx(), &case, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(case),
                hint: text::CASE_NEEDS_CHECK,
            }),
            "case in inference mode must be stuck"
        );
    }

    /// Forcing a non-thunk is a shape mismatch.
    #[test]
    fn force_of_non_thunk()
    {
        let force = Comp::force(Value::var("i"));
        let result = agree_comp(&base_ctx(), &force, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_THUNK,
                actual: Ty::Value(int())
            }),
            "forcing a non-thunk must be a shape mismatch"
        );
    }

    /// Forcing a `0`-graded thunk violates `1 ⊑ r` (rule Force⇑).
    #[test]
    fn force_of_zero_graded_thunk()
    {
        let ctx = base_ctx().with(
            "z",
            ValueType::thunk(Grade::ZERO, CompType::returner(int())),
        );
        let result = agree_comp(&ctx, &Comp::force(Value::var("z")), &Dir::Infer);
        assert_eq!(
            Err(TypeError::GradeError {
                lower: Grade::ONE,
                upper: Grade::ZERO
            }),
            result,
            "forcing a 0-graded thunk must fail the grade order"
        );
    }

    /// Checking `thunk_1` against `U_ω` violates `ω ⊑ 1` (rule Thunk⇓).
    #[test]
    fn thunk_grade_too_small()
    {
        let thunk = Value::thunk(Grade::ONE, Comp::ret(Value::var("i")));
        let expected = ValueType::thunk(Grade::OMEGA, CompType::returner(int()));
        let result = agree_value(&base_ctx(), &thunk, &Dir::Check(expected));
        assert_eq!(
            Err(TypeError::GradeError {
                lower: Grade::OMEGA,
                upper: Grade::ONE
            }),
            result,
            "thunk_1 must not check against U_ω"
        );
    }

    /// Rule Dup conservation fails: `dup (thunk_1 t)` cannot split into
    /// `U_1 × U_1` because `1 + 1 ⋢ 1` — the additive accounting `+` of
    /// §"Grades" catches the over-draw (`GradeError { lower: 1 + 1, upper: 1
    /// }`).
    #[test]
    fn dup_violates_conservation()
    {
        let thunk = Value::thunk(Grade::ONE, Comp::ret(Value::int(1)));
        let half = ValueType::thunk(Grade::ONE, CompType::returner(integer()));
        let expected = CompType::returner(ValueType::prod(half.clone(), half));
        let result = agree_comp(&Ctx::new(), &Comp::dup(thunk), &Dir::Check(expected));
        assert_eq!(
            Err(TypeError::GradeError {
                lower: Grade::fin(gandr_core_checker::boundary::GradeBound::from(2)),
                upper: Grade::ONE
            }),
            result,
            "dup must conserve the budget: 1 + 1 ⋢ 1"
        );
    }

    /// Rule Dup is check-only: in inference mode the split grades are
    /// undetermined, so `dup v` is stuck (annotate / supply the expectation),
    /// exactly as an injection or a lazy pair.
    #[test]
    fn dup_in_inference_is_stuck()
    {
        let dup = Comp::dup(Value::thunk(
            Grade::fin(gandr_core_checker::boundary::GradeBound::from(2)),
            Comp::ret(Value::int(1)),
        ));
        let result = agree_comp(&Ctx::new(), &dup, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(dup),
                hint: text::DUP_NEEDS_RETURNER_PRODUCT
            }),
            "dup only checks against F (U_r B × U_s B)"
        );
    }

    /// Rule Drop requires a thunk: `drop v` for a non-thunk `v` is a shape
    /// mismatch (there is no budget to discard).
    #[test]
    fn drop_of_non_thunk()
    {
        let result = agree_comp(&base_ctx(), &Comp::drop(Value::var("i")), &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_THUNK,
                actual: Ty::Value(int())
            }),
            "dropping a non-thunk must be a shape mismatch"
        );
    }

    /// Applying a non-arrow is a shape mismatch.
    #[test]
    fn application_of_non_arrow()
    {
        let app = Comp::app(Comp::ret(Value::var("i")), Value::var("s"));
        let result = agree_comp(&base_ctx(), &app, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_ARROW,
                actual: Ty::Comp(CompType::returner(int())),
            }),
            "applying a non-arrow must be a shape mismatch"
        );
    }

    /// Binding a non-returner is a shape mismatch.
    #[test]
    fn bind_of_non_returner()
    {
        let bound = Comp::bind(
            Comp::lam_ann("x", int(), Comp::ret(Value::var("x"))),
            "y",
            Comp::ret(Value::var("y")),
        );
        let result = agree_comp(&base_ctx(), &bound, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_RETURNER,
                actual: Ty::Comp(CompType::arrow(int(), CompType::returner(int()))),
            }),
            "binding a non-returner must be a shape mismatch"
        );
    }

    /// An integer literal away from the `Integer` atom fails subsumption
    /// (A2.1 literals extension).
    #[test]
    fn integer_literal_against_other_atom_is_a_type_mismatch()
    {
        let result = agree_value(&Ctx::new(), &Value::int(1), &Dir::Check(txt()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(txt()),
                actual: Ty::Value(integer()),
            }),
            "an integer literal must not check against another atom"
        );
    }

    /// A pair checked against a sum fails subsumption with both types
    /// reported.
    #[test]
    fn pair_against_sum_is_a_type_mismatch()
    {
        let pair = Value::pair(Value::var("i"), Value::var("s"));
        let expected = ValueType::sum(int(), txt());
        let result = agree_value(&base_ctx(), &pair, &Dir::Check(expected.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(expected),
                actual: Ty::Value(ValueType::prod(int(), txt())),
            }),
            "a pair against a sum must be a type mismatch"
        );
    }

    /// Grade subsumption is directional: `U_1 B </: U_ω B`.
    #[test]
    fn thunk_grade_subsumption_is_directional()
    {
        let ctx = base_ctx().with("u", ValueType::thunk(Grade::ONE, CompType::returner(int())));
        let expected = ValueType::thunk(Grade::OMEGA, CompType::returner(int()));
        let result = agree_value(&ctx, &Value::var("u"), &Dir::Check(expected.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(expected),
                actual: Ty::Value(ValueType::thunk(Grade::ONE, CompType::returner(int()))),
            }),
            "U_1 (F Int) must not check against U_ω (F Int)"
        );
    }
}

/// Directed subtyping tests, one positive and one negative per `type-system.md`
/// §"Subtyping decomposition" row, driven through the checker/machine pair.
///
/// Every nontrivial value subtyping at Stage 1 flows from the grade order on
/// thunks (`U_r B <: U_s B` iff `s ⊑ r`); the structural rows lift that witness
/// through their (co/contra)variance. The grade row itself is covered by
/// [`positive::thunk_grade_subsumption`] (positive) and
/// [`negative::thunk_grade_subsumption_is_directional`] (negative).
mod subtype_rows
{
    use super::*;
    /// Row `A₁ × A₂ <: A₁′ × A₂′` (covariant): widening a component widens the
    /// product.
    #[test]
    fn product_is_covariant_positive()
    {
        let sub = ValueType::prod(omega_thunk(f_int()), int());
        let sup = ValueType::prod(one_thunk(f_int()), int());
        let ctx = base_ctx().with("p", sub);
        let result = agree_value(&ctx, &Value::var("p"), &Dir::Check(sup.clone()));
        assert_eq!(result, Ok(Ty::Value(sup)), "(U_ω B) × Int <: (U_1 B) × Int");
    }
    /// Row `A₁ × A₂ <: A₁′ × A₂′`: a narrowed component breaks the product
    /// subtyping.
    #[test]
    fn product_is_covariant_negative()
    {
        let sub = ValueType::prod(one_thunk(f_int()), int());
        let sup = ValueType::prod(omega_thunk(f_int()), int());
        let ctx = base_ctx().with("p", sub.clone());
        let result = agree_value(&ctx, &Value::var("p"), &Dir::Check(sup.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(sup),
                actual: Ty::Value(sub),
            }),
            "(U_1 B) × Int </: (U_ω B) × Int"
        );
    }
    /// Row `A₁ + A₂ <: A₁′ + A₂′` (covariant).
    #[test]
    fn sum_is_covariant_positive()
    {
        let sub = ValueType::sum(omega_thunk(f_int()), int());
        let sup = ValueType::sum(one_thunk(f_int()), int());
        let ctx = base_ctx().with("v", sub);
        let result = agree_value(&ctx, &Value::var("v"), &Dir::Check(sup.clone()));
        assert_eq!(result, Ok(Ty::Value(sup)), "(U_ω B) + Int <: (U_1 B) + Int");
    }

    /// Row `A₁ + A₂ <: A₁′ + A₂′`: narrowing a summand breaks it.
    #[test]
    fn sum_is_covariant_negative()
    {
        let sub = ValueType::sum(one_thunk(f_int()), int());
        let sup = ValueType::sum(omega_thunk(f_int()), int());
        let ctx = base_ctx().with("v", sub.clone());
        let result = agree_value(&ctx, &Value::var("v"), &Dir::Check(sup.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(sup),
                actual: Ty::Value(sub),
            }),
            "(U_1 B) + Int </: (U_ω B) + Int"
        );
    }

    /// Record WIDTH subtyping (ADR-45 D1): a record with MORE fields is a
    /// subtype of one with fewer — `{a:U_ω B, b:Int} <: {a:U_ω B}`.
    #[test]
    fn record_is_width_subtype_positive()
    {
        let sub = ValueType::record([
            ("a".to_owned(), omega_thunk(f_int())),
            ("b".to_owned(), int()),
        ]);
        let sup = ValueType::record([("a".to_owned(), omega_thunk(f_int()))]);
        let ctx = base_ctx().with("r", sub);
        let result = agree_value(&ctx, &Value::var("r"), &Dir::Check(sup.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(sup)),
            "a wider record is a subtype of a narrower one (width)"
        );
    }

    /// Record WIDTH subtyping is one-directional: a record MISSING a required
    /// field is not a subtype — `{a:U_ω B} </: {a:U_ω B, b:Int}`.
    #[test]
    fn record_is_width_subtype_negative()
    {
        let sub = ValueType::record([("a".to_owned(), omega_thunk(f_int()))]);
        let sup = ValueType::record([
            ("a".to_owned(), omega_thunk(f_int())),
            ("b".to_owned(), int()),
        ]);
        let ctx = base_ctx().with("r", sub.clone());
        let result = agree_value(&ctx, &Value::var("r"), &Dir::Check(sup.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(sup),
                actual: Ty::Value(sub),
            }),
            "a record missing a required field is not a subtype"
        );
    }

    /// Record DEPTH subtyping (ADR-45 D1): a shared field is covariant —
    /// `{a:U_ω B} <: {a:U_1 B}`.
    #[test]
    fn record_is_depth_subtype_positive()
    {
        let sub = ValueType::record([("a".to_owned(), omega_thunk(f_int()))]);
        let sup = ValueType::record([("a".to_owned(), one_thunk(f_int()))]);
        let ctx = base_ctx().with("r", sub);
        let result = agree_value(&ctx, &Value::var("r"), &Dir::Check(sup.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(sup)),
            "a shared field widens the record (depth, covariant)"
        );
    }

    /// Record DEPTH subtyping: narrowing a shared field breaks it —
    /// `{a:U_1 B} </: {a:U_ω B}`.
    #[test]
    fn record_is_depth_subtype_negative()
    {
        let sub = ValueType::record([("a".to_owned(), one_thunk(f_int()))]);
        let sup = ValueType::record([("a".to_owned(), omega_thunk(f_int()))]);
        let ctx = base_ctx().with("r", sub.clone());
        let result = agree_value(&ctx, &Value::var("r"), &Dir::Check(sup.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(sup),
                actual: Ty::Value(sub),
            }),
            "narrowing a shared field breaks record subtyping"
        );
    }

    /// The empty record `{}` is the **top** of the record order (ADR-45 D1):
    /// every record is a subtype of it (width subtyping, the vacuous case).
    #[test]
    fn record_empty_is_the_record_top()
    {
        let sub = ValueType::record([("a".to_owned(), ValueType::integer())]);
        let top = ValueType::record([]);
        let ctx = base_ctx().with("r", sub);
        let result = agree_value(&ctx, &Value::var("r"), &Dir::Check(top.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(top)),
            "every record is a subtype of the empty record (the record top)"
        );
    }

    /// Record subtyping inherits the gradual NON-transitivity (ADR-45 / A2.2):
    /// a field at `Unknown` relates both ways, so `{a:Integer} <:
    /// {a:Unknown}` and `{a:Unknown} <: {a:String}`, yet `{a:Integer} ⋦
    /// {a:String}` — the per-field consistency does not compose, exactly as
    /// the scalar case.
    #[test]
    fn record_unknown_field_breaks_transitivity()
    {
        let int_rec = ValueType::record([("a".to_owned(), ValueType::integer())]);
        let unknown_rec = ValueType::record([("a".to_owned(), ValueType::Unknown)]);
        let str_rec = ValueType::record([("a".to_owned(), ValueType::string())]);
        assert!(
            bool::from(value_subtype(&int_rec, &unknown_rec)),
            "an Integer-field record is a consistent subtype of an Unknown-field record"
        );
        assert!(
            bool::from(value_subtype(&unknown_rec, &str_rec)),
            "an Unknown-field record is a consistent subtype of a String-field record"
        );
        assert!(
            !bool::from(value_subtype(&int_rec, &str_rec)),
            "an Integer-field record is NOT a subtype of a String-field record \
             (per-field consistency does not compose)"
        );
    }

    /// A record literal INFERS its principal type (ADR-45 D3) — the first
    /// former with a direct inference inhabitant (unlike the check-only
    /// list / injection), the labeled-product analogue of the eager pair.
    #[test]
    fn record_literal_infers_its_field_types()
    {
        let record = Value::record([
            ("a".to_owned(), Value::int(1)),
            ("b".to_owned(), Value::Unit),
        ]);
        let expected = ValueType::record([
            ("a".to_owned(), ValueType::integer()),
            ("b".to_owned(), ValueType::Unit),
        ]);
        let result = agree_value(&base_ctx(), &record, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Value(expected)),
            "a record literal infers the product of its inferred field types"
        );
    }

    /// A record literal CHECKS against a narrower record by width subtyping
    /// (ADR-45 D3): the matching field is checked, the extra field inferred,
    /// and the assembled record subsumes to the narrower expectation.
    #[test]
    fn record_literal_checks_against_a_narrower_record()
    {
        let record = Value::record([
            ("a".to_owned(), Value::int(1)),
            ("b".to_owned(), Value::int(2)),
        ]);
        let expected = ValueType::record([("a".to_owned(), ValueType::integer())]);
        let result = agree_value(&base_ctx(), &record, &Dir::Check(expected.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(expected)),
            "a wider record literal checks against a narrower record (width)"
        );
    }

    /// Row `F A <: F A′` (covariant): `force` infers `F (U_ω B)`, which
    /// subsumes to `F (U_1 B)`.
    #[test]
    fn returner_is_covariant_positive()
    {
        let h_ty = one_thunk(CompType::returner(omega_thunk(f_int())));
        let ctx = base_ctx().with("h", h_ty);
        let sup = CompType::returner(one_thunk(f_int()));
        let result = agree_comp(
            &ctx,
            &Comp::force(Value::var("h")),
            &Dir::Check(sup.clone()),
        );
        assert_eq!(result, Ok(Ty::Comp(sup)), "F (U_ω B) <: F (U_1 B)");
    }

    /// Row `F A <: F A′`: a narrowed payload breaks it.
    #[test]
    fn returner_is_covariant_negative()
    {
        let h_ty = one_thunk(CompType::returner(one_thunk(f_int())));
        let ctx = base_ctx().with("h", h_ty);
        let sup = CompType::returner(omega_thunk(f_int()));
        let result = agree_comp(
            &ctx,
            &Comp::force(Value::var("h")),
            &Dir::Check(sup.clone()),
        );
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Comp(sup),
                actual: Ty::Comp(CompType::returner(one_thunk(f_int()))),
            }),
            "F (U_1 B) </: F (U_ω B)"
        );
    }

    /// Row `A → B <: A′ → B′` (contravariant argument): *widening* the argument
    /// type (an annotated `λ` over `U_1 B` checked at `U_ω B → …`) succeeds,
    /// because `U_ω B <: U_1 B`.
    #[test]
    fn arrow_argument_is_contravariant_positive()
    {
        let lam = Comp::lam_ann("x", one_thunk(f_int()), Comp::ret(Value::var("i")));
        let sup = CompType::arrow(omega_thunk(f_int()), f_int());
        let result = agree_comp(&base_ctx(), &lam, &Dir::Check(sup.clone()));
        assert_eq!(result, Ok(Ty::Comp(sup)), "U_1 B → F Int <: U_ω B → F Int");
    }

    /// Row `A → B <: A′ → B′`: *narrowing* the argument type fails (the other
    /// direction).
    #[test]
    fn arrow_argument_is_contravariant_negative()
    {
        let lam = Comp::lam_ann("x", omega_thunk(f_int()), Comp::ret(Value::var("i")));
        let sup = CompType::arrow(one_thunk(f_int()), f_int());
        let result = agree_comp(&base_ctx(), &lam, &Dir::Check(sup.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Comp(sup),
                actual: Ty::Comp(CompType::arrow(omega_thunk(f_int()), f_int())),
            }),
            "U_ω B → F Int </: U_1 B → F Int"
        );
    }

    /// Row `B₁ & B₂ <: B₁′ & B₂′` (componentwise): widening a conjunct widens
    /// the lazy product.
    #[test]
    fn with_is_componentwise_positive()
    {
        let h_ty = one_thunk(CompType::with(
            CompType::returner(omega_thunk(f_int())),
            f_int(),
        ));
        let ctx = base_ctx().with("h", h_ty);
        let sup = CompType::with(CompType::returner(one_thunk(f_int())), f_int());
        let result = agree_comp(
            &ctx,
            &Comp::force(Value::var("h")),
            &Dir::Check(sup.clone()),
        );
        assert_eq!(
            result,
            Ok(Ty::Comp(sup)),
            "(F (U_ω B)) & (F Int) <: (F (U_1 B)) & (F Int)"
        );
    }

    /// Row `B₁ & B₂ <: B₁′ & B₂′`: narrowing a conjunct breaks it.
    #[test]
    fn with_is_componentwise_negative()
    {
        let inner = CompType::with(CompType::returner(one_thunk(f_int())), f_int());
        let ctx = base_ctx().with("h", one_thunk(inner.clone()));
        let sup = CompType::with(CompType::returner(omega_thunk(f_int())), f_int());
        let result = agree_comp(
            &ctx,
            &Comp::force(Value::var("h")),
            &Dir::Check(sup.clone()),
        );
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Comp(sup),
                actual: Ty::Comp(inner),
            }),
            "(F (U_1 B)) & (F Int) </: (F (U_ω B)) & (F Int)"
        );
    }

    /// Row `F^ε A <: F^ε′ A` (the row leg `ε ⊆ ε′`, ADR-33 D2): a *pure*
    /// returner widens to an effectful one (`⟨⟩ ⊆ ⟨State⟩`). `force` infers
    /// `F^⟨⟩ Int`, which subsumes to `F^⟨State⟩ Int`.
    #[test]
    fn returner_row_is_subset_widened_positive()
    {
        let ctx = base_ctx().with("h", one_thunk(CompType::returner(int())));
        let sup = CompType::returner_eff(int(), state_row());
        let result = agree_comp(
            &ctx,
            &Comp::force(Value::var("h")),
            &Dir::Check(sup.clone()),
        );
        assert_eq!(result, Ok(Ty::Comp(sup)), "F^⟨⟩ Int <: F^⟨State⟩ Int");
    }

    /// Row `F^ε A <: F^ε′ A`: a *larger* row does not subsume a smaller one
    /// (`⟨State⟩ ⊄ ⟨⟩`). `force` of an effectful thunk infers `F^⟨State⟩ Int`,
    /// which fails to check against the pure `F Int`.
    #[test]
    fn returner_row_is_subset_widened_negative()
    {
        let inner = CompType::returner_eff(int(), state_row());
        let ctx = base_ctx().with("h", one_thunk(inner.clone()));
        let sup = CompType::returner(int());
        let result = agree_comp(
            &ctx,
            &Comp::force(Value::var("h")),
            &Dir::Check(sup.clone()),
        );
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Comp(sup),
                actual: Ty::Comp(inner),
            }),
            "F^⟨State⟩ Int </: F^⟨⟩ Int"
        );
    }

    /// Row `Stk(B, C) <: Stk(B′, C′)` — contravariant in `B`, covariant in `C`
    /// (ADR-33 D6). With `B′ <: B` (`F (U_ω …) <: F (U_1 …)`) and `C <: C′`
    /// (`F (U_ω …) <: F (U_1 …)`), the stack widens.
    #[test]
    fn stack_is_contravariant_in_b_covariant_in_c_positive()
    {
        let sub = ValueType::stk(
            CompType::returner(one_thunk(f_int())),
            CompType::returner(omega_thunk(f_int())),
        );
        let sup = ValueType::stk(
            CompType::returner(omega_thunk(f_int())),
            CompType::returner(one_thunk(f_int())),
        );
        let ctx = base_ctx().with("k", sub);
        let result = agree_value(&ctx, &Value::var("k"), &Dir::Check(sup.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(sup)),
            "Stk(F (U_1 B), F (U_ω B)) <: Stk(F (U_ω B), F (U_1 B))"
        );
    }

    /// Row `Stk(B, C) <: Stk(B′, C′)`: getting the `B` variance backwards
    /// breaks it — `B′ = F (U_1 …)` is not a subtype of `B = F (U_ω …)`.
    #[test]
    fn stack_variance_negative()
    {
        let sub = ValueType::stk(
            CompType::returner(omega_thunk(f_int())),
            CompType::returner(omega_thunk(f_int())),
        );
        let sup = ValueType::stk(
            CompType::returner(one_thunk(f_int())),
            CompType::returner(one_thunk(f_int())),
        );
        let ctx = base_ctx().with("k", sub.clone());
        let result = agree_value(&ctx, &Value::var("k"), &Dir::Check(sup.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(sup),
                actual: Ty::Value(sub),
            }),
            "Stk(F (U_ω B), …) </: Stk(F (U_1 B), …) — contravariant B"
        );
    }

    /// `U_ω B`: a forceable thunk that is a *subtype* of `U_1 B` (`1 ⊑ ω`).
    fn omega_thunk(body: CompType) -> ValueType
    {
        ValueType::thunk(Grade::OMEGA, body)
    }

    /// `F Int`, a convenient witness computation type.
    fn f_int() -> CompType
    {
        CompType::returner(int())
    }

    /// `U_1 B`: a *supertype* of `U_ω B`.
    fn one_thunk(body: CompType) -> ValueType
    {
        ValueType::thunk(Grade::ONE, body)
    }

    /// A one-signature row `⟨State⟩`, enough to witness the row leg of `F^ε`
    /// subtyping (ADR-33 D2).
    fn state_row() -> EffectRow
    {
        EffectRow::singleton(EffectSig::new(
            gandr_core_checker::boundary::EffectSignatureName::from("State"),
            alloc::vec![EffectOp::new(
                gandr_core_checker::boundary::OperationName::from("get"),
                ValueType::Unit,
                int(),
            )],
        ))
    }
}

/// Directed A2.2 hole tests: the hole axioms, every matched-type rule, the
/// `Unknown` consistency rows, and the non-transitivity witness — each
/// produced identically by both implementations (ADR-9 lockstep).
///
/// The bidirectional treatment under test (recorded decision, D5 /
/// `incremental-pipeline.md` §"Holes"):
///
/// - `Γ ⊢ ?hole ⇑ Unknown` and `Γ ⊢ ?hole ⇓ A` for every `A` (axioms);
/// - subsumption uses *consistent subtyping* — `Unknown` relates to every type
///   in both directions;
/// - elimination forms whose principal premise infers `Unknown` succeed via the
///   *matched type* (`Unknown ▶→ Unknown → Unknown`, `▶F`, `▶+`, `▶×`, `▶U`,
///   `▶&`), binding binders at `Unknown`; matched-`U` operations emit **no
///   grade constraint**;
/// - check-only introductions checked against `Unknown` succeed through the
///   same matched types;
/// - holes do **not** rescue *directional* stuckness (case/With/unannotated λ
///   in inference mode stay stuck): holes remove the parse/type wall, not the
///   bidirectional discipline.
mod holes
{
    use gandr_core_checker::error::text;
    use gandr_core_checker::syntax::Term;

    use super::*;
    /// Rule Hole⇑: a hole infers `Unknown`, in both sorts; the identifier is
    /// ignored (two distinct identifiers, same derivation).
    #[test]
    fn holes_infer_unknown()
    {
        for id in [0_u32, 7] {
            let value = agree_value(&Ctx::new(), &Value::Hole(id), &Dir::Infer);
            assert_eq!(value, Ok(Ty::Value(unk_v())), "a value hole infers ?");
            let comp = agree_comp(&Ctx::new(), &Comp::Hole(id), &Dir::Infer);
            assert_eq!(comp, Ok(Ty::Comp(unk_c())), "a computation hole infers ?");
        }
    }
    /// Rule Hole⇓: a hole checks against a concrete type, and the expected
    /// type is the result — the recorded *goal*.
    #[test]
    fn holes_check_and_record_the_goal()
    {
        let expected = ValueType::sum(int(), txt());
        let result = agree_value(&Ctx::new(), &Value::Hole(0), &Dir::Check(expected.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(expected)),
            "the goal is the expectation"
        );

        let expected_comp = CompType::arrow(int(), CompType::returner(txt()));
        let result_comp = agree_comp(
            &Ctx::new(),
            &Comp::Hole(0),
            &Dir::Check(expected_comp.clone()),
        );
        assert_eq!(result_comp, Ok(Ty::Comp(expected_comp)));
    }

    /// Matched arrow, elimination side: applying a hole head checks the
    /// argument against `Unknown` and yields `Unknown` — a hole in head
    /// position localizes instead of cascading. The argument here is an
    /// *injection*, which would be stuck in inference mode: only the matched
    /// `Check(Unknown)` direction lets it through.
    #[test]
    fn application_of_hole_head_infers_unknown()
    {
        let app = Comp::app(Comp::Hole(0), Value::inj1(Value::var("i")));
        let result = agree_comp(&base_ctx(), &app, &Dir::Infer);
        assert_eq!(result, Ok(Ty::Comp(unk_c())), "?hole v must infer Unknown");
    }

    /// Matched thunk, elimination side: forcing an `Unknown` value exposes
    /// `Unknown` and emits **no** `1 ⊑ r` constraint.
    #[test]
    fn force_of_hole_infers_unknown()
    {
        let result = agree_comp(&base_ctx(), &Comp::force(Value::Hole(0)), &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(unk_c())),
            "force ?hole must infer Unknown"
        );
    }

    /// Matched returner: binding a hole binds the variable at `Unknown`, and
    /// the variable's later uses flow anywhere by consistency (here: into an
    /// `Integer` argument position of a prelude-style operator).
    #[test]
    fn bind_of_hole_binds_unknown()
    {
        let ctx = base_ctx().with(
            "inc",
            ValueType::thunk(
                Grade::OMEGA,
                CompType::arrow(integer(), CompType::returner(integer())),
            ),
        );
        let comp = Comp::bind(
            Comp::Hole(0),
            "x",
            Comp::app(Comp::force(Value::var("inc")), Value::var("x")),
        );
        let result = agree_comp(&ctx, &comp, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(CompType::returner(integer()))),
            "?hole >>= x. inc x must infer F Integer"
        );
    }

    /// Matched sum: a case over a hole scrutinee binds both arms at
    /// `Unknown`.
    #[test]
    fn case_on_hole_scrutinee_checks()
    {
        let case = Comp::case(
            Value::Hole(0),
            "x",
            Comp::ret(Value::var("x")),
            "y",
            Comp::ret(Value::var("i")),
        );
        let expected = CompType::returner(int());
        let result = agree_comp(&base_ctx(), &case, &Dir::Check(expected.clone()));
        assert_eq!(
            result,
            Ok(Ty::Comp(expected)),
            "case ?hole must check with Unknown-bound arms (x : ? flows into F Int)"
        );
    }

    /// Matched product: splitting a hole binds both components at `Unknown`.
    /// A motive-less split is check-only (rule Split⇓, ADR-82), so the
    /// matched-Unknown binding is exercised in checking position: the body
    /// `ret y` (with `y : Unknown`) checks against the delivered `F Unknown`.
    #[test]
    fn split_of_hole_binds_unknown()
    {
        let split = Comp::split(Value::Hole(0), "x", "y", Comp::ret(Value::var("y")));
        let expected = CompType::returner(unk_v());
        let result = agree_comp(&base_ctx(), &split, &Dir::Check(expected.clone()));
        assert_eq!(
            result,
            Ok(Ty::Comp(expected)),
            "split ?hole checks with the Unknown-bound binder against F Unknown"
        );
    }

    /// Matched with, elimination side: projecting from a hole yields
    /// `Unknown`.
    #[test]
    fn projection_of_hole_infers_unknown()
    {
        let result = agree_comp(&base_ctx(), &Comp::prj1(Comp::Hole(0)), &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(unk_c())),
            "prj1 ?hole must infer Unknown"
        );
    }

    /// Matched sum, introduction side: an injection checks against
    /// `Unknown`, payload against `Unknown`.
    #[test]
    fn injection_checks_against_unknown()
    {
        let inj = Value::inj1(Value::var("i"));
        let result = agree_value(&base_ctx(), &inj, &Dir::Check(unk_v()));
        assert_eq!(
            result,
            Ok(Ty::Value(unk_v())),
            "inj1 i must check against ?"
        );
    }

    /// Matched arrow, introduction side: an unannotated abstraction checks
    /// against `Unknown`, binder bound at `Unknown`.
    #[test]
    fn unannotated_abs_checks_against_unknown()
    {
        let lam = Comp::lam("x", Comp::ret(Value::var("x")));
        let result = agree_comp(&Ctx::new(), &lam, &Dir::Check(unk_c()));
        assert_eq!(
            result,
            Ok(Ty::Comp(unk_c())),
            "λx. ret x must check against ?"
        );
    }

    /// Matched with, introduction side: a lazy pair checks against
    /// `Unknown`, components against `Unknown`. With is the one rule that
    /// returns the *rebuilt* type — here `Unknown & Unknown`, consistent
    /// with (not equal to) the expectation; this test pins that recorded
    /// convention.
    #[test]
    fn with_checks_against_unknown()
    {
        let lazy = Comp::with(Comp::ret(Value::var("i")), Comp::ret(Value::var("s")));
        let result = agree_comp(&base_ctx(), &lazy, &Dir::Check(unk_c()));
        assert_eq!(
            result,
            Ok(Ty::Comp(CompType::with(unk_c(), unk_c()))),
            "⟨…⟩ against ? rebuilds ? & ?"
        );
    }

    /// Matched thunk, introduction side: a thunk literal checks against
    /// `Unknown` with **no grade constraint** — even a `0`-graded thunk,
    /// which could never be forced, checks (no constraint is emitted; §"Holes":
    /// "no constraint emitted", degenerated honestly).
    #[test]
    fn thunk_checks_against_unknown_without_grade_constraint()
    {
        let thunk = Value::thunk(Grade::ZERO, Comp::ret(Value::var("i")));
        let result = agree_value(&base_ctx(), &thunk, &Dir::Check(unk_v()));
        assert_eq!(
            result,
            Ok(Ty::Value(unk_v())),
            "thunk_0 must check against ? — matched U emits no grade constraint"
        );
    }

    /// Matched product/returner via the shared `Dir` helpers: a pair checked
    /// against `Unknown` distributes `Unknown` componentwise (its injection
    /// component would be stuck under inference), and `ret` checked against
    /// `Unknown` checks its payload against `Unknown`.
    #[test]
    fn pair_and_ret_check_against_unknown()
    {
        let pair = Value::pair(Value::inj1(Value::var("i")), Value::var("s"));
        let result = agree_value(&base_ctx(), &pair, &Dir::Check(unk_v()));
        assert_eq!(
            result,
            Ok(Ty::Value(unk_v())),
            "(inj1 i, s) must check against ?"
        );

        let ret = Comp::ret(Value::inj2(Value::var("s")));
        let result_ret = agree_comp(&base_ctx(), &ret, &Dir::Check(unk_c()));
        assert_eq!(
            result_ret,
            Ok(Ty::Comp(unk_c())),
            "ret (inj2 s) must check against ?"
        );
    }

    /// An annotation whose ascription is `Unknown` (the pipeline's image of
    /// an elided type) checks its subject against `Unknown` and infers
    /// `Unknown`.
    #[test]
    fn annotation_at_unknown_infers_unknown()
    {
        let annot = Value::annot(Value::inj1(Value::var("i")), unk_v());
        let result = agree_value(&base_ctx(), &annot, &Dir::Infer);
        assert_eq!(result, Ok(Ty::Value(unk_v())), "(inj1 i : ?) must infer ?");
    }

    /// Holes do **not** rescue directional stuckness: case in inference mode
    /// stays stuck even over a hole scrutinee, and an unannotated λ in
    /// inference mode stays stuck even with a hole body — the bidirectional
    /// discipline is untouched (recorded decision).
    #[test]
    fn holes_do_not_rescue_directional_stuckness()
    {
        let case = Comp::case(
            Value::Hole(0),
            "x",
            Comp::ret(Value::var("x")),
            "y",
            Comp::ret(Value::var("y")),
        );
        let result = agree_comp(&base_ctx(), &case, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(case),
                hint: text::CASE_NEEDS_CHECK,
            }),
            "case stays check-only over a hole scrutinee"
        );

        let lam = Comp::lam("x", Comp::Hole(0));
        let result_lam = agree_comp(&Ctx::new(), &lam, &Dir::Infer);
        assert_eq!(
            result_lam,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(lam),
                hint: text::ANNOTATE_BINDER,
            }),
            "an unannotated λ stays uninferable even with a hole body"
        );
    }

    /// `Unknown` consistency rows: both directions, both sorts — plus the
    /// recorded **non-transitivity witness** (`Int ≲ ? ≲ Str` yet
    /// `Int ⋦ Str`): consistent subtyping is reflexive but deliberately not
    /// transitive (`gandr_core_checker::subtype` module doc).
    #[test]
    fn consistency_is_not_transitive()
    {
        assert!(bool::from(value_subtype(&unk_v(), &int())), "? ≲ Int");
        assert!(bool::from(value_subtype(&int(), &unk_v())), "Int ≲ ?");
        assert!(
            bool::from(comp_subtype(&unk_c(), &CompType::returner(int()))),
            "? ≲ F Int"
        );
        assert!(
            bool::from(comp_subtype(&CompType::returner(int()), &unk_c())),
            "F Int ≲ ?"
        );

        assert!(
            bool::from(value_subtype(&int(), &unk_v()))
                && bool::from(value_subtype(&unk_v(), &txt()))
        );
        assert!(
            !bool::from(value_subtype(&int(), &txt())),
            "Int ≲ ? ≲ Str must NOT compose: consistency is not transitive"
        );
    }

    /// The gradual cast chain `(1 : ?) : Str` type-checks statically — each
    /// subsumption step is consistent even though the composite is not.
    /// Pinned as documentation of the consistent-subtyping reading (the
    /// dynamic cast error is a later stage's concern).
    #[test]
    fn cast_chains_through_unknown_are_static_successes()
    {
        let chained = Value::annot(Value::annot(Value::int(1), unk_v()), txt());
        let result = agree_value(&Ctx::new(), &chained, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Value(txt())),
            "(1 : ?) : Str must type-check"
        );
    }

    /// `Unknown` of value sort.
    fn unk_v() -> ValueType
    {
        ValueType::Unknown
    }

    /// `Unknown` of computation sort.
    fn unk_c() -> CompType
    {
        CompType::Unknown
    }
}

/// Directed A3.2 `+effects` tests: `perform` (rule Op), `handle` (rule Handle),
/// and the bottom-up row arithmetic at `bind` — the introduction/elimination
/// rules, the bottom-up row union and the residual soundness leg, and every new
/// failure mode, each produced identically by both implementations (ADR-9
/// lockstep; `effects-control-shell.md` §1).
mod effects
{
    use gandr_core_checker::error::text;
    use gandr_core_checker::syntax::Term;

    use super::*;
    /// Rule Op⇑: `perform State.get ()` infers the singleton-row returner
    /// `F^⟨State⟩ Integer` (the op's reply at its signature's singleton row).
    #[test]
    fn perform_infers_singleton_row()
    {
        let perf = Comp::perform(state_sig(), "get", Value::Unit);
        let expected = CompType::returner_eff(integer(), row(state_sig()));
        let result = agree_comp(&Ctx::new(), &perf, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(expected)),
            "perform State.get () must infer F^⟨State⟩ Integer"
        );
    }
    /// Rule Op⇑: the payload is checked against the op's payload type — `put`
    /// takes `Integer`, so performing it on `()` is a type mismatch.
    #[test]
    fn perform_payload_is_checked()
    {
        let perf = Comp::perform(state_sig(), "put", Value::Unit);
        let result = agree_comp(&Ctx::new(), &perf, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(integer()),
                actual: Ty::Value(ValueType::Unit),
            }),
            "perform State.put () must mismatch (put takes Integer)"
        );
    }

    /// Rule Op's side condition `op ∈ E`: an operation absent from the carried
    /// signature is stuck (no (Op) instance applies).
    #[test]
    fn perform_unknown_op_is_stuck()
    {
        let perf = Comp::perform(state_sig(), "absent", Value::Unit);
        let result = agree_comp(&Ctx::new(), &perf, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(perf),
                hint: text::PERFORM_UNKNOWN_OP,
            }),
            "perform of an undeclared op must be stuck"
        );
    }

    /// Bottom-up row union at `bind`: `perform State.get () >>= x. perform
    /// IO.print x` accumulates `⟨State⟩ ∪ ⟨IO⟩` on the result (`effects-
    /// control-shell.md` §1.2).
    #[test]
    fn bind_unions_effect_rows()
    {
        let comp = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::perform(io_sig(), "print", Value::var("x")),
        );
        let unioned = row(state_sig()).union(&row(io_sig()));
        let expected = CompType::returner_eff(ValueType::Unit, unioned);
        let result = agree_comp(&Ctx::new(), &comp, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(expected)),
            "bind must union ⟨State⟩ ∪ ⟨IO⟩"
        );
    }

    /// A *pure* bound computation still sequences into a non-returner
    /// continuation unchanged (the pre-`+effects` behaviour, the empty-row
    /// path of `combine_bind_row`): `ret () >>= x. (λy:Int. ret y)` infers
    /// `Int → F Int`.
    #[test]
    fn pure_bind_into_lambda_is_unchanged()
    {
        let comp = Comp::bind(
            Comp::ret(Value::Unit),
            "x",
            Comp::lam_ann("y", int(), Comp::ret(Value::var("y"))),
        );
        let expected = CompType::arrow(int(), CompType::returner(int()));
        let result = agree_comp(&Ctx::new(), &comp, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(expected)),
            "a pure bind into a λ is unchanged (empty bound row)"
        );
    }

    /// An *effectful* bound computation cannot sequence into a non-returner
    /// continuation (v0: effects live only on `F`): `perform State.get () >>=
    /// x. (λy:Int. ret y)` is a `SHAPE_RETURNER` mismatch on the
    /// continuation.
    #[test]
    fn effectful_bind_into_lambda_is_shape_mismatch()
    {
        let comp = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::lam_ann("y", int(), Comp::ret(Value::var("y"))),
        );
        let result = agree_comp(&Ctx::new(), &comp, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_RETURNER,
                actual: Ty::Comp(CompType::arrow(int(), CompType::returner(int()))),
            }),
            "an effectful bind into a λ must be a SHAPE_RETURNER mismatch"
        );
    }

    /// Rule Handle⇓: a `State` handler discharges the signature from `perform
    /// State.get ()` — the residual `⟨State⟩ ∖ State = ⟨⟩` fits the pure answer
    /// `F Integer`, so the handle checks against it.
    #[test]
    fn handle_discharges_a_signature()
    {
        let handler = Comp::handle(
            state_sig(),
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
            vec![
                OpClause::new("get", "p", "k", Comp::ret(Value::int(0))),
                OpClause::new("put", "p", "k", Comp::ret(Value::var("p"))),
            ],
        );
        let answer = pure(integer());
        let result = agree_comp(&Ctx::new(), &handler, &Dir::Check(answer.clone()));
        assert_eq!(
            result,
            Ok(Ty::Comp(answer)),
            "handle must discharge State (residual ⟨⟩ ⊆ ⟨⟩) and check against F Integer"
        );
    }

    /// Rule Handle⇓, soundness leg: a handler that discharges only `State` from
    /// a computation performing `State` *and* `IO` leaks `⟨IO⟩` — the residual
    /// `⟨State, IO⟩ ∖ State = ⟨IO⟩` does not fit the pure answer `F Integer`,
    /// so the inlined Sub rule reports a type mismatch on the row.
    #[test]
    fn handle_leaking_an_unhandled_effect_is_a_type_mismatch()
    {
        let scrutinee = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::perform(io_sig(), "print", Value::var("x")),
        );
        let handler = Comp::handle(state_sig(), scrutinee, "x", Comp::ret(Value::int(0)), vec![
            OpClause::new("get", "p", "k", Comp::ret(Value::int(0))),
            OpClause::new("put", "p", "k", Comp::ret(Value::int(0))),
        ]);
        let answer = pure(integer());
        let leaked = CompType::returner_eff(integer(), row(io_sig()));
        let result = agree_comp(&Ctx::new(), &handler, &Dir::Check(answer.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Comp(answer),
                actual: Ty::Comp(leaked),
            }),
            "a handler leaking ⟨IO⟩ must fail the soundness leg ⟨IO⟩ ⊆ ⟨⟩"
        );
    }

    /// Rule Handle is check-only: in inference mode it is stuck (like Case).
    #[test]
    fn handle_in_inference_is_stuck()
    {
        let handler = Comp::handle(
            empty_sig(),
            Comp::ret(Value::int(0)),
            "x",
            Comp::ret(Value::var("x")),
            Vec::new(),
        );
        let result = agree_comp(&Ctx::new(), &handler, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(handler),
                hint: text::HANDLE_NEEDS_CHECK,
            }),
            "handle in inference mode must be stuck"
        );
    }

    /// Rule Handle needs a returner answer: checked against a non-returner
    /// (here an arrow) it is stuck (the clauses and `k`'s `Stk(F^ε B, F^ε C)`
    /// need an `F^ε C` answer).
    #[test]
    fn handle_against_a_non_returner_is_stuck()
    {
        let handler = Comp::handle(
            empty_sig(),
            Comp::ret(Value::int(0)),
            "x",
            Comp::ret(Value::var("x")),
            Vec::new(),
        );
        let non_returner = CompType::arrow(int(), CompType::returner(int()));
        let result = agree_comp(&Ctx::new(), &handler, &Dir::Check(non_returner));
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(handler),
                hint: text::HANDLE_NEEDS_RETURNER,
            }),
            "handle against an arrow answer must be stuck"
        );
    }

    /// Deep-handler coverage is exact: a handler missing the `put` clause does
    /// not cover `State`, so no (Handle) instance applies.
    #[test]
    fn handler_coverage_must_be_exact()
    {
        let handler = Comp::handle(
            state_sig(),
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
            vec![OpClause::new("get", "p", "k", Comp::ret(Value::int(0)))],
        );
        let result = agree_comp(&Ctx::new(), &handler, &Dir::Check(pure(integer())));
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(handler),
                hint: text::HANDLER_CLAUSES_MISMATCH,
            }),
            "a handler missing a clause must be stuck"
        );
    }

    /// Rule Handle: the handled computation must be a returner — handling a
    /// (non-returner) abstraction is a `SHAPE_RETURNER` mismatch.
    #[test]
    fn handle_of_a_non_returner_scrutinee_is_a_shape_mismatch()
    {
        let handler = Comp::handle(
            empty_sig(),
            Comp::lam_ann("z", int(), Comp::ret(Value::var("z"))),
            "x",
            Comp::ret(Value::var("x")),
            Vec::new(),
        );
        let result = agree_comp(&Ctx::new(), &handler, &Dir::Check(pure(int())));
        assert_eq!(
            result,
            Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_RETURNER,
                actual: Ty::Comp(CompType::arrow(int(), CompType::returner(int()))),
            }),
            "handling a non-returner must be a SHAPE_RETURNER mismatch"
        );
    }

    /// An empty-signature handler is the degenerate fold: no operation clauses,
    /// so the handle finishes straight after the return clause. The handled
    /// `ret 0` performs nothing, so the residual `⟨⟩` fits the pure answer.
    #[test]
    fn empty_signature_handler_finishes_after_the_return_clause()
    {
        let handler = Comp::handle(
            empty_sig(),
            Comp::ret(Value::int(0)),
            "x",
            Comp::ret(Value::var("x")),
            Vec::new(),
        );
        let answer = pure(integer());
        let result = agree_comp(&Ctx::new(), &handler, &Dir::Check(answer.clone()));
        assert_eq!(
            result,
            Ok(Ty::Comp(answer)),
            "an empty-signature handler checks against the answer"
        );
    }

    /// Matched-hole answer (A2.2 + A3.2): a handler checked against `?` uses
    /// the matched returner — clauses check against `Unknown`, the residual
    /// is absorbed — and the handle infers `Unknown`.
    #[test]
    fn handle_against_unknown_answer()
    {
        let handler = Comp::handle(
            state_sig(),
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
            vec![
                OpClause::new("get", "p", "k", Comp::Hole(0)),
                OpClause::new("put", "p", "k", Comp::Hole(1)),
            ],
        );
        let result = agree_comp(&Ctx::new(), &handler, &Dir::Check(CompType::Unknown));
        assert_eq!(
            Ok(Ty::Comp(CompType::Unknown)),
            result,
            "a handler against ? must check via the matched returner"
        );
    }

    /// Checking-mode `bind` subsumes its **accumulated** row, not just the
    /// continuation's: `(perform State.get ()) >>= x. ret 0` checked against
    /// the *pure* `F Integer` accumulates `⟨State⟩` and so must be rejected
    /// — the bound computation's effect cannot escape into a smaller
    /// checked answer (the final `finish_comp` in rule Bind). This is the
    /// soundness leg a shared-implementation lock-step suite cannot see, so
    /// it is pinned directly.
    #[test]
    fn check_mode_bind_subsumes_the_accumulated_row()
    {
        let comp = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::ret(Value::int(0)),
        );
        let pure_answer = pure(integer());
        let accumulated = CompType::returner_eff(integer(), row(state_sig()));
        let result = agree_comp(&Ctx::new(), &comp, &Dir::Check(pure_answer.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Comp(pure_answer),
                actual: Ty::Comp(accumulated),
            }),
            "a checking-mode bind must subsume its accumulated row (⟨State⟩ ⊄ ⟨⟩)"
        );
    }

    /// The complement: a checking-mode bind whose accumulated row *fits* the
    /// answer checks — `(perform State.get ()) >>= x. ret 0` against
    /// `F^⟨State⟩ Integer` succeeds (`⟨State⟩ ⊆ ⟨State⟩`).
    #[test]
    fn check_mode_bind_accepts_a_contained_row()
    {
        let comp = Comp::bind(
            Comp::perform(state_sig(), "get", Value::Unit),
            "x",
            Comp::ret(Value::int(0)),
        );
        let answer = CompType::returner_eff(integer(), row(state_sig()));
        let result = agree_comp(&Ctx::new(), &comp, &Dir::Check(answer.clone()));
        assert_eq!(
            result,
            Ok(Ty::Comp(answer)),
            "a checking-mode bind whose row fits the answer must check"
        );
    }

    /// A handler clause cannot launder an effect through a `bind`: the return
    /// clause `(perform IO.print 0) >>= z. ret 0`, checked against the pure
    /// answer `F Integer`, accumulates `⟨IO⟩` and is rejected — the same effect
    /// performed directly in a clause body is rejected, and routing it through
    /// a bind no longer evades the answer's row.
    #[test]
    fn handle_clause_cannot_launder_an_effect_through_a_bind()
    {
        let handler = Comp::handle(
            empty_sig(),
            Comp::ret(Value::int(0)),
            "x",
            Comp::bind(
                Comp::perform(io_sig(), "print", Value::int(0)),
                "z",
                Comp::ret(Value::int(0)),
            ),
            Vec::new(),
        );
        let answer = pure(integer());
        let leaked = CompType::returner_eff(integer(), row(io_sig()));
        let result = agree_comp(&Ctx::new(), &handler, &Dir::Check(answer.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Comp(answer),
                actual: Ty::Comp(leaked),
            }),
            "a handler return clause must not launder ⟨IO⟩ through a bind"
        );
    }

    /// A pure-returner answer `F^⟨⟩ A`.
    fn pure(of: ValueType) -> CompType
    {
        CompType::returner(of)
    }

    /// The singleton row `⟨E⟩`.
    fn row(sig: EffectSig) -> EffectRow
    {
        EffectRow::singleton(sig)
    }
}

/// Directed A3.3 `+control` tests: `stk K` reify (rule Reify), `resume` (rule
/// Resume), and `reset` / `shift` (the delimited-control rules) — the
/// introduction/elimination rules in BOTH `Dir::Infer` and `Dir::Check`, the
/// stack-judgment walk and its bottom-up row arithmetic *with the soundness
/// leg* (a reified stack cannot launder an effect row past its delivered answer
/// — the check-only analogue of the A3.2 `bind` row-escape, which the
/// infer/check coherence oracle cannot see because `stk K` never infers), the
/// ambient-answer discipline (a `shift` needs an enclosing `reset`), the
/// matched-hole rules, and every new failure mode — each produced identically
/// by both implementations (ADR-9 lockstep; `effects-control-shell.md` §2).
mod control
{
    use gandr_core_checker::error::text;
    use gandr_core_checker::syntax::Term;

    use super::*;
    /// Rule Reify⇓: the empty stack is the identity `ε : B ⇒ B`, so `stk ε`
    /// checks against `Stk(B, B)`.
    #[test]
    fn reify_empty_stack_is_identity()
    {
        let stk = Value::stk(Stack::empty());
        let value_type = ValueType::stk(pure(integer()), pure(integer()));
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(value_type.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(value_type)),
            "stk ε must check against Stk(F Int, F Int)"
        );
    }
    /// Rule Reify⇓, covariant `C` (ADR-33 D6): the empty stack's delivered type
    /// `B` may subsume *up* to the expected answer `C` — `stk ε` checks against
    /// `Stk(F^⟨⟩ Int, F^⟨State⟩ Int)` since `⟨⟩ ⊆ ⟨State⟩`.
    #[test]
    fn reify_delivered_answer_subsumes_up()
    {
        let stk = Value::stk(Stack::empty());
        let value_type = ValueType::stk(pure(integer()), eff(integer(), state_sig()));
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(value_type.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(value_type)),
            "stk ε : Stk(F Int, F Int) must check against Stk(F Int, F^⟨State⟩ Int)"
        );
    }

    // ── Rule Reify (`stk K`) — check-only against `Stk(B, C)` ─────────────────

    /// Rule Reify⇓, soundness leg: the delivered answer may not subsume *down*
    /// — `stk ε` synthesizing `Stk(F^⟨State⟩ Int, F^⟨State⟩ Int)` does not
    /// check against `Stk(F^⟨State⟩ Int, F^⟨⟩ Int)` (`⟨State⟩ ⊄ ⟨⟩` on the
    /// covariant `C`).
    #[test]
    fn reify_delivered_answer_cannot_drop_a_row()
    {
        let stk = Value::stk(Stack::empty());
        let consumed = eff(integer(), state_sig());
        let synthesized = ValueType::stk(consumed.clone(), eff(integer(), state_sig()));
        let dropped = ValueType::stk(consumed, pure(integer()));
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(dropped.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(dropped),
                actual: Ty::Value(synthesized),
            }),
            "stk ε must not drop ⟨State⟩ from its delivered answer"
        );
    }

    /// Rule Reify⇓, argument frame `v :: K`: consumes a function `A → B`,
    /// checks `v ⇓ A`, and delivers `B` — `stk (0 :: ε)` checks against
    /// `Stk(Integer → F Integer, F Integer)`.
    #[test]
    fn reify_argument_frame()
    {
        let stk = Value::stk(Stack::arg(Value::int(0), Stack::empty()));
        let value_type =
            ValueType::stk(CompType::arrow(integer(), pure(integer())), pure(integer()));
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(value_type.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(value_type)),
            "stk (0 :: ε) must check against Stk(Integer → F Integer, F Integer)"
        );
    }

    /// Rule Reify⇓, projection frame `prjᵢ :: K`: consumes a lazy product
    /// `B₁ & B₂` and delivers `Bᵢ` — `stk (prj1 :: ε)` checks against
    /// `Stk((F Int) & (F Unit), F Int)`.
    #[test]
    fn reify_projection_frame()
    {
        let stk = Value::stk(Stack::prj1(Stack::empty()));
        let value_type = ValueType::stk(
            CompType::with(pure(int()), pure(ValueType::Unit)),
            pure(int()),
        );
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(value_type.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(value_type)),
            "stk (prj1 :: ε) must project the first conjunct"
        );
    }

    /// Rule Reify⇓, bind frame `(x. u) :: K`: consumes `F^ε A`, binds `x : A`,
    /// infers `u`, and folds the consumed row in (as `bind`). With a *pure*
    /// consumed returner and an effectful continuation `u = perform IO.print
    /// x`, the delivered answer carries `⟨IO⟩` — `stk ((x. perform IO.print
    /// x) :: ε)` checks against `Stk(F Integer, F^⟨IO⟩ Unit)`.
    #[test]
    fn reify_bind_frame_carries_the_continuation_row()
    {
        let stk = Value::stk(Stack::bind(
            "x",
            Comp::perform(io_sig(), "print", Value::var("x")),
            Stack::empty(),
        ));
        let value_type = ValueType::stk(pure(integer()), eff(ValueType::Unit, io_sig()));
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(value_type.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(value_type)),
            "a bind frame's effectful continuation must surface ⟨IO⟩ in the delivered answer"
        );
    }

    /// Rule Reify⇓, bind frame, consumed-row fold: the consumed returner's row
    /// folds into the delivered answer exactly as at `bind` (via
    /// `combine_bind_row`). Consuming `F^⟨State⟩ Integer` and continuing with
    /// `perform IO.print x` delivers `F^⟨State, IO⟩ Unit`.
    #[test]
    fn reify_bind_frame_folds_the_consumed_row()
    {
        let stk = Value::stk(Stack::bind(
            "x",
            Comp::perform(io_sig(), "print", Value::var("x")),
            Stack::empty(),
        ));
        let unioned = EffectRow::singleton(state_sig()).union(&EffectRow::singleton(io_sig()));
        let value_type = ValueType::stk(
            eff(integer(), state_sig()),
            CompType::returner_eff(ValueType::Unit, unioned),
        );
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(value_type.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(value_type)),
            "the consumed row ⟨State⟩ must fold into the delivered answer (⟨State⟩ ∪ ⟨IO⟩)"
        );
    }

    /// Rule Reify⇓, the soundness leg `stk K` *cannot* see (it never infers): a
    /// bind frame whose continuation performs `IO` must not check against a
    /// delivered answer that drops `⟨IO⟩`. `stk ((x. perform IO.print x) :: ε)`
    /// synthesizes `Stk(F Integer, F^⟨IO⟩ Unit)`, so checking against
    /// `Stk(F Integer, F^⟨⟩ Unit)` is a row mismatch on the covariant `C`.
    #[test]
    fn reify_bind_frame_cannot_launder_its_row()
    {
        let stk = Value::stk(Stack::bind(
            "x",
            Comp::perform(io_sig(), "print", Value::var("x")),
            Stack::empty(),
        ));
        let synthesized = ValueType::stk(pure(integer()), eff(ValueType::Unit, io_sig()));
        let laundered = ValueType::stk(pure(integer()), pure(ValueType::Unit));
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(laundered.clone()));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(laundered),
                actual: Ty::Value(synthesized),
            }),
            "a reified stack must not launder ⟨IO⟩ out of its delivered answer"
        );
    }

    /// Rule Reify is check-only: in inference mode the consumed type `B` is
    /// undetermined (the empty stack is `B ⇒ B` for any `B`), so `stk K` is
    /// stuck — exactly as an injection.
    #[test]
    fn reify_in_inference_is_stuck()
    {
        let stk = Value::stk(Stack::empty());
        let result = agree_value(&Ctx::new(), &stk, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Value(stk),
                hint: text::STK_NEEDS_STK_TYPE,
            }),
            "stk K in inference mode must be stuck"
        );
    }

    /// Rule Reify needs a stack type: checked against a non-`Stk` value type
    /// (here an atom) it is stuck.
    #[test]
    fn reify_against_non_stack_type_is_stuck()
    {
        let stk = Value::stk(Stack::empty());
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(integer()));
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Value(stk),
                hint: text::STK_NEEDS_STK_TYPE,
            }),
            "stk K against a non-stack type must be stuck"
        );
    }

    /// Matched stack (A2.2 + A3.3): `stk K` checked against `?` uses the
    /// matched stack `Unknown ▶Stk Stk(Unknown, Unknown)` — the walk runs
    /// from `Unknown` and the result subsumes to exactly `Unknown`.
    #[test]
    fn reify_against_unknown_is_the_matched_stack()
    {
        let stk = Value::stk(Stack::bind("x", Comp::ret(Value::var("x")), Stack::empty()));
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(ValueType::Unknown));
        assert_eq!(
            Ok(Ty::Value(ValueType::Unknown)),
            result,
            "stk K against ? must check via the matched stack"
        );
    }

    /// Rule Resume⇑: infer the reified stack `v ⇑ Stk(B, C)` (here an annotated
    /// empty stack), check the fed computation `t ⇓ B`, and deliver `C` —
    /// `resume (stk ε : Stk(F Int, F Int)) (ret 0)` infers `F Int`.
    #[test]
    fn resume_infers_the_delivered_answer()
    {
        let stk_ty = ValueType::stk(pure(integer()), pure(integer()));
        let resume = Comp::resume(
            Value::annot(Value::stk(Stack::empty()), stk_ty),
            Comp::ret(Value::int(0)),
        );
        let result = agree_comp(&Ctx::new(), &resume, &Dir::Infer);
        assert_eq!(
            result,
            Ok(Ty::Comp(pure(integer()))),
            "resume of a Stk(F Int, F Int) must deliver F Int"
        );
    }

    /// Rule Resume⇑: the fed computation is checked against the consumed type
    /// `B` — feeding `ret ()` to a stack consuming `F Integer` is a value
    /// mismatch (`Unit </: Integer`).
    #[test]
    fn resume_checks_the_fed_computation()
    {
        let stk_ty = ValueType::stk(pure(integer()), pure(integer()));
        let resume = Comp::resume(
            Value::annot(Value::stk(Stack::empty()), stk_ty),
            Comp::ret(Value::Unit),
        );
        let result = agree_comp(&Ctx::new(), &resume, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(integer()),
                actual: Ty::Value(ValueType::Unit),
            }),
            "feeding ret () to a stack consuming F Integer must mismatch"
        );
    }

    // ── Rule Resume (`resume v t`) — an inference form ───────────────────────

    /// Rule Resume: the resumed value must be a reified stack — resuming a
    /// non-stack (here a literal) is a `SHAPE_STK` mismatch.
    #[test]
    fn resume_of_a_non_stack_is_a_shape_mismatch()
    {
        let resume = Comp::resume(Value::int(0), Comp::ret(Value::int(0)));
        let result = agree_comp(&Ctx::new(), &resume, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::ShapeMismatch {
                expected: text::SHAPE_STK,
                actual: Ty::Value(integer()),
            }),
            "resuming a non-stack must be a SHAPE_STK mismatch"
        );
    }

    /// Rule Resume⇓ (the inference form reached through subsumption): a resume
    /// delivering an effectful `F^⟨State⟩ Integer` cannot drop its row into a
    /// smaller checked answer — checking against the pure `F Integer` is a row
    /// mismatch (the resume row-soundness leg).
    #[test]
    fn resume_delivering_a_row_cannot_drop_it()
    {
        let stk_ty = ValueType::stk(eff(integer(), state_sig()), eff(integer(), state_sig()));
        let resume = Comp::resume(
            Value::annot(Value::stk(Stack::empty()), stk_ty),
            Comp::perform(state_sig(), "get", Value::Unit),
        );
        let delivered = eff(integer(), state_sig());
        let result = agree_comp(&Ctx::new(), &resume, &Dir::Check(pure(integer())));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Comp(pure(integer())),
                actual: Ty::Comp(delivered),
            }),
            "a resume delivering ⟨State⟩ must not check against the pure F Integer"
        );
    }

    /// Matched stack (A2.2 + A3.3): resuming a hole infers `Unknown` — the
    /// matched stack feeds the computation against `Unknown` and delivers
    /// `Unknown`, so a hole in stack position localizes rather than cascading.
    #[test]
    fn resume_of_a_hole_infers_unknown()
    {
        let resume = Comp::resume(Value::Hole(0), Comp::ret(Value::int(0)));
        let result = agree_comp(&Ctx::new(), &resume, &Dir::Infer);
        assert_eq!(
            Ok(Ty::Comp(CompType::Unknown)),
            result,
            "resume of a hole stack must infer Unknown"
        );
    }

    /// Rule Reset⇓: `reset` is check-only and transparent on the type — `reset
    /// (ret 0)` checks against `F Integer`, returning it.
    #[test]
    fn reset_is_transparent_on_the_type()
    {
        let reset = Comp::reset(Comp::ret(Value::int(0)));
        let answer = pure(integer());
        let result = agree_comp(&Ctx::new(), &reset, &Dir::Check(answer.clone()));
        assert_eq!(
            result,
            Ok(Ty::Comp(answer)),
            "reset (ret 0) must check against F Integer"
        );
    }

    /// Rule Reset is check-only: in inference mode the answer is undetermined,
    /// so it is stuck (like a handler).
    #[test]
    fn reset_in_inference_is_stuck()
    {
        let reset = Comp::reset(Comp::ret(Value::int(0)));
        let result = agree_comp(&Ctx::new(), &reset, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(reset),
                hint: text::RESET_NEEDS_CHECK,
            }),
            "reset in inference mode must be stuck"
        );
    }

    // ── Rules Reset / Shift — delimited control ──────────────────────────────

    /// Rules Reset/Shift/Resume round-trip: `reset (shift k. resume k (ret 0))`
    /// checks against `F Integer`. The `reset` fixes the answer `C = F
    /// Integer`; the `shift` binds `k : Stk(F Integer, F Integer)`
    /// (captured `B` = answer `C` here) and checks its body against `C`;
    /// `resume k (ret 0)` consumes the continuation and delivers `C`.
    #[test]
    fn reset_shift_resume_round_trip()
    {
        let body = Comp::resume(Value::var("k"), Comp::ret(Value::int(0)));
        let comp = Comp::reset(Comp::shift("k", body));
        let answer = pure(integer());
        let result = agree_comp(&Ctx::new(), &comp, &Dir::Check(answer.clone()));
        assert_eq!(
            result,
            Ok(Ty::Comp(answer)),
            "reset (shift k. resume k (ret 0)) must check against F Integer"
        );
    }

    /// Rule Shift⇓ binds `k : Stk(B, C)`: inside `reset` at answer `F Integer`,
    /// `shift k. resume k (ret ())` is rejected — feeding `ret ()` to
    /// `k : Stk(F Integer, F Integer)` mismatches its consumed `F Integer`
    /// (`Unit </: Integer`), proving `k`'s captured type is `B = F Integer`.
    #[test]
    fn shift_binds_the_continuation_at_the_captured_type()
    {
        let body = Comp::resume(Value::var("k"), Comp::ret(Value::Unit));
        let comp = Comp::reset(Comp::shift("k", body));
        let result = agree_comp(&Ctx::new(), &comp, &Dir::Check(pure(integer())));
        assert_eq!(
            result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(integer()),
                actual: Ty::Value(ValueType::Unit),
            }),
            "k must be bound at Stk(F Integer, F Integer): feeding ret () mismatches"
        );
    }

    /// Rule Shift needs an enclosing `reset`: a bare `shift k. ret 0` checked
    /// against `F Integer` (no `reset` to fix the ambient answer) is stuck.
    #[test]
    fn shift_outside_reset_is_stuck()
    {
        let shift = Comp::shift("k", Comp::ret(Value::int(0)));
        let result = agree_comp(&Ctx::new(), &shift, &Dir::Check(pure(integer())));
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(shift),
                hint: text::SHIFT_NEEDS_RESET,
            }),
            "shift with no enclosing reset must be stuck"
        );
    }

    /// Rule Shift is check-only: in inference mode the captured type `B` is
    /// undetermined, so it is stuck — and the direction is checked *before* the
    /// ambient answer, so a `shift` that is both inference-mode and reset-less
    /// reports the direction failure.
    #[test]
    fn shift_in_inference_is_stuck()
    {
        let shift = Comp::shift("k", Comp::ret(Value::int(0)));
        let result = agree_comp(&Ctx::new(), &shift, &Dir::Infer);
        assert_eq!(
            result,
            Err(TypeError::StuckExpr {
                expr: Term::Comp(shift),
                hint: text::SHIFT_NEEDS_CHECK,
            }),
            "shift in inference mode must be stuck on the direction"
        );
    }

    /// Rule Reset⇓ returns the body's *checked* type, not the answer (the A3.3
    /// lock-step regression): a `reset` whose body is a *matched*
    /// form reconstructs a type only consistent with — not equal to — the
    /// answer. `reset ⟨ret 0, ret 0⟩` checked against `?` returns the body's
    /// reconstructed `? & ?`, not `?`. Both faces must agree; a `reset` that
    /// returned the answer `?` would diverge from the machine's `ResetBody`
    /// (which returns the body type), invisibly to the coherence oracle, which
    /// excludes the check-only `reset`.
    #[test]
    fn reset_returns_the_matched_body_type_not_the_answer()
    {
        let reset = Comp::reset(Comp::with(
            Comp::ret(Value::int(0)),
            Comp::ret(Value::int(0)),
        ));
        let reconstructed = CompType::with(CompType::Unknown, CompType::Unknown);
        let result = agree_comp(&Ctx::new(), &reset, &Dir::Check(CompType::Unknown));
        assert_eq!(
            result,
            Ok(Ty::Comp(reconstructed)),
            "reset of a matched ⟨_, _⟩ against ? must return the reconstructed ? & ?, not ?"
        );
    }

    /// Rule Reify⇓, INTERIOR bind frame: a row consumed by the *first* bind
    /// frame must thread through a *subsequent* frame into the delivered answer
    /// (`stack_infer(rest, sequenced)` / the `StkBind` re-entry), not merely
    /// fold at a trailing frame. `stk ((x. perform IO.print x) :: (y. ret
    /// y) :: ε)` consuming `F^⟨State⟩ Int` delivers `F^⟨State, IO⟩ Unit`:
    /// ⟨State⟩ (from the first consumed returner) and ⟨IO⟩ (from the first
    /// continuation) both reach the answer through the second, interior
    /// bind frame. The negative twin pins the soundness leg a
    /// *trailing*-only test cannot: checking against a delivered answer
    /// that drops ⟨State⟩ is a mismatch, so threading cannot
    /// silently launder the accumulated row.
    #[test]
    fn reify_interior_bind_frame_threads_the_accumulated_row()
    {
        let stk = Value::stk(Stack::bind(
            "x",
            Comp::perform(io_sig(), "print", Value::var("x")),
            Stack::bind("y", Comp::ret(Value::var("y")), Stack::empty()),
        ));
        let unioned = EffectRow::singleton(state_sig()).union(&EffectRow::singleton(io_sig()));
        let synthesized = ValueType::stk(
            eff(integer(), state_sig()),
            CompType::returner_eff(ValueType::Unit, unioned),
        );
        let result = agree_value(&Ctx::new(), &stk, &Dir::Check(synthesized.clone()));
        assert_eq!(
            result,
            Ok(Ty::Value(synthesized.clone())),
            "the first frame's ⟨State⟩ must thread through the interior bind frame into ⟨State, IO⟩"
        );
        // Negative twin: the threaded ⟨State⟩ cannot be dropped from the answer.
        let dropped = ValueType::stk(eff(integer(), state_sig()), eff(ValueType::Unit, io_sig()));
        let dropped_result = agree_value(&Ctx::new(), &stk, &Dir::Check(dropped.clone()));
        assert_eq!(
            dropped_result,
            Err(TypeError::TypeMismatch {
                expected: Ty::Value(dropped),
                actual: Ty::Value(synthesized),
            }),
            "an interior bind frame must not launder ⟨State⟩ out of the threaded answer"
        );
    }

    /// Rule Resume⇓, Check-mode success (the positive twin of
    /// `resume_delivering_a_row_cannot_drop_it`; the both-directions
    /// discipline): a `resume` delivering a pure `F Int` checks against a
    /// *larger* answer `F^⟨State⟩ Int` — the covariant-`C` widen through
    /// `finish_comp`.
    #[test]
    fn resume_delivered_answer_subsumes_up()
    {
        let stk_ty = ValueType::stk(pure(integer()), pure(integer()));
        let resume = Comp::resume(
            Value::annot(Value::stk(Stack::empty()), stk_ty),
            Comp::ret(Value::int(0)),
        );
        let widened = eff(integer(), state_sig());
        let result = agree_comp(&Ctx::new(), &resume, &Dir::Check(widened.clone()));
        assert_eq!(
            result,
            Ok(Ty::Comp(widened)),
            "resume delivering F Int must check against the larger F^⟨State⟩ Int"
        );
    }

    /// A pure-returner `F^⟨⟩ A`.
    fn pure(of: ValueType) -> CompType
    {
        CompType::returner(of)
    }

    /// The effectful returner `F^⟨E⟩ A`.
    fn eff(
        of: ValueType,
        sig: EffectSig,
    ) -> CompType
    {
        CompType::returner_eff(of, EffectRow::singleton(sig))
    }
}

/// Well-typed values that *infer* exactly `ty` under `scope`.
/// # Termination
/// - reason: `TypedValueStrategy` uses an explicit generation worklist.
/// - measure: pending generation tasks and result frames.
/// - boundedness: type inputs and `u32` depth fuel are finite.
/// - input recursion: none.
fn value_infer_strategy<D>(
    ty: ValueType,
    scope: Scope,
    depth: D,
) -> BoxedStrategy<Value>
where
    D: Into<GenerationDepth>,
{
    TypedValueStrategy {
        mode: TypedValueMode::Infer,
        ty,
        scope,
        depth: u32::from(depth.into()),
    }
    .boxed()
}

/// Well-typed computations that *check* against `ty` under `scope`.
/// # Termination
/// - reason: `TypedCompStrategy` uses an explicit generation worklist.
/// - measure: pending generation tasks and result frames.
/// - boundedness: type inputs and `u32` depth fuel are finite.
/// - input recursion: none.
fn comp_check_strategy<D>(
    ty: CompType,
    scope: Scope,
    depth: D,
) -> BoxedStrategy<Comp>
where
    D: Into<GenerationDepth>,
{
    TypedCompStrategy {
        mode: TypedCompMode::Check,
        ty,
        scope,
        depth: u32::from(depth.into()),
    }
    .boxed()
}

/// A suffixed `Value::Num` literal of the numeric primitive atom `name`
/// (ADR-39), or `None` when `name` is not a numeric primitive.
///
/// The inhabitant a numeric atom both *infers* and *checks against*
/// (monomorphic, ADR-39 D1/D3); the single realizer shared by the symbolic
/// check / infer strategies and the operational rigid generator, so all three
/// agree on what inhabits a numeric atom.
fn numeric_literal_strategy<'name, N>(name: N) -> Option<BoxedStrategy<Value>>
where
    N: Into<NumericLiteralName<'name>>,
{
    let name = name.into();
    match name.as_ref() {
        | "u32" => Some(proptest::num::u32::ANY.prop_map(Value::u32).boxed()),
        | "u64" => Some(proptest::num::u64::ANY.prop_map(Value::u64).boxed()),
        | "i32" => Some(proptest::num::i32::ANY.prop_map(Value::i32).boxed()),
        | "i64" => Some(proptest::num::i64::ANY.prop_map(Value::i64).boxed()),
        | "f32" => Some(proptest::num::f32::ANY.prop_map(Value::f32).boxed()),
        | "f64" => Some(proptest::num::f64::ANY.prop_map(Value::f64).boxed()),
        | _ => None,
    }
}

/// Well-typed computations that *infer* exactly `ty` under `scope`.
/// # Termination
/// - reason: `TypedCompStrategy` uses an explicit generation worklist.
/// - measure: pending generation tasks and result frames.
/// - boundedness: type inputs and `u32` depth fuel are finite.
/// - input recursion: none.
fn comp_infer_strategy<D>(
    ty: CompType,
    scope: Scope,
    depth: D,
) -> BoxedStrategy<Comp>
where
    D: Into<GenerationDepth>,
{
    TypedCompStrategy {
        mode: TypedCompMode::Infer,
        ty,
        scope,
        depth: u32::from(depth.into()),
    }
    .boxed()
}

/// Well-typed values that *check* against `ty` under `scope`.
/// # Termination
/// - reason: `TypedValueStrategy` uses an explicit generation worklist.
/// - measure: pending generation tasks and result frames.
/// - boundedness: type inputs and `u32` depth fuel are finite.
/// - input recursion: none.
fn value_check_strategy<D>(
    ty: ValueType,
    scope: Scope,
    depth: D,
) -> BoxedStrategy<Value>
where
    D: Into<GenerationDepth>,
{
    TypedValueStrategy {
        mode: TypedValueMode::Check,
        ty,
        scope,
        depth: u32::from(depth.into()),
    }
    .boxed()
}

/// The visible (innermost) bindings of a scope, respecting shadowing, so the
/// generators never pick a variable whose innermost binding has another type.
fn visible(scope: &Scope) -> Scope
{
    let mut seen: Scope = Vec::new();
    for entry in scope.iter().rev() {
        if !seen.iter().any(|kept| {
            let (ref kept_name, _) = *kept;
            kept_name == &entry.0
        }) {
            seen.push(entry.clone());
        }
    }
    seen
}

/// Grades `r` from the standard pool with `lower ⊑ r`.
///
/// Used to draw a thunk *literal's* grade when checking against `U_lower B`:
/// rule Thunk⇓ requires `lower ⊑ r`, so any such `r` succeeds — and because
/// `r` may be *strictly* above `lower`, this exercises the non-trivial
/// `lower ⊏ r` case (e.g. `thunk_ω` checking against `U_1 B`), not only grade
/// equality.
fn grade_geq(lower: Grade) -> BoxedStrategy<Grade>
{
    let mut options: Vec<BoxedStrategy<Grade>> = Vec::new();
    for candidate in [
        Grade::ZERO,
        Grade::ONE,
        Grade::fin(gandr_core_checker::boundary::GradeBound::from(2_u64)),
        Grade::fin(gandr_core_checker::boundary::GradeBound::from(3_u64)),
        Grade::OMEGA,
    ] {
        if bool::from(lower.leq(candidate)) {
            options.push(Just(candidate).boxed());
        }
    }
    // `lower ⊑ lower` always holds, so `options` is non-empty.
    Union::new(options).boxed()
}

#[derive(Clone, Copy, Debug)]
enum TypedValueMode
{
    Infer,
    Check,
    Rigid,
}

#[derive(Clone, Copy, Debug)]
enum TypedCompMode
{
    Infer,
    Check,
    RigidInfer,
    RigidCheck,
}

#[repr(transparent)]
#[derive(Clone, Debug)]
struct GeneratedTree<T>
{
    value: T,
}

impl<T> ValueTree for GeneratedTree<T>
where
    T: Clone + core::fmt::Debug,
{
    type Value = T;

    fn current(&self) -> Self::Value
    {
        self.value.clone()
    }

    fn simplify(&mut self) -> bool
    {
        false
    }

    fn complicate(&mut self) -> bool
    {
        false
    }
}

#[derive(Clone, Debug)]
struct TypedValueStrategy
{
    mode: TypedValueMode,
    ty: ValueType,
    scope: Scope,
    depth: u32,
}

impl Strategy for TypedValueStrategy
{
    type Tree = GeneratedTree<Value>;
    type Value = Value;

    fn new_tree(
        &self,
        runner: &mut proptest::test_runner::TestRunner,
    ) -> NewTree<Self>
    {
        Ok(GeneratedTree {
            value: generate_typed_value(
                self.mode,
                self.ty.clone(),
                self.scope.clone(),
                self.depth,
                runner,
            )?,
        })
    }
}

#[derive(Clone, Debug)]
struct TypedCompStrategy
{
    mode: TypedCompMode,
    ty: CompType,
    scope: Scope,
    depth: u32,
}

impl Strategy for TypedCompStrategy
{
    type Tree = GeneratedTree<Comp>;
    type Value = Comp;

    fn new_tree(
        &self,
        runner: &mut proptest::test_runner::TestRunner,
    ) -> NewTree<Self>
    {
        Ok(GeneratedTree {
            value: generate_typed_comp(
                self.mode,
                self.ty.clone(),
                self.scope.clone(),
                self.depth,
                runner,
            )?,
        })
    }
}

enum TypedStep
{
    ValueTask
    {
        mode: TypedValueMode,
        ty: ValueType,
        scope: Scope,
        depth: u32,
    },
    CompTask
    {
        mode: TypedCompMode,
        ty: CompType,
        scope: Scope,
        depth: u32,
    },
    Frame(TypedFrame),
}

enum TypedFrame
{
    ValuePair,
    ValueInj1,
    ValueInj2,
    ValueList(usize),
    ValueRecord(Vec<String>),
    ValueThunk(Grade),
    ValueAnnot(ValueType),
    CompRet,
    CompForce,
    CompLam(String),
    CompLamAnn
    {
        name: String,
        annot: ValueType,
    },
    CompApp,
    CompBind(String),
    CompCase
    {
        fst_name: String,
        snd_name: String,
        scrut_annot: Option<ValueType>,
    },
    CompListCase
    {
        head_name: String,
        tail_name: String,
        scrut_annot: Option<ValueType>,
    },
    CompSplit
    {
        fst_name: String,
        snd_name: String,
    },
    CompSplitMotive
    {
        fst_name: String,
        snd_name: String,
        motive: CompType,
    },
    CompWith,
    CompPrj(bool),
    CompRecordProj(String),
}

enum TypedOutput
{
    Value(Value),
    Comp(Comp),
}

enum ValueAction
{
    Var(String),
    Unit,
    Pair(ValueType, ValueType, TypedValueMode, u32),
    Inj(bool, ValueType, u32),
    List(ValueType, u32),
    Record(Vec<(String, ValueType)>, TypedValueMode, u32),
    ThunkInfer(Grade, CompType, u32),
    ThunkCheck(Grade, CompType, u32),
    Int,
    String,
    Numeric(String),
    Hole,
    UnknownSum(u32),
    UnknownThunk(u32),
    Annot(u32),
    RigidThunk(CompType, u32),
}

enum CompAction
{
    Hole,
    LamCheck(ValueType, CompType),
    LamAnnCheck(ValueType, CompType),
    LamAnnInfer(ValueType, CompType),
    Ret(ValueType, TypedValueMode),
    With(CompType, CompType),
    UnknownLam(u32),
    UnknownRet(u32),
    Force,
    Bind,
    App,
    Split,
    Case,
    ListCase,
    RecordProj(ValueType),
    Prj,
}

fn draw_strategy<S>(
    runner: &mut proptest::test_runner::TestRunner,
    strategy: S,
) -> Result<S::Value, proptest::test_runner::Reason>
where
    S: Strategy,
{
    let tree = strategy.new_tree(runner)?;
    Ok(ValueTree::current(&tree))
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct ChoiceCount(usize);

#[repr(transparent)]
#[derive(Clone, Copy)]
struct ChoiceIndex(usize);

fn draw_index(
    runner: &mut proptest::test_runner::TestRunner,
    len: ChoiceCount,
) -> Result<ChoiceIndex, proptest::test_runner::Reason>
{
    if len.0 == 0 {
        return Err("typed generator had no finite inhabitant for the requested mode/type".into());
    }
    if len.0 == 1 {
        return Ok(ChoiceIndex(0));
    }
    draw_strategy(runner, 0_usize .. len.0).map(ChoiceIndex)
}

fn generate_typed_value<D>(
    mode: TypedValueMode,
    ty: ValueType,
    scope: Scope,
    depth: D,
    runner: &mut proptest::test_runner::TestRunner,
) -> Result<Value, proptest::test_runner::Reason>
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    let output = generate_typed_from(
        TypedStep::ValueTask {
            mode,
            ty,
            scope,
            depth,
        },
        runner,
    )?;
    match output {
        | TypedOutput::Value(value) => Ok(value),
        | TypedOutput::Comp(_) => panic!("value strategy produced a computation"),
    }
}

fn generate_typed_comp<D>(
    mode: TypedCompMode,
    ty: CompType,
    scope: Scope,
    depth: D,
    runner: &mut proptest::test_runner::TestRunner,
) -> Result<Comp, proptest::test_runner::Reason>
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    let output = generate_typed_from(
        TypedStep::CompTask {
            mode,
            ty,
            scope,
            depth,
        },
        runner,
    )?;
    match output {
        | TypedOutput::Comp(comp) => Ok(comp),
        | TypedOutput::Value(_) => panic!("computation strategy produced a value"),
    }
}

fn generate_typed_from(
    root: TypedStep,
    runner: &mut proptest::test_runner::TestRunner,
) -> Result<TypedOutput, proptest::test_runner::Reason>
{
    let mut steps = vec![root];
    let mut outputs = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            | TypedStep::ValueTask {
                mode,
                ty,
                scope,
                depth,
            } => schedule_value_task(mode, ty, scope, depth, &mut steps, &mut outputs, runner)?,
            | TypedStep::CompTask {
                mode,
                ty,
                scope,
                depth,
            } => schedule_comp_task(mode, ty, scope, depth, &mut steps, &mut outputs, runner)?,
            | TypedStep::Frame(frame) => finish_typed_frame(frame, &mut outputs),
        }
        while matches!(steps.last(), Some(TypedStep::Frame(_))) {
            let Some(TypedStep::Frame(frame)) = steps.pop()
            else {
                panic!("frame already matched");
            };
            finish_typed_frame(frame, &mut outputs);
        }
    }
    Ok(outputs.pop().expect("typed generator produced a root term"))
}

fn push_typed_frame(
    steps: &mut Vec<TypedStep>,
    frame: TypedFrame,
    tasks: Vec<TypedStep>,
)
{
    steps.push(TypedStep::Frame(frame));
    for task in tasks.into_iter().rev() {
        steps.push(task);
    }
}

fn value_task<D>(
    mode: TypedValueMode,
    ty: ValueType,
    scope: Scope,
    depth: D,
) -> TypedStep
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    TypedStep::ValueTask {
        mode,
        ty,
        scope,
        depth,
    }
}

fn comp_task<D>(
    mode: TypedCompMode,
    ty: CompType,
    scope: Scope,
    depth: D,
) -> TypedStep
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    TypedStep::CompTask {
        mode,
        ty,
        scope,
        depth,
    }
}

fn schedule_value_task<D>(
    mode: TypedValueMode,
    ty: ValueType,
    scope: Scope,
    depth: D,
    steps: &mut Vec<TypedStep>,
    outputs: &mut Vec<TypedOutput>,
    runner: &mut proptest::test_runner::TestRunner,
) -> Result<(), proptest::test_runner::Reason>
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    let mut choices = Vec::new();
    for entry in &visible(&scope) {
        let matches = match mode {
            | TypedValueMode::Infer | TypedValueMode::Rigid => entry.1 == ty,
            | TypedValueMode::Check => bool::from(value_subtype(&entry.1, &ty)),
        };
        if matches {
            choices.push(ValueAction::Var(entry.0.clone()));
        }
    }
    if matches!(mode, TypedValueMode::Check) {
        choices.push(ValueAction::Hole);
    }

    match (mode, &ty) {
        | (_, &ValueType::Unit) => choices.push(ValueAction::Unit),
        | (_, &ValueType::Prod(ref fst, ref snd)) => choices.push(ValueAction::Pair(
            fst.as_ref().clone(),
            snd.as_ref().clone(),
            mode,
            depth,
        )),
        | (TypedValueMode::Check | TypedValueMode::Rigid, &ValueType::Sum(ref lhs, ref rhs)) => {
            choices.push(ValueAction::Inj(true, lhs.as_ref().clone(), depth));
            choices.push(ValueAction::Inj(false, rhs.as_ref().clone(), depth));
        },
        | (TypedValueMode::Check | TypedValueMode::Rigid, &ValueType::List(ref elem)) => {
            choices.push(ValueAction::List(elem.as_ref().clone(), depth));
        },
        | (_, &ValueType::Record(ref fields)) => choices.push(ValueAction::Record(
            fields
                .iter()
                .map(|(label, field_ty)| (label.clone(), field_ty.as_ref().clone()))
                .collect(),
            mode,
            depth,
        )),
        | (TypedValueMode::Infer, &ValueType::Thunk(grade, ref body)) => {
            if depth > 0 {
                choices.push(ValueAction::ThunkInfer(
                    grade,
                    body.as_ref().clone(),
                    depth.saturating_sub(1),
                ));
            }
        },
        | (TypedValueMode::Check, &ValueType::Thunk(grade, ref body)) => {
            choices.push(ValueAction::ThunkCheck(grade, body.as_ref().clone(), depth));
        },
        | (TypedValueMode::Rigid, &ValueType::Thunk(_, ref body)) => {
            choices.push(ValueAction::RigidThunk(body.as_ref().clone(), depth));
        },
        | (_, &ValueType::Atom(ref name)) if name == "Integer" => choices.push(ValueAction::Int),
        | (_, &ValueType::Atom(ref name)) if name == "String" => choices.push(ValueAction::String),
        | (_, &ValueType::Atom(ref name)) if numeric_literal_strategy(name).is_some() => {
            choices.push(ValueAction::Numeric(name.clone()));
        },
        | (TypedValueMode::Infer, &ValueType::Unknown) => choices.push(ValueAction::Hole),
        | (TypedValueMode::Check, &ValueType::Unknown) if depth > 0 => {
            let sub_depth = depth.saturating_sub(1);
            choices.push(ValueAction::Pair(
                ValueType::Unknown,
                ValueType::Unknown,
                TypedValueMode::Check,
                sub_depth,
            ));
            choices.push(ValueAction::UnknownSum(sub_depth));
            choices.push(ValueAction::UnknownThunk(sub_depth));
            choices.push(ValueAction::List(ValueType::Unknown, sub_depth));
        },
        | _ => {},
    }

    match mode {
        | TypedValueMode::Infer => choices.push(ValueAction::Annot(depth)),
        | TypedValueMode::Check if depth > 0 => {
            choices.push(ValueAction::Annot(depth.saturating_sub(1)));
        },
        | TypedValueMode::Check | TypedValueMode::Rigid => {},
    }

    let choice = draw_index(runner, ChoiceCount(choices.len()))?;
    match choices.remove(choice.0) {
        | ValueAction::Var(name) => outputs.push(TypedOutput::Value(Value::var(&name))),
        | ValueAction::Unit => outputs.push(TypedOutput::Value(Value::Unit)),
        | ValueAction::Pair(fst, snd, child_mode, child_depth) => {
            push_typed_frame(steps, TypedFrame::ValuePair, vec![
                value_task(child_mode, fst, scope.clone(), child_depth),
                value_task(child_mode, snd, scope, child_depth),
            ]);
        },
        | ValueAction::Inj(first, payload_ty, child_depth) => {
            let child_mode = if matches!(mode, TypedValueMode::Rigid) {
                TypedValueMode::Rigid
            }
            else {
                TypedValueMode::Check
            };
            push_typed_frame(
                steps,
                if first {
                    TypedFrame::ValueInj1
                }
                else {
                    TypedFrame::ValueInj2
                },
                vec![value_task(child_mode, payload_ty, scope, child_depth)],
            );
        },
        | ValueAction::List(elem_ty, child_depth) => {
            let len = draw_strategy(runner, 0_usize ..= 3_usize)?;
            let child_mode = if matches!(mode, TypedValueMode::Rigid) {
                TypedValueMode::Rigid
            }
            else {
                TypedValueMode::Check
            };
            let tasks = core::iter::repeat_with(|| {
                value_task(child_mode, elem_ty.clone(), scope.clone(), child_depth)
            })
            .take(len)
            .collect();
            push_typed_frame(steps, TypedFrame::ValueList(len), tasks);
        },
        | ValueAction::Record(fields, child_mode, child_depth) => {
            let labels = fields
                .iter()
                .map(|field| {
                    let (ref label, _) = *field;
                    label.clone()
                })
                .collect();
            let tasks = fields
                .into_iter()
                .map(|(_, field_ty)| value_task(child_mode, field_ty, scope.clone(), child_depth))
                .collect();
            push_typed_frame(steps, TypedFrame::ValueRecord(labels), tasks);
        },
        | ValueAction::ThunkInfer(grade, body, child_depth) => push_typed_frame(
            steps,
            TypedFrame::ValueThunk(grade),
            vec![comp_task(TypedCompMode::Infer, body, scope, child_depth)],
        ),
        | ValueAction::ThunkCheck(lower, body, child_depth) => {
            let grade = draw_strategy(runner, grade_geq(lower))?;
            push_typed_frame(steps, TypedFrame::ValueThunk(grade), vec![comp_task(
                TypedCompMode::Check,
                body,
                scope,
                child_depth,
            )]);
        },
        | ValueAction::Int => {
            let n = draw_strategy(runner, proptest::num::i64::ANY)?;
            outputs.push(TypedOutput::Value(Value::int(n)));
        },
        | ValueAction::String => {
            let value = draw_strategy(runner, prop_oneof![Just(""), Just("rigid")])?;
            outputs.push(TypedOutput::Value(Value::string(value)));
        },
        | ValueAction::Numeric(name) => {
            let strategy = numeric_literal_strategy(&name)
                .expect("numeric action only built for numeric atoms");
            let value = draw_strategy(runner, strategy)?;
            outputs.push(TypedOutput::Value(value));
        },
        | ValueAction::Hole => {
            let id = draw_strategy(runner, hole_id())?;
            outputs.push(TypedOutput::Value(Value::Hole(id)));
        },
        | ValueAction::UnknownSum(child_depth) => {
            let first = draw_strategy(runner, proptest::bool::ANY)?;
            push_typed_frame(
                steps,
                if first {
                    TypedFrame::ValueInj1
                }
                else {
                    TypedFrame::ValueInj2
                },
                vec![value_task(
                    TypedValueMode::Check,
                    ValueType::Unknown,
                    scope,
                    child_depth,
                )],
            );
        },
        | ValueAction::UnknownThunk(child_depth) => {
            let grade = draw_strategy(runner, any_grade())?;
            push_typed_frame(steps, TypedFrame::ValueThunk(grade), vec![comp_task(
                TypedCompMode::Check,
                CompType::Unknown,
                scope,
                child_depth,
            )]);
        },
        | ValueAction::Annot(child_depth) => push_typed_frame(
            steps,
            TypedFrame::ValueAnnot(ty.clone()),
            vec![value_task(TypedValueMode::Check, ty, scope, child_depth)],
        ),
        | ValueAction::RigidThunk(body, child_depth) => {
            push_typed_frame(steps, TypedFrame::ValueThunk(Grade::OMEGA), vec![
                comp_task(TypedCompMode::RigidInfer, body, scope, child_depth),
            ]);
        },
    }
    Ok(())
}

fn rigid_leaf_value_type() -> impl Strategy<Value = ValueType>
{
    prop_oneof![
        Just(ValueType::Unit),
        Just(integer()),
        Just(string()),
        numeric_atom(),
    ]
}

fn schedule_comp_task<D>(
    mode: TypedCompMode,
    ty: CompType,
    scope: Scope,
    depth: D,
    steps: &mut Vec<TypedStep>,
    outputs: &mut Vec<TypedOutput>,
    runner: &mut proptest::test_runner::TestRunner,
) -> Result<(), proptest::test_runner::Reason>
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    let mut choices = Vec::new();
    match (mode, &ty) {
        | (TypedCompMode::Check, _) | (TypedCompMode::Infer, &CompType::Unknown) => {
            choices.push(CompAction::Hole);
        },
        | _ => {},
    }
    match (mode, &ty) {
        | (
            TypedCompMode::Check | TypedCompMode::RigidCheck,
            &CompType::Arrow(ref arg, ref res),
        ) => {
            choices.push(CompAction::LamCheck(
                arg.as_ref().clone(),
                res.as_ref().clone(),
            ));
            if matches!(mode, TypedCompMode::Check) {
                choices.push(CompAction::LamAnnCheck(
                    arg.as_ref().clone(),
                    res.as_ref().clone(),
                ));
            }
        },
        | (
            TypedCompMode::Infer | TypedCompMode::RigidInfer,
            &CompType::Arrow(ref arg, ref res),
        ) => {
            choices.push(CompAction::LamAnnInfer(
                arg.as_ref().clone(),
                res.as_ref().clone(),
            ));
        },
        | (TypedCompMode::Check, &CompType::F(ref payload, _)) => {
            choices.push(CompAction::Ret(
                payload.as_ref().clone(),
                TypedValueMode::Check,
            ));
        },
        | (TypedCompMode::Infer, &CompType::F(ref payload, _)) => {
            choices.push(CompAction::Ret(
                payload.as_ref().clone(),
                TypedValueMode::Infer,
            ));
        },
        | (TypedCompMode::RigidCheck | TypedCompMode::RigidInfer, &CompType::F(ref payload, _)) => {
            choices.push(CompAction::Ret(
                payload.as_ref().clone(),
                TypedValueMode::Rigid,
            ));
        },
        | (TypedCompMode::Check | TypedCompMode::RigidCheck, &CompType::With(ref lhs, ref rhs)) => {
            choices.push(CompAction::With(lhs.as_ref().clone(), rhs.as_ref().clone()));
        },
        | (TypedCompMode::Check, &CompType::Unknown) if depth > 0 => {
            let sub_depth = depth.saturating_sub(1);
            choices.push(CompAction::UnknownLam(sub_depth));
            choices.push(CompAction::UnknownRet(sub_depth));
        },
        | _ => {},
    }

    choices.push(CompAction::Force);
    if depth > 0 {
        choices.push(CompAction::Bind);
        choices.push(CompAction::App);
        choices.push(CompAction::Split);
        if matches!(mode, TypedCompMode::Check | TypedCompMode::RigidCheck) {
            choices.push(CompAction::Case);
            choices.push(CompAction::ListCase);
        }
        if let CompType::F(ref payload, ref row) = ty
            && bool::from(row.is_empty())
        {
            choices.push(CompAction::RecordProj(payload.as_ref().clone()));
        }
        if !matches!(mode, TypedCompMode::RigidInfer | TypedCompMode::RigidCheck) {
            choices.push(CompAction::Prj);
        }
    }

    let choice = draw_index(runner, ChoiceCount(choices.len()))?;
    match choices.remove(choice.0) {
        | CompAction::Hole => {
            let id = draw_strategy(runner, hole_id())?;
            outputs.push(TypedOutput::Comp(Comp::Hole(id)));
        },
        | CompAction::LamCheck(arg_ty, res_ty) => {
            let name = draw_strategy(runner, binder_name())?;
            let mut inner_scope = scope;
            inner_scope.push((name.clone(), arg_ty));
            let child_mode = if matches!(mode, TypedCompMode::RigidCheck) {
                TypedCompMode::RigidCheck
            }
            else {
                TypedCompMode::Check
            };
            push_typed_frame(steps, TypedFrame::CompLam(name), vec![comp_task(
                child_mode,
                res_ty,
                inner_scope,
                depth,
            )]);
        },
        | CompAction::LamAnnCheck(arg_ty, res_ty) => {
            let name = draw_strategy(runner, binder_name())?;
            let annot = draw_strategy(runner, value_supertype(&arg_ty))?;
            let mut inner_scope = scope;
            inner_scope.push((name.clone(), annot.clone()));
            push_typed_frame(steps, TypedFrame::CompLamAnn { name, annot }, vec![
                comp_task(TypedCompMode::Infer, res_ty, inner_scope, depth),
            ]);
        },
        | CompAction::LamAnnInfer(arg_ty, res_ty) => {
            let name = draw_strategy(runner, binder_name())?;
            let mut inner_scope = scope;
            inner_scope.push((name.clone(), arg_ty.clone()));
            let child_mode = if matches!(mode, TypedCompMode::RigidInfer) {
                TypedCompMode::RigidInfer
            }
            else {
                TypedCompMode::Infer
            };
            push_typed_frame(
                steps,
                TypedFrame::CompLamAnn {
                    name,
                    annot: arg_ty,
                },
                vec![comp_task(child_mode, res_ty, inner_scope, depth)],
            );
        },
        | CompAction::Ret(payload, value_mode) => {
            push_typed_frame(steps, TypedFrame::CompRet, vec![value_task(
                value_mode, payload, scope, depth,
            )]);
        },
        | CompAction::With(lhs, rhs) => {
            let child_mode = if matches!(mode, TypedCompMode::RigidCheck) {
                TypedCompMode::RigidCheck
            }
            else {
                TypedCompMode::Check
            };
            push_typed_frame(steps, TypedFrame::CompWith, vec![
                comp_task(child_mode, lhs, scope.clone(), depth),
                comp_task(child_mode, rhs, scope, depth),
            ]);
        },
        | CompAction::UnknownLam(child_depth) => {
            let name = draw_strategy(runner, binder_name())?;
            let mut inner_scope = scope;
            inner_scope.push((name.clone(), ValueType::Unknown));
            push_typed_frame(steps, TypedFrame::CompLam(name), vec![comp_task(
                TypedCompMode::Check,
                CompType::Unknown,
                inner_scope,
                child_depth,
            )]);
        },
        | CompAction::UnknownRet(child_depth) => {
            push_typed_frame(steps, TypedFrame::CompRet, vec![value_task(
                TypedValueMode::Check,
                ValueType::Unknown,
                scope,
                child_depth,
            )]);
        },
        | CompAction::Force => {
            let rigid = matches!(mode, TypedCompMode::RigidInfer | TypedCompMode::RigidCheck);
            let grade = if rigid {
                Grade::OMEGA
            }
            else {
                draw_strategy(runner, forceable_grade())?
            };
            let value_mode = if rigid {
                TypedValueMode::Rigid
            }
            else {
                TypedValueMode::Infer
            };
            let thunk_ty = ValueType::thunk(grade, ty);
            push_typed_frame(steps, TypedFrame::CompForce, vec![value_task(
                value_mode,
                thunk_ty,
                scope,
                depth.saturating_sub(1),
            )]);
        },
        | CompAction::Bind => {
            let rigid = matches!(mode, TypedCompMode::RigidInfer | TypedCompMode::RigidCheck);
            let payload = if rigid {
                draw_strategy(runner, rigid_leaf_value_type())?
            }
            else {
                draw_strategy(runner, leaf_value_type())?
            };
            let name = draw_strategy(runner, binder_name())?;
            let mut inner_scope = scope.clone();
            inner_scope.push((name.clone(), payload.clone()));
            let (infer_mode, cont_mode) = match mode {
                | TypedCompMode::Infer => (TypedCompMode::Infer, TypedCompMode::Infer),
                | TypedCompMode::Check => (TypedCompMode::Infer, TypedCompMode::Check),
                | TypedCompMode::RigidInfer => {
                    (TypedCompMode::RigidInfer, TypedCompMode::RigidInfer)
                },
                | TypedCompMode::RigidCheck => {
                    (TypedCompMode::RigidInfer, TypedCompMode::RigidCheck)
                },
            };
            let sub = depth.saturating_sub(1);
            push_typed_frame(steps, TypedFrame::CompBind(name), vec![
                comp_task(infer_mode, CompType::returner(payload), scope, sub),
                comp_task(cont_mode, ty, inner_scope, sub),
            ]);
        },
        | CompAction::App => {
            let rigid = matches!(mode, TypedCompMode::RigidInfer | TypedCompMode::RigidCheck);
            let arg_ty = if rigid {
                draw_strategy(runner, rigid_leaf_value_type())?
            }
            else {
                draw_strategy(runner, leaf_value_type())?
            };
            let infer_mode = if rigid {
                TypedCompMode::RigidInfer
            }
            else {
                TypedCompMode::Infer
            };
            let value_mode = if rigid {
                TypedValueMode::Rigid
            }
            else {
                TypedValueMode::Check
            };
            let sub = depth.saturating_sub(1);
            push_typed_frame(steps, TypedFrame::CompApp, vec![
                comp_task(
                    infer_mode,
                    CompType::arrow(arg_ty.clone(), ty),
                    scope.clone(),
                    sub,
                ),
                value_task(value_mode, arg_ty, scope, sub),
            ]);
        },
        | CompAction::Split => {
            let rigid = matches!(mode, TypedCompMode::RigidInfer | TypedCompMode::RigidCheck);
            let name = draw_strategy(runner, binder_name())?;
            let fst_ty = if rigid {
                draw_strategy(runner, rigid_leaf_value_type())?
            }
            else {
                draw_strategy(runner, leaf_value_type())?
            };
            let snd_ty = if rigid {
                draw_strategy(runner, rigid_leaf_value_type())?
            }
            else {
                draw_strategy(runner, leaf_value_type())?
            };
            let snd_name = format!("{name}2");
            let mut inner_scope = scope.clone();
            inner_scope.push((name.clone(), fst_ty.clone()));
            inner_scope.push((snd_name.clone(), snd_ty.clone()));
            let sub = depth.saturating_sub(1);
            let (body_mode, value_mode, frame) = match mode {
                | TypedCompMode::Infer => (
                    TypedCompMode::Infer,
                    TypedValueMode::Infer,
                    TypedFrame::CompSplitMotive {
                        fst_name: name,
                        snd_name,
                        motive: ty.clone(),
                    },
                ),
                | TypedCompMode::Check => (
                    TypedCompMode::Check,
                    TypedValueMode::Infer,
                    TypedFrame::CompSplit {
                        fst_name: name,
                        snd_name,
                    },
                ),
                | TypedCompMode::RigidInfer => (
                    TypedCompMode::RigidInfer,
                    TypedValueMode::Rigid,
                    TypedFrame::CompSplitMotive {
                        fst_name: name,
                        snd_name,
                        motive: ty.clone(),
                    },
                ),
                | TypedCompMode::RigidCheck => (
                    TypedCompMode::RigidCheck,
                    TypedValueMode::Rigid,
                    TypedFrame::CompSplit {
                        fst_name: name,
                        snd_name,
                    },
                ),
            };
            push_typed_frame(steps, frame, vec![
                value_task(value_mode, ValueType::prod(fst_ty, snd_ty), scope, sub),
                comp_task(body_mode, ty, inner_scope, sub),
            ]);
        },
        | CompAction::Case => {
            let rigid = matches!(mode, TypedCompMode::RigidCheck);
            let name = draw_strategy(runner, binder_name())?;
            let lhs_ty = if rigid {
                draw_strategy(runner, rigid_leaf_value_type())?
            }
            else {
                draw_strategy(runner, leaf_value_type())?
            };
            let rhs_ty = if rigid {
                draw_strategy(runner, rigid_leaf_value_type())?
            }
            else {
                draw_strategy(runner, leaf_value_type())?
            };
            let value_mode = if rigid {
                TypedValueMode::Rigid
            }
            else {
                TypedValueMode::Check
            };
            let body_mode = if rigid {
                TypedCompMode::RigidCheck
            }
            else {
                TypedCompMode::Check
            };
            let sum_ty = ValueType::sum(lhs_ty.clone(), rhs_ty.clone());
            let snd_name = format!("{name}2");
            let mut fst_scope = scope.clone();
            fst_scope.push((name.clone(), lhs_ty));
            let mut snd_scope = scope.clone();
            snd_scope.push((snd_name.clone(), rhs_ty));
            let sub = depth.saturating_sub(1);
            push_typed_frame(
                steps,
                TypedFrame::CompCase {
                    fst_name: name,
                    snd_name,
                    scrut_annot: Some(sum_ty.clone()),
                },
                vec![
                    value_task(value_mode, sum_ty, scope, sub),
                    comp_task(body_mode, ty.clone(), fst_scope, sub),
                    comp_task(body_mode, ty, snd_scope, sub),
                ],
            );
        },
        | CompAction::ListCase => {
            let rigid = matches!(mode, TypedCompMode::RigidCheck);
            let name = draw_strategy(runner, binder_name())?;
            let elem_ty = if rigid {
                draw_strategy(runner, rigid_leaf_value_type())?
            }
            else {
                draw_strategy(runner, leaf_value_type())?
            };
            let value_mode = if rigid {
                TypedValueMode::Rigid
            }
            else {
                TypedValueMode::Check
            };
            let body_mode = if rigid {
                TypedCompMode::RigidCheck
            }
            else {
                TypedCompMode::Check
            };
            let list_ty = ValueType::list(elem_ty.clone());
            let tail_name = format!("{name}2");
            let mut cons_scope = scope.clone();
            cons_scope.push((name.clone(), elem_ty.clone()));
            cons_scope.push((tail_name.clone(), ValueType::list(elem_ty)));
            let sub = depth.saturating_sub(1);
            push_typed_frame(
                steps,
                TypedFrame::CompListCase {
                    head_name: name,
                    tail_name,
                    scrut_annot: Some(list_ty.clone()),
                },
                vec![
                    value_task(value_mode, list_ty, scope.clone(), sub),
                    comp_task(body_mode, ty.clone(), scope, sub),
                    comp_task(body_mode, ty, cons_scope, sub),
                ],
            );
        },
        | CompAction::RecordProj(field_ty) => {
            let rigid = matches!(mode, TypedCompMode::RigidInfer | TypedCompMode::RigidCheck);
            let value_mode = if rigid {
                TypedValueMode::Rigid
            }
            else {
                TypedValueMode::Infer
            };
            let label = draw_strategy(runner, record_label())?;
            let record_ty = ValueType::record([(label.clone(), field_ty)]);
            push_typed_frame(steps, TypedFrame::CompRecordProj(label), vec![value_task(
                value_mode,
                record_ty,
                scope,
                depth.saturating_sub(1),
            )]);
        },
        | CompAction::Prj => {
            let first = draw_strategy(runner, proptest::bool::ANY)?;
            let other = CompType::returner(ValueType::Unit);
            let with_ty = if first {
                CompType::with(ty, other)
            }
            else {
                CompType::with(other, ty)
            };
            push_typed_frame(steps, TypedFrame::CompPrj(first), vec![comp_task(
                TypedCompMode::Infer,
                with_ty,
                scope,
                depth.saturating_sub(1),
            )]);
        },
    }
    Ok(())
}

fn pop_typed_value(outputs: &mut Vec<TypedOutput>) -> Value
{
    match outputs.pop() {
        | Some(TypedOutput::Value(value)) => value,
        | _ => panic!("typed frame expected a value"),
    }
}

fn pop_typed_comp(outputs: &mut Vec<TypedOutput>) -> Comp
{
    match outputs.pop() {
        | Some(TypedOutput::Comp(comp)) => comp,
        | _ => panic!("typed frame expected a computation"),
    }
}

fn take_typed_values(
    outputs: &mut Vec<TypedOutput>,
    len: ChoiceCount,
) -> Vec<Value>
{
    let mut values = Vec::with_capacity(len.0);
    for _ in 0 .. len.0 {
        values.push(pop_typed_value(outputs));
    }
    values.reverse();
    values
}

fn finish_typed_frame(
    frame: TypedFrame,
    outputs: &mut Vec<TypedOutput>,
)
{
    match frame {
        | TypedFrame::ValuePair => {
            let snd = pop_typed_value(outputs);
            let fst = pop_typed_value(outputs);
            outputs.push(TypedOutput::Value(Value::pair(fst, snd)));
        },
        | TypedFrame::ValueInj1 => {
            let payload = pop_typed_value(outputs);
            outputs.push(TypedOutput::Value(Value::inj1(payload)));
        },
        | TypedFrame::ValueInj2 => {
            let payload = pop_typed_value(outputs);
            outputs.push(TypedOutput::Value(Value::inj2(payload)));
        },
        | TypedFrame::ValueList(len) => {
            let values = take_typed_values(outputs, ChoiceCount(len));
            outputs.push(TypedOutput::Value(Value::list(values)));
        },
        | TypedFrame::ValueRecord(labels) => {
            let values = take_typed_values(outputs, ChoiceCount(labels.len()));
            outputs.push(TypedOutput::Value(Value::record(
                labels.into_iter().zip(values),
            )));
        },
        | TypedFrame::ValueThunk(grade) => {
            let body = pop_typed_comp(outputs);
            outputs.push(TypedOutput::Value(Value::thunk(grade, body)));
        },
        | TypedFrame::ValueAnnot(ty) => {
            let inner = pop_typed_value(outputs);
            outputs.push(TypedOutput::Value(Value::annot(inner, ty)));
        },
        | TypedFrame::CompRet => {
            let value = pop_typed_value(outputs);
            outputs.push(TypedOutput::Comp(Comp::ret(value)));
        },
        | TypedFrame::CompForce => {
            let value = pop_typed_value(outputs);
            outputs.push(TypedOutput::Comp(Comp::force(value)));
        },
        | TypedFrame::CompLam(name) => {
            let body = pop_typed_comp(outputs);
            outputs.push(TypedOutput::Comp(Comp::lam(&name, body)));
        },
        | TypedFrame::CompLamAnn { name, annot } => {
            let body = pop_typed_comp(outputs);
            outputs.push(TypedOutput::Comp(Comp::lam_ann(&name, annot, body)));
        },
        | TypedFrame::CompApp => {
            let arg = pop_typed_value(outputs);
            let head = pop_typed_comp(outputs);
            outputs.push(TypedOutput::Comp(Comp::app(head, arg)));
        },
        | TypedFrame::CompBind(name) => {
            let cont = pop_typed_comp(outputs);
            let bound = pop_typed_comp(outputs);
            outputs.push(TypedOutput::Comp(Comp::bind(bound, &name, cont)));
        },
        | TypedFrame::CompCase {
            fst_name,
            snd_name,
            scrut_annot,
        } => {
            let snd_body = pop_typed_comp(outputs);
            let fst_body = pop_typed_comp(outputs);
            let mut scrut = pop_typed_value(outputs);
            if let Some(ty) = scrut_annot {
                scrut = Value::annot(scrut, ty);
            }
            outputs.push(TypedOutput::Comp(Comp::case(
                scrut, &fst_name, fst_body, &snd_name, snd_body,
            )));
        },
        | TypedFrame::CompListCase {
            head_name,
            tail_name,
            scrut_annot,
        } => {
            let cons = pop_typed_comp(outputs);
            let nil = pop_typed_comp(outputs);
            let mut scrut = pop_typed_value(outputs);
            if let Some(ty) = scrut_annot {
                scrut = Value::annot(scrut, ty);
            }
            outputs.push(TypedOutput::Comp(Comp::list_case(
                scrut, nil, &head_name, &tail_name, cons,
            )));
        },
        | TypedFrame::CompSplit { fst_name, snd_name } => {
            let body = pop_typed_comp(outputs);
            let scrut = pop_typed_value(outputs);
            outputs.push(TypedOutput::Comp(Comp::split(
                scrut, &fst_name, &snd_name, body,
            )));
        },
        | TypedFrame::CompSplitMotive {
            fst_name,
            snd_name,
            motive,
        } => {
            let body = pop_typed_comp(outputs);
            let scrut = pop_typed_value(outputs);
            outputs.push(TypedOutput::Comp(Comp::split_motive(
                scrut,
                &fst_name,
                &snd_name,
                SplitMotive::new("mtv", motive),
                body,
            )));
        },
        | TypedFrame::CompWith => {
            let snd = pop_typed_comp(outputs);
            let fst = pop_typed_comp(outputs);
            outputs.push(TypedOutput::Comp(Comp::with(fst, snd)));
        },
        | TypedFrame::CompPrj(first) => {
            let target = pop_typed_comp(outputs);
            let comp = if first {
                Comp::prj1(target)
            }
            else {
                Comp::prj2(target)
            };
            outputs.push(TypedOutput::Comp(comp));
        },
        | TypedFrame::CompRecordProj(label) => {
            let record = pop_typed_value(outputs);
            outputs.push(TypedOutput::Comp(Comp::record_proj(record, &label)));
        },
    }
}

/// Strategies for grades that admit at least one force (`1 ⊑ r`).
fn forceable_grade() -> impl Strategy<Value = Grade>
{
    prop_oneof![
        Just(Grade::ONE),
        Just(Grade::fin(gandr_core_checker::boundary::GradeBound::from(
            2_u64
        ))),
        Just(Grade::OMEGA)
    ]
}

struct FreeGenerators
{
    comp: BoxedStrategy<Comp>,
    value: BoxedStrategy<Value>,
    effectful_comp: BoxedStrategy<Comp>,
}

fn free_value_leaves() -> BoxedStrategy<Value>
{
    prop_oneof![
        Just(Value::Unit),
        proptest::num::i64::ANY.prop_map(Value::int),
        Just(Value::string("")),
        Just(Value::string("hello world")),
        // Suffixed numeric literals (ADR-39): one per polarity of carrier so
        // the agreement oracle exercises the `Value::Num` arm and the bitwise
        // float storage.
        proptest::num::u32::ANY.prop_map(Value::u32),
        proptest::num::i64::ANY.prop_map(Value::i64),
        proptest::num::f64::ANY.prop_map(Value::f64),
        hole_id().prop_map(Value::Hole),
        Just(Value::var("i")),
        Just(Value::var("s")),
        Just(Value::var("ghost")),
        // Binder-introduced names: under an enclosing `λx`/`bind x`/… these
        // resolve; elsewhere they are `UnboundVariable`. Either way both
        // implementations must agree.
        Just(Value::var("x")),
        Just(Value::var("y")),
        Just(Value::var("z")),
    ]
    .boxed()
}

fn free_comp_leaves(value_leaves: BoxedStrategy<Value>) -> BoxedStrategy<Comp>
{
    prop_oneof![
        value_leaves.clone().prop_map(Comp::ret),
        value_leaves.prop_map(Comp::force),
        hole_id().prop_map(Comp::Hole),
    ]
    .boxed()
}

fn free_perform_from(value: BoxedStrategy<Value>) -> BoxedStrategy<Comp>
{
    (arb_effect_sig(), op_name_pool(), value)
        .prop_map(|(sig, op, arg)| Comp::perform(sig, &op, arg))
        .boxed()
}

fn free_op_clause_from(comp: BoxedStrategy<Comp>) -> BoxedStrategy<OpClause>
{
    (op_name_pool(), binder_name(), binder_name(), comp)
        .prop_map(|(op, payload, resume, body)| OpClause::new(&op, &payload, &resume, body))
        .boxed()
}

fn free_handle_from(
    comp: BoxedStrategy<Comp>,
    op_clause: BoxedStrategy<OpClause>,
) -> BoxedStrategy<Comp>
{
    (
        arb_effect_sig(),
        comp.clone(),
        binder_name(),
        comp,
        proptest::collection::vec(op_clause, 0 ..= 2),
    )
        .prop_map(|(sig, scrutinee, ret_var, ret_body, ops)| {
            Comp::handle(sig, scrutinee, &ret_var, ret_body, ops)
        })
        .boxed()
}

fn free_generators<D>(depth: D) -> FreeGenerators
where
    D: Into<GenerationDepth>,
{
    let depth = u32::from(depth.into());
    let value_leaves = free_value_leaves();
    let comp_leaves = free_comp_leaves(value_leaves.clone());
    let mut value = value_leaves.clone();
    let mut stack = Just(Stack::empty()).boxed();
    let mut comp = comp_leaves.clone();
    let mut perform = free_perform_from(value.clone());
    let mut op_clause = free_op_clause_from(comp.clone());
    let mut handle = free_handle_from(comp.clone(), op_clause.clone());
    let mut effectful_comp = arb_effectful_perform();

    for _ in 0 .. depth {
        let sub_value = value.clone();
        let sub_stack = stack.clone();
        let sub_comp = comp.clone();
        let sub_perform = perform.clone();
        let sub_handle = handle.clone();
        let sub_effectful_comp = effectful_comp.clone();

        stack = prop_oneof![
            Just(Stack::empty()),
            (sub_value.clone(), sub_stack.clone()).prop_map(|(arg, rest)| Stack::arg(arg, rest)),
            (binder_name(), sub_comp.clone(), sub_stack.clone())
                .prop_map(|(name, cont, rest)| Stack::bind(&name, cont, rest)),
            sub_stack.clone().prop_map(Stack::prj1),
            sub_stack.prop_map(Stack::prj2),
        ]
        .boxed();

        value = prop_oneof![
            value_leaves.clone(),
            (sub_value.clone(), sub_value.clone()).prop_map(|(fst, snd)| Value::pair(fst, snd)),
            sub_value.clone().prop_map(Value::inj1),
            sub_value.clone().prop_map(Value::inj2),
            // A free list literal `[…]` (ADR-40): mostly stuck under the random
            // direction (check-only), and against a `List A` expectation it
            // drives the per-element checks — under checker≡machine agreement.
            proptest::collection::vec(sub_value.clone(), 0 ..= 3).prop_map(Value::list),
            // A free record literal `#{…}` (ADR-45): direction-polymorphic, so
            // it drives per-field infer/check and width/depth subsumption.
            proptest::collection::btree_map(record_label(), sub_value.clone(), 0 ..= 3)
                .prop_map(Value::record),
            (any_grade(), sub_comp.clone()).prop_map(|(grade, body)| Value::thunk(grade, body)),
            (sub_value, arb_value_type(1_u32)).prop_map(|(inner, ty)| Value::annot(inner, ty)),
            // A3.3 `+control`: a reified stack `stk K` over a free stack.
            stack.clone().prop_map(Value::stk),
        ]
        .boxed();

        comp = prop_oneof![
            comp_leaves.clone(),
            // Forcing a thunk *literal*: reaches the force grade side condition,
            // including the `r = 0` failure path.
            (any_grade(), sub_comp.clone())
                .prop_map(|(grade, body)| Comp::force(Value::thunk(grade, body))),
            (binder_name(), sub_comp.clone()).prop_map(|(name, body)| Comp::lam(&name, body)),
            (binder_name(), arb_value_type(1_u32), sub_comp.clone())
                .prop_map(|(name, ty, body)| Comp::lam_ann(&name, ty, body)),
            (sub_comp.clone(), value.clone()).prop_map(|(head, arg)| Comp::app(head, arg)),
            (sub_comp.clone(), binder_name(), sub_comp.clone())
                .prop_map(|(bound, name, cont)| Comp::bind(bound, &name, cont)),
            (
                value.clone(),
                binder_name(),
                sub_comp.clone(),
                binder_name(),
                sub_comp.clone()
            )
                .prop_map(|(scrut, fst_name, fst_body, snd_name, snd_body)| {
                    Comp::case(scrut, &fst_name, fst_body, &snd_name, snd_body)
                }),
            (
                value.clone(),
                sub_comp.clone(),
                binder_name(),
                binder_name(),
                sub_comp.clone()
            )
                .prop_map(|(scrut, nil, head, tail, cons)| {
                    Comp::list_case(scrut, nil, &head, &tail, cons)
                }),
            (
                value.clone(),
                binder_name(),
                binder_name(),
                sub_comp.clone()
            )
                .prop_map(|(scrut, fst_name, snd_name, body)| {
                    Comp::split(scrut, &fst_name, &snd_name, body)
                }),
            (sub_comp.clone(), sub_comp.clone()).prop_map(|(fst, snd)| Comp::with(fst, snd)),
            sub_comp.clone().prop_map(Comp::prj1),
            sub_comp.clone().prop_map(Comp::prj2),
            (value.clone(), record_label())
                .prop_map(|(record, label)| { Comp::record_proj(record, &label) }),
            value.clone().prop_map(Comp::dup),
            value.clone().prop_map(Comp::drop),
            (any_grade(), sub_comp.clone())
                .prop_map(|(grade, body)| Comp::drop(Value::thunk(grade, body))),
            sub_perform,
            sub_handle,
            (value.clone(), sub_comp.clone()).prop_map(|(resumed_stack, resumed_comp)| {
                Comp::resume(resumed_stack, resumed_comp)
            }),
            sub_comp.clone().prop_map(Comp::reset),
            (binder_name(), sub_comp).prop_map(|(k, body)| Comp::shift(&k, body)),
        ]
        .boxed();

        perform = free_perform_from(value.clone());
        op_clause = free_op_clause_from(comp.clone());
        handle = free_handle_from(comp.clone(), op_clause.clone());
        effectful_comp = prop_oneof![
            arb_effectful_perform(),
            sub_effectful_comp.clone().prop_map(|bound| Comp::bind(
                bound,
                "x",
                Comp::ret(Value::var("x"))
            )),
            (sub_effectful_comp.clone(), sub_effectful_comp)
                .prop_map(|(bound, tail)| Comp::bind(bound, "x", tail)),
        ]
        .boxed();
    }

    FreeGenerators {
        comp,
        value,
        effectful_comp,
    }
}

/// A pool of operation names — some declared by the pool signatures
/// (`get`/`put`/`print`), one (`absent`) declared by none — so a free
/// `perform`/`handle` exercises both the resolved and the unknown-op paths.
fn op_name_pool() -> impl Strategy<Value = String>
{
    prop_oneof![
        Just("get".to_owned()),
        Just("put".to_owned()),
        Just("print".to_owned()),
        Just("absent".to_owned()),
    ]
}

/// A small pool of effect signatures for the free generators (A3.2
/// `+effects`): the fixed `State` / `IO` / `Empty` signatures.
fn arb_effect_sig() -> impl Strategy<Value = EffectSig>
{
    prop_oneof![Just(state_sig()), Just(io_sig()), Just(empty_sig()),]
}

/// The empty effect signature `Empty { }`: a zero-operation handler exercises
/// the empty op-clause fold (the handle finishes straight after the return
/// clause).
fn empty_sig() -> EffectSig
{
    EffectSig::new(
        gandr_core_checker::boundary::EffectSignatureName::from("Empty"),
        Vec::new(),
    )
}

/// Free (mostly ill-typed) computation terms.
/// # Termination
/// - reason: `free_generators` folds finite depth layers on the heap.
/// - measure: remaining depth layers in the iterative fold.
/// - boundedness: finite `u32` fuel reaches the leaf layer at zero.
/// - input recursion: none.
fn arb_comp<D>(depth: D) -> BoxedStrategy<Comp>
where
    D: Into<GenerationDepth>,
{
    free_generators(u32::from(depth.into())).comp
}

/// Free (mostly ill-typed) value terms.
/// # Termination
/// - reason: `free_generators` folds finite depth layers on the heap.
/// - measure: remaining depth layers in the iterative fold.
/// - boundedness: finite `u32` fuel reaches the leaf layer at zero.
/// - input recursion: none.
fn arb_value<D>(depth: D) -> BoxedStrategy<Value>
where
    D: Into<GenerationDepth>,
{
    free_generators(u32::from(depth.into())).value
}

/// An arbitrary direction for computations.
fn arb_comp_dir() -> BoxedStrategy<Dir<CompType>>
{
    prop_oneof![Just(Dir::Infer), arb_comp_type(2_u32).prop_map(Dir::Check)].boxed()
}

/// Effectful, **inferring** computations: `perform`s of the pool signatures,
/// sequenced by `bind`. The two recursive shapes are exactly the ones the
/// A3.2 bottom-up row arithmetic must get right — a bind into a *pure* tail
/// (`ret x`), whose bound row must survive into the answer (the hole let it
/// escape), and a bind into another effectful computation, whose answer
/// accumulates the *union* of both rows. Every term infers `F^ε A` with a
/// non-empty `ε`, so the [`comp_near_misses`] row-strip always applies — the
/// free [`arb_comp`] generator produces this shape far too rarely to gate on.
/// # Termination
/// - reason: `free_generators` folds finite depth layers on the heap.
/// - measure: remaining depth layers in the iterative fold.
/// - boundedness: finite `u32` fuel reaches the leaf layer at zero.
/// - input recursion: none.
fn arb_effectful_comp<D>(depth: D) -> BoxedStrategy<Comp>
where
    D: Into<GenerationDepth>,
{
    free_generators(u32::from(depth.into())).effectful_comp
}

/// A `perform` guaranteed to **infer** an effectful returner: a declared
/// operation of a pool signature on a well-typed payload (`State.get ()`,
/// `State.put n`, `IO.print n`). Unlike [`arb_perform`] — whose arbitrary
/// payloads are mostly mistyped, so it rarely infers at all — every term here
/// yields `F^⟨E⟩ B_op`, the non-empty effect row the coherence oracle needs.
fn arb_effectful_perform() -> BoxedStrategy<Comp>
{
    prop_oneof![
        Just(Comp::perform(state_sig(), "get", Value::Unit)),
        proptest::num::i64::ANY.prop_map(|n| Comp::perform(state_sig(), "put", Value::int(n))),
        proptest::num::i64::ANY.prop_map(|n| Comp::perform(io_sig(), "print", Value::int(n))),
    ]
    .boxed()
}

/// A bare integer literal paired with a sized-int atom it is **representable
/// in** (ADR-39 D4): the biased generator for the integer-literal-defaulting
/// coherence class. Every pair drives the check-mode defaulting rule (the
/// literal fits, so check succeeds and the inferred `Integer` is coherent with
/// the checked atom) — the exact shape the free `arb_value` / `arb_comp` cross
/// reaches only by accident, and the shape the adversarial pass flaked on.
fn arb_int_literal_and_sized_atom() -> BoxedStrategy<(IntegerLiteral, ValueType)>
{
    prop_oneof![
        (0_i64 ..= i64::from(u32::MAX)).prop_map(|n| (IntegerLiteral::from(n), ValueType::u32())),
        (i64::from(i32::MIN) ..= i64::from(i32::MAX))
            .prop_map(|n| (IntegerLiteral::from(n), ValueType::i32())),
        proptest::num::i64::ANY.prop_map(|n| (IntegerLiteral::from(n), ValueType::i64())),
        (0_i64 ..= i64::MAX).prop_map(|n| (IntegerLiteral::from(n), ValueType::u64())),
    ]
    .boxed()
}

/// Grade-tightened **near-miss** answers for the value coherence oracle: a
/// thunk type demanding a *larger* budget than inferred (`U_r B → U_ω B`,
/// sound only when `ω ⊑ r`, i.e. `r = ω`). The grade analogue of
/// [`comp_near_misses`]. Empty for a non-thunk or already-`ω` `a_prime`.
fn value_near_misses(a_prime: &ValueType) -> Vec<ValueType>
{
    let mut misses = Vec::new();
    if let ValueType::Thunk(grade, ref body) = *a_prime
        && grade != Grade::OMEGA
    {
        misses.push(ValueType::thunk(Grade::OMEGA, body.as_ref().clone()));
    }
    misses
}

enum CoherenceGoal<'ty>
{
    Value(&'ty ValueType, &'ty ValueType),
    Comp(&'ty CompType, &'ty CompType),
}

fn coherence_subtype_from(root: CoherenceGoal<'_>) -> CoherenceDecision
{
    let mut pending = vec![root];
    while let Some(goal) = pending.pop() {
        match goal {
            | CoherenceGoal::Value(sub, sup) => {
                // Reflexive pointer-equality short-circuit (ADR-50 Decision B):
                // reflexivity holds for the coherence relation too, so this is
                // a pure optimization.
                if core::ptr::eq(sub, sup)
                    || matches!(*sub, ValueType::Unknown)
                    || matches!(*sup, ValueType::Unknown)
                {
                    continue;
                }
                match (sub, sup) {
                    // The one relaxation: the inferred default `Integer` is
                    // coherent with a sized-int atom (ADR-39 D4). The atom set
                    // mirrors `subtype::int_literal_fits` (float atoms excluded
                    // — a bare float infers the monomorphic `f64`).
                    | (&ValueType::Atom(ref lhs), &ValueType::Atom(ref rhs)) => {
                        if lhs != rhs
                            && !(lhs.as_str() == "Integer"
                                && matches!(rhs.as_str(), "u32" | "u64" | "i32" | "i64"))
                        {
                            return false.into();
                        }
                    },
                    | (&ValueType::Unit, &ValueType::Unit) => {},
                    | (
                        &ValueType::Prod(ref lo_fst, ref lo_snd),
                        &ValueType::Prod(ref hi_fst, ref hi_snd),
                    )
                    | (
                        &ValueType::Sum(ref lo_fst, ref lo_snd),
                        &ValueType::Sum(ref hi_fst, ref hi_snd),
                    ) => {
                        pending.push(CoherenceGoal::Value(lo_fst, hi_fst));
                        pending.push(CoherenceGoal::Value(lo_snd, hi_snd));
                    },
                    | (&ValueType::List(ref lo_elem), &ValueType::List(ref hi_elem)) => {
                        pending.push(CoherenceGoal::Value(lo_elem, hi_elem));
                    },
                    | (&ValueType::Record(ref lo_fields), &ValueType::Record(ref hi_fields)) => {
                        for (label, hi_ty) in hi_fields {
                            let Some(lo_ty) = lo_fields.get(label)
                            else {
                                return false.into();
                            };
                            pending.push(CoherenceGoal::Value(lo_ty, hi_ty));
                        }
                    },
                    | (
                        &ValueType::Thunk(lo_grade, ref lo_body),
                        &ValueType::Thunk(hi_grade, ref hi_body),
                    ) => {
                        // Grade leg STRICT (its own companion guards it); body
                        // covariant.
                        if !bool::from(hi_grade.leq(lo_grade)) {
                            return false.into();
                        }
                        pending.push(CoherenceGoal::Comp(lo_body, hi_body));
                    },
                    | (
                        &ValueType::Stk(ref lo_b, ref lo_c),
                        &ValueType::Stk(ref hi_b, ref hi_c),
                    ) => {
                        // Consumed `B` contravariant → STRICT; delivered `C`
                        // covariant.
                        if !bool::from(comp_subtype(hi_b, lo_b)) {
                            return false.into();
                        }
                        pending.push(CoherenceGoal::Comp(lo_c, hi_c));
                    },
                    // Reflexive same-constructor arms above keep the
                    // `core::ptr::eq` short-circuit a PURE optimization for the
                    // coherence relation too; a future variant must add one
                    // (see the `gandr_core_checker::subtype::value_subtype` note it mirrors).
                    | _ => return false.into(),
                }
            },
            | CoherenceGoal::Comp(sub, sup) => {
                // Reflexive pointer-equality short-circuit (ADR-50 Decision B),
                // as `coherence_value_subtype`: pure optimization on a
                // reflexive relation.
                if core::ptr::eq(sub, sup)
                    || matches!(*sub, CompType::Unknown)
                    || matches!(*sup, CompType::Unknown)
                {
                    continue;
                }
                match (sub, sup) {
                    | (
                        &CompType::F(ref lo_of, ref lo_row),
                        &CompType::F(ref hi_of, ref hi_row),
                    ) => {
                        // Payload covariant → relaxed; effect-row leg STRICT
                        // (A3.2 guard).
                        if !bool::from(lo_row.is_subset(hi_row)) {
                            return false.into();
                        }
                        pending.push(CoherenceGoal::Value(lo_of, hi_of));
                    },
                    | (
                        &CompType::Arrow(ref lo_arg, ref lo_res),
                        &CompType::Arrow(ref hi_arg, ref hi_res),
                    ) => {
                        // Argument contravariant → STRICT; result covariant →
                        // relaxed.
                        if !bool::from(value_subtype(hi_arg, lo_arg)) {
                            return false.into();
                        }
                        pending.push(CoherenceGoal::Comp(lo_res, hi_res));
                    },
                    | (
                        &CompType::With(ref lo_fst, ref lo_snd),
                        &CompType::With(ref hi_fst, ref hi_snd),
                    ) => {
                        pending.push(CoherenceGoal::Comp(lo_fst, hi_fst));
                        pending.push(CoherenceGoal::Comp(lo_snd, hi_snd));
                    },
                    // Reflexive same-constructor arms keep the `core::ptr::eq`
                    // short-circuit a PURE optimization; a future variant must
                    // add one (see
                    // `coherence_value_subtype` and `gandr_core_checker::subtype::comp_subtype`).
                    | _ => return false.into(),
                }
            },
        }
    }
    true.into()
}

/// The computation-sort analogue of [`coherence_value_subtype`] (ADR-48):
/// [`comp_subtype`] with the covariant `Integer ⊑ sized-int` relaxation carried
/// into the payload of `F` and the result of `→`, while the effect-row leg (`ε
/// ⊆ ε′`) and the contravariant arrow argument stay strict.
/// # Termination
/// - reason: the helper drains an explicit finite comparison worklist.
/// - measure: pending type-pair comparison goals.
/// - boundedness: compared types are finite Rust values.
/// - input recursion: none.
fn coherence_comp_subtype(
    sub: &CompType,
    sup: &CompType,
) -> CoherenceDecision
{
    coherence_subtype_from(CoherenceGoal::Comp(sub, sup))
}

enum RebuildStep<'ty>
{
    Value(&'ty ValueType),
    Comp(&'ty CompType),
    Frame(RebuildFrame),
}

enum RebuildFrame
{
    ValueProd,
    ValueSum,
    ValueList,
    ValueRecord(Vec<String>),
    ValueThunk(Grade),
    ValueStk,
    ValuePath
    {
        lhs: Value,
        rhs: Value,
    },
    ValueData
    {
        id: gandr_core_checker::types::DataId,
        arity: usize,
    },
    ValueSigma(String),
    ValuePackage(Grade, Vec<String>),
    CompF(EffectRow),
    CompArrow,
    CompWith,
}

enum RebuildOutput
{
    Value(ValueType),
    Comp(CompType),
}

fn pop_rebuilt_value(outputs: &mut Vec<RebuildOutput>) -> ValueType
{
    match outputs.pop() {
        | Some(RebuildOutput::Value(value)) => value,
        | _ => panic!("rebuild frame expected a value type"),
    }
}

fn pop_rebuilt_comp(outputs: &mut Vec<RebuildOutput>) -> CompType
{
    match outputs.pop() {
        | Some(RebuildOutput::Comp(comp)) => comp,
        | _ => panic!("rebuild frame expected a computation type"),
    }
}

fn rebuild_type_from(root: RebuildStep<'_>) -> RebuildOutput
{
    let mut steps = vec![root];
    let mut outputs = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            | RebuildStep::Value(ty) => match *ty {
                | ValueType::Atom(ref name) => {
                    outputs.push(RebuildOutput::Value(ValueType::Atom(name.clone())));
                },
                | ValueType::Unit => outputs.push(RebuildOutput::Value(ValueType::Unit)),
                | ValueType::Sealed(ref seal) => {
                    outputs.push(RebuildOutput::Value(ValueType::Sealed(seal.clone())));
                },
                | ValueType::Prod(ref fst, ref snd) => {
                    steps.push(RebuildStep::Frame(RebuildFrame::ValueProd));
                    steps.push(RebuildStep::Value(snd));
                    steps.push(RebuildStep::Value(fst));
                },
                | ValueType::Sum(ref lhs, ref rhs) => {
                    steps.push(RebuildStep::Frame(RebuildFrame::ValueSum));
                    steps.push(RebuildStep::Value(rhs));
                    steps.push(RebuildStep::Value(lhs));
                },
                | ValueType::List(ref elem) => {
                    steps.push(RebuildStep::Frame(RebuildFrame::ValueList));
                    steps.push(RebuildStep::Value(elem));
                },
                | ValueType::Record(ref fields) => {
                    let labels = fields.keys().cloned().collect::<Vec<_>>();
                    steps.push(RebuildStep::Frame(RebuildFrame::ValueRecord(labels)));
                    for field_ty in fields.values().rev() {
                        steps.push(RebuildStep::Value(field_ty));
                    }
                },
                | ValueType::Thunk(grade, ref body) => {
                    steps.push(RebuildStep::Frame(RebuildFrame::ValueThunk(grade)));
                    steps.push(RebuildStep::Comp(body));
                },
                | ValueType::Stk(ref consumes, ref delivers) => {
                    steps.push(RebuildStep::Frame(RebuildFrame::ValueStk));
                    steps.push(RebuildStep::Comp(delivers));
                    steps.push(RebuildStep::Comp(consumes));
                },
                // Rebuild the identity type with fresh `Rc`s at the carrier and
                // both endpoints, so the reflexivity oracle descends
                // structurally into the `Path` arm rather than short-circuiting
                // on `core::ptr::eq` (ADR-76).
                | ValueType::Path {
                    ty: ref carrier,
                    ref lhs,
                    ref rhs,
                } => {
                    steps.push(RebuildStep::Frame(RebuildFrame::ValuePath {
                        lhs: lhs.as_ref().clone(),
                        rhs: rhs.as_ref().clone(),
                    }));
                    steps.push(RebuildStep::Value(carrier));
                },
                // Rebuild the declared-data handle with fresh `Rc`s at every
                // type argument, so the reflexivity oracle descends structurally
                // into the `Data` arm rather than short-circuiting on
                // `core::ptr::eq` (ADR-80).
                | ValueType::Data { ref id, ref args } => {
                    steps.push(RebuildStep::Frame(RebuildFrame::ValueData {
                        id: id.clone(),
                        arity: args.len(),
                    }));
                    for arg in args.iter().rev() {
                        steps.push(RebuildStep::Value(arg));
                    }
                },
                // The code universe has no children (ADR-81). The dependent
                // pair rebuilds head and tail with fresh `Rc`s so the
                // reflexivity oracle descends structurally into the `Sigma` arm
                // rather than short-circuiting on `core::ptr::eq`.
                | ValueType::Universe => outputs.push(RebuildOutput::Value(ValueType::Universe)),
                | ValueType::Sigma {
                    ref fst,
                    ref binder,
                    ref snd,
                } => {
                    steps.push(RebuildStep::Frame(RebuildFrame::ValueSigma(binder.clone())));
                    steps.push(RebuildStep::Value(snd));
                    steps.push(RebuildStep::Value(fst));
                },
                // A package rebuilds its payload with a fresh `Rc` for the same
                // reason the dependent pair does: so the reflexivity oracle
                // descends structurally rather than short-circuiting on
                // `core::ptr::eq`.
                | ValueType::Package {
                    grade,
                    ref abstracts,
                    ref payload,
                } => {
                    steps.push(RebuildStep::Frame(RebuildFrame::ValuePackage(
                        grade,
                        abstracts.clone(),
                    )));
                    steps.push(RebuildStep::Value(payload));
                },
                | ValueType::Unknown => outputs.push(RebuildOutput::Value(ValueType::Unknown)),
            },
            | RebuildStep::Comp(ty) => match *ty {
                | CompType::F(ref of, ref row) => {
                    steps.push(RebuildStep::Frame(RebuildFrame::CompF(row.clone())));
                    steps.push(RebuildStep::Value(of));
                },
                | CompType::Arrow(ref arg, ref res) => {
                    steps.push(RebuildStep::Frame(RebuildFrame::CompArrow));
                    steps.push(RebuildStep::Comp(res));
                    steps.push(RebuildStep::Value(arg));
                },
                | CompType::With(ref fst, ref snd) => {
                    steps.push(RebuildStep::Frame(RebuildFrame::CompWith));
                    steps.push(RebuildStep::Comp(snd));
                    steps.push(RebuildStep::Comp(fst));
                },
                | CompType::Unknown => outputs.push(RebuildOutput::Comp(CompType::Unknown)),
            },
            | RebuildStep::Frame(frame) => match frame {
                | RebuildFrame::ValueProd => {
                    let snd = pop_rebuilt_value(&mut outputs);
                    let fst = pop_rebuilt_value(&mut outputs);
                    outputs.push(RebuildOutput::Value(ValueType::Prod(
                        Rc::new(fst),
                        Rc::new(snd),
                    )));
                },
                | RebuildFrame::ValueSum => {
                    let rhs = pop_rebuilt_value(&mut outputs);
                    let lhs = pop_rebuilt_value(&mut outputs);
                    outputs.push(RebuildOutput::Value(ValueType::Sum(
                        Rc::new(lhs),
                        Rc::new(rhs),
                    )));
                },
                | RebuildFrame::ValueList => {
                    let elem = pop_rebuilt_value(&mut outputs);
                    outputs.push(RebuildOutput::Value(ValueType::List(Rc::new(elem))));
                },
                | RebuildFrame::ValueRecord(labels) => {
                    let mut values = Vec::with_capacity(labels.len());
                    for _ in 0 .. labels.len() {
                        values.push(pop_rebuilt_value(&mut outputs));
                    }
                    values.reverse();
                    outputs.push(RebuildOutput::Value(ValueType::Record(
                        labels
                            .into_iter()
                            .zip(values)
                            .map(|(label, value)| (label, Rc::new(value)))
                            .collect(),
                    )));
                },
                | RebuildFrame::ValueThunk(grade) => {
                    let body = pop_rebuilt_comp(&mut outputs);
                    outputs.push(RebuildOutput::Value(ValueType::Thunk(grade, Rc::new(body))));
                },
                | RebuildFrame::ValueStk => {
                    let delivers = pop_rebuilt_comp(&mut outputs);
                    let consumes = pop_rebuilt_comp(&mut outputs);
                    outputs.push(RebuildOutput::Value(ValueType::Stk(
                        Rc::new(consumes),
                        Rc::new(delivers),
                    )));
                },
                | RebuildFrame::ValuePath { lhs, rhs } => {
                    let carrier = pop_rebuilt_value(&mut outputs);
                    outputs.push(RebuildOutput::Value(ValueType::Path {
                        ty: Rc::new(carrier),
                        lhs: Rc::new(lhs),
                        rhs: Rc::new(rhs),
                    }));
                },
                | RebuildFrame::ValueData { id, arity } => {
                    let mut args = Vec::with_capacity(arity);
                    for _ in 0 .. arity {
                        args.push(pop_rebuilt_value(&mut outputs));
                    }
                    args.reverse();
                    outputs.push(RebuildOutput::Value(ValueType::Data {
                        id,
                        args: args.into_iter().map(Rc::new).collect(),
                    }));
                },
                | RebuildFrame::ValuePackage(grade, abstracts) => {
                    let payload = pop_rebuilt_value(&mut outputs);
                    outputs.push(RebuildOutput::Value(ValueType::Package {
                        grade,
                        abstracts,
                        payload: Rc::new(payload),
                    }));
                },
                | RebuildFrame::ValueSigma(binder) => {
                    let snd = pop_rebuilt_value(&mut outputs);
                    let fst = pop_rebuilt_value(&mut outputs);
                    outputs.push(RebuildOutput::Value(ValueType::Sigma {
                        fst: Rc::new(fst),
                        binder,
                        snd: Rc::new(snd),
                    }));
                },
                | RebuildFrame::CompF(row) => {
                    let of = pop_rebuilt_value(&mut outputs);
                    outputs.push(RebuildOutput::Comp(CompType::F(Rc::new(of), row)));
                },
                | RebuildFrame::CompArrow => {
                    let res = pop_rebuilt_comp(&mut outputs);
                    let arg = pop_rebuilt_value(&mut outputs);
                    outputs.push(RebuildOutput::Comp(CompType::Arrow(
                        Rc::new(arg),
                        Rc::new(res),
                    )));
                },
                | RebuildFrame::CompWith => {
                    let snd = pop_rebuilt_comp(&mut outputs);
                    let fst = pop_rebuilt_comp(&mut outputs);
                    outputs.push(RebuildOutput::Comp(CompType::With(
                        Rc::new(fst),
                        Rc::new(snd),
                    )));
                },
            },
        }
    }
    outputs.pop().expect("rebuild produced a root type")
}

/// The computation-sort analogue of [`deep_rebuild_value`]: rebuilds a
/// [`CompType`] with a fresh [`Rc`] at every interior node. See
/// [`deep_rebuild_value`] for why the reflexivity oracles need it.
/// # Termination
/// - reason: the helper drains explicit rebuild tasks and result frames.
/// - measure: pending rebuild tasks and result frames.
/// - boundedness: source types are finite Rust values.
/// - input recursion: none.
fn deep_rebuild_comp(ty: &CompType) -> CompType
{
    match rebuild_type_from(RebuildStep::Comp(ty)) {
        | RebuildOutput::Comp(comp) => comp,
        | RebuildOutput::Value(_) => panic!("comp rebuild produced a value type"),
    }
}
/// Sanity-check for [`deep_rebuild_value`]: the rebuilt copy must share NO
/// child address with the original, so the reflexivity oracles genuinely
/// descend structurally (a plain `.clone()` would leave the children aliased
/// and let the `core::ptr::eq` short-circuit pre-empt the descent).
#[test]
fn deep_rebuild_value_yields_distinct_child_addresses()
{
    // `Prod(List(Atom "A"), Thunk(F(Atom "B")))` — a nested type touching a
    // value child, a value grandchild, and a computation child.
    let original = ValueType::prod(
        ValueType::list(ValueType::atom("A")),
        ValueType::thunk(Grade::OMEGA, CompType::returner(ValueType::atom("B"))),
    );
    let rebuilt = deep_rebuild_value(&original);
    // Structurally identical (derived `PartialEq` compares by value).
    assert_eq!(original, rebuilt);
    // Yet every interior `Rc` is a fresh allocation: extract the two `Prod`
    // children from each and confirm no address is shared. The `match (a, b)`
    // idiom (mirroring `gandr_core_checker::subtype`) keeps the explicit reference
    // patterns clippy demands without tripping `needless_borrowed_reference`.
    match (&original, &rebuilt) {
        | (&ValueType::Prod(ref o_fst, ref o_snd), &ValueType::Prod(ref r_fst, ref r_snd)) => {
            assert!(!Rc::ptr_eq(o_fst, r_fst), "first Prod child shares an Rc");
            assert!(!Rc::ptr_eq(o_snd, r_snd), "second Prod child shares an Rc");
            // Descend into the `List` grandchild to confirm freshness is deep.
            match (o_fst.as_ref(), r_fst.as_ref()) {
                | (&ValueType::List(ref o_elem), &ValueType::List(ref r_elem)) => {
                    assert!(!Rc::ptr_eq(o_elem, r_elem), "List element shares an Rc");
                },
                | _ => panic!("expected a List first child"),
            }
        },
        | _ => panic!("deep_rebuild changed the top constructor"),
    }
}

/// Rebuilds a [`ValueType`] with a FRESH [`Rc`] at every interior node, so no
/// child of the result shares an address with the original.
///
/// A plain `.clone()` is insufficient for exercising structural reflexivity:
/// the derived [`Clone`] merely bumps each [`Rc`] refcount, leaving every child
/// address-identical to the source. The `core::ptr::eq(sub, sup)` short-circuit
/// at the top of [`value_subtype`] / [`comp_subtype`] would then fire on the
/// first shared child and preempt structural descent — turning a reflexivity
/// oracle into a tautology. Rebuilding with fresh allocations gives the oracle
/// distinct addresses at every node, forcing the structural path.
///
/// Test-only, over proptest-bounded types (`arb_value_type` / `arb_comp_type`
/// depth ≤ 3), but still heap-worklisted so the termination contract remains
/// true if a regression witness grows deeper.
/// # Termination
/// - reason: the helper drains explicit rebuild tasks and result frames.
/// - measure: pending rebuild tasks and result frames.
/// - boundedness: source types are finite Rust values.
/// - input recursion: none.
fn deep_rebuild_value(ty: &ValueType) -> ValueType
{
    match rebuild_type_from(RebuildStep::Value(ty)) {
        | RebuildOutput::Value(value) => value,
        | RebuildOutput::Comp(_) => panic!("value rebuild produced a computation type"),
    }
}
/// SOUNDNESS-ORACLE-COMPANION
/// The deterministic biased companion (ADR-48) to
/// [`coherence_value_subtype_is_reflexive`]: one witness per [`ValueType`]
/// constructor — atom (including the `Integer` relaxation source and a
/// sized-int target), unit, product, sum, list, record, graded thunks across
/// the grade classes (`0` / `1` / finite / `ω`), reified stack, and the hole —
/// plus a deeply nested combination crossing both sorts (row-carrying `F`
/// under a record-field thunk, `Unknown` in covariant and contravariant
/// positions). Each is related to its [`deep_rebuild_value`] copy, whose fresh
/// `Rc`s keep the `core::ptr::eq` short-circuit from pre-empting the arm under
/// test. A future `ValueType` variant whose reflexive arm lands in
/// [`gandr_core_checker::subtype::value_subtype`] but is omitted from the
/// coherence copy fails here every run, without relying on the free generator
/// reaching the new constructor.
#[test]
fn coherence_value_subtype_reflexivity_arm_sweep()
{
    let witnesses = [
        ValueType::atom("A"),
        ValueType::integer(),
        ValueType::u32(),
        ValueType::Unit,
        ValueType::prod(ValueType::integer(), ValueType::string()),
        ValueType::sum(ValueType::Unit, ValueType::integer()),
        ValueType::list(ValueType::string()),
        ValueType::record([
            ("a".to_owned(), ValueType::integer()),
            ("b".to_owned(), ValueType::list(ValueType::Unit)),
        ]),
        ValueType::thunk(Grade::ZERO, CompType::returner(ValueType::Unit)),
        ValueType::thunk(Grade::ONE, CompType::returner(ValueType::integer())),
        ValueType::thunk(
            Grade::fin(gandr_core_checker::boundary::GradeBound::from(2)),
            CompType::returner(ValueType::integer()),
        ),
        ValueType::thunk(
            Grade::OMEGA,
            CompType::arrow(ValueType::integer(), CompType::returner(ValueType::Unit)),
        ),
        ValueType::stk(
            CompType::returner(ValueType::integer()),
            CompType::returner(ValueType::Unit),
        ),
        ValueType::Unknown,
        ValueType::prod(
            ValueType::record([(
                "f".to_owned(),
                ValueType::thunk(
                    Grade::OMEGA,
                    CompType::with(
                        CompType::returner_eff(
                            ValueType::integer(),
                            EffectRow::singleton(state_sig()),
                        ),
                        CompType::arrow(
                            ValueType::sum(ValueType::Unknown, ValueType::Unit),
                            CompType::returner(ValueType::list(ValueType::string())),
                        ),
                    ),
                ),
            )]),
            ValueType::stk(CompType::Unknown, CompType::returner(ValueType::Unit)),
        ),
    ];
    for ty in &witnesses {
        assert!(
            bool::from(coherence_value_subtype(ty, &deep_rebuild_value(ty))),
            "coherence reflexivity failed on the arm of {ty:?}"
        );
    }
}
/// SOUNDNESS-ORACLE-COMPANION
/// The computation-sort deterministic biased companion (ADR-48) to
/// [`coherence_comp_subtype_is_reflexive`]: one witness per [`CompType`]
/// constructor — pure and row-carrying `F`, arrow, with, and the hole — plus a
/// deeply nested combination crossing both sorts (a graded-thunk argument, a
/// row-carrying `F` over a product, `Unknown` in both polarities). Each is
/// related to its [`deep_rebuild_comp`] copy (fresh `Rc`s defeat the
/// `core::ptr::eq` short-circuit), so a future `CompType` variant whose
/// reflexive arm is omitted from [`coherence_comp_subtype`] fails
/// deterministically.
#[test]
fn coherence_comp_subtype_reflexivity_arm_sweep()
{
    let witnesses = [
        CompType::returner(ValueType::integer()),
        CompType::returner_eff(ValueType::Unit, EffectRow::singleton(state_sig())),
        CompType::arrow(
            ValueType::integer(),
            CompType::returner(ValueType::string()),
        ),
        CompType::with(
            CompType::returner(ValueType::Unit),
            CompType::returner(ValueType::integer()),
        ),
        CompType::Unknown,
        CompType::arrow(
            ValueType::thunk(Grade::ONE, CompType::returner(ValueType::Unknown)),
            CompType::with(
                CompType::returner_eff(
                    ValueType::prod(ValueType::integer(), ValueType::Unknown),
                    EffectRow::singleton(io_sig()),
                ),
                CompType::Unknown,
            ),
        ),
    ];
    for ty in &witnesses {
        assert!(
            bool::from(coherence_comp_subtype(ty, &deep_rebuild_comp(ty))),
            "coherence reflexivity failed on the arm of {ty:?}"
        );
    }
}

/// The first subsumption-coherence violation for a computation, if any (the
/// oracle). If `comp` infers `B'`, then for every candidate answer
/// it *also checks against* — the supplied `extra`, `B'` itself, and each
/// [`comp_near_misses`] — `B'` must be a consistent subtype **up to
/// integer-literal defaulting** ([`coherence_comp_subtype`], ADR-48). Returns
/// the `(B', offending candidate)` of the first check that succeeds without
/// that relation holding; `None` is coherent (or `comp` does not infer, so
/// there is no principal type to relate, and no obligation — note the bare
/// integer literal is the one leaf whose inferred `Integer` is a Rust-style
/// *default*, not a principal type, which is why the relation tolerates its
/// sized-int widening).
///
/// This relates the two *modes* (infer ⇑ vs check ⇓), so — unlike the
/// `arbitrary_comps_agree` differential suite, which sees only that the two
/// *implementations* agree — it catches a soundness bug both implementations
/// share (they are derived from each other, so they agree on the *wrong*
/// answer). Either implementation suffices for the oracle; this uses the
/// recursive checker, and the machine inherits the verdict through the
/// differential suite.
fn comp_coherence_violation(
    comp: &Comp,
    extra: CompType,
) -> Option<(CompType, CompType)>
{
    let (inferred, _) = checker::run_comp(base_ctx(), comp.clone(), Dir::Infer);
    let Ok(Ty::Comp(b_prime)) = inferred
    else {
        return None;
    };
    let mut candidates = alloc::vec![extra, b_prime.clone()];
    candidates.extend(comp_near_misses(&b_prime));
    for candidate in candidates {
        let (checked, _) =
            checker::run_comp(base_ctx(), comp.clone(), Dir::Check(candidate.clone()));
        if checked.is_ok() && !bool::from(coherence_comp_subtype(&b_prime, &candidate)) {
            return Some((b_prime, candidate));
        }
    }
    None
}

/// Row-tightened **near-miss** answers for the computation coherence oracle:
/// types a *sound* checker must reject for a term that infers `b_prime`,
/// because they drop an accumulated effect row (`F^ε A → F^⟨⟩ A`, sound only
/// when `ε = ⟨⟩`). A check that nonetheless succeeds against one — the A3.2
/// `bind` hole let the bound row escape into a smaller checked answer — is
/// caught the instant it does, since then `b_prime ⊄ near_miss`. Empty for a
/// pure or non-returner `b_prime` (no row to drop).
fn comp_near_misses(b_prime: &CompType) -> Vec<CompType>
{
    let mut misses = Vec::new();
    if let CompType::F(ref payload, ref row) = *b_prime
        && !bool::from(row.is_empty())
    {
        misses.push(CompType::returner(payload.as_ref().clone()));
    }
    misses
}

/// The value-sort analogue of [`comp_coherence_violation`]: the grade leg of
/// [`gandr_core_checker::types::ValueType::Thunk`] plays the role the effect
/// row plays for computations (see [`value_near_misses`]). Coherence is decided
/// up to integer-literal defaulting ([`coherence_value_subtype`], ADR-48).
fn value_coherence_violation(
    value: &Value,
    extra: ValueType,
) -> Option<(ValueType, ValueType)>
{
    let (inferred, _) = checker::run_value(base_ctx(), value.clone(), Dir::Infer);
    let Ok(Ty::Value(a_prime)) = inferred
    else {
        return None;
    };
    let mut candidates = alloc::vec![extra, a_prime.clone()];
    candidates.extend(value_near_misses(&a_prime));
    for candidate in candidates {
        let (checked, _) =
            checker::run_value(base_ctx(), value.clone(), Dir::Check(candidate.clone()));
        if checked.is_ok() && !bool::from(coherence_value_subtype(&a_prime, &candidate)) {
            return Some((a_prime, candidate));
        }
    }
    None
}

/// Consistent subtype **up to integer-literal defaulting** — the relation the
/// coherence oracle compares against (ADR-48), NOT a typing rule. It is
/// [`value_subtype`] extended with the single covariant atom rule `Integer ⊑
/// {u32, u64, i32, i64}` (ADR-39 D4): a bare integer literal infers the default
/// `Integer` yet checks against any sized-int atom it is representable in, so
/// an inferred `Integer` in a *covariant* position is coherent with a checked
/// sized-int atom. The relaxation is **covariant only** — contravariant legs
/// (arrow argument, consumed stack) and the grade / effect-row legs delegate to
/// the *strict* [`value_subtype`] / [`comp_subtype`], so the A3.2 row-escape
/// and grade-tightening obligations stay sharp. The relaxation is sound because
/// the widening is *literal-only* (a variable of type `Integer` never widens —
/// the checker routes it through `value_subtype`, atoms-by-equality); were that
/// ever to break, this type-level relaxation could mask it, which is exactly
/// what [`var_of_integer_does_not_widen_to_sized_atom`] guards independently.
/// # Termination
/// - reason: the helper drains an explicit finite comparison worklist.
/// - measure: pending type-pair comparison goals.
/// - boundedness: compared types are finite Rust values.
/// - input recursion: none.
fn coherence_value_subtype(
    sub: &ValueType,
    sup: &ValueType,
) -> CoherenceDecision
{
    coherence_subtype_from(CoherenceGoal::Value(sub, sup))
}

/// Guard for the coherence relation's integer-literal relaxation (ADR-48): the
/// check-mode widening is syntactic to a *literal*
/// (`subtype::finish_int_literal`), so a **variable** of type `Integer` must
/// NOT check against a sized-int atom — only a bare `Int` literal does. This
/// pins the one soundness assumption the type-level [`coherence_value_subtype`]
/// relaxation rests on; were a variable ever to widen (a real unsoundness), the
/// relaxation could mask it in the oracle, so this test fails independently if
/// it does. The variable is bound to `integer()` (the atom `Integer` a bare
/// literal *infers* and the relaxation keys on) — NOT `base_ctx`'s `i`, which
/// is the distinct base realizer atom `Int` (`strategies::int()`) the
/// relaxation never touches; binding that would make the guard vacuous (the
/// adversarial pass caught exactly this mis-wiring).
#[test]
fn var_of_integer_does_not_widen_to_sized_atom()
{
    let u32_ty = ValueType::u32();
    // A variable of the literal-default atom `Integer` does NOT widen to `u32`.
    let ctx = Ctx::new().with("x", integer());
    let (var_checked, _) = checker::run_value(ctx, Value::var("x"), Dir::Check(u32_ty.clone()));
    assert!(
        var_checked.is_err(),
        "a variable of type Integer must not widen to u32 (widening is literal-only): \
         {var_checked:?}"
    );
    // A bare integer literal representable in u32 DOES check against u32 (ADR-39
    // D4).
    let (lit_checked, _) =
        checker::run_value(Ctx::new(), Value::int(0), Dir::Check(u32_ty.clone()));
    assert_eq!(
        lit_checked,
        Ok(Ty::Value(u32_ty)),
        "a bare integer literal representable in u32 must check against u32"
    );
}
/// Companion to [`var_of_integer_does_not_widen_to_sized_atom`] closing the
/// non-literal **computation** vector (adversarial C1): the relaxation in
/// [`coherence_comp_subtype`] is blind to whether the widened `Integer` came
/// from a literal, so its soundness rests on the whole-checker invariant "only
/// a bare `Value::Int` widens `Integer` → sized-int". This pins that invariant
/// at two comps whose `F Integer` payload is NOT a literal: `Ret(Var i)`
/// (payload from a variable) and `perform State.get` (payload from an
/// operation's residual reply, checked by `finish_comp` with no sub-value
/// check). Neither may check against the sized-int returner; a future rule that
/// let one would be masked by the type-level relaxation in the oracle, so this
/// fails independently if it regresses. The `Ret` variable is bound to
/// `integer()` (the atom `Integer`); `state_sig` gives `get : 1 ↠ Integer` (an
/// `Integer` residual reply).
#[test]
fn integer_computation_does_not_widen_to_sized_returner()
{
    // `Ret(Var x)` where `x : Integer` infers `F Integer` (payload from a
    // variable, not a literal); it must not widen to `F u32`.
    let ctx = Ctx::new().with("x", integer());
    let (ret_var, _) = checker::run_comp(
        ctx,
        Comp::ret(Value::var("x")),
        Dir::Check(CompType::returner(ValueType::u32())),
    );
    assert!(
        ret_var.is_err(),
        "Ret of a variable of type Integer must not widen to F u32: {ret_var:?}"
    );

    // `perform State.get` infers `F^⟨State⟩ Integer` — the `Integer` is the
    // operation's residual reply, reached via `finish_comp` with NO sub-value
    // check. Rebuild the *same* effect row with a `u32` payload and confirm the
    // check is rejected (the widening does not reach a residual-typed returner).
    let get = Comp::perform(state_sig(), "get", Value::Unit);
    let (inferred, _) = checker::run_comp(base_ctx(), get.clone(), Dir::Infer);
    let Ok(Ty::Comp(CompType::F(_, row))) = inferred
    else {
        panic!("perform State.get must infer a returner, got {inferred:?}");
    };
    let widened = CompType::F(Rc::new(ValueType::u32()), row);
    let (perform_checked, _) = checker::run_comp(base_ctx(), get, Dir::Check(widened));
    assert!(
        perform_checked.is_err(),
        "perform State.get (F^⟨State⟩ Integer) must not widen to F^⟨State⟩ u32: {perform_checked:?}"
    );
}

/// The `State` effect signature `{ get : 1 ↠ Integer, put : Integer ↠ 1 }`
/// (A3.2 `+effects`): a two-operation signature for the perform/handle tests
/// and the free generators.
fn state_sig() -> EffectSig
{
    EffectSig::new(
        gandr_core_checker::boundary::EffectSignatureName::from("State"),
        vec![
            EffectOp::new(
                gandr_core_checker::boundary::OperationName::from("get"),
                ValueType::Unit,
                integer(),
            ),
            EffectOp::new(
                gandr_core_checker::boundary::OperationName::from("put"),
                integer(),
                ValueType::Unit,
            ),
        ],
    )
}

/// A context with realizers for every atom (see [`base_scope`]): the shadowable
/// `i`/`s` plus the reserve `int_base`/`str_base`.
fn base_ctx() -> Ctx
{
    Ctx::new()
        .with("i", int())
        .with("s", txt())
        .with("int_base", int())
        .with("str_base", txt())
}

/// Proptest configuration for the conformance properties.
///
/// Under Miri this deviates from the native configuration (justification per
/// `.omp/rules/miri-ignore-comment.md` — Miri-unsupported filesystem/IO, not a
/// hidden real failure):
///
/// - `failure_persistence` is disabled: proptest's default
///   `FileFailurePersistence` calls `std::env::current_dir` (`getcwd`) and
///   reads/writes a `proptest-regressions` file at runner startup; both are
///   filesystem operations that Miri's default isolation rejects, aborting the
///   whole test process ("test exited abnormally") before any case runs.
/// - `cases` is reduced: Miri interprets code roughly two orders of magnitude
///   slower than native execution; a few cases per property are enough to
///   exercise the `Rc`-based AST, checker, and machine for memory-safety bugs,
///   while the full 96-case run still executes in the native test jobs.
///
/// Natively, an explicit `PROPTEST_CASES` environment variable overrides the
/// 96-case default (for longer shakeout runs, e.g. `PROPTEST_CASES=512`).
fn conformance_proptest_config() -> ProptestConfig
{
    let mut config = ProptestConfig::default();
    if cfg!(miri) {
        config.cases = 4;
        config.failure_persistence = None;
    }
    else if std::env::var_os("PROPTEST_CASES").is_none() {
        const NATIVE_PROPTEST_CASES: u32 = 96;
        config.cases = NATIVE_PROPTEST_CASES;
    }
    config
}

/// An arbitrary direction for values.
fn arb_value_dir() -> BoxedStrategy<Dir<ValueType>>
{
    prop_oneof![Just(Dir::Infer), arb_value_type(2_u32).prop_map(Dir::Check)].boxed()
}

proptest! {
    #![proptest_config(conformance_proptest_config())]

    /// Well-typed checked computations: both implementations succeed with the
    /// expected type and agree step for step (ADR-9).
    #[test]
    fn checked_comps_agree_and_succeed(
        (ty, comp) in arb_comp_type(2_u32).prop_flat_map(|ty| {
            comp_check_strategy(ty.clone(), base_scope(), 2_u32)
                .prop_map(move |comp| (ty.clone(), comp))
        }),
    )
    {
        let dir = Dir::Check(ty.clone());
        let ((rec_result, rec_trace), (mach_result, mach_trace)) =
            both_comp(&base_ctx(), &comp, &dir);
        prop_assert_eq!(&rec_trace, &mach_trace);
        prop_assert_eq!(&rec_result, &mach_result);
        prop_assert_eq!(mach_result, Ok(Ty::Comp(ty)));
        // (5) A successful run restores Γ: the final context equals the initial.
        let report = machine::run_report(machine::State::new_comp(base_ctx(), comp, dir));
        prop_assert_eq!(report.ctx, base_ctx());
    }

    /// Well-typed inferred computations: both implementations infer exactly
    /// the target type and agree step for step (ADR-9).
    #[test]
    fn inferred_comps_agree_and_succeed(
        (ty, comp) in arb_comp_type(2_u32).prop_flat_map(|ty| {
            comp_infer_strategy(ty.clone(), base_scope(), 2_u32)
                .prop_map(move |comp| (ty.clone(), comp))
        }),
    )
    {
        let ((rec_result, rec_trace), (mach_result, mach_trace)) =
            both_comp(&base_ctx(), &comp, &Dir::Infer);
        prop_assert_eq!(&rec_trace, &mach_trace);
        prop_assert_eq!(&rec_result, &mach_result);
        prop_assert_eq!(mach_result, Ok(Ty::Comp(ty)));
        // (5) A successful run restores Γ: the final context equals the initial.
        let report = machine::run_report(machine::State::new_comp(base_ctx(), comp, Dir::Infer));
        prop_assert_eq!(report.ctx, base_ctx());
    }

    /// Well-typed checked values: success and step-for-step agreement.
    #[test]
    fn checked_values_agree_and_succeed(
        (ty, value) in arb_value_type(2_u32).prop_flat_map(|ty| {
            value_check_strategy(ty.clone(), base_scope(), 2_u32)
                .prop_map(move |value| (ty.clone(), value))
        }),
    )
    {
        let dir = Dir::Check(ty.clone());
        let ((rec_result, rec_trace), (mach_result, mach_trace)) =
            both_value(&base_ctx(), &value, &dir);
        prop_assert_eq!(&rec_trace, &mach_trace);
        prop_assert_eq!(&rec_result, &mach_result);
        prop_assert_eq!(mach_result, Ok(Ty::Value(ty)));
        // (5) A successful run restores Γ: the final context equals the initial.
        let report = machine::run_report(machine::State::new_value(base_ctx(), value, dir));
        prop_assert_eq!(report.ctx, base_ctx());
    }

    /// Well-typed inferred values: exact type and step-for-step agreement.
    #[test]
    fn inferred_values_agree_and_succeed(
        (ty, value) in arb_value_type(2_u32).prop_flat_map(|ty| {
            value_infer_strategy(ty.clone(), base_scope(), 2_u32)
                .prop_map(move |value| (ty.clone(), value))
        }),
    )
    {
        let ((rec_result, rec_trace), (mach_result, mach_trace)) =
            both_value(&base_ctx(), &value, &Dir::Infer);
        prop_assert_eq!(&rec_trace, &mach_trace);
        prop_assert_eq!(&rec_result, &mach_result);
        prop_assert_eq!(mach_result, Ok(Ty::Value(ty)));
        // (5) A successful run restores Γ: the final context equals the initial.
        let report = machine::run_report(machine::State::new_value(base_ctx(), value, Dir::Infer));
        prop_assert_eq!(report.ctx, base_ctx());
    }

    /// Arbitrary (mostly ill-typed) computations in arbitrary directions:
    /// results and traces — including error traces — agree exactly.
    #[test]
    fn arbitrary_comps_agree(comp in arb_comp(3_u32), dir in arb_comp_dir())
    {
        let ((rec_result, rec_trace), (mach_result, mach_trace)) =
            both_comp(&base_ctx(), &comp, &dir);
        prop_assert_eq!(rec_trace, mach_trace);
        prop_assert_eq!(rec_result, mach_result);
    }

    /// Arbitrary (mostly ill-typed) values in arbitrary directions: results
    /// and traces — including error traces — agree exactly.
    #[test]
    fn arbitrary_values_agree(value in arb_value(3_u32), dir in arb_value_dir())
    {
        let ((rec_result, rec_trace), (mach_result, mach_trace)) =
            both_value(&base_ctx(), &value, &dir);
        prop_assert_eq!(rec_trace, mach_trace);
        prop_assert_eq!(rec_result, mach_result);
    }

    /// (10) The machine's `steps` counter is load-bearing: it counts exactly
    /// the `step` calls taken, one fewer than the trace length (the trace also
    /// records the initial control). The relation holds on success and failure.
    #[test]
    fn step_counter_tracks_trace_length_comp(comp in arb_comp(3_u32), dir in arb_comp_dir())
    {
        let report = machine::run_report(machine::State::new_comp(base_ctx(), comp, dir));
        prop_assert!(!report.trace.is_empty());
        prop_assert_eq!(usize::try_from(u64::from(report.steps)).ok(), Some(report.trace.len().saturating_sub(1)));
    }

    /// (10) As [`step_counter_tracks_trace_length_comp`], for value runs.
    #[test]
    fn step_counter_tracks_trace_length_value(value in arb_value(3_u32), dir in arb_value_dir())
    {
        let report = machine::run_report(machine::State::new_value(base_ctx(), value, dir));
        prop_assert!(!report.trace.is_empty());
        prop_assert_eq!(usize::try_from(u64::from(report.steps)).ok(), Some(report.trace.len().saturating_sub(1)));
    }

    /// (a7) The machine-internal polarity guards `SHAPE_VALUE` / `SHAPE_COMP`
    /// are unreachable by construction: they never surface in a generated run's
    /// error (computation generators).
    #[test]
    fn internal_shape_texts_never_surface_comp(comp in arb_comp(3_u32), dir in arb_comp_dir())
    {
        let (result, _) = machine::run_comp(base_ctx(), comp, dir);
        if let Err(TypeError::ShapeMismatch { expected, .. }) = result {
            prop_assert_ne!(expected, gandr_core_checker::error::text::SHAPE_VALUE);
            prop_assert_ne!(expected, gandr_core_checker::error::text::SHAPE_COMP);
        }
    }

    /// (a7) As [`internal_shape_texts_never_surface_comp`], for value runs.
    #[test]
    fn internal_shape_texts_never_surface_value(value in arb_value(3_u32), dir in arb_value_dir())
    {
        let (result, _) = machine::run_value(base_ctx(), value, dir);
        if let Err(TypeError::ShapeMismatch { expected, .. }) = result {
            prop_assert_ne!(expected, gandr_core_checker::error::text::SHAPE_VALUE);
            prop_assert_ne!(expected, gandr_core_checker::error::text::SHAPE_COMP);
        }
    }

    /// (a4) Reflexivity is admissible (never a solver rule,
    /// §"Algorithmic subtyping and the worklist solver"): every value type —
    /// `Unknown` included — is its own (consistent) subtype.
    ///
    /// The supertype is a [`deep_rebuild_value`] of the subtype — structurally
    /// identical but with fresh `Rc`s at every node — so the `core::ptr::eq`
    /// short-circuit in [`value_subtype`] CANNOT preempt: distinct addresses
    /// force the structural reflexive path to be exercised (comparing against
    /// the same binding twice would make this oracle a tautology).
    #[test]
    fn value_subtype_is_reflexive(ty in arb_value_type(3_u32))
    {
        prop_assert!(bool::from(value_subtype(&ty, &deep_rebuild_value(&ty))));
    }

    /// (a4) Reflexivity is admissible: every computation type is its own
    /// subtype. As [`value_subtype_is_reflexive`], the supertype is a
    /// [`deep_rebuild_comp`] copy so the `core::ptr::eq` short-circuit cannot
    /// preempt the structural path.
    #[test]
    fn comp_subtype_is_reflexive(ty in arb_comp_type(3_u32))
    {
        prop_assert!(bool::from(comp_subtype(&ty, &deep_rebuild_comp(&ty))));
    }

    /// (a4) Reflexivity of the **coherence** twin ([`coherence_value_subtype`],
    /// ADR-48) — `value_subtype` extended with the covariant `Integer ⊑
    /// sized-int` relaxation. It must survive the same `core::ptr::eq`
    /// short-circuit the twin carries: the supertype is a [`deep_rebuild_value`]
    /// copy (fresh `Rc`s at every node) so the fast path cannot pre-empt, and
    /// the coherence twin's structural arms are exercised — exactly as
    /// [`value_subtype_is_reflexive`] guards the plain relation.
    /// The free generator may still miss an arm at low case counts, so the
    /// deterministic companion sweeps every constructor each run (ADR-48).
    ///
    /// SOUNDNESS-ORACLE-WITNESS: coherence_value_subtype_reflexivity_arm_sweep
    #[test]
    fn coherence_value_subtype_is_reflexive(ty in arb_value_type(3_u32))
    {
        prop_assert!(bool::from(coherence_value_subtype(&ty, &deep_rebuild_value(&ty))));
    }

    /// (a4) Reflexivity of the computation-sort coherence twin
    /// ([`coherence_comp_subtype`], ADR-48). As
    /// [`coherence_value_subtype_is_reflexive`], the supertype is a
    /// [`deep_rebuild_comp`] copy so the `core::ptr::eq` short-circuit cannot
    /// pre-empt the structural descent; the deterministic
    /// companion sweeps every `CompType` constructor each run (ADR-48).
    ///
    /// SOUNDNESS-ORACLE-WITNESS: coherence_comp_subtype_reflexivity_arm_sweep
    #[test]
    fn coherence_comp_subtype_is_reflexive(ty in arb_comp_type(3_u32))
    {
        prop_assert!(bool::from(coherence_comp_subtype(&ty, &deep_rebuild_comp(&ty))));
    }

    /// (a4) Transitivity is admissible
    /// (§"Algorithmic subtyping and the worklist solver") **on static types**:
    /// `A <: B` and `B <: C` imply `A <: C` when no `Unknown` is involved. With
    /// `Unknown` the relation is consistent subtyping, which is deliberately
    /// not transitive (`gandr_core_checker::subtype` module doc; the witness is pinned by
    /// [`holes::consistency_is_not_transitive`]).
    #[test]
    fn value_subtype_is_transitive_on_static_types(
        a in arb_value_type(2_u32),
        b in arb_value_type(2_u32),
        c in arb_value_type(2_u32),
    )
    {
        let all_static =
            bool::from(value_type_is_static(&a)) && bool::from(value_type_is_static(&b)) && bool::from(value_type_is_static(&c));
        if all_static && bool::from(value_subtype(&a, &b)) && bool::from(value_subtype(&b, &c)) {
            prop_assert!(bool::from(value_subtype(&a, &c)));
        }
    }

    /// (a4) Transitivity is admissible on static computation types; see
    /// [`value_subtype_is_transitive_on_static_types`].
    #[test]
    fn comp_subtype_is_transitive_on_static_types(
        a in arb_comp_type(2_u32),
        b in arb_comp_type(2_u32),
        c in arb_comp_type(2_u32),
    )
    {
        let all_static =
            bool::from(comp_type_is_static(&a)) && bool::from(comp_type_is_static(&b)) && bool::from(comp_type_is_static(&c));
        if all_static && bool::from(comp_subtype(&a, &b)) && bool::from(comp_subtype(&b, &c)) {
            prop_assert!(bool::from(comp_subtype(&a, &c)));
        }
    }

    /// (A2.2) A hole in check mode **never errors**: against any expected
    /// value type, both implementations succeed with the expected type and
    /// agree step for step (rule Hole⇓; the plan's named hole property).
    #[test]
    fn value_hole_checks_against_any_type(ty in arb_value_type(3_u32), id in hole_id())
    {
        let dir = Dir::Check(ty.clone());
        let ((rec_result, rec_trace), (mach_result, mach_trace)) =
            both_value(&base_ctx(), &Value::Hole(id), &dir);
        prop_assert_eq!(rec_trace, mach_trace);
        prop_assert_eq!(&rec_result, &mach_result);
        prop_assert_eq!(mach_result, Ok(Ty::Value(ty)));
    }

    /// (A2.2) As [`value_hole_checks_against_any_type`], for computation
    /// holes.
    #[test]
    fn comp_hole_checks_against_any_type(ty in arb_comp_type(3_u32), id in hole_id())
    {
        let dir = Dir::Check(ty.clone());
        let ((rec_result, rec_trace), (mach_result, mach_trace)) =
            both_comp(&base_ctx(), &Comp::Hole(id), &dir);
        prop_assert_eq!(rec_trace, mach_trace);
        prop_assert_eq!(&rec_result, &mach_result);
        prop_assert_eq!(mach_result, Ok(Ty::Comp(ty)));
    }

    /// Subsumption-coherence (the independent soundness oracle):
    /// for a free (mostly ill-typed) computation `t` and a candidate answer
    /// `B`, if `t` both **infers** a type `B'` and **checks** against `B`, then
    /// `B'` must be a consistent subtype of `B` (see [`comp_coherence_violation`]
    /// for the full obligation, which also relates `B'` to itself and to its
    /// row-stripped near-misses).
    ///
    /// This relates the two *modes* (infer ⇑ vs check ⇓) rather than the two
    /// *implementations*, so it is independent of — and complementary to — the
    /// [`arbitrary_comps_agree`] differential suite, which can only see that the
    /// recursive checker and the typing machine *agree*: a soundness bug both
    /// share (they are derived from each other) leaves them agreeing on the
    /// *wrong* answer, invisible at any case count. The A3.2 check-mode `bind`
    /// row-escape (commit `f930413`) was exactly such a bug
    /// (`infer = F^⟨State⟩ Int`, yet `check` against the pure `F^⟨⟩ Int`
    /// wrongly succeeded though `⟨State⟩ ⊄ ⟨⟩`). The free generator reaches
    /// that *class* only rarely, so [`effectful_bind_subsumption_coherence`]
    /// drives the same obligation over a biased generator that reliably builds
    /// it; this property is the broad net over every inferring form.
    ///
    /// Why it (almost) cannot raise a false alarm on a *correct* checker: an
    /// inferring form's check mode applies the inlined Sub rule directly
    /// ([`gandr_core_checker::subtype::finish_comp`], literally `comp_subtype(constructed,
    /// expected)`) — every elimination, and every leaf (variable, hole) bottoming
    /// out at a consistent-subtype check or at `Unknown`, which relates to
    /// everything — or, for the few direction-*forwarding* forms (`split`),
    /// inherits `B' <: B` from the subterm typed in the same direction. So
    /// `B' <: B` holds whenever both modes succeed, unless a rule's check face
    /// accepts a non-supertype — precisely the unsoundness this oracle catches.
    ///
    /// The **one** leaf not covered by plain subtyping is the bare integer literal
    /// (ADR-39 D4): it infers the Rust-style *default* `Integer` yet checks against
    /// any sized-int atom it is representable in, so its inferred type is not
    /// principal and `Integer ⋢ u32` while the check legitimately succeeds (the
    /// counterexample, `Ret(Int(0))` vs `F u32`). This is *sound* — the
    /// widening is literal-only — so the oracle compares up to
    /// [`coherence_comp_subtype`], subtype extended with the covariant `Integer ⊑
    /// sized-int` relaxation, not plain `comp_subtype`; the effect-row and grade
    /// legs stay strict, and [`var_of_integer_does_not_widen_to_sized_atom`] guards
    /// that no *variable* widens. The relation is the two-point `B' <: B` (so the
    /// non-transitivity of consistent subtyping does not bite), and the literal,
    /// grade, and effect-row classes each carry a biased companion (ADR-48) — the
    /// deterministic witnesses `scripts/check-soundness-oracles.nu` gates.
    ///
    /// SOUNDNESS-ORACLE-WITNESS: effectful_bind_subsumption_coherence, int_literal_defaulting_subsumption_coherence
    #[test]
    fn infer_check_subsumption_coherence_comp(
        comp in arb_comp(3_u32),
        candidate in arb_comp_type(2_u32),
    )
    {
        let violation = comp_coherence_violation(&comp, candidate);
        prop_assert!(
            violation.is_none(),
            "subsumption-coherence violated: {violation:?} — a check against B succeeded \
             yet the inferred B' is not a consistent subtype of B"
        );
    }

    /// Subsumption-coherence over effectful, inferring computations:
    /// the reliable counterpart to
    /// [`infer_check_subsumption_coherence_comp`]. Its [`arb_effectful_comp`]
    /// terms all infer `F^ε A` with a non-empty `ε`, so [`comp_near_misses`]
    /// always offers the pure `F^⟨⟩ A` answer — the exact shape the A3.2 `bind`
    /// hole accepted. A correct checker rejects every such near-miss (the row
    /// cannot escape); a regression that drops the final `finish_comp` from a
    /// check-mode row rule makes one succeed, and the oracle fires. This is the
    /// property that would have caught the A3.2 hole directly.
    ///
    /// SOUNDNESS-ORACLE-COMPANION
    #[test]
    fn effectful_bind_subsumption_coherence(
        comp in arb_effectful_comp(3_u32),
        candidate in arb_comp_type(2_u32),
    )
    {
        let violation = comp_coherence_violation(&comp, candidate);
        prop_assert!(
            violation.is_none(),
            "effect-row coherence violated: {violation:?} — a check-mode rule let an \
             accumulated effect row escape into a smaller checked answer"
        );
    }

    /// Subsumption-coherence for the value sort: the value-mode
    /// analogue of [`infer_check_subsumption_coherence_comp`] (see
    /// [`value_coherence_violation`]). The effect row is absent, but the grade
    /// leg of [`gandr_core_checker::types::ValueType::Thunk`] plays the same role — an
    /// inferred `U_r B` checked against the grade-tightened `U_ω B` near-miss
    /// exercises the `ω ⊑ r` leg directly. The bare-integer-literal defaulting
    /// (ADR-39 D4) is a value leaf too, so this oracle also compares up to
    /// [`coherence_value_subtype`].
    ///
    /// SOUNDNESS-ORACLE-WITNESS: graded_thunk_subsumption_coherence, int_literal_defaulting_subsumption_coherence
    #[test]
    fn infer_check_subsumption_coherence_value(
        value in arb_value(3_u32),
        candidate in arb_value_type(2_u32),
    )
    {
        let violation = value_coherence_violation(&value, candidate);
        prop_assert!(
            violation.is_none(),
            "subsumption-coherence violated: {violation:?} — a check against A succeeded \
             yet the inferred A' is not a consistent subtype of A"
        );
    }

    /// Integer-literal defaulting coherence (ADR-39 D4, ADR-48) — the biased
    /// counterpart to [`infer_check_subsumption_coherence_comp`] / `_value` that
    /// the adversarial pass showed the free cross reaches only by accident. Every generated
    /// pair is a bare `Int` literal representable in a sized-int atom, so
    /// check-mode defaulting fires and the inferred `Integer` is coherent with the
    /// checked atom — a *sound* asymmetry the refined oracle must NOT flag. Covers
    /// the value sort, the comp sort (through `Ret`), and a covariant *nested*
    /// position (a `Prod` component), exercising the covariant `Integer ⊑
    /// sized-int` relaxation of [`coherence_value_subtype`] at each.
    ///
    /// SOUNDNESS-ORACLE-COMPANION
    #[test]
    fn int_literal_defaulting_subsumption_coherence(
        (n, atom) in arb_int_literal_and_sized_atom(),
    )
    {
        prop_assert!(
            value_coherence_violation(&Value::int(n), atom.clone()).is_none(),
            "value-sort integer-literal defaulting flagged incoherent for {atom:?}"
        );
        prop_assert!(
            comp_coherence_violation(&Comp::ret(Value::int(n)), CompType::returner(atom.clone()))
                .is_none(),
            "comp-sort integer-literal defaulting flagged incoherent for {atom:?}"
        );
        prop_assert!(
            value_coherence_violation(
                &Value::pair(Value::int(n), Value::Unit),
                ValueType::prod(atom.clone(), ValueType::Unit),
            )
            .is_none(),
            "nested integer-literal defaulting flagged incoherent for {atom:?}"
        );
    }

    /// Grade-tightening coherence (ADR-48) — the biased counterpart to
    /// [`infer_check_subsumption_coherence_value`] over the `U_r` / `U_ω` grade
    /// leg, the value-sort analogue of the effect-row class. Every generated thunk
    /// infers `U_r B` with `r ≠ ω`, so [`value_near_misses`] offers the
    /// grade-tightened `U_ω B`; a correct checker rejects it (`ω ⊑ r` only when
    /// `r = ω`), so no violation — [`coherence_value_subtype`] keeps the grade leg
    /// strict, and a regression that let a smaller budget satisfy a larger demand
    /// fires here.
    ///
    /// SOUNDNESS-ORACLE-COMPANION
    #[test]
    fn graded_thunk_subsumption_coherence(grade in any_grade())
    {
        prop_assume!(grade != Grade::OMEGA);
        let thunk = Value::thunk(grade, Comp::ret(Value::Unit));
        let omega = ValueType::thunk(Grade::OMEGA, CompType::returner(ValueType::Unit));
        prop_assert!(
            value_coherence_violation(&thunk, omega).is_none(),
            "grade-tightening coherence violated for U_{grade:?}: a smaller budget satisfied U_ω"
        );
    }

    /// (Σ vacuity, ADR-33 D5) The linear zone `Σ` stays **empty through every
    /// run**: no v0 typing rule populates it (every `Σ`-obligation source —
    /// session endpoints, held capabilities, acquired channels — is a deferred
    /// `+feature`), so a reified stack captures no obligations and `resume` /
    /// `discard` / duplication are unrestricted. This is the conformance
    /// meta-invariant the [`gandr_core_checker::ctx::Sigma`] and [`gandr_core_checker::stack`] module docs
    /// reference: it pins the "vacuous in v0" claim across the **whole** free
    /// generator space — the new `stk` / `resume` / `reset` / `shift` control
    /// terms included — so the one-shot/linear discipline (whose laws are
    /// directly unit-tested over [`gandr_core_checker::ctx::Sigma`]) is not merely "green
    /// because nothing ever touches `Σ`". A future rule that binds into `Σ`
    /// without the full linear discipline trips this.
    #[test]
    fn sigma_stays_empty_through_every_comp_run(comp in arb_comp(3_u32), dir in arb_comp_dir())
    {
        let report = machine::run_report(machine::State::new_comp(base_ctx(), comp, dir));
        prop_assert!(
            bool::from(report.ctx.sigma().is_empty()),
            "Σ must stay empty after every run: no v0 rule populates it"
        );
    }

    /// (Σ vacuity, ADR-33 D5) As [`sigma_stays_empty_through_every_comp_run`],
    /// over value runs — `stk K` reification walks the stack-judgment over
    /// `Γ; Σ`, binding the consumed payload of a bind frame into `Γ`, yet binds
    /// nothing into `Σ` in v0.
    #[test]
    fn sigma_stays_empty_through_every_value_run(value in arb_value(3_u32), dir in arb_value_dir())
    {
        let report = machine::run_report(machine::State::new_value(base_ctx(), value, dir));
        prop_assert!(
            bool::from(report.ctx.sigma().is_empty()),
            "Σ must stay empty after every run: no v0 rule populates it"
        );
    }
}

/// Lock-step typing coverage of the native-builtin substrate [`Comp::Native`]
/// (ADR-42; the MVP module layer): the typing axiom (`checker ≡ machine`, via
/// [`agree_comp`]) over source and partially-applied native nodes, including
/// the residual-type preservation the argument-accumulating reduction rests on.
///
/// The native node's **evaluation** — the argument-accumulating reduction,
/// currying, and the prelude free-name resolution — is machine-only and pinned
/// on the L machine in `gandr_core_sequent::conformance_soundness` (the
/// retired reference evaluator's frozen outcome snapshots).
mod native
{
    use gandr_core_checker::prim::NativePrim;

    use super::*;
    /// Rule Native⇑: a source (argument-free) native infers its declared type,
    /// and the recursive checker and the typing machine agree.
    #[test]
    fn native_infers_its_declared_type()
    {
        let id = agree_comp(&Ctx::new(), &Comp::native(NativePrim::Id), &Dir::Infer);
        assert_eq!(id, Ok(Ty::Comp(id_ty())), "I infers Integer → F Integer");
        let konst = agree_comp(&Ctx::new(), &Comp::native(NativePrim::Const), &Dir::Infer);
        assert_eq!(
            konst,
            Ok(Ty::Comp(const_ty())),
            "K infers Integer → Integer → F Integer"
        );
    }
    /// Rule Native⇓: a native checks against its declared type by subsumption.
    #[test]
    fn native_checks_against_its_declared_type()
    {
        let checked = agree_comp(
            &Ctx::new(),
            &Comp::native(NativePrim::Id),
            &Dir::Check(id_ty()),
        );
        assert_eq!(checked, Ok(Ty::Comp(id_ty())));
    }

    /// `I`'s declared type `Integer → F Integer`.
    fn id_ty() -> CompType
    {
        NativePrim::Id.declared_type()
    }

    /// `K`'s declared type `Integer → Integer → F Integer`.
    fn const_ty() -> CompType
    {
        NativePrim::Const.declared_type()
    }
    /// A native head integrates with ordinary application typing: `K 7` infers
    /// the residual `Integer → F Integer` (one arrow peeled), checker ≡
    /// machine.
    #[test]
    fn native_application_types_to_the_residual()
    {
        let app = Comp::app(Comp::native(NativePrim::Const), Value::int(7));
        let inferred = agree_comp(&base_ctx(), &app, &Dir::Infer);
        assert_eq!(
            inferred,
            Ok(Ty::Comp(const_residual())),
            "K applied once is Integer → F Integer"
        );
    }

    /// The mid-evaluation form `Native{Const, [7]}` (one argument already
    /// accumulated into the node) types to the same residual — `residual_type`
    /// keeps subject reduction sound over the node.
    #[test]
    fn partially_applied_native_node_types_to_the_residual()
    {
        let partial = Comp::Native {
            prim: NativePrim::Const,
            args: vec![Rc::new(Value::int(7))],
        };
        let inferred = agree_comp(&Ctx::new(), &partial, &Dir::Infer);
        assert_eq!(inferred, Ok(Ty::Comp(const_residual())));
    }

    /// The residual `Integer → F Integer` (`K` with one argument consumed).
    /// Built from `integer()` (the `Integer` rigid atom the native primitives
    /// and the integer-literal rule use) — *not* the generators' `int()`, which
    /// is the unrelated `Int` atom.
    fn const_residual() -> CompType
    {
        CompType::arrow(integer(), CompType::returner(integer()))
    }

    /// Rule Native⇑ for the band-01-rung-07 additions: each new primitive
    /// infers its declared type, checker ≡ machine, and one application peels
    /// exactly one arrow (`int.div 6 : Unknown → F Unknown`).
    #[test]
    fn rung07_primitives_infer_their_declared_types()
    {
        for prim in [
            NativePrim::Div,
            NativePrim::Mod,
            NativePrim::Not,
            NativePrim::ListLength,
            NativePrim::ListAt,
            NativePrim::StringAppend,
            NativePrim::StringLength,
        ] {
            let inferred = agree_comp(&Ctx::new(), &Comp::native(prim), &Dir::Infer);
            assert_eq!(
                inferred,
                Ok(Ty::Comp(prim.declared_type())),
                "a source native infers its declared type (checker ≡ machine)"
            );
        }
        let partial = Comp::app(Comp::native(NativePrim::Div), Value::int(6));
        let expected = CompType::arrow(ValueType::Unknown, CompType::returner(ValueType::Unknown));
        assert_eq!(
            agree_comp(&base_ctx(), &partial, &Dir::Infer),
            Ok(Ty::Comp(expected)),
            "one consumed argument peels one arrow off the declared type"
        );
    }
}

/// The source-facing combinators (list iteration/update, record
/// access/update, string helpers, regex extraction, and path helpers): the
/// typing axiom (`checker ≡ machine`, via [`agree_comp`]) that a v0 combinator
/// closure is pure, so an effectful closure is rejected at the pure-returner
/// row leg.
///
/// The higher-order combinators' **evaluation** — their unroll into a closed
/// term, the string / path / regex builtins, and the assert + deep-handler
/// test runner dogfooding A3 (`perform` / `handle` / `resume`) — is
/// machine-only and pinned on the L machine in
/// `gandr_core_sequent::conformance_soundness` (the retired reference
/// evaluator's frozen outcome snapshots).
mod combinators
{
    use gandr_core_checker::effect::EffectOp;
    use gandr_core_checker::effect::EffectSig;
    use gandr_core_checker::prim::NativePrim;

    use super::*;
    /// A v0 combinator closure is **pure**: its codomain is a pure returner `F
    /// ?` (empty row), so an *effectful* closure fails the row leg of subtyping
    /// (`⟨E⟩ ⊄ ⟨⟩`) — a type error, not a silently dropped effect. (Effect
    /// polymorphism — propagating the closure's row to the combinator's result
    /// — awaits the `+poly` row variable.)
    #[test]
    fn an_effectful_closure_is_rejected_by_a_pure_combinator()
    {
        let sig = EffectSig::new(
            gandr_core_checker::boundary::EffectSignatureName::from("Bang"),
            vec![EffectOp::new(
                gandr_core_checker::boundary::OperationName::from("bang"),
                ValueType::Unknown,
                ValueType::Unknown,
            )],
        );
        let effectful = Value::thunk(
            Grade::OMEGA,
            Comp::lam("x", Comp::perform(sig, "bang", Value::var("x"))),
        );
        let app = Comp::app(
            Comp::app(Comp::native(NativePrim::Each), effectful),
            ints(&[1]),
        );
        assert!(
            agree_comp(&Ctx::new(), &app, &Dir::Infer).is_err(),
            "an effectful closure violates the pure-returner row leg (checker ≡ machine)"
        );
    }

    /// The pure-returner row leg rejects an effectful closure for EVERY
    /// combinator, not only `each` — so a future edit relaxing any of them to
    /// a non-empty returner row cannot pass silently.
    #[test]
    fn every_pure_combinator_rejects_an_effectful_closure()
    {
        let sig = || {
            EffectSig::new(
                gandr_core_checker::boundary::EffectSignatureName::from("Bang"),
                vec![EffectOp::new(
                    gandr_core_checker::boundary::OperationName::from("bang"),
                    ValueType::Unknown,
                    ValueType::Unknown,
                )],
            )
        };
        let unary = Value::thunk(
            Grade::OMEGA,
            Comp::lam("x", Comp::perform(sig(), "bang", Value::var("x"))),
        );
        for prim in [NativePrim::Where, NativePrim::Any, NativePrim::All] {
            let app = Comp::app(Comp::app(Comp::native(prim), unary.clone()), ints(&[1]));
            assert!(
                agree_comp(&Ctx::new(), &app, &Dir::Infer).is_err(),
                "an effectful predicate must fail the pure-returner row leg (checker ≡ machine)"
            );
        }
        // `reduce` consumes a BINARY closure `λacc. λx. …`.
        let binary = Value::thunk(
            Grade::OMEGA,
            Comp::lam(
                "acc",
                Comp::lam("x", Comp::perform(sig(), "bang", Value::var("x"))),
            ),
        );
        let reduce_app = Comp::app(
            Comp::app(
                Comp::app(Comp::native(NativePrim::Reduce), binary),
                Value::int(0),
            ),
            ints(&[1]),
        );
        assert!(
            agree_comp(&Ctx::new(), &reduce_app, &Dir::Infer).is_err(),
            "an effectful reducer must fail the pure-returner row leg (checker ≡ machine)"
        );
    }

    /// Rule Native⇑: a combinator infers its declared type, checker ≡ machine
    /// (the typing face the operational tests above do not reach).
    #[test]
    fn combinators_infer_their_declared_types()
    {
        for prim in [
            NativePrim::Each,
            NativePrim::Where,
            NativePrim::Reduce,
            NativePrim::Any,
            NativePrim::All,
            NativePrim::Flatten,
            NativePrim::Uniq,
            NativePrim::Sort,
            NativePrim::Get,
            NativePrim::Insert,
        ] {
            let inferred = agree_comp(&Ctx::new(), &Comp::native(prim), &Dir::Infer);
            assert_eq!(
                inferred,
                Ok(Ty::Comp(prim.declared_type())),
                "a source combinator infers its declared type (checker ≡ machine)"
            );
        }
    }

    /// A list value from bare integers.
    fn ints<'source, V>(values: V) -> Value
    where
        V: Into<I64Slice<'source>>,
    {
        let values = values.into();
        Value::list(values.as_ref().iter().copied().map(Value::int).collect())
    }
}

/// Identity types (ADR-76): witnesses that the recursive checker and the typing
/// machine agree on `Path` / `here` / `walk`, including the value-into-type
/// motive substitution and the without-K negative cases. `Path`/`here`/`walk`
/// are not in the property generators (a documented residual), so these are
/// curated positive and negative rows exercised through the shared
/// `agree_value` / `agree_comp` differential.
mod identity_types
{
    use gandr_core_checker::syntax::WalkBase;
    use gandr_core_checker::syntax::WalkMotive;

    use super::*;

    /// Rule Here⇑: `here(7)` infers `Path Integer 7 7` on both engines.
    #[test]
    fn here_infers_path_type()
    {
        let here = Value::here(Value::int(7));
        let expected = ValueType::path(ValueType::integer(), Value::int(7), Value::int(7));
        assert_eq!(
            agree_value(&Ctx::new(), &here, &Dir::Infer),
            Ok(Ty::Value(expected)),
            "here(7) must infer Path Integer 7 7"
        );
    }

    /// Rule Here⇓: `here(7)` checks against a matching `Path Integer 7 7`.
    #[test]
    fn here_checks_against_matching_path()
    {
        let here = Value::here(Value::int(7));
        let expected = ValueType::path(ValueType::integer(), Value::int(7), Value::int(7));
        assert_eq!(
            agree_value(&Ctx::new(), &here, &Dir::Check(expected.clone())),
            Ok(Ty::Value(expected)),
            "here(7) must check against Path Integer 7 7"
        );
    }

    /// The `Path` subtyping arm is **invariant** in the endpoints: `here(7)` is
    /// rejected against `Path Integer 7 8` on both engines (endpoint
    /// inequality).
    #[test]
    fn here_rejects_unequal_endpoints()
    {
        let here = Value::here(Value::int(7));
        let wrong = ValueType::path(ValueType::integer(), Value::int(7), Value::int(8));
        assert!(
            agree_value(&Ctx::new(), &here, &Dir::Check(wrong)).is_err(),
            "here(7) must not check against Path Integer 7 8"
        );
    }

    /// The `back`-at-a-point eliminator infers its instantiated motive on both
    /// engines: `walk(here(7), (x y q). F(Path Integer y x), (x). ret here(x))`
    /// infers `F(Path Integer 7 7)`. The motive binds **both** endpoints, so
    /// this row exercises the value-into-type substitution
    /// (`gandr_core_checker::identity`).
    #[test]
    fn walk_back_infers_instantiated_motive()
    {
        let motive = WalkMotive::new(
            "x",
            "y",
            "q",
            CompType::returner(ValueType::path(
                ValueType::integer(),
                Value::var("y"),
                Value::var("x"),
            )),
        );
        let base = WalkBase::new("x", Comp::ret(Value::here(Value::var("x"))));
        let j = Comp::walk(Value::here(Value::int(7)), motive, base);
        let expected = CompType::returner(ValueType::path(
            ValueType::integer(),
            Value::int(7),
            Value::int(7),
        ));
        assert_eq!(
            agree_comp(&Ctx::new(), &j, &Dir::Infer),
            Ok(Ty::Comp(expected)),
            "walk(here 7, back-motive, ret here) must infer F(Path Integer 7 7)"
        );
    }

    /// A constant-motive `walk` (transport into `F Integer`) infers `F Integer`
    /// on both engines — the base checks against the constant diagonal.
    #[test]
    fn walk_constant_motive_infers_result()
    {
        let motive = WalkMotive::new("x", "y", "q", CompType::returner(ValueType::integer()));
        let base = WalkBase::new("x", Comp::ret(Value::var("x")));
        let j = Comp::walk(Value::here(Value::int(7)), motive, base);
        assert_eq!(
            agree_comp(&Ctx::new(), &j, &Dir::Infer),
            Ok(Ty::Comp(CompType::returner(ValueType::integer()))),
            "constant-motive walk must infer F Integer"
        );
    }

    /// Rule Walk shape check: a non-identity scrutinee is a shape mismatch on
    /// both engines (the scrutinee inferred `Integer`, not a `Path`).
    #[test]
    fn walk_rejects_non_path_scrutinee()
    {
        let motive = WalkMotive::new("x", "y", "q", CompType::returner(ValueType::integer()));
        let base = WalkBase::new("x", Comp::ret(Value::var("x")));
        let j = Comp::walk(Value::int(7), motive, base);
        assert!(
            agree_comp(&Ctx::new(), &j, &Dir::Infer).is_err(),
            "walk on a non-identity scrutinee must be a shape mismatch"
        );
    }

    /// The K-rejection discipline's core face (ADR-76): a `case` scrutinizing
    /// an identity type is rejected on both engines with the reserved
    /// here-pattern diagnostic, whose message carries the literal `without-k`
    /// substring the corpus witness asserts.
    #[test]
    fn case_on_identity_rejects_with_the_without_k_diagnostic()
    {
        let scrut = Value::annot(
            Value::here(Value::int(7)),
            ValueType::path(ValueType::integer(), Value::int(7), Value::int(7)),
        );
        let case = Comp::case(
            scrut,
            "a",
            Comp::ret(Value::int(0)),
            "b",
            Comp::ret(Value::int(1)),
        );
        let result = agree_comp(
            &Ctx::new(),
            &case,
            &Dir::Check(CompType::returner(ValueType::integer())),
        );
        let Err(error) = result
        else {
            panic!("case on an identity type must be rejected (without-K, ADR-76)");
        };
        assert!(
            error.to_string().contains("without-k"),
            "the diagnostic must carry the literal `without-k` substring, got: {error}"
        );
    }
}
