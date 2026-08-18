//! Integration witnesses for the overlap enumerator and its support index.
//!
//! These are integration tests rather than unit tests because two of them read
//! the certificate normal form, which sits one crate above this one: an inline
//! test build is a distinct crate instance, so a dev-dependency cycle cannot
//! unify the types across it while an integration target links the library and
//! does.

#[cfg(test)]
mod tests
{
    extern crate alloc;

    use alloc::vec::Vec;

    use gandr_core_sequent::il::Polarity;
    use gandr_theory_cell_complexes::alphabet::CellAlphabet as _;
    use gandr_theory_cell_complexes::boundary::CausalDepth;
    use gandr_theory_cell_complexes::boundary::CellCount;
    use gandr_theory_cell_complexes::boundary::CertificateIndex;
    use gandr_theory_cell_complexes::boundary::CompletionStatus;
    use gandr_theory_cell_complexes::boundary::EventCount;
    use gandr_theory_cell_complexes::boundary::ReplayLevel;
    use gandr_theory_cell_complexes::cell::Cell;
    use gandr_theory_cell_complexes::cell::CellId;
    use gandr_theory_cell_complexes::cell::CellStore;
    use gandr_theory_cell_complexes::pattern::CmdPat;
    use gandr_theory_cell_complexes::pattern::ConsPat;
    use gandr_theory_cell_complexes::pattern::ProdPat;
    use gandr_theory_cell_complexes::pattern::Sym;
    use gandr_theory_cell_complexes::sequent::CellProvenance;
    use gandr_theory_cell_complexes::sequent::Orientation;
    use gandr_theory_cell_complexes::sequent::SequentAlphabet;
    use gandr_theory_cell_complexes::sequent::frame_defining_cell;
    use gandr_theory_coherent_resolutions::completion::CompletionBudget;
    use gandr_theory_coherent_resolutions::completion::CompletionOutcome;
    use gandr_theory_coherent_resolutions::completion::complete;
    use gandr_theory_coherent_resolutions::overlap::*;
    use gandr_theory_coherent_resolutions::rewrite::CellApp;
    use gandr_theory_coherent_resolutions::rewrite::rewrite_at;
    use gandr_theory_coherent_resolutions::tracelet::confluence_tracelet;
    use gandr_theory_deep_inference::normal_form::ReplayWitness;
    use gandr_theory_deep_inference::normal_form::normalize_certified;

    #[test]
    fn the_frame_and_add_cells_compose_into_the_commutation_cell()
    {
        // (Succ⁻-def) then (add-S) compose to Succ⁻(add(n;α)) ~> add(n;Succ⁻(α)).
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let overlaps = enumerate_overlaps(&store);
        let composition = overlaps
            .iter()
            .find(|o| o.kind == OverlapKind::Composition && o.left == frame && o.right == add)
            .expect("the frame RHS ⟨Succ(v)|β⟩ unifies with add-S's LHS ⟨Succ(m)|add(n;α)⟩");
        let peak = &composition.peak;
        let composite = composition.composite(&store).expect("the composite exists");
        // Peak: ⟨v | Succ⁻(add(n;α))⟩ ; composite: ⟨v | add(n; Succ⁻(α))⟩.
        assert!(
            matches!(peak, CmdPat::Cut {
                cons: ConsPat::Frame { .. },
                ..
            }),
            "the peak feeds a Succ⁻ frame into add"
        );
        assert!(
            matches!(composite, CmdPat::Cut {
                cons: ConsPat::Op { .. },
                ..
            }),
            "the composite drops the intermediate Succ, leaving add with a pushed-in frame"
        );
    }

    #[test]
    fn overlaps_are_a_deterministic_family()
    {
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        store.insert(add_s());
        let first = enumerate_overlaps(&store);
        let second = enumerate_overlaps(&store);
        assert_eq!(first, second, "enumeration is deterministic");
    }

    #[test]
    fn the_pair_query_agrees_with_the_store_wide_family()
    {
        // The pair-keyed query is the store-wide sweep's own inner step, so
        // reassembling it pair by pair must reproduce the family exactly —
        // including the suppressed confluence self-overlap.
        let mut store = CellStore::new();
        store.insert(frame_defining_cell(&Sym::new("Succ")));
        store.insert(add_s());
        let mut reassembled = Vec::new();
        for (id_left, cell_left) in store.iter() {
            for (id_right, cell_right) in store.iter() {
                reassembled.extend(overlaps_between(
                    (id_left, cell_left),
                    (id_right, cell_right),
                ));
            }
        }
        assert_eq!(
            enumerate_overlaps(&store),
            reassembled,
            "the pair query is the store-wide family, one ordered pair at a time"
        );
        assert!(
            !reassembled.is_empty(),
            "and the fixture is one that actually overlaps"
        );
    }

    #[test]
    fn overlap_support_batches_are_pairwise_independent()
    {
        let store = independent_rule_clusters();
        let support = OverlapSupport::from_store(&store);
        let overlaps = confluence_family(&store);
        let batches = support.batches(&overlaps);
        // The fixture is ordered: three two-rule clusters over three disjoint
        // operation symbols, so the family alternates between the clusters and
        // a batch can only be built by taking one overlap from each.
        assert_eq!(
            6_usize,
            overlaps.len(),
            "the ordered fixture enumerates each cluster's critical pair in both directions"
        );
        assert!(
            overlaps
                .iter()
                .all(|overlap| overlap.kind == OverlapKind::Confluence),
            "the batched family is the confluence family the completion worklist schedules"
        );
        let mut endpoints: Vec<CellId> = overlaps
            .iter()
            .flat_map(|overlap| [overlap.left, overlap.right])
            .collect();
        endpoints.sort_unstable();
        endpoints.dedup();
        assert_eq!(
            6_usize,
            endpoints.len(),
            "six distinct cells actually take part in an overlap here"
        );
        assert!(
            overlaps.iter().enumerate().all(|(index, left)| {
                overlaps
                    .iter()
                    .skip(index.saturating_add(1_usize))
                    .all(|right| left != right)
            }),
            "the enumerated family carries no duplicate entry, so an input position is exact"
        );
        assert_eq!(
            2_usize,
            batches.len(),
            "the six overlaps schedule into exactly two nonempty batches"
        );
        assert!(
            batches.iter().all(|batch| batch.len() == 3_usize),
            "each batch holds exactly one overlap from each of the three independent clusters"
        );
        assert!(
            batches.iter().any(|batch| batch.len() > 1_usize),
            "at least one batch is genuinely multi-member, so a fully serialized partition — one \
             overlap per batch — fails here"
        );
        // The flatten identity, stated as positions rather than as a length
        // sum: a length sum survives a partition that drops one overlap and
        // duplicates another, and these do not.
        let positions = batch_input_positions(&overlaps, &batches);
        assert!(
            positions.first().is_some_and(|batch| {
                batch.as_slice()
                    == [
                        InputPosition(0_usize),
                        InputPosition(2_usize),
                        InputPosition(4_usize),
                    ]
                    .as_slice()
            }),
            "the first batch is the family's first, third and fifth overlaps, in that order"
        );
        assert!(
            positions.get(1_usize).is_some_and(|batch| {
                batch.as_slice()
                    == [
                        InputPosition(1_usize),
                        InputPosition(3_usize),
                        InputPosition(5_usize),
                    ]
                    .as_slice()
            }),
            "the second batch is the family's second, fourth and sixth overlaps, in that order"
        );
        let mut covered: Vec<InputPosition> = positions.iter().flatten().copied().collect();
        covered.sort_unstable();
        assert_eq!(
            (0 .. overlaps.len())
                .map(InputPosition)
                .collect::<Vec<InputPosition>>(),
            covered,
            "flattening the batches returns the input family with multiplicity, and each batch \
             reads its members in first-appearance order"
        );
        assert!(
            positions.iter().all(|batch| batch.is_sorted()),
            "each batch is a subsequence of the input family, in input order"
        );
        assert!(
            positions
                .iter()
                .map(|batch| batch.first().copied().unwrap_or(InputPosition(0_usize)))
                .collect::<Vec<InputPosition>>()
                .is_sorted(),
            "batch order follows the first appearance of each batch's opening member"
        );
        assert!(
            batches.iter().all(|batch| {
                batch.iter().enumerate().all(|(left_index, left)| {
                    batch
                        .iter()
                        .skip(left_index.saturating_add(1_usize))
                        .all(|right| bool::from(support.overlaps_are_independent(left, right)))
                })
            }),
            "every batch is pairwise independent under the support relation"
        );
        // The same fixture, carried through to an actual replay plan: the
        // leading cluster's critical pair joins, and the certified derivation
        // it produces is replayed three ways.
        let witness = cluster_replay_witness(&store);
        let plan = witness.replay_plan();
        assert_eq!(
            2_usize,
            plan.levels().len(),
            "the certified path schedules two dependency levels"
        );
        assert_eq!(
            CausalDepth::from(plan.levels().len()),
            plan.critical_path(),
            "the critical-path fuel is the number of dependency levels"
        );
        assert_eq!(
            witness.canonical_path().len(),
            plan.levels().iter().map(Vec::len).sum::<usize>(),
            "the plan schedules every certified step exactly once"
        );
        let start = SequentAlphabet::skolemize(witness.peak());
        let sequential = replay_sequentially(&store, &start, &witness.canonical_path());
        let eager = plan
            .replay_with_fuel(&store, plan.critical_path())
            .expect("critical-path fuel does not obstruct the plan")
            .expect("the complete plan returns an outcome");
        assert_eq!(
            sequential, eager,
            "eager planned replay reaches the sequential replay's term"
        );
        let mut on_demand = SequentAlphabet::skolemize(witness.peak());
        for level in 0 .. plan.levels().len() {
            on_demand = plan
                .replay_level(&store, &on_demand, ReplayLevel::from(level))
                .expect("each dependency level replays on demand");
        }
        assert_eq!(
            sequential, on_demand,
            "per-level on-demand replay reaches the sequential replay's term"
        );
        assert_eq!(
            SequentAlphabet::skolemize(witness.joins_at()),
            sequential,
            "and that term is the certified join"
        );
    }

    #[test]
    fn a_relabelled_twin_schedules_and_replays_identically()
    {
        // The twin renames every binder and every metavariable label and fixes
        // every constructor and operation symbol, so nothing name-free about
        // the schedule, the plan, or the completion result may move.
        let store = independent_rule_clusters();
        let twin = relabelled_rule_clusters();
        assert_ne!(
            store, twin,
            "the twin is a genuinely different store, not the same one twice"
        );
        let support = OverlapSupport::from_store(&store);
        let twin_support = OverlapSupport::from_store(&twin);
        let overlaps = confluence_family(&store);
        let twin_overlaps = confluence_family(&twin);
        let batches = support.batches(&overlaps);
        let twin_batches = twin_support.batches(&twin_overlaps);
        assert_eq!(
            batch_shape(&batches),
            batch_shape(&twin_batches),
            "relabelling moves no batch boundary and no batch member"
        );
        assert_eq!(
            batch_input_positions(&overlaps, &batches),
            batch_input_positions(&twin_overlaps, &twin_batches),
            "relabelling moves no flatten position"
        );
        let witness = cluster_replay_witness(&store);
        let twin_witness = cluster_replay_witness(&twin);
        let plan = witness.replay_plan();
        let twin_plan = twin_witness.replay_plan();
        // A `ReplayPlan` also carries the peak it starts from, and the peak is
        // the one part of it that spells metavariable labels — so the plans are
        // equal in their whole scheduling content and unequal as values. That
        // is the finding, not a weakening: the schedule is name-free and the
        // boundary is not.
        assert_eq!(
            plan.levels(),
            twin_plan.levels(),
            "the twin schedules the same cells at the same positions, level for level"
        );
        assert_eq!(
            plan.critical_path(),
            twin_plan.critical_path(),
            "the twin needs the same critical-path fuel"
        );
        let replayed = plan
            .replay_with_fuel(&store, plan.critical_path())
            .expect("critical-path fuel does not obstruct the plan")
            .expect("the complete plan returns an outcome");
        let twin_replayed = twin_plan
            .replay_with_fuel(&twin, twin_plan.critical_path())
            .expect("critical-path fuel does not obstruct the twin plan")
            .expect("the complete twin plan returns an outcome");
        assert_eq!(
            SequentAlphabet::skolemize(witness.joins_at()),
            replayed,
            "the fixture's plan replays to its own certified join"
        );
        assert_eq!(
            SequentAlphabet::skolemize(twin_witness.joins_at()),
            twin_replayed,
            "the twin's plan replays to its own certified join"
        );
        let budget = CompletionBudget::new(64_usize.into(), 64_usize.into(), 64_usize.into());
        assert_eq!(
            completion_shape(&complete(store, budget)),
            completion_shape(&complete(twin, budget)),
            "and the full completion pipeline derives the same cells and emits the same \
             certificate family for both"
        );
    }

    #[test]
    fn overlap_support_is_symmetric_and_certificate_memoized()
    {
        let mut store = CellStore::new();
        let frame = store.insert(frame_defining_cell(&Sym::new("Succ")));
        let add = store.insert(add_s());
        let support = OverlapSupport::from_store(&store);
        assert!(
            !bool::from(support.independent(frame, add)),
            "the frame and add cells have enumerated overlap support, so they are not independent"
        );
        assert!(
            !bool::from(support.independent(add, frame)),
            "and the query answers the same in the other argument order"
        );
        assert_eq!(
            support.independent(frame, add),
            support.independent(add, frame),
            "the cell relation is symmetric"
        );
        let composition = enumerate_overlaps(&store)
            .into_iter()
            .find(|overlap| {
                overlap.kind == OverlapKind::Composition
                    && overlap.left == frame
                    && overlap.right == add
            })
            .expect("the generated composition overlap exists");
        let (_fused, certificate) =
            gandr_theory_coherent_resolutions::tracelet::derive_fused(&composition, &mut store)
                .expect("certificate generated");
        let frame_step = certificate
            .path_a
            .iter()
            .find(|step| step.cell == frame)
            .cloned()
            .expect("the certificate path contains the frame step");
        let add_step = certificate
            .path_a
            .iter()
            .find(|step| step.cell == add)
            .cloned()
            .expect("the certificate path contains the add step");
        let mut frame_certificate = certificate.clone();
        frame_certificate.path_a = vec![frame_step.clone()];
        frame_certificate.path_b = vec![frame_step];
        let mut add_certificate = certificate.clone();
        add_certificate.path_a = vec![add_step.clone()];
        add_certificate.path_b = vec![add_step];
        let mut primitive_support = OverlapSupport::from_store(&store);
        primitive_support
            .add_certificates(&[frame_certificate, add_certificate])
            .expect("distinct certificate support range fits");
        assert!(
            !bool::from(primitive_support.certificates_independent(
                CertificateIndex::from(0_usize),
                CertificateIndex::from(1_usize)
            )),
            "distinct certificates inherit dependence from their primitive cells"
        );
        let mut support = OverlapSupport::from_store(&store);
        let first_keys = support
            .add_certificates(core::slice::from_ref(&certificate))
            .expect("first certificate range fits");
        let second_keys = support
            .add_certificates(core::slice::from_ref(&certificate))
            .expect("second certificate range fits");
        assert_eq!(
            first_keys,
            (
                CertificateIndex::from(0_usize),
                CertificateIndex::from(1_usize)
            ),
            "the first insertion takes the opening stable key"
        );
        assert_eq!(
            second_keys,
            (
                CertificateIndex::from(1_usize),
                CertificateIndex::from(2_usize)
            ),
            "and the second takes the next one, monotonically"
        );
        assert!(
            !bool::from(support.certificates_independent(
                CertificateIndex::from(0_usize),
                CertificateIndex::from(0_usize)
            )),
            "a certificate is never independent of itself"
        );
        assert!(
            !bool::from(support.certificates_independent(
                CertificateIndex::from(1_usize),
                CertificateIndex::from(1_usize)
            )),
            "and neither is its separately inserted copy"
        );
        assert!(
            !bool::from(support.certificates_independent(
                CertificateIndex::from(0_usize),
                CertificateIndex::from(1_usize)
            )),
            "identical certificates from separate calls retain cross-support"
        );
        assert_ne!(first_keys, second_keys, "stable keys remain distinct");
        let mut batched = OverlapSupport::from_store(&store);
        let batched_keys = batched
            .add_certificates(&[certificate.clone(), certificate.clone()])
            .expect("batched certificate range fits");
        let mut split = OverlapSupport::from_store(&store);
        let split_first = split
            .add_certificates(core::slice::from_ref(&certificate))
            .expect("split first certificate range fits");
        let split_second = split
            .add_certificates(core::slice::from_ref(&certificate))
            .expect("split second certificate range fits");
        assert_eq!(
            batched_keys,
            (
                CertificateIndex::from(0_usize),
                CertificateIndex::from(2_usize)
            ),
            "one batched call takes the whole half-open key range"
        );
        assert_eq!(
            split_first,
            (
                CertificateIndex::from(0_usize),
                CertificateIndex::from(1_usize)
            ),
            "and splitting it hands out the same range in two pieces"
        );
        assert_eq!(
            split_second,
            (
                CertificateIndex::from(1_usize),
                CertificateIndex::from(2_usize)
            ),
            "the second of those two pieces"
        );
        assert_eq!(
            batched, split,
            "support is invariant under call partitioning"
        );
    }

    /// (add-S): ⟨Succ(m) | add(n; α)⟩ ~> ⟨m | add(n; Succ⁻(α))⟩.
    fn add_s() -> Cell
    {
        let lhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
        );
        let rhs = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("m"),
            ConsPat::op(
                "add",
                [ProdPat::meta("n")],
                ConsPat::frame("Succ", ConsPat::meta("alpha")),
            ),
        );
        Cell::new(
            lhs,
            rhs,
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// A rule `⟨K | op(label)⟩ ~> ⟨K | rhs⟩` over a nullary constructor `K`.
    fn ground_rule(
        ctor: &Sym,
        op: &Sym,
        label: &Sym,
        rhs: ConsPat,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor(ctor.as_ref(), []),
                ConsPat::op(op.as_ref(), [], ConsPat::meta(label.as_ref())),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::ctor(ctor.as_ref(), []), rhs),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// A rule `⟨binder | op(label)⟩ ~> ⟨binder | rhs⟩` over a producer
    /// metavariable.
    fn schematic_rule(
        binder: &Sym,
        op: &Sym,
        label: &Sym,
        rhs: ConsPat,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(binder.as_ref()),
                ConsPat::op(op.as_ref(), [], ConsPat::meta(label.as_ref())),
            ),
            CmdPat::cut(Polarity::Positive, ProdPat::meta(binder.as_ref()), rhs),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// Three two-rule clusters over the disjoint operations `f`, `g` and `h`,
    /// followed by the `p`-rule every joined pair replays through.
    ///
    /// Each cluster's ground rule and schematic rule overlap on their own
    /// operation and on nothing else — no right-hand side head (`p`, `q`, `r`)
    /// is any left-hand side head except `p`'s own rule, which pairs with no
    /// one — so the three critical pairs are mutually independent and a batch
    /// can hold one from each. The labels are parameters so the relabelled twin
    /// is the same call with different names.
    fn labelled_clusters(
        label: &Sym,
        binders: [&Sym; 4],
    ) -> CellStore
    {
        let reduced = ConsPat::op("p", [], ConsPat::meta(label.as_ref()));
        let wrapped = ConsPat::op("q", [], ConsPat::op("p", [], ConsPat::meta(label.as_ref())));
        let (f, g, h, p) = (Sym::new("f"), Sym::new("g"), Sym::new("h"), Sym::new("p"));
        let mut store = CellStore::new();
        store.insert(ground_rule(&Sym::new("Zero"), &f, label, reduced.clone()));
        store.insert(schematic_rule(binders[0], &f, label, reduced.clone()));
        store.insert(ground_rule(&Sym::new("Nil"), &g, label, reduced.clone()));
        store.insert(schematic_rule(binders[1], &g, label, wrapped.clone()));
        store.insert(ground_rule(&Sym::new("Unit"), &h, label, reduced));
        store.insert(schematic_rule(binders[2], &h, label, wrapped));
        store.insert(schematic_rule(
            binders[3],
            &p,
            label,
            ConsPat::op("r", [], ConsPat::meta(label.as_ref())),
        ));
        store
    }

    /// The ordered scheduling fixture.
    fn independent_rule_clusters() -> CellStore
    {
        labelled_clusters(&Sym::new("alpha"), [
            &Sym::new("x"),
            &Sym::new("y"),
            &Sym::new("z"),
            &Sym::new("u"),
        ])
    }

    /// The same fixture with every binder and metavariable label renamed and
    /// every constructor and operation symbol held fixed.
    fn relabelled_rule_clusters() -> CellStore
    {
        labelled_clusters(&Sym::new("gamma"), [
            &Sym::new("w"),
            &Sym::new("t"),
            &Sym::new("s"),
            &Sym::new("n"),
        ])
    }

    /// The confluence family — the overlap kind the completion worklist
    /// batches.
    fn confluence_family(store: &CellStore) -> Vec<Overlap>
    {
        enumerate_overlaps(store)
            .into_iter()
            .filter(|overlap| overlap.kind == OverlapKind::Confluence)
            .collect()
    }

    /// The certified replay witness of the fixture's leading critical pair.
    fn cluster_replay_witness(store: &CellStore) -> ReplayWitness
    {
        let overlaps = confluence_family(store);
        let leading = overlaps
            .first()
            .expect("the fixture enumerates a leading confluence overlap");
        let certificate = confluence_tracelet(leading, store, 64_usize.into())
            .expect("the leading cluster's critical pair joins");
        normalize_certified(
            store,
            &certificate.overlap.peak,
            &certificate.joins_at,
            &certificate.path_a,
        )
        .expect("the joined critical pair replays into a certified witness")
    }

    /// Replay a recorded path one step at a time, independently of any plan.
    fn replay_sequentially(
        store: &CellStore,
        start: &CmdPat,
        path: &[CellApp],
    ) -> CmdPat
    {
        let mut current = start.clone();
        for step in path {
            let cell = store
                .get(step.cell)
                .expect("every recorded step names a live cell");
            current = rewrite_at(cell, &current, &step.at).expect("every recorded step fires");
        }
        current
    }

    /// The position of one overlap in an enumerated overlap family.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct InputPosition(usize);

    /// The name-free shape of a completion outcome: whether it completed, the
    /// cells it derived, how many certificates it emitted, and how large its
    /// store ended up.
    #[derive(Debug, Eq, PartialEq)]
    struct CompletionShape
    {
        /// Whether the run reached convergence.
        completed: CompletionStatus,
        /// The cells the run derived, in derivation order.
        derived: Vec<CellId>,
        /// How many certificates the run emitted.
        certificates: EventCount,
        /// How large the run left its store.
        store: CellCount,
    }

    /// The input position of each batch member, per batch.
    fn batch_input_positions(
        overlaps: &[Overlap],
        batches: &[Vec<Overlap>],
    ) -> Vec<Vec<InputPosition>>
    {
        batches
            .iter()
            .map(|batch| {
                batch
                    .iter()
                    .map(|member| {
                        overlaps
                            .iter()
                            .position(|candidate| candidate == member)
                            .map(InputPosition)
                            .expect("every batch member came from the input family")
                    })
                    .collect()
            })
            .collect()
    }

    /// The name-free shape of a batch partition: each member's ordered cell
    /// endpoints and overlap kind.
    fn batch_shape(batches: &[Vec<Overlap>]) -> Vec<Vec<(CellId, CellId, OverlapKind)>>
    {
        batches
            .iter()
            .map(|batch| {
                batch
                    .iter()
                    .map(|overlap| (overlap.left, overlap.right, overlap.kind))
                    .collect()
            })
            .collect()
    }

    /// The name-free shape of one completion outcome.
    fn completion_shape(outcome: &CompletionOutcome) -> CompletionShape
    {
        match *outcome {
            | CompletionOutcome::Completed {
                ref store,
                ref derived,
                ref certificates,
            } => CompletionShape {
                completed: CompletionStatus::from(true),
                derived: derived.clone(),
                certificates: EventCount::from(certificates.len()),
                store: store.len(),
            },
            | CompletionOutcome::Declined {
                ref store,
                ref derived,
                ref certificates,
                ..
            } => CompletionShape {
                completed: CompletionStatus::from(false),
                derived: derived.clone(),
                certificates: EventCount::from(certificates.len()),
                store: store.len(),
            },
        }
    }
}
