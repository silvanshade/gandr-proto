//! Private petgraph adapter over the public dense-u32 graph boundary.

use alloc::vec::Vec;

use petgraph::Directed;
use petgraph::Direction;
use petgraph::visit::Data;
use petgraph::visit::EdgeRef;
use petgraph::visit::GraphBase;
use petgraph::visit::GraphProp;
use petgraph::visit::IntoEdgeReferences;
use petgraph::visit::IntoEdges;
use petgraph::visit::IntoEdgesDirected;
use petgraph::visit::IntoNeighbors;
use petgraph::visit::IntoNeighborsDirected;
use petgraph::visit::IntoNodeIdentifiers;
use petgraph::visit::IntoNodeReferences;
use petgraph::visit::NodeCompactIndexable;
use petgraph::visit::NodeCount as PetNodeCount;
use petgraph::visit::NodeIndexable;
use petgraph::visit::NodeRef;
use petgraph::visit::VisitMap;
use petgraph::visit::Visitable;

use crate::EdgeId;
use crate::EdgeSource;
use crate::NodeCapacity;
use crate::NodeCount;
use crate::NodeId;
use crate::NodeIdRange;
use crate::NodePosition;

/// Crate-private petgraph view over an [`EdgeSource`].
///
/// # Contract
/// - requires: `graph` satisfies [`EdgeSource`]'s dense node contract for the
///   lifetime of the view.
/// - ensures: petgraph node identifiers are the same dense `u32` identifiers
///   exposed by `graph`.
/// - provides: a borrowed petgraph visit façade over the public dense graph
///   boundary.
/// - panics: none.
/// - intension: traversal adapters borrow `graph`; outgoing edge observations
///   preserve successor order, while incoming and all-edge observations scan
///   source nodes in ascending dense order.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the five-node DAG witness separates source
///   order, successor order, merge, and sink cases by observing exact
///   topological order and SCC singleton rows through petgraph traversal; the
///   adapter-unit witnesses additionally pin the public petgraph visit-trait
///   observations for dense node ids, edge sources/ids, directed edge rows, and
///   all-edge iterator exhaustion.
/// - witness: `gandr_theory_graphs::contracts::view_adapter_contract`
/// - witness: `view::tests::node_references_preserve_dense_ids`
/// - witness: `view::tests::edge_references_preserve_endpoints_and_ids`
/// - witness: `view::tests::directed_edge_iterators_preserve_requested_rows`
/// - witness: `view::tests::all_edges_preserve_canonical_rows_and_exhaust`
#[repr(transparent)]
pub struct View<'graph, G: EdgeSource + ?Sized>
{
    /// Borrowed public graph boundary.
    graph: &'graph G,
}

impl<'graph, G: EdgeSource + ?Sized> View<'graph, G>
{
    /// Wraps a dense edge source for internal petgraph traversal.
    ///
    /// # Contract
    /// - requires: `graph` satisfies [`EdgeSource`]'s dense node contract for
    ///   `'graph`.
    /// - ensures: returns a [`View`] tied to `graph`'s borrow and exposing no
    ///   mutable graph access.
    /// - provides: a petgraph traversal adapter for the borrowed dense graph.
    /// - panics: none.
    /// - intension: construction stores only the graph reference and performs
    ///   no traversal, normalization, or successor reordering.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the five-node DAG witness separates adapter
    ///   construction from node and edge traversal mutants by observing exact
    ///   topological order and SCC singleton rows; adapter-unit witnesses pin
    ///   the dense node and edge rows exposed through petgraph visit traits.
    /// - witness: `gandr_theory_graphs::contracts::view_adapter_contract`
    /// - witness: `view::tests::node_references_preserve_dense_ids`
    /// - witness: `view::tests::edge_references_preserve_endpoints_and_ids`
    /// - witness: `view::tests::directed_edge_iterators_preserve_requested_rows`
    /// - witness: `view::tests::all_edges_preserve_canonical_rows_and_exhaust`
    #[inline]
    #[must_use]
    pub const fn new(graph: &'graph G) -> Self
    {
        Self { graph }
    }
}

impl<G: EdgeSource + ?Sized> GraphBase for View<'_, G>
{
    type EdgeId = EdgeId;
    type NodeId = NodeId;
}

impl<G: EdgeSource + ?Sized> Data for View<'_, G>
{
    type EdgeWeight = ();
    type NodeWeight = ();
}

impl<G: EdgeSource + ?Sized> GraphProp for View<'_, G>
{
    type EdgeType = Directed;
}

impl<G: EdgeSource + ?Sized> PetNodeCount for View<'_, G>
{
    #[inline]
    fn node_count(&self) -> usize
    {
        usize::from(node_count_to_bound(self.graph.node_count()))
    }
}

impl<G: EdgeSource + ?Sized> NodeIndexable for View<'_, G>
{
    #[inline]
    fn node_bound(&self) -> usize
    {
        self.node_count()
    }

    #[inline]
    fn to_index(
        &self,
        a: Self::NodeId,
    ) -> usize
    {
        usize::from(node_id_to_index(a))
    }

    #[inline]
    fn from_index(
        &self,
        i: usize,
    ) -> Self::NodeId
    {
        index_to_node_id(NodePosition::from(i))
    }
}

impl<G: EdgeSource + ?Sized> NodeCompactIndexable for View<'_, G>
{
}

/// Converts a dense `u32` node count into petgraph's index bound.
#[inline]
fn node_count_to_bound(node_count: NodeCount) -> NodeCapacity
{
    match NodeCapacity::try_from(node_count) {
        | Ok(node_bound) => node_bound,
        | Err(conversion_error) => {
            debug_assert!(
                NodeCapacity::try_from(node_count).is_ok(),
                "dense u32 node count must fit in petgraph usize index space: {conversion_error}"
            );
            NodeCapacity::default()
        },
    }
}

/// Converts a dense node identity into petgraph's compact index space.
#[inline]
fn node_id_to_index(node: NodeId) -> NodePosition
{
    match NodePosition::try_from(node) {
        | Ok(index) => index,
        | Err(conversion_error) => {
            debug_assert!(
                NodePosition::try_from(node).is_ok(),
                "dense u32 node id must fit in petgraph usize index space: {conversion_error}"
            );
            NodePosition::from(usize::MAX)
        },
    }
}

/// Converts a petgraph compact index back into a dense node identity.
#[inline]
fn index_to_node_id(index: NodePosition) -> NodeId
{
    match NodeId::try_from(index) {
        | Ok(node) => node,
        | Err(conversion_error) => {
            debug_assert!(
                NodeId::try_from(index).is_ok(),
                "petgraph node index must fit in dense u32 node id space: {conversion_error}"
            );
            NodeId::from(u32::MAX)
        },
    }
}

impl<'graph, G> IntoNodeIdentifiers for &View<'graph, G>
where
    G: EdgeSource + ?Sized + 'graph,
{
    type NodeIdentifiers = NodeIdRange;

    #[inline]
    fn node_identifiers(self) -> Self::NodeIdentifiers
    {
        self.graph.node_count().ids()
    }
}

impl<'graph, G> IntoNodeReferences for &View<'graph, G>
where
    G: EdgeSource + ?Sized + 'graph,
{
    type NodeRef = NodeReference;
    type NodeReferences = NodeReferences;

    #[inline]
    fn node_references(self) -> Self::NodeReferences
    {
        NodeReferences {
            nodes: self.node_identifiers(),
        }
    }
}

impl<'graph, G> IntoNeighbors for &View<'graph, G>
where
    G: EdgeSource + ?Sized + 'graph,
{
    type Neighbors = G::Successors<'graph>;

    #[inline]
    fn neighbors(
        self,
        a: Self::NodeId,
    ) -> Self::Neighbors
    {
        self.graph.successors(a)
    }
}

impl<'graph, G> IntoNeighborsDirected for &View<'graph, G>
where
    G: EdgeSource + ?Sized + 'graph,
{
    type NeighborsDirected = DirectedNeighbors<'graph, G>;

    #[inline]
    fn neighbors_directed(
        self,
        n: Self::NodeId,
        d: Direction,
    ) -> Self::NeighborsDirected
    {
        match d {
            | Direction::Outgoing => DirectedNeighbors::Outgoing(self.graph.successors(n)),
            | Direction::Incoming => {
                DirectedNeighbors::Incoming(IncomingNeighbors::new(self.graph, n))
            },
        }
    }
}

impl<'graph, G> IntoEdgeReferences for &View<'graph, G>
where
    G: EdgeSource + ?Sized + 'graph,
{
    type EdgeRef = EdgeReference;
    type EdgeReferences = AllEdges<'graph, G>;

    #[inline]
    fn edge_references(self) -> Self::EdgeReferences
    {
        AllEdges::new(self.graph)
    }
}

impl<'graph, G> IntoEdges for &View<'graph, G>
where
    G: EdgeSource + ?Sized + 'graph,
{
    type Edges = OutgoingEdges<'graph, G>;

    #[inline]
    fn edges(
        self,
        a: Self::NodeId,
    ) -> Self::Edges
    {
        OutgoingEdges::new(a, self.graph.successors(a))
    }
}

impl<'graph, G> IntoEdgesDirected for &View<'graph, G>
where
    G: EdgeSource + ?Sized + 'graph,
{
    type EdgesDirected = DirectedEdges<'graph, G>;

    #[inline]
    fn edges_directed(
        self,
        a: Self::NodeId,
        dir: Direction,
    ) -> Self::EdgesDirected
    {
        match dir {
            | Direction::Outgoing => {
                DirectedEdges::Outgoing(OutgoingEdges::new(a, self.graph.successors(a)))
            },
            | Direction::Incoming => DirectedEdges::Incoming(IncomingEdges::new(self.graph, a)),
        }
    }
}

impl<G: EdgeSource + ?Sized> Visitable for View<'_, G>
{
    type Map = DenseVisitMap;

    #[inline]
    fn visit_map(&self) -> Self::Map
    {
        DenseVisitMap::new(NodeCapacity::from(self.node_count()))
    }

    #[inline]
    fn reset_map(
        &self,
        map: &mut Self::Map,
    )
    {
        map.reset(NodeCapacity::from(self.node_count()));
    }
}

/// Node reference with unit weight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NodeReference
{
    /// Dense node identity.
    node: NodeId,
}

impl NodeRef for NodeReference
{
    type NodeId = NodeId;
    type Weight = ();

    #[inline]
    fn id(&self) -> Self::NodeId
    {
        self.node
    }

    #[inline]
    fn weight(&self) -> &Self::Weight
    {
        &UNIT_WEIGHT
    }
}

/// Edge reference with unit weight.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EdgeReference
{
    /// Dense source node identity.
    source: NodeId,
    /// Dense target node identity.
    target: NodeId,
}

impl EdgeReference
{
    /// Creates a borrowed edge reference value.
    #[inline]
    #[must_use]
    const fn new(
        source: NodeId,
        target: NodeId,
    ) -> Self
    {
        Self { source, target }
    }
}

impl EdgeRef for EdgeReference
{
    type EdgeId = EdgeId;
    type NodeId = NodeId;
    type Weight = ();

    #[inline]
    fn source(&self) -> Self::NodeId
    {
        self.source
    }

    #[inline]
    fn target(&self) -> Self::NodeId
    {
        self.target
    }

    #[inline]
    fn weight(&self) -> &Self::Weight
    {
        &UNIT_WEIGHT
    }

    #[inline]
    fn id(&self) -> Self::EdgeId
    {
        EdgeId::new(self.source, self.target)
    }
}

/// Shared unit weight returned by node and edge references.
static UNIT_WEIGHT: () = ();

/// Iterator over dense node references.
#[repr(transparent)]
pub struct NodeReferences
{
    /// Remaining dense node identities.
    nodes: NodeIdRange,
}

impl Iterator for NodeReferences
{
    type Item = NodeReference;

    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        self.nodes.next().map(|node| NodeReference { node })
    }
}

/// Iterator over outgoing edge references for one source.
pub struct OutgoingEdges<'successors, G: EdgeSource + ?Sized + 'successors>
{
    /// Source node for every yielded edge.
    source: NodeId,
    /// Lending successor iterator from the source graph.
    successors: G::Successors<'successors>,
}

impl<'successors, G: EdgeSource + ?Sized + 'successors> OutgoingEdges<'successors, G>
{
    /// Creates an outgoing edge iterator.
    #[inline]
    #[must_use]
    fn new(
        source: NodeId,
        successors: G::Successors<'successors>,
    ) -> Self
    {
        Self { source, successors }
    }
}

impl<'successors, G: EdgeSource + ?Sized + 'successors> Iterator for OutgoingEdges<'successors, G>
{
    type Item = EdgeReference;

    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        self.successors
            .next()
            .map(|target| EdgeReference::new(self.source, target))
    }
}

/// Iterator over incoming neighbor node identities for one target.
pub struct IncomingNeighbors<'graph, G: EdgeSource + ?Sized>
{
    /// Graph scanned for incoming edges.
    graph: &'graph G,
    /// Target node matched by scanned outgoing edges.
    target: NodeId,
    /// Next source node to open when the active iterator is exhausted.
    next_source: NodeId,
    /// Source node that owns the active successor iterator.
    active_source: NodeId,
    /// Active successor iterator for `active_source`.
    active_successors: Option<G::Successors<'graph>>,
}

impl<'graph, G: EdgeSource + ?Sized + 'graph> IncomingNeighbors<'graph, G>
{
    /// Creates an incoming neighbor iterator.
    #[inline]
    #[must_use]
    fn new(
        graph: &'graph G,
        target: NodeId,
    ) -> Self
    {
        Self {
            graph,
            target,
            next_source: NodeId::from(0_u32),
            active_source: NodeId::from(0_u32),
            active_successors: None,
        }
    }
}

impl<'graph, G: EdgeSource + ?Sized + 'graph> Iterator for IncomingNeighbors<'graph, G>
{
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item>
    {
        loop {
            if let Some(successors) = self.active_successors.as_mut() {
                for target in successors.by_ref() {
                    if target == self.target {
                        return Some(self.active_source);
                    }
                }
                self.active_successors = None;
            }

            if u32::from(self.next_source) >= u32::from(self.graph.node_count()) {
                return None;
            }

            self.active_source = self.next_source;
            self.next_source = match u32::from(self.next_source).checked_add(1_u32) {
                | Some(next) => NodeId::from(next),
                | None => NodeId::from(u32::from(self.graph.node_count())),
            };
            self.active_successors = Some(self.graph.successors(self.active_source));
        }
    }
}

/// Iterator over incoming edge references for one target.
pub struct IncomingEdges<'graph, G: EdgeSource + ?Sized>
{
    /// Target node for every yielded edge.
    target: NodeId,
    /// Incoming source iterator.
    sources: IncomingNeighbors<'graph, G>,
}

impl<'graph, G: EdgeSource + ?Sized + 'graph> IncomingEdges<'graph, G>
{
    /// Creates an incoming edge iterator.
    #[inline]
    #[must_use]
    fn new(
        graph: &'graph G,
        target: NodeId,
    ) -> Self
    {
        Self {
            target,
            sources: IncomingNeighbors::new(graph, target),
        }
    }
}

impl<'graph, G: EdgeSource + ?Sized + 'graph> Iterator for IncomingEdges<'graph, G>
{
    type Item = EdgeReference;

    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        self.sources
            .next()
            .map(|source| EdgeReference::new(source, self.target))
    }
}

/// Direction-polymorphic neighbor iterator.
pub enum DirectedNeighbors<'neighbors, G: EdgeSource + ?Sized>
{
    /// Outgoing successor traversal.
    Outgoing(G::Successors<'neighbors>),
    /// Incoming source traversal.
    Incoming(IncomingNeighbors<'neighbors, G>),
}

impl<'neighbors, G: EdgeSource + ?Sized + 'neighbors> Iterator for DirectedNeighbors<'neighbors, G>
{
    type Item = NodeId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            | Self::Outgoing(ref mut successors) => successors.next(),
            | Self::Incoming(ref mut sources) => sources.next(),
        }
    }
}

/// Direction-polymorphic edge iterator.
pub enum DirectedEdges<'edges, G: EdgeSource + ?Sized>
{
    /// Outgoing edge traversal.
    Outgoing(OutgoingEdges<'edges, G>),
    /// Incoming edge traversal.
    Incoming(IncomingEdges<'edges, G>),
}

impl<'edges, G: EdgeSource + ?Sized + 'edges> Iterator for DirectedEdges<'edges, G>
{
    type Item = EdgeReference;

    #[inline]
    fn next(&mut self) -> Option<Self::Item>
    {
        match *self {
            | Self::Outgoing(ref mut edges) => edges.next(),
            | Self::Incoming(ref mut edges) => edges.next(),
        }
    }
}

/// Iterator over every outgoing edge in dense source order.
pub struct AllEdges<'graph, G: EdgeSource + ?Sized>
{
    /// Graph scanned for all edges.
    graph: &'graph G,
    /// Next source node to open when the active iterator is exhausted.
    next_source: NodeId,
    /// Source node that owns the active successor iterator.
    active_source: NodeId,
    /// Active successor iterator for `active_source`.
    active_successors: Option<G::Successors<'graph>>,
}

impl<'graph, G: EdgeSource + ?Sized + 'graph> AllEdges<'graph, G>
{
    /// Creates an all-edge iterator.
    #[inline]
    #[must_use]
    fn new(graph: &'graph G) -> Self
    {
        Self {
            graph,
            next_source: NodeId::from(0_u32),
            active_source: NodeId::from(0_u32),
            active_successors: None,
        }
    }
}

impl<'graph, G: EdgeSource + ?Sized + 'graph> Iterator for AllEdges<'graph, G>
{
    type Item = EdgeReference;

    fn next(&mut self) -> Option<Self::Item>
    {
        loop {
            if let Some(successors) = self.active_successors.as_mut() {
                let Some(target) = successors.next()
                else {
                    self.active_successors = None;
                    continue;
                };
                return Some(EdgeReference::new(self.active_source, target));
            }

            if u32::from(self.next_source) >= u32::from(self.graph.node_count()) {
                return None;
            }

            self.active_source = self.next_source;
            self.next_source = match u32::from(self.next_source).checked_add(1_u32) {
                | Some(next) => NodeId::from(next),
                | None => NodeId::from(u32::from(self.graph.node_count())),
            };
            self.active_successors = Some(self.graph.successors(self.active_source));
        }
    }
}

/// Dense bitset-backed visit map for petgraph traversals.
#[repr(transparent)]
pub struct DenseVisitMap
{
    /// Per-node visited bits keyed by dense identity.
    visited: Vec<bool>,
}

impl DenseVisitMap
{
    /// Creates an empty visit map sized to the current graph bound.
    #[inline]
    #[must_use]
    fn new(node_bound: NodeCapacity) -> Self
    {
        let mut visited = Vec::new();
        visited.resize(usize::from(node_bound), false);
        Self { visited }
    }

    /// Clears and resizes the map for a new traversal.
    #[inline]
    fn reset(
        &mut self,
        node_bound: NodeCapacity,
    )
    {
        self.visited.clear();
        self.visited.resize(usize::from(node_bound), false);
    }
}

impl VisitMap<NodeId> for DenseVisitMap
{
    #[inline]
    fn visit(
        &mut self,
        a: NodeId,
    ) -> bool
    {
        let Ok(index) = NodePosition::try_from(a)
        else {
            return false;
        };

        match self.visited.get_mut(usize::from(index)) {
            | Some(visited) => {
                let first_visit = !*visited;
                *visited = true;
                first_visit
            },
            | None => false,
        }
    }

    #[inline]
    fn is_visited(
        &self,
        a: &NodeId,
    ) -> bool
    {
        let Ok(index) = NodePosition::try_from(*a)
        else {
            return false;
        };

        self.visited
            .get(usize::from(index))
            .copied()
            .unwrap_or(false)
    }

    #[inline]
    fn unvisit(
        &mut self,
        a: NodeId,
    ) -> bool
    {
        let Ok(index) = NodePosition::try_from(a)
        else {
            return false;
        };

        match self.visited.get_mut(usize::from(index)) {
            | Some(visited) => {
                let was_visited = *visited;
                *visited = false;
                was_visited
            },
            | None => false,
        }
    }
}

#[cfg(test)]
mod tests
{
    use alloc::vec;
    use alloc::vec::Vec;

    use petgraph::Direction;
    use petgraph::visit::EdgeRef as _;
    use petgraph::visit::IntoEdgeReferences as _;
    use petgraph::visit::IntoEdges as _;
    use petgraph::visit::IntoEdgesDirected as _;
    use petgraph::visit::IntoNodeReferences as _;
    use petgraph::visit::NodeRef as _;

    use super::View;
    use crate::EdgeId;
    use crate::EdgeSource;
    use crate::NodeCount;
    use crate::NodeId;
    use crate::NodePosition;

    macro_rules! n {
        ($raw:literal) => {
            NodeId::from($raw)
        };
    }

    macro_rules! edge {
        ($source:literal, $target:literal) => {
            EdgeId::new(n!($source), n!($target))
        };
    }

    static EMPTY: [NodeId; 0] = [];

    #[repr(transparent)]
    struct TestGraph
    {
        rows: Vec<Vec<NodeId>>,
    }

    impl TestGraph
    {
        fn new(rows: Vec<Vec<NodeId>>) -> Self
        {
            Self { rows }
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
            NodeCount::from(u32::try_from(self.rows.len()).unwrap_or(u32::MAX))
        }

        fn successors(
            &self,
            node: NodeId,
        ) -> Self::Successors<'_>
        {
            NodePosition::try_from(node)
                .ok()
                .and_then(|index| self.rows.get(usize::from(index)))
                .map_or_else(|| EMPTY.iter().copied(), |row| row.iter().copied())
        }
    }

    fn edge_tuple(edge: super::EdgeReference) -> (NodeId, NodeId, EdgeId)
    {
        (edge.source(), edge.target(), edge.id())
    }

    #[test]
    fn node_references_preserve_dense_ids()
    {
        let graph = TestGraph::new(vec![vec![n!(2)], vec![], vec![n!(1)], vec![]]);
        let view = View::new(&graph);

        let rows = (&view)
            .node_references()
            .map(|node| (node.id(), *node.weight()))
            .collect::<Vec<_>>();

        assert_eq!(
            rows,
            vec![(n!(0), ()), (n!(1), ()), (n!(2), ()), (n!(3), ())],
            "node references must preserve every dense node id in order"
        );
    }

    #[test]
    fn edge_references_preserve_endpoints_and_ids()
    {
        let graph = TestGraph::new(vec![vec![n!(2), n!(1)], vec![n!(3)], vec![], vec![n!(0)]]);
        let view = View::new(&graph);

        let rows = (&view).edges(n!(0)).map(edge_tuple).collect::<Vec<_>>();

        assert_eq!(
            rows,
            vec![(n!(0), n!(2), edge!(0, 2)), (n!(0), n!(1), edge!(0, 1))],
            "outgoing edge references must preserve source, target, and id"
        );
    }

    #[test]
    fn directed_edge_iterators_preserve_requested_rows()
    {
        let graph = TestGraph::new(vec![vec![n!(2), n!(1)], vec![n!(2)], vec![n!(0)], vec![
            n!(2),
        ]]);
        let view = View::new(&graph);

        let outgoing = (&view)
            .edges_directed(n!(0), Direction::Outgoing)
            .map(edge_tuple)
            .collect::<Vec<_>>();
        let incoming = (&view)
            .edges_directed(n!(2), Direction::Incoming)
            .map(edge_tuple)
            .collect::<Vec<_>>();

        assert_eq!(
            outgoing,
            vec![(n!(0), n!(2), edge!(0, 2)), (n!(0), n!(1), edge!(0, 1))],
            "directed outgoing edges must be exactly the requested successor row"
        );
        assert_eq!(
            incoming,
            vec![
                (n!(0), n!(2), edge!(0, 2)),
                (n!(1), n!(2), edge!(1, 2)),
                (n!(3), n!(2), edge!(3, 2))
            ],
            "directed incoming edges must scan source rows and keep the requested target"
        );
    }

    #[test]
    fn all_edges_preserve_canonical_rows_and_exhaust()
    {
        let graph = TestGraph::new(vec![vec![n!(2), n!(1)], vec![], vec![n!(3)], vec![n!(0)]]);
        let view = View::new(&graph);
        let mut edges = (&view).edge_references();

        assert_eq!(
            Some((n!(0), n!(2), edge!(0, 2))),
            edges.next().map(edge_tuple)
        );
        assert_eq!(
            Some((n!(0), n!(1), edge!(0, 1))),
            edges.next().map(edge_tuple)
        );
        assert_eq!(
            Some((n!(2), n!(3), edge!(2, 3))),
            edges.next().map(edge_tuple)
        );
        assert_eq!(
            Some((n!(3), n!(0), edge!(3, 0))),
            edges.next().map(edge_tuple)
        );
        assert_eq!(
            None,
            edges.next().map(edge_tuple),
            "all-edge iterator must terminate after every canonical edge"
        );
        assert_eq!(
            None,
            edges.next().map(edge_tuple),
            "all-edge iterator must stay exhausted"
        );
    }
}
