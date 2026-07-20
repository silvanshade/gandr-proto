#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps graph algorithm tests readable (docs/workflow/rust.md)"
    )
)]

use core::error::Error;

use gandr_theory_graphs::BlockIndex;
use gandr_theory_graphs::EdgeSource;
use gandr_theory_graphs::GraphValidationError;
use gandr_theory_graphs::NodeCount;
use gandr_theory_graphs::NodeId;
use gandr_theory_graphs::Simulation;
use gandr_theory_graphs::bisimulation_partition;
use gandr_theory_graphs::cycle_witness;
use gandr_theory_graphs::simulation_relation;
use proptest::prelude::*;

static EMPTY: [NodeId; 0] = [];

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TestEdges<'edges>(&'edges [(u32, u32)]);

impl<'edges> From<&'edges [(u32, u32)]> for TestEdges<'edges>
{
    #[inline]
    fn from(value: &'edges [(u32, u32)]) -> Self
    {
        Self(value)
    }
}

impl<'edges, const N: usize> From<&'edges [(u32, u32); N]> for TestEdges<'edges>
{
    #[inline]
    fn from(value: &'edges [(u32, u32); N]) -> Self
    {
        Self(value.as_slice())
    }
}

impl<'edges> From<&'edges Vec<(u32, u32)>> for TestEdges<'edges>
{
    #[inline]
    fn from(value: &'edges Vec<(u32, u32)>) -> Self
    {
        Self(value.as_slice())
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GraphBits<'bits>(&'bits [bool]);

impl<'bits> From<&'bits [bool]> for GraphBits<'bits>
{
    #[inline]
    fn from(value: &'bits [bool]) -> Self
    {
        Self(value)
    }
}

impl<'bits> From<&'bits Vec<bool>> for GraphBits<'bits>
{
    #[inline]
    fn from(value: &'bits Vec<bool>) -> Self
    {
        Self(value.as_slice())
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CanonicalNodePresent(bool);

impl From<bool> for CanonicalNodePresent
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<CanonicalNodePresent> for bool
{
    #[inline]
    fn from(value: CanonicalNodePresent) -> Self
    {
        value.0
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CanonicalEdgePresent(bool);

impl From<bool> for CanonicalEdgePresent
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<CanonicalEdgePresent> for bool
{
    #[inline]
    fn from(value: CanonicalEdgePresent) -> Self
    {
        value.0
    }
}

#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct CanonicalAdjacency(Vec<Vec<NodeId>>);

impl CanonicalAdjacency
{
    #[inline]
    fn contains_node(
        &self,
        node: NodeId,
    ) -> CanonicalNodePresent
    {
        CanonicalNodePresent::from(
            usize::try_from(u32::from(node)).is_ok_and(|position| position < self.0.len()),
        )
    }

    #[inline]
    fn contains_edge(
        &self,
        source: NodeId,
        target: NodeId,
    ) -> CanonicalEdgePresent
    {
        CanonicalEdgePresent::from(
            usize::try_from(u32::from(source))
                .ok()
                .and_then(|position| self.0.get(position))
                .is_some_and(|row| row.contains(&target)),
        )
    }

    #[inline]
    fn targets(
        &self,
        source: NodeId,
    ) -> &[NodeId]
    {
        usize::try_from(u32::from(source))
            .ok()
            .and_then(|position| self.0.get(position))
            .map_or(EMPTY.as_slice(), Vec::as_slice)
    }
}

#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct SimulationPairs(Vec<(u32, Vec<u32>)>);

impl PartialEq<Vec<(u32, Vec<u32>)>> for SimulationPairs
{
    #[inline]
    fn eq(
        &self,
        other: &Vec<(u32, Vec<u32>)>,
    ) -> bool
    {
        &self.0 == other
    }
}

impl PartialEq<SimulationPairs> for Vec<(u32, Vec<u32>)>
{
    #[inline]
    fn eq(
        &self,
        other: &SimulationPairs,
    ) -> bool
    {
        self == &other.0
    }
}

#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct OracleRelation(Vec<Vec<bool>>);

impl core::ops::Index<usize> for OracleRelation
{
    type Output = Vec<bool>;

    #[inline]
    fn index(
        &self,
        index: usize,
    ) -> &Self::Output
    {
        &self.0[index]
    }
}

impl core::ops::IndexMut<usize> for OracleRelation
{
    #[inline]
    fn index_mut(
        &mut self,
        index: usize,
    ) -> &mut Self::Output
    {
        &mut self.0[index]
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SimulationStepMatched(bool);

impl From<bool> for SimulationStepMatched
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<SimulationStepMatched> for bool
{
    #[inline]
    fn from(value: SimulationStepMatched) -> Self
    {
        value.0
    }
}

#[derive(Clone, Debug)]
struct TestGraph
{
    node_count: u32,
    rows: Vec<Vec<NodeId>>,
}

impl TestGraph
{
    fn new<'edges, N, E>(
        node_count: N,
        edges: E,
    ) -> Self
    where
        N: Into<NodeCount>,
        E: Into<TestEdges<'edges>>,
    {
        let node_count = node_count.into();
        let mut rows = Vec::new();
        for _node in node_count.ids() {
            rows.push(Vec::new());
        }
        let edges = edges.into();
        for &(source, target) in edges.0 {
            if let Ok(position) = usize::try_from(source)
                && let Some(row) = rows.get_mut(position)
            {
                row.push(NodeId::from(target));
            }
        }
        Self {
            node_count: u32::from(node_count),
            rows,
        }
    }

    fn canonical_adjacency(&self) -> CanonicalAdjacency
    {
        let mut rows = self.rows.clone();
        for row in &mut rows {
            row.sort_unstable();
            row.dedup();
        }
        CanonicalAdjacency(rows)
    }
}

impl EdgeSource for TestGraph
{
    type Successors<'successors>
        = core::iter::Copied<core::slice::Iter<'successors, NodeId>>
    where
        Self: 'successors;

    fn node_count(&self) -> NodeCount
    {
        NodeCount::from(self.node_count)
    }

    fn successors(
        &self,
        node: NodeId,
    ) -> Self::Successors<'_>
    {
        usize::try_from(u32::from(node))
            .ok()
            .and_then(|position| self.rows.get(position))
            .map_or_else(|| EMPTY.iter().copied(), |row| row.iter().copied())
    }
}

#[cfg(test)]
mod contracts
{
    use super::*;

    #[test]
    fn partition_refinement_contract() -> Result<(), Box<dyn Error>>
    {
        let empty = TestGraph::new(0, &[]);
        let empty_partition = bisimulation_partition(&empty)?;
        assert!(
            empty_partition.blocks().is_empty(),
            "empty graph has no blocks"
        );
        let empty_simulation = simulation_relation(&empty)?;
        assert!(
            empty_simulation.rows().is_empty(),
            "empty graph has no rows"
        );

        let singleton = TestGraph::new(1, &[]);
        let singleton_partition = bisimulation_partition(&singleton)?;
        assert_eq!(
            vec![vec![0]],
            singleton_partition
                .blocks()
                .iter()
                .map(|block| block.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        let singleton_block = singleton_partition.block_of(NodeId::from(0))?;
        assert_eq!(BlockIndex::from(0), singleton_block);
        let singleton_reflexive =
            singleton_partition.equivalent(NodeId::from(0), NodeId::from(0))?;
        assert!(bool::from(singleton_reflexive));
        assert!(matches!(
            singleton_partition.block_of(NodeId::from(1)),
            Err(GraphValidationError::NodeOutOfBounds {
                node,
                node_count,
            }) if node == NodeId::from(1) && node_count == NodeCount::from(1)
        ));
        let singleton_simulation = simulation_relation(&singleton)?;
        let singleton_self_simulates =
            singleton_simulation.is_simulated_by(NodeId::from(0), NodeId::from(0))?;
        assert!(bool::from(singleton_self_simulates));
        let singleton_candidates = singleton_simulation.candidates_for(NodeId::from(0))?;
        assert_eq!(
            vec![0],
            singleton_candidates
                .iter()
                .copied()
                .map(u32::from)
                .collect::<Vec<_>>()
        );
        assert!(matches!(
            singleton_simulation.is_simulated_by(NodeId::from(0), NodeId::from(1)),
            Err(GraphValidationError::NodeOutOfBounds {
                node,
                node_count,
            }) if node == NodeId::from(1) && node_count == NodeCount::from(1)
        ));

        let deadlocks = TestGraph::new(3, &[]);
        let deadlock_partition = bisimulation_partition(&deadlocks)?;
        assert_eq!(
            vec![vec![0, 1, 2]],
            deadlock_partition
                .blocks()
                .iter()
                .map(|block| block.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        let deadlock_simulation = simulation_relation(&deadlocks)?;
        assert_eq!(
            vec![(0, vec![0, 1, 2]), (1, vec![0, 1, 2]), (2, vec![0, 1, 2])],
            rows_as_pairs(&deadlock_simulation)
        );

        let chain = TestGraph::new(3, &[(0, 1), (1, 2)]);
        let chain_partition = bisimulation_partition(&chain)?;
        assert_eq!(
            vec![vec![0], vec![1], vec![2]],
            chain_partition
                .blocks()
                .iter()
                .map(|block| block.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        let chain_distinguishes_zero_one =
            chain_partition.equivalent(NodeId::from(0), NodeId::from(1))?;
        assert!(!bool::from(chain_distinguishes_zero_one));
        let chain_simulation = simulation_relation(&chain)?;
        let chain_zero_self = chain_simulation.is_simulated_by(NodeId::from(0), NodeId::from(0))?;
        let chain_zero_by_one =
            chain_simulation.is_simulated_by(NodeId::from(0), NodeId::from(1))?;
        let chain_one_by_zero =
            chain_simulation.is_simulated_by(NodeId::from(1), NodeId::from(0))?;
        let chain_two_by_one =
            chain_simulation.is_simulated_by(NodeId::from(2), NodeId::from(1))?;
        assert!(bool::from(chain_zero_self));
        assert!(!bool::from(chain_zero_by_one));
        assert!(bool::from(chain_one_by_zero));
        assert!(bool::from(chain_two_by_one));
        assert_eq!(
            vec![(0, vec![0]), (1, vec![0, 1]), (2, vec![0, 1, 2])],
            rows_as_pairs(&chain_simulation)
        );

        let branching = TestGraph::new(5, &[(0, 1), (0, 2), (3, 1), (4, 2)]);
        let branch_partition = bisimulation_partition(&branching)?;
        assert_eq!(
            vec![vec![0, 3, 4], vec![1, 2]],
            branch_partition
                .blocks()
                .iter()
                .map(|block| block.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        let branch_zero_three = branch_partition.equivalent(NodeId::from(0), NodeId::from(3))?;
        let branch_three_four = branch_partition.equivalent(NodeId::from(3), NodeId::from(4))?;
        let branch_zero_one = branch_partition.equivalent(NodeId::from(0), NodeId::from(1))?;
        assert!(bool::from(branch_zero_three));
        assert!(bool::from(branch_three_four));
        assert!(!bool::from(branch_zero_one));
        let branch_simulation = simulation_relation(&branching)?;
        assert_eq!(
            vec![
                (0, vec![0, 3, 4]),
                (1, vec![0, 1, 2, 3, 4]),
                (2, vec![0, 1, 2, 3, 4]),
                (3, vec![0, 3, 4]),
                (4, vec![0, 3, 4]),
            ],
            rows_as_pairs(&branch_simulation)
        );

        let cycle = TestGraph::new(4, &[(0, 1), (1, 0), (2, 3), (3, 2)]);
        let cycle_partition = bisimulation_partition(&cycle)?;
        assert_eq!(
            vec![vec![0, 1, 2, 3]],
            cycle_partition
                .blocks()
                .iter()
                .map(|block| block.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        let cycle_simulation = simulation_relation(&cycle)?;
        assert_eq!(
            vec![
                (0, vec![0, 1, 2, 3]),
                (1, vec![0, 1, 2, 3]),
                (2, vec![0, 1, 2, 3]),
                (3, vec![0, 1, 2, 3]),
            ],
            rows_as_pairs(&cycle_simulation)
        );

        let disconnected = TestGraph::new(6, &[(0, 1), (2, 3), (4, 5), (5, 4)]);
        let disconnected_partition = bisimulation_partition(&disconnected)?;
        assert_eq!(
            vec![vec![0, 2], vec![1, 3], vec![4, 5]],
            disconnected_partition
                .blocks()
                .iter()
                .map(|block| block.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );

        let duplicates = TestGraph::new(4, &[(0, 2), (0, 1), (0, 1), (1, 3), (2, 3), (2, 3)]);
        let duplicate_partition = bisimulation_partition(&duplicates)?;
        assert_eq!(
            vec![vec![0], vec![1, 2], vec![3]],
            duplicate_partition
                .blocks()
                .iter()
                .map(|block| block.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        let duplicate_simulation = simulation_relation(&duplicates)?;
        let duplicate_one_by_two =
            duplicate_simulation.is_simulated_by(NodeId::from(1), NodeId::from(2))?;
        let duplicate_two_by_one =
            duplicate_simulation.is_simulated_by(NodeId::from(2), NodeId::from(1))?;
        assert!(bool::from(duplicate_one_by_two));
        assert!(bool::from(duplicate_two_by_one));

        let invalid = TestGraph::new(2, &[(0, 2)]);
        assert!(matches!(
            bisimulation_partition(&invalid),
            Err(GraphValidationError::EdgeOutOfBounds {
                source,
                target,
                node_count,
            }) if source == NodeId::from(0) && target == NodeId::from(2) && node_count == NodeCount::from(2)
        ));
        assert!(matches!(
            simulation_relation(&invalid),
            Err(GraphValidationError::EdgeOutOfBounds {
                source,
                target,
                node_count,
            }) if source == NodeId::from(0) && target == NodeId::from(2) && node_count == NodeCount::from(2)
        ));

        Ok(())
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn partition_entry_points_reject_layout_hostile_node_count()
    {
        struct LayoutHostileGraph;

        impl EdgeSource for LayoutHostileGraph
        {
            type Successors<'successors>
                = core::iter::Empty<NodeId>
            where
                Self: 'successors;

            fn node_count(&self) -> NodeCount
            {
                NodeCount::from(u32::MAX)
            }

            fn successors(
                &self,
                _node: NodeId,
            ) -> Self::Successors<'_>
            {
                core::iter::empty()
            }
        }

        let graph = LayoutHostileGraph;
        assert!(matches!(
            bisimulation_partition(&graph),
            Err(GraphValidationError::NodeCountTooLarge {
                node_count: u32::MAX
            })
        ));
        assert!(matches!(
            simulation_relation(&graph),
            Err(GraphValidationError::NodeCountTooLarge {
                node_count: u32::MAX
            })
        ));
    }

    #[test]
    fn compose_directed_cycle_witness_is_closed_valid_input_walk() -> Result<(), Box<dyn Error>>
    {
        let flow = TestGraph::new(8, &[
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 2),
            (2, 5),
            (5, 6),
            (6, 7),
        ]);
        let maybe_witness = cycle_witness(&flow)?;
        let witness = maybe_witness.expect("flow graph contains a directed cycle");
        assert!(
            witness.nodes.len() >= 2,
            "cycle witness must contain at least one edge and repeat the start"
        );
        assert_eq!(
            witness.nodes.first(),
            witness.nodes.last(),
            "cycle witness must be closed"
        );
        let adjacency = flow.canonical_adjacency();
        for edge in witness.nodes.windows(2) {
            let &[source, target] = edge
            else {
                panic!("cycle witness windows must yield source/target pairs");
            };
            assert!(
                bool::from(adjacency.contains_node(source)),
                "witness source must be in the input graph"
            );
            assert!(
                bool::from(adjacency.contains_edge(source, target)),
                "witness edge must exist in the input graph"
            );
        }
        assert!(
            witness.nodes.contains(&NodeId::from(2)),
            "the compose-shaped cycle should pass through the join state"
        );
        Ok(())
    }

    fn rows_as_pairs(simulation: &Simulation) -> SimulationPairs
    {
        SimulationPairs(
            simulation
                .rows()
                .iter()
                .map(|row| {
                    (
                        u32::from(row.subject),
                        row.candidates.iter().copied().map(u32::from).collect(),
                    )
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod laws
{
    use super::*;

    proptest! {
        #[test]
        fn partition_matches_independent_naive_oracle(node_count in 0_u32..7, bits in proptest::collection::vec(any::<bool>(), 0..49)) {
            let graph = graph_from_bits(node_count, &bits);
            let partition = match bisimulation_partition(&graph) {
                Ok(value) => value,
                Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("bisimulation_partition failed: {error}"))),
            };
            let oracle = naive_bisimulation(&graph.canonical_adjacency(), node_count);
            for left in 0..node_count {
                for right in 0..node_count {
                    let observed = match partition.equivalent(NodeId::from(left), NodeId::from(right)) {
                        Ok(value) => value,
                        Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("partition query failed: {error}"))),
                    };
                    let left_position = usize::try_from(left).expect("generated node fits usize");
                    let right_position = usize::try_from(right).expect("generated node fits usize");
                    prop_assert_eq!(bool::from(observed), oracle[left_position][right_position], "bisimulation pair must match oracle");
                }
            }
            assert_blocks_are_canonical(partition.blocks());
        }

        #[test]
        fn simulation_matches_independent_naive_oracle(node_count in 0_u32..7, bits in proptest::collection::vec(any::<bool>(), 0..49)) {
            let graph = graph_from_bits(node_count, &bits);
            let simulation = match simulation_relation(&graph) {
                Ok(value) => value,
                Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("simulation_relation failed: {error}"))),
            };
            let oracle = naive_simulation(&graph.canonical_adjacency(), node_count);
            for subject in 0..node_count {
                let candidates = match simulation.candidates_for(NodeId::from(subject)) {
                    Ok(value) => value,
                    Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("candidate query failed: {error}"))),
                };
                let mut expected_candidates = Vec::new();
                for candidate in 0..node_count {
                    let observed = match simulation.is_simulated_by(NodeId::from(subject), NodeId::from(candidate)) {
                        Ok(value) => value,
                        Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("simulation query failed: {error}"))),
                    };
                    let subject_position = usize::try_from(subject).expect("generated node fits usize");
                    let candidate_position = usize::try_from(candidate).expect("generated node fits usize");
                    let expected = oracle[subject_position][candidate_position];
                    prop_assert_eq!(bool::from(observed), expected, "simulation pair must match oracle");
                    if expected {
                        expected_candidates.push(candidate);
                    }
                }
                prop_assert_eq!(candidates.iter().copied().map(u32::from).collect::<Vec<_>>(), expected_candidates, "candidate rows must be canonical and oracle-complete");
            }
        }
    }

    fn graph_from_bits<'bits, N, B>(
        node_count: N,
        bits: B,
    ) -> TestGraph
    where
        N: Into<NodeCount>,
        B: Into<GraphBits<'bits>>,
    {
        let node_count = node_count.into();
        let mut edges = Vec::new();
        let bits = bits.into();
        let mut bit_iter = bits.0.iter().copied();
        for source in node_count.ids() {
            for target in node_count.ids() {
                if bit_iter.next().unwrap_or(false) {
                    edges.push((u32::from(source), u32::from(target)));
                    if bit_iter.next().unwrap_or(false) {
                        edges.push((u32::from(source), u32::from(target)));
                    }
                }
            }
        }
        TestGraph::new(node_count, &edges)
    }

    fn assert_blocks_are_canonical(blocks: &[Vec<NodeId>])
    {
        let mut previous_first = None;
        for block in blocks {
            assert!(!block.is_empty(), "canonical partition omits empty blocks");
            let mut sorted = block.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(&sorted, block, "block members must be ascending and unique");
            let first = block[0];
            if let Some(previous) = previous_first {
                assert!(previous < first, "blocks must be ordered by least member");
            }
            previous_first = Some(first);
        }
    }

    fn naive_bisimulation<N>(
        adjacency: &CanonicalAdjacency,
        node_count: N,
    ) -> OracleRelation
    where
        N: Into<NodeCount>,
    {
        let node_count = node_count.into();
        let node_len = usize::try_from(node_count).expect("generated node count fits usize");
        let mut relation = OracleRelation(vec![vec![true; node_len]; node_len]);
        let mut changed = true;
        while changed {
            changed = false;
            for left in node_count.ids() {
                for right in node_count.ids() {
                    let left_position =
                        usize::try_from(u32::from(left)).expect("generated node fits usize");
                    let right_position =
                        usize::try_from(u32::from(right)).expect("generated node fits usize");
                    if relation[left_position][right_position]
                        && (!bool::from(forth(adjacency, &relation, left, right))
                            || !bool::from(forth(adjacency, &relation, right, left)))
                    {
                        relation[left_position][right_position] = false;
                        changed = true;
                    }
                }
            }
        }
        relation
    }

    fn naive_simulation<N>(
        adjacency: &CanonicalAdjacency,
        node_count: N,
    ) -> OracleRelation
    where
        N: Into<NodeCount>,
    {
        let node_count = node_count.into();
        let node_len = usize::try_from(node_count).expect("generated node count fits usize");
        let mut relation = OracleRelation(vec![vec![true; node_len]; node_len]);
        let mut changed = true;
        while changed {
            changed = false;
            for subject in node_count.ids() {
                for candidate in node_count.ids() {
                    let subject_position =
                        usize::try_from(u32::from(subject)).expect("generated node fits usize");
                    let candidate_position =
                        usize::try_from(u32::from(candidate)).expect("generated node fits usize");
                    if relation[subject_position][candidate_position]
                        && !bool::from(forth(adjacency, &relation, subject, candidate))
                    {
                        relation[subject_position][candidate_position] = false;
                        changed = true;
                    }
                }
            }
        }
        relation
    }

    fn forth(
        adjacency: &CanonicalAdjacency,
        relation: &OracleRelation,
        subject: NodeId,
        candidate: NodeId,
    ) -> SimulationStepMatched
    {
        for &subject_target in adjacency.targets(subject) {
            let subject_target_position =
                usize::try_from(u32::from(subject_target)).expect("generated node fits usize");
            let mut matched = false;
            for &candidate_target in adjacency.targets(candidate) {
                let candidate_target_position = usize::try_from(u32::from(candidate_target))
                    .expect("generated node fits usize");
                if relation[subject_target_position][candidate_target_position] {
                    matched = true;
                }
            }
            if !matched {
                return SimulationStepMatched::from(false);
            }
        }
        SimulationStepMatched::from(true)
    }
}
