//! Three adversarial alphabets, read through the identity relations.
//!
//! Each wrapper breaks exactly one law the shift guard, the convexity discharge
//! or the content address relies on, and each witness names what the engine
//! does when the law fails rather than assuming it holds.

#[cfg(test)]
mod tests
{
    extern crate alloc;

    use gandr_theory_cell_complexes::CellAlphabet;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes::ConvexityDischarge;
    use gandr_theory_cell_complexes::PositionOrder;
    use gandr_theory_cell_complexes_tools::adversarial::CollidingAddresses;
    use gandr_theory_cell_complexes_tools::adversarial::IncomparablePositions;
    use gandr_theory_cell_complexes_tools::adversarial::Lying;
    use gandr_theory_cell_complexes_tools::adversarial::WithheldConvexity;
    use gandr_theory_cell_complexes_tools::adversarial::lying_cell;
    use gandr_theory_cell_complexes_tools::adversarial::reoriented_lying_cell;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyAlphabet;
    use gandr_theory_cell_complexes_tools::toy::ToyPos;
    use gandr_theory_cell_complexes_tools::toy::toy_cell;
    use gandr_theory_coherent_resolutions::rewrite::rewrite_at;

    #[test]
    fn the_incomparable_positions_alphabet_calls_a_nesting_pair_independent()
    {
        // The lie, and its exact shape: the honest alphabet reports the
        // enclosing pair, and this one reports the relation that licenses a
        // shift. `Same` is included, which matters — a primitive is dependent
        // on its own repeat only because `Same` is not `Incomparable`.
        let root = ToyPos(alloc::vec![].into_boxed_slice());
        let child = ToyPos(alloc::vec![0_usize].into_boxed_slice());
        assert_eq!(
            PositionOrder::Encloses,
            ToyAlphabet::position_order(&root, &child),
            "the honest alphabet reports the nesting"
        );
        assert_eq!(
            PositionOrder::Incomparable,
            <Lying<IncomparablePositions> as CellAlphabet>::position_order(&root, &child),
            "and the lying one reports the pair as commutable"
        );
        assert_eq!(
            PositionOrder::Incomparable,
            <Lying<IncomparablePositions> as CellAlphabet>::position_order(&root, &root),
            "including a position against itself"
        );
    }

    #[test]
    fn the_withheld_convexity_alphabet_declines_to_discharge_the_conjunct()
    {
        let honest: CellStore<ToyAlphabet> = CellStore::new();
        let withheld: CellStore<Lying<WithheldConvexity>> = CellStore::new();
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            ToyAlphabet::convexity_discharge(&honest),
            "the toy alphabet discharges the conjunct for every store"
        );
        assert_eq!(
            ConvexityDischarge::ReCheckRequired,
            <Lying<WithheldConvexity> as CellAlphabet>::convexity_discharge(&withheld),
            "and this one withholds the warrant instead"
        );
    }

    #[test]
    fn the_colliding_addresses_alphabet_gives_two_distinct_cells_one_address()
    {
        // The lie's whole effect, isolated: the two cells differ (so a store
        // holds both) and their content addresses agree (so the factorization
        // is asked to hold two different primitives under one key).
        let given = lying_cell::<CollidingAddresses>(Toy::succ(Toy::Zero), Toy::Zero);
        let derived = reoriented_lying_cell::<CollidingAddresses>(Toy::succ(Toy::Zero), Toy::Zero);
        let root = ToyPos(alloc::vec![].into_boxed_slice());
        assert_ne!(
            given, derived,
            "the two cells are structurally distinct, so one store holds both"
        );
        assert_eq!(
            gandr_theory_deep_inference::normal_form::prim_address(&given, &root),
            gandr_theory_deep_inference::normal_form::prim_address(&derived, &root),
            "and the orientation tag they differ in is invisible to the digest"
        );
        // The honest inhabitant keeps them apart, which is what makes the
        // collision attributable to this alphabet rather than to the fixture.
        let honest_given = lying_cell::<IncomparablePositions>(Toy::succ(Toy::Zero), Toy::Zero);
        let honest_derived =
            reoriented_lying_cell::<IncomparablePositions>(Toy::succ(Toy::Zero), Toy::Zero);
        assert_ne!(
            gandr_theory_deep_inference::normal_form::prim_address(&honest_given, &root),
            gandr_theory_deep_inference::normal_form::prim_address(&honest_derived, &root),
            "an honest orientation hash separates them"
        );
    }

    #[test]
    fn a_lying_alphabet_delegates_everything_it_does_not_lie_about()
    {
        // Non-vacuity for every fixture built on `Lying<L>`: the lie is the
        // ONLY difference, so a refusal obtained over the lying alphabet is
        // attributable to `L`. Rewriting is the composite that reads six of the
        // delegated methods at once (subterm, firing permission, matching,
        // substitution, splicing) and the content address reads the cell's
        // whole hash.
        let cell = toy_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero);
        let lying = lying_cell::<IncomparablePositions>(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero);
        let term = Toy::add(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero);
        let at_left = ToyPos(alloc::vec![0_usize].into_boxed_slice());
        assert_eq!(
            rewrite_at(&cell, &term, &at_left),
            rewrite_at(&lying, &term, &at_left),
            "the two alphabets rewrite identically"
        );
        assert_eq!(
            gandr_theory_deep_inference::normal_form::prim_address(&cell, &at_left),
            gandr_theory_deep_inference::normal_form::prim_address(&lying, &at_left),
            "and one content address serves both, so the tie-break is shared"
        );
        assert_eq!(
            ToyAlphabet::skolemize(&Toy::var(
                gandr_theory_cell_complexes_tools::toy::ToyNameRef("x")
            )),
            <Lying<IncomparablePositions> as CellAlphabet>::skolemize(&Toy::var(
                gandr_theory_cell_complexes_tools::toy::ToyNameRef("x")
            )),
            "and skolemization is the toy alphabet's own, not a re-derivation"
        );
    }
}
