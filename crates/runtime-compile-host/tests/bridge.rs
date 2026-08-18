//! The bridge, driven end to end against a built compilation host.
//!
//! The host's toolchain is a discovered MLIR installation rather than a pinned
//! dependency, so these cases are **conditional by construction**: with no
//! host present they report the absence and stop. That would be a weak gate on
//! its own — a skipping test is weakest exactly when the toolchain is what
//! broke — so two things carry the weight beside it.
//!
//! First, `GANDR_COMPILE_HOST_REQUIRED` turns the condition off: with it set,
//! an absent host is a failure rather than a skip, and the present-toolchain
//! lane (`mise run compile-host:wall`) sets it. Second, `contract` holds this
//! crate's mirror of the boundary to the host's own headers unconditionally,
//! so the failure this file cannot see without a host — a host that changed
//! its representation — is caught on the merge wall regardless.

use gandr_core_term::syntax::Comp;
use gandr_core_term::syntax::Value;
use gandr_runtime_compile_host::BridgeError;
use gandr_runtime_compile_host::CompileHost;
use gandr_runtime_compile_host::HostError;
use gandr_runtime_compile_host::compile_and_run;
use gandr_runtime_compile_host::host::HeapWords;
use gandr_runtime_compile_host::host::RefusalStage;
use gandr_runtime_compile_host::is_typed;
use gandr_runtime_compile_host::lower_computation;
use gandr_runtime_compile_host::run_machine_program;

use crate::programs;
use crate::rendering::machine_answer;

/// The variable that turns an absent host from a skip into a failure.
const REQUIRED_VARIABLE: &str = "GANDR_COMPILE_HOST_REQUIRED";

/// The reserved heap words a run needs before it can allocate anything.
///
/// The host declares the same number; `contract` holds the two equal.
const RESERVED_PREFIX_WORDS: u64 = 4;

/// The host, or nothing when this checkout has none built.
///
/// An absent host is reported and the case stops — unless the lane says the
/// host must be there, in which case its absence is the finding.
fn host_or_skip() -> Option<CompileHost>
{
    match CompileHost::discover() {
        | Ok(host) => Some(host),
        | Err(HostError::Unavailable { looked }) => {
            assert!(
                std::env::var_os(REQUIRED_VARIABLE).is_none(),
                "{REQUIRED_VARIABLE} is set and no compilation host was found; looked at {looked}"
            );
            eprintln!("no compilation host built; skipping. looked at {looked}");
            None
        },
        | Err(other) => panic!("a compilation host was found but could not be bound: {other}"),
    }
}

/// The bridge's answer for every named program is the L machine's answer.
///
/// This is the arc's central claim, and it is a differential rather than a
/// restatement: the left side is a core computation checked and run on the L
/// machine in this workspace, and the right side is the same computation
/// lowered to a plain-old-data image, compiled through MLIR in another build
/// with another toolchain, and executed.
#[test]
fn the_bridge_agrees_with_the_l_machine_on_every_named_program()
{
    let Some(host) = host_or_skip()
    else {
        return;
    };

    for program in programs::named() {
        // The grade programs are machine-level rather than typed: the core's
        // `dup` and `drop` want a graded thunk, which the image cannot
        // represent until the codata rung lands. Both routes end in the same
        // compiled run; only the typed gate differs.
        let answered = if bool::from(is_typed(&program.comp)) {
            compile_and_run(&host, &program.comp)
        }
        else {
            run_machine_program(&host, &program.comp)
        };
        let Ok(answer) = answered
        else {
            panic!("{} did not reach an answer: {answered:?}", program.name);
        };
        assert_eq!(
            answer.value.to_string(),
            machine_answer(&program.comp),
            "{} disagrees with the L machine",
            program.name
        );
    }
}

/// The accounted work the run reports is the work the program holds.
///
/// For a program with no dispatch every node runs exactly once, so the image's
/// own duplication and discard counts are the ledger rather than a bound on
/// it — an expectation computed in this workspace, against a ledger read out
/// of a heap the compiled code wrote.
#[test]
fn the_bridge_agrees_with_the_image_on_accounted_work()
{
    let Some(host) = host_or_skip()
    else {
        return;
    };

    let mut exercised = 0_u32;
    for program in programs::named() {
        let image = lower_computation(&program.comp).expect("the named programs lower");
        if bool::from(image.has_dispatch()) {
            continue;
        }
        let expected = image.accounted_work();
        let answer = run_machine_program(&host, &program.comp).expect("the named programs run");
        assert_eq!(
            i64::from(answer.duplications),
            i64::from(expected.duplications),
            "{} executed a different number of duplications",
            program.name
        );
        assert_eq!(
            i64::from(answer.discards),
            i64::from(expected.discards),
            "{} executed a different number of discards",
            program.name
        );
        exercised = exercised.saturating_add(1);
    }
    assert!(exercised > 0, "no dispatch-free program was exercised");
}

/// The host's two paths agree through the bridge.
///
/// The compiled run and the reference walk share the heap layout and the value
/// rendering and nothing else, so this separates a fault in the emitter or the
/// lowering from one in the image the bridge produced.
#[test]
fn the_two_host_paths_agree_through_the_bridge()
{
    let Some(host) = host_or_skip()
    else {
        return;
    };

    for program in programs::named() {
        let image = lower_computation(&program.comp).expect("the named programs lower");
        let bytes = image.encode();
        let compiled = host.run(&bytes).expect("the compiled path answers");
        let reference = host.interpret(&bytes).expect("the reference path answers");

        assert_eq!(
            compiled.value, reference.value,
            "{}'s two host paths disagree on the answer",
            program.name
        );
        assert_eq!(
            compiled.duplications, reference.duplications,
            "{}'s two host paths disagree on duplications",
            program.name
        );
        assert_eq!(
            compiled.discards, reference.discards,
            "{}'s two host paths disagree on discards",
            program.name
        );
        assert!(
            u64::from(compiled.allocated) <= u64::from(reference.allocated),
            "{} allocated more compiled than interpreted",
            program.name
        );
    }
}

/// The compiled bounds check is visible from the Rust side.
///
/// The heap size is measured rather than predicted: the run reports what it
/// consumed, and the boundary case is one word below that.
#[test]
fn the_bridge_sees_the_compiled_bounds_check()
{
    let Some(host) = host_or_skip()
    else {
        return;
    };

    let mut exercised = 0_u32;
    for program in programs::named() {
        let image = lower_computation(&program.comp).expect("the named programs lower");
        let bytes = image.encode();
        let generous = host.run(&bytes).expect("the compiled path answers");
        let consumed = u64::from(generous.allocated);
        if consumed == 0 {
            continue;
        }
        let exact = RESERVED_PREFIX_WORDS.saturating_add(consumed);

        let fitted = host.run_with_heap(&bytes, HeapWords::from(exact));
        let Ok(fitted) = fitted
        else {
            panic!(
                "{} did not fit its own measured heap: {fitted:?}",
                program.name
            );
        };
        assert_eq!(
            fitted.value, generous.value,
            "{}'s answer changed on an exact heap",
            program.name
        );

        let starved = host.run_with_heap(&bytes, HeapWords::from(exact.saturating_sub(1)));
        let Err(HostError::Refused { stage, .. }) = starved
        else {
            panic!(
                "{} answered on a heap one word short: {starved:?}",
                program.name
            );
        };
        assert_eq!(
            stage,
            RefusalStage::LimitExceeded,
            "{}'s refusal named the wrong stage",
            program.name
        );
        exercised = exercised.saturating_add(1);
    }
    assert!(exercised > 0, "no allocating program was exercised");
}

/// A computation outside the slice is refused before anything crosses.
#[test]
fn a_computation_outside_the_slice_is_refused_before_the_boundary()
{
    let Some(host) = host_or_skip()
    else {
        return;
    };

    let outside = Comp::ret(Value::Str(String::from("text")));
    let refused = compile_and_run(&host, &outside);
    assert!(
        matches!(refused, Err(BridgeError::NotLowered { .. })),
        "a value outside the slice reached the boundary: {refused:?}"
    );
}

/// An absent host is an ordinary reported outcome, never a panic and never a
/// build failure.
///
/// This case needs no host, which is the point: the Rust workspace builds and
/// tests with no MLIR anywhere, and the absence is discovered here.
#[test]
fn an_absent_host_is_reported_rather_than_fatal()
{
    let missing = std::path::Path::new("/nonexistent/libgandr-compile-host-abi.dylib");
    let refused = CompileHost::open(missing);
    let Err(HostError::NotBindable { path, .. }) = refused
    else {
        panic!("a missing library was not reported as unbindable: {refused:?}");
    };
    assert_eq!(path.as_path(), missing);

    // Discovery itself is total: it answers with a host or with a typed
    // report, on any machine.
    match CompileHost::discover() {
        | Ok(host) => assert!(host.path().as_path().is_file()),
        | Err(HostError::Unavailable { looked }) => {
            assert!(
                !looked.to_string().is_empty(),
                "the report names where it looked"
            );
        },
        | Err(other) => panic!("discovery reported an unexpected failure: {other}"),
    }
}
