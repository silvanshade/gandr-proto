//! Public differential witnesses for the causal-web analysis surface.
//!
//! These fixtures build canonical event orders through the same tracelet replay
//! entry point used by the deep-inference suite, then compare every web colour
//! with the source [`EventOrder`]. The final cases pin the named refusal
//! boundary without entering evaluation, replay, or wire-format code.

#[cfg(test)]
mod tests
{
    use alloc::vec;
    use alloc::vec::Vec;

    use gandr_theory_cell_complexes::Cell;
    use gandr_theory_cell_complexes::CellStore;
    use gandr_theory_cell_complexes_tools::toy::Toy;
    use gandr_theory_cell_complexes_tools::toy::ToyAlphabet;
    use gandr_theory_cell_complexes_tools::toy::ToyNameRef;
    use gandr_theory_cell_complexes_tools::toy::ToyPos;
    use gandr_theory_cell_complexes_tools::toy::toy_cell;
    use gandr_theory_coherent_resolutions::CellApp;
    use gandr_theory_deep_inference::CausalWeb;
    use gandr_theory_deep_inference::EventOrder;
    use gandr_theory_deep_inference::HomomorphismFrontier;
    use gandr_theory_deep_inference::RefinementVerdict;
    use gandr_theory_deep_inference::WebRelation;
    use gandr_theory_deep_inference::WebVertex;
    use gandr_theory_deep_inference::causal_web;
    use gandr_theory_deep_inference::event_order;
    use gandr_theory_deep_inference::refines;

    extern crate alloc;

    fn at<Steps>(steps: Steps) -> ToyPos
    where
        Steps: IntoIterator<Item = usize>,
    {
        ToyPos(steps.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    fn root() -> ToyPos
    {
        ToyPos(Vec::new().into_boxed_slice())
    }

    fn two_event_order(cell: Cell<ToyAlphabet>) -> EventOrder<ToyAlphabet>
    {
        let mut store = CellStore::new();
        let cell = store.insert(cell);
        let peak = Toy::add(Toy::succ(Toy::Zero), Toy::succ(Toy::Zero));
        let path = vec![CellApp { cell, at: at([0]) }, CellApp { cell, at: at([1]) }];
        event_order(&store, &peak, &path).expect("the two-event fixture replays")
    }

    fn one_event_order(cell: Cell<ToyAlphabet>) -> EventOrder<ToyAlphabet>
    {
        let mut store = CellStore::new();
        let cell = store.insert(cell);
        let peak = Toy::succ(Toy::Zero);
        let path = vec![CellApp { cell, at: root() }];
        event_order(&store, &peak, &path).expect("the one-event fixture replays")
    }

    fn dependent_order() -> EventOrder<ToyAlphabet>
    {
        let mut store = CellStore::new();
        let cell = store.insert(toy_cell(
            Toy::add(Toy::Zero, Toy::var(ToyNameRef("x"))),
            Toy::var(ToyNameRef("x")),
        ));
        let peak = Toy::add(Toy::Zero, Toy::add(Toy::Zero, Toy::Zero));
        let path = vec![CellApp { cell, at: root() }, CellApp { cell, at: root() }];
        event_order(&store, &peak, &path).expect("the dependent fixture replays")
    }

    fn f_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::Zero), Toy::Zero)
    }

    fn alternate_cell() -> Cell<ToyAlphabet>
    {
        toy_cell(Toy::succ(Toy::Zero), Toy::succ(Toy::succ(Toy::Zero)))
    }

    fn assert_web_matches_order(
        order: &EventOrder<ToyAlphabet>,
        web: &CausalWeb,
    )
    {
        let canonical = order.canonical_order();
        assert_eq!(canonical.len(), web.events.len());
        for (vertex, event_index) in canonical.iter().copied().enumerate() {
            assert_eq!(
                order.key(event_index).as_ref(),
                web.event(WebVertex::from(vertex)),
            );
        }
        for (left_index, left) in canonical.iter().copied().enumerate() {
            for (right_index, right) in canonical.iter().copied().enumerate() {
                if left_index == right_index {
                    continue;
                }
                let expected = if bool::from(order.precedes(left, right)) {
                    WebRelation::Precedes
                }
                else if bool::from(order.precedes(right, left)) {
                    WebRelation::Follows
                }
                else {
                    WebRelation::Independent
                };
                assert_eq!(
                    expected,
                    web.relation(WebVertex::from(left_index), WebVertex::from(right_index),),
                );
            }
        }
    }

    #[test]
    fn independent_tracelet_web_matches_canonical_event_order()
    {
        let order = two_event_order(f_cell());
        let web = causal_web(&order);
        assert_web_matches_order(&order, &web);
        assert_eq!(
            WebRelation::Independent,
            web.relation(WebVertex::from(0), WebVertex::from(1)),
        );
        let RefinementVerdict::Refines { witness } = refines(&web, &web)
        else {
            panic!("an identical public web must refine itself");
        };
        assert_eq!(0_usize, usize::from(witness.step_count()));
    }

    #[test]
    fn dependent_tracelet_web_matches_canonical_event_order()
    {
        let order = dependent_order();
        let web = causal_web(&order);
        assert_web_matches_order(&order, &web);
        assert_eq!(
            WebRelation::Precedes,
            web.relation(WebVertex::from(0), WebVertex::from(1))
        );
        assert_eq!(
            WebRelation::Follows,
            web.relation(WebVertex::from(1), WebVertex::from(0))
        );
    }

    #[test]
    fn refusal_frontiers_remain_named_in_public_api()
    {
        let same_cardinality_mismatch = causal_web(&two_event_order(alternate_cell()));
        let source = causal_web(&two_event_order(f_cell()));
        assert_eq!(
            RefinementVerdict::Refused {
                obstruction: HomomorphismFrontier::EdgeStrengtheningSimulation,
            },
            refines(&same_cardinality_mismatch, &source),
        );
        let different_cardinality = causal_web(&one_event_order(f_cell()));
        assert_eq!(
            RefinementVerdict::Refused {
                obstruction: HomomorphismFrontier::OpenHDownHomomorphism,
            },
            refines(&different_cardinality, &source),
        );
        let malformed = CausalWeb {
            events: Vec::new().into_boxed_slice(),
            precedes: source.precedes.clone(),
        };
        assert_eq!(
            RefinementVerdict::Refused {
                obstruction: HomomorphismFrontier::MalformedWeb,
            },
            refines(&malformed, &source),
        );
    }
}
