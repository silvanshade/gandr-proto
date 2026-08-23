//! The **support-tracked replay memo** differential: a memoized replay must
//! answer exactly what the engine answers, and the measurement of what it
//! saves.
//!
//! # The ladder
//!
//! The differential is four rungs, and the fourth is what makes the first three
//! evidence rather than a tautology:
//!
//! - **(a) the pure hit path** — replaying one certificate twice against a
//!   fixed store executes every step once and reuses every step thereafter,
//!   with the verdict unchanged;
//! - **(b) store evolution** — appending a derived cell leaves every recorded
//!   support intact, so an unaffected certificate replays entirely from the
//!   memo and still agrees with a fresh replay;
//! - **(c) the composition corpus** — every `replay_equivalent` answer over
//!   every ordered pair of the corpus is identical with and without the memo,
//!   under one shared memo and under a fresh memo per pair;
//! - **(d) the poisoned memo** — corrupting one recorded entry makes the
//!   memoized verdict disagree with the engine's, in both directions.
//!
//! **Rung (d) lives in the crate's own unit tests**
//! (`tracelet::tests::a_poisoned_memo_entry_makes_the_memoized_verdict_disagree`
//! and `tracelet::tests::a_poisoned_refusal_licenses_a_step_the_engine_refuses`)
//! because corrupting a memo is not an operation the crate offers its callers:
//! the seam exists only under `cfg(test)`, which an integration binary — which
//! compiles against the ordinary build of the crate — cannot reach. The four
//! rungs are one differential split across two targets, not two differentials.
//!
//! # What the memo is not
//!
//! It carries no authority and never persists. Every assertion here is of the
//! form *the memoized route agrees with the engine*; none of them is of the
//! form *the memo says so*. That is the whole discipline: reuse is sound in
//! process because the engine could be asked again at any point and would say
//! the same thing.

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
    use gandr_theory_cell_complexes::Orientation;
    use gandr_theory_cell_complexes::Pos;
    use gandr_theory_cell_complexes::ProdPat;
    use gandr_theory_cell_complexes::StepExecutionCount;
    use gandr_theory_cell_complexes::StepReuseCount;
    use gandr_theory_cell_complexes::Sym;
    use gandr_theory_cell_complexes::frame_defining_cell;
    use gandr_theory_coherent_resolutions::CellApp;
    use gandr_theory_coherent_resolutions::Overlap;
    use gandr_theory_coherent_resolutions::OverlapKind;
    use gandr_theory_coherent_resolutions::ReplayMemo;
    use gandr_theory_coherent_resolutions::Tracelet;
    use gandr_theory_coherent_resolutions::derive_fused;
    use gandr_theory_coherent_resolutions::enumerate_overlaps;
    use gandr_theory_coherent_resolutions::replay_equivalent;
    use gandr_theory_coherent_resolutions::replay_equivalent_memoized;

    /// Number of `add-S` steps in a synthetic long-path certificate.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct ChainLength(usize);

    /// Whether two certificates carry the same peak and join — the comparison
    /// replay-equivalence makes before it replays either side.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct BoundaryAgreement(bool);

    /// The steps a memo executed and the steps it reused, read together.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct MemoWork
    {
        /// Steps answered by the rewriting engine.
        executed: StepExecutionCount,
        /// Steps answered from the memo.
        reused: StepReuseCount,
    }

    impl MemoWork
    {
        /// The work one memo has done so far.
        fn of(memo: &ReplayMemo) -> Self
        {
            Self {
                executed: memo.steps_executed(),
                reused: memo.steps_reused(),
            }
        }

        /// The work a memo with these two counts would report.
        fn new(
            executed: StepExecutionCount,
            reused: StepReuseCount,
        ) -> Self
        {
            Self { executed, reused }
        }
    }

    #[test]
    fn replaying_one_tracelet_twice_reuses_every_step()
    {
        // Rung (a): the pure hit path. The first replay executes the three steps
        // of the fused≡two-step certificate; every replay after it executes
        // nothing and answers the same verdict.
        let (store, tracelet) = fusion_fixture();
        let honest = tracelet.replay(&store);
        let mut memo = ReplayMemo::new();

        let first = tracelet.replay_memoized(&store, &mut memo);
        assert_eq!(
            MemoWork::new(3_usize.into(), 0_usize.into()),
            MemoWork::of(&memo),
            "the first replay executes each of the three steps once"
        );
        assert_eq!(
            bool::from(honest),
            bool::from(first),
            "the memoized verdict agrees with the engine on the first replay"
        );

        for round in 1_usize .. 4 {
            let again = tracelet.replay_memoized(&store, &mut memo);
            assert_eq!(
                bool::from(honest),
                bool::from(again),
                "the memoized verdict is unchanged on replay {round}"
            );
            assert_eq!(
                MemoWork::new(3_usize.into(), round.saturating_mul(3).into()),
                MemoWork::of(&memo),
                "replay {round} reuses all three steps and executes none"
            );
        }
    }

    #[test]
    fn a_shared_derivation_reuses_its_second_path_within_one_replay()
    {
        // Reuse follows the support, not the path: a certificate whose two
        // recorded derivations coincide runs its second path entirely from the
        // memo, because both paths start at the same skolemized peak.
        let (store, tracelet) = fusion_fixture();
        let duplicated = Tracelet {
            overlap: tracelet.overlap.clone(),
            path_a: tracelet.path_a.clone(),
            path_b: tracelet.path_a.clone(),
            joins_at: tracelet.joins_at,
        };
        let mut memo = ReplayMemo::new();
        let verdict = duplicated.replay_memoized(&store, &mut memo);

        assert!(
            bool::from(verdict),
            "the two-step derivation taken twice still reaches the join"
        );
        assert_eq!(
            bool::from(duplicated.replay(&store)),
            bool::from(verdict),
            "the memoized and engine verdicts agree"
        );
        assert_eq!(
            MemoWork::new(2_usize.into(), 2_usize.into()),
            MemoWork::of(&memo),
            "the second path executes nothing: both of its steps recur"
        );
    }

    #[test]
    fn a_derive_fused_append_reuses_the_unaffected_steps()
    {
        // Rung (b): store evolution. Appending a derived cell preserves every
        // memoized support, because the key is resolved content rather than an
        // insertion-order identifier — so a certificate the append did not touch
        // replays entirely from the memo, and still agrees with a fresh replay.
        let mut store = peano_store();
        let compositions = composition_overlaps(&store);
        let (first, rest) = compositions
            .split_first()
            .expect("the Peano store has at least one composition overlap");
        let (_first_id, tracelet) =
            derive_fused(first, &mut store).expect("the first fused cell is derived");

        let mut memo = ReplayMemo::new();
        let before = tracelet.replay_memoized(&store, &mut memo);
        let executed_before = MemoWork::of(&memo).executed;
        assert_eq!(
            bool::from(tracelet.replay(&store)),
            bool::from(before),
            "the memoized verdict agrees before the store grows"
        );

        let mut appended = 0_usize;
        for overlap in rest {
            if derive_fused(overlap, &mut store).is_some() {
                appended = appended.saturating_add(1);
            }
        }
        // Even with nothing further to derive, the unrelated-cell append below
        // exercises the same property, so the rung does not depend on the
        // fixture's overlap count.
        store.insert(unrelated_cell());
        appended = appended.saturating_add(1);
        assert!(appended > 0, "the store grew by at least one cell");

        let after = tracelet.replay_memoized(&store, &mut memo);
        assert_eq!(
            bool::from(tracelet.replay(&store)),
            bool::from(after),
            "the memoized verdict agrees after the store grows"
        );
        assert_eq!(
            bool::from(before),
            bool::from(after),
            "growing the store does not change the certificate's verdict"
        );
        assert_eq!(
            executed_before,
            MemoWork::of(&memo).executed,
            "the append executed no step again: every support survived it"
        );
    }

    #[test]
    fn the_composition_corpus_agrees_with_and_without_the_memo()
    {
        // Rung (c): every replay-equivalence answer over the corpus is identical
        // with and without the memo, under one memo shared across the whole
        // corpus and under a fresh memo per question.
        let (store, corpus) = composition_corpus();
        assert!(
            corpus.len() >= 4,
            "the corpus carries several certificates and their twins"
        );

        // Every certificate replays the same way through a memo as through the
        // engine — the question replay-equivalence is built out of, asked
        // directly so it is not hidden behind a boundary comparison.
        let mut shared = ReplayMemo::new();
        for certificate in &corpus {
            assert_eq!(
                bool::from(certificate.replay(&store)),
                bool::from(certificate.replay_memoized(&store, &mut shared)),
                "a memoized replay of a corpus certificate agrees with the engine"
            );
        }

        let mut compared = 0_usize;
        let mut replayed = 0_usize;
        for left in &corpus {
            for right in &corpus {
                let honest = replay_equivalent(left, right, &store);
                let pooled = replay_equivalent_memoized(left, right, &store, &mut shared);
                let mut isolated = ReplayMemo::new();
                let alone = replay_equivalent_memoized(left, right, &store, &mut isolated);
                assert_eq!(
                    bool::from(honest),
                    bool::from(pooled),
                    "a shared memo answers the corpus exactly as the engine does"
                );
                assert_eq!(
                    bool::from(honest),
                    bool::from(alone),
                    "a fresh memo answers the corpus exactly as the engine does"
                );
                compared = compared.saturating_add(1);
                if shares_boundary(left, right).0 {
                    replayed = replayed.saturating_add(1);
                    assert!(
                        bool::from(honest),
                        "certificates sharing a boundary are one transformation"
                    );
                }
            }
        }
        assert_eq!(
            corpus.len().saturating_mul(corpus.len()),
            compared,
            "every ordered pair of the corpus was compared"
        );
        assert!(
            replayed >= corpus.len(),
            "each certificate pairs with at least itself and its twin, so the \
             comparison rests on replay rather than on the boundary check"
        );
        assert!(
            usize::from(MemoWork::of(&shared).reused) > 0,
            "the corpus shares supports, so the pooled memo did reuse work"
        );
    }

    #[test]
    fn a_permuted_store_misses_wholesale_rather_than_hitting_wrongly()
    {
        // The property keying on resolved content buys. A permuted store rebinds
        // an identifier to different content; an identifier-keyed memo would
        // answer for content the engine would never have fired. Keyed by
        // content, every permuted step is a miss, and the memoized verdict
        // tracks the engine's — which, for an indexed certificate, is negative.
        let (store, tracelet) = fusion_fixture();
        let mut memo = ReplayMemo::new();
        let canonical = tracelet.replay_memoized(&store, &mut memo);
        assert!(bool::from(canonical), "the certificate replays canonically");

        let permuted = permute(&store, &[CellId(2), CellId(1), CellId(0), CellId(3)]);
        let reused_before = MemoWork::of(&memo).reused;
        let verdict = tracelet.replay_memoized(&permuted, &mut memo);

        assert_eq!(
            bool::from(tracelet.replay(&permuted)),
            bool::from(verdict),
            "the memoized verdict tracks the engine under permutation"
        );
        assert!(
            !bool::from(verdict),
            "an indexed certificate does not survive a store permutation"
        );
        assert!(
            usize::from(MemoWork::of(&memo).reused) > usize::from(reused_before),
            "the fused step is still bound to its own content, so it hits"
        );
    }

    #[test]
    fn the_reuse_ratio_is_measured_on_long_paths_and_the_corpus()
    {
        // The arc's headline measurement, pinned as assertions rather than
        // printed: the counts are the record, and a change to the memo's reuse
        // behavior fails here rather than drifting unobserved.
        //
        // Workload one: a synthetic long-path certificate whose two derivations
        // coincide, replayed four times. The first path of the first replay is
        // the only work the engine ever does.
        let (store, chain) = chain_tracelet(ChainLength(32));
        let mut memo = ReplayMemo::new();
        for round in 0_usize .. 4 {
            assert!(
                bool::from(chain.replay_memoized(&store, &mut memo)),
                "the long-path certificate replays on round {round}"
            );
        }
        assert_eq!(
            MemoWork::new(32_usize.into(), 224_usize.into()),
            MemoWork::of(&memo),
            "four replays of a 32-step two-path certificate execute 32 of 256 steps"
        );

        // Workload two: the composition corpus, every ordered pair asked for
        // replay-equivalence through one shared memo.
        let (corpus_store, corpus) = composition_corpus();
        assert_eq!(
            14,
            corpus.len(),
            "the Peano store fuses seven composition overlaps, each with its twin"
        );
        let mut pooled = ReplayMemo::new();
        for left in &corpus {
            for right in &corpus {
                let _answer = replay_equivalent_memoized(left, right, &corpus_store, &mut pooled);
            }
        }
        let work = MemoWork::of(&pooled);
        assert_eq!(
            MemoWork::new(21_usize.into(), 175_usize.into()),
            work,
            "the corpus resolves one hundred and ninety-six steps out of twenty-one \
             distinct supports — three per boundary, seven boundaries"
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

    /// The Peano store extended with its fused cell, and the certificate over
    /// the frame ∘ (add-S) composition overlap.
    fn fusion_fixture() -> (CellStore, Tracelet)
    {
        let mut store = peano_store();
        let overlap = composition_overlaps(&store)
            .into_iter()
            .find(|candidate| candidate.left == CellId(0) && candidate.right == CellId(2))
            .expect("the frame ∘ add-S composition overlap exists");
        let (_id, tracelet) = derive_fused(&overlap, &mut store).expect("fused cell derived");
        (store, tracelet)
    }

    /// Every composition overlap of `store`, in enumeration order.
    fn composition_overlaps(store: &CellStore) -> Vec<Overlap>
    {
        enumerate_overlaps(store)
            .into_iter()
            .filter(|candidate| candidate.kind == OverlapKind::Composition)
            .collect()
    }

    /// The composition corpus: every composition overlap of the Peano store
    /// fused in enumeration order, with the store grown by every derivation,
    /// and each derived certificate paired with a **twin** — the same
    /// boundary derived by taking its two-step path twice.
    ///
    /// The twins are what make the corpus exercise replay rather than the
    /// boundary comparison. Replay-equivalence answers negatively on a
    /// mismatched boundary without replaying either side, so a corpus of
    /// certificates with pairwise-distinct boundaries would compare the
    /// memoized and engine routes on questions neither route ever replays.
    /// A twin shares its original's peak and join, so the pair is decided
    /// by two replays.
    fn composition_corpus() -> (CellStore, Vec<Tracelet>)
    {
        let mut store = peano_store();
        let mut corpus = Vec::new();
        for overlap in composition_overlaps(&store) {
            if let Some((_id, tracelet)) = derive_fused(&overlap, &mut store) {
                corpus.push(twin_of(&tracelet));
                corpus.push(tracelet);
            }
        }
        (store, corpus)
    }

    /// The same boundary derived by taking `tracelet`'s two-step path twice — a
    /// structurally distinct derivation of one transformation.
    fn twin_of(tracelet: &Tracelet) -> Tracelet
    {
        Tracelet {
            overlap: tracelet.overlap.clone(),
            path_a: tracelet.path_a.clone(),
            path_b: tracelet.path_a.clone(),
            joins_at: tracelet.joins_at.clone(),
        }
    }

    /// Whether two certificates share the boundary replay-equivalence compares
    /// before it replays anything.
    fn shares_boundary(
        left: &Tracelet,
        right: &Tracelet,
    ) -> BoundaryAgreement
    {
        BoundaryAgreement(
            left.overlap.peak == right.overlap.peak && left.joins_at == right.joins_at,
        )
    }

    /// A synthetic long-path certificate: `length` applications of (add-S) at
    /// the root, taken as both recorded derivations of one boundary.
    ///
    /// Firing (add-S) at `⟨Succ^n(Zero) | add(Zero; ★)⟩` peels one successor
    /// and wraps the consumer in a `Succ⁻` frame, so `length` steps from a
    /// `length`-deep numeral land on `⟨Zero | add(Zero; Succ⁻^length(★))⟩`.
    fn chain_tracelet(length: ChainLength) -> (CellStore, Tracelet)
    {
        let store = peano_store();
        let seed = composition_overlaps(&store)
            .into_iter()
            .next()
            .expect("the Peano store has a composition overlap to seed the boundary");

        let mut peak_producer = ProdPat::ctor("Zero", []);
        let mut join_consumer = ConsPat::Top;
        let mut path = Vec::with_capacity(length.0);
        for _ in 0 .. length.0 {
            peak_producer = ProdPat::ctor("Succ", [peak_producer]);
            join_consumer = ConsPat::frame("Succ", join_consumer);
            path.push(CellApp {
                cell: CellId(2),
                at: Pos::root(),
            });
        }
        let mut overlap = seed;
        overlap.peak = CmdPat::cut(
            Polarity::Positive,
            peak_producer,
            ConsPat::op("add", [ProdPat::ctor("Zero", [])], ConsPat::Top),
        );
        let joins_at = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Zero", []),
            ConsPat::op("add", [ProdPat::ctor("Zero", [])], join_consumer),
        );
        let tracelet = Tracelet {
            overlap,
            path_a: path.clone(),
            path_b: path,
            joins_at,
        };
        (store, tracelet)
    }

    /// The store's cells re-inserted in `order`, which must list every
    /// position.
    fn permute(
        store: &CellStore,
        order: &[CellId],
    ) -> CellStore
    {
        let cells = store
            .iter()
            .map(|(_id, cell)| cell.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            cells.len(),
            order.len(),
            "the permutation covers every cell of the store"
        );
        let mut permuted = CellStore::new();
        for id in order {
            let cell = cells
                .get(id.0)
                .expect("the permutation names a stored cell");
            permuted.insert(cell.clone());
        }
        permuted
    }

    /// A cell sharing no symbol with the Peano fixture, for append-only growth.
    fn unrelated_cell() -> Cell
    {
        Cell::new(
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
        )
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
}
