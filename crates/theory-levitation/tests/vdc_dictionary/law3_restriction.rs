//! **Law 3 — restriction (the split form; the heart of the suite)** (proposal
//! §3 item 3).
//!
//! * (a) real-structure face-level splitness on the landed
//!   [`gandr_theory_levitation::CellFace`] type;
//! * (b) the four Def 3.2.6 protype-level equalities, structural, holding by
//!   the split representation (Lemma 3.2.8's tuple construction realized);
//! * (c) fibrationality — a framed cell factors data-identically through its
//!   restricted globular form;
//! * (d) real-structure integration — variance invariance and well-formedness
//!   preservation, exercising the real [`derive_cell_var_meta`] and
//!   [`gandr_theory_levitation::check_desc`].

#[cfg(test)]
mod law3
{
    use alloc::rc::Rc;

    use gandr_theory_levitation::CellFace;
    use gandr_theory_levitation::FreeTerm;
    use gandr_theory_levitation::NameRef;
    use gandr_theory_levitation::check_desc;
    use gandr_theory_levitation::wellformed::WfKind;
    use proptest::prelude::*;

    use crate::fixtures::face;
    use crate::fixtures::nat;
    use crate::fixtures::nat_from_names;
    use crate::fixtures::nat_names;
    use crate::fixtures::nat_obj;
    use crate::fixtures::nat_sig;
    use crate::fixtures::real_nat_names;
    use crate::fixtures::renaming;
    use crate::fixtures::sample_faces;
    use crate::fixtures::succ;
    use crate::fixtures::unary_relation;
    use crate::fixtures::var;
    use crate::fixtures::zero;
    use crate::harness::BaseInstance;
    use crate::harness::BaseLoose;
    use crate::harness::Cell;
    use crate::harness::CellClause;
    use crate::harness::CellKind;
    use crate::harness::FormalRestriction;
    use crate::harness::LooseArrow;
    use crate::harness::Relation;
    use crate::harness::SigMorphism;
    use crate::harness::apply_face;
    use crate::harness::compose;
    use crate::harness::factor_globular;
    use crate::harness::left_endpoint;
    use crate::harness::match_pattern;
    use crate::harness::restrict;
    use crate::harness::right_endpoint;
    use crate::harness::subst_term;

    /// A tag strategy over a small alphabet.
    fn tag() -> impl Strategy<Value = &'static str>
    {
        proptest::sample::select(vec!["a", "b", "c", "d", "e", "f"])
    }

    // ---- (a) real-structure face-level splitness ----

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(96))]

        /// `apply_face(id, face) = face` and `apply_face(s∘s′, face) =
        /// apply_face(s′, apply_face(s, face))` — structural, on the real face
        /// type. (Note the contravariance: `s` acts innermost.)
        #[test]
        fn face_action_is_split_on_the_real_cellface(
            ta in tag(), tb in tag(), tc in tag(), idx in 0_usize .. 4
        ) {
            let (a, b, c) = (nat_names(ta.into()), nat_names(tb.into()), nat_names(tc.into()));
            let face_c: CellFace = sample_faces(&c)[idx].clone();

            let identity = SigMorphism::identity(&nat_obj(&c));
            prop_assert!(apply_face(&identity, 0.into(), &face_c) == face_c, "id acts trivially");

            let s = renaming(&b, &c);       // B → C  (acts on C-faces first)
            let s_prime = renaming(&a, &b); // A → B
            let composed = apply_face(&compose(&s_prime, &s), 0.into(), &face_c);
            let staged = apply_face(&s_prime, 0.into(), &apply_face(&s, 0.into(), &face_c));
            prop_assert_eq!(composed, staged, "the face action is split");
        }
    }

    // ---- (b) the four Def 3.2.6 protype-level equalities ----

    #[test]
    fn restriction_by_identities_is_the_identity()
    {
        let mid = nat_names("m".into());
        let alpha = LooseArrow::of_relation(relation_over("R".into(), &mid));
        let id = SigMorphism::identity(&nat_obj(&mid));
        assert_eq!(
            restrict(&alpha, &id, &id),
            alpha,
            "α[id # id] = α, strictly by construction"
        );
    }
    #[test]
    fn restriction_composes_by_construction()
    {
        let mid = nat_names("m".into());
        let alpha = LooseArrow::of_relation(relation_over("R".into(), &mid));
        let s = renaming(&nat_names("i".into()), &mid); // I → M
        let t = renaming(&nat_names("k".into()), &mid); // K → M
        let s_prime = renaming(&nat_names("i2".into()), &nat_names("i".into())); // I2 → I
        let t_prime = renaming(&nat_names("k2".into()), &nat_names("k".into())); // K2 → K

        let stepwise = restrict(&restrict(&alpha, &s, &t), &s_prime, &t_prime);
        let fused = restrict(&alpha, &compose(&s_prime, &s), &compose(&t_prime, &t));
        assert_eq!(
            stepwise, fused,
            "α[s # t][s′ # t′] = α[s∘s′ # t∘t′], strictly"
        );
    }
    #[test]
    fn restriction_distributes_over_meet()
    {
        let mid = nat_names("m".into());
        let alpha = LooseArrow::of_relation(relation_over("R".into(), &mid));
        let beta = LooseArrow::of_relation(relation_over("S".into(), &mid));
        let meet = LooseArrow::meet(&alpha, &beta);
        let s = renaming(&nat_names("i".into()), &mid);
        let t = renaming(&nat_names("k".into()), &mid);

        let restricted_meet = restrict(&meet, &s, &t);
        let meet_of_restricted =
            LooseArrow::meet(&restrict(&alpha, &s, &t), &restrict(&beta, &s, &t));
        assert_eq!(
            restricted_meet, meet_of_restricted,
            "(α ∧ β)[s # t] = α[s # t] ∧ β[s # t]"
        );
    }
    #[test]
    fn a_framed_cell_factors_data_identically_through_its_globular_form()
    {
        let mid = nat_names("m".into());
        let rel = relation_over("R".into(), &mid);
        let cod = LooseArrow::of_relation(rel);
        // Genuine (non-identity) frames the factorization must absorb.
        let left_frame = renaming(&nat_names("i".into()), &mid); // I → M
        let right_frame = renaming(&nat_names("k".into()), &mid); // K → M
        let clause = CellClause {
            matches: vec![0.into()],
            emit: vec![(0.into(), {
                let mut templates = alloc::collections::BTreeMap::new();
                templates.insert("x".into(), var("p0.x".into()));
                templates
            })],
        };
        let cell = Cell {
            dom: vec![cod.clone()],
            cod: cod.clone(),
            left_frame: left_frame.clone(),
            right_frame: right_frame.clone(),
            kind: CellKind::Clauses(vec![clause]),
        };

        let globular = factor_globular(&cell);
        // Uniqueness: the clause data is identical after factorization.
        assert_eq!(
            globular.kind, cell.kind,
            "the factorization is data-identical on clauses"
        );
        // The frames are pushed into the codomain's formal restriction.
        assert_eq!(
            globular.cod,
            restrict(&cod, &left_frame, &right_frame),
            "the frames land in the codomain's restriction"
        );
        assert_eq!(
            globular.left_frame,
            SigMorphism::identity(&globular.cod.src),
            "the factored cell is globular (identity left frame)"
        );
    }
    /// A named relation over `nat_obj(mid)` with one in-signature generating
    /// face.
    fn relation_over(
        name: NameRef<'_>,
        mid: &crate::fixtures::NatNames,
    ) -> Rc<Relation>
    {
        let zero = FreeTerm::ctor(mid.zero.clone(), Vec::new());
        let generating = face(
            FreeTerm::op(mid.plus.clone(), [var("x".into()), zero]),
            var("x".into()),
        );
        Rc::new(Relation {
            name: name.into(),
            src: nat_obj(mid),
            tgt: nat_obj(mid),
            gens: vec![generating],
        })
    }

    #[test]
    fn terminal_loose_arrow_is_restriction_stable()
    {
        let mid = nat_names("m".into());
        let top = LooseArrow::top(&nat_obj(&mid), &nat_obj(&mid));
        let s = renaming(&nat_names("i".into()), &mid);
        let t = renaming(&nat_names("k".into()), &mid);
        assert_eq!(
            restrict(&top, &s, &t),
            LooseArrow::top(&s.src, &t.src),
            "⊤[s # t] = ⊤, strictly"
        );
    }

    // ---- (c) fibrationality: factor through the restricted globular cell ----

    // ---- (d) real-structure integration ----

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(96))]

        /// `derive_cell_var_meta` is invariant under `apply_face`: renamings
        /// touch symbols, never pattern variables.
        #[test]
        fn variance_metadata_is_invariant_under_face_action(
            tb in tag(), tc in tag(), idx in 0_usize .. 4
        ) {
            let (b, c) = (nat_names(tb.into()), nat_names(tc.into()));
            let face_c: CellFace = sample_faces(&c)[idx].clone();
            let s = renaming(&b, &c); // B → C
            let mapped = apply_face(&s, 0.into(), &face_c);
            prop_assert_eq!(
                mapped.vars, face_c.vars,
                "the derived per-variable metadata is unchanged by renaming"
            );
        }
    }

    #[test]
    fn instance_boundaries_are_computed_from_the_real_face_endpoints()
    {
        // A formal restriction over a named relation (`plus(x, Zero) ~> x`),
        // framed by identities; boundaries are computed on read, never stored.
        let rel = unary_relation("R".into());
        assert_eq!(
            BaseLoose::Named(Rc::clone(&rel)).src(),
            nat_sig(),
            "the base source is Nat"
        );
        assert_eq!(
            BaseLoose::Named(Rc::clone(&rel)).tgt(),
            nat_sig(),
            "the base target is Nat"
        );
        let identity = SigMorphism::identity(&nat_sig());
        let factor = FormalRestriction {
            left: identity.clone(),
            base: BaseLoose::Named(rel),
            right: identity,
        };
        let mut subst = alloc::collections::BTreeMap::new();
        subst.insert("x".into(), nat(2.into()));
        let instance = BaseInstance::Gen {
            generator: 0.into(),
            subst,
        };
        assert_eq!(
            left_endpoint(&factor, &instance),
            Some(FreeTerm::op("plus", [nat(2.into()), zero()])),
            "the left boundary is the substituted lhs"
        );
        assert_eq!(
            right_endpoint(&factor, &instance),
            Some(nat(2.into())),
            "the right boundary is the substituted rhs"
        );
    }

    #[test]
    fn morphism_valid_faces_stay_well_formed_after_translation()
    {
        // A clean rule over D: plus(Zero, n) ~> n.
        let d = nat_names("d".into());
        let e = nat_names("e".into());
        let zero_d = FreeTerm::ctor(d.zero.clone(), Vec::new());
        let clean = face(
            FreeTerm::op(d.plus.clone(), [zero_d, var("n".into())]),
            var("n".into()),
        );
        let target = nat_from_names(&d, vec![clean.clone()]);
        assert!(check_desc(&target).is_empty(), "the target rule is clean");

        // f : E → D, a valid renaming; translate the face to E.
        let f = renaming(&e, &d);
        let mapped = apply_face(&f, 0.into(), &clean);
        let source = nat_from_names(&e, vec![mapped]);
        assert!(
            check_desc(&source).is_empty(),
            "the translated face is clean in E (well-formedness is preserved)"
        );
    }

    // ---- located obligations H1-H3: verdict-note witnesses ----

    #[test]
    fn matching_and_substitution_are_supplied_test_side()
    {
        // H1: gandr-theory-levitation ships no production matching/substitution on
        // FreeTerm; the harness supplies it. An F2/L2 obligation.
        let pattern = FreeTerm::op("plus", [succ(var("m".into())), var("n".into())]);
        let ground = FreeTerm::op("plus", [succ(zero()), nat(2.into())]);
        let mut binding = alloc::collections::BTreeMap::new();
        assert!(
            bool::from(match_pattern(&pattern, &ground, &mut binding)),
            "the test-side matcher binds"
        );
        assert_eq!(
            subst_term(&pattern, &binding),
            ground,
            "the test-side substitution reconstructs the ground term"
        );
    }

    #[test]
    fn check_desc_is_single_signature_homogeneous()
    {
        // H2: a face whose rhs names another signature's constructor (`True`)
        // is rejected — check_desc is single-signature, so heterogeneous loose
        // arrows (I ≠ J) have no well-formedness support.
        let bad = face(
            FreeTerm::op("plus", [
                var("x".into()),
                FreeTerm::ctor("Zero", Vec::new()),
            ]),
            FreeTerm::ctor("True", Vec::new()),
        );
        let desc = nat_from_names(&real_nat_names(), vec![bad]);
        assert!(
            check_desc(&desc)
                .iter()
                .any(|diagnostic| diagnostic.kind == WfKind::OutOfSignatureCell),
            "a cross-signature symbol is rejected: well-formedness is homogeneous"
        );
    }

    #[test]
    fn unbound_rhs_rule_is_a_rewrite_discipline_not_a_relation_condition()
    {
        // H2: `R(x # y)` is a legitimate two-sided relation interface, but the
        // UnboundRhsVariable rule (rhs vars ⊆ lhs vars) — a rewrite discipline —
        // rejects the fresh target-side variable `y`.
        let two_sided = face(
            FreeTerm::op("plus", [
                var("x".into()),
                FreeTerm::ctor("Zero", Vec::new()),
            ]),
            var("y".into()),
        );
        let desc = nat_from_names(&real_nat_names(), vec![two_sided]);
        assert!(
            check_desc(&desc)
                .iter()
                .any(|diagnostic| diagnostic.kind == WfKind::UnboundRhsVariable),
            "rhs-vars ⊆ lhs-vars is a rewrite discipline, not the two-sided-interface condition"
        );
    }

    // H3 (no production signature-morphism type) is witnessed by Law 1's
    // `check_morphism` tests: the SigMorphism type and its checker live
    // entirely in the harness.
}
