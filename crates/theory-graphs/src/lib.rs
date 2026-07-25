//! Dense-u32 graph algorithms for gandr.
//!
//! The crate exposes a tiny [`EdgeSource`] boundary and keeps the petgraph view
//! adapter private. Public algorithms consume dense node identifiers only; no
//! petgraph type appears in the public API.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        reason = "the standard test-allow set keeps graph contract tests readable (docs/workflow/rust.md)"
    )
)]
extern crate alloc;

mod algorithms;
mod fingerprint;
mod partition_refine;
mod prec;
mod types;
pub(crate) mod view;
mod walk;

pub use algorithms::AllSimplePaths;
pub use algorithms::Condensation;
pub use algorithms::CycleDetected;
pub use algorithms::CycleError;
pub use algorithms::CycleWitness;
pub use algorithms::DominatorRow;
pub use algorithms::GraphValidationError;
pub use algorithms::ImmediateDominators;
pub use algorithms::PathExists;
pub use algorithms::Reachability;
pub use algorithms::ReachabilityRow;
pub use algorithms::ShortestPathLengths;
pub use algorithms::ShortestPathRow;
pub use algorithms::StronglyConnectedComponents;
pub use algorithms::TransitiveReductionClosure;
pub use algorithms::all_simple_paths;
pub use algorithms::condensation;
pub use algorithms::cycle_witness;
pub use algorithms::has_path;
pub use algorithms::immediate_dominators;
pub use algorithms::is_cyclic;
pub use algorithms::reachability;
pub use algorithms::shortest_path_lengths;
pub use algorithms::strongly_connected_components;
pub use algorithms::topological_sort;
pub use algorithms::transitive_reduction_closure;
pub use fingerprint::adjacency_fingerprint;
pub use partition_refine::*;
pub use prec::*;
pub use types::ComponentEdge;
pub use types::ComponentIndex;
pub use types::EdgeId;
pub use types::Fingerprint;
pub(crate) use types::FingerprintByte;
pub(crate) use types::FingerprintBytes;
pub(crate) use types::FingerprintWord16;
pub(crate) use types::FingerprintWord32;
pub(crate) use types::FingerprintWord64;
pub(crate) use types::NodeCapacity;
pub use types::NodeCount;
pub use types::NodeId;
pub use types::NodeIdRange;
pub(crate) use types::NodePosition;
pub use types::PathLength;
pub use types::SwingHeight;
pub use types::WalkChainLength;
pub use types::WalkHeight;
pub use walk::Dir;
pub use walk::End;
pub use walk::SeenKeyVerdict;
pub use walk::StanceTileSorted;
pub use walk::Swing;
pub use walk::SwingAdvance;
pub use walk::SwingArc;
pub use walk::Walk;
pub use walk::WalkBuildError;
pub use walk::WalkEquality;
pub use walk::WalkIndex;
pub use walk::WalkInequality;
pub use walk::WalkSpec;
pub use walk::WalkStep;
pub use walk::WalkSym;
pub use walk::WalkSymbolKey;

/// Dense directed edge source for deterministic graph algorithms.
///
/// # Contract
/// - requires: implementations expose valid nodes as exactly `0..node_count()`;
///   callers pass [`successors`](Self::successors) only a node in that range.
/// - ensures: [`node_count`](Self::node_count) returns the dense node bound and
///   [`successors`](Self::successors) yields outgoing target identities for the
///   supplied source.
/// - provides: a petgraph-free dense directed edge boundary for graph
///   algorithms and fingerprints.
/// - panics: none.
/// - intension: repeated traversal of the same graph state yields the same
///   successor order when observed through graph algorithm results or
///   [`adjacency_fingerprint`].
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the five-node DAG witness separates
///   dense-range, fork, merge, and sink inputs by observing exact topological
///   order and SCC singleton rows through the view adapter.
/// - witness: `gandr_theory_graphs::contracts::view_adapter_contract`
pub trait EdgeSource
{
    type Successors<'successors>: Iterator<Item = NodeId> + 'successors
    where
        Self: 'successors;

    /// Returns the number of dense nodes in the graph.
    #[must_use]
    fn node_count(&self) -> NodeCount;

    /// Returns the outgoing successors for `node`.
    fn successors(
        &self,
        node: NodeId,
    ) -> Self::Successors<'_>;
}

#[cfg(test)]
mod contracts
{
    use petgraph::algo::tarjan_scc;
    use petgraph::algo::toposort;

    use super::EdgeSource;
    use super::NodeCount;
    use super::NodeId;
    use super::view::View;

    #[repr(transparent)]
    struct DenseSource<'source>
    {
        rows: &'source [&'source [NodeId]],
    }

    impl EdgeSource for DenseSource<'_>
    {
        type Successors<'successors>
            = core::iter::Copied<core::slice::Iter<'successors, NodeId>>
        where
            Self: 'successors;

        #[inline]
        fn node_count(&self) -> NodeCount
        {
            NodeCount::from(
                u32::try_from(self.rows.len()).expect("test graph node count fits in u32"),
            )
        }

        #[inline]
        fn successors(
            &self,
            node: NodeId,
        ) -> Self::Successors<'_>
        {
            let index = usize::try_from(u32::from(node)).expect("test node id fits in usize");
            self.rows[index].iter().copied()
        }
    }

    #[test]
    fn view_adapter_contract()
    {
        let row_zero = [NodeId::from(1), NodeId::from(2), NodeId::from(3)];
        let row_one = [NodeId::from(3), NodeId::from(4)];
        let row_two = [NodeId::from(3), NodeId::from(4)];
        let row_three = [NodeId::from(4)];
        let row_four = [];
        let rows: [&[NodeId]; 5] = [&row_zero, &row_one, &row_two, &row_three, &row_four];
        let source = DenseSource { rows: &rows };
        let view = View::new(&source);

        let order = toposort(&view, None).expect("dense test graph is acyclic");
        assert_eq!(order, vec![
            NodeId::from(0),
            NodeId::from(1),
            NodeId::from(2),
            NodeId::from(3),
            NodeId::from(4)
        ]);

        let mut components = tarjan_scc(&view);
        components.sort_unstable();
        assert_eq!(components, vec![
            vec![NodeId::from(0)],
            vec![NodeId::from(1)],
            vec![NodeId::from(2)],
            vec![NodeId::from(3)],
            vec![NodeId::from(4)]
        ]);
    }
}
