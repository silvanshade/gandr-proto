//! The headless run driver (ADR-35 D4, proposal §3).
//!
//! [`run_program`] drives a lowered [`Comp`] under the shell host to a
//! [`ShellOutcome`]. The durable driver is the **L machine**: `run_program`
//! hands the program and a host handler to
//! [`gandr_core_sequent::machine::run_comp_with_host`] and reads back an
//! [`Eval`]. That entry offers every host-interceptable `perform` to the
//! handler over the ADR-35 D4 host-effect seam
//! ([`gandr_core_checker::host`]) — the same `(signature name, operation name,
//! payload)` projection the retired CEK oracle presented — and enforces the
//! same [`gandr_core_checker::outcome::StuckReason::StepLimit`] guard, so a
//! non-terminating program halts rather than hangs.
//!
//! # The seam adaptation
//!
//! The seam speaks only `Resume` / `Unhandled` ([`HostReply`]). The shell needs
//! two richer outcomes the seam cannot express: a run-truncating `Proc::exit`
//! and a fatal syscall abort. The driver-level entry runs to a terminal with no
//! stepwise loop to early-return through (unlike the CEK's owned step loop), so
//! [`ShellDriver`] captures those two as an *early outcome*: on `Proc::exit` or
//! a fatal `HostAction::Fail` it records the outcome and declines
//! ([`HostReply::Unhandled`]), which the machine turns into a terminal
//! `PerformNoHandler` blame, and [`run_program`] surfaces the recorded early
//! outcome in place of that blamed [`Eval`]. A resumed operation flows straight
//! through as [`HostReply::Resume`].
//!
//! # The source entry
//!
//! `run_source` — the CST → core lowering convenience that runs the pipeline
//! lowerer before handing the program to the host loop — lives in the surface
//! engine (`gandr_surface_engine::run::run_source`), which composes its
//! lowering, linking, and prelude checking with [`run_program_with_prelude`];
//! only the hand-built [`Comp`] entries land on the L-machine seam here.

use gandr_core_checker::boundary::OperationName;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::host::HostHandler;
use gandr_core_checker::host::HostOp;
use gandr_core_checker::host::HostReply;
use gandr_core_checker::outcome::Eval;
use gandr_core_checker::syntax::Comp;
use gandr_core_checker::syntax::Value;
use gandr_core_sequent::machine::run_comp_with_host;
use gandr_core_sequent::machine::run_comp_with_prelude_and_host;

use crate::error::ShellError;
use crate::handler::HostAction;
use crate::handler::ShellHandler;

/// The result of a shell run.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ShellOutcome
{
    /// The program reached a core terminal outcome — a returned value
    /// (`Eval::Value(ret v)`), a defined blame (a gradual hole or a control
    /// blame, including an unclaimed `perform` no shell signature handles), or
    /// a stuck configuration — without the host aborting it. The verbatim
    /// core [`Eval`] is preserved (see [`Self::returned`]).
    Completed(Eval),
    /// The program performed `Proc::exit code`; the host truncated the run.
    Exited
    {
        /// The requested exit code.
        code: i64,
    },
    /// A shell syscall failed fatally (a spawn failure, a missing file behind a
    /// string-typed read, …); the host aborted the run.
    HostFailed(ShellError),
}

impl ShellOutcome
{
    /// The value the program returned, if it completed with a `ret v` terminal.
    ///
    /// # Contract
    /// - ensures: `Some(v)` iff this is [`Self::Completed`] wrapping an
    ///   `Eval::Value(Comp::Ret v)`; `None` for every other outcome (a blame, a
    ///   stuck, an exit, a host failure, or a non-`ret` terminal).
    #[inline]
    #[must_use]
    pub fn returned(&self) -> Option<&Value>
    {
        match *self {
            | Self::Completed(Eval::Value(Comp::Ret(ref value))) => Some(value.as_ref()),
            | _ => None,
        }
    }
}

/// Runs a lowered computation under an ambient value prelude and the shell
/// host on the L machine.
///
/// This is the capability the surface engine's `run::run_source` composes with
/// its lowering, linking, and prelude checking: the runtime owns the host seam
/// and the canonical signatures, never the source pipeline.
///
/// # Contract
/// - ensures: the program runs with `prelude` installed as the ambient value
///   bindings and every host-interceptable `perform` offered to a fresh
///   [`ShellHandler`], with `Proc::exit` and fatal syscalls truncating the run;
/// - panics: none.
#[inline]
#[must_use]
pub fn run_program_with_prelude(
    comp: &Comp,
    prelude: &[(String, Value)],
) -> ShellOutcome
{
    run_with_driver(|driver| run_comp_with_prelude_and_host(comp, prelude, driver))
}

/// Runs a lowered computation under the shell host on the L machine.
///
/// The program is focused and driven by
/// [`gandr_core_sequent::machine::run_comp_with_host`]; each host-interceptable
/// `perform` is offered to a fresh [`ShellHandler`] over the ADR-35 D4 seam.
/// There is no ambient prelude here — the hand-built [`Comp`] entry runs
/// exactly the operators and host effects it names.
/// [`run_program_with_prelude`] installs an ambient value prelude; the surface
/// engine's `run::run_source` is the prelude-bearing source entry built on it.
///
/// Takes the program by reference: the L driver
/// ([`gandr_core_sequent::machine::run_comp_with_host`]) focuses and drives a
/// borrowed [`Comp`], so the caller keeps ownership (and may run the same
/// program again).
#[inline]
#[must_use]
pub fn run_program(comp: &Comp) -> ShellOutcome
{
    run_with_driver(|driver| run_comp_with_host(comp, driver))
}

/// Runs a program on the L machine under a fresh [`ShellDriver`] via `run`,
/// then surfaces any recorded early outcome in place of the terminal [`Eval`].
///
/// Factoring the seam adaptation here keeps the driver's [`HostAction`] →
/// [`HostReply`] mapping and its `Proc::exit` / fatal-abort capture in one
/// place. (Through the L1 migration a second `run` closure drove the retiring
/// CEK host path, so the two were held to the same observable [`ShellOutcome`];
/// that differential leg retired with the CEK at B1 stage F.)
#[inline]
fn run_with_driver<R>(run: R) -> ShellOutcome
where
    R: FnOnce(&mut ShellDriver) -> Eval,
{
    let mut driver = ShellDriver {
        handler: ShellHandler::new(),
        early: None,
    };
    let eval = run(&mut driver);
    match driver.early {
        | Some(ShellEarly::Exit(code)) => ShellOutcome::Exited { code },
        | Some(ShellEarly::Fail(error)) => ShellOutcome::HostFailed(error),
        | None => ShellOutcome::Completed(eval),
    }
}

/// A shell outcome the seam's [`HostReply`] cannot express, captured by
/// [`ShellDriver`] to be surfaced by [`run_with_driver`] after the run
/// terminates.
#[derive(Clone, Debug)]
enum ShellEarly
{
    /// The program performed `Proc::exit code`.
    Exit(i64),
    /// A native syscall failed fatally.
    Fail(ShellError),
}

/// Adapts the pure [`ShellHandler`] dispatcher to the L machine's
/// [`HostHandler`] seam.
///
/// The seam offers each operation as `(signature, operation, payload)` and
/// accepts only [`HostReply::Resume`] or [`HostReply::Unhandled`]. This adapter
/// re-packages the offer as a [`HostOp`] for the dispatcher, maps a resume
/// straight through, and captures the shell's two out-of-band outcomes — a
/// run-truncating `Proc::exit` and a fatal abort — as a [`ShellEarly`], then
/// declines so the machine terminates. Once an early outcome is recorded the
/// machine takes its terminal `PerformNoHandler` step and offers nothing
/// further, so a single field suffices.
struct ShellDriver
{
    /// The native operation dispatcher (owned; recreated per run).
    handler: ShellHandler,
    /// The out-of-band outcome, if the run truncated or aborted.
    early: Option<ShellEarly>,
}

impl HostHandler for ShellDriver
{
    #[inline]
    fn handle<'source, O>(
        &mut self,
        sig: &EffectSig,
        op: O,
        payload: &Value,
    ) -> HostReply
    where
        O: Into<OperationName<'source>>,
    {
        // The L seam offers a name-only signature (`𝓕` erases the operation
        // list); `ShellHandler::dispatch` keys on the signature *name* and the
        // operation name only, so the erased ops list is immaterial.
        let host_op = HostOp::new(sig.clone(), op.into(), payload.clone());
        match self.handler.dispatch(&host_op) {
            | HostAction::Resume(reply) => HostReply::Resume(reply),
            | HostAction::Exit(code) => {
                self.early = Some(ShellEarly::Exit(code));
                HostReply::Unhandled
            },
            | HostAction::Fail(error) => {
                self.early = Some(ShellEarly::Fail(error));
                HostReply::Unhandled
            },
            // Not a shell operation: decline, and the machine blames the
            // unclaimed `perform` exactly as an un-hosted run would.
            | HostAction::Decline => HostReply::Unhandled,
        }
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_checker::boundary::OperationName;
    use gandr_core_checker::effect::EffectOp;
    use gandr_core_checker::effect::EffectSig;
    use gandr_core_checker::grade::Grade;
    use gandr_core_checker::outcome::Blame;
    use gandr_core_checker::outcome::Eval;
    use gandr_core_checker::outcome::StuckReason;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::types::ValueType;

    use super::ShellOutcome;
    use super::run_program;
    use crate::boundary::CommandArgument;
    use crate::boundary::CommandProgram;
    use crate::boundary::FileContents;
    use crate::boundary::FilePath;
    use crate::boundary::ProcessExitCode;
    use crate::boundary::SpawnModeName;
    use crate::error::ShellError;
    use crate::sig;

    /// Sentinel value that must not run after `proc.exit` truncates a program.
    const UNREACHED_AFTER_EXIT_CODE: i64 = 99;

    /// ADR-74 D4 mode selection — the captured contract. An explicit
    /// `mode = "captured"` buffers the child's stdout into the typed reply,
    /// exactly as a bare (mode-less) payload does.
    #[test]
    fn exec_captured_mode_captures_stdout()
    {
        let outcome = run_op(
            sig::exec(),
            sig::EXEC_RUN,
            command_mode("echo", &["hi".into()], sig::MODE_CAPTURED),
        );
        assert_eq!(Some("hi\n"), reply_stdout(&outcome).as_deref());
        assert_eq!(Some(ProcessExitCode::from(0)), reply_exit_code(&outcome));
    }

    /// ADR-74 D4 mode selection — the inherit spawn. `mode = "inherit"` lets
    /// the child drive the terminal directly, so nothing is captured: the
    /// reply's `stdout` is empty while `exit_code` is still meaningful.
    #[test]
    fn exec_inherit_mode_does_not_capture_stdout()
    {
        let outcome = run_op(
            sig::exec(),
            sig::EXEC_RUN,
            command_mode("echo", &["hi".into()], sig::MODE_INHERIT),
        );
        assert_eq!(
            Some(""),
            reply_stdout(&outcome).as_deref(),
            "inherit spawns capture nothing"
        );
        assert_eq!(Some(ProcessExitCode::from(0)), reply_exit_code(&outcome));
    }

    /// A mode-less `{program, args}` payload decodes as captured (the
    /// drift-safe default): the corpus and every hand-built perform keep
    /// capturing.
    #[test]
    fn exec_missing_mode_defaults_to_captured()
    {
        let outcome = run_op(sig::exec(), sig::EXEC_RUN, command("echo", &["hi".into()]));
        assert_eq!(Some("hi\n"), reply_stdout(&outcome).as_deref());
    }

    /// An unrecognized `mode` string is a host failure — the mode vocabulary is
    /// closed (ADR-74 D4).
    #[test]
    fn exec_unknown_mode_is_a_host_failure()
    {
        let outcome = run_op(
            sig::exec(),
            sig::EXEC_RUN,
            command_mode("echo", &["hi".into()], "bogus"),
        );
        assert!(
            matches!(
                outcome,
                ShellOutcome::HostFailed(ShellError::Payload { .. })
            ),
            "an unknown spawn mode is a Payload host failure: {outcome:?}"
        );
    }

    #[test]
    fn exec_true_exits_zero()
    {
        let outcome = run_op(sig::exec(), sig::EXEC_RUN, command("true", &[]));
        let reply = outcome.returned().expect("exec replies with a record");
        let fields = reply.as_record().expect("the reply is a record");
        assert_eq!(
            Some(0),
            fields
                .get(sig::FIELD_EXIT_CODE)
                .and_then(|value| value.as_int())
                .map(i64::from)
        );
    }

    #[test]
    fn exec_echo_captures_stdout()
    {
        let outcome = run_op(
            sig::exec(),
            sig::EXEC_RUN,
            command("echo", &["hello".into()]),
        );
        let reply = outcome.returned().expect("exec replies with a record");
        let fields = reply.as_record().expect("the reply is a record");
        assert_eq!(
            Some("hello\n"),
            fields
                .get(sig::FIELD_STDOUT)
                .and_then(|value| value.as_str())
                .map(<&str>::from)
        );
        assert_eq!(
            Some(0),
            fields
                .get(sig::FIELD_EXIT_CODE)
                .and_then(|value| value.as_int())
                .map(i64::from)
        );
    }

    #[test]
    fn exec_nonzero_exit_is_a_normal_reply_not_an_error()
    {
        let outcome = run_op(sig::exec(), sig::EXEC_RUN, command("false", &[]));
        let code = outcome
            .returned()
            .and_then(Value::as_record)
            .and_then(|fields| fields.get(sig::FIELD_EXIT_CODE))
            .and_then(|value| value.as_int())
            .map(i64::from)
            .expect("a process that runs and exits has an exit code reply");
        assert_ne!(code, 0, "a non-zero exit is a reply, not a HostFailed");
    }

    #[test]
    fn exec_unspawnable_program_aborts_the_run()
    {
        let outcome = run_op(
            sig::exec(),
            sig::EXEC_RUN,
            command("gandr-runtime-host-no-such-binary-zzq", &[]),
        );
        assert!(
            matches!(outcome, ShellOutcome::HostFailed(ShellError::Spawn { .. })),
            "a spawn failure aborts the run: {outcome:?}"
        );
    }

    #[test]
    fn fs_write_read_stat_roundtrips()
    {
        let dir = scratch_dir();
        let file = format!("{dir}/note.txt");

        let write = run_op(sig::fs(), sig::FS_WRITE, write_payload(&file, "hi there"));
        assert_eq!(Some(&Value::Unit), write.returned(), "write replies unit");

        let read = run_op(sig::fs(), sig::FS_READ, Value::string(&file));
        assert_eq!(
            Some("hi there"),
            read.returned().and_then(Value::as_str).map(<&str>::from),
            "read returns exactly what write wrote"
        );

        let stat = run_op(sig::fs(), sig::FS_STAT, Value::string(&file));
        let fields = stat
            .returned()
            .and_then(Value::as_record)
            .expect("stat record");
        assert_eq!(
            Some("file"),
            fields
                .get(sig::FIELD_KIND)
                .and_then(|value| value.as_str())
                .map(<&str>::from)
        );
        assert_eq!(
            Some(8),
            fields
                .get(sig::FIELD_SIZE)
                .and_then(|value| value.as_int())
                .map(i64::from),
            "`hi there` is 8 bytes"
        );
    }

    #[test]
    fn fs_mkdir_creates_nested_and_stat_sees_a_dir()
    {
        let dir = scratch_dir();
        let nested = format!("{dir}/x/y/z");
        let mkdir = run_op(sig::fs(), sig::FS_MKDIR, Value::string(&nested));
        assert_eq!(Some(&Value::Unit), mkdir.returned());

        let stat = run_op(sig::fs(), sig::FS_STAT, Value::string(&nested));
        assert_eq!(
            Some("dir"),
            stat.returned()
                .and_then(Value::as_record)
                .and_then(|fields| fields.get(sig::FIELD_KIND))
                .and_then(|value| value.as_str())
                .map(<&str>::from)
        );
    }

    #[test]
    fn fs_stat_of_a_missing_path_is_missing_not_an_error()
    {
        let outcome = run_op(
            sig::fs(),
            sig::FS_STAT,
            Value::string("/gandr-runtime-host/definitely/not/here"),
        );
        assert_eq!(
            Some("missing"),
            outcome
                .returned()
                .and_then(Value::as_record)
                .and_then(|fields| fields.get(sig::FIELD_KIND))
                .and_then(|value| value.as_str())
                .map(<&str>::from)
        );
    }

    #[test]
    fn fs_ls_files_lists_the_tracked_repo_root()
    {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
        let outcome = run_op(sig::fs(), sig::FS_LS_FILES, Value::string(root));
        let files = outcome
            .returned()
            .and_then(Value::as_list)
            .expect("a list of files");
        assert!(
            files
                .iter()
                .any(|file| file.as_str().map(<&str>::from) == Some("Cargo.toml")),
            "git ls-files at the repo root lists the tracked root Cargo.toml"
        );
    }

    #[test]
    fn proc_exit_truncates_the_run()
    {
        // perform Proc::exit 3 >>= u. ret 99 — the `ret 99` must never run.
        let program = Comp::bind(
            Comp::perform(sig::proc(), sig::PROC_EXIT, Value::int(3)),
            "u",
            Comp::ret(Value::int(UNREACHED_AFTER_EXIT_CODE)),
        );
        assert_eq!(
            ShellOutcome::Exited { code: 3 },
            run_program(&program),
            "exit halts the run before the continuation"
        );
    }

    #[test]
    fn a_non_shell_perform_is_declined_and_blames()
    {
        let other = EffectSig::new("Other".into(), vec![EffectOp::new(
            "beep".into(),
            ValueType::Unit,
            ValueType::Unit,
        )]);
        let outcome = run_op(other, "beep", Value::Unit);
        assert!(
            matches!(
                outcome,
                ShellOutcome::Completed(Eval::Blame(Blame::PerformNoHandler))
            ),
            "a declined perform blames PerformNoHandler (not resumed, not host-failed): {outcome:?}"
        );
    }

    #[test]
    fn env_path_and_get_read_the_process_environment()
    {
        let path = run_op(sig::env(), sig::ENV_PATH, Value::Unit);
        let entries = path
            .returned()
            .and_then(Value::as_list)
            .expect("PATH is a list");
        assert!(
            !entries.is_empty(),
            "PATH has entries in a test environment"
        );

        let value = run_op(sig::env(), sig::ENV_GET, Value::string("PATH"));
        assert!(
            value
                .returned()
                .and_then(Value::as_str)
                .map(<&str>::from)
                .is_some_and(|text| !text.is_empty()),
            "Env::get PATH is a non-empty string"
        );
    }

    #[test]
    fn a_non_terminating_program_halts_with_step_limit()
    {
        // The CBPV Ω-combinator: a finite, closed, ill-typed term that
        // β-reduces to itself forever. The host driver halts it at STEP_BUDGET
        // with `Stuck(StepLimit)` rather than hanging.
        // (self = λx. (force x) x ; ω = thunk self ; Ω = (force ω) ω)
        let self_app = Comp::lam(
            "x",
            Comp::app(Comp::force(Value::var("x")), Value::var("x")),
        );
        let omega_thunk = Value::thunk(Grade::OMEGA, self_app);
        let omega = Comp::app(Comp::force(omega_thunk.clone()), omega_thunk);
        assert_eq!(
            ShellOutcome::Completed(Eval::Stuck(StuckReason::StepLimit)),
            run_program(&omega),
            "a non-terminating program halts at the step budget, it does not hang"
        );
    }

    #[test]
    fn a_malformed_exec_payload_is_a_host_failure()
    {
        // Not a record at all.
        let non_record = run_op(sig::exec(), sig::EXEC_RUN, Value::int(5));
        assert!(
            matches!(
                non_record,
                ShellOutcome::HostFailed(ShellError::Payload { .. })
            ),
            "a non-record exec payload is a Payload host failure: {non_record:?}"
        );

        // A record whose `args` element is not a string.
        let bad_args = Value::record([
            (sig::FIELD_PROGRAM.to_owned(), Value::string("echo")),
            (sig::FIELD_ARGS.to_owned(), Value::list(vec![Value::int(1)])),
        ]);
        let bad_element = run_op(sig::exec(), sig::EXEC_RUN, bad_args);
        assert!(
            matches!(
                bad_element,
                ShellOutcome::HostFailed(ShellError::Payload { .. })
            ),
            "a non-string args element is a Payload host failure: {bad_element:?}"
        );
    }

    #[test]
    fn fs_read_of_a_missing_file_is_a_host_failure()
    {
        let outcome = run_op(
            sig::fs(),
            sig::FS_READ,
            Value::string("/gandr-runtime-host/definitely/not/here.txt"),
        );
        assert!(
            matches!(outcome, ShellOutcome::HostFailed(ShellError::Fs { .. })),
            "a missing file behind the string-typed read aborts the run: {outcome:?}"
        );
    }

    #[test]
    fn an_unclaimed_perform_blames_perform_no_handler()
    {
        // A foreign signature whose op name (`read`) collides with `Fs::read`:
        // dispatch keys on the signature name first, so this is DECLINED, not
        // misrouted, and the machine blames `PerformNoHandler`.
        let foreign = EffectSig::new("Other".into(), vec![EffectOp::new(
            sig::FS_READ.into(),
            ValueType::string(),
            ValueType::string(),
        )]);
        let outcome = run_op(foreign, sig::FS_READ, Value::string("x"));
        assert!(
            matches!(
                outcome,
                ShellOutcome::Completed(Eval::Blame(Blame::PerformNoHandler))
            ),
            "an op named `read` in a foreign signature blames PerformNoHandler rather than routing \
             to Fs::read: {outcome:?}"
        );
    }

    #[test]
    fn fs_cwd_returns_a_directory()
    {
        let outcome = run_op(sig::fs(), sig::FS_CWD, Value::Unit);
        let cwd = outcome
            .returned()
            .and_then(Value::as_str)
            .map(<&str>::from)
            .expect("cwd replies with a path string")
            .to_owned();
        assert!(!cwd.is_empty(), "cwd is a non-empty path");
        let stat = run_op(sig::fs(), sig::FS_STAT, Value::string(&cwd));
        assert_eq!(
            Some("dir"),
            stat.returned()
                .and_then(Value::as_record)
                .and_then(|fields| fields.get(sig::FIELD_KIND))
                .and_then(|value| value.as_str())
                .map(<&str>::from),
            "the reported cwd stats as a directory"
        );
    }

    #[test]
    fn env_get_of_an_unset_variable_is_empty()
    {
        let outcome = run_op(
            sig::env(),
            sig::ENV_GET,
            Value::string("GANDR_RUNTIME_HOST_DEFINITELY_UNSET_VARIABLE_ZZQ"),
        );
        assert_eq!(
            Some(""),
            outcome.returned().and_then(Value::as_str).map(<&str>::from),
            "an unset variable reads as the empty string"
        );
    }

    #[test]
    fn two_tempdirs_in_one_run_are_distinct()
    {
        // perform tempdir () >>= a. perform tempdir () >>= b. ret {a, b}
        let program = Comp::bind(
            Comp::perform(sig::fs(), sig::FS_TEMPDIR, Value::Unit),
            "a",
            Comp::bind(
                Comp::perform(sig::fs(), sig::FS_TEMPDIR, Value::Unit),
                "b",
                Comp::ret(Value::record([
                    ("a".to_owned(), Value::var("a")),
                    ("b".to_owned(), Value::var("b")),
                ])),
            ),
        );
        let outcome = run_program(&program);
        let fields = outcome
            .returned()
            .and_then(Value::as_record)
            .expect("a record of two tempdir paths");
        let first = fields
            .get("a")
            .and_then(|value| value.as_str())
            .map(<&str>::from);
        let second = fields
            .get("b")
            .and_then(|value| value.as_str())
            .map(<&str>::from);
        assert!(first.is_some() && second.is_some());
        assert_ne!(first, second, "two tempdirs in one run have distinct paths");
    }

    #[test]
    fn fs_glob_runs_end_to_end_and_dedups_consecutive_globstars()
    {
        let base = scratch_dir();
        let subdir = format!("{base}/sub");
        let mkdir = run_op(sig::fs(), sig::FS_MKDIR, Value::string(&subdir));
        assert_eq!(
            Some(&Value::Unit),
            mkdir.returned(),
            "glob fixture subdirectory is created through Fs::mkdir"
        );
        for (path, contents) in [
            (format!("{base}/top.txt"), ""),
            (format!("{base}/sub/deep.txt"), ""),
        ] {
            let write = run_op(sig::fs(), sig::FS_WRITE, write_payload(&path, contents));
            assert_eq!(
                Some(&Value::Unit),
                write.returned(),
                "glob fixture file is written through Fs::write"
            );
        }

        let shown = base.as_str();
        // Two consecutive `**` reach `sub/deep.txt` by more than one split; the
        // reply must still list it once (sorted).
        let pattern = format!("{shown}/**/**/*.txt");
        let outcome = run_op(sig::fs(), sig::FS_GLOB, Value::string(&pattern));
        let matches = outcome
            .returned()
            .and_then(Value::as_list)
            .expect("glob replies with a list");
        let paths: Vec<&str> = matches
            .iter()
            .filter_map(|value| value.as_str().map(<&str>::from))
            .collect();
        assert_eq!(
            paths,
            [format!("{shown}/sub/deep.txt"), format!("{shown}/top.txt"),],
            "consecutive `**` matches are de-duplicated and sorted"
        );
    }

    #[test]
    fn unknown_ops_within_known_signatures_decline_and_blame()
    {
        // Each shell signature is claimed, but an operation name it does not
        // define is DECLINED — the machine then blames the unclaimed perform.
        for signature in [sig::exec(), sig::fs(), sig::env(), sig::proc()] {
            let outcome = run_op(signature, "no_such_op", Value::Unit);
            assert!(
                matches!(
                    outcome,
                    ShellOutcome::Completed(Eval::Blame(Blame::PerformNoHandler))
                ),
                "an unknown op in a claimed signature declines and blames: {outcome:?}"
            );
        }
    }

    #[test]
    fn proc_exit_with_a_non_integer_payload_is_a_host_failure()
    {
        let outcome = run_op(sig::proc(), sig::PROC_EXIT, Value::string("three"));
        assert!(
            matches!(
                outcome,
                ShellOutcome::HostFailed(ShellError::Payload { .. })
            ),
            "a non-integer proc.exit payload is a Payload host failure: {outcome:?}"
        );
    }

    #[test]
    fn fs_ls_files_outside_a_git_repository_is_a_host_failure()
    {
        // A fresh tempdir under the OS temp root is not inside a git work tree,
        // so `git ls-files` exits non-zero and the host aborts the run.
        let dir = scratch_dir();
        let outcome = run_op(sig::fs(), sig::FS_LS_FILES, Value::string(&dir));
        assert!(
            matches!(outcome, ShellOutcome::HostFailed(ShellError::Fs { .. })),
            "git ls-files in a non-repository directory fails fatally: {outcome:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn fs_stat_classifies_a_symlink_without_following_it()
    {
        let dir = scratch_dir();
        let target = format!("{dir}/target.txt");
        let link = format!("{dir}/link");
        let write = run_op(sig::fs(), sig::FS_WRITE, write_payload(&target, "x"));
        assert_eq!(
            Some(&Value::Unit),
            write.returned(),
            "symlink target is written through Fs::write"
        );
        let symlink = run_op(
            sig::exec(),
            sig::EXEC_RUN,
            command("ln", &["-s".into(), (&target).into(), (&link).into()]),
        );
        assert_eq!(
            Some(0),
            symlink
                .returned()
                .and_then(Value::as_record)
                .and_then(|fields| fields.get(sig::FIELD_EXIT_CODE))
                .and_then(|value| value.as_int())
                .map(i64::from),
            "`ln -s` creates the symlink fixture"
        );

        let outcome = run_op(sig::fs(), sig::FS_STAT, Value::string(&link));
        assert_eq!(
            Some("symlink"),
            outcome
                .returned()
                .and_then(Value::as_record)
                .and_then(|fields| fields.get(sig::FIELD_KIND))
                .and_then(|value| value.as_str())
                .map(<&str>::from),
            "symlink_metadata classifies the link itself, not its regular-file target"
        );
    }

    #[test]
    fn returned_is_none_for_non_ret_outcomes()
    {
        assert_eq!(
            None,
            ShellOutcome::Exited { code: 0 }.returned(),
            "an exited run returned no value"
        );
        assert_eq!(
            None,
            ShellOutcome::HostFailed(ShellError::Internal {
                detail: "x".to_owned()
            })
            .returned(),
            "a host-failed run returned no value"
        );
    }

    /// Creates a scratch directory via the `Fs::tempdir` operation itself.
    fn scratch_dir() -> String
    {
        let outcome = run_op(sig::fs(), sig::FS_TEMPDIR, Value::Unit);
        outcome
            .returned()
            .and_then(Value::as_str)
            .map(<&str>::from)
            .expect("tempdir replies with a path string")
            .to_owned()
    }

    /// Runs `perform sig::op payload >>= reply. ret reply`, so the outcome's
    /// [`ShellOutcome::returned`] is the operation's reply value.
    fn run_op<'op>(
        signature: EffectSig,
        op: impl Into<OperationName<'op>>,
        payload: Value,
    ) -> ShellOutcome
    {
        let op = op.into();
        run_program(&Comp::bind(
            Comp::perform(signature, op, payload),
            "reply",
            Comp::ret(Value::var("reply")),
        ))
    }

    /// An `Exec::exec` command payload `{program, args}` — no `mode` field, so
    /// the decoder's captured default (ADR-74 D4) applies.
    fn command<'program>(
        program: impl Into<CommandProgram<'program>>,
        args: &[CommandArgument<'_>],
    ) -> Value
    {
        let program = program.into();
        Value::record([
            (
                sig::FIELD_PROGRAM.to_owned(),
                Value::string(program.as_ref()),
            ),
            (
                sig::FIELD_ARGS.to_owned(),
                Value::list(args.iter().map(|arg| Value::string(arg.as_ref())).collect()),
            ),
        ])
    }

    /// An `Exec::exec` command payload `{program, args, mode}` with an explicit
    /// spawn mode (ADR-74 D4).
    fn command_mode<'program, 'mode>(
        program: impl Into<CommandProgram<'program>>,
        args: &[CommandArgument<'_>],
        mode: impl Into<SpawnModeName<'mode>>,
    ) -> Value
    {
        let program = program.into();
        let mode = mode.into();
        Value::record([
            (
                sig::FIELD_PROGRAM.to_owned(),
                Value::string(program.as_ref()),
            ),
            (
                sig::FIELD_ARGS.to_owned(),
                Value::list(args.iter().map(|arg| Value::string(arg.as_ref())).collect()),
            ),
            (sig::FIELD_MODE.to_owned(), Value::string(mode.as_ref())),
        ])
    }

    /// Read an `Exec::exec` reply's `stdout` string.
    fn reply_stdout(outcome: &ShellOutcome) -> Option<String>
    {
        let returned = outcome.returned()?;
        let fields = returned.as_record()?;
        let stdout = fields.get(sig::FIELD_STDOUT)?;
        stdout.as_str().map(|text| text.as_ref().to_owned())
    }

    /// Read an `Exec::exec` reply's `exit_code`.
    fn reply_exit_code(outcome: &ShellOutcome) -> Option<ProcessExitCode>
    {
        let returned = outcome.returned()?;
        let fields = returned.as_record()?;
        let exit_code = fields.get(sig::FIELD_EXIT_CODE)?;
        exit_code.as_int().map(i64::from).map(ProcessExitCode::from)
    }

    /// An `Fs::write` payload `{path, contents}`.
    fn write_payload<'path, 'contents>(
        path: impl Into<FilePath<'path>>,
        contents: impl Into<FileContents<'contents>>,
    ) -> Value
    {
        let path = path.into();
        let contents = contents.into();
        Value::record([
            (sig::FIELD_PATH.to_owned(), Value::string(path.as_ref())),
            (
                sig::FIELD_CONTENTS.to_owned(),
                Value::string(contents.as_ref()),
            ),
        ])
    }
}

/// L-driver shell-outcome insurance: [`run_program`] (the L path,
/// `run_comp_with_host`) preserves the observable [`ShellOutcome`] of a
/// resuming host op and of an unhandled decline.
///
/// Through stage E these were an L-vs-CEK **differential** — each case also ran
/// the retiring CEK host path (`gandr_core_checker::eval::run_with_host`) and
/// asserted the two agreed, cheap insurance that the retarget from the CEK
/// binding to the L driver changed no observable outcome. The CEK leg **retired
/// with the CEK at stage F**; each case keeps its own expected L outcome.
#[cfg(test)]
mod l_host_outcomes
{
    use gandr_core_checker::effect::EffectOp;
    use gandr_core_checker::effect::EffectSig;
    use gandr_core_checker::outcome::Blame;
    use gandr_core_checker::outcome::Eval;
    use gandr_core_checker::syntax::Comp;
    use gandr_core_checker::syntax::Value;
    use gandr_core_checker::types::ValueType;

    use super::ShellOutcome;
    use super::run_program;
    use crate::sig;

    #[test]
    fn resuming_host_op_takes_the_reply_as_the_outcome()
    {
        // A deterministic resuming op: `Env::get` of an unset variable resumes
        // with the empty string (no filesystem or process nondeterminism,
        // unlike `tempdir` or `exec`).
        let program = Comp::bind(
            Comp::perform(
                sig::env(),
                sig::ENV_GET,
                Value::string("GANDR_RUNTIME_HOST_DIFFERENTIAL_UNSET_ZZQ"),
            ),
            "reply",
            Comp::ret(Value::var("reply")),
        );
        let l = run_program(&program);
        assert_eq!(
            Some(""),
            l.returned().and_then(Value::as_str).map(<&str>::from),
            "the resuming op actually resumed (not declined) — the empty string reply"
        );
    }

    #[test]
    fn unhandled_perform_blames_perform_no_handler()
    {
        // A foreign signature the shell host does not claim: declined, blaming
        // `PerformNoHandler`.
        let other = EffectSig::new("Other".into(), vec![EffectOp::new(
            "beep".into(),
            ValueType::Unit,
            ValueType::Unit,
        )]);
        let program = Comp::bind(
            Comp::perform(other, "beep", Value::Unit),
            "u",
            Comp::ret(Value::Unit),
        );
        let l = run_program(&program);
        assert!(
            matches!(
                l,
                ShellOutcome::Completed(Eval::Blame(Blame::PerformNoHandler))
            ),
            "the declined perform blames PerformNoHandler: {l:?}"
        );
    }
}
