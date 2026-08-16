//! The script-runner face, driven through the real `gandr` binary.
//!
//! Every case here spawns the built driver, because the contract under test is
//! a *process* contract: what a calling shell — a `#!/usr/bin/env gandr`
//! script, a `mise` task, a future build step — observes on standard output, on
//! standard error, and in the exit status.

#[cfg(test)]
mod tests
{
    use std::path::PathBuf;
    use std::process::Command;
    use std::process::Output;
    use std::sync::atomic::AtomicU32;
    use std::sync::atomic::Ordering;

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

    /// The exit status a driven process left with, absent when it was
    /// signalled.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ProcessStatus(Option<i32>);

    /// The exit status the driver reported.
    fn status_of(output: &Output) -> ProcessStatus
    {
        ProcessStatus(output.status.code())
    }

    #[test]
    fn no_argument_prints_usage_and_refuses()
    {
        let output = drive::<[&str; 0]>([]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(2_i32)),
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
            ProcessStatus(Some(0_i32)),
            "asking for help is not an error"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("usage: gandr <file>"),
            "help was asked for, so it belongs on standard output; got {stdout}"
        );
        assert!(
            output.stderr.is_empty(),
            "a satisfied request writes no complaint; got {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn a_bare_dash_is_a_path_not_standard_input()
    {
        // There is no standard-input face for `-` to mean, so it is an ordinary
        // (and almost certainly absent) filename. Pinned because the flag guard
        // exempts it deliberately, and an exemption with no test reads as one.
        let output = drive(["-"]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(2_i32)),
            "a bare dash is a path, so it fails as a missing file"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("cannot read `-`"),
            "the refusal must name the path it tried; got {stderr}"
        );
    }

    #[test]
    fn a_deferred_subcommand_name_is_read_as_a_path()
    {
        // `tui` carries no leading dash, so the flag guard does not see it and
        // it is taken as a filename. Honest only while no subcommand exists;
        // the first one to land must replace this with a real command table,
        // and this case is what will fail when it does.
        let output = drive(["tui"]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(2_i32)),
            "a deferred face name is refused, though as a missing file"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("cannot read `tui`"),
            "the refusal names it as a path, not as a subcommand; got {stderr}"
        );
    }

    #[test]
    fn a_negative_exit_code_wraps_the_way_a_shell_wraps_it()
    {
        let script = ScratchScript::write("exit-negative", "{ proc.exit(-1) }\n");
        let output = drive([&script.path]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(255_i32)),
            "`exit -1` leaves 255, as it does from a shell"
        );
    }

    #[test]
    fn an_out_of_range_exit_code_is_reduced_to_a_byte()
    {
        let script = ScratchScript::write("exit-wide", "{ proc.exit(300) }\n");
        let output = drive([&script.path]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(44_i32)),
            "a code wider than a byte is reduced modulo 256"
        );
    }

    #[test]
    fn a_second_operand_is_refused()
    {
        let output = drive(["one.gandr", "two.gandr"]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(2_i32)),
            "the script-runner face takes exactly one operand"
        );
    }

    #[test]
    fn an_unknown_flag_is_refused()
    {
        let output = drive(["--repl"]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(2_i32)),
            "a deferred face must be refused, never read as a path"
        );
    }

    #[test]
    fn a_script_that_returns_a_value_leaves_successfully()
    {
        let script = ScratchScript::write("value", "{ ret 40 + 2 }\n");
        let output = drive([&script.path]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(0_i32)),
            "a completed run succeeds"
        );
        assert_eq!(
            output.stdout, b"Int(42)\n",
            "a completed run routes its returned result to the caller"
        );
    }

    #[test]
    fn a_script_routes_its_result_after_captured_tool_calls()
    {
        // Two host calls must each resume their own continuation; the second
        // result is the returned value routed to the caller.
        let script = ScratchScript::write(
            "output",
            "{\n  run first <- #!{ printf 'first\\n'; };\n  run second <- #!{ \
             printf 'second\\n'; };\n  ret second.stdout\n}\n",
        );
        let output = drive([&script.path]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(0_i32)),
            "the program must complete after multiple captured tool calls"
        );
        assert_eq!(
            output.stdout, b"Str(\"second\\n\")\n",
            "the caller receives the owning second result, not the first tool result"
        );
    }

    #[test]
    fn a_script_that_exits_leaves_with_its_own_status()
    {
        let script = ScratchScript::write("exit", "{ proc.exit(7) }\n");
        let output = drive([&script.path]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(7_i32)),
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
            ProcessStatus(Some(1_i32)),
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
            ProcessStatus(Some(2_i32)),
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
            ProcessStatus(Some(2_i32)),
            "a source with no runnable item never reaches the machine"
        );
    }
}
