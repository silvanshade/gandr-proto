//! The phase-L2 gate (`proposal-sequent-kernel.md` §9, phase L2): the
//! **fused ≡ two-step** differential, certificate **replay**, the completion
//! **budget** decline, and the **η-polarity** pathological pin.
//!
//! # Adequacy discipline (ADR-71)
//!
//! The fused cell a composition overlap derives is, by construction, the
//! two-step composite of its parts; the differential is the **concurrency
//! theorem adopted as a test** (§7.3.4): over generated ground configurations,
//! *applying the fused cell* must agree with *applying the two constituent
//! cells in sequence*. A disagreement is a genuine defect in the matching /
//! substitution / splicing engine (a finding), never a tolerated divergence —
//! the same posture as the L1 `L ≡ run` differential. The category-theoretic
//! proof machinery of the concurrency theorem is explicitly *not* implemented
//! (§7.3.4); the property test is the witness.

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellId;
    use gandr_theory_computads::CellProvenance;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::CmdPat;
    use gandr_theory_computads::CompletionBudget;
    use gandr_theory_computads::CompletionOutcome;
    use gandr_theory_computads::ConsPat;
    use gandr_theory_computads::DeclineReason;
    use gandr_theory_computads::EtaKind;
    use gandr_theory_computads::Orientation;
    use gandr_theory_computads::OverlapKind;
    use gandr_theory_computads::Pos;
    use gandr_theory_computads::ProdPat;
    use gandr_theory_computads::Sym;
    use gandr_theory_computads::complete;
    use gandr_theory_computads::derive_fused;
    use gandr_theory_computads::enumerate_overlaps;
    use gandr_theory_computads::frame_defining_cell;
    use gandr_theory_computads::rewrite::rewrite_at;
    use proptest::prelude::*;

    /// Number of Peano successors in generated Nat fixtures.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct NatSuccCount(u8);

    /// The Peano `n` as `Succ^count(Zero)`.
    fn nat_of(count: NatSuccCount) -> ProdPat
    {
        let mut acc = ProdPat::ctor("Zero", []);
        for _ in 0 .. count.0 {
            acc = ProdPat::ctor("Succ", [acc]);
        }
        acc
    }

    /// The fused commutation cell `Succ⁻(add(n; α)) ~> add(n; Succ⁻(α))`,
    /// derived from the (Succ⁻-def) ∘ (add-S) composition overlap.
    fn fused_commutation_cell() -> Cell
    {
        let base = peano_store();
        let overlap = enumerate_overlaps(&base)
            .into_iter()
            .find(|candidate| {
                candidate.kind == OverlapKind::Composition
                    && candidate.left == CellId(0)
                    && candidate.right == CellId(2)
            })
            .expect("the frame ∘ add-S composition overlap exists");
        let composite = overlap.composite(&base).expect("the composite is formed");
        Cell::new(
            overlap.peak,
            composite,
            Orientation::CompletionDerived,
            CellProvenance::DerivedByCompletion,
        )
    }

    #[test]
    fn the_fused_cell_certificate_replays_over_the_store()
    {
        let mut store = peano_store();
        let overlap = enumerate_overlaps(&store)
            .into_iter()
            .find(|candidate| {
                candidate.kind == OverlapKind::Composition
                    && candidate.left == CellId(0)
                    && candidate.right == CellId(2)
            })
            .expect("the composition overlap exists");
        let (_id, tracelet) = derive_fused(&overlap, &mut store).expect("fused cell derived");
        assert!(
            bool::from(tracelet.replay(&store)),
            "the fused ≡ two-step certificate replays"
        );
    }

    /// The Peano-add cell store: the `Succ⁻` frame cell (id 0), (add-Z) (id 1),
    /// (add-S) (id 2).
    fn peano_store() -> CellStore
    {
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        store.insert(add_z());
        store.insert(add_s());
        store
    }

    /// (add-Z): ⟨Zero | add(n; α)⟩ ~> ⟨n | α⟩.
    fn add_z() -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("n"),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    proptest! {
        // The core phase gate; the ground fragment is cheap to sample.
        #![proptest_config(ProptestConfig::with_cases(4096))]

        /// `apply(fused) ≡ apply(two-step)` on generated ground configurations
        /// (§7.3.4). For a ground instance `⟨Succ^a(Zero) | Succ⁻(add(Succ^b(Zero);
        /// ★))⟩`, firing the fused commutation cell once agrees with firing
        /// (Succ⁻-def) then (add-S).
        #[test]
        fn fused_equals_two_step(a in 0u8 .. 64, b in 0u8 .. 64)
        {
            let fused = fused_commutation_cell();
            let frame = frame_defining_cell(&Sym::new("Succ"));
            let add_s_cell = add_s();
            let instance = CmdPat::cut(
                Polarity::Positive,
                nat_of(NatSuccCount(a)),
                ConsPat::frame("Succ", ConsPat::op("add", [nat_of(NatSuccCount(b))], ConsPat::Top)),
            );
            let via_fused = rewrite_at(&fused, &instance, &Pos::root());
            let via_two_step = rewrite_at(&frame, &instance, &Pos::root())
                .and_then(|after_frame| rewrite_at(&add_s_cell, &after_frame, &Pos::root()));
            prop_assert!(via_fused.is_some(), "the fused cell fires on the instance");
            prop_assert_eq!(
                via_fused,
                via_two_step,
                "the fused cell and the two-step composite agree"
            );
        }
    }

    /// (add-S): ⟨Succ(m) | add(n; α)⟩ ~> ⟨m | add(n; Succ⁻(α))⟩.
    fn add_s() -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Succ", [ProdPat::meta("m")]),
                ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("m"),
                ConsPat::op(
                    "add",
                    [ProdPat::meta("n")],
                    ConsPat::frame("Succ", ConsPat::meta("alpha")),
                ),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    #[test]
    fn completion_certificates_replay()
    {
        let outcome = complete(
            joinable_store(),
            CompletionBudget::new(64_usize.into(), 32_usize.into(), 128_usize.into()),
        );
        assert!(
            bool::from(outcome.is_completed()),
            "the joinable system completes"
        );
        assert!(
            !outcome.certificates().is_empty(),
            "a joinable critical pair emits a certificate"
        );
        for certificate in outcome.certificates() {
            assert!(
                bool::from(certificate.replay(outcome.store())),
                "every coherence certificate replays"
            );
        }
    }

    #[test]
    fn a_starved_completion_declines_with_what_was_left()
    {
        // The joinable system has two confluence overlaps ((r1,r2) and (r2,r1));
        // a one-step budget cannot drain the worklist, so completion declines
        // carrying the rest.
        let outcome = complete(
            joinable_store(),
            CompletionBudget::new(1_usize.into(), 32_usize.into(), 128_usize.into()),
        );
        match outcome {
            | CompletionOutcome::Declined {
                reason, pending, ..
            } => {
                assert_eq!(DeclineReason::StepBudget, reason, "the step ceiling bit");
                assert!(
                    !pending.is_empty(),
                    "the pending overlaps are carried, not dropped"
                );
            },
            | _ => panic!("a one-step budget over a multi-overlap system must decline"),
        }
    }

    /// A joinable overlap system: two rules erasing `f`, whose reducts
    /// coincide.
    fn joinable_store() -> CellStore
    {
        let mut store = CellStore::new();
        // r1: ⟨Zero | f(α)⟩ ~> ⟨Zero | α⟩
        store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::op("f", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Zero", []),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        // r2: ⟨x | f(α)⟩ ~> ⟨x | α⟩
        store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::op("f", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        store
    }

    #[test]
    fn eta_at_the_wrong_polarity_is_rejected()
    {
        // The `eta-wrong-polarity` pathological pin (§11): a data-η cell (positive)
        // must not fire at a negative cut.
        let negative_cut = CmdPat::cut(Polarity::Negative, ProdPat::meta("x"), ConsPat::meta("a"));
        let data_eta = Cell::new(
            negative_cut.clone(),
            negative_cut.clone(),
            Orientation::PolarityDerived,
            CellProvenance::Eta(EtaKind::Data),
        );
        assert_eq!(
            None,
            rewrite_at(&data_eta, &negative_cut, &Pos::root()),
            "data-η is refused at a negative cut"
        );
        // A codata-η cell, requiring negative, fires there.
        let codata_eta = Cell::new(
            negative_cut.clone(),
            negative_cut.clone(),
            Orientation::PolarityDerived,
            CellProvenance::Eta(EtaKind::Codata),
        );
        assert!(
            rewrite_at(&codata_eta, &negative_cut, &Pos::root()).is_some(),
            "codata-η is admitted at a negative cut"
        );
    }
}
