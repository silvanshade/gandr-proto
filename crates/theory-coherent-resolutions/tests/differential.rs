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
    use gandr_theory_cell_complexes::Cell;
    use gandr_theory_cell_complexes::CellId;
    use gandr_theory_cell_complexes::CellProvenance;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes::CmdPat;
    use gandr_theory_cell_complexes::ConsPat;
    use gandr_theory_cell_complexes::EtaKind;
    use gandr_theory_cell_complexes::Orientation;
    use gandr_theory_cell_complexes::Pos;
    use gandr_theory_cell_complexes::ProdPat;
    use gandr_theory_cell_complexes::Sym;
    use gandr_theory_cell_complexes::frame_defining_cell;
    use gandr_theory_coherent_resolutions::CompletionBudget;
    use gandr_theory_coherent_resolutions::CompletionOutcome;
    use gandr_theory_coherent_resolutions::DeclineReason;
    use gandr_theory_coherent_resolutions::OverlapKind;
    use gandr_theory_coherent_resolutions::ReplayPathOutcome;
    use gandr_theory_coherent_resolutions::Tracelet;
    use gandr_theory_coherent_resolutions::complete;
    use gandr_theory_coherent_resolutions::derive_fused;
    use gandr_theory_coherent_resolutions::enumerate_overlaps;
    use gandr_theory_coherent_resolutions::rewrite::rewrite_at;
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

    /// The insertion-ordered Peano store and its fused-equals-two-step
    /// certificate.
    fn fusion_fixture() -> (CellStore, Tracelet)
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
        (store, tracelet)
    }

    #[test]
    fn the_fused_cell_certificate_replays_over_the_store()
    {
        let (store, tracelet) = fusion_fixture();
        assert!(
            bool::from(tracelet.replay(&store)),
            "the fused ≡ two-step certificate replays"
        );
    }

    #[test]
    fn replay_is_pure_over_a_fixed_certificate_and_store()
    {
        let (store, tracelet) = fusion_fixture();
        let first = tracelet.replay_trace(&store);
        let second = tracelet.replay_trace(&store);
        let cloned_store = store.clone();
        let over_clone = tracelet.replay_trace(&cloned_store);

        let emitted_path_a = first
            .path_a
            .steps
            .iter()
            .map(|step| &step.application)
            .collect::<Vec<_>>();
        let emitted_path_b = first
            .path_b
            .steps
            .iter()
            .map(|step| &step.application)
            .collect::<Vec<_>>();
        assert_eq!(
            emitted_path_a,
            tracelet.path_a.iter().collect::<Vec<_>>(),
            "the observable first path contains both recorded applications"
        );
        assert_eq!(
            emitted_path_b,
            tracelet.path_b.iter().collect::<Vec<_>>(),
            "the observable second path contains the recorded fused application"
        );
        assert_eq!(
            first.path_a.outcome,
            ReplayPathOutcome::Reached(first.joins_at.clone()),
            "the first emitted path reaches the skolemized join"
        );
        assert_eq!(
            first.path_b.outcome,
            ReplayPathOutcome::Reached(first.joins_at.clone()),
            "the second emitted path reaches the skolemized join"
        );

        assert_eq!(first, second, "repeating replay emits the same step trace");
        assert_eq!(
            first, over_clone,
            "cloning the fixed store preserves the emitted step trace"
        );
        assert_eq!(
            bool::from(tracelet.replay(&store)),
            bool::from(first.verdict()),
            "the trace verdict agrees with the non-tracing replay verdict"
        );
    }

    #[test]
    fn append_only_store_extension_preserves_replay_trace()
    {
        let (mut store, tracelet) = fusion_fixture();
        let before = tracelet.replay_trace(&store);
        let unrelated = Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Unrelated", []),
                ConsPat::Top,
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("UnrelatedResult", []),
                ConsPat::Top,
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        );

        store.insert(unrelated);
        let after = tracelet.replay_trace(&store);

        assert_eq!(
            before, after,
            "appending an unrelated cell preserves indexed certificate replay"
        );
        assert_eq!(
            bool::from(tracelet.replay(&store)),
            bool::from(after.verdict()),
            "the append-stable trace and verdict agree"
        );
    }

    #[test]
    fn store_permutation_is_not_an_indexed_certificate_invariant()
    {
        let (store, tracelet) = fusion_fixture();
        let canonical = tracelet.replay_trace(&store);
        let cells = store
            .iter()
            .map(|(_id, cell)| cell.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            cells.len(),
            4,
            "the fixture has three primitives and one fused cell"
        );

        let mut permuted = CellStore::new();
        for index in [2_usize, 1, 0, 3] {
            let cell = cells.get(index).expect("the permutation index is in range");
            permuted.insert(cell.clone());
        }
        assert!(
            store
                .iter()
                .all(|(_id, cell)| permuted.iter().any(|(_other_id, other)| other == cell)),
            "the permuted store contains the same structural cell multiset"
        );

        let replayed = tracelet.replay_trace(&permuted);
        assert_ne!(
            canonical.path_a, replayed.path_a,
            "rebinding positional cell zero changes the first emitted path"
        );
        assert_eq!(
            canonical.path_b, replayed.path_b,
            "the fused cell retained at positional cell three still emits the same path"
        );
        assert_eq!(
            replayed.path_a.outcome,
            ReplayPathOutcome::Stuck {
                application: tracelet
                    .path_a
                    .first()
                    .expect("the two-step path has a first application")
                    .clone(),
            },
            "replay exposes the first application rebound by store permutation"
        );
        assert!(
            !bool::from(replayed.verdict()),
            "an indexed certificate is not invariant under store permutation"
        );
        assert_eq!(
            bool::from(tracelet.replay(&permuted)),
            bool::from(replayed.verdict()),
            "the observable and non-tracing routes agree under permutation"
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
        let data_eta: Cell = Cell::new(
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
        let codata_eta: Cell = Cell::new(
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
