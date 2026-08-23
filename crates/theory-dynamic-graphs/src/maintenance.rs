//! Incremental **acyclicity maintenance**: a topological order of a directed
//! graph kept current under edge insertion.
//!
//! [`AcyclicityMaintenance`] holds every admitted node in a
//! [`OrderMaintenance`] whose list order *is* the topological order, so
//! comparing two nodes is one integer comparison. An insertion the standing
//! order already witnesses is admitted on that comparison alone. An insertion
//! that runs against the order is repaired by a bounded two-way search around
//! the offending pair, which either finds the cycle the insertion would close
//! or relocates exactly the nodes whose relative order the insertion changed.
//!
//! # The search is bounded by the order, not by the graph
//!
//! When `source` already sits after `target`, only nodes lying between them in
//! the standing order can have their relative order changed by the new edge.
//! The forward search from `target` therefore follows successors only while
//! they precede `source`, and the backward search from `source` follows
//! predecessors only while they follow `target`. Everything outside that window
//! keeps its position, and the repair touches nothing outside the two searched
//! sets — which is what makes the cost a function of the affected region rather
//! than of the graph.
//!
//! # The relocation preserves the slots it found
//!
//! The two searched sets are relocated into **exactly the order positions they
//! already occupied**, with the backward set's members first and the forward
//! set's after them. Nodes interleaved between those positions that belong to
//! neither set are not moved and keep their neighbours, which is what keeps
//! their relation to the relocated nodes correct without inspecting them.

use alloc::vec::Vec;
use core::cmp::Ordering;

use gandr_theory_graphs::CycleWitness;
use gandr_theory_graphs::EdgeId;
use gandr_theory_graphs::EdgeSource;
use gandr_theory_graphs::NodeCount;
use gandr_theory_graphs::NodeId;
use gandr_theory_orders::OrderError;
use gandr_theory_orders::OrderMaintenance;
use gandr_theory_orders::Pos;

use crate::slot::SlotIndex;

/// The number of edges an [`AcyclicityMaintenance`] currently holds.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AdmittedEdgeCount(u64);

impl AdmittedEdgeCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<AdmittedEdgeCount> for u64
{
    #[inline]
    fn from(value: AdmittedEdgeCount) -> Self
    {
        return value.0;
    }
}

/// The number of edge insertions offered to an [`AcyclicityMaintenance`].
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InsertionCount(u64);

impl InsertionCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    pub(crate) fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<InsertionCount> for u64
{
    #[inline]
    fn from(value: InsertionCount) -> Self
    {
        return value.0;
    }
}

/// The number of insertions that ran against the standing order and were
/// repaired.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RepairCount(u64);

impl RepairCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<RepairCount> for u64
{
    #[inline]
    fn from(value: RepairCount) -> Self
    {
        return value.0;
    }
}

/// The number of insertions refused because they would close a cycle.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RefusalCount(u64);

impl RefusalCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<RefusalCount> for u64
{
    #[inline]
    fn from(value: RefusalCount) -> Self
    {
        return value.0;
    }
}

/// The number of nodes the bounded searches have reached, across every
/// insertion.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct VisitCount(u64);

impl VisitCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<VisitCount> for u64
{
    #[inline]
    fn from(value: VisitCount) -> Self
    {
        return value.0;
    }
}

/// The number of nodes moved within the maintained order, across every
/// insertion.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelocationCount(u64);

impl RelocationCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<RelocationCount> for u64
{
    #[inline]
    fn from(value: RelocationCount) -> Self
    {
        return value.0;
    }
}

/// Whether the maintained order is a topological order of the admitted edges.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TopologicalOrderStatus(bool);

impl From<TopologicalOrderStatus> for bool
{
    #[inline]
    fn from(value: TopologicalOrderStatus) -> Self
    {
        return value.0;
    }
}

impl core::ops::Not for TopologicalOrderStatus
{
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output
    {
        return Self(!self.0);
    }
}

/// A search epoch, stamped into the reusable mark buffers so a bounded search
/// clears its state in constant time instead of rewriting the buffers.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Epoch(u64);

impl Epoch
{
    /// The epoch no search ever runs under, so a freshly grown buffer entry is
    /// unmarked for every real search.
    const UNVISITED: Self = Self(0);

    /// This epoch advanced by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

/// A failure of an [`AcyclicityMaintenance`] operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MaintenanceError
{
    /// The maintained order could not admit or relocate an element.
    #[error("the maintained order rejected an element: {0}")]
    Order(#[from] OrderError),
    /// A node identifier does not address a slot of this structure.
    #[error("a node identifier does not address a maintained node")]
    NodeCapacity,
    /// A handle the structure still tracks no longer resolves in the
    /// maintained order.
    #[error("the maintained order and the position table disagree")]
    OrderDesynchronized,
    /// An internal checked arithmetic operation overflowed.
    #[error("an internal checked arithmetic operation overflowed")]
    ArithmeticOverflow,
}

/// The verdict on one offered edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EdgeVerdict
{
    /// The standing order already places the source before the target, so the
    /// edge was recorded without touching the order.
    Admitted,
    /// The edge ran against the standing order and was recorded after
    /// relocating the affected region.
    AdmittedAfterRepair,
    /// The edge would close a cycle and was **not** recorded; the witness is
    /// the closed walk it would have closed.
    Refused(CycleWitness),
}

/// The work an [`AcyclicityMaintenance`] has performed since construction.
///
/// The two search counters are what price the structure against a batch
/// recheck: a batch acyclicity check costs one pass over every node and edge
/// per insertion, while [`MaintenanceTelemetry::nodes_visited`] and
/// [`MaintenanceTelemetry::nodes_relocated`] together are the whole cost the
/// maintenance paid.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MaintenanceTelemetry
{
    /// Edges offered to [`AcyclicityMaintenance::insert_edge`].
    pub insertions: InsertionCount,
    /// Offered edges that ran against the standing order and were repaired.
    pub repairs: RepairCount,
    /// Offered edges refused as cycle-closing.
    pub refusals: RefusalCount,
    /// Nodes reached by the bounded searches.
    pub nodes_visited: VisitCount,
    /// Nodes moved within the maintained order.
    pub nodes_relocated: RelocationCount,
}

/// Reusable per-insertion search state, kept across insertions so a bounded
/// repair allocates nothing once the buffers have grown.
#[derive(Debug, Default)]
struct Scratch
{
    /// The epoch at which each node was last reached by a forward search.
    forward_mark: Vec<Epoch>,
    /// The epoch at which each node was last reached by a backward search.
    backward_mark: Vec<Epoch>,
    /// The forward search's predecessor of each node, meaningful exactly where
    /// [`Scratch::forward_mark`] carries the current epoch.
    forward_parent: Vec<NodeId>,
    /// The epoch of the search most recently begun.
    epoch: Epoch,
    /// The explicit depth-first work stack; recursion is never used, so the
    /// search depth is bounded by the heap rather than the native stack.
    stack: Vec<NodeId>,
}

/// What the bounded two-way search around a violating insertion found.
enum Discovery
{
    /// The insertion closes a cycle; the witness is that closed walk.
    Cycle(CycleWitness),
    /// The insertion is admissible once the affected region is relocated.
    Region
    {
        /// Ancestors of the edge's source that follow the edge's target in the
        /// standing order.
        backward: Vec<NodeId>,
        /// Descendants of the edge's target that precede the edge's source in
        /// the standing order.
        forward: Vec<NodeId>,
    },
}

/// A directed graph whose **topological order is maintained under edge
/// insertion**, refusing exactly the edges that would close a cycle.
///
/// Nodes are dense [`NodeId`]s and are created on demand: an insertion naming a
/// node the structure has not seen appends it to the end of the order, which is
/// always topologically valid because a fresh node has no edges.
pub struct AcyclicityMaintenance
{
    /// The maintained topological order; each element's payload is the node it
    /// orders, and list order is topological order.
    order: OrderMaintenance<NodeId>,
    /// Each node's handle into [`AcyclicityMaintenance::order`], indexed by
    /// dense node id.
    positions: Vec<Pos>,
    /// Admitted outgoing edges per node, indexed by dense node id.
    successors: Vec<Vec<NodeId>>,
    /// Admitted incoming edges per node, indexed by dense node id.
    predecessors: Vec<Vec<NodeId>>,
    /// The number of admitted edges.
    edges: AdmittedEdgeCount,
    /// The work counters.
    telemetry: MaintenanceTelemetry,
    /// The reusable search buffers.
    scratch: Scratch,
}

impl AcyclicityMaintenance
{
    /// An empty structure with no nodes and no edges.
    ///
    /// # Contract
    /// - ensures: a structure whose node count and admitted-edge count are both
    ///   zero.
    /// - fails: propagates [`OrderError::StructureIdExhausted`] as
    ///   [`MaintenanceError::Order`] when the process has no distinct
    ///   order-structure identity left.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`MaintenanceError::Order`] when the underlying order structure
    /// cannot be constructed.
    #[inline]
    pub fn new() -> Result<Self, MaintenanceError>
    {
        let order = OrderMaintenance::new()?;
        return Ok(Self {
            order,
            positions: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
            edges: AdmittedEdgeCount::default(),
            telemetry: MaintenanceTelemetry::default(),
            scratch: Scratch::default(),
        });
    }

    /// An edgeless structure already holding `count` nodes, in dense id order.
    ///
    /// # Contract
    /// - ensures: nodes `0 .. count` exist and stand in ascending dense-id
    ///   order, which is topologically valid because no edge exists yet.
    /// - fails: [`MaintenanceError::Order`] when the order structure cannot be
    ///   constructed or cannot admit that many elements;
    ///   [`MaintenanceError::ArithmeticOverflow`] when the count does not fit
    ///   the host address space.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`MaintenanceError::Order`] or
    /// [`MaintenanceError::ArithmeticOverflow`] as above.
    #[inline]
    pub fn with_nodes(count: NodeCount) -> Result<Self, MaintenanceError>
    {
        let mut structure = Self::new()?;
        let total =
            usize::try_from(count).map_err(|_ignored| MaintenanceError::ArithmeticOverflow)?;
        if let Some(last) = total.checked_sub(1) {
            let raw =
                u32::try_from(last).map_err(|_ignored| MaintenanceError::ArithmeticOverflow)?;
            structure.ensure_node(NodeId::from(raw))?;
        }
        return Ok(structure);
    }

    /// The number of nodes the structure holds.
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> NodeCount
    {
        return NodeCount::from(u32::try_from(self.positions.len()).unwrap_or(u32::MAX));
    }

    /// The number of admitted edges.
    #[inline]
    #[must_use]
    pub fn admitted_edges(&self) -> AdmittedEdgeCount
    {
        return self.edges;
    }

    /// The work performed since construction.
    #[inline]
    #[must_use]
    pub fn telemetry(&self) -> MaintenanceTelemetry
    {
        return self.telemetry;
    }

    /// The relative position of two nodes in the maintained order.
    ///
    /// # Contract
    /// - ensures: `Some(Less)` exactly when `left` precedes `right`, and
    ///   `Some(Equal)` exactly when the two identifiers are the same node.
    /// - fails: returns `None` when either identifier addresses no maintained
    ///   node.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn compare(
        &self,
        left: NodeId,
        right: NodeId,
    ) -> Option<Ordering>
    {
        let left_position = self.position(left).ok()?;
        let right_position = self.position(right).ok()?;
        return self.order.cmp(left_position, right_position);
    }

    /// The nodes in maintained order.
    #[inline]
    pub fn nodes_in_order(&self) -> impl Iterator<Item = NodeId> + '_
    {
        return self.order.iter().map(|(_position, &node)| node);
    }

    /// Whether every admitted edge runs forward in the maintained order.
    ///
    /// This is the structure's own invariant, stated as a query so a consumer —
    /// or a differential — can check it rather than trust it.
    ///
    /// # Contract
    /// - ensures: positive exactly when every admitted edge `source -> target`
    ///   has `source` strictly preceding `target` in the maintained order.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 boundary — a hand-relabelled order in which one
    ///   admitted edge runs backwards is distinguished from the same graph
    ///   before the relabelling, which is what makes this query the
    ///   differential's teeth rather than a restatement of the insertion path.
    /// - witness: `maintenance::tests::a_corrupted_order_is_caught_by_the_invariant`
    /// - witness: `maintenance::tests::the_maintained_order_is_topological`
    #[inline]
    #[must_use]
    pub fn order_is_topological(&self) -> TopologicalOrderStatus
    {
        for (index, row) in self.successors.iter().enumerate() {
            let Ok(raw) = u32::try_from(index)
            else {
                return TopologicalOrderStatus(false);
            };
            let source = NodeId::from(raw);
            for &target in row {
                if self.compare(source, target) != Some(Ordering::Less) {
                    return TopologicalOrderStatus(false);
                }
            }
        }
        return TopologicalOrderStatus(true);
    }

    /// **Offer one directed edge**, admitting it exactly when the resulting
    /// graph stays acyclic.
    ///
    /// The standing order decides the cheap case on its own: when it already
    /// places the source before the target the edge is recorded and nothing
    /// moves. Otherwise a bounded two-way search around the pair either returns
    /// the cycle the edge would close — in which case the edge is **not**
    /// recorded — or identifies the region whose relative order the edge
    /// changes, which is relocated before the edge is recorded.
    ///
    /// A repeated edge is admitted without being recorded twice, and a self
    /// loop is refused with the one-node closed walk as its witness.
    ///
    /// # Contract
    /// - requires: nothing; unseen nodes are created at the end of the order.
    /// - ensures: [`EdgeVerdict::Refused`] exactly when the offered edge closes
    ///   a cycle over the admitted edges, in which case the structure is
    ///   unchanged apart from any nodes the offer created; otherwise the edge
    ///   is admitted and the maintained order stays topological.
    /// - provides: a [`CycleWitness`] on refusal whose walk is closed and whose
    ///   edges are the offered edge together with admitted edges.
    /// - fails: [`MaintenanceError::Order`] when the order cannot admit or
    ///   relocate an element, [`MaintenanceError::OrderDesynchronized`] when a
    ///   tracked handle no longer resolves, and
    ///   [`MaintenanceError::ArithmeticOverflow`] on an internal conversion the
    ///   node capacity precludes.
    /// - panics: none.
    /// - intension: the work is bounded by the region between the two endpoints
    ///   in the standing order — the searches never leave it, and no node
    ///   outside the two searched sets is moved.
    ///
    /// # Errors
    /// Returns [`MaintenanceError`] as enumerated above.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence + property — over generated edge streams every
    ///   verdict agrees with a batch cycle check run over the admitted edges
    ///   plus the offered one, and the maintained order stays topological after
    ///   every admission. Boundary: the same edge is `Admitted` when the order
    ///   already witnesses it and `AdmittedAfterRepair` when it does not, and a
    ///   self loop is `Refused` where a parallel edge is not.
    /// - witness: `maintenance::tests::an_edge_the_order_witnesses_is_admitted_without_moving_anything`
    /// - witness: `maintenance::tests::a_violating_edge_is_repaired_locally`
    /// - witness: `maintenance::tests::a_cycle_closing_edge_is_refused_with_its_walk`
    /// - witness: `maintenance::tests::a_self_loop_is_refused`
    /// - witness: `dynamic_graphs::differential::incremental_verdicts_equal_the_batch_answer`
    #[inline]
    pub fn insert_edge(
        &mut self,
        edge: EdgeId,
    ) -> Result<EdgeVerdict, MaintenanceError>
    {
        self.telemetry.insertions = self.telemetry.insertions.saturating_increment();
        self.ensure_node(edge.source)?;
        self.ensure_node(edge.target)?;
        if edge.source == edge.target {
            self.telemetry.refusals = self.telemetry.refusals.saturating_increment();
            return Ok(EdgeVerdict::Refused(CycleWitness {
                nodes: alloc::vec![edge.source, edge.source],
                edges: alloc::vec![edge],
            }));
        }
        let source_position = self.position(edge.source)?;
        let target_position = self.position(edge.target)?;
        let standing = self
            .order
            .cmp(source_position, target_position)
            .ok_or(MaintenanceError::OrderDesynchronized)?;
        if standing == Ordering::Less {
            self.record_edge(edge)?;
            return Ok(EdgeVerdict::Admitted);
        }
        let discovery = self.discover(edge)?;
        match discovery {
            | Discovery::Cycle(witness) => {
                self.telemetry.refusals = self.telemetry.refusals.saturating_increment();
                Ok(EdgeVerdict::Refused(witness))
            },
            | Discovery::Region { backward, forward } => {
                self.relocate(&backward, &forward)?;
                self.record_edge(edge)?;
                self.telemetry.repairs = self.telemetry.repairs.saturating_increment();
                Ok(EdgeVerdict::AdmittedAfterRepair)
            },
        }
    }

    // ----- internal helpers ------------------------------------------------

    /// The dense vector index addressed by `node`.
    ///
    /// # Errors
    /// Returns [`MaintenanceError::ArithmeticOverflow`] when the identifier
    /// does not fit the host address space.
    #[inline]
    fn index_of(node: NodeId) -> Result<SlotIndex, MaintenanceError>
    {
        return SlotIndex::try_from(node).map_err(|_ignored| MaintenanceError::ArithmeticOverflow);
    }

    /// The order handle of `node`.
    ///
    /// # Errors
    /// Returns [`MaintenanceError::NodeCapacity`] when the identifier addresses
    /// no maintained node.
    #[inline]
    fn position(
        &self,
        node: NodeId,
    ) -> Result<Pos, MaintenanceError>
    {
        let index = Self::index_of(node)?;
        return self
            .positions
            .get(usize::from(index))
            .copied()
            .ok_or(MaintenanceError::NodeCapacity);
    }

    /// Creates every node up to and including `node`, appending each to the end
    /// of the order.
    ///
    /// Appending is topologically valid because a node created here has no
    /// edges yet.
    ///
    /// # Errors
    /// Returns [`MaintenanceError::Order`] when the order cannot admit an
    /// element, or [`MaintenanceError::ArithmeticOverflow`] on an internal
    /// conversion.
    #[inline]
    fn ensure_node(
        &mut self,
        node: NodeId,
    ) -> Result<(), MaintenanceError>
    {
        let index = Self::index_of(node)?;
        let required = usize::from(index)
            .checked_add(1)
            .ok_or(MaintenanceError::ArithmeticOverflow)?;
        while self.positions.len() < required {
            let raw = u32::try_from(self.positions.len())
                .map_err(|_ignored| MaintenanceError::ArithmeticOverflow)?;
            let fresh = NodeId::from(raw);
            let position = self.order.push_back(fresh)?;
            self.positions.push(position);
            self.successors.push(Vec::new());
            self.predecessors.push(Vec::new());
        }
        return Ok(());
    }

    /// Records `edge` in both adjacency directions, ignoring a repeat.
    ///
    /// # Errors
    /// Returns [`MaintenanceError::NodeCapacity`] when either endpoint
    /// addresses no maintained node, or
    /// [`MaintenanceError::ArithmeticOverflow`] on an internal conversion.
    #[inline]
    fn record_edge(
        &mut self,
        edge: EdgeId,
    ) -> Result<(), MaintenanceError>
    {
        let source_index = Self::index_of(edge.source)?;
        let target_index = Self::index_of(edge.target)?;
        let outgoing = self
            .successors
            .get_mut(usize::from(source_index))
            .ok_or(MaintenanceError::NodeCapacity)?;
        if outgoing.contains(&edge.target) {
            return Ok(());
        }
        outgoing.push(edge.target);
        let incoming = self
            .predecessors
            .get_mut(usize::from(target_index))
            .ok_or(MaintenanceError::NodeCapacity)?;
        incoming.push(edge.source);
        self.edges = self.edges.saturating_increment();
        return Ok(());
    }

    /// Runs the bounded two-way search around a violating insertion.
    ///
    /// The scratch buffers are taken out for the duration so the search can
    /// call the ordinary shared-reference helpers, and are restored on
    /// every path.
    ///
    /// # Errors
    /// Returns [`MaintenanceError`] as [`AcyclicityMaintenance::insert_edge`]
    /// enumerates.
    #[inline]
    fn discover(
        &mut self,
        edge: EdgeId,
    ) -> Result<Discovery, MaintenanceError>
    {
        let mut scratch = core::mem::take(&mut self.scratch);
        let outcome = self.search(&mut scratch, edge);
        self.scratch = scratch;
        return outcome;
    }

    /// The bounded forward and backward searches, over borrowed scratch state.
    ///
    /// # Errors
    /// Returns [`MaintenanceError`] as [`AcyclicityMaintenance::insert_edge`]
    /// enumerates.
    #[inline]
    fn search(
        &mut self,
        scratch: &mut Scratch,
        edge: EdgeId,
    ) -> Result<Discovery, MaintenanceError>
    {
        let width = self.positions.len();
        scratch.forward_mark.resize(width, Epoch::UNVISITED);
        scratch.backward_mark.resize(width, Epoch::UNVISITED);
        scratch.forward_parent.resize(width, NodeId::default());
        scratch.epoch = scratch.epoch.saturating_increment();
        let epoch = scratch.epoch;

        let forward = match self.search_forward(scratch, edge, epoch)? {
            | Ok(reached) => reached,
            | Err(witness) => return Ok(Discovery::Cycle(witness)),
        };
        let backward = self.search_backward(scratch, edge, epoch)?;
        return Ok(Discovery::Region { backward, forward });
    }

    /// The forward search: descendants of the edge's target that precede the
    /// edge's source in the standing order.
    ///
    /// Returns `Err(witness)` in the inner result when the search reaches the
    /// edge's source, which is exactly the cycle the insertion would close.
    ///
    /// # Errors
    /// Returns [`MaintenanceError`] as [`AcyclicityMaintenance::insert_edge`]
    /// enumerates.
    #[inline]
    fn search_forward(
        &mut self,
        scratch: &mut Scratch,
        edge: EdgeId,
        epoch: Epoch,
    ) -> Result<Result<Vec<NodeId>, CycleWitness>, MaintenanceError>
    {
        let mut reached: Vec<NodeId> = Vec::new();
        scratch.stack.clear();
        let target_index = Self::index_of(edge.target)?;
        let mark = scratch
            .forward_mark
            .get_mut(usize::from(target_index))
            .ok_or(MaintenanceError::NodeCapacity)?;
        *mark = epoch;
        scratch.stack.push(edge.target);
        while let Some(node) = scratch.stack.pop() {
            reached.push(node);
            self.telemetry.nodes_visited = self.telemetry.nodes_visited.saturating_increment();
            let node_index = Self::index_of(node)?;
            let row = self
                .successors
                .get(usize::from(node_index))
                .ok_or(MaintenanceError::NodeCapacity)?;
            // The row is cloned rather than borrowed because the loop stamps
            // the scratch marks, and the two borrows would otherwise overlap.
            let successors = row.clone();
            for successor in successors {
                if successor == edge.source {
                    let witness = Self::cycle_from(scratch, edge, node)?;
                    return Ok(Err(witness));
                }
                if self.compare(successor, edge.source) != Some(Ordering::Less) {
                    continue;
                }
                let successor_index = Self::index_of(successor)?;
                let successor_mark = scratch
                    .forward_mark
                    .get_mut(usize::from(successor_index))
                    .ok_or(MaintenanceError::NodeCapacity)?;
                if *successor_mark == epoch {
                    continue;
                }
                *successor_mark = epoch;
                let parent = scratch
                    .forward_parent
                    .get_mut(usize::from(successor_index))
                    .ok_or(MaintenanceError::NodeCapacity)?;
                *parent = node;
                scratch.stack.push(successor);
            }
        }
        return Ok(Ok(reached));
    }

    /// The backward search: ancestors of the edge's source that follow the
    /// edge's target in the standing order.
    ///
    /// # Errors
    /// Returns [`MaintenanceError`] as [`AcyclicityMaintenance::insert_edge`]
    /// enumerates.
    #[inline]
    fn search_backward(
        &mut self,
        scratch: &mut Scratch,
        edge: EdgeId,
        epoch: Epoch,
    ) -> Result<Vec<NodeId>, MaintenanceError>
    {
        let mut reached: Vec<NodeId> = Vec::new();
        scratch.stack.clear();
        let source_index = Self::index_of(edge.source)?;
        let mark = scratch
            .backward_mark
            .get_mut(usize::from(source_index))
            .ok_or(MaintenanceError::NodeCapacity)?;
        *mark = epoch;
        scratch.stack.push(edge.source);
        while let Some(node) = scratch.stack.pop() {
            reached.push(node);
            self.telemetry.nodes_visited = self.telemetry.nodes_visited.saturating_increment();
            let node_index = Self::index_of(node)?;
            let row = self
                .predecessors
                .get(usize::from(node_index))
                .ok_or(MaintenanceError::NodeCapacity)?;
            let predecessors = row.clone();
            for predecessor in predecessors {
                if self.compare(predecessor, edge.target) != Some(Ordering::Greater) {
                    continue;
                }
                let predecessor_index = Self::index_of(predecessor)?;
                let predecessor_mark = scratch
                    .backward_mark
                    .get_mut(usize::from(predecessor_index))
                    .ok_or(MaintenanceError::NodeCapacity)?;
                if *predecessor_mark == epoch {
                    continue;
                }
                *predecessor_mark = epoch;
                scratch.stack.push(predecessor);
            }
        }
        return Ok(reached);
    }

    /// Rebuilds the closed walk the offered edge would close, from the forward
    /// search's parent links.
    ///
    /// `last` is the node whose successor is the edge's source, so the walk
    /// runs from the edge's target down the parent chain to `last`, on to
    /// the edge's source, and back to the target along the offered edge.
    ///
    /// # Errors
    /// Returns [`MaintenanceError::OrderDesynchronized`] when the parent chain
    /// does not reach the edge's target within the node count, which the
    /// search's own construction precludes.
    #[inline]
    fn cycle_from(
        scratch: &Scratch,
        edge: EdgeId,
        last: NodeId,
    ) -> Result<CycleWitness, MaintenanceError>
    {
        let mut walk: Vec<NodeId> = alloc::vec![last];
        let mut cursor = last;
        let mut remaining = scratch.forward_parent.len();
        while cursor != edge.target {
            let index = Self::index_of(cursor)?;
            let parent = scratch
                .forward_parent
                .get(usize::from(index))
                .copied()
                .ok_or(MaintenanceError::NodeCapacity)?;
            walk.push(parent);
            cursor = parent;
            remaining = remaining
                .checked_sub(1)
                .ok_or(MaintenanceError::OrderDesynchronized)?;
        }
        walk.reverse();
        walk.push(edge.source);
        walk.push(edge.target);
        let mut edges: Vec<EdgeId> = Vec::new();
        let mut previous: Option<NodeId> = None;
        for &node in &walk {
            if let Some(source) = previous {
                edges.push(EdgeId::new(source, node));
            }
            previous = Some(node);
        }
        return Ok(CycleWitness { nodes: walk, edges });
    }

    /// The nodes of `region`, ordered as the standing order already orders
    /// them.
    ///
    /// The standing order is a topological order of the admitted edges, so
    /// sorting a set by it *is* a topological sort of that set — which is what
    /// the relocation needs, and it needs no traversal to obtain.
    ///
    /// # Errors
    /// Returns [`MaintenanceError::NodeCapacity`] when a node addresses no
    /// maintained slot, or [`MaintenanceError::OrderDesynchronized`] when a
    /// tracked handle no longer resolves — checked up front so the sort's
    /// comparator is a total order.
    #[inline]
    fn sorted_by_order(
        &self,
        region: &[NodeId],
    ) -> Result<Vec<(NodeId, Pos)>, MaintenanceError>
    {
        let mut sorted: Vec<(NodeId, Pos)> = Vec::with_capacity(region.len());
        for &node in region {
            let position = self.position(node)?;
            if self.order.get(position).is_none() {
                return Err(MaintenanceError::OrderDesynchronized);
            }
            sorted.push((node, position));
        }
        sorted.sort_by(|&(_, left), &(_, right)| {
            self.order.cmp(left, right).unwrap_or(Ordering::Equal)
        });
        return Ok(sorted);
    }

    /// Relocates the affected region into the order positions it already
    /// occupies, backward set first and forward set after it.
    ///
    /// Each set keeps its own internal order, which the standing order already
    /// makes topological; placing the whole backward set before the whole
    /// forward set is what the new edge demands, and no edge runs from the
    /// forward set to the backward set — such an edge would have made the
    /// insertion cycle-closing, which the search reports instead.
    ///
    /// # Errors
    /// Returns [`MaintenanceError`] as [`AcyclicityMaintenance::insert_edge`]
    /// enumerates.
    #[inline]
    fn relocate(
        &mut self,
        backward: &[NodeId],
        forward: &[NodeId],
    ) -> Result<(), MaintenanceError>
    {
        // The backward and forward sets are disjoint whenever no cycle was
        // found: a node in both would put the edge's target on a path to its
        // source, which the forward search reports as a cycle instead.
        debug_assert!(
            backward.iter().all(|node| !forward.contains(node)),
            "an acyclic insertion leaves the two searched sets disjoint"
        );
        let mut arrangement: Vec<NodeId> = self
            .sorted_by_order(backward)?
            .into_iter()
            .map(|(node, _position)| node)
            .collect();
        arrangement.extend(
            self.sorted_by_order(forward)?
                .into_iter()
                .map(|(node, _position)| node),
        );
        if arrangement.is_empty() {
            return Ok(());
        }
        let slots = self.sorted_by_order(&arrangement)?;

        // Each slot's anchor is the nearest element before it that is not
        // itself a slot; consecutive slots share one anchor, which is what
        // keeps unaffected neighbours between the slots exactly where they are.
        let mut anchors: Vec<Option<Pos>> = Vec::with_capacity(slots.len());
        let mut previous: Option<(Pos, Option<Pos>)> = None;
        for &(_, slot) in &slots {
            let candidate = self.order.prev(slot);
            let anchor = match (candidate, previous) {
                | (Some(before), Some((previous_slot, previous_anchor)))
                    if before == previous_slot =>
                {
                    previous_anchor
                },
                | _ => candidate,
            };
            anchors.push(anchor);
            previous = Some((slot, anchor));
        }

        for &(_, slot) in &slots {
            self.order
                .remove(slot)
                .ok_or(MaintenanceError::OrderDesynchronized)?;
        }

        let mut placed: Option<(Option<Pos>, Pos)> = None;
        for (index, &node) in arrangement.iter().enumerate() {
            let anchor = anchors.get(index).copied().flatten();
            let attach = match placed {
                | Some((previous_anchor, previous_position)) if previous_anchor == anchor => {
                    Some(previous_position)
                },
                | _ => anchor,
            };
            let fresh = match attach {
                | Some(after) => self.order.insert_after(after, node)?,
                | None => self.order.push_front(node)?,
            };
            let node_index = Self::index_of(node)?;
            let entry = self
                .positions
                .get_mut(usize::from(node_index))
                .ok_or(MaintenanceError::NodeCapacity)?;
            *entry = fresh;
            placed = Some((anchor, fresh));
            self.telemetry.nodes_relocated = self.telemetry.nodes_relocated.saturating_increment();
        }
        return Ok(());
    }
}

impl EdgeSource for AcyclicityMaintenance
{
    type Successors<'successors>
        = core::iter::Copied<core::slice::Iter<'successors, NodeId>>
    where
        Self: 'successors;

    #[inline]
    fn node_count(&self) -> NodeCount
    {
        return self.nodes();
    }

    #[inline]
    fn successors(
        &self,
        node: NodeId,
    ) -> Self::Successors<'_>
    {
        let empty: &[NodeId] = &[];
        return Self::index_of(node)
            .ok()
            .and_then(|index| self.successors.get(usize::from(index)))
            .map_or(empty, Vec::as_slice)
            .iter()
            .copied();
    }
}

#[cfg(test)]
mod tests
{
    use alloc::vec;
    use alloc::vec::Vec;

    use gandr_theory_graphs::EdgeId;
    use gandr_theory_graphs::NodeId;
    use gandr_theory_graphs::cycle_witness;

    use super::AcyclicityMaintenance;
    use super::EdgeVerdict;

    /// A node identifier from a small dense index.
    fn node<Index>(index: Index) -> NodeId
    where
        Index: Into<NodeId>,
    {
        index.into()
    }

    /// A directed edge from two small dense indices.
    fn edge<Source, Target>(
        source: Source,
        target: Target,
    ) -> EdgeId
    where
        Source: Into<NodeId>,
        Target: Into<NodeId>,
    {
        EdgeId::new(source.into(), target.into())
    }

    /// A structure holding exactly the offered edges that were admitted.
    fn admit(offers: &[EdgeId]) -> AcyclicityMaintenance
    {
        let mut structure = AcyclicityMaintenance::new().expect("a fresh structure is available");
        for &offer in offers {
            structure
                .insert_edge(offer)
                .expect("insertion is total over well-formed identifiers");
        }
        structure
    }

    #[test]
    fn an_edge_the_order_witnesses_is_admitted_without_moving_anything()
    {
        let mut structure = AcyclicityMaintenance::new().expect("a fresh structure is available");
        // Creating the nodes in ascending order already orders 0 before 1.
        structure
            .ensure_node(node(1u32))
            .expect("nodes are creatable");
        let verdict = structure
            .insert_edge(edge(0u32, 1u32))
            .expect("insertion succeeds");
        assert_eq!(EdgeVerdict::Admitted, verdict, "the order already agreed");
        assert_eq!(
            0,
            u64::from(structure.telemetry().nodes_relocated),
            "an admitted edge the order witnesses moves nothing"
        );
        assert_eq!(
            0,
            u64::from(structure.telemetry().nodes_visited),
            "an admitted edge the order witnesses searches nothing"
        );
    }

    #[test]
    fn a_violating_edge_is_repaired_locally()
    {
        let mut structure = AcyclicityMaintenance::new().expect("a fresh structure is available");
        structure
            .ensure_node(node(3u32))
            .expect("nodes are creatable");
        // The order is 0, 1, 2, 3; the edge 2 -> 1 runs against it.
        let verdict = structure
            .insert_edge(edge(2u32, 1u32))
            .expect("insertion succeeds");
        assert_eq!(
            EdgeVerdict::AdmittedAfterRepair,
            verdict,
            "the order had to change"
        );
        assert_eq!(
            vec![node(0u32), node(2u32), node(1u32), node(3u32)],
            structure.nodes_in_order().collect::<Vec<_>>(),
            "only the two affected nodes swapped, and the untouched ones kept their slots"
        );
        assert!(
            bool::from(structure.order_is_topological()),
            "the repaired order is topological"
        );
    }

    #[test]
    fn a_cycle_closing_edge_is_refused_with_its_walk()
    {
        let mut structure = admit(&[edge(0u32, 1u32), edge(1u32, 2u32)]);
        let verdict = structure
            .insert_edge(edge(2u32, 0u32))
            .expect("insertion succeeds");
        let EdgeVerdict::Refused(witness) = verdict
        else {
            panic!("closing 0 -> 1 -> 2 -> 0 is a cycle");
        };
        assert_eq!(
            witness.nodes.first(),
            witness.nodes.last(),
            "the witness walk is closed"
        );
        assert_eq!(
            vec![node(0u32), node(1u32), node(2u32), node(0u32)],
            witness.nodes,
            "the walk runs the cycle the offer would close"
        );
        assert_eq!(
            2,
            u64::from(structure.admitted_edges()),
            "a refused edge is not recorded"
        );
        assert!(
            bool::from(structure.order_is_topological()),
            "a refusal leaves the order intact"
        );
    }

    #[test]
    fn a_self_loop_is_refused()
    {
        let mut structure = AcyclicityMaintenance::new().expect("a fresh structure is available");
        let verdict = structure
            .insert_edge(edge(0u32, 0u32))
            .expect("insertion succeeds");
        let EdgeVerdict::Refused(witness) = verdict
        else {
            panic!("a self loop is a cycle");
        };
        assert_eq!(
            vec![node(0u32), node(0u32)],
            witness.nodes,
            "the walk is the loop"
        );
        assert_eq!(
            0,
            u64::from(structure.admitted_edges()),
            "a self loop is not recorded"
        );
    }

    #[test]
    fn a_repeated_edge_is_admitted_once()
    {
        let mut structure = admit(&[edge(0u32, 1u32), edge(0u32, 1u32), edge(0u32, 1u32)]);
        assert_eq!(
            1,
            u64::from(structure.admitted_edges()),
            "a repeat does not grow the edge set"
        );
        assert_eq!(
            3,
            u64::from(structure.telemetry().insertions),
            "every offer is counted"
        );
        let verdict = structure
            .insert_edge(edge(0u32, 1u32))
            .expect("insertion succeeds");
        assert_eq!(EdgeVerdict::Admitted, verdict, "a repeat is still admitted");
    }

    #[test]
    fn the_maintained_order_is_topological()
    {
        let structure = admit(&[
            edge(4u32, 3u32),
            edge(3u32, 2u32),
            edge(2u32, 1u32),
            edge(1u32, 0u32),
            edge(4u32, 0u32),
        ]);
        assert!(
            bool::from(structure.order_is_topological()),
            "a reversing chain is fully repaired"
        );
        assert_eq!(
            vec![node(4u32), node(3u32), node(2u32), node(1u32), node(0u32)],
            structure.nodes_in_order().collect::<Vec<_>>(),
            "the chain ends up exactly reversed"
        );
        assert_eq!(
            None,
            cycle_witness(&structure).expect("the dense graph is well formed"),
            "the admitted graph is acyclic"
        );
    }

    #[test]
    fn a_corrupted_order_is_caught_by_the_invariant()
    {
        // The seeded corruption the differential must have teeth against:
        // the admitted edges are left alone and the maintained order is
        // rewritten behind the structure's back, so nothing but the invariant
        // query can notice.
        let mut structure = admit(&[edge(0u32, 1u32), edge(1u32, 2u32)]);
        assert!(
            bool::from(structure.order_is_topological()),
            "the structure starts sound"
        );

        let head = structure.order.first().expect("the order is non-empty");
        let displaced = structure.order.remove(head).expect("the head is removable");
        let tail = structure
            .order
            .last()
            .expect("the order is still non-empty");
        let moved = structure
            .order
            .insert_after(tail, displaced)
            .expect("reinsertion succeeds");
        let index = AcyclicityMaintenance::index_of(displaced).expect("the node is addressable");
        *structure
            .positions
            .get_mut(usize::from(index))
            .expect("the node has a slot") = moved;

        assert!(
            !bool::from(structure.order_is_topological()),
            "moving a source behind its target is caught"
        );
    }

    #[test]
    fn a_refusal_leaves_the_structure_usable()
    {
        let mut structure = admit(&[edge(0u32, 1u32), edge(1u32, 2u32)]);
        let refused = structure
            .insert_edge(edge(2u32, 0u32))
            .expect("insertion succeeds");
        assert!(
            matches!(refused, EdgeVerdict::Refused(_)),
            "the offer closes a cycle"
        );
        let admitted = structure
            .insert_edge(edge(0u32, 2u32))
            .expect("insertion succeeds");
        assert_eq!(
            EdgeVerdict::Admitted,
            admitted,
            "the structure keeps working after a refusal"
        );
        assert!(
            bool::from(structure.order_is_topological()),
            "and stays sound"
        );
    }

    #[test]
    fn unaffected_nodes_between_the_endpoints_keep_their_slots()
    {
        // Nodes 1 and 2 sit between the endpoints in the order but neither
        // reaches 4 nor is reached by 3, so the repair must step over them.
        let mut structure = AcyclicityMaintenance::new().expect("a fresh structure is available");
        structure
            .ensure_node(node(4u32))
            .expect("nodes are creatable");
        let verdict = structure
            .insert_edge(edge(3u32, 0u32))
            .expect("insertion succeeds");
        assert_eq!(
            EdgeVerdict::AdmittedAfterRepair,
            verdict,
            "the order had to change"
        );
        assert_eq!(
            vec![node(3u32), node(1u32), node(2u32), node(0u32), node(4u32)],
            structure.nodes_in_order().collect::<Vec<_>>(),
            "the two affected nodes exchanged their own slots and nothing else moved"
        );
        assert!(
            bool::from(structure.order_is_topological()),
            "the repaired order is topological"
        );
    }
}
