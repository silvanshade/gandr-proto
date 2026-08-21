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

/// The requested page width used by the cost ordering.
///
/// # Contract
/// - requires: the value is a scalar column ceiling.
/// - ensures: the width remains distinct from computation and indentation.
/// - provides: the public page-width currency.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PageWidth(u32);

/// The width within which the optimality theorem is computed.
///
/// # Contract
/// - requires: the value is at least the page width for valid options.
/// - ensures: in-bound resolver contexts are representable.
/// - provides: the public computation-width currency.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ComputationWidth(u32);

/// A current output column.
///
/// # Contract
/// - requires: the value is a checked scalar column.
/// - ensures: column arithmetic remains nominal inside resolution.
/// - provides: the resolver's column currency.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct Column(u32);

/// An indentation column.
///
/// # Contract
/// - requires: the value is a checked indentation.
/// - ensures: indentation cannot be confused with a page width.
/// - provides: the resolver's indentation currency.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub(crate) struct Indentation(u32);

/// Squared overflow accumulated by a layout.
///
/// # Contract
/// - requires: each increment was checked before addition.
/// - ensures: ordering is the lexicographic cost's first component.
/// - provides: the public overflow currency.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct SquaredOverflow(u64);

/// Layout-owned physical line breaks.
///
/// # Contract
/// - requires: every counted ending was emitted by a layout node.
/// - ensures: the count is cumulative and checked.
/// - provides: the public line-break cost component.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LineBreaks(u64);

/// Exact output bytes associated with a resolved plan.
///
/// # Contract
/// - requires: bytes were counted from stored text and endings.
/// - ensures: the count is checked before it is retained.
/// - provides: the public output-size projection.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct OutputBytes(u64);

/// A cumulative memo-state ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxMemoStates(u64);

/// A cumulative frontier-entry ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxFrontierEntries(u64);

/// A cumulative plan-allocation ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxPlanNodesCreated(u64);

/// A simultaneous live-plan ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxLivePlanNodes(u64);

/// A cumulative output-byte ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxOutputBytes(u64);

/// A cumulative layout-step ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxLayoutSteps(u64);

/// A cumulative resolver-work-entry ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxResolverWorkEntries(u64);

/// A simultaneous resolver-stack ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxResolverStack(u64);

/// A cumulative virtual-machine-step ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxVmSteps(u64);

/// A simultaneous virtual-machine-stack ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MaxVmStack(u64);

/// Cumulative memo states used by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct MemoStatesUsed(u64);

/// Cumulative frontier entries used by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FrontierEntriesUsed(u64);

/// Cumulative plan nodes created by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PlanNodesCreated(u64);

/// Peak live plan nodes observed by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PeakLivePlanNodes(u64);

/// Cumulative output bytes used by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct OutputBytesUsed(u64);

/// Cumulative layout steps used by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct LayoutStepsUsed(u64);

/// Cumulative resolver work entries used by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ResolverWorkEntriesUsed(u64);

/// Peak resolver stack observed by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PeakResolverStack(u64);

/// Cumulative virtual-machine steps used by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct VmStepsUsed(u64);

/// Peak virtual-machine stack observed by a render meter.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PeakVmStack(u64);

/// Implements exact `u32` conversions for a transparent currency.
macro_rules! u32_currency {
    ($name:ty) => {
        impl From<u32> for $name
        {
            #[inline]
            fn from(value: u32) -> Self
            {
                Self(value)
            }
        }

        impl From<$name> for u32
        {
            #[inline]
            fn from(value: $name) -> Self
            {
                value.0
            }
        }
    };
}

/// Implements exact `u64` conversions for a transparent currency.
macro_rules! u64_currency {
    ($name:ty) => {
        impl From<u64> for $name
        {
            #[inline]
            fn from(value: u64) -> Self
            {
                Self(value)
            }
        }

        impl From<$name> for u64
        {
            #[inline]
            fn from(value: $name) -> Self
            {
                value.0
            }
        }
    };
}

u32_currency!(PageWidth);
u32_currency!(ComputationWidth);
u32_currency!(Column);
u32_currency!(Indentation);
u64_currency!(SquaredOverflow);
u64_currency!(LineBreaks);
u64_currency!(OutputBytes);
u64_currency!(MaxMemoStates);
u64_currency!(MaxFrontierEntries);
u64_currency!(MaxPlanNodesCreated);
u64_currency!(MaxLivePlanNodes);
u64_currency!(MaxOutputBytes);
u64_currency!(MaxLayoutSteps);
u64_currency!(MaxResolverWorkEntries);
u64_currency!(MaxResolverStack);
u64_currency!(MaxVmSteps);
u64_currency!(MaxVmStack);
u64_currency!(MemoStatesUsed);
u64_currency!(FrontierEntriesUsed);
u64_currency!(PlanNodesCreated);
u64_currency!(PeakLivePlanNodes);
u64_currency!(OutputBytesUsed);
u64_currency!(LayoutStepsUsed);
u64_currency!(ResolverWorkEntriesUsed);
u64_currency!(PeakResolverStack);
u64_currency!(VmStepsUsed);
u64_currency!(PeakVmStack);

impl From<ScalarWidth> for Column
{
    #[inline]
    fn from(width: ScalarWidth) -> Self
    {
        Self(u32::from(width))
    }
}

impl From<NestAmount> for Indentation
{
    #[inline]
    fn from(amount: NestAmount) -> Self
    {
        Self(u32::from(amount))
    }
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

    /// Converts a platform width into the checked scalar-width currency.
    ///
    /// # Contract
    /// - requires: `width` is the source scalar count.
    /// - ensures: success preserves the count exactly.
    /// - provides: the narrowing conversion used by text ingestion.
    /// - fails: returns `ArithmeticOverflow` when the count exceeds `u32`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for an unrepresentable scalar count.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — accepted text widths remain exact at the nominal
    ///   conversion boundary.
    /// - witness: `algebra::text_emits_at_the_current_column`.
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

    /// Converts a platform byte count into nominal text-byte usage.
    ///
    /// # Contract
    /// - requires: `bytes` is the complete stored-byte count.
    /// - ensures: success preserves the count exactly.
    /// - provides: the usage currency for text-byte accounting.
    /// - fails: returns `ArithmeticOverflow` when the count is not
    ///   representable.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for an unrepresentable byte count.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — exact byte boundaries accept the final byte and
    ///   reject the next charge.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::a_second_edge_to_a_shared_handle_charges_no_new_text_bytes`.
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

    /// Converts a platform fragment count into nominal verbatim-line usage.
    ///
    /// # Contract
    /// - requires: `lines` is the complete scan count for one verbatim value.
    /// - ensures: success preserves the count exactly.
    /// - provides: the usage currency for physical-fragment accounting.
    /// - fails: returns `ArithmeticOverflow` when the count is not
    ///   representable.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for an unrepresentable fragment count.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — the exact fragment boundary accepts the final record
    ///   and rejects the next one.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::verbatim_with_a_trailing_ending_stores_an_empty_final_fragment`.
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
    ///
    /// # Contract
    /// - requires: the current node count is charged against `limit`.
    /// - ensures: success increments usage exactly once.
    /// - provides: node accounting for original and flattened images.
    /// - fails: returns `ArithmeticOverflow` or `LimitExceeded` without
    ///   changing the current usage.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for counter overflow or `LimitExceeded` at
    /// the configured node ceiling.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the exact node ceiling accepts its final node and
    ///   refuses one additional charge.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::a_second_edge_to_a_shared_handle_charges_no_new_node`.
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
    ///
    /// # Contract
    /// - requires: `amount` is the new stored-byte count.
    /// - ensures: success increments usage exactly by `amount`.
    /// - provides: text-byte accounting for unique identities.
    /// - fails: returns `ArithmeticOverflow` or `LimitExceeded` without
    ///   changing the current usage.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for counter or limit conversion overflow,
    /// or `LimitExceeded` at the configured byte ceiling.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — shared identities charge once and the exact byte
    ///   boundary rejects only the next charge.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::a_second_edge_to_a_shared_handle_charges_no_new_text_bytes`.
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
    ///
    /// # Contract
    /// - requires: `amount` is the complete new physical-fragment count.
    /// - ensures: success increments usage exactly by `amount`.
    /// - provides: verbatim-line accounting for opaque content.
    /// - fails: returns `ArithmeticOverflow` or `LimitExceeded` without
    ///   changing the current usage.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for counter overflow or `LimitExceeded` at
    /// the configured fragment ceiling.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the trailing empty fragment is charged and the exact
    ///   fragment boundary refuses only the next charge.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::verbatim_with_a_trailing_ending_stores_an_empty_final_fragment`.
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
    ///
    /// # Contract
    /// - requires: the caller has identified one checked operation.
    /// - ensures: success increments usage exactly once.
    /// - provides: the work budget for construction and finalization.
    /// - fails: returns `ArithmeticOverflow` or `LimitExceeded` without
    ///   changing the current usage.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for counter overflow or `LimitExceeded` at
    /// the configured step ceiling.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — finalization steps are charged and the exact step
    ///   boundary refuses only the next step.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::every_finalization_visit_edge_and_probe_charges_a_build_step`.
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
