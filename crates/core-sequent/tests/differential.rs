//! The phase-L1 property gate (`proposal-sequent-kernel.md` §9, phase L1),
//! re-anchored on the L machine's own outcomes.
//!
//! # From oracle agreement to L-outcome regression (ADR-71)
//!
//! Through stage E this was an L-vs-CEK **agreement** differential: the CEK
//! machine (`gandr_core_checker::eval::run_comp`) was the external oracle — a
//! distinct implementation of the same operational semantics, sharing no step
//! code with the L machine (`gandr_core_sequent::machine::run_comp`) — and each
//! case asserted the two canonicalized [`Eval`] outcomes were equal. With the
//! CEK retiring (B1 phase-3 stage F), the differential's evidence is preserved
//! by **freezing** the L machine's own canonical outcome for each hand-built
//! case — captured once from the final oracle-agreeing run — into a checked-in
//! regression snapshot ([`Check`]), and by
//! keeping the **intrinsic** L-machine properties the generated lane witnesses:
//! every run reaches a **defined** outcome and is **deterministic**
//! ([`l_machine_is_total_and_deterministic`]). The frozen snapshots *are* the
//! retired oracle for the hand-built fragment; the corpus outcome-snapshot
//! sweep ([`crate::corpus_differential`]) anchors the adequacy hypothesis over
//! the whole corpus.
//!
//! Comparison stays on the canonicalized [`Eval`] outcome
//! ([`gandr_core_sequent::differential::canonical`]) — the outcome kind, the
//! stuck reason, and the returned value exactly on the first-order fragment
//! (the observable fragment the corpus checks); higher-order terminals
//! (returned thunks, bare functions / lazy pairs, partial natives) are compared
//! **structurally** through the un-focusing readback `𝓕⁻¹`, only a returned
//! reified stack staying at kind granularity (the k-in-value residual; see the
//! module docs of [`gandr_core_sequent::differential`]).
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
//! CEK's α-renamed side-table name; the k-in-value residual), so a generated
//! `shift` body only ever `resume`s `k` or ignores it. Hand-built cases pin the
//! prelude free-name resolution (ADR-42), the ADR-76 identity formers, the
//! ADR-80 declared data, the **higher-order native combinators** (`each` /
//! `where` / `reduce` / `any` / `all` / `update_where` over pure, effectful,
//! and blaming closures — now dispatched through the un-focusing readback), the
//! higher-order **host payloads**, and the **exact structural readback** of
//! thunk / function / lazy-pair / partial-native terminals against the oracle.

#[cfg(test)]
mod tests
{
    use gandr_core_checker::boundary::EffectSignatureName;
    use gandr_core_checker::boundary::GenerationDepth;
    use gandr_core_checker::boundary::NameRef;
    use gandr_core_checker::boundary::OperationName;
    use gandr_core_checker::effect::EffectSig;
    use gandr_core_checker::effect::host::HostHandler;
    use gandr_core_checker::effect::host::HostReply;
    use gandr_core_checker::grade::Grade;
    use gandr_core_checker::outcome::Blame;
    use gandr_core_checker::outcome::Eval;
    use gandr_core_checker::prim::NativePrim;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::OpClause;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::syntax::WalkBase;
    use gandr_core_checker::syntax::WalkMotive;
    use gandr_core_checker::types::CompType;
    use gandr_core_checker::types::DataId;
    use gandr_core_checker::types::ValueType;
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
                    .prop_map(|value| Value::annot(value, ValueType::integer()))
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
            // a captured continuation whose representation in *value* position
            // the un-focusing readback `𝓕⁻¹` CANNOT reconcile — the CEK
            // α-renames it to a fresh, run-unique side-table neutral (so its
            // readback is a nondeterministic `Value::Var("%k…")`), while the L
            // machine binds it a boxed continuation (whose readback is an opaque
            // `Value::Stk`). The two carriers disagree by construction and the
            // CEK's is not even stable across runs, so returning `ek` as data
            // stays the k-in-value residual (§7a); the resumption is exercised
            // only in `resume ek …` position (the explicit resuming clause
            // below), where both machines agree.
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

            // `shift` — now faithful. Two safe shapes (a captured continuation
            // in *value* position is the k-in-value residual the un-focusing
            // readback `𝓕⁻¹` cannot reconcile — see the `perform` clause above —
            // so `k` is never returned as data): (1) `shift k. body` whose body
            // is a pure comp under `scope` (WITHOUT `k`, so the capture is
            // discarded); and (2) `shift k. resume k (ret fed)` (the captured
            // continuation is re-invoked once with a pure value). Under an
            // enclosing `reset` the capture reaches the prompt; undelimited (or
            // across a handler) both machines agree on the `ShiftNoReset` blame.
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
    fn eff_sig() -> EffectSig
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

        let mut check = Check::load(("pure_spine").into());

        // ret + first-order data.
        check.pin(&Comp::ret(Value::int(PINNED_RETURN_INTEGER)));
        check.pin(&Comp::ret(Value::pair(Value::int(1), Value::string("s"))));
        check.pin(&Comp::ret(Value::inj1(Value::Unit)));
        check.pin(&Comp::ret(Value::list(vec![
            Value::int(1),
            Value::int(2),
            Value::int(3),
        ])));
        check.pin(&Comp::ret(Value::record([(
            String::from("a"),
            Value::int(7),
        )])));

        // bind threads a value.
        check.pin(&Comp::bind(
            Comp::ret(Value::int(3)),
            "y",
            Comp::ret(Value::var("y")),
        ));

        // force a thunk (call-by-need) and a re-forced (shared) thunk.
        check.pin(&Comp::force(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(5)),
        )));
        check.pin(&Comp::bind(
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
        check.pin(&Comp::app(
            Comp::lam("x", Comp::ret(Value::var("x"))),
            Value::int(BETA_ARGUMENT),
        ));
        check.pin(&Comp::app(
            Comp::app(
                Comp::lam("x", Comp::lam("y", Comp::ret(Value::var("x")))),
                Value::int(1),
            ),
            Value::int(2),
        ));

        // case on both injections.
        check.pin(&Comp::case(
            Value::inj1(Value::int(1)),
            "l",
            Comp::ret(Value::var("l")),
            "r",
            Comp::ret(Value::int(0)),
        ));
        check.pin(&Comp::case(
            Value::inj2(Value::int(2)),
            "l",
            Comp::ret(Value::int(0)),
            "r",
            Comp::ret(Value::var("r")),
        ));

        // split a pair.
        check.pin(&Comp::split(
            Value::pair(Value::int(1), Value::int(2)),
            "a",
            "b",
            Comp::ret(Value::var("b")),
        ));

        // listcase (nil and cons).
        check.pin(&Comp::list_case(
            Value::list(vec![]),
            Comp::ret(Value::int(0)),
            "h",
            "t",
            Comp::ret(Value::var("h")),
        ));
        check.pin(&Comp::list_case(
            Value::list(vec![Value::int(1), Value::int(2)]),
            Comp::ret(Value::int(0)),
            "h",
            "t",
            Comp::ret(Value::var("h")),
        ));

        // record projection.
        check.pin(&Comp::record_proj(
            Value::record([(String::from("a"), Value::int(8))]),
            "a",
        ));

        // lazy pair projection (both sides).
        check.pin(&Comp::prj1(Comp::with(
            Comp::ret(Value::int(1)),
            Comp::ret(Value::int(2)),
        )));
        check.pin(&Comp::prj2(Comp::with(
            Comp::ret(Value::int(1)),
            Comp::ret(Value::int(2)),
        )));

        // dup / drop (grade structural ops).
        check.pin(&Comp::dup(Value::int(4)));
        check.pin(&Comp::drop(Value::int(4)));

        // holes: a computation hole blames; a returned value hole is a value.
        check.pin(&Comp::hole(0));
        check.pin(&Comp::ret(Value::hole(0)));
        check.pin(&Comp::bind(Comp::hole(0), "y", Comp::ret(Value::var("y"))));

        // ill-typed redexes must reach the same stuck reason on both machines.
        check.pin(&Comp::app(Comp::ret(Value::int(1)), Value::int(2)));
        check.pin(&Comp::force(Value::int(1)));
        check.pin(&Comp::case(
            Value::int(1),
            "l",
            Comp::ret(Value::Unit),
            "r",
            Comp::ret(Value::Unit),
        ));
    }

    proptest! {
        // A wider case budget than the default — the property gate is the
        // phase's core generated coverage, and the pure-spine generator is cheap
        // to sample.
        #![proptest_config(ProptestConfig::with_cases(4096))]

        /// The intrinsic L-machine properties over generated closed pure-spine
        /// computations: every run reaches a **defined** outcome (it does not
        /// panic, and canonicalizes) and is **deterministic** across two runs.
        /// The step-budget net (`STEP_BUDGET`) guarantees termination, so a
        /// returned outcome witnesses budget discipline. The L ≡ CEK agreement
        /// property this generator once drove retired with the oracle; the
        /// L machine's operational adequacy is now anchored by the corpus
        /// outcome-snapshot sweep and the hand-built L-outcome regressions above.
        #[test]
        fn l_machine_is_total_and_deterministic(comp in arb_comp(&[], gen_depth()))
        {
            let first = canonical(&machine::run_comp(&comp));
            let second = canonical(&machine::run_comp(&comp));
            prop_assert_eq!(
                &first, &second,
                "the L machine is non-deterministic on {:?}", comp
            );
        }
    }

    /// Hand-built identity-former cases (ADR-76) pinning Walk-β against the
    /// oracle: the eliminator fires on `here(w)` binding the diagonal base
    /// binder to the WITNESS (not the whole proof), a non-`here` scrutinee is
    /// `WalkOnNonHere`, a hole blames, and a returned `here` compares opaquely.
    #[test]
    fn hand_built_identity_cases_agree()
    {
        let mut check = Check::load(("identity").into());
        // Walk-β threads the WITNESS into the base binder: `(x). ret x` on
        // `here(7)` yields `ret 7`.
        check.pin(&Comp::walk(
            Value::here(Value::int(7)),
            walk_motive(),
            WalkBase::new("x", Comp::ret(Value::var("x"))),
        ));
        // The base body may rebuild the proof: `(x). ret here(x)` yields
        // `ret here(7)` — a returned identity value, compared opaquely on both.
        check.pin(&Comp::walk(
            Value::here(Value::int(7)),
            walk_motive(),
            WalkBase::new("x", Comp::ret(Value::here(Value::var("x")))),
        ));
        // A structured witness threads through and is eliminated: `here((1, 2))`
        // splits to 1.
        check.pin(&Comp::walk(
            Value::here(Value::pair(Value::int(1), Value::int(2))),
            walk_motive(),
            WalkBase::new(
                "x",
                Comp::split(Value::var("x"), "a", "b", Comp::ret(Value::var("a"))),
            ),
        ));
        // Walk-β under a bind continues: `bind (walk here(5) …) y (ret (y, y))`.
        check.pin(&Comp::bind(
            Comp::walk(
                Value::here(Value::int(5)),
                walk_motive(),
                WalkBase::new("x", Comp::ret(Value::var("x"))),
            ),
            "y",
            Comp::ret(Value::pair(Value::var("y"), Value::var("y"))),
        ));
        // A bare returned identity proof compares opaquely (kind granularity).
        check.pin(&Comp::ret(Value::here(Value::int(9))));
        // A non-`here` scrutinee is `WalkOnNonHere` on both (ill-typed input;
        // a well-typed closed `Path` scrutinee is always a `here`).
        check.pin(&Comp::walk(
            Value::int(1),
            walk_motive(),
            WalkBase::new("x", Comp::ret(Value::var("x"))),
        ));
        // A hole scrutinee blames on both.
        check.pin(&Comp::walk(
            Value::hole(0),
            walk_motive(),
            WalkBase::new("x", Comp::ret(Value::var("x"))),
        ));
    }

    /// A trivial (runtime-erased) Walk motive `(x y q). F Integer` — the
    /// identity β-rule ignores it, so its shape is inert to both machines.
    fn walk_motive() -> WalkMotive
    {
        WalkMotive::new("x", "y", "q", CompType::returner(ValueType::integer()))
    }

    /// Hand-built declared-data cases (ADR-80) pinning the case-over-tag
    /// reduction against the oracle: `DataCase` selects the arm at the
    /// constructor's position and binds its single binder to the WHOLE payload
    /// (exactly the CEK's `arms.nth(tag)`), a non-`Ctor` scrutinee or an
    /// out-of-range tag is `DataCasedNonCtor`, a hole blames, and a returned
    /// `Ctor` compares opaquely.
    #[test]
    fn hand_built_declared_data_cases_agree()
    {
        let mut check = Check::load(("declared_data").into());
        // Selects the matching arm and binds the payload: tag 0 yields 7.
        check.pin(&Comp::data_case(
            Value::ctor(did(), 0_usize, Value::int(7)),
            vec![
                (String::from("y"), Comp::ret(Value::var("y"))),
                (String::from("z"), Comp::ret(Value::int(0))),
            ],
        ));
        // Selects a later arm by position: tag 1 yields 5.
        check.pin(&Comp::data_case(
            Value::ctor(did(), 1_usize, Value::int(5)),
            vec![
                (String::from("y"), Comp::ret(Value::int(0))),
                (String::from("z"), Comp::ret(Value::var("z"))),
            ],
        ));
        // A nullary constructor binds its unit payload.
        check.pin(&Comp::data_case(
            Value::ctor(did(), 0_usize, Value::Unit),
            vec![(String::from("y"), Comp::ret(Value::int(99)))],
        ));
        // A structured payload threads through and is eliminated: `(1, 2)`
        // splits to 2.
        check.pin(&Comp::data_case(
            Value::ctor(did(), 0_usize, Value::pair(Value::int(1), Value::int(2))),
            vec![(
                String::from("p"),
                Comp::split(Value::var("p"), "a", "b", Comp::ret(Value::var("b"))),
            )],
        ));
        // Continued evaluation after the selection.
        check.pin(&Comp::bind(
            Comp::data_case(Value::ctor(did(), 0_usize, Value::int(3)), vec![(
                String::from("y"),
                Comp::ret(Value::var("y")),
            )]),
            "w",
            Comp::ret(Value::pair(Value::var("w"), Value::var("w"))),
        ));
        // A bare returned constructor compares opaquely (kind granularity).
        check.pin(&Comp::ret(Value::ctor(did(), 0_usize, Value::int(4))));
        // A non-`Ctor` scrutinee is `DataCasedNonCtor` on both.
        check.pin(&Comp::data_case(Value::int(1), vec![(
            String::from("y"),
            Comp::ret(Value::var("y")),
        )]));
        // A hole scrutinee blames on both.
        check.pin(&Comp::data_case(Value::hole(0), vec![(
            String::from("y"),
            Comp::ret(Value::var("y")),
        )]));
        // An out-of-range tag (tag 2 over a 2-arm case) is `DataCasedNonCtor`.
        check.pin(&Comp::data_case(
            Value::ctor(did(), 2_usize, Value::Unit),
            vec![
                (String::from("y"), Comp::ret(Value::var("y"))),
                (String::from("z"), Comp::ret(Value::var("z"))),
            ],
        ));
        // The absurd (empty) match: a `Ctor` and a non-`Ctor` scrutinee both
        // reach `DataCasedNonCtor` (the empty-arm case classifies as data).
        check.pin(&Comp::data_case(
            Value::ctor(did(), 0_usize, Value::Unit),
            vec![],
        ));
        check.pin(&Comp::data_case(Value::int(1), vec![]));
    }

    /// A fixed declared-data nominal id (the serial and name are render-only —
    /// erased by `𝓕`, invisible to the differential).
    fn did() -> DataId
    {
        DataId::new(0_u64, "D")
    }

    /// Hand-built native-dispatch cases pinning the currying registry against
    /// the oracle.
    #[test]
    fn hand_built_native_cases_agree()
    {
        const FIRST_NATIVE_ARGUMENT: i64 = 10;
        const SECOND_NATIVE_ARGUMENT: i64 = 20;

        let mut check = Check::load(("native").into());

        // Saturated arithmetic and comparison.
        check.pin(&Comp::app(
            Comp::app(Comp::native(NativePrim::Add), Value::int(2)),
            Value::int(3),
        ));
        check.pin(&Comp::app(
            Comp::app(Comp::native(NativePrim::Mul), Value::int(4)),
            Value::int(5),
        ));
        check.pin(&Comp::app(
            Comp::app(Comp::native(NativePrim::Lt), Value::int(1)),
            Value::int(2),
        ));
        // Unary native.
        check.pin(&Comp::app(Comp::native(NativePrim::Neg), Value::int(7)));
        // Identity passing through a first-order value.
        check.pin(&Comp::app(
            Comp::native(NativePrim::Id),
            Value::pair(Value::int(1), Value::int(2)),
        ));
        // Bare and partial natives are curried function terminals.
        check.pin(&Comp::native(NativePrim::Add));
        check.pin(&Comp::app(Comp::native(NativePrim::Add), Value::int(1)));
        // A native forced from a bound thunk, then applied (the prelude-free
        // higher-order path).
        check.pin(&Comp::bind(
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
        check.pin(&Comp::app(
            Comp::native(NativePrim::Id),
            Value::thunk(Grade::ONE, Comp::ret(Value::Unit)),
        ));
        // ... and the survivor must still FORCE with its real body (a
        // placeholder-body corruption would blame or diverge here).
        check.pin(&Comp::bind(
            Comp::app(
                Comp::native(NativePrim::Id),
                Value::thunk(Grade::ONE, Comp::ret(Value::int(7))),
            ),
            "x",
            Comp::force(Value::var("x")),
        ));
        // Nested resolution: a thunk inside a record, rearranged by the
        // native, projected back out, and forced.
        check.pin(&Comp::bind(
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
        check.pin(&Comp::bind(
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
        check.pin(&Comp::app(
            Comp::app(Comp::native(NativePrim::Add), Value::string("x")),
            Value::Unit,
        ));
        // Sequencing through a saturated native result.
        check.pin(&Comp::bind(
            Comp::app(
                Comp::app(Comp::native(NativePrim::Add), Value::int(1)),
                Value::int(2),
            ),
            "s",
            Comp::ret(Value::pair(Value::var("s"), Value::var("s"))),
        ));
    }

    /// Borrowed binder text for one closure layer.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct ClosureBinder<'binder>(&'binder str);

    impl<'binder> From<&'binder str> for ClosureBinder<'binder>
    {
        fn from(binder: &'binder str) -> Self
        {
            Self(binder)
        }
    }

    /// A pure unary closure `U(? → F ?)` — a thunk over `λx. body`, the shape a
    /// higher-order combinator forces and applies.
    fn closure1(
        binder: ClosureBinder<'_>,
        body: Comp,
    ) -> Value
    {
        Value::thunk(Grade::OMEGA, Comp::lam(binder.0, body))
    }

    /// A pure binary closure `U(? → ? → F ?)` — the shape `reduce` folds with.
    fn closure2(
        fst: ClosureBinder<'_>,
        snd: ClosureBinder<'_>,
        body: Comp,
    ) -> Value
    {
        Value::thunk(Grade::OMEGA, Comp::lam(fst.0, Comp::lam(snd.0, body)))
    }

    /// Borrowed integers for one checked-core list fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct IntegerListFixture<'values>(&'values [i64]);

    impl<'values, const LENGTH: usize> From<&'values [i64; LENGTH]> for IntegerListFixture<'values>
    {
        fn from(values: &'values [i64; LENGTH]) -> Self
        {
            Self(values)
        }
    }

    impl<'values> From<&'values Vec<i64>> for IntegerListFixture<'values>
    {
        fn from(values: &'values Vec<i64>) -> Self
        {
            Self(values.as_slice())
        }
    }

    /// A small integer list value.
    fn int_list(values: IntegerListFixture<'_>) -> Value
    {
        Value::list(values.0.iter().copied().map(Value::int).collect())
    }

    /// A saturated `each f xs` computation.
    fn each(
        closure: Value,
        list: Value,
    ) -> Comp
    {
        Comp::app(Comp::app(Comp::native(NativePrim::Each), closure), list)
    }

    /// Hand-built **higher-order native** cases pinning the un-focusing
    /// dispatch (`𝓕⁻¹`) of every closure-taking combinator against the CEK
    /// oracle. The L machine un-focuses each thunk-closure argument to a
    /// source value, invokes the builtin exactly as the CEK does, and
    /// re-focuses the unrolled result against the ambient continuation — so
    /// a pure, an effectful (handled), and a blaming closure all reach the
    /// identical outcome the oracle does.
    #[test]
    fn hand_built_higher_order_native_cases_agree()
    {
        const MAPPED_LIST: [i64; 2] = [10, 20];
        const UPDATE_LIST: [i64; 3] = [10, 20, 30];
        const MAPPING_INCREMENT: i64 = 100;

        let mut check = Check::load(("higher_order_native").into());
        // `each (\x. x + 1) [1, 2, 3]` maps to `[2, 3, 4]`.
        check.pin(&each(
            closure1(
                ("x").into(),
                Comp::app(
                    Comp::app(Comp::native(NativePrim::Add), Value::var("x")),
                    Value::int(1),
                ),
            ),
            int_list((&[1, 2, 3]).into()),
        ));
        // `each` over the empty list is the empty list.
        check.pin(&each(
            closure1(("x").into(), Comp::ret(Value::var("x"))),
            int_list((&[]).into()),
        ));

        // The mapped result threads into an enclosing bind.
        check.pin(&Comp::bind(
            each(
                closure1(
                    ("x").into(),
                    Comp::app(
                        Comp::app(Comp::native(NativePrim::Mul), Value::var("x")),
                        Value::int(2),
                    ),
                ),
                int_list((&MAPPED_LIST).into()),
            ),
            "ys",
            Comp::ret(Value::pair(Value::var("ys"), Value::var("ys"))),
        ));

        // `where (\x. x < 2) [1, 2, 3]` keeps `[1]`.
        check.pin(&Comp::app(
            Comp::app(
                Comp::native(NativePrim::Where),
                closure1(
                    ("x").into(),
                    Comp::app(
                        Comp::app(Comp::native(NativePrim::Lt), Value::var("x")),
                        Value::int(2),
                    ),
                ),
            ),
            int_list((&[1, 2, 3]).into()),
        ));

        // `reduce (\a x. a + x) 0 [1, 2, 3, 4]` folds to `10`.
        check.pin(&Comp::app(
            Comp::app(
                Comp::app(
                    Comp::native(NativePrim::Reduce),
                    closure2(
                        ("a").into(),
                        ("x").into(),
                        Comp::app(
                            Comp::app(Comp::native(NativePrim::Add), Value::var("a")),
                            Value::var("x"),
                        ),
                    ),
                ),
                Value::int(0),
            ),
            int_list((&[1, 2, 3, 4]).into()),
        ));

        // `any (\x. x == 2) [1, 2, 3]` is `true` (short-circuits).
        check.pin(&Comp::app(
            Comp::app(
                Comp::native(NativePrim::Any),
                closure1(
                    ("x").into(),
                    Comp::app(
                        Comp::app(Comp::native(NativePrim::Eq), Value::var("x")),
                        Value::int(2),
                    ),
                ),
            ),
            int_list((&[1, 2, 3]).into()),
        ));

        // `all (\x. x < 5) [1, 2, 3]` is `true`.
        check.pin(&Comp::app(
            Comp::app(
                Comp::native(NativePrim::All),
                closure1(
                    ("x").into(),
                    Comp::app(
                        Comp::app(Comp::native(NativePrim::Lt), Value::var("x")),
                        Value::int(5),
                    ),
                ),
            ),
            int_list((&[1, 2, 3]).into()),
        ));

        // `update_where (\x. x < 3) (\x. x + 100) [1, 2, 3, 4]` transforms the
        // matched elements to `[101, 102, 3, 4]`.
        check.pin(&Comp::app(
            Comp::app(
                Comp::app(
                    Comp::native(NativePrim::UpdateWhere),
                    closure1(
                        ("x").into(),
                        Comp::app(
                            Comp::app(Comp::native(NativePrim::Lt), Value::var("x")),
                            Value::int(3),
                        ),
                    ),
                ),
                closure1(
                    ("x").into(),
                    Comp::app(
                        Comp::app(Comp::native(NativePrim::Add), Value::var("x")),
                        Value::int(MAPPING_INCREMENT),
                    ),
                ),
            ),
            int_list((&[1, 2, 3, 4]).into()),
        ));

        // `update_at [10, 20, 30] 1 (\x. x + 5)` transforms the element at
        // index 1, yielding `[10, 25, 30]` (the closure-taking list update).
        check.pin(&Comp::app(
            Comp::app(
                Comp::app(
                    Comp::native(NativePrim::UpdateAt),
                    int_list((&UPDATE_LIST).into()),
                ),
                Value::int(1),
            ),
            closure1(
                ("x").into(),
                Comp::app(
                    Comp::app(Comp::native(NativePrim::Add), Value::var("x")),
                    Value::int(5),
                ),
            ),
        ));

        // An **effectful** closure under a handler: `each (\x. perform op x)`
        // performs per element; the handler resumes with `p + 10`, so the map
        // runs against the ambient continuation and yields `[11, 12]`.
        check.pin(&Comp::handle(
            sig("E".into(), "op".into()),
            each(
                Value::thunk(
                    Grade::OMEGA,
                    Comp::lam(
                        "x",
                        Comp::perform(sig("E".into(), "op".into()), "op", Value::var("x")),
                    ),
                ),
                int_list((&[1, 2]).into()),
            ),
            "r",
            Comp::ret(Value::var("r")),
            vec![OpClause::new(
                "op",
                "p",
                "k",
                Comp::resume(Value::var("k"), Comp::ret(Value::var("p"))),
            )],
        ));

        // A **blaming** closure: applying `Add` to a non-numeric argument is the
        // gradual hole, so `each` over a non-empty list blames `Blame::Hole` on
        // both machines (the closure body runs against the continuation).
        check.pin(&each(
            closure1(
                ("x").into(),
                Comp::app(
                    Comp::app(Comp::native(NativePrim::Add), Value::var("x")),
                    Value::Unit,
                ),
            ),
            int_list((&[1, 2]).into()),
        ));

        // A higher-order combinator whose list carries a bound value (the
        // closure closes over the outer environment): `let n <- ret 5; each (\x.
        // x + n) [1, 2]` maps to `[6, 7]` — the un-focused closure closes `n`.
        check.pin(&Comp::bind(
            Comp::ret(Value::int(5)),
            "n",
            each(
                closure1(
                    ("x").into(),
                    Comp::app(
                        Comp::app(Comp::native(NativePrim::Add), Value::var("x")),
                        Value::var("n"),
                    ),
                ),
                int_list((&[1, 2]).into()),
            ),
        ));
    }

    /// Hand-built **exact readback** cases: a returned thunk / function / lazy
    /// pair / partial native reads back to an exact source term on both
    /// machines and compares **structurally** (retiring the kind-granularity
    /// arms). These would have agreed vacuously under kind granularity; they
    /// now exercise the body comparison, and an **intentional-difference
    /// probe** pins that the L machine's readback distinguishes
    /// structurally different terminals the old comparison called equal.
    #[test]
    fn hand_built_exact_readback_cases_agree()
    {
        let mut check = Check::load(("exact_readback").into());
        // A returned thunk — its body compared exactly.
        check.pin(&Comp::ret(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(5)),
        )));
        // A returned thunk with a compound body (a bind through a native).
        check.pin(&Comp::ret(Value::thunk(
            Grade::OMEGA,
            Comp::bind(
                Comp::app(
                    Comp::app(Comp::native(NativePrim::Add), Value::int(1)),
                    Value::int(2),
                ),
                "y",
                Comp::ret(Value::pair(Value::var("y"), Value::var("y"))),
            ),
        )));
        // A returned thunk that closes over an outer binding: `let n <- ret 9;
        // ret (thunk { ret n })` returns `thunk { ret 9 }` on both machines.
        check.pin(&Comp::bind(
            Comp::ret(Value::int(9)),
            "n",
            Comp::ret(Value::thunk(Grade::ONE, Comp::ret(Value::var("n")))),
        ));

        // A bare function terminal — its body compared exactly.
        check.pin(&Comp::lam("x", Comp::ret(Value::var("x"))));
        // A function terminal that closes over an outer binding: `let n <- ret 3;
        // \x. x + n` is the function `\x. x + 3` on both machines.
        check.pin(&Comp::bind(
            Comp::ret(Value::int(3)),
            "n",
            Comp::lam(
                "x",
                Comp::app(
                    Comp::app(Comp::native(NativePrim::Add), Value::var("x")),
                    Value::var("n"),
                ),
            ),
        ));

        // A bare lazy-pair terminal — both components compared exactly.
        check.pin(&Comp::with(
            Comp::ret(Value::int(1)),
            Comp::lam("z", Comp::ret(Value::var("z"))),
        ));

        // A partial native terminal — its accumulated argument compared exactly.
        check.pin(&Comp::app(Comp::native(NativePrim::Add), Value::int(7)));

        // The intentional-difference probe: the L machine's readback
        // distinguishes structurally different terminals that the retired
        // kind-granularity comparison would have called equal.
        let thunk_one = machine::run_comp(&Comp::ret(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(1)),
        )));
        let thunk_two = machine::run_comp(&Comp::ret(Value::thunk(
            Grade::ONE,
            Comp::ret(Value::int(2)),
        )));
        assert_ne!(
            canonical(&thunk_one),
            canonical(&thunk_two),
            "the readback distinguishes same-grade thunks with different bodies"
        );

        let fn_id = machine::run_comp(&Comp::lam("x", Comp::ret(Value::var("x"))));
        let fn_const = machine::run_comp(&Comp::lam("x", Comp::ret(Value::int(0))));
        assert_ne!(
            canonical(&fn_id),
            canonical(&fn_const),
            "the readback distinguishes functions with different bodies"
        );

        let native_seven =
            machine::run_comp(&Comp::app(Comp::native(NativePrim::Add), Value::int(7)));
        let native_eight =
            machine::run_comp(&Comp::app(Comp::native(NativePrim::Add), Value::int(8)));
        assert_ne!(
            canonical(&native_seven),
            canonical(&native_eight),
            "the readback distinguishes partial natives with different arguments"
        );
    }

    /// A higher-order combinator resolved through a **non-empty prelude** (the
    /// stage-D prelude path end to end): the force-position free name `g`
    /// resolves to a thunk whose body is an `each` program, so the prelude
    /// resolution drives the un-focusing native dispatch.
    #[test]
    fn hand_built_higher_order_prelude_case_agrees()
    {
        let mut check = Check::load(("higher_order_prelude").into());
        let program = each(
            closure1(
                ("x").into(),
                Comp::app(
                    Comp::app(Comp::native(NativePrim::Add), Value::var("x")),
                    Value::int(1),
                ),
            ),
            int_list((&[1, 2, 3]).into()),
        );
        check.pin_with_prelude(&Comp::force(Value::var("g")), &[(
            String::from("g"),
            Value::thunk(Grade::OMEGA, program),
        )]);
    }

    /// Hand-built prelude cases (ADR-42) pinning force-position free-name
    /// resolution against the oracle. Both machines drive the SAME prelude:
    /// a thunk-valued name forces to its body under the empty environment (a
    /// hit), an absent or non-thunk name stays `ForcedNonThunk` (a miss), the
    /// last binding shadows earlier ones, and a performing prelude body walks
    /// the live continuation to an enclosing handler exactly as a thunk body
    /// does.
    #[test]
    fn hand_built_prelude_cases_agree()
    {
        const FORCED_THUNK_BODY: i64 = 42;

        let mut check = Check::load(("prelude").into());
        // A hit: a thunk-valued name forces to its body, which continues.
        check.pin_with_prelude(&Comp::force(Value::var("f")), &[(
            String::from("f"),
            Value::thunk(Grade::ONE, Comp::ret(Value::int(FORCED_THUNK_BODY))),
        )]);
        // The forced body's value threads into the continuation.
        check.pin_with_prelude(
            &Comp::bind(
                Comp::force(Value::var("f")),
                "x",
                Comp::ret(Value::pair(Value::var("x"), Value::var("x"))),
            ),
            &[(
                String::from("f"),
                Value::thunk(Grade::OMEGA, Comp::ret(Value::int(7))),
            )],
        );
        // A name-bound native: force it, then apply it (the prelude'd
        // higher-order path — a thunk wrapping a builtin, as the CEK's prelude).
        check.pin_with_prelude(
            &Comp::app(
                Comp::app(Comp::force(Value::var("plus")), Value::int(3)),
                Value::int(4),
            ),
            &[(
                String::from("plus"),
                Value::thunk(Grade::OMEGA, Comp::native(NativePrim::Add)),
            )],
        );
        // A miss: the name is absent from the prelude — `ForcedNonThunk` on both.
        check.pin_with_prelude(&Comp::force(Value::var("missing")), &[(
            String::from("f"),
            Value::thunk(Grade::ONE, Comp::ret(Value::int(1))),
        )]);
        // A miss: the winning binding is NOT a thunk — `ForcedNonThunk` on both.
        check.pin_with_prelude(&Comp::force(Value::var("f")), &[(
            String::from("f"),
            Value::int(5),
        )]);
        // Shadowing: the LAST binding wins (a later thunk resolves to 2).
        check.pin_with_prelude(&Comp::force(Value::var("f")), &[
            (
                String::from("f"),
                Value::thunk(Grade::ONE, Comp::ret(Value::int(1))),
            ),
            (
                String::from("f"),
                Value::thunk(Grade::ONE, Comp::ret(Value::int(2))),
            ),
        ]);
        // Shadowing: a later NON-thunk shadows an earlier thunk —
        // `ForcedNonThunk`.
        check.pin_with_prelude(&Comp::force(Value::var("f")), &[
            (
                String::from("f"),
                Value::thunk(Grade::ONE, Comp::ret(Value::int(1))),
            ),
            (String::from("f"), Value::int(9)),
        ]);
        // The empty prelude is a force miss (`ForcedNonThunk`), as no-prelude.
        check.pin_with_prelude(&Comp::force(Value::var("f")), &[]);

        // A prelude body that itself performs: unhandled blames
        // `PerformNoHandler` on both.
        let eff_binding = (
            String::from("eff"),
            Value::thunk(
                Grade::OMEGA,
                Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
            ),
        );
        check.pin_with_prelude(
            &Comp::force(Value::var("eff")),
            core::slice::from_ref(&eff_binding),
        );
        // ... and under a handler the performed operation is caught (the
        // keep-continuation force path): `resume k (ret 8)`, so the handle
        // yields 8 on both machines.
        check.pin_with_prelude(
            &Comp::handle(
                sig("E".into(), "op".into()),
                Comp::force(Value::var("eff")),
                "x",
                Comp::ret(Value::var("x")),
                vec![OpClause::new(
                    "op",
                    "p",
                    "k",
                    Comp::resume(Value::var("k"), Comp::ret(Value::int(8))),
                )],
            ),
            core::slice::from_ref(&eff_binding),
        );

        // The empty prelude does not perturb the plain L run (byte-identical).
        let plain = Comp::force(Value::var("f"));
        assert_eq!(
            canonical(&machine::run_comp(&plain)),
            canonical(&machine::run_comp_with_prelude(&plain, &[])),
            "the empty prelude perturbs the plain L run"
        );
    }

    /// Drives `comp` under the SAME prelude `bindings` on the CEK oracle and
    /// the L machine, asserting they agree on the canonicalized outcome.
    /// A handler under an outer eliminator must consume that continuation once.
    ///
    /// The delimiter entry installs the projection on the live L-machine stack.
    /// Handler return and operation clauses therefore fall through to that
    /// stack rather than embedding the same projection in their focused
    /// commands.
    #[test]
    fn handler_resumption_under_projection_uses_the_ambient_continuation_once()
    {
        let mut check = Check::load(("handler_resumption_under_projection").into());
        check.pin(&Comp::prj1(Comp::handle(
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
        let mut check = Check::load(("effect").into());
        // An unhandled perform blames PerformNoHandler on both.
        check.pin(&Comp::perform(
            sig("E".into(), "op".into()),
            "op",
            Value::Unit,
        ));
        // ... including one buried under a bind (the operation propagates past
        // the μ̃ to search for a handler, finds none).
        check.pin(&Comp::bind(
            Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
        ));

        // A handled perform, non-resuming clause: the handle's value is the
        // clause's (the continuation is discarded).
        check.pin(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::perform(sig("E".into(), "op".into()), "op", Value::Unit),
            "x",
            Comp::ret(Value::var("x")),
            vec![OpClause::new("op", "p", "k", Comp::ret(Value::int(99)))],
        ));

        // A resuming clause: `resume k (ret 42)` feeds the resumption, so the
        // perform "returns" 42 to the scrutinee, then the return clause.
        check.pin(&Comp::handle(
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
        check.pin(&Comp::handle(
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
        check.pin(&Comp::handle(
            sig("E".into(), "op".into()),
            Comp::ret(Value::int(7)),
            "x",
            Comp::ret(Value::pair(Value::var("x"), Value::var("x"))),
            vec![OpClause::new("op", "p", "k", Comp::ret(Value::int(0)))],
        ));

        // A perform buried under a bind inside the handled scrutinee: resume
        // returns 7 to bind `x`, then `ret x`.
        check.pin(&Comp::handle(
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
        check.pin(&Comp::handle(
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
        check.pin(&Comp::handle(
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
        check.pin(&Comp::handle(
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
        check.pin(&Comp::reset(Comp::ret(Value::int(5))));
        // A `reset` delimiter blocks a `perform` from reaching an outer handler
        // (the v0 single-handler scope), so both blame PerformNoHandler.
        check.pin(&Comp::handle(
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
        check.pin(&Comp::handle(
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
        check.pin(&Comp::handle(
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
        let mut check = Check::load(("shift").into());
        // Discard the captured continuation: the shift's value is the reset's.
        check.pin(&Comp::reset(Comp::shift("k", Comp::ret(Value::int(5)))));
        // Discard across a bind: the enclosing `let x <- []; ret (x, x)`
        // continuation is dropped, so the reset yields the shift body's value.
        check.pin(&Comp::reset(Comp::bind(
            Comp::shift("k", Comp::ret(Value::int(9))),
            "x",
            Comp::ret(Value::pair(Value::var("x"), Value::var("x"))),
        )));
        // Resume once (identity-shaped): `resume k (ret 3)` re-invokes the
        // captured `let x <- []; ret (x, x)`, so x = 3.
        check.pin(&Comp::reset(Comp::bind(
            Comp::shift("k", Comp::resume(Value::var("k"), Comp::ret(Value::int(3)))),
            "x",
            Comp::ret(Value::pair(Value::var("x"), Value::var("x"))),
        )));
        // The resumed continuation runs real work: the native `Add` inside the
        // captured continuation computes 3 + 4 = 7.
        check.pin(&Comp::reset(Comp::bind(
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
        check.pin(&Comp::reset(Comp::bind(
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
        check.pin(&Comp::reset(Comp::reset(Comp::shift(
            "k",
            Comp::ret(Value::int(4)),
        ))));
        // A shift under an outer reset but an inner-most bind, resuming, then the
        // reset delimits: x = 6, result (6, 6).
        check.pin(&Comp::reset(Comp::bind(
            Comp::shift("k", Comp::resume(Value::var("k"), Comp::ret(Value::int(6)))),
            "x",
            Comp::ret(Value::pair(Value::var("x"), Value::var("x"))),
        )));

        // Undelimited: a bare `shift` reaches no prompt, so both blame
        // ShiftNoReset.
        check.pin(&Comp::shift("k", Comp::ret(Value::int(1))));
        // ... including one buried under a bind (still no enclosing prompt).
        check.pin(&Comp::bind(
            Comp::shift("k", Comp::ret(Value::int(1))),
            "x",
            Comp::ret(Value::var("x")),
        ));
        // Across a handler: the capture would cross a `KHandle` frame, which the
        // v0 structural model cannot reify, so both blame ShiftNoReset (a handler
        // is not a prompt).
        check.pin(&Comp::handle(
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
    // (signature name, operation, payload) on both. The payload reads back
    // through the un-focusing readback `𝓕⁻¹` — exact on the first-order fragment
    // AND on higher-order payloads (a thunk closure closes under its captured
    // environment) — so both are exercised below and match byte-for-byte.

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

    /// Borrowed expected host-offer rows.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct ExpectedHostLog<'rows>(&'rows [(&'static str, &'static str, Value)]);

    impl<'rows, const COUNT: usize> From<&'rows [(&'static str, &'static str, Value); COUNT]>
        for ExpectedHostLog<'rows>
    {
        fn from(rows: &'rows [(&'static str, &'static str, Value); COUNT]) -> Self
        {
            Self(rows)
        }
    }

    /// Drives `comp` with the scripted host on the L machine, asserting its
    /// final outcome matches `expected_final` and its host offer log equals
    /// `expected_log`. Each case carries its own expected outcome, so the CEK
    /// leg this cross-checked against retired with the oracle.
    fn assert_host_seam(
        comp: &Comp,
        replies: &[HostReply],
        expected_log: ExpectedHostLog<'_>,
        expected_final: &Eval,
    )
    {
        let mut l_host = ScriptedHost::new(replies);
        let machine = machine::run_comp_with_host(comp, &mut l_host);

        assert!(
            bool::from(agree(&machine, expected_final)),
            "host-seam final mismatch on {comp:?}\n  got      = {:?}\n  expected = {:?}",
            canonical(&machine),
            canonical(expected_final)
        );

        let expected: Vec<(String, String, Value)> = expected_log
            .0
            .iter()
            .map(|&(sig, op, ref payload)| (sig.to_owned(), op.to_owned(), payload.clone()))
            .collect();
        assert_eq!(l_host.log, expected, "L host log mismatch on {comp:?}");
    }

    /// (a) A single unhandled `perform`, resumed by the host: both machines
    /// take the reply as the outcome.
    #[test]
    fn host_seam_resumes_a_single_unhandled_perform()
    {
        const HOST_REPLY: i64 = 42;

        assert_host_seam(
            &Comp::perform(sig("E".into(), "op".into()), "op", Value::int(5)),
            &[HostReply::Resume(Value::int(HOST_REPLY))],
            (&[("E", "op", Value::int(5))]).into(),
            &Eval::Value(Comp::ret(Value::int(HOST_REPLY))),
        );
    }

    /// (b) The host declines the offer: both machines blame `PerformNoHandler`.
    #[test]
    fn host_seam_decline_blames_perform_no_handler()
    {
        assert_host_seam(
            &Comp::perform(sig("E".into(), "op".into()), "op", Value::int(5)),
            &[HostReply::Unhandled],
            (&[("E", "op", Value::int(5))]).into(),
            &Eval::Blame(Blame::PerformNoHandler),
        );
    }

    /// (c) Sequential performs with deep re-entry: the resumed continuation
    /// performs again and is offered again, in execution order, and the second
    /// offer's payload is the value the first reply bound.
    #[test]
    fn host_seam_offers_deep_re_entry_in_order()
    {
        const SECOND_REPLY: i64 = 99;

        assert_host_seam(
            &Comp::bind(
                Comp::perform(sig("E".into(), "op".into()), "op", Value::int(1)),
                "x",
                Comp::perform(sig("E".into(), "op".into()), "op", Value::var("x")),
            ),
            &[
                HostReply::Resume(Value::int(10)),
                HostReply::Resume(Value::int(SECOND_REPLY)),
            ],
            (&[("E", "op", Value::int(1)), ("E", "op", Value::int(10))]).into(),
            &Eval::Value(Comp::ret(Value::int(SECOND_REPLY))),
        );
    }

    /// (d) A `perform` under a source handler that claims a DIFFERENT operation
    /// reaches the host; the source handler still claims its own operation
    /// (which never reaches the host).
    #[test]
    fn host_seam_offers_across_a_non_matching_handler()
    {
        const HANDLED_CLAUSE_VALUE: i64 = 555;
        const CROSS_HANDLER_REPLY: i64 = 20;

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
                vec![OpClause::new(
                    "keep",
                    "p",
                    "k",
                    Comp::ret(Value::int(HANDLED_CLAUSE_VALUE)),
                )],
            ),
            &[HostReply::Resume(Value::int(CROSS_HANDLER_REPLY))],
            // Only the escaping `esc` reaches the host; the source handler claims
            // `keep` itself, so it is never offered.
            (&[("E", "esc", Value::int(7))]).into(),
            &Eval::Value(Comp::ret(Value::int(HANDLED_CLAUSE_VALUE))),
        );
    }

    /// (e) A `perform` a source handler claims never consults the host (empty
    /// log).
    #[test]
    fn host_seam_never_consulted_for_a_claimed_perform()
    {
        const CLAIMED_CLAUSE_VALUE: i64 = 77;

        assert_host_seam(
            &Comp::handle(
                sig("E".into(), "op".into()),
                Comp::perform(sig("E".into(), "op".into()), "op", Value::int(5)),
                "r",
                Comp::ret(Value::var("r")),
                vec![OpClause::new(
                    "op",
                    "p",
                    "k",
                    Comp::ret(Value::int(CLAIMED_CLAUSE_VALUE)),
                )],
            ),
            // A non-empty script would be a bug magnet; the host must not be
            // consulted at all.
            &[HostReply::Resume(Value::int(0))],
            (&[]).into(),
            &Eval::Value(Comp::ret(Value::int(CLAIMED_CLAUSE_VALUE))),
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
            (&[("E", "op", Value::Unit)]).into(),
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
            (&[("E", "op", Value::pair(Value::int(1), Value::int(2)))]).into(),
            &Eval::Value(Comp::ret(Value::Unit)),
        );
        assert_host_seam(
            &Comp::perform(
                sig("E".into(), "op".into()),
                "op",
                Value::list(vec![Value::int(1), Value::int(2), Value::int(3)]),
            ),
            &[HostReply::Resume(Value::Unit)],
            (&[(
                "E",
                "op",
                Value::list(vec![Value::int(1), Value::int(2), Value::int(3)]),
            )])
                .into(),
            &Eval::Value(Comp::ret(Value::Unit)),
        );
        assert_host_seam(
            &Comp::perform(
                sig("E".into(), "op".into()),
                "op",
                Value::record([
                    (String::from("a"), Value::int(1)),
                    (String::from("b"), Value::string("hi")),
                ]),
            ),
            &[HostReply::Resume(Value::Unit)],
            (&[(
                "E",
                "op",
                Value::record([
                    (String::from("a"), Value::int(1)),
                    (String::from("b"), Value::string("hi")),
                ]),
            )])
                .into(),
            &Eval::Value(Comp::ret(Value::Unit)),
        );
    }

    /// (h) A **higher-order** payload (a thunk closure) reads back to the
    /// identical public `Value` on both machines, so the host log matches
    /// byte-for-byte — the un-focusing readback `𝓕⁻¹` closes the thunk body
    /// under its captured environment exactly as the CEK's `quote_value` does.
    #[test]
    fn host_seam_higher_order_payload_reads_back_exactly()
    {
        // A bare thunk payload.
        assert_host_seam(
            &Comp::perform(
                sig("E".into(), "op".into()),
                "op",
                Value::thunk(Grade::ONE, Comp::ret(Value::int(5))),
            ),
            &[HostReply::Resume(Value::Unit)],
            (&[(
                "E",
                "op",
                Value::thunk(Grade::ONE, Comp::ret(Value::int(5))),
            )])
                .into(),
            &Eval::Value(Comp::ret(Value::Unit)),
        );
        // A thunk closure that closes over an outer binding: the readback closes
        // `n ↦ 7` into the body, so both machines present `thunk { ret 7 }`.
        assert_host_seam(
            &Comp::bind(
                Comp::ret(Value::int(7)),
                "n",
                Comp::perform(
                    sig("E".into(), "op".into()),
                    "op",
                    Value::thunk(Grade::OMEGA, Comp::ret(Value::var("n"))),
                ),
            ),
            &[HostReply::Resume(Value::Unit)],
            (&[(
                "E",
                "op",
                Value::thunk(Grade::OMEGA, Comp::ret(Value::int(7))),
            )])
                .into(),
            &Eval::Value(Comp::ret(Value::Unit)),
        );
    }

    /// Asserts the two machines agree on `comp`, panicking with the disagreeing
    /// canonicalization otherwise.
    /// The regeneration switch for the hand-built differential L-outcome
    /// snapshots.
    const BLESS_ENV: &str = "GANDR_BLESS_DIFFERENTIAL_OUTCOMES";

    /// Borrowed snapshot-suite label.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct SnapshotLabel<'label>(&'label str);

    impl<'label> From<&'label str> for SnapshotLabel<'label>
    {
        fn from(label: &'label str) -> Self
        {
            Self(label)
        }
    }

    impl core::fmt::Display for SnapshotLabel<'_>
    {
        fn fmt(
            &self,
            f: &mut core::fmt::Formatter<'_>,
        ) -> core::fmt::Result
        {
            f.write_str(self.0)
        }
    }

    impl SnapshotLabel<'_>
    {
        /// Resolve the label's checked-in snapshot path.
        fn path(self) -> std::path::PathBuf
        {
            std::path::PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/tests/fixtures/differential"
            ))
            .join(format!("{}.snap", self.0))
        }
    }

    /// An ordered **L-outcome regression** check for a hand-built case suite.
    ///
    /// Through stage E each hand-built case asserted `canonical(L) ==
    /// canonical(CEK)` against the retiring oracle. With the CEK gone, each
    /// case instead pins the L machine's own canonical outcome — frozen
    /// once from the final oracle-agreeing run — as a checked-in
    /// regression, and re-verifies the outcome is deterministic across two
    /// runs (an intrinsic L-machine property). The snapshots live under
    /// `tests/fixtures/differential/<label>.snap`, one canonical outcome per
    /// case in call order; regenerate them with
    /// `GANDR_BLESS_DIFFERENTIAL_OUTCOMES=1`. The [`Drop`] guard checks that a
    /// non-blessing run consumed exactly the recorded outcomes (so a
    /// removed / added case is caught), and writes the file on a blessing run.
    struct Check
    {
        /// The snapshot file stem under `tests/fixtures/differential`.
        label: SnapshotLabel<'static>,
        /// The recorded per-case canonical outcomes, in call order (empty while
        /// blessing).
        expected: Vec<String>,
        /// The next expected-outcome index.
        cursor: usize,
        /// Whether this run regenerates the snapshot instead of checking it.
        blessing: bool,
        /// The outcomes observed this run (written back only when blessing).
        recorded: Vec<String>,
    }

    impl Check
    {
        /// Loads (or, when blessing, prepares to regenerate) the snapshot suite
        /// `label`.
        fn load(label: SnapshotLabel<'static>) -> Self
        {
            let blessing = std::env::var_os(BLESS_ENV).is_some();
            let expected = if blessing {
                Vec::new()
            }
            else {
                read_snapshot(label)
            };
            Self {
                label,
                expected,
                cursor: 0,
                blessing,
                recorded: Vec::new(),
            }
        }

        /// Pins the L machine's canonical outcome of `comp` (empty prelude)
        /// against the recorded regression, checking determinism.
        fn pin(
            &mut self,
            comp: &Comp,
        )
        {
            self.record(comp, &machine::run_comp(comp), &machine::run_comp(comp));
        }

        /// Pins the L outcome of `comp` under a prelude binding-environment
        /// (ADR-42).
        fn pin_with_prelude(
            &mut self,
            comp: &Comp,
            bindings: &[(String, Value)],
        )
        {
            self.record(
                comp,
                &machine::run_comp_with_prelude(comp, bindings),
                &machine::run_comp_with_prelude(comp, bindings),
            );
        }

        /// Records / checks one case: the two runs must canonicalize equally
        /// (determinism), and the outcome must match the recorded regression.
        fn record(
            &mut self,
            comp: &Comp,
            first: &Eval,
            second: &Eval,
        )
        {
            let outcome = canonical(first);
            assert_eq!(
                outcome,
                canonical(second),
                "the L machine is non-deterministic on {comp:?}"
            );
            let rendered = format!("{outcome:?}");
            if self.blessing {
                self.recorded.push(rendered);
                return;
            }
            let expected = self.expected.get(self.cursor).unwrap_or_else(|| {
                panic!(
                    "differential snapshot `{}` has no outcome for case {} ({comp:?}); regenerate \
                     with {BLESS_ENV}=1",
                    self.label, self.cursor
                )
            });
            assert_eq!(
                &rendered, expected,
                "L-outcome regression on {comp:?} (case {} of `{}`); regenerate with {BLESS_ENV}=1 \
                 if the change is intended",
                self.cursor, self.label
            );
            self.cursor = self.cursor.saturating_add(1);
        }
    }

    impl Drop for Check
    {
        fn drop(&mut self)
        {
            // Never assert while unwinding a case failure — that would abort on
            // a double panic and hide the real assertion.
            if std::thread::panicking() {
                return;
            }
            if self.blessing {
                write_snapshot(self.label, &self.recorded);
                return;
            }
            assert_eq!(
                self.cursor,
                self.expected.len(),
                "differential snapshot `{}` records {} outcomes but the suite pinned {}; \
                 regenerate with {BLESS_ENV}=1",
                self.label,
                self.expected.len(),
                self.cursor
            );
        }
    }

    /// Reads a suite's recorded canonical outcomes, in order.
    fn read_snapshot(label: SnapshotLabel<'_>) -> Vec<String>
    {
        let path = label.path();
        let text = std::fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "cannot read differential snapshot `{}` ({error}); regenerate with {BLESS_ENV}=1",
                path.display()
            )
        });
        text.lines()
            .filter(|line| !line.starts_with(';'))
            .map(str::to_owned)
            .collect()
    }

    /// Writes a suite's canonical outcomes (blessing path).
    fn write_snapshot(
        label: SnapshotLabel<'_>,
        outcomes: &[String],
    )
    {
        let path = label.path();
        std::fs::create_dir_all(path.parent().expect("snapshot has a parent"))
            .unwrap_or_else(|error| panic!("cannot create fixture dir: {error}"));
        let mut lines = vec![
            format!("; gandr differential L-outcome snapshot (B1 exit gate): {label}"),
            format!("; cases: {}", outcomes.len()),
        ];
        lines.extend(outcomes.iter().cloned());
        let mut out = lines.join("\n");
        out.push('\n');
        std::fs::write(&path, out)
            .unwrap_or_else(|error| panic!("cannot write `{}`: {error}", path.display()));
    }

    /// A one-operation effect signature (the sig's operation types are inert to
    /// the CEK's `run_comp`, which routes by operation name — ADR-33 D3).
    fn sig(
        name: EffectSignatureName<'_>,
        op: OperationName<'_>,
    ) -> EffectSig
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
