//! The gandr toolchain driver — the script-runner face.
//!
//! `gandr <file>` runs one gandr source file: the driver hands the path to
//! [`gandr_surface_engine::run::run_source_file`], which lowers, links,
//! prelude-checks, and runs the program under the host-effect handler, and then
//! turns the run's [`ShellOutcome`] into a process exit status. The driver owns
//! no pipeline of its own; it is the process boundary and nothing else.
//!
//! **A successful run prints nothing.** A script says what it has to say
//! through its own host effects — the shell blocks and `fs`/`env`/`proc` module
//! calls it performs — so the driver adding a rendering of the returned value
//! would put a second, rival value printer in the tree beside the corpus
//! harness's deliberately structural one. The returned value reaches the
//! process as an exit status only.
//!
//! The REPL, `tui`, `lsp`, `mcp`, `fmt`, and `build` faces are **deferred**:
//! each needs a crate this reboot has not landed, and `Cargo.toml` records
//! which. They arrive with their dependency edges, not by uncommenting.

use std::ffi::OsString;
use std::io::Write as _;
use std::process::ExitCode;

use gandr_core_checker::outcome::Eval;
use gandr_runtime_host::ShellOutcome;
use gandr_surface_engine::run::RunFileError;

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
    /// - panics: none — the reduction is checked and the reduced value is
    ///   always a byte.
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
/// Diagnostics go to standard error and nothing else goes to standard output,
/// so a script's own output reaches its consumer unmixed.
///
/// # Contract
/// - requires: `arguments` begins with the executable name.
/// - ensures: a usage request prints [`USAGE`] and reports [`EXIT_COMPLETED`];
///   a run request reports the status [`classify`] derives from the run.
/// - provides: the driver's complete argument-to-status behaviour, separated
///   from the process boundary so a test can drive it.
/// - fails: a malformed command line prints [`USAGE`] to standard error and
///   reports [`EXIT_REFUSED`]; a source that never reached the machine prints
///   its typed error and reports [`EXIT_REFUSED`].
/// - panics: none — a diagnostic that cannot be written is dropped rather than
///   escalated, since the status still carries the outcome.
///
/// # Adequacy
/// - hypothesis: L3 only — the four statuses are separated by four inputs: no
///   argument, a script returning a value, a script performing `proc.exit`, and
///   an absent path.
/// - witness: `cli::tests::no_argument_prints_usage_and_refuses`
/// - witness: `cli::tests::a_script_that_returns_a_value_leaves_successfully`
/// - witness: `cli::tests::a_script_that_exits_leaves_with_its_own_status`
/// - witness: `cli::tests::an_absent_script_is_refused_by_path`
fn serve<Arguments>(arguments: Arguments) -> ExitStatus
where
    Arguments: IntoIterator<Item = OsString>,
{
    match parse_request(arguments) {
        | Some(Request::Usage) => {
            report(USAGE);
            EXIT_COMPLETED
        },
        | Some(Request::Run(path)) => {
            match gandr_surface_engine::run::run_source_file(std::path::Path::new(&path)) {
                | Ok(outcome) => classify(&outcome),
                | Err(error) => {
                    report(&refusal(&error));
                    EXIT_REFUSED
                },
            }
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
///   `Some(Request::Run(path))` for exactly one non-flag operand.
/// - provides: the closed accepted-command-line surface of the script-runner
///   face; a deferred subcommand is not silently read as a path.
/// - fails: returns `None` for no operand, more than one operand, or an operand
///   that looks like a flag.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the accepted forms and the four refusals are
///   separated by the argument lists the CLI suite passes to the real binary.
/// - witness: `cli::tests::no_argument_prints_usage_and_refuses`
/// - witness: `cli::tests::a_second_operand_is_refused`
/// - witness: `cli::tests::an_unknown_flag_is_refused`
/// - witness: `cli::tests::help_prints_usage_and_leaves_successfully`
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
///   one blaming on an unhandled `perform`.
/// - witness: `cli::tests::a_script_that_returns_a_value_leaves_successfully`
/// - witness: `cli::tests::a_script_that_exits_leaves_with_its_own_status`
/// - witness: `cli::tests::a_script_that_blames_leaves_with_a_failure_status`
fn classify(outcome: &ShellOutcome) -> ExitStatus
{
    match *outcome {
        | ShellOutcome::Completed(Eval::Value(_)) => EXIT_COMPLETED,
        | ShellOutcome::Completed(Eval::Blame(blame)) => {
            report(&format!("gandr: the program blamed: {blame:?}\n"));
            EXIT_FAILED
        },
        | ShellOutcome::Completed(Eval::Stuck(ref reason)) => {
            report(&format!("gandr: the program stuck: {reason:?}\n"));
            EXIT_FAILED
        },
        | ShellOutcome::Exited { code } => ExitStatus(code),
        | ShellOutcome::HostFailed(ref error) => {
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
///   absent path and a source with no runnable program.
/// - witness: `cli::tests::an_absent_script_is_refused_by_path`
/// - witness: `cli::tests::a_script_with_no_program_is_refused`
fn refusal(error: &RunFileError) -> String
{
    format!("gandr: {error}\n")
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
