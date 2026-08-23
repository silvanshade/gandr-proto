//! Shared fixtures, and the witnesses that the generator produces the program
//! the assertions assume.
//!
//! The measurements are only as good as the workload's structure: every
//! asserted count is derived from "chains of this length, edited here", so a
//! generator that quietly produced something else would make the whole table
//! wrong without any single number looking implausible. These witnesses pin the
//! structure itself.

use gandr_core_incremental_assessment::arena::ItemStore;
use gandr_core_incremental_assessment::baseline::from_scratch;
use gandr_core_incremental_assessment::baseline::rescan_footprints;
use gandr_core_incremental_assessment::boundary::BlockCount;
use gandr_core_incremental_assessment::boundary::BlockLength;
use gandr_core_incremental_assessment::boundary::ItemCount;
use gandr_core_incremental_assessment::workload::EditKind;
use gandr_core_incremental_assessment::workload::Workload;
use gandr_core_incremental_assessment::workload::apply_edit;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;

/// The measured workload: a thousand items in a hundred independent chains of
/// ten.
///
/// A thousand items is the size the assessment is stated over; chains of ten
/// give a type-changing edit a dirty set an order of magnitude below the
/// program, which is the range where a traversal floor is visible at all.
pub fn thousand_items() -> Workload
{
    Workload::new(BlockCount::from(100_usize), BlockLength::from(10_usize))
}

/// A small workload, for witnesses that want to be read rather than measured.
pub fn small() -> Workload
{
    Workload::new(BlockCount::from(3_usize), BlockLength::from(4_usize))
}

#[test]
fn chain_heads_are_literals_and_readers_are_linked()
{
    let workload = small();
    let program = workload.program();
    assert_eq!(program.items.len(), 12, "three chains of four");
    for (index, item) in program.items.iter().enumerate() {
        let is_head = index % 4 == 0;
        match item.term {
            | Term::Value(Value::Int(_)) => {
                assert!(is_head, "only a chain head is a bare literal: item {index}");
            },
            | Term::Value(Value::Annot(ref inner, _)) => {
                assert!(!is_head, "only a reader is ascribed: item {index}");
                let Value::Var(ref name) = **inner
                else {
                    panic!("a reader reads a name: item {index}");
                };
                assert_eq!(
                    name.as_str(),
                    format!("d{}", index - 1),
                    "a reader reads its immediate predecessor"
                );
            },
            | _ => panic!("the generator emits only literals and ascribed reads"),
        }
    }
}

#[test]
fn generated_names_are_distinct()
{
    // Distinctness is load-bearing beyond tidiness: a repeated definition name
    // would put a mutual dependency into the query graph and, in the checker,
    // reach the unbounded conversion path a duplicate identity opens.
    let program = thousand_items().program();
    let mut names: Vec<&str> = program
        .items
        .iter()
        .filter_map(|item| item.name.as_deref())
        .collect();
    let total = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), total, "every generated name occurs once");
    assert_eq!(total, 1000, "every item is a definition");
}

#[test]
fn value_only_edit_preserves_the_bound_type()
{
    let workload = small();
    let program = workload.program();
    let slot = workload.middle_block_head().expect("a chain head exists");
    let edited = apply_edit(&program, slot, EditKind::ValueOnly).expect("the slot is in range");

    let index: usize = slot.into();
    assert_ne!(
        program.items[index].term, edited.items[index].term,
        "the edit changed the term"
    );
    let before = from_scratch(&program);
    let after = from_scratch(&edited);
    assert_eq!(
        before, after,
        "a value-only edit leaves every item's typing where it was"
    );
}

#[test]
fn type_changing_edit_moves_the_bound_type()
{
    let workload = small();
    let program = workload.program();
    let slot = workload.middle_block_head().expect("a chain head exists");
    let edited = apply_edit(&program, slot, EditKind::TypeChanging).expect("the slot is in range");

    let before = from_scratch(&program);
    let after = from_scratch(&edited);
    let moved = before
        .iter()
        .zip(after.iter())
        .filter(|&(left, right)| left != right)
        .count();
    let expected: usize = workload.dirty_items(EditKind::TypeChanging).into();
    assert_eq!(
        moved, expected,
        "a type-changing edit at a chain head moves exactly its chain"
    );
}

#[test]
fn digest_moves_with_the_term()
{
    let workload = small();
    let program = workload.program();
    let slot = workload.middle_block_head().expect("a chain head exists");
    let edited = apply_edit(&program, slot, EditKind::ValueOnly).expect("the slot is in range");
    let index: usize = slot.into();

    let before = ItemStore::digest(&program.items[index]).expect("the item is addressable");
    let after = ItemStore::digest(&edited.items[index]).expect("the item is addressable");
    assert_ne!(before, after, "an edited item's digest moves");

    let untouched = usize::from(index == 0);
    let stable_before =
        ItemStore::digest(&program.items[untouched]).expect("the item is addressable");
    let stable_after =
        ItemStore::digest(&edited.items[untouched]).expect("the item is addressable");
    assert_eq!(
        stable_before, stable_after,
        "an untouched item's digest holds still"
    );
}

#[test]
fn rescan_visits_every_item()
{
    let workload = thousand_items();
    let program = workload.program();
    let scanned = rescan_footprints(&program);
    assert_eq!(
        scanned,
        ItemCount::from(1000_usize),
        "the rescan touches every item of the program, which is what makes its cost a function of program size"
    );
    assert_eq!(scanned, workload.item_count());
}

#[test]
fn the_name_table_preserves_source_order()
{
    use gandr_core_incremental_assessment::boundary::DefinitionKey;
    use gandr_core_incremental_assessment::boundary::SlotIndex;
    use gandr_core_incremental_assessment::engine::EngineSession;

    let workload = small();
    let program = workload.program();
    let session = EngineSession::install(&program).expect("the program installs");
    let table = session.name_table();

    // Every generated name resolves to its own slot, from any reader below it.
    for index in 1 .. program.items.len() {
        let previous = index - 1;
        let key = DefinitionKey::from(format!("d{previous}"));
        assert_eq!(
            table.definer(&key, SlotIndex::from(index)),
            Some(SlotIndex::from(previous)),
            "the immediate predecessor resolves for a reader at {index}"
        );
    }
    // A name does not resolve for a reader at or before its own definition,
    // which is the forward-only threading the checkpoint engine performs.
    let key = DefinitionKey::from("d0".to_owned());
    assert_eq!(
        table.definer(&key, SlotIndex::from(0_usize)),
        None,
        "a definition is not in scope for itself"
    );
}
