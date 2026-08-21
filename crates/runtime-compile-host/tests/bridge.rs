//! The bridge, driven end to end against the linked compilation host.
//!
//! **These cases are unconditional.** This module compiles only under the
//! `full` feature, which links the host, so a binary that exists at all has
//! already had every boundary entry resolved by the linker. There is no
//! absence to report and no skip to weaken the gate.
//!
//! Two checks divide the work and neither substitutes for the other. The
//! linker proves that every symbol is present and bound, which is what
//! `a_boundary_symbol_that_drifts_fails_at_link_time` exercises from the
//! outside. `contract` proves that the layout this crate mirrors is the layout
//! the host declares, and it runs on any machine with no host at all — a
//! linker cannot see that a struct field moved.

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

/// The reserved heap words a run needs before it can allocate anything.
///
/// The host declares the same number; `contract` holds the two equal.
const RESERVED_PREFIX_WORDS: u64 = 4;

/// The linked host.
///
/// There is no absence to report. This module compiles only under the feature
/// that links the host, so reaching this function at all means the linker
/// already resolved every entry; the one thing left to establish is that the
/// linked host agrees with this crate about the boundary version.
fn host() -> CompileHost
{
    CompileHost::bind().expect("the linked host declares this crate's boundary version")
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
    let host = host();

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
    let host = host();

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
    let host = host();

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
    let host = host();

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
    let host = host();

    let outside = Comp::ret(Value::Str(String::from("text")));
    let refused = compile_and_run(&host, &outside);
    assert!(
        matches!(refused, Err(BridgeError::NotLowered { .. })),
        "a value outside the slice reached the boundary: {refused:?}"
    );
}

/// A library that exports no release entry is refused before a run allocates.
///
/// This is the boundary-order witness, and it is a witness rather than an
/// argument because the compilation host builds a **deliberately incomplete**
/// boundary beside the real one: `gandr-compile-host-abi-partial` declares the
/// same ABI version and the same run entry, and exports no release at all.
///
/// A caller that resolved the release entry only after invoking a run would,
/// against this library, allocate the outcome's text and then fail to find the
/// function that frees it — leaking on the way to reporting the error. Because
/// the release is resolved first, the failure arrives before the run is
/// invoked at all, and `CompileHost::finish` takes the resolved entry by type
/// rather than looking one up, so the leaking order is unreachable rather than
/// A boundary that drifts is a LINK error naming the symbol.
///
/// This is the property the `full` feature buys, and it replaces a run-time
/// one. The bridge used to resolve each entry by name, so a host missing an
/// entry was a refusal at the moment of the call — late, and only on a machine
/// that ran the case. The linker resolves every entry now, so the same defect
/// stops the build.
///
/// Proving that from inside a test binary takes a second link, because a
/// failing link is exactly the thing that would stop this binary existing. The
/// case compiles two tiny C translation units against the host's own archive:
/// one calling an entry the boundary declares, one calling a name it does not.
/// The first links, the second does not, and the failure names the symbol.
#[test]
fn a_boundary_symbol_that_drifts_fails_at_link_time()
{
    let build = compile_host_build_directory();
    let link_line = std::fs::read_to_string(build.join("gandr-compile-host-link.txt"))
        .expect("the host build writes its link line");
    let libraries: Vec<&str> = link_line
        .lines()
        .filter(|entry| !entry.trim().is_empty())
        .collect();
    assert!(!libraries.is_empty(), "the link line names libraries");

    let clang = compile_host_clang();
    let scratch =
        std::env::temp_dir().join(alloc::format!("gandr-link-witness-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("a scratch directory");

    // The declared entry, and a name the boundary does not declare. Nothing
    // else differs, so a difference in outcome is the symbol and not the setup.
    //
    // The absent name is BUILT rather than written. A misspelling of the real
    // entry is what this case wants and is exactly what the repository's typo
    // formatter silently repairs — which would turn the negative case into a
    // second copy of the positive one, and a witness that cannot fail is worse
    // than no witness.
    let declared = "gandr_compile_host_abi_version";
    let absent = alloc::format!("{declared}_no_such_entry");
    let cases = [
        ("present", declared, true),
        ("drifted", absent.as_str(), false),
    ];
    for (name, symbol, should_link) in cases {
        // Declared `extern "C"`, because that is what the boundary is: the
        // compiler CMake recorded is a C++ driver, and an unqualified
        // declaration would be mangled into a different symbol entirely.
        let source = scratch.join(alloc::format!("{name}.cpp"));
        let program = alloc::format!(
            r#"extern "C" unsigned int {symbol}(void);
int main() {{ return static_cast<int>({symbol}()); }}
"#
        );
        std::fs::write(&source, program).expect("the witness source is writable");

        let mut command = std::process::Command::new(&clang);
        command
            .arg(&source)
            .arg("-o")
            .arg(scratch.join(name))
            .arg("-lc++");
        for library in &libraries {
            command.arg(library);
        }
        let linked = command.output().expect("the pinned clang runs");

        if should_link {
            assert!(
                linked.status.success(),
                "the declared entry did not link: {}",
                String::from_utf8_lossy(&linked.stderr)
            );
        }
        else {
            assert!(
                !linked.status.success(),
                "a symbol the boundary does not declare linked anyway"
            );
            let reported = String::from_utf8_lossy(&linked.stderr);
            assert!(
                reported.contains(symbol),
                "the link failure names the missing symbol: {reported}"
            );
        }
    }

    drop(std::fs::remove_dir_all(&scratch));
}

/// The host's build directory, from this crate's manifest.
fn compile_host_build_directory() -> std::path::PathBuf
{
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crates/<name> sits under the workspace")
        .join("runtime/compile-host/build")
}

/// The `clang` that ships beside the pinned MLIR, as `CMake` recorded it.
///
/// Read from the host's own `CMake` cache rather than found on `PATH`: the
/// witness must link with the compiler the host was built by, and a different
/// one on `PATH` would be a different question.
fn compile_host_clang() -> std::path::PathBuf
{
    let cache = compile_host_build_directory().join("CMakeCache.txt");
    let text = std::fs::read_to_string(&cache).expect("the host build has a cache");
    // The task passes the compiler on the command line, so CMake records it
    // as UNINITIALIZED rather than as a cached FILEPATH; both spellings are
    // accepted so the witness does not depend on how the build was invoked.
    for line in text.lines() {
        for key in [
            "CMAKE_CXX_COMPILER:FILEPATH=",
            "CMAKE_CXX_COMPILER:UNINITIALIZED=",
            "CMAKE_CXX_COMPILER:STRING=",
        ] {
            if let Some(value) = line.strip_prefix(key) {
                return std::path::PathBuf::from(value);
            }
        }
    }
    panic!("the `CMake` cache names no C++ compiler");
}
