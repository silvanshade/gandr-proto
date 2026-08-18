//! Certificate-**composition** fixtures (ADR-69 D3;
//! `proposal-vdc-reflection.md` §4.3, §9).
//!
//! These crate-level fixtures **mirror the intended surface corpus programs**
//! `compose-directed-cycle.gandr` and `fanout-family.gandr` (§9), realized over
//! the engine's public API. A description's `rule` members do reach the cell
//! store today (the description route), so the corpus measurement below builds
//! its cells that way; what these fixtures still need the API for is the
//! *certificate* level, which no surface syntax reaches.
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
//! - [`a_single_polarity_partner_hides_the_divergence_from_every_probe`] is why
//!   that divergence needs a partner chosen for it. A loop takes a `Both`
//!   endpoint on **each** side, so a partner carrying none admits every
//!   presentation and the left side's presentation stops mattering — which
//!   looks like invariance and is not.
//! - [`the_refined_seam_criterion_declines_strictly_less_than_the_union_reading`]
//!   is the **measurement** the refinement was required to take first: over a
//!   corpus of certificate pairs built from real description-route rules, it
//!   runs the superseded union reading beside the shipped pairwise one, and
//!   reports the over-decline the refinement recovers.
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
//! [`a_single_polarity_partner_hides_the_divergence_from_every_probe`]: tests::a_single_polarity_partner_hides_the_divergence_from_every_probe
//! [`the_refined_seam_criterion_declines_strictly_less_than_the_union_reading`]: tests::the_refined_seam_criterion_declines_strictly_less_than_the_union_reading
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

        // The partner carries the join's producer hole at BOTH polarities. A
        // seam loop needs an endpoint that emits and absorbs on each side, so
        // the partner supplies one half unconditionally and the presentation in
        // front of the gate supplies — or fails to supply — the other. That is
        // what makes the probe sensitive to the presentation rather than to the
        // partner.
        let onward = mixed_onward_certificate(&mut store, &fused_derivation);

        let declined = compose_directed(&two_step, &onward, &store).expect_err(
            "the two-step presentation records a mixed seam hole, which meets the partner's and \
             loops",
        );
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
    fn a_single_polarity_partner_hides_the_divergence_from_every_probe()
    {
        // Why the divergence above needs a partner chosen for it, and why
        // replay-equivalent presentations were once measured agreeing. A seam
        // loop needs an endpoint that both emits and absorbs on EACH side, so a
        // partner whose seam holes are single-polarity contributes no half of
        // one, and every presentation of the left certificate is then admitted
        // — including the one carrying a mixed hole of its own. The agreement
        // is the partner's doing, not the certificate's.
        let (mut store, fused_derivation) = mixed_certificate();
        let two_step = presentation(&fused_derivation, &fused_derivation.path_a);
        let single_step = presentation(&fused_derivation, &fused_derivation.path_b);
        assert!(
            variances(&two_step, &store).contains(&CellVariance::Mixed),
            "the hypothesis: one presentation does record a mixed hole"
        );
        let onward = replayed_onward_partner(&mut store, &fused_derivation);
        for (label, presented) in [
            ("the two-step form", &two_step),
            ("the fused form", &single_step),
        ] {
            let admitted = compose_directed(presented, &onward, &store)
                .unwrap_or_else(|_| panic!("{label} composes: the partner closes no loop"));
            assert!(
                bool::from(admitted.replay(&store)),
                "{label} composite is a real certificate — it replays"
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

    #[test]
    fn the_refined_seam_criterion_declines_strictly_less_than_the_union_reading()
    {
        // The measurement the refinement owed before it was taken (ADR-69's
        // "refine the criterion, never remove the gate"). The superseded
        // reading took its two flow flags from the UNION of both sides'
        // endpoints and then drew the whole bipartite edge set, so one producer
        // occurrence anywhere and one consumer occurrence anywhere closed a
        // loop between every pair — which is the ordinary sequential seam, not
        // a cycle. The shipped reading asks each pair: does the left endpoint
        // emit and the right absorb?
        //
        // `union_reading` below is that superseded criterion, kept here as an
        // independent oracle rather than as a second production path. It reads
        // only the public store metadata, so it cannot drift into agreeing with
        // the gate by sharing its implementation.
        let (store, corpus) = certificate_corpus();
        assert!(
            corpus.len() >= 4,
            "the corpus carries several real certificates, not one probe"
        );
        let mut pairs = 0_usize;
        let mut union_declines = 0_usize;
        let mut shipped_declines = 0_usize;
        let mut recovered = 0_usize;
        for left in &corpus {
            for right in &corpus {
                pairs = pairs.saturating_add(1);
                let union = union_reading(left, right, &store);
                let shipped = compose_directed(left, right, &store);
                if union {
                    union_declines = union_declines.saturating_add(1);
                }
                match shipped {
                    | Err(_) => {
                        shipped_declines = shipped_declines.saturating_add(1);
                        assert!(
                            union,
                            "the refinement only ever admits more: a shipped decline is a \
                             decline under the superseded reading too"
                        );
                    },
                    | Ok(_) => {
                        if union {
                            recovered = recovered.saturating_add(1);
                        }
                    },
                }
            }
        }
        assert_eq!(
            union_declines.saturating_sub(shipped_declines),
            recovered,
            "the over-decline is exactly the recovered set, since the refinement is monotone"
        );
        assert!(
            recovered > 0,
            "measured, not asserted: over {pairs} pairs the superseded reading declined \
             {union_declines} and the shipped criterion declines {shipped_declines}, so the \
             refinement recovers {recovered}"
        );
        assert!(
            shipped_declines > 0,
            "and the gate is refined rather than removed — it still declines {shipped_declines} \
             of {pairs}"
        );

        // The counts above range over every ordered pair, because the gate's
        // verdict is a function of the two certificates and the store alone.
        // What a recovered composition is *worth* needs the sequential seam as
        // well (`a.joins_at == b.overlap.peak`), so one recovered pair is built
        // to satisfy it and the composite is replayed.
        let (mut seamed, left) = mixed_certificate();
        let two_step = presentation(&left, &left.path_a);
        let partner = replayed_onward_partner(&mut seamed, &left);
        assert!(
            union_reading(&two_step, &partner, &seamed),
            "the superseded reading declines this pair: a mixed hole on one side set both flags"
        );
        let composite = compose_directed(&two_step, &partner, &seamed)
            .expect("the shipped criterion admits it: the partner carries no `Both` endpoint");
        assert!(
            bool::from(composite.replay(&seamed)),
            "and the composition the refinement recovers is a real certificate — it replays"
        );
    }

    /// A corpus of real certificates over one store, for the measurement.
    ///
    /// The cells are the **description route's own**: a `Nat` signature with
    /// `add` and `double` and their four written faces, elaborated through
    /// [`elaborate_data_desc`] exactly as a `data` block's rule members are —
    /// so the measurement runs over rules a user could write, not over probes
    /// shaped to produce an answer. The certificates are every composition
    /// overlap the resulting store enumerates, fused; the mixed- and
    /// split-variance step cells are appended so the corpus also carries the
    /// dinaturality shape a written first-order rule cannot spell.
    ///
    /// [`elaborate_data_desc`]: gandr_theory_computads::elaborate_data_desc
    fn certificate_corpus() -> (CellStore, alloc::vec::Vec<Tracelet>)
    {
        use gandr_theory_levitation::Attrs;
        use gandr_theory_levitation::BridgeArity;
        use gandr_theory_levitation::Code;
        use gandr_theory_levitation::CtorDesc;
        use gandr_theory_levitation::DeclPolarity;
        use gandr_theory_levitation::FreeTerm;
        use gandr_theory_levitation::NominalId;
        use gandr_theory_levitation::OperDesc;
        use gandr_theory_levitation::RuleFace;
        use gandr_theory_levitation::SignDesc;
        use gandr_theory_levitation::SortRef;
        use gandr_theory_levitation::SurfaceSpan;

        let nat_op = |name: &str, inputs: alloc::vec::Vec<SortRef>| {
            OperDesc::new(
                name,
                BridgeArity::single_output(inputs, SortRef::new("out", "Nat")),
                Attrs::empty(),
            )
        };
        let face = |lhs: FreeTerm, rhs: FreeTerm| {
            RuleFace::new(
                lhs,
                rhs,
                alloc::vec::Vec::new(),
                SurfaceSpan::new(0_usize.into(), 0_usize.into()),
            )
        };
        let desc = SignDesc::new(
            NominalId::new(0_u64.into(), "Nat"),
            alloc::vec::Vec::new(),
            [
                CtorDesc::new("Zero", Code::Unit, "Nat", Attrs::empty()),
                CtorDesc::new("Succ", Code::var("Nat"), "Nat", Attrs::empty()),
            ],
            [
                nat_op("add", alloc::vec![
                    SortRef::new("m", "Nat"),
                    SortRef::new("n", "Nat")
                ]),
                nat_op("double", alloc::vec![SortRef::new("m", "Nat")]),
            ],
            [
                face(
                    FreeTerm::op("add", [FreeTerm::ctor("Zero", []), FreeTerm::var("n")]),
                    FreeTerm::var("n"),
                ),
                face(
                    FreeTerm::op("add", [
                        FreeTerm::ctor("Succ", [FreeTerm::var("m")]),
                        FreeTerm::var("n"),
                    ]),
                    FreeTerm::ctor("Succ", [FreeTerm::op("add", [
                        FreeTerm::var("m"),
                        FreeTerm::var("n"),
                    ])]),
                ),
                face(
                    FreeTerm::op("double", [FreeTerm::ctor("Zero", [])]),
                    FreeTerm::ctor("Zero", []),
                ),
                face(
                    FreeTerm::op("double", [FreeTerm::ctor("Succ", [FreeTerm::var("m")])]),
                    FreeTerm::ctor("Succ", [FreeTerm::ctor("Succ", [FreeTerm::op(
                        "double",
                        [FreeTerm::var("m")],
                    )])]),
                ),
            ],
            DeclPolarity::Data,
            Attrs::empty(),
        );
        let elaborated = gandr_theory_computads::elaborate_data_desc(&desc);
        assert!(
            elaborated.declined_faces.is_empty() && elaborated.declined_opers.is_empty(),
            "the corpus description elaborates whole, so the measurement runs over real rules"
        );
        let mut store = elaborated.store;
        // The dinaturality shape, which a written first-order face cannot
        // spell: a hole worn at both polarities.
        let mixed_left = store.insert(mixed_step(
            FixtureHoleName("r"),
            OperationName("dn1"),
            OperationName("mid1"),
        ));
        let mixed_right = store.insert(mixed_step(
            FixtureHoleName("s"),
            OperationName("mid1"),
            OperationName("up1"),
        ));
        let split_left = store.insert(split_step(
            FixtureHoleName("p"),
            FixtureHoleName("c"),
            OperationName("dn2"),
            OperationName("mid2"),
        ));
        let split_right = store.insert(split_step(
            FixtureHoleName("q"),
            FixtureHoleName("d"),
            OperationName("mid2"),
            OperationName("up2"),
        ));
        let mut corpus = alloc::vec::Vec::new();
        corpus.push(fused(&mut store, mixed_left, mixed_right));
        corpus.push(fused(&mut store, split_left, split_right));
        // Every composition overlap the description's own cells enumerate.
        let overlaps: alloc::vec::Vec<Overlap> = enumerate_overlaps(&store)
            .into_iter()
            .filter(|candidate| candidate.kind == OverlapKind::Composition)
            .collect();
        for overlap in overlaps {
            if let Some((_, certificate)) = derive_fused(&overlap, &mut store) {
                corpus.push(certificate);
            }
        }
        (store, corpus)
    }

    /// The **superseded** seam criterion, as an independent oracle.
    ///
    /// For each hole of `a`'s recorded join it collects the `(cell, hole)`
    /// endpoints on both sides, takes one forward flag and one backward flag
    /// over their **union**, and draws every bipartite edge in each flagged
    /// direction. A decline is any directed cycle in that graph — which, for a
    /// complete bipartite edge set, is exactly "both flags are set and both
    /// sides are non-empty".
    ///
    /// # Contract
    /// - ensures: positive exactly when some seam hole of `a.joins_at` has an
    ///   endpoint on each side and the union of those endpoints carries a
    ///   producer occurrence and a consumer occurrence.
    /// - panics: none.
    fn union_reading(
        a: &Tracelet,
        b: &Tracelet,
        store: &CellStore,
    ) -> bool
    {
        let a_cells = participating(a);
        let b_cells = participating(b);
        for hole in seam_hole_names(&a.joins_at) {
            let a_side = endpoint_variances(&a_cells, &hole, store);
            let b_side = endpoint_variances(&b_cells, &hole, store);
            if a_side.is_empty() || b_side.is_empty() {
                continue;
            }
            let forward = a_side
                .iter()
                .chain(&b_side)
                .any(|variance| matches!(*variance, CellVariance::Producer | CellVariance::Mixed));
            let backward = a_side
                .iter()
                .chain(&b_side)
                .any(|variance| matches!(*variance, CellVariance::Consumer | CellVariance::Mixed));
            if forward && backward {
                return true;
            }
        }
        false
    }

    /// The distinct metavariable **names** of a command pattern — the seam
    /// variables, keyed exactly as the gate keys them.
    fn seam_hole_names(cmd: &CmdPat) -> alloc::vec::Vec<alloc::string::String>
    {
        let CmdPat::Cut {
            ref prod, ref cons, ..
        } = *cmd;
        let mut names = alloc::vec::Vec::new();
        collect_producer_names(prod, &mut names);
        collect_consumer_names(cons, &mut names);
        names
    }

    /// Append the distinct metavariable names of a producer pattern.
    fn collect_producer_names(
        prod: &ProdPat,
        names: &mut alloc::vec::Vec<alloc::string::String>,
    )
    {
        match *prod {
            | ProdPat::Meta(ref var) => push_name(&var.name, names),
            | ProdPat::Ctor { ref args, .. } => {
                for arg in args {
                    collect_producer_names(arg, names);
                }
            },
        }
    }

    /// Append the distinct metavariable names of a consumer pattern.
    fn collect_consumer_names(
        cons: &ConsPat,
        names: &mut alloc::vec::Vec<alloc::string::String>,
    )
    {
        match *cons {
            | ConsPat::Meta(ref var) => push_name(&var.name, names),
            | ConsPat::Op {
                ref args, ref ret, ..
            } => {
                for arg in args {
                    collect_producer_names(arg, names);
                }
                collect_consumer_names(ret, names);
            },
            | ConsPat::Frame { ref ret, .. } => collect_consumer_names(ret, names),
            | ConsPat::Top => {},
        }
    }

    /// Push `name` if it is not already recorded.
    fn push_name(
        name: &str,
        names: &mut alloc::vec::Vec<alloc::string::String>,
    )
    {
        if !names.iter().any(|held| held == name) {
            names.push(alloc::string::String::from(name));
        }
    }

    /// The variance each of `cells` classifies `hole` at, skipping cells that
    /// do not carry it.
    fn endpoint_variances(
        cells: &[CellId],
        hole: &str,
        store: &CellStore,
    ) -> alloc::vec::Vec<CellVariance>
    {
        let mut out = alloc::vec::Vec::new();
        for &cell in cells {
            let Some(entry) = store.get(cell)
            else {
                continue;
            };
            for var in &entry.meta.vars {
                if &*var.var.name == hole {
                    out.push(var.variance);
                }
            }
        }
        out
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
    /// A single-polarity partner contributes no half of a seam loop, so it
    /// admits every presentation shown to it — which is what makes it the right
    /// probe for the *invertible* lane, whose composite must be an invariant
    /// with no gate in the way at all.
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

    /// A partner certificate for `certificate`'s join recording one cell whose
    /// hole is worn at **both** polarities on the join's producer name.
    ///
    /// The counterpart of [`onward_certificate`], and the pair is what makes a
    /// presentation probe honest. A seam loop needs an endpoint that emits and
    /// absorbs on **each** side, so this partner supplies one half and leaves
    /// the other to whichever presentation is shown; the single-polarity
    /// partner supplies neither and admits them all.
    ///
    /// Its cell reduces the same join as [`onward_certificate`]'s: matching
    /// `⟨s | up1(; s')⟩` binds the producer `s` and the consumer `s` to
    /// different images, because a metavariable's identity is its
    /// `(name, category)` pair.
    fn mixed_onward_certificate(
        store: &mut CellStore,
        certificate: &Tracelet,
    ) -> Tracelet
    {
        let cell = store.insert(mixed_step(
            FixtureHoleName("s"),
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
