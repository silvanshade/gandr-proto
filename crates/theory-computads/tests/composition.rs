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
//! [`directed_composition_declines_a_mixed_variance_cycle`]: tests::directed_composition_declines_a_mixed_variance_cycle
//! [`invertible_composition_of_a_ground_chain_replays`]: tests::invertible_composition_of_a_ground_chain_replays
//! [`directed_composition_of_a_ground_chain_replays`]: tests::directed_composition_of_a_ground_chain_replays
//! [`fanout_family_is_a_multi_sum_not_a_single_rule`]: tests::fanout_family_is_a_multi_sum_not_a_single_rule

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
    use gandr_theory_computads::Tracelet;
    use gandr_theory_computads::compose_directed;
    use gandr_theory_computads::compose_invertible;
    use gandr_theory_computads::derive_fused;
    use gandr_theory_computads::enumerate_overlaps;

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
        let mut store = CellStore::new();
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
