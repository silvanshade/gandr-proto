//! Memoized, iterative resolution over the finalized document DAG.
//!
//! Resolution owns the frontier, taint, plan-retention, and render-meter
//! accounting because those representations are inseparable. The work machine
//! below uses one explicit vector and never calls itself.

use crate::arena::DocArena;
use crate::arena::DocId;
use crate::arena::DocNode;
use crate::arena::NodeId;
use crate::error::RenderArithmetic;
use crate::error::RenderError;
use crate::limits::RenderMeter;
use crate::measure::LayoutCost;
use crate::measure::LayoutOptions;
use crate::measure::Measure;
use crate::measure::WidthTaint;
use crate::measure::absolute_overflow;
use crate::measure::add_cost;
use crate::measure::add_output_bytes;
use crate::measure::incoming_overflow;
use crate::measure::line_cost;
use crate::plan::PlanArena;
use crate::plan::PlanId;
use crate::plan::PlanNode;
use crate::taint::MeasureSet;
use crate::taint::TaintPromise;
use crate::taint::deferred;
use crate::taint::first;
use crate::taint::merge;
use crate::taint::taint;
use crate::units::Column;
use crate::units::Indentation;
use crate::units::LineBreaks;
use crate::units::OutputBytes;

/// The public summary of one winning layout.
///
/// # Contract
/// - requires: the result came from [`resolve`] and its plan store remains
///   owned by this value.
/// - ensures: cost, taint, output bytes, and plan identity describe one winner.
/// - provides: the complete slice-two observable resolution surface.
/// - panics: none.
#[derive(Debug)]
pub struct Resolved
{
    /// The retained plan arena.
    plans: PlanArena,
    /// The selected winning plan identity.
    plan: PlanId,
    /// The selected plan's cost.
    cost: LayoutCost,
    /// Whether the root required a taint promise.
    width_taint: WidthTaint,
    /// Exact bytes the winning plan emits.
    output_bytes: OutputBytes,
}

impl Resolved
{
    /// Returns the winning plan identity.
    ///
    /// # Contract
    /// - requires: this result remains alive.
    /// - ensures: the identity remains valid in this result's retained arena.
    /// - provides: the handoff to the slice-three render machine.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn plan(&self) -> PlanId
    {
        let _ = self.plans.get(self.plan);
        self.plan
    }

    /// Returns the winning lexicographic cost.
    ///
    /// # Contract
    /// - requires: this result came from successful resolution.
    /// - ensures: the cost is the direct projection of the selected measure.
    /// - provides: observable optimality metadata.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn cost(&self) -> LayoutCost
    {
        self.cost
    }

    /// Returns whether width taint was required.
    ///
    /// # Contract
    /// - requires: this result came from successful resolution.
    /// - ensures: taint is reported without truncating output.
    /// - provides: the root theorem-status projection.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn width_taint(&self) -> WidthTaint
    {
        self.width_taint
    }

    /// Returns the exact selected output byte count.
    ///
    /// # Contract
    /// - requires: this result came from successful resolution.
    /// - ensures: the count includes stored bytes and layout-owned endings.
    /// - provides: output-size metadata for later reservation.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn output_bytes(&self) -> OutputBytes
    {
        self.output_bytes
    }
}
/// Evaluation mode for one document context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResolutionMode
{
    /// Use and populate the in-bound memo table.
    Memoized,
    /// Evaluate the exact tainted context without memoization.
    Forced,
}

/// Strict Pareto dominance result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Dominance
{
    /// The left measure strictly dominates the right measure.
    Strict,
    /// The left measure does not strictly dominate the right measure.
    None,
}

/// Memoization key for one in-bound context.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct MemoKey
{
    /// Finalized document node.
    node: NodeId,
    /// Incoming output column.
    column: Column,
    /// Active indentation.
    indentation: Indentation,
}

/// Continuation state for one left measure in concatenation.
#[derive(Debug)]
struct ConcatState
{
    /// Right document node to evaluate.
    right: NodeId,
    /// Indentation passed to the right child.
    indentation: Indentation,
    /// Whether deferred children must be forced.
    force: ResolutionMode,

    /// Current left-side measure.
    left: Measure,
    /// Remaining left-side frontier measures.
    remaining: Vec<Measure>,
    /// Products accumulated from completed right evaluations.
    results: Vec<Measure>,
    /// Whether any product came from a tainted state.
    tainted: bool,
}

/// One explicit resolver work entry.
#[derive(Debug)]
enum WorkItem
{
    /// Enter one document state.
    Eval
    {
        /// Document node to enter.
        node: NodeId,
        /// Incoming output column.
        column: Column,
        /// Active indentation.
        indentation: Indentation,
        /// Whether this state bypasses memoization.
        force: ResolutionMode,
    },
    /// Store the completed in-bound state.
    StoreMemo
    {
        /// Memoization key for the completed state.
        key: MemoKey,
    },
    /// Release one plan reference and any newly unreachable children.
    ReleasePlan
    {
        /// Plan identity whose reference is released.
        plan: PlanId,
    },

    /// Resume a single-child nesting operation.
    AfterNest,
    /// Resume a single-child alignment operation.
    AfterAlign,
    /// Resume a flattened-image operation.
    AfterFlatten,
    /// Resume the left branch of a choice.
    AfterChoiceLeft
    {
        /// Right branch to evaluate next.
        right: NodeId,
        /// Incoming output column.
        column: Column,
        /// Active indentation.
        indentation: Indentation,
        /// Whether the right state bypasses memoization.
        force: ResolutionMode,
    },
    /// Resume the right branch of a choice.
    AfterChoiceRight
    {
        /// Completed left branch result.
        left: MeasureSet,
    },
    /// Resume the left branch of a concatenation.
    AfterConcatLeft
    {
        /// Right branch to evaluate next.
        right: NodeId,
        /// Active indentation.
        indentation: Indentation,
        /// Whether the right state bypasses memoization.
        force: ResolutionMode,
    },
    /// Force an exact deferred left concatenation state.
    ForceConcatLeft
    {
        /// Right branch to evaluate next.
        right: NodeId,
        /// Active indentation.
        indentation: Indentation,
        /// Whether the right state bypasses memoization.
        force: ResolutionMode,
    },
    /// Resume one right-side concatenation state.
    ConcatNext(ConcatState),
    /// Force an exact deferred right-side state.
    ForceConcatRight(ConcatState),
}

/// The iterative resolver and its one private work vector.
struct Resolver<'arena, 'meter>
{
    /// Finalized document arena being resolved.
    arena: &'arena DocArena,
    /// Constant width and ending policy for this invocation.
    options: LayoutOptions,
    /// Shared render budget meter.
    meter: &'meter mut RenderMeter,
    /// Generational plan store.
    plans: PlanArena,
    /// In-bound memoized measure sets.
    memo: std::collections::HashMap<MemoKey, MeasureSet>,
    /// One explicit enter/resume work vector.
    work: Vec<WorkItem>,
}

impl<'arena, 'meter> Resolver<'arena, 'meter>
{
    /// Creates a resolver with empty memo, plan, and work stores.
    fn new(
        arena: &'arena DocArena,
        options: LayoutOptions,
        meter: &'meter mut RenderMeter,
    ) -> Self
    {
        Self {
            arena,
            options,
            meter,
            plans: PlanArena::new(),
            memo: std::collections::HashMap::new(),
            work: Vec::new(),
        }
    }

    /// Pushes one work item through the shared cumulative and peak checks.
    fn push(
        &mut self,
        item: WorkItem,
    ) -> Result<(), RenderError>
    {
        let depth = u64::try_from(self.work.len())
            .map_err(|_error| RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::ResolverWorkCounter,
            })?
            .checked_add(1u64)
            .ok_or(RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::ResolverWorkCounter,
            })?;
        self.meter
            .push_resolver_work(crate::units::PeakResolverStack::from(depth))?;
        self.work
            .try_reserve(1usize)
            .map_err(|_error| RenderError::AllocationFailed {
                site: crate::error::RenderAllocationSite::ResolverStack,
            })?;
        self.work.push(item);
        Ok(())
    }
    /// Retains every plan owned by a copied measure set.
    fn retain_set(
        &mut self,
        set: &MeasureSet,
    ) -> Result<(), RenderError>
    {
        match set {
            | &MeasureSet::Frontier(ref frontier) => {
                for measure in frontier {
                    self.plans.retain(measure.plan)?;
                }
            },
            | &MeasureSet::Tainted(TaintPromise::Ready(ref measure)) => {
                self.plans.retain(measure.plan)?;
            },
            | &MeasureSet::Tainted(TaintPromise::Deferred { .. }) => {},
        }
        Ok(())
    }
    /// Releases every plan reference owned by a measure set.
    fn release_set(
        &mut self,
        set: MeasureSet,
    ) -> Result<(), RenderError>
    {
        match set {
            | MeasureSet::Frontier(frontier) => {
                for measure in frontier {
                    self.release_measure(measure)?;
                }
            },
            | MeasureSet::Tainted(TaintPromise::Ready(measure)) => {
                self.release_measure(measure)?;
            },
            | MeasureSet::Tainted(TaintPromise::Deferred { .. }) => {},
        }
        Ok(())
    }

    /// Releases a measure whose owning set has been consumed.
    fn release_measure(
        &mut self,
        measure: Measure,
    ) -> Result<(), RenderError>
    {
        self.release_plan(measure.plan)
    }

    /// Drains plan-release records on the resolver's shared work vector.
    fn release_plan(
        &mut self,
        plan: PlanId,
    ) -> Result<(), RenderError>
    {
        self.push(WorkItem::ReleasePlan { plan })?;
        while matches!(self.work.last(), Some(WorkItem::ReleasePlan { .. })) {
            let Some(WorkItem::ReleasePlan { plan }) = self.work.pop()
            else {
                break;
            };
            let Some((left, right)) = self.plans.release_one(plan, self.meter)?
            else {
                continue;
            };
            self.push(WorkItem::ReleasePlan { plan: right })?;
            self.push(WorkItem::ReleasePlan { plan: left })?;
        }

        Ok(())
    }

    /// Releases a ready promise whose owning set has been discarded.
    fn release_promise(
        &mut self,
        promise: TaintPromise,
    ) -> Result<(), RenderError>
    {
        if let TaintPromise::Ready(measure) = promise {
            self.release_measure(measure)?;
        }
        Ok(())
    }

    /// Keeps only the least-cost measure while releasing discarded plans.
    fn taint_set(
        &mut self,
        set: MeasureSet,
    ) -> Result<MeasureSet, RenderError>
    {
        taint(set, |measure| self.release_measure(measure))
    }

    /// Merges choices while releasing tainted plans discarded by the bias.
    fn merge_sets(
        &mut self,
        mut left: MeasureSet,
        right: MeasureSet,
    ) -> Result<MeasureSet, RenderError>
    {
        let right_len = match right {
            | MeasureSet::Frontier(ref frontier) => Some(frontier.len()),
            | MeasureSet::Tainted(_) => None,
        };
        if let &mut MeasureSet::Frontier(ref mut left_frontier) = &mut left
            && let Some(right_len) = right_len
        {
            left_frontier.try_reserve(right_len).map_err(|_error| {
                RenderError::AllocationFailed {
                    site: crate::error::RenderAllocationSite::Frontier,
                }
            })?;
        }
        merge(left, right, |promise| self.release_promise(promise))
    }

    /// Runs the work machine from one root context.
    fn run(
        mut self,
        root: NodeId,
    ) -> Result<(Measure, WidthTaint, PlanArena), RenderError>
    {
        self.push(WorkItem::Eval {
            node: root,
            column: Column::from(0u32),
            indentation: Indentation::from(0u32),
            force: ResolutionMode::Memoized,
        })?;
        let mut pending: Option<MeasureSet> = None;
        loop {
            if let Some(set) = pending.take() {
                let Some(item) = self.work.pop()
                else {
                    if let MeasureSet::Tainted(TaintPromise::Deferred {
                        doc,
                        column,
                        indentation,
                    }) = set
                    {
                        self.push(WorkItem::Eval {
                            node: doc,
                            column,
                            indentation,
                            force: ResolutionMode::Forced,
                        })?;
                        continue;
                    }
                    let tainted = matches!(set, MeasureSet::Tainted(_));
                    let Some(measure) = first(&set)
                    else {
                        return Err(RenderError::ArithmeticOverflow {
                            operation: RenderArithmetic::StepCounter,
                        });
                    };
                    let status = if tainted {
                        WidthTaint::Tainted
                    }
                    else {
                        WidthTaint::Untainted
                    };
                    let mut memo = std::mem::take(&mut self.memo)
                        .into_iter()
                        .collect::<Vec<_>>();
                    memo.sort_unstable_by_key(|entry| entry.0);
                    for (_key, memo_set) in memo {
                        self.release_set(memo_set)?;
                    }
                    return Ok((measure, status, self.plans));
                };
                match item {
                    | WorkItem::StoreMemo { key } => {
                        self.memo.try_reserve(1usize).map_err(|_error| {
                            RenderError::AllocationFailed {
                                site: crate::error::RenderAllocationSite::MemoTable,
                            }
                        })?;
                        self.retain_set(&set)?;
                        self.memo.insert(key, set.clone());
                        pending = Some(set);
                    },
                    | WorkItem::ReleasePlan { plan } => {
                        self.release_plan(plan)?;
                        pending = Some(set);
                    },

                    | WorkItem::AfterNest | WorkItem::AfterAlign | WorkItem::AfterFlatten => {
                        pending = Some(set);
                    },
                    | WorkItem::AfterChoiceLeft {
                        right,
                        column,
                        indentation,
                        force,
                    } => {
                        self.push(WorkItem::AfterChoiceRight { left: set })?;
                        self.push(WorkItem::Eval {
                            node: right,
                            column,
                            indentation,
                            force,
                        })?;
                    },
                    | WorkItem::AfterChoiceRight { left } => {
                        let combined = self.merge_sets(left, set)?;
                        pending = Some(self.normalize_set(combined)?);
                    },
                    | WorkItem::AfterConcatLeft {
                        right,
                        indentation,
                        force,
                    } => {
                        if let MeasureSet::Tainted(TaintPromise::Deferred {
                            doc,
                            column,
                            indentation: deferred_indentation,
                        }) = set
                        {
                            self.push(WorkItem::ForceConcatLeft {
                                right,
                                indentation,
                                force,
                            })?;
                            self.push(WorkItem::Eval {
                                node: doc,
                                column,
                                indentation: deferred_indentation,
                                force: ResolutionMode::Forced,
                            })?;
                        }
                        else {
                            self.start_concat(right, indentation, force, set)?;
                        }
                    },
                    | WorkItem::ForceConcatLeft {
                        right,
                        indentation,
                        force,
                    } => {
                        self.start_concat(right, indentation, force, set)?;
                    },
                    | WorkItem::ConcatNext(state) | WorkItem::ForceConcatRight(state) => {
                        self.resume_concat(state, set, &mut pending)?;
                    },
                    | WorkItem::Eval { .. } => {
                        return Err(RenderError::ArithmeticOverflow {
                            operation: RenderArithmetic::StepCounter,
                        });
                    },
                }
                continue;
            }
            let Some(item) = self.work.pop()
            else {
                return Err(RenderError::ArithmeticOverflow {
                    operation: RenderArithmetic::StepCounter,
                });
            };
            match item {
                | WorkItem::ReleasePlan { plan } => {
                    self.release_plan(plan)?;
                },
                | WorkItem::Eval {
                    node,
                    column,
                    indentation,
                    force,
                } => {
                    pending = self.begin_eval(node, column, indentation, force)?;
                },
                | WorkItem::StoreMemo { .. }
                | WorkItem::AfterNest
                | WorkItem::AfterAlign
                | WorkItem::AfterFlatten
                | WorkItem::AfterChoiceLeft { .. }
                | WorkItem::AfterChoiceRight { .. }
                | WorkItem::AfterConcatLeft { .. }
                | WorkItem::ForceConcatLeft { .. }
                | WorkItem::ConcatNext(_)
                | WorkItem::ForceConcatRight(_) => {
                    return Err(RenderError::ArithmeticOverflow {
                        operation: RenderArithmetic::StepCounter,
                    });
                },
            }
        }
    }

    /// Starts one state, either returning a leaf result or pushing its
    /// continuation and child states.
    fn begin_eval(
        &mut self,
        node: NodeId,
        column: Column,
        indentation: Indentation,
        force: ResolutionMode,
    ) -> Result<Option<MeasureSet>, RenderError>
    {
        self.meter.charge_layout_step()?;
        if force == ResolutionMode::Memoized
            && (u32::from(column) > u32::from(self.options.computation_width)
                || u32::from(indentation) > u32::from(self.options.computation_width))
        {
            return Ok(Some(deferred(node, column, indentation)));
        }
        let key = MemoKey {
            node,
            column,
            indentation,
        };
        if force == ResolutionMode::Memoized {
            if let Some(result) = self.memo.get(&key).cloned() {
                self.retain_set(&result)?;
                return Ok(Some(result));
            }
            self.meter.charge_memo_state()?;
            self.push(WorkItem::StoreMemo { key })?;
        }
        let Some(stored) = self.arena.node(node)
        else {
            return Err(RenderError::UnknownDoc);
        };
        let result = match stored {
            | DocNode::Empty => Some(self.empty()?),
            | DocNode::Text(text) => Some(self.text(text, column)?),
            | DocNode::Verbatim(verbatim) => Some(self.verbatim(verbatim, column)?),
            | DocNode::Line | DocNode::HardLine => Some(self.line(indentation)?),
            | DocNode::Nest { amount, doc } => {
                let next_indentation = u32::from(indentation).checked_add(amount).ok_or(
                    RenderError::ArithmeticOverflow {
                        operation: RenderArithmetic::Indentation,
                    },
                )?;
                self.push(WorkItem::AfterNest)?;
                self.push(WorkItem::Eval {
                    node: doc,
                    column,
                    indentation: Indentation::from(next_indentation),
                    force,
                })?;
                None
            },
            | DocNode::Align { doc } => {
                self.push(WorkItem::AfterAlign)?;
                self.push(WorkItem::Eval {
                    node: doc,
                    column,
                    indentation: Indentation::from(u32::from(column)),

                    force,
                })?;
                None
            },
            | DocNode::Flatten { .. } => {
                let Some(image) = self.arena.flattened_node(node)
                else {
                    return Err(RenderError::UnknownDoc);
                };
                self.push(WorkItem::AfterFlatten)?;
                self.push(WorkItem::Eval {
                    node: image,
                    column,
                    indentation,
                    force,
                })?;
                None
            },
            | DocNode::Choice { left, right } => {
                self.push(WorkItem::AfterChoiceLeft {
                    right,
                    column,
                    indentation,
                    force,
                })?;
                self.push(WorkItem::Eval {
                    node: left,
                    column,
                    indentation,
                    force,
                })?;
                None
            },
            | DocNode::Concat { left, right } => {
                self.push(WorkItem::AfterConcatLeft {
                    right,
                    indentation,
                    force,
                })?;
                self.push(WorkItem::Eval {
                    node: left,
                    column,
                    indentation,
                    force,
                })?;
                None
            },
        };
        Ok(result)
    }
    /// Retains one candidate in a fallibly reserved, metered frontier.
    fn singleton(
        &mut self,
        measure: Measure,
    ) -> Result<MeasureSet, RenderError>
    {
        let mut frontier = Vec::new();
        frontier
            .try_reserve(1usize)
            .map_err(|_error| RenderError::AllocationFailed {
                site: crate::error::RenderAllocationSite::Frontier,
            })?;
        if let Err(error) = self.meter.charge_frontier_entry() {
            self.release_measure(measure)?;

            return Err(error);
        }
        frontier.push(measure);
        Ok(MeasureSet::Frontier(frontier))
    }

    /// Builds the empty leaf measure.
    fn empty(&mut self) -> Result<MeasureSet, RenderError>
    {
        let plan = self.plans.alloc(PlanNode::Empty, self.meter)?;
        self.singleton(Measure {
            last_column: Column::from(0u32),
            cost: LayoutCost::zero(),
            plan,
            output_bytes: OutputBytes::from(0u64),
        })
    }

    /// Builds a text leaf measure.
    fn text(
        &mut self,
        text: crate::arena::TextId,
        column: Column,
    ) -> Result<MeasureSet, RenderError>
    {
        let Some(identity) = self.arena.text_identity(text)
        else {
            return Err(RenderError::UnknownDoc);
        };
        let width = identity.width();
        let end = u32::from(column).checked_add(u32::from(width)).ok_or(
            RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::Column,
            },
        )?;
        let overflow = incoming_overflow(column, width, self.options.page_width)?;
        let bytes = identity
            .bytes_used()
            .map_err(|_error| RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::OutputBytes,
            })?;
        let plan = self.plans.alloc(PlanNode::Text(text), self.meter)?;
        let set = self.singleton(Measure {
            last_column: Column::from(end),
            cost: LayoutCost {
                squared_overflow: overflow,
                line_breaks: LineBreaks::from(0u64),
            },
            plan,
            output_bytes: OutputBytes::from(u64::from(bytes)),
        })?;

        if end > u32::from(self.options.computation_width) {
            self.taint_set(set)
        }
        else {
            Ok(set)
        }
    }

    /// Builds a verbatim leaf measure with per-fragment charging.
    fn verbatim(
        &mut self,
        verbatim: crate::arena::VerbatimId,
        column: Column,
    ) -> Result<MeasureSet, RenderError>
    {
        let Some(identity) = self.arena.verbatim_identity(verbatim)
        else {
            return Err(RenderError::UnknownDoc);
        };
        let Some(first_line) = identity.lines().first().copied()
        else {
            return Err(RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::StepCounter,
            });
        };
        let mut cost = LayoutCost::zero();
        let first_width = u32::from(first_line.scalar_width());
        let first_end =
            u32::from(column)
                .checked_add(first_width)
                .ok_or(RenderError::ArithmeticOverflow {
                    operation: RenderArithmetic::Column,
                })?;
        let first_overflow =
            incoming_overflow(column, first_line.scalar_width(), self.options.page_width)?;
        cost.squared_overflow = crate::units::SquaredOverflow::from(u64::from(first_overflow));
        let mut tainted = first_end > u32::from(self.options.computation_width);
        let mut last_column = Column::from(first_end);
        for (index, line) in identity.lines().iter().copied().enumerate() {
            if index > 0usize {
                let overflow = absolute_overflow(line.scalar_width(), self.options.page_width)?;
                let next = u64::from(cost.squared_overflow)
                    .checked_add(u64::from(overflow))
                    .ok_or(RenderError::ArithmeticOverflow {
                        operation: RenderArithmetic::SquaredOverflow,
                    })?;
                cost.squared_overflow = crate::units::SquaredOverflow::from(next);
                last_column = Column::from(u32::from(line.scalar_width()));
                if u32::from(last_column) > u32::from(self.options.computation_width) {
                    tainted = true;
                }
            }
            if line.ending().is_some() {
                let next = u64::from(cost.line_breaks).checked_add(1u64).ok_or(
                    RenderError::ArithmeticOverflow {
                        operation: RenderArithmetic::LineBreaks,
                    },
                )?;
                cost.line_breaks = LineBreaks::from(next);
            }
        }
        let bytes = identity
            .bytes_used()
            .map_err(|_error| RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::OutputBytes,
            })?;
        let plan = self.plans.alloc(PlanNode::Verbatim(verbatim), self.meter)?;
        let set = self.singleton(Measure {
            last_column,
            cost,
            plan,
            output_bytes: OutputBytes::from(u64::from(bytes)),
        })?;

        if tainted {
            self.taint_set(set)
        }
        else {
            Ok(set)
        }
    }

    /// Builds a line or hard-line measure.
    fn line(
        &mut self,
        indentation: Indentation,
    ) -> Result<MeasureSet, RenderError>
    {
        let cost = line_cost(indentation, self.options.page_width)?;
        let ending_bytes = u64::from(self.options.line_ending.byte_width());
        let bytes = ending_bytes
            .checked_add(u64::from(u32::from(indentation)))
            .ok_or(RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::OutputBytes,
            })?;
        let plan = self.plans.alloc(
            PlanNode::Newline {
                indentation: u32::from(indentation),
                ending: self.options.line_ending,
            },
            self.meter,
        )?;
        let set = self.singleton(Measure {
            last_column: Column::from(u32::from(indentation)),
            cost,
            plan,
            output_bytes: OutputBytes::from(bytes),
        })?;
        if u32::from(indentation) > u32::from(self.options.computation_width) {
            self.taint_set(set)
        }
        else {
            Ok(set)
        }
    }

    /// Starts right-side evaluation for a left result.
    fn start_concat(
        &mut self,
        right: NodeId,
        indentation: Indentation,
        force: ResolutionMode,

        left_set: MeasureSet,
    ) -> Result<(), RenderError>
    {
        let (left, remaining, tainted) = match left_set {
            | MeasureSet::Frontier(mut frontier) => {
                if frontier.is_empty() {
                    return Err(RenderError::ArithmeticOverflow {
                        operation: RenderArithmetic::StepCounter,
                    });
                }
                let first = frontier.remove(0usize);
                (first, frontier, false)
            },
            | MeasureSet::Tainted(TaintPromise::Ready(measure)) => (measure, Vec::new(), true),
            | MeasureSet::Tainted(TaintPromise::Deferred { .. }) => {
                return Err(RenderError::ArithmeticOverflow {
                    operation: RenderArithmetic::StepCounter,
                });
            },
        };
        let state = ConcatState {
            right,
            indentation,
            force,
            left,
            remaining,
            results: Vec::new(),
            tainted,
        };
        self.push(WorkItem::ConcatNext(state))?;
        self.push(WorkItem::Eval {
            node: right,
            column: left.last_column,
            indentation,
            force,
        })?;
        Ok(())
    }

    /// Resumes a right-side concatenation and schedules the next left measure.
    fn resume_concat(
        &mut self,
        mut state: ConcatState,
        right_set: MeasureSet,
        pending: &mut Option<MeasureSet>,
    ) -> Result<(), RenderError>
    {
        if let MeasureSet::Tainted(TaintPromise::Deferred {
            doc,
            column,
            indentation,
        }) = right_set
        {
            self.push(WorkItem::ForceConcatRight(state))?;
            self.push(WorkItem::Eval {
                node: doc,
                column,
                indentation,
                force: ResolutionMode::Forced,
            })?;
            return Ok(());
        }
        let right = match right_set {
            | MeasureSet::Frontier(frontier) => frontier,
            | MeasureSet::Tainted(TaintPromise::Ready(measure)) => {
                state.tainted = true;
                vec![measure]
            },
            | MeasureSet::Tainted(TaintPromise::Deferred { .. }) => Vec::new(),
        };
        for right_measure in right {
            self.meter.charge_layout_step()?;
            let column = right_measure.last_column;
            let cost = add_cost(state.left.cost, right_measure.cost)?;
            let output_bytes =
                add_output_bytes(state.left.output_bytes, right_measure.output_bytes)?;
            let plan = self
                .plans
                .alloc_seq(state.left.plan, right_measure.plan, self.meter)?;
            state
                .results
                .try_reserve(1usize)
                .map_err(|_error| RenderError::AllocationFailed {
                    site: crate::error::RenderAllocationSite::Frontier,
                })?;
            state.results.push(Measure {
                last_column: column,
                cost,
                plan,
                output_bytes,
            });
            self.release_measure(right_measure)?;
        }
        self.release_measure(state.left)?;

        if !state.remaining.is_empty() {
            let next_left = state.remaining.remove(0usize);
            state.left = next_left;
            let right = state.right;
            let indentation = state.indentation;
            let force = state.force;
            let column = next_left.last_column;

            self.push(WorkItem::ConcatNext(state))?;
            self.push(WorkItem::Eval {
                node: right,
                column,
                indentation,
                force,
            })?;
            return Ok(());
        }
        let result = MeasureSet::Frontier(self.normalize(state.results)?);
        *pending = Some(if state.tainted {
            self.taint_set(result)?
        }
        else {
            result
        });

        Ok(())
    }

    /// Normalizes a candidate set into a sorted, mutually non-dominating
    /// frontier.
    fn normalize_set(
        &mut self,
        set: MeasureSet,
    ) -> Result<MeasureSet, RenderError>
    {
        match set {
            | MeasureSet::Frontier(frontier) => Ok(MeasureSet::Frontier(self.normalize(frontier)?)),
            | MeasureSet::Tainted(promise) => Ok(MeasureSet::Tainted(promise)),
        }
    }

    /// Inserts all candidates while charging each comparison and retained
    /// entry.
    fn normalize(
        &mut self,
        candidates: Vec<Measure>,
    ) -> Result<Vec<Measure>, RenderError>
    {
        let mut frontier = Vec::new();
        for candidate in candidates {
            let mut dominated = false;
            for existing in &frontier {
                self.meter.charge_layout_step()?;
                if dominates(*existing, candidate) == Dominance::Strict
                    || (existing.cost == candidate.cost
                        && existing.last_column == candidate.last_column)
                {
                    dominated = true;
                    break;
                }
            }
            if dominated {
                self.release_measure(candidate)?;

                continue;
            }
            let retained_capacity = frontier.len();
            let mut retained = Vec::new();
            retained.try_reserve(retained_capacity).map_err(|_error| {
                RenderError::AllocationFailed {
                    site: crate::error::RenderAllocationSite::Frontier,
                }
            })?;
            for existing in frontier {
                self.meter.charge_layout_step()?;
                if dominates(candidate, existing) == Dominance::Strict {
                    self.release_measure(existing)?;
                }
                else {
                    retained.push(existing);
                }
            }
            frontier = retained;
            self.meter.charge_frontier_entry()?;
            frontier
                .try_reserve(1usize)
                .map_err(|_error| RenderError::AllocationFailed {
                    site: crate::error::RenderAllocationSite::Frontier,
                })?;
            frontier.push(candidate);
        }
        frontier.sort_by(|left, right| {
            left.cost
                .cmp(&right.cost)
                .then_with(|| u32::from(right.last_column).cmp(&u32::from(left.last_column)))
        });
        Ok(frontier)
    }
}

/// Returns whether `left` strictly dominates `right`.
fn dominates(
    left: Measure,
    right: Measure,
) -> Dominance
{
    let no_worse = left.cost <= right.cost && left.last_column <= right.last_column;
    let strict = left.cost < right.cost || left.last_column < right.last_column;
    if no_worse && strict {
        Dominance::Strict
    }
    else {
        Dominance::None
    }
}

/// Resolves a document root into its winning plan summary.
///
/// # Contract
/// - requires: `root` belongs to `arena`, and `options` has computation width
///   at least as large as page width.
/// - ensures: the selected plan has least lexicographic cost among the
///   untainted frontier and preserves exact tainted fallback context.
/// - provides: winning plan identity, cost, taint status, and output bytes.
/// - fails: returns unknown-handle, arithmetic, allocation, or render-limit
///   errors without returning partial output.
/// - panics: none.
///
/// # Errors
/// Returns [`RenderError`] for invalid handles, widths, checked arithmetic,
/// allocation failure, or a named render limit.
///
/// # Adequacy
/// - hypothesis: L4 — exhaustive small-document enumeration, frontier
///   invariants, taint-context separation, memo reuse, plan release, and exact
///   boundary witnesses distinguish the resolver's semantic decisions.
/// - witness: `algebra::exhaustive_small_documents_match_the_direct_oracle`
/// - witness: `algebra::shared_contexts_reuse_memo_states`
/// - witness: `algebra::tainted_contexts_remain_distinct`
#[inline]
pub fn resolve(
    arena: &DocArena,
    root: DocId,
    options: LayoutOptions,
    meter: &mut RenderMeter,
) -> Result<Resolved, RenderError>
{
    if !matches!(arena.contains(root), crate::arena::DocHandleStatus::Present) {
        return Err(RenderError::UnknownDoc);
    }
    if u32::from(options.computation_width) < u32::from(options.page_width) {
        return Err(RenderError::InvalidWidth);
    }
    let root_node = root.node_id();
    let (measure, width_taint, plans) = Resolver::new(arena, options, meter).run(root_node)?;
    meter.charge_output_bytes(measure.output_bytes)?;
    Ok(Resolved {
        plans,
        plan: measure.plan,
        cost: measure.cost,
        width_taint,
        output_bytes: measure.output_bytes,
    })
}
