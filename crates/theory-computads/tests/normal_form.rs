//! The **tracelet normal form differential** — generated NF-equal derivation
//! pairs, checked against the replay oracle.
//!
//! # What this suite is for, and why it is here rather than in the crate
//!
//! The normal form's one load-bearing claim is an implication: **NF-equal
//! implies replay-equal**. A unit test can pin the shape of the machinery; only
//! a differential can attack the implication, because the implication is a
//! statement about *every* pair the quotient identifies, not about the pairs
//! someone thought to write down.
//!
//! The suite runs on the **toy alphabet** for the reason
//! [`gandr_theory_computads::shift`] runs there: a sequent command pattern is
//! one cut whose children are a producer and a consumer, so a sequent term has
//! exactly one command position and no two applications can ever be
//! incomparable — the shift quotient's extension over the sequent alphabet is
//! **empty** and a differential over it would exercise nothing. The toy
//! alphabet nests commands, which is what a circuit-shaped body will do, and it
//! lives in this integration crate.
//!
//! # The oracle, and why it is external
//!
//! The oracle is [`gandr_theory_computads::replay_equivalent`] — the engine's
//! own identity criterion, which re-executes both derivations by ground
//! rewriting. It is external to the code under test: nothing in
//! [`gandr_theory_computads::normal_form`] participates in the answer, so a
//! mutant that shifts both the normal form and the oracle identically cannot
//! exist. That criterion is also exactly the engine content of the reflection
//! layer's `cells_equal`, whose cell branch bottoms out in `replay_equivalent`
//! through `elaborations_replay_equivalent`, so "the generated NF-equal pairs
//! satisfy `cells_equal` by replay" is what these properties assert.
//!
//! # The kill signal
//!
//! A shift-equivalent, replay-divergent pair is a **soundness defect in
//! position or overlap bookkeeping**, not a case to accommodate. It surfaces in
//! two places, and both are failures rather than skips:
//!
//! - inside [`gandr_theory_computads::normal_form::normalize`], which replays
//!   its own canonical schedule and refuses with `ShiftedScheduleDoesNotFire` /
//!   `ShiftedScheduleMissesTheJoin` — these properties assert the normalization
//!   succeeds, so either refusal fails the suite with the offending schedule
//!   attached;
//! - in [`every_nf_equal_pair_is_replay_equivalent`], which asserts the
//!   implication against the replay oracle directly.
//!
//! # The fixtures
//!
//! - [`a_shuffled_independent_schedule_has_one_normal_form`] is the quotient
//!   doing work: any permutation of pairwise-independent redexes on a spine
//!   normalizes to the **same** normal form.
//! - [`every_nf_equal_pair_is_replay_equivalent`] is the differential proper —
//!   the implication, over generated pairs, with a non-vacuity assertion so a
//!   generator that stopped producing NF-equal pairs would fail rather than
//!   pass quietly.
//! - [`an_overlapping_pair_keeps_its_recorded_order`] is the
//!   under-approximation exhibited: two orders that demonstrably reach one term
//!   are **not** identified, because the cells overlap. NF-distinct says
//!   nothing.
//! - [`a_reversed_independent_schedule_is_the_canonical_one`] is the
//!   deterministic companion to the shuffle property, on the exact five-step
//!   fixture whose transposition cost the shift suite measures — the case a
//!   generator that quietly stopped permuting would leave uncovered.
//! - [`a_layered_derivation_keeps_its_dependent_step_last`] exercises a
//!   canonical schedule with **two** layers, so the layering is not degenerate:
//!   two independent leaf applications may permute, and the application that
//!   encloses both may not move ahead of them.
//! - [`a_repeated_primitive_is_graded_by_multiplicity`] pins the integer
//!   grading.
//! - [`a_unit_step_is_eliminated_over_the_toy_alphabet`] pins `equiv_T`.
//!
//! [`a_shuffled_independent_schedule_has_one_normal_form`]: tests::a_shuffled_independent_schedule_has_one_normal_form
//! [`every_nf_equal_pair_is_replay_equivalent`]: tests::every_nf_equal_pair_is_replay_equivalent
//! [`an_overlapping_pair_keeps_its_recorded_order`]: tests::an_overlapping_pair_keeps_its_recorded_order
//! [`a_reversed_independent_schedule_is_the_canonical_one`]: tests::a_reversed_independent_schedule_is_the_canonical_one
//! [`a_layered_derivation_keeps_its_dependent_step_last`]: tests::a_layered_derivation_keeps_its_dependent_step_last
//! [`a_repeated_primitive_is_graded_by_multiplicity`]: tests::a_repeated_primitive_is_graded_by_multiplicity
//! [`a_unit_step_is_eliminated_over_the_toy_alphabet`]: tests::a_unit_step_is_eliminated_over_the_toy_alphabet

#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;

    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellApp;
    use gandr_theory_computads::CellId;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::Overlap;
    use gandr_theory_computads::PrimMultiplicity;
    use gandr_theory_computads::Tracelet;
    use gandr_theory_computads::nf_equal;
    use gandr_theory_computads::normal_form::TraceletNf;
    use gandr_theory_computads::normal_form::normalize;
    use gandr_theory_computads::overlaps_between;
    use gandr_theory_computads::prim_address;
    use gandr_theory_computads::replay_equivalent;
    use gandr_theory_computads::rewrite::rewrite_at;
    use gandr_theory_computads::tracelets_nf_equal;
    use proptest::prelude::*;

    use crate::toy_alphabet::Toy;
    use crate::toy_alphabet::ToyAlphabet;
    use crate::toy_alphabet::ToyNameRef;
    use crate::toy_alphabet::ToyPos;
    use crate::toy_alphabet::toy_cell;

    extern crate alloc;

    /// Number of pairwise-independent redexes on a generated spine.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct RedexCount(usize);

    /// A toy position from child indices.
    fn at<Steps>(steps: Steps) -> ToyPos
    where
        Steps: IntoIterator<Item = usize>,
    {
        ToyPos(steps.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    /// (f): `Succ(Zero) ~> Zero` — a ground rule that overlaps nothing, not
    /// even itself, so any two of its applications at incomparable positions
    /// earn the shift witness.
    fn f_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::Zero), Toy::Zero)
    }

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

    /// (nop): `Zero ~> Zero` — a reflexive cell, the unit `equiv_T` eliminates.
    fn nop_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::Zero, Toy::Zero)
    }

    /// The right-nested spine `Add(r, Add(r, … r))` carrying `count`
    /// `f`-redexes at pairwise incomparable positions.
    ///
    /// # Contract
    /// - requires: `count` is at least two.
    /// - ensures: a term whose `count` redex positions are
    ///   [`spine_positions`]'s, all pairwise incomparable.
    /// - panics: none.
    fn spine(count: RedexCount) -> Toy
    {
        let redex = Toy::succ(Toy::Zero);
        let mut term = redex.clone();
        for _ in 0 .. count.0.saturating_sub(1) {
            term = Toy::add(redex.clone(), term);
        }
        term
    }

    /// The `count` redex positions of [`spine`], in outer-to-inner order.
    ///
    /// # Contract
    /// - ensures: `[1]*i ++ [0]` for every redex but the last, and
    ///   `[1]*(count-1)` for the last; the positions are pairwise incomparable.
    /// - panics: none.
    fn spine_positions(count: RedexCount) -> Vec<ToyPos>
    {
        let last = count.0.saturating_sub(1);
        let mut out = Vec::with_capacity(count.0);
        for index in 0 .. last {
            let mut path: Vec<usize> = alloc::vec![1_usize; index];
            path.push(0_usize);
            out.push(at(path));
        }
        out.push(at(alloc::vec![1_usize; last]));
        out
    }

    /// Run a recorded path from `start`, or fail the test naming the step.
    fn run(
        store: &CellStore<ToyAlphabet>,
        start: &Toy,
        path: &[CellApp<ToyAlphabet>],
    ) -> Toy
    {
        let mut current = start.clone();
        for step in path {
            let cell = store.get(step.cell).expect("the step names a stored cell");
            current = rewrite_at(cell, &current, &step.at)
                .expect("the step fires at its recorded position");
        }
        current
    }

    /// An [`Overlap`] to fill a [`Tracelet`]'s enumerator-constructed field.
    ///
    /// [`Overlap`] carries a private apartness-renamed leg, so it is only ever
    /// built by the enumerator; a test that needs a tracelet over a *chosen*
    /// boundary sources one here and overrides its peak. Nothing else about it
    /// is read: [`replay_equivalent`] consults `overlap.peak` and the two
    /// paths, and nothing else. (The welding of [`Tracelet`] to an
    /// [`Overlap`] it does not need for replay is a recorded structural
    /// observation, not a defect this suite introduces.)
    fn filler_overlap() -> Overlap<ToyAlphabet>
    {
        let mut throwaway = CellStore::new();
        let z = throwaway.insert(add_z());
        let s = throwaway.insert(add_s());
        let z_cell = throwaway.get(z).expect("add-Z is stored");
        let s_cell = throwaway.get(s).expect("add-S is stored");
        overlaps_between((s, s_cell), (z, z_cell))
            .into_iter()
            .next()
            .expect("add-S's right-hand side runs into add-Z's left-hand side at a seam")
    }

    /// A tracelet over a chosen boundary and a chosen pair of paths.
    fn tracelet_over(
        peak: &Toy,
        joins_at: &Toy,
        path_a: Vec<CellApp<ToyAlphabet>>,
        path_b: Vec<CellApp<ToyAlphabet>>,
    ) -> Tracelet<ToyAlphabet>
    {
        let mut overlap = filler_overlap();
        overlap.peak = peak.clone();
        Tracelet {
            overlap,
            path_a,
            path_b,
            joins_at: joins_at.clone(),
        }
    }

    /// The `f`-only store, its cell identifier, a spine peak, and the canonical
    /// schedule over it.
    fn spine_fixture(
        count: RedexCount
    ) -> (
        CellStore<ToyAlphabet>,
        CellId,
        Toy,
        Vec<CellApp<ToyAlphabet>>,
    )
    {
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let peak = spine(count);
        let schedule = spine_positions(count)
            .into_iter()
            .map(|position| CellApp {
                cell: f,
                at: position,
            })
            .collect();
        (store, f, peak, schedule)
    }

    /// Normalize a path over the spine fixture, or fail the test with the
    /// obstruction — which is how a kill-signal refusal surfaces.
    fn normalized(
        store: &CellStore<ToyAlphabet>,
        peak: &Toy,
        joins_at: &Toy,
        path: &[CellApp<ToyAlphabet>],
    ) -> TraceletNf<ToyAlphabet>
    {
        match normalize(store, peak, joins_at, path) {
            | Ok(normal) => normal,
            | Err(obstruction) => panic!(
                "the derivation was refused a normal form; a ShiftedSchedule* refusal here is \
                 the lane's KILL SIGNAL: {obstruction:?}"
            ),
        }
    }

    #[test]
    fn an_overlapping_pair_keeps_its_recorded_order()
    {
        // The under-approximation, exhibited. These two applications sit at
        // disjoint positions and their two orders demonstrably reach the SAME
        // term — and the quotient still refuses to identify them, because the
        // cell pair overlaps. So the two normal forms differ while the replay
        // oracle calls the derivations the same transformation: NF-distinct
        // carries no information, exactly as documented.
        let mut store = CellStore::new();
        let z = store.insert(add_z());
        let s = store.insert(add_s());
        let peak = Toy::add(
            Toy::add(Toy::Zero, Toy::succ(Toy::Zero)),
            Toy::add(Toy::succ(Toy::Zero), Toy::Zero),
        );
        let first = CellApp {
            cell: z,
            at: at([0]),
        };
        let second = CellApp {
            cell: s,
            at: at([1]),
        };
        let forward = alloc::vec![first.clone(), second.clone()];
        let backward = alloc::vec![second, first];
        let join = run(&store, &peak, &forward);
        assert_eq!(
            join,
            run(&store, &peak, &backward),
            "the two orders do reach one term at this instance"
        );
        let forward_nf = normalized(&store, &peak, &join, &forward);
        let backward_nf = normalized(&store, &peak, &join, &backward);
        assert!(
            !bool::from(nf_equal(&forward_nf, &backward_nf)),
            "the cells overlap, so no transposition is licensed and the schedules stay apart"
        );
        assert_eq!(
            forward_nf.primitives, backward_nf.primitives,
            "the factorizations agree — it is exactly the schedule the quotient will not merge"
        );
        let a = tracelet_over(&peak, &join, forward.clone(), forward);
        let b = tracelet_over(&peak, &join, backward.clone(), backward);
        assert!(
            bool::from(replay_equivalent(&a, &b, &store)),
            "and the replay oracle identifies them all the same — NF-distinct means nothing"
        );
    }

    #[test]
    fn a_repeated_primitive_is_graded_by_multiplicity()
    {
        // One cell at one position, firing twice: `equiv_A` collapses the two
        // occurrences into one primitive carrying the integer grade 2, and the
        // schedule still has two entries because the second occurrence depends
        // on the first (same position).
        let mut store = CellStore::new();
        let z = store.insert(add_z());
        let peak = Toy::add(Toy::Zero, Toy::add(Toy::Zero, Toy::succ(Toy::Zero)));
        let step = CellApp {
            cell: z,
            at: at([]),
        };
        let path = alloc::vec![step.clone(), step];
        let join = run(&store, &peak, &path);
        assert_eq!(
            Toy::succ(Toy::Zero),
            join,
            "two add-Z steps at the root peel both frames"
        );
        let normal = normalized(&store, &peak, &join, &path);
        assert_eq!(
            1,
            normal.primitives.len(),
            "the two occurrences are one content-addressed primitive"
        );
        assert_eq!(
            2,
            normal.schedule.len(),
            "and the schedule keeps both, because a repeat depends on its predecessor"
        );
        let graded = normal
            .primitives
            .values()
            .next()
            .expect("the factorization holds the one primitive");
        assert_eq!(
            PrimMultiplicity::from(2_u32),
            graded.1,
            "the integer grade is the occurrence count"
        );
    }

    #[test]
    fn a_unit_step_is_eliminated_over_the_toy_alphabet()
    {
        // `equiv_T`: a step that fires and moves nothing is dropped, and the
        // derivation carrying it has the same normal form as the one without.
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let nop = store.insert(nop_cell());
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let real = CellApp {
            cell: f,
            at: at([0]),
        };
        let unit = CellApp {
            cell: nop,
            at: at([1]),
        };
        let bare = alloc::vec![real.clone()];
        let padded = alloc::vec![unit.clone(), real, unit];
        let join = run(&store, &peak, &bare);
        assert_eq!(
            join,
            run(&store, &peak, &padded),
            "the padded derivation reaches the same term"
        );
        let bare_nf = normalized(&store, &peak, &join, &bare);
        let padded_nf = normalized(&store, &peak, &join, &padded);
        assert_eq!(1, padded_nf.schedule.len(), "both unit steps were dropped");
        assert!(
            bool::from(nf_equal(&bare_nf, &padded_nf)),
            "so the padded derivation has the bare one's normal form"
        );
    }

    #[test]
    fn a_reversed_independent_schedule_is_the_canonical_one()
    {
        // The deterministic companion to the shuffle property: the reversed
        // five-step schedule is exactly the fixture whose adjacent-transposition
        // cost the shift suite measures, and it is the case a generator that
        // quietly stopped permuting would leave uncovered.
        let count = RedexCount(5);
        let (store, _f, peak, canonical) = spine_fixture(count);
        let join = run(&store, &peak, &canonical);
        let reversed: Vec<CellApp<ToyAlphabet>> = canonical.iter().rev().cloned().collect();
        assert_ne!(
            canonical, reversed,
            "the reversed schedule is a genuinely different recorded order"
        );
        let canonical_nf = normalized(&store, &peak, &join, &canonical);
        let reversed_nf = normalized(&store, &peak, &join, &reversed);
        assert!(
            bool::from(nf_equal(&canonical_nf, &reversed_nf)),
            "and the quotient identifies the two orders"
        );
        assert_eq!(
            5,
            canonical_nf.schedule.len(),
            "with every primitive retained"
        );
    }

    #[test]
    fn a_layered_derivation_keeps_its_dependent_step_last()
    {
        // A canonical schedule with TWO layers, so the layering is exercised
        // rather than degenerating to one free layer. `c` overlaps nothing, so
        // the two leaf applications are independent of each other and may
        // permute; the root application encloses both, so it depends on both
        // and cannot move ahead of them. Both leaf orders must therefore give
        // one normal form whose last entry is the root step.
        let mut store = CellStore::new();
        let c = store.insert(toy_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero));
        let peak = Toy::add(
            Toy::add(Toy::Zero, Toy::Zero),
            Toy::add(Toy::Zero, Toy::Zero),
        );
        let left = CellApp {
            cell: c,
            at: at([0]),
        };
        let right = CellApp {
            cell: c,
            at: at([1]),
        };
        let root = CellApp {
            cell: c,
            at: at([]),
        };
        let forward = alloc::vec![left.clone(), right.clone(), root.clone()];
        let backward = alloc::vec![right, left, root];
        let join = run(&store, &peak, &forward);
        assert_eq!(Toy::Zero, join, "the three steps collapse the spine");
        assert_eq!(
            join,
            run(&store, &peak, &backward),
            "and the leaf orders agree at this instance"
        );
        let forward_nf = normalized(&store, &peak, &join, &forward);
        let backward_nf = normalized(&store, &peak, &join, &backward);
        assert!(
            bool::from(nf_equal(&forward_nf, &backward_nf)),
            "the two leaf orders are one normal form"
        );
        let cell = store.get(c).expect("the cell is stored");
        let root_address = prim_address(cell, &at([]));
        assert_eq!(
            Some(&root_address),
            forward_nf.schedule.last(),
            "the enclosing application depends on both leaves, so it stays in the later layer"
        );
        assert_eq!(
            3,
            forward_nf.schedule.len(),
            "and all three primitives survive"
        );
        assert_eq!(
            3,
            forward_nf.primitives.len(),
            "one cell at three distinct positions is three distinct primitives, each graded once"
        );
    }

    proptest! {
        // Each case pays two replays per derivation plus a quadratic number of
        // independence questions, so the case count is modest by design.
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// Any permutation of pairwise-independent redexes normalizes to ONE
        /// normal form — the shift quotient doing the work it exists for.
        ///
        /// If the independence relation ever licensed a commutation the
        /// semantics does not have, `normalize` would refuse its own canonical
        /// schedule and this property would fail with the kill-signal
        /// obstruction attached.
        #[test]
        fn a_shuffled_independent_schedule_has_one_normal_form(
            (count, order) in (2_usize ..= 6_usize).prop_flat_map(|count| {
                (
                    Just(count),
                    Just((0 .. count).collect::<Vec<usize>>()).prop_shuffle(),
                )
            })
        )
        {
            let count = RedexCount(count);
            let (store, _f, peak, canonical) = spine_fixture(count);
            let join = run(&store, &peak, &canonical);
            let mut shuffled = Vec::with_capacity(order.len());
            for index in &order {
                let step = canonical.get(*index).expect("the permutation indexes the schedule");
                shuffled.push(step.clone());
            }
            prop_assert_eq!(
                &join,
                &run(&store, &peak, &shuffled),
                "the permuted schedule reaches the same term to begin with"
            );
            let canonical_nf = normalized(&store, &peak, &join, &canonical);
            let shuffled_nf = normalized(&store, &peak, &join, &shuffled);
            prop_assert!(
                bool::from(nf_equal(&canonical_nf, &shuffled_nf)),
                "the two schedules are one normal form"
            );
            prop_assert_eq!(
                canonical_nf.schedule.len(),
                count.0,
                "and no primitive was lost to the quotient"
            );
        }

        /// **The differential.** Every generated NF-equal pair satisfies the
        /// replay oracle — the implication the fast path owes, attacked over
        /// generated pairs rather than chosen ones.
        ///
        /// The `prop_assert!` on the antecedent keeps the property from passing
        /// vacuously: a generator that stopped producing NF-equal pairs would
        /// fail here rather than sail through on an empty hypothesis.
        #[test]
        fn every_nf_equal_pair_is_replay_equivalent(
            (count, order) in (2_usize ..= 6_usize).prop_flat_map(|count| {
                (
                    Just(count),
                    Just((0 .. count).collect::<Vec<usize>>()).prop_shuffle(),
                )
            })
        )
        {
            let count = RedexCount(count);
            let (store, _f, peak, canonical) = spine_fixture(count);
            let join = run(&store, &peak, &canonical);
            let mut shuffled = Vec::with_capacity(order.len());
            for index in &order {
                let step = canonical.get(*index).expect("the permutation indexes the schedule");
                shuffled.push(step.clone());
            }
            let left = tracelet_over(&peak, &join, canonical.clone(), canonical);
            let right = tracelet_over(&peak, &join, shuffled.clone(), shuffled);
            let certified = tracelets_nf_equal(&store, &left, &right);
            prop_assert!(
                bool::from(certified),
                "non-vacuity: the generator produces NF-equal pairs"
            );
            prop_assert!(
                bool::from(replay_equivalent(&left, &right, &store)),
                "NF-EQUAL IMPLIES REPLAY-EQUAL; a failure here is the lane's kill signal"
            );
        }
    }
}
