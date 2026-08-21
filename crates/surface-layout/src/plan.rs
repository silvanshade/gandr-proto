//! Reference-counted render plans retained by resolved summaries.
//!
//! Slice two owns plan identity and retention because resolution allocates,
//! retains, and releases plans while it prunes its frontier. Slice three adds
//! the machine that walks these nodes.

use crate::arena::TextId;
use crate::arena::VerbatimId;
use crate::error::RenderArithmetic;
use crate::error::RenderError;
use crate::limits::RenderMeter;
use crate::measure::PhysicalLineEnding;

/// A generational identity in the plan arena.
///
/// # Contract
/// - requires: the identity was minted by one [`PlanArena`].
/// - ensures: a recycled slot cannot be mistaken for its previous node.
/// - provides: the winning-plan handle returned by resolution.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlanId
{
    /// The dense arena slot.
    slot: u32,
    /// The slot generation.
    generation: u32,
}

/// One first-order plan node.
///
/// # Contract
/// - requires: child identities belong to the same plan arena.
/// - ensures: the node contains no closure or recursive continuation.
/// - provides: the data a later VM walks to emit bytes.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum PlanNode
{
    /// Emits no bytes.
    Empty,
    /// Emits one stored text identity.
    Text(TextId),
    /// Emits one stored verbatim identity.
    Verbatim(VerbatimId),
    /// Emits a configured layout ending and indentation.
    Newline
    {
        /// Number of spaces after the ending.
        indentation: u32,
        /// Physical ending bytes.
        ending: PhysicalLineEnding,
    },
    /// Executes left before right.
    Seq
    {
        /// First child.
        left: PlanId,
        /// Second child.
        right: PlanId,
    },
}

/// One slot in the generational plan arena.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlanSlot
{
    /// Generation guarding this slot's identity.
    generation: u32,
    /// Number of live references to the node.
    references: u32,
    /// The plan node, or `None` after release.
    node: Option<PlanNode>,
}

/// The private plan store held by one [`crate::resolve::Resolved`] result.
#[derive(Debug)]
pub(crate) struct PlanArena
{
    /// Generational slots, including recycled entries.
    slots: Vec<PlanSlot>,
    /// Slot indices available for reuse.
    free: Vec<u32>,
}

impl PlanArena
{
    /// Creates an empty plan arena.
    ///
    /// # Contract
    /// - requires: no prior plan identities are live.
    /// - ensures: the first allocation starts at slot zero.
    /// - provides: the retention store for one resolution.
    /// - panics: none.
    #[inline]
    pub(crate) fn new() -> Self
    {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Allocates one plan node with one owning reference.
    ///
    /// # Contract
    /// - requires: `meter` is the operation's shared render meter.
    /// - ensures: the returned identity resolves until its final release.
    /// - provides: generational plan allocation.
    /// - fails: reports a plan limit, allocation failure, or generation
    ///   overflow.
    /// - panics: none.
    pub(crate) fn alloc(
        &mut self,
        node: PlanNode,
        meter: &mut RenderMeter,
    ) -> Result<PlanId, RenderError>
    {
        meter.charge_plan_node()?;
        if let Some(slot) = self.free.pop() {
            let index =
                usize::try_from(slot).map_err(|_error| RenderError::ArithmeticOverflow {
                    operation: RenderArithmetic::PlanRefcount,
                })?;
            let Some(entry) = self.slots.get_mut(index)
            else {
                return Err(RenderError::ArithmeticOverflow {
                    operation: RenderArithmetic::PlanRefcount,
                });
            };
            entry.generation =
                entry
                    .generation
                    .checked_add(1u32)
                    .ok_or(RenderError::ArithmeticOverflow {
                        operation: RenderArithmetic::PlanRefcount,
                    })?;
            entry.references = 1u32;
            entry.node = Some(node);
            return Ok(PlanId {
                slot,
                generation: entry.generation,
            });
        }
        self.slots
            .try_reserve(1usize)
            .map_err(|_error| RenderError::AllocationFailed {
                site: crate::error::RenderAllocationSite::PlanArena,
            })?;
        let slot =
            u32::try_from(self.slots.len()).map_err(|_error| RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::PlanRefcount,
            })?;
        self.slots.push(PlanSlot {
            generation: 0u32,
            references: 1u32,
            node: Some(node),
        });
        Ok(PlanId {
            slot,
            generation: 0u32,
        })
    }

    /// Allocates a sequence and retains its two child identities.
    ///
    /// # Contract
    /// - requires: both children are live in this arena.
    /// - ensures: the sequence owns one reference to each child.
    /// - provides: the plan operation used by concatenation.
    /// - fails: returns stale-identity, allocation, or render-budget errors.
    /// - panics: none.
    pub(crate) fn alloc_seq(
        &mut self,
        left: PlanId,
        right: PlanId,
        meter: &mut RenderMeter,
    ) -> Result<PlanId, RenderError>
    {
        self.retain(left)?;
        self.retain(right)?;
        match self.alloc(PlanNode::Seq { left, right }, meter) {
            | Ok(plan) => Ok(plan),
            | Err(error) => {
                self.release(left, meter);
                self.release(right, meter);
                Err(error)
            },
        }
    }

    /// Returns a plan node when `id` has the current slot generation.
    ///
    /// # Contract
    /// - requires: `id` may be stale or foreign.
    /// - ensures: stale generations never expose a recycled node.
    /// - provides: checked machine lookup.
    /// - panics: none.
    #[inline]
    pub(crate) fn get(
        &self,
        id: PlanId,
    ) -> Option<PlanNode>
    {
        let slot = usize::try_from(id.slot).ok()?;
        let entry = self.slots.get(slot)?;
        if entry.generation != id.generation {
            return None;
        }
        entry.node
    }

    /// Retains one live reference to a plan.
    ///
    /// # Contract
    /// - requires: `id` is a current plan identity.
    /// - ensures: the reference count increases exactly once.
    /// - provides: memo and sequence retention.
    /// - fails: rejects stale identities or reference overflow.
    /// - panics: none.
    pub(crate) fn retain(
        &mut self,
        id: PlanId,
    ) -> Result<(), RenderError>
    {
        let slot = usize::try_from(id.slot).map_err(|_error| RenderError::ArithmeticOverflow {
            operation: RenderArithmetic::PlanRefcount,
        })?;
        let Some(entry) = self.slots.get_mut(slot)
        else {
            return Err(RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::PlanRefcount,
            });
        };
        if entry.generation != id.generation || entry.node.is_none() {
            return Err(RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::PlanRefcount,
            });
        }
        entry.references =
            entry
                .references
                .checked_add(1u32)
                .ok_or(RenderError::ArithmeticOverflow {
                    operation: RenderArithmetic::PlanRefcount,
                })?;
        Ok(())
    }

    /// Releases a plan identity and all now-unreferenced children iteratively.
    ///
    /// # Contract
    /// - requires: `id` is a current plan identity with one releasable
    ///   reference.
    /// - ensures: unreachable plan nodes are recycled without recursive drop.
    /// - provides: dominance-pruning release semantics.
    /// - fails: stale identities are ignored rather than reused.
    /// - panics: none.
    pub(crate) fn release(
        &mut self,
        id: PlanId,
        meter: &mut RenderMeter,
    )
    {
        let mut pending = Vec::new();
        pending.push(id);
        while let Some(current) = pending.pop() {
            let Ok(slot) = usize::try_from(current.slot)
            else {
                continue;
            };
            let Some(entry) = self.slots.get_mut(slot)
            else {
                continue;
            };
            if entry.generation != current.generation || entry.node.is_none() {
                continue;
            }
            if entry.references > 1u32 {
                entry.references = entry.references.saturating_sub(1u32);
                continue;
            }
            let node = entry.node.take();
            entry.references = 0u32;
            self.free.push(current.slot);
            meter.release_plan_node();
            let Some(node) = node
            else {
                continue;
            };
            if let PlanNode::Seq { left, right } = node {
                pending.push(right);
                pending.push(left);
            }
        }
    }
    /// Releases one reference and returns children that became unreachable.
    ///
    /// The resolver supplies the shared work-vector accounting around these
    /// returned children, so release records never need a second stack.
    pub(crate) fn release_one(
        &mut self,
        id: PlanId,
        meter: &mut RenderMeter,
    ) -> Result<Option<(PlanId, PlanId)>, RenderError>
    {
        let slot = usize::try_from(id.slot).map_err(|_error| RenderError::ArithmeticOverflow {
            operation: RenderArithmetic::PlanRefcount,
        })?;
        let Some(entry) = self.slots.get_mut(slot)
        else {
            return Err(RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::PlanRefcount,
            });
        };
        if entry.generation != id.generation || entry.node.is_none() {
            return Err(RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::PlanRefcount,
            });
        }
        if entry.references > 1u32 {
            entry.references = entry.references.saturating_sub(1u32);
            return Ok(None);
        }
        self.free
            .try_reserve(1usize)
            .map_err(|_error| RenderError::AllocationFailed {
                site: crate::error::RenderAllocationSite::PlanArena,
            })?;
        let node = entry.node.take();
        entry.references = 0u32;
        self.free.push(id.slot);
        meter.release_plan_node();
        Ok(match node {
            | Some(PlanNode::Seq { left, right }) => Some((left, right)),
            | Some(_) | None => None,
        })
    }
}
#[cfg(test)]
mod tests
{
    use super::*;
    use crate::limits::RenderLimits;
    use crate::limits::RenderMeter;

    fn meter() -> RenderMeter
    {
        RenderMeter::try_new(RenderLimits::default()).expect("default limits are valid")
    }

    /// Recycled slots reject the identity from the prior generation.
    #[test]
    fn plan_generation_rejects_recycled_identity()
    {
        let mut arena = PlanArena::new();
        let mut meter = meter();
        let stale = arena.alloc(PlanNode::Empty, &mut meter).expect("allocate");
        arena.release(stale, &mut meter);
        let current = arena
            .alloc(PlanNode::Empty, &mut meter)
            .expect("reallocate");
        assert_ne!(stale, current);
        assert!(arena.get(stale).is_none());
        assert_eq!(arena.get(current), Some(PlanNode::Empty));
    }

    /// Deep sequence release walks children without using the native stack.
    #[test]
    fn plan_release_recycles_a_deep_sequence_iteratively()
    {
        let mut arena = PlanArena::new();
        let mut meter = meter();
        let mut root = arena.alloc(PlanNode::Empty, &mut meter).expect("allocate");
        for _ in 0u32 .. 1024u32 {
            let child = arena.alloc(PlanNode::Empty, &mut meter).expect("allocate");
            let parent = arena
                .alloc_seq(root, child, &mut meter)
                .expect("allocate sequence");
            arena.release(root, &mut meter);
            arena.release(child, &mut meter);
            root = parent;
        }
        arena.release(root, &mut meter);
        let replacement = arena.alloc(PlanNode::Empty, &mut meter).expect("reuse");
        assert!(arena.get(replacement).is_some());
    }
}
