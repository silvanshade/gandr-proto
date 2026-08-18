//! Integration witnesses for the static pathway engine's obstruction handling.
//!
//! The engine drops a speculative composite that is merely not a pathway, which
//! is what keeps one bad candidate from failing a whole query. The risk that
//! creates is the opposite one: an obstruction that reports a **defect** being
//! swallowed by the same path and reading as "no pathway found". These tests
//! hold the line between the two against the real normalizer rather than
//! against a classification table.

#[cfg(test)]
mod tests
{
    extern crate alloc;

    use alloc::vec::Vec;

    use gandr_theory_cell_complexes::cell::CellStore;
    use gandr_theory_cell_complexes_tools::adversarial::IncomparablePositions;
    use gandr_theory_cell_complexes_tools::adversarial::Lying;
    use gandr_theory_cell_complexes_tools::adversarial::lying_cell;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyPos;
    use gandr_theory_coherent_resolutions::rewrite::CellApp;
    use gandr_theory_decomposition_spaces::pathway::certify_candidate;
    use gandr_theory_deep_inference::normal_form::NormalFormObstruction;
    use gandr_theory_deep_inference::normal_form::prim_address;

    /// A toy position from child indices.
    fn at<Steps>(steps: Steps) -> ToyPos
    where
        Steps: IntoIterator<Item = usize>,
    {
        ToyPos(steps.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    /// **A kill signal reaches the caller rather than reading as no pathway.**
    ///
    /// The alphabet answers `Incomparable` for every position pair, so the
    /// independence relation licenses a transposition the semantics does not
    /// have. All three applications land in one layer, the canonical schedule
    /// leads with the root, and the root's redex does not exist until the
    /// leaves reduce — so the schedule cannot fire and the normalizer raises
    /// `ShiftedScheduleDoesNotFire`.
    ///
    /// The engine must **propagate** that. Dropping it would report an empty
    /// answer computed over a broken independence relation, which is the exact
    /// reading the kill signal exists to prevent.
    #[test]
    fn a_kill_signal_stops_the_query_rather_than_refusing_a_candidate()
    {
        let mut store: CellStore<Lying<IncomparablePositions>> = CellStore::new();
        let c = store.insert(lying_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero));
        let peak = Toy::add(
            Toy::add(Toy::Zero, Toy::Zero),
            Toy::add(Toy::Zero, Toy::Zero),
        );
        let root = CellApp {
            cell: c,
            at: at([]),
        };
        let recorded = alloc::vec![
            CellApp {
                cell: c,
                at: at([0]),
            },
            CellApp {
                cell: c,
                at: at([1]),
            },
            root.clone(),
        ];
        // NON-VACUITY. The arm reached is `DoesNotFire` rather than
        // `MissesTheJoin` only because the root application's content address
        // sorts first. Retuning the digest would silently move this fixture to
        // a different arm, so the ordering is asserted rather than assumed.
        let cell = store.get(c).expect("the cell is stored");
        let root_address = prim_address(cell, &at([]));
        let left_address = prim_address(cell, &at([0]));
        let right_address = prim_address(cell, &at([1]));
        assert!(
            root_address < left_address.max(right_address),
            "the fixture needs the root application not to come last in the flattened layer"
        );
        let raised = certify_candidate(&store, &peak, &Toy::Zero, &recorded)
            .expect_err("the kill signal must reach the caller");
        assert_eq!(
            NormalFormObstruction::ShiftedScheduleDoesNotFire {
                step: alloc::boxed::Box::new(root)
            },
            raised,
            "the propagated obstruction is the kill signal, naming the step that carried no redex"
        );
    }

    /// The same call shape, over an ordinary refusal, answers `Ok(None)`.
    ///
    /// This is the other half of the line: were the engine to propagate every
    /// obstruction instead, one non-replaying speculative composite would fail
    /// an entire query.
    #[test]
    fn an_ordinary_non_replaying_candidate_is_refused_without_failing()
    {
        let mut store: CellStore<Lying<IncomparablePositions>> = CellStore::new();
        let c = store.insert(lying_cell(Toy::add(Toy::Zero, Toy::Zero), Toy::Zero));
        let peak = Toy::add(Toy::Zero, Toy::Zero);
        let recorded = alloc::vec![CellApp {
            cell: c,
            at: at([]),
        }];
        // The path fires and reaches `Zero`; asking it to land on the peak
        // instead is a candidate that does not replay.
        let refused = certify_candidate(&store, &peak, &peak, &recorded)
            .expect("a candidate that misses its join is refused, not fatal");
        assert!(refused.is_none());
    }
}
