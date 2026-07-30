//! **Law 1 — the tight category** (proposal §3 item 1; strict, expected
//! trivial).
//!
//! Signature-morphism composition is strictly associative and unital, its term
//! action is (contravariantly) functorial, and the chosen product satisfies the
//! β-laws and terminal uniqueness on the nose. All equalities are
//! **structural** on the landed [`gandr_theory_levitation`] structures.

#[cfg(test)]
mod law1
{
    use alloc::collections::BTreeMap;

    use gandr_theory_levitation::FreeTerm;
    use gandr_theory_levitation::Name;
    use proptest::prelude::*;

    use super::fixtures::nat_names;
    use super::fixtures::nat_obj;
    use super::fixtures::renaming;
    use super::fixtures::sample_faces;
    use super::harness::FactorRoute;
    use super::harness::SigMorphism;
    use super::harness::apply_term;
    use super::harness::check_morphism;
    use super::harness::compose;

    /// A tag strategy over a small alphabet (distinct-name generation is
    /// role-prefixed in `nat_names`, so tags may repeat harmlessly).
    fn tag() -> impl Strategy<Value = &'static str>
    {
        proptest::sample::select(vec!["a", "b", "c", "d", "e", "f"])
    }

    #[test]
    fn check_morphism_accepts_a_role_matched_renaming()
    {
        let source = nat_names("a".into());
        let target = nat_names("b".into());
        let morphism = renaming(&source, &target);
        assert!(
            check_morphism(&morphism).is_empty(),
            "a role-matched renaming is a valid signature morphism"
        );
    }

    #[test]
    fn check_morphism_rejects_code_and_arity_violations()
    {
        // Map `Zero`-role (code `1`) to `Succ`-role (code `var`) and the binary
        // op to the unary op: both violate payload identity.
        let source = nat_names("a".into());
        let target = nat_names("b".into());
        let mut map: BTreeMap<Name, Name> = BTreeMap::new();
        map.insert(target.zero.clone(), source.succ.clone());
        map.insert(target.succ.clone(), source.succ.clone());
        map.insert(target.plus.clone(), source.double.clone());
        map.insert(target.double.clone(), source.double.clone());
        let bad = SigMorphism {
            src: nat_obj(&source),
            tgt: nat_obj(&target),
            routes: vec![FactorRoute {
                src_factor: 0.into(),
                map,
            }],
        };
        let errors = check_morphism(&bad);
        assert!(
            errors.len() >= 2,
            "the code violation and the arity violation are both reported: {errors:?}"
        );
    }

    #[test]
    fn the_diagonal_projects_back_to_the_identity()
    {
        let a = nat_obj(&nat_names("a".into()));
        let diagonal = SigMorphism::diagonal(&a);
        assert!(
            check_morphism(&diagonal).is_empty(),
            "the diagonal is a valid morphism"
        );
        let proj0 = SigMorphism::projection(&diagonal.tgt, 0.into());
        let proj1 = SigMorphism::projection(&diagonal.tgt, 1.into());
        assert_eq!(
            compose(&diagonal, &proj0),
            SigMorphism::identity(&a),
            "π₀ ∘ Δ = id"
        );
        assert_eq!(
            compose(&diagonal, &proj1),
            SigMorphism::identity(&a),
            "π₁ ∘ Δ = id"
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(96))]

        /// Composition is strictly associative as structural equality.
        #[test]
        fn composition_is_strictly_associative(
            ta in tag(), tb in tag(), tc in tag(), td in tag()
        ) {
            let (a, b, c, d) = (nat_names(ta.into()), nat_names(tb.into()), nat_names(tc.into()), nat_names(td.into()));
            let f_ab = renaming(&a, &b);
            let f_bc = renaming(&b, &c);
            let f_cd = renaming(&c, &d);
            let left = compose(&compose(&f_ab, &f_bc), &f_cd);
            let right = compose(&f_ab, &compose(&f_bc, &f_cd));
            prop_assert_eq!(left, right, "(h∘g)∘f = h∘(g∘f)");
        }

        /// Identity is a strict two-sided unit for composition.
        #[test]
        fn identity_is_a_strict_unit(ta in tag(), tb in tag()) {
            let (a, b) = (nat_names(ta.into()), nat_names(tb.into()));
            let f_ab = renaming(&a, &b);
            let id_a = SigMorphism::identity(&nat_obj(&a));
            let id_b = SigMorphism::identity(&nat_obj(&b));
            prop_assert!(compose(&id_a, &f_ab) == f_ab, "id ∘ f = f");
            prop_assert!(compose(&f_ab, &id_b) == f_ab, "f ∘ id = f");
        }

        /// The term action is contravariantly functorial and identity-preserving.
        #[test]
        fn term_action_is_contravariantly_functorial(
            ta in tag(), tb in tag(), tc in tag(), idx in 0_usize .. 4
        ) {
            let (a, b, c) = (nat_names(ta.into()), nat_names(tb.into()), nat_names(tc.into()));
            let outer = renaming(&a, &b); // f : A → B
            let inner = renaming(&b, &c); // g : B → C
            let term: FreeTerm = sample_faces(&c)[idx].lhs.clone();
            // apply_term(g∘f, t) = apply_term(f, apply_term(g, t))  (note variance)
            let composed = apply_term(&compose(&outer, &inner), 0.into(), &term);
            let staged = apply_term(&outer, 0.into(), &apply_term(&inner, 0.into(), &term));
            prop_assert_eq!(composed, staged, "the action is contravariantly functorial");

            let identity = SigMorphism::identity(&nat_obj(&c));
            prop_assert_eq!(apply_term(&identity, 0.into(), &term), term, "id acts trivially");
        }

        /// The chosen product satisfies the projection β-laws and terminal
        /// uniqueness strictly.
        #[test]
        fn chosen_product_beta_and_terminal_hold_strictly(
            ta in tag(), tb in tag(), tc in tag()
        ) {
            let (a, b, c) = (nat_names(ta.into()), nat_names(tb.into()), nat_names(tc.into()));
            let f = renaming(&a, &b); // f : A → B
            let g = renaming(&a, &c); // g : A → C, shared source A
            let paired = SigMorphism::pairing(&f, &g); // ⟨f, g⟩ : A → B × C
            let proj0 = SigMorphism::projection(&paired.tgt, 0.into());
            let proj1 = SigMorphism::projection(&paired.tgt, 1.into());
            prop_assert_eq!(compose(&paired, &proj0), f, "π₀ ∘ ⟨f, g⟩ = f");
            prop_assert_eq!(compose(&paired, &proj1), g, "π₁ ∘ ⟨f, g⟩ = g");

            // Terminal uniqueness: `!` has no routes, and `! ∘ f = !`.
            let bang_a = SigMorphism::terminal(&nat_obj(&a));
            prop_assert!(bang_a.routes.is_empty(), "the terminal arrow has no routes");
            let f2 = renaming(&a, &b);
            let bang_b = SigMorphism::terminal(&nat_obj(&b));
            prop_assert_eq!(compose(&f2, &bang_b), bang_a, "! ∘ f = ! (uniqueness)");
        }
    }
}
