//! Runtime behavior of first-class module packages on the L machine.
//!
//! The package former buys its abstraction **statically**: the checker refuses
//! a client that reaches the representation, and `𝓕` then erases the whole
//! device — a pack becomes its payload and an unpack becomes a bind of a
//! return. What is worth testing at this layer is that the erasure is
//! *faithful*, and the sharpest witness is the one the module design names:
//! **module-as-value dispatch**, choosing an implementation at run time.
//!
//! Each test below runs the *same* well-typed program through both the checker
//! and the machine, so the two halves of the claim are pinned together: a
//! program the checker accepts is a program the machine runs to the answer the
//! chosen implementation gives.

/// Module-as-value dispatch and the erasure that carries it.
#[cfg(test)]
mod tests
{
    use alloc::rc::Rc;

    use gandr_core_checker::machine::control::Dir;
    use gandr_core_sequent::machine::run_comp;
    use gandr_core_term::boundary::IntegerLiteral;
    use gandr_core_term::ctx::Ctx;
    use gandr_core_term::effect::EffectRow;
    use gandr_core_term::grade::Grade;
    use gandr_core_term::outcome::Eval;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::Side;
    use gandr_core_term::syntax::Value;
    use gandr_core_term::types::CompType;
    use gandr_core_term::types::SealId;
    use gandr_core_term::types::Ty;
    use gandr_core_term::types::ValueType;

    /// `U_ω (F payload)`.
    fn returner_thunk(payload: ValueType) -> ValueType
    {
        ValueType::thunk(
            Grade::OMEGA,
            CompType::F(Rc::new(payload), EffectRow::EMPTY),
        )
    }

    /// The counter signature, abstracting its state type `t`:
    /// `Package_ω ⟨t⟩ U_ω (F #{ read: U_ω (t → F Integer), seed: t })`.
    fn counter() -> ValueType
    {
        ValueType::package(
            Grade::OMEGA,
            ["t"],
            returner_thunk(ValueType::record([
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
            ])),
        )
    }

    /// A counter over `Integer` whose reader is the identity, so its answer is
    /// its own seed.
    fn integer_counter(seed: IntegerLiteral) -> Value
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
                    ("seed".to_owned(), Value::int(seed)),
                ])),
            ),
        )
    }

    /// A counter over `String` — a structurally different representation behind
    /// the same signature — whose reader answers a constant.
    fn string_counter(answer: IntegerLiteral) -> Value
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
                            Comp::lam_ann("s", ValueType::string(), Comp::ret(Value::int(answer))),
                        ),
                    ),
                    ("seed".to_owned(), Value::string("seven")),
                ])),
            ),
        )
    }

    /// Force the module, project its reader and its seed, and apply the one to
    /// the other — the only route the signature offers.
    fn read_the_seed() -> Comp
    {
        Comp::bind(
            Comp::force(Value::var("m")),
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

    /// The dispatching program: pick a counter by `side`, then consume
    /// whichever arrived through the signature alone.
    ///
    /// The chooser is a thunk ascribed at `U_ω (F Counter)` so the `case` — a
    /// check-only form — is reached in checking mode and the bind above it
    /// still infers, which is how a check-only computation is sequenced in
    /// this core.
    fn dispatch(side: Side) -> Comp
    {
        let choice = Value::annot(
            Value::Inj(side, Rc::new(Value::Unit)),
            ValueType::sum(ValueType::Unit, ValueType::Unit),
        );
        let chooser = Value::annot(
            Value::thunk(
                Grade::OMEGA,
                Comp::case(
                    choice,
                    "_left",
                    Comp::ret(integer_counter(IntegerLiteral::from(7_i64))),
                    "_right",
                    Comp::ret(string_counter(IntegerLiteral::from(42_i64))),
                ),
            ),
            returner_thunk(counter()),
        );
        Comp::bind(
            Comp::force(chooser),
            "p",
            Comp::unpack(
                Value::var("p"),
                counter(),
                [SealId::new(0_u64, "dispatch", "t")],
                "m",
                read_the_seed(),
            ),
        )
    }

    /// **Module-as-value dispatch.** The implementation is chosen at run time,
    /// the consumer sees only the signature, and the answer is the chosen
    /// implementation's — so the choice is observable and the abstraction costs
    /// nothing at run time.
    #[test]
    fn a_package_chosen_at_runtime_answers_through_its_signature()
    {
        for (side, answer) in [(Side::Fst, 7_i64), (Side::Snd, 42_i64)] {
            let program = dispatch(side);
            let expected = CompType::returner(ValueType::integer());
            let (checked, _) = gandr_core_checker::judgements::checker::run_comp(
                Ctx::new(),
                program.clone(),
                Dir::Check(expected.clone()),
            );
            assert_eq!(
                checked,
                Ok(Ty::Comp(expected)),
                "the dispatching program is well typed at F Integer"
            );
            assert_eq!(
                run_comp(&program),
                Eval::Value(Comp::ret(Value::int(answer))),
                "the answer is the chosen implementation's, so the dispatch is observable"
            );
        }
    }

    /// The erasure is faithful in the small: `unpack (pack ⟨Ā⟩ v) as ⟨ā⟩ m in
    /// t` runs exactly as `t[v/m]` does, both focusing to a bind of a
    /// return.
    #[test]
    fn unpacking_a_literal_package_runs_as_a_binding()
    {
        let signature =
            ValueType::package(Grade::OMEGA, ["t"], returner_thunk(ValueType::atom("t")));
        let packed = Value::pack(
            [ValueType::integer()],
            Value::thunk(Grade::OMEGA, Comp::ret(Value::int(3_i64))),
        );
        let payload = Value::thunk(Grade::OMEGA, Comp::ret(Value::int(3_i64)));
        let unpacked = Comp::unpack(
            packed,
            signature,
            [SealId::new(0_u64, "literal", "t")],
            "m",
            Comp::force(Value::var("m")),
        );
        let substituted = Comp::force(payload);
        assert_eq!(
            run_comp(&unpacked),
            run_comp(&substituted),
            "the elimination of a literal package is the substitution of its payload"
        );
        assert_eq!(
            run_comp(&unpacked),
            Eval::Value(Comp::ret(Value::int(3_i64))),
            "and the answer is the payload's own"
        );
    }
}
