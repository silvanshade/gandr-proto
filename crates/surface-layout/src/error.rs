//! The build-phase error vocabulary.
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
//! The render-phase vocabulary is a separate closed space and arrives with
//! slice three; build accounting and render accounting never share a counter,
//! and a build failure can never consume a render budget.

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
