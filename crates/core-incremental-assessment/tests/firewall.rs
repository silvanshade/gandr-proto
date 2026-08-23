//! The mechanism the engine path's whole advantage rests on, pinned directly.
//!
//! # What would otherwise go unnoticed
//!
//! The engine's advantage on a value-only edit is that the binding query
//! recomputes an equal value and the invalidation stops there. Turn that
//! equality comparison off — one attribute, and a real consumer of this engine
//! does exactly that across its whole query surface — and everything still
//! answers correctly, every differential stays green, and the cost table
//! quietly becomes a table about re-executing the program.
//!
//! So the equality is asserted on the bytes themselves, not inferred from a
//! timing or a count. If the binding encoding ever starts carrying something
//! that moves with an item's body, these fail.

use gandr_core_incremental_assessment::engine::EngineSession;
use gandr_core_incremental_assessment::workload::EditKind;
use gandr_core_incremental_assessment::workload::apply_edit;

use crate::support;

#[test]
fn value_only_edit_leaves_the_binding_bytes_equal()
{
    let workload = support::small();
    let program = workload.program();
    let slot = workload.middle_block_head().expect("a chain head exists");

    let mut session = EngineSession::install(&program).expect("the program installs");
    let before = session
        .binding_bytes(slot)
        .expect("the binding computes")
        .expect("a chain head binds its name");

    let edited = apply_edit(&program, slot, EditKind::ValueOnly).expect("the slot is in range");
    session.apply(&edited).expect("the edit applies");
    let after = session
        .binding_bytes(slot)
        .expect("the binding computes")
        .expect("a chain head still binds its name");

    assert_eq!(
        before, after,
        "a value-only edit recomputes the same binding, which is what stops the invalidation wave"
    );
}

#[test]
fn type_changing_edit_moves_the_binding_bytes()
{
    let workload = support::small();
    let program = workload.program();
    let slot = workload.middle_block_head().expect("a chain head exists");

    let mut session = EngineSession::install(&program).expect("the program installs");
    let before = session
        .binding_bytes(slot)
        .expect("the binding computes")
        .expect("a chain head binds its name");

    let edited = apply_edit(&program, slot, EditKind::TypeChanging).expect("the slot is in range");
    session.apply(&edited).expect("the edit applies");
    let after = session
        .binding_bytes(slot)
        .expect("the binding computes")
        .expect("a chain head still binds its name");

    assert_ne!(
        before, after,
        "a type-changing edit moves the binding, so the wave must not stop"
    );
}

#[test]
fn the_wave_stops_at_the_edited_item_on_a_value_only_edit()
{
    // The counting form of the same claim, and the one that would catch a
    // configuration where the encoding is right and the comparison is switched
    // off: exactly one item's typing is recomputed, out of a thousand.
    let workload = support::thousand_items();
    let comparison = gandr_core_incremental_assessment::measure::run(workload, EditKind::ValueOnly)
        .expect("the run completes");

    let retyped: usize = comparison.engine_all_counts.typing_executions.into();
    let expected: usize = workload.dirty_items(EditKind::ValueOnly).into();
    assert_eq!(
        retyped, expected,
        "exactly the edited item is re-typed; a re-execution count near the program size would mean the equality cutoff is not firing"
    );

    let bindings: usize = comparison.engine_all_counts.binding_executions.into();
    assert!(
        bindings >= 1,
        "the edited item's binding really was recomputed — the wave was stopped by an equal result, not by nothing having been marked dirty"
    );
}
