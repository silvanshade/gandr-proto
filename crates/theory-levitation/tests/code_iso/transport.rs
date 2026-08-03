//! **U3.0b — transport of the generic consumers across a `CodeIso`** (design
//! note §4.3; proposal §6.2, "transport computes by certificate replay").
//!
//! For a certificate `iso : A → B`, the landed stage-0 generic programs behave
//! compatibly across the translation, with **zero new machinery**:
//!
//! * **`generic_eq` agreement** — equality on `A`-values agrees with equality
//!   on their forward images (`iso` is injective, so it neither merges nor
//!   splits equivalence classes);
//! * **`serialize_value` naturality up to re-encoding** — the iso does not
//!   preserve the *bytes* (the codes differ), but it preserves the
//!   *byte-equality relation*, and re-encoding after a round trip recovers the
//!   original bytes;
//! * **interner coherence** — interning is content-addressed by decidable code
//!   equality, so it commutes with an auto-iso trivially (translation acts on
//!   values, not codes) and, for a cross-code iso, exposes the genuine code
//!   difference the iso bridges at the value level (the observable the
//!   `CodeInterner` API affords).

#[cfg(test)]
mod u30b
{
    use gandr_theory_levitation::CodeId;
    use gandr_theory_levitation::CodeInterner;
    use gandr_theory_levitation::DescValue;
    use gandr_theory_levitation::InternedCodeCount;
    use gandr_theory_levitation::SignDesc;
    use gandr_theory_levitation::generic_eq;
    use gandr_theory_levitation::serialize_value;
    use proptest::prelude::*;

    use crate::code_iso::fixtures;
    use crate::code_iso::harness::CodeIso;

    #[test]
    fn generic_eq_agrees_across_every_iso()
    {
        for (iso, source, target, values) in transportable() {
            for left in &values {
                for right in &values {
                    let source_eq = generic_eq(&source, left, right);
                    let target_eq =
                        generic_eq(&target, &iso.forward_value(left), &iso.forward_value(right));
                    assert_eq!(
                        source_eq,
                        target_eq,
                        "eq on A-values agrees with eq on their images under `{}`",
                        iso.label()
                    );
                }
            }
        }
    }
    #[test]
    fn serialize_value_is_natural_up_to_re_encoding()
    {
        for (iso, source, target, values) in transportable() {
            // The byte-equality relation is preserved (bytes themselves are
            // not, across distinct codes).
            for left in &values {
                for right in &values {
                    let source_same =
                        serialize_value(&source, left) == serialize_value(&source, right);
                    let target_same = serialize_value(&target, &iso.forward_value(left))
                        == serialize_value(&target, &iso.forward_value(right));
                    assert_eq!(
                        source_same,
                        target_same,
                        "byte-equality is preserved across `{}` (naturality up to re-encoding)",
                        iso.label()
                    );
                }
            }
            // Re-encoding after a round trip recovers the original bytes.
            for value in &values {
                let round = iso.backward_value(&iso.forward_value(value));
                assert_eq!(
                    serialize_value(&source, &round),
                    serialize_value(&source, value),
                    "encode ∘ back ∘ fwd = encode under `{}`",
                    iso.label()
                );
            }
        }
    }
    /// The four named certificates with their boundary descriptions and the
    /// finite source-value space to transport.
    fn transportable() -> Vec<(CodeIso, SignDesc, SignDesc, Vec<DescValue>)>
    {
        vec![
            (
                fixtures::identity_bool(),
                fixtures::bool_two_ctor(),
                fixtures::bool_two_ctor(),
                fixtures::bool_two_ctor_values(),
            ),
            (
                fixtures::negation_bool(),
                fixtures::bool_two_ctor(),
                fixtures::bool_two_ctor(),
                fixtures::bool_two_ctor_values(),
            ),
            (
                fixtures::bool_bridge(),
                fixtures::bool_two_ctor(),
                fixtures::bool_sum(),
                fixtures::bool_two_ctor_values(),
            ),
            (
                fixtures::rgb_rotate(),
                fixtures::rgb(),
                fixtures::rgb(),
                fixtures::rgb_values(),
            ),
        ]
    }

    #[test]
    fn auto_iso_leaves_code_interning_fixed()
    {
        // An auto-iso (source == target) acts on values, never on codes, so
        // interning its source and target constructor codes coincides —
        // interning commutes with the translation trivially.
        for iso in [
            fixtures::identity_bool(),
            fixtures::negation_bool(),
            fixtures::rgb_rotate(),
        ] {
            let mut interner = CodeInterner::new();
            let source_ids = intern_ctor_codes(&mut interner, iso.source());
            let target_ids = intern_ctor_codes(&mut interner, iso.target());
            assert_eq!(
                source_ids,
                target_ids,
                "the auto-iso `{}` leaves code interning fixed",
                iso.label()
            );
        }
    }
    #[test]
    fn cross_code_iso_interns_to_distinct_codes()
    {
        // The flagship bridge relates *structurally distinct* codes; interning
        // distinguishes them, so the identity it carries is emphatically NOT
        // interned code equality (the design-note §4.1 point, and the value-side
        // of the U3.0c guard).
        let bridge = fixtures::bool_bridge();
        let mut interner = CodeInterner::new();
        let source_ids = intern_ctor_codes(&mut interner, bridge.source());
        let target_ids = intern_ctor_codes(&mut interner, bridge.target());
        assert_ne!(
            source_ids, target_ids,
            "the bridge's boundary codes intern distinctly (the codes genuinely differ)"
        );
        assert_eq!(
            InternedCodeCount::from(2),
            interner.len(),
            "content-addressing dedups the two Unit codes and mints one fresh id for the inline sum"
        );
    }
    /// The interned [`CodeId`]s of a description's constructor codes, in
    /// declaration order, against a shared interner.
    fn intern_ctor_codes(
        interner: &mut CodeInterner,
        desc: &SignDesc,
    ) -> Vec<CodeId>
    {
        desc.ctors
            .iter()
            .map(|ctor| interner.intern(ctor.code.clone()))
            .collect()
    }

    // ---- generic_eq agreement ----

    // ---- serialize_value naturality (up to re-encoding) ----

    // ---- interner coherence ----

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// `generic_eq` agreement across the flagship bridge, over generated
        /// pairs (property-test rendering of the exhaustive check).
        #[test]
        fn bridge_generic_eq_agreement_on_generated_pairs(
            left in proptest::sample::select(fixtures::bool_two_ctor_values()),
            right in proptest::sample::select(fixtures::bool_two_ctor_values()),
        ) {
            let iso = fixtures::bool_bridge();
            prop_assert_eq!(
                generic_eq(&fixtures::bool_two_ctor(), &left, &right),
                generic_eq(
                    &fixtures::bool_sum(),
                    &iso.forward_value(&left),
                    &iso.forward_value(&right),
                ),
                "eq agrees across the bridge on generated pairs"
            );
        }

        /// `serialize_value` byte-equality is preserved across rotation, over
        /// generated RGB pairs.
        #[test]
        fn rotate_serialize_naturality_on_generated_pairs(
            left in proptest::sample::select(fixtures::rgb_values()),
            right in proptest::sample::select(fixtures::rgb_values()),
        ) {
            let iso = fixtures::rgb_rotate();
            let source_same =
                serialize_value(&fixtures::rgb(), &left) == serialize_value(&fixtures::rgb(), &right);
            let target_same = serialize_value(&fixtures::rgb(), &iso.forward_value(&left))
                == serialize_value(&fixtures::rgb(), &iso.forward_value(&right));
            prop_assert_eq!(
                source_same, target_same,
                "byte-equality is preserved across rotation on generated pairs"
            );
        }
    }
}
