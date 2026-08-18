//! The source-level contract gate over the compilation host.
//!
//! The host's own gates need a discovered MLIR installation, so a change that
//! broke the host without touching Rust would pass the merge wall. This file
//! narrows that gap from the Rust side, and it is deliberate about how far it
//! reaches.
//!
//! **What it proves.** Every number and declaration this crate's mirror of the
//! boundary depends on still says what it says in the host's own sources: the
//! heap layout, the cell and node numbering, the constructor arities, the wire
//! version and bound, the ABI version and its status constants, the boundary
//! struct's field order, and the two ruled disciplines the host's regression
//! suite defends — the verifier wall opening every pipeline, and duplication
//! and discard declaring memory effects rather than purity.
//!
//! **What it does not prove.** It reads text. It cannot tell whether the host
//! *behaves* the way those declarations say, and it will not notice a change
//! that leaves the declarations alone. Behaviour is the host's own suite's
//! job, and reaching it needs the toolchain. This gate is the strongest thing
//! that rides the wall unconditionally, not a substitute for the one that does
//! not.

use std::path::Path;
use std::path::PathBuf;

use gandr_runtime_compile_host::host::ABI_VERSION;
use gandr_runtime_compile_host::image::CtorTag;
use gandr_runtime_compile_host::image::IMAGE_WIRE_VERSION;
use gandr_runtime_compile_host::image::MAX_IMAGE_NODES;
use gandr_runtime_compile_host::image::NodeKind;

/// The host's root, relative to this crate's manifest.
fn host_root() -> PathBuf
{
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates = manifest.parent().expect("the crate sits under crates/");
    let workspace = crates.parent().expect("crates/ sits under the workspace");
    workspace.join("runtime").join("compile-host")
}

/// Reads one of the host's source files.
fn host_source(relative: &str) -> String
{
    let path = host_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is unreadable: {error}", path.display()))
}

/// Asserts that a source file contains a line, after trimming.
fn assert_declares(
    source: &str,
    declaration: &str,
    what: &str,
)
{
    let found = source.lines().any(|line| line.trim() == declaration.trim());
    assert!(
        found,
        "the host no longer declares {what}: expected `{declaration}`"
    );
}

/// The heap's reserved prefix is what this crate assumes it is.
///
/// The bridge reads the exhaustion flag's meaning out of the boundary rather
/// than the heap, but the reserved-prefix width is arithmetic the bounds
/// witness does on the Rust side, so a move here would silently shift the
/// boundary case it tests.
#[test]
fn the_heap_layout_is_what_the_bridge_assumes()
{
    let source = host_source("include/gandr/compile_host/value.hpp");
    for (offset, field) in [
        (0, "bump_cursor"),
        (1, "duplication_ledger"),
        (2, "discard_ledger"),
        (3, "exhaustion_flag"),
        (4, "arena_base"),
    ] {
        assert_declares(
            &source,
            &alloc::format!("static constexpr std::size_t {field} = {offset};"),
            &alloc::format!("the heap's {field} at word {offset}"),
        );
    }
}

/// The cell tags keep the numbering the compiled code and the interpreter
/// hard-code, and which the rendered value grammar depends on.
#[test]
fn the_cell_tag_numbering_is_unchanged()
{
    let source = host_source("include/gandr/compile_host/value.hpp");
    for (value, tag) in [(0, "Int"), (1, "Unit"), (2, "Pair"), (3, "Inl"), (4, "Inr")] {
        assert_declares(
            &source,
            &alloc::format!("{tag} = {value},"),
            &alloc::format!("the cell tag {tag} at {value}"),
        );
    }
}

/// The node kinds and constructor tags keep the numbering this crate writes
/// into the wire form.
#[test]
fn the_wire_numbering_matches_this_crates_mirror()
{
    let source = host_source("include/gandr/compile_host/image.hpp");

    for kind in [
        NodeKind::Lit,
        NodeKind::Var,
        NodeKind::Ctor,
        NodeKind::Dup,
        NodeKind::Drop,
        NodeKind::Bind,
        NodeKind::Case,
        NodeKind::Cut,
    ] {
        let byte = u8::from(kind.wire_byte());
        let name = alloc::format!("{kind:?}");
        assert_declares(
            &source,
            &alloc::format!("{name} = {byte},"),
            &alloc::format!("the node kind {name} at {byte}"),
        );
    }

    for tag in [CtorTag::Unit, CtorTag::Pair, CtorTag::Inl, CtorTag::Inr] {
        let byte = u8::from(tag.wire_byte());
        let name = alloc::format!("{tag:?}");
        assert_declares(
            &source,
            &alloc::format!("{name} = {byte},"),
            &alloc::format!("the constructor tag {name} at {byte}"),
        );
    }
}

/// Each constructor tag declares the arity this crate emits fields for.
#[test]
fn the_constructor_arities_match_this_crates_mirror()
{
    let source = host_source("include/gandr/compile_host/image.hpp");
    let arities = source
        .split("constexpr std::uint32_t ctor_arity")
        .nth(1)
        .expect("the host declares ctor_arity");

    // The arity table is a switch whose arms fall through for the two
    // injections, so the check reads the arm bodies in order rather than
    // matching each tag to a literal.
    let returns: Vec<&str> = arities
        .lines()
        .take_while(|line| !line.contains("ctor_tag_name"))
        .filter_map(|line| line.trim().strip_prefix("return "))
        .filter_map(|value| value.strip_suffix(';'))
        .collect();

    assert_eq!(
        returns,
        alloc::vec!["0", "2", "1", "0"],
        "the host's arity table changed shape; this crate emits \
         {} fields for a pair and {} for an injection",
        usize::from(CtorTag::Pair.arity()),
        usize::from(CtorTag::Inl.arity())
    );
    assert_eq!(usize::from(CtorTag::Unit.arity()), 0);
    assert_eq!(usize::from(CtorTag::Pair.arity()), 2);
    assert_eq!(usize::from(CtorTag::Inl.arity()), 1);
    assert_eq!(usize::from(CtorTag::Inr.arity()), 1);
}

/// The wire version and the arena bound are what this crate writes and
/// enforces.
#[test]
fn the_wire_version_and_arena_bound_are_unchanged()
{
    let image = host_source("include/gandr/compile_host/image.hpp");
    assert_declares(
        &image,
        &alloc::format!("inline constexpr std::size_t max_image_nodes = {MAX_IMAGE_NODES};"),
        "the arena bound",
    );

    let decode = host_source("src/decode.cpp");
    assert_declares(
        &decode,
        &alloc::format!("constexpr std::uint8_t image_version = {IMAGE_WIRE_VERSION};"),
        "the wire version",
    );
}

/// The boundary's version and status constants are what this crate speaks.
#[test]
fn the_boundary_version_and_statuses_are_unchanged()
{
    let source = host_source("include/gandr/compile_host/abi.h");
    assert_declares(
        &source,
        &alloc::format!("#define GANDR_COMPILE_HOST_ABI_VERSION {ABI_VERSION}u"),
        "the boundary version",
    );

    for (value, name) in [
        (0, "OK"),
        (1, "MALFORMED_IMAGE"),
        (2, "VERIFIER_REJECTED"),
        (3, "LOWERING_FAILED"),
        (4, "CONVERSION_FAILED"),
        (5, "EXECUTION_FAILED"),
        (6, "RESULT_UNREADABLE"),
        (7, "LIMIT_EXCEEDED"),
        (8, "FIXTURE_UNREADABLE"),
        (100, "BAD_CALL"),
    ] {
        assert_declares(
            &source,
            &alloc::format!("#define GANDR_COMPILE_HOST_STATUS_{name} {value}"),
            &alloc::format!("the status {name} at {value}"),
        );
    }
}

/// The boundary struct's fields are in the order and the widths this crate's
/// mirror declares.
///
/// A field reordered on one side and not the other is the failure a dynamic
/// boundary has no other symptom for: the call succeeds and the numbers are
/// wrong.
#[test]
fn the_boundary_struct_layout_is_unchanged()
{
    let source = host_source("include/gandr/compile_host/abi.h");
    let declaration = source
        .split("typedef struct GandrCompileHostOutcome")
        .nth(1)
        .expect("the host declares the outcome struct");
    let fields: Vec<&str> = declaration
        .lines()
        .take_while(|line| !line.contains("GandrCompileHostOutcome;"))
        .filter_map(|line| line.trim().strip_suffix(';'))
        .filter(|line| !line.starts_with('/') && !line.starts_with('*'))
        .collect();

    assert_eq!(
        fields,
        alloc::vec![
            "int32_t status",
            "int64_t duplications",
            "int64_t discards",
            "uint64_t allocated_words",
            "const char* text",
        ],
        "the boundary struct's fields changed; this crate's RawOutcome mirrors them positionally"
    );
}

/// The verifier wall still opens every pipeline entry.
///
/// The proving spike's first measured price was that arity moves from a
/// declaration to a checking pass someone has to run. This holds the pass in
/// place at the source level: the optimization entry's first statement is the
/// verification, and the lowering entry reaches it through that.
#[test]
fn the_verifier_still_opens_the_pipeline()
{
    let source = host_source("src/pipeline.cpp");

    let optimize = source
        .split("Expected<void> optimize_module")
        .nth(1)
        .expect("the host declares optimize_module");
    let verification = optimize
        .find("verify_module(module)")
        .expect("optimize_module verifies");
    let first_pass = optimize.find("manager.addPass").unwrap_or(usize::MAX);
    assert!(
        verification < first_pass,
        "a pass now runs before the verifier in optimize_module"
    );

    let lower = source
        .split("Expected<void> lower_module")
        .nth(1)
        .expect("the host declares lower_module");
    let optimization = lower
        .find("optimize_module(module, optimization)")
        .expect("lower_module goes through optimize_module");
    let structural = lower
        .find("lower_dialect_operations(module)")
        .expect("lower_module lowers the dialect");
    assert!(
        optimization < structural,
        "lowering now runs before the verified optimization stage"
    );
}

/// Duplication and discard still declare memory effects rather than purity.
///
/// The spike's second measured price was that these survive canonicalization
/// only as a trait decision, and that nothing checks it. The host's own suite
/// checks the behaviour; this checks the declaration, which is the half that
/// can be checked without the toolchain.
#[test]
fn the_grade_operations_still_declare_their_effects()
{
    let source = host_source("src/dialect/GandrOps.td");
    for operation in ["Gandr_DupOp", "Gandr_DropOp"] {
        let declaration = source
            .split(&alloc::format!("def {operation} :"))
            .nth(1)
            .unwrap_or_else(|| panic!("the host declares {operation}"));
        let traits = declaration
            .split_once('{')
            .map(|(head, _)| head)
            .unwrap_or(declaration);
        assert!(
            traits.contains("MemoryEffects<[MemRead, MemWrite]"),
            "{operation} no longer declares memory effects; a canonicalization may now delete accounted work"
        );
        assert!(
            !traits.contains("Pure"),
            "{operation} is declared pure; accounted work would be deleted silently"
        );
    }
}

/// The agreement fixture names exactly the programs this crate drives.
#[test]
fn the_agreement_fixture_names_this_crates_programs()
{
    let fixture = host_source("fixtures/positive-core-samples.txt");
    let named: Vec<&str> = fixture
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split('\t').next())
        .collect();

    let driven: Vec<&str> = crate::programs::named()
        .iter()
        .map(|program| program.name)
        .collect();
    assert_eq!(
        named, driven,
        "the fixture and this crate name different programs"
    );
}
