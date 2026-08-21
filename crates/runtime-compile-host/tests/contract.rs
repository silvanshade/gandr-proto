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

/// One of the host's source files, read whole.
#[repr(transparent)]
struct HostSource(String);

/// A line the host is expected to declare.
#[repr(transparent)]
struct Declaration(String);

/// What a declaration is about, for the failure message.
#[repr(transparent)]
struct Subject(String);

/// A marker some section of a source file starts after.
#[repr(transparent)]
struct Marker(String);

/// The tail of a source file, from just past a marker.
#[repr(transparent)]
struct Section<'source>(&'source str);

impl AsRef<str> for Section<'_>
{
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

/// A string with every run of whitespace collapsed to one space.
fn collapse_whitespace(text: &str) -> alloc::string::String
{
    text.split_whitespace()
        .collect::<alloc::vec::Vec<_>>()
        .join(" ")
}

/// The host's root, relative to this crate's manifest.
fn host_root() -> PathBuf
{
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let crates = manifest.parent().expect("the crate sits under crates/");
    let workspace = crates.parent().expect("crates/ sits under the workspace");
    workspace.join("runtime").join("compile-host")
}

/// Reads one of the host's source files.
fn host_source(relative: &Path) -> HostSource
{
    let path = host_root().join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} is unreadable: {error}", path.display()));
    HostSource(text)
}

impl HostSource
{
    /// Asserts that some line of this file declares `declaration`.
    ///
    /// Both sides are compared with runs of whitespace collapsed. The host's
    /// sources are formatted by `runtime/.clang-format`, which aligns
    /// consecutive macro definitions and may change the spacing inside a
    /// declaration without changing what it declares; this gate is about the
    /// contract, and the format policy has a lane of its own.
    fn assert_declares(
        &self,
        declaration: &Declaration,
        what: &Subject,
    )
    {
        let expected = collapse_whitespace(&declaration.0);
        let found = self
            .0
            .lines()
            .any(|line| collapse_whitespace(line) == expected);
        assert!(
            found,
            "the host no longer declares {}: expected `{expected}`",
            what.0
        );
    }

    /// The text after the first occurrence of `marker`.
    fn after(
        &self,
        marker: &Marker,
    ) -> Section<'_>
    {
        let tail = self.0.split_once(marker.0.as_str()).map_or_else(
            || panic!("the host no longer holds `{}`", marker.0),
            |(_, tail)| tail,
        );
        Section(tail)
    }
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
    let source = host_source(Path::new("include/gandr/compile_host/value.hpp"));
    for (offset, field) in [
        (0_u8, "bump_cursor"),
        (1_u8, "duplication_ledger"),
        (2_u8, "discard_ledger"),
        (3_u8, "exhaustion_flag"),
        (4_u8, "arena_base"),
    ] {
        source.assert_declares(
            &Declaration(alloc::format!(
                "static constexpr std::size_t {field} = {offset};"
            )),
            &Subject(alloc::format!("the heap's {field} at word {offset}")),
        );
    }
}

/// The cell tags keep the numbering the compiled code and the interpreter
/// hard-code, and which the rendered value grammar depends on.
#[test]
fn the_cell_tag_numbering_is_unchanged()
{
    let source = host_source(Path::new("include/gandr/compile_host/value.hpp"));
    for (value, tag) in [
        (0_u8, "Int"),
        (1_u8, "Unit"),
        (2_u8, "Pair"),
        (3_u8, "Inl"),
        (4_u8, "Inr"),
    ] {
        source.assert_declares(
            &Declaration(alloc::format!("{tag} = {value},")),
            &Subject(alloc::format!("the cell tag {tag} at {value}")),
        );
    }
}

/// The node kinds and constructor tags keep the numbering this crate writes
/// into the wire form.
#[test]
fn the_wire_numbering_matches_this_crates_mirror()
{
    let source = host_source(Path::new("include/gandr/compile_host/image.hpp"));

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
        source.assert_declares(
            &Declaration(alloc::format!("{name} = {byte},")),
            &Subject(alloc::format!("the node kind {name} at {byte}")),
        );
    }

    for tag in [CtorTag::Unit, CtorTag::Pair, CtorTag::Inl, CtorTag::Inr] {
        let byte = u8::from(tag.wire_byte());
        let name = alloc::format!("{tag:?}");
        source.assert_declares(
            &Declaration(alloc::format!("{name} = {byte},")),
            &Subject(alloc::format!("the constructor tag {name} at {byte}")),
        );
    }
}

/// Each constructor tag declares the arity this crate emits fields for.
#[test]
fn the_constructor_arities_match_this_crates_mirror()
{
    let source = host_source(Path::new("include/gandr/compile_host/image.hpp"));
    let section = source.after(&Marker(String::from("ctor_arity(CtorTag tag)")));
    let arities: &str = section.as_ref();

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
        "the host's arity table changed shape; this crate emits {} fields for a pair and {} for \
         an injection",
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
    let image = host_source(Path::new("include/gandr/compile_host/image.hpp"));
    image.assert_declares(
        &Declaration(alloc::format!(
            "inline constexpr std::size_t max_image_nodes = {MAX_IMAGE_NODES};"
        )),
        &Subject(String::from("the arena bound")),
    );

    let decode = host_source(Path::new("src/decode.cpp"));
    decode.assert_declares(
        &Declaration(alloc::format!(
            "constexpr std::uint8_t image_version = {IMAGE_WIRE_VERSION};"
        )),
        &Subject(String::from("the wire version")),
    );
}

/// The boundary's version and status constants are what this crate speaks.
#[test]
fn the_boundary_version_and_statuses_are_unchanged()
{
    let source = host_source(Path::new("include/gandr/compile_host/abi.h"));
    source.assert_declares(
        &Declaration(alloc::format!(
            "#define GANDR_COMPILE_HOST_ABI_VERSION {ABI_VERSION}u"
        )),
        &Subject(String::from("the boundary version")),
    );

    for (value, name) in [
        (0_u8, "OK"),
        (1_u8, "MALFORMED_IMAGE"),
        (2_u8, "VERIFIER_REJECTED"),
        (3_u8, "LOWERING_FAILED"),
        (4_u8, "CONVERSION_FAILED"),
        (5_u8, "EXECUTION_FAILED"),
        (6_u8, "RESULT_UNREADABLE"),
        (7_u8, "LIMIT_EXCEEDED"),
        (8_u8, "FIXTURE_UNREADABLE"),
        (100_u8, "BAD_CALL"),
    ] {
        source.assert_declares(
            &Declaration(alloc::format!(
                "#define GANDR_COMPILE_HOST_STATUS_{name} {value}"
            )),
            &Subject(alloc::format!("the status {name} at {value}")),
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
    let source = host_source(Path::new("include/gandr/compile_host/abi.h"));
    let section = source.after(&Marker(String::from(
        "typedef struct GandrCompileHostOutcome",
    )));
    let declaration: &str = section.as_ref();
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
            "char const* text",
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
    let source = host_source(Path::new("src/pipeline.cpp"));

    let optimize_section = source.after(&Marker(String::from("optimize_module(mlir::ModuleOp")));
    let optimize: &str = optimize_section.as_ref();
    let verification = optimize
        .find("verify_module(module)")
        .expect("optimize_module verifies");
    let first_pass = optimize.find("manager.addPass").unwrap_or(usize::MAX);
    assert!(
        verification < first_pass,
        "a pass now runs before the verifier in optimize_module"
    );

    let lower_section = source.after(&Marker(String::from("lower_module(mlir::ModuleOp")));
    let lower: &str = lower_section.as_ref();
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
    let source = host_source(Path::new("src/dialect/GandrOps.td"));
    for operation in ["Gandr_DupOp", "Gandr_DropOp"] {
        let section = source.after(&Marker(alloc::format!("def {operation} :")));
        let declaration: &str = section.as_ref();
        let traits = declaration
            .split_once('{')
            .map_or(declaration, |(head, _)| head);
        assert!(
            traits.contains("MemoryEffects<[MemRead, MemWrite]"),
            "{operation} no longer declares memory effects; a canonicalization may now delete \
             accounted work"
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
    let fixture = host_source(Path::new("fixtures/positive-core-samples.txt"));
    let named: Vec<&str> = fixture
        .0
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
