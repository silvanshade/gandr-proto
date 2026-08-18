//! The named core computations the suite drives.
//!
//! These are the eight programs the compilation host compiles, written here as
//! core computations. The host holds the same eight under the same names, and
//! `crates/core-sequent/tests/compile_host_agreement.rs` holds the fixture
//! stating what the L machine answers for each. Three independent statements
//! of one program set is what makes the agreement a differential.

use alloc::rc::Rc;

use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;
use gandr_core_term::types::ValueType;

/// An injection annotated with the sum it belongs to.
///
/// The core's injection is check-only — nothing in `inl v` says what the other
/// summand is — so a program that reaches the bridge's checker gate carries
/// the annotation. The annotation has no runtime content and no image node:
/// the focusing translation drops it, and so does the lowering.
fn injection(
    side: Side,
    payload: Value,
) -> Value
{
    let integers = ValueType::sum(ValueType::integer(), ValueType::integer());
    Value::annot(Value::Inj(side, Rc::new(payload)), integers)
}

/// One named program.
pub struct Program
{
    /// The name the host knows it by.
    pub name: &'static str,
    /// The computation.
    pub comp: Comp,
}

/// The eight named programs, in the fixture's order.
pub fn named() -> Vec<Program>
{
    alloc::vec![
        Program {
            name: "cut",
            comp: Comp::ret(Value::Int(5)),
        },
        Program {
            name: "bind",
            comp: Comp::bind(Comp::ret(Value::Int(7)), "x", Comp::ret(Value::var("x"))),
        },
        Program {
            name: "case",
            comp: Comp::case(
                injection(Side::Fst, Value::Int(3)),
                "l",
                Comp::ret(Value::var("l")),
                "r",
                Comp::ret(Value::var("r")),
            ),
        },
        Program {
            name: "ctor",
            comp: Comp::ret(Value::pair(Value::Int(1), Value::Int(2))),
        },
        Program {
            name: "dup",
            comp: Comp::dup(Value::Int(4)),
        },
        Program {
            name: "drop",
            comp: Comp::drop(Value::Int(9)),
        },
        Program {
            name: "compound",
            comp: Comp::bind(
                Comp::ret(injection(Side::Fst, Value::Int(8))),
                "s",
                Comp::case(
                    Value::var("s"),
                    "l",
                    Comp::ret(Value::var("l")),
                    "r",
                    Comp::ret(Value::var("r")),
                ),
            ),
        },
        Program {
            name: "accounted-work",
            comp: Comp::bind(
                Comp::dup(Value::Int(4)),
                "p",
                Comp::bind(Comp::drop(Value::var("p")), "q", Comp::ret(Value::Int(0))),
            ),
        },
    ]
}
