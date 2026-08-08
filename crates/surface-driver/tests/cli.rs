//! The script-runner face, driven through the real `gandr` binary.
//!
//! Every case here spawns the built driver, because the contract under test is
//! a *process* contract: what a calling shell — a `#!/usr/bin/env gandr`
//! script, a `mise` task, a future build step — observes on standard output, on
//! standard error, and in the exit status.
//!
//! Two further cases anchor the project's own production script,
//! `scripts/agda-deps.gandr`, without running it: it clones a repository over
//! the network, so the suite proves the script lowers and that it has not
//! drifted from the literate corpus copy that explains it, and leaves execution
//! to `mise run agda:deps`.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

use gandr_surface_engine::boundary::PipelineSource;
use gandr_surface_engine::lower::lower_source_total;

/// A per-process suffix source keeping concurrent scratch scripts distinct.
static NEXT_SCRATCH: AtomicU32 = AtomicU32::new(0);

/// Borrowed text crossing a test-helper boundary.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct TestText<'text>(&'text str);

impl<'text> From<&'text str> for TestText<'text>
{
    fn from(value: &'text str) -> Self
    {
        Self(value)
    }
}

/// A scratch script file that removes itself when the test drops it.
#[repr(transparent)]
struct ScratchScript
{
    /// The path the script text was written to.
    path: PathBuf,
}

impl ScratchScript
{
    /// Write `text` to a fresh scratch path named after `label`.
    fn write<'text, Label, Text>(
        label: Label,
        text: Text,
    ) -> Self
    where
        Label: Into<TestText<'text>>,
        Text: Into<TestText<'text>>,
    {
        let label = label.into().0;
        let suffix = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "gandr-driver-cli-{label}-{}-{suffix}.gandr",
            std::process::id()
        ));
        std::fs::write(&path, text.into().0).expect("the scratch script must be writable");
        Self { path }
    }
}

impl Drop for ScratchScript
{
    fn drop(&mut self)
    {
        drop(std::fs::remove_file(&self.path));
    }
}

/// Run the built driver with `arguments` and capture its output.
fn drive<Arguments>(arguments: Arguments) -> Output
where
    Arguments: IntoIterator,
    Arguments::Item: AsRef<std::ffi::OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_gandr"))
        .args(arguments)
        .output()
        .expect("the built driver must be spawnable")
}

/// The exit status the driver reported, as a comparable integer.
fn status_of(output: &Output) -> Option<i32>
{
    output.status.code()
}

/// The workspace root, reached from this crate's manifest directory.
fn workspace_root() -> PathBuf
{
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The lowered core rendering of a source file — the well-formedness shape.
///
/// Comments and layout are stripped by lowering, so this is what the file
/// actually is as a program.
fn lowered_shape(path: &Path) -> String
{
    let text = read_source(path);
    let lowered = lower_source_total(PipelineSource(text.as_str()))
        .unwrap_or_else(|error| panic!("`{}` must lower totally: {error}", path.display()));
    let terms: Vec<String> = lowered
        .items
        .iter()
        .map(|item| format!("{:?}", item.term))
        .collect();
    terms.join("\n")
}

/// The source text of a file the suite anchors.
fn read_source(path: &Path) -> String
{
    std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read `{}`: {error}", path.display()))
}

/// The single `sh -c` command line a provisioning source carries.
///
/// This is the load-bearing payload the production script and its literate
/// corpus copy must agree on: everything else about the two files is wrapping,
/// and the command is what drifted between them historically.
fn shell_command_line(path: &Path) -> String
{
    let text = read_source(path);
    let mut lines = text.lines().filter(|line| line.starts_with("sh -c "));
    let command = lines
        .next()
        .unwrap_or_else(|| panic!("`{}` must carry an `sh -c` command", path.display()))
        .to_owned();
    assert!(
        lines.next().is_none(),
        "`{}` must carry exactly one `sh -c` command",
        path.display()
    );
    command
}

#[test]
fn no_argument_prints_usage_and_refuses()
{
    let output = drive::<[&str; 0]>([]);
    assert_eq!(
        status_of(&output),
        Some(2),
        "a missing operand is a refusal"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("usage: gandr <file>"),
        "a refusal must print the usage text; got {stderr}"
    );
}

#[test]
fn help_prints_usage_and_leaves_successfully()
{
    let output = drive(["--help"]);
    assert_eq!(
        status_of(&output),
        Some(0),
        "asking for help is not an error"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("usage: gandr <file>"),
        "help must print the usage text; got {stderr}"
    );
}

#[test]
fn a_second_operand_is_refused()
{
    let output = drive(["one.gandr", "two.gandr"]);
    assert_eq!(
        status_of(&output),
        Some(2),
        "the script-runner face takes exactly one operand"
    );
}

#[test]
fn an_unknown_flag_is_refused()
{
    let output = drive(["--repl"]);
    assert_eq!(
        status_of(&output),
        Some(2),
        "a deferred face must be refused, never read as a path"
    );
}

#[test]
fn a_script_that_returns_a_value_leaves_successfully()
{
    let script = ScratchScript::write("value", "{ ret 40 + 2 }\n");
    let output = drive([&script.path]);
    assert_eq!(status_of(&output), Some(0), "a completed run succeeds");
    assert!(
        output.stdout.is_empty(),
        "a successful run prints nothing of its own; got {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn a_scripts_shell_output_is_captured_for_the_program_and_not_relayed()
{
    // A `#!{ … }` block always lowers with the captured spawn mode, and no
    // source-level surface reaches the inheriting one or writes to standard
    // output. So a script's command output is a value the PROGRAM can compute
    // over and is invisible to the CALLER. That is the current contract, pinned
    // here rather than left to be rediscovered: a long-running command under
    // `gandr <file>` is silent until it finishes.
    //
    // Exit 3 witnesses that the reply carried the text; an empty standard
    // output witnesses that the caller never saw it. When a printing capability
    // lands, this case fails and states exactly what changed.
    let script = ScratchScript::write(
        "output",
        "{\n  run said <- #!{ printf 'ran under the driver\\n'; };\n  run code <- (if \
         string.contains(said.stdout, \"under the driver\") { ret 3 } else { ret 4 } : F \
         Integer);\n  proc.exit(code)\n}\n",
    );
    let output = drive([&script.path]);
    assert_eq!(
        status_of(&output),
        Some(3),
        "the program must be able to compute over its command's captured output"
    );
    assert!(
        output.stdout.is_empty(),
        "a captured command's output does not reach the caller; got {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn a_script_that_exits_leaves_with_its_own_status()
{
    let script = ScratchScript::write("exit", "{ proc.exit(7) }\n");
    let output = drive([&script.path]);
    assert_eq!(
        status_of(&output),
        Some(7),
        "`proc.exit` must reach the calling shell as the process status"
    );
}

#[test]
fn a_script_that_blames_leaves_with_a_failure_status()
{
    let script = ScratchScript::write("blame", "{ ?hole }\n");
    let output = drive([&script.path]);
    assert_eq!(
        status_of(&output),
        Some(1),
        "a run that reaches a blame is a failure, not a success"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("blamed"),
        "a blame must be reported; got {stderr}"
    );
}

#[test]
fn an_absent_script_is_refused_by_path()
{
    let absent = std::env::temp_dir().join(format!(
        "gandr-driver-cli-absent-{}.gandr",
        std::process::id()
    ));
    drop(std::fs::remove_file(&absent));
    let output = drive([&absent]);
    assert_eq!(
        status_of(&output),
        Some(2),
        "a source that never reached the machine is a refusal, not a run failure"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("gandr-driver-cli-absent"),
        "a read refusal must name the path; got {stderr}"
    );
}

#[test]
fn a_script_with_no_program_is_refused()
{
    let script = ScratchScript::write("no-program", "def only = 1;\n");
    let output = drive([&script.path]);
    assert_eq!(
        status_of(&output),
        Some(2),
        "a source with no runnable item never reaches the machine"
    );
}

#[test]
fn the_agda_deps_script_lowers_and_reaches_the_host()
{
    let script = workspace_root().join("scripts/agda-deps.gandr");
    let shape = lowered_shape(&script);
    assert!(
        shape.contains("Exec"),
        "the provisioning script must reach the host `Exec` effect; got {shape}"
    );
    assert!(
        shape.contains("Proc"),
        "the provisioning script must propagate its status through `proc.exit`; got {shape}"
    );
}

#[test]
fn the_agda_deps_script_and_its_corpus_walkthrough_have_not_drifted()
{
    // The corpus copy explains the production script, and the two drifted apart
    // once already — the copy was left behind at an older shape while the real
    // script moved. This is what keeps them together. It compares the `sh -c`
    // command, not the whole program: the copy deliberately stops before the
    // real script's `proc.exit` status propagation, because the copy's lowering
    // is a frozen byte anchor for the kernel manifests. That one declared
    // difference is stated in both files; anything else is drift.
    let root = workspace_root();
    let script = root.join("scripts/agda-deps.gandr");
    let walkthrough =
        root.join("crates/surface-corpus/examples/model/14-agda-deps-walkthrough.gandr");
    assert_eq!(
        shell_command_line(&script),
        shell_command_line(&walkthrough),
        "the literate corpus copy must carry the production script's command verbatim"
    );
}
