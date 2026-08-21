//! Width taint and exact deferred promises.
//!
//! Taint is a semantic state, not an error or truncation marker. It preserves
//! the exact context that fell outside the computation theorem and keeps the
//! resolver able to produce a complete plan.

use crate::arena::NodeId;
use crate::error::RenderError;
use crate::measure::Measure;
use crate::units::Column;
use crate::units::Indentation;

/// A private measure result: an untainted frontier or one retained promise.
#[derive(Clone, Debug)]
pub(crate) enum MeasureSet
{
    /// A non-empty Pareto frontier.
    Frontier(Vec<Measure>),
    /// A width-tainted promise.
    Tainted(TaintPromise),
}

/// The exact promise retained by a tainted state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaintPromise
{
    /// A concrete least-cost measure retained without forcing another branch.
    Ready(Measure),
    /// A subproblem deferred with its exact context.
    Deferred
    {
        /// The document identity to force.
        doc: NodeId,
        /// The exact incoming column.
        column: Column,
        /// The exact indentation.
        indentation: Indentation,
    },
}

/// Keeps the first frontier measure as a ready taint promise and reports
/// discarded ready measures to `release`.
///
/// # Contract
/// - requires: `set` is a valid resolver result.
/// - ensures: a frontier retains only its first least-cost measure.
/// - provides: the paper's `taint(Frontier)` operation.
/// - panics: none.
#[inline]
pub(crate) fn taint(
    set: MeasureSet,
    mut release: impl FnMut(Measure) -> Result<(), RenderError>,
) -> Result<MeasureSet, RenderError>
{
    match set {
        | MeasureSet::Frontier(frontier) => {
            let mut iter = frontier.into_iter();
            match iter.next() {
                | Some(first) => {
                    for measure in iter {
                        release(measure)?;
                    }
                    Ok(MeasureSet::Tainted(TaintPromise::Ready(first)))
                },
                | None => Ok(MeasureSet::Frontier(Vec::new())),
            }
        },
        | MeasureSet::Tainted(promise) => Ok(MeasureSet::Tainted(promise)),
    }
}

/// Constructs an exact deferred promise for an out-of-bound state.
///
/// # Contract
/// - requires: `doc`, `column`, and `indentation` are the exact context.
/// - ensures: no in-bound frontier is substituted for this promise.
/// - provides: context-preserving width taint.
/// - panics: none.
#[inline]
pub(crate) fn deferred(
    doc: NodeId,
    column: Column,
    indentation: Indentation,
) -> MeasureSet
{
    MeasureSet::Tainted(TaintPromise::Deferred {
        doc,
        column,
        indentation,
    })
}

/// Merges two choice results with the prescribed taint bias and reports
/// discarded ready promises to `release`.
///
/// # Contract
/// - requires: both values came from the same resolution context.
/// - ensures: frontier/frontier is left-biased on equal pairs, a frontier wins
///   over taint, and taint/taint returns the left promise unforced.
/// - provides: the six binding taint operations' choice merge.
/// - panics: none.
#[inline]
pub(crate) fn merge(
    left: MeasureSet,
    right: MeasureSet,
    mut release: impl FnMut(TaintPromise) -> Result<(), RenderError>,
) -> Result<MeasureSet, RenderError>
{
    match (left, right) {
        | (MeasureSet::Frontier(left), MeasureSet::Frontier(right)) => {
            let mut combined = left;
            combined
                .try_reserve(right.len())
                .map_err(|_error| RenderError::AllocationFailed {
                    site: crate::error::RenderAllocationSite::Frontier,
                })?;
            combined.extend(right);
            Ok(MeasureSet::Frontier(combined))
        },
        | (MeasureSet::Frontier(frontier), MeasureSet::Tainted(promise))
        | (MeasureSet::Tainted(promise), MeasureSet::Frontier(frontier)) => {
            release(promise)?;
            Ok(MeasureSet::Frontier(frontier))
        },
        | (MeasureSet::Tainted(left), MeasureSet::Tainted(right)) => {
            release(right)?;
            Ok(MeasureSet::Tainted(left))
        },
    }
}

/// Returns the first measure from a set when it is ready.
///
/// # Contract
/// - requires: the set is valid.
/// - ensures: tainted ready states expose only their retained first measure.
/// - provides: the least-cost fallback seed.
/// - panics: none.
#[inline]
pub(crate) fn first(set: &MeasureSet) -> Option<Measure>
{
    match set {
        | &MeasureSet::Frontier(ref frontier) => frontier.first().copied(),
        | &MeasureSet::Tainted(TaintPromise::Ready(ref measure)) => Some(*measure),
        | &MeasureSet::Tainted(TaintPromise::Deferred { .. }) => None,
    }
}
#[cfg(test)]
mod tests
{
    use super::*;
    use crate::limits::RenderLimits;
    use crate::limits::RenderMeter;
    use crate::measure::LayoutCost;
    use crate::plan::PlanArena;
    use crate::plan::PlanNode;
    use crate::units::Column;
    use crate::units::OutputBytes;

    fn measure(
        plans: &mut PlanArena,
        meter: &mut RenderMeter,
        column: Column,
    ) -> Measure
    {
        let plan = plans.alloc(PlanNode::Empty, meter).expect("allocate");
        Measure {
            last_column: column,
            cost: LayoutCost::zero(),
            plan,
            output_bytes: OutputBytes::from(0u64),
        }
    }

    /// Taint keeps the first candidate and reports every discarded candidate.
    #[test]
    fn taint_reports_discarded_frontier_measures()
    {
        let mut plans = PlanArena::new();
        let mut meter =
            RenderMeter::try_new(RenderLimits::default()).expect("default limits are valid");
        let first_measure = measure(&mut plans, &mut meter, Column::from(1u32));
        let second_measure = measure(&mut plans, &mut meter, Column::from(2u32));
        let mut discarded = Vec::new();
        let result = taint(
            MeasureSet::Frontier(vec![first_measure, second_measure]),
            |measure| {
                discarded.push(measure.plan);
                Ok(())
            },
        )
        .expect("taint release succeeds");
        assert_eq!(
            first(&result).map(|value| value.plan),
            Some(first_measure.plan),
        );
        assert_eq!(discarded, vec![second_measure.plan]);
    }

    /// Taint merging keeps a frontier and releases a discarded ready promise.
    #[test]
    fn merge_frontier_wins_over_ready_taint()
    {
        let mut plans = PlanArena::new();
        let mut meter =
            RenderMeter::try_new(RenderLimits::default()).expect("default limits are valid");
        let frontier_measure = measure(&mut plans, &mut meter, Column::from(1u32));
        let tainted_measure = measure(&mut plans, &mut meter, Column::from(2u32));
        let mut discarded = Vec::new();
        let result = merge(
            MeasureSet::Frontier(vec![frontier_measure]),
            MeasureSet::Tainted(TaintPromise::Ready(tainted_measure)),
            |promise| {
                if let TaintPromise::Ready(measure) = promise {
                    discarded.push(measure.plan);
                }
                Ok(())
            },
        )
        .expect("merge release succeeds");
        assert!(matches!(result, MeasureSet::Frontier(_)));
        assert_eq!(discarded, vec![tainted_measure.plan]);
    }
}
