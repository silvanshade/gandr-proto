//! The script-runner face, driven through the real `gandr` binary.
//!
//! Every case here spawns the built driver, because the contract under test is
//! a *process* contract: what a calling shell — a `#!/usr/bin/env gandr`
//! script, a `mise` task, a future build step — observes on standard output, on
//! standard error, and in the exit status.

#[cfg(test)]
mod tests
{
    use std::path::Path;
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

    /// A scratch directory that removes itself when the test drops it.
    #[repr(transparent)]
    struct ScratchDirectory
    {
        /// The path the directory was created at.
        path: PathBuf,
    }

    impl ScratchDirectory
    {
        /// Create a fresh empty directory at a path named after `label`,
        /// clearing any residue a previous run of this process id left behind.
        fn create<'text, Label>(label: Label) -> Self
        where
            Label: Into<TestText<'text>>,
        {
            let label = label.into().0;
            let suffix = NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gandr-driver-cli-{label}-{}-{suffix}",
                std::process::id()
            ));
            drop(std::fs::remove_dir_all(&path));
            // `create_dir` (not `create_dir_all`) is deliberate, as it is in the
            // host's own `Fs::tempdir`: its fail-on-exists is what leaves the
            // directory provably fresh, and an empty search path is this
            // fixture's whole guarantee — `create_dir_all` would accept a
            // pre-existing directory with anything at all in it.
            #[expect(
                clippy::create_dir,
                reason = "the fixture needs create_dir's fail-on-exists to guarantee an EMPTY \
                          search path"
            )]
            let created = std::fs::create_dir(&path);
            created.expect("the scratch directory must be creatable");
            Self { path }
        }
    }

    impl Drop for ScratchDirectory
    {
        fn drop(&mut self)
        {
            drop(std::fs::remove_dir_all(&self.path));
        }
    }

    /// The exact filename used by the fixture source on this target.
    fn fixture_library_name() -> PathBuf
    {
        if cfg!(target_os = "windows") {
            PathBuf::from("testlib.dll")
        }
        else if cfg!(target_os = "macos") {
            PathBuf::from("testlib.dylib")
        }
        else {
            PathBuf::from("testlib.so")
        }
    }

    /// Copy the build-script-produced fixture under the source's exact path.
    fn copy_fixture(fixture_dir: &Path)
    {
        std::fs::copy(
            gandr_runtime_ffi::hermetic_testlib_path(),
            fixture_dir.join(fixture_library_name()),
        )
        .expect("the FFI fixture library must be copied");
    }

    /// Rewrite the corpus source to the copied target-specific fixture path.
    fn native_ffi_script() -> String
    {
        include_str!("../../surface-corpus/examples/model/21-ffi-native-call.gandr").replace(
            "\"./testlib\"",
            &format!("\"./{}\"", fixture_library_name().display()),
        )
    }

    /// The command that runs the built driver with `arguments`.
    fn driver<Arguments>(arguments: Arguments) -> Command
    where
        Arguments: IntoIterator,
        Arguments::Item: AsRef<std::ffi::OsStr>,
    {
        let mut command = Command::new(env!("CARGO_BIN_EXE_gandr"));
        command.args(arguments);
        command
    }

    /// Run the built driver with `arguments` and capture its output.
    fn drive<Arguments>(arguments: Arguments) -> Output
    where
        Arguments: IntoIterator,
        Arguments::Item: AsRef<std::ffi::OsStr>,
    {
        driver(arguments)
            .output()
            .expect("the built driver must be spawnable")
    }

    /// Run the built driver with `arguments`, with `tools` as the whole of the
    /// search path a tool the script runs is looked up in.
    ///
    /// A driven tool is spawned by name, so which program that name reaches is
    /// decided by the driver's own `PATH`. Handing the driver one directory the
    /// test owns makes the tool surface of the run a fixture rather than a
    /// property of the machine, which is what lets a case pin what happens when
    /// a name resolves to nothing.
    fn drive_with_tools<Arguments>(
        arguments: Arguments,
        tools: &ScratchDirectory,
    ) -> Output
    where
        Arguments: IntoIterator,
        Arguments::Item: AsRef<std::ffi::OsStr>,
    {
        driver(arguments)
            .env("PATH", &tools.path)
            .output()
            .expect("the built driver must be spawnable")
    }

    /// Run the built driver from a fixture directory.
    fn drive_in<Arguments>(
        directory: &std::path::Path,
        arguments: Arguments,
    ) -> Output
    where
        Arguments: IntoIterator,
        Arguments::Item: AsRef<std::ffi::OsStr>,
    {
        Command::new(env!("CARGO_BIN_EXE_gandr"))
            .current_dir(directory)
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
    fn lsp_capabilities_prints_the_token_legend()
    {
        let output = drive(["lsp", "--capabilities"]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(0_i32)),
            "the capabilities smoke path leaves successfully"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("\"keyword\""),
            "the smoke path must name the token legend; got {stdout}"
        );
        assert!(
            stdout.contains("semanticTokensProvider"),
            "the smoke path must advertise semantic tokens; got {stdout}"
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
    fn the_native_ffi_corpus_script_runs_through_the_cli()
    {
        let fixture_dir = std::env::temp_dir().join(format!(
            "gandr-driver-ffi-fixture-{}-{}",
            std::process::id(),
            NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&fixture_dir).expect("the FFI fixture directory must be creatable");
        copy_fixture(&fixture_dir);
        let script_text = native_ffi_script();
        let script = ScratchScript::write("ffi", script_text.as_str());
        let output = drive_in(&fixture_dir, [&script.path]);
        drop(std::fs::remove_dir_all(&fixture_dir));
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(0_i32)),
            "the hermetic FFI script must complete successfully"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("42") && stdout.contains('5'),
            "the native result must reach the CLI caller; got {stdout}"
        );
    }

    #[test]
    fn an_ffi_library_load_failure_is_a_failed_run()
    {
        let script = ScratchScript::write(
            "ffi-load-failure",
            "extern \"c\" from \"missing_library\" {\n  def missing() -> i32;\n}\n\nmissing_library.missing()\n",
        );
        let output = drive([&script.path]);
        assert_eq!(status_of(&output), ProcessStatus(Some(1_i32)));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("could not be loaded"),
            "loader failures are host failures: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn an_ffi_missing_symbol_is_a_failed_run()
    {
        let fixture_dir = std::env::temp_dir().join(format!(
            "gandr-driver-ffi-symbol-{}-{}",
            std::process::id(),
            NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&fixture_dir).expect("the FFI fixture directory must be creatable");
        copy_fixture(&fixture_dir);
        let script = ScratchScript::write(
            "ffi-symbol-failure",
            format!(
                "extern \"c\" from \"./{}\" as testlib {{\n  def missing() -> i32;\n}}\n\ntestlib.missing()\n",
                fixture_library_name().display()
            )
            .as_str(),
        );
        let output = drive_in(&fixture_dir, [&script.path]);
        drop(std::fs::remove_dir_all(&fixture_dir));
        assert_eq!(status_of(&output), ProcessStatus(Some(1_i32)));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("ffi symbol `missing`"),
            "symbol failures are host symbol failures: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn an_ffi_null_returned_string_is_a_failed_run()
    {
        let fixture_dir = std::env::temp_dir().join(format!(
            "gandr-driver-ffi-string-{}-{}",
            std::process::id(),
            NEXT_SCRATCH.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&fixture_dir).expect("the FFI fixture directory must be creatable");
        copy_fixture(&fixture_dir);
        let script = ScratchScript::write(
            "ffi-string-failure",
            format!(
                "extern \"c\" from \"./{}\" as testlib {{\n  def gandr_null_string() -> CStr;\n}}\n\ntestlib.gandr_null_string()\n",
                fixture_library_name().display()
            )
            .as_str(),
        );
        let output = drive_in(&fixture_dir, [&script.path]);
        drop(std::fs::remove_dir_all(&fixture_dir));
        assert_eq!(status_of(&output), ProcessStatus(Some(1_i32)));
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("invalid returned C string"),
            "invalid returned strings are host failures: {}",
            String::from_utf8_lossy(&output.stderr)
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
    fn an_ill_typed_script_is_refused_by_the_checker()
    {
        // The source parses and links, so only the checker's refusal stands
        // between it and the machine: a checker failure is a refusal (2), the
        // status a run failure (1) must never take, because nothing ran.
        let script = ScratchScript::write("ill-typed", "{ ret (\"five\" : Integer) }\n");
        let output = drive([&script.path]);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(2_i32)),
            "an ill-typed script never reaches the machine"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("type checking failed"),
            "the refusal names the checker as its source; got {stderr}"
        );
        assert!(
            output.stdout.is_empty(),
            "a refused script routes no result; got {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }

    #[test]
    fn a_script_whose_tool_cannot_spawn_leaves_with_a_failure_status()
    {
        // A spawn the host cannot perform is a fatal host failure, not a
        // representable reply: the run aborts, so the status is a failure (1)
        // and the caller reads the host error, distinct from a blame and from
        // a refusal.
        //
        // The tool is unspawnable by construction rather than by hope: the
        // driver runs with a fresh empty directory as the whole of its search
        // path, so the name reaches no program whatever this machine has
        // installed. Naming a plausibly-absent binary and trusting the ambient
        // path would make the case a claim about the developer's machine, and
        // one `cargo install` away from testing the opposite thing.
        let tools = ScratchDirectory::create("no-tools");
        let script = ScratchScript::write(
            "unspawnable",
            "{\n  run r <- #!{ gandr-absent-tool };\n  ret r\n}\n",
        );
        let output = drive_with_tools([&script.path], &tools);
        assert_eq!(
            status_of(&output),
            ProcessStatus(Some(1_i32)),
            "a fatal host failure is a run failure, not a refusal"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("could not spawn"),
            "the caller reads the host's own error; got {stderr}"
        );
        assert!(
            stderr.contains("gandr-absent-tool"),
            "the host error names the tool that would not spawn, so the failure \
             is this script's and not some other spawn's; got {stderr}"
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
