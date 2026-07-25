#[cfg(test)]
mod contracts
{
    use alloc::boxed::Box;
    use alloc::collections::BTreeSet;
    use alloc::format;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::error::Error;

    use gandr_theory_graphs::Assoc;
    use gandr_theory_graphs::Bound;
    use gandr_theory_graphs::NodeCount;
    use gandr_theory_graphs::Prec;
    use gandr_theory_graphs::PrecDag;
    use gandr_theory_graphs::PrecGroupCount;
    use gandr_theory_graphs::PrecIndex;
    use gandr_theory_graphs::PrecSpec;
    use gandr_theory_graphs::PrecSpecError;
    use proptest::prelude::*;

    #[test]
    fn prec_dag_size_and_boundary_contract() -> Result<(), Box<dyn Error>>
    {
        let empty_spec = PrecSpec::new();
        let empty_dag = PrecDag::build(&empty_spec)?;
        assert_eq!(PrecGroupCount::from(0), empty_dag.len());
        assert!(bool::from(empty_dag.is_empty()));
        assert_eq!(None, empty_dag.name(Prec::new(PrecIndex::from(0))));
        assert_eq!(None, empty_dag.assoc(Prec::new(PrecIndex::from(0))));
        assert!(!bool::from(empty_dag.lt(
            Prec::new(PrecIndex::from(0)),
            Prec::new(PrecIndex::from(0)),
            None
        )));
        assert!(!bool::from(empty_dag.gt(
            Prec::new(PrecIndex::from(0)),
            Prec::new(PrecIndex::from(0)),
            None
        )));
        assert!(!bool::from(empty_dag.eq(
            Prec::new(PrecIndex::from(0)),
            Prec::new(PrecIndex::from(0)),
            None
        )));
        assert!(!bool::from(empty_dag.comparable(
            Prec::new(PrecIndex::from(0)),
            Prec::new(PrecIndex::from(0))
        )));

        let mut single_spec = PrecSpec::new();
        let only = single_spec.insert("only", None)?;
        let single_dag = PrecDag::build(&single_spec)?;
        assert_eq!(PrecGroupCount::from(1), single_dag.len());
        assert!(!bool::from(single_dag.is_empty()));
        assert_eq!(Some("only"), single_dag.name(only).map(<&str>::from));
        assert_eq!(Some(None), single_dag.assoc(only));
        assert_eq!(None, single_dag.name(Prec::new(PrecIndex::from(1))));
        assert_eq!(None, single_dag.assoc(Prec::new(PrecIndex::from(1))));

        let (ordinary_dag, [loose, left_mid, right_mid, tight]) = diamond()?;
        assert_eq!(PrecGroupCount::from(4), ordinary_dag.len());
        assert!(!bool::from(ordinary_dag.is_empty()));
        assert_eq!(Some("tight"), ordinary_dag.name(tight).map(<&str>::from));
        assert_eq!(Some(Some(Assoc::Right)), ordinary_dag.assoc(right_mid));
        assert!(bool::from(ordinary_dag.lt(loose, tight, None)));
        let last_valid = Prec::new(PrecIndex::from(3));
        let one_past = Prec::new(PrecIndex::from(4));
        assert_eq!(
            Some("tight"),
            ordinary_dag.name(last_valid).map(<&str>::from)
        );
        assert_eq!(Some(None), ordinary_dag.assoc(last_valid));
        assert_eq!(None, ordinary_dag.name(one_past));
        assert_eq!(None, ordinary_dag.assoc(one_past));
        assert!(!bool::from(ordinary_dag.lt(one_past, tight, None)));
        assert!(!bool::from(ordinary_dag.gt(tight, one_past, None)));
        assert!(!bool::from(ordinary_dag.eq(one_past, one_past, None)));
        assert!(!bool::from(ordinary_dag.comparable(one_past, left_mid)));
        assert!(bool::from(ordinary_dag.bound_lt(
            Bound::Bottom,
            Bound::Value(last_valid),
            None
        )));
        assert!(!bool::from(ordinary_dag.bound_lt(
            Bound::Bottom,
            Bound::Value(one_past),
            None
        )));
        assert!(bool::from(
            ordinary_dag.bound_comparable(Bound::Value(last_valid), Bound::Root)
        ));
        assert!(!bool::from(
            ordinary_dag.bound_comparable(Bound::Value(one_past), Bound::Root)
        ));
        Ok(())
    }
    #[test]
    fn prec_dag_contract() -> Result<(), Box<dyn Error>>
    {
        let (dag, [loose, left_mid, right_mid, tight]) = diamond()?;

        assert!(bool::from(dag.lt(loose, left_mid, None)));
        assert!(bool::from(dag.lt(loose, right_mid, Some(Assoc::Left))));
        assert!(bool::from(dag.lt(left_mid, tight, Some(Assoc::Right))));
        assert!(bool::from(dag.lt(right_mid, tight, None)));
        assert!(bool::from(dag.lt(loose, tight, None)));
        assert!(bool::from(dag.gt(tight, loose, None)));

        assert!(!bool::from(dag.lt(left_mid, right_mid, None)));
        assert!(!bool::from(dag.gt(left_mid, right_mid, None)));
        assert!(!bool::from(dag.comparable(left_mid, right_mid)));
        assert!(bool::from(dag.comparable(loose, tight)));
        assert!(bool::from(dag.comparable(left_mid, left_mid)));

        assert!(bool::from(dag.gt(left_mid, left_mid, Some(Assoc::Left))));
        assert!(!bool::from(dag.gt(left_mid, left_mid, None)));
        assert!(bool::from(dag.lt(right_mid, right_mid, Some(Assoc::Right))));
        assert!(!bool::from(dag.lt(right_mid, right_mid, Some(Assoc::Left))));
        assert!(bool::from(dag.eq(loose, loose, None)));
        assert!(!bool::from(dag.eq(loose, loose, Some(Assoc::Right))));

        assert_eq!(Some("left-mid"), dag.name(left_mid).map(<&str>::from));
        assert_eq!(Some(Some(Assoc::Right)), dag.assoc(right_mid));
        assert_eq!(None, dag.name(Prec::new(PrecIndex::from(u16::MAX))));
        assert_eq!(None, dag.assoc(Prec::new(PrecIndex::from(u16::MAX))));
        assert!(!bool::from(dag.lt(
            Prec::new(PrecIndex::from(u16::MAX)),
            loose,
            None
        )));
        assert!(!bool::from(
            dag.comparable(Prec::new(PrecIndex::from(u16::MAX)), loose)
        ));

        let groups = dag
            .groups()
            .map(|(prec, name, assoc)| (prec, <&str>::from(name), assoc))
            .collect::<Vec<_>>();
        assert_eq!(
            &[
                (loose, "loose", None),
                (left_mid, "left-mid", Some(Assoc::Left)),
                (right_mid, "right-mid", Some(Assoc::Right)),
                (tight, "tight", None),
            ],
            groups.as_slice()
        );
        assert_eq!(
            vec![
                (left_mid, loose),
                (right_mid, loose),
                (tight, left_mid),
                (tight, right_mid),
            ],
            dag.edges().collect::<Vec<_>>()
        );
        Ok(())
    }
    fn diamond() -> Result<(PrecDag, [Prec; 4]), Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        let loose = spec.insert("loose", None)?;
        let left_mid = spec.insert("left-mid", Some(Assoc::Left))?;
        let right_mid = spec.insert("right-mid", Some(Assoc::Right))?;
        let tight = spec.insert("tight", None)?;
        spec.add_edge(tight, left_mid)?;
        spec.add_edge(tight, right_mid)?;
        spec.add_edge(left_mid, loose)?;
        spec.add_edge(right_mid, loose)?;
        let dag = PrecDag::build(&spec)?;
        Ok((dag, [loose, left_mid, right_mid, tight]))
    }
    #[test]
    fn prec_cycle_witness_contract() -> Result<(), Box<dyn Error>>
    {
        let mut self_spec = PrecSpec::new();
        let node = self_spec.insert("self", None)?;
        self_spec.add_edge(node, node)?;
        let self_cycle = PrecDag::build(&self_spec).expect_err("self edge must cycle");
        assert_eq!(self_cycle.witness, vec![node, node]);
        assert_closed_adjacent_cycle(&self_cycle.witness, &self_spec.edges().collect::<Vec<_>>());

        let mut spec = PrecSpec::new();
        let a = spec.insert("a", None)?;
        let b = spec.insert("b", None)?;
        let c = spec.insert("c", None)?;
        spec.add_edge(a, b)?;
        spec.add_edge(b, c)?;
        spec.add_edge(c, a)?;
        let cycle = PrecDag::build(&spec).expect_err("three-cycle must be rejected");
        assert_closed_adjacent_cycle(&cycle.witness, &spec.edges().collect::<Vec<_>>());
        assert!(format!("{cycle}").contains("precedence cycle"));
        Ok(())
    }
    fn assert_closed_adjacent_cycle(
        witness: &[Prec],
        edges: &[(Prec, Prec)],
    )
    {
        assert!(
            witness.len() >= 2,
            "cycle witness must be closed and non-empty"
        );
        assert_eq!(witness.first(), witness.last(), "cycle witness must close");
        let edge_set = edges.iter().copied().collect::<BTreeSet<_>>();
        for (from, to) in witness.iter().copied().zip(witness.iter().copied().skip(1)) {
            assert!(
                edge_set.contains(&(from, to)),
                "cycle witness adjacent pair must be an input edge"
            );
        }
    }
    #[test]
    fn virtual_bound_comparisons() -> Result<(), Box<dyn Error>>
    {
        let (dag, nodes) = integer_chain(&[None, None, None])?;
        let &[loose, _, tight] = nodes.as_slice()
        else {
            panic!("three-node chain oracle must expose exactly three nodes");
        };

        assert!(bool::from(dag.bound_lt(
            Bound::Bottom,
            Bound::Value(loose),
            None
        )));
        assert!(bool::from(dag.bound_lt(
            Bound::Value(loose),
            Bound::Value(tight),
            None
        )));
        assert!(bool::from(dag.bound_lt(
            Bound::Value(tight),
            Bound::Root,
            None
        )));
        assert!(bool::from(dag.bound_lt(Bound::Bottom, Bound::Root, None)));
        assert!(bool::from(dag.bound_gt(
            Bound::Root,
            Bound::Value(tight),
            None
        )));
        assert!(bool::from(dag.bound_eq(
            Bound::Bottom::<Prec>,
            Bound::Bottom,
            None
        )));
        assert!(bool::from(dag.bound_eq(
            Bound::Root::<Prec>,
            Bound::Root,
            None
        )));
        assert!(!bool::from(dag.bound_eq(
            Bound::Bottom::<Prec>,
            Bound::Root,
            None
        )));
        assert!(!bool::from(dag.bound_eq(
            Bound::Root::<Prec>,
            Bound::Bottom,
            None
        )));
        assert!(!bool::from(dag.bound_eq(
            Bound::Bottom,
            Bound::Value(loose),
            None
        )));
        assert!(!bool::from(dag.bound_eq(
            Bound::Value(loose),
            Bound::Bottom,
            None
        )));
        assert!(!bool::from(dag.bound_eq(
            Bound::Root,
            Bound::Value(tight),
            None
        )));
        assert!(!bool::from(dag.bound_eq(
            Bound::Value(tight),
            Bound::Root,
            None
        )));
        assert!(bool::from(
            dag.bound_comparable(Bound::Bottom, Bound::Value(loose))
        ));
        assert!(bool::from(
            dag.bound_comparable(Bound::Value(tight), Bound::Root)
        ));
        assert!(!bool::from(dag.bound_comparable(
            Bound::Value(Prec::new(PrecIndex::from(99))),
            Bound::Root
        )));
        Ok(())
    }
    #[test]
    fn prec_integer_chain_oracle() -> Result<(), Box<dyn Error>>
    {
        let (dag, nodes) = integer_chain(&[None, Some(Assoc::Left), Some(Assoc::Right)])?;
        let &[loose, left_assoc, right_assoc] = nodes.as_slice()
        else {
            panic!("three-node chain oracle must expose exactly three nodes");
        };
        assert!(bool::from(dag.eq(loose, loose, None)));
        assert!(bool::from(dag.gt(
            left_assoc,
            left_assoc,
            Some(Assoc::Left)
        )));
        assert!(bool::from(dag.lt(
            right_assoc,
            right_assoc,
            Some(Assoc::Right)
        )));
        let chain_nodes = [loose, left_assoc, right_assoc];
        let expected_lt = [[false, true, true], [false, false, true], [
            false, false, false,
        ]];
        let expected_greater = [[false, false, false], [true, false, false], [
            true, true, false,
        ]];
        let expected_comparable = [[true, true, true], [true, true, true], [true, true, true]];
        for (((left_node, lt_row), gt_row), comparable_row) in chain_nodes
            .into_iter()
            .zip(expected_lt)
            .zip(expected_greater)
            .zip(expected_comparable)
        {
            for (((right_node, lt_expected), gt_expected), comparable_expected) in chain_nodes
                .into_iter()
                .zip(lt_row)
                .zip(gt_row)
                .zip(comparable_row)
            {
                assert_eq!(lt_expected, bool::from(dag.lt(left_node, right_node, None)));
                assert_eq!(gt_expected, bool::from(dag.gt(left_node, right_node, None)));
                assert_eq!(
                    comparable_expected,
                    bool::from(dag.comparable(left_node, right_node))
                );
            }
        }
        Ok(())
    }
    fn integer_chain(assocs: &[Option<Assoc>]) -> Result<(PrecDag, Vec<Prec>), Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        let mut nodes = Vec::new();
        for (index, assoc) in assocs.iter().copied().enumerate() {
            let node = spec.insert(format!("p{index}"), assoc)?;
            nodes.push(node);
        }
        for (looser_node, tighter_node) in nodes.iter().copied().zip(nodes.iter().copied().skip(1))
        {
            spec.add_edge(tighter_node, looser_node)?;
        }
        let dag = PrecDag::build(&spec)?;
        Ok((dag, nodes))
    }

    #[test]
    fn prec_spec_size_and_boundary_contract() -> Result<(), Box<dyn Error>>
    {
        let empty = PrecSpec::new();
        assert_eq!(PrecGroupCount::from(0), empty.len());
        assert!(bool::from(empty.is_empty()));
        assert_eq!(None, empty.name(Prec::new(PrecIndex::from(0))));
        assert_eq!(None, empty.assoc(Prec::new(PrecIndex::from(0))));

        let mut spec = PrecSpec::new();
        let first = spec.insert("first", Some(Assoc::Left))?;
        let second = spec.insert("second", None)?;

        assert_eq!(PrecGroupCount::from(2), spec.len());
        assert!(!bool::from(spec.is_empty()));
        assert_eq!(Some("first"), spec.name(first).map(<&str>::from));
        assert_eq!(Some(Some(Assoc::Left)), spec.assoc(first));
        assert_eq!(Some("second"), spec.name(second).map(<&str>::from));
        assert_eq!(Some(None), spec.assoc(second));

        let last_valid = Prec::new(PrecIndex::from(1));
        let one_past = Prec::new(PrecIndex::from(2));
        assert_eq!(Some("second"), spec.name(last_valid).map(<&str>::from));
        assert_eq!(Some(None), spec.assoc(last_valid));
        assert_eq!(None, spec.name(one_past));
        assert_eq!(None, spec.assoc(one_past));
        assert_eq!(
            Err(PrecSpecError::InvalidEdge {
                tighter: one_past,
                looser: first,
                node_count: NodeCount::from(2),
            }),
            spec.add_edge(one_past, first)
        );
        Ok(())
    }

    #[test]
    fn duplicate_edge_canonicalization_and_invalid_edges() -> Result<(), Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        let loose = spec.insert("loose", None)?;
        let tight = spec.insert("tight", None)?;
        spec.add_edge(tight, loose)?;
        spec.add_edge(tight, loose)?;
        assert_eq!(vec![(tight, loose)], spec.edges().collect::<Vec<_>>());
        assert_eq!(
            Err(PrecSpecError::InvalidEdge {
                tighter: Prec::new(PrecIndex::from(99)),
                looser: loose,
                node_count: NodeCount::from(2),
            }),
            spec.add_edge(Prec::new(PrecIndex::from(99)), loose)
        );
        assert_eq!(
            Err(PrecSpecError::DuplicateName {
                name: "tight".to_owned(),
            }),
            spec.insert("tight", None)
        );
        let dag = PrecDag::build(&spec)?;
        assert_eq!(vec![(tight, loose)], dag.edges().collect::<Vec<_>>());
        Ok(())
    }

    #[test]
    fn capacity_beyond_u16_is_typed() -> Result<(), Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        for index in 0_u32 ..= u32::from(u16::MAX) {
            let id = spec.insert(format!("p{index}"), None)?;
            assert_eq!(index, u32::from(id.index()));
        }
        assert_eq!(
            Err(PrecSpecError::CapacityExceeded),
            spec.insert("overflow", None)
        );
        Ok(())
    }

    #[test]
    fn bound_value_reflexive_association_tracks_direction() -> Result<(), Box<dyn Error>>
    {
        let mut left_spec = PrecSpec::new();
        let left_p = left_spec.insert("p", Some(Assoc::Left))?;
        let neutral = left_spec.insert("neutral", None)?;
        let left_dag = PrecDag::build(&left_spec)?;

        assert!(bool::from(left_dag.gt(left_p, left_p, Some(Assoc::Left))));
        assert!(bool::from(left_dag.bound_gt(
            Bound::Value(left_p),
            Bound::Value(left_p),
            Some(Assoc::Left)
        )));
        assert!(!bool::from(left_dag.lt(left_p, left_p, Some(Assoc::Left))));
        assert!(!bool::from(left_dag.bound_lt(
            Bound::Value(left_p),
            Bound::Value(left_p),
            Some(Assoc::Left)
        )));
        assert!(bool::from(left_dag.bound_eq(
            Bound::Value(neutral),
            Bound::Value(neutral),
            None
        )));
        assert!(!bool::from(left_dag.bound_eq(
            Bound::Value(left_p),
            Bound::Value(left_p),
            Some(Assoc::Left)
        )));

        let mut right_spec = PrecSpec::new();
        let right_p = right_spec.insert("p", Some(Assoc::Right))?;
        let right_dag = PrecDag::build(&right_spec)?;

        assert!(bool::from(right_dag.lt(
            right_p,
            right_p,
            Some(Assoc::Right)
        )));
        assert!(bool::from(right_dag.bound_lt(
            Bound::Value(right_p),
            Bound::Value(right_p),
            Some(Assoc::Right)
        )));
        assert!(!bool::from(right_dag.gt(
            right_p,
            right_p,
            Some(Assoc::Right)
        )));
        assert!(!bool::from(right_dag.bound_gt(
            Bound::Value(right_p),
            Bound::Value(right_p),
            Some(Assoc::Right)
        )));
        assert!(!bool::from(right_dag.bound_eq(
            Bound::Value(right_p),
            Bound::Value(right_p),
            Some(Assoc::Right)
        )));
        Ok(())
    }

    #[test]
    fn stable_fingerprint_sensitivity() -> Result<(), Box<dyn Error>>
    {
        let mut left = PrecSpec::new();
        let a = left.insert("a", None)?;
        let b = left.insert("b", Some(Assoc::Left))?;
        let c = left.insert("c", Some(Assoc::Right))?;
        left.add_edge(c, b)?;
        left.add_edge(b, a)?;
        left.add_edge(c, b)?;

        let mut reordered = PrecSpec::new();
        let ar = reordered.insert("a", None)?;
        let br = reordered.insert("b", Some(Assoc::Left))?;
        let cr = reordered.insert("c", Some(Assoc::Right))?;
        reordered.add_edge(br, ar)?;
        reordered.add_edge(cr, br)?;

        let mut renamed = PrecSpec::new();
        let an = renamed.insert("a-renamed", None)?;
        let bn = renamed.insert("b", Some(Assoc::Left))?;
        let cn = renamed.insert("c", Some(Assoc::Right))?;
        renamed.add_edge(cn, bn)?;
        renamed.add_edge(bn, an)?;

        let mut assoc_changed = PrecSpec::new();
        let assoc_loose = assoc_changed.insert("a", None)?;
        let assoc_middle = assoc_changed.insert("b", None)?;
        let assoc_tight = assoc_changed.insert("c", Some(Assoc::Right))?;
        assoc_changed.add_edge(assoc_tight, assoc_middle)?;
        assoc_changed.add_edge(assoc_middle, assoc_loose)?;

        let mut relation_changed = PrecSpec::new();
        let arel = relation_changed.insert("a", None)?;
        let brel = relation_changed.insert("b", Some(Assoc::Left))?;
        let crel = relation_changed.insert("c", Some(Assoc::Right))?;
        relation_changed.add_edge(crel, arel)?;
        relation_changed.add_edge(brel, arel)?;

        let left_dag = PrecDag::build(&left)?;
        let reordered_dag = PrecDag::build(&reordered)?;
        let renamed_dag = PrecDag::build(&renamed)?;
        let assoc_changed_dag = PrecDag::build(&assoc_changed)?;
        let relation_changed_dag = PrecDag::build(&relation_changed)?;
        let left_hash = left_dag.fingerprint();
        let reordered_hash = reordered_dag.fingerprint();
        let renamed_hash = renamed_dag.fingerprint();
        let assoc_changed_hash = assoc_changed_dag.fingerprint();
        let relation_changed_hash = relation_changed_dag.fingerprint();
        assert_eq!(left_hash, reordered_hash);
        assert_ne!(left_hash, renamed_hash);
        assert_ne!(left_hash, assoc_changed_hash);
        assert_ne!(left_hash, relation_changed_hash);
        Ok(())
    }

    #[test]
    fn deterministic_linear_extension_uses_smallest_ready_id() -> Result<(), Box<dyn Error>>
    {
        let mut spec = PrecSpec::new();
        let a = spec.insert("a", None)?;
        let b = spec.insert("b", None)?;
        let c = spec.insert("c", None)?;
        let d = spec.insert("d", None)?;
        spec.add_edge(c, d)?;
        let dag = PrecDag::build(&spec)?;
        assert_eq!(vec![a, b, c, d], dag.linear_extension());
        Ok(())
    }

    proptest! {
        #[test]
        fn lt_gt_duality_for_distinct_chain_nodes(chain_size in 2_usize..8, alpha_draw in 0_usize..8, omega_draw in 0_usize..8) {
            let assocs = vec![None; chain_size];
            let (dag, nodes) = integer_chain(&assocs).expect("chain builds");
            prop_assert_eq!(chain_size, nodes.len());
            let alpha_index = alpha_draw % chain_size;
            let omega_index = omega_draw % chain_size;
            prop_assume!(alpha_index != omega_index);
            let alpha_node = nodes.get(alpha_index).copied().expect("alpha index is modulo chain size");
            let omega_node = nodes.get(omega_index).copied().expect("omega index is modulo chain size");
            prop_assert_eq!(dag.lt(alpha_node, omega_node, None), dag.gt(omega_node, alpha_node, Some(Assoc::Left)));
        }

        #[test]
        fn comparable_is_symmetric_and_rejects_invalid_boundaries(chain_size in 1_usize..8, alpha_draw in 0_usize..8, omega_draw in 0_usize..8) {
            let assocs = vec![None; chain_size];
            let (dag, nodes) = integer_chain(&assocs).expect("chain builds");
            prop_assert_eq!(chain_size, nodes.len());
            let alpha_node = nodes
                .get(alpha_draw % chain_size)
                .copied()
                .expect("alpha index is modulo chain size");
            let omega_node = nodes
                .get(omega_draw % chain_size)
                .copied()
                .expect("omega index is modulo chain size");
            prop_assert_eq!(dag.comparable(alpha_node, omega_node), dag.comparable(omega_node, alpha_node));
            prop_assert!(bool::from(dag.bound_comparable(Bound::Bottom, Bound::Value(alpha_node))));
            prop_assert!(bool::from(dag.bound_comparable(Bound::Value(omega_node), Bound::Root)));
            prop_assert!(!bool::from(dag.comparable(Prec::new(PrecIndex::from(u16::MAX)), alpha_node)));
            prop_assert!(!bool::from(dag.bound_comparable(Bound::Value(Prec::new(PrecIndex::from(u16::MAX))), Bound::Bottom)));
        }

        #[test]
        fn associativity_affects_reflexive_pairs_only(chain_size in 2_usize..8, alpha_draw in 0_usize..8, omega_draw in 0_usize..8) {
            let pattern = [None, Some(Assoc::Left), Some(Assoc::Right)];
            let assocs = pattern.into_iter().cycle().take(chain_size).collect::<Vec<_>>();
            let (dag, nodes) = integer_chain(&assocs).expect("chain builds");
            prop_assert_eq!(chain_size, nodes.len());
            let alpha_index = alpha_draw % chain_size;
            let omega_index = omega_draw % chain_size;
            let alpha_node = nodes.get(alpha_index).copied().expect("alpha index is modulo chain size");
            let omega_node = nodes.get(omega_index).copied().expect("omega index is modulo chain size");
            if alpha_index == omega_index {
                match assocs.get(alpha_index).copied().expect("alpha index is modulo chain size") {
                    None => prop_assert!(bool::from(dag.eq(alpha_node, alpha_node, None))),
                    Some(Assoc::Left) => prop_assert!(bool::from(dag.gt(alpha_node, alpha_node, Some(Assoc::Left)))),
                    Some(Assoc::Right) => prop_assert!(bool::from(dag.lt(alpha_node, alpha_node, Some(Assoc::Right)))),
                    Some(_future) => prop_assert!(false, "unexpected future associativity variant"),
                }
            }
            else {
                let none = dag.lt(alpha_node, omega_node, None);
                prop_assert_eq!(none, dag.lt(alpha_node, omega_node, Some(Assoc::Left)));
                prop_assert_eq!(none, dag.lt(alpha_node, omega_node, Some(Assoc::Right)));
                let none_gt = dag.gt(alpha_node, omega_node, None);
                prop_assert_eq!(none_gt, dag.gt(alpha_node, omega_node, Some(Assoc::Left)));
                prop_assert_eq!(none_gt, dag.gt(alpha_node, omega_node, Some(Assoc::Right)));
            }
        }
    }
}
