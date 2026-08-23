//! The runner that produces the comparison rows, and the table they are
//! reported as.
//!
//! # The two demand shapes, and why both are here
//!
//! **The recheck-traversal floor is a property of the demand, not of the
//! engine.** Asked for every item's typing, any path must at minimum touch
//! every item's cached answer, and the engine's advantage narrows to how cheap
//! a touch is. Asked for one item's typing, a demand-driven path computes the
//! dirty cone and stops — and the hand-rolled path cannot answer at all,
//! because its signature produces the whole checkpoint set or nothing.
//!
//! Reporting only the first shape prices the engine on the workload least able
//! to show what it buys. Reporting only the second compares against a
//! capability the baseline never claimed. So both are measured, and the table
//! says which is which.
//!
//! # What is compared, and against what
//!
//! Both paths' answers are compared against **from-scratch typing**, never
//! against each other. A rule-level defect is invisible to a comparison between
//! two implementations of the same rule; only incremental-equals-batch sees it.
//!
//! # Reading the numbers
//!
//! The counts are profile-independent and are the substance. Wall time is
//! reported beside them and is not: it depends on the build profile, and the
//! suite runs in the development profile. A claim resting on the counts is
//! reproducible; a claim resting on the timings names the profile it was taken
//! under.

use std::time::Instant;

use gandr_core_incremental::checkpoint::ItemTyping;

use crate::baseline::BaselineSession;
use crate::baseline::from_scratch;
use crate::baseline::rescan_footprints;
use crate::boundary::BoundaryByteCount;
use crate::boundary::ElapsedNanos;
use crate::boundary::ItemCount;
use crate::boundary::RetainedByteCount;
use crate::boundary::RowLabel;
use crate::boundary::SlotIndex;
use crate::boundary::ValidationCount;
use crate::engine::EngineSession;
use crate::ledger::AssessmentError;
use crate::ledger::LedgerSnapshot;
use crate::workload::EditKind;
use crate::workload::Workload;
use crate::workload::apply_edit;

/// Which path a measured row belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathKind
{
    /// The hand-rolled validated resume as built.
    Baseline,
    /// The memoized query graph.
    Engine,
}

impl PathKind
{
    /// The path's name, for the reported table.
    #[inline]
    #[must_use]
    pub fn label(self) -> RowLabel
    {
        RowLabel::from(match self {
            | Self::Baseline => "baseline",
            | Self::Engine => "engine",
        })
    }
}

/// What a measured recheck was asked for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DemandShape
{
    /// Every item's typing — what the hand-rolled path's signature produces.
    AllItems,
    /// One item's typing — what the hand-rolled path cannot express.
    SingleItem,
}

impl DemandShape
{
    /// The demand's name, for the reported table.
    #[inline]
    #[must_use]
    pub fn label(self) -> RowLabel
    {
        RowLabel::from(match self {
            | Self::AllItems => "all items",
            | Self::SingleItem => "one item",
        })
    }
}

/// One measured recheck.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecheckRow
{
    /// Which path this row measures.
    pub path: PathKind,
    /// What the recheck was asked for.
    pub demand: DemandShape,
    /// Items whose typing was actually recomputed.
    pub items_retyped: ItemCount,
    /// Items reached and classified — the traversal floor.
    pub items_visited: ItemCount,
    /// Items whose dependency footprint was rescanned.
    pub footprint_scans: ItemCount,
    /// Cached answers reused without recomputation.
    pub memo_reuses: ValidationCount,
    /// Bytes encoded or decoded at the ownership boundary.
    pub boundary_bytes: BoundaryByteCount,
    /// Wall-clock time for the recheck.
    pub elapsed: ElapsedNanos,
}

/// One workload's complete comparison.
#[derive(Clone, Debug)]
pub struct Comparison
{
    /// The workload measured.
    pub workload: Workload,
    /// The edit applied.
    pub edit: EditKind,
    /// The slot the edit was applied at.
    pub edited_slot: SlotIndex,
    /// The slot the single-item demand asked about — the deepest reader of the
    /// edited chain, so the answer genuinely depends on the edit.
    pub demanded_slot: SlotIndex,
    /// The measured rows.
    pub rows: Vec<RecheckRow>,
    /// The engine's per-query work counts under the all-items demand.
    pub engine_all_counts: LedgerSnapshot,
    /// The engine's per-query work counts under the single-item demand.
    pub engine_single_counts: LedgerSnapshot,
    /// From-scratch typings for the edited program — the correctness reference.
    pub reference_typings: Vec<ItemTyping>,
    /// The baseline path's typings after its recheck.
    pub baseline_typings: Vec<ItemTyping>,
    /// The engine path's typings after its recheck.
    pub engine_typings: Vec<ItemTyping>,
    /// The engine's answer under the single-item demand.
    pub engine_single_typing: ItemTyping,
    /// Retained state, taken with one instrument on both paths: the canonical
    /// encoding's size.
    pub baseline_state_bytes: BoundaryByteCount,
    /// Retained state for the engine path, same instrument.
    pub engine_state_bytes: BoundaryByteCount,
    /// What the engine reports retained in its own tables — a second
    /// instrument, engine-only, excluding everything held outside the database.
    pub engine_table_bytes: RetainedByteCount,
    /// Time to rescan every item's footprint, measured on its own.
    pub footprint_rescan_elapsed: ElapsedNanos,
    /// The engine's per-ingredient retention breakdown, which is how its
    /// aggregate figure is meant to be read.
    pub engine_memory_report: String,
}

impl Comparison
{
    /// Renders the comparison as the table the assessment reports.
    ///
    /// # Contract
    /// - ensures: returns one line per measured row, plus the retained-state
    ///   and rescan lines, with every number taken from this comparison rather
    ///   than restated.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn render(&self) -> String
    {
        let items: usize = self.workload.item_count().into();
        let block: usize = self.workload.block_length().into();
        let baseline_state: usize = self.baseline_state_bytes.into();
        let engine_state: usize = self.engine_state_bytes.into();
        let table_bytes: usize = self.engine_table_bytes.into();
        let rescan_nanos: u128 = self.footprint_rescan_elapsed.into();
        let rescan_micros = rescan_nanos.checked_div(1000).unwrap_or(rescan_nanos);

        let mut lines: Vec<String> = Vec::new();
        lines.push(format!(
            "workload: {items} items in chains of {block}; edit: {:?}",
            self.edit
        ));
        lines.push(
            "path      demand      retyped  visited  scans  reuses  boundary_bytes  micros"
                .to_owned(),
        );
        for row in &self.rows {
            let retyped: usize = row.items_retyped.into();
            let visited: usize = row.items_visited.into();
            let scans: usize = row.footprint_scans.into();
            let reuses: usize = row.memo_reuses.into();
            let bytes: usize = row.boundary_bytes.into();
            let nanos: u128 = row.elapsed.into();
            let micros = nanos.checked_div(1000).unwrap_or(nanos);
            lines.push(format!(
                "{:<9} {:<11} {retyped:>7}  {visited:>7}  {scans:>5}  {reuses:>6}  {bytes:>14}  {micros:>6}",
                row.path.label(),
                row.demand.label()
            ));
        }
        lines.push(format!(
            "retained state (canonical encoding): baseline {baseline_state} B, engine {engine_state} B"
        ));
        lines.push(format!(
            "engine-reported table bytes (excludes the out-of-database item store): {table_bytes} B"
        ));
        lines.push(format!(
            "footprint rescan of every item, measured alone: {rescan_micros} micros"
        ));
        lines.join("\n")
    }
}

/// Measures one workload under one edit, on both paths and both demand shapes.
///
/// # Contract
/// - requires: the workload generates at least one chain, and its definition
///   names are distinct — a program with a repeated name would put a mutual
///   dependency into the query graph, which is out of this measurement's scope.
/// - ensures: returns every measured row together with the typings needed to
///   check both paths against from-scratch typing.
/// - fails: returns the store or codec failure that prevented a measurement, or
///   [`AssessmentError::MissingSlot`] when the workload has no chain head.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: the measurement must exercise the general resume path and a
///   genuinely dirty engine cone; a run that reached the append fast path, or
///   demanded an item the edit cannot affect, would report large reuse for the
///   wrong reason.
/// - witness: `differential::both_paths_agree_with_from_scratch`
/// - witness: `floor::engine_visits_less_than_the_program_under_single_demand`
///
/// # Errors
///
/// Returns the store or codec failure that prevented a measurement, or
/// [`AssessmentError::MissingSlot`] when the workload has no chain head.
#[inline]
pub fn run(
    workload: Workload,
    edit: EditKind,
) -> Result<Comparison, AssessmentError>
{
    let program = workload.program();
    let item_count: usize = workload.item_count().into();
    let edited_slot = workload
        .middle_block_head()
        .ok_or_else(|| AssessmentError::MissingSlot(SlotIndex::from(0_usize)))?;
    let edited =
        apply_edit(&program, edited_slot, edit).ok_or(AssessmentError::MissingSlot(edited_slot))?;

    let block_length: usize = workload.block_length().into();
    let edited_index: usize = edited_slot.into();
    let demanded_index = edited_index
        .checked_add(block_length)
        .and_then(|end| end.checked_sub(1))
        .ok_or(AssessmentError::MissingSlot(edited_slot))?;
    let demanded_slot = SlotIndex::from(demanded_index);

    let reference_typings = from_scratch(&edited);

    // The baseline path.
    let mut baseline = BaselineSession::install(&program);
    let started = Instant::now();
    let resumed = baseline.recheck(&edited);
    let baseline_elapsed = ElapsedNanos::from(started.elapsed());
    let visited = resumed.adopted().len();
    let adopted: usize = usize::from(resumed.adopted_count());
    let retyped = item_count.saturating_sub(adopted);
    baseline.adopt(resumed);
    let baseline_typings = baseline.typings();
    let baseline_state_bytes = baseline.encoded_state_size()?;

    let started = Instant::now();
    let rescanned = rescan_footprints(&edited);
    let footprint_rescan_elapsed = ElapsedNanos::from(started.elapsed());

    let baseline_row = RecheckRow {
        path: PathKind::Baseline,
        demand: DemandShape::AllItems,
        items_retyped: ItemCount::from(retyped),
        items_visited: ItemCount::from(visited),
        footprint_scans: rescanned,
        memo_reuses: ValidationCount::from(adopted),
        boundary_bytes: BoundaryByteCount::from(0_usize),
        elapsed: baseline_elapsed,
    };

    // The engine path, all-items demand.
    let mut engine = EngineSession::install(&program)?;
    let _warmed = engine.typings()?;
    engine.ledger().reset();
    engine.apply(&edited)?;
    let started = Instant::now();
    let engine_typings = engine.typings()?;
    let engine_elapsed = ElapsedNanos::from(started.elapsed());
    let engine_all_counts = engine.ledger().snapshot();
    let engine_state_bytes = engine.encoded_state_size()?;
    let engine_table_bytes = engine.retained_bytes();
    let engine_memory_report = engine.memory_report();

    let engine_all_row = RecheckRow {
        path: PathKind::Engine,
        demand: DemandShape::AllItems,
        items_retyped: ItemCount::from(usize::from(engine_all_counts.typing_executions)),
        items_visited: ItemCount::from(item_count),
        footprint_scans: ItemCount::from(usize::from(engine_all_counts.footprint_executions)),
        memo_reuses: engine_all_counts.memo_validations,
        boundary_bytes: engine_all_counts.boundary_bytes,
        elapsed: engine_elapsed,
    };

    // The engine path, single-item demand, from an identically warmed database
    // so the two demands differ only in what was asked.
    let mut single = EngineSession::install(&program)?;
    let _warmed = single.typings()?;
    single.ledger().reset();
    single.apply(&edited)?;
    let started = Instant::now();
    let engine_single_typing = single.typing_at(demanded_slot)?;
    let single_elapsed = ElapsedNanos::from(started.elapsed());
    let engine_single_counts = single.ledger().snapshot();

    let engine_single_row = RecheckRow {
        path: PathKind::Engine,
        demand: DemandShape::SingleItem,
        items_retyped: ItemCount::from(usize::from(engine_single_counts.typing_executions)),
        items_visited: ItemCount::from(usize::from(engine_single_counts.typing_executions)),
        footprint_scans: ItemCount::from(usize::from(engine_single_counts.footprint_executions)),
        memo_reuses: engine_single_counts.memo_validations,
        boundary_bytes: engine_single_counts.boundary_bytes,
        elapsed: single_elapsed,
    };

    Ok(Comparison {
        workload,
        edit,
        edited_slot,
        demanded_slot,
        rows: vec![baseline_row, engine_all_row, engine_single_row],
        engine_all_counts,
        engine_single_counts,
        reference_typings,
        baseline_typings,
        engine_typings,
        engine_single_typing,
        baseline_state_bytes,
        engine_state_bytes,
        engine_table_bytes,
        footprint_rescan_elapsed,
        engine_memory_report,
    })
}
