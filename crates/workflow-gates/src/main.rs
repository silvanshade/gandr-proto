//! Command-line entry point for Rust-backed workspace gates.
//!
//! The binary keeps argument parsing explicit and operating-system-string safe.
//! Domain modules own validation and side effects; this layer maps the retained
//! command inventory onto typed domain entry points and process exit semantics.

extern crate alloc;

use std::ffi::OsStr;
use std::ffi::OsString;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use gandr_workflow_gates::Finding;
use gandr_workflow_gates::GateError;
use gandr_workflow_gates::docs;
use gandr_workflow_gates::maintenance;
use gandr_workflow_gates::mutants;

gandr_workflow_gates::semantic_str!(pub struct CommandNameText);
gandr_workflow_gates::semantic_copy!(pub struct NonceNonce(u128));
gandr_workflow_gates::semantic_copy!(pub struct AttemptCount(u16));
gandr_workflow_gates::semantic_str!(pub struct ActionText);
gandr_workflow_gates::semantic_str!(pub struct OptionNameText);
gandr_workflow_gates::semantic_str!(pub struct LabelText);
gandr_workflow_gates::semantic_str!(pub struct DetailText);
gandr_workflow_gates::semantic_copy!(pub struct SanitizedGitFlag(bool));
gandr_workflow_gates::semantic_str!(pub struct ModeText);
gandr_workflow_gates::semantic_str!(pub struct SuffixText);
gandr_workflow_gates::semantic_str!(pub struct ExpectedText);
gandr_workflow_gates::semantic_str!(pub struct ValueText);
gandr_workflow_gates::semantic_str!(pub struct NameText);
gandr_workflow_gates::semantic_copy!(pub struct MutantsTemporaryPathsAreAvailableFlag(bool));
gandr_workflow_gates::semantic_copy!(pub struct ConsumeMutantsCommonArgumentFlag(bool));
gandr_workflow_gates::semantic_copy!(pub struct OptionTokenFlag(bool));
gandr_workflow_gates::semantic_str!(pub struct UsageTextText);
gandr_workflow_gates::semantic_str!(pub struct FuzzTargetNameText);
gandr_workflow_gates::semantic_str!(pub struct FuzzFeatureNameText);
gandr_workflow_gates::semantic_copy!(pub struct RunningUnderBinaryUnitTestFlag(bool));

/// Process exit code for a clean gate run.
const EXIT_CLEAN: u8 = 0;
/// Process exit code for a gate run with semantic findings.
const EXIT_FINDINGS: u8 = 1;
/// Process exit code for usage or operational failure.
const EXIT_OPERATIONAL: u8 = 2;

/// Cargo program used for deterministic AFL smoke builds.
const CARGO_PROGRAM: &str = "cargo";
/// AFL fuzz harness manifest path required by the smoke command.
const FUZZ_MANIFEST_PATH: &str = "fuzz/Cargo.toml";
/// Root that contains one corpus directory per allowed AFL target.
const FUZZ_CORPUS_ROOT: &str = "fuzz/corpus";
/// Directory where `cargo afl build` places target debug binaries.
const FUZZ_TARGET_DEBUG_ROOT: &str = "fuzz/target/debug";
/// Cargo feature that exposes the parity AFL target.
const FUZZ_PARITY_FEATURE: &str = "parity";
/// Cargo feature that exposes the Rust gate-suite AFL target.
const FUZZ_GATES_FEATURE: &str = "gates";
/// Default mutants cache image before home-directory expansion.
const MUTANTS_DEFAULT_CACHE_IMAGE: &str = "~/.microsandbox/gandr-mutants-cache.btrfs";
/// Default upper ref for configured push campaigns without push-event metadata.
const MUTANTS_DEFAULT_PUSH_TO_REF: &str = "HEAD";
/// Prefix for CLI-generated temporary mutation-campaign paths.
const MUTANTS_TEMPORARY_PATH_PREFIX: &str = "gandr-workflow-gates-mutants";
/// Bounded attempts for collision-resistant mutation temporary path names.
const MUTANTS_TEMPORARY_PATH_ATTEMPTS: u16 = 1024;
/// All allowed fuzz-smoke targets in deterministic execution order.
const FUZZ_TARGETS: [FuzzSmokeTarget; 5] = [
    FuzzSmokeTarget::Lower,
    FuzzSmokeTarget::Parse,
    FuzzSmokeTarget::Check,
    FuzzSmokeTarget::Parity,
    FuzzSmokeTarget::Gates,
];

/// Git executable used by Agda dependency provisioning.
const GIT_PROGRAM: &str = "git";
/// Agda standard-library repository required by `agda:deps`.
const AGDA_STDLIB_REPOSITORY: &str = "https://github.com/agda/agda-stdlib.git";
/// Agda standard-library branch validated for the pinned Agda toolchain.
const AGDA_STDLIB_BRANCH: &str = "v2.4";
/// Relative vendor directory that stores Agda dependencies.
const AGDA_VENDOR_DIR: &str = "metatheory/vendor";
/// Relative Agda standard-library checkout directory.
const AGDA_STDLIB_DIR: &str = "metatheory/vendor/agda-stdlib";
/// Relative `.agda-lib` file written to the Agda libraries file.
const AGDA_STDLIB_LIB: &str = "metatheory/vendor/agda-stdlib/standard-library.agda-lib";
/// Relative Agda libraries file consumed by `agda`.
const AGDA_LIBRARIES_FILE: &str = "metatheory/libraries";
/// Stable success message preserved from the Gandr Agda dependency script.
const AGDA_DEPS_READY_STDOUT: &str = "agda deps ready: stdlib v2.4\n";

/// Run the CLI and convert typed outcomes to process exit codes.
///
/// # Contract
/// - requires: process arguments and current directory are supplied by the
///   host.
/// - ensures: clean commands exit `0`, finding-bearing commands print each
///   finding and exit `1`, and usage/operational errors print one diagnostic
///   and exit `2`.
/// - provides: the only process-exit boundary for the binary.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — clean, finding, operational, and external-status
///   outcomes are observed through integration tests and the domain gate
///   suites.
/// - witness: `tooling::top_level_command_inventory_is_exact`
#[inline]
#[must_use]
pub fn main() -> std::process::ExitCode
{
    match run() {
        | Ok(outcome) => outcome.into_exit_code(),
        | Err(error) => {
            let mut stderr = std::io::stderr();
            drop(writeln!(stderr, "{error}"));
            std::process::ExitCode::from(EXIT_OPERATIONAL)
        },
    }
}

/// Parse process arguments, execute the selected command, and return an
/// outcome.
///
/// # Contract
/// - requires: process arguments begin with the executable path.
/// - ensures: delegates to [`run_with_args`] without changing argument bytes.
/// - provides: testable separation between host argument capture and dispatch.
/// - fails: returns [`GateError`] from parsing or the selected operation.
/// - panics: none.
///
/// # Errors
/// Returns usage errors from CLI parsing and operational errors from domains.
///
/// # Adequacy
/// - hypothesis: L1 — this seam is covered by [`run_with_args`], which owns all
///   observable command decisions.
#[inline]
pub fn run() -> Result<GateOutcome, GateError>
{
    run_with_args(std::env::args_os())
}

/// Parse the selected command from `arguments`, execute it, and classify
/// output.
///
/// # Contract
/// - requires: `arguments` begins with the executable name followed by one
///   supported command form.
/// - ensures: executes exactly one selected operation synchronously and returns
///   the corresponding process outcome.
/// - provides: one typed dispatcher for every retained Rust gate command.
/// - fails: returns usage errors, analyzer errors, process errors, or
///   fuzz-smoke failures without continuing to later operations.
/// - panics: none.
///
/// # Errors
/// Returns any parsing or operational error raised before a complete outcome.
///
/// # Adequacy
/// - hypothesis: L3 only — pure fixture dispatch, workflow-plan parsing,
///   external status mapping, and fuzz-smoke planning are split by integration
///   witnesses in `tooling.rs`.
#[inline]
pub fn run_with_args<Arguments>(arguments: Arguments) -> Result<GateOutcome, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    match parse_command(arguments)? {
        | Command::Contracts {
            scopes,
            nextest_list_fixture,
        } => {
            let findings =
                gandr_workflow_gates::contracts::run(&scopes, nextest_list_fixture.as_deref())?;
            Ok(GateOutcome::from_findings(findings))
        },
        | Command::CiContracts { workflow } => {
            let findings = gandr_workflow_gates::contracts::run_ci_workflow(&workflow)?;
            Ok(GateOutcome::from_findings(findings))
        },
        | Command::GraphBoundary {
            workspace_root,
            metadata_fixture,
        } => {
            let findings = gandr_workflow_gates::graph_boundary::run(
                &workspace_root,
                metadata_fixture.as_deref(),
            )?;
            Ok(GateOutcome::from_findings(findings))
        },
        | Command::DocsManifest { manifest_path } => {
            let findings = docs::manifest::run_manifest_drift(&manifest_path)?;
            Ok(GateOutcome::from_findings(findings))
        },
        | Command::DocsReference { manifest_path } => {
            let findings = docs::references::run_reference_integrity(&manifest_path)?;
            Ok(GateOutcome::from_findings(findings))
        },
        | Command::PageBalance { cwd } => {
            let report = docs::commands::run_page_balance(cwd.as_deref())?;
            Ok(GateOutcome::PageBalance(report))
        },
        | Command::Rumdl { mode, paths, cwd } => {
            let outcome = docs::commands::run_guarded_rumdl(mode, &paths, cwd.as_deref())?;
            Ok(rumdl_outcome(&outcome))
        },
        | Command::OptionsPolicy { workspace_root } => {
            let findings =
                gandr_workflow_gates::source_policy::run_options_policy(&workspace_root)?;
            Ok(GateOutcome::from_findings(findings))
        },
        | Command::SoundnessOracles { workspace_root } => {
            let findings = gandr_workflow_gates::source_policy::run_default_soundness_oracles(
                &workspace_root,
            )?;
            Ok(GateOutcome::from_findings(findings))
        },
        | Command::DefaultGraph { workspace_root } => {
            let findings =
                gandr_workflow_gates::project::check_default_dependency_graph(&workspace_root)?;
            Ok(GateOutcome::from_findings(findings))
        },
        | Command::IuPin { workspace_root } => {
            let findings = gandr_workflow_gates::project::check_default_iu_pin(&workspace_root)?;
            Ok(GateOutcome::from_findings(findings))
        },
        | Command::AgdaDeps { workspace_root } => {
            let workspace_root = agda_workspace_root(workspace_root.as_deref())?;
            let plan = AgdaDependencyPlan::new(workspace_root);
            run_agda_deps_plan(&plan)?;
            Ok(GateOutcome::Clean)
        },
        | Command::Coverage {
            mode,
            summary_path,
            floors_path,
            repo_root,
        } => match mode {
            | CoverageCommand::Check => {
                let findings =
                    gandr_workflow_gates::coverage::check(&summary_path, &floors_path, &repo_root)?;
                Ok(GateOutcome::from_findings(findings))
            },
            | CoverageCommand::Ratchet => {
                gandr_workflow_gates::coverage::ratchet(&summary_path, &floors_path, &repo_root)?;
                Ok(GateOutcome::Clean)
            },
        },
        | Command::MaintenanceRange {
            github_output,
            head,
            explicit_from,
            watermark,
        } => {
            let request = maintenance::MaintenanceRangeRequest::new(
                &github_output,
                head,
                explicit_from,
                watermark.as_deref(),
                None,
                maintenance::HeadExpectation::CurrentHead,
            );
            maintenance::resolve_and_append_github_output(&request)?;
            Ok(GateOutcome::Clean)
        },
        | Command::MaintenanceAdvance { watermark, to } => {
            let request = maintenance::MaintenanceAdvanceRequest::new(
                &watermark,
                to,
                None,
                maintenance::HeadExpectation::CurrentHead,
            );
            maintenance::advance_watermark(&request)?;
            Ok(GateOutcome::Clean)
        },
        | Command::Mutants { command, options } => {
            mutants::run(&command, &options)?;
            Ok(GateOutcome::Clean)
        },
        | Command::Workflow { tier, cwd } => {
            gandr_workflow_gates::workflow::execute(tier, cwd.as_deref())?;
            Ok(GateOutcome::Clean)
        },
        | Command::FuzzSmoke { plan } => {
            run_fuzz_smoke_plan(&plan)?;
            Ok(GateOutcome::Clean)
        },
    }
}

/// Convert an external process status into a portable process exit code.
fn exit_code_from_status(status: std::process::ExitStatus) -> std::process::ExitCode
{
    match status.code().and_then(|code| u8::try_from(code).ok()) {
        | Some(code) => std::process::ExitCode::from(code),
        | None => std::process::ExitCode::from(EXIT_OPERATIONAL),
    }
}

/// Print page-balance informational notes.
fn print_page_balance_report(report: &docs::commands::PageBalanceReport)
{
    let mut stdout_lock = std::io::stdout().lock();
    for probe in &report.late_probes {
        drop(writeln!(
            stdout_lock,
            "NOTE page-balance probe -- `{}` opens on page {} at {:.2}mm",
            probe.kind, probe.page, probe.y_mm
        ));
    }
}

/// Parse the exact supported command line from an OS argument iterator.
///
/// # Contract
/// - requires: `arguments` includes an executable-name element before CLI
///   tokens.
/// - ensures: returns the exact supported [`Command`] variant for every command
///   listed by [`top_level_command_names`].
/// - provides: the top-level CLI grammar discriminator.
/// - fails: returns usage errors for missing commands, unknown commands,
///   non-UTF-8 command names, malformed nested modes, and malformed options.
/// - panics: none.
///
/// # Errors
/// Returns usage errors for malformed command lines.
///
/// # Adequacy
/// - hypothesis: L3 only — inventory, missing-command, unknown-command,
///   nested-mode, and delegated option parsing are killed by `tooling.rs`
///   integration witnesses.
#[inline]
pub fn parse_command<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter();
    drop(arguments.next());
    let command_name = arguments
        .next()
        .ok_or_else(|| GateError::usage(usage_text().into().0))?;
    match command_name.to_str() {
        | Some("contracts") => parse_contracts(arguments),
        | Some("ci-contracts") => parse_ci_contracts(arguments),
        | Some("graph-boundary") => parse_graph_boundary(arguments),
        | Some("docs-manifest") => parse_docs_manifest(arguments),
        | Some("docs-reference") => parse_docs_reference(arguments),
        | Some("page-balance") => parse_page_balance(arguments),
        | Some("rumdl") => parse_rumdl(arguments),
        | Some("options-policy") => parse_options_policy(arguments),
        | Some("soundness-oracles") => parse_soundness_oracles(arguments),
        | Some("default-graph") => parse_default_graph(arguments),
        | Some("iu-pin") => parse_iu_pin(arguments),
        | Some("agda-deps") => parse_agda_deps(arguments),
        | Some("coverage") => parse_coverage(arguments),
        | Some("maintenance-range") => parse_maintenance_range(arguments),
        | Some("mutants") => parse_mutants(arguments),
        | Some("workflow") => parse_workflow(arguments),
        | Some("fuzz-smoke") => parse_fuzz_smoke(arguments),
        | Some(other) => Err(GateError::usage(format!("unknown command `{other}`"))),
        | None => Err(GateError::usage("command must be valid UTF-8")),
    }
}

/// Convert a guarded-rumdl domain result into process-level CLI semantics.
///
/// # Contract
/// - requires: `outcome` is the exact result returned by the documentation
///   command wrapper.
/// - ensures: rumdl process statuses are forwarded exactly.
/// - provides: external-command status preservation without changing finding
///   rendering for semantic gates.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — rumdl statuses are witnessed through the domain
///   wrapper tests and CLI status conversion tests.
fn rumdl_outcome(outcome: &docs::commands::RumdlOutcome) -> GateOutcome
{
    match *outcome {
        | docs::commands::RumdlOutcome::RumdlStatus { status } => {
            GateOutcome::ExternalStatus(status)
        },
    }
}

/// Parse `contracts --scope PATH [--scope PATH ...] [--nextest-list-fixture
/// PATH]`.
fn parse_contracts<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut scopes = Vec::new();
    let mut nextest_list_fixture = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--scope") {
            let value = take_option_value("--scope", &mut arguments)?;
            scopes.push(PathBuf::from(value));
        }
        else if argument == OsStr::new("--nextest-list-fixture") {
            let value = take_option_value("--nextest-list-fixture", &mut arguments)?;
            set_once(
                &mut nextest_list_fixture,
                "--nextest-list-fixture",
                PathBuf::from(value),
            )?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    if scopes.is_empty() {
        return Err(GateError::usage(
            "contracts requires at least one --scope PATH",
        ));
    }
    Ok(Command::Contracts {
        scopes,
        nextest_list_fixture,
    })
}

/// Parse `ci-contracts --workflow PATH`.
fn parse_ci_contracts<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut workflow = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--workflow") {
            let value = take_option_value("--workflow", &mut arguments)?;
            set_once(&mut workflow, "--workflow", PathBuf::from(value))?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    let workflow = required_value(workflow, "ci-contracts requires --workflow PATH")?;
    Ok(Command::CiContracts { workflow })
}

/// Parse `graph-boundary --workspace-root PATH [--metadata-fixture PATH]`.
fn parse_graph_boundary<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut workspace_root = None;
    let mut metadata_fixture = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--workspace-root") {
            let value = take_option_value("--workspace-root", &mut arguments)?;
            set_once(
                &mut workspace_root,
                "--workspace-root",
                PathBuf::from(value),
            )?;
        }
        else if argument == OsStr::new("--metadata-fixture") {
            let value = take_option_value("--metadata-fixture", &mut arguments)?;
            set_once(
                &mut metadata_fixture,
                "--metadata-fixture",
                PathBuf::from(value),
            )?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    let workspace_root = required_value(
        workspace_root,
        "graph-boundary requires --workspace-root PATH",
    )?;
    Ok(Command::GraphBoundary {
        workspace_root,
        metadata_fixture,
    })
}

/// Parse `docs-manifest [--manifest PATH]`.
fn parse_docs_manifest<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let manifest_path = parse_optional_manifest_path(arguments)?;
    Ok(Command::DocsManifest { manifest_path })
}

/// Parse `docs-reference [--manifest PATH]`.
fn parse_docs_reference<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let manifest_path = parse_optional_manifest_path(arguments)?;
    Ok(Command::DocsReference { manifest_path })
}

/// Parse a documentation command's optional manifest path.
fn parse_optional_manifest_path<Arguments>(arguments: Arguments) -> Result<PathBuf, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut manifest_path = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--manifest") {
            let value = take_option_value("--manifest", &mut arguments)?;
            set_once(&mut manifest_path, "--manifest", PathBuf::from(value))?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    Ok(manifest_path.unwrap_or_else(default_manifest_path))
}

/// Parse `page-balance [--cwd PATH]`.
fn parse_page_balance<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    Ok(Command::PageBalance {
        cwd: parse_optional_cwd(arguments)?,
    })
}

/// Parse `rumdl fmt|check [--cwd PATH] [PATH ...]`.
fn parse_rumdl<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter();
    let mode = arguments
        .next()
        .ok_or_else(|| GateError::usage("rumdl requires fmt or check"))?;
    let mode = os_string_into_utf8("rumdl mode", mode)?;
    let mode = docs::commands::RumdlMode::parse(&mode)?;
    let mut cwd = None;
    let mut paths = Vec::new();
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--cwd") {
            let value = take_option_value("--cwd", &mut arguments)?;
            set_once(&mut cwd, "--cwd", PathBuf::from(value))?;
        }
        else if is_option_token(&argument).into().0 {
            return Err(unknown_argument(&argument));
        }
        else {
            paths.push(PathBuf::from(argument));
        }
    }
    Ok(Command::Rumdl { mode, paths, cwd })
}

/// Parse `options-policy --workspace-root PATH`.
fn parse_options_policy<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    Ok(Command::OptionsPolicy {
        workspace_root: parse_required_workspace_root(arguments, "options-policy")?,
    })
}

/// Parse `soundness-oracles --workspace-root PATH`.
fn parse_soundness_oracles<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    Ok(Command::SoundnessOracles {
        workspace_root: parse_required_workspace_root(arguments, "soundness-oracles")?,
    })
}

/// Parse `default-graph --workspace-root PATH`.
fn parse_default_graph<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    Ok(Command::DefaultGraph {
        workspace_root: parse_required_workspace_root(arguments, "default-graph")?,
    })
}

/// Parse `iu-pin --workspace-root PATH`.
fn parse_iu_pin<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    Ok(Command::IuPin {
        workspace_root: parse_required_workspace_root(arguments, "iu-pin")?,
    })
}

/// Parse `agda-deps [--workspace-root PATH]`.
fn parse_agda_deps<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    Ok(Command::AgdaDeps {
        workspace_root: parse_optional_workspace_root(arguments)?,
    })
}

/// Parse a command that accepts exactly one required `--workspace-root PATH`.
fn parse_required_workspace_root<'semantic, Arguments, CommandName>(
    arguments: Arguments,
    command_name: CommandName,
) -> Result<PathBuf, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
    CommandName: Into<CommandNameText<'semantic>>,
{
    let command_name = command_name.into().0;
    let mut workspace_root = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--workspace-root") {
            let value = take_option_value("--workspace-root", &mut arguments)?;
            set_once(
                &mut workspace_root,
                "--workspace-root",
                PathBuf::from(value),
            )?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    required_value(
        workspace_root,
        format!("{command_name} requires --workspace-root PATH"),
    )
}

/// Parse an optional `--workspace-root PATH`.
fn parse_optional_workspace_root<Arguments>(
    arguments: Arguments
) -> Result<Option<PathBuf>, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut workspace_root = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--workspace-root") {
            let value = take_option_value("--workspace-root", &mut arguments)?;
            set_once(
                &mut workspace_root,
                "--workspace-root",
                PathBuf::from(value),
            )?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    Ok(workspace_root)
}

/// Parse `coverage check|ratchet --repo-root PATH [--summary PATH] [--floors
/// PATH]`.
fn parse_coverage<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter();
    let mode = arguments
        .next()
        .ok_or_else(|| GateError::usage("coverage requires check or ratchet"))?;
    let mode_text = os_string_into_utf8("coverage mode", mode)?;
    let mode = match mode_text.as_str() {
        | "check" => CoverageCommand::Check,
        | "ratchet" => CoverageCommand::Ratchet,
        | other => {
            return Err(GateError::usage(format!(
                "unsupported coverage command `{other}`"
            )));
        },
    };
    let mut repo_root = None;
    let mut summary_path = None;
    let mut floors_path = None;
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--repo-root") {
            let value = take_option_value("--repo-root", &mut arguments)?;
            set_once(&mut repo_root, "--repo-root", PathBuf::from(value))?;
        }
        else if argument == OsStr::new("--summary") {
            let value = take_option_value("--summary", &mut arguments)?;
            set_once(&mut summary_path, "--summary", PathBuf::from(value))?;
        }
        else if argument == OsStr::new("--floors") {
            let value = take_option_value("--floors", &mut arguments)?;
            set_once(&mut floors_path, "--floors", PathBuf::from(value))?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    let repo_root = required_value(repo_root, "coverage requires --repo-root PATH")?;
    let summary_path = summary_path
        .unwrap_or_else(|| repo_root.join(gandr_workflow_gates::coverage::DEFAULT_SUMMARY));
    let floors_path = floors_path
        .unwrap_or_else(|| repo_root.join(gandr_workflow_gates::coverage::DEFAULT_FLOORS));
    Ok(Command::Coverage {
        mode,
        summary_path,
        floors_path,
        repo_root,
    })
}

/// Parse maintenance range selection and publication options.
fn parse_maintenance_range<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter().peekable();
    if arguments
        .peek()
        .is_some_and(|argument| argument == OsStr::new("advance"))
    {
        drop(arguments.next());
        return parse_maintenance_advance(arguments);
    }
    parse_maintenance_resolve(arguments)
}

/// Parse `maintenance-range advance --watermark PATH [--to REF]`.
fn parse_maintenance_advance<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut watermark = None;
    let mut to = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--watermark") {
            let value = take_option_value("--watermark", &mut arguments)?;
            set_once(&mut watermark, "--watermark", PathBuf::from(value))?;
        }
        else if argument == OsStr::new("--to") {
            let value = take_utf8_option_value("--to", &mut arguments)?;
            let to_ref = maintenance::GitRef::new(&value)?;
            set_once(&mut to, "--to", to_ref)?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    let watermark = required_value(
        watermark,
        "maintenance-range advance requires --watermark PATH",
    )?;
    let to = match to {
        | Some(to) => to,
        | None => maintenance::GitRef::head()?,
    };
    Ok(Command::MaintenanceAdvance { watermark, to })
}

/// Parse legacy-compatible maintenance range resolution options.
fn parse_maintenance_resolve<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut github_output = None;
    let mut head = None;
    let mut explicit_from = None;
    let mut watermark = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--github-output") {
            let value = take_option_value("--github-output", &mut arguments)?;
            set_once(&mut github_output, "--github-output", PathBuf::from(value))?;
        }
        else if argument == OsStr::new("--head") {
            let value = take_utf8_option_value("--head", &mut arguments)?;
            let head_ref = maintenance::GitRef::new(&value)?;
            set_once(&mut head, "--head", head_ref)?;
        }
        else if argument == OsStr::new("--from") {
            let value = take_utf8_option_value("--from", &mut arguments)?;
            let from_ref = maintenance::GitRef::new(value.trim())?;
            set_once(&mut explicit_from, "--from", from_ref)?;
        }
        else if argument == OsStr::new("--watermark") {
            let value = take_option_value("--watermark", &mut arguments)?;
            set_once(&mut watermark, "--watermark", PathBuf::from(value))?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    let github_output = required_value(
        github_output,
        "maintenance-range requires --github-output PATH",
    )?;
    let head = match head {
        | Some(head) => head,
        | None => maintenance::GitRef::head()?,
    };
    Ok(Command::MaintenanceRange {
        github_output,
        head,
        explicit_from,
        watermark,
    })
}

/// Parse the mutation campaign facade command.
fn parse_mutants<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter();
    let mode = arguments
        .next()
        .ok_or_else(|| GateError::usage("mutants requires a mode"))?;
    let mode_text = os_string_into_utf8("mutants mode", mode)?;
    match mode_text.as_str() {
        | "snapshot" => parse_mutants_host(MutantsHostMode::Snapshot, arguments),
        | "push" => parse_mutants_host(MutantsHostMode::Push, arguments),
        | "merge" => parse_mutants_host(MutantsHostMode::Merge, arguments),
        | "scheduled" => parse_mutants_host(MutantsHostMode::Scheduled, arguments),
        | "sweep" => parse_mutants_host(MutantsHostMode::Sweep, arguments),
        | "clean" => parse_mutants_host(MutantsHostMode::Clean, arguments),
        | "guest" => parse_mutants_guest(arguments),
        | other => Err(GateError::usage(format!(
            "unsupported mutants mode `{other}`"
        ))),
    }
}

/// Parse guest-side mutants flags.
fn parse_mutants_guest<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut common = MutantsCommonOptions::default();
    let mut package = None;
    let mut diff = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if consume_mutants_common_argument(&argument, &mut arguments, &mut common)
            .map(|value| value.into().0)?
        {
            continue;
        }
        if argument == OsStr::new("--package") {
            let value = take_utf8_option_value("--package", &mut arguments)?;
            set_once(&mut package, "--package", value)?;
        }
        else if argument == OsStr::new("--diff") {
            let value = take_option_value("--diff", &mut arguments)?;
            set_once(&mut diff, "--diff", PathBuf::from(value))?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    Ok(Command::Mutants {
        command: mutants::MutantsCommand::Guest { package, diff },
        options: common.into_guest_options(),
    })
}

/// Return the current working directory as the default mutants workspace.
fn current_workspace_root() -> Result<PathBuf, GateError>
{
    gandr_workflow_gates::support::HOST_FILESYSTEM.current_dir()
}

/// Expand the default mutants cache image under the current home directory.
fn default_mutants_cache_image() -> Result<PathBuf, GateError>
{
    let Some(home) = std::env::var_os("HOME")
    else {
        return Err(GateError::operational(format!(
            "mutants host modes require HOME to expand {MUTANTS_DEFAULT_CACHE_IMAGE}"
        )));
    };
    if home.as_os_str().is_empty() {
        return Err(GateError::operational(format!(
            "mutants host modes require nonempty HOME to expand {MUTANTS_DEFAULT_CACHE_IMAGE}"
        )));
    }
    Ok(PathBuf::from(home).join(".microsandbox/gandr-mutants-cache.btrfs"))
}

/// Create collision-resistant default temporary paths for one mutants mode.
fn default_mutants_temporary_paths(
    workspace_root: &Path,
    mode: MutantsHostMode,
) -> Result<MutantsTemporaryPaths, GateError>
{
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| {
            GateError::operational(format!("system clock before UNIX epoch: {source}"))
        })?;
    let nonce = nonce.as_nanos();

    for attempt in 0_u16 .. MUTANTS_TEMPORARY_PATH_ATTEMPTS {
        let paths = mutants_temporary_paths_candidate(workspace_root, mode, nonce, attempt);
        if mutants_temporary_paths_are_available(&paths).map(|value| value.into().0)? {
            return Ok(paths);
        }
    }

    Err(GateError::operational(format!(
        "could not create unique temporary mutation paths for {}",
        mode.as_str().as_ref()
    )))
}

/// Build one default temporary path candidate set.
fn mutants_temporary_paths_candidate<Nonce, Attempt>(
    workspace_root: &Path,
    mode: MutantsHostMode,
    nonce: Nonce,
    attempt: Attempt,
) -> MutantsTemporaryPaths
where
    Nonce: Into<NonceNonce>,
    Attempt: Into<AttemptCount>,
{
    let attempt = attempt.into().0;
    let nonce = nonce.into().0;
    let mut hasher = blake3::Hasher::new();
    hasher.update(workspace_root.as_os_str().as_encoded_bytes());
    hasher.update(b"\0");
    hasher.update(mode.as_str().as_ref().as_bytes());
    hasher.update(b"\0");
    hasher.update(&std::process::id().to_le_bytes());
    hasher.update(&nonce.to_le_bytes());
    hasher.update(&attempt.to_le_bytes());
    let token = hasher.finalize();
    let token_hex = token.to_hex();
    let base_name = format!(
        "{MUTANTS_TEMPORARY_PATH_PREFIX}-{}-{}",
        mode.as_str().as_ref(),
        token_hex.as_str()
    );
    let temp_dir = std::env::temp_dir();
    MutantsTemporaryPaths {
        source_archive: temp_dir.join(format!("{base_name}-source.tar")),
        diff_file: temp_dir.join(format!("{base_name}-diff.patch")),
        working_report: temp_dir.join(format!("{base_name}-report")),
    }
}

/// Return whether no generated temporary path already exists.
fn mutants_temporary_paths_are_available(
    paths: &MutantsTemporaryPaths
) -> Result<impl Into<MutantsTemporaryPathsAreAvailableFlag>, GateError>
{
    for path in [
        &paths.source_archive,
        &paths.diff_file,
        &paths.working_report,
    ] {
        if gandr_workflow_gates::support::HOST_FILESYSTEM
            .try_exists(path)
            .map(bool::from)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Consume a common mutants option when `argument` names one.
fn consume_mutants_common_argument<Arguments>(
    argument: &OsStr,
    arguments: &mut core::iter::Peekable<Arguments>,
    common: &mut MutantsCommonOptions,
) -> Result<impl Into<ConsumeMutantsCommonArgumentFlag>, GateError>
where
    Arguments: Iterator<Item = OsString>,
{
    if argument == OsStr::new("--workspace-root") {
        let value = take_option_value("--workspace-root", arguments)?;
        set_once(
            &mut common.workspace_root,
            "--workspace-root",
            PathBuf::from(value),
        )?;
        return Ok(true);
    }
    if argument == OsStr::new("--cache-image") {
        let value = take_option_value("--cache-image", arguments)?;
        set_once(
            &mut common.cache_image,
            "--cache-image",
            PathBuf::from(value),
        )?;
        return Ok(true);
    }
    if argument == OsStr::new("--source-archive") {
        let value = take_option_value("--source-archive", arguments)?;
        set_once(
            &mut common.source_archive,
            "--source-archive",
            PathBuf::from(value),
        )?;
        return Ok(true);
    }
    if argument == OsStr::new("--diff-file") {
        let value = take_option_value("--diff-file", arguments)?;
        set_once(&mut common.diff_file, "--diff-file", PathBuf::from(value))?;
        return Ok(true);
    }
    if argument == OsStr::new("--working-report") {
        let value = take_option_value("--working-report", arguments)?;
        set_once(
            &mut common.working_report,
            "--working-report",
            PathBuf::from(value),
        )?;
        return Ok(true);
    }
    Ok(false)
}

/// Consume a `mutants push` option.
fn consume_mutants_push_argument<Arguments>(
    argument: &OsStr,
    arguments: &mut core::iter::Peekable<Arguments>,
    push: &mut MutantsPushOptions,
) -> Result<(), GateError>
where
    Arguments: Iterator<Item = OsString>,
{
    if argument == OsStr::new("--range-mode") {
        let value = take_utf8_option_value("--range-mode", arguments)?;
        let mode = match value.as_str() {
            | "range" => MutantsPushRangeMode::Range,
            | "full" => MutantsPushRangeMode::Full,
            | "last" => MutantsPushRangeMode::Last,
            | other => {
                return Err(GateError::usage(format!(
                    "unsupported mutants push range mode `{other}`"
                )));
            },
        };
        return set_once(&mut push.range_mode, "--range-mode", mode);
    }
    if argument == OsStr::new("--from") {
        let value = take_utf8_option_value("--from", arguments)?;
        return set_once(&mut push.from, "--from", value);
    }
    if argument == OsStr::new("--root") {
        let value = take_utf8_option_value("--root", arguments)?;
        return set_once(&mut push.root, "--root", value);
    }
    if argument == OsStr::new("--to") {
        let value = take_utf8_option_value("--to", arguments)?;
        return set_once(&mut push.to, "--to", value);
    }
    Err(unknown_argument(argument))
}

/// Consume a `mutants scheduled` option.
fn consume_mutants_scheduled_argument<Arguments>(
    argument: &OsStr,
    arguments: &mut core::iter::Peekable<Arguments>,
    scheduled: &mut MutantsScheduledOptions,
) -> Result<(), GateError>
where
    Arguments: Iterator<Item = OsString>,
{
    if argument == OsStr::new("--from") {
        let value = take_utf8_option_value("--from", arguments)?;
        return set_once(&mut scheduled.from_ref, "--from", value);
    }
    if argument == OsStr::new("--to") {
        let value = take_utf8_option_value("--to", arguments)?;
        return set_once(&mut scheduled.to_ref, "--to", value);
    }
    Err(unknown_argument(argument))
}

/// Parse `workflow merge|push [--cwd PATH]`.
fn parse_workflow<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut arguments = arguments.into_iter();
    let mode = arguments
        .next()
        .ok_or_else(|| GateError::usage("workflow requires merge or push"))?;
    let tier_text = os_string_into_utf8("workflow mode", mode)?;
    let tier = match tier_text.as_str() {
        | "merge" => gandr_workflow_gates::workflow::Tier::Merge,
        | "push" => gandr_workflow_gates::workflow::Tier::Push,
        | other => {
            return Err(GateError::usage(format!(
                "unsupported workflow mode `{other}`"
            )));
        },
    };
    Ok(Command::Workflow {
        tier,
        cwd: parse_optional_cwd(arguments)?,
    })
}

/// Parse `fuzz-smoke [--target lower|parse|check|parity|gates]`.
fn parse_fuzz_smoke<Arguments>(arguments: Arguments) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut target = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--target") {
            let value = take_utf8_option_value("--target", &mut arguments)?;
            let parsed_target = parse_fuzz_smoke_target(&value)?;
            set_once(&mut target, "--target", parsed_target)?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    let targets = match target {
        | Some(target) => vec![target],
        | None => FUZZ_TARGETS.to_vec(),
    };
    Ok(Command::FuzzSmoke {
        plan: FuzzSmokePlan::new(targets),
    })
}

/// Resolve the Agda dependency workspace root.
///
/// # Contract
/// - ensures: returns the caller-supplied workspace root unchanged, or the
///   current directory when the CLI omitted `--workspace-root`.
/// - provides: legacy-compatible current-directory defaulting for `agda-deps`.
/// - fails: returns [`GateError::Io`] when the current directory cannot be
///   read.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Io`] if [`std::env::current_dir`] fails.
///
/// # Adequacy
/// - hypothesis: L3 only — parser and command-plan tests observe explicit
///   roots; runtime current-directory failure is delegated to the standard
///   library.
/// - witness: `tooling::agda_deps_plan_uses_sanitized_git_commands_without_execution`
fn agda_workspace_root(workspace_root: Option<&Path>) -> Result<PathBuf, GateError>
{
    match workspace_root {
        | Some(root) => Ok(root.to_path_buf()),
        | None => gandr_workflow_gates::support::HOST_FILESYSTEM.current_dir(),
    }
}

/// Run the Agda standard-library dependency provisioning plan.
///
/// # Contract
/// - requires: `plan.workspace_root()` names the workspace root whose
///   `metatheory/` tree should receive the dependency checkout.
/// - ensures: writes `metatheory/libraries` with the canonical standard-library
///   `.agda-lib` path and emits the stable ready line on success.
/// - provides: a Rust-backed second path to what `scripts/agda-deps.gandr`
///   provisions, with typed Git argv and shared Git environment sanitization.
///   No task invokes it: `mise run agda:deps` runs the gandr script, and
///   whether this command is retired or kept as a driver-independent fallback
///   is an open owner decision.
/// - fails: returns typed filesystem, output, or Git status errors.
/// - panics: none.
/// - intension: skips Git when the `.agda-lib` file already exists; otherwise
///   clone is used for a missing checkout and fetch+checkout for a present
///   checkout that lacks the ready file.
///
/// # Errors
/// Returns [`GateError`] for directory creation, Git subprocess failure,
/// canonicalization, library-file publication, or stdout write failure.
///
/// # Adequacy
/// - hypothesis: L3 only — pure CLI tests pin the clone/fetch/checkout argv and
///   sanitization plan; support tests pin the inherited Git controls removed by
///   the shared runner.
/// - witness: `tooling::agda_deps_plan_uses_sanitized_git_commands_without_execution`
/// - witness: `gandr_workflow_gates::support::tests::git_environment_sanitizer_removes_only_git_keys`
fn run_agda_deps_plan(plan: &AgdaDependencyPlan) -> Result<(), GateError>
{
    ensure_agda_stdlib(plan)?;
    write_agda_libraries_file(plan)?;
    write_agda_ready_stdout()
}

/// Ensure the Agda standard-library checkout exists at the requested branch.
///
/// # Contract
/// - requires: `plan.workspace_root()` is the checkout root to provision.
/// - ensures: returns successfully only when the standard-library `.agda-lib`
///   file is present after any needed Git operation.
/// - provides: the cache-aware clone-or-refresh decision for Agda dependencies.
/// - fails: returns typed directory, Git status, or missing-ready-file errors.
/// - panics: none.
/// - intension: the ready file short-circuits Git; an existing checkout
///   refreshes through fetch+checkout, while a missing checkout uses clone.
///
/// # Errors
/// Returns [`GateError`] for vendor directory creation, checkout existence
/// probing, nonzero Git status, or a missing ready file after provisioning.
///
/// # Adequacy
/// - hypothesis: L3 only — command-plan tests pin the side-effecting Git
///   choices and support tests pin environment sanitization for those choices.
/// - witness: `tooling::agda_deps_plan_uses_sanitized_git_commands_without_execution`
/// - witness: `gandr_workflow_gates::support::tests::git_environment_sanitizer_removes_only_git_keys`
fn ensure_agda_stdlib(plan: &AgdaDependencyPlan) -> Result<(), GateError>
{
    let stdlib_lib = agda_stdlib_lib_path(plan);
    if stdlib_lib.is_file() {
        return Ok(());
    }

    let vendor = agda_vendor_dir(plan);
    gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(vendor)?;

    let stdlib = agda_stdlib_dir(plan);
    if gandr_workflow_gates::support::HOST_FILESYSTEM
        .try_exists(stdlib)
        .map(bool::from)?
    {
        run_agda_git_command("fetch", &agda_fetch_command_plan(plan))?;
        run_agda_git_command("checkout", &agda_checkout_command_plan(plan))?;
    }
    else {
        run_agda_git_command("clone", &agda_clone_command_plan(plan))?;
    }

    if agda_stdlib_lib_path(plan).is_file() {
        Ok(())
    }
    else {
        Err(GateError::operational(format!(
            "agda-deps: missing {} after provisioning",
            agda_stdlib_lib_path(plan).display()
        )))
    }
}

/// Write the Agda libraries file used by `agda`.
///
/// # Contract
/// - requires: the standard-library `.agda-lib` file exists under the planned
///   workspace root.
/// - ensures: writes one newline-terminated canonical `.agda-lib` path to
///   `metatheory/libraries`.
/// - provides: the legacy Agda library registration side effect.
/// - fails: returns typed canonicalization or write errors.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Io`] when canonicalizing the library path or writing
/// the libraries file fails.
///
/// # Adequacy
/// - hypothesis: L3 only — the smoke run observes the canonical libraries file
///   contents after provisioning.
/// - witness: `manual smoke: cargo run -p gandr-workflow-gates -- agda-deps
///   --workspace-root <temp>`
fn write_agda_libraries_file(plan: &AgdaDependencyPlan) -> Result<(), GateError>
{
    let stdlib_lib = agda_stdlib_lib_path(plan);
    let canonical_stdlib_lib =
        gandr_workflow_gates::support::HOST_FILESYSTEM.canonicalize(stdlib_lib)?;
    let mut line = canonical_stdlib_lib.to_string_lossy().into_owned();
    line.push('\n');
    let libraries = agda_libraries_file(plan);
    gandr_workflow_gates::support::HOST_FILESYSTEM.write(libraries, line)
}

/// Emit the stable Agda dependency success line.
///
/// # Contract
/// - ensures: writes and flushes `agda deps ready: stdlib v2.4\n` to stdout.
/// - provides: the same success marker `scripts/agda-deps.gandr` prints, so the
///   two provisioning paths stay observably identical while both exist.
/// - fails: returns a typed stdout write or flush error.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Io`] when stdout cannot be written or flushed.
///
/// # Adequacy
/// - hypothesis: L3 only — the smoke run observes the exact stdout marker.
/// - witness: `manual smoke: cargo run -p gandr-workflow-gates -- agda-deps
///   --workspace-root <temp>`
fn write_agda_ready_stdout() -> Result<(), GateError>
{
    let mut stdout_lock = std::io::stdout().lock();
    stdout_lock
        .write_all(AGDA_DEPS_READY_STDOUT.as_bytes())
        .and_then(|()| stdout_lock.flush())
        .map_err(|source| GateError::Io {
            path: PathBuf::from("stdout"),
            source,
        })
}

/// Return the Agda standard-library checkout path for `plan`.
fn agda_stdlib_dir(plan: &AgdaDependencyPlan) -> PathBuf
{
    plan.workspace_root().join(AGDA_STDLIB_DIR)
}

/// Return the Agda standard-library `.agda-lib` path for `plan`.
fn agda_stdlib_lib_path(plan: &AgdaDependencyPlan) -> PathBuf
{
    plan.workspace_root().join(AGDA_STDLIB_LIB)
}

/// Return the Agda vendor directory path for `plan`.
fn agda_vendor_dir(plan: &AgdaDependencyPlan) -> PathBuf
{
    plan.workspace_root().join(AGDA_VENDOR_DIR)
}

/// Return the `git fetch` command plan for an existing Agda stdlib checkout.
///
/// # Contract
/// - ensures: returns `-C metatheory/vendor/agda-stdlib fetch --depth 1 origin
///   v2.4` under the workspace root.
/// - provides: typed refresh argv without shell interpolation or ambient Git
///   repository controls.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the CLI plan witness asserts the exact argv, cwd,
///   and sanitization flag.
/// - witness: `tooling::agda_deps_plan_uses_sanitized_git_commands_without_execution`
#[must_use]
pub(crate) fn agda_fetch_command_plan(plan: &AgdaDependencyPlan) -> AgdaDependencyGitCommandPlan
{
    AgdaDependencyGitCommandPlan::new(plan.workspace_root().to_path_buf(), vec![
        OsString::from("-C"),
        OsString::from(AGDA_STDLIB_DIR),
        OsString::from("fetch"),
        OsString::from("--depth"),
        OsString::from("1"),
        OsString::from("origin"),
        OsString::from(AGDA_STDLIB_BRANCH),
    ])
}

/// Run one sanitized Git command plan for Agda dependency provisioning.
///
/// # Contract
/// - requires: `plan` was constructed by an Agda dependency command-plan
///   function.
/// - ensures: returns success only for a successful Git exit status.
/// - provides: the single Agda dependency boundary into
///   [`gandr_workflow_gates::support::run_status`] with Git sanitization
///   enabled.
/// - fails: returns process I/O errors or an operational nonzero-status error.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError`] from the support runner or for a nonzero Git status.
///
/// # Adequacy
/// - hypothesis: L3 only — command-plan tests assert every Agda Git command is
///   marked sanitized, while support tests assert which environment keys are
///   removed by the runner.
/// - witness: `tooling::agda_deps_plan_uses_sanitized_git_commands_without_execution`
/// - witness: `gandr_workflow_gates::support::tests::git_environment_sanitizer_removes_only_git_keys`
fn run_agda_git_command<'semantic, Action>(
    action: Action,
    plan: &AgdaDependencyGitCommandPlan,
) -> Result<(), GateError>
where
    Action: Into<ActionText<'semantic>>,
{
    let action = action.into().0;
    let status = gandr_workflow_gates::support::run_status(
        OsStr::new(GIT_PROGRAM),
        plan.args(),
        Some(plan.cwd()),
        plan.sanitized_git().0,
    )?;
    if status.success() {
        Ok(())
    }
    else {
        Err(GateError::operational(format!(
            "agda-deps: git {action} failed with {}",
            status_detail(status)
        )))
    }
}

/// Return the `git checkout` command plan for a fetched Agda stdlib revision.
///
/// # Contract
/// - ensures: returns `-C metatheory/vendor/agda-stdlib checkout --detach
///   FETCH_HEAD` under the workspace root.
/// - provides: typed checkout argv without shell interpolation or ambient Git
///   repository controls.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the CLI plan witness asserts the exact argv, cwd,
///   and sanitization flag.
/// - witness: `tooling::agda_deps_plan_uses_sanitized_git_commands_without_execution`
#[must_use]
pub(crate) fn agda_checkout_command_plan(plan: &AgdaDependencyPlan)
-> AgdaDependencyGitCommandPlan
{
    AgdaDependencyGitCommandPlan::new(plan.workspace_root().to_path_buf(), vec![
        OsString::from("-C"),
        OsString::from(AGDA_STDLIB_DIR),
        OsString::from("checkout"),
        OsString::from("--detach"),
        OsString::from("FETCH_HEAD"),
    ])
}

/// Return the `git clone` command plan for the Agda standard library.
///
/// # Contract
/// - ensures: returns `clone --depth 1 --branch v2.4 <repo>
///   metatheory/vendor/agda-stdlib` under the workspace root.
/// - provides: typed clone argv without shell interpolation or ambient Git
///   repository controls.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — the CLI plan witness asserts the exact argv, cwd,
///   and sanitization flag.
/// - witness: `tooling::agda_deps_plan_uses_sanitized_git_commands_without_execution`
#[must_use]
pub(crate) fn agda_clone_command_plan(plan: &AgdaDependencyPlan) -> AgdaDependencyGitCommandPlan
{
    AgdaDependencyGitCommandPlan::new(plan.workspace_root().to_path_buf(), vec![
        OsString::from("clone"),
        OsString::from("--depth"),
        OsString::from("1"),
        OsString::from("--branch"),
        OsString::from(AGDA_STDLIB_BRANCH),
        OsString::from(AGDA_STDLIB_REPOSITORY),
        OsString::from(AGDA_STDLIB_DIR),
    ])
}

/// Return the Agda libraries file path for `plan`.
fn agda_libraries_file(plan: &AgdaDependencyPlan) -> PathBuf
{
    plan.workspace_root().join(AGDA_LIBRARIES_FILE)
}

/// Run the deterministic AFL smoke plan.
///
/// # Contract
/// - requires: each target corpus directory exists under `fuzz/corpus/<target>`
///   and `cargo afl` is available on `PATH`.
/// - ensures: builds each target in plan order, then replays sorted regular
///   seed files once each by piping seed bytes to target stdin.
/// - provides: the runtime behavior behind `fuzz-smoke` and `mise run
///   fuzz:rust-smoke`.
/// - fails: returns an operational error on empty corpus, failed build, nonzero
///   replay, or process status without a portable exit code; returns I/O errors
///   for unreadable corpora, seeds, or commands.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError`] for process, filesystem, or smoke-contract failures.
///
/// # Adequacy
/// - hypothesis: L3 only — pure plan tests pin target order and argv; runtime
///   execution is deterministic by construction over sorted regular files.
fn run_fuzz_smoke_plan(plan: &FuzzSmokePlan) -> Result<(), GateError>
{
    for target in plan.targets() {
        run_fuzz_build(*target)?;
        let seeds = fuzz_seed_files(*target)?;
        if seeds.is_empty() {
            return Err(fuzz_error(format!(
                "fuzz-smoke: corpus for target `{}` is empty",
                target.as_str().as_ref()
            )));
        }
        for seed in seeds {
            replay_fuzz_seed(*target, &seed)?;
        }
    }
    Ok(())
}

/// Run `cargo afl build` for one target.
fn run_fuzz_build(target: FuzzSmokeTarget) -> Result<(), GateError>
{
    let plan = fuzz_build_command_plan(target);
    let status = run_streaming_status(&plan)?;
    if status.success() {
        return Ok(());
    }
    Err(fuzz_error(format!(
        "fuzz-smoke: build for target `{}` failed with {}",
        target.as_str().as_ref(),
        status_detail(status)
    )))
}

/// Return the streaming process plan for one AFL build.
#[must_use]
#[inline]
pub fn fuzz_build_command_plan(target: FuzzSmokeTarget) -> FuzzExternalCommandPlan
{
    FuzzExternalCommandPlan::new(
        PathBuf::from(CARGO_PROGRAM),
        fuzz_build_args(target),
        ExternalStream::Inherit,
        ExternalStream::Inherit,
        ExternalStream::Inherit,
    )
}

/// Return exact `cargo afl build` arguments for a target.
///
/// # Contract
/// - ensures: returns `afl build --manifest-path fuzz/Cargo.toml --bin
///   <target>` and appends the target's required Cargo feature exactly once.
/// - provides: pure argv planning for fuzz-smoke tests and execution.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — lower, parity, and gates target plan tests
///   distinguish the feature-free, parity-feature, and gates-feature branches.
#[must_use]
#[inline]
pub fn fuzz_build_args(target: FuzzSmokeTarget) -> Vec<OsString>
{
    let mut args = vec![
        OsString::from("afl"),
        OsString::from("build"),
        OsString::from("--manifest-path"),
        OsString::from(FUZZ_MANIFEST_PATH),
        OsString::from("--bin"),
        OsString::from(target.as_str().as_ref()),
    ];
    if let Some(feature) = target.required_feature() {
        args.push(OsString::from("--features"));
        args.push(OsString::from(feature.as_ref()));
    }
    args
}

/// Return the target binary path used for seed replay.
#[must_use]
#[inline]
pub fn fuzz_binary_path(target: FuzzSmokeTarget) -> PathBuf
{
    Path::new(FUZZ_TARGET_DEBUG_ROOT).join(target.as_str().as_ref())
}

/// Return sorted regular seed files for one target.
fn fuzz_seed_files(target: FuzzSmokeTarget) -> Result<Vec<PathBuf>, GateError>
{
    let corpus = fuzz_corpus_dir(target);
    let entries = std::fs::read_dir(&corpus).map_err(|source| GateError::Io {
        path: corpus.clone(),
        source,
    })?;
    let mut seeds = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| GateError::Io {
            path: corpus.clone(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| GateError::Io {
            path: path.clone(),
            source,
        })?;
        let metadata = gandr_workflow_gates::support::HOST_FILESYSTEM.metadata(&path)?;
        if !file_type.is_symlink() && metadata.is_file() {
            seeds.push(path);
        }
    }
    seeds.sort();
    Ok(seeds)
}

/// Return the corpus directory for one target.
#[must_use]
#[inline]
pub fn fuzz_corpus_dir(target: FuzzSmokeTarget) -> PathBuf
{
    Path::new(FUZZ_CORPUS_ROOT).join(target.as_str().as_ref())
}

/// Run a status-only command with inherited stdout and stderr.
fn run_streaming_status(
    plan: &FuzzExternalCommandPlan
) -> Result<std::process::ExitStatus, GateError>
{
    let mut command = configured_process_command(plan);
    command.status().map_err(|source| GateError::Io {
        path: plan.program().to_path_buf(),
        source,
    })
}

/// Spawn a streaming command from a pure process plan.
fn spawn_streaming_process(plan: &FuzzExternalCommandPlan)
-> Result<std::process::Child, GateError>
{
    let mut command = configured_process_command(plan);
    command.spawn().map_err(|source| GateError::Io {
        path: plan.program().to_path_buf(),
        source,
    })
}

/// Build a process command with the exact stream policy from `plan`.
fn configured_process_command(plan: &FuzzExternalCommandPlan) -> std::process::Command
{
    let mut command = std::process::Command::new(plan.program());
    command.args(plan.args());
    command.stdin(stdio_from_mode(plan.stdin()));
    command.stdout(stdio_from_mode(plan.stdout()));
    command.stderr(stdio_from_mode(plan.stderr()));
    command
}

/// Convert a pure stream policy to `std::process::Stdio`.
fn stdio_from_mode(mode: ExternalStream) -> std::process::Stdio
{
    match mode {
        | ExternalStream::Inherit => std::process::Stdio::inherit(),
        | ExternalStream::Piped => std::process::Stdio::piped(),
    }
}

/// Build a stable fuzz-smoke operational error.
fn fuzz_error<Detail>(detail: Detail) -> GateError
where
    Detail: Into<String>,
{
    let detail = detail.into();
    GateError::operational(detail)
}

/// Replay one seed through a previously built target binary.
fn replay_fuzz_seed(
    target: FuzzSmokeTarget,
    seed: &Path,
) -> Result<(), GateError>
{
    let seed_bytes = gandr_workflow_gates::support::HOST_FILESYSTEM.read(seed)?;
    let plan = fuzz_replay_command_plan(target);
    let mut child = spawn_streaming_process(&plan)?;
    let write_result = match child.stdin.as_mut() {
        | Some(stdin) => stdin.write_all(seed_bytes.as_bytes().into()),
        | None => Err(std::io::Error::other(
            "fuzz-smoke replay stdin was unavailable",
        )),
    };
    if let Err(source) = write_result {
        drop(child.kill());
        drop(child.wait());
        return Err(GateError::Io {
            path: seed.to_path_buf(),
            source,
        });
    }
    drop(child.stdin.take());
    let status = child.wait().map_err(|source| GateError::Io {
        path: plan.program().to_path_buf(),
        source,
    })?;
    if status.success() {
        return Ok(());
    }
    Err(fuzz_error(format!(
        "fuzz-smoke: target `{}` failed replaying seed `{}` with {}",
        target.as_str().as_ref(),
        display_path(seed),
        status_detail(status)
    )))
}

/// Return the streaming process plan for one seed replay.
#[must_use]
#[inline]
pub fn fuzz_replay_command_plan(target: FuzzSmokeTarget) -> FuzzExternalCommandPlan
{
    FuzzExternalCommandPlan::new(
        fuzz_binary_path(target),
        Vec::new(),
        ExternalStream::Piped,
        ExternalStream::Inherit,
        ExternalStream::Inherit,
    )
}

/// Take an option value, rejecting missing values and option-like tokens.
///
/// # Contract
/// - requires: `arguments` is positioned immediately after `option_name`.
/// - ensures: consumes and returns the next token when it is not option-shaped.
/// - provides: shared option-value validation for singleton and repeated
///   options.
/// - fails: returns a usage error when no value remains or the next token
///   starts with `--`.
/// - panics: none.
///
/// # Errors
/// Returns a usage error for missing option values.
///
/// # Adequacy
/// - hypothesis: L3 only — present-value, following-option, and end-of-iterator
///   branches are exercised by parser tests.
fn take_option_value<'semantic, Arguments, OptionName>(
    option_name: OptionName,
    arguments: &mut core::iter::Peekable<Arguments>,
) -> Result<OsString, GateError>
where
    Arguments: Iterator<Item = OsString>,
    OptionName: Into<OptionNameText<'semantic>>,
{
    let option_name = option_name.into().0;
    if arguments
        .peek()
        .is_some_and(|value| is_option_token(value).into().0)
    {
        return Err(GateError::usage(format!("missing value for {option_name}")));
    }
    arguments
        .next()
        .ok_or_else(|| GateError::usage(format!("missing value for {option_name}")))
}

/// Convert an OS string into UTF-8 for domains that require textual tokens.
fn os_string_into_utf8<'semantic, Label>(
    label: Label,
    value: OsString,
) -> Result<String, GateError>
where
    Label: Into<LabelText<'semantic>>,
{
    let label = label.into().0;
    value.into_string().map_err(|invalid| {
        GateError::usage(format!(
            "value for {label} must be valid UTF-8: `{}`",
            invalid.to_string_lossy()
        ))
    })
}

/// Parse host-side mutants flags shared by every non-guest mode.
fn parse_mutants_host<Arguments>(
    mode: MutantsHostMode,
    arguments: Arguments,
) -> Result<Command, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut common = MutantsCommonOptions::default();
    let mut push = MutantsPushOptions::default();
    let mut scheduled = MutantsScheduledOptions::default();
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if consume_mutants_common_argument(&argument, &mut arguments, &mut common)
            .map(|value| value.into().0)?
        {
            continue;
        }
        match mode {
            | MutantsHostMode::Push => {
                consume_mutants_push_argument(&argument, &mut arguments, &mut push)?;
            },
            | MutantsHostMode::Scheduled => {
                consume_mutants_scheduled_argument(&argument, &mut arguments, &mut scheduled)?;
            },
            | MutantsHostMode::Snapshot
            | MutantsHostMode::Merge
            | MutantsHostMode::Sweep
            | MutantsHostMode::Clean => return Err(unknown_argument(&argument)),
        }
    }
    let options = common.into_host_options(mode)?;
    let command = match mode {
        | MutantsHostMode::Snapshot => mutants::MutantsCommand::Snapshot,
        | MutantsHostMode::Push => mutants::MutantsCommand::Push {
            range: push.into_range_plan()?,
        },
        | MutantsHostMode::Merge => mutants::MutantsCommand::Merge,
        | MutantsHostMode::Scheduled => {
            let (from_ref, to_ref) = scheduled.into_required_refs()?;
            mutants::MutantsCommand::Scheduled { from_ref, to_ref }
        },
        | MutantsHostMode::Sweep => mutants::MutantsCommand::Sweep,
        | MutantsHostMode::Clean => mutants::MutantsCommand::Clean,
    };
    Ok(Command::Mutants { command, options })
}

/// Parse an optional `--cwd PATH` flag with no other accepted options.
fn parse_optional_cwd<Arguments>(arguments: Arguments) -> Result<Option<PathBuf>, GateError>
where
    Arguments: IntoIterator<Item = OsString>,
{
    let mut cwd = None;
    let mut arguments = arguments.into_iter().peekable();
    while let Some(argument) = arguments.next() {
        if argument == OsStr::new("--cwd") {
            let value = take_option_value("--cwd", &mut arguments)?;
            set_once(&mut cwd, "--cwd", PathBuf::from(value))?;
        }
        else {
            return Err(unknown_argument(&argument));
        }
    }
    Ok(cwd)
}

/// Return whether an OS string is shaped like a command option.
fn is_option_token(value: &OsStr) -> impl Into<OptionTokenFlag>
{
    value.as_encoded_bytes().starts_with(b"--")
}

/// Set an optional singleton or return its duplicate-flag usage error.
fn set_once<'semantic, T, OptionName>(
    slot: &mut Option<T>,
    option_name: OptionName,
    value: T,
) -> Result<(), GateError>
where
    OptionName: Into<OptionNameText<'semantic>>,
{
    let option_name = option_name.into().0;
    if slot.is_some() {
        return Err(GateError::usage(format!("duplicate {option_name}")));
    }
    *slot = Some(value);
    Ok(())
}

/// Take an option value that must be valid UTF-8.
fn take_utf8_option_value<'semantic, Arguments, OptionName>(
    option_name: OptionName,
    arguments: &mut core::iter::Peekable<Arguments>,
) -> Result<String, GateError>
where
    Arguments: Iterator<Item = OsString>,
    OptionName: Into<OptionNameText<'semantic>>,
{
    let option_name = option_name.into().0;
    let value = take_option_value(option_name, arguments)?;
    os_string_into_utf8(option_name, value)
}

/// Parse one allowed `fuzz-smoke --target` value.
fn parse_fuzz_smoke_target<'semantic, Value>(value: Value) -> Result<FuzzSmokeTarget, GateError>
where
    Value: Into<ValueText<'semantic>>,
{
    let value = value.into().0;
    match value {
        | "lower" => Ok(FuzzSmokeTarget::Lower),
        | "parse" => Ok(FuzzSmokeTarget::Parse),
        | "check" => Ok(FuzzSmokeTarget::Check),
        | "parity" => Ok(FuzzSmokeTarget::Parity),
        | "gates" => Ok(FuzzSmokeTarget::Gates),
        | other => Err(GateError::usage(format!(
            "unsupported fuzz-smoke target `{other}`"
        ))),
    }
}

/// Reject a mode-incompatible option that was supplied.
fn reject_present<T, Detail>(
    value: Option<&T>,
    detail: Detail,
) -> Result<(), GateError>
where
    Detail: Into<DetailText<'static>>,
{
    let detail = detail.into().0;
    if value.is_some() {
        return Err(GateError::usage(detail));
    }
    Ok(())
}

/// Build a stable unknown-argument usage error.
fn unknown_argument(argument: &OsStr) -> GateError
{
    GateError::usage(format!(
        "unknown argument `{}`",
        display_os_argument(argument)
    ))
}

/// Extract a required parser value or return a stable usage error.
fn required_value<T, Detail>(
    value: Option<T>,
    detail: Detail,
) -> Result<T, GateError>
where
    Detail: Into<String>,
{
    let detail = detail.into();
    value.ok_or_else(|| GateError::usage(detail))
}

/// Render an OS argument for stable diagnostics.
fn display_os_argument(argument: &OsStr) -> String
{
    argument.to_string_lossy().into_owned()
}

/// Render a path with lossy UTF-8 replacement for diagnostics.
fn display_path(path: &Path) -> String
{
    path.to_string_lossy().into_owned()
}

/// Render an exit status in stable operational diagnostics.
fn status_detail(status: std::process::ExitStatus) -> String
{
    match status.code() {
        | Some(code) => format!("exit status {code}"),
        | None => String::from("termination without exit code"),
    }
}

/// Return the default documentation manifest path from the docs domain.
fn default_manifest_path() -> PathBuf
{
    PathBuf::from(docs::manifest::DEFAULT_MANIFEST_PATH)
}

/// Return the short stable usage string for a missing command.
#[inline]
#[must_use]
pub fn usage_text() -> impl Into<UsageTextText<'static>>
{
    "usage: gandr-workflow-gates <command>; commands: contracts, ci-contracts, graph-boundary, docs-manifest, docs-reference, page-balance, rumdl, options-policy, soundness-oracles, default-graph, iu-pin, agda-deps [--workspace-root PATH], coverage, maintenance-range [advance], mutants, workflow, fuzz-smoke [--target lower|parse|check|parity|gates]"
}

/// Parsed supported CLI command.
///
/// # Contract
/// - ensures: variants carry domain-typed state, not stringly dispatch
///   payloads, for every retained command.
/// - provides: the typed boundary between manual parsing and synchronous domain
///   execution.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — integration tests pattern-match representative
///   variants and exact domain plan values.
pub enum Command
{
    /// Contract documentation adequacy gate.
    Contracts
    {
        /// One or more crate or source scopes to inspect.
        scopes: Vec<PathBuf>,
        /// Optional exact nextest list JSON fixture.
        nextest_list_fixture: Option<PathBuf>,
    },
    /// CI workflow run-step contract gate.
    CiContracts
    {
        /// GitHub Actions workflow file to inspect.
        workflow: PathBuf,
    },
    /// Graph dependency-boundary gate.
    GraphBoundary
    {
        /// Workspace root used for cargo metadata and path normalization.
        workspace_root: PathBuf,
        /// Optional cargo metadata JSON fixture.
        metadata_fixture: Option<PathBuf>,
    },
    /// Documentation-manifest drift gate.
    DocsManifest
    {
        /// Documentation manifest path.
        manifest_path: PathBuf,
    },
    /// Documentation-reference integrity gate.
    DocsReference
    {
        /// Documentation manifest path.
        manifest_path: PathBuf,
    },
    /// Typst page-balance probe command.
    PageBalance
    {
        /// Optional current working directory for the Typst probe.
        cwd: Option<PathBuf>,
    },
    /// Guarded rumdl command.
    Rumdl
    {
        /// Typed rumdl subcommand.
        mode: docs::commands::RumdlMode,
        /// Markdown paths forwarded to the guard and rumdl.
        paths: Vec<PathBuf>,
        /// Optional command working directory.
        cwd: Option<PathBuf>,
    },
    /// Agda OPTIONS policy gate.
    OptionsPolicy
    {
        /// Workspace root used to resolve default governed roots.
        workspace_root: PathBuf,
    },
    /// Rust soundness-oracle companion gate.
    SoundnessOracles
    {
        /// Workspace root used to resolve the default conformance source.
        workspace_root: PathBuf,
    },
    /// Default dependency graph policy gate.
    DefaultGraph
    {
        /// Cargo workspace root.
        workspace_root: PathBuf,
    },
    /// IU submodule pin policy gate.
    IuPin
    {
        /// Git workspace root.
        workspace_root: PathBuf,
    },
    /// Agda standard-library dependency provisioning command.
    AgdaDeps
    {
        /// Optional workspace root; current directory is used when absent.
        workspace_root: Option<PathBuf>,
    },
    /// Coverage floor policy command.
    Coverage
    {
        /// Coverage mode selected by the nested subcommand.
        mode: CoverageCommand,
        /// cargo-llvm-cov summary JSON path.
        summary_path: PathBuf,
        /// Coverage floors TOML path.
        floors_path: PathBuf,
        /// Repository root used for path normalization and Git base policy.
        repo_root: PathBuf,
    },
    /// Maintenance range selection and GitHub output publication.
    MaintenanceRange
    {
        /// GitHub Actions output file that receives `base=<oid>`.
        github_output: PathBuf,
        /// Upper revision for the next range.
        head: maintenance::GitRef,
        /// Optional explicit lower-bound ref.
        explicit_from: Option<maintenance::GitRef>,
        /// Optional runner-local watermark path.
        watermark: Option<PathBuf>,
    },
    /// Maintenance watermark advancement command.
    MaintenanceAdvance
    {
        /// Watermark path to replace with JSON state.
        watermark: PathBuf,
        /// Upper revision whose resolved commit becomes the next base.
        to: maintenance::GitRef,
    },
    /// Mutation campaign facade command.
    Mutants
    {
        /// Typed mutation mode and mode-specific options.
        command: mutants::MutantsCommand,
        /// Common mutation campaign paths.
        options: mutants::MutantsOptions,
    },
    /// Local workflow tier execution command.
    Workflow
    {
        /// Workflow tier selected by the nested mode.
        tier: gandr_workflow_gates::workflow::Tier,
        /// Optional working directory for `mise run` tasks.
        cwd: Option<PathBuf>,
    },
    /// Deterministic AFL harness smoke command.
    FuzzSmoke
    {
        /// Ordered smoke plan for one or all allowed AFL targets.
        plan: FuzzSmokePlan,
    },
}

/// Coverage nested command mode.
///
/// # Contract
/// - ensures: only `coverage check` and `coverage ratchet` are representable.
/// - provides: typed dispatch into the coverage policy domain.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — supported and unsupported mode strings are observed
///   by parser integration tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverageCommand
{
    /// Validate coverage floors without modifying them.
    Check,
    /// Rewrite coverage floors to the ratcheted policy.
    Ratchet,
}

/// Pure Agda dependency provisioning plan.
///
/// # Contract
/// - ensures: carries the workspace root used to derive all Agda dependency
///   filesystem paths and Git working directories.
/// - provides: a side-effect-free value for CLI parsing, tests, and runtime
///   provisioning.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — CLI tests distinguish explicit and default roots and
///   assert every Git command plan projected from the root.
/// - witness: `tooling::agda_deps_plan_uses_sanitized_git_commands_without_execution`
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub(crate) struct AgdaDependencyPlan
{
    /// Workspace root that owns `metatheory/`.
    workspace_root: PathBuf,
}

impl AgdaDependencyPlan
{
    /// Build a plan rooted at `workspace_root`.
    #[inline]
    #[must_use]
    pub(crate) fn new(workspace_root: PathBuf) -> Self
    {
        Self { workspace_root }
    }

    /// Borrow the workspace root.
    #[inline]
    #[must_use]
    pub(crate) fn workspace_root(&self) -> &Path
    {
        &self.workspace_root
    }
}

/// Pure Git command plan for one Agda dependency operation.
///
/// # Contract
/// - ensures: stores a working directory, exact argv vector, and the decision
///   to route execution through the shared sanitized Git runner.
/// - provides: process-free command witnesses for Agda dependency clone, fetch,
///   and checkout operations.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — CLI tests assert clone/fetch/checkout argv and the
///   sanitization bit before any process launch.
/// - witness: `tooling::agda_deps_plan_uses_sanitized_git_commands_without_execution`
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AgdaDependencyGitCommandPlan
{
    /// Working directory supplied to the Git subprocess.
    cwd: PathBuf,
    /// Exact argument vector supplied to Git.
    args: Vec<OsString>,
    /// Whether the shared support runner must remove Git repository controls.
    sanitized_git: bool,
}

impl AgdaDependencyGitCommandPlan
{
    /// Build a sanitized Git command plan.
    fn new(
        cwd: PathBuf,
        args: Vec<OsString>,
    ) -> Self
    {
        Self {
            cwd,
            args,
            sanitized_git: true,
        }
    }

    /// Borrow the Git working directory.
    #[inline]
    #[must_use]
    pub(crate) fn cwd(&self) -> &Path
    {
        &self.cwd
    }

    /// Borrow the exact Git argument vector.
    #[inline]
    #[must_use]
    pub(crate) fn args(&self) -> &[OsString]
    {
        &self.args
    }

    /// Return whether the shared support runner sanitizes Git controls.
    #[inline]
    #[must_use]
    pub(crate) const fn sanitized_git(&self) -> SanitizedGitFlag
    {
        SanitizedGitFlag(self.sanitized_git)
    }
}

/// AFL harness target accepted by `fuzz-smoke`.
///
/// # Contract
/// - ensures: arbitrary binary names cannot be represented.
/// - provides: the closed target set used for build, corpus, and replay paths.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — target parser tests cover every allowed spelling and
///   one rejected spelling.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FuzzSmokeTarget
{
    /// Lowering harness.
    Lower,
    /// Parser harness.
    Parse,
    /// Type/check harness.
    Check,
    /// Parity harness, built with the `parity` Cargo feature.
    Parity,
    /// Rust gate-suite harness.
    Gates,
}

impl FuzzSmokeTarget
{
    /// Return the fixed binary/corpus name for this target.
    ///
    /// # Contract
    /// - ensures: names are exactly `lower`, `parse`, `check`, `parity`, and
    ///   `gates`.
    /// - provides: the only string projection used in process argv and paths.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — pure plan tests assert every target projection.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> FuzzTargetNameText<'static>
    {
        match self {
            | Self::Lower => FuzzTargetNameText("lower"),
            | Self::Parse => FuzzTargetNameText("parse"),
            | Self::Check => FuzzTargetNameText("check"),
            | Self::Parity => FuzzTargetNameText("parity"),
            | Self::Gates => FuzzTargetNameText("gates"),
        }
    }

    /// Return the Cargo feature required to expose this binary, when any.
    #[inline]
    #[must_use]
    const fn required_feature(self) -> Option<FuzzFeatureNameText<'static>>
    {
        match self {
            | Self::Parity => Some(FuzzFeatureNameText(FUZZ_PARITY_FEATURE)),
            | Self::Gates => Some(FuzzFeatureNameText(FUZZ_GATES_FEATURE)),
            | Self::Lower | Self::Parse | Self::Check => None,
        }
    }
}

/// Deterministic `fuzz-smoke` execution plan.
///
/// # Contract
/// - ensures: targets are stored in the exact order they will be built and
///   replayed.
/// - provides: a pure plan value for inventory tests and the side-effecting
///   smoke runner.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — no-target, one-target, and all-target parser paths
///   are observed without invoking AFL.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct FuzzSmokePlan
{
    /// Ordered target list.
    targets: Vec<FuzzSmokeTarget>,
}

impl FuzzSmokePlan
{
    /// Build a smoke plan from already validated targets.
    #[inline]
    #[must_use]
    fn new(targets: Vec<FuzzSmokeTarget>) -> Self
    {
        Self { targets }
    }

    /// Borrow targets in execution order.
    #[inline]
    #[must_use]
    pub fn targets(&self) -> &[FuzzSmokeTarget]
    {
        &self.targets
    }
}

/// Streaming policy for a child-process stream.
///
/// # Contract
/// - ensures: fuzz-smoke process plans can distinguish inherited live streams
///   from the single piped stdin needed for seed replay.
/// - provides: a pure regression surface so CLI tests can reject buffered
///   execution paths.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — tooling tests assert build stdout/stderr inheritance and
///   replay stdin piping with inherited diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalStream
{
    /// Stream is inherited live by the child process.
    Inherit,
    /// Stream is piped to or from the child process.
    Piped,
}

/// Pure child-process plan used by fuzz-smoke.
///
/// # Contract
/// - ensures: program, argv, and stdio policy are represented before any
///   process launch occurs.
/// - provides: the shared plan used by runtime execution and integration tests.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — lower/parity build plans and gate replay plans
///   assert exact argv, binary path, and stream policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FuzzExternalCommandPlan
{
    /// Executable path or program name.
    program: PathBuf,
    /// Exact argument vector.
    args: Vec<OsString>,
    /// Standard-input policy.
    stdin: ExternalStream,
    /// Standard-output policy.
    stdout: ExternalStream,
    /// Standard-error policy.
    stderr: ExternalStream,
}

impl FuzzExternalCommandPlan
{
    /// Build a process plan from typed program, arguments, and streams.
    fn new(
        program: PathBuf,
        args: Vec<OsString>,
        stdin: ExternalStream,
        stdout: ExternalStream,
        stderr: ExternalStream,
    ) -> Self
    {
        Self {
            program,
            args,
            stdin,
            stdout,
            stderr,
        }
    }

    /// Borrow the executable path.
    #[inline]
    #[must_use]
    pub fn program(&self) -> &Path
    {
        &self.program
    }

    /// Borrow the exact argument vector.
    #[inline]
    #[must_use]
    pub fn args(&self) -> &[OsString]
    {
        &self.args
    }

    /// Return the stdin policy.
    #[inline]
    #[must_use]
    pub const fn stdin(&self) -> ExternalStream
    {
        self.stdin
    }

    /// Return the stdout policy.
    #[inline]
    #[must_use]
    pub const fn stdout(&self) -> ExternalStream
    {
        self.stdout
    }

    /// Return the stderr policy.
    #[inline]
    #[must_use]
    pub const fn stderr(&self) -> ExternalStream
    {
        self.stderr
    }
}

/// Result of a successfully executed command.
///
/// # Contract
/// - ensures: semantic findings retain the historical `1` exit, clean commands
///   retain `0`, usage/operational failures are excluded, and external commands
///   can preserve their own status.
/// - provides: stable process-exit classification after dispatch.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — empty/nonempty findings, rumdl status, and
///   page-balance note paths are separately represented.
pub enum GateOutcome
{
    /// No semantic findings were emitted.
    Clean,
    /// Stable semantic findings to print before exiting with code `1`.
    Findings(Vec<Finding>),
    /// External command status to forward.
    ExternalStatus(std::process::ExitStatus),
    /// Page-balance report with optional informational notes.
    PageBalance(docs::commands::PageBalanceReport),
}

impl GateOutcome
{
    /// Convert analyzer findings into an output outcome.
    fn from_findings(findings: Vec<Finding>) -> Self
    {
        if findings.is_empty() {
            Self::Clean
        }
        else {
            Self::Findings(findings)
        }
    }

    /// Print any outcome payload and return the required process exit code.
    fn into_exit_code(self) -> std::process::ExitCode
    {
        match self {
            | Self::Clean => std::process::ExitCode::from(EXIT_CLEAN),
            | Self::Findings(findings) => {
                let mut stderr = std::io::stderr();
                for finding in findings {
                    drop(writeln!(stderr, "{finding}"));
                }
                std::process::ExitCode::from(EXIT_FINDINGS)
            },
            | Self::ExternalStatus(status) => exit_code_from_status(status),
            | Self::PageBalance(report) => {
                print_page_balance_report(&report);
                std::process::ExitCode::from(EXIT_CLEAN)
            },
        }
    }
}

/// Mutants host mode selected before parsing mode-specific flags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MutantsHostMode
{
    /// Provision or refresh the reusable microVM snapshot.
    Snapshot,
    /// Run a push-scoped campaign.
    Push,
    /// Run a merge-scoped campaign.
    Merge,
    /// Run a scheduled campaign.
    Scheduled,
    /// Run a full sweep campaign.
    Sweep,
    /// Clean stray owned sandboxes.
    Clean,
}

impl MutantsHostMode
{
    /// Return the stable lowercase mode name used in generated temporary paths.
    const fn as_str(self) -> ModeText<'static>
    {
        match self {
            | Self::Snapshot => ModeText("snapshot"),
            | Self::Push => ModeText("push"),
            | Self::Merge => ModeText("merge"),
            | Self::Scheduled => ModeText("scheduled"),
            | Self::Sweep => ModeText("sweep"),
            | Self::Clean => ModeText("clean"),
        }
    }
}

/// Optional common mutants fields collected before mode validation.
#[derive(Default)]
struct MutantsCommonOptions
{
    /// Workspace root for host campaign operations.
    workspace_root: Option<PathBuf>,
    /// Microsandbox cache image path.
    cache_image: Option<PathBuf>,
    /// Source archive path.
    source_archive: Option<PathBuf>,
    /// Unified diff path.
    diff_file: Option<PathBuf>,
    /// Working report directory path.
    working_report: Option<PathBuf>,
}

/// CLI-generated temporary paths for host mutation modes.
struct MutantsTemporaryPaths
{
    /// Temporary tracked-source archive path.
    source_archive: PathBuf,
    /// Temporary unified diff path.
    diff_file: PathBuf,
    /// Temporary cargo-mutants report directory.
    working_report: PathBuf,
}

impl MutantsCommonOptions
{
    /// Convert parsed host fields into the public mutants options facade.
    fn into_host_options(
        self,
        mode: MutantsHostMode,
    ) -> Result<mutants::MutantsOptions, GateError>
    {
        if mode == MutantsHostMode::Clean {
            return Ok(self.into_ignored_options());
        }

        let workspace_root = match self.workspace_root {
            | Some(workspace_root) => workspace_root,
            | None => current_workspace_root()?,
        };
        let cache_image = match self.cache_image {
            | Some(cache_image) => cache_image,
            | None => default_mutants_cache_image()?,
        };
        let temporary_paths = default_mutants_temporary_paths(&workspace_root, mode)?;

        Ok(mutants::MutantsOptions::new(
            workspace_root,
            cache_image,
            self.source_archive
                .unwrap_or(temporary_paths.source_archive),
            self.diff_file.unwrap_or(temporary_paths.diff_file),
            self.working_report
                .unwrap_or(temporary_paths.working_report),
        ))
    }

    /// Convert optional guest fields into a facade value that guest mode
    /// ignores.
    fn into_guest_options(self) -> mutants::MutantsOptions
    {
        self.into_ignored_options()
    }

    /// Convert fields for modes whose implementation ignores host paths.
    fn into_ignored_options(self) -> mutants::MutantsOptions
    {
        mutants::MutantsOptions::new(
            self.workspace_root.unwrap_or_default(),
            self.cache_image.unwrap_or_default(),
            self.source_archive.unwrap_or_default(),
            self.diff_file.unwrap_or_default(),
            self.working_report.unwrap_or_default(),
        )
    }
}

/// Push-range mode selected by `mutants push --range-mode`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MutantsPushRangeMode
{
    /// Shared-history `from...to` mode.
    Range,
    /// Force-rewrite `root...to` mode.
    Full,
    /// Last-commit mode.
    Last,
}

/// Optional `mutants push` parser state.
#[derive(Default)]
struct MutantsPushOptions
{
    /// Range mode selector.
    range_mode: Option<MutantsPushRangeMode>,
    /// Lower endpoint for range mode.
    from: Option<String>,
    /// Root endpoint for full mode.
    root: Option<String>,
    /// Upper endpoint for every mode.
    to: Option<String>,
}

impl MutantsPushOptions
{
    /// Convert parsed push options into the public push range plan.
    fn into_range_plan(self) -> Result<mutants::range::PushRangePlan, GateError>
    {
        if self.range_mode.is_none()
            && self.from.is_none()
            && self.root.is_none()
            && self.to.is_none()
        {
            return mutants::range::PushRangePlan::last(MUTANTS_DEFAULT_PUSH_TO_REF);
        }

        let range_mode = required_value(
            self.range_mode,
            "mutants push requires --range-mode range|full|last",
        )?;
        let to = required_value(self.to, "mutants push requires --to REF")?;
        match range_mode {
            | MutantsPushRangeMode::Range => {
                reject_present(
                    self.root.as_ref(),
                    "mutants push --range-mode range rejects --root",
                )?;
                let from =
                    required_value(self.from, "mutants push range mode requires --from REF")?;
                mutants::range::PushRangePlan::range(&from, &to)
            },
            | MutantsPushRangeMode::Full => {
                reject_present(
                    self.from.as_ref(),
                    "mutants push --range-mode full rejects --from",
                )?;
                let root = required_value(self.root, "mutants push full mode requires --root REF")?;
                mutants::range::PushRangePlan::full(&root, &to)
            },
            | MutantsPushRangeMode::Last => {
                reject_present(
                    self.from.as_ref(),
                    "mutants push --range-mode last rejects --from",
                )?;
                reject_present(
                    self.root.as_ref(),
                    "mutants push --range-mode last rejects --root",
                )?;
                mutants::range::PushRangePlan::last(&to)
            },
        }
    }
}

/// Optional `mutants scheduled` parser state.
#[derive(Default)]
struct MutantsScheduledOptions
{
    /// Lower ref for the scheduled range.
    from_ref: Option<String>,
    /// Upper ref for the scheduled range.
    to_ref: Option<String>,
}

impl MutantsScheduledOptions
{
    /// Convert scheduled parser state into required refs.
    fn into_required_refs(self) -> Result<(String, String), GateError>
    {
        let from_ref = required_value(self.from_ref, "mutants scheduled requires --from REF")?;
        let to_ref = required_value(self.to_ref, "mutants scheduled requires --to REF")?;
        Ok((from_ref, to_ref))
    }
}

/// Regression witnesses for typed CLI parsing, plans, and dispatch behavior.
#[cfg(test)]
mod tests
{
    use core::error::Error;
    use core::sync::atomic::AtomicU64;
    use core::sync::atomic::Ordering;

    use gandr_workflow_gates::semantic_value;

    use super::*;

    /// Shared result type for CLI driver unit witnesses.
    type TestResult<T = ()> = Result<T, Box<dyn Error>>;

    /// Per-process suffix keeping filesystem fixtures disjoint in parallel
    /// tests.
    static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

    /// Assert one generated mutants temporary path names the expected mode and
    /// role.
    fn assert_mutants_temp_path<'semantic, Mode, Suffix>(
        path: &Path,
        mode: Mode,
        suffix: Suffix,
    ) -> TestResult
    where
        Mode: Into<ModeText<'semantic>>,
        Suffix: Into<SuffixText<'semantic>>,
    {
        let suffix = suffix.into().0;
        let mode = mode.into().0;
        let Some(file_name) = path.file_name()
        else {
            return Err(Box::new(std::io::Error::other(
                "temporary mutants path has no file name",
            )));
        };
        let text = file_name.to_string_lossy();
        assert!(text.contains(MUTANTS_TEMPORARY_PATH_PREFIX));
        assert!(text.contains(mode));
        assert!(text.ends_with(suffix));
        Ok(())
    }

    /// Parser accepts every retained command shape and default.
    #[test]
    fn parser_accepts_every_command_variant_and_default() -> TestResult
    {
        let under_unit_test = running_under_binary_unit_test()?;
        if !under_unit_test.0 {
            return Ok(());
        }
        let cases = [
            ValidCommandCase {
                label: "contracts",
                args: &[
                    "gandr-workflow-gates",
                    "contracts",
                    "--scope",
                    "crates/a",
                    "--scope",
                    "crates/b",
                    "--nextest-list-fixture",
                    "nextest.json",
                ],
                expected: ExpectedCommand::Contracts,
            },
            ValidCommandCase {
                label: "ci-contracts",
                args: &[
                    "gandr-workflow-gates",
                    "ci-contracts",
                    "--workflow",
                    ".github/workflows/ci.yml",
                ],
                expected: ExpectedCommand::CiContracts,
            },
            ValidCommandCase {
                label: "graph-boundary",
                args: &[
                    "gandr-workflow-gates",
                    "graph-boundary",
                    "--workspace-root",
                    "workspace",
                    "--metadata-fixture",
                    "metadata.json",
                ],
                expected: ExpectedCommand::GraphBoundary,
            },
            ValidCommandCase {
                label: "docs-manifest-default",
                args: &["gandr-workflow-gates", "docs-manifest"],
                expected: ExpectedCommand::DocsManifestDefault,
            },
            ValidCommandCase {
                label: "docs-reference-explicit",
                args: &[
                    "gandr-workflow-gates",
                    "docs-reference",
                    "--manifest",
                    "docs/MANIFEST.yml",
                ],
                expected: ExpectedCommand::DocsReferenceExplicit,
            },
            ValidCommandCase {
                label: "page-balance",
                args: &["gandr-workflow-gates", "page-balance", "--cwd", "docs"],
                expected: ExpectedCommand::PageBalanceCwd,
            },
            ValidCommandCase {
                label: "rumdl-fmt",
                args: &["gandr-workflow-gates", "rumdl", "fmt", "a.md", "b.md"],
                expected: ExpectedCommand::RumdlFmt,
            },
            ValidCommandCase {
                label: "rumdl-check",
                args: &[
                    "gandr-workflow-gates",
                    "rumdl",
                    "check",
                    "--cwd",
                    "docs",
                    "README.md",
                ],
                expected: ExpectedCommand::RumdlCheck,
            },
            ValidCommandCase {
                label: "options-policy",
                args: &[
                    "gandr-workflow-gates",
                    "options-policy",
                    "--workspace-root",
                    "workspace",
                ],
                expected: ExpectedCommand::OptionsPolicy,
            },
            ValidCommandCase {
                label: "soundness-oracles",
                args: &[
                    "gandr-workflow-gates",
                    "soundness-oracles",
                    "--workspace-root",
                    "workspace",
                ],
                expected: ExpectedCommand::SoundnessOracles,
            },
            ValidCommandCase {
                label: "default-graph",
                args: &[
                    "gandr-workflow-gates",
                    "default-graph",
                    "--workspace-root",
                    "workspace",
                ],
                expected: ExpectedCommand::DefaultGraph,
            },
            ValidCommandCase {
                label: "iu-pin",
                args: &[
                    "gandr-workflow-gates",
                    "iu-pin",
                    "--workspace-root",
                    "workspace",
                ],
                expected: ExpectedCommand::IuPin,
            },
            ValidCommandCase {
                label: "agda-default",
                args: &["gandr-workflow-gates", "agda-deps"],
                expected: ExpectedCommand::AgdaDefault,
            },
            ValidCommandCase {
                label: "agda-explicit",
                args: &[
                    "gandr-workflow-gates",
                    "agda-deps",
                    "--workspace-root",
                    "workspace",
                ],
                expected: ExpectedCommand::AgdaExplicit,
            },
            ValidCommandCase {
                label: "coverage-check-default",
                args: &[
                    "gandr-workflow-gates",
                    "coverage",
                    "check",
                    "--repo-root",
                    "repo",
                ],
                expected: ExpectedCommand::CoverageCheckDefault,
            },
            ValidCommandCase {
                label: "coverage-ratchet-explicit",
                args: &[
                    "gandr-workflow-gates",
                    "coverage",
                    "ratchet",
                    "--repo-root",
                    "repo",
                    "--summary",
                    "summary.json",
                    "--floors",
                    "floors.toml",
                ],
                expected: ExpectedCommand::CoverageRatchetExplicit,
            },
            ValidCommandCase {
                label: "maintenance-resolve-explicit",
                args: &[
                    "gandr-workflow-gates",
                    "maintenance-range",
                    "--github-output",
                    "out.env",
                    "--head",
                    "feature",
                    "--from",
                    "main",
                    "--watermark",
                    "watermark.json",
                ],
                expected: ExpectedCommand::MaintenanceResolveExplicit,
            },
            ValidCommandCase {
                label: "maintenance-resolve-default-head",
                args: &[
                    "gandr-workflow-gates",
                    "maintenance-range",
                    "--github-output",
                    "out.env",
                ],
                expected: ExpectedCommand::MaintenanceResolveDefaultHead,
            },
            ValidCommandCase {
                label: "maintenance-advance-explicit",
                args: &[
                    "gandr-workflow-gates",
                    "maintenance-range",
                    "advance",
                    "--watermark",
                    "watermark.json",
                    "--to",
                    "feature",
                ],
                expected: ExpectedCommand::MaintenanceAdvanceExplicit,
            },
            ValidCommandCase {
                label: "maintenance-advance-default-to",
                args: &[
                    "gandr-workflow-gates",
                    "maintenance-range",
                    "advance",
                    "--watermark",
                    "watermark.json",
                ],
                expected: ExpectedCommand::MaintenanceAdvanceDefaultTo,
            },
            ValidCommandCase {
                label: "mutants-snapshot",
                args: &["gandr-workflow-gates", "mutants", "snapshot"],
                expected: ExpectedCommand::MutantsSnapshot,
            },
            ValidCommandCase {
                label: "mutants-merge-explicit",
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "merge",
                    "--workspace-root",
                    "repo",
                    "--cache-image",
                    "cache.btrfs",
                    "--source-archive",
                    "source.tar",
                    "--diff-file",
                    "diff.patch",
                    "--working-report",
                    "report",
                ],
                expected: ExpectedCommand::MutantsMergeExplicit,
            },
            ValidCommandCase {
                label: "mutants-push-default",
                args: &["gandr-workflow-gates", "mutants", "push"],
                expected: ExpectedCommand::MutantsPushDefault,
            },
            ValidCommandCase {
                label: "mutants-push-range",
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "push",
                    "--range-mode",
                    "range",
                    "--from",
                    "main",
                    "--to",
                    "HEAD",
                ],
                expected: ExpectedCommand::MutantsPushRange,
            },
            ValidCommandCase {
                label: "mutants-push-full",
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "push",
                    "--range-mode",
                    "full",
                    "--root",
                    "last-release",
                    "--to",
                    "HEAD",
                ],
                expected: ExpectedCommand::MutantsPushFull,
            },
            ValidCommandCase {
                label: "mutants-scheduled",
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "scheduled",
                    "--from",
                    "nightly-base",
                    "--to",
                    "HEAD",
                ],
                expected: ExpectedCommand::MutantsScheduled,
            },
            ValidCommandCase {
                label: "mutants-guest",
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "guest",
                    "--workspace-root",
                    "guest-workspace",
                    "--package",
                    "gandr-workflow-gates",
                    "--diff",
                    "changes.diff",
                ],
                expected: ExpectedCommand::MutantsGuest,
            },
            ValidCommandCase {
                label: "mutants-clean",
                args: &["gandr-workflow-gates", "mutants", "clean"],
                expected: ExpectedCommand::MutantsClean,
            },
            ValidCommandCase {
                label: "mutants-sweep",
                args: &["gandr-workflow-gates", "mutants", "sweep"],
                expected: ExpectedCommand::MutantsSweep,
            },
            ValidCommandCase {
                label: "workflow-push",
                args: &["gandr-workflow-gates", "workflow", "push", "--cwd", "repo"],
                expected: ExpectedCommand::WorkflowPush,
            },
            ValidCommandCase {
                label: "workflow-merge",
                args: &["gandr-workflow-gates", "workflow", "merge"],
                expected: ExpectedCommand::WorkflowMerge,
            },
            ValidCommandCase {
                label: "fuzz-smoke-default",
                args: &["gandr-workflow-gates", "fuzz-smoke"],
                expected: ExpectedCommand::FuzzDefault,
            },
        ];

        for case in cases {
            assert_valid_command(case)?;
        }
        Ok(())
    }

    /// Parse and assert one accepted CLI row.
    fn assert_valid_command(case: ValidCommandCase) -> TestResult
    {
        let command = parse_command(os_args(case.args))?;
        match (case.expected, command) {
            | (
                ExpectedCommand::Contracts,
                Command::Contracts {
                    scopes,
                    nextest_list_fixture,
                },
            ) => {
                assert_eq!(
                    vec![PathBuf::from("crates/a"), PathBuf::from("crates/b")],
                    scopes
                );
                assert_eq!(Some(PathBuf::from("nextest.json")), nextest_list_fixture);
            },
            | (ExpectedCommand::CiContracts, Command::CiContracts { workflow }) => {
                assert_eq!(workflow, PathBuf::from(".github/workflows/ci.yml"));
            },
            | (
                ExpectedCommand::GraphBoundary,
                Command::GraphBoundary {
                    workspace_root,
                    metadata_fixture,
                },
            ) => {
                assert_eq!(workspace_root, PathBuf::from("workspace"));
                assert_eq!(Some(PathBuf::from("metadata.json")), metadata_fixture);
            },
            | (ExpectedCommand::DocsManifestDefault, Command::DocsManifest { manifest_path }) => {
                assert_eq!(manifest_path, default_manifest_path());
            },
            | (
                ExpectedCommand::DocsReferenceExplicit,
                Command::DocsReference { manifest_path },
            ) => {
                assert_eq!(manifest_path, PathBuf::from("docs/MANIFEST.yml"));
            },
            | (ExpectedCommand::PageBalanceCwd, Command::PageBalance { cwd }) => {
                assert_eq!(Some(PathBuf::from("docs")), cwd);
            },
            | (ExpectedCommand::RumdlFmt, Command::Rumdl { mode, paths, cwd }) => {
                assert_eq!(
                    "fmt",
                    semantic_value::<docs::commands::AsStrText<'_>, _>(mode.as_str()).as_ref()
                );
                assert_eq!(vec![PathBuf::from("a.md"), PathBuf::from("b.md")], paths);
                assert_eq!(None, cwd);
            },
            | (ExpectedCommand::RumdlCheck, Command::Rumdl { mode, paths, cwd }) => {
                assert_eq!(
                    "check",
                    semantic_value::<docs::commands::AsStrText<'_>, _>(mode.as_str()).as_ref()
                );
                assert_eq!(vec![PathBuf::from("README.md")], paths);
                assert_eq!(Some(PathBuf::from("docs")), cwd);
            },
            | (ExpectedCommand::OptionsPolicy, Command::OptionsPolicy { workspace_root })
            | (ExpectedCommand::SoundnessOracles, Command::SoundnessOracles { workspace_root })
            | (ExpectedCommand::DefaultGraph, Command::DefaultGraph { workspace_root })
            | (ExpectedCommand::IuPin, Command::IuPin { workspace_root }) => {
                assert_eq!(workspace_root, PathBuf::from("workspace"));
            },
            | (ExpectedCommand::AgdaDefault, Command::AgdaDeps { workspace_root }) => {
                assert_eq!(None, workspace_root);
            },
            | (ExpectedCommand::AgdaExplicit, Command::AgdaDeps { workspace_root }) => {
                assert_eq!(Some(PathBuf::from("workspace")), workspace_root);
            },
            | (
                ExpectedCommand::CoverageCheckDefault,
                Command::Coverage {
                    mode,
                    summary_path,
                    floors_path,
                    repo_root,
                },
            ) => {
                assert_eq!(CoverageCommand::Check, mode);
                assert_eq!(
                    summary_path,
                    Path::new("repo").join(gandr_workflow_gates::coverage::DEFAULT_SUMMARY)
                );
                assert_eq!(
                    floors_path,
                    Path::new("repo").join(gandr_workflow_gates::coverage::DEFAULT_FLOORS)
                );
                assert_eq!(repo_root, PathBuf::from("repo"));
            },
            | (
                ExpectedCommand::CoverageRatchetExplicit,
                Command::Coverage {
                    mode,
                    summary_path,
                    floors_path,
                    repo_root,
                },
            ) => {
                assert_eq!(CoverageCommand::Ratchet, mode);
                assert_eq!(summary_path, PathBuf::from("summary.json"));
                assert_eq!(floors_path, PathBuf::from("floors.toml"));
                assert_eq!(repo_root, PathBuf::from("repo"));
            },
            | (
                ExpectedCommand::MaintenanceResolveExplicit,
                Command::MaintenanceRange {
                    github_output,
                    head,
                    explicit_from,
                    watermark,
                },
            ) => {
                assert_eq!(github_output, PathBuf::from("out.env"));
                assert_eq!(
                    "feature",
                    semantic_value::<maintenance::AsStrText<'_>, _>(head.as_str()).as_ref()
                );
                assert!(explicit_from.as_ref().is_some_and(|value| {
                    semantic_value::<maintenance::AsStrText<'_>, _>(value.as_str()).as_ref()
                        == "main"
                }));
                assert_eq!(Some(PathBuf::from("watermark.json")), watermark);
            },
            | (
                ExpectedCommand::MaintenanceResolveDefaultHead,
                Command::MaintenanceRange {
                    github_output,
                    head,
                    explicit_from,
                    watermark,
                },
            ) => {
                assert_eq!(github_output, PathBuf::from("out.env"));
                assert_eq!(
                    "HEAD",
                    semantic_value::<maintenance::AsStrText<'_>, _>(head.as_str()).as_ref()
                );
                assert_eq!(None, explicit_from);
                assert_eq!(None, watermark);
            },
            | (
                ExpectedCommand::MaintenanceAdvanceExplicit,
                Command::MaintenanceAdvance { watermark, to },
            ) => {
                assert_eq!(watermark, PathBuf::from("watermark.json"));
                assert_eq!(
                    "feature",
                    semantic_value::<maintenance::AsStrText<'_>, _>(to.as_str()).as_ref()
                );
            },
            | (
                ExpectedCommand::MaintenanceAdvanceDefaultTo,
                Command::MaintenanceAdvance { watermark, to },
            ) => {
                assert_eq!(watermark, PathBuf::from("watermark.json"));
                assert_eq!(
                    "HEAD",
                    semantic_value::<maintenance::AsStrText<'_>, _>(to.as_str()).as_ref()
                );
            },
            | (
                ExpectedCommand::MutantsSnapshot,
                Command::Mutants {
                    command: mutants::MutantsCommand::Snapshot,
                    options,
                },
            ) => {
                assert_default_mutants_options(&options, "snapshot")?;
            },
            | (
                ExpectedCommand::MutantsMergeExplicit,
                Command::Mutants {
                    command: mutants::MutantsCommand::Merge,
                    options,
                },
            ) => {
                assert_eq!(options.workspace_root, PathBuf::from("repo"));
                assert_eq!(options.cache_image, PathBuf::from("cache.btrfs"));
                assert_eq!(options.source_archive, PathBuf::from("source.tar"));
                assert_eq!(options.diff_file, PathBuf::from("diff.patch"));
                assert_eq!(options.working_report, PathBuf::from("report"));
            },
            | (
                ExpectedCommand::MutantsPushDefault,
                Command::Mutants {
                    command:
                        mutants::MutantsCommand::Push {
                            range: mutants::range::PushRangePlan::Last { to },
                        },
                    options,
                },
            ) => {
                assert_eq!(MUTANTS_DEFAULT_PUSH_TO_REF, to);
                assert_default_mutants_options(&options, "push")?;
            },
            | (
                ExpectedCommand::MutantsPushRange,
                Command::Mutants {
                    command:
                        mutants::MutantsCommand::Push {
                            range: mutants::range::PushRangePlan::Range { from, to },
                        },
                    options,
                },
            ) => {
                assert_eq!("main", from);
                assert_eq!("HEAD", to);
                assert_default_mutants_options(&options, "push")?;
            },
            | (
                ExpectedCommand::MutantsPushFull,
                Command::Mutants {
                    command:
                        mutants::MutantsCommand::Push {
                            range: mutants::range::PushRangePlan::Full { root, to },
                        },
                    options,
                },
            ) => {
                assert_eq!("last-release", root);
                assert_eq!("HEAD", to);
                assert_default_mutants_options(&options, "push")?;
            },
            | (
                ExpectedCommand::MutantsScheduled,
                Command::Mutants {
                    command: mutants::MutantsCommand::Scheduled { from_ref, to_ref },
                    options,
                },
            ) => {
                assert_eq!("nightly-base", from_ref);
                assert_eq!("HEAD", to_ref);
                assert_default_mutants_options(&options, "scheduled")?;
            },
            | (
                ExpectedCommand::MutantsGuest,
                Command::Mutants {
                    command: mutants::MutantsCommand::Guest { package, diff },
                    options,
                },
            ) => {
                assert_eq!(Some("gandr-workflow-gates"), package.as_deref());
                assert_eq!(Some(PathBuf::from("changes.diff")), diff);
                assert_eq!(options.workspace_root, PathBuf::from("guest-workspace"));
            },
            | (
                ExpectedCommand::MutantsClean,
                Command::Mutants {
                    command: mutants::MutantsCommand::Clean,
                    options,
                },
            ) => {
                assert!(options.workspace_root.as_os_str().is_empty());
                assert!(options.cache_image.as_os_str().is_empty());
                assert!(options.source_archive.as_os_str().is_empty());
                assert!(options.diff_file.as_os_str().is_empty());
                assert!(options.working_report.as_os_str().is_empty());
            },
            | (
                ExpectedCommand::MutantsSweep,
                Command::Mutants {
                    command: mutants::MutantsCommand::Sweep,
                    options,
                },
            ) => {
                assert_default_mutants_options(&options, "sweep")?;
            },
            | (ExpectedCommand::WorkflowPush, Command::Workflow { tier, cwd }) => {
                assert_eq!("push", tier.as_str().as_ref());
                assert_eq!(Some(PathBuf::from("repo")), cwd);
                assert_eq!("push", tier.plan().tier().as_str().as_ref());
            },
            | (ExpectedCommand::WorkflowMerge, Command::Workflow { tier, cwd }) => {
                assert_eq!("merge", tier.as_str().as_ref());
                assert_eq!(None, cwd);
            },
            | (ExpectedCommand::FuzzDefault, Command::FuzzSmoke { plan }) => {
                assert_eq!(
                    vec!["lower", "parse", "check", "parity", "gates"],
                    plan.targets()
                        .iter()
                        .map(|target| target.as_str().0)
                        .collect::<Vec<_>>()
                );
            },
            | _ => return Err(unexpected(case.label)),
        }
        Ok(())
    }

    /// Assert that generated mutants defaults point at cwd, cache, and owned
    /// temp paths.
    fn assert_default_mutants_options<'semantic, Mode>(
        options: &mutants::MutantsOptions,
        mode: Mode,
    ) -> TestResult
    where
        Mode: Into<ModeText<'semantic>>,
    {
        let mode = mode.into().0;
        let current_dir = gandr_workflow_gates::support::HOST_FILESYSTEM.current_dir()?;
        assert_eq!(options.workspace_root, current_dir);
        assert!(
            options
                .cache_image
                .ends_with(PathBuf::from(".microsandbox/gandr-mutants-cache.btrfs"))
        );
        assert_mutants_temp_path(&options.source_archive, mode, "source.tar")?;
        assert_mutants_temp_path(&options.diff_file, mode, "diff.patch")?;
        assert_mutants_temp_path(&options.working_report, mode, "report")?;
        Ok(())
    }

    /// Parser reports exact usage details for malformed command lines.
    #[test]
    fn parser_rejects_malformed_options_modes_and_precedence() -> TestResult
    {
        let under_unit_test = running_under_binary_unit_test()?;
        if !under_unit_test.0 {
            return Ok(());
        }
        let cases = [
            UsageCase {
                args: &["gandr-workflow-gates"],
                detail: usage_text().into().0,
            },
            UsageCase {
                args: &["gandr-workflow-gates", "unknown"],
                detail: "unknown command `unknown`",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "contracts"],
                detail: "contracts requires at least one --scope PATH",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "contracts", "--scope"],
                detail: "missing value for --scope",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "contracts",
                    "--scope",
                    "--nextest-list-fixture",
                ],
                detail: "missing value for --scope",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "contracts",
                    "--scope",
                    "crates/a",
                    "--nextest-list-fixture",
                    "one.json",
                    "--nextest-list-fixture",
                    "two.json",
                ],
                detail: "duplicate --nextest-list-fixture",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "ci-contracts"],
                detail: "ci-contracts requires --workflow PATH",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "graph-boundary",
                    "--workspace-root",
                    "one",
                    "--workspace-root",
                    "two",
                ],
                detail: "duplicate --workspace-root",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "docs-reference", "--bogus"],
                detail: "unknown argument `--bogus`",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "docs-manifest",
                    "--manifest",
                    "one.yml",
                    "--manifest",
                    "two.yml",
                ],
                detail: "duplicate --manifest",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "page-balance",
                    "--cwd",
                    "one",
                    "--cwd",
                    "two",
                ],
                detail: "duplicate --cwd",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "rumdl"],
                detail: "rumdl requires fmt or check",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "rumdl", "lint"],
                detail: "unsupported rumdl mode: lint",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "rumdl", "check", "--unknown"],
                detail: "unknown argument `--unknown`",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "options-policy"],
                detail: "options-policy requires --workspace-root PATH",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "coverage"],
                detail: "coverage requires check or ratchet",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "coverage", "publish"],
                detail: "unsupported coverage command `publish`",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "coverage", "check"],
                detail: "coverage requires --repo-root PATH",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "maintenance-range"],
                detail: "maintenance-range requires --github-output PATH",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "maintenance-range",
                    "--github-output",
                    "out",
                    "--head",
                    "one",
                    "--head",
                    "two",
                ],
                detail: "duplicate --head",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "maintenance-range", "advance"],
                detail: "maintenance-range advance requires --watermark PATH",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "mutants"],
                detail: "mutants requires a mode",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "mutants", "arbitrary"],
                detail: "unsupported mutants mode `arbitrary`",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "snapshot",
                    "--from",
                    "main",
                ],
                detail: "unknown argument `--from`",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "push",
                    "--range-mode",
                    "middle",
                ],
                detail: "unsupported mutants push range mode `middle`",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "push",
                    "--range-mode",
                    "range",
                    "--to",
                    "HEAD",
                ],
                detail: "mutants push range mode requires --from REF",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "push",
                    "--range-mode",
                    "full",
                    "--from",
                    "main",
                    "--root",
                    "base",
                    "--to",
                    "HEAD",
                ],
                detail: "mutants push --range-mode full rejects --from",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "push",
                    "--range-mode",
                    "last",
                    "--root",
                    "base",
                    "--to",
                    "HEAD",
                ],
                detail: "mutants push --range-mode last rejects --root",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "mutants",
                    "scheduled",
                    "--from",
                    "main",
                ],
                detail: "mutants scheduled requires --to REF",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "workflow"],
                detail: "workflow requires merge or push",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "workflow", "release"],
                detail: "unsupported workflow mode `release`",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "fuzz-smoke",
                    "--target",
                    "lower",
                    "--target",
                    "parse",
                ],
                detail: "duplicate --target",
            },
            UsageCase {
                args: &["gandr-workflow-gates", "fuzz-smoke", "--target", "vm"],
                detail: "unsupported fuzz-smoke target `vm`",
            },
            UsageCase {
                args: &[
                    "gandr-workflow-gates",
                    "agda-deps",
                    "--workspace-root",
                    "one",
                    "--workspace-root",
                    "two",
                ],
                detail: "duplicate --workspace-root",
            },
        ];

        for case in cases {
            assert_usage(parse_command(os_args(case.args)), case.detail)?;
        }
        Ok(())
    }

    /// OS-string parser handling rejects textual non-UTF-8 while preserving
    /// paths.
    #[cfg(unix)]
    #[test]
    fn non_utf8_command_text_and_path_arguments_are_classified() -> TestResult
    {
        let under_unit_test = running_under_binary_unit_test()?;
        if !under_unit_test.0 {
            return Ok(());
        }
        assert_usage(
            parse_command([
                OsString::from("gandr-workflow-gates"),
                <OsString as std::os::unix::ffi::OsStringExt>::from_vec(vec![0xFF]),
            ]),
            "command must be valid UTF-8",
        )?;

        let invalid_mode = <OsString as std::os::unix::ffi::OsStringExt>::from_vec(vec![0xFF]);
        assert_usage(
            parse_command([
                OsString::from("gandr-workflow-gates"),
                OsString::from("workflow"),
                invalid_mode.clone(),
            ]),
            &format!(
                "value for workflow mode must be valid UTF-8: `{}`",
                invalid_mode.to_string_lossy()
            ),
        )?;

        let path_value = <OsString as std::os::unix::ffi::OsStringExt>::from_vec(vec![
            b'c', b'r', b'a', b't', b'e', 0xFF,
        ]);
        let parsed = parse_command([
            OsString::from("gandr-workflow-gates"),
            OsString::from("contracts"),
            OsString::from("--scope"),
            path_value.clone(),
        ])?;
        match parsed {
            | Command::Contracts { scopes, .. } => {
                assert_eq!(1_usize, scopes.len());
                let scope = scopes
                    .first()
                    .ok_or_else(|| unexpected("missing non-utf8 contracts scope"))?;
                assert_eq!(
                    scope.as_os_str().as_encoded_bytes(),
                    path_value.as_os_str().as_encoded_bytes()
                );
            },
            | _ => return Err(unexpected("non-utf8 contracts scope")),
        }
        Ok(())
    }

    /// Assert that a parser result fails with the exact usage detail.
    fn assert_usage<'semantic, T, Expected>(
        result: Result<T, GateError>,
        expected: Expected,
    ) -> TestResult
    where
        Expected: Into<ExpectedText<'semantic>>,
    {
        let expected = expected.into().0;
        match result {
            | Err(GateError::Usage { detail }) => {
                assert_eq!(detail, expected);
                Ok(())
            },
            | Err(error) => Err(Box::new(error)),
            | Ok(_) => Err(Box::new(std::io::Error::other(
                "command unexpectedly parsed successfully",
            ))),
        }
    }

    /// Convert borrowed UTF-8 arguments into owned OS arguments.
    fn os_args<Args>(values: Args) -> Vec<OsString>
    where
        Args: IntoIterator,
        Args::Item: Into<OsString>,
    {
        values.into_iter().map(Into::into).collect()
    }

    /// Fuzz target parsing accepts every closed spelling and preserves order.
    #[test]
    fn fuzz_target_parser_and_plan_projection_are_exact() -> TestResult
    {
        let under_unit_test = running_under_binary_unit_test()?;
        if !under_unit_test.0 {
            return Ok(());
        }
        let cases = [
            ("lower", FuzzSmokeTarget::Lower, None),
            ("parse", FuzzSmokeTarget::Parse, None),
            ("check", FuzzSmokeTarget::Check, None),
            ("parity", FuzzSmokeTarget::Parity, Some(FUZZ_PARITY_FEATURE)),
            ("gates", FuzzSmokeTarget::Gates, Some(FUZZ_GATES_FEATURE)),
        ];

        for (name, target, feature) in cases {
            let parsed_target = parse_fuzz_smoke_target(name)?;
            assert_eq!(parsed_target, target);
            assert_eq!(target.as_str().as_ref(), name);
            assert_eq!(target.required_feature().map(|value| value.0), feature);

            let parsed = parse_command(os_args([
                "gandr-workflow-gates",
                "fuzz-smoke",
                "--target",
                name,
            ]))?;
            match parsed {
                | Command::FuzzSmoke { plan } => assert_eq!(plan.targets(), &[target]),
                | _ => return Err(unexpected("fuzz-smoke target")),
            }
        }

        assert_eq!(
            fuzz_build_args(FuzzSmokeTarget::Lower),
            os_strings([
                "afl",
                "build",
                "--manifest-path",
                FUZZ_MANIFEST_PATH,
                "--bin",
                "lower",
            ])
        );
        assert_eq!(
            fuzz_build_args(FuzzSmokeTarget::Parity),
            os_strings([
                "afl",
                "build",
                "--manifest-path",
                FUZZ_MANIFEST_PATH,
                "--bin",
                "parity",
                "--features",
                FUZZ_PARITY_FEATURE,
            ])
        );
        assert_eq!(
            fuzz_build_args(FuzzSmokeTarget::Gates),
            os_strings([
                "afl",
                "build",
                "--manifest-path",
                FUZZ_MANIFEST_PATH,
                "--bin",
                "gates",
                "--features",
                FUZZ_GATES_FEATURE,
            ])
        );

        for target in FUZZ_TARGETS {
            let build = fuzz_build_command_plan(target);
            assert_eq!(build.program(), Path::new(CARGO_PROGRAM));
            assert_eq!(ExternalStream::Inherit, build.stdin());
            assert_eq!(ExternalStream::Inherit, build.stdout());
            assert_eq!(ExternalStream::Inherit, build.stderr());

            let replay = fuzz_replay_command_plan(target);
            assert_eq!(replay.program(), fuzz_binary_path(target));
            assert!(replay.args().is_empty());
            assert_eq!(ExternalStream::Piped, replay.stdin());
            assert_eq!(ExternalStream::Inherit, replay.stdout());
            assert_eq!(ExternalStream::Inherit, replay.stderr());
        }
        Ok(())
    }

    /// Agda command planning and ready checkout short-circuit avoid ambient
    /// Git.
    #[test]
    fn agda_plans_and_ready_checkout_are_exact_without_git() -> TestResult
    {
        let under_unit_test = running_under_binary_unit_test()?;
        if !under_unit_test.0 {
            return Ok(());
        }
        let root = TempRoot::create("agda-ready")?;
        let plan = AgdaDependencyPlan::new(root.path().to_path_buf());
        assert_eq!(plan.workspace_root(), root.path());
        let explicit_root = agda_workspace_root(Some(root.path()))?;
        assert_eq!(explicit_root, root.path());
        let default_root = agda_workspace_root(None)?;
        let current_dir = gandr_workflow_gates::support::HOST_FILESYSTEM.current_dir()?;
        assert_eq!(default_root, current_dir);
        assert_eq!(agda_vendor_dir(&plan), root.path().join(AGDA_VENDOR_DIR));
        assert_eq!(agda_stdlib_dir(&plan), root.path().join(AGDA_STDLIB_DIR));
        assert_eq!(
            agda_stdlib_lib_path(&plan),
            root.path().join(AGDA_STDLIB_LIB)
        );
        assert_eq!(
            agda_libraries_file(&plan),
            root.path().join(AGDA_LIBRARIES_FILE)
        );

        let clone = agda_clone_command_plan(&plan);
        assert_eq!(clone.cwd(), root.path());
        assert!(clone.sanitized_git().0);
        assert_eq!(
            clone.args(),
            os_strings([
                "clone",
                "--depth",
                "1",
                "--branch",
                AGDA_STDLIB_BRANCH,
                AGDA_STDLIB_REPOSITORY,
                AGDA_STDLIB_DIR,
            ])
            .as_slice()
        );

        let fetch = agda_fetch_command_plan(&plan);
        assert_eq!(fetch.cwd(), root.path());
        assert!(fetch.sanitized_git().0);
        assert_eq!(
            fetch.args(),
            os_strings([
                "-C",
                AGDA_STDLIB_DIR,
                "fetch",
                "--depth",
                "1",
                "origin",
                AGDA_STDLIB_BRANCH,
            ])
            .as_slice()
        );

        let checkout = agda_checkout_command_plan(&plan);
        assert_eq!(checkout.cwd(), root.path());
        assert!(checkout.sanitized_git().0);
        assert_eq!(
            checkout.args(),
            os_strings(["-C", AGDA_STDLIB_DIR, "checkout", "--detach", "FETCH_HEAD"]).as_slice()
        );

        let stdlib_lib = agda_stdlib_lib_path(&plan);
        let Some(parent) = stdlib_lib.parent()
        else {
            return Err(Box::new(std::io::Error::other(
                "stdlib lib path has no parent",
            )));
        };
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(parent)?;
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .write(&stdlib_lib, "name: standard-library\n")?;
        run_agda_deps_plan(&plan)?;
        let libraries = gandr_workflow_gates::support::HOST_FILESYSTEM
            .read_to_string(agda_libraries_file(&plan))?;
        let canonical_lib =
            gandr_workflow_gates::support::HOST_FILESYSTEM.canonicalize(stdlib_lib)?;
        assert_eq!(libraries, format!("{}\n", canonical_lib.to_string_lossy()));
        Ok(())
    }

    /// Return whether this copy runs inside the binary unit-test executable.
    fn running_under_binary_unit_test() -> TestResult<RunningUnderBinaryUnitTestFlag>
    {
        let executable = gandr_workflow_gates::support::HOST_FILESYSTEM.current_exe()?;
        let Some(name) = executable.file_name().and_then(OsStr::to_str)
        else {
            return Ok(RunningUnderBinaryUnitTestFlag(false));
        };
        Ok(RunningUnderBinaryUnitTestFlag(
            name.starts_with("gandr_workflow_gates-"),
        ))
    }

    /// Process-plan helpers preserve status text and stream policy.
    #[test]
    fn process_status_and_stream_plans_are_observable_without_afl() -> TestResult
    {
        let under_unit_test = running_under_binary_unit_test()?;
        if !under_unit_test.0 {
            return Ok(());
        }
        #[cfg(unix)]
        {
            let exited =
                <std::process::ExitStatus as std::os::unix::process::ExitStatusExt>::from_raw(
                    7_i32 << 8,
                );
            assert_eq!("exit status 7", status_detail(exited));
            let _exit_code = exit_code_from_status(exited);

            let signaled =
                <std::process::ExitStatus as std::os::unix::process::ExitStatusExt>::from_raw(
                    9_i32,
                );
            assert_eq!("termination without exit code", status_detail(signaled));
            let _signal_code = exit_code_from_status(signaled);
        }

        let executable = gandr_workflow_gates::support::HOST_FILESYSTEM.current_exe()?;
        let status_plan = FuzzExternalCommandPlan::new(
            executable.clone(),
            os_strings(["--list"]),
            ExternalStream::Inherit,
            ExternalStream::Inherit,
            ExternalStream::Inherit,
        );
        let status = run_streaming_status(&status_plan)?;
        assert!(status.success());

        let spawn_plan = FuzzExternalCommandPlan::new(
            executable,
            os_strings(["--list"]),
            ExternalStream::Inherit,
            ExternalStream::Piped,
            ExternalStream::Piped,
        );
        let child = spawn_streaming_process(&spawn_plan)?;
        let output = child.wait_with_output().map_err(|source| GateError::Io {
            path: spawn_plan.program().to_path_buf(),
            source,
        })?;
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
        Ok(())
    }

    /// Outcome classification covers clean, findings, and external status.
    #[test]
    fn outcomes_classify_payloads_before_process_exit() -> TestResult
    {
        let under_unit_test = running_under_binary_unit_test()?;
        if !under_unit_test.0 {
            return Ok(());
        }
        match GateOutcome::from_findings(Vec::new()) {
            | GateOutcome::Clean => {},
            | _ => return Err(unexpected("empty findings")),
        }

        let finding = Finding::new("kind", "pkg", "path.rs", "item", "detail");
        match GateOutcome::from_findings(vec![finding]) {
            | GateOutcome::Findings(findings) => assert_eq!(1_usize, findings.len()),
            | _ => return Err(unexpected("nonempty findings")),
        }

        let _clean_code = GateOutcome::Clean.into_exit_code();
        #[cfg(unix)]
        {
            let _external_code = GateOutcome::ExternalStatus(
                <std::process::ExitStatus as std::os::unix::process::ExitStatusExt>::from_raw(
                    3_i32 << 8,
                ),
            )
            .into_exit_code();
        }
        Ok(())
    }

    /// Return a stable boxed error for an unexpected command variant.
    fn unexpected<'semantic, Label>(label: Label) -> Box<dyn Error>
    where
        Label: Into<LabelText<'semantic>>,
    {
        let label = label.into().0;
        Box::new(std::io::Error::other(format!(
            "`{label}` parsed as the wrong command variant"
        )))
    }

    /// Convert borrowed UTF-8 arguments into owned OS strings.
    fn os_strings<Args>(values: Args) -> Vec<OsString>
    where
        Args: IntoIterator,
        Args::Item: Into<OsString>,
    {
        values.into_iter().map(Into::into).collect()
    }

    /// Temporary directory removed when the test exits.
    #[repr(transparent)]
    struct TempRoot
    {
        /// Root path.
        path: PathBuf,
    }

    impl TempRoot
    {
        /// Create one unique temporary root for a test.
        fn create<'semantic, Name>(name: Name) -> TestResult<Self>
        where
            Name: Into<NameText<'semantic>>,
        {
            let name = name.into().0;
            let suffix = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
            let temp_root = std::env::temp_dir().join(format!(
                "gandr-workflow-gates-main-{name}-{}-{suffix}",
                std::process::id()
            ));
            drop(gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(&temp_root));
            gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&temp_root)?;
            Ok(Self { path: temp_root })
        }

        /// Borrow the root path.
        fn path(&self) -> &Path
        {
            &self.path
        }
    }

    impl Drop for TempRoot
    {
        /// Remove the temporary root best-effort.
        fn drop(&mut self)
        {
            drop(gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(&self.path));
        }
    }

    /// Expected command shape for one parser acceptance row.
    #[derive(Clone, Copy)]
    enum ExpectedCommand
    {
        /// `contracts`.
        Contracts,
        /// `ci-contracts`.
        CiContracts,
        /// `graph-boundary`.
        GraphBoundary,
        /// `docs-manifest` with the default manifest path.
        DocsManifestDefault,
        /// `docs-reference` with an explicit manifest path.
        DocsReferenceExplicit,
        /// `page-balance` with a cwd.
        PageBalanceCwd,
        /// `rumdl fmt`.
        RumdlFmt,
        /// `rumdl check`.
        RumdlCheck,
        /// `options-policy`.
        OptionsPolicy,
        /// `soundness-oracles`.
        SoundnessOracles,
        /// `default-graph`.
        DefaultGraph,
        /// `iu-pin`.
        IuPin,
        /// `agda-deps` without a workspace root.
        AgdaDefault,
        /// `agda-deps` with a workspace root.
        AgdaExplicit,
        /// `coverage check` with default policy paths.
        CoverageCheckDefault,
        /// `coverage ratchet` with explicit policy paths.
        CoverageRatchetExplicit,
        /// `maintenance-range` with every resolve option.
        MaintenanceResolveExplicit,
        /// `maintenance-range` with default `HEAD`.
        MaintenanceResolveDefaultHead,
        /// `maintenance-range advance` with explicit `--to`.
        MaintenanceAdvanceExplicit,
        /// `maintenance-range advance` with default `HEAD`.
        MaintenanceAdvanceDefaultTo,
        /// `mutants snapshot`.
        MutantsSnapshot,
        /// `mutants merge` with explicit common paths.
        MutantsMergeExplicit,
        /// `mutants push` default range.
        MutantsPushDefault,
        /// `mutants push --range-mode range`.
        MutantsPushRange,
        /// `mutants push --range-mode full`.
        MutantsPushFull,
        /// `mutants scheduled`.
        MutantsScheduled,
        /// `mutants guest`.
        MutantsGuest,
        /// `mutants clean`.
        MutantsClean,
        /// `mutants sweep`.
        MutantsSweep,
        /// `workflow push`.
        WorkflowPush,
        /// `workflow merge`.
        WorkflowMerge,
        /// `fuzz-smoke` default target inventory.
        FuzzDefault,
    }

    /// One parser acceptance row.
    #[derive(Clone, Copy)]
    struct ValidCommandCase
    {
        /// Diagnostic label for assertion failures.
        label: &'static str,
        /// Full argv including executable name.
        args: &'static [&'static str],
        /// Expected typed command.
        expected: ExpectedCommand,
    }

    /// One parser rejection row.
    #[derive(Clone, Copy)]
    struct UsageCase
    {
        /// Full argv including executable name.
        args: &'static [&'static str],
        /// Exact usage detail.
        detail: &'static str,
    }
}
