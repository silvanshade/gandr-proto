//! **Atom-occurrence flow** differential fixtures — where the projection
//! agrees with its two neighbours and where it does not.
//!
//! # Why these live here, and on the toy alphabet
//!
//! The projection's resolution is the alphabet's address vocabulary, and a
//! sequent command pattern has exactly one command position: over
//! `SequentAlphabet` every term is one atom occurrence and no two applications
//! can sit at incomparable positions, so the permutation-tile class is empty
//! there. The toy alphabet nests commands, so it is the one alphabet in the
//! tree where a tile exists to be projected. The sequent-side degeneration and
//! the two refusals are pinned in the module's own unit tests.
//!
//! # What the suite measures
//!
//! The three relations on certificate data are strictly nested, and each
//! strictness has a fixture here rather than an argument:
//!
//! - [`the_two_legs_of_a_permutation_tile_have_one_flow`] and
//!   [`disjoint_steps_share_no_thread`] are the positive half: on the tile
//!   class the two sequentializations project to one flow, and they do so
//!   because neither step touches an occurrence the other created.
//! - [`the_shift_guard_refuses_a_pair_the_projection_identifies`] is the first
//!   strictness — **shift equivalence is strictly inside flow equality**. The
//!   guard asks [`overlaps_between`], a question about the *alphabet*, and
//!   refuses a pair whose cells could interfere somewhere; the projection reads
//!   the *instance*, where two disjoint match images share no occurrence, and
//!   identifies it.
//! - [`flow_equality_is_strictly_finer_than_replay_equivalence`] and
//!   [`replay_equivalent_certificates_can_carry_different_flows`] are the
//!   second — **flow equality is strictly inside replay-equivalence**.
//!   [`replay_equivalent`] compares a boundary and asks each side to replay,
//!   ignoring the recorded paths outright, so no relation reading the paths can
//!   coincide with it.
//! - [`equal_flows_over_different_boundaries_are_not_one_certificate`] is the
//!   **containment** that strictness is a strictness *of*, and it is the one
//!   the suite originally left unpinned. A relation on certificates that
//!   compares the projected flows and nothing else is not inside
//!   replay-equivalence at all: one cell fired on two unrelated instances of
//!   its left-hand side gives one flow over two boundaries.
//!   [`flow_equality_implies_replay_equivalence`] checks the implication
//!   pairwise over every certificate the suite builds.
//!
//! # The fourth relation: the declined games quotient, measured
//!
//! The certificate algebra declined the asynchronous-games quotient —
//! identification by a matching step-index bijection — and the projection's
//! design record suspected it sat strictly inside flow equality, which would
//! have extended the chain leftward into a decision already taken. The
//! fixtures here measure that suspicion against the discharge class and
//! refute it in reverse:
//!
//! - [`the_games_quotient_identifies_two_firings_the_peak_anchor_separates`]
//!   and [`the_games_quotient_identifies_a_tile_the_flow_declines_a_canonical_form`]
//!   are the two separating families — the quotient identifies what flow
//!   equality separates or declines, so the quotient is **not** contained in
//!   flow equality. The second family is vocabulary-robust: it needs no
//!   commitment about what a step's identity is, because the two legs record
//!   the very same steps.
//! - [`flow_equality_sits_inside_the_games_quotient_on_the_discharge_class`]
//!   checks the surviving containment on every flow-equal leg pair the suite
//!   builds. The measured verdict is that **flow equality is strictly finer
//!   than the games quotient on the discharge class** — the chain does not
//!   extend leftward, and the template-identity story sits to the quotient's
//!   right.
//!
//! [`the_two_legs_of_a_permutation_tile_have_one_flow`]: tests::the_two_legs_of_a_permutation_tile_have_one_flow
//! [`disjoint_steps_share_no_thread`]: tests::disjoint_steps_share_no_thread
//! [`the_shift_guard_refuses_a_pair_the_projection_identifies`]: tests::the_shift_guard_refuses_a_pair_the_projection_identifies
//! [`flow_equality_is_strictly_finer_than_replay_equivalence`]: tests::flow_equality_is_strictly_finer_than_replay_equivalence
//! [`replay_equivalent_certificates_can_carry_different_flows`]: tests::replay_equivalent_certificates_can_carry_different_flows
//! [`equal_flows_over_different_boundaries_are_not_one_certificate`]: tests::equal_flows_over_different_boundaries_are_not_one_certificate
//! [`flow_equality_implies_replay_equivalence`]: tests::flow_equality_implies_replay_equivalence
//! [`the_games_quotient_identifies_two_firings_the_peak_anchor_separates`]: tests::the_games_quotient_identifies_two_firings_the_peak_anchor_separates
//! [`the_games_quotient_identifies_a_tile_the_flow_declines_a_canonical_form`]: tests::the_games_quotient_identifies_a_tile_the_flow_declines_a_canonical_form
//! [`flow_equality_sits_inside_the_games_quotient_on_the_discharge_class`]: tests::flow_equality_sits_inside_the_games_quotient_on_the_discharge_class
//! [`overlaps_between`]: gandr_theory_coherent_resolutions::overlaps_between
//! [`replay_equivalent`]: gandr_theory_coherent_resolutions::replay_equivalent

#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;

    use gandr_theory_cell_complexes::Cell;
    use gandr_theory_cell_complexes::CellId;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes::ConvexityDischarge;
    use gandr_theory_cell_complexes::FlowEquality;
    use gandr_theory_cell_complexes::FlowVertexIndex;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyAlphabet;
    use gandr_theory_cell_complexes_tools::toy::ToyNameRef;
    use gandr_theory_cell_complexes_tools::toy::ToyPos;
    use gandr_theory_cell_complexes_tools::toy::toy_cell;
    use gandr_theory_coherent_resolutions::CellApp;
    use gandr_theory_coherent_resolutions::OverlapKind;
    use gandr_theory_coherent_resolutions::Tracelet;
    use gandr_theory_coherent_resolutions::derive_fused;
    use gandr_theory_coherent_resolutions::enumerate_overlaps;
    use gandr_theory_coherent_resolutions::replay_equivalent;
    use gandr_theory_deep_inference::Flow;
    use gandr_theory_deep_inference::FlowEnd;
    use gandr_theory_deep_inference::FlowObstruction;
    use gandr_theory_deep_inference::cell_address;
    use gandr_theory_deep_inference::derive_shift_equivalence;
    use gandr_theory_deep_inference::flows_equal;
    use gandr_theory_deep_inference::legs_flow_equal;
    use gandr_theory_deep_inference::project_flow;
    use gandr_theory_deep_inference::tracelet_flow;
    use gandr_theory_deep_inference::tracelets_flow_equal;

    extern crate alloc;

    /// A toy position from child indices.
    fn at<Steps>(steps: Steps) -> ToyPos
    where
        Steps: IntoIterator<Item = usize>,
    {
        ToyPos(steps.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    /// Every permutation of the flow's vertex indices, built iteratively by
    /// insertion.
    ///
    /// The quotient check brute-forces the step-index bijection; the suite's
    /// legs never exceed three steps, so the factorial enumeration is the
    /// honest implementation rather than a matching algorithm.
    fn index_permutations(flow: &Flow) -> Vec<Vec<FlowVertexIndex>>
    {
        let mut permutations: Vec<Vec<FlowVertexIndex>> = alloc::vec![Vec::new()];
        for item in 0 .. flow.labels.len() {
            let item = FlowVertexIndex::from(item);
            let mut next: Vec<Vec<FlowVertexIndex>> = Vec::new();
            for permutation in &permutations {
                for slot in 0 ..= permutation.len() {
                    let mut candidate = permutation.clone();
                    candidate.insert(slot, item);
                    next.push(candidate);
                }
            }
            permutations = next;
        }
        permutations
    }

    /// Whether `earlier` reaches `later` through the flow's vertex-to-vertex
    /// threads — the dependence order the recorded leg induces, read off the
    /// projection rather than re-derived from the alphabet.
    fn depends_before(
        flow: &Flow,
        earlier: FlowVertexIndex,
        later: FlowVertexIndex,
    ) -> FlowEquality
    {
        let mut reached: Vec<FlowVertexIndex> = alloc::vec![earlier];
        let mut work: Vec<FlowVertexIndex> = alloc::vec![earlier];
        while let Some(current) = work.pop() {
            for thread in &flow.threads {
                let (FlowEnd::Vertex { vertex: up, .. }, FlowEnd::Vertex { vertex: lo, .. }) =
                    (thread.up, thread.lo)
                else {
                    continue;
                };
                if up == current && !reached.contains(&lo) {
                    if lo == later {
                        return FlowEquality::from(true);
                    }
                    reached.push(lo);
                    work.push(lo);
                }
            }
        }
        FlowEquality::from(false)
    }

    /// Whether two legs are identified by the **asynchronous-games quotient**
    /// — the relation the certificate algebra declined, stated over the
    /// flow's own data: some bijection on step indices matches the
    /// position-free cell content of each pair of matched steps and
    /// preserves the dependence order both ways.
    ///
    /// `project_flow` emits `labels` in recorded order, so a flow carries
    /// the leg's steps indexed exactly as the quotient reads them; the
    /// recorded `CellApp`s are not needed separately.
    fn games_equivalent(
        left: &Flow,
        right: &Flow,
    ) -> FlowEquality
    {
        if left.labels.len() != right.labels.len() {
            return FlowEquality::from(false);
        }
        let matched = index_permutations(left).into_iter().any(|permutation| {
            let labels_match = left.labels.iter().enumerate().all(|(index, label)| {
                permutation
                    .get(index)
                    .and_then(|mapped| right.labels.get(usize::from(*mapped)))
                    == Some(label)
            });
            let order_preserved = (0 .. left.labels.len()).all(|earlier| {
                (0 .. left.labels.len()).all(|later| {
                    let mapped = |at: usize| permutation.get(at).copied().unwrap_or_default();
                    let (earlier, later) =
                        (FlowVertexIndex::from(earlier), FlowVertexIndex::from(later));
                    bool::from(depends_before(left, earlier, later))
                        == bool::from(depends_before(
                            right,
                            mapped(usize::from(earlier)),
                            mapped(usize::from(later)),
                        ))
                })
            });
            labels_match && order_preserved
        });
        FlowEquality::from(matched)
    }

    /// (f): `Succ(Zero) ~> Zero` — the rule the left redex node carries.
    fn f_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::Zero), Toy::Zero)
    }

    /// (g): `Succ(Succ(Zero)) ~> Zero` — the rule the right redex node carries.
    fn g_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::succ(Toy::Zero)), Toy::Zero)
    }

    /// (dup): `x ~> Add(x, x)` — the duplicating rule the tie fixture fires
    /// once, so that its two copies can each carry an `f` firing.
    fn dup_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(
            Toy::var(ToyNameRef("x")),
            Toy::add(Toy::var(ToyNameRef("x")), Toy::var(ToyNameRef("x"))),
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

    /// The `cong2` store and its two rule identifiers.
    fn cong2_store() -> (CellStore<ToyAlphabet>, CellId, CellId)
    {
        let mut store = CellStore::new();
        let f = store.insert(f_cell());
        let g = store.insert(g_cell());
        (store, f, g)
    }

    /// The `cong2` body — two redex nodes whiskered into one `add` frame.
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

    #[test]
    fn the_two_legs_of_a_permutation_tile_have_one_flow()
    {
        // The permutation tile: one peak, two sequentializations of one pair of
        // independent steps, one composite. The two legs record the same events
        // in opposite orders, and the projection identifies them — which is
        // what makes flow equality a witness for the shift quotient rather than
        // a restatement of the recorded order.
        let (store, f, g) = cong2_store();
        let peak = cong2_body();
        let (first, second) = cong2_pair(f, g);
        let witness = derive_shift_equivalence(&store, &peak, &first, &second)
            .expect("the cong2 pair earns its shift witness");
        let forward = project_flow(&store, &peak, &witness.first_then_second())
            .expect("f then g is a derivation of the peak");
        let backward = project_flow(&store, &peak, &witness.second_then_first())
            .expect("g then f is a derivation of the peak");
        assert!(
            bool::from(flows_equal(&forward, &backward)),
            "the two sequentializations of a permutation tile project to one flow"
        );
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            forward.convexity,
            "and the flow carries the fence its soundness rests on, as the shift witness does"
        );
    }

    #[test]
    fn the_games_quotient_identifies_two_firings_the_peak_anchor_separates()
    {
        // THE LEFTWARD EXTENSION FAILS, first separating family. The
        // certificate algebra declined the asynchronous-games quotient —
        // identification by a matching step-index bijection — and the
        // projection page suspected it sat strictly inside flow equality. It
        // does not: one cell fired at two different positions of one peak is
        // ONE play to the quotient (one step, one label, no order to
        // preserve), while the flow's peak anchor keeps the two firings
        // apart. The quotient is therefore not contained in flow equality.
        let (store, f, _g) = cong2_store();
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero));
        let left = project_flow(&store, &peak, &alloc::vec![CellApp {
            cell: f,
            at: at([0]),
        }])
        .expect("f fires at the left argument");
        let right = project_flow(&store, &peak, &alloc::vec![CellApp {
            cell: f,
            at: at([1]),
        }])
        .expect("f fires at the right argument");
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            left.convexity,
            "the pair sits on the discharge class the experiment is scoped to"
        );
        assert!(
            bool::from(games_equivalent(&left, &right)),
            "the step-index bijection matches trivially: one step, one label, an empty order"
        );
        assert!(
            !bool::from(flows_equal(&left, &right)),
            "but the flows differ at the peak anchor, so the quotient is not inside flow equality"
        );
    }

    #[test]
    fn the_games_quotient_identifies_a_tile_the_flow_declines_a_canonical_form()
    {
        // THE LEFTWARD EXTENSION FAILS on the tile class itself, and this
        // family is vocabulary-robust. `dup` duplicates a subterm; firing `f`
        // under each copy yields two vertices at one depth, under one label,
        // consuming nothing from the peak — a tie the canonical form declines
        // rather than orders. The two recorded orders of that independent
        // pair are one play under ANY step-identity finer than the recorded
        // steps themselves (the legs record the very same three steps), so no
        // refinement of the bijection's vocabulary escapes the separation:
        // the quotient identifies the tile, and flow equality declines it —
        // negatively, reflexively, and on the permutation-tile class where it
        // otherwise decides the shift identification.
        let mut store = CellStore::new();
        let dup = store.insert(dup_cell());
        let f = store.insert(f_cell());
        let peak = Toy::succ(Toy::Zero);
        let forward_path = alloc::vec![
            CellApp {
                cell: dup,
                at: at([]),
            },
            CellApp {
                cell: f,
                at: at([0]),
            },
            CellApp {
                cell: f,
                at: at([1]),
            },
        ];
        let backward_path = alloc::vec![
            CellApp {
                cell: dup,
                at: at([]),
            },
            CellApp {
                cell: f,
                at: at([1]),
            },
            CellApp {
                cell: f,
                at: at([0]),
            },
        ];
        let forward = project_flow(&store, &peak, &forward_path)
            .expect("dup then the two f firings is a derivation of the peak");
        let backward = project_flow(&store, &peak, &backward_path)
            .expect("the two f firings commute under the duplicated frame");
        assert_eq!(
            ConvexityDischarge::LeftConnectedOverAcyclicTarget,
            forward.convexity,
            "the pair sits on the discharge class the experiment is scoped to"
        );
        assert!(
            forward.canonical().is_none() && backward.canonical().is_none(),
            "two same-labelled vertices at one depth tie the key, and the tie is refused an order"
        );
        assert!(
            bool::from(games_equivalent(&forward, &backward)),
            "the quotient identifies the two orders of the tile — the bijection exists"
        );
        assert!(
            !bool::from(flows_equal(&forward, &backward)),
            "and the flow declines the identification the quotient makes"
        );
        // The certificate-level reading agrees: a tracelet recording the two
        // orders replays, and its two legs still have no shared canonical
        // form to compare.
        let (_fixture_store, composition) = fusion_fixture();
        let mut carrier = composition;
        carrier.peak = peak;
        let tracelet = Tracelet {
            overlap: carrier,
            path_a: forward_path,
            path_b: backward_path,
            joins_at: Toy::add(Toy::Zero, Toy::Zero),
        };
        assert!(
            bool::from(tracelet.replay(&store)),
            "both orders are derivations of the recorded boundary"
        );
        assert!(
            !bool::from(
                legs_flow_equal(&tracelet, &store).expect("both legs project and reach the join")
            ),
            "so the certificate-level relation declines the tile the shift witness would license"
        );
    }

    #[test]
    fn flow_equality_sits_inside_the_games_quotient_on_the_discharge_class()
    {
        // THE CONTAINMENT THAT SURVIVES, checked rather than argued: every
        // flow-equal leg pair this suite builds admits the matching
        // bijection. Canonical equality re-indexes vertices so that equal
        // canonical forms carry the same labels and the same thread
        // structure, and a thread-structure isomorphism is a
        // dependence-preserving step-index bijection — so flow equality is
        // contained in the games quotient on the discharge class, and the two
        // separating families above make the containment strict. Measured:
        // flow equality is strictly FINER than the quotient the page
        // suspected was finer than it.
        let (store, f, g) = cong2_store();
        let peak = cong2_body();
        let (first, second) = cong2_pair(f, g);
        let witness = derive_shift_equivalence(&store, &peak, &first, &second)
            .expect("the cong2 pair earns its shift witness");
        let forward = project_flow(&store, &peak, &witness.first_then_second())
            .expect("f then g is a derivation of the peak");
        let backward = project_flow(&store, &peak, &witness.second_then_first())
            .expect("g then f is a derivation of the peak");
        assert!(
            bool::from(flows_equal(&forward, &backward)),
            "the permutation tile is flow-equal, as the suite already pins"
        );
        assert!(
            bool::from(games_equivalent(&forward, &backward)),
            "and the flow-equal pair admits the matching bijection"
        );
        // The guard-refused pair — the first strictness's witness — is the
        // suite's other flow-equal leg pair, over a peak whose two cells the
        // shift guard refuses to commute.
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
        let forward = project_flow(&store, &peak, &alloc::vec![first.clone(), second.clone()])
            .expect("add-Z then add-S is a derivation of the peak");
        let backward = project_flow(&store, &peak, &alloc::vec![second, first])
            .expect("add-S then add-Z is a derivation of the peak");
        assert!(
            bool::from(flows_equal(&forward, &backward)),
            "the pair the guard refuses is flow-equal, as the suite already pins"
        );
        assert!(
            bool::from(games_equivalent(&forward, &backward)),
            "and it too admits the matching bijection"
        );
    }

    #[test]
    fn disjoint_steps_share_no_thread()
    {
        // Why the tile's two legs agree: neither step consumes an occurrence
        // the other created, so no thread runs from one vertex to the other and
        // the causal order is empty. This is the projection's independence
        // relation, read off the instance.
        let (store, f, g) = cong2_store();
        let peak = cong2_body();
        let (first, second) = cong2_pair(f, g);
        let flow = project_flow(&store, &peak, &alloc::vec![first, second])
            .expect("f then g is a derivation of the peak");
        assert_eq!(2, flow.labels.len(), "two cell applications, two vertices");
        assert!(
            !flow.threads.iter().any(|thread| matches!(
                (thread.up, thread.lo),
                (FlowEnd::Vertex { .. }, FlowEnd::Vertex { .. })
            )),
            "no occurrence created by one step is consumed by the other"
        );
        assert!(
            flow.threads.iter().any(|thread| matches!(
                (thread.up, thread.lo),
                (FlowEnd::Peak { .. }, FlowEnd::Join)
            )),
            "and the frame node neither step touches threads straight through"
        );
    }

    #[test]
    fn the_shift_guard_refuses_a_pair_the_projection_identifies()
    {
        // THE FIRST STRICTNESS. This is `tests/shift.rs`'s
        // `an_overlapping_toy_pair_at_disjoint_positions_is_refused` fixture,
        // read from the projection's side: two applications at disjoint
        // positions whose two orders reach one term, refused the shift witness
        // because the CELLS overlap — a question about the alphabet, asked
        // whatever this instance did. The projection asks the instance instead,
        // finds two disjoint match images sharing no occurrence, and licenses
        // the pair the guard refuses. Shift equivalence is therefore strictly
        // inside flow equality.
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
        assert!(
            derive_shift_equivalence(&store, &peak, &first, &second).is_err(),
            "the guard refuses the pair on its overlap conjunct"
        );
        let forward = project_flow(&store, &peak, &alloc::vec![first.clone(), second.clone()])
            .expect("add-Z then add-S is a derivation of the peak");
        let backward = project_flow(&store, &peak, &alloc::vec![second, first])
            .expect("add-S then add-Z is a derivation of the peak");
        assert!(
            bool::from(flows_equal(&forward, &backward)),
            "and the projection identifies the two orders the guard would not"
        );
    }

    #[test]
    fn flow_equality_is_strictly_finer_than_replay_equivalence()
    {
        // THE SECOND STRICTNESS, at one certificate. `derive_fused` builds one
        // boundary whose `path_a` is the two-step derivation and whose `path_b`
        // is the single fused step. It replays — that is the L2 gate's own
        // `fused ≡ two-step` contract — and its two legs carry different vertex
        // label multisets, so no re-indexing makes their flows agree.
        let (mut store, composition) = fusion_fixture();
        let (_fused, tracelet) =
            derive_fused(&composition, &mut store).expect("the fused cell derives");
        assert!(
            bool::from(tracelet.replay(&store)),
            "the certificate replays: both legs reach the recorded join"
        );
        assert!(
            !bool::from(legs_flow_equal(&tracelet, &store).expect("both legs project")),
            "and its two legs have different flows — one boundary, two flows"
        );
    }

    #[test]
    fn replay_equivalent_certificates_can_carry_different_flows()
    {
        // THE SECOND STRICTNESS, between two certificates, which is the form
        // the arc's question was asked in. Two structurally distinct
        // derivations of one boundary are one certificate under replay
        // identity — `tracelet.rs`'s
        // `distinct_derivations_of_one_boundary_are_replay_equivalent` is the
        // same pair. Their flows differ, because one leg is two steps and the
        // other is one.
        let (mut store, composition) = fusion_fixture();
        let (_fused, fused_derivation) =
            derive_fused(&composition, &mut store).expect("the fused cell derives");
        let two_step_derivation = Tracelet {
            overlap: fused_derivation.overlap.clone(),
            path_a: fused_derivation.path_a.clone(),
            path_b: fused_derivation.path_a.clone(),
            joins_at: fused_derivation.joins_at.clone(),
        };
        assert!(
            bool::from(replay_equivalent(
                &fused_derivation,
                &two_step_derivation,
                &store
            )),
            "the two are one certificate under replay identity"
        );
        assert!(
            !bool::from(
                tracelets_flow_equal(&fused_derivation, &two_step_derivation, &store)
                    .expect("both certificates project")
            ),
            "and two flows under the projection — so the two relations do not coincide"
        );
    }

    #[test]
    fn a_certificate_has_its_own_flow()
    {
        let (mut store, composition) = fusion_fixture();
        let (_fused, tracelet) =
            derive_fused(&composition, &mut store).expect("the fused cell derives");
        assert!(
            bool::from(
                tracelets_flow_equal(&tracelet, &tracelet, &store)
                    .expect("the certificate projects")
            ),
            "the relation is reflexive on a certificate whose legs project"
        );
    }

    #[test]
    fn a_single_step_leg_threads_the_whole_term_through_one_vertex()
    {
        // A ground rule whose match image is the whole term: every occurrence
        // is consumed at the one vertex, and every occurrence of the result is
        // created there and reaches the conclusion. No thread bypasses the
        // vertex, because there is no frame.
        let (store, f, _g) = cong2_store();
        let peak = Toy::succ(Toy::Zero);
        let flow = project_flow(&store, &peak, &alloc::vec![CellApp {
            cell: f,
            at: at([]),
        }])
        .expect("f fires at the root of Succ(Zero)");
        assert_eq!(1, flow.labels.len(), "one vertex");
        assert!(
            flow.threads.iter().all(|thread| matches!(
                thread.up,
                FlowEnd::Peak { .. } | FlowEnd::Vertex { .. }
            ) && matches!(
                thread.lo,
                FlowEnd::Vertex { .. } | FlowEnd::Join
            )),
            "every thread has an end at the vertex or at a boundary"
        );
        assert!(
            !flow.threads.iter().any(|thread| matches!(
                (thread.up, thread.lo),
                (FlowEnd::Peak { .. }, FlowEnd::Join)
            )),
            "and none bypasses it, because the redex covers the whole term"
        );
    }

    #[test]
    fn a_consumed_creation_is_one_thread_between_two_vertices()
    {
        // The dependent case: add-S at the root creates a `Succ` node with an
        // `Add` beneath it, and add-Z then consumes that `Add`. The occurrence
        // the first step created is the one the second destroys, so exactly the
        // thread that carries the dependence runs from vertex to vertex.
        let mut store = CellStore::new();
        let z = store.insert(add_z());
        let s = store.insert(add_s());
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::Zero);
        let flow = project_flow(&store, &peak, &alloc::vec![
            CellApp {
                cell: s,
                at: at([]),
            },
            CellApp {
                cell: z,
                at: at([0]),
            },
        ])
        .expect("add-S then add-Z is a derivation of the peak");
        assert_eq!(2, flow.labels.len(), "two vertices");
        assert!(
            flow.threads.iter().any(|thread| matches!(
                (thread.up, thread.lo),
                (FlowEnd::Vertex { .. }, FlowEnd::Vertex { .. })
            )),
            "the created-and-then-consumed occurrence is a vertex-to-vertex thread"
        );
    }

    #[test]
    fn the_projection_forgets_where_a_cell_fired()
    {
        // The definition's load-bearing choice, exercised: a vertex is labelled
        // by the CELL and not by where it fired. Two legs that fire one cell at
        // two different positions carry the same label, and what tells them
        // apart is which occurrences their threads touch — which is the
        // source's own arrangement of the data.
        let (store, f, _g) = cong2_store();
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero));
        let left = project_flow(&store, &peak, &alloc::vec![CellApp {
            cell: f,
            at: at([0]),
        }])
        .expect("f fires at the left argument");
        let right = project_flow(&store, &peak, &alloc::vec![CellApp {
            cell: f,
            at: at([1]),
        }])
        .expect("f fires at the right argument");
        let label = cell_address(store.get(f).expect("f is stored"));
        assert_eq!(
            alloc::vec![label],
            left.labels,
            "the vertex names the cell, not the position"
        );
        assert_eq!(left.labels, right.labels, "and the two legs agree on it");
        assert!(
            !bool::from(flows_equal(&left, &right)),
            "while the threads still tell the two firings apart, at the peak boundary"
        );
    }

    #[test]
    fn equal_flows_over_different_boundaries_are_not_one_certificate()
    {
        // THE CONTAINMENT, and the regression that fails if the boundary
        // conjunct is dropped again. A flow forgets the formula-level
        // arrangement, so one cell fired on two unrelated instances of its
        // left-hand side projects to ONE flow — same vertex label, same
        // occurrence count, same threads — over two boundaries that transform
        // different things into different things. Comparing the flows and
        // nothing else identifies them, and replay-equivalence does not, so a
        // flow-only relation is not inside replay-equivalence at all.
        let (store, composition) = fusion_fixture();
        let ground = one_step_certificate(
            &composition,
            Toy::add(Toy::Zero, Toy::Zero),
            Toy::Zero,
            CellApp {
                cell: composition.right,
                at: at([]),
            },
        );
        let schematic = one_step_certificate(
            &composition,
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("y"))),
            Toy::var(ToyNameRef("y")),
            CellApp {
                cell: composition.right,
                at: at([]),
            },
        );
        assert!(
            bool::from(ground.replay(&store)) && bool::from(schematic.replay(&store)),
            "both are certificates: add-Z fires on each peak and reaches each recorded join"
        );
        assert!(
            !bool::from(replay_equivalent(&ground, &schematic, &store)),
            "and they are two certificates, because their boundaries differ"
        );
        let ground_flow =
            project_flow(&store, &ground.overlap.peak, &ground.path_a).expect("the leg projects");
        let schematic_flow = project_flow(&store, &schematic.overlap.peak, &schematic.path_a)
            .expect("the leg projects");
        assert!(
            bool::from(flows_equal(&ground_flow, &schematic_flow)),
            "their two legs carry ONE flow — which is what makes this the sharp case"
        );
        assert!(
            !bool::from(
                tracelets_flow_equal(&ground, &schematic, &store)
                    .expect("both certificates project")
            ),
            "so the certificate-level relation must separate them on the boundary, or it is not \
             inside replay-equivalence"
        );
    }

    #[test]
    fn flow_equality_implies_replay_equivalence()
    {
        // The containment as a checked implication rather than a claim, over
        // every certificate this suite builds: the fused derivation, the
        // two-step presentation of the same boundary, and the two one-step
        // certificates whose flows coincide over different boundaries.
        let (mut store, composition) = fusion_fixture();
        let (_fused, fused_derivation) =
            derive_fused(&composition, &mut store).expect("the fused cell derives");
        let two_step = Tracelet {
            overlap: fused_derivation.overlap.clone(),
            path_a: fused_derivation.path_a.clone(),
            path_b: fused_derivation.path_a.clone(),
            joins_at: fused_derivation.joins_at.clone(),
        };
        let step = CellApp {
            cell: composition.right,
            at: at([]),
        };
        let ground = one_step_certificate(
            &composition,
            Toy::add(Toy::Zero, Toy::Zero),
            Toy::Zero,
            step.clone(),
        );
        let schematic = one_step_certificate(
            &composition,
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("y"))),
            Toy::var(ToyNameRef("y")),
            step,
        );
        let certificates = alloc::vec![fused_derivation, two_step, ground, schematic];
        for left in &certificates {
            for right in &certificates {
                let flow_equal = tracelets_flow_equal(left, right, &store)
                    .expect("every certificate in the set projects");
                if bool::from(flow_equal) {
                    assert!(
                        bool::from(replay_equivalent(left, right, &store)),
                        "flow equality is inside replay-equivalence, so a positive here forces a \
                         positive there"
                    );
                }
            }
        }
    }

    #[test]
    fn a_leg_that_lands_off_the_join_has_no_certificate_flow()
    {
        // The refusal the containment rests on. Every recorded step fires, so
        // the leg is a derivation — of the wrong boundary. A certificate whose
        // leg lands short of its recorded join does not replay, and projecting
        // it must refuse rather than hand back a flow that would then be
        // compared as if it stood for that boundary.
        let (store, composition) = fusion_fixture();
        let strays = one_step_certificate(
            &composition,
            Toy::add(Toy::Zero, Toy::Zero),
            Toy::succ(Toy::Zero),
            CellApp {
                cell: composition.right,
                at: at([]),
            },
        );
        assert!(
            !bool::from(strays.replay(&store)),
            "the recorded join is not where add-Z lands"
        );
        let obstruction =
            tracelet_flow(&strays, &store).expect_err("so the certificate is refused");
        assert_eq!(
            FlowObstruction::LegMissesTheJoin {
                reached: alloc::boxed::Box::new(Toy::Zero),
            },
            obstruction,
            "and the refusal carries where the leg actually landed"
        );
    }

    /// A one-step certificate over a boundary the caller chooses.
    ///
    /// The overlap is a carrier for the recorded peak and nothing else: both
    /// replay and the projection read `overlap.peak`, and these fixtures are
    /// about which boundary a certificate **records** rather than which
    /// critical pair produced it.
    fn one_step_certificate(
        carrier: &gandr_theory_coherent_resolutions::Overlap<ToyAlphabet>,
        peak: Toy,
        joins_at: Toy,
        step: CellApp<ToyAlphabet>,
    ) -> Tracelet<ToyAlphabet>
    {
        let mut overlap = carrier.clone();
        overlap.peak = peak;
        Tracelet {
            overlap,
            path_a: alloc::vec![step.clone()],
            path_b: alloc::vec![step],
            joins_at,
        }
    }

    /// The toy composition overlap `derive_fused` is exercised on, with its
    /// store.
    ///
    /// `add-S`'s right-hand side subterm `Add(m, n)` unifies with `add-Z`'s
    /// left-hand side, so the pair composes at a seam.
    fn fusion_fixture() -> (
        CellStore<ToyAlphabet>,
        gandr_theory_coherent_resolutions::Overlap<ToyAlphabet>,
    )
    {
        let mut store = CellStore::new();
        let z = store.insert(add_z());
        let s = store.insert(add_s());
        let composition = enumerate_overlaps(&store)
            .into_iter()
            .find(|overlap| {
                overlap.kind == OverlapKind::Composition && overlap.left == s && overlap.right == z
            })
            .expect("the composition overlap exists");
        (store, composition)
    }
}
