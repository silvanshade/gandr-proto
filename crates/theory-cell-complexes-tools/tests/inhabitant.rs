//! The fixture is an honest inhabitant, and each wrapper lies about exactly one
//! thing.
//!
//! These witnesses hold the fixture itself rather than any engine above it: a
//! wrapper that drifted into lying about a second law would silently change
//! what every adversarial test one crate over is measuring, and nothing else in
//! the tree would notice.

#[cfg(test)]
mod tests
{
    extern crate alloc;

    use gandr_theory_cell_complexes::CellAlphabet;
    use gandr_theory_cell_complexes::PositionOrder;
    use gandr_theory_cell_complexes::SubstitutionDecision;
    use gandr_theory_cell_complexes_tools::adversarial::IncomparablePositions;
    use gandr_theory_cell_complexes_tools::adversarial::Lying;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyAlphabet;
    use gandr_theory_cell_complexes_tools::toy::ToyNameRef;
    use gandr_theory_cell_complexes_tools::toy::ToyPos;

    /// **Matching a schematic pattern and applying the result is the identity
    /// on the matched term.**
    ///
    /// This is the law every engine above the substrate spends: a cell fires by
    /// matching its left-hand side and substituting into its right-hand side,
    /// so a fixture whose match and substitution disagree would make every
    /// alphabet-generic result measured over it meaningless.
    #[test]
    fn matching_then_substituting_returns_the_matched_term()
    {
        let pattern = Toy::add(Toy::var(ToyNameRef("x")), Toy::var(ToyNameRef("y")));
        let ground = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let mut subst = <ToyAlphabet as CellAlphabet>::Subst::default();
        assert_eq!(
            ToyAlphabet::match_cmd(&pattern, &ground, &mut subst),
            SubstitutionDecision::from(true),
            "the schematic pattern matches the ground term"
        );
        assert_eq!(
            ToyAlphabet::apply_subst(&subst, &pattern),
            ground,
            "applying the match to the pattern reproduces the term it matched"
        );
    }

    /// **Every metavariable the pattern names is bound by a successful match.**
    #[test]
    fn a_successful_match_binds_every_metavariable_the_pattern_names()
    {
        let pattern = Toy::add(Toy::var(ToyNameRef("x")), Toy::var(ToyNameRef("y")));
        let ground = Toy::add(Toy::Zero, Toy::succ(Toy::Zero));
        let mut subst = <ToyAlphabet as CellAlphabet>::Subst::default();
        assert_eq!(
            ToyAlphabet::match_cmd(&pattern, &ground, &mut subst),
            SubstitutionDecision::from(true),
            "the schematic pattern matches the ground term"
        );
        let applied = ToyAlphabet::apply_subst(&subst, &pattern);
        assert!(
            ToyAlphabet::metavariables(&applied).is_empty(),
            "the substituted term names no metavariable the match left free"
        );
    }

    /// **Splicing at a position returns the subterm the same position reads.**
    #[test]
    fn splicing_at_a_position_agrees_with_reading_it()
    {
        let term = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let left = ToyPos(alloc::vec![0_usize].into_boxed_slice());
        let read =
            ToyAlphabet::subterm_cmd_at(&term, &left).expect("the left child is addressable");
        let spliced =
            ToyAlphabet::splice_cmd_at(&term, &left, read).expect("splicing back what was read");
        assert_eq!(
            spliced, term,
            "splicing a subterm back where it came from is the identity"
        );
    }

    /// **The delegating wrapper differs from the honest alphabet on the one law
    /// it is built to break, and on nothing else it is asked here.**
    ///
    /// `IncomparablePositions` exists to answer `Incomparable` for a nesting
    /// pair the honest alphabet orders. If it ever started delegating that
    /// answer too, every adversarial witness built on it would pass for the
    /// wrong reason.
    #[test]
    fn the_incomparable_wrapper_breaks_the_position_order_and_keeps_the_match()
    {
        let outer = ToyPos(alloc::vec![].into_boxed_slice());
        let inner = ToyPos(alloc::vec![0_usize].into_boxed_slice());
        assert_eq!(
            ToyAlphabet::position_order(&outer, &inner),
            PositionOrder::Encloses,
            "the honest alphabet orders a nesting pair"
        );
        assert_eq!(
            <Lying<IncomparablePositions> as CellAlphabet>::position_order(&outer, &inner),
            PositionOrder::Incomparable,
            "the wrapper calls the same pair incomparable, which is its whole lie"
        );
        let pattern = Toy::add(Toy::var(ToyNameRef("x")), Toy::Zero);
        let ground = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let mut honest = <ToyAlphabet as CellAlphabet>::Subst::default();
        let mut lying = <Lying<IncomparablePositions> as CellAlphabet>::Subst::default();
        assert_eq!(
            <Lying<IncomparablePositions> as CellAlphabet>::match_cmd(
                &pattern, &ground, &mut lying
            ),
            ToyAlphabet::match_cmd(&pattern, &ground, &mut honest),
            "and it delegates the match decision unchanged"
        );
        assert_eq!(
            <Lying<IncomparablePositions> as CellAlphabet>::apply_subst(&lying, &pattern),
            ToyAlphabet::apply_subst(&honest, &pattern),
            "with the same bindings behind it"
        );
    }
}
