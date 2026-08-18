//! The L machine's half of the compile host's agreement differential.
//!
//! The compile host under `runtime/compile-host/` compiles the positive core
//! through MLIR and runs it. Its answers are compared against a checked-in
//! fixture; this module is what keeps that fixture equal to what the L machine
//! itself produces, so the composition of the two checks is a real differential
//! rather than two copies of one belief.
//!
//! The fixture is deliberately plain text keyed by program name: the host is a
//! separate build with a separate toolchain, and a shared binary encoding
//! neither side owns would be a third thing to keep true.

use alloc::rc::Rc;

use gandr_core_sequent::machine::run_comp;
use gandr_core_term::outcome::Eval;
use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Side;
use gandr_core_term::syntax::Value;

/// The fixture the compile host reads, relative to this crate's manifest.
const FIXTURE_PATH: &str = "../../runtime/compile-host/fixtures/positive-core-samples.txt";

/// One named program's key in the fixture.
///
/// The wrapper exists for the boundary rather than for behaviour: it is what
/// keeps the program list a collection of nominal records instead of a
/// collection of bare string-and-term tuples.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ProgramName(&'static str);

/// One of the programs the compile host compiles, with the core computation
/// the L machine runs for it.
struct Program
{
    /// The fixture key.
    name: ProgramName,
    /// The computation the machine runs.
    comp: Comp,
}

/// One step of the renderer's traversal: a value still to be read, or text
/// already decided.
enum RenderStep<'value>
{
    /// A value whose rendering has not started.
    Node(&'value Value),
    /// Text to append when the step is reached.
    Text(&'static str),
}

/// Renders a terminal value in the canonical grammar both sides compare on.
///
/// The grammar is `(int N)`, `(unit)`, `(pair V V)`, `(inl V)`, `(inr V)`.
/// A value outside the positive core has no rendering here on purpose: the
/// slice's boundary is what the fixture is about, so a form outside it must
/// fail loudly rather than acquire a spelling nobody agreed on.
///
/// The traversal is an explicit stack rather than recursion, mirroring the
/// compile host's own renderer, so a value's depth is bounded by the heap on
/// both sides of the differential.
fn render(value: &Value) -> String
{
    let mut rendered = String::new();
    let mut pending: Vec<RenderStep<'_>> = vec![RenderStep::Node(value)];

    while let Some(step) = pending.pop() {
        let node = match step {
            | RenderStep::Text(text) => {
                rendered.push_str(text);
                continue;
            },
            | RenderStep::Node(node) => node,
        };

        match *node {
            | Value::Int(literal) => {
                rendered.push_str("(int ");
                rendered.push_str(&literal.to_string());
                rendered.push(')');
            },
            | Value::Unit => rendered.push_str("(unit)"),
            | Value::Pair(ref first, ref second) => {
                rendered.push_str("(pair ");
                pending.push(RenderStep::Text(")"));
                pending.push(RenderStep::Node(second));
                pending.push(RenderStep::Text(" "));
                pending.push(RenderStep::Node(first));
            },
            | Value::Inj(side, ref payload) => {
                rendered.push_str(if side == Side::Fst { "(inl " } else { "(inr " });
                pending.push(RenderStep::Text(")"));
                pending.push(RenderStep::Node(payload));
            },
            | _ => panic!("value lies outside the compile host's positive-core slice: {node:?}"),
        }
    }

    rendered
}

/// Runs a computation on the L machine and renders its terminal value.
fn machine_answer(comp: &Comp) -> String
{
    match run_comp(comp) {
        | Eval::Value(Comp::Ret(value)) => render(&value),
        | other => panic!("the L machine did not reach a returned value: {other:?}"),
    }
}

/// The named programs the compile host compiles, as core computations.
///
/// One per positive-core transition, one compound program threading a binder
/// frame with a dispatch and a constructor, and the accounted-work program
/// whose answer mentions neither the duplication nor the discard it performs.
fn programs() -> Vec<Program>
{
    vec![
        Program {
            name: ProgramName("cut"),
            comp: Comp::ret(Value::Int(5)),
        },
        Program {
            name: ProgramName("bind"),
            comp: Comp::bind(Comp::ret(Value::Int(7)), "x", Comp::ret(Value::var("x"))),
        },
        Program {
            name: ProgramName("case"),
            comp: Comp::case(
                Value::Inj(Side::Fst, Rc::new(Value::Int(3))),
                "l",
                Comp::ret(Value::var("l")),
                "r",
                Comp::ret(Value::var("r")),
            ),
        },
        Program {
            name: ProgramName("ctor"),
            comp: Comp::ret(Value::pair(Value::Int(1), Value::Int(2))),
        },
        Program {
            name: ProgramName("dup"),
            comp: Comp::dup(Value::Int(4)),
        },
        Program {
            name: ProgramName("drop"),
            comp: Comp::drop(Value::Int(9)),
        },
        Program {
            name: ProgramName("compound"),
            comp: Comp::bind(
                Comp::ret(Value::Inj(Side::Fst, Rc::new(Value::Int(8)))),
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
            name: ProgramName("accounted-work"),
            comp: Comp::bind(
                Comp::dup(Value::Int(4)),
                "p",
                Comp::bind(Comp::drop(Value::var("p")), "q", Comp::ret(Value::Int(0))),
            ),
        },
    ]
}

/// Every program's name paired with what the L machine answers for it.
fn machine_answers() -> Vec<(String, String)>
{
    programs()
        .iter()
        .map(|program| (program.name.0.to_owned(), machine_answer(&program.comp)))
        .collect()
}

/// The fixture the L machine's answers must reproduce exactly.
///
/// A mismatch means one of two things, and both are findings: the fixture is
/// stale, or the machine changed. Neither is repaired by editing this test.
#[test]
fn the_fixture_states_what_the_l_machine_answers()
{
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = std::fs::read_to_string(manifest.join(FIXTURE_PATH))
        .expect("the compile host's agreement fixture is readable");

    let expected: Vec<(String, String)> = fixture
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, value) = line
                .split_once('\t')
                .expect("every fixture row is a tab-separated name and value");
            (name.to_owned(), value.to_owned())
        })
        .collect();

    assert_eq!(
        machine_answers(),
        expected,
        "the compile host's fixture disagrees with the L machine"
    );
}

/// Every transition the slice claims to cover reaches a distinct answer.
///
/// A fixture whose rows all said the same thing would agree with a host that
/// computed one constant, so the rows carry the discrimination the differential
/// rests on.
#[test]
fn every_sampled_transition_reaches_its_own_answer()
{
    let answers: Vec<String> = machine_answers()
        .into_iter()
        .map(|(_, answer)| answer)
        .collect();
    assert_eq!(answers.len(), 8, "the slice names eight programs");

    assert_eq!(answers.first().map(String::as_str), Some("(int 5)"));
    assert_eq!(answers.get(1).map(String::as_str), Some("(int 7)"));
    assert_eq!(answers.get(2).map(String::as_str), Some("(int 3)"));
    assert_eq!(
        answers.get(3).map(String::as_str),
        Some("(pair (int 1) (int 2))")
    );
    assert_eq!(
        answers.get(4).map(String::as_str),
        Some("(pair (int 4) (int 4))")
    );
    assert_eq!(answers.get(5).map(String::as_str), Some("(unit)"));
    assert_eq!(answers.get(6).map(String::as_str), Some("(int 8)"));
    assert_eq!(answers.get(7).map(String::as_str), Some("(int 0)"));
}
