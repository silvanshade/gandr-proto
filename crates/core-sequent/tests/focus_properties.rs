//! Property and coverage tests for the focusing translation `𝓕`
//! (`proposal-sequent-kernel.md` §3, phase-L0 gate).
//!
//! No free (input-free) generator of well-formed core *terms* is exported
//! anywhere: `gandr-core-checker-tools`'s `strategies` module carries only
//! type / grade / name generators, and the type-directed well-typed term
//! generators stay beside the conformance target that drives them rather than
//! becoming library surface (**noted gap**). So this file builds its own
//! scope-threaded generator of well-formed *closed* core computations (covering
//! the pure spine plus the grade, control-delimiter, and native formers) and
//! asserts `𝓕` is total and typed-IL-well-formed on them, complemented by
//! hand-built cases that reach every remaining former — the effect / handler /
//! resumption / reified-stack surface the random generator does not build.

/// Property and coverage tests.
#[cfg(test)]
mod tests
{
    use gandr_core_sequent::FocusOrigin;
    use gandr_core_sequent::focus_comp;
    use gandr_core_sequent::focus_value;
    use gandr_core_sequent::inspect;
    use gandr_core_sequent::pretty;
    use gandr_core_sequent::wellformed;
    use gandr_core_term::boundary::EffectSignatureName;
    use gandr_core_term::boundary::GenerationDepth;
    use gandr_core_term::boundary::NameRef;
    use gandr_core_term::boundary::OperationName;
    use gandr_core_term::effect::EffectOp;
    use gandr_core_term::effect::EffectSig;
    use gandr_core_term::grade::Grade;
    use gandr_core_term::prim::NativePrim;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::OpClause;
    use gandr_core_term::syntax::Stack;
    use gandr_core_term::syntax::Value;
    use gandr_core_term::syntax::WalkBase;
    use gandr_core_term::syntax::WalkMotive;
    use gandr_core_term::types::CompType;
    use gandr_core_term::types::DataId;
    use gandr_core_term::types::ValueType;
    use proptest::prelude::*;
    use proptest::strategy::BoxedStrategy;
    use proptest::strategy::Union;

    /// A generator of well-formed closed values under the in-scope binders
    /// `scope`.
    ///
    /// # Termination
    /// - reason: builds a finite proptest strategy tree down to scalar leaves.
    /// - measure: [`GenerationDepth`] strictly decreases before recursive
    ///   strategy construction.
    /// - boundedness: [`gen_depth`] supplies a small fixed ceiling for every
    ///   test invocation.
    /// - input recursion: `scope` and `depth` flow into recursive strategy
    ///   construction; `depth` strictly bounds the expansion.
    #[cfg_attr(
        dylint_lib = "gandr_workflow_dylint",
        allow(
            unknown_lints,
            recursive_function_needs_termination,
            reason = "test-only proptest expansion is explicitly fuel-bounded; input-carrying recursion outside the model checker opts out at the narrowest function scope"
        )
    )]
    fn arb_value(
        scope: &[String],
        depth: GenerationDepth,
    ) -> BoxedStrategy<Value>
    {
        let mut choices: Vec<BoxedStrategy<Value>> = vec![
            Just(Value::Unit).boxed(),
            any::<i64>().prop_map(Value::int).boxed(),
            prop::sample::select(vec!["", "hi", "gandr"])
                .prop_map(Value::string)
                .boxed(),
            Just(Value::hole(0)).boxed(),
        ];
        let depth_value = u32::from(depth);
        if !scope.is_empty() {
            let scope_names = Vec::from(scope);
            choices.push(
                prop::sample::select(scope_names)
                    .prop_map(|name| Value::var(&name))
                    .boxed(),
            );
        }
        if depth_value > 0 {
            let below: GenerationDepth = depth_value.saturating_sub(1).into();
            choices.push(
                (arb_value(scope, below), arb_value(scope, below))
                    .prop_map(|(fst, snd)| Value::pair(fst, snd))
                    .boxed(),
            );
            choices.push(arb_value(scope, below).prop_map(Value::inj1).boxed());
            choices.push(arb_value(scope, below).prop_map(Value::inj2).boxed());
            choices.push(
                prop::collection::vec(arb_value(scope, below), 0 .. 3)
                    .prop_map(Value::list)
                    .boxed(),
            );
            choices.push(
                (arb_value(scope, below), arb_value(scope, below))
                    .prop_map(|(a, b)| {
                        Value::record([(String::from("a"), a), (String::from("b"), b)])
                    })
                    .boxed(),
            );
            choices.push(
                arb_comp(scope, below)
                    .prop_map(|body| Value::thunk(Grade::ONE, body))
                    .boxed(),
            );
            choices.push(
                arb_value(scope, below)
                    .prop_map(|value| Value::annot(value, ValueType::integer()))
                    .boxed(),
            );
        }
        Union::new(choices).boxed()
    }

    /// A generator of well-formed closed computations under `scope`.
    ///
    /// # Termination
    /// - reason: builds a finite proptest strategy tree down to pure-spine
    ///   computation leaves.
    /// - measure: [`GenerationDepth`] strictly decreases before recursive
    ///   strategy construction.
    /// - boundedness: [`gen_depth`] supplies a small fixed ceiling for every
    ///   test invocation.
    /// - input recursion: `scope` and `depth` flow into recursive strategy
    ///   construction; `depth` strictly bounds the expansion.
    #[cfg_attr(
        dylint_lib = "gandr_workflow_dylint",
        allow(
            unknown_lints,
            recursive_function_needs_termination,
            reason = "test-only proptest expansion is explicitly fuel-bounded; input-carrying recursion outside the model checker opts out at the narrowest function scope"
        )
    )]
    fn arb_comp(
        scope: &[String],
        depth: GenerationDepth,
    ) -> BoxedStrategy<Comp>
    {
        let mut choices: Vec<BoxedStrategy<Comp>> = vec![
            arb_value(scope, depth).prop_map(Comp::ret).boxed(),
            arb_value(scope, depth).prop_map(Comp::force).boxed(),
            Just(Comp::hole(0)).boxed(),
            Just(Comp::native(NativePrim::Id)).boxed(),
        ];
        let depth_value = u32::from(depth);
        if depth_value > 0 {
            let below: GenerationDepth = depth_value.saturating_sub(1).into();

            let abs_scope = extended(scope, &["x".into()]);
            choices.push(
                arb_comp(&abs_scope, below)
                    .prop_map(|body| Comp::lam("x", body))
                    .boxed(),
            );

            choices.push(
                (arb_comp(scope, below), arb_value(scope, below))
                    .prop_map(|(head, arg)| Comp::app(head, arg))
                    .boxed(),
            );

            let bind_scope = extended(scope, &["y".into()]);
            choices.push(
                (arb_comp(scope, below), arb_comp(&bind_scope, below))
                    .prop_map(|(bound, cont)| Comp::bind(bound, "y", cont))
                    .boxed(),
            );

            let left_scope = extended(scope, &["l".into()]);
            let right_scope = extended(scope, &["r".into()]);
            choices.push(
                (
                    arb_value(scope, below),
                    arb_comp(&left_scope, below),
                    arb_comp(&right_scope, below),
                )
                    .prop_map(|(scrut, left, right)| Comp::case(scrut, "l", left, "r", right))
                    .boxed(),
            );

            let split_scope = extended(scope, &["a".into(), "b".into()]);
            choices.push(
                (arb_value(scope, below), arb_comp(&split_scope, below))
                    .prop_map(|(scrut, body)| Comp::split(scrut, "a", "b", body))
                    .boxed(),
            );

            let cons_scope = extended(scope, &["h".into(), "t".into()]);
            choices.push(
                (
                    arb_value(scope, below),
                    arb_comp(scope, below),
                    arb_comp(&cons_scope, below),
                )
                    .prop_map(|(scrut, nil, cons)| Comp::list_case(scrut, nil, "h", "t", cons))
                    .boxed(),
            );

            choices.push(
                arb_value(scope, below)
                    .prop_map(|record| Comp::record_proj(record, "a"))
                    .boxed(),
            );

            choices.push(
                (arb_comp(scope, below), arb_comp(scope, below))
                    .prop_map(|(fst, snd)| Comp::with(fst, snd))
                    .boxed(),
            );
            choices.push(arb_comp(scope, below).prop_map(Comp::prj1).boxed());
            choices.push(arb_value(scope, below).prop_map(Comp::dup).boxed());
            choices.push(arb_value(scope, below).prop_map(Comp::drop).boxed());
            choices.push(arb_comp(scope, below).prop_map(Comp::reset).boxed());

            let shift_scope = extended(scope, &["k".into()]);
            choices.push(
                arb_comp(&shift_scope, below)
                    .prop_map(|body| Comp::shift("k", body))
                    .boxed(),
            );
        }
        Union::new(choices).boxed()
    }

    /// The generator's recursion ceiling (kept small — the strategy tree is
    /// built eagerly, so depth trades directly against construction cost).
    fn gen_depth() -> GenerationDepth
    {
        3_u32.into()
    }

    /// Extends a scope with fresh binder names.
    fn extended(
        scope: &[String],
        names: &[NameRef<'_>],
    ) -> Vec<String>
    {
        let mut out = scope.to_vec();
        for name in names {
            out.push(String::from(name.as_ref()));
        }
        out
    }

    proptest! {
        /// `𝓕` is total and typed-IL-well-formed on generated closed computations,
        /// with no free variables or covariables.
        #[test]
        fn focusing_is_total_on_generated_computations(comp in arb_comp(&[], gen_depth()))
        {
            let focused = focus_comp(&comp).expect("𝓕 is total on well-formed core terms");
            let frees = wellformed(focused.arena(), focused.root())
                .expect("the focused command is typed-IL well-formed");
            prop_assert!(
                frees.covars.is_empty(),
                "a 𝓕-produced command has no free covariables, found {:?}",
                frees.covars
            );
            prop_assert!(
                frees.vars.is_empty(),
                "a closed source term focuses to a command with no free variables, found {:?}",
                frees.vars
            );
        }
    }

    /// Every remaining former — the effect / control / native / reified-stack
    /// surface the random generator does not build — focuses total and
    /// well-formed, carries the expected provenance, and renders the
    /// expected IL head.
    #[test]
    fn hand_built_cases_cover_every_former()
    {
        // (label, term, expected provenance of the root or a nested command,
        //  expected substring in the rendered IL).
        let cases: Vec<(&str, Comp, FocusOrigin, &str)> = vec![
            (
                "perform",
                Comp::perform(state_sig(), "get", Value::Unit),
                FocusOrigin::Perform,
                "Op[State.get]",
            ),
            (
                "handle",
                Comp::handle(
                    state_sig(),
                    Comp::perform(state_sig(), "get", Value::Unit),
                    "x",
                    Comp::ret(Value::var("x")),
                    vec![OpClause::new("get", "p", "k", Comp::ret(Value::var("p")))],
                ),
                FocusOrigin::Perform,
                "handler[State]",
            ),
            (
                "reset-shift",
                Comp::reset(Comp::shift("k", Comp::ret(Value::var("k")))),
                FocusOrigin::Shift,
                "prompt(",
            ),
            (
                "resume",
                Comp::resume(
                    Value::stk(Stack::arg(Value::int(1), Stack::empty())),
                    Comp::ret(Value::Unit),
                ),
                FocusOrigin::Resume,
                "resume(",
            ),
            (
                "native",
                Comp::native(NativePrim::Add),
                FocusOrigin::Native,
                "native[add]",
            ),
            (
                "with-prj",
                Comp::prj1(Comp::with(Comp::ret(Value::Unit), Comp::ret(Value::int(2)))),
                FocusOrigin::With,
                "cocase { prj1(",
            ),
            (
                "dup",
                Comp::dup(Value::thunk(Grade::ONE, Comp::ret(Value::Unit))),
                FocusOrigin::Dup,
                "dup(",
            ),
            (
                "drop",
                Comp::drop(Value::thunk(Grade::ONE, Comp::ret(Value::Unit))),
                FocusOrigin::Drop,
                "drop(",
            ),
            (
                "list-case",
                Comp::list_case(
                    Value::list(vec![Value::int(1), Value::int(2)]),
                    Comp::ret(Value::Unit),
                    "h",
                    "t",
                    Comp::ret(Value::var("h")),
                ),
                FocusOrigin::ListCase,
                "Nil(; ) ⇒",
            ),
            (
                "split",
                Comp::split(
                    Value::pair(Value::int(1), Value::int(2)),
                    "a",
                    "b",
                    Comp::ret(Value::var("a")),
                ),
                FocusOrigin::Split,
                "Pair(a, b; ) ⇒",
            ),
            (
                "record-proj",
                Comp::record_proj(Value::record([(String::from("a"), Value::int(1))]), "a"),
                FocusOrigin::RecordProj,
                ".a(",
            ),
            (
                "walk",
                Comp::walk(
                    Value::here(Value::int(7)),
                    WalkMotive::new("x", "y", "q", CompType::returner(ValueType::integer())),
                    WalkBase::new("x", Comp::ret(Value::var("x"))),
                ),
                FocusOrigin::Walk,
                "Here(x; ) ⇒",
            ),
            (
                "data-case",
                Comp::data_case(
                    Value::ctor(DataId::new(0_u64, "D"), 0_usize, Value::int(1)),
                    vec![(String::from("y"), Comp::ret(Value::var("y")))],
                ),
                FocusOrigin::DataCase,
                "Data[0](y; ) ⇒",
            ),
            (
                "stk-value",
                Comp::ret(Value::stk(Stack::bind(
                    "x",
                    Comp::ret(Value::var("x")),
                    Stack::prj1(Stack::empty()),
                ))),
                FocusOrigin::Ret,
                "box(μ̃x.",
            ),
        ];

        for (label, comp, expected_origin, expected_render) in cases {
            let focused = focus_comp(&comp)
                .unwrap_or_else(|error| panic!("case `{label}`: 𝓕 must be total, got {error}"));
            let frees = wellformed(focused.arena(), focused.root())
                .unwrap_or_else(|error| panic!("case `{label}`: typed-IL check failed: {error}"));
            assert!(
                frees.covars.is_empty(),
                "case `{label}`: no free covariables, found {:?}",
                frees.covars
            );
            let rendered = pretty::render_command(focused.arena(), focused.root());
            assert!(
                rendered.contains(expected_render),
                "case `{label}`: rendered IL `{rendered}` should contain `{expected_render}`"
            );
            let histogram = inspect::origin_histogram(&focused);
            assert!(
                histogram.contains_key(&expected_origin),
                "case `{label}`: provenance histogram should record the {expected_render} origin"
            );
        }
    }

    /// A fixed one-operation effect signature `State { get : () ↠ Integer }`.
    fn state_sig() -> EffectSig
    {
        EffectSig::new(EffectSignatureName::from("State"), vec![EffectOp::new(
            OperationName::from("get"),
            ValueType::Unit,
            ValueType::integer(),
        )])
    }

    /// A top-level value term focuses to a positive cut against `★`.
    #[test]
    fn top_level_value_focuses_against_top()
    {
        let focused = focus_value(&Value::int(7)).expect("𝓕 is total on values");
        let frees = wellformed(focused.arena(), focused.root()).expect("well-formed");
        assert!(frees.covars.is_empty(), "no free covariables");
        assert_eq!(
            "⟨7 |+ ★⟩",
            pretty::render_command(focused.arena(), focused.root()),
            "a top-level value is a positive cut against the terminal consumer"
        );
        assert_eq!(
            Some(FocusOrigin::TopValue),
            focused.origin(focused.root()),
            "its provenance is the top-level value"
        );
    }
}
