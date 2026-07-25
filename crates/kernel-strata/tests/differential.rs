//! Differential property tests for the level oracle over the public API: a
//! free `{0, var, +1, max}` term AST is generated, folded into canonical
//! [`Level`]s through the smart constructors, and every oracle answer is
//! cross-checked against an **independent semantic reference** — direct AST
//! evaluation over a provably complete finite valuation family (the zero
//! valuation plus, per variable, a spike one past the right term's successor
//! count: a violation of atom domination shows at a spike, a violation of
//! the constant bound at zero, and domination itself is pointwise). The
//! reference never looks at canonical forms, offsets, or witnesses — it only
//! evaluates — so it is external to everything the oracle computes
//! (`../../../docs/workflow/mutation-adequacy.md`, the external-oracle rule).
//!
//! The suite also pins the algebraic laws of the ADR-78 algebra (`max`
//! commutative, associative, and idempotent with `zero` unit, successor
//! distribution), the order laws (reflexivity, irreflexivity, antisymmetry
//! against canonical equality, transitivity), the `lt ≡ leq ∘ succ`
//! consistency law, and evidence validation on every decided pair.

extern crate alloc;

/// Differential and law properties for the level oracle.
#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::collections::BTreeSet;

    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelValue;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;
    use gandr_kernel_strata::OrderComparison;
    use gandr_kernel_strata::Strictness;
    use gandr_kernel_strata::validate_refutation;
    use gandr_kernel_strata::validate_witness;
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
    /// depth at most six.
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

    /// The semantic reference decision: `left + shift <= right` at every
    /// member of the complete valuation family.
    fn semantic_leq(
        left: &Ast,
        right: &Ast,
        strict: Strictness,
    ) -> OrderComparison
    {
        let shift = u128::from(bool::from(strict));
        let holds = valuation_family(left, right).iter().all(|valuation| {
            u128::from(eval_ast(left, valuation)) + shift <= u128::from(eval_ast(right, valuation))
        });
        OrderComparison::from(holds)
    }

    /// The complete valuation family for deciding `left + shift <= right`:
    /// the zero valuation plus, for each variable of either side, a spike
    /// one past `right`'s successor count. Domination failures on an atom
    /// show at that variable's spike; constant-bound failures show at zero;
    /// and when domination holds the order holds pointwise everywhere.
    fn valuation_family(
        left: &Ast,
        right: &Ast,
    ) -> Vec<BTreeMap<LevelVarIndex, LevelValue>>
    {
        let mut vars = BTreeSet::new();
        collect_vars(left, &mut vars);
        collect_vars(right, &mut vars);
        let spike = LevelValue::from(u128::from(succ_count(right)) + 1);
        let mut family = vec![BTreeMap::new()];
        for &index in &vars {
            let mut valuation = BTreeMap::new();
            valuation.insert(index, spike);
            family.push(valuation);
        }
        family
    }

    /// Collect the variable indices of a term.
    fn collect_vars(
        term: &Ast,
        into: &mut BTreeSet<LevelVarIndex>,
    )
    {
        let mut stack = vec![term];
        while let Some(current) = stack.pop() {
            match *current {
                | Ast::Zero => {},
                | Ast::Var(index) => {
                    into.insert(LevelVarIndex::from(index));
                },
                | Ast::Succ(ref inner) => stack.push(inner),
                | Ast::Max(ref left, ref right) => {
                    stack.push(right);
                    stack.push(left);
                },
            }
        }
    }

    /// Evaluate a free term directly under a valuation (absent variables
    /// read as zero). This is the semantic reference: it never consults
    /// canonical forms.
    fn eval_ast(
        term: &Ast,
        valuation: &BTreeMap<LevelVarIndex, LevelValue>,
    ) -> LevelValue
    {
        enum Frame<'term>
        {
            Visit(&'term Ast),
            ApplySucc,
            ApplyMax,
        }

        let mut stack = vec![Frame::Visit(term)];
        let mut values = Vec::new();
        while let Some(frame) = stack.pop() {
            match frame {
                | Frame::Visit(current) => match *current {
                    | Ast::Zero => values.push(0_u128),
                    | Ast::Var(index) => values.push(
                        valuation
                            .get(&LevelVarIndex::from(index))
                            .copied()
                            .map_or(0_u128, u128::from),
                    ),
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
                    let value = values.pop().unwrap();
                    values.push(value.checked_add(1_u128).unwrap());
                },
                | Frame::ApplyMax => {
                    let right = values.pop().unwrap();
                    let left = values.pop().unwrap();
                    values.push(left.max(right));
                },
            }
        }
        LevelValue::from(values.pop().unwrap())
    }

    /// Fold a free term into a canonical [`Level`] through the smart
    /// constructors (the system under test).
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

    /// Count the successor nodes of a term — a syntactic upper bound on
    /// every constant and offset of its canonical form.
    fn succ_count(term: &Ast) -> LevelValue
    {
        enum Frame<'term>
        {
            Visit(&'term Ast),
            ApplySucc,
            ApplyMax,
        }

        let mut stack = vec![Frame::Visit(term)];
        let mut counts = Vec::new();
        while let Some(frame) = stack.pop() {
            match frame {
                | Frame::Visit(current) => match *current {
                    | Ast::Zero | Ast::Var(_) => counts.push(0_u128),
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
                    let inner = counts.pop().unwrap();
                    counts.push(inner.checked_add(1_u128).unwrap());
                },
                | Frame::ApplyMax => {
                    let right = counts.pop().unwrap();
                    let left = counts.pop().unwrap();
                    counts.push(left.checked_add(right).unwrap());
                },
            }
        }
        LevelValue::from(counts.pop().unwrap())
    }

    proptest! {
        /// The oracle's non-strict order agrees with the semantic reference.
        #[test]
        fn prop_leq_agrees_with_semantic_reference((left, right) in (ast_strategy(), ast_strategy()))
        {
            let oracle = to_level(&left).leq(&to_level(&right));
            let reference = semantic_leq(&left, &right, Strictness::NON_STRICT);
            prop_assert_eq!(oracle, reference);
        }

        /// The oracle's strict order agrees with the semantic reference.
        #[test]
        fn prop_lt_agrees_with_semantic_reference((left, right) in (ast_strategy(), ast_strategy()))
        {
            let oracle = to_level(&left).lt(&to_level(&right));
            let reference = semantic_leq(&left, &right, Strictness::STRICT);
            prop_assert_eq!(oracle, reference);
        }

        /// Canonical equality agrees with two-sided semantic order — the
        /// canonical form is a complete invariant.
        #[test]
        fn prop_eq_agrees_with_semantic_reference((left, right) in (ast_strategy(), ast_strategy()))
        {
            let oracle = to_level(&left) == to_level(&right);
            let reference = bool::from(semantic_leq(&left, &right, Strictness::NON_STRICT))
                && bool::from(semantic_leq(&right, &left, Strictness::NON_STRICT));
            prop_assert_eq!(oracle, reference);
        }

        /// Every piece of evidence the oracle returns validates against its
        /// levels, in both strictness modes and both directions.
        #[test]
        fn prop_evidence_validates((left, right) in (ast_strategy(), ast_strategy()))
        {
            let left_level = to_level(&left);
            let right_level = to_level(&right);
            for strict in [Strictness::NON_STRICT, Strictness::STRICT] {
                let decision = if bool::from(strict) {
                    left_level.lt_with_evidence(&right_level)
                }
                else {
                    left_level.leq_with_evidence(&right_level)
                };
                match decision {
                    Ok(witness) => {
                        prop_assert_eq!(witness.strict(), strict);
                        prop_assert_eq!(validate_witness(&left_level, &right_level, &witness), Ok(()));
                    },
                    Err(refutation) => {
                        prop_assert_eq!(refutation.strict(), strict);
                        prop_assert_eq!(validate_refutation(&left_level, &right_level, &refutation), Ok(()));
                    },
                }
            }
        }

        /// The strict order is exactly the non-strict order after a
        /// successor on the left.
        #[test]
        fn prop_lt_equals_succ_leq((left, right) in (ast_strategy(), ast_strategy()))
        {
            let left_level = to_level(&left);
            let right_level = to_level(&right);
            let via_succ = left_level.succ().unwrap().leq(&right_level);
            prop_assert_eq!(left_level.lt(&right_level), via_succ);
        }

        /// `max` is commutative, associative, idempotent, and has `zero` as
        /// unit — as canonical-form identities.
        #[test]
        fn prop_max_laws((a, b, c) in (ast_strategy(), ast_strategy(), ast_strategy()))
        {
            let la = to_level(&a);
            let lb = to_level(&b);
            let lc = to_level(&c);
            prop_assert_eq!(la.max(&lb), lb.max(&la));
            prop_assert_eq!(la.max(&lb).max(&lc), la.max(&lb.max(&lc)));
            prop_assert_eq!(la.max(&Level::zero()), la.max(&la));
            prop_assert_eq!(la.max(&la), la);
        }

        /// The successor distributes over `max`.
        #[test]
        fn prop_succ_distributes_over_max((a, b) in (ast_strategy(), ast_strategy()))
        {
            let la = to_level(&a);
            let lb = to_level(&b);
            let joined_then_succ = la.max(&lb).succ().unwrap();
            let succ_then_joined = la.succ().unwrap().max(&lb.succ().unwrap());
            prop_assert_eq!(joined_then_succ, succ_then_joined);
        }

        /// Order laws: reflexivity of `leq`, irreflexivity of `lt`,
        /// antisymmetry against canonical equality, and the join as an
        /// upper bound.
        #[test]
        fn prop_order_laws((a, b) in (ast_strategy(), ast_strategy()))
        {
            let la = to_level(&a);
            let lb = to_level(&b);
            prop_assert!(bool::from(la.leq(&la)));
            prop_assert!(!bool::from(la.lt(&la)));
            prop_assert_eq!(bool::from(la.leq(&lb)) && bool::from(lb.leq(&la)), la == lb);
            prop_assert!(bool::from(la.leq(&la.max(&lb))));
            prop_assert!(bool::from(lb.leq(&la.max(&lb))));
        }

        /// Transitivity of the oracle's order on generated triples.
        #[test]
        fn prop_leq_is_transitive((a, b, c) in (ast_strategy(), ast_strategy(), ast_strategy()))
        {
            let la = to_level(&a);
            let lb = to_level(&b);
            let lc = to_level(&c);
            if bool::from(la.leq(&lb)) && bool::from(lb.leq(&lc)) {
                prop_assert!(bool::from(la.leq(&lc)));
            }
        }

        /// The in-crate evaluator agrees with the reference evaluator under
        /// random small valuations — the semantic anchor is anchored.
        #[test]
        fn prop_eval_agrees_with_reference(
            term in ast_strategy(),
            values in proptest::collection::vec(0_u128..10_u128, 4)
        )
        {
            let mut reference_valuation = BTreeMap::new();
            let mut level_valuation = BTreeMap::new();
            for (index, &value) in values.iter().enumerate() {
                let index = u32::try_from(index).unwrap();
                let wrapped_index = LevelVarIndex::from(index);
                let wrapped_value = LevelValue::from(value);
                reference_valuation.insert(wrapped_index, wrapped_value);
                level_valuation.insert(LevelVar::new(wrapped_index), wrapped_value);
            }
            let reference = eval_ast(&term, &reference_valuation);
            let oracle = to_level(&term).eval(&level_valuation).unwrap();
            prop_assert_eq!(oracle, reference);
        }
    }
}
