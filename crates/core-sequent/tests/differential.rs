//! The phase-L1 gate (`proposal-sequent-kernel.md` §9, phase L1): the L machine
//! agrees with the CEK oracle — `L-run ∘ 𝓕 ≡ run`.
//!
//! # Adequacy discipline (ADR-71)
//!
//! The **external oracle** is the CEK machine
//! (`gandr_core_checker::eval::run_comp`): a distinct implementation of the
//! same operational semantics, sharing no step code with the L machine
//! (`gandr_core_sequent::machine::run_comp`). The **adequacy hypothesis**: for
//! a checked-core computation `t`, `run(t)` and `L-run(𝓕(t))` denote the same
//! observable outcome, so agreement is evidence for the L
//! machine's correctness. Comparison is on the canonicalized [`Eval`] outcome
//! ([`gandr_core_sequent::differential::canonical`]) — the outcome kind, the
//! stuck reason, and the returned value exactly on the first-order fragment
//! (the observable fragment the corpus checks), with higher-order terminals
//! (returned thunks, bare functions / lazy pairs) compared at kind granularity
//! (the un-focusing readback residual; see the module docs).
//!
//! # Coverage at this checkpoint
//!
//! The generator and hand-built cases exercise the **pure CBPV spine** the L
//! machine realizes: `ret` / `bind` / `force` (call-by-need) / the positive
//! eliminations (`case` / `split` / `listcase` / `recordproj`) / the negative
//! intros and eliminations (`λ` / lazy pairs, application / projection) / the
//! grade structural ops (`dup` / `drop`) / holes / the native registry, **and
//! the effect / control surface** (`perform` / `handle` / `resume` / `reset` /
//! `shift`): unhandled and handled operations, resuming and non-resuming
//! clauses, deep re-entry, nested-handler routing, the v0 single-handler scope,
//! reset transparency / delimiting, and — now faithful —
//! `shift`'s delimited capture: a `reset`-delimited `shift`
//! that discards or `resume`s its captured continuation, and an undelimited
//! `shift` (agreeing on the `ShiftNoReset` blame). A captured continuation used
//! in *value* position stays excluded (its representation diverges from the
//! CEK's α-renamed side-table name; a listed residual), so a generated `shift`
//! body only ever `resume`s `k` or ignores it. The prelude resolution and the
//! higher-order combinators grow the differential in later checkpoints.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the differential harness readable \
                  (docs/workflow/rust.md)"
    )
)]

#[cfg(test)]
mod tests
{
    use gandr_core_checker::boundary::EffectSignatureName;
    use gandr_core_checker::boundary::GenerationDepth;
    use gandr_core_checker::boundary::NameRef;
    use gandr_core_checker::boundary::OperationName;
    use gandr_core_checker::effect::EffectSig;
    use gandr_core_checker::eval::Blame;
    use gandr_core_checker::eval::Eval;
    use gandr_core_checker::eval::run_comp;
    use gandr_core_checker::eval::run_with_host;
    use gandr_core_checker::grade::Grade;
    use gandr_core_checker::host::HostHandler;
    use gandr_core_checker::host::HostReply;
    use gandr_core_checker::prim::NativePrim;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::OpClause;
    use gandr_core_checker::syntax::Value;
    use gandr_core_sequent::differential::agree;
    use gandr_core_sequent::differential::canonical;
    use gandr_core_sequent::machine;
    use proptest::prelude::*;
    use proptest::strategy::BoxedStrategy;
    use proptest::strategy::Union;

    /// A generator of well-formed closed **first-order-ish** values under the
    /// in-scope binders `scope` (the pure fragment the L machine realizes; no
    /// native / control / reified-stack value).
    ///
    /// # Termination
    /// - reason: builds a finite proptest strategy tree down to scalar leaves.
    /// - measure: [`GenerationDepth`] strictly decreases before recursive
    ///   strategy construction.
    /// - boundedness: [`gen_depth`] supplies a small fixed ceiling for property
    ///   inputs.
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
                    .prop_map(|value| {
                        Value::annot(value, gandr_core_checker::types::ValueType::integer())
                    })
                    .boxed(),
            );
        }
        Union::new(choices).boxed()
    }

    /// A generator of well-formed closed **pure-spine** computations under
    /// `scope`.
    ///
    /// # Termination
    /// - reason: builds a finite proptest strategy tree down to pure-spine
    ///   computation leaves.
    /// - measure: [`GenerationDepth`] strictly decreases before recursive
    ///   strategy construction.
    /// - boundedness: [`gen_depth`] supplies a small fixed ceiling for property
    ///   inputs.
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
            // A bare native is a curried function terminal. `Add` blames on a
            // non-numeric argument regardless of its body, so applying it to an
            // arbitrary (possibly thunk-carrying) generated value still agrees;
            // a PASS-THROUGH native (`id` / `const`) is excluded here because it
            // would return a thunk the first-order readback cannot rebuild — it
            // is exercised through the first-order applications below instead.
            Just(Comp::native(NativePrim::Add)).boxed(),
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
            choices.push(arb_comp(scope, below).prop_map(Comp::prj2).boxed());
            choices.push(arb_value(scope, below).prop_map(Comp::dup).boxed());
            choices.push(arb_value(scope, below).prop_map(Comp::drop).boxed());

            // Unary natives applied to a first-order argument.
            for prim in [NativePrim::Id, NativePrim::Neg] {
                choices.push(
                    arb_first_order(scope, below)
                        .prop_map(move |arg| Comp::app(Comp::native(prim), arg))
                        .boxed(),
                );
            }
            // Binary natives, partially and fully applied to first-order
            // arguments (the partial application is a curried terminal).
            for prim in [
                NativePrim::Add,
                NativePrim::Sub,
                NativePrim::Mul,
                NativePrim::Eq,
                NativePrim::Lt,
            ] {
                choices.push(
                    arb_first_order(scope, below)
                        .prop_map(move |arg| Comp::app(Comp::native(prim), arg))
                        .boxed(),
                );
                choices.push(
                    (arb_first_order(scope, below), arb_first_order(scope, below))
                        .prop_map(move |(lhs, rhs)| {
                            Comp::app(Comp::app(Comp::native(prim), lhs), rhs)
                        })
                        .boxed(),
                );
            }

            // Effect / control (all over the single operation `op` — routing is
            // by operation name, ADR-33 D3): a `perform`, a `handle` with a
            // non-resuming clause, a `handle` with a resuming clause, a `reset`,
            // and (below) `shift` — now faithful. A resuming clause
            // resumes with a PURE returner (a returned value, never another
            // `perform op`), so a deep re-entry resolves a bounded (AST-sized)
            // number of performs and cannot loop.
            choices.push(
                arb_value(scope, below)
                    .prop_map(|payload| Comp::perform(eff_sig(), "op", payload))
                    .boxed(),
            );
            // The clause bodies bind the payload `ep` (an ordinary value, bound
            // identically on both machines) but NOT the resumption `ek`: `ek` is
            // a captured continuation, whose representation in *value* position
            // diverges (the CEK α-renames it to a fresh neutral name in a side
            // table; the L machine binds it a boxed stack), so returning `ek` as
            // data is a listed residual — the resumption is exercised only in
            // `resume ek …` position (the explicit resuming clause below).
            let clause_scope = extended(scope, &["ep".into()]);
            let ret_scope = extended(scope, &["ex".into()]);
            choices.push(
                (
                    arb_comp(scope, below),
                    arb_comp(&ret_scope, below),
                    arb_comp(&clause_scope, below),
                )
                    .prop_map(|(body, ret_body, clause_body)| {
                        Comp::handle(eff_sig(), body, "ex", ret_body, vec![OpClause::new(
                            "op",
                            "ep",
                            "ek",
                            clause_body,
                        )])
                    })
                    .boxed(),
            );
            choices.push(
                (
                    arb_comp(scope, below),
                    arb_comp(&ret_scope, below),
                    arb_value(&clause_scope, below),
                )
                    .prop_map(|(body, ret_body, fed)| {
                        Comp::handle(eff_sig(), body, "ex", ret_body, vec![OpClause::new(
                            "op",
                            "ep",
                            "ek",
                            Comp::resume(Value::var("ek"), Comp::ret(fed)),
                        )])
                    })
                    .boxed(),
            );
            choices.push(arb_comp(scope, below).prop_map(Comp::reset).boxed());

            // `shift` — now faithful. Two safe shapes (a captured
            // continuation in *value* position is a listed residual, so `k` is
            // never returned as data): (1) `shift k. body` whose body is a pure
            // comp under `scope` (WITHOUT `k`, so the capture is discarded); and
            // (2) `shift k. resume k (ret fed)` (the captured continuation is
            // re-invoked once with a pure value). Under an enclosing `reset` the
            // capture reaches the prompt; undelimited (or across a handler) both
            // machines agree on the `ShiftNoReset` blame.
            choices.push(
                arb_comp(scope, below)
                    .prop_map(|body| Comp::shift("k", body))
                    .boxed(),
            );
            choices.push(
                arb_value(scope, below)
                    .prop_map(|fed| Comp::shift("k", Comp::resume(Value::var("k"), Comp::ret(fed))))
                    .boxed(),
            );
        }
        Union::new(choices).boxed()
    }

    /// The generator's recursion ceiling (kept small — the strategy tree is
    /// built eagerly).
    fn gen_depth() -> GenerationDepth
    {
        3_u32.into()
    }

    /// The single effect signature the generated effect programs perform and
    /// handle over (the operation types are inert to `run_comp`, which routes
    /// by operation name — ADR-33 D3).
    fn eff_sig() -> gandr_core_checker::effect::EffectSig
    {
        use gandr_core_checker::effect::EffectOp;
        use gandr_core_checker::effect::EffectSig;
        use gandr_core_checker::types::ValueType;
        EffectSig::new(EffectSignatureName::from("E"), vec![EffectOp::new(
            OperationName::from("op"),
            ValueType::integer(),
            ValueType::integer(),
        )])
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

    /// A generator of **first-order** values (no thunk) — the arguments a
    /// native dispatch reads back exactly. Applying a native to a thunk
    /// that the native passes through (`id` / `const`) can only be read
    /// back with un-focusing, so the property differential feeds natives
    /// first-order arguments; the corpus differential exercises the rest
    /// tolerantly.
    ///
    /// # Termination
    /// - reason: builds a finite proptest strategy tree down to first-order
    ///   leaves.
    /// - measure: [`GenerationDepth`] strictly decreases before recursive
    ///   strategy construction.
    /// - boundedness: [`gen_depth`] supplies a small fixed ceiling for property
    ///   inputs.
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
    fn arb_first_order(
        scope: &[String],
        depth: GenerationDepth,
    ) -> BoxedStrategy<Value>
    {
        let mut choices: Vec<BoxedStrategy<Value>> = vec![
            Just(Value::Unit).boxed(),
            any::<i64>().prop_map(Value::int).boxed(),
            prop::sample::select(vec!["", "hi"])
                .prop_map(Value::string)
                .boxed(),
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
                (arb_first_order(scope, below), arb_first_order(scope, below))
                    .prop_map(|(fst, snd)| Value::pair(fst, snd))
                    .boxed(),
            );
            choices.push(arb_first_order(scope, below).prop_map(Value::inj1).boxed());
            choices.push(
                prop::collection::vec(arb_first_order(scope, below), 0 .. 3)
                    .prop_map(Value::list)
                    .boxed(),
            );
        }
        Union::new(choices).boxed()
    }

    /// Hand-built cases pinning each pure-spine former against the oracle.
    #[test]
    fn hand_built_pure_spine_cases_agree()
    {
        const PINNED_RETURN_INTEGER: i64 = 42;
        const BETA_ARGUMENT: i64 = 11;

        // ret + first-order data.
        assert_agree(&Comp::ret(Value::int(PINNED_RETURN_INTEGER)));
        assert_agree(&Comp::ret(Value::pair(Value::int(1), Value::string("s"))));
        assert_agree(&Comp::ret(Value::inj1(Value::Unit)));
        assert_agree(&Comp::ret(Value::list(vec![
            Value::int(1),
            Value::int(2),
            Value::int(3),
        ])));
        assert_agree(&Comp::ret(Value::record([(
            String::from("a"),
            Value::int(7),
        )])));

        // bind threads a value.
        assert_agree(&Comp::bind(
            Comp::ret(Value::int(3)),
            "y",
            Comp::ret(Value::var("y")),
        ));

        // force a thunk (call-by-need) and a re-forced (shared) thunk.
        assert_agree(&Comp::force(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(5)),
        )));
        assert_agree(&Comp::bind(
            Comp::ret(Value::thunk(Grade::OMEGA, Comp::ret(Value::int(9)))),
            "t",
            Comp::bind(
                Comp::force(Value::var("t")),
                "a",
                Comp::bind(
                    Comp::force(Value::var("t")),
                    "b",
                    Comp::ret(Value::pair(Value::var("a"), Value::var("b"))),
                ),
            ),
        ));

        // application (β), curried, and higher-order.
        assert_agree(&Comp::app(
            Comp::lam("x", Comp::ret(Value::var("x"))),
            Value::int(BETA_ARGUMENT),
        ));
        assert_agree(&Comp::app(
            Comp::app(
                Comp::lam("x", Comp::lam("y", Comp::ret(Value::var("x")))),
                Value::int(1),
            ),
            Value::int(2),
        ));

        // case on both injections.
        assert_agree(&Comp::case(
            Value::inj1(Value::int(1)),
            "l",
            Comp::ret(Value::var("l")),
            "r",
            Comp::ret(Value::int(0)),
        ));
        assert_agree(&Comp::case(
            Value::inj2(Value::int(2)),
            "l",
            Comp::ret(Value::int(0)),
            "r",
            Comp::ret(Value::var("r")),
        ));

        // split a pair.
        assert_agree(&Comp::split(
            Value::pair(Value::int(1), Value::int(2)),
            "a",
            "b",
            Comp::ret(Value::var("b")),
        ));

        // listcase (nil and cons).
        assert_agree(&Comp::list_case(
            Value::list(vec![]),
            Comp::ret(Value::int(0)),
            "h",
            "t",
            Comp::ret(Value::var("h")),
        ));
        assert_agree(&Comp::list_case(
            Value::list(vec![Value::int(1), Value::int(2)]),
            Comp::ret(Value::int(0)),
            "h",
            "t",
            Comp::ret(Value::var("h")),
        ));

        // record projection.
        assert_agree(&Comp::record_proj(
            Value::record([(String::from("a"), Value::int(8))]),
            "a",
        ));

        // lazy pair projection (both sides).
        assert_agree(&Comp::prj1(Comp::with(
            Comp::ret(Value::int(1)),
            Comp::ret(Value::int(2)),
        )));
        assert_agree(&Comp::prj2(Comp::with(
            Comp::ret(Value::int(1)),
            Comp::ret(Value::int(2)),
        )));

        // dup / drop (grade structural ops).
        assert_agree(&Comp::dup(Value::int(4)));
        assert_agree(&Comp::drop(Value::int(4)));

        // holes: a computation hole blames; a returned value hole is a value.
        assert_agree(&Comp::hole(0));
        assert_agree(&Comp::ret(Value::hole(0)));
        assert_agree(&Comp::bind(Comp::hole(0), "y", Comp::ret(Value::var("y"))));

        // ill-typed redexes must reach the same stuck reason on both machines.
        assert_agree(&Comp::app(Comp::ret(Value::int(1)), Value::int(2)));
        assert_agree(&Comp::force(Value::int(1)));
        assert_agree(&Comp::case(
            Value::int(1),
            "l",
            Comp::ret(Value::Unit),
            "r",
            Comp::ret(Value::Unit),
        ));
    }

    proptest! {
        // A wider case budget than the default — the differential is the phase's
        // core gate, and the pure-spine generator is cheap to sample.
        #![proptest_config(ProptestConfig::with_cases(4096))]

        /// `L-run ∘ 𝓕 ≡ run` on generated closed pure-spine computations.
        #[test]
        fn l_machine_agrees_with_the_cek_oracle(comp in arb_comp(&[], gen_depth()))
        {
            let oracle = run_comp(comp.clone());
            let machine = machine::run_comp(&comp);
            prop_assert!(
                bool::from(agree(&oracle, &machine)),
                "L-run ∘ 𝓕 ≢ run on {comp:?}\n  oracle    = {:?}\n  L machine = {:?}",
                canonical(&oracle),
                canonical(&machine)
            );
        }
    }

    /// Hand-built native-dispatch cases pinning the currying registry against
    /// the oracle.
    #[test]
    fn hand_built_native_cases_agree()
    {
        const FIRST_NATIVE_ARGUMENT: i64 = 10;
        const SECOND_NATIVE_ARGUMENT: i64 = 20;

        // Saturated arithmetic and comparison.
        assert_agree(&Comp::app(
            Comp::app(Comp::native(NativePrim::Add), Value::int(2)),
            Value::int(3),
        ));
        assert_agree(&Comp::app(
            Comp::app(Comp::native(NativePrim::Mul), Value::int(4)),
            Value::int(5),
        ));
        assert_agree(&Comp::app(
            Comp::app(Comp::native(NativePrim::Lt), Value::int(1)),
            Value::int(2),
        ));
        // Unary native.
        assert_agree(&Comp::app(Comp::native(NativePrim::Neg), Value::int(7)));
        // Identity passing through a first-order value.
        assert_agree(&Comp::app(
            Comp::native(NativePrim::Id),
            Value::pair(Value::int(1), Value::int(2)),
        ));
        // Bare and partial natives are curried function terminals.
        assert_agree(&Comp::native(NativePrim::Add));
        assert_agree(&Comp::app(Comp::native(NativePrim::Add), Value::int(1)));
        // A native forced from a bound thunk, then applied (the prelude-free
        // higher-order path).
        assert_agree(&Comp::bind(
            Comp::ret(Value::thunk(Grade::OMEGA, Comp::native(NativePrim::Add))),
            "f",
            Comp::app(
                Comp::app(
                    Comp::force(Value::var("f")),
                    Value::int(FIRST_NATIVE_ARGUMENT),
                ),
                Value::int(SECOND_NATIVE_ARGUMENT),
            ),
        ));
        // SEQUENT-001: a first-order native's result may carry an argument
        // thunk (bare or nested); the marker round trip must resolve it back
        // to the original machine thunk instead of declining.
        assert_agree(&Comp::app(
            Comp::native(NativePrim::Id),
            Value::thunk(Grade::ONE, Comp::ret(Value::Unit)),
        ));
        // ... and the survivor must still FORCE with its real body (a
        // placeholder-body corruption would blame or diverge here).
        assert_agree(&Comp::bind(
            Comp::app(
                Comp::native(NativePrim::Id),
                Value::thunk(Grade::ONE, Comp::ret(Value::int(7))),
            ),
            "x",
            Comp::force(Value::var("x")),
        ));
        // Nested resolution: a thunk inside a record, rearranged by the
        // native, projected back out, and forced.
        assert_agree(&Comp::bind(
            Comp::app(
                Comp::native(NativePrim::Id),
                Value::record([
                    (
                        String::from("a"),
                        Value::thunk(Grade::ONE, Comp::ret(Value::int(9))),
                    ),
                    (String::from("b"), Value::Unit),
                ]),
            ),
            "x",
            Comp::bind(
                Comp::record_proj(Value::var("x"), "a"),
                "t",
                Comp::force(Value::var("t")),
            ),
        ));
        // Multi-mark resolution: `const` keeps its FIRST thunk and discards
        // the second — the marker indices must not cross.
        assert_agree(&Comp::bind(
            Comp::app(
                Comp::app(
                    Comp::native(NativePrim::Const),
                    Value::thunk(Grade::ONE, Comp::ret(Value::int(1))),
                ),
                Value::thunk(Grade::ONE, Comp::ret(Value::int(2))),
            ),
            "x",
            Comp::force(Value::var("x")),
        ));
        // Bad-shape arguments blame the same gradual hole on both machines.
        assert_agree(&Comp::app(
            Comp::app(Comp::native(NativePrim::Add), Value::string("x")),
            Value::Unit,
        ));
        // Sequencing through a saturated native result.
        assert_agree(&Comp::bind(
            Comp::app(
                Comp::app(Comp::native(NativePrim::Add), Value::int(1)),
                Value::int(2),
            ),
            "s",
            Comp::ret(Value::pair(Value::var("s"), Value::var("s"))),
        ));
    }

    /// A handler under an outer eliminator must consume that continuation once.
    ///
    /// The delimiter entry installs the projection on the live L-machine stack.
    /// Handler return and operation clauses therefore fall through to that
    /// stack rather than embedding the same projection in their focused
    /// commands.
    #[test]
    fn handler_resumption_under_projection_uses_the_ambient_continuation_once()
    {
        assert_agree(&Comp::prj1(Comp::handle(
            sig("E".into(), "op".into()),
            Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
            "x",
            Comp::with(Comp::ret(Value::Unit), Comp::ret(Value::Unit)),
            vec![OpClause::new(
                "op",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::Unit)),
            )],
        )));
    }

    /// Hand-built effect / control cases pinning `perform` / `handle` /
    /// `resume` / `reset` against the oracle (the corpus exercises only
    /// unhandled `perform`, so these are the handler / resumption /
    /// delimiter coverage).
    #[test]
    fn hand_built_effect_cases_agree()
    {
        // An unhandled perform blames PerformNoHandler on both.
        assert_agree(&Comp::perform(
            sig("E".into(), "op".into()),
            "op",
            Value::Unit,
        ));
        // ... including one buried under a bind (the operation propagates past
        // the μ̃ to search for a handler, finds none).
        assert_agree(&Comp::bind(
            Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
        ));

        // A handled perform, non-resuming clause: the handle's value is the
        // clause's (the continuation is discarded).
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
            vec![OpClause::new("op", "p", "k", Comp::ret(Value::int(99)))],
        ));

        // A resuming clause: `resume k (ret 42)` feeds the resumption, so the
        // perform "returns" 42 to the scrutinee, then the return clause.
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
            vec![OpClause::new(
                "op",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::int(42))),
            )],
        ));

        // The payload flows to the clause: `resume k (ret p)` returns the
        // performed payload.
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::perform(sig("E".into(), "op".into()), "op", Value::int(5)),
            "x",
            Comp::ret(Value::var("x")),
            vec![OpClause::new(
                "op",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::var("p"))),
            )],
        ));

        // The return clause transforms the scrutinee's value (no perform).
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::ret(Value::int(7)),
            "x",
            Comp::ret(Value::pair(Value::var("x"), Value::var("x"))),
            vec![OpClause::new("op", "p", "k", Comp::ret(Value::int(0)))],
        ));

        // A perform buried under a bind inside the handled scrutinee: resume
        // returns 7 to bind `x`, then `ret x`.
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::bind(
                Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
                "x",
                Comp::ret(Value::var("x")),
            ),
            "y",
            Comp::ret(Value::var("y")),
            vec![OpClause::new(
                "op",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::int(7))),
            )],
        ));

        // Deep re-entry: a resumed continuation re-installs the handler, so a
        // SECOND perform in the resumption is caught again — `a` and `b` both
        // resume with 1.
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::bind(
                Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
                "a",
                Comp::bind(
                    Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
                    "b",
                    Comp::ret(Value::pair(Value::var("a"), Value::var("b"))),
                ),
            ),
            "y",
            Comp::ret(Value::var("y")),
            vec![OpClause::new(
                "op",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::int(1))),
            )],
        ));

        // Nested handlers route by operation name: `perform op2` is caught by the
        // inner E2 handler, not the outer E1 one.
        assert_agree(&Comp::handle(
            sig("E1".into(), "op1".into()),
            Comp::handle(
                sig("E2".into(), "op2".into()),
                Comp::perform(sig("E2".into(), "op2".into()), "op2", Value::Unit),
                "x",
                Comp::ret(Value::var("x")),
                vec![OpClause::new(
                    "op2",
                    "p",
                    "k",
                    Comp::resume(Value::var("k"), Comp::ret(Value::int(20))),
                )],
            ),
            "y",
            Comp::ret(Value::var("y")),
            vec![OpClause::new(
                "op1",
                "q",
                "j",
                Comp::resume(Value::var("j"), Comp::ret(Value::int(10))),
            )],
        ));

        // The v0 single-handler scope: an intervening handler that does not
        // declare the operation blocks the search — `perform op1` reaches the
        // inner E2 handler first, which does not handle it, so both blame
        // PerformNoHandler.
        assert_agree(&Comp::handle(
            sig("E1".into(), "op1".into()),
            Comp::handle(
                sig("E2".into(), "op2".into()),
                Comp::perform(sig("E1".into(), "op1".into()), "op1", Value::Unit),
                "x",
                Comp::ret(Value::var("x")),
                vec![OpClause::new("op2", "p", "k", Comp::ret(Value::int(0)))],
            ),
            "y",
            Comp::ret(Value::var("y")),
            vec![OpClause::new("op1", "q", "j", Comp::ret(Value::int(1)))],
        ));

        // `reset` is transparent to a returning value.
        assert_agree(&Comp::reset(Comp::ret(Value::int(5))));
        // A `reset` delimiter blocks a `perform` from reaching an outer handler
        // (the v0 single-handler scope), so both blame PerformNoHandler.
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::reset(Comp::perform(
                sig("E".into(), "op".into()),
                "op",
                Value::Unit,
            )),
            "y",
            Comp::ret(Value::var("y")),
            vec![OpClause::new("op", "p", "k", Comp::ret(Value::int(0)))],
        ));

        // A handler whose clause reuses one name for the payload and the
        // resumption: the resumption binds innermost and wins.
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
            vec![OpClause::new(
                "op",
                "k",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::int(3))),
            )],
        ));

        // A handled effect forced from a thunk body (the keep-continuation force
        // path): the perform inside the forced thunk must still find the handler.
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::force(Value::thunk(
                Grade::OMEGA,
                Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
            )),
            "x",
            Comp::ret(Value::var("x")),
            vec![OpClause::new(
                "op",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::int(8))),
            )],
        ));
    }

    /// Hand-built `shift` cases pinning the faithful delimited capture
    /// against the oracle: capture up to the
    /// enclosing prompt, the continuation discarded or `resume`d (single- and
    /// multi-shot), the payload flowing through the resumed continuation, and
    /// the undelimited / across-a-handler `ShiftNoReset` agreement.
    #[test]
    fn hand_built_shift_cases_agree()
    {
        // Discard the captured continuation: the shift's value is the reset's.
        assert_agree(&Comp::reset(Comp::shift("k", Comp::ret(Value::int(5)))));
        // Discard across a bind: the enclosing `let x <- []; ret (x, x)`
        // continuation is dropped, so the reset yields the shift body's value.
        assert_agree(&Comp::reset(Comp::bind(
            Comp::shift("k", Comp::ret(Value::int(9))),
            "x",
            Comp::ret(Value::pair(Value::var("x"), Value::var("x"))),
        )));
        // Resume once (identity-shaped): `resume k (ret 3)` re-invokes the
        // captured `let x <- []; ret (x, x)`, so x = 3.
        assert_agree(&Comp::reset(Comp::bind(
            Comp::shift("k", Comp::resume(Value::var("k"), Comp::ret(Value::int(3)))),
            "x",
            Comp::ret(Value::pair(Value::var("x"), Value::var("x"))),
        )));
        // The resumed continuation runs real work: the native `Add` inside the
        // captured continuation computes 3 + 4 = 7.
        assert_agree(&Comp::reset(Comp::bind(
            Comp::shift("k", Comp::resume(Value::var("k"), Comp::ret(Value::int(3)))),
            "x",
            Comp::bind(
                Comp::app(
                    Comp::app(Comp::native(NativePrim::Add), Value::var("x")),
                    Value::int(4),
                ),
                "y",
                Comp::ret(Value::var("y")),
            ),
        )));
        // Multi-shot: the captured continuation is invoked twice; the second
        // (innermost) invocation is the reset's value (2).
        assert_agree(&Comp::reset(Comp::bind(
            Comp::shift(
                "k",
                Comp::bind(
                    Comp::resume(Value::var("k"), Comp::ret(Value::int(1))),
                    "a",
                    Comp::resume(Value::var("k"), Comp::ret(Value::int(2))),
                ),
            ),
            "x",
            Comp::ret(Value::var("x")),
        )));
        // Nested prompts: the shift captures up to the INNER reset only; the
        // value returns transparently through both.
        assert_agree(&Comp::reset(Comp::reset(Comp::shift(
            "k",
            Comp::ret(Value::int(4)),
        ))));
        // A shift under an outer reset but an inner-most bind, resuming, then the
        // reset delimits: x = 6, result (6, 6).
        assert_agree(&Comp::reset(Comp::bind(
            Comp::shift("k", Comp::resume(Value::var("k"), Comp::ret(Value::int(6)))),
            "x",
            Comp::ret(Value::pair(Value::var("x"), Value::var("x"))),
        )));

        // Undelimited: a bare `shift` reaches no prompt, so both blame
        // ShiftNoReset.
        assert_agree(&Comp::shift("k", Comp::ret(Value::int(1))));
        // ... including one buried under a bind (still no enclosing prompt).
        assert_agree(&Comp::bind(
            Comp::shift("k", Comp::ret(Value::int(1))),
            "x",
            Comp::ret(Value::var("x")),
        ));
        // Across a handler: the capture would cross a `KHandle` frame, which the
        // v0 structural model cannot reify, so both blame ShiftNoReset (a handler
        // is not a prompt).
        assert_agree(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::shift("k", Comp::ret(Value::int(1))),
            "x",
            Comp::ret(Value::var("x")),
            vec![OpClause::new("op", "p", "j", Comp::ret(Value::int(0)))],
        ));
    }

    // ── The host-effect seam (ADR-35 D4) ─────────────────────────────────────
    //
    // The seam is a preserved boundary: the L machine
    // (`machine::run_comp_with_host`) must present the *identical* seam to a host
    // that the CEK oracle (`eval::run_with_host`) does. These cases drive a
    // scripted host on both machines and require (a) the machines agree on the
    // final outcome and (b) the host received the same ordered log of offers
    // (signature name, operation, payload) on both. Payloads stay first-order,
    // where readback is exact on both sides (higher-order payloads await the
    // un-focusing readback `𝓕⁻¹` — a listed residual, not exercised here).

    /// A scripted host: it answers offers from a fixed reply queue (declining
    /// once exhausted) and records the `(signature-name, operation, payload)`
    /// of every offer it receives, so two machines' logs can be compared.
    struct ScriptedHost
    {
        replies: Vec<HostReply>,
        next: usize,
        log: Vec<(String, String, Value)>,
    }

    impl ScriptedHost
    {
        fn new(replies: &[HostReply]) -> Self
        {
            Self {
                replies: replies.to_vec(),
                next: 0,
                log: Vec::new(),
            }
        }
    }

    impl HostHandler for ScriptedHost
    {
        fn handle<'source, O>(
            &mut self,
            sig: &EffectSig,
            op: O,
            payload: &Value,
        ) -> HostReply
        where
            O: Into<OperationName<'source>>,
        {
            let op = op.into();
            self.log.push((
                sig.name().as_ref().to_owned(),
                op.as_ref().to_owned(),
                payload.clone(),
            ));
            let reply = self
                .replies
                .get(self.next)
                .cloned()
                .unwrap_or(HostReply::Unhandled);
            self.next = self.next.saturating_add(1);
            reply
        }
    }

    /// Drives `comp` with the same scripted host on the CEK oracle and the L
    /// machine, asserting they agree on the final outcome AND on the host log,
    /// which must equal `expected_log`; the shared final must also match
    /// `expected_final`.
    fn assert_host_seam(
        comp: &Comp,
        replies: &[HostReply],
        expected_log: &[(&str, &str, Value)],
        expected_final: &Eval,
    )
    {
        let mut cek_host = ScriptedHost::new(replies);
        let oracle = run_with_host(comp.clone(), &mut cek_host);

        let mut l_host = ScriptedHost::new(replies);
        let machine = machine::run_comp_with_host(comp, &mut l_host);

        assert!(
            bool::from(agree(&oracle, &machine)),
            "host-seam L-run ≢ run on {comp:?}\n  oracle    = {:?}\n  L machine = {:?}",
            canonical(&oracle),
            canonical(&machine)
        );
        assert!(
            bool::from(agree(&oracle, expected_final)),
            "host-seam final mismatch on {comp:?}\n  got      = {:?}\n  expected = {:?}",
            canonical(&oracle),
            canonical(expected_final)
        );

        let expected: Vec<(String, String, Value)> = expected_log
            .iter()
            .map(|&(sig, op, ref payload)| (sig.to_owned(), op.to_owned(), payload.clone()))
            .collect();
        assert_eq!(cek_host.log, expected, "CEK host log mismatch on {comp:?}");
        assert_eq!(l_host.log, expected, "L host log mismatch on {comp:?}");
    }

    /// (a) A single unhandled `perform`, resumed by the host: both machines
    /// take the reply as the outcome.
    #[test]
    fn host_seam_resumes_a_single_unhandled_perform()
    {
        assert_host_seam(
            &Comp::perform(sig("E".into(), "op".into()), "op", Value::int(5)),
            &[HostReply::Resume(Value::int(42))],
            &[("E", "op", Value::int(5))],
            &Eval::Value(Comp::ret(Value::int(42))),
        );
    }

    /// (b) The host declines the offer: both machines blame `PerformNoHandler`.
    #[test]
    fn host_seam_decline_blames_perform_no_handler()
    {
        assert_host_seam(
            &Comp::perform(sig("E".into(), "op".into()), "op", Value::int(5)),
            &[HostReply::Unhandled],
            &[("E", "op", Value::int(5))],
            &Eval::Blame(Blame::PerformNoHandler),
        );
    }

    /// (c) Sequential performs with deep re-entry: the resumed continuation
    /// performs again and is offered again, in execution order, and the second
    /// offer's payload is the value the first reply bound.
    #[test]
    fn host_seam_offers_deep_re_entry_in_order()
    {
        assert_host_seam(
            &Comp::bind(
                Comp::perform(sig("E".into(), "op".into()), "op", Value::int(1)),
                "x",
                Comp::perform(sig("E".into(), "op".into()), "op", Value::var("x")),
            ),
            &[
                HostReply::Resume(Value::int(10)),
                HostReply::Resume(Value::int(99)),
            ],
            &[("E", "op", Value::int(1)), ("E", "op", Value::int(10))],
            &Eval::Value(Comp::ret(Value::int(99))),
        );
    }

    /// (d) A `perform` under a source handler that claims a DIFFERENT operation
    /// reaches the host; the source handler still claims its own operation
    /// (which never reaches the host).
    #[test]
    fn host_seam_offers_across_a_non_matching_handler()
    {
        assert_host_seam(
            &Comp::handle(
                sig("E".into(), "keep".into()),
                Comp::bind(
                    Comp::perform(sig("E".into(), "esc".into()), "esc", Value::int(7)),
                    "x",
                    Comp::perform(sig("E".into(), "keep".into()), "keep", Value::int(8)),
                ),
                "r",
                Comp::ret(Value::var("r")),
                vec![OpClause::new("keep", "p", "k", Comp::ret(Value::int(555)))],
            ),
            &[HostReply::Resume(Value::int(20))],
            // Only the escaping `esc` reaches the host; the source handler claims
            // `keep` itself, so it is never offered.
            &[("E", "esc", Value::int(7))],
            &Eval::Value(Comp::ret(Value::int(555))),
        );
    }

    /// (e) A `perform` a source handler claims never consults the host (empty
    /// log).
    #[test]
    fn host_seam_never_consulted_for_a_claimed_perform()
    {
        assert_host_seam(
            &Comp::handle(
                sig("E".into(), "op".into()),
                Comp::perform(sig("E".into(), "op".into()), "op", Value::int(5)),
                "r",
                Comp::ret(Value::var("r")),
                vec![OpClause::new("op", "p", "k", Comp::ret(Value::int(77)))],
            ),
            // A non-empty script would be a bug magnet; the host must not be
            // consulted at all.
            &[HostReply::Resume(Value::int(0))],
            &[],
            &Eval::Value(Comp::ret(Value::int(77))),
        );
    }

    /// (f) The reply value flows into the subsequent computation (a native
    /// add).
    #[test]
    fn host_seam_reply_flows_into_subsequent_computation()
    {
        assert_host_seam(
            &Comp::bind(
                Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
                "x",
                Comp::app(
                    Comp::app(Comp::native(NativePrim::Add), Value::var("x")),
                    Value::int(1),
                ),
            ),
            &[HostReply::Resume(Value::int(9))],
            &[("E", "op", Value::Unit)],
            &Eval::Value(Comp::ret(Value::int(10))),
        );
    }

    /// (g) First-order structured payloads read back to the identical public
    /// `Value` on both machines (pair / list / record).
    #[test]
    fn host_seam_first_order_payloads_read_back_identically()
    {
        assert_host_seam(
            &Comp::perform(
                sig("E".into(), "op".into()),
                "op",
                Value::pair(Value::int(1), Value::int(2)),
            ),
            &[HostReply::Resume(Value::Unit)],
            &[("E", "op", Value::pair(Value::int(1), Value::int(2)))],
            &Eval::Value(Comp::ret(Value::Unit)),
        );
        assert_host_seam(
            &Comp::perform(
                sig("E".into(), "op".into()),
                "op",
                Value::list(vec![Value::int(1), Value::int(2), Value::int(3)]),
            ),
            &[HostReply::Resume(Value::Unit)],
            &[(
                "E",
                "op",
                Value::list(vec![Value::int(1), Value::int(2), Value::int(3)]),
            )],
            &Eval::Value(Comp::ret(Value::Unit)),
        );
        assert_host_seam(
            &Comp::perform(
                sig("E".into(), "op".into()),
                "op",
                Value::record(vec![
                    (String::from("a"), Value::int(1)),
                    (String::from("b"), Value::string("hi")),
                ]),
            ),
            &[HostReply::Resume(Value::Unit)],
            &[(
                "E",
                "op",
                Value::record(vec![
                    (String::from("a"), Value::int(1)),
                    (String::from("b"), Value::string("hi")),
                ]),
            )],
            &Eval::Value(Comp::ret(Value::Unit)),
        );
    }

    /// Asserts the two machines agree on `comp`, panicking with the disagreeing
    /// canonicalization otherwise.
    fn assert_agree(comp: &Comp)
    {
        let oracle = run_comp(comp.clone());
        let machine = machine::run_comp(comp);
        assert!(
            bool::from(agree(&oracle, &machine)),
            "L-run ∘ 𝓕 ≢ run on {comp:?}\n  oracle    = {:?}\n  L machine = {:?}",
            canonical(&oracle),
            canonical(&machine)
        );
    }

    /// A one-operation effect signature (the sig's operation types are inert to
    /// the CEK's `run_comp`, which routes by operation name — ADR-33 D3).
    fn sig(
        name: EffectSignatureName<'_>,
        op: OperationName<'_>,
    ) -> gandr_core_checker::effect::EffectSig
    {
        use gandr_core_checker::effect::EffectOp;
        use gandr_core_checker::effect::EffectSig;
        use gandr_core_checker::types::ValueType;
        EffectSig::new(name, vec![EffectOp::new(
            op,
            ValueType::integer(),
            ValueType::integer(),
        )])
    }
}
