//! Sequential, fail-closed mutation campaign support.
//!
//! Range selection, snapshot provisioning, containment proof, `microVM`
//! execution, and report publication are separate typed stages so no host
//! mutation path can bypass the containment boundary.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

mod containment;
pub mod range;
mod report;
mod sandbox;

use crate::GateError;
use crate::support;

crate::semantic_copy!(pub struct SuccessFlag(bool));
crate::semantic_copy!(pub struct CodeExitCode(Option<i32>));
crate::semantic_copy!(pub struct SanitizedGitFlag(bool));
crate::semantic_bytes!(pub struct BytesBytes);
crate::semantic_str!(pub struct ArchiveRefText);
crate::semantic_optional_str!(pub struct OptionalPackageText);
crate::semantic_str!(pub struct FromRefText);
crate::semantic_str!(pub struct ToRefText);
crate::semantic_str!(pub struct DiffSourceText);
crate::semantic_str!(pub struct ReportLabelText);
crate::semantic_str!(pub struct ContextText);
crate::semantic_copy!(pub struct MsbAvailableFlag(bool));
crate::semantic_copy!(pub struct SnapshotExistsFlag(bool));
crate::semantic_copy!(pub struct CacheImageExistsFlag(bool));
crate::semantic_str!(pub struct StdoutText);
crate::semantic_str!(pub struct ValueText);
crate::semantic_str!(pub struct TestNameText);
crate::semantic_str!(pub struct NameText);
crate::semantic_optional_copy!(pub struct OptionalCodeCode(i32));
crate::semantic_copy!(pub struct PathExistsFlag(bool));
crate::semantic_copy!(pub struct GitStatusSuccessFlag(bool));
crate::semantic_copy!(pub struct GitStatusSuccessWithHostFlag(bool));

/// Host command result retained across real support commands and test doubles.
#[derive(Clone, Debug, Eq, PartialEq)]
struct HostCommandOutcome
{
    /// Whether the command completed successfully.
    success: bool,
    /// Platform exit code when the command terminated normally.
    code: Option<i32>,
    /// Captured standard output retained for semantic parsers.
    stdout: String,
}

impl HostCommandOutcome
{
    /// Build an outcome from its observable fields.
    #[inline]
    #[must_use]
    fn new<S, C>(
        success: S,
        code: C,
        stdout: String,
    ) -> Self
    where
        S: Into<SuccessFlag>,
        C: Into<CodeExitCode>,
    {
        let code = code.into().0;
        let success = success.into().0;
        Self {
            success,
            code,
            stdout,
        }
    }

    /// Return whether the command succeeded.
    #[inline]
    #[must_use]
    fn success(&self) -> impl Into<SuccessFlag>
    {
        self.success
    }

    /// Return the command exit code.
    #[inline]
    #[must_use]
    fn code(&self) -> impl Into<OptionalCodeCode>
    {
        self.code
    }

    /// Return retained standard output.
    #[inline]
    #[must_use]
    fn stdout(&self) -> impl Into<StdoutText<'_>>
    {
        &self.stdout
    }
}

/// Host side effects used by mutation orchestration.
trait MutantsHost
{
    /// Run a host Git command and retain standard output.
    ///
    /// # Errors
    /// Returns the support or injected command failure.
    fn run_git_output<G>(
        &mut self,
        args: &[OsString],
        cwd: Option<&Path>,
        sanitized_git: G,
    ) -> Result<HostCommandOutcome, GateError>
    where
        G: Into<SanitizedGitFlag>;

    /// Run a host Git command and retain only the status.
    ///
    /// # Errors
    /// Returns the support or injected command failure.
    fn run_git_status<G>(
        &mut self,
        args: &[OsString],
        cwd: Option<&Path>,
        sanitized_git: G,
    ) -> Result<HostCommandOutcome, GateError>
    where
        G: Into<SanitizedGitFlag>;

    /// Run a non-Git host command and retain only the status.
    ///
    /// # Errors
    /// Returns the support or injected command failure.
    fn run_host_status(
        &mut self,
        program: &OsStr,
        args: &[OsString],
        cwd: Option<&Path>,
    ) -> Result<HostCommandOutcome, GateError>;

    /// Return whether a path exists.
    ///
    /// # Errors
    /// Returns filesystem probe failures.
    fn path_exists(
        &mut self,
        path: &Path,
    ) -> Result<impl Into<PathExistsFlag>, GateError>;

    /// Create a directory tree.
    ///
    /// # Errors
    /// Returns filesystem creation failures.
    fn create_dir_all(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>;

    /// Remove a directory tree.
    ///
    /// # Errors
    /// Returns filesystem removal failures.
    fn remove_dir_all(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>;

    /// Remove a file.
    ///
    /// # Errors
    /// Returns filesystem removal failures.
    fn remove_file(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>;

    /// Atomically write bytes to a file.
    ///
    /// # Errors
    /// Returns support publication failures.
    fn write_atomic<'semantic, B>(
        &mut self,
        path: &Path,
        bytes: B,
    ) -> Result<(), GateError>
    where
        B: Into<BytesBytes<'semantic>>;

    /// Publish a completed report into the workspace.
    ///
    /// # Errors
    /// Returns report publication failures.
    fn publish_report(
        &mut self,
        report: &Path,
        workspace_root: &Path,
    ) -> Result<(), GateError>;
}

/// Production host adapter backed by support helpers and the real filesystem.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct SupportMutantsHost;

impl MutantsHost for SupportMutantsHost
{
    #[inline]
    fn run_git_output<G>(
        &mut self,
        args: &[OsString],
        cwd: Option<&Path>,
        sanitized_git: G,
    ) -> Result<HostCommandOutcome, GateError>
    where
        G: Into<SanitizedGitFlag>,
    {
        let sanitized_git = sanitized_git.into().0;
        let output = support::run_output(OsStr::new(GIT_PROGRAM), args, cwd, sanitized_git)?;
        Ok(HostCommandOutcome::new(
            crate::semantic_value::<crate::support::SuccessFlag, _>(output.success()).0,
            crate::semantic_value::<crate::support::OptionalCodeCode, _>(output.code()).0,
            output.stdout_lossy().into_owned(),
        ))
    }

    #[inline]
    fn run_git_status<G>(
        &mut self,
        args: &[OsString],
        cwd: Option<&Path>,
        sanitized_git: G,
    ) -> Result<HostCommandOutcome, GateError>
    where
        G: Into<SanitizedGitFlag>,
    {
        let sanitized_git = sanitized_git.into().0;
        let status = support::run_status(OsStr::new(GIT_PROGRAM), args, cwd, sanitized_git)?;
        Ok(HostCommandOutcome::new(
            status.success(),
            status.code(),
            String::new(),
        ))
    }

    #[inline]
    fn run_host_status(
        &mut self,
        program: &OsStr,
        args: &[OsString],
        cwd: Option<&Path>,
    ) -> Result<HostCommandOutcome, GateError>
    {
        let status = support::run_status(program, args, cwd, false)?;
        Ok(HostCommandOutcome::new(
            status.success(),
            status.code(),
            String::new(),
        ))
    }

    #[inline]
    fn path_exists(
        &mut self,
        path: &Path,
    ) -> Result<impl Into<PathExistsFlag>, GateError>
    {
        crate::support::HOST_FILESYSTEM
            .try_exists(path)
            .map(bool::from)
    }

    #[inline]
    fn create_dir_all(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM.create_dir_all(path)
    }

    #[inline]
    fn remove_dir_all(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM.remove_dir_all(path)
    }

    #[inline]
    fn remove_file(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM.remove_file(path)
    }

    #[inline]
    fn write_atomic<'semantic, B>(
        &mut self,
        path: &Path,
        bytes: B,
    ) -> Result<(), GateError>
    where
        B: Into<BytesBytes<'semantic>>,
    {
        let bytes = bytes.into().0;
        support::write_atomic(path, bytes)
    }

    #[inline]
    fn publish_report(
        &mut self,
        report: &Path,
        workspace_root: &Path,
    ) -> Result<(), GateError>
    {
        report::publish_report(report, workspace_root)
    }
}

/// Program name used for sanitized Git host and guest commands.
const GIT_PROGRAM: &str = "git";
/// Program name used for host `mise` bootstrap checks.
const MISE_PROGRAM: &str = "mise";
/// Program name used by the guest kernel containment probe.
const UNAME_PROGRAM: &str = "uname";
/// Git ref archived for snapshot and default diff-scoped campaigns.
const SNAPSHOT_ARCHIVE_REF: &str = "HEAD";
/// Host `msb` scratch-volume name reserved for the mutation cache image.
const SNAPSHOT_CACHE_SCRATCH: &str = "gandr-mutants-mkfs";
/// `macOS` host filesystem roots that must not be reachable inside the guest.
const HOST_MARKERS: [&str; 3] = ["/Users", "/Volumes", "/System"];
/// Prefix for Git override variables recorded as ignored containment metadata.
const GIT_ENVIRONMENT_PREFIX: &str = "GIT_";

/// Common filesystem inputs for mutation commands.
///
/// # Contract
/// - requires: `workspace_root` is the repository root whose `mutants.out`
///   sibling is published, `cache_image` is the btrfs raw image used by msb,
///   and the three working paths name caller-owned temporary campaign files.
/// - ensures: host campaigns write only the supplied working paths before
///   rollback-safe publication into `workspace_root`.
/// - provides: a CLI-owned boundary between argument parsing and mutation
///   execution without exposing host cargo-mutants execution.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — host campaign integration tests distinguish the
///   archive, diff, report, and cache image paths, killing mutants that publish
///   from an unrelated directory or mount a host tree into the guest.
/// - witness: `mutants::sandbox::tests::sandbox_boot_plan_has_no_forbidden_host_mounts`
/// - witness: `mutants::report::tests::successful_publication_replaces_current_report`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutantsOptions
{
    /// Repository root used as the working directory for Git and report
    /// publish.
    pub workspace_root: PathBuf,
    /// Raw btrfs cache image attached to mutation sandboxes as a block device.
    pub cache_image: PathBuf,
    /// Temporary tracked-source tar archive path.
    pub source_archive: PathBuf,
    /// Temporary unified-diff path for changed-code campaigns.
    pub diff_file: PathBuf,
    /// Temporary cargo-mutants report directory before durable publication.
    pub working_report: PathBuf,
}

impl MutantsOptions
{
    /// Build mutation options from CLI-owned paths.
    ///
    /// # Contract
    /// - requires: each path is already resolved according to CLI policy; this
    ///   constructor does not touch the filesystem.
    /// - ensures: stores the paths exactly so execution can use borrowed
    ///   options without cloning or reparsing caller state.
    /// - provides: a stable constructor for external CLI crates while the
    ///   struct remains non-exhaustive for forward-compatible lint policy.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 exactness — CLI construction tests kill field-order
    ///   swaps because each stored path is observable through the public
    ///   fields.
    #[inline]
    #[must_use]
    pub fn new(
        workspace_root: PathBuf,
        cache_image: PathBuf,
        source_archive: PathBuf,
        diff_file: PathBuf,
        working_report: PathBuf,
    ) -> Self
    {
        Self {
            workspace_root,
            cache_image,
            source_archive,
            diff_file,
            working_report,
        }
    }
}

/// Mutation command selected by the CLI.
///
/// # Contract
/// - requires: each variant carries exactly the typed data needed by that mode:
///   push uses the shared push-range plan, scheduled uses sanitized ref tokens,
///   and guest carries only in-guest cargo-mutants selectors.
/// - ensures: snapshot, host campaigns, cleanup, and guest execution remain
///   distinct code paths with no host cargo-mutants bypass.
/// - provides: the minimal public facade consumed by the CLI sibling.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — CLI routing tests kill mutants that map one public
///   subcommand onto another mode or smuggle a host cargo-mutants invocation
///   into push/merge/scheduled/sweep.
/// - witness: `mutants::containment::tests::cargo_mutants_package_and_workspace_argv_are_exact`
/// - witness: `mutants::sandbox::tests::timeout_caps_and_sequential_jobs_are_planned`
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MutantsCommand
{
    /// Build the reusable `microVM` snapshot and btrfs cache image.
    Snapshot,
    /// Run a changed-code pre-push campaign over a shared push-range plan.
    Push
    {
        /// Push range resolved by the shared push resolver.
        range: range::PushRangePlan,
    },
    /// Run a changed-code pre-merge campaign against `main...HEAD`.
    Merge,
    /// Run a scheduled changed-code campaign over validated refs.
    Scheduled
    {
        /// Lower endpoint branch/tag/commit-ish token.
        from_ref: String,
        /// Upper endpoint branch/tag/commit-ish token; must resolve to `HEAD`.
        to_ref: String,
    },
    /// Run a whole-workspace sweep inside one `microVM`.
    Sweep,
    /// Reap stray mutation sandboxes owned by this driver.
    Clean,
    /// Run cargo-mutants inside an already-contained guest.
    Guest
    {
        /// Optional package selector passed after `--package`.
        package: Option<String>,
        /// Optional unified diff passed after `--in-diff`.
        diff: Option<PathBuf>,
    },
}

/// Execute one mutation command through the typed sequential facade.
///
/// # Contract
/// - requires: `command` is parsed by the CLI and `options` names caller-owned
///   temporary paths plus the repository root.
/// - ensures: commands execute sequentially, fail on the first operational
///   error, keep cargo-mutants behind the guest containment proof, preserve the
///   no-Rust fast path, and publish reports with rollback-safe staging.
/// - provides: the only public entry point for retained mutation machinery.
/// - fails: returns [`GateError`] for usage, Git, msb, filesystem, containment,
///   publication, or cargo-mutants failures.
/// - panics: none.
///
/// # Errors
/// Returns a typed gate error describing the first failed command or violated
/// contract. Mutation campaign failures are reported after their report has
/// been published when a report was preserved.
///
/// # Adequacy
/// - hypothesis: L3 end-to-end — command routing, infra failure, no-Rust skip,
///   teardown precedence, guest containment, and rollback publication each have
///   focused tests in the retained modules and are all reachable from this
///   facade.
/// - witness: `mutants::range::tests::non_rust_diff_is_noop_campaign`
/// - witness: `mutants::sandbox::tests::teardown_error_takes_precedence_over_payload_error`
/// - witness: `mutants::report::tests::simulated_final_rename_failure_restores_prior_report`
#[inline]
pub fn run(
    command: &MutantsCommand,
    options: &MutantsOptions,
) -> Result<(), GateError>
{
    let mut host = SupportMutantsHost;
    let mut infrastructure = sandbox::SupportSandboxInfrastructure;
    let mut runner = sandbox::SupportMsbAdapter;
    let mut sink = sandbox::SupportCampaignReportSink;
    run_with_environment(
        command,
        options,
        &mut host,
        &mut infrastructure,
        &mut runner,
        &mut sink,
    )
}

/// Execute one mutation command with injected host, infrastructure, runner,
/// and report sink adapters.
fn run_with_environment<Host, Infrastructure, Runner, Sink>(
    command: &MutantsCommand,
    options: &MutantsOptions,
    host: &mut Host,
    infrastructure: &mut Infrastructure,
    runner: &mut Runner,
    sink: &mut Sink,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    Infrastructure: sandbox::SandboxInfrastructure,
    Runner: sandbox::MsbAdapter,
    Sink: sandbox::CampaignReportSink,
{
    match *command {
        | MutantsCommand::Snapshot => {
            run_snapshot_with_environment(host, infrastructure, runner, options)
        },
        | MutantsCommand::Push { ref range } => {
            let diff = range::push_diff_plan(range);
            run_diff_campaign_with_environment(
                host,
                infrastructure,
                runner,
                sink,
                options,
                sandbox::CampaignMode::Push,
                diff,
                SNAPSHOT_ARCHIVE_REF,
            )
        },
        | MutantsCommand::Merge => {
            let diff = range::merge_diff_plan();
            run_diff_campaign_with_environment(
                host,
                infrastructure,
                runner,
                sink,
                options,
                sandbox::CampaignMode::Merge,
                diff,
                SNAPSHOT_ARCHIVE_REF,
            )
        },
        | MutantsCommand::Scheduled {
            ref from_ref,
            ref to_ref,
        } => {
            let (diff, archive_ref) =
                resolve_scheduled_diff_with_host(host, options, from_ref, to_ref)?;
            run_diff_campaign_with_environment(
                host,
                infrastructure,
                runner,
                sink,
                options,
                sandbox::CampaignMode::Scheduled,
                diff,
                &archive_ref,
            )
        },
        | MutantsCommand::Sweep => {
            run_sweep_with_environment(host, infrastructure, runner, sink, options)
        },
        | MutantsCommand::Clean => run_clean_with_environment(infrastructure, runner),
        | MutantsCommand::Guest {
            ref package,
            ref diff,
        } => run_guest(package.as_deref(), diff.as_deref()),
    }
}

/// Execute the exact snapshot build path retained from `mutants-vm.nu
/// snapshot`.
///
/// # Contract
/// - requires: `msb` is installed and `options.cache_image` names the reusable
///   btrfs cache image path.
/// - ensures: the cache image is formatted through a temporary msb disk volume,
///   the builder boots from `ubuntu:24.04`, the sentinel/toolchain/mise warm
///   path runs in the builder, and `gandr-mutants-base` is created from that
///   builder.
/// - provides: real snapshot provisioning; it never substitutes a probe-only
///   fake.
/// - fails: returns the first command, cleanup, Git archive, or filesystem
///   error with teardown errors taking precedence where the script made them
///   fatal.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — snapshot-plan tests distinguish cache formatting,
///   provision/warm execs, source copy, stop-before-create, and builder
///   removal.
/// - witness: `AuditCiCampaignScripts` snapshot branch notes for
///   `scripts/mutants-vm.nu`
///
/// Execute snapshot provisioning with injected host and msb adapters.
fn run_snapshot_with_environment<Host, Infrastructure, Runner>(
    host: &mut Host,
    infrastructure: &mut Infrastructure,
    runner: &mut Runner,
    options: &MutantsOptions,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    Infrastructure: sandbox::SandboxInfrastructure,
    Runner: sandbox::MsbAdapter,
{
    require_msb_available(infrastructure)?;

    let config = sandbox::SandboxConfig::new(&options.cache_image);
    format_cache_image(host, runner, &options.cache_image)?;
    run_snapshot_builder(host, runner, options, &config)
}

/// Execute a diff-selected host campaign and publish its report.
///
/// # Contract
/// - requires: `diff` is the exact Git diff plan for `mode` and `archive_ref`
///   names the commit that should be archived into the guest.
/// - ensures: required VM infrastructure is checked before the
///   script-compatible campaign path, the diff is materialized, no-Rust diffs
///   skip VM boot, and Rust diffs execute one sandbox with sequential jobs.
/// - provides: push, merge, and scheduled host campaign behavior.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — merge/push/scheduled diff tests and no-Rust
///   fast path tests kill wrong range specs and accidental VM boots for
///   docs-only diffs.
///
/// Execute a diff-selected host campaign with injected adapters.
#[expect(
    clippy::too_many_arguments,
    reason = "test seam names each boundary explicitly so host, VM, and report side effects cannot be conflated"
)]
fn run_diff_campaign_with_environment<'semantic, Host, Infrastructure, Runner, Sink, A>(
    host: &mut Host,
    infrastructure: &mut Infrastructure,
    runner: &mut Runner,
    sink: &mut Sink,
    options: &MutantsOptions,
    mode: sandbox::CampaignMode,
    diff: range::GitDiffPlan,
    archive_ref: A,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    Infrastructure: sandbox::SandboxInfrastructure,
    Runner: sandbox::MsbAdapter,
    Sink: sandbox::CampaignReportSink,
    A: Into<ArchiveRefText<'semantic>>,
{
    let archive_ref = archive_ref.into().0;
    require_campaign_infra(infrastructure, options)?;
    prepare_report_dir(host, &options.working_report)?;
    let diff_text = materialize_diff(host, options, &diff)?;
    match range::plan_in_diff_campaign(diff, &diff_text) {
        | range::RangeCampaignPlan::NoRustChanges { report } => {
            run_no_rust_campaign(runner, sink, options, mode, &diff_text, report)?;
            publish_campaign_result(host, options)?;
            cleanup_success_workspace(host, options)
        },
        | range::RangeCampaignPlan::RunInDiff { .. } => {
            run_contained_campaign(host, runner, sink, &ContainedCampaign {
                options,
                mode,
                diff_path: Some(&options.diff_file),
                diff_source: &diff_text,
                archive_ref,
            })
        },
    }
}

/// Reap stray mutation sandboxes without touching reports or cache volumes.
///
/// # Contract
/// - requires: `msb` is installed.
/// - ensures: only sandbox names with the owned prefix are stopped and removed;
///   cache-image scratch volumes are not part of clean mode.
/// - provides: the original `mutants-vm.nu clean` behavior.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — prefix cleanup tests distinguish owned sandboxes
///   from foreign ones and aggregate stop/remove failures.
///
/// Reap stray mutation sandboxes with injected infrastructure and runner.
fn run_clean_with_environment<Infrastructure, Runner>(
    infrastructure: &mut Infrastructure,
    runner: &mut Runner,
) -> Result<(), GateError>
where
    Infrastructure: sandbox::SandboxInfrastructure,
    Runner: sandbox::MsbAdapter,
{
    require_msb_available(infrastructure)?;
    let _cleaned = sandbox::cleanup_stray_sandboxes(runner)?;
    Ok(())
}

/// Run cargo-mutants inside an already-contained guest.
///
/// # Contract
/// - requires: the current process is running in the mutation guest or another
///   environment that satisfies the positive containment evidence.
/// - ensures: containment is proven before `cargo mutants` can run, a minimal
///   Git repository is synthesized for archive extracts, and `--jobs` is fixed
///   at one.
/// - provides: the only cargo-mutants execution path in the Rust facade.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — containment proof tests and cargo-mutants argv
///   tests kill host bypasses, missing diff acceptance, package/workspace
///   swaps, and parallel job counts.
fn run_guest<'semantic, P>(
    package: P,
    diff: Option<&Path>,
) -> Result<(), GateError>
where
    P: Into<OptionalPackageText<'semantic>>,
{
    let package = package.into().0;
    let evidence = containment_evidence()?;
    let _report = containment::require_containment(&evidence)?;
    ensure_guest_git_repo()?;

    let scope = match package {
        | Some(name) => containment::CargoMutantsScope::package(name)?,
        | None => containment::CargoMutantsScope::Workspace,
    };
    let diff_state = match diff {
        | Some(path)
            if crate::support::HOST_FILESYSTEM
                .try_exists(path)
                .map(bool::from)? =>
        {
            containment::DiffPath::Present(path)
        },
        | Some(path) => containment::DiffPath::Missing(path),
        | None => containment::DiffPath::Absent,
    };
    let request = containment::CargoMutantsRequest {
        scope,
        diff: diff_state,
        jobs: containment::CargoMutantsJobs::from_requested(containment::DEFAULT_MUTANTS_JOBS)?,
    };
    let plan = containment::cargo_mutants_plan(&request)?;
    let status = run_guest_cargo_mutants(&plan)?;
    if crate::semantic_value::<SuccessFlag, _>(status.success()).0 {
        return Ok(());
    }
    Err(GateError::operational(
        "mutants-guest: cargo-mutants reported failures.\nExit codes: 2 surviving mutant, 3 timeout, 4 baseline already failing,\n5 diff does not match the source tree, 6 unparseable diff.",
    ))
}

/// Resolve a scheduled diff and its archive ref through sanitized Git commands.
/// Resolve a scheduled diff through an injected host adapter.
fn resolve_scheduled_diff_with_host<'semantic, Host, F, T>(
    host: &mut Host,
    options: &MutantsOptions,
    from_ref: F,
    to_ref: T,
) -> Result<(range::GitDiffPlan, String), GateError>
where
    Host: MutantsHost,
    F: Into<FromRefText<'semantic>>,
    T: Into<ToRefText<'semantic>>,
{
    let to_ref = to_ref.into().0;
    let from_ref = from_ref.into().0;
    let from_token = range::validate_scheduled_ref_token(from_ref, "from")?;
    let to_token = range::validate_scheduled_ref_token(to_ref, "to")?;
    let from_oid = git_output_trimmed_with_host(
        host,
        options,
        &[
            os("rev-parse"),
            os("--verify"),
            os("--quiet"),
            OsString::from(format!(
                "{}^{{commit}}",
                crate::semantic_value::<range::AsStrText<'_>, _>(from_token.as_str()).0
            )),
        ],
        "scheduled --from ref resolution",
    )?;
    let to_oid = git_output_trimmed_with_host(
        host,
        options,
        &[
            os("rev-parse"),
            os("--verify"),
            os("--quiet"),
            OsString::from(format!(
                "{}^{{commit}}",
                crate::semantic_value::<range::AsStrText<'_>, _>(to_token.as_str()).0
            )),
        ],
        "scheduled --to ref resolution",
    )?;
    let head_oid = git_output_trimmed_with_host(
        host,
        options,
        &[
            os("rev-parse"),
            os("--verify"),
            os("--quiet"),
            os("HEAD^{commit}"),
        ],
        "scheduled HEAD resolution",
    )?;
    let from_is_ancestor_of_to = git_status_success_with_host(host, options, &[
        os("merge-base"),
        os("--is-ancestor"),
        OsString::from(&from_oid),
        OsString::from(&to_oid),
    ])
    .map(|value| value.into().0)?;
    let input = range::ScheduledRangeInput {
        from_ref,
        to_ref,
        from_oid: &from_oid,
        to_oid: &to_oid,
        head_oid: &head_oid,
        from_is_ancestor_of_to,
    };
    let diff = range::scheduled_diff_plan(&input)?;
    Ok((diff, to_oid))
}

/// Execute a whole-workspace sweep and publish its report.
///
/// # Contract
/// - requires: snapshot and cache infrastructure exist.
/// - ensures: one sweep sandbox runs with the fixed sequential job count even
///   though the historical script accepted a wider sweep jobs value.
/// - provides: overnight sweep behavior without host cargo-mutants execution.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — timeout and job-count plan tests kill mutations that
///   restore parallel cargo-mutants execution or omit the sweep timeout cap.
///
/// Execute a whole-workspace sweep with injected adapters.
fn run_sweep_with_environment<Host, Infrastructure, Runner, Sink>(
    host: &mut Host,
    infrastructure: &mut Infrastructure,
    runner: &mut Runner,
    sink: &mut Sink,
    options: &MutantsOptions,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    Infrastructure: sandbox::SandboxInfrastructure,
    Runner: sandbox::MsbAdapter,
    Sink: sandbox::CampaignReportSink,
{
    require_campaign_infra(infrastructure, options)?;
    prepare_report_dir(host, &options.working_report)?;
    run_contained_campaign(host, runner, sink, &ContainedCampaign {
        options,
        mode: sandbox::CampaignMode::Sweep,
        diff_path: None,
        diff_source: "",
        archive_ref: SNAPSHOT_ARCHIVE_REF,
    })
}

/// Require campaign infrastructure exactly as the host script did.
fn require_campaign_infra<Infrastructure>(
    infrastructure: &mut Infrastructure,
    options: &MutantsOptions,
) -> Result<(), GateError>
where
    Infrastructure: sandbox::SandboxInfrastructure,
{
    let config = sandbox::SandboxConfig::new(&options.cache_image);
    sandbox::require_infra(infrastructure, &config)
}

/// Require only the msb executable for snapshot and clean modes.
fn require_msb_available<Infrastructure>(
    infrastructure: &mut Infrastructure
) -> Result<(), GateError>
where
    Infrastructure: sandbox::SandboxInfrastructure,
{
    if infrastructure.msb_available().map(|value| value.into().0)? {
        return Ok(());
    }
    Err(GateError::operational(
        "mutants-vm: `msb` (microsandbox) not found. It is pinned in mise.toml; run `mise install` to provision it.",
    ))
}

/// Write the no-Rust report through the same campaign sink used by sandbox
/// execution.
fn run_no_rust_campaign<'semantic, Runner, Sink, D, L>(
    runner: &mut Runner,
    sink: &mut Sink,
    options: &MutantsOptions,
    mode: sandbox::CampaignMode,
    diff_source: D,
    report_label: L,
) -> Result<(), GateError>
where
    Runner: sandbox::MsbAdapter,
    Sink: sandbox::CampaignReportSink,
    D: Into<DiffSourceText<'semantic>>,
    L: Into<ReportLabelText<'semantic>>,
{
    let report_label = report_label.into().0;
    let diff_source = diff_source.into().0;
    if report_label.is_empty() {
        return Err(GateError::operational(
            "mutants-vm: no-Rust campaign report label is empty",
        ));
    }
    let config = sandbox::SandboxConfig::new(&options.cache_image);
    let sandbox_name = temporary_sandbox_name(mode)?;
    let request = sandbox::CampaignRequest::new(
        mode,
        &sandbox_name,
        &options.source_archive,
        Some(&options.diff_file),
        &options.working_report,
    );
    let summary = sandbox::execute_campaign_request(runner, sink, &config, &request, diff_source)?;
    if summary.succeeded().into().0 {
        return Ok(());
    }
    Err(campaign_failure())
}

/// Write a tracked-file source archive for the guest.
fn write_source_archive<'semantic, Host, A>(
    host: &mut Host,
    options: &MutantsOptions,
    archive_ref: A,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    A: Into<ArchiveRefText<'semantic>>,
{
    let archive_ref = archive_ref.into().0;
    ensure_parent_dir(host, &options.source_archive)?;
    run_git_status_checked_with_host(
        host,
        &[
            os("archive"),
            os("--format=tar"),
            os("--output"),
            options.source_archive.as_os_str().to_os_string(),
            OsString::from(archive_ref),
        ],
        Some(options.workspace_root.as_path()),
        "mutants-vm: git archive failed",
    )
}

/// Remove stale temporary report data before starting a campaign.
fn prepare_report_dir<Host>(
    host: &mut Host,
    report_dir: &Path,
) -> Result<(), GateError>
where
    Host: MutantsHost,
{
    if host.path_exists(report_dir).map(|value| value.into().0)? {
        host.remove_dir_all(report_dir)?;
    }
    ensure_parent_dir(host, report_dir)
}

/// Materialize a Git diff plan into the caller-provided diff file.
fn materialize_diff<Host>(
    host: &mut Host,
    options: &MutantsOptions,
    diff: &range::GitDiffPlan,
) -> Result<String, GateError>
where
    Host: MutantsHost,
{
    let diff_text = git_output_with_host(host, options, diff.args(), "diff")?;
    ensure_parent_dir(host, &options.diff_file)?;
    host.write_atomic(&options.diff_file, diff_text.as_bytes())?;
    Ok(diff_text)
}

/// Remove caller-owned success temporaries after durable publication.
fn cleanup_success_workspace<Host>(
    host: &mut Host,
    options: &MutantsOptions,
) -> Result<(), GateError>
where
    Host: MutantsHost,
{
    remove_file_if_exists(host, &options.source_archive)?;
    remove_file_if_exists(host, &options.diff_file)?;
    if host
        .path_exists(&options.working_report)
        .map(|value| value.into().0)?
    {
        host.remove_dir_all(&options.working_report)?;
    }
    Ok(())
}

/// Format the btrfs cache image through the script's temporary msb volume path.
fn format_cache_image<Host, Runner>(
    host: &mut Host,
    runner: &mut Runner,
    cache_image: &Path,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    Runner: sandbox::MsbAdapter,
{
    sandbox::remove_cache_scratch(runner, SNAPSHOT_CACHE_SCRATCH).map(|value| value.into().0)?;
    if host.path_exists(cache_image).map(|value| value.into().0)? {
        return Ok(());
    }

    let outcome = (|| -> Result<(), GateError> {
        run_msb_checked_status(
            runner,
            &sandbox::volume_create_plan(SNAPSHOT_CACHE_SCRATCH),
            "mutants-vm: failed to create cache scratch volume",
        )?;
        let inspect = run_msb_checked_output(
            runner,
            &sandbox::volume_inspect_plan(SNAPSHOT_CACHE_SCRATCH),
            "mutants-vm: volume inspect failed",
        )?;
        let backing = volume_backing_path(inspect.stdout().into().0)?;
        run_msb_checked_status(
            runner,
            &sandbox::format_cache_image_plan(SNAPSHOT_CACHE_SCRATCH),
            "mutants-vm: failed to format btrfs cache image",
        )?;
        ensure_parent_dir(host, cache_image)?;
        run_host_status_checked_with_host(
            host,
            OsStr::new(MISE_PROGRAM),
            &[
                os("exec"),
                os("--"),
                os("coreutils"),
                os("cp"),
                os("--reflink=auto"),
                backing.as_os_str().to_os_string(),
                cache_image.as_os_str().to_os_string(),
            ],
            None,
            "mutants-vm: failed to preserve cache image",
        )
    })();

    let cleanup =
        sandbox::remove_cache_scratch(runner, SNAPSHOT_CACHE_SCRATCH).map(|value| value.into().0);
    if let Err(cleanup_error) = cleanup {
        return Err(GateError::operational(format!(
            "{}; cache-image result: {}",
            cleanup_error,
            result_detail(&outcome)
        )));
    }
    outcome
}

/// Run the script-compatible snapshot builder lifecycle.
fn run_snapshot_builder<Host, Runner>(
    host: &mut Host,
    runner: &mut Runner,
    options: &MutantsOptions,
    config: &sandbox::SandboxConfig,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    Runner: sandbox::MsbAdapter,
{
    let builder = sandbox::snapshot_builder_name()?;
    let _ignored = runner.run_status(sandbox::remove_plan(&builder).args());
    run_msb_checked_status(
        runner,
        &sandbox::snapshot_builder_boot_plan(config, &builder),
        "mutants-vm: failed to boot snapshot builder",
    )?;

    let tar_path = temporary_tar_path()?;
    let mut builder_stopped = false;
    let outcome = (|| -> Result<(), GateError> {
        run_msb_checked_status(
            runner,
            &sandbox::snapshot_builder_provision_plan(&builder),
            "mutants-vm: snapshot provision command failed",
        )?;
        write_snapshot_source_archive(host, options, &tar_path)?;
        run_msb_checked_status(
            runner,
            &sandbox::snapshot_builder_copy_source_plan(&builder, &tar_path),
            "mutants-vm: failed to copy source archive into snapshot builder",
        )?;
        run_msb_checked_status(
            runner,
            &sandbox::snapshot_builder_warm_plan(&builder),
            "mutants-vm: snapshot warm command failed",
        )?;
        run_msb_checked_status(
            runner,
            &sandbox::stop_plan(&builder),
            "mutants-vm: failed to stop snapshot builder",
        )?;
        builder_stopped = true;
        run_msb_checked_status(
            runner,
            &sandbox::snapshot_create_plan(&builder),
            "mutants-vm: snapshot create failed",
        )
    })();

    let _removed_tar = remove_file_if_exists(host, &tar_path);
    let stop_error = if builder_stopped {
        None
    }
    else {
        let stopped = runner.run_status(sandbox::stop_plan(&builder).args());
        match stopped {
            | Ok(output) if output.success_status().into().0 => None,
            | Ok(output) => Some(format!(
                "mutants-vm: failed to stop snapshot builder: {}",
                msb_output_detail(&output)
            )),
            | Err(error) => Some(format!(
                "mutants-vm: failed to stop snapshot builder: {error}"
            )),
        }
    };
    let removed = runner.run_status(sandbox::remove_plan(&builder).args());
    match removed {
        | Ok(output) if output.success_status().into().0 => {},
        | Ok(output) => {
            return Err(GateError::operational(format!(
                "mutants-vm: failed to remove snapshot builder: {}; provisioning result: {}",
                msb_output_detail(&output),
                result_detail(&outcome)
            )));
        },
        | Err(error) => {
            return Err(GateError::operational(format!(
                "mutants-vm: failed to remove snapshot builder: {}; provisioning result: {}",
                error,
                result_detail(&outcome)
            )));
        },
    }
    if let Some(error) = stop_error {
        return Err(GateError::operational(format!(
            "{}; provisioning result: {}",
            error,
            result_detail(&outcome)
        )));
    }
    outcome.map_err(|error| {
        GateError::operational(format!("mutants-vm: snapshot provisioning failed: {error}"))
    })
}

/// Write the tracked source archive used to warm the snapshot cache.
fn write_snapshot_source_archive<Host>(
    host: &mut Host,
    options: &MutantsOptions,
    tar_path: &Path,
) -> Result<(), GateError>
where
    Host: MutantsHost,
{
    run_git_status_checked_with_host(
        host,
        &[
            os("archive"),
            os("--format=tar"),
            os("--output"),
            tar_path.as_os_str().to_os_string(),
            os(SNAPSHOT_ARCHIVE_REF),
        ],
        Some(options.workspace_root.as_path()),
        "mutants-vm: git archive failed",
    )
}

/// Build containment evidence from process and filesystem probes.
fn containment_evidence() -> Result<containment::ContainmentEvidence, GateError>
{
    Ok(containment::ContainmentEvidence {
        kernel_name: kernel_name()?,
        act_value: env::var(ACT_ENVIRONMENT_VARIABLE).ok(),
        reachable_host_markers: reachable_host_markers()?,
        sentinel: guest_sentinel(),
        git_environment_markers: git_environment_markers(),
    })
}

/// Read the kernel name used by the non-Darwin containment proof.
fn kernel_name() -> Result<String, GateError>
{
    let output = support::run_output(OsStr::new(UNAME_PROGRAM), &[os("-s")], None, false)?;
    if !crate::semantic_value::<crate::support::SuccessFlag, _>(output.success()).0 {
        return Err(GateError::operational(format!(
            "mutants-guest: failed to read kernel name: {}",
            support::command_status_detail(
                UNAME_PROGRAM,
                crate::semantic_value::<crate::support::OptionalCodeCode, _>(output.code()).0
            )
        )));
    }
    Ok(output.stdout_lossy().as_ref().trim().to_owned())
}

/// Return host roots visible from the current process.
fn reachable_host_markers() -> Result<Vec<PathBuf>, GateError>
{
    let mut markers = Vec::new();
    for marker in HOST_MARKERS {
        let path = Path::new(marker);
        if crate::support::HOST_FILESYSTEM
            .try_exists(path)
            .map(bool::from)?
        {
            markers.push(path.to_path_buf());
        }
    }
    Ok(markers)
}

/// Return the guest sentinel state without accepting read failures as proof.
fn guest_sentinel() -> containment::GuestSentinel
{
    match crate::support::HOST_FILESYSTEM.read_to_string(containment::SENTINEL_PATH) {
        | Ok(token) if token.trim() == containment::SENTINEL_TOKEN => {
            containment::GuestSentinel::Valid
        },
        | Ok(observed) => containment::GuestSentinel::Invalid {
            observed: observed.trim().to_owned(),
        },
        | Err(_source) => containment::GuestSentinel::Absent,
    }
}

/// Collect Git environment markers that are diagnostics but not containment
/// proof.
fn git_environment_markers() -> Vec<String>
{
    let mut markers = Vec::new();
    for (key, value) in env::vars() {
        if key.starts_with(GIT_ENVIRONMENT_PREFIX) {
            markers.push(format!("{key}={value}"));
        }
    }
    markers
}

/// Create a minimal Git repository for cargo-mutants archive execution.
fn ensure_guest_git_repo() -> Result<(), GateError>
{
    if crate::support::HOST_FILESYSTEM
        .try_exists(".git")
        .map(bool::from)?
    {
        return Ok(());
    }
    run_git_status_checked(
        &[os("init"), os("--quiet"), os("--initial-branch=main")],
        None,
        "mutants-guest: git init failed",
    )?;
    run_git_status_checked(
        &[os("add"), os("--all")],
        None,
        "mutants-guest: git add failed",
    )?;
    run_git_status_checked(
        &[
            os("commit"),
            os("--quiet"),
            os("-m"),
            os("mutants baseline"),
        ],
        None,
        "mutants-guest: git commit failed",
    )
}

/// Run sanitized Git and return trimmed stdout.
#[cfg(test)]
fn git_output_trimmed<'semantic, C>(
    options: &MutantsOptions,
    args: &[OsString],
    context: C,
) -> Result<String, GateError>
where
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let mut host = SupportMutantsHost;
    git_output_trimmed_with_host(&mut host, options, args, context)
}

/// Run sanitized Git through an injected host and return trimmed stdout.
fn git_output_trimmed_with_host<'semantic, Host, C>(
    host: &mut Host,
    options: &MutantsOptions,
    args: &[OsString],
    context: C,
) -> Result<String, GateError>
where
    Host: MutantsHost,
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    Ok(git_output_with_host(host, options, args, context)?
        .trim()
        .to_owned())
}

/// Run sanitized Git through an injected host and return complete stdout text.
fn git_output_with_host<'semantic, Host, C>(
    host: &mut Host,
    options: &MutantsOptions,
    args: &[OsString],
    context: C,
) -> Result<String, GateError>
where
    Host: MutantsHost,
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let output = host.run_git_output(args, Some(options.workspace_root.as_path()), true)?;
    if !crate::semantic_value::<SuccessFlag, _>(output.success()).0 {
        return Err(GateError::operational(format!(
            "mutants-vm: git {context} failed: {}",
            support::command_status_detail(
                GIT_PROGRAM,
                crate::semantic_value::<OptionalCodeCode, _>(output.code()).0
            )
        )));
    }
    Ok(output.stdout().into().0.to_owned())
}

/// Create a parent directory when a path has one.
fn ensure_parent_dir<Host>(
    host: &mut Host,
    path: &Path,
) -> Result<(), GateError>
where
    Host: MutantsHost,
{
    let Some(parent) = path.parent()
    else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    host.create_dir_all(parent)
}

/// Run Git status command and return its success bit.
#[cfg(test)]
fn git_status_success(
    options: &MutantsOptions,
    args: &[OsString],
) -> Result<impl Into<GitStatusSuccessFlag>, GateError>
{
    let mut host = SupportMutantsHost;
    git_status_success_with_host(&mut host, options, args).map(|value| value.into().0)
}

/// Run Git status command through an injected host and return its success bit.
fn git_status_success_with_host<Host>(
    host: &mut Host,
    options: &MutantsOptions,
    args: &[OsString],
) -> Result<impl Into<GitStatusSuccessWithHostFlag>, GateError>
where
    Host: MutantsHost,
{
    let status = host.run_git_status(args, Some(options.workspace_root.as_path()), true)?;
    Ok(status.success().into().0)
}

/// Run an msb plan and require bounded captured-stdout success.
fn run_msb_checked_output<'semantic, Runner, C>(
    runner: &mut Runner,
    plan: &sandbox::MsbPlan,
    context: C,
) -> Result<sandbox::CommandOutcome, GateError>
where
    Runner: sandbox::MsbAdapter,
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let output = runner.run_output(plan.args())?;
    if output.success_status().into().0 {
        return Ok(output);
    }
    Err(GateError::operational(format!(
        "{context}: {}",
        msb_output_detail(&output)
    )))
}

/// Run an msb plan and require inherited-stream status success.
fn run_msb_checked_status<'semantic, Runner, C>(
    runner: &mut Runner,
    plan: &sandbox::MsbPlan,
    context: C,
) -> Result<(), GateError>
where
    Runner: sandbox::MsbAdapter,
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let output = runner.run_status(plan.args())?;
    if output.success_status().into().0 {
        return Ok(());
    }
    Err(GateError::operational(format!(
        "{context}: {}",
        msb_output_detail(&output)
    )))
}

/// Run cargo-mutants with Git overrides cleared before it inspects the guest
/// repo.
fn run_guest_cargo_mutants(
    plan: &containment::CargoMutantsPlan
) -> Result<process::ExitStatus, GateError>
{
    support::run_status(plan.program.as_os_str(), plan.args(), None, true)
}

/// Run a sanitized Git status command through the support adapter and require
/// success.
fn run_git_status_checked<'semantic, C>(
    args: &[OsString],
    cwd: Option<&Path>,
    context: C,
) -> Result<(), GateError>
where
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let args = support::stateless_git_args(args);
    let mut host = SupportMutantsHost;
    run_git_status_checked_with_host(&mut host, &args, cwd, context)
}

/// Run a sanitized Git status command through an injected host and require
/// success.
fn run_git_status_checked_with_host<'semantic, Host, C>(
    host: &mut Host,
    args: &[OsString],
    cwd: Option<&Path>,
    context: C,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let status = host.run_git_status(args, cwd, true)?;
    if crate::semantic_value::<SuccessFlag, _>(status.success()).0 {
        return Ok(());
    }
    Err(GateError::operational(format!(
        "{context}: {}",
        support::command_status_detail(
            GIT_PROGRAM,
            crate::semantic_value::<OptionalCodeCode, _>(status.code()).0
        )
    )))
}

/// Run a non-Git host command through an injected host and require success.
fn run_host_status_checked_with_host<'semantic, Host, C>(
    host: &mut Host,
    program: &OsStr,
    args: &[OsString],
    cwd: Option<&Path>,
    context: C,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    C: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let status = host.run_host_status(program, args, cwd)?;
    if crate::semantic_value::<SuccessFlag, _>(status.success()).0 {
        return Ok(());
    }
    Err(GateError::operational(format!(
        "{context}: {}",
        support::command_status_detail(
            program.to_string_lossy().as_ref(),
            crate::semantic_value::<OptionalCodeCode, _>(status.code()).0
        )
    )))
}

/// Extract the raw image path from `msb volume inspect` output.
fn volume_backing_path<'semantic, S>(stdout: S) -> Result<PathBuf, GateError>
where
    S: Into<StdoutText<'semantic>>,
{
    let stdout = stdout.into().0;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("Path:") {
            let path = rest.trim();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }
    Err(GateError::operational(
        "mutants-vm: could not resolve the volume backing image path",
    ))
}

/// Build a unique sandbox name within the owned prefix.
fn temporary_sandbox_name(mode: sandbox::CampaignMode) -> Result<sandbox::SandboxName, GateError>
{
    let nonce = nonce_text()?;
    sandbox::SandboxName::new(&format!(
        "{}{}-{}",
        sandbox::SANDBOX_PREFIX,
        crate::semantic_value::<sandbox::AsStrText<'static>, _>(mode.as_str()).0,
        nonce
    ))
}

/// Publish the temporary report with rollback protection.
fn publish_campaign_result<Host>(
    host: &mut Host,
    options: &MutantsOptions,
) -> Result<(), GateError>
where
    Host: MutantsHost,
{
    let result_file = options.working_report.join("campaign.nuon");
    if !host.path_exists(&result_file).map(|value| value.into().0)? {
        return Err(GateError::operational(format!(
            "mutants-vm: campaign report was not preserved; temporary data remains at {}",
            options.working_report.display()
        )));
    }
    host.publish_report(&options.working_report, &options.workspace_root)
}

/// Run a contained campaign, publish its report, and fail after publication on
/// mutation failures.
fn run_contained_campaign<Host, Runner, Sink>(
    host: &mut Host,
    runner: &mut Runner,
    sink: &mut Sink,
    campaign: &ContainedCampaign<'_>,
) -> Result<(), GateError>
where
    Host: MutantsHost,
    Runner: sandbox::MsbAdapter,
    Sink: sandbox::CampaignReportSink,
{
    write_source_archive(host, campaign.options, campaign.archive_ref)?;
    let config = sandbox::SandboxConfig::new(&campaign.options.cache_image);
    let sandbox_name = temporary_sandbox_name(campaign.mode)?;
    let request = sandbox::CampaignRequest::new(
        campaign.mode,
        &sandbox_name,
        &campaign.options.source_archive,
        campaign.diff_path,
        &campaign.options.working_report,
    );
    let summary =
        sandbox::execute_campaign_request(runner, sink, &config, &request, campaign.diff_source)?;
    publish_campaign_result(host, campaign.options)?;
    cleanup_success_workspace(host, campaign.options)?;
    if summary.succeeded().into().0 {
        return Ok(());
    }
    Err(campaign_failure())
}

/// Build a unique temporary tar path for snapshot provisioning.
fn temporary_tar_path() -> Result<PathBuf, GateError>
{
    let nonce = nonce_text()?;
    Ok(env::temp_dir().join(format!("gandr-mutants-src.{nonce}.tar")))
}

/// Remove one optional temporary file.
fn remove_file_if_exists<Host>(
    host: &mut Host,
    path: &Path,
) -> Result<(), GateError>
where
    Host: MutantsHost,
{
    if !host.path_exists(path).map(|value| value.into().0)? {
        return Ok(());
    }
    host.remove_file(path)
}

/// Return a filesystem-safe process/time nonce.
fn nonce_text() -> Result<String, GateError>
{
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|source| {
            GateError::operational(format!("mutants-vm: system clock error: {source}"))
        })?;
    Ok(format!("{}-{}", process::id(), elapsed.as_nanos()))
}

/// Return an operational campaign failure after report publication.
fn campaign_failure() -> GateError
{
    GateError::operational("mutants-vm: cargo-mutants failed inside the microVM (see output above)")
}

/// Render an msb command detail from consumed stdout or status.
fn msb_output_detail(output: &sandbox::CommandOutcome) -> String
{
    if !output.stdout().into().0.is_empty() {
        return output.stdout().into().0.to_owned();
    }
    support::command_status_detail("msb", output.code().into().0)
}

/// Render a `Result` as compact failure detail for cleanup precedence messages.
fn result_detail(result: &Result<(), GateError>) -> String
{
    match *result {
        | Ok(()) => String::new(),
        | Err(ref error) => error.to_string(),
    }
}

/// Convert a UTF-8 literal to an operating-system argument.
fn os<'semantic, V>(value: V) -> OsString
where
    V: Into<ValueText<'semantic>>,
{
    let value = value.into().0;
    OsString::from(value)
}

/// Immutable inputs identifying one contained mutation campaign.
///
/// # Contract
/// - requires: `archive_ref` identifies the source revision archived for the
///   guest and `diff_path` belongs to `options` when the mode is diff-selected.
/// - ensures: preserves borrowed campaign inputs without copying paths or
///   source text.
/// - provides: a typed boundary between campaign identity and injected
///   side-effect adapters.
/// - panics: none.
struct ContainedCampaign<'campaign>
{
    /// Shared workspace paths and campaign policy.
    options: &'campaign MutantsOptions,
    /// Mutation campaign mode.
    mode: sandbox::CampaignMode,
    /// Optional guest-visible diff path.
    diff_path: Option<&'campaign Path>,
    /// Exact diff text passed to the sandbox planner.
    diff_source: &'campaign str,
    /// Git revision archived into the guest.
    archive_ref: &'campaign str,
}

/// Environment variable set by GitHub-local `act` runs.
const ACT_ENVIRONMENT_VARIABLE: &str = "ACT";

#[cfg(test)]
mod tests
{
    //! Integration-boundary witnesses for mutation Git environment hygiene.

    use alloc::boxed::Box;
    use alloc::collections::VecDeque;
    use alloc::format;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::error::Error;
    use std::env;
    use std::ffi::OsStr;
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    use super::*;

    /// Result type used by mutation integration-boundary tests.
    type TestResult<T = ()> = Result<T, Box<dyn Error>>;

    /// Scripted host adapter that fakes commands and delegates filesystem
    /// operations to the temporary fixture tree.
    #[derive(Default)]
    struct FakeHost
    {
        /// Captured Git stdout command calls.
        git_output_calls: Vec<Vec<OsString>>,
        /// Captured Git status command calls.
        git_status_calls: Vec<Vec<OsString>>,
        /// Captured non-Git host status command calls.
        host_status_calls: Vec<(String, Vec<OsString>)>,
        /// Queued Git stdout outcomes.
        git_outputs: VecDeque<HostCommandOutcome>,
        /// Queued Git status outcomes.
        git_statuses: VecDeque<HostCommandOutcome>,
        /// Queued non-Git status outcomes.
        host_statuses: VecDeque<HostCommandOutcome>,
    }

    impl FakeHost
    {
        /// Build a fake host with queued command outcomes.
        fn new(
            git_outputs: Vec<HostCommandOutcome>,
            git_statuses: Vec<HostCommandOutcome>,
            host_statuses: Vec<HostCommandOutcome>,
        ) -> Self
        {
            Self {
                git_output_calls: Vec::new(),
                git_status_calls: Vec::new(),
                host_status_calls: Vec::new(),
                git_outputs: VecDeque::from(git_outputs),
                git_statuses: VecDeque::from(git_statuses),
                host_statuses: VecDeque::from(host_statuses),
            }
        }
    }

    impl MutantsHost for FakeHost
    {
        fn run_git_output<G>(
            &mut self,
            args: &[OsString],
            _cwd: Option<&Path>,
            sanitized_git: G,
        ) -> Result<HostCommandOutcome, GateError>
        where
            G: Into<SanitizedGitFlag>,
        {
            let _sanitized_git = sanitized_git.into().0;
            self.git_output_calls.push(args.to_vec());
            self.git_outputs.pop_front().ok_or_else(|| {
                GateError::operational(format!(
                    "fake host has no git output result for {}",
                    render_call(args)
                ))
            })
        }

        fn run_git_status<G>(
            &mut self,
            args: &[OsString],
            _cwd: Option<&Path>,
            sanitized_git: G,
        ) -> Result<HostCommandOutcome, GateError>
        where
            G: Into<SanitizedGitFlag>,
        {
            let _sanitized_git = sanitized_git.into().0;
            self.git_status_calls.push(args.to_vec());
            self.git_statuses.pop_front().ok_or_else(|| {
                GateError::operational(format!(
                    "fake host has no git status result for {}",
                    render_call(args)
                ))
            })
        }

        fn run_host_status(
            &mut self,
            program: &OsStr,
            args: &[OsString],
            _cwd: Option<&Path>,
        ) -> Result<HostCommandOutcome, GateError>
        {
            self.host_status_calls
                .push((program.to_string_lossy().into_owned(), args.to_vec()));
            self.host_statuses.pop_front().ok_or_else(|| {
                GateError::operational(format!(
                    "fake host has no status result for {} {}",
                    program.to_string_lossy(),
                    render_call(args)
                ))
            })
        }

        fn path_exists(
            &mut self,
            path: &Path,
        ) -> Result<impl Into<PathExistsFlag>, GateError>
        {
            crate::support::HOST_FILESYSTEM
                .try_exists(path)
                .map(bool::from)
        }

        fn create_dir_all(
            &mut self,
            path: &Path,
        ) -> Result<(), GateError>
        {
            crate::support::HOST_FILESYSTEM.create_dir_all(path)
        }

        fn remove_dir_all(
            &mut self,
            path: &Path,
        ) -> Result<(), GateError>
        {
            crate::support::HOST_FILESYSTEM.remove_dir_all(path)
        }

        fn remove_file(
            &mut self,
            path: &Path,
        ) -> Result<(), GateError>
        {
            crate::support::HOST_FILESYSTEM.remove_file(path)
        }

        fn write_atomic<'semantic, B>(
            &mut self,
            path: &Path,
            bytes: B,
        ) -> Result<(), GateError>
        where
            B: Into<BytesBytes<'semantic>>,
        {
            let bytes = bytes.into().0;
            support::write_atomic(path, bytes)
        }

        fn publish_report(
            &mut self,
            report: &Path,
            workspace_root: &Path,
        ) -> Result<(), GateError>
        {
            report::publish_report(report, workspace_root)
        }
    }

    /// Scripted infrastructure probes for command routing tests.
    struct FakeInfrastructure
    {
        /// Whether `msb --version` succeeds.
        msb_available: bool,
        /// Whether the mutation snapshot exists.
        snapshot_exists: bool,
        /// Whether the cache image exists.
        cache_image_exists: bool,
    }

    impl FakeInfrastructure
    {
        /// Build an all-present infrastructure fixture.
        fn present() -> Self
        {
            Self {
                msb_available: true,
                snapshot_exists: true,
                cache_image_exists: true,
            }
        }
    }

    impl sandbox::SandboxInfrastructure for FakeInfrastructure
    {
        fn msb_available(&mut self) -> Result<impl Into<sandbox::MsbAvailableFlag>, GateError>
        {
            Ok(self.msb_available)
        }

        fn snapshot_exists(
            &mut self,
            _config: &sandbox::SandboxConfig,
        ) -> Result<impl Into<sandbox::SnapshotExistsFlag>, GateError>
        {
            Ok(self.snapshot_exists)
        }

        fn cache_image_exists(
            &mut self,
            _config: &sandbox::SandboxConfig,
        ) -> Result<impl Into<sandbox::CacheImageExistsFlag>, GateError>
        {
            Ok(self.cache_image_exists)
        }
    }

    /// Scripted msb adapter that records all planned calls.
    #[derive(Default)]
    struct FakeMsbAdapter
    {
        /// Captured msb calls.
        calls: Vec<Vec<OsString>>,
        /// Queued captured-output outcomes.
        output_results: VecDeque<sandbox::CommandOutcome>,
        /// Queued status outcomes.
        status_results: VecDeque<sandbox::CommandOutcome>,
    }

    impl FakeMsbAdapter
    {
        /// Build a fake msb adapter from output and status queues.
        fn new(
            output_results: Vec<sandbox::CommandOutcome>,
            status_results: Vec<sandbox::CommandOutcome>,
        ) -> Self
        {
            Self {
                calls: Vec::new(),
                output_results: VecDeque::from(output_results),
                status_results: VecDeque::from(status_results),
            }
        }

        /// Return rendered calls for assertions.
        fn rendered_calls(&self) -> Vec<String>
        {
            self.calls.iter().map(|call| render_call(call)).collect()
        }
    }

    impl sandbox::MsbAdapter for FakeMsbAdapter
    {
        fn run_output(
            &mut self,
            args: &[OsString],
        ) -> Result<sandbox::CommandOutcome, GateError>
        {
            self.calls.push(args.to_vec());
            self.output_results.pop_front().ok_or_else(|| {
                GateError::operational(format!(
                    "fake msb has no output result for {}",
                    render_call(args)
                ))
            })
        }

        fn run_status(
            &mut self,
            args: &[OsString],
        ) -> Result<sandbox::CommandOutcome, GateError>
        {
            self.calls.push(args.to_vec());
            self.status_results.pop_front().ok_or_else(|| {
                GateError::operational(format!(
                    "fake msb has no status result for {}",
                    render_call(args)
                ))
            })
        }
    }

    /// Return rendered command text.
    fn render_call(args: &[OsString]) -> String
    {
        let mut text = String::new();
        let mut first = true;
        for arg in args {
            if first {
                first = false;
            }
            else {
                text.push(' ');
            }
            text.push_str(arg.to_string_lossy().as_ref());
        }
        text
    }

    /// Snapshot routing formats the cache image and creates the reusable
    /// snapshot through only fake host and msb outcomes.
    #[test]
    fn snapshot_command_runs_builder_lifecycle_without_booting_vm() -> TestResult
    {
        let fixture = TestWorkspace::create("snapshot-route")?;
        let options = test_options(fixture.path(), &fixture.path().join("scratch"));
        let mut host = FakeHost::new(Vec::new(), vec![host_success()], vec![host_success()]);
        let mut infrastructure = FakeInfrastructure::present();
        let mut runner = FakeMsbAdapter::new(
            vec![
                msb_stdout("NAME KIND\n"),
                msb_stdout("Name: gandr-mutants-mkfs\nPath: /tmp/msb-cache.raw\n"),
                msb_stdout("NAME KIND\n"),
            ],
            vec![
                msb_success(),
                msb_success(),
                msb_success(),
                msb_success(),
                msb_success(),
                msb_success(),
                msb_success(),
                msb_success(),
                msb_success(),
                msb_success(),
            ],
        );
        let mut sink = report_sink();

        run_with_environment(
            &MutantsCommand::Snapshot,
            &options,
            &mut host,
            &mut infrastructure,
            &mut runner,
            &mut sink,
        )?;

        assert_eq!(
            1_usize,
            host.git_status_calls.len(),
            "snapshot should archive tracked source exactly once for cache warming"
        );
        assert!(
            host.host_status_calls.iter().any(|call| {
                call.0 == MISE_PROGRAM
                    && render_call(&call.1)
                        .contains("coreutils cp --reflink=auto /tmp/msb-cache.raw")
            }),
            "snapshot cache image should be preserved through the host coreutils copy plan"
        );
        let calls = runner.rendered_calls();
        assert!(
            calls
                .iter()
                .any(|call| call == "snapshot create --from gandr-mutants-build gandr-mutants-base --force --integrity"),
            "snapshot creation should publish the stopped builder with integrity checking"
        );
        assert!(
            calls
                .iter()
                .any(|call| call == "remove gandr-mutants-build --quiet"),
            "builder removal should be attempted before and after provisioning"
        );
        Ok(())
    }

    /// Build a successful host status outcome.
    fn host_success() -> HostCommandOutcome
    {
        HostCommandOutcome::new(true, Some(0_i32), String::new())
    }

    /// Push, merge, and scheduled no-Rust campaigns publish reports without
    /// issuing any msb command.
    #[test]
    fn diff_commands_publish_no_rust_reports_without_vm_boot() -> TestResult
    {
        let cases = [
            MutantsCommand::Push {
                range: range::PushRangePlan::last("HEAD")?,
            },
            MutantsCommand::Merge,
            MutantsCommand::Scheduled {
                from_ref: String::from("main"),
                to_ref: String::from("HEAD"),
            },
        ];
        let expected_modes = ["push", "merge", "scheduled"];
        for (command, expected_mode) in cases.iter().zip(expected_modes) {
            let fixture = TestWorkspace::create(expected_mode)?;
            let options = test_options(fixture.path(), &fixture.path().join("scratch"));
            let mut git_outputs = Vec::new();
            if matches!(command, MutantsCommand::Scheduled { .. }) {
                git_outputs.push(host_stdout("1111111111111111111111111111111111111111\n"));
                git_outputs.push(host_stdout("2222222222222222222222222222222222222222\n"));
                git_outputs.push(host_stdout("2222222222222222222222222222222222222222\n"));
            }
            git_outputs.push(host_stdout(
                "diff --git a/docs/readme.md b/docs/readme.md\n+++ b/docs/readme.md\n",
            ));
            let git_statuses = if matches!(command, MutantsCommand::Scheduled { .. }) {
                vec![host_success()]
            }
            else {
                Vec::new()
            };
            let mut host = FakeHost::new(git_outputs, git_statuses, Vec::new());
            let mut infrastructure = FakeInfrastructure::present();
            let mut runner = FakeMsbAdapter::default();
            let mut sink = report_sink();

            run_with_environment(
                command,
                &options,
                &mut host,
                &mut infrastructure,
                &mut runner,
                &mut sink,
            )?;

            assert!(
                runner.calls.is_empty(),
                "{expected_mode} no-Rust campaign should not issue msb calls"
            );
            let campaign = published_campaign(&options)?;
            assert!(
                campaign.contains(&format!("mode: '{expected_mode}'")),
                "published campaign summary should name the routed mode"
            );
            assert!(
                campaign.contains("report: 'no-rust-changes'"),
                "published campaign summary should classify the no-Rust fast path"
            );
            assert!(
                !crate::support::HOST_FILESYSTEM
                    .try_exists(&options.diff_file)
                    .map(bool::from)?,
                "successful no-Rust campaign should clean the temporary diff"
            );
            assert!(
                !crate::support::HOST_FILESYSTEM
                    .try_exists(&options.working_report)
                    .map(bool::from)?,
                "successful no-Rust campaign should clean the staging report"
            );
        }
        Ok(())
    }

    /// A scheduled campaign whose upper endpoint does not resolve to HEAD fails
    /// before any diff is materialized.
    #[test]
    fn scheduled_command_rejects_non_head_before_diff_materialization() -> TestResult
    {
        let fixture = TestWorkspace::create("scheduled-reject")?;
        let options = test_options(fixture.path(), &fixture.path().join("scratch"));
        let mut host = FakeHost::new(
            vec![
                host_stdout("1111111111111111111111111111111111111111\n"),
                host_stdout("2222222222222222222222222222222222222222\n"),
                host_stdout("3333333333333333333333333333333333333333\n"),
            ],
            vec![host_success()],
            Vec::new(),
        );
        let mut infrastructure = FakeInfrastructure::present();
        let mut runner = FakeMsbAdapter::default();
        let mut sink = report_sink();

        let error = run_with_environment(
            &MutantsCommand::Scheduled {
                from_ref: String::from("main"),
                to_ref: String::from("release"),
            },
            &options,
            &mut host,
            &mut infrastructure,
            &mut runner,
            &mut sink,
        )
        .err()
        .ok_or_else(|| GateError::operational("scheduled --to unexpectedly resolved"))?;

        assert!(
            error
                .to_string()
                .contains("must resolve to the current HEAD"),
            "scheduled rejection should preserve the range validation diagnostic"
        );
        assert_eq!(
            3_usize,
            host.git_output_calls.len(),
            "scheduled rejection should stop before requesting the campaign diff"
        );
        assert!(
            runner.calls.is_empty(),
            "scheduled rejection should not reach msb execution"
        );
        Ok(())
    }

    /// Build a successful host command outcome with retained stdout.
    fn host_stdout<'semantic, S>(stdout: S) -> HostCommandOutcome
    where
        S: Into<StdoutText<'semantic>>,
    {
        let stdout = stdout.into().0;
        HostCommandOutcome::new(true, Some(0_i32), String::from(stdout))
    }

    /// Sweep campaigns execute one contained VM plan, publish the report, and
    /// remove temporary staging data.
    #[test]
    fn sweep_command_runs_single_vm_and_cleans_after_publish() -> TestResult
    {
        let fixture = TestWorkspace::create("sweep-route")?;
        let options = test_options(fixture.path(), &fixture.path().join("scratch"));
        let mut host = FakeHost::new(Vec::new(), vec![host_success()], Vec::new());
        let mut infrastructure = FakeInfrastructure::present();
        let mut runner = FakeMsbAdapter::new(Vec::new(), vec![
            msb_success(),
            msb_success(),
            msb_success(),
            msb_success(),
            sandbox::CommandOutcome::failure(Some(1_i32)),
            msb_success(),
            msb_success(),
        ]);
        let mut sink = report_sink();

        run_with_environment(
            &MutantsCommand::Sweep,
            &options,
            &mut host,
            &mut infrastructure,
            &mut runner,
            &mut sink,
        )?;

        let calls = runner.rendered_calls();
        assert!(
            calls.iter().any(|call| call.starts_with("run --snapshot")),
            "sweep should boot exactly one sandbox"
        );
        assert!(
            !calls.iter().any(|call| call.contains("--diff")),
            "sweep guest command should not receive an in-diff argument"
        );
        let campaign = published_campaign(&options)?;
        assert!(
            campaign.contains("mode: 'sweep'")
                && campaign.contains("report: 'no-mutants-selected'"),
            "sweep should publish a no-mutants-selected summary when the guest report is absent"
        );
        assert!(
            !crate::support::HOST_FILESYSTEM
                .try_exists(&options.working_report)
                .map(bool::from)?,
            "successful sweep should remove the temporary report"
        );
        Ok(())
    }

    /// Read the published campaign summary text.
    fn published_campaign(options: &MutantsOptions) -> Result<String, GateError>
    {
        crate::support::HOST_FILESYSTEM
            .read_to_string(options.workspace_root.join("mutants.out/campaign.nuon"))
    }

    /// Mutation failures still publish and clean the report before returning
    /// the cargo-mutants failure diagnostic.
    #[test]
    fn contained_campaign_failure_publishes_before_returning_error() -> TestResult
    {
        let fixture = TestWorkspace::create("contained-failure")?;
        let options = test_options(fixture.path(), &fixture.path().join("scratch"));
        let mut host = FakeHost::new(Vec::new(), vec![host_success()], Vec::new());
        let mut runner = FakeMsbAdapter::new(Vec::new(), vec![
            msb_success(),
            msb_success(),
            msb_success(),
            msb_success(),
            sandbox::CommandOutcome::failure(Some(2_i32)),
            sandbox::CommandOutcome::failure(Some(1_i32)),
            msb_success(),
            msb_success(),
        ]);
        let mut sink = report_sink();

        let error = run_contained_campaign(&mut host, &mut runner, &mut sink, &ContainedCampaign {
            options: &options,
            mode: sandbox::CampaignMode::Merge,
            diff_path: Some(&options.diff_file),
            diff_source: "+++ b/src/lib.rs\n",
            archive_ref: SNAPSHOT_ARCHIVE_REF,
        })
        .err()
        .ok_or_else(|| GateError::operational("failed campaign unexpectedly succeeded"))?;

        assert!(
            error
                .to_string()
                .contains("cargo-mutants failed inside the microVM"),
            "contained campaign failure should return the stable cargo-mutants diagnostic"
        );
        let campaign = published_campaign(&options)?;
        assert!(
            campaign.contains("succeeded: false"),
            "failed contained campaign should publish the failed summary"
        );
        assert!(
            !crate::support::HOST_FILESYSTEM
                .try_exists(&options.working_report)
                .map(bool::from)?,
            "failed contained campaign should still clean staging after publication"
        );
        Ok(())
    }

    /// Missing msb fails snapshot/clean modes before any runner command.
    #[test]
    fn clean_command_requires_msb_before_runner_calls() -> TestResult
    {
        let fixture = TestWorkspace::create("clean-missing-msb")?;
        let options = test_options(fixture.path(), &fixture.path().join("scratch"));
        let mut host = FakeHost::default();
        let mut infrastructure = FakeInfrastructure {
            msb_available: false,
            snapshot_exists: true,
            cache_image_exists: true,
        };
        let mut runner = FakeMsbAdapter::default();
        let mut sink = report_sink();

        let error = run_with_environment(
            &MutantsCommand::Clean,
            &options,
            &mut host,
            &mut infrastructure,
            &mut runner,
            &mut sink,
        )
        .err()
        .ok_or_else(|| GateError::operational("clean unexpectedly ran without msb"))?;

        assert!(
            error.to_string().contains("`msb` (microsandbox) not found"),
            "missing msb diagnostic should match snapshot and clean requirements"
        );
        assert!(
            runner.calls.is_empty(),
            "missing msb should stop before cleanup inventory commands"
        );
        Ok(())
    }

    /// Existing cache images skip formatting after stale scratch cleanup.
    #[test]
    fn existing_cache_image_skips_formatting_after_scratch_probe() -> TestResult
    {
        let fixture = TestWorkspace::create("existing-cache")?;
        let cache_image = fixture.path().join("cache.img");
        crate::support::HOST_FILESYSTEM.write(&cache_image, b"cache image")?;
        let mut host = FakeHost::default();
        let mut runner = FakeMsbAdapter::new(vec![msb_stdout("NAME KIND\n")], Vec::new());

        format_cache_image(&mut host, &mut runner, &cache_image)?;

        assert_eq!(
            vec![String::from("volume list")],
            runner.rendered_calls(),
            "existing cache image should skip scratch creation, formatting, and host copy"
        );
        Ok(())
    }

    /// Campaign publication refuses to publish when the summary file was not
    /// preserved.
    #[test]
    fn publish_campaign_requires_preserved_summary() -> TestResult
    {
        let fixture = TestWorkspace::create("missing-campaign")?;
        let options = test_options(fixture.path(), &fixture.path().join("scratch"));
        let mut host = FakeHost::default();

        let error = publish_campaign_result(&mut host, &options)
            .err()
            .ok_or_else(|| GateError::operational("missing summary unexpectedly published"))?;

        assert!(
            error
                .to_string()
                .contains("campaign report was not preserved"),
            "publication should fail before touching mutants.out without campaign.nuon"
        );
        assert!(
            !crate::support::HOST_FILESYSTEM
                .try_exists(fixture.path().join("mutants.out"))
                .map(bool::from)?,
            "missing summary should not create a published report"
        );
        Ok(())
    }

    /// Volume backing parsing rejects inspect output without a nonempty Path
    /// field.
    #[test]
    fn volume_backing_path_requires_nonempty_path_field() -> TestResult
    {
        for stdout in ["Name: cache\n", "Path:   \n"] {
            let error = volume_backing_path(stdout).err().ok_or_else(|| {
                GateError::operational("missing backing path unexpectedly parsed")
            })?;
            assert!(
                error
                    .to_string()
                    .contains("could not resolve the volume backing image path"),
                "volume inspect parsing should require a nonempty Path field"
            );
        }
        Ok(())
    }

    /// Clean mode requires only msb availability and reaps the fake runner's
    /// deterministic owned-sandbox inventory.
    #[test]
    fn clean_command_reaps_deterministic_owned_inventory() -> TestResult
    {
        let fixture = TestWorkspace::create("clean-route")?;
        let options = test_options(fixture.path(), &fixture.path().join("scratch"));
        let mut host = FakeHost::default();
        let mut infrastructure = FakeInfrastructure {
            msb_available: true,
            snapshot_exists: false,
            cache_image_exists: false,
        };
        let mut runner = FakeMsbAdapter::new(
            vec![msb_stdout(
                "NAME STATE\ngandr-mutants-a Running\nforeign Running\ngandr-mutants-b Stopped\n",
            )],
            vec![msb_success(), msb_success(), msb_success(), msb_success()],
        );
        let mut sink = report_sink();

        run_with_environment(
            &MutantsCommand::Clean,
            &options,
            &mut host,
            &mut infrastructure,
            &mut runner,
            &mut sink,
        )?;

        assert_eq!(
            vec![
                String::from("list"),
                String::from("stop gandr-mutants-a --quiet"),
                String::from("remove gandr-mutants-a --quiet"),
                String::from("stop gandr-mutants-b --quiet"),
                String::from("remove gandr-mutants-b --quiet"),
            ],
            runner.rendered_calls(),
            "clean mode should process owned sandboxes in listing order only"
        );
        Ok(())
    }

    /// Return a support report sink.
    fn report_sink() -> sandbox::SupportCampaignReportSink
    {
        sandbox::SupportCampaignReportSink
    }

    /// Host Git helpers ignore injected Git repository override variables.
    #[test]
    fn host_git_helpers_ignore_injected_overrides() -> TestResult
    {
        run_child_with_git_overrides("child_host_git_helpers_ignore_injected_overrides")
    }

    /// Guest repository setup ignores injected Git repository override
    /// variables.
    #[test]
    fn guest_git_repo_setup_ignores_injected_overrides() -> TestResult
    {
        run_child_with_git_overrides("child_guest_git_repo_setup_ignores_injected_overrides")
    }

    /// Ignored child fixture that exercises host diff, archive, ref, and status
    /// helpers.
    #[test]
    #[ignore = "child-process fixture invoked explicitly by parent tests"]
    fn child_host_git_helpers_ignore_injected_overrides() -> TestResult
    {
        let fixture = TestWorkspace::create("host-git-sanitize")?;
        let workspace = fixture.path().join("repo");
        let scratch = fixture.path().join("scratch");
        create_committed_repo(&workspace)?;
        let options = test_options(&workspace, &scratch);

        let head = git_output_trimmed(
            &options,
            &[
                os("rev-parse"),
                os("--verify"),
                os("--quiet"),
                os("HEAD^{commit}"),
            ],
            "test HEAD resolution",
        )?;
        assert_eq!(
            40_usize,
            head.chars().count(),
            "sanitized ref resolution should read the fixture repository HEAD"
        );
        let head_is_ancestor = git_status_success(&options, &[
            os("merge-base"),
            os("--is-ancestor"),
            OsString::from(&head),
            OsString::from(&head),
        ])
        .map(|value| value.into().0)?;
        assert!(
            head_is_ancestor,
            "sanitized status checks should inspect the fixture repository"
        );

        let mut host = SupportMutantsHost;
        let diff_text = materialize_diff(&mut host, &options, &range::merge_diff_plan())?;
        assert!(
            diff_text.is_empty(),
            "clean fixture repository should materialize an empty sanitized diff"
        );
        write_source_archive(&mut host, &options, SNAPSHOT_ARCHIVE_REF)?;
        assert!(
            crate::support::HOST_FILESYSTEM
                .try_exists(&options.source_archive)
                .map(bool::from)?,
            "sanitized archive helper should write the requested archive path"
        );
        Ok(())
    }

    /// Ignored child fixture that creates the guest throwaway Git repository.
    #[test]
    #[ignore = "child-process fixture invoked explicitly by parent tests"]
    fn child_guest_git_repo_setup_ignores_injected_overrides() -> TestResult
    {
        let fixture = TestWorkspace::create("guest-git-sanitize")?;
        crate::support::HOST_FILESYSTEM
            .write(fixture.path().join("lib.rs"), b"pub fn marker() {}\n")?;
        let original_dir = crate::support::HOST_FILESYSTEM.current_dir()?;
        crate::support::HOST_FILESYSTEM.set_isolated_process_current_dir(fixture.path())?;
        let repo_result = ensure_guest_git_repo();
        let restore_result =
            crate::support::HOST_FILESYSTEM.set_isolated_process_current_dir(&original_dir);
        restore_result?;
        repo_result?;

        assert!(
            crate::support::HOST_FILESYSTEM
                .try_exists(fixture.path().join(".git"))
                .map(bool::from)?,
            "guest repo setup should create a local .git directory despite injected overrides"
        );
        Ok(())
    }

    /// Create a one-commit Git repository through the sanitized local helper.
    fn create_committed_repo(root: &Path) -> TestResult
    {
        crate::support::HOST_FILESYSTEM.create_dir_all(root)?;
        run_git_status_checked(
            &[os("init"), os("--quiet"), os("--initial-branch=main")],
            Some(root),
            "test git init failed",
        )?;
        crate::support::HOST_FILESYSTEM.write(root.join("lib.rs"), b"pub fn marker() {}\n")?;
        run_git_status_checked(&[os("add"), os("--all")], Some(root), "test git add failed")?;
        run_git_status_checked(
            &[os("commit"), os("--quiet"), os("-m"), os("initial")],
            Some(root),
            "test git commit failed",
        )?;
        Ok(())
    }

    /// Build options whose mutable artifacts live outside the fixture
    /// repository.
    fn test_options(
        workspace_root: &Path,
        scratch_root: &Path,
    ) -> MutantsOptions
    {
        MutantsOptions::new(
            workspace_root.to_path_buf(),
            scratch_root.join("cache.img"),
            scratch_root.join("src.tar"),
            scratch_root.join("diff.patch"),
            scratch_root.join("report"),
        )
    }

    /// Build a successful msb output outcome.
    fn msb_stdout<'semantic, S>(stdout: S) -> sandbox::CommandOutcome
    where
        S: Into<StdoutText<'semantic>>,
    {
        let stdout = stdout.into().0;
        sandbox::CommandOutcome::success_with_stdout(stdout)
    }

    /// Build a successful msb status outcome.
    fn msb_success() -> sandbox::CommandOutcome
    {
        sandbox::CommandOutcome::success()
    }

    /// Run one ignored child fixture with Git override variables injected.
    fn run_child_with_git_overrides<'semantic, N>(test_name: N) -> TestResult
    where
        N: Into<TestNameText<'semantic>>,
    {
        let test_name = test_name.into().0;
        let mut command = child_command_with_git_overrides(test_name)?;
        let output = command.output().map_err(|source| GateError::Io {
            path: PathBuf::from(test_name),
            source,
        })?;
        assert!(
            output.status.success(),
            "child fixture {test_name} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }

    /// Build a child test command whose environment would redirect unsanitized
    /// Git.
    fn child_command_with_git_overrides<'semantic, N>(test_name: N) -> Result<Command, GateError>
    where
        N: Into<TestNameText<'semantic>>,
    {
        let test_name = test_name.into().0;
        let child = crate::support::HOST_FILESYSTEM.current_exe()?;
        let mut command = Command::new(child);
        command.args(child_test_args(test_name));
        command.env("GIT_INDEX_FILE", "/tmp/gandr-mutants-poison/index");
        command.env("GIT_DIR", "/tmp/gandr-mutants-poison/git-dir");
        command.env("GIT_WORK_TREE", "/tmp/gandr-mutants-poison/work-tree");
        Ok(command)
    }

    /// Build libtest arguments that run one ignored child fixture exactly.
    fn child_test_args<'semantic, N>(test_name: N) -> Vec<OsString>
    where
        N: Into<TestNameText<'semantic>>,
    {
        let test_name = test_name.into().0;
        let mut exact_name = String::from("mutants::tests::");
        exact_name.push_str(test_name);
        vec![
            OsString::from("--ignored"),
            OsString::from("--exact"),
            OsString::from(exact_name),
            OsString::from("--nocapture"),
        ]
    }

    /// Remove a directory when it exists.
    fn remove_dir_if_exists(path: &Path) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM.remove_dir_if_exists(path)
    }

    /// Return a filesystem-safe nonce for test fixture names.
    fn test_nonce() -> Result<String, std::time::SystemTimeError>
    {
        let elapsed = SystemTime::now().duration_since(UNIX_EPOCH)?;
        Ok(format!("{}-{}", process::id(), elapsed.as_nanos()))
    }

    /// Temporary directory removed when a test finishes.
    #[repr(transparent)]
    struct TestWorkspace
    {
        /// Directory owned by this fixture.
        path: PathBuf,
    }

    impl TestWorkspace
    {
        /// Create a clean temporary workspace for one test.
        fn create<'semantic, N>(name: N) -> TestResult<Self>
        where
            N: Into<NameText<'semantic>>,
        {
            let name = name.into().0;
            let nonce = test_nonce()?;
            let path = env::temp_dir().join(format!("gandr-workflow-gates-mutants-{name}-{nonce}"));
            remove_dir_if_exists(&path)?;
            crate::support::HOST_FILESYSTEM.create_dir_all(&path)?;
            Ok(Self { path })
        }

        /// Return the owned workspace path.
        fn path(&self) -> &Path
        {
            &self.path
        }
    }

    impl Drop for TestWorkspace
    {
        fn drop(&mut self)
        {
            match crate::support::HOST_FILESYSTEM.remove_dir_all(&self.path) {
                | Ok(()) | Err(_) => {},
            }
        }
    }
}
