//! The value-mediated read: the case that separates a query graph modelling the
//! adoption rule from one modelling a convenient half of it.
//!
//! # Why this program and not another
//!
//! Most readers depend on the *type* of what they read. A reader whose
//! ascription mentions a name depends on that name's **value** as well, because
//! deciding its ascription normalizes across the definition. Editing that
//! definition's body at a constant type therefore has to reach the reader,
//! while leaving every ordinary reader of the same definition alone.
//!
//! That distinction is exactly what the model's split contribution encodes — a
//! type dependency through the binding query, a value dependency through the
//! unfolding query, taken over different name sets. A model carrying only the
//! binding answers this program **wrongly**, and answers it wrongly quietly:
//! every count looks better, and one verdict is false.
//!
//! So this is where the correctness differential earns its place. The engine's
//! answer is compared against from-scratch typing, and the work counts are
//! checked to confirm the invalidation was targeted rather than a blanket
//! re-type that would pass the same equality by doing everything again.

use gandr_core_incremental::region::Item;
use gandr_core_incremental::region::Program;
use gandr_core_incremental_assessment::baseline::from_scratch;
use gandr_core_incremental_assessment::boundary::BlockLength;
use gandr_core_incremental_assessment::boundary::DefinitionKey;
use gandr_core_incremental_assessment::boundary::SlotIndex;
use gandr_core_incremental_assessment::engine::EngineSession;
use gandr_core_term::syntax::Term;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::Ty;
use gandr_core_term::types::ValueType;

/// `def <name> = <body>`.
fn definition(
    name: &DefinitionKey,
    body: Value,
) -> Item
{
    Item::new(Some(name.as_ref().to_owned()), None, Term::Value(body))
}

/// The program the case runs on, with `bystanders` ordinary readers of `one`:
///
/// ```text
/// def one = 1
/// def p   = here(1)                        -- infers Path Integer 1 1
/// def r : Path Integer one 1 = p           -- checks because one unfolds to 1
/// def b0 = (one : Integer)                 -- reads one's type, never its value
/// def b1 = (one : Integer)
/// ...
/// ```
///
/// The bystander count is a parameter because the sharp claim is not "few items
/// were re-typed" but "the number re-typed does not depend on how many ordinary
/// readers there are".
fn mediated_program(
    head: Value,
    bystanders: BlockLength,
) -> Program
{
    let bystanders: usize = bystanders.into();
    let mut items = vec![
        definition(&DefinitionKey::from("one".to_owned()), head),
        definition(
            &DefinitionKey::from("p".to_owned()),
            Value::here(Value::int(1_i64)),
        ),
        Item::new(
            Some("r".to_owned()),
            Some(Ty::Value(ValueType::path(
                ValueType::integer(),
                Value::var("one"),
                Value::int(1_i64),
            ))),
            Term::Value(Value::var("p")),
        ),
    ];
    for index in 0 .. bystanders {
        items.push(Item::new(
            Some(format!("b{index}")),
            None,
            Term::Value(Value::Annot(
                alloc::rc::Rc::new(Value::var("one")),
                alloc::rc::Rc::new(ValueType::integer()),
            )),
        ));
    }
    Program::new(items)
}

#[test]
fn the_engine_follows_a_value_only_edit_through_an_ascription()
{
    let base = mediated_program(Value::int(1_i64), BlockLength::from(1_usize));
    let edited = mediated_program(Value::int(2_i64), BlockLength::from(1_usize));

    // The premise: this edit really does change an answer. Without it the test
    // would pass for the uninteresting reason.
    let before = from_scratch(&base);
    let after = from_scratch(&edited);
    assert_ne!(
        before, after,
        "editing `one` from 1 to 2 changes what `r` types to, which is what makes this case a case"
    );

    let mut session = EngineSession::install(&base).expect("the program installs");
    let warmed = session.typings().expect("the base program types");
    assert_eq!(warmed, before, "the engine agrees before the edit");

    session.ledger().reset();
    session.apply(&edited).expect("the edit applies");
    let answers = session.typings().expect("the edited program types");

    assert_eq!(
        answers, after,
        "the engine follows the value-only edit through `r`'s ascription and agrees with from-scratch typing"
    );
}

#[test]
fn the_invalidation_is_targeted_rather_than_wholesale()
{
    // Agreement alone does not distinguish a model that tracked the value
    // dependency from one that gave up and re-typed everything, because both
    // agree. What distinguishes them is that the re-typed count here does not
    // grow with the number of ordinary readers.
    //
    // Three items are re-typed whatever that number is: `one`, which was
    // edited; `p`, whose term is an identity form the footprint scan cannot
    // represent as a read set and so conservatively treats as reading
    // everything; and `r`, whose ascription mentions `one` and whose typing
    // therefore consults its value. Every `b<n>` reads only `one`'s type, which
    // did not move.
    for bystanders in [1_usize, 8, 64] {
        let bystanders = BlockLength::from(bystanders);
        let base = mediated_program(Value::int(1_i64), bystanders);
        let edited = mediated_program(Value::int(2_i64), bystanders);

        let mut session = EngineSession::install(&base).expect("the program installs");
        let _warmed = session.typings().expect("the base program types");
        session.ledger().reset();
        session.apply(&edited).expect("the edit applies");
        let answers = session.typings().expect("the edited program types");
        assert_eq!(
            answers,
            from_scratch(&edited),
            "the answer is right at {bystanders:?} bystanders, which is the precondition for the count meaning anything"
        );

        let counts = session.ledger().snapshot();
        let retyped: usize = counts.typing_executions.into();
        assert_eq!(
            retyped, 3,
            "the re-typed set is `one`, `p` and `r` — and does not grow with {bystanders:?} ordinary readers: {counts:?}"
        );
    }
}

#[test]
fn a_value_only_edit_moves_the_unfolding_not_the_binding()
{
    // The mechanism, pinned on the encodings themselves: one contribution holds
    // still and the other moves. A model with a single contribution has to lose
    // one of these two properties.
    let base = mediated_program(Value::int(1_i64), BlockLength::from(1_usize));
    let edited = mediated_program(Value::int(2_i64), BlockLength::from(1_usize));
    let head = SlotIndex::from(0_usize);

    let mut session = EngineSession::install(&base).expect("the program installs");
    let binding_before = session.binding_bytes(head).expect("the binding computes");
    let unfolding_before = session
        .unfolding_bytes(head)
        .expect("the unfolding computes");

    session.apply(&edited).expect("the edit applies");
    let binding_after = session.binding_bytes(head).expect("the binding computes");
    let unfolding_after = session
        .unfolding_bytes(head)
        .expect("the unfolding computes");

    assert_eq!(
        binding_before, binding_after,
        "the type `one` binds did not move, so its type dependents keep their answers"
    );
    assert_ne!(
        unfolding_before, unfolding_after,
        "the value `one` unfolds to did move, so its value dependents must not"
    );
}
