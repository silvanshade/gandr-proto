//! The work counters, and the typed failures the harness reports.
//!
//! # Why the counters exist
//!
//! **A differential whose inputs never reach the code under test is green and
//! worthless, and only a work count catches that.** Timing alone cannot: a path
//! that silently skipped the work is fast. So every measured recheck carries a
//! count of what actually ran, and the runner asserts those counts against the
//! workload's known structure rather than reporting them as telemetry for a
//! reader to eyeball.
//!
//! The counters are separated per query rather than totalled, because the
//! engine's whole claim lives in the *difference* between them: on a value-only
//! edit the binding query must re-execute (the edited item really did change)
//! while the typing queries of its readers must not (their inputs recomputed
//! equal). One total hides exactly that.

use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;

use gandr_core_incremental::persistence::CheckpointStoreError;

use crate::boundary::BoundaryByteCount;
use crate::boundary::ExecutionCount;
use crate::boundary::SlotIndex;
use crate::boundary::ValidationCount;

/// A failure the assessment harness reports rather than absorbing.
///
/// Every variant names a broken invariant of the harness itself, not a typing
/// outcome: an ill-typed item is an ordinary result and never appears here.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssessmentError
{
    /// A value could not cross the engine's ownership boundary, because the
    /// checkpoint codec declined to encode or decode it.
    Boundary(CheckpointStoreError),
    /// A query asked the item store for a slot the installed program does not
    /// have.
    MissingSlot(SlotIndex),
    /// An item's recomputed content address disagreed with the digest its query
    /// input carries — the item store and the database have diverged, so every
    /// downstream measurement would be meaningless.
    DigestMismatch(SlotIndex),
    /// The item store was already borrowed when a query tried to read it.
    StoreUnavailable,
    /// A decoded binding did not carry the definition shape it was encoded
    /// from.
    MalformedBinding,
}

impl From<CheckpointStoreError> for AssessmentError
{
    #[inline]
    fn from(error: CheckpointStoreError) -> Self
    {
        Self::Boundary(error)
    }
}

/// The mutable work counters one measured run accumulates.
///
/// Shared by the database and its event callback, so every field is an atomic
/// rather than a cell: the engine requires its callback to be [`Send`] and
/// [`Sync`], and a counter that cannot be shared cannot be read from both.
#[derive(Debug, Default)]
pub struct Ledger
{
    /// Bodies of the item-typing query that ran.
    typing_executions: AtomicUsize,
    /// Bodies of the binding query that ran — the firewall whose recomputation
    /// is expected even when nothing downstream re-executes.
    binding_executions: AtomicUsize,
    /// Bodies of the footprint query that ran.
    footprint_executions: AtomicUsize,
    /// Bodies of the unfolding query that ran — the second firewall, whose
    /// invalidation condition differs from the binding's.
    unfolding_executions: AtomicUsize,
    /// Bodies of the name-table query that ran.
    name_table_executions: AtomicUsize,
    /// Memoized values reused after their dependencies verified unchanged.
    memo_validations: AtomicUsize,
    /// Bytes encoded or decoded at the ownership boundary.
    boundary_bytes: AtomicUsize,
    /// Fixpoint iterations the engine ran over a query cycle.
    cycle_iterations: AtomicUsize,
}

impl Ledger
{
    /// Creates a ledger with every counter at zero.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Resets every counter, so one database can be measured across successive
    /// revisions without a run's counts leaking into the next.
    ///
    /// # Contract
    /// - ensures: every counter reads zero afterwards.
    /// - panics: none.
    #[inline]
    pub fn reset(&self)
    {
        self.typing_executions.store(0, Ordering::Relaxed);
        self.binding_executions.store(0, Ordering::Relaxed);
        self.footprint_executions.store(0, Ordering::Relaxed);
        self.unfolding_executions.store(0, Ordering::Relaxed);
        self.name_table_executions.store(0, Ordering::Relaxed);
        self.memo_validations.store(0, Ordering::Relaxed);
        self.boundary_bytes.store(0, Ordering::Relaxed);
        self.cycle_iterations.store(0, Ordering::Relaxed);
    }

    /// Records one execution of the item-typing query body.
    #[inline]
    pub fn record_typing_execution(&self)
    {
        let _previous = self.typing_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one execution of the binding query body.
    #[inline]
    pub fn record_binding_execution(&self)
    {
        let _previous = self.binding_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one execution of the footprint query body.
    #[inline]
    pub fn record_footprint_execution(&self)
    {
        let _previous = self.footprint_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one execution of the unfolding query body.
    #[inline]
    pub fn record_unfolding_execution(&self)
    {
        let _previous = self.unfolding_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one execution of the name-table query body.
    #[inline]
    pub fn record_name_table_execution(&self)
    {
        let _previous = self.name_table_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one memoized value reused without re-execution.
    #[inline]
    pub fn record_memo_validation(&self)
    {
        let _previous = self.memo_validations.fetch_add(1, Ordering::Relaxed);
    }

    /// Records one fixpoint iteration over a query cycle.
    #[inline]
    pub fn record_cycle_iteration(&self)
    {
        let _previous = self.cycle_iterations.fetch_add(1, Ordering::Relaxed);
    }

    /// Records `bytes` crossing the ownership boundary in either direction.
    ///
    /// # Contract
    /// - ensures: the boundary total grows by `bytes`, saturating rather than
    ///   overflowing.
    /// - panics: none.
    #[inline]
    pub fn record_boundary_bytes(
        &self,
        bytes: BoundaryByteCount,
    )
    {
        let bytes: usize = bytes.into();
        let _previous = self.boundary_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Takes a consistent-enough reading of every counter.
    ///
    /// # Contract
    /// - requires: no query is running concurrently — the harness measures
    ///   single-threaded, so the per-field reads cannot straddle an update.
    /// - ensures: returns each counter's current value.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn snapshot(&self) -> LedgerSnapshot
    {
        LedgerSnapshot {
            typing_executions: ExecutionCount::from(self.typing_executions.load(Ordering::Relaxed)),
            binding_executions: ExecutionCount::from(
                self.binding_executions.load(Ordering::Relaxed),
            ),
            footprint_executions: ExecutionCount::from(
                self.footprint_executions.load(Ordering::Relaxed),
            ),
            unfolding_executions: ExecutionCount::from(
                self.unfolding_executions.load(Ordering::Relaxed),
            ),
            name_table_executions: ExecutionCount::from(
                self.name_table_executions.load(Ordering::Relaxed),
            ),
            memo_validations: ValidationCount::from(self.memo_validations.load(Ordering::Relaxed)),
            boundary_bytes: BoundaryByteCount::from(self.boundary_bytes.load(Ordering::Relaxed)),
            cycle_iterations: ExecutionCount::from(self.cycle_iterations.load(Ordering::Relaxed)),
        }
    }
}

/// One reading of the work counters.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LedgerSnapshot
{
    /// Bodies of the item-typing query that ran.
    pub typing_executions: ExecutionCount,
    /// Bodies of the binding query that ran.
    pub binding_executions: ExecutionCount,
    /// Bodies of the footprint query that ran.
    pub footprint_executions: ExecutionCount,
    /// Bodies of the unfolding query that ran.
    pub unfolding_executions: ExecutionCount,
    /// Bodies of the name-table query that ran.
    pub name_table_executions: ExecutionCount,
    /// Memoized values reused without re-execution.
    pub memo_validations: ValidationCount,
    /// Bytes encoded or decoded at the ownership boundary.
    pub boundary_bytes: BoundaryByteCount,
    /// Fixpoint iterations over a query cycle.
    pub cycle_iterations: ExecutionCount,
}
