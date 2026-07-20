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

use gandr_theory_graphs::CycleError;
use gandr_theory_graphs::EdgeSource;
use gandr_theory_graphs::GraphValidationError;
use gandr_theory_graphs::NodeCount;
use gandr_theory_graphs::NodeId;
use gandr_theory_graphs::PathLength;
use gandr_theory_graphs::adjacency_fingerprint;
use gandr_theory_graphs::all_simple_paths;
use gandr_theory_graphs::condensation;
use gandr_theory_graphs::cycle_witness;
use gandr_theory_graphs::has_path;
use gandr_theory_graphs::immediate_dominators;
use gandr_theory_graphs::is_cyclic;
use gandr_theory_graphs::reachability;
use gandr_theory_graphs::shortest_path_lengths;
use gandr_theory_graphs::strongly_connected_components;
use gandr_theory_graphs::topological_sort;
use gandr_theory_graphs::transitive_reduction_closure;
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
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SuccessorCallCount(usize);

impl From<usize> for SuccessorCallCount
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl PartialEq<usize> for SuccessorCallCount
{
    #[inline]
    fn eq(
        &self,
        other: &usize,
    ) -> bool
    {
        self.0 == *other
    }
}

impl PartialEq<SuccessorCallCount> for usize
{
    #[inline]
    fn eq(
        &self,
        other: &SuccessorCallCount,
    ) -> bool
    {
        *self == other.0
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
}

#[derive(Debug)]
struct CountingGraph
{
    node_count: u32,
    rows: Vec<Vec<NodeId>>,
    successor_calls: core::cell::Cell<usize>,
}

impl CountingGraph
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
            successor_calls: core::cell::Cell::new(0),
        }
    }

    fn successor_calls(&self) -> SuccessorCallCount
    {
        SuccessorCallCount::from(self.successor_calls.get())
    }
}

impl EdgeSource for CountingGraph
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
        self.successor_calls
            .set(self.successor_calls.get().saturating_add(1));
        usize::try_from(u32::from(node))
            .ok()
            .and_then(|position| self.rows.get(position))
            .map_or_else(|| EMPTY.iter().copied(), |row| row.iter().copied())
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
    fn algorithm_menu_contract() -> Result<(), Box<dyn Error>>
    {
        let dag = TestGraph::new(6, &[(0, 2), (0, 1), (1, 3), (2, 3), (4, 5)]);
        let topological = topological_sort(&dag)?;
        assert_eq!(
            vec![4, 5, 0, 2, 1, 3],
            topological
                .iter()
                .copied()
                .map(u32::from)
                .collect::<Vec<_>>(),
            "topological sort must expose petgraph View traversal order"
        );

        let component_graph = TestGraph::new(6, &[(0, 1), (1, 0), (1, 2), (2, 3), (3, 2), (3, 4)]);
        let scc = strongly_connected_components(&component_graph)?;
        assert_eq!(
            scc.components
                .iter()
                .map(|component| component.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![vec![0, 1], vec![2, 3], vec![4], vec![5]],
            "SCC rows and members must be sorted canonically after petgraph Kosaraju"
        );

        let condensed = condensation(&component_graph)?;
        assert_eq!(
            condensed
                .components
                .iter()
                .map(|component| component.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![vec![0, 1], vec![2, 3], vec![4], vec![5]],
            "condensation must preserve canonical SCC rows"
        );
        assert_eq!(
            condensed
                .edges
                .iter()
                .map(|edge| (u32::from(edge.source), u32::from(edge.target)))
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 2)],
            "condensation edges must be deduplicated and sorted"
        );

        let paths_graph = TestGraph::new(6, &[(0, 2), (0, 1), (1, 3), (2, 3), (3, 4)]);
        let forward_path_exists = has_path(&paths_graph, NodeId::from(0), NodeId::from(4))?;
        let reverse_path_exists = has_path(&paths_graph, NodeId::from(4), NodeId::from(0))?;
        assert!(bool::from(forward_path_exists), "0 must reach 4");
        assert!(!bool::from(reverse_path_exists), "4 must not reach 0");

        let reachable = reachability(&paths_graph)?;
        assert_eq!(
            vec![
                (0, vec![1, 2, 3, 4]),
                (1, vec![3, 4]),
                (2, vec![3, 4]),
                (3, vec![4]),
                (4, vec![]),
                (5, vec![]),
            ],
            reachable
                .rows
                .iter()
                .map(|row| {
                    (
                        u32::from(row.source),
                        row.targets
                            .iter()
                            .copied()
                            .map(u32::from)
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
            "reachability rows must be sorted and omit disconnected nodes"
        );

        let distances = shortest_path_lengths(&paths_graph, NodeId::from(0))?;
        assert_eq!(
            vec![(0, 0), (1, 1), (2, 1), (3, 2), (4, 3)],
            distances
                .rows
                .iter()
                .map(|row| (u32::from(row.node), u32::from(row.distance)))
                .collect::<Vec<_>>(),
            "shortest path rows must choose minimum edge counts and sort by node"
        );

        let paths = all_simple_paths(
            &paths_graph,
            NodeId::from(0),
            NodeId::from(3),
            PathLength::from(2),
        )?;
        assert_eq!(
            paths
                .paths
                .iter()
                .map(|path| path.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![vec![0, 1, 3], vec![0, 2, 3]],
            "all simple paths must be lexicographic and obey the edge-depth bound"
        );
        let bounded = all_simple_paths(
            &paths_graph,
            NodeId::from(0),
            NodeId::from(3),
            PathLength::from(1),
        )?;
        assert!(
            bounded.paths.is_empty(),
            "depth bound must exclude longer paths"
        );

        let reduction_graph = TestGraph::new(4, &[(0, 1), (1, 2), (0, 2), (2, 3), (0, 3)]);
        let reduced = transitive_reduction_closure(&reduction_graph)?;
        assert_eq!(
            reduced
                .reduction_edges
                .iter()
                .map(|edge| (u32::from(edge.source), u32::from(edge.target)))
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 2), (2, 3)],
            "transitive reduction must remove edges implied by alternate paths"
        );
        assert_eq!(
            vec![
                (0, vec![1, 2, 3]),
                (1, vec![2, 3]),
                (2, vec![3]),
                (3, vec![])
            ],
            reduced
                .closure
                .rows
                .iter()
                .map(|row| {
                    (
                        u32::from(row.source),
                        row.targets
                            .iter()
                            .copied()
                            .map(u32::from)
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
            "closure must retain all original reachability"
        );

        let dominator_graph = TestGraph::new(6, &[(0, 1), (0, 2), (1, 3), (2, 3), (3, 4)]);
        let dominators = immediate_dominators(&dominator_graph, NodeId::from(0))?;
        assert_eq!(
            vec![
                (0, None, vec![0]),
                (1, Some(0), vec![0, 1]),
                (2, Some(0), vec![0, 2]),
                (3, Some(0), vec![0, 3]),
                (4, Some(3), vec![0, 3, 4]),
                (5, None, vec![]),
            ],
            dominators
                .rows
                .iter()
                .map(|row| {
                    (
                        u32::from(row.node),
                        row.immediate.map(u32::from),
                        row.dominators
                            .iter()
                            .copied()
                            .map(u32::from)
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
            "dominator rows must resolve ties and disconnected nodes deterministically"
        );

        let invalid_edge = TestGraph::new(2, &[(0, 2)]);
        assert!(
            matches!(
                reachability(&invalid_edge),
                Err(GraphValidationError::EdgeOutOfBounds {
                    source,
                    target,
                    node_count,
                }) if source == NodeId::from(0) && target == NodeId::from(2) && node_count == NodeCount::from(2)
            ),
            "invalid successor edge must be reported with source and target"
        );
        assert!(
            matches!(
                has_path(&TestGraph::new(1, &[]), NodeId::from(0), NodeId::from(1)),
                Err(GraphValidationError::NodeOutOfBounds {
                    node,
                    node_count,
                }) if node == NodeId::from(1) && node_count == NodeCount::from(1)
            ),
            "invalid caller node must be reported separately"
        );
        Ok(())
    }

    #[test]
    fn deep_acyclic_chain_uses_iterative_graph_algorithms() -> Result<(), Box<dyn Error>>
    {
        let node_count = 20_000_u32;
        let edges = (0 .. node_count.saturating_sub(1))
            .map(|source| (source, source.saturating_add(1)))
            .collect::<Vec<_>>();
        let graph = TestGraph::new(node_count, &edges);

        let scc = strongly_connected_components(&graph)?;
        let node_len = usize::try_from(node_count)?;
        assert_eq!(
            node_len,
            scc.components.len(),
            "acyclic chain must produce one SCC per node"
        );
        for node in 0 .. node_count {
            let node_position = usize::try_from(node)?;
            let component = scc
                .components
                .get(node_position)
                .expect("each chain node has a singleton component");
            assert_eq!(
                vec![node],
                component.iter().copied().map(u32::from).collect::<Vec<_>>(),
                "each chain node must be its own singleton SCC"
            );
        }
        let graph_is_cyclic = is_cyclic(&graph)?;
        assert!(
            !bool::from(graph_is_cyclic),
            "deep acyclic chain must not be classified as cyclic"
        );
        let witness = cycle_witness(&graph)?;
        assert_eq!(
            None, witness,
            "deep acyclic chain must not produce cycle evidence"
        );
        Ok(())
    }

    #[test]
    fn trivial_simple_path_validates_rows_without_petgraph_enumeration()
    -> Result<(), Box<dyn Error>>
    {
        let graph = CountingGraph::new(3, &[(0, 1), (1, 2)]);
        let paths = all_simple_paths(
            &graph,
            NodeId::from(1),
            NodeId::from(1),
            PathLength::from(0),
        )?;
        assert_eq!(
            paths
                .paths
                .iter()
                .map(|path| path.iter().copied().map(u32::from).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![vec![1]],
            "start==end preserves the canonical trivial path even at depth zero"
        );
        let expected_successor_calls = usize::try_from(graph.node_count())?;
        assert_eq!(
            expected_successor_calls,
            graph.successor_calls(),
            "trivial path must validate every adjacency row and skip later petgraph traversal"
        );

        let invalid_other_row = CountingGraph::new(2, &[(1, 2)]);
        assert!(
            matches!(
                all_simple_paths(&invalid_other_row, NodeId::from(0), NodeId::from(0), PathLength::from(0)),
                Err(GraphValidationError::EdgeOutOfBounds {
                    source,
                    target,
                    node_count,
                }) if source == NodeId::from(1) && target == NodeId::from(2) && node_count == NodeCount::from(2)
            ),
            "start==end must still validate adjacency rows not reachable from the endpoint"
        );
        let expected_invalid_successor_calls = usize::try_from(invalid_other_row.node_count())?;
        assert_eq!(
            expected_invalid_successor_calls,
            invalid_other_row.successor_calls(),
            "validation must inspect every row before returning the trivial path"
        );
        Ok(())
    }

    #[test]
    fn cycle_invalid_graph_source_contract()
    {
        let invalid_edge = TestGraph::new(2, &[(0, 2)]);
        let Err(error) = topological_sort(&invalid_edge)
        else {
            panic!("invalid graph unexpectedly sorted");
        };
        let CycleError::InvalidGraph(ref expected) = error
        else {
            panic!("invalid graph did not return CycleError::InvalidGraph: {error:?}");
        };
        let observed_source = error
            .source()
            .and_then(|source| source.downcast_ref::<GraphValidationError>());

        assert_eq!(
            Some(expected),
            observed_source,
            "CycleError::InvalidGraph must expose the exact GraphValidationError as source"
        );
        assert_eq!(
            &GraphValidationError::EdgeOutOfBounds {
                source: NodeId::from(0),
                target: NodeId::from(2),
                node_count: NodeCount::from(2),
            },
            expected,
            "source error must preserve the concrete invalid edge"
        );
    }

    #[test]
    fn cycle_witness_contract() -> Result<(), Box<dyn Error>>
    {
        let graph = TestGraph::new(4, &[(0, 1), (1, 2), (2, 1), (2, 3)]);
        let Some(witness) = cycle_witness(&graph)?
        else {
            return Err(std::io::Error::other("missing cycle witness").into());
        };
        assert_eq!(
            witness
                .nodes
                .iter()
                .copied()
                .map(u32::from)
                .collect::<Vec<_>>(),
            vec![1, 2, 1],
            "cycle node walk must be closed"
        );
        assert_eq!(
            witness
                .edges
                .iter()
                .map(|edge| (u32::from(edge.source), u32::from(edge.target)))
                .collect::<Vec<_>>(),
            vec![(1, 2), (2, 1)],
            "cycle edges must match the walk"
        );
        let graph_is_cyclic = is_cyclic(&graph)?;
        assert!(
            bool::from(graph_is_cyclic),
            "is_cyclic must agree with witness existence"
        );

        let cyclic = TestGraph::new(3, &[(0, 1), (1, 2), (2, 0)]);
        let error = topological_sort(&cyclic).err();
        assert!(
            matches!(error, Some(CycleError::Cycle { .. })),
            "cyclic topological sort must return typed cycle evidence"
        );
        assert!(
            matches!(
                transitive_reduction_closure(&cyclic),
                Err(CycleError::Cycle { .. })
            ),
            "transitive reduction is only defined for DAGs"
        );
        Ok(())
    }

    #[test]
    fn fingerprint_contract() -> Result<(), Box<dyn Error>>
    {
        let graph = TestGraph::new(3, &[(0, 2), (0, 1), (0, 1), (1, 2)]);
        let reordered = TestGraph::new(3, &[(1, 2), (0, 1), (0, 2)]);
        let graph_fingerprint = adjacency_fingerprint(&graph)?;
        let reordered_fingerprint = adjacency_fingerprint(&reordered)?;
        assert_eq!(
            graph_fingerprint, reordered_fingerprint,
            "fingerprint must ignore successor order and duplicate edges"
        );
        let changed_fingerprint = adjacency_fingerprint(&TestGraph::new(3, &[(0, 1), (2, 1)]))?;
        assert_ne!(
            graph_fingerprint, changed_fingerprint,
            "different canonical adjacency must hash differently"
        );
        assert!(
            matches!(
                adjacency_fingerprint(&TestGraph::new(1, &[(0, 1)])),
                Err(GraphValidationError::EdgeOutOfBounds {
                    source,
                    target,
                    node_count,
                }) if source == NodeId::from(0) && target == NodeId::from(1) && node_count == NodeCount::from(1)
            ),
            "fingerprint must validate target bounds"
        );
        Ok(())
    }
}

#[cfg(test)]
mod laws
{
    use super::*;

    proptest! {
        #[test]
        fn reachability_implies_has_path_law(edges in proptest::collection::vec((0_u32..6, 0_u32..6), 0..18)) {
            let graph = TestGraph::new(6, &edges);
            let reachable = match reachability(&graph) {
                Ok(value) => value,
                Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("reachability failed: {error}"))),
            };
            for row in &reachable.rows {
                for &target in &row.targets {
                    let has_path_result = match has_path(&graph, row.source, target) {
                        Ok(value) => value,
                        Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("has_path failed: {error}"))),
                    };
                    prop_assert!(bool::from(has_path_result), "reachability row target must satisfy has_path");
                }
            }
        }

        #[test]
        fn topological_order_respects_every_dag_edge(bits in proptest::collection::vec(any::<bool>(), 0..15)) {
            let mut edges = Vec::new();
            let mut bit_iter = bits.into_iter();
            for source in 0_u32..6 {
                for target in source.saturating_add(1)..6 {
                    if bit_iter.next().unwrap_or(false) {
                        edges.push((source, target));
                    }
                }
            }
            let graph = TestGraph::new(6, &edges);
            let order = match topological_sort(&graph) {
                Ok(value) => value,
                Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("topological_sort failed: {error}"))),
            };
            for &(source, target) in &edges {
                let source_position = order.iter().position(|&node| node == NodeId::from(source));
                let target_position = order.iter().position(|&node| node == NodeId::from(target));
                prop_assert!(source_position.is_some(), "source must appear in topological order");
                prop_assert!(target_position.is_some(), "target must appear in topological order");
                prop_assert!(source_position < target_position, "topological order must place sources before targets");
            }
        }

        #[test]
        fn fingerprint_ignores_edge_order_and_duplicates_law(edges in proptest::collection::vec((0_u32..6, 0_u32..6), 0..18)) {
            let mut duplicated = edges.clone();
            duplicated.extend(edges.iter().copied());
            duplicated.reverse();
            let graph = TestGraph::new(6, &edges);
            let reordered = TestGraph::new(6, &duplicated);
            let left = match adjacency_fingerprint(&graph) {
                Ok(value) => value,
                Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("fingerprint failed: {error}"))),
            };
            let right = match adjacency_fingerprint(&reordered) {
                Ok(value) => value,
                Err(error) => return Err(proptest::test_runner::TestCaseError::fail(format!("reordered fingerprint failed: {error}"))),
            };
            prop_assert_eq!(left, right, "fingerprint must be canonical over order and duplicates");
        }
    }
}
