//! Deterministic partition refinement and simulation over dense graphs.
//!
//! # Contract
//! - requires: callers provide graphs through [`crate::EdgeSource`] with dense
//!   node identities in `0..node_count()`.
//! - ensures: public results contain only gandr-owned rows, blocks, and dense
//!   node identifiers; adjacency is validated and canonicalized through the
//!   crate's shared helper before either algorithm observes it.
//! - provides: coarsest strong bisimulation partitions and greatest forward
//!   simulation preorders for unlabeled finite transition systems.
//! - fails: invalid dense boundaries surface as [`GraphValidationError`].
//! - panics: none.
//! - intension: refinement uses bitset splitter predecessor sets over canonical
//!   adjacency; simulation uses a monotone bitset relation-elimination
//!   fixpoint.
//!
//! # Adequacy
//! - hypothesis: L3 pointwise + L2 generative — explicit empty, singleton,
//!   deadlock, chain, branching, cyclic, disconnected, duplicate-successor,
//!   invalid-boundary, and deterministic-order witnesses distinguish public
//!   observations; small finite systems compare every returned pair against
//!   independent naive strong-bisimulation and simulation fixpoint oracles.
//! - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`

use alloc::vec;
use alloc::vec::Vec;
use core::convert::TryFrom as _;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;

use fixedbitset::FixedBitSet;

use super::GraphValidationError;
use crate::EdgeSource;
use crate::NodeCapacity;
use crate::NodeCount;
use crate::NodeId;
use crate::NodePosition;
use crate::algorithms::adjacency_rows;

/// Canonical partition-block identity.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BlockIndex(u32);

impl From<u32> for BlockIndex
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<BlockIndex> for u32
{
    #[inline]
    fn from(value: BlockIndex) -> Self
    {
        value.0
    }
}

impl Display for BlockIndex
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

/// Whether two states inhabit the same bisimulation block.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EquivalenceDecision(bool);

impl From<bool> for EquivalenceDecision
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<EquivalenceDecision> for bool
{
    #[inline]
    fn from(value: EquivalenceDecision) -> Self
    {
        value.0
    }
}

impl Display for EquivalenceDecision
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

/// Whether one state simulates another in the greatest forward preorder.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SimulationDecision(bool);

impl From<bool> for SimulationDecision
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<SimulationDecision> for bool
{
    #[inline]
    fn from(value: SimulationDecision) -> Self
    {
        value.0
    }
}

impl Display for SimulationDecision
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

/// Whether a bitset contains at least one member.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct BitsetHasMember(bool);

impl From<bool> for BitsetHasMember
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<BitsetHasMember> for bool
{
    #[inline]
    fn from(value: BitsetHasMember) -> Self
    {
        value.0
    }
}

/// Whether a candidate target matches one subject target.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct MatchingCandidateTarget(bool);

impl From<bool> for MatchingCandidateTarget
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<MatchingCandidateTarget> for bool
{
    #[inline]
    fn from(value: MatchingCandidateTarget) -> Self
    {
        value.0
    }
}

/// Whether one candidate currently simulates one subject.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SimulationMatch(bool);

impl From<bool> for SimulationMatch
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<SimulationMatch> for bool
{
    #[inline]
    fn from(value: SimulationMatch) -> Self
    {
        value.0
    }
}

/// Canonical coarsest strong-bisimulation partition.
///
/// # Contract
/// - requires: construction goes through [`bisimulation_partition`].
/// - ensures: [`blocks`](Self::blocks) returns blocks with ascending members;
///   blocks are ordered by least member and cover each dense node exactly once.
/// - provides: checked read-only block and equivalence queries over dense node
///   identities.
/// - fails: query methods return [`GraphValidationError::NodeOutOfBounds`] for
///   states outside `0..node_count`.
/// - panics: none.
/// - intension: `state_to_block` is an owned dense map aligned with the
///   canonical `blocks` rows.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — named deadlock, branch, cycle, and disconnected
///   witnesses observe exact block membership and ordering through this type.
/// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Partition
{
    /// Dense node bound used by checked query methods.
    node_count: NodeCount,
    /// Canonical blocks ordered by their least member.
    blocks: Vec<Vec<NodeId>>,
    /// Dense state-to-block map; entries are canonical block indices.
    state_to_block: Vec<BlockIndex>,
}

impl Partition
{
    /// Returns canonical partition blocks.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: returns blocks with ascending members, ordered by each
    ///   block's least member.
    /// - provides: read-only observation of the coarsest strong-bisimulation
    ///   partition.
    /// - panics: none.
    /// - intension: exposes the owned canonical rows without recomputation.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — deterministic-order witnesses compare this
    ///   exact row order.
    /// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
    #[inline]
    #[must_use]
    pub fn blocks(&self) -> &[Vec<NodeId>]
    {
        &self.blocks
    }

    /// Returns the canonical block index containing `state`.
    ///
    /// # Contract
    /// - requires: `state` is a caller-selected dense node id.
    /// - ensures: valid states return the block index for the row in
    ///   [`blocks`](Self::blocks).
    /// - fails: returns [`GraphValidationError::NodeOutOfBounds`] for invalid
    ///   states.
    /// - panics: none.
    /// - intension: validates against the stored dense bound, then reads the
    ///   dense state-to-block map.
    ///
    /// # Errors
    /// Returns [`GraphValidationError::NodeOutOfBounds`] when `state` is
    /// outside `0..node_count`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — singleton and invalid-query witnesses
    ///   distinguish the exact block index and error variant.
    /// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
    #[inline]
    pub fn block_of(
        &self,
        state: NodeId,
    ) -> Result<BlockIndex, GraphValidationError>
    {
        let position = query_index(state, self.node_count)?;
        self.state_to_block
            .get(usize::from(position))
            .copied()
            .ok_or(GraphValidationError::NodeOutOfBounds {
                node: state,
                node_count: self.node_count,
            })
    }

    /// Returns whether `left` and `right` inhabit the same block.
    ///
    /// # Contract
    /// - requires: `left` and `right` are caller-selected dense node ids.
    /// - ensures: valid states return `true` exactly when both states are
    ///   strongly bisimilar.
    /// - fails: returns [`GraphValidationError::NodeOutOfBounds`] for either
    ///   invalid state.
    /// - panics: none.
    /// - intension: compares checked canonical block identities.
    ///
    /// # Errors
    /// Returns [`GraphValidationError::NodeOutOfBounds`] when either endpoint
    /// is outside `0..node_count`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — chain, branch, cycle, and deadlock
    ///   witnesses distinguish equivalent and non-equivalent pairs.
    /// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
    #[inline]
    pub fn equivalent(
        &self,
        left: NodeId,
        right: NodeId,
    ) -> Result<EquivalenceDecision, GraphValidationError>
    {
        let left_block = self.block_of(left)?;
        let right_block = self.block_of(right)?;
        Ok(EquivalenceDecision::from(left_block == right_block))
    }
}

/// Canonical row of the greatest forward simulation preorder.
///
/// # Contract
/// - requires: construction goes through [`simulation_relation`].
/// - ensures: `candidates` is ascending and contains exactly the states that
///   simulate `subject`.
/// - provides: gandr-owned row data for the preorder orientation documented by
///   [`Simulation::is_simulated_by`].
/// - panics: none.
/// - intension: each row is materialized from one final bitset row.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — deterministic-row witnesses compare row and
///   candidate order exactly.
/// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationRow
{
    /// Subject state whose simulators are listed in `candidates`.
    pub subject: NodeId,
    /// Candidate states that simulate `subject`, ascending.
    pub candidates: Vec<NodeId>,
}

/// Greatest forward simulation preorder.
///
/// `is_simulated_by(subject, candidate)` is true iff every outgoing transition
/// from `subject` can be matched by an outgoing transition from `candidate` to
/// a target that remains related to the subject transition target.
///
/// # Contract
/// - requires: construction goes through [`simulation_relation`].
/// - ensures: rows are ordered by ascending subject; each candidate list is
///   ascending and duplicate input successors are semantically ignored.
/// - provides: checked read-only row and pair queries for the greatest forward
///   simulation preorder.
/// - fails: query methods return [`GraphValidationError::NodeOutOfBounds`] for
///   states outside `0..node_count`.
/// - panics: none.
/// - intension: `relation` stores the final bitset fixpoint in the same
///   orientation as [`is_simulated_by`](Self::is_simulated_by): row = subject,
///   bit = candidate.
///
/// # Adequacy
/// - hypothesis: L3 pointwise + L2 generative — explicit chain, branch,
///   deadlock, cycle, and deterministic-order witnesses observe the public
///   orientation; proptest cases compare every pair against an independent
///   naive relation-elimination oracle.
/// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Simulation
{
    /// Dense node bound used by checked query methods.
    node_count: NodeCount,
    /// Canonical public rows.
    rows: Vec<SimulationRow>,
    /// Dense relation bitsets in subject-to-candidate orientation.
    relation: Vec<FixedBitSet>,
}

impl Simulation
{
    /// Returns canonical simulation rows.
    ///
    /// # Contract
    /// - requires: none.
    /// - ensures: rows are sorted by subject and candidates by state id.
    /// - provides: read-only observation of the greatest forward simulation
    ///   preorder.
    /// - panics: none.
    /// - intension: exposes materialized rows without recomputation.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — deterministic-row witnesses compare this
    ///   exact row order.
    /// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
    #[inline]
    #[must_use]
    pub fn rows(&self) -> &[SimulationRow]
    {
        &self.rows
    }

    /// Returns whether `candidate` simulates `subject`.
    ///
    /// Orientation: `is_simulated_by(subject, candidate)` is true iff for every
    /// edge `subject -> subject_target` there exists an edge
    /// `candidate -> candidate_target` such that
    /// `is_simulated_by(subject_target, candidate_target)` remains true.
    ///
    /// # Contract
    /// - requires: `subject` and `candidate` are caller-selected dense node
    ///   ids.
    /// - ensures: valid states return the greatest forward simulation relation
    ///   entry in subject-to-candidate orientation.
    /// - fails: returns [`GraphValidationError::NodeOutOfBounds`] for either
    ///   invalid state.
    /// - panics: none.
    /// - intension: validates both endpoints before checking the final bitset
    ///   relation.
    ///
    /// # Errors
    /// Returns [`GraphValidationError::NodeOutOfBounds`] when either endpoint
    /// is outside `0..node_count`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — chain and branch witnesses distinguish the
    ///   forward-simulation orientation from its converse.
    /// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
    #[inline]
    pub fn is_simulated_by(
        &self,
        subject: NodeId,
        candidate: NodeId,
    ) -> Result<SimulationDecision, GraphValidationError>
    {
        let subject_position = query_index(subject, self.node_count)?;
        let candidate_position = query_index(candidate, self.node_count)?;
        self.relation
            .get(usize::from(subject_position))
            .map(|row| SimulationDecision::from(row.contains(usize::from(candidate_position))))
            .ok_or(GraphValidationError::NodeOutOfBounds {
                node: subject,
                node_count: self.node_count,
            })
    }

    /// Returns the candidate list for `subject`.
    ///
    /// # Contract
    /// - requires: `subject` is a caller-selected dense node id.
    /// - ensures: valid states return the ascending candidates that simulate
    ///   `subject`.
    /// - fails: returns [`GraphValidationError::NodeOutOfBounds`] for invalid
    ///   states.
    /// - panics: none.
    /// - intension: reads the canonical public row whose subject equals the
    ///   dense row index.
    ///
    /// # Errors
    /// Returns [`GraphValidationError::NodeOutOfBounds`] when `subject` is
    /// outside `0..node_count`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — deterministic-row witnesses compare the
    ///   exact returned candidate order.
    /// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
    #[inline]
    pub fn candidates_for(
        &self,
        subject: NodeId,
    ) -> Result<&[NodeId], GraphValidationError>
    {
        let subject_position = query_index(subject, self.node_count)?;
        self.rows
            .get(usize::from(subject_position))
            .map(|row| row.candidates.as_slice())
            .ok_or(GraphValidationError::NodeOutOfBounds {
                node: subject,
                node_count: self.node_count,
            })
    }
}

/// Computes the coarsest strong-bisimulation partition.
///
/// # Contract
/// - requires: `graph` exposes dense nodes in `0..graph.node_count()`.
/// - ensures: returns the coarsest partition such that states in the same block
///   have matching transitions into every block in both directions.
/// - provides: canonical block rows as [`Partition`].
/// - fails: returns [`GraphValidationError`] for invalid dense boundaries.
/// - panics: none.
/// - intension: validates and deduplicates adjacency first, then runs splitter
///   predecessor-set refinement with [`FixedBitSet`] blocks until stable.
///
/// # Errors
/// Returns [`GraphValidationError`] when a successor target is out of bounds or
/// the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise + L2 generative — explicit empty, singleton,
///   deadlock, chain, branch, cycle, duplicate-successor, disconnected, and
///   invalid-boundary witnesses distinguish results and errors; small finite
///   systems compare every pair against an independent naive
///   strong-bisimulation oracle.
/// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
#[inline]
pub fn bisimulation_partition<G>(graph: &G) -> Result<Partition, GraphValidationError>
where
    G: EdgeSource,
{
    let adjacency = adjacency_rows(graph)?;
    let node_count = graph.node_count();
    let node_len = checked_node_len(node_count)?;
    let predecessors = predecessor_rows(&adjacency, node_count)?;
    let refined_blocks = refine_partition(&predecessors, node_count, node_len)?;
    materialize_partition(&refined_blocks, node_count, node_len)
}

/// Computes the greatest forward simulation preorder.
///
/// # Contract
/// - requires: `graph` exposes dense nodes in `0..graph.node_count()`.
/// - ensures: `is_simulated_by(subject, candidate)` is true exactly when every
///   `subject` transition can be matched by a `candidate` transition whose
///   targets remain related by the same preorder.
/// - provides: canonical subject rows as [`Simulation`].
/// - fails: returns [`GraphValidationError`] for invalid dense boundaries.
/// - panics: none.
/// - intension: validates and deduplicates adjacency first, initializes the
///   full bitset relation, and monotonically removes failing pairs to a
///   greatest fixpoint.
///
/// # Errors
/// Returns [`GraphValidationError`] when a successor target is out of bounds or
/// the node count cannot fit the host address space.
///
/// # Adequacy
/// - hypothesis: L3 pointwise + L2 generative — explicit deadlock, chain,
///   branch, cycle, duplicate-successor, deterministic-order, and
///   invalid-boundary witnesses distinguish orientation and canonical rows;
///   small finite systems compare every pair against an independent naive
///   simulation oracle.
/// - witness: `gandr_theory_graphs::partition_refine::contracts::partition_refinement_contract`
#[inline]
pub fn simulation_relation<G>(graph: &G) -> Result<Simulation, GraphValidationError>
where
    G: EdgeSource,
{
    let adjacency = adjacency_rows(graph)?;
    let node_count = graph.node_count();
    let node_len = checked_node_len(node_count)?;
    let relation = greatest_simulation(&adjacency, node_count, node_len)?;
    materialize_simulation(relation, node_count)
}
/// Converts a dense node count to a vector length.
fn checked_node_len(node_count: NodeCount) -> Result<NodeCapacity, GraphValidationError>
{
    NodeCapacity::try_from(node_count)
        .map_err(|_conversion_error| GraphValidationError::NodeCountTooLarge { node_count })
}

/// Builds reverse adjacency rows from canonical forward adjacency.
fn predecessor_rows(
    adjacency: &[Vec<NodeId>],
    node_count: NodeCount,
) -> Result<Vec<Vec<NodeId>>, GraphValidationError>
{
    let node_len = checked_node_len(node_count)?;
    let mut predecessors = vec![Vec::new(); usize::from(node_len)];
    for source in node_count.ids() {
        let source_position = checked_node_index(source, node_count)?;
        let Some(row) = adjacency.get(usize::from(source_position))
        else {
            return Err(GraphValidationError::NodeOutOfBounds {
                node: source,
                node_count,
            });
        };
        for &target in row {
            let target_position = checked_node_index(target, node_count)?;
            let Some(target_predecessors) = predecessors.get_mut(usize::from(target_position))
            else {
                return Err(GraphValidationError::NodeOutOfBounds {
                    node: target,
                    node_count,
                });
            };
            target_predecessors.push(source);
        }
    }
    Ok(predecessors)
}

/// Runs splitter predecessor-set partition refinement to a stable partition.
fn refine_partition(
    predecessors: &[Vec<NodeId>],
    node_count: NodeCount,
    node_len: NodeCapacity,
) -> Result<Vec<FixedBitSet>, GraphValidationError>
{
    if u32::from(node_count) == 0 {
        return Ok(Vec::new());
    }

    let initial = full_bitset(node_count, node_len)?;
    let mut blocks = vec![initial.clone()];
    let mut splitters = vec![initial];

    while let Some(splitter) = splitters.pop() {
        let predecessor_set =
            predecessor_set_for_splitter(predecessors, &splitter, node_count, node_len)?;
        if !bool::from(has_any(&predecessor_set)) {
            continue;
        }
        let mut next_blocks = Vec::with_capacity(blocks.len().saturating_add(1));
        for block in &blocks {
            let inside = intersection(block, &predecessor_set, node_len);
            let outside = difference(block, &predecessor_set, node_len);
            if bool::from(has_any(&inside)) && bool::from(has_any(&outside)) {
                push_split_blocks(&mut next_blocks, &mut splitters, inside, outside);
            }
            else {
                next_blocks.push(block.clone());
            }
        }
        blocks = next_blocks;
    }

    Ok(blocks)
}
/// Computes the greatest forward simulation by monotone pair elimination.
fn greatest_simulation(
    adjacency: &[Vec<NodeId>],
    node_count: NodeCount,
    node_len: NodeCapacity,
) -> Result<Vec<FixedBitSet>, GraphValidationError>
{
    let full = full_bitset(node_count, node_len)?;
    let mut relation = Vec::with_capacity(usize::from(node_len));
    for _subject in node_count.ids() {
        relation.push(full.clone());
    }

    let mut changed = true;
    while changed {
        changed = false;
        for subject in node_count.ids() {
            let subject_position = checked_node_index(subject, node_count)?;
            for candidate in node_count.ids() {
                let candidate_position = checked_node_index(candidate, node_count)?;
                let Some(row) = relation.get(usize::from(subject_position))
                else {
                    return Err(GraphValidationError::NodeOutOfBounds {
                        node: subject,
                        node_count,
                    });
                };
                if row.contains(usize::from(candidate_position)) {
                    let candidate_matches = candidate_matches_subject(
                        adjacency, &relation, subject, candidate, node_count,
                    )?;
                    if !bool::from(candidate_matches) {
                        let Some(row_mut) = relation.get_mut(usize::from(subject_position))
                        else {
                            return Err(GraphValidationError::NodeOutOfBounds {
                                node: subject,
                                node_count,
                            });
                        };
                        row_mut.set(usize::from(candidate_position), false);
                        changed = true;
                    }
                }
            }
        }
    }

    Ok(relation)
}

/// Creates a bitset containing every dense node.
fn full_bitset(
    node_count: NodeCount,
    node_len: NodeCapacity,
) -> Result<FixedBitSet, GraphValidationError>
{
    let mut bitset = FixedBitSet::with_capacity(usize::from(node_len));
    for state in node_count.ids() {
        let state_position = checked_node_index(state, node_count)?;
        bitset.insert(usize::from(state_position));
    }
    Ok(bitset)
}
/// Returns all predecessors of any state in `splitter`.
fn predecessor_set_for_splitter(
    predecessors: &[Vec<NodeId>],
    splitter: &FixedBitSet,
    node_count: NodeCount,
    node_len: NodeCapacity,
) -> Result<FixedBitSet, GraphValidationError>
{
    let mut result = FixedBitSet::with_capacity(usize::from(node_len));
    for target_position in splitter.ones() {
        let target = checked_node_from_index(NodePosition::from(target_position))?;
        let Some(row) = predecessors.get(target_position)
        else {
            return Err(GraphValidationError::NodeOutOfBounds {
                node: target,
                node_count,
            });
        };
        for &source in row {
            let source_position = checked_node_index(source, node_count)?;
            result.insert(usize::from(source_position));
        }
    }
    Ok(result)
}
/// Returns whether a bitset has at least one member.
fn has_any(bitset: &FixedBitSet) -> BitsetHasMember
{
    BitsetHasMember::from(bitset.ones().next().is_some())
}
/// Returns the intersection of two bitsets.
fn intersection(
    left: &FixedBitSet,
    right: &FixedBitSet,
    node_len: NodeCapacity,
) -> FixedBitSet
{
    let mut result = FixedBitSet::with_capacity(usize::from(node_len));
    for member in left.ones() {
        if right.contains(member) {
            result.insert(member);
        }
    }
    result
}
/// Returns members of `left` absent from `right`.
fn difference(
    left: &FixedBitSet,
    right: &FixedBitSet,
    node_len: NodeCapacity,
) -> FixedBitSet
{
    let mut result = FixedBitSet::with_capacity(usize::from(node_len));
    for member in left.ones() {
        if !right.contains(member) {
            result.insert(member);
        }
    }
    result
}
/// Adds split blocks and deterministic future splitters.
fn push_split_blocks(
    next_blocks: &mut Vec<FixedBitSet>,
    splitters: &mut Vec<FixedBitSet>,
    inside: FixedBitSet,
    outside: FixedBitSet,
)
{
    next_blocks.push(inside.clone());
    next_blocks.push(outside.clone());
    splitters.push(outside);
    splitters.push(inside);
}

/// Converts internal bitset blocks to a public canonical partition.
fn materialize_partition(
    blocks: &[FixedBitSet],
    node_count: NodeCount,
    node_len: NodeCapacity,
) -> Result<Partition, GraphValidationError>
{
    let mut canonical_blocks = Vec::with_capacity(blocks.len());
    for block in blocks {
        let members = bitset_members(block)?;
        canonical_blocks.push(members);
    }
    canonical_blocks.sort_by_key(|block| first_member(block));

    let mut state_to_block = vec![BlockIndex::default(); usize::from(node_len)];
    for block_index in 0 .. canonical_blocks.len() {
        let block_raw = u32::try_from(block_index)
            .map_err(|_conversion_error| GraphValidationError::NodeCountTooLarge { node_count })?;
        let block_id = BlockIndex::from(block_raw);
        let Some(block) = canonical_blocks.get(block_index)
        else {
            return Err(GraphValidationError::ArithmeticOverflow);
        };
        for &state in block {
            let state_position = checked_node_index(state, node_count)?;
            let Some(slot) = state_to_block.get_mut(usize::from(state_position))
            else {
                return Err(GraphValidationError::NodeOutOfBounds {
                    node: state,
                    node_count,
                });
            };
            *slot = block_id;
        }
    }

    Ok(Partition {
        node_count,
        blocks: canonical_blocks,
        state_to_block,
    })
}

/// Converts relation bitsets to public canonical rows.
fn materialize_simulation(
    relation: Vec<FixedBitSet>,
    node_count: NodeCount,
) -> Result<Simulation, GraphValidationError>
{
    let node_len = checked_node_len(node_count)?;
    let mut rows = Vec::with_capacity(usize::from(node_len));
    for subject in node_count.ids() {
        let subject_position = checked_node_index(subject, node_count)?;
        let Some(row) = relation.get(usize::from(subject_position))
        else {
            return Err(GraphValidationError::NodeOutOfBounds {
                node: subject,
                node_count,
            });
        };
        let candidates = bitset_members(row)?;
        rows.push(SimulationRow {
            subject,
            candidates,
        });
    }
    Ok(Simulation {
        node_count,
        rows,
        relation,
    })
}
/// Converts bitset members to ascending dense nodes.
fn bitset_members(bitset: &FixedBitSet) -> Result<Vec<NodeId>, GraphValidationError>
{
    let mut members = Vec::with_capacity(usize::from(bit_count(bitset)));
    for member in bitset.ones() {
        let node = checked_node_from_index(NodePosition::from(member))?;
        members.push(node);
    }
    Ok(members)
}
/// Counts set bits.
fn bit_count(bitset: &FixedBitSet) -> NodeCapacity
{
    NodeCapacity::from(bitset.ones().count())
}
/// Converts a public query node to a vector index.
fn query_index(
    node: NodeId,
    node_count: NodeCount,
) -> Result<NodePosition, GraphValidationError>
{
    checked_node_index(node, node_count)
}
/// Converts a bitset index back into a dense node id.
fn checked_node_from_index(index: NodePosition) -> Result<NodeId, GraphValidationError>
{
    NodeId::try_from(index).map_err(|_conversion_error| GraphValidationError::ArithmeticOverflow)
}
/// Converts and validates a dense node id to a vector index.
fn checked_node_index(
    node: NodeId,
    node_count: NodeCount,
) -> Result<NodePosition, GraphValidationError>
{
    if u32::from(node) < u32::from(node_count) {
        NodePosition::try_from(node)
            .map_err(|_conversion_error| GraphValidationError::NodeCountTooLarge { node_count })
    }
    else {
        Err(GraphValidationError::NodeOutOfBounds { node, node_count })
    }
}

/// Tests whether one candidate currently simulates one subject.
fn candidate_matches_subject(
    adjacency: &[Vec<NodeId>],
    relation: &[FixedBitSet],
    subject: NodeId,
    candidate: NodeId,
    node_count: NodeCount,
) -> Result<SimulationMatch, GraphValidationError>
{
    let subject_position = checked_node_index(subject, node_count)?;
    let candidate_position = checked_node_index(candidate, node_count)?;
    let Some(subject_successors) = adjacency.get(usize::from(subject_position))
    else {
        return Err(GraphValidationError::NodeOutOfBounds {
            node: subject,
            node_count,
        });
    };
    let Some(candidate_successors) = adjacency.get(usize::from(candidate_position))
    else {
        return Err(GraphValidationError::NodeOutOfBounds {
            node: candidate,
            node_count,
        });
    };

    for &subject_target in subject_successors {
        let target_matched = simulation_support::has_matching_candidate_target(
            relation,
            candidate_successors,
            subject_target,
            node_count,
        )?;
        if !bool::from(target_matched) {
            return Ok(SimulationMatch::from(false));
        }
    }
    Ok(SimulationMatch::from(true))
}

/// Candidate-target matching helpers for simulation refinement.
mod simulation_support
{
    use super::FixedBitSet;
    use super::GraphValidationError;
    use super::MatchingCandidateTarget;
    use super::NodeCount;
    use super::NodeId;
    use super::checked_node_index;

    /// Tests whether one subject target has a related candidate target.
    pub(super) fn has_matching_candidate_target(
        relation: &[FixedBitSet],
        candidate_successors: &[NodeId],
        subject_target: NodeId,
        node_count: NodeCount,
    ) -> Result<MatchingCandidateTarget, GraphValidationError>
    {
        let subject_target_position = checked_node_index(subject_target, node_count)?;
        let Some(row) = relation.get(usize::from(subject_target_position))
        else {
            return Err(GraphValidationError::NodeOutOfBounds {
                node: subject_target,
                node_count,
            });
        };
        for &candidate_target in candidate_successors {
            let candidate_target_position = checked_node_index(candidate_target, node_count)?;
            if row.contains(usize::from(candidate_target_position)) {
                return Ok(MatchingCandidateTarget::from(true));
            }
        }
        Ok(MatchingCandidateTarget::from(false))
    }
}

/// Returns the first member for canonical block sorting.
fn first_member(block: &[NodeId]) -> Option<NodeId>
{
    block.first().copied()
}
