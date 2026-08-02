//! **Cell-admission linearity** fixtures — the pathological coverage for the
//! linearity ruling (owner ruling 2026-08-01,
//! `docs/gandr/spec/implementation/circuit-terms.md` §"The design questions",
//! `circuit-terms-question-17`; placement decided 2026-08-02).
//!
//! # Why these live here and not in the surface corpus
//!
//! These crate-level fixtures **mirror the intended surface corpus programs**
//! `nonlinear-cell-refused.gandr` and `linear-cell-admitted.gandr`. The
//! refusal is not reachable from a runnable `.gandr` file today, and the
//! blocker is exact and single: surface `rule` members parse and are
//! **declined** in the surface lowering (`gandr-surface-engine`'s
//! `lower/codata.rs`), and neither `gandr-surface-engine` nor
//! `gandr-surface-corpus` depends on this crate, so no description ever reaches
//! [`gandr_theory_computads::elaborate_data_desc`] — the admission boundary the
//! refusal lives at. Promoting these fixtures to `examples/pathological/` is
//! the ADR-54 acceptance flip: wire the surface `rule` members through to this
//! crate's elaborator, then the two programs below become runnable files with
//! harness assertions on the diagnostic. Until then this is internal-only work
//! under the internal-before-surface rule (`docs/workflow/corpus.md`), and the
//! feature is **not user-writable**.
//!
//! # The fixtures
//!
//! - [`the_idempotence_rule_written_with_a_repeated_hole_is_refused`] mirrors
//!   **`nonlinear-cell-refused.gandr`**: `rule and(x, x) ~> x`, the idempotence
//!   law written with a copy, refused at admission with the copy named and the
//!   respelling pointed at.
//! - [`the_cancellation_rule_written_with_a_repeated_hole_is_refused`] mirrors
//!   the same program's second member, `rule sub(x, x) ~> Zero` — the
//!   cancellation shape, refused for the same reason, so the diagnostic is not
//!   tied to one operation.
//! - [`the_linear_companion_rule_is_admitted`] mirrors
//!   **`linear-cell-admitted.gandr`**: the respelled neighbour `rule and(x, y)
//!   ~> y` is admitted unchanged, so the refusal is the copy and not the
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
    use gandr_theory_computads::CellCount;
    use gandr_theory_computads::DeclinedFaceIndex;
    use gandr_theory_computads::ElaborateError;
    use gandr_theory_computads::elaborate_data_desc;
    use gandr_theory_levitation::Attrs;
    use gandr_theory_levitation::CellFace;
    use gandr_theory_levitation::Code;
    use gandr_theory_levitation::CtorDesc;
    use gandr_theory_levitation::DataDesc;
    use gandr_theory_levitation::DeclPolarity;
    use gandr_theory_levitation::FreeTerm;
    use gandr_theory_levitation::NominalId;
    use gandr_theory_levitation::SurfaceSpan;

    /// The intended gandr program, as a description: a one-constructor `Bit`
    /// carrying the given rule members.
    fn bit_theory<C>(cells: C) -> DataDesc
    where
        C: Into<Box<[CellFace]>>,
    {
        DataDesc::new(
            NominalId::new(0_u64.into(), "Bit"),
            Vec::new(),
            [CtorDesc::new("Off", Code::Unit, None, Attrs::empty())],
            Vec::new(),
            cells,
            DeclPolarity::Data,
            Attrs::empty(),
        )
    }

    /// One surface `rule lhs ~> rhs` member.
    fn rule(
        lhs: FreeTerm,
        rhs: FreeTerm,
    ) -> CellFace
    {
        CellFace::new(
            lhs,
            rhs,
            Vec::new(),
            SurfaceSpan::new(0_usize.into(), 0_usize.into()),
        )
    }

    /// The single refusal a description reports, with its face index.
    fn sole_refusal(desc: &DataDesc) -> (DeclinedFaceIndex, String)
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
        // `rule and(x, x) ~> x`.
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
            diagnostic.contains("and(x, x) ~> x") && diagnostic.contains("x - x ~> 0"),
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
        // `rule sub(x, x) ~> Off` — the cancellation shape.
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
        // `rule and(x, y) ~> y` — the respelled neighbour, every hole once.
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
