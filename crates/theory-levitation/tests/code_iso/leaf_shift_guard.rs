//! **U3.0d — the permanent leaf-shift guard** (U3.0c's sibling: the dual of the
//! negation guard; the ua-base vocabulary design note §2's three-alphabet
//! table, §6).
//!
//! # What this guards
//!
//! That `ua-base`'s completeness quantifier (O2) must range over the
//! **leaf-natural** translator stock — those uniform in leaf contents — and
//! *not* the unrestricted closure stock. The witness: over
//! [`fixtures::int_box`] (`Box = Integer`, an endo-boundary with one *infinite*
//! primitive leaf), the successor/predecessor [`fixtures::leaf_shift`] is a
//! legitimate `CodeIso` member — its round trips hold on every integer — yet it
//! is replay-distinct from the identity and has **no structural preimage**:
//! `IntBox` has exactly one constructor, so the only leaf-natural
//! (constructor-permuting, leaf-preserving) auto-iso is the identity, while the
//! shift moves the leaf content. A completeness statement quantifying over the
//! unrestricted stock is therefore refuted at the first infinite leaf.
//!
//! # Why it is permanent
//!
//! U3.0c pinches the ua-base completeness quantifier from *below* — a
//! code-equality path protype `⤳` collapses the instance set at
//! `(Boolean, Boolean)`. This guard pinches it from *above*: were the O2
//! quantifier ever to range over the unrestricted closure stock — admitting
//! leaf-content shifts as members — then `leaf_shift` would be a member with no
//! structural / code-path preimage, and completeness over the leaf-natural
//! vocabulary would be **refuted at the infinite leaf**. The repair lives in
//! the *statement*, not the vocabulary: O2 ranges over the leaf-natural stock
//! (translators uniform in leaf contents; naturality-in-parameters at stage 1),
//! which excludes `leaf_shift` by construction.
//!
//! This test pins the facts that make the shift unreachable: it is a genuine
//! auto-iso (the round trips hold), it is replay-distinct from the identity,
//! its boundary is the endo-boundary over the infinite `Integer` leaf, and —
//! the crux — `IntBox`'s single constructor forces the structural auto-iso
//! group trivial, so the identity is the sole leaf-natural member. Should the
//! completeness quantifier ever drift to admit leaf-content translators, this
//! named test breaks — a standing guard, like U3.0c, not to be deleted or
//! weakened without owner sign-off.

#[cfg(test)]
mod u30d
{
    use gandr_theory_levitation::Code;
    use gandr_theory_levitation::PrimTy;
    use gandr_theory_levitation::ValueTypeRef;
    use gandr_theory_levitation::generic_eq;

    use crate::fixtures;
    use crate::harness::replay_disagreement;
    use crate::harness::replay_equivalent;

    #[test]
    fn leaf_shift_is_a_replay_distinct_auto_iso_member()
    {
        let shift = fixtures::leaf_shift();
        let identity = fixtures::int_box_identity();
        let samples = fixtures::int_box_values();

        // A legitimate `CodeIso` member: the round trips hold on every sample.
        let report = shift.round_trips(&samples, &samples);
        assert!(
            bool::from(report.holds()),
            "the leaf shift round-trips on every integer: {:?}",
            report.failures
        );
        assert!(
            bool::from(shift.is_monomorphic()),
            "the leaf-shift boundary is monomorphic"
        );
        // …yet it is NOT the identity: replay-distinct over the samples.
        assert!(
            !bool::from(replay_equivalent(&shift, &identity, &samples, &samples)),
            "the leaf shift is replay-distinct from the identity (it moves every leaf)"
        );
    }

    #[test]
    fn the_endo_boundary_is_a_single_constructor_infinite_leaf()
    {
        // The structural premise the "no preimage" claim rests on.
        let shift = fixtures::leaf_shift();
        assert_eq!(
            shift.source(),
            shift.target(),
            "the leaf shift is an auto-iso (endo-boundary)"
        );
        let desc = fixtures::int_box();
        assert_eq!(
            1,
            desc.ctors.len(),
            "IntBox has one constructor, so the structural auto-iso group is trivial"
        );
        assert!(
            matches!(
                desc.ctors[0].code,
                Code::Field(ValueTypeRef::Prim(PrimTy::Integer), _, _)
            ),
            "IntBox's sole field is the unbounded Integer leaf"
        );
    }

    #[test]
    fn the_shift_is_witnessed_as_the_successor_with_no_structural_preimage()
    {
        let shift = fixtures::leaf_shift();
        let identity = fixtures::int_box_identity();
        let samples = fixtures::int_box_values();
        let box_desc = fixtures::int_box();

        // The disagreement is witnessed as the successor: at the leaf `0` the
        // shift yields `1` while the identity yields `0` (`0` is the first
        // sample, so it is the earliest disagreement).
        let (input, shift_image, identity_image) =
            replay_disagreement(&shift, &identity, &samples, &samples)
                .expect("the leaf shift disagrees with the identity under replay");
        assert!(
            bool::from(generic_eq(
                &box_desc,
                &input,
                &fixtures::int_leaf(fixtures::IntBoxLeaf::ZERO)
            )),
            "the earliest disagreement is at the leaf 0"
        );
        assert!(
            bool::from(generic_eq(
                &box_desc,
                &shift_image,
                &fixtures::int_leaf(fixtures::IntBoxLeaf::ONE)
            )),
            "the shift sends 0 to its successor 1"
        );
        assert!(
            bool::from(generic_eq(
                &box_desc,
                &identity_image,
                &fixtures::int_leaf(fixtures::IntBoxLeaf::ZERO)
            )),
            "the identity fixes 0"
        );

        // Content-dependence: the image depends on the leaf read — distinct
        // leaves map to distinct successors — so it is not a constant relabel a
        // leaf-uniform structural translator could mimic.
        assert!(
            !bool::from(generic_eq(
                &box_desc,
                &shift.forward_value(&fixtures::int_leaf(fixtures::IntBoxLeaf::ZERO)),
                &shift.forward_value(&fixtures::int_leaf(fixtures::IntBoxLeaf::ONE)),
            )),
            "the shift maps distinct leaves to distinct images (it reads the content)"
        );

        // No structural preimage: with one constructor the only leaf-natural
        // auto-iso is the identity, yet the shift is replay-distinct from it — so
        // no leaf-content-uniform certificate reaches the shift. O2 must restrict
        // to the leaf-natural stock.
        assert_eq!(
            1,
            box_desc.ctors.len(),
            "the structural auto-iso group is trivial"
        );
        assert!(
            !bool::from(replay_equivalent(&shift, &identity, &samples, &samples)),
            "no leaf-natural (structural) certificate is replay-equivalent to the shift"
        );
    }
}
