//! The traversal floor, asserted against the workload's known structure.
//!
//! # The claim under test
//!
//! **The recheck-traversal floor is a property of the demand, not of the
//! engine.** Asked for every item's typing, both paths must reach every item's
//! answer, and what separates them is how expensive reaching one is. Asked for
//! one item's typing, the engine computes the dirty cone and stops — and the
//! hand-rolled path has no such mode, because its signature produces the whole
//! checkpoint set or nothing.
//!
//! Every number below is asserted against a quantity derived from the
//! workload's construction, never against a number read off a previous run. A
//! measurement that agreed with its own last output would be a measurement of
//! nothing.

use gandr_core_incremental_assessment::boundary::ItemCount;
use gandr_core_incremental_assessment::measure;
use gandr_core_incremental_assessment::measure::DemandShape;
use gandr_core_incremental_assessment::measure::PathKind;
use gandr_core_incremental_assessment::workload::EditKind;

use crate::support;

#[test]
fn the_baseline_visits_every_item_whatever_the_edit()
{
    let workload = support::thousand_items();
    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(workload, edit).expect("the run completes");
        let row = comparison
            .rows
            .iter()
            .find(|row| row.path == PathKind::Baseline)
            .expect("the baseline is measured");
        assert_eq!(
            row.items_visited,
            workload.item_count(),
            "the resume classifies every item of the edited program under {edit:?}, which is the floor"
        );
        assert_eq!(
            row.footprint_scans,
            workload.item_count(),
            "and rescans every item's footprint under {edit:?}"
        );
    }
}

#[test]
fn the_baseline_retypes_only_the_dirty_set()
{
    // The baseline's reuse is real, and saying so is part of an honest
    // comparison: what it does not have is a way to find the dirty set without
    // walking the program.
    let workload = support::thousand_items();
    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(workload, edit).expect("the run completes");
        let row = comparison
            .rows
            .iter()
            .find(|row| row.path == PathKind::Baseline)
            .expect("the baseline is measured");
        assert_eq!(
            row.items_retyped,
            workload.dirty_items(edit),
            "the resume re-types exactly the dirty set under {edit:?}"
        );
    }
}

#[test]
fn the_engine_retypes_only_the_dirty_set()
{
    let workload = support::thousand_items();
    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(workload, edit).expect("the run completes");
        let row = comparison
            .rows
            .iter()
            .find(|row| row.path == PathKind::Engine && row.demand == DemandShape::AllItems)
            .expect("the engine is measured under the all-items demand");
        assert_eq!(
            row.items_retyped,
            workload.dirty_items(edit),
            "the engine re-types exactly the dirty set under {edit:?}"
        );
    }
}

#[test]
fn the_engine_rescans_only_the_edited_item()
{
    // The sharpest of the count comparisons, and the one that does not depend
    // on the demand shape: a footprint is a function of an item's own content,
    // so exactly one item's footprint moved, whatever the edit did downstream.
    let workload = support::thousand_items();
    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(workload, edit).expect("the run completes");
        let row = comparison
            .rows
            .iter()
            .find(|row| row.path == PathKind::Engine && row.demand == DemandShape::AllItems)
            .expect("the engine is measured under the all-items demand");
        assert_eq!(
            row.footprint_scans,
            ItemCount::from(1_usize),
            "one item's content changed, so one footprint is recomputed under {edit:?} — against the whole program on the baseline"
        );
    }
}

#[test]
fn the_single_item_demand_touches_far_less_than_the_program()
{
    // The capability the baseline does not have, measured: answering one
    // question costs the dirty cone rather than the program.
    let workload = support::thousand_items();
    let items: usize = workload.item_count().into();
    let block: usize = workload.block_length().into();

    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(workload, edit).expect("the run completes");
        let all: usize = comparison.engine_all_counts.memo_validations.into();
        let single: usize = comparison.engine_single_counts.memo_validations.into();

        assert!(
            all >= items,
            "the all-items demand verifies at least one memo per item under {edit:?}: {all} against {items}"
        );
        assert!(
            single < items,
            "the one-item demand verifies fewer memos than the program has items under {edit:?}: {single} against {items}"
        );
        assert!(
            single <= block.saturating_mul(8),
            "and stays within a small multiple of the dirty chain under {edit:?}: {single} against a chain of {block}"
        );
    }
}

#[test]
fn engine_visits_less_than_the_program_under_single_demand()
{
    let workload = support::thousand_items();
    let comparison = measure::run(workload, EditKind::ValueOnly).expect("the run completes");
    let row = comparison
        .rows
        .iter()
        .find(|row| row.path == PathKind::Engine && row.demand == DemandShape::SingleItem)
        .expect("the engine is measured under the single-item demand");
    assert!(
        row.items_visited < workload.item_count(),
        "answering one item's typing does not visit the program"
    );
}
