//! The canonical rendering, checked against the L machine without a host.
//!
//! What this defends is that the Rust projection into the host's grammar is
//! total on the slice and refuses everything else — the boundary the fixture
//! comparison rests on.

use alloc::rc::Rc;

use gandr_core_sequent::machine::run_comp;
use gandr_core_term::outcome::Eval;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;
use gandr_runtime_compile_host::RenderError;
use gandr_runtime_compile_host::render::canonical;

use crate::programs;

/// The L machine's answer for a computation, rendered canonically.
pub fn machine_answer(comp: &Comp) -> String
{
    let Eval::Value(Comp::Ret(value)) = run_comp(comp)
    else {
        panic!("the L machine did not reach a returned value for {comp:?}");
    };
    let rendered = canonical(&value).expect("a terminal value of the slice renders");
    rendered.to_string()
}

/// Every named program's answer renders, and the renderings are the ones the
/// slice's grammar states.
#[test]
fn every_named_program_renders_its_own_answer()
{
    let answers: Vec<String> = programs::named()
        .iter()
        .map(|program| machine_answer(&program.comp))
        .collect();

    assert_eq!(answers, alloc::vec![
        String::from("(int 5)"),
        String::from("(int 7)"),
        String::from("(int 3)"),
        String::from("(pair (int 1) (int 2))"),
        String::from("(pair (int 4) (int 4))"),
        String::from("(unit)"),
        String::from("(int 8)"),
        String::from("(int 0)"),
    ]);
}

/// Nesting renders left to right, so a deep value is not reversed by the
/// explicit traversal that keeps the renderer off the host stack.
#[test]
fn a_nested_value_renders_in_source_order()
{
    let value = Value::pair(
        Value::Inj(Side::Fst, Rc::new(Value::Int(1))),
        Value::pair(Value::Int(2), Value::Unit),
    );
    let rendered = canonical(&value).expect("the value is inside the slice");
    assert_eq!(
        rendered.to_string(),
        "(pair (inl (int 1)) (pair (int 2) (unit)))"
    );
}

/// A value outside the slice has no spelling, loudly.
#[test]
fn a_value_outside_the_slice_has_no_spelling()
{
    let outside = Value::Str(String::from("text"));
    assert_eq!(canonical(&outside), Err(RenderError::OutsideSlice));

    // The refusal survives nesting: a value is outside the slice when any part
    // of it is.
    let nested = Value::pair(Value::Int(1), Value::Str(String::from("text")));
    assert_eq!(canonical(&nested), Err(RenderError::OutsideSlice));
}
