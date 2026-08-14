//! Certificate-**composition** fixtures (ADR-69 D3;
//! `proposal-vdc-reflection.md` §4.3, §9).
//!
//! These crate-level fixtures **mirror the intended surface corpus programs**
//! `compose-directed-cycle.gandr` and `fanout-family.gandr` (§9). The surface
//! `rule` blocks still parse-and-decline in the pipeline (the ADR-54 acceptance
//! flip is a separate open bead), so composition cannot yet be
//! exercised end-to-end from a runnable `.gandr` file; the intended programs
//! are realized here over the engine's public API instead, and the surface-case
//! gap is a reported residual.
//!
//! - [`directed_composition_declines_a_mixed_variance_cycle`] mirrors
//!   **`compose-directed-cycle.gandr`**: two mixed-variance certificates whose
//!   `compose_directed` the acyclicity gate declines, the `(CellId, PatVar)`
//!   cycle the golden. It drives `compose_directed` (not the raw graph witness)
//!   so the witness→obstruction **construction and mapping** is what is tested
//!   (ADR-69 D3; the generic `gandr-theory-graphs` witness test does not
//!   discharge it).
//! - [`invertible_composition_of_a_ground_chain_replays`] and
//!   [`directed_composition_of_a_ground_chain_replays`] are the **differential
//!   rows for composed certificates in both modes** (ADR-69 consequences): a
//!   linear seam composes in the invertible and the directed lane and the
//!   composite replays.
//! - [`fanout_family_is_a_multi_sum_not_a_single_rule`] mirrors
//!   **`fanout-family.gandr`**: a non-linear seam is a family of overlaps,
//!   never a single fused rule (the canonicity honesty of §4.1, §7.4).
//!
//! # What the gate's verdict is, and is not, an invariant of
//!
//! Four fixtures answer one question — which identity on certificates the
//! acyclicity verdict respects — and they are read together:
//!
//! - [`the_acyclicity_verdict_reads_the_recorded_cell_support_and_nothing_finer`]
//!   is the **positive** half: reordering a leg or recording a cell twice
//!   leaves the verdict alone, because the gate's whole input is the recorded
//!   cell support, the recorded join's hole names, and the store.
//! - [`the_acyclicity_verdict_is_not_invariant_under_certificate_identity`] is
//!   the **counterexample**: a cell support is not certificate data, so two
//!   presentations of one certificate can reach the gate as two inputs, and
//!   here they reach `Err` and `Ok` — with the admitted composite replaying.
//! - [`a_mixed_seam_hole_on_either_side_closes_the_loop_whatever_the_presentation`]
//!   is why that divergence needs a partner chosen for it. The gate reads its
//!   two flow flags off **both** sides' endpoints together, so one mixed seam
//!   hole anywhere decides the verdict and every presentation then agrees —
//!   which looks like invariance and is not.
//! - [`the_composite_is_a_certificate_invariant_even_where_the_verdict_is_not`]
//!   and [`invertible_composition_is_well_defined_on_the_replay_quotient`]
//!   bound the damage: the graft's boundary is data replay-equivalence already
//!   compares, so what is presentation-dependent is the decision and never the
//!   thing decided. Each **composes two presentations and compares the two
//!   composites** — a probe that composes one presentation and compares a
//!   boundary it already holds cannot see a presentation effect at all, and a
//!   measured agreement produced that way is evidence of nothing.
//!
//! [`directed_composition_declines_a_mixed_variance_cycle`]: tests::directed_composition_declines_a_mixed_variance_cycle
//! [`invertible_composition_of_a_ground_chain_replays`]: tests::invertible_composition_of_a_ground_chain_replays
//! [`directed_composition_of_a_ground_chain_replays`]: tests::directed_composition_of_a_ground_chain_replays
//! [`fanout_family_is_a_multi_sum_not_a_single_rule`]: tests::fanout_family_is_a_multi_sum_not_a_single_rule
//! [`the_acyclicity_verdict_reads_the_recorded_cell_support_and_nothing_finer`]: tests::the_acyclicity_verdict_reads_the_recorded_cell_support_and_nothing_finer
//! [`the_acyclicity_verdict_is_not_invariant_under_certificate_identity`]: tests::the_acyclicity_verdict_is_not_invariant_under_certificate_identity
//! [`a_mixed_seam_hole_on_either_side_closes_the_loop_whatever_the_presentation`]: tests::a_mixed_seam_hole_on_either_side_closes_the_loop_whatever_the_presentation
//! [`the_composite_is_a_certificate_invariant_even_where_the_verdict_is_not`]: tests::the_composite_is_a_certificate_invariant_even_where_the_verdict_is_not
//! [`invertible_composition_is_well_defined_on_the_replay_quotient`]: tests::invertible_composition_is_well_defined_on_the_replay_quotient

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellId;
    use gandr_theory_computads::CellProvenance;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::CellVariance;
    use gandr_theory_computads::CmdPat;
    use gandr_theory_computads::ConsPat;
    use gandr_theory_computads::Orientation;
    use gandr_theory_computads::Overlap;
    use gandr_theory_computads::OverlapKind;
    use gandr_theory_computads::ProdPat;
    use gandr_theory_computads::ReplayPathOutcome;
    use gandr_theory_computads::Tracelet;
    use gandr_theory_computads::compose_directed;
    use gandr_theory_computads::compose_invertible;
    use gandr_theory_computads::derive_fused;
    use gandr_theory_computads::enumerate_overlaps;
    use gandr_theory_computads::replay_equivalent;

    extern crate alloc;

    /// Nullary constructor name used by fixture-building helpers.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct ConstructorName<'fixture>(&'fixture str);

    /// Metavariable-hole name used by fixture-building helpers.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct FixtureHoleName<'fixture>(&'fixture str);

    /// Operation/frame name used by fixture-building helpers.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct OperationName<'fixture>(&'fixture str);

    #[test]
    fn a_mixed_step_cell_is_classified_mixed()
    {
        // The LIVE derivation (Item 1) — a name at both polarities is `Mixed`.
        let cell = mixed_step(
            FixtureHoleName("r"),
            OperationName("dn"),
            OperationName("up"),
        );
        let hole = cell
            .meta
            .vars
            .iter()
            .find(|v| &*v.var.name == "r")
            .expect("r present");
        assert_eq!(
            CellVariance::Mixed,
            hole.variance,
            "r spans a producer and a consumer position"
        );
    }

    #[test]
    fn invertible_composition_of_a_ground_chain_replays()
    {
        // The coherence lane (LLV Thm 4.5): unconditional, no gate.
        let (store, a, b) = ground_chain();
        assert_eq!(
            a.joins_at, b.overlap.peak,
            "the certificates share the seam ⟨C|★⟩"
        );
        let composite = compose_invertible(&a, &b);
        assert_eq!(
            composite.path_a.len(),
            a.path_a.len() + b.path_a.len(),
            "the composite grafts b's derivation onto a's"
        );
        assert!(
            bool::from(composite.replay(&store)),
            "the invertible composite replays A ~> C ~> E"
        );
    }

    #[test]
    fn directed_composition_of_a_ground_chain_replays()
    {
        // The directed lane over a ground (metavariable-free) seam: the flow
        // graph is empty, so the gate passes and the composite replays — the
        // differential row for the directed mode.
        let (store, a, b) = ground_chain();
        let composite =
            compose_directed(&a, &b, &store).expect("a ground (metavariable-free) seam is acyclic");
        assert!(
            bool::from(composite.replay(&store)),
            "the directed composite replays A ~> C ~> E"
        );
    }

    /// The linear ground chain `A ~> B ~> C ~> D ~> E`: two composable fused
    /// certificates whose seam is the ground term `⟨C | ★⟩` (no metavariables,
    /// so the seam variable-flow graph is empty and the composition is
    /// acyclic in both modes).
    fn ground_chain() -> (CellStore, Tracelet, Tracelet)
    {
        let mut store = CellStore::new();
        let ab = store.insert(ground_step(ConstructorName("A"), ConstructorName("B")));
        let bc = store.insert(ground_step(ConstructorName("B"), ConstructorName("C")));
        let cd = store.insert(ground_step(ConstructorName("C"), ConstructorName("D")));
        let de = store.insert(ground_step(ConstructorName("D"), ConstructorName("E")));
        let first = fused(&mut store, ab, bc); // ⟨A|★⟩ ~> ⟨C|★⟩
        let second = fused(&mut store, cd, de); // ⟨C|★⟩ ~> ⟨E|★⟩
        (store, first, second)
    }

    /// A ground step cell `⟨from | ★⟩ ~> ⟨to | ★⟩` over nullary constructors.
    fn ground_step(
        from: ConstructorName<'_>,
        to: ConstructorName<'_>,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(Polarity::Positive, ProdPat::ctor(from.0, []), ConsPat::Top),
            CmdPat::cut(Polarity::Positive, ProdPat::ctor(to.0, []), ConsPat::Top),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    #[test]
    fn directed_composition_declines_a_mixed_variance_cycle()
    {
        // `compose-directed-cycle.gandr` mirror: two certificates sharing a
        // mixed-variance seam hole. Each `mixed_step` wears the name (`r`, `s`)
        // at both polarities, so the seam flow runs both ways and the acyclicity
        // gate declines — carrying the closed `(CellId, PatVar)` cycle.
        let mut store = CellStore::new();
        let u1 = store.insert(mixed_step(
            FixtureHoleName("r"),
            OperationName("dn1"),
            OperationName("mid1"),
        ));
        let v1 = store.insert(mixed_step(
            FixtureHoleName("s"),
            OperationName("mid1"),
            OperationName("up1"),
        ));
        let u2 = store.insert(mixed_step(
            FixtureHoleName("r"),
            OperationName("dn2"),
            OperationName("mid2"),
        ));
        let v2 = store.insert(mixed_step(
            FixtureHoleName("s"),
            OperationName("mid2"),
            OperationName("up2"),
        ));
        let a = fused(&mut store, u1, v1);
        let b = fused(&mut store, u2, v2);

        let obstruction = compose_directed(&a, &b, &store)
            .expect_err("a shared mixed-variance seam hole cycles the flow");

        // The witness→obstruction mapping (ADR-69 D3): every cycle node is a
        // real `(participating cell, mixed hole)` pair.
        assert!(
            obstruction.cycle.len() >= 2,
            "a 2-cycle through distinct cells (never a self-loop), not empty"
        );
        let reachable: Vec<CellId> = participating(&a)
            .into_iter()
            .chain(participating(&b))
            .collect();
        let mut cycle_cells = Vec::new();
        for node in &obstruction.cycle {
            let cell = node.0;
            let hole = &node.1;
            assert!(
                reachable.contains(&cell),
                "the cycle node names a cell the composition fires"
            );
            let meta = store.get(cell).expect("cell present");
            let classified = meta
                .meta
                .vars
                .iter()
                .find(|v| v.var.name == hole.name)
                .expect("the hole is one of the cell's metavariables");
            assert_eq!(
                CellVariance::Mixed,
                classified.variance,
                "the cycling hole is mixed-variance in its cell"
            );
            if !cycle_cells.contains(&cell) {
                cycle_cells.push(cell);
            }
        }
        assert!(
            cycle_cells.len() >= 2,
            "the loop passes through the seam between two distinct cells"
        );
    }

    #[test]
    fn the_acyclicity_verdict_reads_the_recorded_cell_support_and_nothing_finer()
    {
        // The gate's whole input, positively: the set of cells the two
        // certificates fire, the hole names of the recorded join, and the
        // store. Two presentations that record the same CELLS in a different
        // order, or record one of them twice, are one input to the gate — so
        // the verdict cannot move, whatever else changed about the derivation.
        let (store, a, b) = mixed_pair();
        let baseline = compose_directed(&a, &b, &store);
        let mut reversed = a.clone();
        reversed.path_a.reverse();
        let mut repeated = a.clone();
        repeated.path_a.extend(a.path_a.iter().cloned());
        for (label, variant) in [("a reversed leg", &reversed), ("a repeated leg", &repeated)] {
            assert_eq!(
                cell_support(&a),
                cell_support(variant),
                "{label} records the same cells, which is the hypothesis"
            );
            assert_eq!(
                baseline.is_ok(),
                compose_directed(variant, &b, &store).is_ok(),
                "{label} is the same input to the gate, so the verdict cannot move"
            );
        }
        // The *support* is what the verdict reads; the recorded order still
        // reaches the graph, as the order its nodes are interned in. That
        // orders the reported cycle and never whether there is one.
        assert_ne!(
            participating(&a),
            participating(&reversed),
            "a reversed leg does reorder the cells the graph interns"
        );
    }

    #[test]
    fn the_acyclicity_verdict_is_not_invariant_under_certificate_identity()
    {
        // THE COUNTEREXAMPLE. `replay_equivalent` is a boundary and two
        // replays, so it forgets which cells a derivation recorded; the gate
        // builds its flow graph over exactly those cells. Two presentations of
        // ONE certificate therefore reach the gate as two different inputs, and
        // here they reach two different verdicts.
        //
        // The two presentations are the ones `derive_fused` hands over
        // unaltered: its `path_a` is the two-step derivation and its `path_b`
        // is the single fused step, and taking each as both legs gives two
        // certificates over one boundary. The two-step form records the
        // mixed-variance `v` cell, whose seam hole flows both ways; the fused
        // form records only the fused cell, which wears the join's producer
        // hole and its consumer hole under DIFFERENT names and so flows one way
        // each.
        let (mut store, fused_derivation) = mixed_certificate();
        let two_step = presentation(&fused_derivation, &fused_derivation.path_a);
        let single_step = presentation(&fused_derivation, &fused_derivation.path_b);
        assert!(
            bool::from(replay_equivalent(&two_step, &single_step, &store)),
            "the two presentations are ONE certificate: one boundary, both replay"
        );
        assert_ne!(
            cell_support(&two_step),
            cell_support(&single_step),
            "and they record different cells, which is what the identity forgets"
        );

        // The mechanism, pinned rather than described: every hole of every cell
        // the fused presentation records is single-polarity, and the two-step
        // presentation records a `Mixed` one. Unification renames the peak's
        // producer and consumer apart, so the fused cell — whose faces are the
        // peak and the join — cannot wear one name at both polarities, while
        // the step cells the derivation went through were written to.
        assert!(
            variances(&single_step, &store)
                .iter()
                .all(|variance| *variance != CellVariance::Mixed),
            "the fused presentation records no mixed hole anywhere"
        );
        assert!(
            variances(&two_step, &store).contains(&CellVariance::Mixed),
            "and the two-step presentation records one"
        );

        // The partner carries the join's two holes at ONE polarity each, so it
        // contributes no backward flow of its own and the verdict is left to
        // say what the presentation put in front of it.
        let onward = onward_certificate(&mut store, &fused_derivation);

        let declined = compose_directed(&two_step, &onward, &store)
            .expect_err("the two-step presentation records a mixed seam hole, which loops");
        let recorded = participating(&two_step);
        assert!(
            declined.cycle.iter().any(|node| recorded.contains(&node.0)),
            "the cycle runs through a cell only that presentation records"
        );
        let admitted = compose_directed(&single_step, &onward, &store)
            .expect("the fused presentation records no mixed seam hole, so nothing loops");
        assert!(
            bool::from(admitted.replay(&store)),
            "and the composite it admits is a real certificate — it replays"
        );
    }

    #[test]
    fn a_mixed_seam_hole_on_either_side_closes_the_loop_whatever_the_presentation()
    {
        // Why the divergence above needs a partner chosen for it, and why
        // three replay-equivalent presentations were once measured agreeing.
        // The gate reads the forward and backward flags off the union of BOTH
        // sides' endpoints, so one mixed seam hole anywhere in either
        // certificate closes the loop and the other side's presentation stops
        // mattering. Against a partner that carries one, every presentation
        // declines — which looks like invariance and is not.
        let (store, a, b) = mixed_pair();
        let two_step = presentation(&a, &a.path_a);
        let single_step = presentation(&a, &a.path_b);
        for (label, presented) in [
            ("the two-step form", &two_step),
            ("the fused form", &single_step),
        ] {
            assert!(
                compose_directed(presented, &b, &store).is_err(),
                "{label} declines against a partner whose own seam hole is mixed"
            );
        }
        assert_ne!(
            participating(&two_step),
            participating(&single_step),
            "even though the two presentations put different cells in front of the gate"
        );
    }

    #[test]
    fn the_composite_is_a_certificate_invariant_even_where_the_verdict_is_not()
    {
        // The other half, and it bounds the damage. What the gate returns on
        // its `Ok` branch is the graft, whose boundary is the left peak and the
        // right join — both of them data `replay_equivalent` compares — so two
        // presentations that are BOTH admitted compose to one certificate.
        // The non-invariance is confined to the decision, and can therefore
        // cost availability and never soundness: the gate is a sufficient
        // loop-freeness check by construction, and which presentation it is
        // shown decides how conservative it is.
        //
        // The fixture has to compose TWO presentations and compare the two
        // composites. The mixed-variance certificate cannot serve as the
        // subject: its two presentations are precisely the pair the
        // counterexample above splits into `Err` and `Ok`, so only one of them
        // ever reaches a composite. This one is built from SPLIT-variance
        // steps instead — no cell either presentation records wears a hole at
        // both polarities, so the seam graph is non-empty and acyclic whichever
        // presentation is shown, and the gate admits both while they still
        // record different cells.
        let (mut store, fused_derivation) = split_certificate();
        let two_step = presentation(&fused_derivation, &fused_derivation.path_a);
        let single_step = presentation(&fused_derivation, &fused_derivation.path_b);
        assert!(
            bool::from(replay_equivalent(&two_step, &single_step, &store)),
            "the hypothesis: two presentations of ONE certificate"
        );
        assert_ne!(
            cell_support(&two_step),
            cell_support(&single_step),
            "recording different cells, or there is no presentation effect to be invariant under"
        );

        // Why both are admitted, pinned rather than described. Each
        // presentation records metavariable-carrying cells, so the seam graph
        // has nodes rather than being the empty graph a ground seam gives; and
        // none of those holes is worn at both polarities, so no presentation
        // contributes the back edge the counterexample's two-step form does.
        for (label, presented) in [("the two-step", &two_step), ("the fused", &single_step)] {
            let holes = variances(presented, &store);
            assert!(
                !holes.is_empty(),
                "{label} presentation records a metavariable seam, not a ground one"
            );
            assert!(
                holes
                    .iter()
                    .all(|variance| *variance != CellVariance::Mixed),
                "{label} presentation records no mixed hole, so its flow runs one way"
            );
        }

        let onward = replayed_onward_partner(&mut store, &fused_derivation);
        let from_two_step = compose_directed(&two_step, &onward, &store)
            .expect("the two-step presentation records no mixed seam hole, so nothing loops");
        let from_single_step = compose_directed(&single_step, &onward, &store)
            .expect("and neither does the fused presentation");
        assert!(
            bool::from(from_two_step.replay(&store)) && bool::from(from_single_step.replay(&store)),
            "each admitted composite is a real certificate — it replays"
        );
        assert!(
            bool::from(replay_equivalent(&from_two_step, &from_single_step, &store)),
            "and the two composites are ONE certificate: the composite is an invariant"
        );
        assert_ne!(
            from_two_step.path_a, from_single_step.path_a,
            "while the recorded derivations still differ, which is what makes that a claim"
        );
    }

    #[test]
    fn invertible_composition_is_well_defined_on_the_replay_quotient()
    {
        // The invertible lane is unaffected, checked rather than inherited.
        // `compose_invertible` never consults a recorded cell: it grafts, and
        // the graft's boundary is `a`'s peak and `b`'s join, which are exactly
        // what `replay_equivalent` compares. So two presentations of one
        // certificate compose to two presentations of one certificate — the
        // operation descends to the quotient, which is what the directed lane's
        // verdict fails to do.
        let (mut store, fused_derivation) = mixed_certificate();
        let two_step = presentation(&fused_derivation, &fused_derivation.path_a);
        let single_step = presentation(&fused_derivation, &fused_derivation.path_b);
        assert!(
            bool::from(replay_equivalent(&two_step, &single_step, &store)),
            "the hypothesis: two presentations of one certificate"
        );
        let onward = onward_certificate(&mut store, &fused_derivation);
        let from_two_step = compose_invertible(&two_step, &onward);
        let from_single_step = compose_invertible(&single_step, &onward);
        assert!(
            bool::from(from_two_step.replay(&store)) && bool::from(from_single_step.replay(&store)),
            "the graft of two replaying certificates replays, on either presentation"
        );
        assert!(
            bool::from(replay_equivalent(&from_two_step, &from_single_step, &store)),
            "and the two composites are one certificate — the lane descends to the quotient"
        );
        assert_ne!(
            from_two_step.path_a, from_single_step.path_a,
            "while the recorded derivations still differ, which is the point of the quotient"
        );
    }

    /// The variance of every hole of every cell a certificate records.
    fn variances(
        tracelet: &Tracelet,
        store: &CellStore,
    ) -> alloc::vec::Vec<CellVariance>
    {
        let mut out = alloc::vec::Vec::new();
        for cell in participating(tracelet) {
            let Some(entry) = store.get(cell)
            else {
                continue;
            };
            out.extend(entry.meta.vars.iter().map(|var| var.variance));
        }
        out
    }

    /// The distinct cells a certificate fires, ordered — the **support** the
    /// gate's flow graph is built over, as a set rather than as the recorded
    /// sequence [`participating`] returns.
    fn cell_support(tracelet: &Tracelet) -> alloc::vec::Vec<CellId>
    {
        let mut cells = participating(tracelet);
        cells.sort_unstable();
        cells
    }

    /// The mixed-variance pair: two certificates sharing a mixed-variance seam
    /// hole, each derived from its own composition overlap.
    fn mixed_pair() -> (CellStore, Tracelet, Tracelet)
    {
        let mut store = CellStore::new();
        let u1 = store.insert(mixed_step(
            FixtureHoleName("r"),
            OperationName("dn1"),
            OperationName("mid1"),
        ));
        let v1 = store.insert(mixed_step(
            FixtureHoleName("s"),
            OperationName("mid1"),
            OperationName("up1"),
        ));
        let u2 = store.insert(mixed_step(
            FixtureHoleName("r"),
            OperationName("dn2"),
            OperationName("mid2"),
        ));
        let v2 = store.insert(mixed_step(
            FixtureHoleName("s"),
            OperationName("mid2"),
            OperationName("up2"),
        ));
        let a = fused(&mut store, u1, v1);
        let b = fused(&mut store, u2, v2);
        (store, a, b)
    }

    /// The left half of [`mixed_pair`] alone, with its store still open so a
    /// partner can be added to it.
    fn mixed_certificate() -> (CellStore, Tracelet)
    {
        let mut store = CellStore::new();
        let u1 = store.insert(mixed_step(
            FixtureHoleName("r"),
            OperationName("dn1"),
            OperationName("mid1"),
        ));
        let v1 = store.insert(mixed_step(
            FixtureHoleName("s"),
            OperationName("mid1"),
            OperationName("up1"),
        ));
        let a = fused(&mut store, u1, v1);
        (store, a)
    }

    /// A partner certificate for `certificate`'s join whose recorded join is
    /// **computed by replaying it** rather than written down: its one cell is
    /// applied at the certificate's join and the term that comes back is taken
    /// as the partner's `joins_at`.
    ///
    /// [`onward_certificate`] spells its join out, which is correct only for
    /// the certificate it was written against — the hole names a fused
    /// derivation leaves in its join are the unifier's, not the fixture's. This
    /// one composes with any certificate whose join the recorded cell matches,
    /// which is what a fixture comparing **two** presentations needs.
    ///
    /// The cell is single-polarity for the same reason [`onward_certificate`]'s
    /// is: a partner carrying a mixed hole of its own would decide the verdict
    /// before the left presentation was consulted.
    fn replayed_onward_partner(
        store: &mut CellStore,
        certificate: &Tracelet,
    ) -> Tracelet
    {
        let cell = store.insert(split_step(
            FixtureHoleName("s"),
            FixtureHoleName("s'"),
            OperationName("up1"),
            OperationName("up3"),
        ));
        let mut overlap = certificate.overlap.clone();
        overlap.peak = certificate.joins_at.clone();
        let step = gandr_theory_computads::CellApp {
            cell,
            at: gandr_theory_computads::Pos::root(),
        };
        let provisional = Tracelet {
            overlap,
            path_a: alloc::vec![step.clone()],
            path_b: alloc::vec![step],
            joins_at: certificate.joins_at.clone(),
        };
        let ReplayPathOutcome::Reached(reached) = provisional.replay_trace(store).path_a.outcome
        else {
            panic!("the partner's single step applies at the certificate's join")
        };
        Tracelet {
            joins_at: reached,
            ..provisional
        }
    }

    /// A certificate over **split**-variance steps, with its store still open
    /// so a partner can be added to it.
    ///
    /// The counterpart of [`mixed_certificate`], and the difference is exactly
    /// what makes it usable as the composite fixture's subject: no cell either
    /// of its presentations records wears a hole at both polarities, so the
    /// seam flow graph is acyclic on **both** presentations and the gate admits
    /// each — where [`mixed_certificate`]'s two presentations are the pair the
    /// verdict counterexample splits.
    fn split_certificate() -> (CellStore, Tracelet)
    {
        let mut store = CellStore::new();
        let u1 = store.insert(split_step(
            FixtureHoleName("p"),
            FixtureHoleName("c"),
            OperationName("dn1"),
            OperationName("mid1"),
        ));
        let v1 = store.insert(split_step(
            FixtureHoleName("q"),
            FixtureHoleName("d"),
            OperationName("mid1"),
            OperationName("up1"),
        ));
        let a = fused(&mut store, u1, v1);
        (store, a)
    }

    /// One presentation of `certificate`: the same boundary, with `leg`
    /// recorded as both paths.
    ///
    /// Every presentation of a certificate that replays is
    /// [`replay_equivalent`] to every other, which is the identity criterion
    /// under test; what they differ in is which cells they record.
    fn presentation(
        certificate: &Tracelet,
        leg: &[gandr_theory_computads::CellApp],
    ) -> Tracelet
    {
        Tracelet {
            overlap: certificate.overlap.clone(),
            path_a: leg.to_vec(),
            path_b: leg.to_vec(),
            joins_at: certificate.joins_at.clone(),
        }
    }

    /// A partner certificate for `certificate`'s join, recording one cell that
    /// wears the join's two holes at **one polarity each**.
    ///
    /// The single-polarity choice is what makes it a fair probe: the gate reads
    /// its forward and backward flags off both sides' endpoints together, so a
    /// partner carrying a mixed hole of its own would decide the verdict before
    /// the left presentation was consulted.
    ///
    /// Its overlap is a carrier for the recorded peak: the gate and replay both
    /// read `overlap.peak`, and what this fixture is about is the boundary a
    /// certificate records rather than the critical pair that produced it.
    fn onward_certificate(
        store: &mut CellStore,
        certificate: &Tracelet,
    ) -> Tracelet
    {
        let cell = store.insert(split_step(
            FixtureHoleName("s"),
            FixtureHoleName("s'"),
            OperationName("up1"),
            OperationName("up3"),
        ));
        let mut overlap = certificate.overlap.clone();
        overlap.peak = certificate.joins_at.clone();
        let step = gandr_theory_computads::CellApp {
            cell,
            at: gandr_theory_computads::Pos::root(),
        };
        Tracelet {
            overlap,
            path_a: alloc::vec![step.clone()],
            path_b: alloc::vec![step],
            joins_at: CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("s"),
                ConsPat::op("up3", [], ConsPat::meta("s'")),
            ),
        }
    }

    /// A **split**-variance step cell `⟨p | in(; c)⟩ ~> ⟨p | out(; c)⟩`: the
    /// producer and the consumer wear different names, so `p` is
    /// [`CellVariance::Producer`] and `c` is [`CellVariance::Consumer`] and
    /// neither is `Mixed`.
    ///
    /// It is strictly more general than [`mixed_step`], which forces the two
    /// positions to share one name — and a derivation may record either, which
    /// is what the gate turns out to be sensitive to.
    fn split_step(
        producer: FixtureHoleName<'_>,
        consumer: FixtureHoleName<'_>,
        in_op: OperationName<'_>,
        out_op: OperationName<'_>,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(producer.0),
                ConsPat::op(in_op.0, [], ConsPat::meta(consumer.0)),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(producer.0),
                ConsPat::op(out_op.0, [], ConsPat::meta(consumer.0)),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// A mixed-variance step cell `⟨r | in(; r)⟩ ~> ⟨r | out(; r)⟩`: the name
    /// `r` is worn by a producer *and* a consumer metavariable, so the LIVE
    /// derivation classifies it `Mixed` — the dinaturality shape `μ`/`μ̃`
    /// create.
    fn mixed_step(
        hole: FixtureHoleName<'_>,
        in_op: OperationName<'_>,
        out_op: OperationName<'_>,
    ) -> Cell
    {
        Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(hole.0),
                ConsPat::op(in_op.0, [], ConsPat::meta(hole.0)),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta(hole.0),
                ConsPat::op(out_op.0, [], ConsPat::meta(hole.0)),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        )
    }

    /// The fused certificate of the composition overlap `left ∘ right`.
    fn fused(
        store: &mut CellStore,
        left: CellId,
        right: CellId,
    ) -> Tracelet
    {
        let overlap = composition_overlap(store, left, right);
        derive_fused(&overlap, store)
            .expect("the fused cell is derived")
            .1
    }

    /// The single composition overlap `left ∘ right` in `store`.
    fn composition_overlap(
        store: &CellStore,
        left: CellId,
        right: CellId,
    ) -> Overlap
    {
        enumerate_overlaps(store)
            .into_iter()
            .find(|candidate| {
                candidate.kind == OverlapKind::Composition
                    && candidate.left == left
                    && candidate.right == right
            })
            .expect("the composition overlap exists")
    }

    /// The distinct cells a certificate fires (`path_a` then `path_b`).
    fn participating(tracelet: &Tracelet) -> Vec<CellId>
    {
        let mut cells = Vec::new();
        for step in tracelet.path_a.iter().chain(&tracelet.path_b) {
            if !cells.contains(&step.cell) {
                cells.push(step.cell);
            }
        }
        cells
    }

    #[test]
    fn fanout_family_is_a_multi_sum_not_a_single_rule()
    {
        // `fanout-family.gandr` mirror: a non-linear seam. The producer `plus`
        // rule leaves a `g`-redex; two `g`-consumers match it, so composition is
        // the multi-sum FAMILY of both overlaps — never a single fused rule (the
        // canonicity cost of §4.1). One consumer is non-linear (`Pair(y; y)`
        // duplicates `y`), which the LIVE derivation marks.
        let mut store: CellStore = CellStore::new();
        let left = store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("n"),
                ConsPat::op("plus", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("n"),
                ConsPat::op("g", [], ConsPat::meta("alpha")),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        let linear_consumer = store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::op("g", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("x"),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));
        let nonlinear_consumer = store.insert(Cell::new(
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::ctor("Pair", [ProdPat::meta("y"), ProdPat::meta("y")]),
                ConsPat::op("g", [], ConsPat::meta("alpha")),
            ),
            CmdPat::cut(
                Polarity::Positive,
                ProdPat::meta("y"),
                ConsPat::meta("alpha"),
            ),
            Orientation::PolarityDerived,
            CellProvenance::SurfaceRule,
        ));

        // The non-linear consumer's `y` is marked non-linear by the derivation.
        let y = store
            .get(nonlinear_consumer)
            .expect("cell present")
            .meta
            .vars
            .iter()
            .find(|v| &*v.var.name == "y")
            .expect("y present");
        assert!(
            !bool::from(y.linear),
            "Pair(y; y) duplicates y, so y is non-linear"
        );

        // Composition at the `g` seam is a family: `left` composes with BOTH
        // consumers, so there is no single canonical fused rule.
        let family: Vec<Overlap> = enumerate_overlaps(&store)
            .into_iter()
            .filter(|candidate| {
                candidate.kind == OverlapKind::Composition && candidate.left == left
            })
            .collect();
        assert!(
            family.len() >= 2,
            "the g-seam composition fans out to a family (multi-sum), not one rule"
        );
        let mut fused_rules = Vec::new();
        for overlap in &family {
            let composite = overlap.composite(&store).expect("the composite exists");
            if !fused_rules.contains(&composite) {
                fused_rules.push(composite);
            }
        }
        assert!(
            fused_rules.len() >= 2,
            "the family yields distinct fused right-hand sides, never a single rule"
        );
        let _ = (linear_consumer, nonlinear_consumer);
    }
}
