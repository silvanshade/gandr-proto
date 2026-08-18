//! **Earned shift-equivalence** fixtures — the two-redex body whose two
//! readings are one composite
//! (`spec:implementation/circuit-terms.md`,
//! `circuit-terms-question-15` and `circuit-terms-spike-07`).
//!
//! # Why these live here, and on the toy alphabet
//!
//! The target is the surface `cong2` block: two redex nodes whiskered into one
//! `add` frame, whose derived boundaries are tree-shaped, so it lands on a
//! term-shaped store and needs no cell-alphabet growth. The surface route to it
//! is not built yet (the circuit block form parses nowhere), so the block is
//! realized here over the engine's public API, exactly as
//! `tests/composition.rs` realizes its corpus programs — the
//! internal-before-surface rule of `docs/workflow/corpus.md`.
//!
//! The alphabet is the toy one rather than the sequent one for a reason worth
//! stating rather than working around: a sequent command pattern is one cut
//! whose children are a producer and a consumer, so a sequent term has exactly
//! one command position and **cannot** carry two applications at incomparable
//! positions. The toy alphabet nests commands, which is what a circuit-shaped
//! body will do, so it is where the guard has a nonempty extension today. The
//! sequent-side refusals are pinned in the module's own unit tests.
//!
//! # The fixtures
//!
//! - [`the_cong2_cell_pair_has_trivial_overlap`] pins the second conjunct's
//!   input: the two rules the frame whiskers share no seam, so the enumerator's
//!   verdict for the pair is the empty family.
//! - [`the_cong2_pair_earns_its_shift_equivalence_witness`] is the earning
//!   itself — incomparable positions, trivial overlap, the discharge carried as
//!   a certificate rather than swept for.
//! - [`the_cong2_composite_replays_under_both_sequentializations`] is the
//!   confirmation half: the identification is checked by replay, never granted
//!   by the guard alone.
//! - [`a_retargeted_composite_no_longer_replays`] keeps that check falsifiable.
//! - [`an_overlapping_toy_pair_at_disjoint_positions_is_refused`] is the
//!   refusal that matters: a pair whose two orders **do** reach one term is
//!   still refused, because the cells overlap. The guard is not a restatement
//!   of "these two happened to commute".
//! - [`a_nested_toy_pair_is_refused`] refuses the pair whose positions nest.
//! - [`an_adjacent_transposition_schedule_asks_a_quadratic_number_of_questions`]
//!   measures the falsifier the cheapness argument can lose to, rather than
//!   assuming it away.
//!
//! [`the_cong2_cell_pair_has_trivial_overlap`]: tests::the_cong2_cell_pair_has_trivial_overlap
//! [`the_cong2_pair_earns_its_shift_equivalence_witness`]: tests::the_cong2_pair_earns_its_shift_equivalence_witness
//! [`the_cong2_composite_replays_under_both_sequentializations`]: tests::the_cong2_composite_replays_under_both_sequentializations
//! [`a_retargeted_composite_no_longer_replays`]: tests::a_retargeted_composite_no_longer_replays
//! [`an_overlapping_toy_pair_at_disjoint_positions_is_refused`]: tests::an_overlapping_toy_pair_at_disjoint_positions_is_refused
//! [`a_nested_toy_pair_is_refused`]: tests::a_nested_toy_pair_is_refused
//! [`an_adjacent_transposition_schedule_asks_a_quadratic_number_of_questions`]: tests::an_adjacent_transposition_schedule_asks_a_quadratic_number_of_questions

#[cfg(test)]
mod tests
{
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    use gandr_theory_cell_complexes::Cell;
    use gandr_theory_cell_complexes::CellAlphabet as _;
    use gandr_theory_cell_complexes::CellId;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes::ConvexityDischarge;
    use gandr_theory_cell_complexes::PositionOrder;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyAlphabet;
    use gandr_theory_cell_complexes_tools::toy::ToyNameRef;
    use gandr_theory_cell_complexes_tools::toy::ToyPos;
    use gandr_theory_cell_complexes_tools::toy::toy_cell;
    use gandr_theory_coherent_resolutions::CellApp;
    use gandr_theory_coherent_resolutions::overlaps_between;
    use gandr_theory_coherent_resolutions::rewrite::rewrite_at;
    use gandr_theory_deep_inference::ShiftObstruction;
    use gandr_theory_deep_inference::derive_shift_equivalence;

    extern crate alloc;

    /// A toy position from child indices.
    fn at<Steps>(steps: Steps) -> ToyPos
    where
        Steps: IntoIterator<Item = usize>,
    {
        ToyPos(steps.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    /// (f): `Succ(Zero) ~> Zero` — the rule the left redex node carries.
    fn f_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::Zero), Toy::Zero)
    }

    /// (g): `Succ(Succ(Zero)) ~> Zero` — the rule the right redex node carries.
    ///
    /// Both faces are ground and neither right-hand side offers a seam the
    /// other left-hand side unifies with, which is what makes the pair's
    /// overlap family empty — the surface reading "f and g share no ports".
    fn g_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::succ(Toy::Zero)), Toy::Zero)
    }

    /// The `cong2` store and its two rule identifiers.
    fn cong2_store() -> (CellStore<ToyAlphabet>, CellId, CellId)
    {
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let g = store.insert(g_cell());
        (store, f, g)
    }

    /// The `cong2` body — two redex nodes whiskered into one `add` frame.
    ///
    /// `add(f(..), g(..))`, the shape of the surface block's
    /// `*f(-x, +x'); *g(-y, +y'); *add(-x', -y', +z);`, with the two redexes at
    /// the frame's two argument positions.
    fn cong2_body() -> Toy
    {
        Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::succ(Toy::Zero)))
    }

    /// The two applications the `cong2` body whiskers, in block order.
    fn cong2_pair(
        f: CellId,
        g: CellId,
    ) -> (CellApp<ToyAlphabet>, CellApp<ToyAlphabet>)
    {
        (
            CellApp {
                cell: f,
                at: at([0]),
            },
            CellApp {
                cell: g,
                at: at([1]),
            },
        )
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

    /// Fire one recorded step, or fail the test naming it.
    fn fire(
        store: &CellStore<ToyAlphabet>,
        term: &Toy,
        step: &CellApp<ToyAlphabet>,
    ) -> Toy
    {
        let cell = store.get(step.cell).expect("the step names a stored cell");
        rewrite_at(cell, term, &step.at).expect("the step fires at its recorded position")
    }

    /// Run a whole recorded schedule from `start`.
    fn run(
        store: &CellStore<ToyAlphabet>,
        start: &Toy,
        schedule: &[CellApp<ToyAlphabet>],
    ) -> Toy
    {
        let mut current = start.clone();
        for step in schedule {
            current = fire(store, &current, step);
        }
        current
    }

    #[test]
    fn the_cong2_cell_pair_has_trivial_overlap()
    {
        let (store, f, g) = cong2_store();
        let f_cell = store.get(f).expect("f is stored");
        let g_cell = store.get(g).expect("g is stored");
        assert!(
            overlaps_between((f, f_cell), (g, g_cell)).is_empty(),
            "f's right-hand side offers g no seam"
        );
        assert!(
            overlaps_between((g, g_cell), (f, f_cell)).is_empty(),
            "and g's offers f none either, so the pair is trivial in both orders"
        );
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            ToyAlphabet::convexity_discharge(&store),
            "and the store carries the discharge the third conjunct is skipped under"
        );
    }

    #[test]
    fn the_cong2_pair_earns_its_shift_equivalence_witness()
    {
        let (store, f, g) = cong2_store();
        let peak = cong2_body();
        let (first, second) = cong2_pair(f, g);
        assert_eq!(
            PositionOrder::Incomparable,
            ToyAlphabet::position_order(&first.at, &second.at),
            "the two redex nodes sit at the frame's two argument positions"
        );
        let witness = derive_shift_equivalence(&store, &peak, &first, &second)
            .expect("disjoint positions, trivial overlap, discharge applies");
        assert_eq!(peak, witness.peak, "the witness records the body it spans");
        assert_eq!(
            Toy::add(Toy::Zero, Toy::Zero),
            witness.joins_at,
            "both readings reach one composite"
        );
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            witness.convexity,
            "and the witness carries the warrant its convexity conjunct was skipped under"
        );
    }

    #[test]
    fn the_cong2_composite_replays_under_both_sequentializations()
    {
        let (store, f, g) = cong2_store();
        let peak = cong2_body();
        let (first, second) = cong2_pair(f, g);
        let witness = derive_shift_equivalence(&store, &peak, &first, &second)
            .expect("the cong2 pair earns its witness");
        assert!(
            bool::from(witness.replay(&store)),
            "replay confirms the identification instead of the guard asserting it"
        );
        // The identification is not a claim about the recorded order: running
        // either sequentialization by hand lands on the same composite.
        assert_eq!(
            witness.joins_at,
            run(&store, &peak, &witness.first_then_second()),
            "f then g reaches the composite"
        );
        assert_eq!(
            witness.joins_at,
            run(&store, &peak, &witness.second_then_first()),
            "and so does g then f"
        );
    }

    #[test]
    fn a_retargeted_composite_no_longer_replays()
    {
        let (store, f, g) = cong2_store();
        let peak = cong2_body();
        let (first, second) = cong2_pair(f, g);
        let mut witness = derive_shift_equivalence(&store, &peak, &first, &second)
            .expect("the cong2 pair earns its witness");
        witness.joins_at = Toy::Zero;
        assert!(
            !bool::from(witness.replay(&store)),
            "a composite neither reading reaches fails replay, so the check is real"
        );
    }

    #[test]
    fn an_overlapping_toy_pair_at_disjoint_positions_is_refused()
    {
        // The two applications sit at disjoint positions and their two orders
        // reach the SAME term, so nothing about this instance is wrong; the
        // pair is refused anyway, because the enumerator reports the cell pair
        // overlapping. The guard is a property of the cells, not a
        // rediscovery of "these two happened to commute".
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
        let forward = run(&store, &peak, &alloc::vec![first.clone(), second.clone()]);
        let backward = run(&store, &peak, &alloc::vec![second.clone(), first.clone()]);
        assert_eq!(
            forward, backward,
            "the two orders do reach one term at this instance"
        );
        let refusal = derive_shift_equivalence(&store, &peak, &first, &second)
            .expect_err("and the pair is refused all the same");
        let ShiftObstruction::GenuineOverlap { overlap } = refusal
        else {
            panic!("the overlap conjunct is what refuses this pair: {refusal:?}");
        };
        assert_eq!(
            (z, s),
            (overlap.left, overlap.right),
            "the refusal carries an overlap of exactly this cell pair"
        );
        // The seam that makes the pair genuinely interfering is add-S's
        // right-hand side running into add-Z's left-hand side; the enumerator
        // reports it in the other order of the pair, which the guard also asks.
        let z_cell = store.get(z).expect("add-Z is stored");
        let s_cell = store.get(s).expect("add-S is stored");
        assert!(
            !overlaps_between((s, s_cell), (z, z_cell)).is_empty(),
            "add-S's right-hand side runs into add-Z's left-hand side at a seam"
        );
    }

    #[test]
    fn a_nested_toy_pair_is_refused()
    {
        let (store, f, g) = cong2_store();
        let peak = cong2_body();
        let outer = CellApp {
            cell: g,
            at: at([1]),
        };
        let inner = CellApp {
            cell: f,
            at: at([1, 0]),
        };
        let refusal = derive_shift_equivalence(&store, &peak, &outer, &inner)
            .expect_err("one application inside the other is not an adjacent pair");
        assert_eq!(
            ShiftObstruction::ComparablePositions {
                order: PositionOrder::Encloses
            },
            refusal,
            "the position conjunct refuses the nesting"
        );
    }

    #[test]
    fn an_adjacent_transposition_schedule_asks_a_quadratic_number_of_questions()
    {
        // The falsifier the guard's cheapness argument can lose to
        // (`circuit-terms-spike-07`): the per-query cost is linear, but the
        // number of queries a canonical schedule needs is not. Five pairwise
        // disjoint redexes on one spine, scheduled in reverse, are bubbled into
        // canonical order by adjacent transpositions — and every transposition
        // is one independence question.
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let redex = Toy::succ(Toy::Zero);
        let peak = Toy::add(
            redex.clone(),
            Toy::add(
                redex.clone(),
                Toy::add(redex.clone(), Toy::add(redex.clone(), redex)),
            ),
        );
        let canonical: Vec<CellApp<ToyAlphabet>> = alloc::vec![
            CellApp {
                cell: f,
                at: at([0])
            },
            CellApp {
                cell: f,
                at: at([1, 0])
            },
            CellApp {
                cell: f,
                at: at([1, 1, 0])
            },
            CellApp {
                cell: f,
                at: at([1, 1, 1, 0])
            },
            CellApp {
                cell: f,
                at: at([1, 1, 1, 1])
            },
        ];
        let length = canonical.len();
        let reached = run(&store, &peak, &canonical);

        let mut schedule: Vec<CellApp<ToyAlphabet>> = canonical.iter().rev().cloned().collect();
        assert_eq!(
            reached,
            run(&store, &peak, &schedule),
            "the reversed schedule reaches the same term to begin with"
        );
        let mut questions = 0_usize;
        let mut settled = false;
        while !settled {
            settled = true;
            for index in 0 .. length.saturating_sub(1) {
                let next = index.saturating_add(1);
                let left = schedule[index].clone();
                let right = schedule[next].clone();
                if left.at.0 <= right.at.0 {
                    continue;
                }
                let prefix = run(&store, &peak, &schedule[.. index]);
                derive_shift_equivalence(&store, &prefix, &left, &right)
                    .expect("pairwise disjoint redexes earn the witness at every intermediate");
                questions = questions.saturating_add(1);
                schedule.swap(index, next);
                settled = false;
            }
        }

        assert_eq!(canonical, schedule, "the schedule reached canonical order");
        assert_eq!(
            reached,
            run(&store, &peak, &schedule),
            "and the identification held: the canonical schedule reaches the same term"
        );
        // Five steps replay in five rewrites; sorting them cost ten questions.
        // The quadratic side is the number of questions, and it overtakes the
        // linear replay it accelerates — the measurement the argument stands or
        // falls on, taken rather than assumed.
        assert_eq!(5, length, "five steps");
        assert_eq!(
            10, questions,
            "and ten independence questions, which is the reversed schedule's worst case"
        );
    }

    #[test]
    fn a_pair_naming_an_unstored_cell_is_refused()
    {
        let (store, f, _g) = cong2_store();
        let peak = cong2_body();
        let missing = CellId(7);
        let refusal = derive_shift_equivalence(
            &store,
            &peak,
            &CellApp {
                cell: f,
                at: at([0]),
            },
            &CellApp {
                cell: missing,
                at: at([1]),
            },
        )
        .expect_err("an unresolvable identifier is refused");
        assert_eq!(
            ShiftObstruction::<ToyAlphabet>::UnknownCell { cell: missing },
            refusal,
            "the refusal names the identifier that resolved to nothing"
        );
    }

    /// Keep the boxed-payload variants reachable in this suite's imports.
    #[test]
    fn a_step_that_does_not_fire_carries_the_step()
    {
        let (store, f, g) = cong2_store();
        // A body whose right argument is not a g-redex: the guard passes and
        // the instance check declines, carrying the step.
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let (first, second) = cong2_pair(f, g);
        let refusal = derive_shift_equivalence(&store, &peak, &first, &second)
            .expect_err("g has no redex at the frame's right argument");
        assert_eq!(
            ShiftObstruction::StepDoesNotFire {
                step: Box::new(second)
            },
            refusal,
            "the refusal carries the step that did not fire"
        );
    }
}
