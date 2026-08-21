//! Build- and render-phase error vocabulary.
//!
//! Construction and finalization fail in exactly the ways enumerated here, and
//! every one of them surfaces as a value. Nothing on a production path panics,
//! and a failure never leaves partial state behind: a builder that exceeds a
//! limit stays unfinalized and yields no arena.
//!
//! The three classification enums are deliberately closed. A caller switching
//! on a kind, a site, or an operation is reading the whole space, so a new
//! failure mode is a deliberate change here rather than a silent widening.
//!
//! The render-phase vocabulary is also closed. Resolution and the later
//! machine use separate render counters, and a build failure can never consume
//! a render budget.

use crate::units::LimitBound;

/// Which build limit was crossed.
///
/// # Contract
/// - requires: the value names the limit whose ceiling the builder reached.
/// - ensures: the set is exactly the four build limits and never widens
///   silently.
/// - provides: the machine-readable half of a limit-exceeded build error.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BuildLimitKind
{
    /// Stored document nodes, flatten images included.
    DocNodes,
    /// Uniquely stored text and verbatim bytes.
    TextBytes,
    /// Stored verbatim physical fragments.
    VerbatimLines,
    /// Constructor and finalization steps.
    BuildSteps,
}

/// Which store failed to grow.
///
/// Every named store checks its limit, then reserves fallibly, so an allocation
/// failure is attributable to one site rather than to the process.
///
/// # Contract
/// - requires: the value names the store whose fallible reservation failed.
/// - ensures: the set is exactly the five build-phase stores.
/// - provides: the machine-readable half of an allocation-failure build error.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BuildAllocationSite
{
    /// The document node arena.
    NodeArena,
    /// The text arena.
    TextArena,
    /// The verbatim arena.
    VerbatimArena,
    /// The structural interner backing the flatten pass.
    FlattenMemo,
    /// The explicit work stack finalization runs on.
    FinalizeStack,
}

/// Which checked build-phase arithmetic overflowed.
///
/// # Contract
/// - requires: the value names the operation whose checked step returned no
///   result.
/// - ensures: the set is exactly the six build-phase arithmetic sites.
/// - provides: the machine-readable half of an arithmetic-overflow build error.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BuildArithmetic
{
    /// Incrementing the stored node count.
    NodeCount,
    /// Accumulating stored text bytes.
    TextBytes,
    /// Accumulating stored verbatim fragments.
    VerbatimLines,
    /// Incrementing the build-step counter.
    BuildSteps,
    /// Narrowing an insertion position into a dense identity.
    IdConversion,
    /// Adding a nesting amount to a current indentation.
    NestAmount,
}

/// Which render limit was crossed.
///
/// # Contract
/// - requires: the value names the render counter whose ceiling was reached.
/// - ensures: every resolution budget has one closed machine-readable kind.
/// - provides: the limit classification in [`RenderError`].
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RenderLimitKind
{
    /// Number of memoized in-bound states.
    MemoStates,
    /// Number of retained frontier entries.
    FrontierEntries,
    /// Number of plan nodes ever created.
    PlanNodesCreated,
    /// Number of simultaneously live plan nodes.
    LivePlanNodes,
    /// Number of output bytes accounted for.
    OutputBytes,
    /// Number of layout transitions and comparisons.
    LayoutSteps,
    /// Number of resolver work entries pushed.
    ResolverWorkEntries,
    /// Peak resolver work-vector length.
    ResolverStack,
    /// Number of virtual-machine instructions.
    VmSteps,
    /// Peak virtual-machine stack length.
    VmStack,
}

/// Which render store or work stack failed to reserve.
///
/// # Contract
/// - requires: the value identifies the allocation site that refused growth.
/// - ensures: no unmetered render allocation is reported generically.
/// - provides: the allocation classification in [`RenderError`].
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RenderAllocationSite
{
    /// The in-bound memo table.
    MemoTable,
    /// A retained frontier.
    Frontier,
    /// The generational plan arena.
    PlanArena,
    /// The resolver's explicit work vector.
    ResolverStack,
    /// The virtual-machine stack.
    VmStack,
    /// The final output buffer.
    Output,
}

/// Which checked render arithmetic operation overflowed.
///
/// # Contract
/// - requires: the operation is the exact failed checked step.
/// - ensures: arithmetic failures remain distinguishable from limits.
/// - provides: the arithmetic classification in [`RenderError`].
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RenderArithmetic
{
    /// Advancing a current column.
    Column,
    /// Advancing indentation.
    Indentation,
    /// Squaring or adding overflow cost.
    SquaredOverflow,
    /// Adding a line break.
    LineBreaks,
    /// Adding output bytes.
    OutputBytes,
    /// Incrementing the layout-step counter.
    StepCounter,
    /// Incrementing resolver work entries.
    ResolverWorkCounter,
    /// Incrementing a plan reference count.
    PlanRefcount,
}

/// Why memoized layout resolution refused.
///
/// # Contract
/// - requires: the error came from a checked render operation.
/// - ensures: resolution returns a typed failure without partial output.
/// - provides: the closed render-phase error space.
/// - panics: none.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum RenderError
{
    /// A document handle does not belong to the supplied arena.
    #[error("the document handle does not belong to this layout arena")]
    UnknownDoc,
    /// The computation width is smaller than the page width.
    #[error("the computation width must be at least the page width")]
    InvalidWidth,
    /// A checked render arithmetic operation overflowed.
    #[error("a checked layout render computation overflowed: {operation:?}")]
    ArithmeticOverflow
    {
        /// The operation whose checked step returned no result.
        operation: RenderArithmetic,
    },
    /// A render store or work stack could not reserve capacity.
    #[error("a layout render store could not reserve capacity: {site:?}")]
    AllocationFailed
    {
        /// The store whose reservation failed.
        site: RenderAllocationSite,
    },
    /// A named render ceiling was reached.
    #[error("a layout render limit was reached: {kind:?}")]
    LimitExceeded
    {
        /// The limit whose ceiling was reached.
        kind: RenderLimitKind,
        /// The configured ceiling.
        limit: LimitBound,
    },
}

/// Why document construction or finalization refused.
///
/// # Contract
/// - requires: the value is produced by a builder operation that refused.
/// - ensures: the builder is left unfinalized and no partial arena escapes.
/// - provides: the closed failure space of the build phase.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BuildError
{
    /// The process-local arena-key counter has no value left to mint.
    #[error("the layout arena key counter is exhausted")]
    ArenaKeyExhausted,
    /// The dense node identity space is full.
    #[error("the layout document node identity space is exhausted")]
    NodeIdExhausted,
    /// A handle came from another arena, or names no node in this one.
    #[error("the document handle does not belong to this layout arena")]
    UnknownDoc,
    /// Text carried a carriage return, a line feed, or a tab.
    #[error("layout text must not contain a carriage return, a line feed, or a tab")]
    InvalidText,
    /// Verbatim text carried a bare carriage return.
    #[error("verbatim text must not contain a bare carriage return")]
    InvalidVerbatimLineEnding,
    /// A checked build-phase arithmetic step overflowed.
    #[error("a checked layout build computation overflowed: {operation:?}")]
    ArithmeticOverflow
    {
        /// The operation whose checked step returned no result.
        operation: BuildArithmetic,
    },
    /// A named store could not reserve the capacity it needed.
    #[error("a layout build store could not reserve capacity: {site:?}")]
    AllocationFailed
    {
        /// The store whose fallible reservation failed.
        site: BuildAllocationSite,
    },
    /// A build limit was reached.
    #[error("a layout build limit was reached: {kind:?}")]
    LimitExceeded
    {
        /// The limit whose ceiling was reached.
        kind: BuildLimitKind,
        /// That ceiling, widened without loss.
        limit: LimitBound,
    },
}
