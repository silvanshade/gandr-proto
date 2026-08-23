//! The measurement, reported.
//!
//! # Observation path
//!
//! ```text
//! cargo test -p gandr-core-incremental-assessment --test assessment -- --nocapture table
//! ```
//!
//! The counts are the substance and are profile-independent. The wall-clock
//! figures depend on the build profile and are reported beside them rather than
//! instead of them; a claim resting on a timing names the profile it was taken
//! under, and this target's default is the development profile.

use gandr_core_incremental_assessment::measure;
use gandr_core_incremental_assessment::workload::EditKind;

use crate::support;

#[test]
fn measurement_table()
{
    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(support::thousand_items(), edit).expect("the run completes");
        println!("{}", comparison.render());
        println!(
            "engine query counts, all-items demand:  {:?}",
            comparison.engine_all_counts
        );
        println!(
            "engine query counts, one-item demand:   {:?}",
            comparison.engine_single_counts
        );
        println!();
    }
}

#[test]
fn memory_breakdown()
{
    let comparison =
        measure::run(support::thousand_items(), EditKind::ValueOnly).expect("the run completes");
    println!("retained-state breakdown, one thousand items:");
    println!("{}", comparison.engine_memory_report);
}

#[test]
fn recheck_cost_against_program_size()
{
    // Both paths' recheck cost is measured against program size, because the
    // question an editing session asks is not "how expensive is one recheck"
    // but "how does one recheck grow as the program does".
    println!("items  baseline_micros  engine_all_micros  engine_one_micros");
    for blocks in [25_usize, 50, 100, 200] {
        let workload = gandr_core_incremental_assessment::workload::Workload::new(
            gandr_core_incremental_assessment::boundary::BlockCount::from(blocks),
            gandr_core_incremental_assessment::boundary::BlockLength::from(10_usize),
        );
        let comparison = measure::run(workload, EditKind::ValueOnly).expect("the run completes");
        let items: usize = workload.item_count().into();
        let mut baseline = 0_u128;
        let mut engine_all = 0_u128;
        let mut engine_one = 0_u128;
        for row in &comparison.rows {
            let micros: u128 = u128::from(row.elapsed).checked_div(1000).unwrap_or(0);
            match (row.path, row.demand) {
                | (measure::PathKind::Baseline, _) => baseline = micros,
                | (measure::PathKind::Engine, measure::DemandShape::AllItems) => {
                    engine_all = micros;
                },
                | (measure::PathKind::Engine, measure::DemandShape::SingleItem) => {
                    engine_one = micros;
                },
            }
        }
        println!("{items:>5}  {baseline:>15}  {engine_all:>17}  {engine_one:>17}");
    }
}
