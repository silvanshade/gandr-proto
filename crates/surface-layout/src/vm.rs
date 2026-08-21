//! The first-order render machine.
//!
//! The winning plan executes with an explicit heap stack of plan identities.
//! No choiceless document tree, candidate string, closure, or input-scaled
//! recursion is materialized.

use crate::arena::DocArena;
use crate::error::RenderArithmetic;
use crate::error::RenderError;
use crate::error::RenderLimitKind;
#[cfg(test)]
use crate::limits::RenderLimits;
use crate::limits::RenderMeter;
use crate::plan::PlanArena;
use crate::plan::PlanId;
use crate::plan::PlanNode;
use crate::render::RenderedText;
use crate::units::OutputBytes;
use crate::units::PeakVmStack;
use crate::units::VmStepsUsed;

/// Fallible output storage with an exact cumulative byte projection.
#[derive(Debug)]
pub(crate) struct OutputBuffer
{
    /// The reserved output bytes.
    text: String,
    /// Bytes appended so far.
    bytes: OutputBytes,
}

impl OutputBuffer
{
    /// Reserves exactly the selected measure's output size once.
    ///
    /// # Contract
    /// - requires: `capacity` is the checked output size of the selected plan.
    /// - ensures: the buffer has one exact fallible reservation before appends.
    /// - provides: output storage that cannot grow during VM execution.
    /// - fails: returns `AllocationFailed` when the output reservation fails,
    ///   or `ArithmeticOverflow` when the capacity cannot be represented.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns a typed render allocation or arithmetic error.
    pub(crate) fn try_new(capacity: OutputBytes) -> Result<Self, RenderError>
    {
        let capacity = usize::try_from(u64::from(capacity)).map_err(|_error| {
            RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::OutputBytes,
            }
        })?;
        let mut text = String::new();
        text.try_reserve_exact(capacity)
            .map_err(|_error| RenderError::AllocationFailed {
                site: crate::error::RenderAllocationSite::Output,
            })?;
        Ok(Self {
            text,
            bytes: OutputBytes::from(0u64),
        })
    }

    /// Converts complete output storage into the public text wrapper.
    ///
    /// # Contract
    /// - requires: the VM has completed without an error.
    /// - ensures: ownership moves without another allocation.
    /// - provides: the exact rendered bytes.
    /// - panics: none.
    pub(crate) fn into_text(self) -> RenderedText
    {
        RenderedText::from(self.text)
    }
}

/// Charges and appends one UTF-8 fragment without allowing partial output.
///
/// # Contract
/// - requires: `fragment` is one exact output fragment and `buffer` has been
///   reserved for the selected measure.
/// - ensures: the output counter advances before the string append.
/// - provides: one shared append boundary for text, endings, and spaces.
/// - fails: returns a checked output arithmetic or render-limit error.
/// - panics: none.
///
/// # Errors
/// Returns `ArithmeticOverflow` if the local output counter cannot advance, or
/// the named output limit when the meter refuses the append.
fn append_fragment<T>(
    buffer: &mut OutputBuffer,
    meter: &mut RenderMeter,
    fragment: T,
) -> Result<(), RenderError>
where
    T: AsRef<str>,
{
    let fragment = fragment.as_ref();
    let amount =
        u64::try_from(fragment.len()).map_err(|_error| RenderError::ArithmeticOverflow {
            operation: RenderArithmetic::OutputBytes,
        })?;
    let next =
        u64::from(buffer.bytes)
            .checked_add(amount)
            .ok_or(RenderError::ArithmeticOverflow {
                operation: RenderArithmetic::OutputBytes,
            })?;
    meter.charge_output_bytes(OutputBytes::from(amount))?;
    buffer.bytes = OutputBytes::from(next);
    buffer.text.push_str(fragment);
    Ok(())
}

/// Executes one retained plan with a first-order VM stack.
///
/// # Contract
/// - requires: `root` is a live identity in `plans`, and `expected` is the
///   selected measure's checked output byte count.
/// - ensures: every plan identity is popped iteratively, every sequence pushes
///   right then left under the VM-stack ceiling, and output bytes are exact.
/// - provides: complete rendered output with cumulative VM and output charges.
/// - fails: returns unknown-plan, allocation, arithmetic, or render-limit
///   errors without returning partial output.
/// - panics: none.
///
/// # Errors
/// Returns [`RenderError`] at the first refused machine step, stack growth,
/// output append, or counter/measure disagreement.
pub(crate) fn execute(
    arena: &DocArena,
    plans: &PlanArena,
    root: PlanId,
    expected: OutputBytes,
    meter: &mut RenderMeter,
) -> Result<OutputBuffer, RenderError>
{
    let mut buffer = OutputBuffer::try_new(expected)?;
    let mut stack = Vec::new();
    meter.observe_vm_stack(PeakVmStack::from(1u64))?;
    stack
        .try_reserve(1usize)
        .map_err(|_error| RenderError::AllocationFailed {
            site: crate::error::RenderAllocationSite::VmStack,
        })?;
    stack.push(root);
    while let Some(plan) = stack.pop() {
        meter.charge_vm_step()?;
        let Some(node) = plans.get(plan)
        else {
            return Err(RenderError::UnknownDoc);
        };
        match node {
            | PlanNode::Empty => {},
            | PlanNode::Text(text) => {
                let Some(identity) = arena.text_identity(text)
                else {
                    return Err(RenderError::UnknownDoc);
                };
                append_fragment(&mut buffer, meter, identity.as_ref())?;
            },
            | PlanNode::Verbatim(verbatim) => {
                let Some(identity) = arena.verbatim_identity(verbatim)
                else {
                    return Err(RenderError::UnknownDoc);
                };
                append_fragment(&mut buffer, meter, identity.as_ref())?;
            },
            | PlanNode::Newline {
                indentation,
                ending,
            } => {
                let ending = match ending {
                    | crate::measure::PhysicalLineEnding::Lf => "\n",
                    | crate::measure::PhysicalLineEnding::CrLf => "\r\n",
                };
                append_fragment(&mut buffer, meter, ending)?;
                for _space in 0u32 .. indentation {
                    append_fragment(&mut buffer, meter, " ")?;
                }
            },
            | PlanNode::Seq { left, right } => {
                let depth = u64::try_from(stack.len()).map_err(|_error| {
                    RenderError::ArithmeticOverflow {
                        operation: RenderArithmetic::StepCounter,
                    }
                })?;
                let next_depth =
                    depth
                        .checked_add(2u64)
                        .ok_or(RenderError::ArithmeticOverflow {
                            operation: RenderArithmetic::StepCounter,
                        })?;
                meter.observe_vm_stack(PeakVmStack::from(next_depth))?;
                stack
                    .try_reserve(2usize)
                    .map_err(|_error| RenderError::AllocationFailed {
                        site: crate::error::RenderAllocationSite::VmStack,
                    })?;
                stack.push(right);
                stack.push(left);
            },
        }
    }
    if buffer.bytes != expected {
        return Err(RenderError::ArithmeticOverflow {
            operation: RenderArithmetic::OutputBytes,
        });
    }
    Ok(buffer)
}

impl RenderMeter
{
    /// Charges one VM step.
    ///
    /// # Contract
    /// - requires: one plan identity is about to be popped.
    /// - ensures: the cumulative VM ceiling is checked before execution.
    /// - provides: VM-step accounting for slice three.
    /// - fails: returns the VM-step limit or arithmetic error.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`RenderError::LimitExceeded`] when the VM-step ceiling is
    /// reached, or [`RenderError::ArithmeticOverflow`] if the counter cannot
    /// advance.
    #[inline]
    pub(crate) fn charge_vm_step(&mut self) -> Result<(), RenderError>
    {
        let current = u64::from(self.used.vm_steps);
        let next = current
            .checked_add(1u64)
            .ok_or(RenderError::ArithmeticOverflow {
                operation: crate::error::RenderArithmetic::StepCounter,
            })?;
        let limit = u64::from(self.limits.max_vm_steps);
        if next > limit {
            return Err(RenderError::LimitExceeded {
                kind: RenderLimitKind::VmSteps,
                limit: crate::units::LimitBound::from(limit),
            });
        }
        self.used.vm_steps = VmStepsUsed::from(next);
        Ok(())
    }

    /// Checks and records a VM-stack peak.
    ///
    /// # Contract
    /// - requires: `depth` is the resulting live VM-stack length.
    /// - ensures: the configured peak is checked before a push.
    /// - provides: VM-stack accounting for slice three.
    /// - fails: returns the VM-stack limit when exceeded.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`RenderError::LimitExceeded`] when `depth` exceeds the
    /// configured VM-stack ceiling.
    #[inline]
    pub(crate) fn observe_vm_stack(
        &mut self,
        depth: crate::units::PeakVmStack,
    ) -> Result<(), RenderError>
    {
        let value = u64::from(depth);
        let limit = u64::from(self.limits.max_vm_stack);
        if value > limit {
            return Err(RenderError::LimitExceeded {
                kind: RenderLimitKind::VmStack,
                limit: crate::units::LimitBound::from(limit),
            });
        }
        if value > u64::from(self.used.peak_vm_stack) {
            self.used.peak_vm_stack = PeakVmStack::from(value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::error::RenderLimitKind;
    use crate::units::MaxVmStack;
    use crate::units::MaxVmSteps;
    use crate::units::PeakVmStack;
    use crate::units::VmStepsUsed;

    /// VM-step spending is private and refuses the first over-limit step.
    #[test]
    fn vm_step_limit_is_checked_before_usage_changes()
    {
        let limits = RenderLimits {
            max_vm_steps: MaxVmSteps::from(0u64),
            ..RenderLimits::default()
        };
        let mut meter = RenderMeter::try_new(limits).expect("valid limits");
        assert!(matches!(
            meter.charge_vm_step(),
            Err(RenderError::LimitExceeded {
                kind: RenderLimitKind::VmSteps,
                ..
            })
        ));
        assert_eq!(meter.usage().vm_steps, VmStepsUsed::from(0u64));
    }

    /// VM-stack spending is private and refuses the first over-limit depth.
    #[test]
    fn vm_stack_limit_is_checked_before_peak_changes()
    {
        let limits = RenderLimits {
            max_vm_stack: MaxVmStack::from(0u64),
            ..RenderLimits::default()
        };
        let mut meter = RenderMeter::try_new(limits).expect("valid limits");
        assert!(matches!(
            meter.observe_vm_stack(PeakVmStack::from(1u64)),
            Err(RenderError::LimitExceeded {
                kind: RenderLimitKind::VmStack,
                ..
            })
        ));
        assert_eq!(meter.usage().peak_vm_stack, PeakVmStack::from(0u64));
    }
}
