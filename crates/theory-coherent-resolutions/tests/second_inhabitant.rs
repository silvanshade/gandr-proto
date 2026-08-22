//! The engines run over a **second alphabet**, unmodified.
//!
//! The workspace ships one production alphabet, so a claim that the enumerator,
//! the normalizer and completion are alphabet-polymorphic is only worth what a
//! second inhabitant makes of it. These witnesses drive all three over the toy
//! alphabet, and the last one holds the budgeted decline intact.

#[cfg(test)]
mod tests
{
    use gandr_theory_cell_complexes::Cell;
    use gandr_theory_cell_complexes::CellAlphabet as _;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyAlphabet;
    use gandr_theory_cell_complexes_tools::toy::ToyNameRef;
    use gandr_theory_cell_complexes_tools::toy::toy_cell;
    use gandr_theory_coherent_resolutions::CompletionBudget;
    use gandr_theory_coherent_resolutions::CompletionOutcome;
    use gandr_theory_coherent_resolutions::OverlapKind;
    use gandr_theory_coherent_resolutions::complete;
    use gandr_theory_coherent_resolutions::derive_fused;
    use gandr_theory_coherent_resolutions::enumerate_overlaps;
    use gandr_theory_coherent_resolutions::normalize;

    /// (add-Z): `Add(Zero, x) ~> x`.
    fn add_z() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::var(ToyNameRef("x")),
        )
    }

    /// (add-S): `Add(Succ(m), n) ~> Succ(Add(m, n))`.
    fn add_s() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(
                Toy::succ(Toy::var(ToyNameRef("m"))),
                Toy::var(ToyNameRef("n")),
            ),
            Toy::succ(Toy::add(
                Toy::var(ToyNameRef("m")),
                Toy::var(ToyNameRef("n")),
            )),
        )
    }

    /// Every confluence entry the enumeration emits carries the root seam,
    /// over an alphabet that HAS non-root positions.
    ///
    /// **The check that makes the root-only clause non-vacuous.** The sequent
    /// alphabet's command grammar has a single constructor, so every command
    /// position it can produce is the root and the same assertion there is
    /// structural rather than separating. The toy alphabet nests commands, so
    /// interior positions genuinely exist — and a confluence entry must still
    /// carry the root, because the confluence branch unifies whole left-hand
    /// sides and never descends.
    ///
    /// This is what the completeness exception rests on. A confluence entry at
    /// an interior position would be a Knuth-Bendix self-overlap, and the
    /// diagonal exclusion would then be dropping real work.
    #[test]
    fn every_toy_confluence_entry_carries_the_root_seam()
    {
        // Two rules whose left-hand sides unify at the root and whose
        // right-hand sides differ, so the pair survives the diagonal
        // exclusion and the check is not vacuous.
        let mut store: CellStore<ToyAlphabet> = CellStore::new();
        store.insert(toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::var(ToyNameRef("x")),
        ));
        store.insert(toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("y"))),
            Toy::succ(Toy::var(ToyNameRef("y"))),
        ));

        let interior = ToyAlphabet::command_positions(&Toy::add(
            Toy::succ(Toy::var(ToyNameRef("m"))),
            Toy::var(ToyNameRef("n")),
        ));
        assert!(
            interior.len() > 1_usize,
            "the toy alphabet has interior command positions, so the root is a real choice"
        );

        let root = ToyAlphabet::root_position();
        let mut seen = 0_usize;
        for overlap in enumerate_overlaps(&store) {
            if overlap.kind == OverlapKind::Confluence {
                seen = seen.saturating_add(1_usize);
                assert_eq!(
                    root, overlap.seam,
                    "a confluence overlap is a root overlap by construction"
                );
            }
        }
        assert!(
            seen > 0_usize,
            "the fixture produces confluence entries, so the check is not vacuous"
        );
    }

    #[test]
    fn the_enumerator_finds_the_toy_composition_overlap()
    {
        let mut store = CellStore::new();
        let z = store.insert(add_z());
        let s = store.insert(add_s());
        let overlaps = enumerate_overlaps(&store);
        let composition = overlaps
            .iter()
            .find(|o| o.kind == OverlapKind::Composition && o.left == s && o.right == z)
            .expect("add-S's RHS subterm Add(m, n) unifies with add-Z's LHS");
        // Peak: Add(Succ(Zero), x) — the superposition at m = Zero.
        assert_eq!(
            Toy::add(Toy::succ(Toy::Zero), Toy::var(ToyNameRef("x"))),
            composition.peak,
            "the superposition instantiates m to Zero"
        );
        // Composite: Succ(x) — left fires, then add-Z at the seam.
        let composite = composition.composite(&store).expect("the composite exists");
        assert_eq!(
            Toy::succ(Toy::var(ToyNameRef("x"))),
            composite,
            "the fused right-hand side drops the intermediate Add"
        );
        // The derived fused cell replays as a certificate.
        let (_fused, tracelet) =
            derive_fused(composition, &mut store).expect("the fused cell derives");
        assert!(
            bool::from(tracelet.replay(&store)),
            "the toy fused≡two-step certificate replays"
        );
    }

    #[test]
    fn the_normalizer_runs_over_the_toy_alphabet()
    {
        let mut store = CellStore::new();
        store.insert(add_z());
        store.insert(add_s());
        let term = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let out = normalize(&store, &term, 16_usize.into());
        assert_eq!(Toy::succ(Toy::Zero), out.normal, "add-S then add-Z");
        assert_eq!(2, out.path.len(), "two steps");
        assert!(!out.exhausted, "a normal form was reached");
    }

    #[test]
    fn completion_orients_and_certifies_over_the_toy_alphabet()
    {
        // c1: Add(Zero, x) ~> x and c2: Add(Zero, x) ~> Add(x, Zero) share a
        // left-hand side — a confluence critical pair whose reducts diverge.
        let mut store = CellStore::new();
        store.insert(add_z());
        store.insert(toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::add(Toy::var(ToyNameRef("x")), Toy::Zero),
        ));
        let outcome = complete(
            store,
            CompletionBudget::new(64_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        let CompletionOutcome::Completed {
            store: completed,
            derived,
            certificates,
        } = outcome
        else {
            panic!("the toy system completes within budget");
        };
        assert_eq!(
            1,
            derived.len(),
            "the divergence oriented into one new cell"
        );
        assert!(
            !certificates.is_empty(),
            "the derived cell joined the original pair"
        );
        assert!(
            certificates
                .iter()
                .all(|tracelet| bool::from(tracelet.replay(&completed))),
            "every toy certificate replays"
        );
    }

    #[test]
    fn a_starved_toy_budget_declines_with_pending()
    {
        let mut store = CellStore::new();
        store.insert(add_z());
        store.insert(toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::add(Toy::var(ToyNameRef("x")), Toy::Zero),
        ));
        let outcome = complete(
            store,
            CompletionBudget::new(0_usize.into(), 16_usize.into(), 64_usize.into()),
        );
        match outcome {
            | CompletionOutcome::Declined { pending, .. } => {
                assert!(!pending.is_empty(), "the pending overlaps are carried");
            },
            | CompletionOutcome::Completed { .. } => {
                panic!("a zero step budget must decline")
            },
        }
    }
}
