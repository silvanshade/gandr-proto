//! **Cell-admission linearity** fixtures — the pathological coverage for the
//! linearity ruling (owner ruling 2026-08-01,
//! `spec:implementation/circuit-terms.md` §"The design questions",
//! `circuit-terms-question-17`; placement decided 2026-08-02).
//!
//! # Why these live here, and what half is promoted
//!
//! These crate-level fixtures cover the refusal exhaustively at the boundary.
//! The **description route is promoted**: the corpus program
//! `examples/pathological/desc/nonlinear-cell-refused.gandr` reaches
//! [`gandr_theory_computads::elaborate_data_desc`] through
//! `gandr-surface-engine`'s description cell pass, and the harness asserts the
//! diagnostic names the copy (`expect-desc-cell-decline`). What stays parked is
//! the **surface `rule`-member route**: `rule` members of ordinary programs
//! parse and are **declined** in the surface lowering (`gandr-surface-engine`'s
//! `lower/codata.rs`), so the refusal is not reachable from the main lowering
//! path; flipping that is the ADR-54 acceptance step, and the fixtures below
//! remain the exhaustive coverage until it lands (`docs/workflow/corpus.md`,
//! the internal-before-surface rule).
//!
//! # The fixtures
//!
//! - [`the_idempotence_rule_written_with_a_repeated_hole_is_refused`] mirrors
//!   **`nonlinear-cell-refused.gandr`**: `rule and(x, x) ==> x`, the
//!   idempotence law written with a copy, refused at admission with the copy
//!   named and the respelling pointed at.
//! - [`the_cancellation_rule_written_with_a_repeated_hole_is_refused`] mirrors
//!   the same program's second member, `rule sub(x, x) ==> Zero` — the
//!   cancellation shape, refused for the same reason, so the diagnostic is not
//!   tied to one operation.
//! - [`the_linear_companion_rule_is_admitted`] mirrors
//!   **`linear-cell-admitted.gandr`**: the respelled neighbour `rule and(x, y)
//!   ==> y` is admitted unchanged, so the refusal is the copy and not the
//!   operation.
//! - [`a_refused_face_does_not_stop_its_linear_neighbours`] pins the
//!   decline-and-report posture: one refused face declines by index and the
//!   description's other rules still land.
//!
//! [`the_idempotence_rule_written_with_a_repeated_hole_is_refused`]: tests::the_idempotence_rule_written_with_a_repeated_hole_is_refused
//! [`the_cancellation_rule_written_with_a_repeated_hole_is_refused`]: tests::the_cancellation_rule_written_with_a_repeated_hole_is_refused
//! [`the_linear_companion_rule_is_admitted`]: tests::the_linear_companion_rule_is_admitted
//! [`a_refused_face_does_not_stop_its_linear_neighbours`]: tests::a_refused_face_does_not_stop_its_linear_neighbours

#[cfg(test)]
mod tests
{
    use gandr_theory_cell_complexes::CellCount;
    use gandr_theory_cell_complexes::DeclinedFaceIndex;
    use gandr_theory_computads::ElaborateError;
    use gandr_theory_computads::elaborate_data_desc;
    use gandr_theory_levitation::Attrs;
    use gandr_theory_levitation::Code;
    use gandr_theory_levitation::CtorDesc;
    use gandr_theory_levitation::DeclPolarity;
    use gandr_theory_levitation::FreeTerm;
    use gandr_theory_levitation::NominalId;
    use gandr_theory_levitation::RuleFace;
    use gandr_theory_levitation::SignDesc;
    use gandr_theory_levitation::SurfaceSpan;

    /// The intended gandr program, as a description: a one-constructor `Bit`
    /// carrying the given rule members.
    fn bit_theory<C>(cells: C) -> SignDesc
    where
        C: Into<Box<[RuleFace]>>,
    {
        SignDesc::new(
            NominalId::new(0_u64.into(), "Bit"),
            Vec::new(),
            [CtorDesc::new("Off", Code::Unit, "Bit", Attrs::empty())],
            Vec::new(),
            cells,
            DeclPolarity::Data,
            Attrs::empty(),
        )
    }

    /// One surface `rule lhs ==> rhs` member.
    fn rule(
        lhs: FreeTerm,
        rhs: FreeTerm,
    ) -> RuleFace
    {
        RuleFace::new(
            lhs,
            rhs,
            Vec::new(),
            SurfaceSpan::new(0_usize.into(), 0_usize.into()),
        )
    }

    /// The single refusal a description reports, with its face index.
    fn sole_refusal(desc: &SignDesc) -> (DeclinedFaceIndex, String)
    {
        let elaborated = elaborate_data_desc(desc);
        let declines = elaborated.declined_faces;
        assert_eq!(1, declines.len(), "exactly one face is declined");
        let (index, error) = declines.into_iter().next().expect("the decline exists");
        let ElaborateError::NonLinear(refusal) = error
        else {
            panic!("the decline is the linearity refusal, not a fragment decline")
        };
        (index, format!("{refusal}"))
    }

    #[test]
    fn the_idempotence_rule_written_with_a_repeated_hole_is_refused()
    {
        // `rule and(x, x) ==> x`.
        let desc = bit_theory([rule(
            FreeTerm::op("and", [FreeTerm::var("x"), FreeTerm::var("x")]),
            FreeTerm::var("x"),
        )]);
        let (index, diagnostic) = sole_refusal(&desc);
        assert_eq!(
            DeclinedFaceIndex::from(0_usize),
            index,
            "the refusal is reported against the rule member"
        );
        assert!(
            diagnostic.contains("the producer hole `x`"),
            "the diagnostic names the copy: {diagnostic}"
        );
        assert!(
            diagnostic.contains("and(x, x) ==> x") && diagnostic.contains("x - x ==> 0"),
            "the diagnostic points at the idempotence and cancellation respellings: {diagnostic}"
        );
        assert!(
            diagnostic.contains("matching through the copying cell"),
            "the diagnostic says how to respell, not only that it refused: {diagnostic}"
        );
        assert!(
            diagnostic.contains("cocommutative comonoid"),
            "the diagnostic names the type-supplied hosting generalization: {diagnostic}"
        );
    }

    #[test]
    fn the_cancellation_rule_written_with_a_repeated_hole_is_refused()
    {
        // `rule sub(x, x) ==> Off` — the cancellation shape.
        let desc = bit_theory([rule(
            FreeTerm::op("sub", [FreeTerm::var("x"), FreeTerm::var("x")]),
            FreeTerm::ctor("Off", []),
        )]);
        let (_index, diagnostic) = sole_refusal(&desc);
        assert!(
            diagnostic.contains("the producer hole `x`"),
            "the same copy is named whatever the operation: {diagnostic}"
        );
    }

    #[test]
    fn the_linear_companion_rule_is_admitted()
    {
        // `rule and(x, y) ==> y` — the respelled neighbour, every hole once.
        let desc = bit_theory([rule(
            FreeTerm::op("and", [FreeTerm::var("x"), FreeTerm::var("y")]),
            FreeTerm::var("y"),
        )]);
        let elaborated = elaborate_data_desc(&desc);
        assert!(
            elaborated.declined_faces.is_empty(),
            "a linear rule is admitted: {:?}",
            elaborated.declined_faces
        );
        assert_eq!(
            CellCount::from(2_usize),
            elaborated.store.len(),
            "the Off frame cell and the rule cell"
        );
    }

    #[test]
    fn a_refused_face_does_not_stop_its_linear_neighbours()
    {
        let desc = bit_theory([
            rule(
                FreeTerm::op("and", [FreeTerm::var("x"), FreeTerm::var("y")]),
                FreeTerm::var("y"),
            ),
            rule(
                FreeTerm::op("and", [FreeTerm::var("z"), FreeTerm::var("z")]),
                FreeTerm::var("z"),
            ),
        ]);
        let elaborated = elaborate_data_desc(&desc);
        let declines = &elaborated.declined_faces;
        assert_eq!(1, declines.len(), "only the copying face declines");
        let &(index, ref error) = declines.first().expect("the decline exists");
        assert_eq!(
            DeclinedFaceIndex::from(1_usize),
            index,
            "the decline is indexed to the second member"
        );
        let ElaborateError::NonLinear(ref refusal) = *error
        else {
            panic!("the decline is the linearity refusal")
        };
        assert_eq!(
            &*refusal.copied.name, "z",
            "the copy the second member wrote is the one named"
        );
        assert_eq!(
            CellCount::from(2_usize),
            elaborated.store.len(),
            "the frame cell and the linear rule still land"
        );
    }
}
