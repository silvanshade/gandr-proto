//! Incremental **potential maintenance**: a valuation kept current under
//! offset-carrying constraints `value(target) >= value(source) + offset`.
//!
//! [`PotentialMaintenance`] is the algebra-carrying counterpart of
//! [`crate::AcyclicityMaintenance`]. Where the order structure asks whether the
//! admitted edges can be linearized at all, this one asks whether the admitted
//! constraints admit a valuation — and those are different questions the moment
//! an offset is not strictly positive.
//!
//! # What the offsets change
//!
//! A constraint set is unsatisfiable exactly when some cycle's offsets sum to a
//! **positive** number: going once round such a cycle demands that a value
//! exceed itself. A cycle summing to zero forces its nodes to share one value
//! and is satisfiable; a cycle summing negative is satisfied outright.
//!
//! So refusing every cycle — which is all a topological order can do — is
//! sound and incomplete. It is exactly complete on the regime where every
//! offset is at least one, because there every cycle sums positive. That regime
//! is the trivial coupling, and leaving it is what forces a consumer off the
//! graph-theoretic structure and onto this one.
//!
//! # The propagation
//!
//! Inserting a constraint the standing valuation already satisfies costs one
//! comparison. Otherwise the target's value is raised to meet it and the raise
//! is propagated along outgoing constraints until nothing more is forced. The
//! propagation never expands the new constraint's source: reaching it is
//! precisely the positive-cycle condition, so it is reported as a refutation
//! instead, and the valuation is rolled back to what it was before the offer.

use alloc::vec::Vec;

use gandr_theory_graphs::CycleWitness;
use gandr_theory_graphs::EdgeId;
use gandr_theory_graphs::NodeCount;
use gandr_theory_graphs::NodeId;

use crate::maintenance::InsertionCount;
use crate::slot::SlotIndex;

/// The integer offset a constraint carries: the constraint
/// `source -> target` with offset `k` demands `value(target) >= value(source) +
/// k`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Offset(i64);

impl From<i64> for Offset
{
    #[inline]
    fn from(value: i64) -> Self
    {
        return Self(value);
    }
}

impl From<Offset> for i64
{
    #[inline]
    fn from(value: Offset) -> Self
    {
        return value.0;
    }
}

/// A node's standing value in the maintained valuation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Potential(i64);

impl From<i64> for Potential
{
    #[inline]
    fn from(value: i64) -> Self
    {
        return Self(value);
    }
}

impl From<Potential> for i64
{
    #[inline]
    fn from(value: Potential) -> Self
    {
        return value.0;
    }
}

/// The number of constraints a [`PotentialMaintenance`] currently holds.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AdmittedConstraintCount(u64);

impl AdmittedConstraintCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<AdmittedConstraintCount> for u64
{
    #[inline]
    fn from(value: AdmittedConstraintCount) -> Self
    {
        return value.0;
    }
}

/// The number of value raises the propagation has performed.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RaiseCount(u64);

impl RaiseCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<RaiseCount> for u64
{
    #[inline]
    fn from(value: RaiseCount) -> Self
    {
        return value.0;
    }
}

/// The number of constraints the propagation has examined.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RelaxationCount(u64);

impl RelaxationCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<RelaxationCount> for u64
{
    #[inline]
    fn from(value: RelaxationCount) -> Self
    {
        return value.0;
    }
}

/// The number of offers refused as unsatisfiable.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RefutationCount(u64);

impl RefutationCount
{
    /// This count raised by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

impl From<RefutationCount> for u64
{
    #[inline]
    fn from(value: RefutationCount) -> Self
    {
        return value.0;
    }
}

/// Whether the standing valuation satisfies every admitted constraint.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FeasibilityStatus(bool);

impl From<FeasibilityStatus> for bool
{
    #[inline]
    fn from(value: FeasibilityStatus) -> Self
    {
        return value.0;
    }
}

impl core::ops::Not for FeasibilityStatus
{
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output
    {
        return Self(!self.0);
    }
}

/// A search epoch, stamped into the reusable mark buffer so a propagation
/// clears its state in constant time.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Epoch(u64);

impl Epoch
{
    /// The epoch no propagation ever runs under.
    const UNVISITED: Self = Self(0);

    /// This epoch advanced by one, saturating at the representable maximum.
    #[inline]
    fn saturating_increment(self) -> Self
    {
        return Self(self.0.saturating_add(1));
    }
}

/// A failure of a [`PotentialMaintenance`] operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PotentialError
{
    /// A node identifier does not address a slot of this structure.
    #[error("a node identifier does not address a maintained node")]
    NodeCapacity,
    /// A value or offset sum left the representable range.
    #[error("a maintained value left the representable range")]
    ValueOverflow,
    /// The propagation exceeded the budget its node count allows, which a
    /// satisfiable constraint set never does.
    #[error("the constraint propagation exceeded its budget")]
    PropagationBudgetExhausted,
    /// An internal checked arithmetic operation overflowed.
    #[error("an internal checked arithmetic operation overflowed")]
    ArithmeticOverflow,
}

/// The verdict on one offered constraint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConstraintVerdict
{
    /// The standing valuation already satisfies the constraint, so nothing
    /// moved.
    Satisfied,
    /// The constraint was satisfied by raising values through the affected
    /// cone.
    SatisfiedAfterRaise,
    /// No valuation satisfies the constraint set the offer would create; the
    /// witness is the positive-weight cycle, and the offer was **not**
    /// recorded.
    Refuted(CycleWitness),
}

/// The work a [`PotentialMaintenance`] has performed since construction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PotentialTelemetry
{
    /// Constraints offered to [`PotentialMaintenance::insert_constraint`].
    pub insertions: InsertionCount,
    /// Value raises performed by the propagation.
    pub raises: RaiseCount,
    /// Constraints examined by the propagation.
    pub relaxations: RelaxationCount,
    /// Offers refused as unsatisfiable.
    pub refutations: RefutationCount,
}

/// One admitted constraint, held on its source's row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Constraint
{
    /// The constrained node.
    target: NodeId,
    /// The offset the constraint demands.
    offset: Offset,
}

/// Reusable propagation state, kept across insertions so a bounded raise
/// allocates nothing once the buffers have grown.
#[derive(Debug, Default)]
struct Scratch
{
    /// The epoch at which each node was last raised.
    mark: Vec<Epoch>,
    /// The node whose raise forced each node's, meaningful exactly where
    /// [`Scratch::mark`] carries the current epoch.
    parent: Vec<NodeId>,
    /// The epoch of the propagation most recently begun.
    epoch: Epoch,
    /// The explicit work queue; recursion is never used.
    queue: Vec<NodeId>,
    /// Each raised node paired with the value it held before the raise, so a
    /// refuted offer can be rolled back exactly.
    journal: Vec<(NodeId, Potential)>,
}

/// A constraint system whose **valuation is maintained under constraint
/// insertion**, refusing exactly the constraints no valuation satisfies.
///
/// Nodes are dense [`NodeId`]s and are created on demand at value zero.
pub struct PotentialMaintenance
{
    /// The standing valuation, indexed by dense node id.
    values: Vec<Potential>,
    /// Admitted outgoing constraints per node, indexed by dense node id.
    constraints: Vec<Vec<Constraint>>,
    /// The number of admitted constraints.
    admitted: AdmittedConstraintCount,
    /// The work counters.
    telemetry: PotentialTelemetry,
    /// The reusable propagation buffers.
    scratch: Scratch,
}

impl PotentialMaintenance
{
    /// An empty structure with no nodes and no constraints.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        return Self {
            values: Vec::new(),
            constraints: Vec::new(),
            admitted: AdmittedConstraintCount::default(),
            telemetry: PotentialTelemetry::default(),
            scratch: Scratch::default(),
        };
    }

    /// The number of nodes the structure holds.
    #[inline]
    #[must_use]
    pub fn nodes(&self) -> NodeCount
    {
        return NodeCount::from(u32::try_from(self.values.len()).unwrap_or(u32::MAX));
    }

    /// The number of admitted constraints.
    #[inline]
    #[must_use]
    pub fn admitted_constraints(&self) -> AdmittedConstraintCount
    {
        return self.admitted;
    }

    /// The work performed since construction.
    #[inline]
    #[must_use]
    pub fn telemetry(&self) -> PotentialTelemetry
    {
        return self.telemetry;
    }

    /// The standing value of `node`, or `None` when the identifier addresses no
    /// maintained node.
    #[inline]
    #[must_use]
    pub fn value(
        &self,
        node: NodeId,
    ) -> Option<Potential>
    {
        let index = Self::index_of(node).ok()?;
        return self.values.get(usize::from(index)).copied();
    }

    /// Whether the standing valuation satisfies every admitted constraint.
    ///
    /// This is the structure's own invariant, stated as a query so a consumer —
    /// or a differential — can check it rather than trust it.
    ///
    /// # Contract
    /// - ensures: positive exactly when `value(target) >= value(source) +
    ///   offset` holds for every admitted constraint; an offset sum that leaves
    ///   the representable range reports negative rather than failing, because
    ///   an unrepresentable demand is not a satisfied one.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 boundary — a hand-lowered value that breaks one
    ///   admitted constraint is distinguished from the same system before the
    ///   change, which is what makes this query the probe's teeth rather than a
    ///   restatement of the propagation.
    /// - witness: `potential::tests::a_corrupted_valuation_is_caught_by_the_invariant`
    /// - witness: `potential::tests::the_valuation_stays_feasible`
    #[inline]
    #[must_use]
    pub fn valuation_is_feasible(&self) -> FeasibilityStatus
    {
        for (index, row) in self.constraints.iter().enumerate() {
            let Some(&source_value) = self.values.get(index)
            else {
                return FeasibilityStatus(false);
            };
            for constraint in row {
                let Some(target_value) = self.value(constraint.target)
                else {
                    return FeasibilityStatus(false);
                };
                let Some(demanded) = source_value.0.checked_add(constraint.offset.0)
                else {
                    return FeasibilityStatus(false);
                };
                if target_value.0 < demanded {
                    return FeasibilityStatus(false);
                }
            }
        }
        return FeasibilityStatus(true);
    }

    /// **Offer one constraint** `value(target) >= value(source) + offset`,
    /// admitting it exactly when the resulting system is satisfiable.
    ///
    /// The standing valuation decides the cheap case on its own. Otherwise the
    /// target is raised to meet the constraint and the raise propagates until
    /// nothing further is forced; reaching the constraint's own source means
    /// the offer closes a positive-weight cycle, which is refused with the
    /// cycle as its witness and the valuation restored.
    ///
    /// A self constraint is satisfiable exactly when its offset is not
    /// positive, which is the sharpest visible difference from
    /// [`crate::AcyclicityMaintenance`]: that structure refuses every self loop,
    /// because a loop is a cycle whatever it weighs.
    ///
    /// # Contract
    /// - requires: nothing; unseen nodes are created at value zero.
    /// - ensures: [`ConstraintVerdict::Refuted`] exactly when no valuation
    ///   satisfies the admitted constraints together with the offer, in which
    ///   case the valuation and the constraint set are unchanged apart from any
    ///   nodes the offer created; otherwise the constraint is recorded and the
    ///   standing valuation satisfies it.
    /// - provides: a [`CycleWitness`] on refutation whose walk is closed and
    ///   whose offsets sum positive.
    /// - fails: [`PotentialError::ValueOverflow`] when a demanded value leaves
    ///   the representable range,
    ///   [`PotentialError::PropagationBudgetExhausted`] when the propagation
    ///   runs past the bound a satisfiable system respects, and
    ///   [`PotentialError::NodeCapacity`] or
    ///   [`PotentialError::ArithmeticOverflow`] on an internal conversion the
    ///   node capacity precludes.
    /// - panics: none.
    /// - intension: the work is bounded by the cone the raise actually forces —
    ///   a satisfied offer touches nothing, and a refuted offer leaves the
    ///   valuation byte-for-byte as it found it.
    ///
    /// # Errors
    /// Returns [`PotentialError`] as enumerated above.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence + property — over generated constraint streams
    ///   the valuation satisfies every admitted constraint after every offer,
    ///   and a refutation's witness is a closed walk whose offsets sum
    ///   positive. Boundary: the same cycle is `Refuted` when its offsets sum
    ///   to one and `SatisfiedAfterRaise` when they sum to zero.
    /// - witness: `potential::tests::a_zero_weight_cycle_is_satisfiable`
    /// - witness: `potential::tests::a_positive_weight_cycle_is_refuted`
    /// - witness: `potential::tests::a_refuted_offer_restores_the_valuation`
    /// - witness: `dynamic_graphs::probe::acyclicity_refuses_a_superset_of_what_offsets_refute`
    #[inline]
    pub fn insert_constraint(
        &mut self,
        edge: EdgeId,
        offset: Offset,
    ) -> Result<ConstraintVerdict, PotentialError>
    {
        self.telemetry.insertions = self.telemetry.insertions.saturating_increment();
        self.ensure_node(edge.source)?;
        self.ensure_node(edge.target)?;
        if edge.source == edge.target {
            if offset > Offset(0) {
                self.telemetry.refutations = self.telemetry.refutations.saturating_increment();
                return Ok(ConstraintVerdict::Refuted(CycleWitness {
                    nodes: alloc::vec![edge.source, edge.source],
                    edges: alloc::vec![edge],
                }));
            }
            self.record(edge, offset)?;
            return Ok(ConstraintVerdict::Satisfied);
        }
        let source_value = self
            .value(edge.source)
            .ok_or(PotentialError::NodeCapacity)?;
        let target_value = self
            .value(edge.target)
            .ok_or(PotentialError::NodeCapacity)?;
        let demanded = source_value
            .0
            .checked_add(offset.0)
            .ok_or(PotentialError::ValueOverflow)?;
        if target_value.0 >= demanded {
            self.record(edge, offset)?;
            return Ok(ConstraintVerdict::Satisfied);
        }
        let mut scratch = core::mem::take(&mut self.scratch);
        let outcome = self.propagate(&mut scratch, edge, Potential(demanded));
        self.scratch = scratch;
        match outcome? {
            | Some(witness) => {
                self.telemetry.refutations = self.telemetry.refutations.saturating_increment();
                Ok(ConstraintVerdict::Refuted(witness))
            },
            | None => {
                self.record(edge, offset)?;
                Ok(ConstraintVerdict::SatisfiedAfterRaise)
            },
        }
    }

    // ----- internal helpers ------------------------------------------------

    /// The dense vector index addressed by `node`.
    ///
    /// # Errors
    /// Returns [`PotentialError::ArithmeticOverflow`] when the identifier does
    /// not fit the host address space.
    #[inline]
    fn index_of(node: NodeId) -> Result<SlotIndex, PotentialError>
    {
        return SlotIndex::try_from(node).map_err(|_ignored| PotentialError::ArithmeticOverflow);
    }

    /// Creates every node up to and including `node`, at value zero.
    ///
    /// # Errors
    /// Returns [`PotentialError::ArithmeticOverflow`] on an internal
    /// conversion.
    #[inline]
    fn ensure_node(
        &mut self,
        node: NodeId,
    ) -> Result<(), PotentialError>
    {
        let index = Self::index_of(node)?;
        let required = usize::from(index)
            .checked_add(1)
            .ok_or(PotentialError::ArithmeticOverflow)?;
        while self.values.len() < required {
            self.values.push(Potential::default());
            self.constraints.push(Vec::new());
        }
        return Ok(());
    }

    /// Records a constraint, ignoring an exact repeat.
    ///
    /// # Errors
    /// Returns [`PotentialError::NodeCapacity`] when the source addresses no
    /// maintained node, or [`PotentialError::ArithmeticOverflow`] on an
    /// internal conversion.
    #[inline]
    fn record(
        &mut self,
        edge: EdgeId,
        offset: Offset,
    ) -> Result<(), PotentialError>
    {
        let index = Self::index_of(edge.source)?;
        let row = self
            .constraints
            .get_mut(usize::from(index))
            .ok_or(PotentialError::NodeCapacity)?;
        let entry = Constraint {
            target: edge.target,
            offset,
        };
        if row.contains(&entry) {
            return Ok(());
        }
        row.push(entry);
        self.admitted = self.admitted.saturating_increment();
        return Ok(());
    }

    /// Raises the offered constraint's target to `demanded` and propagates,
    /// returning the positive-weight cycle when the offer is unsatisfiable.
    ///
    /// The valuation is restored exactly when a cycle is found, so a refuted
    /// offer leaves no trace.
    ///
    /// # Errors
    /// Returns [`PotentialError`] as
    /// [`PotentialMaintenance::insert_constraint`] enumerates.
    #[inline]
    fn propagate(
        &mut self,
        scratch: &mut Scratch,
        edge: EdgeId,
        demanded: Potential,
    ) -> Result<Option<CycleWitness>, PotentialError>
    {
        let width = self.values.len();
        scratch.mark.resize(width, Epoch::UNVISITED);
        scratch.parent.resize(width, NodeId::default());
        scratch.epoch = scratch.epoch.saturating_increment();
        scratch.queue.clear();
        scratch.journal.clear();
        let epoch = scratch.epoch;
        // A satisfiable system needs at most one raise per node per node, so a
        // run past that bound is a defect rather than a slow answer.
        let budget = width.saturating_mul(width).saturating_add(1);

        let target_index = Self::index_of(edge.target)?;
        let previous = self
            .values
            .get(usize::from(target_index))
            .copied()
            .ok_or(PotentialError::NodeCapacity)?;
        scratch.journal.push((edge.target, previous));
        let slot = self
            .values
            .get_mut(usize::from(target_index))
            .ok_or(PotentialError::NodeCapacity)?;
        *slot = demanded;
        let target_mark = scratch
            .mark
            .get_mut(usize::from(target_index))
            .ok_or(PotentialError::NodeCapacity)?;
        *target_mark = epoch;
        scratch.queue.push(edge.target);
        self.telemetry.raises = self.telemetry.raises.saturating_increment();

        while let Some(node) = scratch.queue.pop() {
            let node_index = Self::index_of(node)?;
            let base = self
                .values
                .get(usize::from(node_index))
                .copied()
                .ok_or(PotentialError::NodeCapacity)?;
            let row = self
                .constraints
                .get(usize::from(node_index))
                .ok_or(PotentialError::NodeCapacity)?;
            // The row is cloned rather than borrowed because the loop writes
            // both the valuation and the scratch marks.
            let outgoing = row.clone();
            for constraint in outgoing {
                self.telemetry.relaxations = self.telemetry.relaxations.saturating_increment();
                let needed = base
                    .0
                    .checked_add(constraint.offset.0)
                    .ok_or(PotentialError::ValueOverflow)?;
                let constrained_index = Self::index_of(constraint.target)?;
                let standing = self
                    .values
                    .get(usize::from(constrained_index))
                    .copied()
                    .ok_or(PotentialError::NodeCapacity)?;
                if standing.0 >= needed {
                    continue;
                }
                if constraint.target == edge.source {
                    let witness = Self::cycle_from(scratch, edge, node, epoch)?;
                    Self::restore(&mut self.values, &scratch.journal)?;
                    return Ok(Some(witness));
                }
                if scratch.journal.len() > budget {
                    Self::restore(&mut self.values, &scratch.journal)?;
                    return Err(PotentialError::PropagationBudgetExhausted);
                }
                scratch.journal.push((constraint.target, standing));
                let constrained = self
                    .values
                    .get_mut(usize::from(constrained_index))
                    .ok_or(PotentialError::NodeCapacity)?;
                *constrained = Potential(needed);
                let constrained_mark = scratch
                    .mark
                    .get_mut(usize::from(constrained_index))
                    .ok_or(PotentialError::NodeCapacity)?;
                *constrained_mark = epoch;
                let parent = scratch
                    .parent
                    .get_mut(usize::from(constrained_index))
                    .ok_or(PotentialError::NodeCapacity)?;
                *parent = node;
                scratch.queue.push(constraint.target);
                self.telemetry.raises = self.telemetry.raises.saturating_increment();
            }
        }
        return Ok(None);
    }

    /// Restores every journalled value, most recent first.
    ///
    /// # Errors
    /// Returns [`PotentialError::NodeCapacity`] when a journalled node no
    /// longer addresses a slot, which the journal's construction precludes.
    #[inline]
    fn restore(
        values: &mut [Potential],
        journal: &[(NodeId, Potential)],
    ) -> Result<(), PotentialError>
    {
        for &(node, previous) in journal.iter().rev() {
            let index = Self::index_of(node)?;
            let slot = values
                .get_mut(usize::from(index))
                .ok_or(PotentialError::NodeCapacity)?;
            *slot = previous;
        }
        return Ok(());
    }

    /// Rebuilds the positive-weight cycle the offered constraint would close,
    /// from the propagation's parent links.
    ///
    /// `last` is the node whose constraint forces the offer's source, so the
    /// cycle runs from the offer's target down the parent chain to `last`, on
    /// to the offer's source, and back along the offered constraint.
    ///
    /// # Errors
    /// Returns [`PotentialError::PropagationBudgetExhausted`] when the parent
    /// chain does not reach the offer's target within the node count, which the
    /// propagation's own construction precludes.
    #[inline]
    fn cycle_from(
        scratch: &Scratch,
        edge: EdgeId,
        last: NodeId,
        epoch: Epoch,
    ) -> Result<CycleWitness, PotentialError>
    {
        let mut walk: Vec<NodeId> = alloc::vec![last];
        let mut cursor = last;
        let mut remaining = scratch.parent.len();
        while cursor != edge.target {
            let index = Self::index_of(cursor)?;
            let mark = scratch
                .mark
                .get(usize::from(index))
                .copied()
                .ok_or(PotentialError::NodeCapacity)?;
            if mark != epoch {
                return Err(PotentialError::PropagationBudgetExhausted);
            }
            let parent = scratch
                .parent
                .get(usize::from(index))
                .copied()
                .ok_or(PotentialError::NodeCapacity)?;
            walk.push(parent);
            cursor = parent;
            remaining = remaining
                .checked_sub(1)
                .ok_or(PotentialError::PropagationBudgetExhausted)?;
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
}

impl Default for PotentialMaintenance
{
    #[inline]
    fn default() -> Self
    {
        return Self::new();
    }
}

#[cfg(test)]
mod tests
{
    use alloc::vec;

    use gandr_theory_graphs::EdgeId;
    use gandr_theory_graphs::NodeId;

    use super::ConstraintVerdict;
    use super::Offset;
    use super::Potential;
    use super::PotentialMaintenance;

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

    /// A structure holding exactly the offered constraints that were admitted.
    fn admit(offers: &[(EdgeId, Offset)]) -> PotentialMaintenance
    {
        let mut structure = PotentialMaintenance::new();
        for &(offer, offset) in offers {
            structure
                .insert_constraint(offer, offset)
                .expect("insertion is total over well-formed identifiers");
        }
        structure
    }

    /// One offered constraint from small dense indices and an offset.
    fn constraint<Source, Target, Weight>(
        source: Source,
        target: Target,
        offset: Weight,
    ) -> (EdgeId, Offset)
    where
        Source: Into<NodeId>,
        Target: Into<NodeId>,
        Weight: Into<Offset>,
    {
        (edge(source, target), offset.into())
    }

    #[test]
    fn a_zero_weight_cycle_is_satisfiable()
    {
        let mut structure = admit(&[constraint(0, 1, 0), constraint(1, 2, 0)]);
        let verdict = structure
            .insert_constraint(edge(2u32, 0u32), Offset::from(0))
            .expect("insertion succeeds");
        assert_eq!(
            ConstraintVerdict::Satisfied,
            verdict,
            "a zero-weight cycle forces equality and is satisfiable"
        );
        assert!(
            bool::from(structure.valuation_is_feasible()),
            "the valuation still satisfies every constraint"
        );
        assert_eq!(
            structure.value(node(0u32)),
            structure.value(node(2u32)),
            "the cycle collapsed the three values into one"
        );
    }

    #[test]
    fn a_positive_weight_cycle_is_refuted()
    {
        let mut structure = admit(&[constraint(0, 1, 1), constraint(1, 2, 1)]);
        let verdict = structure
            .insert_constraint(edge(2u32, 0u32), Offset::from(1))
            .expect("insertion succeeds");
        let ConstraintVerdict::Refuted(witness) = verdict
        else {
            panic!("a cycle of total weight three admits no valuation");
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
            u64::from(structure.admitted_constraints()),
            "a refuted constraint is not recorded"
        );
    }

    #[test]
    fn a_negative_offset_cycle_is_satisfiable()
    {
        let mut structure = admit(&[constraint(0, 1, 3), constraint(1, 2, 3)]);
        let verdict = structure
            .insert_constraint(edge(2u32, 0u32), Offset::from(-7))
            .expect("insertion succeeds");
        assert_eq!(
            ConstraintVerdict::Satisfied,
            verdict,
            "the cycle sums to minus one and is satisfied outright"
        );
        assert!(
            bool::from(structure.valuation_is_feasible()),
            "the valuation satisfies every constraint"
        );
    }

    #[test]
    fn a_refuted_offer_restores_the_valuation()
    {
        let mut structure = admit(&[
            constraint(0, 1, 1),
            constraint(1, 2, 1),
            constraint(2, 3, 1),
        ]);
        let before: alloc::vec::Vec<Option<Potential>> =
            (0 .. 4).map(|index| structure.value(node(index))).collect();
        let verdict = structure
            .insert_constraint(edge(3u32, 0u32), Offset::from(1))
            .expect("insertion succeeds");
        assert!(
            matches!(verdict, ConstraintVerdict::Refuted(_)),
            "the offer closes a positive cycle"
        );
        let after: alloc::vec::Vec<Option<Potential>> =
            (0 .. 4).map(|index| structure.value(node(index))).collect();
        assert_eq!(
            before, after,
            "a refuted offer leaves the valuation exactly as it found it"
        );
        assert!(
            bool::from(structure.valuation_is_feasible()),
            "and the system is still feasible"
        );
    }

    #[test]
    fn a_self_constraint_is_refuted_only_when_positive()
    {
        let mut structure = PotentialMaintenance::new();
        let benign = structure
            .insert_constraint(edge(0u32, 0u32), Offset::from(0))
            .expect("insertion succeeds");
        assert_eq!(
            ConstraintVerdict::Satisfied,
            benign,
            "a zero self constraint demands nothing"
        );
        let hostile = structure
            .insert_constraint(edge(0u32, 0u32), Offset::from(1))
            .expect("insertion succeeds");
        assert!(
            matches!(hostile, ConstraintVerdict::Refuted(_)),
            "a positive self constraint demands a value exceed itself"
        );
    }

    #[test]
    fn the_valuation_stays_feasible()
    {
        let structure = admit(&[
            constraint(0, 1, 2),
            constraint(1, 2, 3),
            constraint(0, 2, 9),
            constraint(2, 3, 1),
            constraint(0, 3, 4),
        ]);
        assert!(
            bool::from(structure.valuation_is_feasible()),
            "the propagation reaches a feasible valuation"
        );
        let third = structure.value(node(2u32)).expect("the node exists");
        assert_eq!(
            9,
            i64::from(third),
            "the binding constraint is the direct one, not the chain"
        );
    }

    #[test]
    fn a_corrupted_valuation_is_caught_by_the_invariant()
    {
        // The seeded corruption the probe must have teeth against: the
        // constraints are left alone and one value is lowered behind the
        // structure's back, so nothing but the invariant query can notice.
        let mut structure = admit(&[constraint(0, 1, 2), constraint(1, 2, 3)]);
        assert!(
            bool::from(structure.valuation_is_feasible()),
            "the structure starts sound"
        );
        let index = PotentialMaintenance::index_of(node(2u32)).expect("the node is addressable");
        *structure
            .values
            .get_mut(usize::from(index))
            .expect("the node has a slot") = Potential::from(0i64);
        assert!(
            !bool::from(structure.valuation_is_feasible()),
            "lowering a constrained value below its demand is caught"
        );
    }
}
