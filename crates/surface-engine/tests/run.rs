//! The one-shot source-program driver and its script (path-shaped) face.
//!
//! [`gandr_surface_engine::run::run_source`] is the language-level source entry
//! point and [`gandr_surface_engine::run::run_source_file`] is the seam the
//! `gandr <file>` script runner stands on. These suites pin the three outcomes
//! the file face separates — the read failure, the source failure, and the
//! successful run — plus the shebang line every executable gandr script opens
//! with.

use alloc::string::String;
use alloc::string::ToString as _;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;
use std::path::PathBuf;

use gandr_runtime_effects::ShellOutcome;
use gandr_surface_engine::run::RunFileError;
use gandr_surface_engine::run::run_source;
use gandr_surface_engine::run::run_source_file;

use crate::common::TestText;

/// A per-process suffix source keeping concurrent scratch files distinct.
static NEXT_SCRATCH: AtomicU32 = AtomicU32::new(0);

/// A scratch source file that removes itself when the test drops it.
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
            "gandr-surface-engine-run-{label}-{}-{suffix}.gandr",
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

/// The returned value of a completed run, rendered structurally for assertions.
fn returned_text(outcome: &ShellOutcome) -> String
{
    match *outcome {
        | ShellOutcome::Completed(ref eval) => format!("{eval:?}"),
        | ShellOutcome::Exited { code } => format!("exited {code}"),
        | ShellOutcome::HostFailed(ref error) => format!("host failed: {error}"),
    }
}

#[test]
fn run_source_runs_source_text()
{
    let outcome = run_source("{ ret 40 + 2 }").expect("the source must prepare for a run");
    let rendered = returned_text(&outcome);
    assert!(
        rendered.contains("42"),
        "a completed run must return the computed value; got {rendered}"
    );
}

#[test]
fn run_source_file_runs_a_script_file()
{
    let script = ScratchScript::write("value", "{ ret 40 + 2 }\n");
    let outcome = run_source_file(&script.path).expect("the script file must prepare for a run");
    let rendered = returned_text(&outcome);
    assert!(
        rendered.contains("42"),
        "a script file must run exactly as its source text does; got {rendered}"
    );
}

#[test]
fn run_source_file_accepts_an_executable_shebang_line()
{
    let script = ScratchScript::write("shebang", "#!/usr/bin/env gandr\n{ ret 40 + 2 }\n");
    let outcome = run_source_file(&script.path).expect("a shebang line must be grammar trivia");
    let rendered = returned_text(&outcome);
    assert!(
        rendered.contains("42"),
        "a shebang line must not change what a script computes; got {rendered}"
    );
}

#[test]
fn run_source_file_reports_the_path_of_an_absent_file()
{
    let absent = std::env::temp_dir().join(format!(
        "gandr-surface-engine-run-absent-{}.gandr",
        std::process::id()
    ));
    drop(std::fs::remove_file(&absent));
    let error = run_source_file(&absent).expect_err("an absent file must not run");
    assert!(
        matches!(error, RunFileError::Read { .. }),
        "an unreadable file must surface as a read failure; got {error:?}"
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("gandr-surface-engine-run-absent"),
        "a read failure must name the path it could not read; got {rendered}"
    );
}

#[test]
fn run_source_file_surfaces_a_source_failure_unchanged()
{
    let script = ScratchScript::write("no-program", "def only = 1;\n");
    let error = run_source_file(&script.path).expect_err("a source with no program must not run");
    assert!(
        matches!(error, RunFileError::Run(_)),
        "a source-level failure must surface as a run failure; got {error:?}"
    );
    let rendered = error.to_string();
    assert!(
        rendered.contains("no final runnable program"),
        "the wrapped run error must reach the caller unchanged; got {rendered}"
    );
}
