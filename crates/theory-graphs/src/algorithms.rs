//! Deterministic graph algorithms over the public dense [`EdgeSource`]
//! boundary.
//!
//! # Contract
//! - requires: callers provide graphs through the crate's dense `u32` graph
//!   façade.
//! - ensures: successful public algorithms return owned graph observations.
//! - provides: the graph menu exercised by the integration witnesses without
//!   exposing petgraph or private view types in signatures.
//! - fails: invalid dense boundaries surface as [`GraphValidationError`] or
//!   [`CycleError::InvalidGraph`]; cyclic DAG-only inputs surface as
//!   [`CycleError::Cycle`].
//! - panics: none.
//! - intension: computes unit-cost distances with petgraph
//!   [`dijkstra`](fn@dijkstra) and canonicalize semantically free owned results
//!   by sorted dense node or edge order.
//!
//! # Adequacy
//! - hypothesis: L3 pointwise — the exact graph menu cases are the named DAG,
//!   component, path, trivial-path, reduction, dominator, invalid-boundary, and
//!   deep acyclic chain shapes with exact returned vectors and error variants;
//!   the invalid topological-sort witness additionally proves
//!   `CycleError::InvalidGraph` exposes the exact `GraphValidationError`
//!   through its `Error::source` chain. L1 evidence is reserved for concrete
//!   cycle witnesses validated against input edges.
//! - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
//! - witness: `gandr_theory_graphs::algorithms::contracts::cycle_invalid_graph_source_contract`
//! - witness: `gandr_theory_graphs::algorithms::contracts::cycle_witness_contract`

use alloc::vec;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::convert::TryFrom as _;
use core::error::Error;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;
use core::hash::BuildHasher;
use core::hash::Hasher;

use petgraph::algo::all_simple_paths as petgraph_all_simple_paths;
use petgraph::algo::condensation as petgraph_condensation;
use petgraph::algo::dijkstra;
use petgraph::algo::dominators;
use petgraph::algo::has_path_connecting;
use petgraph::algo::kosaraju_scc;
use petgraph::algo::toposort;
use petgraph::graph::DefaultIx;
use petgraph::graph::DiGraph;
use petgraph::graph::IndexType;
use petgraph::graph::NodeIndex as PetNodeIndex;
use petgraph::visit::EdgeRef as _;

use crate::ComponentEdge;
use crate::ComponentIndex;
use crate::EdgeId;
use crate::EdgeSource;
use crate::NodeCapacity;
use crate::NodeCount;
use crate::NodeId;
use crate::NodePosition;
use crate::PathLength;
use crate::view::View;

/// Internal unit edge payload used when owning a petgraph projection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PetgraphEdge;

/// A typed boundary failure for dense graph algorithms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphValidationError
{
    /// `node_count` could not fit in the host address space.
    NodeCountTooLarge
    {
        node_count: NodeCount
    },
    /// A caller-supplied node is not in `0..node_count`.
    NodeOutOfBounds
    {
        node: NodeId, node_count: NodeCount
    },
    /// A successor edge targets a node outside `0..node_count`.
    EdgeOutOfBounds
    {
        source: NodeId,
        target: NodeId,
        node_count: NodeCount,
    },
    /// An internal checked arithmetic operation overflowed.
    ArithmeticOverflow,
}

impl Display for GraphValidationError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        match *self {
            | Self::NodeCountTooLarge { node_count } => {
                write!(f, "node_count {node_count} does not fit usize")
            },
            | Self::NodeOutOfBounds { node, node_count } => {
                write!(f, "node {node} is outside 0..{node_count}")
            },
            | Self::EdgeOutOfBounds {
                source,
                target,
                node_count,
            } => write!(
                f,
                "edge {source}->{target} targets a node outside 0..{node_count}"
            ),
            | Self::ArithmeticOverflow => f.write_str("graph algorithm arithmetic overflowed"),
        }
    }
}

impl Error for GraphValidationError
{
}

/// A typed topological failure carrying concrete cycle evidence when available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CycleError
{
    /// The graph boundary was invalid before cycle-specific work could run.
    InvalidGraph(GraphValidationError),
    /// The graph is cyclic and the witness names the closed walk.
    Cycle
    {
        witness: CycleWitness
    },
}

impl From<GraphValidationError> for CycleError
{
    #[inline]
    fn from(value: GraphValidationError) -> Self
    {
        Self::InvalidGraph(value)
    }
}

impl Display for CycleError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        match *self {
            | Self::InvalidGraph(ref error) => Display::fmt(error, f),
            | Self::Cycle { ref witness } => write!(
                f,
                "cycle witness with {} nodes and {} edges",
                witness.nodes.len(),
                witness.edges.len()
            ),
        }
    }
}

impl Error for CycleError
{
    #[inline]
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match *self {
            | Self::InvalidGraph(ref error) => Some(error),
            | Self::Cycle { .. } => None,
        }
    }
}

/// Whether a graph cycle was detected.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CycleDetected(bool);

impl From<bool> for CycleDetected
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<CycleDetected> for bool
{
    #[inline]
    fn from(value: CycleDetected) -> Self
    {
        value.0
    }
}

impl Display for CycleDetected
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Whether a destination is reachable from a source.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PathExists(bool);

impl From<bool> for PathExists
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<PathExists> for bool
{
    #[inline]
    fn from(value: PathExists) -> Self
    {
        value.0
    }
}

impl Display for PathExists
{
    #[inline]
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        Display::fmt(&self.0, f)
    }
}

/// Whether a node has a non-empty path back to itself.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct NonEmptySelfPath(bool);

impl From<bool> for NonEmptySelfPath
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<NonEmptySelfPath> for bool
{
    #[inline]
    fn from(value: NonEmptySelfPath) -> Self
    {
        value.0
    }
}

/// Color state for iterative depth-first cycle search.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DfsColor
{
    /// Node has not been reached by DFS.
    Unvisited,
    /// Node is on the active DFS stack.
    Active,
    /// Node and all descendants have been closed.
    Finished,
}

/// A concrete closed cycle witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CycleWitness
{
    /// Closed node walk; first and last entries are equal.
    pub nodes: Vec<NodeId>,
    /// Directed edges between adjacent nodes in `nodes`.
    pub edges: Vec<EdgeId>,
}

/// Canonical strongly-connected component rows.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StronglyConnectedComponents
{
    /// Components sorted by their first node; nodes inside each component are
    /// ascending.
    pub components: Vec<Vec<NodeId>>,
}

/// Reachability row for one source node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReachabilityRow
{
    /// Source node for this row.
    pub source: NodeId,
    /// Reachable targets in ascending order, excluding `source` unless a cycle
    /// reaches it.
    pub targets: Vec<NodeId>,
}

/// Canonical transitive reachability rows.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reachability
{
    /// One row per source node in ascending source order.
    pub rows: Vec<ReachabilityRow>,
}

/// Canonical DAG closure and reduction result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitiveReductionClosure
{
    /// Transitive closure rows for the DAG.
    pub closure: Reachability,
    /// Transitive-reduction edges in ascending `(source, target)` order.
    pub reduction_edges: Vec<EdgeId>,
}

/// Dominator row for one node relative to a start node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DominatorRow
{
    /// Node described by this row.
    pub node: NodeId,
    /// Immediate dominator, or `None` for the start node and unreachable nodes.
    pub immediate: Option<NodeId>,
    /// Full dominator set in ascending node order.
    pub dominators: Vec<NodeId>,
}

/// Immediate-dominator result rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImmediateDominators
{
    /// Start node used for the computation.
    pub start: NodeId,
    /// One row per node in ascending order.
    pub rows: Vec<DominatorRow>,
}

/// Shortest unweighted distance row.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortestPathRow
{
    /// Reachable target node.
    pub node: NodeId,
    /// Number of edges in a shortest path from the start node.
    pub distance: PathLength,
}

/// Shortest unweighted path lengths from one start node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortestPathLengths
{
    /// Start node used for the computation.
    pub start: NodeId,
    /// Reachable rows in ascending node order.
    pub rows: Vec<ShortestPathRow>,
}

/// All bounded simple paths between two nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllSimplePaths
{
    /// Start node.
    pub start: NodeId,
    /// End node.
    pub end: NodeId,
    /// Maximum edge count admitted by the search.
    pub max_depth: PathLength,
    /// Canonical lexicographic path list.
    pub paths: Vec<Vec<NodeId>>,
}

/// Condensed graph over strongly-connected components.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Condensation
{
    /// SCC rows, sorted canonically.
    pub components: Vec<Vec<NodeId>>,
    /// Deduplicated component edges in ascending order.
    pub edges: Vec<ComponentEdge>,
}

/// Return a deterministic topological order for an acyclic graph.
///
/// # Contract
/// - requires: `graph` exposes dense nodes in `0..graph.node_count()`.
/// - ensures: on acyclic input, returns a node order where each input edge
///   source precedes its target.
/// - fails: returns [`CycleError::InvalidGraph`] for invalid boundaries and
///   [`CycleError::Cycle`] for cyclic input.
/// - panics: none.
/// - intension: the success order is petgraph [`toposort`] over the
///   deterministic [`View`] traversal.
///
/// # Errors
/// Returns [`CycleError::InvalidGraph`] for boundary failures and
/// [`CycleError::Cycle`] for cyclic graphs.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named DAG distinguishes the exact petgraph
///   traversal order, and the named three-cycle distinguishes the typed cyclic
///   error variant.
/// - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
/// - witness: `gandr_theory_graphs::algorithms::contracts::cycle_witness_contract`
#[inline]
pub fn topological_sort<G>(graph: &G) -> Result<Vec<NodeId>, CycleError>
where
    G: EdgeSource,
{
    let adjacency = adjacency_rows(graph)?;
    let view = View::new(graph);
    let Ok(order) = toposort(&view, None)
    else {
        let maybe_witness = cycle_witness_from_adjacency(&adjacency, graph.node_count())?;
        let witness = maybe_witness.ok_or(GraphValidationError::ArithmeticOverflow)?;
        return Err(CycleError::Cycle { witness });
    };
    Ok(order)
}

/// Return canonical strongly-connected components.
///
/// # Contract
/// - requires: `graph` exposes dense nodes in `0..graph.node_count()`.
/// - ensures: returns rows whose members are strongly connected in the input
///   graph.
/// - provides: canonical component rows as [`StronglyConnectedComponents`].
/// - fails: returns [`GraphValidationError`] for invalid boundaries.
/// - panics: none.
/// - intension: computes components with petgraph [`kosaraju_scc`] through
///   [`View`], then sorts row members and component rows by dense node order.
///
/// # Errors
/// Returns [`GraphValidationError`] when a successor target is out of bounds or
/// the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named component graph distinguishes the
///   exact canonical SCC rows and member ordering; the deep acyclic chain
///   distinguishes singleton SCCs without recursive native stack use.
/// - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
#[inline]
pub fn strongly_connected_components<G>(
    graph: &G
) -> Result<StronglyConnectedComponents, GraphValidationError>
where
    G: EdgeSource,
{
    adjacency_rows(graph)?;
    let view = View::new(graph);
    let mut components = kosaraju_scc(&view);
    canonicalize_components(&mut components);
    Ok(StronglyConnectedComponents { components })
}

/// Return a deterministic concrete cycle witness when one exists.
///
/// # Contract
/// - requires: `graph` exposes dense nodes in `0..graph.node_count()`.
/// - ensures: cyclic input returns `Ok(Some(_))` with a closed node walk and
///   directed adjacent edges drawn from the input graph.
/// - provides: `Ok(None)` when no cycle witness exists.
/// - fails: returns [`GraphValidationError`] for invalid boundaries.
/// - panics: none.
/// - intension: validates once by materializing sorted adjacency rows, then
///   extracts evidence with the crate's iterative DFS witness traversal.
///
/// # Errors
/// Returns [`GraphValidationError`] when a successor target is out of bounds or
/// the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the named two-node cycle validates the returned
///   closed walk and exact directed edges against the input edge set; L3
///   residue checks the exact `Some` observation and the deep acyclic chain
///   checks `None` without recursive native stack use.
/// - witness: `gandr_theory_graphs::algorithms::contracts::cycle_witness_contract`
#[inline]
pub fn cycle_witness<G>(graph: &G) -> Result<Option<CycleWitness>, GraphValidationError>
where
    G: EdgeSource,
{
    let adjacency = adjacency_rows(graph)?;
    cycle_witness_from_adjacency(&adjacency, graph.node_count())
}

/// Return whether the graph contains a cycle.
///
/// # Contract
/// - requires: `graph` exposes dense nodes in `0..graph.node_count()`.
/// - ensures: returns `Ok(true)` exactly when the valid graph is cyclic.
/// - fails: returns [`GraphValidationError`] for invalid boundaries.
/// - panics: none.
/// - intension: validates once by materializing sorted adjacency rows, then
///   derives the boolean from the crate's iterative DFS witness traversal.
///
/// # Errors
/// Returns [`GraphValidationError`] when a successor target is out of bounds or
/// the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named two-node cycle distinguishes the
///   `true` observation and ties it to witness existence; the deep acyclic
///   chain distinguishes `false` without recursive native stack use.
/// - witness: `gandr_theory_graphs::algorithms::contracts::cycle_witness_contract`
#[inline]
pub fn is_cyclic<G>(graph: &G) -> Result<CycleDetected, GraphValidationError>
where
    G: EdgeSource,
{
    let adjacency = adjacency_rows(graph)?;
    cycle_witness_from_adjacency(&adjacency, graph.node_count())
        .map(|witness| CycleDetected::from(witness.is_some()))
}

/// Return whether `end` is reachable from `start`.
///
/// # Contract
/// - requires: `start` and `end` are caller-selected dense node ids.
/// - ensures: returns whether a directed path of zero or more edges reaches
///   `end` from `start`.
/// - fails: returns [`GraphValidationError`] for invalid endpoints or
///   boundaries.
/// - panics: none.
/// - intension: validates both endpoints before petgraph
///   [`has_path_connecting`] traverses the private [`View`].
///
/// # Errors
/// Returns [`GraphValidationError`] when either endpoint or a successor target
/// is out of bounds, or when the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named path graph distinguishes exact `true`
///   and `false` reachability observations, and the one-node boundary graph
///   distinguishes the exact invalid-endpoint variant.
/// - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
#[inline]
pub fn has_path<G>(
    graph: &G,
    start: NodeId,
    end: NodeId,
) -> Result<PathExists, GraphValidationError>
where
    G: EdgeSource,
{
    validation_support::validate_node(start, graph.node_count())?;
    validation_support::validate_node(end, graph.node_count())?;
    adjacency_rows(graph)?;
    let view = View::new(graph);
    Ok(PathExists::from(has_path_connecting(
        &view, start, end, None,
    )))
}

/// Return the transitive closure and reduction of a DAG.
///
/// # Contract
/// - requires: `graph` exposes dense nodes in `0..graph.node_count()` and the
///   operation is defined only on acyclic graphs.
/// - ensures: on DAG input, returns a closure with the same reachability
///   relation and reduction edges with alternate-path-implied edges removed.
/// - provides: paired closure rows and reduction edges as
///   [`TransitiveReductionClosure`].
/// - fails: returns [`CycleError::InvalidGraph`] for invalid boundaries and
///   [`CycleError::Cycle`] for cyclic input.
/// - panics: none.
/// - intension: validates acyclicity with petgraph [`toposort`], derives the
///   closure through [`reachability`], and keeps exactly the canonical edges
///   that are not implied by an alternate successor path.
///
/// # Errors
/// Returns [`CycleError::InvalidGraph`] for boundary failures and
/// [`CycleError::Cycle`] for cyclic graphs.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named reduction DAG distinguishes exact
///   closure rows and reduction edges, and the named three-cycle distinguishes
///   the typed cyclic error variant.
/// - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
/// - witness: `gandr_theory_graphs::algorithms::contracts::cycle_witness_contract`
#[inline]
pub fn transitive_reduction_closure<G>(graph: &G) -> Result<TransitiveReductionClosure, CycleError>
where
    G: EdgeSource,
{
    let adjacency = adjacency_rows(graph)?;
    let view = View::new(graph);
    let Ok(_order) = toposort(&view, None)
    else {
        let maybe_witness = cycle_witness_from_adjacency(&adjacency, graph.node_count())?;
        let witness = maybe_witness.ok_or(GraphValidationError::ArithmeticOverflow)?;
        return Err(CycleError::Cycle { witness });
    };
    let closure = reachability(graph)?;
    let node_count = graph.node_count();
    let mut reduction_edges = Vec::new();
    for source in node_count.ids() {
        let source_position = validation_support::node_index(source, node_count)?;
        let Some(successors) = adjacency.get(usize::from(source_position))
        else {
            return Err(GraphValidationError::NodeOutOfBounds {
                node: source,
                node_count,
            }
            .into());
        };
        for &target in successors {
            let mut redundant = false;
            for &candidate in successors {
                if candidate == target {
                    continue;
                }
                let candidate_position = validation_support::node_index(candidate, node_count)?;
                let Some(candidate_row) = closure.rows.get(usize::from(candidate_position))
                else {
                    return Err(GraphValidationError::NodeOutOfBounds {
                        node: candidate,
                        node_count,
                    }
                    .into());
                };
                if candidate_row.targets.contains(&target) {
                    redundant = true;
                    break;
                }
            }
            if !redundant {
                reduction_edges.push(EdgeId::new(source, target));
            }
        }
    }
    reduction_edges.sort_unstable();
    Ok(TransitiveReductionClosure {
        closure,
        reduction_edges,
    })
}

/// Return immediate dominators for every node relative to `start`.
///
/// # Contract
/// - requires: `start` is a caller-selected dense node id.
/// - ensures: reachable rows contain each node's immediate dominator and full
///   dominator set relative to `start`; unreachable rows have no immediate
///   dominator and an empty dominator set.
/// - provides: one dominator row per dense node as [`ImmediateDominators`].
/// - fails: returns [`GraphValidationError`] for an invalid `start` or
///   boundary.
/// - panics: none.
/// - intension: computes dominance with petgraph [`dominators::simple_fast`]
///   and sorts each dominator set by dense node order.
///
/// # Errors
/// Returns [`GraphValidationError`] when the start node or a successor target
/// is out of bounds, or when the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named dominator graph distinguishes the
///   exact immediate-dominator rows, sorted dominator sets, and
///   unreachable-node row.
/// - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
#[inline]
pub fn immediate_dominators<G>(
    graph: &G,
    start: NodeId,
) -> Result<ImmediateDominators, GraphValidationError>
where
    G: EdgeSource,
{
    validation_support::validate_node(start, graph.node_count())?;
    adjacency_rows(graph)?;
    let view = View::new(graph);
    let dominators = dominators::simple_fast(&view, start);
    let node_len = validation_support::usize_node_count(graph.node_count())?;
    let mut rows = Vec::with_capacity(usize::from(node_len));
    for node in graph.node_count().ids() {
        let immediate = dominators.immediate_dominator(node);
        let mut node_dominators = dominators
            .dominators(node)
            .map_or_else(Vec::new, Iterator::collect);
        node_dominators.sort_unstable();
        rows.push(DominatorRow {
            node,
            immediate,
            dominators: node_dominators,
        });
    }
    Ok(ImmediateDominators { start, rows })
}

/// Return shortest unweighted path lengths from `start`.
///
/// # Contract
/// - requires: `start` is a caller-selected dense node id.
/// - ensures: each returned distance is the minimum directed edge count from
///   `start` to the row node.
/// - provides: reachable target rows as [`ShortestPathLengths`].
/// - fails: returns [`GraphValidationError`] for an invalid `start` or
///   boundary.
/// - panics: none. [`dijkstra`](fn@dijkstra) and emits rows sorted by target
///   node.
///
/// # Errors
/// Returns [`GraphValidationError`] when the start node or a successor target
/// is out of bounds, or when the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named path graph distinguishes exact
///   reachable shortest-path rows and the omission of disconnected targets.
/// - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
#[inline]
pub fn shortest_path_lengths<G>(
    graph: &G,
    start: NodeId,
) -> Result<ShortestPathLengths, GraphValidationError>
where
    G: EdgeSource,
{
    validation_support::validate_node(start, graph.node_count())?;
    adjacency_rows(graph)?;
    let view = View::new(graph);
    let distances = dijkstra(&view, start, None, |_| 1_u32);
    let mut rows = Vec::new();
    for node in graph.node_count().ids() {
        if let Some(distance) = distances.get(&node).copied() {
            rows.push(ShortestPathRow {
                node,
                distance: PathLength::from(distance),
            });
        }
    }
    Ok(ShortestPathLengths { start, rows })
}

/// Return every simple path from `start` to `end` with at most `max_depth`
/// edges.
///
/// # Contract
/// - requires: `start` and `end` are caller-selected dense node ids.
/// - ensures: returned paths are directed paths from `start` to `end` within
///   `max_depth`.
/// - provides: bounded path rows as [`AllSimplePaths`].
/// - fails: returns [`GraphValidationError`] for invalid endpoints or
///   boundaries.
/// - panics: none.
/// - intension: validates endpoints and adjacency first, returns the canonical
///   trivial path directly for `start == end`, otherwise enumerates non-empty
///   paths with petgraph [`all_simple_paths`] and canonicalizes the rows.
///
/// # Errors
/// Returns [`GraphValidationError`] when either endpoint or a successor target
/// is out of bounds, or when the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named path graph distinguishes the exact
///   two length-two paths in lexicographic order and the empty result at depth
///   one; the trivial-path witness distinguishes validation-only adjacency
///   inspection from petgraph enumeration when `start == end`.
/// - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
#[inline]
pub fn all_simple_paths<G>(
    graph: &G,
    start: NodeId,
    end: NodeId,
    max_depth: PathLength,
) -> Result<AllSimplePaths, GraphValidationError>
where
    G: EdgeSource,
{
    validation_support::validate_node(start, graph.node_count())?;
    validation_support::validate_node(end, graph.node_count())?;
    adjacency_rows(graph)?;
    if start == end {
        return Ok(AllSimplePaths {
            start,
            end,
            max_depth,
            paths: vec![vec![start]],
        });
    }
    let view = View::new(graph);
    let max_intermediate_count = usize::try_from(max_depth.saturating_predecessor())
        .map_err(|_conversion_error| GraphValidationError::ArithmeticOverflow)?;
    let max_intermediate = Some(max_intermediate_count);
    let mut paths = petgraph_all_simple_paths::<Vec<NodeId>, _, NodeIdBuildHasher>(
        &view,
        start,
        end,
        0,
        max_intermediate,
    )
    .collect::<Vec<_>>();
    if u32::from(max_depth) == 0 {
        paths.clear();
    }
    paths.sort();
    paths.dedup();
    Ok(AllSimplePaths {
        start,
        end,
        max_depth,
        paths,
    })
}

/// Return the SCC condensation graph.
///
/// # Contract
/// - requires: `graph` exposes dense nodes in `0..graph.node_count()`.
/// - ensures: each original edge crossing SCC boundaries appears as one
///   component edge after condensation.
/// - provides: canonical component rows and deduplicated component edges as
///   [`Condensation`].
/// - fails: returns [`GraphValidationError`] for invalid boundaries.
/// - panics: none.
/// - intension: materializes an owned petgraph graph only for petgraph
///   [`condensation`], then sorts component rows and component-edge pairs by
///   dense order.
///
/// # Errors
/// Returns [`GraphValidationError`] when a successor target is out of bounds or
/// the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named component graph distinguishes exact
///   condensation components and sorted deduplicated component edges.
/// - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
#[inline]
pub fn condensation<G>(graph: &G) -> Result<Condensation, GraphValidationError>
where
    G: EdgeSource,
{
    let adjacency = adjacency_rows(graph)?;
    let owned = owned_petgraph_from_adjacency::<DefaultIx>(&adjacency, graph.node_count())?;
    let condensed = petgraph_condensation(owned, true);
    materialize_condensation(&condensed)
}
/// Build sorted, validated adjacency rows for boundary checks and witnesses.
///
/// # Errors
/// Returns [`GraphValidationError::NodeCountTooLarge`] when the dense bound
/// does not fit the host address space, or
/// [`GraphValidationError::EdgeOutOfBounds`] when a successor escapes that
/// bound.
#[inline]
pub fn adjacency_rows<G>(graph: &G) -> Result<Vec<Vec<NodeId>>, GraphValidationError>
where
    G: EdgeSource,
{
    let node_count = graph.node_count();
    let node_len = validation_support::usize_node_count(node_count)?;
    validation_support::validate_adjacency_row_layout(node_count, node_len)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(usize::from(node_len))
        .map_err(|_reserve_error| GraphValidationError::NodeCountTooLarge { node_count })?;
    for source in node_count.ids() {
        let mut row = Vec::new();
        for target in graph.successors(source) {
            validation_support::validate_node(target, node_count).map_err(|validation_error| {
                if let GraphValidationError::NodeOutOfBounds {
                    node,
                    node_count: graph_node_count,
                } = validation_error
                {
                    GraphValidationError::EdgeOutOfBounds {
                        source,
                        target: node,
                        node_count: graph_node_count,
                    }
                }
                else {
                    validation_error
                }
            })?;
            row.push(target);
        }
        row.sort_unstable();
        row.dedup();
        rows.push(row);
    }
    Ok(rows)
}
/// Iteratively detect the first deterministic cycle in validated adjacency.
fn cycle_witness_from_adjacency(
    adjacency: &[Vec<NodeId>],
    node_count: NodeCount,
) -> Result<Option<CycleWitness>, GraphValidationError>
{
    let node_len = validation_support::usize_node_count(node_count)?;
    let mut color = vec![DfsColor::Unvisited; usize::from(node_len)];
    let mut active_stack = Vec::<NodeId>::new();
    for root in node_count.ids() {
        let root_index = validation_support::node_index(root, node_count)?;
        if color_at(&color, root_index) != DfsColor::Unvisited {
            continue;
        }
        let mut frames = vec![DfsFrame {
            node: root,
            next_child: 0,
        }];
        let root_color = color
            .get_mut(usize::from(root_index))
            .ok_or(GraphValidationError::ArithmeticOverflow)?;
        *root_color = DfsColor::Active;
        active_stack.push(root);
        while !frames.is_empty() {
            let frame_position = frames
                .len()
                .checked_sub(1)
                .ok_or(GraphValidationError::ArithmeticOverflow)?;
            let Some(frame) = frames.get_mut(frame_position)
            else {
                return Err(GraphValidationError::ArithmeticOverflow);
            };
            let row_index = validation_support::node_index(frame.node, node_count)?;
            let Some(row) = adjacency.get(usize::from(row_index))
            else {
                return Err(GraphValidationError::NodeOutOfBounds {
                    node: frame.node,
                    node_count,
                });
            };
            let Some(target) = row.get(frame.next_child).copied()
            else {
                let finished_frame = frames
                    .pop()
                    .ok_or(GraphValidationError::ArithmeticOverflow)?;
                let finished = finished_frame.node;
                let finished_index = validation_support::node_index(finished, node_count)?;
                let finished_color = color
                    .get_mut(usize::from(finished_index))
                    .ok_or(GraphValidationError::ArithmeticOverflow)?;
                *finished_color = DfsColor::Finished;
                let removed = active_stack
                    .pop()
                    .ok_or(GraphValidationError::ArithmeticOverflow)?;
                if removed != finished {
                    return Err(GraphValidationError::ArithmeticOverflow);
                }
                continue;
            };
            frame.next_child = frame
                .next_child
                .checked_add(1_usize)
                .ok_or(GraphValidationError::ArithmeticOverflow)?;
            let target_index = validation_support::node_index(target, node_count)?;
            let target_color = color_at(&color, target_index);
            if target_color == DfsColor::Unvisited {
                let color_slot = color
                    .get_mut(usize::from(target_index))
                    .ok_or(GraphValidationError::ArithmeticOverflow)?;
                *color_slot = DfsColor::Active;
                active_stack.push(target);
                frames.push(DfsFrame {
                    node: target,
                    next_child: 0,
                });
            }
            else if target_color == DfsColor::Active {
                return Ok(Some(cycle_from_stack(&active_stack, target, frame.node)));
            }
        }
    }
    Ok(None)
}

/// A DFS frame for iterative cycle detection.
#[derive(Clone, Debug)]
struct DfsFrame
{
    /// Node under exploration.
    node: NodeId,
    /// Next successor index to inspect.
    next_child: usize,
}

/// Deterministic hasher builder for dense `u32` node identities.
#[derive(Clone, Default)]
struct NodeIdBuildHasher;

impl BuildHasher for NodeIdBuildHasher
{
    type Hasher = NodeIdHasher;

    #[inline]
    fn build_hasher(&self) -> Self::Hasher
    {
        NodeIdHasher { state: 0_u64 }
    }
}

/// Deterministic hasher optimized for dense `u32` node identities.
#[repr(transparent)]
#[derive(Clone, Default)]
struct NodeIdHasher
{
    /// Current hash state.
    state: u64,
}

impl Hasher for NodeIdHasher
{
    #[inline]
    fn finish(&self) -> u64
    {
        self.state
    }

    #[inline]
    fn write(
        &mut self,
        bytes: &[u8],
    )
    {
        for &byte in bytes {
            self.state ^= u64::from(byte);
            self.state = self.state.rotate_left(8);
        }
    }

    #[inline]
    fn write_u32(
        &mut self,
        i: u32,
    )
    {
        self.state = u64::from(i);
    }
}

/// Checked dense-boundary conversions and layout validation helpers.
mod validation_support
{
    use super::*;

    /// Convert a `u32` node count to `usize`.
    pub(super) fn usize_node_count(
        node_count: NodeCount
    ) -> Result<NodeCapacity, GraphValidationError>
    {
        NodeCapacity::try_from(node_count)
            .map_err(|_conversion_error| GraphValidationError::NodeCountTooLarge { node_count })
    }

    /// Convert a node id to an addressable vector position.
    pub(super) fn node_index(
        node: NodeId,
        node_count: NodeCount,
    ) -> Result<NodePosition, GraphValidationError>
    {
        validate_node(node, node_count)?;
        NodePosition::try_from(node)
            .map_err(|_conversion_error| GraphValidationError::NodeCountTooLarge { node_count })
    }

    /// Validate the outer adjacency-row vector layout before reserving storage.
    pub(super) fn validate_adjacency_row_layout(
        node_count: NodeCount,
        node_len: NodeCapacity,
    ) -> Result<(), GraphValidationError>
    {
        Layout::array::<Vec<NodeId>>(usize::from(node_len))
            .map(|_layout| ())
            .map_err(|_layout_error| GraphValidationError::NodeCountTooLarge { node_count })
    }

    /// Validate that `node` is inside `0..node_count`.
    pub(super) fn validate_node(
        node: NodeId,
        node_count: NodeCount,
    ) -> Result<(), GraphValidationError>
    {
        if u32::from(node) < u32::from(node_count) {
            Ok(())
        }
        else {
            Err(GraphValidationError::NodeOutOfBounds { node, node_count })
        }
    }
}
/// Materialize an owned petgraph graph from validated adjacency rows.
fn owned_petgraph_from_adjacency<Ix>(
    adjacency: &[Vec<NodeId>],
    node_count: NodeCount,
) -> Result<DiGraph<NodeId, PetgraphEdge, Ix>, GraphValidationError>
where
    Ix: IndexType,
{
    let node_len = validation_support::usize_node_count(node_count)?;
    let mut graph = DiGraph::<NodeId, PetgraphEdge, Ix>::with_capacity(usize::from(node_len), 0);
    let mut nodes = Vec::with_capacity(usize::from(node_len));
    for node in node_count.ids() {
        nodes.push(graph.add_node(node));
    }
    for source in node_count.ids() {
        let source_position = validation_support::node_index(source, node_count)?;
        let Some(row) = adjacency.get(usize::from(source_position))
        else {
            return Err(GraphValidationError::NodeOutOfBounds {
                node: source,
                node_count,
            });
        };
        let source_index = node_index_from_nodes(&nodes, source, node_count)?;
        for &target in row {
            let target_index = node_index_from_nodes(&nodes, target, node_count)?;
            graph.add_edge(source_index, target_index, PetgraphEdge);
        }
    }
    Ok(graph)
}

/// Return canonical reachability rows for every source.
///
/// # Contract
/// - requires: `graph` exposes dense nodes in `0..graph.node_count()`.
/// - ensures: each row contains targets reachable from its source by one or
///   more directed edges.
/// - provides: one row per source node as [`Reachability`].
/// - fails: returns [`GraphValidationError`] for invalid boundaries.
/// - panics: none.
/// - intension: classifies reachability with petgraph [`has_path_connecting`]
///   over [`View`] and emits rows sorted by source and target nodes.
///
/// # Errors
/// Returns [`GraphValidationError`] when a successor target is out of bounds or
/// the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the named path graph distinguishes the exact
///   reachability rows, and the invalid-edge graph distinguishes the exact
///   `EdgeOutOfBounds` variant.
/// - witness: `gandr_theory_graphs::algorithms::contracts::algorithm_menu_contract`
#[inline]
pub fn reachability<G>(graph: &G) -> Result<Reachability, GraphValidationError>
where
    G: EdgeSource,
{
    let adjacency = adjacency_rows(graph)?;
    let view = View::new(graph);
    let node_count = graph.node_count();
    let node_len = validation_support::usize_node_count(node_count)?;
    let mut rows = Vec::with_capacity(usize::from(node_len));
    for source in node_count.ids() {
        let mut targets = Vec::new();
        for target in node_count.ids() {
            let mut reachable = has_path_connecting(&view, source, target, None);
            if source == target {
                let self_reachable =
                    has_non_empty_path_to_self(&view, &adjacency, node_count, source)?;
                reachable = bool::from(self_reachable);
            }
            if reachable {
                targets.push(target);
            }
        }
        rows.push(ReachabilityRow { source, targets });
    }
    Ok(Reachability { rows })
}

/// Return whether `source` can reach itself through at least one edge.
fn has_non_empty_path_to_self<G>(
    view: &View<'_, G>,
    adjacency: &[Vec<NodeId>],
    node_count: NodeCount,
    source: NodeId,
) -> Result<NonEmptySelfPath, GraphValidationError>
where
    G: EdgeSource,
{
    let source_index = validation_support::node_index(source, node_count)?;
    let Some(row) = adjacency.get(usize::from(source_index))
    else {
        return Err(GraphValidationError::NodeOutOfBounds {
            node: source,
            node_count,
        });
    };
    for &target in row {
        if has_path_connecting(view, target, source, None) {
            return Ok(NonEmptySelfPath::from(true));
        }
    }
    Ok(NonEmptySelfPath::from(false))
}

/// Convert petgraph condensation output to canonical public rows.
fn materialize_condensation<Ix>(
    graph: &DiGraph<Vec<NodeId>, PetgraphEdge, Ix>
) -> Result<Condensation, GraphValidationError>
where
    Ix: IndexType,
{
    let mut indexed_components = Vec::new();
    for node in graph.node_indices() {
        let Some(component) = graph.node_weight(node)
        else {
            return Err(GraphValidationError::ArithmeticOverflow);
        };
        let mut members = component.clone();
        members.sort_unstable();
        members.dedup();
        indexed_components.push((node, members));
    }

    let mut components = indexed_components
        .iter()
        .cloned()
        .map(|(_, component)| component)
        .collect::<Vec<_>>();
    canonicalize_components(&mut components);

    let component_count = components.len();
    let mut condensed_to_public = vec![None; component_count];
    for indexed_component in &indexed_components {
        let Some(public_index) = components
            .iter()
            .position(|candidate| candidate == &indexed_component.1)
        else {
            return Err(GraphValidationError::ArithmeticOverflow);
        };
        let Some(slot) = condensed_to_public.get_mut(indexed_component.0.index())
        else {
            return Err(GraphValidationError::ArithmeticOverflow);
        };
        let public_component_raw = u32::try_from(public_index)
            .map_err(|_conversion_error| GraphValidationError::ArithmeticOverflow)?;
        let public_component = ComponentIndex::from(public_component_raw);
        *slot = Some(public_component);
    }

    let mut edges = Vec::new();
    for edge in graph.edge_references() {
        let source_component = public_component_index(&condensed_to_public, edge.source())?;
        let target_component = public_component_index(&condensed_to_public, edge.target())?;
        if source_component != target_component {
            edges.push(ComponentEdge::new(source_component, target_component));
        }
    }
    edges.sort_unstable();
    edges.dedup();
    Ok(Condensation { components, edges })
}
/// Sort components by first member and sort members inside each component.
fn canonicalize_components(components: &mut [Vec<NodeId>])
{
    for component in components.iter_mut() {
        component.sort_unstable();
        component.dedup();
    }
    components.sort_by(|left, right| left.first().cmp(&right.first()));
}
/// Resolve a condensed petgraph node to its public component index.
fn public_component_index<Ix>(
    map: &[Option<ComponentIndex>],
    node: PetNodeIndex<Ix>,
) -> Result<ComponentIndex, GraphValidationError>
where
    Ix: IndexType,
{
    map.get(node.index())
        .copied()
        .flatten()
        .ok_or(GraphValidationError::ArithmeticOverflow)
}
/// Return a DFS color, treating missing slots as unvisited for defensive
/// bounds.
fn color_at(
    colors: &[DfsColor],
    index: NodePosition,
) -> DfsColor
{
    colors
        .get(usize::from(index))
        .copied()
        .unwrap_or(DfsColor::Unvisited)
}

/// Look up a petgraph node index by dense `u32` node id.
fn node_index_from_nodes<Ix>(
    nodes: &[PetNodeIndex<Ix>],
    node: NodeId,
    node_count: NodeCount,
) -> Result<PetNodeIndex<Ix>, GraphValidationError>
where
    Ix: IndexType,
{
    let position = validation_support::node_index(node, node_count)?;
    nodes
        .get(usize::from(position))
        .copied()
        .ok_or(GraphValidationError::NodeOutOfBounds { node, node_count })
}

/// Convert an active stack back-edge into a closed witness.
fn cycle_from_stack(
    active_stack: &[NodeId],
    target: NodeId,
    source: NodeId,
) -> CycleWitness
{
    let mut nodes = Vec::new();
    let mut inside = false;
    for &node in active_stack {
        if node == target {
            inside = true;
        }
        if inside {
            nodes.push(node);
        }
    }
    if nodes.last().copied() != Some(source) {
        nodes.push(source);
    }
    nodes.push(target);
    let mut edges = Vec::new();
    let mut previous = None;
    for &node in &nodes {
        if let Some(left) = previous {
            edges.push(EdgeId::new(left, node));
        }
        previous = Some(node);
    }
    CycleWitness { nodes, edges }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn adjacency_row_layout_boundary_is_typed_before_allocation()
    {
        let maximum_representable = maximum_adjacency_row_layout_len();
        let maximum_representable_raw = usize::from(maximum_representable);
        let first_unrepresentable = maximum_representable_raw.saturating_add(1);

        if let Ok(node_count) = u32::try_from(maximum_representable_raw) {
            let typed_count = NodeCount::from(node_count);
            let typed_len = NodeCapacity::try_from(typed_count).expect("test node count fits");
            assert!(
                validation_support::validate_adjacency_row_layout(typed_count, typed_len).is_ok(),
                "largest representable adjacency row count must pass the layout guard"
            );
        }

        if let Ok(node_count) = u32::try_from(first_unrepresentable) {
            let typed_count = NodeCount::from(node_count);
            let typed_len = NodeCapacity::try_from(typed_count).unwrap_or_default();
            assert_eq!(
                validation_support::validate_adjacency_row_layout(typed_count, typed_len),
                Err(GraphValidationError::NodeCountTooLarge {
                    node_count: typed_count
                })
            );
        }
        else {
            let node_count = NodeCount::from(u32::MAX);
            let Ok(node_len) = NodeCapacity::try_from(node_count)
            else {
                return;
            };
            assert!(
                validation_support::validate_adjacency_row_layout(node_count, node_len).is_ok(),
                "host usize layout can represent every u32 adjacency row count"
            );
        }
    }

    fn maximum_adjacency_row_layout_len() -> NodeCapacity
    {
        let mut low = 0usize;
        let mut high = usize::MAX;
        while low < high {
            let midpoint = low.saturating_add(high.saturating_sub(low).div_ceil(2));
            if Layout::array::<Vec<NodeId>>(midpoint).is_ok() {
                low = midpoint;
            }
            else {
                high = midpoint.saturating_sub(1);
            }
        }
        NodeCapacity::from(low)
    }
}
