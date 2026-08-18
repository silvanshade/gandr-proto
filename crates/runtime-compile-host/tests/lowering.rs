//! The lowering, checked without a compilation host.
//!
//! Everything here runs on the merge wall: the lowering, the wire form and the
//! checker gate need no MLIR, so the half of the bridge that can be held
//! unconditionally is held unconditionally.

use alloc::rc::Rc;

use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;
use gandr_runtime_compile_host::BridgeError;
use gandr_runtime_compile_host::LowerError;
use gandr_runtime_compile_host::check_and_lower;
use gandr_runtime_compile_host::image::CtorTag;
use gandr_runtime_compile_host::image::NodeKind;
use gandr_runtime_compile_host::is_typed;
use gandr_runtime_compile_host::lower_computation;

use crate::programs;

/// Every named program lowers, and its arena ends in the terminal cut.
#[test]
fn every_named_program_lowers_to_an_arena_ending_in_its_cut()
{
    for program in programs::named() {
        let lowered = lower_computation(&program.comp);
        let Ok(image) = lowered
        else {
            panic!("{} did not lower: {lowered:?}", program.name);
        };
        assert!(
            !bool::from(image.is_empty()),
            "{} lowered to nothing",
            program.name
        );

        let nodes = image.nodes();
        let last = nodes.last().expect("a non-empty arena has a last node");
        assert_eq!(
            last.kind,
            NodeKind::Cut,
            "{}'s arena does not end in the terminal cut",
            program.name
        );
        assert_eq!(
            nodes
                .iter()
                .filter(|node| node.kind == NodeKind::Cut)
                .count(),
            1,
            "{} lowered to more than one cut",
            program.name
        );
    }
}

/// Every operand addresses a strictly earlier node, which is what makes the
/// arena a dependency order rather than a graph the host would have to sort.
#[test]
fn every_operand_addresses_a_strictly_earlier_node()
{
    for program in programs::named() {
        let image = lower_computation(&program.comp).expect("the named programs lower");
        for (position, node) in image.nodes().iter().enumerate() {
            for operand in &node.operands {
                let addressed = usize::try_from(u32::from(*operand)).expect("an index fits");
                assert!(
                    addressed < position,
                    "{} has a node at {position} naming {addressed}",
                    program.name
                );
            }
        }
    }
}

/// A binder frame's body sees the binder, and the distance is counted inwards.
#[test]
fn a_variable_lowers_to_its_distance_from_the_innermost_binder()
{
    // `bind (ret 1) as outer. bind (ret 2) as inner. ret outer` — the
    // reference reaches past one binder, so its distance is one rather than
    // zero, which is what distinguishes an index from a level.
    let comp = Comp::bind(
        Comp::ret(Value::Int(1)),
        "outer",
        Comp::bind(
            Comp::ret(Value::Int(2)),
            "inner",
            Comp::ret(Value::var("outer")),
        ),
    );
    let image = lower_computation(&comp).expect("the computation lowers");
    let variables: Vec<u32> = image
        .nodes()
        .iter()
        .filter(|node| node.kind == NodeKind::Var)
        .map(|node| u32::from(node.binder))
        .collect();
    assert_eq!(variables, alloc::vec![1], "the reference skips one binder");
}

/// A dispatch's arms each bind their own payload, so both arms' references are
/// innermost.
#[test]
fn each_dispatch_arm_binds_its_own_payload()
{
    let comp = Comp::case(
        Value::Inj(Side::Fst, Rc::new(Value::Int(3))),
        "l",
        Comp::ret(Value::var("l")),
        "r",
        Comp::ret(Value::var("r")),
    );
    let image = lower_computation(&comp).expect("the computation lowers");
    let variables: Vec<u32> = image
        .nodes()
        .iter()
        .filter(|node| node.kind == NodeKind::Var)
        .map(|node| u32::from(node.binder))
        .collect();
    assert_eq!(variables, alloc::vec![0, 0], "both arms bind innermost");
}

/// A pair lowers to the constructor its tag declares, with its fields in
/// source order.
#[test]
fn a_pair_lowers_to_its_tag_with_two_fields()
{
    let comp = Comp::ret(Value::pair(Value::Int(1), Value::Int(2)));
    let image = lower_computation(&comp).expect("the computation lowers");
    let constructors: Vec<&_> = image
        .nodes()
        .iter()
        .filter(|node| node.kind == NodeKind::Ctor)
        .collect();
    assert_eq!(constructors.len(), 1, "one constructor");
    let pair = constructors.first().expect("one constructor");
    assert_eq!(pair.tag, CtorTag::Pair);
    assert_eq!(pair.operands.len(), usize::from(CtorTag::Pair.arity()));
}

/// Each excluded core form is refused, and the refusal names the form.
#[test]
fn every_excluded_core_form_is_refused_by_name()
{
    let outside: Vec<(&str, Comp)> = alloc::vec![
        (
            "an abstraction",
            Comp::Abs(String::from("x"), None, Rc::new(Comp::ret(Value::Unit)))
        ),
        (
            "an application",
            Comp::App(Rc::new(Comp::ret(Value::Unit)), Rc::new(Value::Unit)),
        ),
        ("a forced thunk", Comp::Force(Rc::new(Value::Unit))),
        ("a string literal", Comp::ret(Value::Str(String::from("s")))),
        ("a list literal", Comp::ret(Value::List(alloc::vec![]))),
    ];

    for (expected, comp) in outside {
        let refused = lower_computation(&comp);
        let Err(LowerError::OutsideSlice { form }) = refused
        else {
            panic!("{expected} was not refused as outside the slice: {refused:?}");
        };
        assert_eq!(
            form.to_string(),
            expected,
            "the refusal named the wrong form"
        );
    }
}

/// A free variable is refused rather than lowered to some binder.
#[test]
fn a_free_variable_is_refused_rather_than_lowered()
{
    let comp = Comp::ret(Value::var("loose"));
    let refused = lower_computation(&comp);
    let Err(LowerError::UnboundVariable { name }) = refused
    else {
        panic!("a free variable was not refused: {refused:?}");
    };
    assert_eq!(name.to_string(), "loose");
}

/// The checker runs before the lowering, so an ill-typed computation is
/// refused at the stage that owns the refusal.
#[test]
fn a_computation_the_checker_refuses_never_reaches_the_lowering()
{
    // `case 3 of …` dispatches on an integer, which the checker refuses; the
    // lowering on its own would have accepted the shape.
    let comp = Comp::case(
        Value::Int(3),
        "l",
        Comp::ret(Value::var("l")),
        "r",
        Comp::ret(Value::var("r")),
    );
    assert!(
        lower_computation(&comp).is_ok(),
        "the lowering alone accepts the shape, so the refusal below is the checker's"
    );

    let refused = check_and_lower(&comp);
    assert!(
        matches!(refused, Err(BridgeError::NotChecked { .. })),
        "the checker did not refuse the dispatch on an integer: {refused:?}"
    );
}

/// The typed gate admits the typed programs and refuses the grade ones.
///
/// This is the arc's sharpest finding about the slice, pinned as a test rather
/// than left in prose: the machine's positive core is **wider** than the typed
/// core in exactly one place. The machine's duplication and discard are
/// structural operations over any runtime value, while the core's `dup` and
/// `drop` are the grade rules and want a graded thunk — so `dup 4` runs on the
/// machine and is not a typed computation at all.
///
/// The delta is nameable and small: the slice can lower a typed `dup` exactly
/// when the image can represent a thunk, which is the codata rung. Until then
/// the grade programs reach the host through `run_machine_program`.
#[test]
fn the_typed_gate_admits_the_typed_programs_and_refuses_the_grade_ones()
{
    let mut typed: Vec<&str> = Vec::new();
    let mut machine_only: Vec<&str> = Vec::new();
    for program in programs::named() {
        if bool::from(is_typed(&program.comp)) {
            typed.push(program.name);
        }
        else {
            machine_only.push(program.name);
        }
    }

    assert_eq!(
        typed,
        alloc::vec!["cut", "bind", "case", "ctor", "compound"],
        "the typed half of the named set changed"
    );
    assert_eq!(
        machine_only,
        alloc::vec!["dup", "drop", "accounted-work"],
        "the machine-only half of the named set changed"
    );

    // The refusal is the grade rule's, not the lowering's: every one of these
    // lowers perfectly well.
    for program in programs::named() {
        assert!(
            lower_computation(&program.comp).is_ok(),
            "{} does not lower",
            program.name
        );
    }
}
