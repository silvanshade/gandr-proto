//! The correctness gate: every measured path's answer against from-scratch
//! typing.
//!
//! # Why the reference is from-scratch and never the other path
//!
//! Both measured paths implement the same adoption rule by different means. A
//! comparison between them therefore cannot see a defect *in the rule* — both
//! would be wrong together, agree perfectly, and report green. Only
//! incremental-equals-batch sees that class, so from-scratch typing is the
//! reference for both, and the two paths are never compared to each other as
//! evidence.
//!
//! # What the engine path is really being asked here
//!
//! The engine types each item against a context holding **only** its
//! footprint's bindings, where the hand-rolled path types against the whole
//! accumulated context. Those agree exactly when the footprint
//! over-approximates the item's reads. That is an unproved property of the
//! footprint scan, not an assumption this harness is entitled to; the agreement
//! below is what tests it, and a disagreement would be a finding about
//! footprint completeness rather than a bug in the harness.

use gandr_core_incremental_assessment::measure;
use gandr_core_incremental_assessment::workload::EditKind;

use crate::support;

#[test]
fn engine_agrees_with_from_scratch_on_the_workload()
{
    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(support::thousand_items(), edit).expect("the run completes");
        assert_eq!(
            comparison.engine_typings, comparison.reference_typings,
            "the engine path's answer equals from-scratch typing under {edit:?}"
        );
    }
}

#[test]
fn both_paths_agree_with_from_scratch()
{
    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(support::thousand_items(), edit).expect("the run completes");
        assert_eq!(
            comparison.baseline_typings, comparison.reference_typings,
            "the baseline path's answer equals from-scratch typing under {edit:?}"
        );
        assert_eq!(
            comparison.engine_typings, comparison.reference_typings,
            "the engine path's answer equals from-scratch typing under {edit:?}"
        );
    }
}

#[test]
fn the_single_item_demand_answers_what_the_whole_program_would()
{
    // The demand shapes must agree, or the engine's cheaper answer is cheaper
    // for the wrong reason.
    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(support::thousand_items(), edit).expect("the run completes");
        let demanded: usize = comparison.demanded_slot.into();
        let reference = comparison
            .reference_typings
            .get(demanded)
            .expect("the demanded slot is in range");
        assert_eq!(
            &comparison.engine_single_typing, reference,
            "the single-item demand agrees with from-scratch typing under {edit:?}"
        );
    }
}

#[test]
fn the_differential_has_teeth()
{
    // A differential nobody can make fail proves nothing. The corruption below
    // is constructed to change an answer rather than hoping one flips: the
    // type-changing edit is applied to the program the engine sees, while the
    // reference is taken from the *unedited* program. The two disagree on
    // exactly the edited chain, by construction — so if the comparison above
    // could pass while the answers differed, this assertion would fail.
    let workload = support::thousand_items();
    let comparison = measure::run(workload, EditKind::TypeChanging).expect("the run completes");
    let unedited = gandr_core_incremental_assessment::baseline::from_scratch(&workload.program());

    let differing = comparison
        .engine_typings
        .iter()
        .zip(unedited.iter())
        .filter(|&(left, right)| left != right)
        .count();
    let expected: usize = workload.dirty_items(EditKind::TypeChanging).into();
    assert_eq!(
        differing, expected,
        "the engine's post-edit answer differs from the pre-edit answer on exactly the edited chain, so an equality assertion over these values can fail"
    );
    assert_ne!(
        comparison.engine_typings, unedited,
        "the reference the differential uses is not trivially equal to every answer"
    );
}
