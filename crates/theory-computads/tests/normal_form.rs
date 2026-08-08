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
//! - [`a_three_layer_derivation_orders_each_layer_by_content_address`] takes
//!   that one layer further, where the depth recurrence stops being a
//!   two-valued flag: a step whose deepest dependence is *not* its nearest one,
//!   a singleton middle layer, and the declared ascending address order inside
//!   a layer.
//! - [`a_repeated_primitive_is_graded_by_multiplicity`] pins the integer
//!   grading at three occurrences — the smallest grade that separates counting
//!   from any bounded stand-in for it.
//! - [`a_unit_step_is_eliminated_over_the_toy_alphabet`] pins `equiv_T`.
//! - [`a_derivation_from_a_different_peak_is_nf_distinct`] pins that the
//!   **boundary** is part of the normal form: an erasing rule carries two
//!   different peaks to one join by one schedule, and the two are two
//!   transformations.
//! - [`a_certificate_that_does_not_replay_is_not_certified`] and
//!   [`a_tracelet_pair_agreeing_only_on_its_first_leg_is_not_certified`] pin
//!   the certificate-level entry point's two negative directions: a refusal is
//!   collapsed to a negative rather than to an acceptance, and the answer is a
//!   conjunction over **both** legs.
//!
//! [`a_shuffled_independent_schedule_has_one_normal_form`]: tests::a_shuffled_independent_schedule_has_one_normal_form
//! [`every_nf_equal_pair_is_replay_equivalent`]: tests::every_nf_equal_pair_is_replay_equivalent
//! [`an_overlapping_pair_keeps_its_recorded_order`]: tests::an_overlapping_pair_keeps_its_recorded_order
//! [`a_reversed_independent_schedule_is_the_canonical_one`]: tests::a_reversed_independent_schedule_is_the_canonical_one
//! [`a_layered_derivation_keeps_its_dependent_step_last`]: tests::a_layered_derivation_keeps_its_dependent_step_last
//! [`a_three_layer_derivation_orders_each_layer_by_content_address`]: tests::a_three_layer_derivation_orders_each_layer_by_content_address
//! [`a_repeated_primitive_is_graded_by_multiplicity`]: tests::a_repeated_primitive_is_graded_by_multiplicity
//! [`a_unit_step_is_eliminated_over_the_toy_alphabet`]: tests::a_unit_step_is_eliminated_over_the_toy_alphabet
//! [`a_derivation_from_a_different_peak_is_nf_distinct`]: tests::a_derivation_from_a_different_peak_is_nf_distinct
//! [`a_certificate_that_does_not_replay_is_not_certified`]: tests::a_certificate_that_does_not_replay_is_not_certified
//! [`a_tracelet_pair_agreeing_only_on_its_first_leg_is_not_certified`]: tests::a_tracelet_pair_agreeing_only_on_its_first_leg_is_not_certified

#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;

    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellApp;
    use gandr_theory_computads::CellId;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::NormalFormObstruction;
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

    /// (drop): `Add(Zero, x) ~> Zero` — an **erasing** rule, whose right-hand
    /// side forgets the metavariable its left-hand side binds.
    ///
    /// It is what makes two *different* peaks reach one join by one schedule,
    /// which is the only way to separate the normal form's boundary from its
    /// factorization.
    fn drop_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))), Toy::Zero)
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
        // One cell at one position, firing THREE times: `equiv_A` collapses the
        // three occurrences into one primitive carrying the integer grade 3,
        // and the schedule still has three entries because each repeat depends
        // on the one before it (same position).
        //
        // Three rather than two is the point. Two is the first grade above the
        // vacuous one, so it cannot separate "count the occurrences" from any
        // implementation that stops counting after the first repeat; three is
        // the smallest grade that can.
        let mut store = CellStore::new();
        let z = store.insert(add_z());
        let peak = Toy::add(
            Toy::Zero,
            Toy::add(Toy::Zero, Toy::add(Toy::Zero, Toy::succ(Toy::Zero))),
        );
        let step = CellApp {
            cell: z,
            at: at([]),
        };
        let path = alloc::vec![step.clone(), step.clone(), step];
        let join = run(&store, &peak, &path);
        assert_eq!(
            Toy::succ(Toy::Zero),
            join,
            "three add-Z steps at the root peel all three frames"
        );
        let normal = normalized(&store, &peak, &join, &path);
        assert_eq!(
            1,
            normal.primitives.len(),
            "the three occurrences are one content-addressed primitive"
        );
        assert_eq!(
            3,
            normal.schedule.len(),
            "and the schedule keeps all three, because a repeat depends on its predecessor"
        );
        let graded = normal
            .primitives
            .values()
            .next()
            .expect("the factorization holds the one primitive");
        assert_eq!(
            PrimMultiplicity::from(3_u32),
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

    #[test]
    fn a_three_layer_derivation_orders_each_layer_by_content_address()
    {
        // THREE layers, which is where the depth recurrence stops being a
        // two-valued flag. `c` overlaps nothing, so dependence here is position
        // containment alone:
        //
        //   inner  @ [1,1]  depends on nothing            -> layer 0
        //   branch @ [0]    depends on nothing            -> layer 0
        //   middle @ [1]    depends on `inner`            -> layer 1
        //   root   @ []     depends on all three          -> layer 2
        //
        // `root`'s NEAREST earlier dependence in the recorded order is
        // `branch`, at layer 0, and its FIRST is `inner`, also at layer 0,
        // while its DEEPEST is `middle`, at layer 1. So a recurrence that takes
        // the nearest dependence, or the first, instead of the maximum over all
        // of them puts `root` in layer 1 beside `middle` — and the assertion
        // below on the *address order inside a layer* is what makes that
        // observable rather than accidental.
        //
        // The last assertion is the one the two-layer fixture cannot make: with
        // two occupants in layer 0, the declared `(depth, address)` order is
        // observable, and it must ascend.
        let mut store = CellStore::new();
        let c = store.insert(toy_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero));
        let peak = Toy::add(
            Toy::add(Toy::Zero, Toy::Zero),
            Toy::add(Toy::Zero, Toy::add(Toy::Zero, Toy::Zero)),
        );
        let inner = CellApp {
            cell: c,
            at: at([1, 1]),
        };
        let middle = CellApp {
            cell: c,
            at: at([1]),
        };
        let branch = CellApp {
            cell: c,
            at: at([0]),
        };
        let root = CellApp {
            cell: c,
            at: at([]),
        };
        let recorded = alloc::vec![inner.clone(), middle.clone(), branch.clone(), root.clone()];
        let join = run(&store, &peak, &recorded);
        assert_eq!(Toy::Zero, join, "the four steps collapse the whole peak");
        let normal = normalized(&store, &peak, &join, &recorded);
        let cell = store.get(c).expect("the cell is stored");
        // NON-VACUITY. The fixture only separates "maximum over dependences"
        // from "nearest" or "first" because the root primitive's content
        // address sorts BEFORE the middle one's: a recurrence that collapsed
        // the two into one layer would therefore emit `root` ahead of `middle`,
        // which cannot fire. Were the two addresses ever to sort the other way
        // the fixture would silently stop distinguishing them, so the ordering
        // is asserted rather than assumed.
        assert!(
            prim_address(cell, &root.at) < prim_address(cell, &middle.at),
            "the fixture needs the root primitive to sort ahead of the middle one"
        );
        assert_eq!(
            4,
            normal.schedule.len(),
            "one cell at four distinct positions is four primitives"
        );
        assert_eq!(
            4,
            normal.primitives.len(),
            "each graded once, so none of them merged"
        );
        assert_eq!(
            Some(&prim_address(cell, &root.at)),
            normal.schedule.get(3_usize),
            "the root application depends on every other, so it is the deepest layer alone"
        );
        assert_eq!(
            Some(&prim_address(cell, &middle.at)),
            normal.schedule.get(2_usize),
            "and the middle layer is the enclosing-but-not-outermost application, alone"
        );
        let layer_zero = normal
            .schedule
            .get(0_usize .. 2_usize)
            .expect("the schedule has four entries");
        let mut expected = alloc::vec![
            prim_address(cell, &inner.at),
            prim_address(cell, &branch.at)
        ];
        expected.sort_unstable();
        assert_eq!(
            expected, layer_zero,
            "layer zero holds exactly the two independent applications, in ASCENDING content-address \
             order — the tie-break the canonical schedule declares"
        );
        // And the layer really is free: permuting its two occupants in the
        // recorded derivation leaves one normal form.
        let permuted = alloc::vec![branch, inner, middle, root];
        assert_eq!(
            join,
            run(&store, &peak, &permuted),
            "the permuted derivation reaches the same term to begin with"
        );
        let permuted_nf = normalized(&store, &peak, &join, &permuted);
        assert!(
            bool::from(nf_equal(&normal, &permuted_nf)),
            "so the two recorded orders are one normal form"
        );
    }

    #[test]
    fn a_derivation_from_a_different_peak_is_nf_distinct()
    {
        // The BOUNDARY is part of the normal form, and this is the fixture that
        // can tell. An erasing rule forgets what it matched, so two different
        // peaks reach one join under one schedule: the factorizations agree
        // primitive for primitive, the schedules agree entry for entry, and the
        // two derivations are still two transformations. A normal-form equality
        // that compared only the factorization and the schedule would identify
        // them, and the replay oracle would not.
        let mut store = CellStore::new();
        let dropper = store.insert(drop_cell());
        let path = alloc::vec![CellApp {
            cell: dropper,
            at: at([]),
        }];
        let near = Toy::add(Toy::Zero, Toy::Zero);
        let far = Toy::add(Toy::Zero, Toy::succ(Toy::Zero));
        let join = Toy::Zero;
        assert_ne!(near, far, "the two peaks are genuinely different terms");
        assert_eq!(join, run(&store, &near, &path), "and both reach one join");
        assert_eq!(join, run(&store, &far, &path), "by the same one step");
        let near_nf = normalized(&store, &near, &join, &path);
        let far_nf = normalized(&store, &far, &join, &path);
        assert_eq!(
            near_nf.primitives, far_nf.primitives,
            "the graded factorizations are identical"
        );
        assert_eq!(
            near_nf.schedule, far_nf.schedule,
            "and so are the canonical schedules"
        );
        assert!(
            !bool::from(nf_equal(&near_nf, &far_nf)),
            "so it is the recorded peak, and only the recorded peak, that keeps them apart"
        );
        let near_cert = tracelet_over(&near, &join, path.clone(), path.clone());
        let far_cert = tracelet_over(&far, &join, path.clone(), path);
        assert!(
            !bool::from(replay_equivalent(&near_cert, &far_cert, &store)),
            "and the replay oracle keeps them apart too — this is a case where a positive would \
             be UNSOUND, not merely imprecise"
        );
        assert!(
            !bool::from(tracelets_nf_equal(&store, &near_cert, &far_cert)),
            "so the certificate-level fast path must decline it"
        );
    }

    #[test]
    fn a_certificate_that_does_not_replay_is_not_certified()
    {
        // The certificate-level entry point collapses EVERY obstruction to a
        // negative — including, deliberately, the two kill-signal refusals.
        // The direction of that collapse is the whole safety of the fast path:
        // a refusal must never read as an acceptance.
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let peak = Toy::succ(Toy::Zero);
        let join = Toy::Zero;
        let fabricated = alloc::vec![CellApp {
            cell: f,
            at: at([0]),
        }];
        assert!(
            matches!(
                normalize(&store, &peak, &join, &fabricated),
                Err(NormalFormObstruction::StepDoesNotFire { .. })
            ),
            "the recorded step names a position carrying no redex, so it has no normal form"
        );
        let certificate = tracelet_over(&peak, &join, fabricated.clone(), fabricated);
        assert!(
            !bool::from(tracelets_nf_equal(&store, &certificate, &certificate)),
            "so the fast path declines it — even against itself, where every structural \
             comparison would succeed"
        );
    }

    #[test]
    fn a_tracelet_pair_agreeing_only_on_its_first_leg_is_not_certified()
    {
        // A tracelet is TWO derivations of one boundary, so the fast path is a
        // conjunction over both legs. Here the `path_a` legs are literally the
        // same derivation while the `path_b` legs are the two orders of an
        // overlapping pair, which the quotient will not merge.
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
        let left = tracelet_over(&peak, &join, forward.clone(), forward.clone());
        let right = tracelet_over(&peak, &join, forward, backward);
        assert!(
            !bool::from(tracelets_nf_equal(&store, &left, &right)),
            "one agreeing leg is not a certificate — both legs have to agree"
        );
        assert!(
            bool::from(replay_equivalent(&left, &right, &store)),
            "and the replay oracle identifies the pair all the same, so the negative is the \
             under-approximation rather than a claim that they differ"
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
