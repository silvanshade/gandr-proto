//! Two adversarial alphabets, read through the rewriting step.
//!
//! The delegating wrapper must be transparent everywhere it does not lie, and
//! an alphabet whose splice disturbs a sibling it was not asked about must show
//! that through the step rather than be caught by a shape check the engine does
//! not perform.

#[cfg(test)]
mod tests
{
    extern crate alloc;

    use gandr_theory_cell_complexes_tools::adversarial::NonLocalSplice;
    use gandr_theory_cell_complexes_tools::adversarial::lying_cell;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyPos;
    use gandr_theory_cell_complexes_tools::toy::toy_cell;
    use gandr_theory_coherent_resolutions::rewrite::rewrite_at;

    #[test]
    fn the_non_local_splice_alphabet_disturbs_a_sibling_it_was_not_asked_about()
    {
        // The lie is in `splice_cmd_at`, so it shows up through `rewrite_at`:
        // firing at `[0]` also restores `[1]`, which the honest alphabet leaves
        // exactly as it found it.
        let peel = lying_cell::<NonLocalSplice>(Toy::succ(Toy::Zero), Toy::Zero);
        let honest_peel = toy_cell(Toy::succ(Toy::Zero), Toy::Zero);
        let term = Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero));
        let at_left = ToyPos(alloc::vec![0_usize].into_boxed_slice());
        assert_eq!(
            Some(Toy::add(Toy::Zero, Toy::succ(Toy::Zero))),
            rewrite_at(&honest_peel, &term, &at_left),
            "the honest splice touches only the position it was given"
        );
        assert_eq!(
            Some(Toy::add(Toy::Zero, Toy::add(Toy::Zero, Toy::Zero))),
            rewrite_at(&peel, &term, &at_left),
            "and the non-local one resets the sibling as well"
        );
    }
}
