//! **The asynchronous-graph axioms for the shift quotient** — determinism and
//! the cube property, checked against the landed witness as built.
//!
//! # The structure being checked
//!
//! An asynchronous graph `(G, ◇)` is a graph `G` together with a set `◇` of
//! **squares** — pairs `(p, q)` of paths of length 2 with the same source and
//! the same target \[@mellies-2021-template-games, §III-A\]. It must satisfy
//! three properties, stated there as:
//!
//! 1. **symmetry** — `p ◇ q` implies `q ◇ p`;
//! 2. **determinism** — `p ◇ q` and `p ◇ q'` imply `q = q'`;
//! 3. **the cube property** — for all pairs `p, q : x ↠ y` of paths of length 3
//!    with the same source and target, written `p = u₁·u₂·u₃` and `q =
//!    v₁·v₂·v₃`, there are edges `w₃, u₂', v₂'` and permutation tiles `u₂·u₃ ◇
//!    u₂'·w₃`, `u₁·u₂' ◇ v₁·v₂'`, `v₂'·w₃ ◇ v₂·v₃` **if and only if** there are
//!    edges `w₁, u₂'', v₂''` and permutation tiles `u₁·u₂ ◇ w₁·u₂''`, `u₂''·u₃
//!    ◇ v₂''·v₃`, `w₁·v₂'' ◇ v₁·v₂`.
//!
//! Read as permutations of a three-letter word, the two chains of axiom 3 are
//! the two reduced words for the longest element of `S₃`: the cube property is
//! the braid relation on permutation tiles.
//!
//! # The pairing this suite fixes, because the axioms are not invariant under
//! a different one
//!
//! - a **vertex** is a term of the alphabet;
//! - an **edge** is one [`CellApp`] *firing* at a term — the graph is a
//!   multigraph whose edges carry the `(cell, position)` label, exactly as a
//!   graph in the internal-category sense is. Forgetting the label (taking an
//!   edge to be a source/target pair) is a **different structure**, and one on
//!   which determinism is not the same claim;
//! - a **square** `(p, q)` is a permutation tile exactly when
//!   [`derive_shift_equivalence`] earns a witness at the shared source whose
//!   two sequentializations are `p` and `q`. `◇` has no other generators: the
//!   shift constructor is the only tile-maker in the tree.
//!
//! # Verdicts, and how strong the evidence behind them is
//!
//! **Determinism — HOLDS.** Two facts carry it, both pinned below. First, the
//! graph is deterministic as a labelled transition system:
//! [`rewrite_at`] is a function of `(cell, term, position)`, so an edge's
//! target is fixed by its source and label. Second, a tile's two paths carry
//! the *same two labels transposed* — an application at a position incomparable
//! to the other's survives it verbatim — so a 2-path `a·b` determines the
//! witness `(x, a, b)` and hence the unique `q = b·a`. Grade: **small-scope
//! exhaustive** over the fixture family (every earned square at every fixture
//! peak), plus the structural argument above.
//!
//! Worth saying plainly, because it is the thing an adopter will get wrong:
//! gandr's cells *are* non-deterministic as a rewrite system — two cells can
//! fire at one position — and this axiom **does not quantify over that**. It
//! quantifies over tiles, and the guard's first conjunct refuses a pair at one
//! position before any tile exists, so branching never reaches the tile set.
//!
//! **The cube property — HOLDS.** The argument is that gandr's independence
//! relation is *term-independent*: [`derive_shift_equivalence`]'s guard reads
//! only the store, the two cells, and the two positions, never the peak. A pair
//! that is a tile at one vertex is therefore a tile at every vertex where both
//! applications still fire — and, by locality, firing one at an incomparable
//! position leaves the other firing. Both chains of axiom 3 then need exactly
//! the same three pairwise tiles, so neither can exist without the other.
//! Grade: **small-scope exhaustive** — every ordered pair of 3-paths out of the
//! three-redex fixture peak, with both chains *searched* over the graph's edges
//! rather than assumed, plus the structural argument.
//!
//! # A hazard the axioms rest on and the alphabet contract does not state
//!
//! Both arguments use a **locality** property of splicing: rewriting at one
//! position leaves the subterm at an incomparable position untouched, so a
//! recorded application survives its neighbour verbatim.
//! [`CellAlphabet::splice_cmd_at`]'s contract says what happens *at* the
//! position and says nothing about what happens elsewhere. Both in-tree
//! alphabets satisfy locality, and
//! [`an_application_survives_an_incomparable_neighbour_verbatim`] pins it — but
//! an alphabet that violated it would break both axioms silently, and the trait
//! owes the clause.
//!
//! [`CellApp`]: gandr_theory_computads::CellApp
//! [`derive_shift_equivalence`]: gandr_theory_computads::derive_shift_equivalence
//! [`rewrite_at`]: gandr_theory_computads::rewrite::rewrite_at
//! [`CellAlphabet::splice_cmd_at`]: gandr_theory_computads::CellAlphabet::splice_cmd_at
//! [`an_application_survives_an_incomparable_neighbour_verbatim`]: tests::an_application_survives_an_incomparable_neighbour_verbatim

#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;

    use gandr_theory_computads::Cell;
    use gandr_theory_computads::CellAlphabet as _;
    use gandr_theory_computads::CellApp;
    use gandr_theory_computads::CellStore;
    use gandr_theory_computads::PositionOrder;
    use gandr_theory_computads::ShiftObstruction;
    use gandr_theory_computads::derive_shift_equivalence;
    use gandr_theory_computads::rewrite::rewrite_at;

    use crate::toy_alphabet::Toy;
    use crate::toy_alphabet::ToyAlphabet;
    use crate::toy_alphabet::ToyPos;
    use crate::toy_alphabet::toy_cell;

    extern crate alloc;

    /// Whether a searched-for tile or chain of tiles is there.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Presence
    {
        /// The graph exhibits it.
        Present,
        /// It is not there.
        Absent,
    }

    /// A toy position from child indices.
    fn at<Steps>(steps: Steps) -> ToyPos
    where
        Steps: IntoIterator<Item = usize>,
    {
        ToyPos(steps.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    /// (a): `Add(Succ(Zero), Zero) ~> Zero`.
    fn a_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::add(Toy::succ(Toy::Zero), Toy::Zero), Toy::Zero)
    }

    /// (b): `Add(Zero, Succ(Zero)) ~> Succ(Succ(Zero))`.
    fn b_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::Zero, Toy::succ(Toy::Zero)),
            Toy::succ(Toy::succ(Toy::Zero)),
        )
    }

    /// (c): `Add(Succ(Zero), Succ(Zero)) ~> Succ(Succ(Succ(Zero)))`.
    ///
    /// The three cells are ground on both faces, pairwise non-overlapping, and
    /// their reducts match no left-hand side in the store — so the fixture's
    /// graph has exactly the three edges the fixture puts there, and its cube
    /// is a cube rather than a cube with extra corridors.
    fn c_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero)),
            Toy::succ(Toy::succ(Toy::succ(Toy::Zero))),
        )
    }

    /// (c-into-a): `Add(Succ(Zero), Succ(Zero)) ~> Add(Succ(Zero), Zero)`.
    ///
    /// A variant of `c` whose right-hand side *is* `a`'s left-hand side, so the
    /// pair `(a, c)` has a genuine composition overlap and is refused the
    /// witness — the third face of the cube removed on purpose.
    fn c_into_a_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero)),
            Toy::add(Toy::succ(Toy::Zero), Toy::Zero),
        )
    }

    /// The three-redex peak: `Add(a-redex, Add(b-redex, c-redex))`.
    fn cube_peak() -> Toy
    {
        Toy::add(
            Toy::add(Toy::succ(Toy::Zero), Toy::Zero),
            Toy::add(
                Toy::add(Toy::Zero, Toy::succ(Toy::Zero)),
                Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero)),
            ),
        )
    }

    /// The three-cell store and the three applications the cube is built from.
    fn cube_fixture() -> (
        CellStore<ToyAlphabet>,
        CellApp<ToyAlphabet>,
        CellApp<ToyAlphabet>,
        CellApp<ToyAlphabet>,
    )
    {
        let mut store = CellStore::new();
        let first = store.insert(a_cell());
        let second = store.insert(b_cell());
        let third = store.insert(c_cell());
        (
            store,
            CellApp {
                cell: first,
                at: at([0]),
            },
            CellApp {
                cell: second,
                at: at([1, 0]),
            },
            CellApp {
                cell: third,
                at: at([1, 1]),
            },
        )
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

    /// Run a recorded schedule from `start`.
    fn run(
        store: &CellStore<ToyAlphabet>,
        start: &Toy,
        schedule: &[CellApp<ToyAlphabet>],
    ) -> Option<Toy>
    {
        let mut current = start.clone();
        for step in schedule {
            current = fire(store, &current, step)?;
        }
        Some(current)
    }

    /// Every edge out of `term` — one per `(cell, position)` pair that fires.
    fn edges_at(
        store: &CellStore<ToyAlphabet>,
        term: &Toy,
    ) -> Vec<CellApp<ToyAlphabet>>
    {
        let mut out = Vec::new();
        for position in ToyAlphabet::command_positions(term) {
            for (id, cell) in store.iter() {
                if rewrite_at(cell, term, &position).is_some() {
                    out.push(CellApp {
                        cell: id,
                        at: position.clone(),
                    });
                }
            }
        }
        out
    }

    /// Whether the square `(first·second, other_first·other_second)` out of
    /// `vertex` is a permutation tile.
    ///
    /// The tile set has one generator — the shift witness — so this is its
    /// definition, not a re-derivation: the square is a tile exactly when the
    /// witness for the first path exists and the second path is its other
    /// sequentialization.
    fn is_tile(
        store: &CellStore<ToyAlphabet>,
        vertex: &Toy,
        first: &CellApp<ToyAlphabet>,
        second: &CellApp<ToyAlphabet>,
        other_first: &CellApp<ToyAlphabet>,
        other_second: &CellApp<ToyAlphabet>,
    ) -> Presence
    {
        let Ok(witness) = derive_shift_equivalence(store, vertex, first, second)
        else {
            return Presence::Absent;
        };
        if witness.second_then_first() == alloc::vec![other_first.clone(), other_second.clone()] {
            Presence::Present
        }
        else {
            Presence::Absent
        }
    }

    /// Search for the **first** chain of axiom 3: edges `w₃, u₂', v₂'` with
    /// `u₂·u₃ ◇ u₂'·w₃`, `u₁·u₂' ◇ v₁·v₂'`, `v₂'·w₃ ◇ v₂·v₃`.
    fn front_chain(
        store: &CellStore<ToyAlphabet>,
        peak: &Toy,
        forward: (
            &CellApp<ToyAlphabet>,
            &CellApp<ToyAlphabet>,
            &CellApp<ToyAlphabet>,
        ),
        backward: (
            &CellApp<ToyAlphabet>,
            &CellApp<ToyAlphabet>,
            &CellApp<ToyAlphabet>,
        ),
    ) -> Presence
    {
        let (u1, u2, u3) = forward;
        let (v1, v2, v3) = backward;
        let (Some(after_u1), Some(after_v1)) = (fire(store, peak, u1), fire(store, peak, v1))
        else {
            return Presence::Absent;
        };
        for u2_prime in edges_at(store, &after_u1) {
            let Some(after_u2_prime) = fire(store, &after_u1, &u2_prime)
            else {
                continue;
            };
            for w3 in edges_at(store, &after_u2_prime) {
                if is_tile(store, &after_u1, u2, u3, &u2_prime, &w3) != Presence::Present {
                    continue;
                }
                for v2_prime in edges_at(store, &after_v1) {
                    if is_tile(store, peak, u1, &u2_prime, v1, &v2_prime) != Presence::Present {
                        continue;
                    }
                    if is_tile(store, &after_v1, &v2_prime, &w3, v2, v3) == Presence::Present {
                        return Presence::Present;
                    }
                }
            }
        }
        Presence::Absent
    }

    /// Search for the **second** chain of axiom 3: edges `w₁, u₂'', v₂''` with
    /// `u₁·u₂ ◇ w₁·u₂''`, `u₂''·u₃ ◇ v₂''·v₃`, `w₁·v₂'' ◇ v₁·v₂`.
    fn back_chain(
        store: &CellStore<ToyAlphabet>,
        peak: &Toy,
        forward: (
            &CellApp<ToyAlphabet>,
            &CellApp<ToyAlphabet>,
            &CellApp<ToyAlphabet>,
        ),
        backward: (
            &CellApp<ToyAlphabet>,
            &CellApp<ToyAlphabet>,
            &CellApp<ToyAlphabet>,
        ),
    ) -> Presence
    {
        let (u1, u2, u3) = forward;
        let (v1, v2, v3) = backward;
        for w1 in edges_at(store, peak) {
            let Some(after_w1) = fire(store, peak, &w1)
            else {
                continue;
            };
            for u2_second in edges_at(store, &after_w1) {
                if is_tile(store, peak, u1, u2, &w1, &u2_second) != Presence::Present {
                    continue;
                }
                for v2_second in edges_at(store, &after_w1) {
                    if is_tile(store, &after_w1, &u2_second, u3, &v2_second, v3)
                        != Presence::Present
                    {
                        continue;
                    }
                    if is_tile(store, peak, &w1, &v2_second, v1, v2) == Presence::Present {
                        return Presence::Present;
                    }
                }
            }
        }
        Presence::Absent
    }

    /// Every path of length 3 out of `peak`.
    fn three_paths(
        store: &CellStore<ToyAlphabet>,
        peak: &Toy,
    ) -> Vec<(
        CellApp<ToyAlphabet>,
        CellApp<ToyAlphabet>,
        CellApp<ToyAlphabet>,
    )>
    {
        let mut out = Vec::new();
        for first in edges_at(store, peak) {
            let Some(after_first) = fire(store, peak, &first)
            else {
                continue;
            };
            for second in edges_at(store, &after_first) {
                let Some(after_second) = fire(store, &after_first, &second)
                else {
                    continue;
                };
                for third in edges_at(store, &after_second) {
                    out.push((first.clone(), second.clone(), third));
                }
            }
        }
        out
    }

    /// One element of `◇` — a pair of 2-paths sharing a source and a target.
    #[derive(Clone, Debug, Eq, PartialEq)]
    struct Square
    {
        /// The first of the two paths.
        path: Vec<CellApp<ToyAlphabet>>,
        /// The path it is tiled with.
        other: Vec<CellApp<ToyAlphabet>>,
    }

    /// Every square the tile set generates out of `peak`, symmetry closed.
    fn squares_at(
        store: &CellStore<ToyAlphabet>,
        peak: &Toy,
    ) -> Vec<Square>
    {
        let mut out = Vec::new();
        let edges = edges_at(store, peak);
        for first in &edges {
            for second in &edges {
                let Ok(witness) = derive_shift_equivalence(store, peak, first, second)
                else {
                    continue;
                };
                out.push(Square {
                    path: witness.first_then_second(),
                    other: witness.second_then_first(),
                });
                out.push(Square {
                    path: witness.second_then_first(),
                    other: witness.first_then_second(),
                });
            }
        }
        out
    }

    #[test]
    fn an_edge_target_is_fixed_by_its_source_and_label()
    {
        // The graph is a deterministic labelled transition system, which is
        // the half of determinism that lives below the tile set.
        let (store, first, _second, _third) = cube_fixture();
        let peak = cube_peak();
        assert_eq!(
            fire(&store, &peak, &first),
            fire(&store, &peak, &first),
            "one label at one source reaches one target"
        );
        assert!(
            fire(&store, &peak, &first).is_some(),
            "and the fixture's edge is an edge"
        );
    }

    #[test]
    fn rewrite_branching_never_reaches_the_tile_set()
    {
        // gandr's cells are non-deterministic AS A REWRITE SYSTEM: two cells
        // fire at one position and reach different terms. The determinism
        // axiom does not quantify over that, and the guard's first conjunct is
        // why it never has to: a pair at one position is refused before any
        // square exists.
        let mut store = CellStore::new();
        let down = store.insert(toy_cell(Toy::succ(Toy::Zero), Toy::Zero));
        let up = store.insert(toy_cell(
            Toy::succ(Toy::Zero),
            Toy::succ(Toy::succ(Toy::Zero)),
        ));
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let left = CellApp {
            cell: down,
            at: at([0]),
        };
        let right = CellApp {
            cell: up,
            at: at([0]),
        };
        assert_ne!(
            fire(&store, &peak, &left),
            fire(&store, &peak, &right),
            "two cells at one position genuinely branch"
        );
        assert_eq!(
            Err(ShiftObstruction::ComparablePositions {
                order: PositionOrder::Same
            }),
            derive_shift_equivalence(&store, &peak, &left, &right),
            "and the branch is refused a tile, so determinism is never asked about it"
        );
    }

    #[test]
    fn an_application_survives_an_incomparable_neighbour_verbatim()
    {
        // The locality property both axiom arguments rest on, and the one the
        // CellAlphabet contract does not state: rewriting at one position
        // leaves the subterm at an incomparable position identical, so the
        // residual of an application is the application itself — same cell,
        // same position.
        let (store, first, second, third) = cube_fixture();
        let peak = cube_peak();
        let after = fire(&store, &peak, &first).expect("the first application fires");
        for neighbour in [&second, &third] {
            assert_eq!(
                PositionOrder::Incomparable,
                ToyAlphabet::position_order(&first.at, &neighbour.at),
                "the fixture's positions are pairwise incomparable"
            );
            assert_eq!(
                ToyAlphabet::subterm_cmd_at(&peak, &neighbour.at),
                ToyAlphabet::subterm_cmd_at(&after, &neighbour.at),
                "and the neighbour's subterm is untouched by the rewrite"
            );
            assert!(
                fire(&store, &after, neighbour).is_some(),
                "so the neighbour still fires at its recorded position, unrelabelled"
            );
        }
    }

    #[test]
    fn every_permutation_tile_is_symmetric()
    {
        // Axiom 1. Symmetry is by construction — the guard is symmetric in the
        // pair and both orders are exercised — but "by construction" is a
        // claim about the code, so it is checked over the fixture family.
        let (store, first, second, third) = cube_fixture();
        let mut tiles = 0_usize;
        for peak in fixture_vertices(&store, &cube_peak(), &[&first, &second, &third]) {
            for left in edges_at(&store, &peak) {
                for right in edges_at(&store, &peak) {
                    let Ok(witness) = derive_shift_equivalence(&store, &peak, &left, &right)
                    else {
                        continue;
                    };
                    let swapped = derive_shift_equivalence(&store, &peak, &right, &left)
                        .expect("the reverse ordered pair earns the witness too");
                    assert_eq!(
                        witness.joins_at, swapped.joins_at,
                        "the swapped square joins where the original does"
                    );
                    assert_eq!(
                        witness.first_then_second(),
                        swapped.second_then_first(),
                        "and the two squares are the same square, transposed"
                    );
                    tiles = tiles.saturating_add(1);
                }
            }
        }
        // Six ordered pairs at the peak and two at each of the three one-step
        // vertices: the cube's twelve edges, read as ordered pairs.
        assert_eq!(
            12, tiles,
            "the fixture family's tile count, so the sweep is not vacuous"
        );
    }

    #[test]
    fn every_permutation_tile_is_deterministic()
    {
        // Axiom 2, checked as stated: collect every square the tile set
        // generates out of a vertex, then assert that no 2-path is the first
        // component of two squares with different second components.
        let (store, first, second, third) = cube_fixture();
        let mut checked = 0_usize;
        for peak in fixture_vertices(&store, &cube_peak(), &[&first, &second, &third]) {
            let squares = squares_at(&store, &peak);
            for square in &squares {
                for candidate in &squares {
                    if candidate.path != square.path {
                        continue;
                    }
                    assert_eq!(
                        square.other, candidate.other,
                        "one 2-path is tiled with at most one other 2-path"
                    );
                    checked = checked.saturating_add(1);
                }
            }
        }
        assert_eq!(
            48, checked,
            "the fixture family's square comparisons, so the sweep is not vacuous"
        );
    }

    #[test]
    fn the_cube_closes_on_three_pairwise_independent_applications()
    {
        // All six interleavings reach one term, and all six faces of the cube
        // are earned witnesses that replay.
        let (store, first, second, third) = cube_fixture();
        let peak = cube_peak();
        let orders = [
            [&first, &second, &third],
            [&first, &third, &second],
            [&second, &first, &third],
            [&second, &third, &first],
            [&third, &first, &second],
            [&third, &second, &first],
        ];
        let reached = run(&store, &peak, &alloc::vec![
            first.clone(),
            second.clone(),
            third.clone()
        ])
        .expect("the recorded order runs");
        for order in orders {
            let schedule: Vec<CellApp<ToyAlphabet>> =
                order.iter().map(|step| (*step).clone()).collect();
            assert_eq!(
                Some(reached.clone()),
                run(&store, &peak, &schedule),
                "every interleaving of three disjoint applications reaches one term"
            );
        }
        // The six faces: each unordered pair, at the peak and at the vertex
        // reached by firing the third application.
        let pairs = [
            (&first, &second, &third),
            (&first, &third, &second),
            (&second, &third, &first),
        ];
        let mut faces = 0_usize;
        for (left, right, other) in pairs {
            for vertex in [
                peak.clone(),
                fire(&store, &peak, other).expect("the third application fires"),
            ] {
                let witness = derive_shift_equivalence(&store, &vertex, left, right)
                    .expect("each face of the cube is an earned tile");
                assert!(
                    bool::from(witness.replay(&store)),
                    "and each face replays rather than being asserted"
                );
                faces = faces.saturating_add(1);
            }
        }
        assert_eq!(6, faces, "a cube has six faces and all six are tiles");
    }

    #[test]
    fn the_cube_property_holds_for_every_pair_of_three_paths()
    {
        // Axiom 3, checked as stated and by search: for every ordered pair of
        // 3-paths out of the peak with the same target, the first chain exists
        // if and only if the second does. Neither chain is assumed — both are
        // looked for among the graph's own edges.
        let (store, first, second, third) = cube_fixture();
        let peak = cube_peak();
        let paths = three_paths(&store, &peak);
        assert_eq!(
            6,
            paths.len(),
            "three pairwise disjoint redexes and nothing else"
        );
        let mut pairs = 0_usize;
        let mut present = 0_usize;
        for forward in &paths {
            for backward in &paths {
                let forward_ref = (&forward.0, &forward.1, &forward.2);
                let backward_ref = (&backward.0, &backward.1, &backward.2);
                let front = front_chain(&store, &peak, forward_ref, backward_ref);
                let back = back_chain(&store, &peak, forward_ref, backward_ref);
                assert_eq!(
                    front, back,
                    "the cube property is an iff, and both sides agree here"
                );
                if front == Presence::Present {
                    present = present.saturating_add(1);
                }
                pairs = pairs.saturating_add(1);
            }
        }
        assert_eq!(36, pairs, "every ordered pair of the six 3-paths");
        assert_eq!(
            6, present,
            "and the iff is not vacuous: each path's reversal is reached by both chains"
        );
        // Named for the record: the reversal pair is the one both chains reach.
        let forward = (&first, &second, &third);
        let backward = (&third, &second, &first);
        assert_eq!(
            Presence::Present,
            front_chain(&store, &peak, forward, backward),
            "the first chain reshuffles abc to cba"
        );
        assert_eq!(
            Presence::Present,
            back_chain(&store, &peak, forward, backward),
            "and so does the second, which is the braid relation"
        );
    }

    #[test]
    fn a_missing_pairwise_tile_removes_both_cube_routes()
    {
        // Adequacy for the axiom-3 check: with one of the three pairwise tiles
        // refused, the iff must still hold — and it holds by both sides
        // vanishing, not by one of them surviving.
        let mut store = CellStore::new();
        let first_cell = store.insert(a_cell());
        let second_cell = store.insert(b_cell());
        let third_cell = store.insert(c_into_a_cell());
        let first = CellApp {
            cell: first_cell,
            at: at([0]),
        };
        let second = CellApp {
            cell: second_cell,
            at: at([1, 0]),
        };
        let third = CellApp {
            cell: third_cell,
            at: at([1, 1]),
        };
        let peak = cube_peak();
        let refusal = derive_shift_equivalence(&store, &peak, &first, &third)
            .expect_err("c's right-hand side is a's left-hand side, so the pair overlaps");
        assert!(
            matches!(refusal, ShiftObstruction::GenuineOverlap { .. }),
            "and the overlap conjunct is what refuses it: {refusal:?}"
        );
        assert!(
            derive_shift_equivalence(&store, &peak, &first, &second).is_ok(),
            "the other two pairs are still tiles"
        );
        let after_first = fire(&store, &peak, &first).expect("the first application fires");
        assert!(
            derive_shift_equivalence(&store, &after_first, &second, &third).is_ok(),
            "including the one at the vertex the front chain uses"
        );
        let forward = (&first, &second, &third);
        let backward = (&third, &second, &first);
        assert_eq!(
            Presence::Absent,
            front_chain(&store, &peak, forward, backward),
            "the first chain cannot be closed without the missing tile"
        );
        assert_eq!(
            Presence::Absent,
            back_chain(&store, &peak, forward, backward),
            "and neither can the second, so the iff holds by both sides vanishing"
        );
    }

    /// Every vertex the fixture's three applications reach from `peak`.
    fn fixture_vertices(
        store: &CellStore<ToyAlphabet>,
        peak: &Toy,
        steps: &[&CellApp<ToyAlphabet>],
    ) -> Vec<Toy>
    {
        let mut out = alloc::vec![peak.clone()];
        let mut frontier = alloc::vec![peak.clone()];
        while let Some(vertex) = frontier.pop() {
            for step in steps {
                let Some(next) = fire(store, &vertex, step)
                else {
                    continue;
                };
                if out.contains(&next) {
                    continue;
                }
                out.push(next.clone());
                frontier.push(next);
            }
        }
        out
    }
}
