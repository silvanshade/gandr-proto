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
//! - [`two_interleaved_dependence_chains_layer_by_depth_and_not_by_position`]
//!   is the shape a single dependence chain cannot reach: two chains side by
//!   side, where a layering read off recorded *positions* rather than depths
//!   would give two orders of one trace class two different schedules.
//!
//! # The causal-order fixtures
//!
//! The **finite event partial order** the canonical schedule is a linear
//! extension of is its own surface ([`gandr_theory_computads::causal`]), and it
//! is exercised here rather than in a suite of its own because it needs exactly
//! the toy fixtures above: a sequent term has one command position, so every
//! sequent-side derivation is a chain and the order degenerates.
//!
//! The generated shape is a **balanced `Add`-tree**, whose internal nodes are
//! redexes only once their children have collapsed. It is the smallest shape
//! carrying all three things a spine cannot: several layers, several occupants
//! per layer, and dependence edges that are ancestor-descendant rather than
//! all-or-nothing.
//!
//! - [`causal_precedence_is_a_strict_partial_order`] and
//!   [`independence_is_symmetric_and_irreflexive`] assert the order laws over
//!   generated derivations. They are properties rather than fixtures on
//!   purpose: a relation can be irreflexive on every hand-written case and
//!   still fail transitivity on a shape nobody wrote down.
//! - [`events_sharing_a_layer_are_pairwise_concurrent`] is the theorem the
//!   depth recurrence buys — a dependent pair has strictly increasing depth, so
//!   a shared depth is an antichain — and it is what makes a layer a batch that
//!   could fire together rather than a group that happens to sort adjacently.
//! - [`the_layers_concatenate_to_the_canonical_order`] and
//!   [`the_canonical_key_never_ties`] pin the two claims the schedule rests on:
//!   that the grouping is a partition of the same sequence, and that the sort
//!   key is total, without which the canonical form would depend on the
//!   recorded order.
//! - [`an_exchange_witness_replays_to_its_target_order`] and
//!   [`the_canonical_order_is_always_reachable_by_licensed_transpositions`] are
//!   the exchange half: canonicalization stays inside the trace class, and the
//!   evidence is a list of transpositions each of which the independence
//!   relation licensed. The second is the load-bearing one — it says the
//!   canonical key really is a linear extension of the causal order, which was
//!   previously an argument rather than a check.
//! - [`the_dependence_edges_are_the_pairs_the_guard_refuses`] and
//!   [`a_three_layer_derivation_gives_three_layers`] are the deterministic
//!   companions on the three-layer fixture, where the expected edges and layer
//!   contents can be written down in full.
//! - [`an_independent_pair_is_reordered_by_licensed_transpositions`] is the
//!   non-trivial exchange, so the licensing loop is exercised at a known
//!   transposition count rather than only on whatever the generator produces,
//!   and [`a_containment_dependent_pair_refuses_its_transposition`] raises the
//!   exchange kill signal for a pair that is dependent by position
//!   **containment** — a different reason from the sequent side's single
//!   command position.
//! - [`the_order_taken_alone_agrees_with_the_normalizers`] is the anti-drift
//!   check: the order built without a join must be the order the normalizer
//!   layered by, or the crate has two copies of one relation.
//!
//! # The fixtures that need an adversarial alphabet
//!
//! Four of the normal form's `- fails:` modes fire only when an alphabet gives
//! an answer neither shipped alphabet can give, so their witnesses run over the
//! deliberately-broken inhabitants in [`crate::adversarial_alphabet`]. Each of
//! the four also asserts the same derivation over the honest toy alphabet, or
//! the honest premise the lie replaces, so the refusal is attributable to the
//! one lie rather than to the fixture.
//!
//! - [`an_alphabet_that_calls_nesting_incomparable_trips_the_kill_signal`] and
//!   [`a_non_local_term_algebra_trips_the_kill_signal_at_the_join`] are the two
//!   **kill-signal** arms, raised. The second is the sharper one: every
//!   conjunct of the shift guard holds honestly there, and the pair still fails
//!   to commute, because locality of the term algebra is a premise the guard
//!   cannot read.
//! - [`a_withheld_convexity_warrant_empties_the_shift_quotient`] makes the
//!   guard's third conjunct observable end to end for the first time: with the
//!   warrant withheld every pair is dependent and the quotient is empty.
//! - [`two_primitives_sharing_a_content_address_are_refused_rather_than_merged`]
//!   reaches the **collision** arm without a digest collision, by giving a cell
//!   a field the digest cannot see.
//! - [`the_metavariable_seam_is_what_keeps_a_diverging_nested_pair_dependent`]
//!   asserts a reachability *premise* rather than a refusal, and is written to
//!   fail when the premise changes: a nested pair that would diverge under a
//!   transposition stays dependent only because the overlap enumerator counts a
//!   metavariable position as a composition seam.
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
//! [`two_interleaved_dependence_chains_layer_by_depth_and_not_by_position`]: tests::two_interleaved_dependence_chains_layer_by_depth_and_not_by_position
//! [`an_alphabet_that_calls_nesting_incomparable_trips_the_kill_signal`]: tests::an_alphabet_that_calls_nesting_incomparable_trips_the_kill_signal
//! [`a_non_local_term_algebra_trips_the_kill_signal_at_the_join`]: tests::a_non_local_term_algebra_trips_the_kill_signal_at_the_join
//! [`a_withheld_convexity_warrant_empties_the_shift_quotient`]: tests::a_withheld_convexity_warrant_empties_the_shift_quotient
//! [`two_primitives_sharing_a_content_address_are_refused_rather_than_merged`]: tests::two_primitives_sharing_a_content_address_are_refused_rather_than_merged
//! [`the_metavariable_seam_is_what_keeps_a_diverging_nested_pair_dependent`]: tests::the_metavariable_seam_is_what_keeps_a_diverging_nested_pair_dependent
//! [`causal_precedence_is_a_strict_partial_order`]: tests::causal_precedence_is_a_strict_partial_order
//! [`independence_is_symmetric_and_irreflexive`]: tests::independence_is_symmetric_and_irreflexive
//! [`events_sharing_a_layer_are_pairwise_concurrent`]: tests::events_sharing_a_layer_are_pairwise_concurrent
//! [`the_layers_concatenate_to_the_canonical_order`]: tests::the_layers_concatenate_to_the_canonical_order
//! [`the_canonical_key_never_ties`]: tests::the_canonical_key_never_ties
//! [`an_exchange_witness_replays_to_its_target_order`]: tests::an_exchange_witness_replays_to_its_target_order
//! [`the_canonical_order_is_always_reachable_by_licensed_transpositions`]: tests::the_canonical_order_is_always_reachable_by_licensed_transpositions
//! [`the_dependence_edges_are_the_pairs_the_guard_refuses`]: tests::the_dependence_edges_are_the_pairs_the_guard_refuses
//! [`a_three_layer_derivation_gives_three_layers`]: tests::a_three_layer_derivation_gives_three_layers
//! [`the_order_taken_alone_agrees_with_the_normalizers`]: tests::the_order_taken_alone_agrees_with_the_normalizers
//! [`an_independent_pair_is_reordered_by_licensed_transpositions`]: tests::an_independent_pair_is_reordered_by_licensed_transpositions
//! [`a_containment_dependent_pair_refuses_its_transposition`]: tests::a_containment_dependent_pair_refuses_its_transposition

#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;

    use gandr_theory_computads::CausalDepth;
    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellAlphabet;
    use gandr_theory_computads::CellApp;
    use gandr_theory_computads::CellId;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::ConvexityDischarge;
    use gandr_theory_computads::DerivationEvent;
    use gandr_theory_computads::EventIndex;
    use gandr_theory_computads::ExchangeObstruction;
    use gandr_theory_computads::NormalFormObstruction;
    use gandr_theory_computads::Overlap;
    use gandr_theory_computads::PositionOrder;
    use gandr_theory_computads::PrimCert;
    use gandr_theory_computads::PrimId;
    use gandr_theory_computads::PrimMultiplicity;
    use gandr_theory_computads::Tracelet;
    use gandr_theory_computads::TranspositionCount;
    use gandr_theory_computads::event_order;
    use gandr_theory_computads::nf_equal;
    use gandr_theory_computads::normal_form::TraceletNf;
    use gandr_theory_computads::normal_form::normalize;
    use gandr_theory_computads::normalize_certified;
    use gandr_theory_computads::overlaps_between;
    use gandr_theory_computads::prim_address;
    use gandr_theory_computads::replay_equivalent;
    use gandr_theory_computads::rewrite::rewrite_at;
    use gandr_theory_computads::tracelets_nf_equal;
    use proptest::prelude::*;

    use crate::adversarial_alphabet::CollidingAddresses;
    use crate::adversarial_alphabet::IncomparablePositions;
    use crate::adversarial_alphabet::Lying;
    use crate::adversarial_alphabet::NonLocalSplice;
    use crate::adversarial_alphabet::WithheldConvexity;
    use crate::adversarial_alphabet::lying_cell;
    use crate::adversarial_alphabet::reoriented_lying_cell;
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
    ///
    /// Generic over the alphabet so the adversarial inhabitants in
    /// [`crate::adversarial_alphabet`] share it: the fixtures below run the
    /// same recorded paths over a lying alphabet as over the toy one, and a
    /// second copy of this loop would be a second chance to diverge.
    fn run<A>(
        store: &CellStore<A>,
        start: &A::Cmd,
        path: &[CellApp<A>],
    ) -> A::Cmd
    where
        A: CellAlphabet,
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
    ///
    /// Generic over the alphabet for the reason [`run`] is: the fixtures that
    /// expect a normal form over an adversarial alphabet want the same "a
    /// refusal here is the kill signal" failure message as the toy ones.
    fn normalized<A>(
        store: &CellStore<A>,
        peak: &A::Cmd,
        joins_at: &A::Cmd,
        path: &[CellApp<A>],
    ) -> TraceletNf<A>
    where
        A: CellAlphabet,
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

    #[test]
    fn two_interleaved_dependence_chains_layer_by_depth_and_not_by_position()
    {
        // TWO INTERLEAVED CHAINS, which is the shape a single chain cannot
        // reach. `c` overlaps nothing, so dependence is position containment
        // alone:
        //
        //   a @ [0,0] ── enclosed by ── x @ [0]
        //   b @ [1,0] ── enclosed by ── y @ [1]
        //
        // and every cross pair — a/b, a/y, b/x, x/y — is incomparable, hence
        // independent. So the dependence order is two two-element chains side
        // by side, and the causal layering is `{a, b}` then `{x, y}`.
        //
        // What that separates: a recurrence taking the earlier step's POSITION
        // INDEX in the recorded order instead of its depth is still a valid
        // topological layering — it increases strictly along every dependence
        // edge — so it fires, it reaches the join, and every single-chain
        // fixture agrees with it. Here it does not: recorded feet-then-heads
        // left to right it puts the left head at 1 and the right head at 2, and
        // recorded right to left it puts them the other way round, so the two
        // recorded orders of ONE trace class get two different schedules. The
        // depth recurrence gives both the same one.
        let mut store = CellStore::new();
        let cell = store.insert(toy_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero));
        let branch = Toy::add(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero);
        let peak = Toy::add(branch.clone(), branch);
        let left_foot = CellApp {
            cell,
            at: at([0, 0]),
        };
        let left_head = CellApp { cell, at: at([0]) };
        let right_foot = CellApp {
            cell,
            at: at([1, 0]),
        };
        let right_head = CellApp { cell, at: at([1]) };
        // NON-VACUITY, on the dependence shape rather than on the addresses:
        // the fixture is only two chains if these four answers hold.
        assert_eq!(
            PositionOrder::EnclosedBy,
            ToyAlphabet::position_order(&left_foot.at, &left_head.at),
            "the left head encloses the left foot, so the pair is dependent"
        );
        assert_eq!(
            PositionOrder::EnclosedBy,
            ToyAlphabet::position_order(&right_foot.at, &right_head.at),
            "and the right head encloses the right foot"
        );
        assert_eq!(
            PositionOrder::Incomparable,
            ToyAlphabet::position_order(&left_foot.at, &right_foot.at),
            "while the two chains' feet are disjoint"
        );
        assert_eq!(
            PositionOrder::Incomparable,
            ToyAlphabet::position_order(&left_head.at, &right_head.at),
            "and so are their heads"
        );
        let recorded = alloc::vec![
            left_foot.clone(),
            right_foot.clone(),
            left_head.clone(),
            right_head.clone()
        ];
        let swapped = alloc::vec![right_foot, left_foot, right_head, left_head];
        let join = run(&store, &peak, &recorded);
        assert_eq!(
            Toy::add(Toy::Zero, Toy::Zero),
            join,
            "the four steps collapse both branches"
        );
        assert_eq!(
            join,
            run(&store, &peak, &swapped),
            "and the two recorded orders reach one term to begin with"
        );
        let recorded_nf = normalized(&store, &peak, &join, &recorded);
        let swapped_nf = normalized(&store, &peak, &join, &swapped);
        assert!(
            bool::from(nf_equal(&recorded_nf, &swapped_nf)),
            "the two recorded orders are one trace class, so they are one normal form"
        );
        // And the layering is asserted directly as well: layer zero is the two
        // chain feet, layer one the two heads, each in ascending address order.
        let stored = store.get(cell).expect("the cell is stored");
        let mut feet = alloc::vec![
            prim_address(stored, &at([0, 0])),
            prim_address(stored, &at([1, 0]))
        ];
        feet.sort_unstable();
        let mut heads = alloc::vec![
            prim_address(stored, &at([0])),
            prim_address(stored, &at([1]))
        ];
        heads.sort_unstable();
        let expected: Vec<PrimId> = feet.into_iter().chain(heads).collect();
        assert_eq!(
            expected, recorded_nf.schedule,
            "the schedule is layer zero then layer one, each ascending by content address"
        );
    }

    #[test]
    fn an_alphabet_that_calls_nesting_incomparable_trips_the_kill_signal()
    {
        // THE KILL SIGNAL, RAISED. This is the two-layer fixture verbatim, run
        // over an alphabet whose `position_order` answers `Incomparable` for
        // every pair. Nothing else changes: the cell is the same cell, the peak
        // is the same peak, and the recorded path is the same derivation that
        // normalizes cleanly over the toy alphabet.
        //
        // With the enclosing pair reported as commutable, all three
        // applications land in layer zero and the canonical schedule is their
        // content-address order — which does not put the ROOT application last.
        // So the schedule reaches the root while a leaf is still unreduced and
        // the root's redex does not exist yet. `normalize` replays that
        // schedule, watches the step fail to fire, and refuses with the kill
        // signal. This is the witness `ShiftedScheduleDoesNotFire` did not have.
        let mut store: CellStore<Lying<IncomparablePositions>> = CellStore::new();
        let c = store.insert(lying_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero));
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
        let recorded = alloc::vec![left, right, root.clone()];
        let join = run(&store, &peak, &recorded);
        assert_eq!(Toy::Zero, join, "the recorded order collapses the spine");
        // NON-VACUITY. The refusal is `DoesNotFire` rather than
        // `MissesTheJoin` only because the root application's content address
        // sorts first, so the flattened layer-zero order leads with it. Were
        // the digest ever retuned so that it sorted last, the canonical
        // schedule would be a firing one and this fixture would stop reaching
        // the arm — so the ordering is asserted rather than assumed.
        let cell = store.get(c).expect("the cell is stored");
        let root_address = prim_address(cell, &at([]));
        let left_address = prim_address(cell, &at([0]));
        let right_address = prim_address(cell, &at([1]));
        assert!(
            root_address < left_address.max(right_address),
            "the fixture needs the root application NOT to come last in the flattened layer: \
             {root_address:?} {left_address:?} {right_address:?}"
        );
        let refusal = normalize(&store, &peak, &join, &recorded)
            .expect_err("the licensed transposition produces a schedule that cannot fire");
        assert_eq!(
            NormalFormObstruction::ShiftedScheduleDoesNotFire {
                step: Box::new(root)
            },
            refusal,
            "the kill signal names the canonical step that carried no redex"
        );
        // And the same derivation over the honest alphabet normalizes, so the
        // refusal is attributable to the alphabet's one lie.
        let mut honest = CellStore::new();
        let honest_c = honest.insert(toy_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero));
        let honest_path = alloc::vec![
            CellApp {
                cell: honest_c,
                at: at([0]),
            },
            CellApp {
                cell: honest_c,
                at: at([1]),
            },
            CellApp {
                cell: honest_c,
                at: at([]),
            },
        ];
        assert!(
            normalize(&honest, &peak, &join, &honest_path).is_ok(),
            "the honest alphabet keeps the enclosing application last and normalizes"
        );
    }

    #[test]
    fn a_non_local_term_algebra_trips_the_kill_signal_at_the_join()
    {
        // THE SECOND KILL SIGNAL, RAISED — and by a defect the guard cannot see
        // even in principle. Here both guard premises hold honestly: the two
        // positions are genuinely incomparable and the cell pair has genuinely
        // trivial overlap, so the transposition is licensed for exactly the
        // reasons the guard states. The alphabet's splice is what lies: a
        // rewrite at `[i]` also resets `[1-i]`, so the two applications do not
        // commute although nothing about positions or cell contents can say so.
        //
        // Both orders fire, and they reach different terms — which is what
        // separates this arm from the one above. `normalize` replays the
        // canonical schedule, reaches a term other than the recorded join, and
        // refuses with `ShiftedScheduleMissesTheJoin`.
        let mut store: CellStore<Lying<NonLocalSplice>> = CellStore::new();
        let c = store.insert(lying_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero));
        let peak = Toy::add(
            Toy::add(Toy::Zero, Toy::Zero),
            Toy::add(Toy::Zero, Toy::Zero),
        );
        let cell = store.get(c).expect("the cell is stored");
        // NON-VACUITY, part one: the guard's own two premises really do hold.
        assert_eq!(
            PositionOrder::Incomparable,
            <Lying<NonLocalSplice> as CellAlphabet>::position_order(&at([0]), &at([1])),
            "the positions are honestly incomparable"
        );
        assert!(
            overlaps_between((c, cell), (c, cell)).is_empty(),
            "and the cell has honestly trivial overlap with itself"
        );
        let low = CellApp {
            cell: c,
            at: at([0]),
        };
        let high = CellApp {
            cell: c,
            at: at([1]),
        };
        // The canonical schedule of a one-layer factorization is its ascending
        // content-address order, so recording the DESCENDING one makes the
        // canonical schedule the transposition.
        let (first, second) = if prim_address(cell, &low.at) < prim_address(cell, &high.at) {
            (high, low)
        }
        else {
            (low, high)
        };
        let recorded = alloc::vec![first, second];
        let transposed: Vec<CellApp<Lying<NonLocalSplice>>> =
            recorded.iter().rev().cloned().collect();
        let join = run(&store, &peak, &recorded);
        let elsewhere = run(&store, &peak, &transposed);
        // NON-VACUITY, part two: the transposition really does diverge, and it
        // really does fire — a non-firing transposition would reach the other
        // kill-signal arm instead.
        assert_ne!(
            join, elsewhere,
            "the two orders fire and reach different terms"
        );
        let refusal = normalize(&store, &peak, &join, &recorded)
            .expect_err("the licensed transposition reaches a different join");
        assert_eq!(
            NormalFormObstruction::ShiftedScheduleMissesTheJoin {
                reached: Box::new(elsewhere)
            },
            refusal,
            "the kill signal carries the term the canonical schedule reached"
        );
    }

    #[test]
    fn a_withheld_convexity_warrant_empties_the_shift_quotient()
    {
        // THE THIRD CONJUNCT, END TO END. `convexity_discharge` is the
        // alphabet's answer, and both shipped alphabets discharge it for every
        // store — so nothing in the tree could previously tell whether
        // `normalize` asks the alphabet at all, whether it layers under the
        // answer it got, or whether the warrant it records is the one it used.
        //
        // Over an alphabet that withholds the warrant every pair is dependent,
        // because the guard's third conjunct refuses and the normal form reads
        // any refusal as dependence. So the quotient is empty: the canonical
        // schedule is the recorded order, and two orders of a pairwise
        // independent spine — which the toy alphabet identifies — stay apart.
        let count = RedexCount(3);
        let mut store: CellStore<Lying<WithheldConvexity>> = CellStore::new();
        let f = store.insert(lying_cell(Toy::succ(Toy::Zero), Toy::Zero));
        let peak = spine(count);
        let recorded: Vec<CellApp<Lying<WithheldConvexity>>> = spine_positions(count)
            .into_iter()
            .map(|position| CellApp {
                cell: f,
                at: position,
            })
            .collect();
        let reversed: Vec<CellApp<Lying<WithheldConvexity>>> =
            recorded.iter().rev().cloned().collect();
        let join = run(&store, &peak, &recorded);
        assert_eq!(
            join,
            run(&store, &peak, &reversed),
            "the spine's redexes commute semantically, whatever the warrant says"
        );
        let recorded_nf = normalized(&store, &peak, &join, &recorded);
        let reversed_nf = normalized(&store, &peak, &join, &reversed);
        assert_eq!(
            ConvexityDischarge::ReCheckRequired,
            recorded_nf.convexity,
            "the normal form records the warrant it was actually taken under"
        );
        let cell = store.get(f).expect("the cell is stored");
        let expected: Vec<PrimId> = recorded
            .iter()
            .map(|step| prim_address(cell, &step.at))
            .collect();
        assert_eq!(
            expected, recorded_nf.schedule,
            "with no warrant no transposition is licensed, so the schedule is the recorded order"
        );
        assert!(
            !bool::from(nf_equal(&recorded_nf, &reversed_nf)),
            "so the two orders are two normal forms — the quotient is empty here"
        );
        // The contrast: the identical spine over the honest alphabet, which
        // discharges the conjunct, identifies exactly this pair.
        let mut honest = CellStore::new();
        let honest_f = honest.insert(f_cell());
        let honest_recorded: Vec<CellApp<ToyAlphabet>> = spine_positions(count)
            .into_iter()
            .map(|position| CellApp {
                cell: honest_f,
                at: position,
            })
            .collect();
        let honest_reversed: Vec<CellApp<ToyAlphabet>> =
            honest_recorded.iter().rev().cloned().collect();
        let honest_join = run(&honest, &peak, &honest_recorded);
        let honest_nf = normalized(&honest, &peak, &honest_join, &honest_recorded);
        let honest_reversed_nf = normalized(&honest, &peak, &honest_join, &honest_reversed);
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            honest_nf.convexity,
            "the toy alphabet discharges the conjunct"
        );
        assert!(
            bool::from(nf_equal(&honest_nf, &honest_reversed_nf)),
            "and with the warrant in hand the same two orders are one normal form"
        );
    }

    #[test]
    fn two_primitives_sharing_a_content_address_are_refused_rather_than_merged()
    {
        // THE COLLISION ARM, REACHED. The address is an ordering device and
        // nowhere the identity witness, and this is the fixture that can tell:
        // two structurally distinct cells whose difference the digest cannot see
        // are two primitives under one key, and `normalize` declines the normal
        // form instead of merging them.
        //
        // Reaching it needs an alphabet, not a fixture. Within one store a
        // content has exactly one identifier, because `CellStore::insert`
        // deduplicates on structural equality — so over both shipped alphabets
        // two occurrences sharing an address share a primitive and the arm is
        // dead. It stops being dead as soon as some field of a cell is outside
        // the digest's reach, which `CollidingAddresses` arranges legally: the
        // orientation tag hashes to nothing.
        let mut store: CellStore<Lying<CollidingAddresses>> = CellStore::new();
        let faces = (
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::add(Toy::Zero, Toy::succ(Toy::var(ToyNameRef("x")))),
        );
        let given = store.insert(lying_cell(faces.0.clone(), faces.1.clone()));
        let derived = store.insert(reoriented_lying_cell(faces.0, faces.1));
        assert_ne!(
            given, derived,
            "the store holds the two orientations under two identifiers"
        );
        let first = CellApp {
            cell: given,
            at: at([]),
        };
        let second = CellApp {
            cell: derived,
            at: at([]),
        };
        let peak = Toy::add(Toy::Zero, Toy::Zero);
        let recorded = alloc::vec![first.clone(), second.clone()];
        let join = run(&store, &peak, &recorded);
        assert_eq!(
            Toy::add(Toy::Zero, Toy::succ(Toy::succ(Toy::Zero))),
            join,
            "each application wraps the second argument once more, so both fire and both move it"
        );
        // NON-VACUITY: the two occurrences really do share an address, and they
        // really are two different primitives.
        let given_cell = store.get(given).expect("the given cell is stored");
        let derived_cell = store.get(derived).expect("the derived cell is stored");
        let address = prim_address(given_cell, &at([]));
        assert_eq!(
            address,
            prim_address(derived_cell, &at([])),
            "the digest cannot see the orientation the two cells differ in"
        );
        assert_ne!(first, second, "and the two recorded steps are not one step");
        let refusal = normalize(&store, &peak, &join, &recorded)
            .expect_err("two primitives under one address are refused, never merged");
        assert_eq!(
            NormalFormObstruction::ContentAddressCollision {
                address,
                held: Box::new(PrimCert(first)),
                offered: Box::new(PrimCert(second)),
            },
            refusal,
            "the refusal names the shared address and both primitives"
        );
    }

    #[test]
    fn the_metavariable_seam_is_what_keeps_a_diverging_nested_pair_dependent()
    {
        // A REACHABILITY PREMISE, PINNED SO IT FAILS WHEN IT CHANGES. This
        // fixture reaches no refusal, and that is what it asserts.
        //
        // `swap` and `peel` at nested positions both fire in either order and
        // reach DIFFERENT terms, so an independence relation that licensed
        // their transposition would hand `normalize` a firing, diverging
        // canonical schedule. Over an alphabet that calls every position pair
        // incomparable, the only conjunct left standing between the guard and
        // that transposition is the overlap one — and it holds for a reason
        // that is an over-approximation rather than a fact about these two
        // cells: the enumerator treats a metavariable position in a right-hand
        // side as a composition seam, and both cells expose a hole there.
        //
        // So `normalize` keeps the recorded order and returns a normal form.
        // When the enumerator stops counting a bare hole as a seam, this pair
        // becomes independent, the canonical schedule becomes the transposition,
        // and the `Ok` below becomes `ShiftedScheduleMissesTheJoin` — at which
        // point this fixture is the one to convert into that arm's witness over
        // an honest term algebra.
        let mut store: CellStore<Lying<IncomparablePositions>> = CellStore::new();
        let swap = store.insert(lying_cell(
            Toy::add(Toy::var(ToyNameRef("x")), Toy::var(ToyNameRef("y"))),
            Toy::add(Toy::var(ToyNameRef("y")), Toy::var(ToyNameRef("x"))),
        ));
        let peel = store.insert(lying_cell(
            Toy::succ(Toy::var(ToyNameRef("m"))),
            Toy::var(ToyNameRef("m")),
        ));
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::succ(Toy::Zero)));
        let outer = CellApp {
            cell: swap,
            at: at([]),
        };
        let inner = CellApp {
            cell: peel,
            at: at([0]),
        };
        let swap_cell = store.get(swap).expect("swap is stored");
        let peel_cell = store.get(peel).expect("peel is stored");
        // NON-VACUITY, part one: the pair really would diverge under a
        // transposition, in both directions of firing.
        let forward = alloc::vec![outer.clone(), inner.clone()];
        let backward = alloc::vec![inner, outer];
        assert_ne!(
            run(&store, &peak, &forward),
            run(&store, &peak, &backward),
            "the two orders fire and reach different terms"
        );
        // NON-VACUITY, part two: the position conjunct is neutralized, so the
        // overlap conjunct is the whole of what keeps the pair dependent — and
        // it answers because of the hole, not because these two cells interfere
        // at a ground seam.
        assert_eq!(
            PositionOrder::Incomparable,
            <Lying<IncomparablePositions> as CellAlphabet>::position_order(&at([]), &at([0])),
            "this alphabet reports the nesting pair as commutable"
        );
        assert!(
            !overlaps_between((swap, swap_cell), (peel, peel_cell)).is_empty(),
            "and the enumerator answers a composition overlap at the swap's hole"
        );
        assert!(
            !overlaps_between((peel, peel_cell), (swap, swap_cell)).is_empty(),
            "in the other ordered direction too, at the peel's own hole"
        );
        let (descending, ascending) =
            if prim_address(swap_cell, &at([])) < prim_address(peel_cell, &at([0])) {
                (&backward, &forward)
            }
            else {
                (&forward, &backward)
            };
        let join = run(&store, &peak, descending);
        let normal = normalize(&store, &peak, &join, descending)
            .expect("the overlapping pair keeps its recorded order, so the schedule replays");
        let expected: Vec<PrimId> = descending
            .iter()
            .map(|step| {
                let cell = store.get(step.cell).expect("the step names a stored cell");
                prim_address(cell, &step.at)
            })
            .collect();
        assert_eq!(
            expected, normal.schedule,
            "the recorded order survives, although the ascending address order is the other one"
        );
        assert_ne!(
            expected,
            ascending
                .iter()
                .map(|step| {
                    let cell = store.get(step.cell).expect("the step names a stored cell");
                    prim_address(cell, &step.at)
                })
                .collect::<Vec<PrimId>>(),
            "and the two orders really are two different schedules"
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

    /// Height of the balanced `Add`-tree fixture: `height + 1` causal layers.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
    struct TreeHeight(usize);

    /// One generated sort key, permuting a causal layer of the tree fixture.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
    struct LayerSortKey(usize);

    /// The generated tree case: a height, and one sort key per redex so each
    /// causal layer is permuted independently.
    ///
    /// Permuting **within** a layer rather than across the whole derivation is
    /// what keeps every generated case a valid firing order: a layer is an
    /// antichain, so its members may fire in any order, while a deeper node
    /// must fire before the node enclosing it.
    fn tree_case() -> impl Strategy<Value = (TreeHeight, Vec<LayerSortKey>)>
    {
        (0_usize ..= 2_usize).prop_flat_map(|height| {
            let height = TreeHeight(height);
            let redexes = tree_layers(height).iter().fold(0_usize, |running, layer| {
                running.saturating_add(layer.len())
            });
            (
                Just(height),
                proptest::collection::vec((0_usize .. 64_usize).prop_map(LayerSortKey), redexes),
            )
        })
    }

    /// (c): `Add(Zero, Zero) ~> Zero` — a ground rule overlapping nothing, so
    /// dependence between two of its applications is position containment
    /// alone.
    fn c_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero)
    }

    /// The balanced tree of `Add` nodes at the given height.
    ///
    /// Every internal node is a `c`-redex, but only once both of its children
    /// have collapsed to `Zero` — which is what makes the derivation's causal
    /// order a tree rather than an antichain.
    fn tree(height: TreeHeight) -> Toy
    {
        let mut term = Toy::add(Toy::Zero, Toy::Zero);
        for _ in 0 .. height.0 {
            term = Toy::add(term.clone(), term);
        }
        term
    }

    /// The redex positions of [`tree`], grouped into causal layers, deepest
    /// first.
    ///
    /// Layer `k` holds every position of length `height - k`, so the deepest
    /// layer is the widest and the last layer is the root alone.
    fn tree_layers(height: TreeHeight) -> Vec<Vec<ToyPos>>
    {
        let mut layers: Vec<Vec<ToyPos>> = Vec::with_capacity(height.0.saturating_add(1_usize));
        let mut length = height.0;
        loop {
            let mut paths: Vec<Vec<usize>> = alloc::vec![Vec::new()];
            for _ in 0 .. length {
                let mut grown: Vec<Vec<usize>> = Vec::with_capacity(paths.len());
                for path in &paths {
                    for child in 0_usize ..= 1_usize {
                        let mut extended = path.clone();
                        extended.push(child);
                        grown.push(extended);
                    }
                }
                paths = grown;
            }
            layers.push(paths.into_iter().map(at).collect());
            let Some(next) = length.checked_sub(1_usize)
            else {
                break;
            };
            length = next;
        }
        layers
    }

    /// The tree fixture: a store holding `c`, the peak, and the redex positions
    /// grouped into causal layers, deepest first.
    fn tree_fixture(height: TreeHeight) -> (CellStore<ToyAlphabet>, CellId, Toy, Vec<Vec<ToyPos>>)
    {
        let mut store = CellStore::new();
        let c = store.insert(c_cell());
        (store, c, tree(height), tree_layers(height))
    }

    /// A recorded derivation over the tree fixture, firing each layer in turn
    /// and ordering inside a layer by the supplied keys.
    ///
    /// Any within-layer order is a valid firing order, because a layer is an
    /// antichain — which is the property the suite is checking, so the fixture
    /// asserts the derivation replays rather than assuming it.
    fn tree_path(
        cell: CellId,
        layers: Vec<Vec<ToyPos>>,
        keys: &[LayerSortKey],
    ) -> Vec<CellApp<ToyAlphabet>>
    {
        let mut supply = keys.iter();
        let mut recorded: Vec<CellApp<ToyAlphabet>> = Vec::new();
        for layer in layers {
            let mut keyed: Vec<(LayerSortKey, ToyPos)> = layer
                .into_iter()
                .map(|position| (supply.next().copied().unwrap_or_default(), position))
                .collect();
            keyed.sort_by_key(|entry| entry.0);
            for entry in keyed {
                recorded.push(CellApp { cell, at: entry.1 });
            }
        }
        recorded
    }

    /// The four-step, three-layer derivation the deterministic causal fixtures
    /// share: the store, the peak, the join, and the recorded path
    /// `[inner, middle, branch, root]`.
    ///
    /// The dependence order is position containment alone: `inner @ [1,1]` and
    /// `branch @ [0]` depend on nothing, `middle @ [1]` encloses `inner`, and
    /// `root @ []` encloses all three.
    fn three_layer_fixture() -> (CellStore<ToyAlphabet>, Toy, Toy, Vec<CellApp<ToyAlphabet>>)
    {
        let mut store = CellStore::new();
        let c = store.insert(c_cell());
        let peak = Toy::add(
            Toy::add(Toy::Zero, Toy::Zero),
            Toy::add(Toy::Zero, Toy::add(Toy::Zero, Toy::Zero)),
        );
        let recorded = alloc::vec![
            CellApp {
                cell: c,
                at: at([1, 1]),
            },
            CellApp {
                cell: c,
                at: at([1]),
            },
            CellApp {
                cell: c,
                at: at([0]),
            },
            CellApp {
                cell: c,
                at: at([]),
            },
        ];
        let join = run(&store, &peak, &recorded);
        (store, peak, join, recorded)
    }

    #[test]
    fn the_dependence_edges_are_the_pairs_the_guard_refuses()
    {
        // The edges are written down in full, because on this fixture they can
        // be: `c` overlaps nothing, so the independence relation is exactly
        // "the two positions are incomparable" and containment is the whole
        // story. A relation that collapsed to "everything depends on
        // everything earlier" or to "nothing depends on anything" fails here on
        // the first assertion it reaches.
        let (store, peak, _join, recorded) = three_layer_fixture();
        let order = event_order(&store, &peak, &recorded).expect("the derivation replays");
        let inner = EventIndex::from(0_usize);
        let middle = EventIndex::from(1_usize);
        let branch = EventIndex::from(2_usize);
        let root = EventIndex::from(3_usize);
        assert!(
            bool::from(order.depends_directly(middle, inner)),
            "the middle application encloses the inner one"
        );
        assert!(
            !bool::from(order.depends_directly(branch, inner)),
            "the branch application is incomparable with the inner one"
        );
        assert!(
            !bool::from(order.depends_directly(branch, middle)),
            "and with the middle one"
        );
        for earlier in [inner, middle, branch] {
            assert!(
                bool::from(order.depends_directly(root, earlier)),
                "the root application encloses every other"
            );
        }
        assert!(
            bool::from(order.precedes(inner, root)),
            "so the inner application precedes the root one"
        );
        assert!(
            bool::from(order.concurrent(inner, branch)),
            "and the two independent leaves are concurrent"
        );
        assert!(
            !bool::from(order.concurrent(inner, middle)),
            "while a dependent pair is not"
        );
    }

    #[test]
    fn a_three_layer_derivation_gives_three_layers()
    {
        let (store, peak, _join, recorded) = three_layer_fixture();
        let order = event_order(&store, &peak, &recorded).expect("the derivation replays");
        let layers = order.layers();
        assert_eq!(
            3,
            layers.len(),
            "two independent leaves, then the enclosing middle, then the root"
        );
        let sizes: Vec<usize> = layers.iter().map(Vec::len).collect();
        assert_eq!(
            alloc::vec![2_usize, 1_usize, 1_usize],
            sizes,
            "and the widest layer is the deepest one"
        );
        let first = layers.first().expect("there is a first layer");
        assert!(
            first.contains(&EventIndex::from(0_usize))
                && first.contains(&EventIndex::from(2_usize)),
            "the first layer is exactly the two applications depending on nothing"
        );
        assert_eq!(
            Some(&alloc::vec![EventIndex::from(3_usize)]),
            layers.get(2_usize),
            "and the root application sits alone in the deepest layer"
        );
    }

    #[test]
    fn an_independent_pair_is_reordered_by_licensed_transpositions()
    {
        // A NON-TRIVIAL exchange, so the licensing loop is not vacuous: the
        // branch application is independent of both applications it moves past,
        // so bringing it to the front costs two licensed adjacent swaps — and
        // the rest of the order is left exactly as recorded, because the inner
        // and middle applications are dependent and may not be reordered.
        let (store, peak, _join, recorded) = three_layer_fixture();
        let order = event_order(&store, &peak, &recorded).expect("the derivation replays");
        let target = alloc::vec![
            EventIndex::from(2_usize),
            EventIndex::from(0_usize),
            EventIndex::from(1_usize),
            EventIndex::from(3_usize),
        ];
        let witness = order
            .exchange_between(&order.recorded_order(), &target)
            .expect("the branch application is independent of both it passes");
        assert_eq!(
            TranspositionCount::from(2_usize),
            witness.transposition_count(),
            "two adjacent swaps carry it to the front"
        );
        assert_eq!(
            Some(target),
            witness.apply(&order.recorded_order()),
            "and applying them reproduces the target order"
        );
    }

    #[test]
    fn a_containment_dependent_pair_refuses_its_transposition()
    {
        // THE EXCHANGE KILL SIGNAL over the toy alphabet, where dependence is
        // position CONTAINMENT rather than the sequent alphabet's single
        // command position — a different reason for the same refusal.
        let (store, peak, _join, recorded) = three_layer_fixture();
        let order = event_order(&store, &peak, &recorded).expect("the derivation replays");
        let inner = EventIndex::from(0_usize);
        let middle = EventIndex::from(1_usize);
        let refusal = order.exchange_between(&order.recorded_order(), &alloc::vec![
            middle,
            inner,
            EventIndex::from(2_usize),
            EventIndex::from(3_usize)
        ]);
        assert_eq!(
            Err(ExchangeObstruction::DependentTransposition {
                earlier: inner,
                later: middle,
            }),
            refusal,
            "the middle application encloses the inner one, so they do not commute"
        );
    }

    #[test]
    fn the_order_taken_alone_agrees_with_the_normalizers()
    {
        // ANTI-DRIFT. The event order is reachable without a join, and the
        // normalizer layers by the same object rather than by a second copy of
        // the relation. If the two ever diverged the schedules would, and this
        // is where that shows.
        let (store, peak, join, recorded) = three_layer_fixture();
        let order = event_order(&store, &peak, &recorded).expect("the derivation replays");
        let normal = normalized(&store, &peak, &join, &recorded);
        let addresses: Vec<PrimId> = order
            .canonical_order()
            .into_iter()
            .filter_map(|index| order.event(index).map(DerivationEvent::address))
            .collect();
        assert_eq!(
            normal.schedule, addresses,
            "the normal form's schedule is this order's canonical order, flattened"
        );
        let witness = normalize_certified(&store, &peak, &join, &recorded)
            .expect("the derivation normalizes");
        assert_eq!(
            &order,
            witness.event_order(),
            "and the receipt carries that same order"
        );
    }

    proptest! {
        // The generated tree has at most seven events, so the cubic
        // transitivity sweep below is bounded well inside a property run.
        #![proptest_config(ProptestConfig::with_cases(256))]

        /// Causal precedence is a strict partial order: irreflexive,
        /// asymmetric, and transitive, on every generated derivation.
        #[test]
        fn causal_precedence_is_a_strict_partial_order(
            (height, keys) in tree_case()
        )
        {
            let (store, cell, peak, layers) = tree_fixture(height);
            let recorded = tree_path(cell, layers, &keys);
            let join = run(&store, &peak, &recorded);
            prop_assert_eq!(&Toy::Zero, &join, "the tree collapses, so the layering is a firing order");
            let order = event_order(&store, &peak, &recorded)
                .expect("the layered derivation replays, so it has an event order");
            let events = usize::from(order.event_count());
            for left in 0 .. events {
                let left = EventIndex::from(left);
                prop_assert!(
                    !bool::from(order.precedes(left, left)),
                    "precedence is irreflexive"
                );
                for right in 0 .. events {
                    let right = EventIndex::from(right);
                    if bool::from(order.precedes(left, right)) {
                        prop_assert!(
                            !bool::from(order.precedes(right, left)),
                            "precedence is asymmetric"
                        );
                        for far in 0 .. events {
                            let far = EventIndex::from(far);
                            if bool::from(order.precedes(right, far)) {
                                prop_assert!(
                                    bool::from(order.precedes(left, far)),
                                    "precedence is transitive"
                                );
                            }
                        }
                    }
                }
            }
        }

        /// Independence is symmetric and irreflexive, on every generated
        /// derivation.
        ///
        /// Symmetry is a fact about the shift guard rather than about this
        /// module, and the causal layering depends on it: an asymmetric
        /// relation would make a licensed transposition change which pairs
        /// count as dependent, and depths would stop being a property of the
        /// derivation.
        #[test]
        fn independence_is_symmetric_and_irreflexive(
            (height, keys) in tree_case()
        )
        {
            let (store, cell, peak, layers) = tree_fixture(height);
            let recorded = tree_path(cell, layers, &keys);
            let order = event_order(&store, &peak, &recorded)
                .expect("the layered derivation replays, so it has an event order");
            let events = usize::from(order.event_count());
            for left in 0 .. events {
                let left = EventIndex::from(left);
                prop_assert!(
                    !bool::from(order.independent(left, left)),
                    "a step is always dependent on itself"
                );
                for right in 0 .. events {
                    let right = EventIndex::from(right);
                    prop_assert_eq!(
                        bool::from(order.independent(left, right)),
                        bool::from(order.independent(right, left)),
                        "independence does not read the argument order"
                    );
                }
            }
        }

        /// Two events at one depth are causally unordered — the theorem that
        /// makes a layer a batch rather than a coincidence of the sort.
        #[test]
        fn events_sharing_a_layer_are_pairwise_concurrent(
            (height, keys) in tree_case()
        )
        {
            let (store, cell, peak, layers) = tree_fixture(height);
            let recorded = tree_path(cell, layers, &keys);
            let order = event_order(&store, &peak, &recorded)
                .expect("the layered derivation replays, so it has an event order");
            for layer in order.layers() {
                for left in &layer {
                    for right in &layer {
                        if left == right {
                            continue;
                        }
                        prop_assert!(
                            bool::from(order.concurrent(*left, *right)),
                            "a dependent pair has strictly increasing depth, so a shared \
                             depth is an antichain"
                        );
                    }
                }
            }
        }

        /// The layers partition the canonical order, in ascending depth.
        #[test]
        fn the_layers_concatenate_to_the_canonical_order(
            (height, keys) in tree_case()
        )
        {
            let (store, cell, peak, layers) = tree_fixture(height);
            let recorded = tree_path(cell, layers, &keys);
            let order = event_order(&store, &peak, &recorded)
                .expect("the layered derivation replays, so it has an event order");
            let mut flattened: Vec<EventIndex> = Vec::new();
            let mut held: Option<CausalDepth> = None;
            for layer in order.layers() {
                prop_assert!(!layer.is_empty(), "a layer is never empty");
                let first = layer.first().copied().unwrap_or_default();
                let depth = order.depth(first);
                if let (Some(previous), Some(current)) = (held, depth) {
                    prop_assert!(previous < current, "the layers ascend in depth");
                }
                for index in &layer {
                    prop_assert_eq!(
                        depth,
                        order.depth(*index),
                        "every event in one layer shares its depth"
                    );
                }
                held = depth;
                flattened.extend_from_slice(&layer);
            }
            prop_assert_eq!(
                order.canonical_order(),
                flattened,
                "and the layers concatenate to the canonical order"
            );
        }

        /// No two events tie on the canonical sort key.
        ///
        /// Without this the canonical order would depend on the recorded one
        /// through the sort's stability, and the normal form would stop being
        /// canonical.
        #[test]
        fn the_canonical_key_never_ties(
            (height, keys) in tree_case()
        )
        {
            let (store, cell, peak, layers) = tree_fixture(height);
            let recorded = tree_path(cell, layers, &keys);
            let order = event_order(&store, &peak, &recorded)
                .expect("the layered derivation replays, so it has an event order");
            let events = usize::from(order.event_count());
            for left in 0 .. events {
                for right in 0 .. events {
                    if left == right {
                        continue;
                    }
                    let left = EventIndex::from(left);
                    let right = EventIndex::from(right);
                    let same_depth = order.depth(left) == order.depth(right);
                    let same_address = order.event(left).map(DerivationEvent::address)
                        == order.event(right).map(DerivationEvent::address);
                    prop_assert!(
                        !(same_depth && same_address),
                        "two events sharing both key components would make the sort's \
                         stability observable"
                    );
                }
            }
        }

        /// An exchange witness really performs the rearrangement it describes.
        #[test]
        fn an_exchange_witness_replays_to_its_target_order(
            (height, keys) in tree_case()
        )
        {
            let (store, cell, peak, layers) = tree_fixture(height);
            let recorded = tree_path(cell, layers, &keys);
            let order = event_order(&store, &peak, &recorded)
                .expect("the layered derivation replays, so it has an event order");
            let witness = order
                .exchange_to_canonical()
                .expect("the canonical key is a linear extension of the causal order");
            prop_assert_eq!(
                Some(order.canonical_order()),
                witness.apply(&order.recorded_order()),
                "applying the witness to the recorded order gives the canonical one"
            );
            prop_assert_eq!(
                order.recorded_order() == order.canonical_order(),
                usize::from(witness.transposition_count()) == 0_usize,
                "and the witness is empty exactly when the two orders already coincide"
            );
        }

        /// The canonical order is always reachable from the recorded one by
        /// **licensed** adjacent transpositions.
        ///
        /// This is the load-bearing claim canonicalization rests on: the
        /// canonical key is a linear extension of the causal order, so
        /// reordering to it never crosses a dependence edge and the normal form
        /// stays inside the trace class. It used to be an argument; here it is
        /// checked, and every transposition the witness holds is re-asked of
        /// the independence relation rather than taken on the witness's word.
        #[test]
        fn the_canonical_order_is_always_reachable_by_licensed_transpositions(
            (height, keys) in tree_case()
        )
        {
            let (store, cell, peak, layers) = tree_fixture(height);
            let recorded = tree_path(cell, layers, &keys);
            let order = event_order(&store, &peak, &recorded)
                .expect("the layered derivation replays, so it has an event order");
            let witness = order.exchange_to_canonical().expect(
                "THE EXCHANGE KILL SIGNAL: canonicalization required transposing a dependent \
                 pair, so the canonical key is not a linear extension of the causal order",
            );
            let mut current = order.recorded_order();
            for transposition in witness.transpositions() {
                let below = usize::from(transposition.position());
                let above = below.saturating_add(1_usize);
                let lower = current.get(below).copied().unwrap_or_default();
                let upper = current.get(above).copied().unwrap_or_default();
                prop_assert!(
                    bool::from(order.independent(lower, upper)),
                    "every transposition the witness performs swaps an independent pair"
                );
                current.swap(below, above);
            }
            prop_assert_eq!(
                order.canonical_order(),
                current,
                "and the swaps land on the canonical order"
            );
        }
    }
}
