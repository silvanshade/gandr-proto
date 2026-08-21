//! Nominal scalar types for the layout engine.
//!
//! Every quantity the engine carries has a name. A width is not an index, an
//! indentation is not a column, and a byte budget is not a node budget, so none
//! of them is spelled as a bare integer anywhere a caller can see it. The
//! wrappers are the API boundary rather than a convenience alias: each is
//! `#[repr(transparent)]`, each has a checked constructor or an exact external
//! conversion, and none exposes an inherent accessor that hands the primitive
//! back.
//!
//! The pretty-printing design authority writes these quantities as bare
//! primitives. That spelling is representation; the operation set, the
//! fallibility, the arena and identity model, and the algebra are what bind.
//! Naming them here preserves all of that and satisfies the workspace rule that
//! no crate-defined signature exposes a primitive.
//!
//! # Conversions slice one owns
//!
//! Each wrapper below states the exact conversions it must gain. The rule
//! throughout: a widening conversion is a `From`, a narrowing one is a
//! `TryFrom` whose error is the owning crate error, and neither is an inherent
//! method.
//!
//! ```text
//! impl From<u32> for NestAmount
//! impl From<u32> for ScalarWidth
//! impl TryFrom<usize> for ScalarWidth
//! impl From<u32> for MaxDocNodes
//! impl From<usize> for MaxTextBytes
//! impl From<u32> for MaxVerbatimLines
//! impl From<u64> for MaxBuildSteps
//! impl From<ScalarWidth> for LimitBound
//! impl From<NestAmount> for ScalarWidth
//! ```

use crate::error::BuildArithmetic;
use crate::error::BuildError;
use crate::error::BuildLimitKind;

/// A count of Unicode scalar values occupying one line of output.
///
/// Width in this engine is scalar count rather than display cell count. A
/// client that owns its tabs expands them before construction; a tab preserved
/// inside verbatim text counts as one scalar and is never rewritten. Moving to
/// display cells later is one change here rather than two estimators.
///
/// # Contract
/// - requires: the value is a scalar count already checked against overflow.
/// - ensures: ordering agrees with the ordering of the underlying counts.
/// - provides: the one width currency the measure, cost, and taint rules read.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ScalarWidth
{
    /// The scalar count.
    width: u32,
}

/// The additional indentation a `Nest` node applies to its child.
///
/// # Contract
/// - requires: the value is the amount written at the construction site.
/// - ensures: addition against a current indentation is checked by the caller.
/// - provides: the argument type of the builder's nesting constructor.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct NestAmount
{
    /// The indentation increment.
    amount: u32,
}

/// The ceiling on stored document nodes, flatten images included.
///
/// # Contract
/// - requires: the value is the caller's chosen ceiling.
/// - ensures: the builder refuses to store a node once the count reaches it.
/// - provides: one field of the build limit record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxDocNodes
{
    /// The node count.
    nodes: u32,
}

/// The ceiling on uniquely stored text and verbatim bytes.
///
/// # Contract
/// - requires: the value is the caller's chosen ceiling.
/// - ensures: the builder refuses to store text once the byte count reaches it.
/// - provides: one field of the build limit record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxTextBytes
{
    /// The byte count.
    bytes: usize,
}

/// The ceiling on stored verbatim physical fragments.
///
/// # Contract
/// - requires: the value is the caller's chosen ceiling.
/// - ensures: the builder refuses a verbatim node whose scan would cross it.
/// - provides: one field of the build limit record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxVerbatimLines
{
    /// The fragment count.
    lines: u32,
}

/// The ceiling on constructor and finalization steps.
///
/// A step is one checked input edge, one interner probe, one visit, or one
/// flatten edge. It is the budget that bounds work rather than storage.
///
/// # Contract
/// - requires: the value is the caller's chosen ceiling.
/// - ensures: construction and finalization refuse once the count reaches it.
/// - provides: one field of the build limit record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxBuildSteps
{
    /// The step count.
    steps: u64,
}

/// Document nodes stored so far, flatten images included.
///
/// # Contract
/// - requires: the counter is owned by exactly one build meter.
/// - ensures: the count is monotone for the meter's whole lifetime.
/// - provides: one field of the build usage record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct DocNodesUsed
{
    /// The node count.
    nodes: u64,
}

/// Uniquely stored text and verbatim bytes so far.
///
/// # Contract
/// - requires: the counter is owned by exactly one build meter.
/// - ensures: a second edge to an existing identity adds nothing to it.
/// - provides: the byte count of stored text and verbatim content.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct TextBytesUsed
{
    /// The byte count.
    bytes: u64,
}

/// Stored verbatim physical fragments so far.
///
/// # Contract
/// - requires: the counter is owned by exactly one build meter.
/// - ensures: the count is monotone for the meter's whole lifetime.
/// - provides: one field of the build usage record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VerbatimLinesUsed
{
    /// The fragment count.
    lines: u64,
}

/// Constructor and finalization steps consumed so far.
///
/// # Contract
/// - requires: the counter is owned by exactly one build meter.
/// - ensures: the count is monotone for the meter's whole lifetime.
/// - provides: one field of the build usage record.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct BuildStepsUsed
{
    /// The step count.
    steps: u64,
}

/// The numeric ceiling reported beside an exceeded limit.
///
/// One widened currency keeps the error's shape independent of which limit was
/// crossed, so a caller reads the kind for meaning and this for the number.
///
/// # Contract
/// - requires: the value is the limit that was crossed, widened without loss.
/// - ensures: the widening is exact for every limit currency in the crate.
/// - provides: the numeric payload of a limit-exceeded error.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LimitBound
{
    /// The widened ceiling.
    bound: u64,
}

impl From<u32> for NestAmount
{
    #[inline]
    fn from(amount: u32) -> Self
    {
        Self { amount }
    }
}

impl From<u32> for ScalarWidth
{
    #[inline]
    fn from(width: u32) -> Self
    {
        Self { width }
    }
}

impl TryFrom<usize> for ScalarWidth
{
    type Error = BuildError;

    #[inline]
    fn try_from(width: usize) -> Result<Self, Self::Error>
    {
        let Ok(width) = u32::try_from(width)
        else {
            return Err(BuildError::ArithmeticOverflow {
                operation: BuildArithmetic::IdConversion,
            });
        };
        Ok(Self { width })
    }
}

impl From<u32> for MaxDocNodes
{
    #[inline]
    fn from(nodes: u32) -> Self
    {
        Self { nodes }
    }
}

impl From<usize> for MaxTextBytes
{
    #[inline]
    fn from(bytes: usize) -> Self
    {
        Self { bytes }
    }
}

impl From<u32> for MaxVerbatimLines
{
    #[inline]
    fn from(lines: u32) -> Self
    {
        Self { lines }
    }
}

impl From<u64> for MaxBuildSteps
{
    #[inline]
    fn from(steps: u64) -> Self
    {
        Self { steps }
    }
}

impl From<ScalarWidth> for LimitBound
{
    #[inline]
    fn from(width: ScalarWidth) -> Self
    {
        Self {
            bound: u64::from(width.width),
        }
    }
}

impl From<NestAmount> for ScalarWidth
{
    #[inline]
    fn from(amount: NestAmount) -> Self
    {
        Self {
            width: amount.amount,
        }
    }
}
impl From<NestAmount> for u32
{
    #[inline]
    fn from(amount: NestAmount) -> Self
    {
        amount.amount
    }
}

impl From<ScalarWidth> for u32
{
    #[inline]
    fn from(width: ScalarWidth) -> Self
    {
        width.width
    }
}

impl From<MaxDocNodes> for u32
{
    #[inline]
    fn from(nodes: MaxDocNodes) -> Self
    {
        nodes.nodes
    }
}

impl From<MaxTextBytes> for usize
{
    #[inline]
    fn from(bytes: MaxTextBytes) -> Self
    {
        bytes.bytes
    }
}

impl From<MaxVerbatimLines> for u32
{
    #[inline]
    fn from(lines: MaxVerbatimLines) -> Self
    {
        lines.lines
    }
}

impl From<MaxBuildSteps> for u64
{
    #[inline]
    fn from(steps: MaxBuildSteps) -> Self
    {
        steps.steps
    }
}

impl From<DocNodesUsed> for u64
{
    #[inline]
    fn from(nodes: DocNodesUsed) -> Self
    {
        nodes.nodes
    }
}

impl From<TextBytesUsed> for u64
{
    #[inline]
    fn from(bytes: TextBytesUsed) -> Self
    {
        bytes.bytes
    }
}

impl From<VerbatimLinesUsed> for u64
{
    #[inline]
    fn from(lines: VerbatimLinesUsed) -> Self
    {
        lines.lines
    }
}

impl From<BuildStepsUsed> for u64
{
    #[inline]
    fn from(steps: BuildStepsUsed) -> Self
    {
        steps.steps
    }
}

impl From<u64> for LimitBound
{
    #[inline]
    fn from(bound: u64) -> Self
    {
        Self { bound }
    }
}

impl From<LimitBound> for u64
{
    #[inline]
    fn from(bound: LimitBound) -> Self
    {
        bound.bound
    }
}
impl From<u64> for DocNodesUsed
{
    #[inline]
    fn from(nodes: u64) -> Self
    {
        Self { nodes }
    }
}

impl From<u64> for TextBytesUsed
{
    #[inline]
    fn from(bytes: u64) -> Self
    {
        Self { bytes }
    }
}

impl From<u64> for VerbatimLinesUsed
{
    #[inline]
    fn from(lines: u64) -> Self
    {
        Self { lines }
    }
}

impl From<u64> for BuildStepsUsed
{
    #[inline]
    fn from(steps: u64) -> Self
    {
        Self { steps }
    }
}
impl TryFrom<usize> for TextBytesUsed
{
    type Error = BuildError;

    #[inline]
    fn try_from(bytes: usize) -> Result<Self, Self::Error>
    {
        let bytes = u64::try_from(bytes).map_err(|_error| BuildError::ArithmeticOverflow {
            operation: BuildArithmetic::TextBytes,
        })?;
        Ok(Self { bytes })
    }
}

impl TryFrom<usize> for VerbatimLinesUsed
{
    type Error = BuildError;

    #[inline]
    fn try_from(lines: usize) -> Result<Self, Self::Error>
    {
        let lines = u64::try_from(lines).map_err(|_error| BuildError::ArithmeticOverflow {
            operation: BuildArithmetic::VerbatimLines,
        })?;
        Ok(Self { lines })
    }
}

impl DocNodesUsed
{
    /// Charges one node against the nominal node ceiling.
    #[inline]
    pub(crate) fn checked_charge(
        self,
        limit: MaxDocNodes,
    ) -> Result<Self, BuildError>
    {
        let next = self
            .nodes
            .checked_add(1u64)
            .ok_or(BuildError::ArithmeticOverflow {
                operation: BuildArithmetic::NodeCount,
            })?;
        if next > u64::from(limit.nodes) {
            return Err(BuildError::LimitExceeded {
                kind: BuildLimitKind::DocNodes,
                limit: LimitBound::from(u64::from(limit.nodes)),
            });
        }
        Ok(Self { nodes: next })
    }
}

impl TextBytesUsed
{
    /// Charges nominal new bytes against the text-byte ceiling.
    #[inline]
    pub(crate) fn checked_charge(
        self,
        amount: Self,
        limit: MaxTextBytes,
    ) -> Result<Self, BuildError>
    {
        let next = self
            .bytes
            .checked_add(amount.bytes)
            .ok_or(BuildError::ArithmeticOverflow {
                operation: BuildArithmetic::TextBytes,
            })?;
        let limit =
            u64::try_from(limit.bytes).map_err(|_error| BuildError::ArithmeticOverflow {
                operation: BuildArithmetic::TextBytes,
            })?;
        if next > limit {
            return Err(BuildError::LimitExceeded {
                kind: BuildLimitKind::TextBytes,
                limit: LimitBound::from(limit),
            });
        }
        Ok(Self { bytes: next })
    }
}

impl VerbatimLinesUsed
{
    /// Charges nominal new fragments against the verbatim-line ceiling.
    #[inline]
    pub(crate) fn checked_charge(
        self,
        amount: Self,
        limit: MaxVerbatimLines,
    ) -> Result<Self, BuildError>
    {
        let next = self
            .lines
            .checked_add(amount.lines)
            .ok_or(BuildError::ArithmeticOverflow {
                operation: BuildArithmetic::VerbatimLines,
            })?;
        if next > u64::from(limit.lines) {
            return Err(BuildError::LimitExceeded {
                kind: BuildLimitKind::VerbatimLines,
                limit: LimitBound::from(u64::from(limit.lines)),
            });
        }
        Ok(Self { lines: next })
    }
}

impl BuildStepsUsed
{
    /// Charges one nominal build step against the step ceiling.
    #[inline]
    pub(crate) fn checked_charge(
        self,
        limit: MaxBuildSteps,
    ) -> Result<Self, BuildError>
    {
        let next = self
            .steps
            .checked_add(1u64)
            .ok_or(BuildError::ArithmeticOverflow {
                operation: BuildArithmetic::BuildSteps,
            })?;
        if next > limit.steps {
            return Err(BuildError::LimitExceeded {
                kind: BuildLimitKind::BuildSteps,
                limit: LimitBound::from(limit.steps),
            });
        }
        Ok(Self { steps: next })
    }
}
