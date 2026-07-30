//! **U3.0a — `CodeIso` certificates and the invertible-mode groupoid** (design
//! note §4.3; ADR-69).
//!
//! A [`CodeIso`](crate::code_iso::harness::CodeIso) is a paired value translator whose
//! evidence is the replay of its round trips against `generic_eq` over
//! generated values. This module checks the certificate discipline and the
//! groupoid structure — identity, inverse, and invertible-mode composition —
//! all **up to replay-equivalence** (ADR-69 D1), the only identity a stage-0
//! certificate has.

#[cfg(test)]
mod u30a
{
    use gandr_theory_levitation::check_desc;
    use gandr_theory_levitation::generic_eq;
    use proptest::prelude::*;

    use super::fixtures;
    use super::harness::CodeIso;
    use super::harness::replay_equivalent;

    // ---- certificate hygiene: monomorphic, well-formed, round-tripping ----

    #[test]
    fn every_description_is_monomorphic_and_well_formed()
    {
        for desc in [
            fixtures::bool_two_ctor(),
            fixtures::bool_sum(),
            fixtures::rgb(),
        ] {
            assert!(
                desc.params.is_empty(),
                "the U3.0 descriptions are monomorphic (no parameters): {}",
                desc.id.name
            );
            assert!(
                check_desc(&desc).is_empty(),
                "the U3.0 description is well-formed: {}",
                desc.id.name
            );
        }
        for iso in [
            fixtures::identity_bool(),
            fixtures::negation_bool(),
            fixtures::bool_bridge(),
            fixtures::rgb_rotate(),
        ] {
            assert!(
                bool::from(iso.is_monomorphic()),
                "the certificate boundary is monomorphic: {}",
                iso.label()
            );
        }
    }

    #[test]
    fn every_named_iso_holds_its_round_trips_exhaustively()
    {
        // The full (finite) value space of each boundary is replayed; a valid
        // certificate holds `back ∘ fwd ≡ id` and `fwd ∘ back ≡ id`.
        let cases = [
            (
                fixtures::identity_bool(),
                fixtures::bool_two_ctor_values(),
                fixtures::bool_two_ctor_values(),
            ),
            (
                fixtures::negation_bool(),
                fixtures::bool_two_ctor_values(),
                fixtures::bool_two_ctor_values(),
            ),
            (
                fixtures::bool_bridge(),
                fixtures::bool_two_ctor_values(),
                fixtures::bool_sum_values(),
            ),
            (
                fixtures::rgb_rotate(),
                fixtures::rgb_values(),
                fixtures::rgb_values(),
            ),
        ];
        for (iso, source_samples, target_samples) in cases {
            let report = iso.round_trips(&source_samples, &target_samples);
            assert!(
                bool::from(report.holds()),
                "the certificate `{}` round-trips on every value: {:?}",
                iso.label(),
                report.failures
            );
            assert_eq!(
                report.forward_checked,
                source_samples.len().into(),
                "every source sample was replayed"
            );
            assert_eq!(
                report.backward_checked,
                target_samples.len().into(),
                "every target sample was replayed"
            );
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Negation round-trips (`back ∘ fwd ≡ id`) on every generated Boolean.
        #[test]
        fn negation_round_trips_on_generated_values(
            value in proptest::sample::select(fixtures::bool_two_ctor_values())
        ) {
            let iso = fixtures::negation_bool();
            let round = iso.backward_value(&iso.forward_value(&value));
            prop_assert!(
                bool::from(generic_eq(&fixtures::bool_two_ctor(), &round, &value)),
                "negation back∘fwd recovers the value up to generic_eq"
            );
        }

        /// The cross-code bridge round-trips forward from every generated
        /// Boolean (`back ∘ fwd ≡ id`).
        #[test]
        fn bridge_round_trips_forward_on_generated_values(
            value in proptest::sample::select(fixtures::bool_two_ctor_values())
        ) {
            let iso = fixtures::bool_bridge();
            let round = iso.backward_value(&iso.forward_value(&value));
            prop_assert!(
                bool::from(generic_eq(&fixtures::bool_two_ctor(), &round, &value)),
                "bridge back∘fwd recovers the Boolean up to generic_eq"
            );
        }

        /// The cross-code bridge round-trips backward from every generated
        /// `BoolSum` value (`fwd ∘ back ≡ id`).
        #[test]
        fn bridge_round_trips_backward_on_generated_values(
            value in proptest::sample::select(fixtures::bool_sum_values())
        ) {
            let iso = fixtures::bool_bridge();
            let round = iso.forward_value(&iso.backward_value(&value));
            prop_assert!(
                bool::from(generic_eq(&fixtures::bool_sum(), &round, &value)),
                "bridge fwd∘back recovers the BoolSum value up to generic_eq"
            );
        }

        /// Rotation round-trips (`back ∘ fwd ≡ id`) on every generated RGB.
        #[test]
        fn rotate_round_trips_on_generated_values(
            value in proptest::sample::select(fixtures::rgb_values())
        ) {
            let iso = fixtures::rgb_rotate();
            let round = iso.backward_value(&iso.forward_value(&value));
            prop_assert!(
                bool::from(generic_eq(&fixtures::rgb(), &round, &value)),
                "rotate back∘fwd recovers the RGB value up to generic_eq"
            );
        }
    }

    // ---- groupoid structure, up to replay-equivalence (ADR-69 D1) ----

    #[test]
    fn inverse_swaps_the_boundary_and_is_involutive()
    {
        let bridge = fixtures::bool_bridge();
        let inverse = bridge.inverse();
        assert_eq!(
            inverse.source().id.name,
            bridge.target().id.name,
            "the inverse's source is the original's target"
        );
        assert_eq!(
            inverse.target().id.name,
            bridge.source().id.name,
            "the inverse's target is the original's source"
        );
        // inverse ∘ inverse ≡ id (as certificates, up to replay).
        assert!(
            bool::from(replay_equivalent(
                &bridge.inverse().inverse(),
                &bridge,
                &fixtures::bool_two_ctor_values(),
                &fixtures::bool_sum_values(),
            )),
            "double inverse is replay-equivalent to the original"
        );
    }

    #[test]
    fn inverse_undoes_composition_up_to_replay()
    {
        // iso ⨟ iso⁻¹ ≡ id(source) and iso⁻¹ ⨟ iso ≡ id(target).
        let bridge = fixtures::bool_bridge();
        let forward_then_back = bridge
            .compose_invertible(&bridge.inverse())
            .expect("iso ⨟ iso⁻¹ shares the middle boundary");
        assert!(
            bool::from(replay_equivalent(
                &forward_then_back,
                &fixtures::identity_bool(),
                &fixtures::bool_two_ctor_values(),
                &fixtures::bool_two_ctor_values(),
            )),
            "iso ⨟ iso⁻¹ is replay-equivalent to the identity on the source"
        );
        let back_then_forward = bridge
            .inverse()
            .compose_invertible(&bridge)
            .expect("iso⁻¹ ⨟ iso shares the middle boundary");
        assert!(
            bool::from(replay_equivalent(
                &back_then_forward,
                &CodeIso::identity("id[BoolSum]", fixtures::bool_sum()),
                &fixtures::bool_sum_values(),
                &fixtures::bool_sum_values(),
            )),
            "iso⁻¹ ⨟ iso is replay-equivalent to the identity on the target"
        );
    }

    #[test]
    fn negation_is_self_inverse_up_to_replay()
    {
        let negation = fixtures::negation_bool();
        let twice = negation
            .compose_invertible(&negation)
            .expect("negation ⨟ negation shares the Boolean boundary");
        assert!(
            bool::from(replay_equivalent(
                &twice,
                &fixtures::identity_bool(),
                &fixtures::bool_two_ctor_values(),
                &fixtures::bool_two_ctor_values(),
            )),
            "negation ⨟ negation is replay-equivalent to the identity"
        );
    }

    #[test]
    fn rotation_has_order_three_up_to_replay()
    {
        let rotate = fixtures::rgb_rotate();
        let twice = rotate
            .compose_invertible(&rotate)
            .expect("rotate ⨟ rotate shares the RGB boundary");
        let thrice = twice
            .compose_invertible(&rotate)
            .expect("rotate ⨟ rotate ⨟ rotate shares the RGB boundary");
        assert!(
            bool::from(replay_equivalent(
                &thrice,
                &CodeIso::identity("id[RGB]", fixtures::rgb()),
                &fixtures::rgb_values(),
                &fixtures::rgb_values(),
            )),
            "rotate cubed is replay-equivalent to the identity"
        );
    }

    #[test]
    fn composition_is_associative_up_to_replay()
    {
        let rotate = fixtures::rgb_rotate();
        // (rotate ⨟ rotate) ⨟ rotate
        let left = rotate
            .compose_invertible(&rotate)
            .and_then(|inner| inner.compose_invertible(&rotate))
            .expect("(rotate ⨟ rotate) ⨟ rotate is defined");
        // rotate ⨟ (rotate ⨟ rotate)
        let right = rotate
            .compose_invertible(&rotate)
            .and_then(|inner| rotate.compose_invertible(&inner))
            .expect("rotate ⨟ (rotate ⨟ rotate) is defined");
        assert!(
            bool::from(replay_equivalent(
                &left,
                &right,
                &fixtures::rgb_values(),
                &fixtures::rgb_values(),
            )),
            "invertible-mode composition is associative up to replay"
        );
    }

    #[test]
    fn identity_is_a_two_sided_unit_up_to_replay()
    {
        let bridge = fixtures::bool_bridge();
        let left_unit = fixtures::identity_bool()
            .compose_invertible(&bridge)
            .expect("id(source) ⨟ iso is defined");
        let right_unit = bridge
            .compose_invertible(&CodeIso::identity("id[BoolSum]", fixtures::bool_sum()))
            .expect("iso ⨟ id(target) is defined");
        for unit in [&left_unit, &right_unit] {
            assert!(
                bool::from(replay_equivalent(
                    unit,
                    &bridge,
                    &fixtures::bool_two_ctor_values(),
                    &fixtures::bool_sum_values(),
                )),
                "composing with the identity is replay-equivalent to the iso alone"
            );
        }
    }

    #[test]
    fn invertible_composition_declines_only_on_a_boundary_mismatch()
    {
        // A matching boundary always composes — invertible mode is
        // unconditional (ADR-69 D3: no acyclicity gate in this mode).
        let bridge = fixtures::bool_bridge();
        assert!(
            bridge
                .compose_invertible(&CodeIso::identity("id[BoolSum]", fixtures::bool_sum()))
                .is_some(),
            "a shared boundary composes unconditionally"
        );
        // A mismatched boundary is the only decline: BoolSum ≠ Boolean.
        assert!(
            bridge
                .compose_invertible(&fixtures::negation_bool())
                .is_none(),
            "a boundary-object mismatch is the sole decline (a domain error, not a gate)"
        );
    }
}
