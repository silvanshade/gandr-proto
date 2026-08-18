//! **Differential fixtures for the polarized-footprint prototype** — where the
//! instance-derived polarized test and the decided shift guard disagree, and
//! why.
//!
//! The prototype (`gandr_theory_deep_inference::footprint`) is inert: nothing
//! consumes it, and the guard's behaviour is unchanged. This suite is the
//! measurement the template-games read asked for — does a polarized
//! independence test license a strictly larger commuting class than the
//! incomparable-positions conjunct? — run as a comparison against
//! [`derive_shift_equivalence`] on the same pairs, with a third column the
//! read did not ask for and the answer needs: whether the pair *actually*
//! commutes with the addresses as recorded.
//!
//! The alphabet is the toy one for the same reason the shift fixtures use it:
//! it is the only alphabet in the tree whose terms nest commands, so it is the
//! only one where two applications in one term exist at all. Over the sequent
//! alphabet the prototype degenerates to "the redex is written", pinned in the
//! module's own unit test.
//!
//! # The four cells of the comparison
//!
//! - **licensed by both** — a pair at incomparable positions whose cells do not
//!   overlap ([`the_cong2_pair_is_licensed_by_both_tests`]);
//! - **footprint only** — three genuinely distinct causes, each its own
//!   fixture: a read/read overlap at *one* address
//!   ([`two_root_rules_sharing_only_a_read_node_are_independent`]), a nested
//!   pair inside a carried frame
//!   ([`a_redex_inside_a_carried_frame_is_licensed`]), and two pairs the
//!   guard's *cell-keyed* overlap conjunct refuses although the *instance* is
//!   disjoint — one whose overlap family also carries a real composition seam
//!   ([`a_schematic_overlap_does_not_survive_as_an_instance_overlap`]) and one
//!   whose family is nothing but metavariable seams
//!   ([`a_hole_seam_refusal_is_the_enumerator_gap_and_not_polarization`]);
//! - **guard only** — asserted **empty** over the whole table
//!   ([`the_guard_licenses_nothing_the_polarized_test_refuses`]), which is the
//!   containment the "strictly larger" claim needs;
//! - **neither** — a pair that genuinely does not commute
//!   ([`the_polarized_test_refuses_a_pair_that_does_not_commute`]).
//!
//! # The caveat the measurement must carry
//!
//! Both cell-keyed refusals above are *reported* at a **metavariable position**
//! the overlap enumerator counts as a composition seam, which is recorded as
//! the independence relation's own gap and is not polarization's win. Reported
//! is not the same as caused, and the suite now pins the difference on both
//! rows rather than narrating it: the hole-seam pair's **whole** overlap family
//! is bare metavariables, so repairing the enumerator would stop the guard
//! refusing it; the schematic pair's family also contains a genuine composition
//! seam — add-S's right-hand side running into add-Z's left-hand side at a
//! constructor position — so that refusal survives the repair and the row is
//! cell-keying against instance-keying, exactly as its name says.
//!
//! **So one polarized-only row is attributable to the enumerator gap, not
//! two.** The earlier reading of this suite counted both, because the
//! schematic fixture asserted only the refusing overlap's *kind* while its
//! prose described a different overlap of the same family.
//!
//! # What the suite pins besides the four cells
//!
//! Two invariants the comparison rests on, asserted over the fixture family
//! rather than at one fixture: every footprint's three classes are pairwise
//! disjoint, cover the match image exactly, and keep the alphabet's own
//! enumeration order
//! ([`every_footprint_partitions_exactly_the_addresses_its_redex_covers`]);
//! and a pair colliding at several addresses reports the earliest one its scan
//! reaches, so the verdict is a function of the input
//! ([`a_pair_colliding_several_ways_reports_the_earliest_address_it_scans`]).
//!
//! Addresses in this carrier **move**, and the source's do not. A rule that
//! relocates its hole instance therefore reports the vacated addresses as
//! written and is refused, even though the two redexes commute once the inner
//! one is re-addressed
//! ([`a_relocated_frame_is_refused_because_addresses_move`]). That refusal is
//! conservative rather than wrong, and it names the debt an adoption would take
//! on: a residual map on positions, which [`CellApp`] has no room for today.
//!
//! [`the_cong2_pair_is_licensed_by_both_tests`]: tests::the_cong2_pair_is_licensed_by_both_tests
//! [`two_root_rules_sharing_only_a_read_node_are_independent`]: tests::two_root_rules_sharing_only_a_read_node_are_independent
//! [`a_redex_inside_a_carried_frame_is_licensed`]: tests::a_redex_inside_a_carried_frame_is_licensed
//! [`a_schematic_overlap_does_not_survive_as_an_instance_overlap`]: tests::a_schematic_overlap_does_not_survive_as_an_instance_overlap
//! [`a_hole_seam_refusal_is_the_enumerator_gap_and_not_polarization`]: tests::a_hole_seam_refusal_is_the_enumerator_gap_and_not_polarization
//! [`the_guard_licenses_nothing_the_polarized_test_refuses`]: tests::the_guard_licenses_nothing_the_polarized_test_refuses
//! [`the_polarized_test_refuses_a_pair_that_does_not_commute`]: tests::the_polarized_test_refuses_a_pair_that_does_not_commute
//! [`a_relocated_frame_is_refused_because_addresses_move`]: tests::a_relocated_frame_is_refused_because_addresses_move
//! [`every_footprint_partitions_exactly_the_addresses_its_redex_covers`]: tests::every_footprint_partitions_exactly_the_addresses_its_redex_covers
//! [`a_pair_colliding_several_ways_reports_the_earliest_address_it_scans`]: tests::a_pair_colliding_several_ways_reports_the_earliest_address_it_scans
//! [`derive_shift_equivalence`]: gandr_theory_deep_inference::derive_shift_equivalence
//! [`CellApp`]: gandr_theory_coherent_resolutions::CellApp

#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;

    use gandr_theory_cell_complexes::Cell;
    use gandr_theory_cell_complexes::CellAlphabet as _;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes::PositionOrder;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyAlphabet;
    use gandr_theory_cell_complexes_tools::toy::ToyNameRef;
    use gandr_theory_cell_complexes_tools::toy::ToyPos;
    use gandr_theory_cell_complexes_tools::toy::toy_cell;
    use gandr_theory_coherent_resolutions::CellApp;
    use gandr_theory_coherent_resolutions::OverlapKind;
    use gandr_theory_coherent_resolutions::overlaps_between;
    use gandr_theory_coherent_resolutions::rewrite::rewrite_at;
    use gandr_theory_deep_inference::FootprintIndependence;
    use gandr_theory_deep_inference::ShiftObstruction;
    use gandr_theory_deep_inference::derive_shift_equivalence;
    use gandr_theory_deep_inference::footprint_independence;
    use gandr_theory_deep_inference::match_footprint;

    extern crate alloc;

    /// Which of the two independence tests license a pair at one peak.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum LicensedBy
    {
        /// Both the decided guard and the polarized prototype license it.
        Both,
        /// Only the polarized prototype licenses it.
        FootprintOnly,
        /// Only the decided guard licenses it — the cell the containment claim
        /// says must stay empty.
        GuardOnly,
        /// Neither test licenses it.
        Neither,
    }

    /// Whether the two recorded orders both fire from a peak and reach one
    /// term — the ground truth the two verdicts are measured against.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Commutation
    {
        /// Both orders fire and reach the same term.
        Commutes,
        /// An order fails to fire, or the two reach different terms.
        Diverges,
    }

    /// One row of the differential table.
    struct Row
    {
        /// The store the pair's cells live in.
        store: CellStore<ToyAlphabet>,
        /// The term both orders start from.
        peak: Toy,
        /// The application recorded first.
        left: CellApp<ToyAlphabet>,
        /// The application recorded second.
        right: CellApp<ToyAlphabet>,
        /// Which tests license the pair.
        licensed: LicensedBy,
        /// Whether the pair actually commutes as recorded.
        commutation: Commutation,
    }

    /// A toy position from child indices.
    fn at<Steps>(steps: Steps) -> ToyPos
    where
        Steps: IntoIterator<Item = usize>,
    {
        ToyPos(steps.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    /// Addresses in a deterministic order, so a footprint class can be
    /// compared entrywise regardless of the alphabet's enumeration order.
    fn sorted(mut positions: Vec<ToyPos>) -> Vec<ToyPos>
    {
        positions.sort_by(|left, right| left.0.cmp(&right.0));
        positions
    }

    /// (f): `Succ(Zero) ~> Zero`.
    fn f_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::Zero), Toy::Zero)
    }

    /// (g): `Succ(Succ(Zero)) ~> Zero`.
    fn g_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::succ(Toy::Zero)), Toy::Zero)
    }

    /// (bump-left): `Add(Zero, y) ~> Add(Succ(Zero), y)`.
    ///
    /// The root `Add` is matched and left standing, the left child is
    /// rewritten, and `y` is carried through untouched and unmoved — the three
    /// footprint classes in one cell.
    fn bump_left() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("y"))),
            Toy::add(Toy::succ(Toy::Zero), Toy::var(ToyNameRef("y"))),
        )
    }

    /// (bump-left-twice): `Add(Zero, y) ~> Add(Succ(Succ(Zero)), y)`.
    fn bump_left_twice() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("y"))),
            Toy::add(Toy::succ(Toy::succ(Toy::Zero)), Toy::var(ToyNameRef("y"))),
        )
    }

    /// (bump-right): `Add(y, Zero) ~> Add(y, Succ(Zero))` — bump-left's mirror.
    fn bump_right() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::var(ToyNameRef("y")), Toy::Zero),
            Toy::add(Toy::var(ToyNameRef("y")), Toy::succ(Toy::Zero)),
        )
    }

    /// (relocate): `Add(Zero, y) ~> Succ(y)` — the hole instance changes
    /// address.
    fn relocate() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("y"))),
            Toy::succ(Toy::var(ToyNameRef("y"))),
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

    /// (peel): `Succ(Succ(x)) ~> Succ(x)` — a right-hand side exposing a hole.
    fn peel() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::succ(Toy::succ(Toy::var(ToyNameRef("x")))),
            Toy::succ(Toy::var(ToyNameRef("x"))),
        )
    }

    /// (add-zz): `Add(Zero, Zero) ~> Zero` — ground on both faces.
    fn add_zz() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero)
    }

    /// Fire one recorded step, or report that it does not fire.
    fn fire(
        store: &CellStore<ToyAlphabet>,
        term: &Toy,
        step: &CellApp<ToyAlphabet>,
    ) -> Option<Toy>
    {
        let cell = store.get(step.cell)?;
        rewrite_at(cell, term, &step.at)
    }

    /// Whether the two recorded orders both fire from `peak` and reach one
    /// term.
    fn commutation(
        store: &CellStore<ToyAlphabet>,
        peak: &Toy,
        left: &CellApp<ToyAlphabet>,
        right: &CellApp<ToyAlphabet>,
    ) -> Commutation
    {
        let forward = fire(store, peak, left).and_then(|term| fire(store, &term, right));
        let backward = fire(store, peak, right).and_then(|term| fire(store, &term, left));
        match (forward, backward) {
            | (Some(first), Some(second)) if first == second => Commutation::Commutes,
            | _ => Commutation::Diverges,
        }
    }

    /// The polarized prototype's verdict for a pair at one peak.
    fn polarized(
        store: &CellStore<ToyAlphabet>,
        peak: &Toy,
        left: &CellApp<ToyAlphabet>,
        right: &CellApp<ToyAlphabet>,
    ) -> FootprintIndependence<ToyAlphabet>
    {
        let Ok(first) = match_footprint(store, peak, left)
        else {
            return FootprintIndependence::WriteWrite {
                position: left.at.clone(),
            };
        };
        let Ok(second) = match_footprint(store, peak, right)
        else {
            return FootprintIndependence::WriteWrite {
                position: right.at.clone(),
            };
        };
        footprint_independence(&first, &second)
    }

    /// Which of the two tests license a pair at one peak.
    fn licensed_by(
        store: &CellStore<ToyAlphabet>,
        peak: &Toy,
        left: &CellApp<ToyAlphabet>,
        right: &CellApp<ToyAlphabet>,
    ) -> LicensedBy
    {
        let guard = derive_shift_equivalence(store, peak, left, right).is_ok();
        let polarized = polarized(store, peak, left, right) == FootprintIndependence::Independent;
        match (guard, polarized) {
            | (true, true) => LicensedBy::Both,
            | (false, true) => LicensedBy::FootprintOnly,
            | (true, false) => LicensedBy::GuardOnly,
            | (false, false) => LicensedBy::Neither,
        }
    }

    /// The whole differential table, one row per fixture.
    fn table() -> Vec<Row>
    {
        let mut rows = Vec::new();
        // Licensed by both: the cong2 pair.
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let g = store.insert(g_cell());
        rows.push(Row {
            store,
            peak: Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::succ(Toy::Zero))),
            left: CellApp {
                cell: f,
                at: at([0]),
            },
            right: CellApp {
                cell: g,
                at: at([1]),
            },
            licensed: LicensedBy::Both,
            commutation: Commutation::Commutes,
        });
        // Footprint only: a read/read overlap at one address.
        let mut store = CellStore::new();
        let left_bump = store.insert(bump_left());
        let right_bump = store.insert(bump_right());
        rows.push(Row {
            store,
            peak: Toy::add(Toy::Zero, Toy::Zero),
            left: CellApp {
                cell: left_bump,
                at: at([]),
            },
            right: CellApp {
                cell: right_bump,
                at: at([]),
            },
            licensed: LicensedBy::FootprintOnly,
            commutation: Commutation::Commutes,
        });
        // Footprint only: a redex inside a carried frame.
        let mut store = CellStore::new();
        let bump = store.insert(bump_left());
        let f = store.insert(f_cell());
        rows.push(Row {
            store,
            peak: Toy::add(Toy::Zero, Toy::succ(Toy::Zero)),
            left: CellApp {
                cell: bump,
                at: at([]),
            },
            right: CellApp {
                cell: f,
                at: at([1]),
            },
            licensed: LicensedBy::FootprintOnly,
            commutation: Commutation::Commutes,
        });
        // Footprint only: a cell-keyed overlap on a real composition seam that
        // the instance does not realize.
        let mut store = CellStore::new();
        let zero_rule = store.insert(add_z());
        let succ_rule = store.insert(add_s());
        rows.push(Row {
            store,
            peak: Toy::add(
                Toy::add(Toy::Zero, Toy::succ(Toy::Zero)),
                Toy::add(Toy::succ(Toy::Zero), Toy::Zero),
            ),
            left: CellApp {
                cell: zero_rule,
                at: at([0]),
            },
            right: CellApp {
                cell: succ_rule,
                at: at([1]),
            },
            licensed: LicensedBy::FootprintOnly,
            commutation: Commutation::Commutes,
        });
        // Footprint only: a cell-keyed overlap whose only seam is a hole.
        let mut store = CellStore::new();
        let peel_rule = store.insert(peel());
        let ground = store.insert(add_zz());
        rows.push(Row {
            store,
            peak: Toy::add(
                Toy::succ(Toy::succ(Toy::Zero)),
                Toy::add(Toy::Zero, Toy::Zero),
            ),
            left: CellApp {
                cell: peel_rule,
                at: at([0]),
            },
            right: CellApp {
                cell: ground,
                at: at([1]),
            },
            licensed: LicensedBy::FootprintOnly,
            commutation: Commutation::Commutes,
        });
        // Neither: two rules at one address, one destroying what the other
        // matches and preserves.
        let mut store = CellStore::new();
        let ground = store.insert(add_zz());
        let bump = store.insert(bump_left());
        rows.push(Row {
            store,
            peak: Toy::add(Toy::Zero, Toy::Zero),
            left: CellApp {
                cell: ground,
                at: at([]),
            },
            right: CellApp {
                cell: bump,
                at: at([]),
            },
            licensed: LicensedBy::Neither,
            commutation: Commutation::Diverges,
        });
        // Neither: two rules rewriting one child from one root.
        let mut store = CellStore::new();
        let once = store.insert(bump_left());
        let twice = store.insert(bump_left_twice());
        rows.push(Row {
            store,
            peak: Toy::add(Toy::Zero, Toy::Zero),
            left: CellApp {
                cell: once,
                at: at([]),
            },
            right: CellApp {
                cell: twice,
                at: at([]),
            },
            licensed: LicensedBy::Neither,
            commutation: Commutation::Diverges,
        });
        // Neither: a relocating rule whose frame carries the other's redex.
        let mut store = CellStore::new();
        let mover = store.insert(relocate());
        let f = store.insert(f_cell());
        rows.push(Row {
            store,
            peak: Toy::add(Toy::Zero, Toy::succ(Toy::Zero)),
            left: CellApp {
                cell: mover,
                at: at([]),
            },
            right: CellApp {
                cell: f,
                at: at([1]),
            },
            licensed: LicensedBy::Neither,
            commutation: Commutation::Diverges,
        });
        rows
    }

    #[test]
    fn a_ground_redex_writes_its_whole_match_image()
    {
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let footprint = match_footprint(&store, &peak, &CellApp {
            cell: f,
            at: at([0]),
        })
        .expect("f fires at the frame's left argument");
        assert_eq!(
            alloc::vec![at([0]), at([0, 0])],
            sorted(footprint.written),
            "a ground rule matches and destroys every node it covers"
        );
        assert!(
            footprint.read.is_empty(),
            "nothing under a ground redex survives to be read-only"
        );
        assert!(
            footprint.framed.is_empty(),
            "and a ground rule has no hole instance to carry"
        );
    }

    #[test]
    fn a_preserved_root_is_read_and_its_hole_is_framed()
    {
        // The three classes, separated pointwise by one application: the root
        // `Add` is matched and survives (read), the left child is rewritten
        // (written), and `y`'s instance is carried without being inspected
        // (framed) — so it is not in the footprint at all.
        let mut store = CellStore::new();
        let bump = store.insert(bump_left());
        let peak = Toy::add(Toy::Zero, Toy::succ(Toy::Zero));
        let footprint = match_footprint(&store, &peak, &CellApp {
            cell: bump,
            at: at([]),
        })
        .expect("bump-left fires at the root");
        assert_eq!(
            alloc::vec![at([])],
            sorted(footprint.read),
            "the matched root survives the rewrite as the same node"
        );
        assert_eq!(
            alloc::vec![at([0])],
            sorted(footprint.written),
            "and the child it matched as Zero does not"
        );
        assert_eq!(
            alloc::vec![at([1]), at([1, 0])],
            sorted(footprint.framed),
            "the hole instance is carried, so it is frame and not footprint"
        );
    }

    #[test]
    fn two_root_rules_sharing_only_a_read_node_are_independent()
    {
        // The headline: two applications at the SAME address. The guard refuses
        // them at its very first conjunct, and they commute — because their
        // only shared address is one both of them merely read.
        let mut store = CellStore::new();
        let left_bump = store.insert(bump_left());
        let right_bump = store.insert(bump_right());
        let peak = Toy::add(Toy::Zero, Toy::Zero);
        let left = CellApp {
            cell: left_bump,
            at: at([]),
        };
        let right = CellApp {
            cell: right_bump,
            at: at([]),
        };
        let first = match_footprint(&store, &peak, &left).expect("bump-left fires");
        let second = match_footprint(&store, &peak, &right).expect("bump-right fires");
        assert_eq!(
            alloc::vec![at([])],
            sorted(first.read.clone()),
            "bump-left reads the root"
        );
        assert_eq!(
            alloc::vec![at([])],
            sorted(second.read.clone()),
            "and so does bump-right, so the overlap is read/read"
        );
        assert_eq!(
            FootprintIndependence::Independent,
            footprint_independence(&first, &second),
            "read/read overlap is not a collision — the polarized reading's whole content"
        );
        assert_eq!(
            Err(ShiftObstruction::ComparablePositions {
                order: PositionOrder::Same
            }),
            derive_shift_equivalence(&store, &peak, &left, &right),
            "and the guard refuses at its first conjunct, one address being one position"
        );
        assert_eq!(
            Commutation::Commutes,
            commutation(&store, &peak, &left, &right),
            "the extra licence is not bogus: both orders reach one term"
        );
        assert_eq!(
            Some(Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero))),
            fire(&store, &peak, &left).and_then(|term| fire(&store, &term, &right)),
            "and the term they reach is the one both children bumped"
        );
    }

    #[test]
    fn a_redex_inside_a_carried_frame_is_licensed()
    {
        // The classic term-rewriting fact the position conjunct cannot see: a
        // redex inside the substitution part of another redex commutes with it.
        // The polarized test sees it because the frame is not in the footprint.
        let mut store = CellStore::new();
        let bump = store.insert(bump_left());
        let f = store.insert(f_cell());
        let peak = Toy::add(Toy::Zero, Toy::succ(Toy::Zero));
        let outer = CellApp {
            cell: bump,
            at: at([]),
        };
        let inner = CellApp {
            cell: f,
            at: at([1]),
        };
        assert_eq!(
            FootprintIndependence::Independent,
            polarized(&store, &peak, &outer, &inner),
            "the inner redex sits in the outer's frame, which the footprint excludes"
        );
        assert_eq!(
            Err(ShiftObstruction::ComparablePositions {
                order: PositionOrder::Encloses
            }),
            derive_shift_equivalence(&store, &peak, &outer, &inner),
            "the guard refuses the nesting without asking what the nesting is inside"
        );
        assert_eq!(
            Commutation::Commutes,
            commutation(&store, &peak, &outer, &inner),
            "and the pair commutes, addresses and all"
        );
    }

    #[test]
    fn a_schematic_overlap_does_not_survive_as_an_instance_overlap()
    {
        // The guard's second conjunct is CELL-keyed: it asks whether the two
        // rules could ever interfere, over every instance. The polarized test
        // is INSTANCE-keyed. On a peak where the two matches are disjoint
        // subtrees, the schematic overlap buys nothing and the pair commutes.
        let mut store = CellStore::new();
        let zero_rule = store.insert(add_z());
        let succ_rule = store.insert(add_s());
        let peak = Toy::add(
            Toy::add(Toy::Zero, Toy::succ(Toy::Zero)),
            Toy::add(Toy::succ(Toy::Zero), Toy::Zero),
        );
        let left = CellApp {
            cell: zero_rule,
            at: at([0]),
        };
        let right = CellApp {
            cell: succ_rule,
            at: at([1]),
        };
        let refusal = derive_shift_equivalence(&store, &peak, &left, &right)
            .expect_err("the cell pair overlaps, so the guard refuses");
        let ShiftObstruction::GenuineOverlap { overlap } = refusal
        else {
            panic!("the overlap conjunct is what refuses this pair: {refusal:?}");
        };
        assert_eq!(
            OverlapKind::Composition,
            overlap.kind,
            "the enumerator reports a composition seam for this pair"
        );
        // Attribution, pinned rather than narrated. The seam the guard refuses
        // AT is add-Z's bare right-hand side, which is the recorded enumerator
        // gap and would not survive its repair.
        let zero_cell = store.get(zero_rule).expect("the zero rule is stored");
        let succ_cell = store.get(succ_rule).expect("the succ rule is stored");
        assert_eq!(zero_rule, overlap.left, "the reported seam is add-Z's");
        assert_eq!(
            Some(Toy::var(ToyNameRef("x"))),
            ToyAlphabet::subterm_cmd_at(&zero_cell.rhs, &overlap.seam),
            "and it addresses a bare metavariable, as the hole-seam row's does"
        );
        // What separates this row from the hole-seam one: the pair's family
        // ALSO carries a genuine composition seam — add-S's right-hand side
        // running into add-Z's left-hand side at a constructor position — so
        // the guard refuses this pair on real grounds too, and the refusal
        // survives an enumerator repair. Only the instance is disjoint.
        let genuine = overlaps_between((succ_rule, succ_cell), (zero_rule, zero_cell))
            .into_iter()
            .find(|candidate| {
                ToyAlphabet::subterm_cmd_at(&succ_cell.rhs, &candidate.seam)
                    == Some(Toy::add(
                        Toy::var(ToyNameRef("m")),
                        Toy::var(ToyNameRef("n")),
                    ))
            })
            .expect("add-S's right-hand side runs into add-Z's left-hand side");
        assert_eq!(
            OverlapKind::Composition,
            genuine.kind,
            "that seam is a composition overlap and not a critical pair"
        );
        assert_eq!(
            FootprintIndependence::Independent,
            polarized(&store, &peak, &left, &right),
            "and this instance's two match images do not meet at any address"
        );
        assert_eq!(
            Commutation::Commutes,
            commutation(&store, &peak, &left, &right),
            "the pair commutes here, as the shift suite already records"
        );
    }

    #[test]
    fn a_hole_seam_refusal_is_the_enumerator_gap_and_not_polarization()
    {
        // Attribution, not celebration: this row's guard refusal is the
        // recorded enumerator defect — a metavariable position counted as a
        // composition seam — so it must not be scored as a win for the
        // polarized reading. The assertion pins the seam as a bare hole.
        let mut store = CellStore::new();
        let peel_rule = store.insert(peel());
        let ground = store.insert(add_zz());
        let peak = Toy::add(
            Toy::succ(Toy::succ(Toy::Zero)),
            Toy::add(Toy::Zero, Toy::Zero),
        );
        let left = CellApp {
            cell: peel_rule,
            at: at([0]),
        };
        let right = CellApp {
            cell: ground,
            at: at([1]),
        };
        assert_eq!(
            PositionOrder::Incomparable,
            ToyAlphabet::position_order(&left.at, &right.at),
            "the first conjunct passes: the two redexes are disjoint subtrees"
        );
        let refusal = derive_shift_equivalence(&store, &peak, &left, &right)
            .expect_err("the enumerator reports the pair overlapping");
        let ShiftObstruction::GenuineOverlap { overlap } = refusal
        else {
            panic!("the overlap conjunct is what refuses this pair: {refusal:?}");
        };
        let peel_cell = store.get(overlap.left).expect("the left cell is stored");
        assert_eq!(
            Some(Toy::var(ToyNameRef("x"))),
            ToyAlphabet::subterm_cmd_at(&peel_cell.rhs, &overlap.seam),
            "the seam addresses a bare metavariable, which critical-pair theory excludes"
        );
        // And this row's whole family is that gap, which is what separates it
        // from the schematic row above: repair the enumerator and the guard
        // stops refusing this pair, while the schematic pair stays refused.
        let ground_cell = store.get(ground).expect("the ground cell is stored");
        let mut family = overlaps_between((peel_rule, peel_cell), (ground, ground_cell));
        family.extend(overlaps_between(
            (ground, ground_cell),
            (peel_rule, peel_cell),
        ));
        assert!(!family.is_empty(), "the enumerator does report the pair");
        for candidate in &family {
            assert_eq!(
                Some(Toy::var(ToyNameRef("x"))),
                store
                    .get(candidate.left)
                    .and_then(|cell| ToyAlphabet::subterm_cmd_at(&cell.rhs, &candidate.seam)),
                "every seam in this pair's family is the bare metavariable"
            );
        }
        assert_eq!(
            FootprintIndependence::Independent,
            polarized(&store, &peak, &left, &right),
            "the polarized test never consults the enumerator, so it is not affected"
        );
        assert_eq!(
            Commutation::Commutes,
            commutation(&store, &peak, &left, &right),
            "and the pair commutes"
        );
    }

    #[test]
    fn a_rule_that_destroys_a_node_another_only_reads_is_refused()
    {
        // Read/write is a collision even though read/read is not: bump-left
        // matches the root and leaves it standing, add-Z tears it down.
        let mut store = CellStore::new();
        let bump = store.insert(bump_left());
        let zero_rule = store.insert(add_z());
        let peak = Toy::add(Toy::Zero, Toy::succ(Toy::Zero));
        let reader = match_footprint(&store, &peak, &CellApp {
            cell: bump,
            at: at([]),
        })
        .expect("bump-left fires at the root");
        let writer = match_footprint(&store, &peak, &CellApp {
            cell: zero_rule,
            at: at([]),
        })
        .expect("add-Z fires at the root");
        assert_eq!(
            FootprintIndependence::ReadWrite { position: at([0]) },
            footprint_independence(&reader, &writer),
            "bump-left rewrites the Zero add-Z matched and left standing"
        );
        assert_eq!(
            FootprintIndependence::ReadWrite { position: at([]) },
            footprint_independence(&writer, &reader),
            "and add-Z tears down the root bump-left matched and left standing"
        );
    }

    #[test]
    fn two_rules_rewriting_one_child_collide_on_writes()
    {
        let mut store = CellStore::new();
        let once = store.insert(bump_left());
        let twice = store.insert(bump_left_twice());
        let peak = Toy::add(Toy::Zero, Toy::Zero);
        let first = match_footprint(&store, &peak, &CellApp {
            cell: once,
            at: at([]),
        })
        .expect("bump-left fires");
        let second = match_footprint(&store, &peak, &CellApp {
            cell: twice,
            at: at([]),
        })
        .expect("bump-left-twice fires");
        assert_eq!(
            FootprintIndependence::WriteWrite { position: at([0]) },
            footprint_independence(&first, &second),
            "two rules rewriting one child collide there, read/read root notwithstanding"
        );
    }

    #[test]
    fn a_pair_colliding_several_ways_reports_the_earliest_address_it_scans()
    {
        // The verdict's intension clause, pinned: collisions are searched in
        // the left footprint's `written` order, which is the alphabet's own, so
        // a pair colliding at every address of one match image names the first
        // of them rather than an arbitrary one.
        let mut store = CellStore::new();
        let ground = store.insert(add_zz());
        let peak = Toy::add(Toy::Zero, Toy::Zero);
        let step = CellApp {
            cell: ground,
            at: at([]),
        };
        let footprint = match_footprint(&store, &peak, &step).expect("add-ZZ fires at the root");
        assert_eq!(
            alloc::vec![at([]), at([1]), at([0])],
            footprint.written,
            "a ground rule writes its whole match image, in the alphabet's own enumeration \
             order — which is not left-to-right, and is exactly why the scan order is a \
             declared projection rather than an assumption"
        );
        assert_eq!(
            FootprintIndependence::WriteWrite { position: at([]) },
            footprint_independence(&footprint, &footprint),
            "and the collision reported is the earliest address of that scan"
        );
    }

    #[test]
    fn every_footprint_partitions_exactly_the_addresses_its_redex_covers()
    {
        // The datum's own shape claim, taken over the whole fixture family
        // rather than at one fixture: the three classes are pairwise disjoint,
        // they cover the match image exactly — the redex root and everything it
        // encloses, and nothing above it — and each comes out in the alphabet's
        // enumeration order, so two runs over one input agree entrywise.
        for row in table() {
            for step in [&row.left, &row.right] {
                let footprint = match_footprint(&row.store, &row.peak, step)
                    .expect("every step the differential table records is a transition");
                assert_eq!(
                    step.at, footprint.at,
                    "the footprint is attached to the step that was fired"
                );
                let addresses = ToyAlphabet::command_positions(&row.peak);
                let covered = addresses
                    .iter()
                    .filter(|position| {
                        matches!(
                            ToyAlphabet::position_order(&step.at, position),
                            PositionOrder::Same | PositionOrder::Encloses
                        )
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let mut classified = footprint.written.clone();
                classified.extend(footprint.read.iter().cloned());
                classified.extend(footprint.framed.iter().cloned());
                let mut seen: Vec<ToyPos> = Vec::new();
                for position in &classified {
                    assert!(
                        !seen.contains(position),
                        "the three classes are disjoint, so {position:?} is classified once"
                    );
                    seen.push(position.clone());
                }
                assert_eq!(
                    sorted(covered),
                    sorted(classified),
                    "the classes cover the match image exactly, at {:?}",
                    step.at
                );
                for class in [&footprint.written, &footprint.read, &footprint.framed] {
                    let enumerated = class
                        .iter()
                        .map(|position| {
                            addresses.iter().position(|candidate| candidate == position)
                        })
                        .collect::<Option<Vec<_>>>()
                        .expect("every classified address is one the alphabet enumerates");
                    let ordered =
                        enumerated
                            .windows(2)
                            .all(|pair| match (pair.first(), pair.get(1)) {
                                | (Some(earlier), Some(later)) => earlier < later,
                                | _ => true,
                            });
                    assert!(
                        ordered,
                        "each class keeps the alphabet's position order, at {:?}",
                        step.at
                    );
                }
            }
        }
    }

    #[test]
    fn a_relocated_frame_is_refused_because_addresses_move()
    {
        // The source's footprints range over machine addresses, which do not
        // move; a term position does. `relocate` carries its hole instance from
        // address [1] to address [0], so the prototype reports [1] as written
        // and refuses — conservatively, because the recorded schedule really
        // cannot be reordered. The two redexes do commute once the inner one is
        // re-addressed, and that residual map is the debt an adoption inherits.
        let mut store = CellStore::new();
        let mover = store.insert(relocate());
        let f = store.insert(f_cell());
        let peak = Toy::add(Toy::Zero, Toy::succ(Toy::Zero));
        let outer = CellApp {
            cell: mover,
            at: at([]),
        };
        let inner = CellApp {
            cell: f,
            at: at([1]),
        };
        assert_eq!(
            FootprintIndependence::WriteWrite { position: at([1]) },
            polarized(&store, &peak, &outer, &inner),
            "the vacated address counts as written, so the pair is refused"
        );
        assert_eq!(
            Commutation::Diverges,
            commutation(&store, &peak, &outer, &inner),
            "and the refusal is right: the recorded inner address is gone after the move"
        );
        // The redexes themselves commute, at the re-addressed inner position.
        let relocated = fire(&store, &peak, &outer).expect("the mover fires");
        assert_eq!(
            fire(&store, &peak, &inner).and_then(|term| fire(&store, &term, &outer)),
            fire(&store, &relocated, &CellApp {
                cell: f,
                at: at([0]),
            }),
            "the two derivations agree once the inner redex is tracked to its residual"
        );
    }

    #[test]
    fn the_cong2_pair_is_licensed_by_both_tests()
    {
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let g = store.insert(g_cell());
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::succ(Toy::Zero)));
        assert_eq!(
            LicensedBy::Both,
            licensed_by(
                &store,
                &peak,
                &CellApp {
                    cell: f,
                    at: at([0])
                },
                &CellApp {
                    cell: g,
                    at: at([1])
                }
            ),
            "the pair the guard was built for is licensed by the prototype too"
        );
    }

    #[test]
    fn the_polarized_test_refuses_a_pair_that_does_not_commute()
    {
        let mut store = CellStore::new();
        let ground = store.insert(add_zz());
        let bump = store.insert(bump_left());
        let peak = Toy::add(Toy::Zero, Toy::Zero);
        let left = CellApp {
            cell: ground,
            at: at([]),
        };
        let right = CellApp {
            cell: bump,
            at: at([]),
        };
        assert_eq!(
            LicensedBy::Neither,
            licensed_by(&store, &peak, &left, &right),
            "a pair at one address whose rules destroy each other's match is refused twice"
        );
        assert_eq!(
            Commutation::Diverges,
            commutation(&store, &peak, &left, &right),
            "and the refusals are right"
        );
    }

    #[test]
    fn the_guard_licenses_nothing_the_polarized_test_refuses()
    {
        // The containment the "strictly larger commuting class" claim needs:
        // over the whole table, no row is licensed by the guard alone.
        let mut guard_only = 0_usize;
        let mut rows = 0_usize;
        for row in table() {
            if licensed_by(&row.store, &row.peak, &row.left, &row.right) == LicensedBy::GuardOnly {
                guard_only = guard_only.saturating_add(1);
            }
            rows = rows.saturating_add(1);
        }
        assert_eq!(0, guard_only, "no fixture is licensed by the guard alone");
        assert_eq!(8, rows, "and the table is the one this suite documents");
    }

    #[test]
    fn every_row_of_the_differential_table_rules_as_recorded()
    {
        let mut both = 0_usize;
        let mut footprint_only = 0_usize;
        let mut neither = 0_usize;
        for row in table() {
            let licensed = licensed_by(&row.store, &row.peak, &row.left, &row.right);
            assert_eq!(
                row.licensed, licensed,
                "row at {:?}/{:?} is classified as recorded",
                row.left, row.right
            );
            let commutes = commutation(&row.store, &row.peak, &row.left, &row.right);
            assert_eq!(
                row.commutation, commutes,
                "row at {:?}/{:?} commutes as recorded",
                row.left, row.right
            );
            match licensed {
                | LicensedBy::Both => {
                    assert_eq!(
                        Commutation::Commutes,
                        commutes,
                        "a pair both tests license must commute"
                    );
                    both = both.saturating_add(1);
                },
                | LicensedBy::FootprintOnly => {
                    assert_eq!(
                        Commutation::Commutes,
                        commutes,
                        "every extra licence the polarized test grants must be earned"
                    );
                    footprint_only = footprint_only.saturating_add(1);
                },
                | LicensedBy::GuardOnly => panic!("the containment claim says this cell is empty"),
                | LicensedBy::Neither => {
                    neither = neither.saturating_add(1);
                },
            }
        }
        assert_eq!(1, both, "one row is licensed by both tests");
        assert_eq!(
            4, footprint_only,
            "four rows are licensed by the polarized test alone"
        );
        assert_eq!(3, neither, "and three rows by neither");
    }
}
