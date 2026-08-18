//! The gandr toolchain driver — the script-runner face.
//!
//! `gandr <file>` runs one gandr source file: the driver hands the path to
//! [`gandr_runtime_ffi::run_source_file`], which lowers, links, prelude-checks,
//! and runs the program under the combined native/shell host. The caller
//! receives a rendered returned value on standard output, while the run's
//! [`gandr_runtime_ffi::FfiShellOutcome`] determines the process exit status.
//!
//! The REPL, `tui`, `lsp`, `mcp`, `fmt`, and `build` faces are **deferred**:
//! the REPL waits on a line-editor decision wired to the landed grammar,
//! parser, and syntax crates, and the rest have no implementing crate in the
//! tree. `Cargo.toml` records which. They arrive with their dependency edges,
//! not by uncommenting.

use std::ffi::OsString;
use std::io::Write as _;
use std::process::ExitCode;

use gandr_core_checker::outcome::Eval;
use gandr_core_checker::term::syntax::Comp;
use gandr_runtime_ffi::FfiRunError;
use gandr_runtime_ffi::FfiShellOutcome;
use gandr_runtime_ffi::run_source_file;
/// Route a completed returned value to the process caller.
///
/// # Contract
/// - ensures: a returned value is written exactly once to standard output in
///   the stable structural representation supplied by its `Debug`
///   implementation; non-returning outcomes produce no result output.
/// - provides: caller-visible script results without changing status reporting.
/// - panics: none.
fn announce_result(outcome: &FfiShellOutcome)
{
    if let &FfiShellOutcome::Completed(Eval::Value(Comp::Ret(ref value))) = outcome {
        let rendered = format!("{value:?}\n");
        announce(rendered.as_str());
    }
}

/// The exit status of a run whose program completed normally.
const EXIT_COMPLETED: ExitStatus = ExitStatus(0_i64);

/// The exit status of a run the machine or the host did not complete.
const EXIT_FAILED: ExitStatus = ExitStatus(1_i64);

/// The exit status of a usage error or a source that never reached the machine.
const EXIT_REFUSED: ExitStatus = ExitStatus(2_i64);

/// The modulus a `proc.exit` code is reduced by before it leaves the process.
///
/// A process exit status is one byte on every host this driver targets, so a
/// script that exits with a wider integer is reduced the way a shell reduces
/// one rather than being rejected after its effects have already happened.
const EXIT_STATUS_MODULUS: i64 = 256_i64;

/// What the driver accepts on the command line.
const USAGE: &str = "usage: gandr <file>\n\nRuns one gandr source file. The REPL and the tui/lsp/mcp/fmt/build faces are deferred.\n";

/// A process exit status the driver is prepared to leave with.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ExitStatus(i64);

/// One diagnostic line on its way to standard error.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DiagnosticText<'text>(&'text str);

impl<'text> From<&'text str> for DiagnosticText<'text>
{
    #[inline]
    fn from(value: &'text str) -> Self
    {
        Self(value)
    }
}

impl<'text> From<&'text String> for DiagnosticText<'text>
{
    #[inline]
    fn from(value: &'text String) -> Self
    {
        Self(value.as_str())
    }
}

impl From<ExitStatus> for ExitCode
{
    /// Reduce an exit status to the byte a process may actually leave with.
    ///
    /// # Contract
    /// - ensures: a status already in `0..256` passes through unchanged, and a
    ///   wider or negative one is reduced modulo 256 the way a shell reduces it
    ///   (`exit -1` leaves 255).
    /// - provides: the total status-to-`ExitCode` conversion the driver's exit
    ///   paths share.
    /// - panics: none. Neither fallback below is reachable:
    ///   `checked_rem_euclid` returns `None` only for a zero divisor or
    ///   `i64::MIN % -1`, and the divisor here is the constant 256;
    ///   `rem_euclid` with a positive divisor yields `0..256`, which
    ///   `u8::try_from` always accepts. They are written as totals rather than
    ///   as live failure modes.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — identity, negative wrap, and above-range wrap
    ///   are separated by three exit codes a script can actually request: 7
    ///   passes through, -1 becomes 255, and 300 becomes 44.
    /// - witness: `cli::tests::a_script_that_exits_leaves_with_its_own_status`
    /// - witness: `cli::tests::a_negative_exit_code_wraps_the_way_a_shell_wraps_it`
    /// - witness: `cli::tests::an_out_of_range_exit_code_is_reduced_to_a_byte`
    #[inline]
    fn from(value: ExitStatus) -> Self
    {
        let reduced = value
            .0
            .checked_rem_euclid(EXIT_STATUS_MODULUS)
            .unwrap_or(1_i64);
        let byte = u8::try_from(reduced).unwrap_or(1_u8);
        Self::from(byte)
    }
}

/// What the driver was asked to do.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Request
{
    /// Run the source file at this path.
    Run(OsString),
    /// Print the usage text and leave successfully.
    Usage,
}

/// Program entry point.
///
/// # Contract
/// - ensures: exactly one request is served, and the process leaves with the
///   status [`serve`] chose for it.
/// - provides: the only process-exit boundary in the driver.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 — every observable decision belongs to [`parse_request`]
///   and [`serve`], which the CLI suite drives through the real binary.
fn main() -> ExitCode
{
    let status = serve(std::env::args_os());
    ExitCode::from(status)
}

/// Parse `arguments`, serve the request, and report the status to leave with.
///
/// An explicit help request is not an error, so [`USAGE`] goes to standard
/// output when it was asked for and to standard error when it is a complaint.
/// A completed script routes its returned value to standard output exactly once
/// before [`classify`] reports the process status.
///
/// # Contract
/// - requires: `arguments` begins with the executable name.
/// - ensures: a usage request prints [`USAGE`] to standard output and reports
///   [`EXIT_COMPLETED`]; a run request reports the status [`classify`] derives
///   from the run.
/// - provides: the driver's complete argument-to-status behaviour, separated
///   from the process boundary so a test can drive it.
/// - fails: a malformed command line prints [`USAGE`] to standard error and
///   reports [`EXIT_REFUSED`]; a source that never reached the machine —
///   unreadable, or refused by the checker — prints its typed error and reports
///   [`EXIT_REFUSED`].
/// - panics: none — a diagnostic that cannot be written is dropped rather than
///   escalated, since the status still carries the outcome.
///
/// # Adequacy
/// - hypothesis: L3 only — the four statuses are separated by four inputs: no
///   argument, a script returning a value, a script performing `proc.exit`, and
///   an absent path; the checker refusal is pinned beside the absent path
///   because it shares the refusal status without sharing its cause.
/// - witness: `cli::tests::no_argument_prints_usage_and_refuses`
/// - witness: `cli::tests::a_script_that_returns_a_value_leaves_successfully`
/// - witness: `cli::tests::a_script_that_exits_leaves_with_its_own_status`
/// - witness: `cli::tests::an_absent_script_is_refused_by_path`
/// - witness: `cli::tests::an_ill_typed_script_is_refused_by_the_checker`
fn serve<Arguments>(arguments: Arguments) -> ExitStatus
where
    Arguments: IntoIterator<Item = OsString>,
{
    match parse_request(arguments) {
        | Some(Request::Usage) => {
            announce(USAGE);
            EXIT_COMPLETED
        },
        | Some(Request::Run(path)) => match run_source_file(std::path::Path::new(&path)) {
            | Ok(outcome) => {
                announce_result(&outcome);
                classify(&outcome)
            },
            | Err(error) => {
                report(&refusal(&error));
                EXIT_REFUSED
            },
        },
        | None => {
            report(USAGE);
            EXIT_REFUSED
        },
    }
}

/// Recognize the driver's one accepted command line.
///
/// # Contract
/// - requires: `arguments` begins with the executable name, which is skipped.
/// - ensures: `Some(Request::Usage)` for exactly `--help` or `-h`, and
///   `Some(Request::Run(path))` for exactly one operand that is not one of
///   those and does not begin with `-`.
/// - provides: the closed accepted-command-line surface of the script-runner
///   face.
/// - fails: returns `None` for no operand, for more than one argument, and for
///   a UTF-8 argument that begins with `-` and is neither help spelling.
/// - panics: none.
/// - intension: the arity check runs BEFORE the help check, so `--help` with a
///   trailing argument is a malformed command line rather than a help request.
///   A bare `-` and a non-UTF-8 argument beginning with `-` are both taken as
///   paths: there is no standard-input face for `-` to mean, and a path is not
///   required to be UTF-8. A deferred face named WITHOUT a leading dash —
///   `gandr tui` — is therefore read as a path and fails as a missing file, not
///   as an unknown subcommand; that is honest only while no subcommand exists,
///   and the first subcommand to land owes this function a real command table.
///
/// # Adequacy
/// - hypothesis: L3 only — the accepted forms and the three refusal returns are
///   separated by the argument lists the CLI suite passes to the real binary,
///   with the bare-dash and deferred-subcommand paths pinned separately because
///   the intension above is the surprising part.
/// - witness: `cli::tests::no_argument_prints_usage_and_refuses`
/// - witness: `cli::tests::a_second_operand_is_refused`
/// - witness: `cli::tests::an_unknown_flag_is_refused`
/// - witness: `cli::tests::help_prints_usage_and_leaves_successfully`
/// - witness: `cli::tests::a_bare_dash_is_a_path_not_standard_input`
/// - witness: `cli::tests::a_deferred_subcommand_name_is_read_as_a_path`
fn parse_request<Arguments>(arguments: Arguments) -> Option<Request>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut operands = arguments.into_iter().skip(1);
    let first = operands.next()?;
    if operands.next().is_some() {
        return None;
    }
    if first == *"--help" || first == *"-h" {
        return Some(Request::Usage);
    }
    let looks_like_a_flag = first
        .to_str()
        .is_some_and(|text| text.starts_with('-') && text != "-");
    if looks_like_a_flag {
        return None;
    }
    Some(Request::Run(first))
}

/// Derive the process status from the run's outcome.
///
/// # Contract
/// - ensures: a value terminal reports [`EXIT_COMPLETED`]; `proc.exit code`
///   reports that code reduced to a byte; a blame, a stuck configuration, or a
///   fatal host abort prints one diagnostic and reports [`EXIT_FAILED`].
/// - provides: the script runner's outcome-to-status contract, which is what a
///   calling shell reads.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the completed, exited, and failed arms are separated
///   by three scripts: one returning a value, one performing `proc.exit`, and
///   one blaming on an unhandled `perform`; the failed arm's fatal host abort
///   is pinned separately because it reaches the same status through a
///   different diagnostic.
/// - witness: `cli::tests::a_script_that_returns_a_value_leaves_successfully`
/// - witness: `cli::tests::a_script_that_exits_leaves_with_its_own_status`
/// - witness: `cli::tests::a_script_that_blames_leaves_with_a_failure_status`
/// - witness: `cli::tests::a_script_whose_tool_cannot_spawn_leaves_with_a_failure_status`
fn classify(outcome: &FfiShellOutcome) -> ExitStatus
{
    match outcome {
        | &FfiShellOutcome::Completed(Eval::Value(_)) => EXIT_COMPLETED,
        | &FfiShellOutcome::Completed(Eval::Blame(ref blame)) => {
            report(&format!("gandr: the program blamed: {blame:?}\n"));
            EXIT_FAILED
        },
        | &FfiShellOutcome::Completed(Eval::Stuck(ref reason)) => {
            report(&format!("gandr: the program stuck: {reason:?}\n"));
            EXIT_FAILED
        },
        | &FfiShellOutcome::Exited { code } => ExitStatus(code),
        | &FfiShellOutcome::ShellFailed(ref error) => {
            report(&format!("gandr: {error}\n"));
            EXIT_FAILED
        },
        | &FfiShellOutcome::FfiFailed(ref error) => {
            report(&format!("gandr: {error}\n"));
            EXIT_FAILED
        },
    }
}

/// Render a source-preparation failure as one diagnostic line.
///
/// # Contract
/// - ensures: the rendering names the file for a read failure and the typed
///   source failure otherwise, and always ends with a newline.
/// - provides: the single place the refusal wording lives.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the read arm and the source arm are separated by an
///   absent path and a source with no runnable program; the checker refusal is
///   the source arm's second cause, pinned because it shares the rendering
///   shape without sharing the failure stage.
/// - witness: `cli::tests::an_absent_script_is_refused_by_path`
/// - witness: `cli::tests::a_script_with_no_program_is_refused`
/// - witness: `cli::tests::an_ill_typed_script_is_refused_by_the_checker`
fn refusal(error: &FfiRunError) -> String
{
    format!("gandr: {error}\n")
}

/// Write one line to standard output, dropping a write failure.
///
/// # Contract
/// - ensures: `text` is written to standard output when standard output accepts
///   it, and is discarded when it does not.
/// - provides: the driver's only standard-output sink, used for output the
///   caller asked for rather than for a complaint.
/// - fails: a closed or full standard output is dropped rather than escalated,
///   for the same reason [`report`] drops one.
/// - panics: none.
fn announce<'text, Text>(text: Text)
where
    Text: Into<DiagnosticText<'text>>,
{
    let mut stdout = std::io::stdout();
    drop(stdout.write_all(text.into().0.as_bytes()));
    drop(stdout.flush());
}

/// Write one diagnostic to standard error, dropping a write failure.
///
/// # Contract
/// - ensures: `text` is written to standard error when standard error accepts
///   it, and is discarded when it does not.
/// - provides: the driver's only diagnostic sink.
/// - fails: a closed or full standard error is dropped rather than escalated —
///   the exit status still carries the outcome, and a driver that died trying
///   to complain would report the wrong one.
/// - panics: none.
fn report<'text, Text>(text: Text)
where
    Text: Into<DiagnosticText<'text>>,
{
    let mut stderr = std::io::stderr();
    drop(stderr.write_all(text.into().0.as_bytes()));
    drop(stderr.flush());
}
