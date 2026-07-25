//! The slice-2 acceptance gate (kernel-boundary design record §8):
//! **empty-poset agreement** — with no landmark constraints declared,
//! Bezem–Coquand entailment must agree with the slice-1 free-fragment
//! oracle on every generated input, exactly and in both strictness modes.
//! The two deciders share no decision machinery: slice 1 compares canonical
//! forms by domination, slice 2 saturates a Horn clause system — so
//! agreement is a genuine differential, not a tautology
//! (docs/workflow/mutation-adequacy.md, the external-oracle rule, with the
//! slice-1 oracle — itself differentially anchored to a semantic
//! reference — as the external oracle).
//!
//! The suite also pins, under a fixed nonempty poset: evidence validation
//! on every decided query (both dichotomy branches), the
//! `lt ≡ leq ∘ succ` law, and monotonicity of entailment in the
//! hypotheses (whatever the free oracle accepts, an admitted poset
//! accepts).

/// Differential properties for landmark-poset entailment.
#[cfg(test)]
mod tests
{
    use gandr_kernel_strata::AdmissionOutcome;
    use gandr_kernel_strata::Entailment;
    use gandr_kernel_strata::EntailmentHolds;
    use gandr_kernel_strata::LandmarkConstraint;
    use gandr_kernel_strata::LandmarkPoset;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;
    use gandr_kernel_strata::Strictness;
    use gandr_kernel_strata::validate_entailment_countermodel;
    use gandr_kernel_strata::validate_entailment_witness;
    use proptest::prelude::*;

    /// A free term of the level algebra, prior to canonicalization.
    #[derive(Clone, Debug)]
    enum Ast
    {
        /// The level `0`.
        Zero,
        /// A level variable, by index.
        Var(u32),
        /// The successor of a term.
        Succ(Box<Self>),
        /// The join of two terms.
        Max(Box<Self>, Box<Self>),
    }

    /// The number of generated variables in the free level algebra.
    const VARIABLE_COUNT: u32 = 4;
    /// The maximum recursion depth of generated level ASTs.
    const AST_RECURSION_DEPTH: u32 = 6;
    /// The maximum generated level-AST node budget.
    const AST_NODE_BUDGET: u32 = 48;
    /// The number of recursive branches per generated level-AST node.
    const AST_BRANCH_COUNT: u32 = 2;

    /// The proptest strategy: free terms over four variables, recursion
    /// depth at most six (the slice-1 differential's strategy).
    fn ast_strategy() -> impl Strategy<Value = Ast>
    {
        let leaf = prop_oneof![
            Just(Ast::Zero),
            (0_u32 .. VARIABLE_COUNT).prop_map(Ast::Var)
        ];
        leaf.prop_recursive(
            AST_RECURSION_DEPTH,
            AST_NODE_BUDGET,
            AST_BRANCH_COUNT,
            |inner| {
                prop_oneof![
                    inner.clone().prop_map(|term| Ast::Succ(Box::new(term))),
                    (inner.clone(), inner)
                        .prop_map(|(left, right)| Ast::Max(Box::new(left), Box::new(right))),
                ]
            },
        )
    }

    /// Fold a free term into a canonical [`Level`] through the smart
    /// constructors.
    fn to_level(term: &Ast) -> Level
    {
        enum Frame<'term>
        {
            Visit(&'term Ast),
            ApplySucc,
            ApplyMax,
        }

        let mut stack = vec![Frame::Visit(term)];
        let mut levels = Vec::new();
        while let Some(frame) = stack.pop() {
            match frame {
                | Frame::Visit(current) => match *current {
                    | Ast::Zero => levels.push(Level::zero()),
                    | Ast::Var(index) => {
                        levels.push(Level::var(LevelVar::new(LevelVarIndex::from(index))));
                    },
                    | Ast::Succ(ref inner) => {
                        stack.push(Frame::ApplySucc);
                        stack.push(Frame::Visit(inner));
                    },
                    | Ast::Max(ref left, ref right) => {
                        stack.push(Frame::ApplyMax);
                        stack.push(Frame::Visit(right));
                        stack.push(Frame::Visit(left));
                    },
                },
                | Frame::ApplySucc => {
                    let inner = levels.pop().unwrap();
                    levels.push(inner.succ().unwrap());
                },
                | Frame::ApplyMax => {
                    let right = levels.pop().unwrap();
                    let left = levels.pop().unwrap();
                    levels.push(left.max(&right));
                },
            }
        }
        levels.pop().unwrap()
    }

    /// The empty poset — the degeneration the acceptance gate quantifies
    /// over.
    fn empty_poset() -> LandmarkPoset
    {
        admitted(vec![])
    }

    /// A fixed nonempty poset over the strategy's variable range:
    /// `x0 ≤ x1` and `x1 + 1 ≤ x2` (so hypothesis paths, strict and
    /// non-strict, are genuinely exercised by generated queries).
    fn fixed_poset() -> LandmarkPoset
    {
        let x0 = Level::var(LevelVar::new(LevelVarIndex::from(0_u32)));
        let x1 = Level::var(LevelVar::new(LevelVarIndex::from(1_u32)));
        let x1_plus_1 = x1.succ().unwrap();
        let x2 = Level::var(LevelVar::new(LevelVarIndex::from(2_u32)));
        admitted(vec![
            LandmarkConstraint::leq(x0, x1).unwrap(),
            LandmarkConstraint::leq(x1_plus_1, x2).unwrap(),
        ])
    }

    /// Admit a constraint set that must admit.
    fn admitted(constraints: Vec<LandmarkConstraint>) -> LandmarkPoset
    {
        match LandmarkPoset::admit(constraints).unwrap() {
            | AdmissionOutcome::Admitted(poset) => poset,
            | AdmissionOutcome::Loop(_witness) => panic!("expected admission"),
        }
    }

    /// Decide under `poset`, validating whichever evidence comes back, and
    /// return the boolean verdict.
    fn decide_validated(
        poset: &LandmarkPoset,
        left: &Level,
        right: &Level,
        strict: Strictness,
    ) -> EntailmentHolds
    {
        let decision = if bool::from(strict) {
            poset.entails_lt_with_evidence(left, right).unwrap()
        }
        else {
            poset.entails_leq_with_evidence(left, right).unwrap()
        };
        match decision {
            | Entailment::Holds(witness) => {
                assert_eq!(witness.strict(), strict, "the witness records its mode");
                assert_eq!(
                    Ok(()),
                    validate_entailment_witness(poset, left, right, &witness),
                    "every oracle witness must validate"
                );
                EntailmentHolds::from(true)
            },
            | Entailment::Refuted(countermodel) => {
                assert_eq!(
                    countermodel.strict(),
                    strict,
                    "the countermodel records its mode"
                );
                assert_eq!(
                    Ok(()),
                    validate_entailment_countermodel(poset, left, right, &countermodel),
                    "every oracle countermodel must validate"
                );
                EntailmentHolds::from(false)
            },
        }
    }

    proptest! {
        /// THE ACCEPTANCE GATE — empty-poset entailment is the slice-1
        /// oracle, exactly, in both strictness modes.
        #[test]
        fn prop_empty_poset_agrees_with_the_slice1_oracle(
            (left, right) in (ast_strategy(), ast_strategy())
        )
        {
            let poset = empty_poset();
            let left_level = to_level(&left);
            let right_level = to_level(&right);
            prop_assert_eq!(
                bool::from(poset.entails_leq(&left_level, &right_level).unwrap()),
                bool::from(left_level.leq(&right_level))
            );
            prop_assert_eq!(
                bool::from(poset.entails_lt(&left_level, &right_level).unwrap()),
                bool::from(left_level.lt(&right_level))
            );
        }

        /// Every piece of evidence entailment returns validates against
        /// its query — empty poset, both modes, both branches.
        #[test]
        fn prop_empty_poset_evidence_validates(
            (left, right) in (ast_strategy(), ast_strategy())
        )
        {
            let poset = empty_poset();
            let left_level = to_level(&left);
            let right_level = to_level(&right);
            for strict in [Strictness::NON_STRICT, Strictness::STRICT] {
                let _verdict = decide_validated(&poset, &left_level, &right_level, strict);
            }
        }

        /// Every piece of evidence entailment returns validates against
        /// its query — fixed nonempty poset, both modes, both branches
        /// (hypothesis paths included).
        #[test]
        fn prop_fixed_poset_evidence_validates(
            (left, right) in (ast_strategy(), ast_strategy())
        )
        {
            let poset = fixed_poset();
            let left_level = to_level(&left);
            let right_level = to_level(&right);
            for strict in [Strictness::NON_STRICT, Strictness::STRICT] {
                let _verdict = decide_validated(&poset, &left_level, &right_level, strict);
            }
        }

        /// The strict order is exactly the non-strict order after a
        /// successor on the left — under hypotheses too.
        #[test]
        fn prop_lt_equals_succ_leq_under_the_fixed_poset(
            (left, right) in (ast_strategy(), ast_strategy())
        )
        {
            let poset = fixed_poset();
            let left_level = to_level(&left);
            let right_level = to_level(&right);
            let via_lt = poset.entails_lt(&left_level, &right_level).unwrap();
            let via_succ = poset
                .entails_leq(&left_level.succ().unwrap(), &right_level)
                .unwrap();
            prop_assert_eq!(via_lt, via_succ);
        }

        /// Hypotheses only ever add entailments: whatever the free oracle
        /// accepts, an admitted poset accepts (monotonicity).
        #[test]
        fn prop_hypotheses_are_monotone(
            (left, right) in (ast_strategy(), ast_strategy())
        )
        {
            let poset = fixed_poset();
            let left_level = to_level(&left);
            let right_level = to_level(&right);
            if bool::from(left_level.leq(&right_level)) {
                prop_assert!(bool::from(poset.entails_leq(&left_level, &right_level).unwrap()));
            }
            if bool::from(left_level.lt(&right_level)) {
                prop_assert!(bool::from(poset.entails_lt(&left_level, &right_level).unwrap()));
            }
        }

        /// Entailment under hypotheses stays reflexive and transitive on
        /// generated levels (it is a provability relation).
        #[test]
        fn prop_entailment_order_laws_under_the_fixed_poset(
            (a, b, c) in (ast_strategy(), ast_strategy(), ast_strategy())
        )
        {
            let poset = fixed_poset();
            let la = to_level(&a);
            let lb = to_level(&b);
            let lc = to_level(&c);
            prop_assert!(bool::from(poset.entails_leq(&la, &la).unwrap()));
            prop_assert!(!bool::from(poset.entails_lt(&la, &la).unwrap()));
            if bool::from(poset.entails_leq(&la, &lb).unwrap())
                && bool::from(poset.entails_leq(&lb, &lc).unwrap())
            {
                prop_assert!(bool::from(poset.entails_leq(&la, &lc).unwrap()));
            }
        }
    }
}
