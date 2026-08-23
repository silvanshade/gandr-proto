//! What the engine's cycle handling is worth at item granularity.
//!
//! # The answer is that it is not reachable here, and that is a result
//!
//! Cycle recovery is one of the things a general engine is usually credited
//! with, so it belongs in the assessment — but crediting it without exercising
//! it would be crediting a capability the workload never reaches.
//!
//! It is not reachable at this granularity, and the reason is structural rather
//! than accidental. An item's typing depends on the bindings of names resolved
//! **strictly ahead of it**: name resolution takes a reader's position and
//! searches only slots before it, mirroring the way the hand-rolled path
//! threads bindings forward through the item list. A query for item *i* can
//! therefore only ever depend on queries for items *j < i*, so the query graph
//! is a directed acyclic graph by construction and no fixpoint iteration can
//! arise.
//!
//! What that means for the assessment: **cycle handling is not a buy for
//! item-granular typing in gandr as the item seam is built.** It would become
//! one only if the seam admitted forward or mutual references between items —
//! which is a language-surface question, not an engine question, and which the
//! checker's own conversion path is separately not ready for.
//!
//! The test below is the exercised form of that claim rather than a restatement
//! of it: the engine reports every fixpoint iteration it runs, and over the
//! measured workloads it runs none.

use gandr_core_incremental_assessment::boundary::ExecutionCount;
use gandr_core_incremental_assessment::measure;
use gandr_core_incremental_assessment::workload::EditKind;

use crate::support;

#[test]
fn the_query_graph_never_iterates_a_cycle()
{
    for edit in [EditKind::ValueOnly, EditKind::TypeChanging] {
        let comparison = measure::run(support::thousand_items(), edit).expect("the run completes");
        assert_eq!(
            comparison.engine_all_counts.cycle_iterations,
            ExecutionCount::from(0_usize),
            "forward-only name resolution makes the query graph acyclic, so the engine's cycle recovery never runs under {edit:?}"
        );
        assert_eq!(
            comparison.engine_single_counts.cycle_iterations,
            ExecutionCount::from(0_usize),
            "the single-item demand reaches no cycle either under {edit:?}"
        );
    }
}
