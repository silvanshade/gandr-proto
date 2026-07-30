//! Host-side planning and execution for mutation-test microVM campaigns.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::ffi::c_int;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use crate::GateError;
use crate::support;

crate::semantic_copy!(pub struct SucceededFlag(bool));
crate::semantic_copy!(pub struct SuccessFlag(bool));
crate::semantic_str!(pub struct DiffSourceText);
crate::semantic_str!(pub struct ScratchNameText);
crate::semantic_str!(pub struct ContextText);
crate::semantic_str!(pub struct ValueText);
crate::semantic_str!(pub struct ExtensionText);
crate::semantic_str!(pub struct GuestPathText);
crate::semantic_str!(pub struct ListStdoutText);
crate::semantic_copy!(pub struct MsbAvailableFlag(bool));
crate::semantic_copy!(pub struct SnapshotExistsFlag(bool));
crate::semantic_copy!(pub struct CacheImageExistsFlag(bool));
crate::semantic_str!(pub struct NeedleText);
crate::semantic_str!(pub struct FlagText);
crate::semantic_copy!(pub struct ValueFlag(bool));
crate::semantic_str!(pub struct AsStrText);
crate::semantic_copy!(pub struct NeedsDiffFlag(bool));
crate::semantic_str!(pub struct GuestTimeoutText);
crate::semantic_str!(pub struct SandboxTimeoutText);
crate::semantic_copy!(pub struct ForbiddenHostMountFlag(bool));
crate::semantic_copy!(pub struct SuccessStatusFlag(bool));
crate::semantic_copy!(pub struct OptionalExitCode(Option<c_int>));
crate::semantic_str!(pub struct StdoutText);
crate::semantic_copy!(pub struct ShouldSkipVmFlag(bool));
crate::semantic_copy!(pub struct DiffTouchesRustFlag(bool));
crate::semantic_copy!(pub struct RemoveCacheScratchFlag(bool));
crate::semantic_copy!(pub struct PathHasExtensionFlag(bool));
crate::semantic_str!(pub struct FormatCacheScriptText);
crate::semantic_str!(pub struct NuonBoolText);
crate::semantic_copy!(pub struct VolumeListingContainsFlag(bool));
crate::semantic_copy!(pub struct ArgSequenceFlag(bool));
crate::semantic_copy!(pub struct ArgFlag(bool));

/// Microsandbox executable name.
const MSB_PROGRAM: &str = "msb";
/// Reusable mutation-test snapshot name.
const SNAPSHOT_NAME: &str = "gandr-mutants-base";
/// Base image used to provision the reusable mutation-test snapshot.
const BASE_IMAGE: &str = "ubuntu:24.04";
/// Raw btrfs cache image size used by the snapshot formatter.
const CACHE_SIZE: &str = "64G";
/// Name of the temporary snapshot builder sandbox.
const SNAPSHOT_BUILDER_NAME: &str = "gandr-mutants-build";
/// Snapshot builder lifetime cap.
const SNAPSHOT_BUILDER_TIMEOUT: &str = "90m";
/// Temporary cache formatter lifetime cap.
const CACHE_FORMAT_TIMEOUT: &str = "15m";
/// Stable Rust toolchain used while provisioning the snapshot.
const RUST_TOOLCHAIN: &str = "1.97.1";
/// Mise tool allow-list without the guest environment prefix.
const GUEST_MISE_TOOLS: &str = "cargo:cargo-binstall,cargo:cargo-mutants,cargo:cargo-nextest";
/// Prefix owned by this driver for ephemeral sandboxes.
pub(super) const SANDBOX_PREFIX: &str = "gandr-mutants-";
/// Guest work directory on the btrfs cache image.
const GUEST_WORK_DIR: &str = "/cache/work";
/// Guest repository directory on the btrfs cache image.
const GUEST_REPO_DIR: &str = "/cache/work/repo";
/// Guest path for the copied source archive.
const GUEST_SOURCE_ARCHIVE: &str = "/cache/work/src.tar";
/// Guest path for the copied unified diff.
const GUEST_DIFF: &str = "/cache/work/changes.diff";
/// Guest path for the cargo-mutants report directory.
const GUEST_REPORT_DIR: &str = "/cache/work/repo/mutants.out";
/// Cargo home inside the mounted cache disk.
const GUEST_CARGO_HOME_ENV: &str = "CARGO_HOME=/cache/cargo";
/// Rustup home inside the snapshot.
const GUEST_RUSTUP_HOME_ENV: &str = "RUSTUP_HOME=/usr/local/rustup";
/// Stable Rust toolchain used for mutation testing.
const GUEST_RUST_TOOLCHAIN_ENV: &str = "RUSTUP_TOOLCHAIN=1.97.1";
/// Guest temporary directory on btrfs.
const GUEST_TMPDIR_ENV: &str = "TMPDIR=/cache/tmp";
/// Guest path with Rustup, mise, and system tools.
const GUEST_PATH_ENV: &str =
    "PATH=/cache/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";
/// Mise tool allow-list needed by cargo-mutants.
const GUEST_MISE_TOOLS_ENV: &str =
    "MISE_ENABLE_TOOLS=cargo:cargo-binstall,cargo:cargo-mutants,cargo:cargo-nextest";
/// Trusted mise configuration root inside the guest.
const GUEST_MISE_TRUST_ENV: &str = "MISE_TRUSTED_CONFIG_PATHS=/cache/work/repo";
/// In-diff guest command timeout.
const GUEST_IN_DIFF_TIMEOUT: &str = "45m";
/// Sweep guest command timeout.
const GUEST_SWEEP_TIMEOUT: &str = "8h";
/// In-diff sandbox lifetime cap.
const SANDBOX_IN_DIFF_TIMEOUT: &str = "55m";
/// Sweep sandbox lifetime cap.
const SANDBOX_SWEEP_TIMEOUT: &str = "8h15m";
/// Exit code for successful report probe.
const EXIT_SUCCESS_CODE: c_int = 0x0;
/// Exit code from `test -d` when no cargo-mutants report exists.
const EXIT_REPORT_ABSENT_CODE: c_int = 0x1;
/// Number of header lines in `msb list` output.
const MSB_LIST_HEADER_LINES: usize = 0x1;

/// Mutation campaign mode selected by the host driver.
///
/// # Contract
/// - ensures: every mode maps to fixed guest and sandbox timeout caps.
/// - provides: the single source of truth for mode labels and timeout caps used
///   in generated `msb` plans; the unified Rust guest facade fixes its own
///   sequential worker count.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — timeout and Rust-guest route tests distinguish
///   in-diff campaigns from sweep campaigns and reject a forwarded worker flag.
/// - witness: `mutants::sandbox::tests::timeout_caps_and_sequential_jobs_are_planned`
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CampaignMode
{
    /// Pre-push changed-line mutation campaign.
    Push,
    /// Pre-merge changed-line mutation campaign.
    Merge,
    /// Scheduled changed-line mutation campaign.
    Scheduled,
    /// Whole-workspace sweep campaign.
    Sweep,
}

impl CampaignMode
{
    /// Return the stable mode label used in campaign reports and sandbox names.
    #[inline]
    #[must_use]
    pub(super) fn as_str(self) -> impl Into<AsStrText<'static>>
    {
        match self {
            | Self::Push => return "push",
            | Self::Merge => return "merge",
            | Self::Scheduled => return "scheduled",
            | Self::Sweep => return "sweep",
        }
    }

    /// Return whether this mode needs a copied unified diff.
    #[inline]
    #[must_use]
    pub(super) fn needs_diff(self) -> impl Into<NeedsDiffFlag>
    {
        match self {
            | Self::Push | Self::Merge | Self::Scheduled => return true,
            | Self::Sweep => return false,
        }
    }

    /// Return the guest command timeout cap.
    #[inline]
    #[must_use]
    pub(super) fn guest_timeout(self) -> impl Into<GuestTimeoutText<'static>>
    {
        match self {
            | Self::Push | Self::Merge | Self::Scheduled => return GUEST_IN_DIFF_TIMEOUT,
            | Self::Sweep => return GUEST_SWEEP_TIMEOUT,
        }
    }

    /// Return the sandbox lifetime timeout cap.
    #[inline]
    #[must_use]
    pub(super) fn sandbox_timeout(self) -> impl Into<SandboxTimeoutText<'static>>
    {
        match self {
            | Self::Push | Self::Merge | Self::Scheduled => return SANDBOX_IN_DIFF_TIMEOUT,
            | Self::Sweep => return SANDBOX_SWEEP_TIMEOUT,
        }
    }
}

/// Validated microsandbox name owned by this driver.
///
/// # Contract
/// - requires: the value starts with [`SANDBOX_PREFIX`] and contains no path
///   separators.
/// - ensures: cleanup and teardown commands cannot target names outside the
///   exact prefix namespace owned by this module.
/// - provides: the sandbox name as a host-side `msb` argument.
/// - fails: returns [`GateError`] when the value is empty after the prefix or
///   contains unsupported separators.
/// - panics: none.
///
/// # Errors
/// Returns an operational error when the sandbox name is outside the owned
/// prefix namespace.
///
/// # Adequacy
/// - hypothesis: L3 only — prefix cleanup tests kill mutants that reap
///   non-prefixed sandboxes or accept path-shaped names.
/// - witness: `mutants::sandbox::tests::prefix_cleanup_only_reaps_owned_sandboxes`
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub(super) struct SandboxName
{
    /// Validated sandbox name text.
    value: String,
}

impl SandboxName
{
    /// Validate and store a sandbox name.
    ///
    /// # Errors
    /// Returns an operational error when `value` does not use the owned prefix,
    /// is missing its unique suffix, or contains a path separator.
    #[inline]
    pub(super) fn new<'semantic, Value>(value: Value) -> Result<Self, GateError>
    where
        Value: Into<ValueText<'semantic>>,
    {
        let value = value.into().0;
        if !value.starts_with(SANDBOX_PREFIX) {
            return Err(GateError::operational(format!(
                "mutants-vm: sandbox name `{value}` does not use owned prefix `{SANDBOX_PREFIX}`"
            )));
        }
        if value == SANDBOX_PREFIX {
            return Err(GateError::operational(
                "mutants-vm: sandbox name is missing its unique suffix",
            ));
        }
        if value.contains('/') || value.contains('\\') {
            return Err(GateError::operational(format!(
                "mutants-vm: sandbox name `{value}` must not contain path separators"
            )));
        }
        Ok(Self {
            value: String::from(value),
        })
    }

    /// Return the name as UTF-8 text.
    #[inline]
    #[must_use]
    pub(super) fn as_str(&self) -> impl Into<AsStrText<'_>>
    {
        return &self.value;
    }

    /// Return the name as an operating-system argument.
    #[inline]
    #[must_use]
    pub(super) fn as_os_str(&self) -> &OsStr
    {
        return OsStr::new(&self.value);
    }
}

/// Static host-side sandbox configuration.
///
/// # Contract
/// - requires: `cache_image` names the btrfs raw image that may be attached as
///   a block device.
/// - ensures: the derived mount argument always uses `:/cache:fstype=btrfs` and
///   never creates a host directory or file passthrough mount.
/// - provides: snapshot and cache arguments for pure `msb` plan builders.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — forbidden-mount plan tests distinguish the block
///   device mount from host passthrough flags.
/// - witness: `mutants::sandbox::tests::sandbox_boot_plan_has_no_forbidden_host_mounts`
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SandboxConfig
{
    /// Snapshot name used to boot mutation sandboxes.
    snapshot: OsString,
    /// Host path to the btrfs cache image.
    cache_image: PathBuf,
    /// Preformatted cache mount argument for `msb run`.
    cache_mount: OsString,
}

impl SandboxConfig
{
    /// Build a sandbox configuration from a host cache image path.
    #[inline]
    #[must_use]
    pub(super) fn new(cache_image: &Path) -> Self
    {
        Self {
            snapshot: OsString::from(SNAPSHOT_NAME),
            cache_image: cache_image.to_path_buf(),
            cache_mount: cache_mount_argument(cache_image),
        }
    }

    /// Return the configured snapshot name.
    #[inline]
    #[must_use]
    pub(super) fn snapshot(&self) -> &OsStr
    {
        return &self.snapshot;
    }

    /// Return the cache image path.
    #[inline]
    #[must_use]
    pub(super) fn cache_image(&self) -> &Path
    {
        return &self.cache_image;
    }

    /// Return the `--mount-disk` argument.
    #[inline]
    #[must_use]
    pub(super) fn cache_mount(&self) -> &OsStr
    {
        return &self.cache_mount;
    }
}

/// Pure `msb` argument vector.
///
/// # Contract
/// - ensures: contains arguments only; the program is always [`MSB_PROGRAM`].
/// - provides: a testable command plan separated from command execution.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — pure plan tests observe argv without invoking `msb`,
///   killing mutants that add host mounts, drop timeout caps, or reintroduce
///   sweep parallelism.
/// - witness: `mutants::sandbox::tests::sandbox_boot_plan_has_no_forbidden_host_mounts`
/// - witness: `mutants::sandbox::tests::timeout_caps_and_sequential_jobs_are_planned`
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub(super) struct MsbPlan
{
    /// Ordered `msb` arguments.
    args: Vec<OsString>,
}

impl MsbPlan
{
    /// Store an ordered `msb` argument vector.
    #[inline]
    #[must_use]
    pub(super) fn new(args: Vec<OsString>) -> Self
    {
        Self { args }
    }

    /// Return the planned arguments.
    #[inline]
    #[must_use]
    pub(super) fn args(&self) -> &[OsString]
    {
        return &self.args;
    }

    /// Return whether this plan contains a forbidden host passthrough mount
    /// flag.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(super) fn contains_forbidden_host_mount(&self) -> impl Into<ForbiddenHostMountFlag>
    {
        for argument in &self.args {
            if argument == OsStr::new("-v")
                || argument == OsStr::new("--mount-dir")
                || argument == OsStr::new("--mount-file")
                || argument == OsStr::new("--mount-named")
            {
                return true;
            }
        }
        false
    }
}

/// Host paths and mode for one mutation campaign.
///
/// # Contract
/// - requires: `source_archive` points at a tracked-file tar archive and `diff`
///   is present exactly for diff-scoped modes.
/// - ensures: the request names a single sandbox and a single report directory.
/// - provides: immutable inputs for constructing a [`CampaignExecutionPlan`].
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — execution and fast-path tests distinguish the single
///   sandbox path from no-Rust no-VM publication.
/// - witness: `mutants::sandbox::tests::teardown_error_takes_precedence_over_payload_error`
/// - witness: `mutants::sandbox::tests::non_rust_diff_skips_vm_and_writes_report`
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CampaignRequest<'request>
{
    /// Campaign mode.
    mode: CampaignMode,
    /// Sandbox name.
    sandbox_name: &'request SandboxName,
    /// Host source archive path.
    source_archive: &'request Path,
    /// Host diff path for diff-scoped modes.
    diff: Option<&'request Path>,
    /// Host report directory path.
    report_dir: &'request Path,
}

impl<'request> CampaignRequest<'request>
{
    /// Build a campaign request.
    #[inline]
    #[must_use]
    pub(crate) fn new(
        mode: CampaignMode,
        sandbox_name: &'request SandboxName,
        source_archive: &'request Path,
        diff: Option<&'request Path>,
        report_dir: &'request Path,
    ) -> Self
    {
        Self {
            mode,
            sandbox_name,
            source_archive,
            diff,
            report_dir,
        }
    }

    /// Return the campaign mode.
    #[inline]
    #[must_use]
    pub(crate) fn mode(&self) -> CampaignMode
    {
        return self.mode;
    }

    /// Return the host report directory.
    #[inline]
    #[must_use]
    pub(crate) fn report_dir(&self) -> &Path
    {
        return self.report_dir;
    }
}

/// Complete pure plan for a sandboxed campaign.
///
/// # Contract
/// - requires: diff-scoped modes carry a diff path.
/// - ensures: the plan contains one boot command, at most one diff copy, one
///   guest execution, one probe/copy-out path, and one stop/remove teardown
///   pair.
/// - provides: all `msb` argv needed by the side-effect adapter.
/// - fails: returns [`GateError`] when a diff-scoped mode lacks a diff path.
/// - panics: none.
///
/// # Errors
/// Returns an operational error for an incomplete diff-scoped request.
///
/// # Adequacy
/// - hypothesis: L3 only — plan tests observe the forbidden-mount, timeout, and
///   sequential-job projections without requiring a VM.
/// - witness: `mutants::sandbox::tests::sandbox_boot_plan_has_no_forbidden_host_mounts`
/// - witness: `mutants::sandbox::tests::timeout_caps_and_sequential_jobs_are_planned`
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CampaignExecutionPlan
{
    /// Campaign mode.
    mode: CampaignMode,
    /// Sandbox name.
    sandbox_name: SandboxName,
    /// Host report directory path.
    report_dir: PathBuf,
    /// Sandbox boot plan.
    boot: MsbPlan,
    /// Source archive copy-in plan.
    copy_source: MsbPlan,
    /// Optional diff copy-in plan.
    copy_diff: Option<MsbPlan>,
    /// Guest extraction plan.
    extract: MsbPlan,
    /// Guest cargo-mutants execution plan.
    guest: MsbPlan,
    /// Guest report probe plan.
    probe_report: MsbPlan,
    /// Guest report copy-out plan.
    copy_report: MsbPlan,
    /// Sandbox stop plan.
    stop: MsbPlan,
    /// Sandbox removal plan.
    remove: MsbPlan,
}

impl CampaignExecutionPlan
{
    /// Build a complete `msb` plan for `request`.
    ///
    /// # Errors
    /// Returns an operational error when a diff-scoped request omits its diff
    /// path.
    #[inline]
    pub(crate) fn new(
        config: &SandboxConfig,
        request: &CampaignRequest<'_>,
    ) -> Result<Self, GateError>
    {
        let copy_diff = request
            .mode
            .needs_diff()
            .into()
            .0
            .then(|| {
                let diff_path = request.diff.ok_or_else(|| {
                    GateError::operational(format!(
                        "mutants-vm: {} campaign requires a diff path",
                        crate::semantic_value::<AsStrText<'_>, _>(request.mode.as_str()).0
                    ))
                })?;
                Ok(copy_diff_plan(request.sandbox_name, diff_path))
            })
            .transpose()?;
        Ok(Self {
            mode: request.mode,
            sandbox_name: request.sandbox_name.clone(),
            report_dir: request.report_dir.to_path_buf(),
            boot: sandbox_boot_plan(config, request.sandbox_name, request.mode),
            copy_source: copy_source_plan(request.sandbox_name, request.source_archive),
            copy_diff,
            extract: extract_source_plan(request.sandbox_name),
            guest: guest_execution_plan(request.sandbox_name, request.mode),
            probe_report: probe_report_plan(request.sandbox_name),
            copy_report: copy_report_plan(request.sandbox_name, request.report_dir),
            stop: stop_plan(request.sandbox_name),
            remove: remove_plan(request.sandbox_name),
        })
    }

    /// Return the sandbox boot plan.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) fn boot(&self) -> &MsbPlan
    {
        return &self.boot;
    }

    /// Return the guest execution plan.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(super) fn guest(&self) -> &MsbPlan
    {
        return &self.guest;
    }
}

/// Report kind preserved from a campaign.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CampaignReportKind
{
    /// A cargo-mutants `mutants.out` directory was copied out of the guest.
    CargoMutants,
    /// The guest ran but selected no mutants and produced no report directory.
    NoMutantsSelected,
    /// The host diff touched no Rust files, so no VM was booted.
    NoRustChanges,
}

impl CampaignReportKind
{
    /// Return the stable report-kind label used in `campaign.nuon`.
    #[inline]
    #[must_use]
    pub(super) fn as_str(self) -> impl Into<AsStrText<'static>>
    {
        match self {
            | Self::CargoMutants => return "cargo-mutants",
            | Self::NoMutantsSelected => return "no-mutants-selected",
            | Self::NoRustChanges => return "no-rust-changes",
        }
    }
}

/// Campaign summary written to `campaign.nuon`.
///
/// # Contract
/// - ensures: schema, mode, success, and report kind are represented exactly.
/// - provides: a rollback-safe artifact for workflow upload and later
///   publication.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — no-Rust and guest-report tests distinguish every
///   retained report kind and success flag.
/// - witness: `mutants::sandbox::tests::non_rust_diff_skips_vm_and_writes_report`
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CampaignSummary
{
    /// Campaign mode.
    mode: CampaignMode,
    /// Whether cargo-mutants succeeded.
    succeeded: bool,
    /// Preserved report kind.
    report: CampaignReportKind,
}

impl CampaignSummary
{
    /// Build a campaign summary.
    #[inline]
    #[must_use]
    pub(super) fn new<Succeeded>(
        mode: CampaignMode,
        succeeded: Succeeded,
        report: CampaignReportKind,
    ) -> Self
    where
        Succeeded: Into<SucceededFlag>,
    {
        let succeeded = succeeded.into().0;
        Self {
            mode,
            succeeded,
            report,
        }
    }

    /// Return whether the campaign succeeded.
    #[inline]
    #[must_use]
    pub(super) fn succeeded(&self) -> impl Into<SucceededFlag>
    {
        return self.succeeded;
    }

    /// Return the report kind.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(super) fn report(&self) -> CampaignReportKind
    {
        return self.report;
    }

    /// Render the summary as compact NUON.
    #[inline]
    #[must_use]
    pub(super) fn to_nuon(&self) -> String
    {
        format!(
            "{{schema: 1, mode: '{}', succeeded: {}, report: '{}'}}\n",
            crate::semantic_value::<AsStrText<'_>, _>(self.mode.as_str()).0,
            nuon_bool(self.succeeded).into().0,
            crate::semantic_value::<AsStrText<'_>, _>(self.report.as_str()).0
        )
    }
}

/// Command outcome normalized for `msb` adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CommandOutcome
{
    /// Whether the command exited successfully.
    success: bool,
    /// Process exit code, absent if the process was terminated by signal.
    code: Option<c_int>,
    /// Lossy UTF-8 stdout retained only for commands whose stdout is parsed.
    stdout: String,
}

impl CommandOutcome
{
    /// Build a successful command outcome with empty output.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(super) fn success() -> Self
    {
        Self {
            success: true,
            code: Some(EXIT_SUCCESS_CODE),
            stdout: String::new(),
        }
    }

    /// Build a successful command outcome with stdout.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) fn success_with_stdout<Stdout>(stdout: Stdout) -> Self
    where
        Stdout: Into<String>,
    {
        Self {
            success: true,
            code: Some(EXIT_SUCCESS_CODE),
            stdout: stdout.into(),
        }
    }

    /// Build a failed command outcome with no captured output.
    #[cfg(test)]
    #[inline]
    #[must_use]
    pub(crate) fn failure<Code>(code: Code) -> Self
    where
        Code: Into<OptionalExitCode>,
    {
        let code = code.into().0;
        Self {
            success: false,
            code,
            stdout: String::new(),
        }
    }

    /// Return whether the command succeeded.
    #[inline]
    #[must_use]
    pub(crate) fn success_status(&self) -> impl Into<SuccessStatusFlag>
    {
        return self.success;
    }

    /// Return the exit code.
    #[inline]
    #[must_use]
    pub(crate) fn code(&self) -> impl Into<OptionalExitCode>
    {
        return self.code;
    }

    /// Return stdout retained for semantic parsing.
    #[inline]
    #[must_use]
    pub(super) fn stdout(&self) -> impl Into<StdoutText<'_>>
    {
        return &self.stdout;
    }
}

/// Side-effect adapter for `msb` command execution.
pub(super) trait MsbAdapter
{
    /// Run an `msb` command whose stdout is semantically parsed.
    ///
    /// # Contract
    /// - extension: stdout is streamed live and retained through the bounded
    ///   support capture path; stderr inherits the parent stream and is not
    ///   stored.
    /// - intension: use only for list/inspect commands whose stdout is
    ///   consumed.
    ///
    /// # Errors
    /// Returns an operational or I/O error when the host cannot start, stream,
    /// or wait on `msb`, or when stdout exceeds the support capture cap.
    fn run_output(
        &mut self,
        args: &[OsString],
    ) -> Result<CommandOutcome, GateError>;

    /// Run an `msb` command where output should stream through the host.
    ///
    /// # Contract
    /// - extension: returns only success/code; stdout and stderr inherit the
    ///   parent and are not retained.
    /// - intension: use for boot, copy, exec, stop, and remove commands whose
    ///   stdout is not parsed by this driver.
    ///
    /// # Errors
    /// Returns an operational or I/O error when the host cannot start or wait
    /// on `msb`.
    fn run_status(
        &mut self,
        args: &[OsString],
    ) -> Result<CommandOutcome, GateError>;
}

/// Support-backed `msb` adapter used by production entry points.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct SupportMsbAdapter;

impl MsbAdapter for SupportMsbAdapter
{
    #[inline]
    fn run_output(
        &mut self,
        args: &[OsString],
    ) -> Result<CommandOutcome, GateError>
    {
        let output = support::run_output(OsStr::new(MSB_PROGRAM), args, None, false)?;
        Ok(CommandOutcome {
            success: crate::semantic_value::<crate::support::SuccessFlag, _>(output.success()).0,
            code: crate::semantic_value::<crate::support::OptionalCodeCode, _>(output.code()).0,
            stdout: output.stdout_lossy().into_owned(),
        })
    }

    #[inline]
    fn run_status(
        &mut self,
        args: &[OsString],
    ) -> Result<CommandOutcome, GateError>
    {
        let status = support::run_status(OsStr::new(MSB_PROGRAM), args, None, false)?;
        Ok(CommandOutcome {
            success: status.success(),
            code: status.code(),
            stdout: String::new(),
        })
    }
}

/// Side-effect adapter for host report directory creation and campaign files.
pub(super) trait CampaignReportSink
{
    /// Create an empty host report directory.
    ///
    /// # Errors
    /// Returns the filesystem error when the report directory cannot be
    /// created.
    fn create_empty_report(
        &mut self,
        report_dir: &Path,
    ) -> Result<(), GateError>;

    /// Write `campaign.nuon` under the host report directory.
    ///
    /// # Errors
    /// Returns the filesystem error when the report directory cannot be created
    /// or the campaign file cannot be written atomically.
    fn write_campaign(
        &mut self,
        report_dir: &Path,
        summary: &CampaignSummary,
    ) -> Result<(), GateError>;
}

/// Support-backed report sink used by production entry points.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct SupportCampaignReportSink;

impl CampaignReportSink for SupportCampaignReportSink
{
    #[inline]
    fn create_empty_report(
        &mut self,
        report_dir: &Path,
    ) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM.create_dir_all(report_dir)
    }

    #[inline]
    fn write_campaign(
        &mut self,
        report_dir: &Path,
        summary: &CampaignSummary,
    ) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM.create_dir_all(report_dir)?;
        support::write_atomic(
            &report_dir.join("campaign.nuon"),
            summary.to_nuon().as_bytes(),
        )
    }
}

/// Side-effect adapter for infrastructure probes.
pub(super) trait SandboxInfrastructure
{
    /// Return whether the `msb` executable is available.
    ///
    /// # Errors
    /// Returns an adapter failure when availability cannot be determined.
    fn msb_available(&mut self) -> Result<impl Into<MsbAvailableFlag>, GateError>;

    /// Return whether the configured snapshot exists.
    ///
    /// # Errors
    /// Returns an adapter failure when snapshot inspection cannot run.
    fn snapshot_exists(
        &mut self,
        config: &SandboxConfig,
    ) -> Result<impl Into<SnapshotExistsFlag>, GateError>;

    /// Return whether the configured cache image exists.
    ///
    /// # Errors
    /// Returns an adapter failure when the path cannot be queried.
    fn cache_image_exists(
        &mut self,
        config: &SandboxConfig,
    ) -> Result<impl Into<CacheImageExistsFlag>, GateError>;
}

/// Support-backed infrastructure probe.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct SupportSandboxInfrastructure;

impl SandboxInfrastructure for SupportSandboxInfrastructure
{
    #[inline]
    fn msb_available(&mut self) -> Result<impl Into<MsbAvailableFlag>, GateError>
    {
        let args = vec![OsString::from("--version")];
        match support::run_status(OsStr::new(MSB_PROGRAM), &args, None, false) {
            | Ok(status) => return Ok(status.success()),
            | Err(_source) => return Ok(false),
        }
    }

    #[inline]
    fn snapshot_exists(
        &mut self,
        config: &SandboxConfig,
    ) -> Result<impl Into<SnapshotExistsFlag>, GateError>
    {
        let args = vec![
            OsString::from("snapshot"),
            OsString::from("inspect"),
            config.snapshot().to_os_string(),
        ];
        let status = support::run_status(OsStr::new(MSB_PROGRAM), &args, None, false)?;
        Ok(status.success())
    }

    #[inline]
    fn cache_image_exists(
        &mut self,
        config: &SandboxConfig,
    ) -> Result<impl Into<CacheImageExistsFlag>, GateError>
    {
        crate::support::HOST_FILESYSTEM
            .try_exists(config.cache_image())
            .map(bool::from)
    }
}

/// Fail closed when required microVM infrastructure is missing.
///
/// # Contract
/// - requires: `infrastructure` checks the current host and `config` points at
///   the expected snapshot/cache image.
/// - ensures: missing `msb`, snapshot, or cache image is reported as a hard
///   operational failure.
/// - provides: the same fail-closed guard as the Nushell `require-infra` path.
/// - fails: returns [`GateError`] for missing infrastructure or probe failures.
/// - panics: none.
///
/// # Errors
/// Returns operational errors for missing `msb`, snapshot, and cache image, or
/// propagates probe errors from `infrastructure`.
///
/// # Adequacy
/// - hypothesis: L3 only — callers can inject false probes for each required
///   component and observe distinct hard failures instead of silent skips.
/// - witness: `mutants::sandbox::tests::non_rust_diff_skips_vm_and_writes_report`
#[inline]
pub(super) fn require_infra<Infrastructure>(
    infrastructure: &mut Infrastructure,
    config: &SandboxConfig,
) -> Result<(), GateError>
where
    Infrastructure: SandboxInfrastructure,
{
    if !infrastructure.msb_available().map(|value| value.into().0)? {
        return Err(GateError::operational(
            "mutants-vm: `msb` (microsandbox) not found. It is pinned in mise.toml; run `mise install` to provision it.",
        ));
    }
    if !infrastructure
        .snapshot_exists(config)
        .map(|value| value.into().0)?
    {
        return Err(GateError::operational(format!(
            "mutants-vm: microVM snapshot `{}` not found. Build it once with `mise run mutants:snapshot`.",
            config.snapshot().to_string_lossy()
        )));
    }
    if !infrastructure
        .cache_image_exists(config)
        .map(|value| value.into().0)?
    {
        return Err(GateError::operational(format!(
            "mutants-vm: btrfs cache image `{}` not found. Build it once with `mise run mutants:snapshot`.",
            config.cache_image().display()
        )));
    }
    Ok(())
}

/// Execute a campaign request or write a no-Rust report without booting a VM.
///
/// # Contract
/// - requires: `request` carries the source archive, optional diff path, and
///   report directory for one campaign.
/// - ensures: no VM command is run when `diff_source` has no Rust additions for
///   diff-scoped modes; otherwise exactly one sandbox plan is executed.
/// - provides: a side-effecting adapter boundary over pure `msb` plans.
/// - fails: returns infrastructure, transport, copy-in, guest, copy-out,
///   report, or teardown errors as [`GateError`] values.
/// - panics: none.
///
/// # Errors
/// Returns command adapter failures, non-zero infrastructure command outcomes,
/// report sink failures, or teardown failures.
///
/// # Adequacy
/// - hypothesis: L3 only — fast-path and teardown-precedence tests distinguish
///   skipped campaigns from single-sandbox execution and teardown error
///   priority.
/// - witness: `mutants::sandbox::tests::non_rust_diff_skips_vm_and_writes_report`
/// - witness: `mutants::sandbox::tests::teardown_error_takes_precedence_over_payload_error`
#[inline]
pub(super) fn execute_campaign_request<'semantic, Runner, Sink, DiffSource>(
    runner: &mut Runner,
    sink: &mut Sink,
    config: &SandboxConfig,
    request: &CampaignRequest<'_>,
    diff_source: DiffSource,
) -> Result<CampaignSummary, GateError>
where
    Runner: MsbAdapter,
    Sink: CampaignReportSink,
    DiffSource: Into<DiffSourceText<'semantic>>,
{
    let diff_source = diff_source.into().0;
    if should_skip_vm(request.mode(), diff_source).into().0 {
        return write_no_rust_report(sink, request.mode(), request.report_dir());
    }
    let plan = CampaignExecutionPlan::new(config, request)?;
    execute_campaign_plan(runner, sink, &plan)
}

/// Return whether a campaign should skip the VM because no Rust changed.
#[inline]
#[must_use]
pub(super) fn should_skip_vm<'semantic, DiffSource>(
    mode: CampaignMode,
    diff_source: DiffSource,
) -> impl Into<ShouldSkipVmFlag>
where
    DiffSource: Into<DiffSourceText<'semantic>>,
{
    let diff_source = diff_source.into().0;
    if !mode.needs_diff().into().0 {
        return false;
    }
    !diff_touches_rust(diff_source).into().0
}

/// Return whether a unified diff touches Rust source files.
///
/// # Contract
/// - ensures: returns true only for added-file diff header lines that end in
///   `.rs`.
/// - provides: the host-side no-Rust fast path before any VM boot.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — no-Rust fast-path tests distinguish Rust and
///   non-Rust `+++` lines.
/// - witness: `mutants::sandbox::tests::non_rust_diff_skips_vm_and_writes_report`
#[inline]
#[must_use]
pub(super) fn diff_touches_rust<'semantic, DiffSource>(
    diff_source: DiffSource
) -> impl Into<DiffTouchesRustFlag>
where
    DiffSource: Into<DiffSourceText<'semantic>>,
{
    let diff_source = diff_source.into().0;
    for line in diff_source.lines() {
        if line
            .strip_prefix("+++ ")
            .is_some_and(|target| path_has_extension(target, "rs").into().0)
        {
            return true;
        }
    }
    false
}

/// Cleanup stray sandboxes whose names use the owned prefix.
///
/// # Contract
/// - ensures: only names beginning with [`SANDBOX_PREFIX`] are stopped and
///   removed.
/// - provides: exact prefix-scoped cleanup for sandboxes left by interrupted
///   campaigns.
/// - fails: returns a single aggregated error when any stop/remove operation
///   fails.
/// - panics: none.
///
/// # Errors
/// Returns list-command failures or aggregated stop/remove failures.
///
/// # Adequacy
/// - hypothesis: L3 only — prefix cleanup tests include owned and foreign
///   names, killing mutants that broaden the cleanup scope.
/// - witness: `mutants::sandbox::tests::prefix_cleanup_only_reaps_owned_sandboxes`
#[inline]
pub(super) fn cleanup_stray_sandboxes<Runner>(runner: &mut Runner) -> Result<Vec<String>, GateError>
where
    Runner: MsbAdapter,
{
    let list_output = runner.run_output(msb_list_plan().args())?;
    if !list_output.success_status().into().0 {
        return Err(command_failure(
            "mutants-vm: `msb list` failed",
            &list_output,
        ));
    }

    let stale_names = stale_sandbox_names(list_output.stdout().into().0);
    let mut failures = Vec::new();
    for name_text in &stale_names {
        let sandbox_name = SandboxName::new(name_text)?;
        let stop_output = runner.run_status(stop_plan(&sandbox_name).args())?;
        if !stop_output.success_status().into().0 {
            failures.push(format!(
                "failed to stop {}: {}",
                crate::semantic_value::<AsStrText<'_>, _>(sandbox_name.as_str()).0,
                command_detail(&stop_output)
            ));
        }
        let remove_output = runner.run_status(remove_plan(&sandbox_name).args())?;
        if !remove_output.success_status().into().0 {
            failures.push(format!(
                "failed to remove {}: {}",
                crate::semantic_value::<AsStrText<'_>, _>(sandbox_name.as_str()).0,
                command_detail(&remove_output)
            ));
        }
    }

    if failures.is_empty() {
        return Ok(stale_names);
    }
    Err(GateError::operational(format!(
        "mutants-vm: cleanup failed: {}",
        failures.join("; ")
    )))
}

/// Remove a cache scratch volume when it exists.
///
/// # Contract
/// - requires: `scratch_name` is the exact temporary volume name to reap.
/// - ensures: an existing scratch volume is removed before cache-image reuse is
///   trusted; a cleanup failure is fail-closed.
/// - provides: the cache cleanup behavior used by snapshot provisioning.
/// - fails: returns list or remove failures as [`GateError`].
/// - panics: none.
///
/// # Errors
/// Returns `msb volume list` failures or `msb volume remove` failures.
///
/// # Adequacy
/// - hypothesis: L3 only — cache cleanup failure tests kill mutants that
///   silently continue after a failed scratch-volume removal.
/// - witness: `mutants::sandbox::tests::cache_cleanup_failure_is_hard_error`
#[inline]
pub(super) fn remove_cache_scratch<'semantic, Runner, ScratchName>(
    runner: &mut Runner,
    scratch_name: ScratchName,
) -> Result<impl Into<RemoveCacheScratchFlag>, GateError>
where
    Runner: MsbAdapter,
    ScratchName: Into<ScratchNameText<'semantic>>,
{
    let scratch_name = scratch_name.into().0;
    let list_output = runner.run_output(volume_list_plan().args())?;
    if !list_output.success_status().into().0 {
        return Err(command_failure(
            "mutants-vm: volume list failed",
            &list_output,
        ));
    }
    if !volume_listing_contains(list_output.stdout().into().0, scratch_name)
        .into()
        .0
    {
        return Ok(false);
    }

    let remove_output = runner.run_status(volume_remove_plan(scratch_name).args())?;
    if !remove_output.success_status().into().0 {
        return Err(command_failure(
            &format!("mutants-vm: failed to remove cache scratch volume {scratch_name}"),
            &remove_output,
        ));
    }
    Ok(true)
}

/// Build the sandbox boot plan.
#[inline]
#[must_use]
pub(super) fn sandbox_boot_plan(
    config: &SandboxConfig,
    name: &SandboxName,
    mode: CampaignMode,
) -> MsbPlan
{
    MsbPlan::new(vec![
        os("run"),
        os("--snapshot"),
        config.snapshot().to_os_string(),
        os("--detach"),
        os("--name"),
        name.as_os_str().to_os_string(),
        os("--replace"),
        os("--mount-disk"),
        config.cache_mount().to_os_string(),
        os("--cpus"),
        os("8"),
        os("--memory"),
        os("24G"),
        os("--rlimit"),
        os("nproc=2048"),
        os("--rlimit"),
        os("nofile=8192"),
        os("--no-net"),
        os("--max-duration"),
        os(mode.sandbox_timeout().into().0),
        os("--security-model"),
        os("complete"),
        os("--no-tty"),
    ])
}

/// Build the `msb copy` plan for the source archive.
#[inline]
#[must_use]
pub(super) fn copy_source_plan(
    name: &SandboxName,
    source_archive: &Path,
) -> MsbPlan
{
    MsbPlan::new(vec![
        os("copy"),
        os("--quiet"),
        source_archive.as_os_str().to_os_string(),
        guest_target(name, GUEST_SOURCE_ARCHIVE),
    ])
}

/// Build the `msb copy` plan for the unified diff.
#[inline]
#[must_use]
pub(super) fn copy_diff_plan(
    name: &SandboxName,
    diff: &Path,
) -> MsbPlan
{
    MsbPlan::new(vec![
        os("copy"),
        os("--quiet"),
        diff.as_os_str().to_os_string(),
        guest_target(name, GUEST_DIFF),
    ])
}

/// Build the guest extraction command plan.
#[inline]
#[must_use]
pub(super) fn extract_source_plan(name: &SandboxName) -> MsbPlan
{
    MsbPlan::new(vec![
        os("exec"),
        name.as_os_str().to_os_string(),
        os("--no-tty"),
        os("--workdir"),
        os(GUEST_WORK_DIR),
        os("--"),
        os("sh"),
        os("-lc"),
        os("rm -rf repo && mkdir -p repo && tar -xf src.tar -C repo"),
    ])
}

/// Build the guest mutation command plan.
#[inline]
#[must_use]
pub(super) fn guest_execution_plan(
    name: &SandboxName,
    mode: CampaignMode,
) -> MsbPlan
{
    let mut args = vec![
        os("exec"),
        name.as_os_str().to_os_string(),
        os("--no-tty"),
        os("--workdir"),
        os(GUEST_REPO_DIR),
        os("--env"),
        os(GUEST_CARGO_HOME_ENV),
        os("--env"),
        os(GUEST_RUSTUP_HOME_ENV),
        os("--env"),
        os(GUEST_RUST_TOOLCHAIN_ENV),
        os("--env"),
        os(GUEST_TMPDIR_ENV),
        os("--env"),
        os(GUEST_PATH_ENV),
        os("--env"),
        os(GUEST_MISE_TOOLS_ENV),
        os("--env"),
        os(GUEST_MISE_TRUST_ENV),
        os("--timeout"),
        os(mode.guest_timeout().into().0),
        os("--"),
        os("mise"),
        os("exec"),
        os("--"),
        os("cargo"),
        os("run"),
        os("--quiet"),
        os("-p"),
        os("gandr-workflow-gates"),
        os("--"),
        os("mutants"),
        os("guest"),
    ];
    if mode.needs_diff().into().0 {
        args.push(os("--diff"));
        args.push(os(GUEST_DIFF));
    }
    MsbPlan::new(args)
}

/// Build the guest report probe plan.
#[inline]
#[must_use]
pub(super) fn probe_report_plan(name: &SandboxName) -> MsbPlan
{
    MsbPlan::new(vec![
        os("exec"),
        name.as_os_str().to_os_string(),
        os("--no-tty"),
        os("--"),
        os("test"),
        os("-d"),
        os(GUEST_REPORT_DIR),
    ])
}

/// Build the report copy-out plan.
#[inline]
#[must_use]
pub(super) fn copy_report_plan(
    name: &SandboxName,
    report_dir: &Path,
) -> MsbPlan
{
    MsbPlan::new(vec![
        os("copy"),
        os("--quiet"),
        guest_target(name, GUEST_REPORT_DIR),
        report_dir.as_os_str().to_os_string(),
    ])
}

/// Build the sandbox list plan.
#[inline]
#[must_use]
pub(super) fn msb_list_plan() -> MsbPlan
{
    MsbPlan::new(vec![os("list")])
}

/// Build the cache volume list plan.
#[inline]
#[must_use]
pub(super) fn volume_list_plan() -> MsbPlan
{
    MsbPlan::new(vec![os("volume"), os("list")])
}

/// Build the cache scratch volume creation plan used by snapshot provisioning.
///
/// # Contract
/// - requires: `scratch_name` is the exact temporary msb volume name.
/// - ensures: creates a disk volume with the historical 64G cache size.
/// - provides: the first side-effecting step of `format-cache-image`.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — snapshot plan tests kill mutants that create a host
///   mount or use the wrong scratch size/name.
#[inline]
#[must_use]
pub(super) fn volume_create_plan<'semantic, ScratchName>(scratch_name: ScratchName) -> MsbPlan
where
    ScratchName: Into<ScratchNameText<'semantic>>,
{
    let scratch_name = scratch_name.into().0;
    MsbPlan::new(vec![
        os("volume"),
        os("create"),
        os(scratch_name),
        os("--kind"),
        os("disk"),
        os("--size"),
        os(CACHE_SIZE),
        os("--quiet"),
    ])
}

/// Build the cache scratch volume inspection plan.
///
/// # Contract
/// - requires: `scratch_name` names an msb managed volume.
/// - ensures: returns the command whose stdout contains the documented `Path:`
///   backing-image field.
/// - provides: a typed plan for discovering the raw volume image.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — snapshot execution fails if this plan cannot expose
///   a backing image path before host-side reflink copy.
#[inline]
#[must_use]
pub(super) fn volume_inspect_plan<'semantic, ScratchName>(scratch_name: ScratchName) -> MsbPlan
where
    ScratchName: Into<ScratchNameText<'semantic>>,
{
    let scratch_name = scratch_name.into().0;
    MsbPlan::new(vec![os("volume"), os("inspect"), os(scratch_name)])
}

/// Build the temporary VM plan that reformats the scratch disk as btrfs.
///
/// # Contract
/// - requires: `scratch_name` names the scratch disk volume created for cache
///   formatting.
/// - ensures: runs `mkfs.btrfs` inside an Ubuntu guest with a named disk mount;
///   this is snapshot provisioning and intentionally not a campaign sandbox.
/// - provides: exact `mutants-vm.nu format-cache-image` msb arguments.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — plan tests distinguish the named scratch disk used
///   for formatting from forbidden campaign host passthrough mounts.
#[inline]
#[must_use]
pub(super) fn format_cache_image_plan<'semantic, ScratchName>(scratch_name: ScratchName) -> MsbPlan
where
    ScratchName: Into<ScratchNameText<'semantic>>,
{
    let scratch_name = scratch_name.into().0;
    let mount = format!("{scratch_name}:/cache:kind=disk,size={CACHE_SIZE}");
    MsbPlan::new(vec![
        os("run"),
        os(BASE_IMAGE),
        os("--mount-named"),
        OsString::from(mount),
        os("--no-tty"),
        os("--quiet"),
        os("--timeout"),
        os(CACHE_FORMAT_TIMEOUT),
        os("--"),
        os("bash"),
        os("-lc"),
        os(format_cache_script().into().0),
    ])
}

/// Return the validated snapshot builder sandbox name.
///
/// # Contract
/// - ensures: the builder uses the same owned prefix as campaign sandboxes so
///   teardown helpers can validate it.
/// - provides: the fixed `gandr-mutants-build` name from the original script.
/// - fails: returns an operational error only if the fixed constant drifts out
///   of the owned namespace.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 constant drift — the prefix validator kills edits that
///   rename the builder outside the driver's ownership boundary.
#[inline]
pub(super) fn snapshot_builder_name() -> Result<SandboxName, GateError>
{
    SandboxName::new(SNAPSHOT_BUILDER_NAME)
}

/// Build the snapshot builder boot plan.
///
/// # Contract
/// - requires: `config` names the cache image mounted as a btrfs block device
///   and `builder` is [`snapshot_builder_name`].
/// - ensures: boots `ubuntu:24.04` with the script's network-capable builder
///   resources, not the no-net campaign sandbox profile.
/// - provides: exact snapshot-builder `msb run` arguments.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — snapshot tests kill mutants that confuse builder
///   resources with campaign no-net/security-model arguments.
#[inline]
#[must_use]
pub(super) fn snapshot_builder_boot_plan(
    config: &SandboxConfig,
    builder: &SandboxName,
) -> MsbPlan
{
    MsbPlan::new(vec![
        os("run"),
        os(BASE_IMAGE),
        os("--detach"),
        os("--name"),
        builder.as_os_str().to_os_string(),
        os("--replace"),
        os("--mount-disk"),
        config.cache_mount().to_os_string(),
        os("--cpus"),
        os("8"),
        os("--memory"),
        os("16G"),
        os("--max-duration"),
        os(SNAPSHOT_BUILDER_TIMEOUT),
        os("--no-tty"),
        os("--quiet"),
    ])
}

/// Build the snapshot provisioning command plan.
///
/// # Contract
/// - requires: `builder` is a running snapshot builder.
/// - ensures: installs OS prerequisites, writes the guest sentinel, installs
///   the pinned Rust toolchain, and installs mise exactly as the script did.
/// - provides: the networked provisioning half of `main snapshot`.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — containment tests kill snapshots that omit or alter
///   the sentinel token, while warm-cache tests kill missing toolchain setup.
#[inline]
#[must_use]
pub(super) fn snapshot_builder_provision_plan(builder: &SandboxName) -> MsbPlan
{
    MsbPlan::new(vec![
        os("exec"),
        builder.as_os_str().to_os_string(),
        os("--no-tty"),
        os("--"),
        os("bash"),
        os("-lc"),
        OsString::from(snapshot_provision_script()),
    ])
}

/// Build the snapshot source archive copy-in plan.
///
/// # Contract
/// - requires: `source_archive` is a host tar created from tracked files.
/// - ensures: copies the tar over the guest-agent channel, not through a host
///   passthrough mount.
/// - provides: exact source transfer for the snapshot warm step.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — no-host-share assertions kill mutants that replace
///   this copy with a mounted source tree.
#[inline]
#[must_use]
pub(super) fn snapshot_builder_copy_source_plan(
    builder: &SandboxName,
    source_archive: &Path,
) -> MsbPlan
{
    MsbPlan::new(vec![
        os("copy"),
        os("--quiet"),
        source_archive.as_os_str().to_os_string(),
        guest_target(builder, GUEST_SOURCE_ARCHIVE),
    ])
}

/// Build the snapshot warm-cache command plan.
///
/// # Contract
/// - requires: `/cache/work/src.tar` exists in the builder.
/// - ensures: extracts the archived repo onto btrfs, runs `mise install`,
///   fetches cargo dependencies with the lockfile, and removes the warm source
///   tree.
/// - provides: the offline-ready cache preparation from `mutants-vm.nu
///   snapshot`.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — warm-cache tests kill missing `mise install`,
///   missing `cargo fetch --locked`, and stale source archive retention.
#[inline]
#[must_use]
pub(super) fn snapshot_builder_warm_plan(builder: &SandboxName) -> MsbPlan
{
    MsbPlan::new(vec![
        os("exec"),
        builder.as_os_str().to_os_string(),
        os("--no-tty"),
        os("--"),
        os("bash"),
        os("-lc"),
        OsString::from(snapshot_warm_script()),
    ])
}

/// Build the final snapshot creation plan.
///
/// # Contract
/// - requires: `builder` has been stopped successfully after provisioning.
/// - ensures: creates or replaces `gandr-mutants-base` with integrity checking.
/// - provides: the durable snapshot publication step of snapshot provisioning.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — snapshot lifecycle tests kill mutants that create
///   the snapshot before stopping the builder or omit `--integrity`.
#[inline]
#[must_use]
pub(super) fn snapshot_create_plan(builder: &SandboxName) -> MsbPlan
{
    MsbPlan::new(vec![
        os("snapshot"),
        os("create"),
        os("--from"),
        builder.as_os_str().to_os_string(),
        os(SNAPSHOT_NAME),
        os("--force"),
        os("--integrity"),
    ])
}

/// Execute copy-in, guest command, report copy-out, and campaign writing.
fn execute_payload<Runner, Sink>(
    runner: &mut Runner,
    sink: &mut Sink,
    plan: &CampaignExecutionPlan,
) -> Result<CampaignSummary, GateError>
where
    Runner: MsbAdapter,
    Sink: CampaignReportSink,
{
    run_checked_status(
        runner,
        &plan.copy_source,
        &format!(
            "mutants-vm: failed to copy source archive into sandbox {}",
            crate::semantic_value::<AsStrText<'_>, _>(plan.sandbox_name.as_str()).0
        ),
    )?;
    if let Some(ref copy_diff) = plan.copy_diff {
        run_checked_status(
            runner,
            copy_diff,
            &format!(
                "mutants-vm: failed to copy diff into sandbox {}",
                crate::semantic_value::<AsStrText<'_>, _>(plan.sandbox_name.as_str()).0
            ),
        )?;
    }
    run_checked_status(
        runner,
        &plan.extract,
        &format!(
            "mutants-vm: failed to extract source archive in sandbox {}",
            crate::semantic_value::<AsStrText<'_>, _>(plan.sandbox_name.as_str()).0
        ),
    )?;

    let guest_output = runner.run_status(plan.guest.args())?;
    let succeeded = guest_output.success_status().into().0;
    let report_kind = preserve_guest_report(runner, sink, plan)?;
    let summary = CampaignSummary::new(plan.mode, succeeded, report_kind);
    sink.write_campaign(&plan.report_dir, &summary)?;
    Ok(summary)
}

/// Write a no-Rust campaign report without invoking `msb`.
fn write_no_rust_report<Sink>(
    sink: &mut Sink,
    mode: CampaignMode,
    report_dir: &Path,
) -> Result<CampaignSummary, GateError>
where
    Sink: CampaignReportSink,
{
    let summary = CampaignSummary::new(mode, true, CampaignReportKind::NoRustChanges);
    sink.create_empty_report(report_dir)?;
    sink.write_campaign(report_dir, &summary)?;
    Ok(summary)
}

/// Execute a previously constructed campaign plan.
///
/// # Contract
/// - requires: `plan` was built from [`CampaignExecutionPlan::new`].
/// - ensures: boot happens once; payload commands run in order; stop and remove
///   are attempted even when the payload fails.
/// - provides: teardown-error precedence of remove over stop over payload.
/// - fails: returns boot, payload, stop, or remove failure according to the
///   precedence contract.
/// - panics: none.
///
/// # Errors
/// Returns non-zero `msb` outcomes or adapter/report-sink failures.
///
/// # Adequacy
/// - hypothesis: L3 only — a scripted payload failure plus stop failure kills
///   mutants that return the payload error before teardown status.
/// - witness: `mutants::sandbox::tests::teardown_error_takes_precedence_over_payload_error`
#[inline]
pub(super) fn execute_campaign_plan<Runner, Sink>(
    runner: &mut Runner,
    sink: &mut Sink,
    plan: &CampaignExecutionPlan,
) -> Result<CampaignSummary, GateError>
where
    Runner: MsbAdapter,
    Sink: CampaignReportSink,
{
    let boot_output = runner.run_status(plan.boot.args())?;
    if !boot_output.success_status().into().0 {
        return Err(command_failure(
            &format!(
                "mutants-vm: failed to boot sandbox {}",
                crate::semantic_value::<AsStrText<'_>, _>(plan.sandbox_name.as_str()).0
            ),
            &boot_output,
        ));
    }

    let payload_result = execute_payload(runner, sink, plan);
    let stop_result = runner.run_status(plan.stop.args());
    let remove_result = runner.run_status(plan.remove.args());

    match remove_result {
        | Ok(remove_output) => {
            if !remove_output.success_status().into().0 {
                return Err(command_failure(
                    &format!(
                        "mutants-vm: failed to remove sandbox {}",
                        crate::semantic_value::<AsStrText<'_>, _>(plan.sandbox_name.as_str()).0
                    ),
                    &remove_output,
                ));
            }
        },
        | Err(remove_error) => return Err(remove_error),
    }

    match stop_result {
        | Ok(stop_output) => {
            if !stop_output.success_status().into().0 {
                return Err(command_failure(
                    &format!(
                        "mutants-vm: failed to stop sandbox {}",
                        crate::semantic_value::<AsStrText<'_>, _>(plan.sandbox_name.as_str()).0
                    ),
                    &stop_output,
                ));
            }
        },
        | Err(stop_error) => return Err(stop_error),
    }

    payload_result
}

/// Run a status-only command and require success.
fn run_checked_status<'semantic, Runner, Context>(
    runner: &mut Runner,
    plan: &MsbPlan,
    context: Context,
) -> Result<(), GateError>
where
    Runner: MsbAdapter,
    Context: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let output = runner.run_status(plan.args())?;
    if output.success_status().into().0 {
        return Ok(());
    }
    Err(command_failure(context, &output))
}

/// Preserve a guest report or create an empty no-mutants report.
fn preserve_guest_report<Runner, Sink>(
    runner: &mut Runner,
    sink: &mut Sink,
    plan: &CampaignExecutionPlan,
) -> Result<CampaignReportKind, GateError>
where
    Runner: MsbAdapter,
    Sink: CampaignReportSink,
{
    let probe_output = runner.run_status(plan.probe_report.args())?;
    match probe_output.code().into().0 {
        | Some(EXIT_SUCCESS_CODE) if probe_output.success_status().into().0 => {
            run_checked_status(
                runner,
                &plan.copy_report,
                &format!(
                    "mutants-vm: failed to preserve mutation report from sandbox {}",
                    crate::semantic_value::<AsStrText<'_>, _>(plan.sandbox_name.as_str()).0
                ),
            )?;
            Ok(CampaignReportKind::CargoMutants)
        },
        | Some(EXIT_REPORT_ABSENT_CODE) => {
            sink.create_empty_report(&plan.report_dir)?;
            Ok(CampaignReportKind::NoMutantsSelected)
        },
        | Some(_) | None => Err(command_failure(
            "mutants-vm: failed to probe mutation report",
            &probe_output,
        )),
    }
}

/// Return whether a path-like value has `extension` as its file extension.
fn path_has_extension<'semantic, Value, Extension>(
    value: Value,
    extension: Extension,
) -> impl Into<PathHasExtensionFlag>
where
    Value: Into<ValueText<'semantic>>,
    Extension: Into<ExtensionText<'semantic>>,
{
    let extension = extension.into().0;
    let value = value.into().0;
    Path::new(value)
        .extension()
        .is_some_and(|candidate| candidate == OsStr::new(extension))
}

/// Shell script that formats the mounted scratch disk as btrfs.
fn format_cache_script() -> impl Into<FormatCacheScriptText<'static>>
{
    "set -eu\nexport DEBIAN_FRONTEND=noninteractive\napt-get update -qq >/dev/null && apt-get install -y -qq btrfs-progs >/dev/null\ndev=$(awk '$2 == \"/cache\" { print $1 }' /proc/mounts)\numount /cache\nmkfs.btrfs -f -q \"$dev\"\n"
}

/// Shell script that provisions the reusable snapshot builder.
fn snapshot_provision_script() -> String
{
    format!(
        "set -eu\nexport DEBIAN_FRONTEND=noninteractive\napt-get update -qq\napt-get install -y -qq --no-install-recommends ca-certificates curl git build-essential pkg-config btrfs-progs\ninstall -d /cache/cargo /cache/tmp /cache/work\nprintf '%s\\n' '{sentinel}' > {sentinel_path}\nexport TMPDIR=/cache/tmp\nexport CARGO_HOME=/cache/cargo RUSTUP_HOME=/usr/local/rustup\ncurl -fsSL https://sh.rustup.rs | sh -s -- -y --no-modify-path --profile minimal --default-toolchain {toolchain}\ncurl -fsSL https://mise.run | MISE_INSTALL_PATH=/usr/local/bin/mise sh\n",
        sentinel = super::containment::SENTINEL_TOKEN,
        sentinel_path = super::containment::SENTINEL_PATH,
        toolchain = RUST_TOOLCHAIN,
    )
}

/// Shell script that warms the cache image for offline mutation campaigns.
fn snapshot_warm_script() -> String
{
    format!(
        "set -eu\nexport TMPDIR=/cache/tmp\nexport CARGO_HOME=/cache/cargo RUSTUP_HOME=/usr/local/rustup RUSTUP_TOOLCHAIN={toolchain}\nexport PATH=\"$CARGO_HOME/bin:$RUSTUP_HOME/bin:$PATH\"\nexport MISE_ENABLE_TOOLS={mise_tools} MISE_TRUSTED_CONFIG_PATHS={guest_repo}\nrm -rf {guest_repo} && mkdir -p {guest_repo} && tar -xf {source_archive} -C {guest_repo}\ncd {guest_repo}\nmise install\nmise exec -- cargo fetch --locked\nrm -rf {guest_repo} {source_archive}\n",
        toolchain = RUST_TOOLCHAIN,
        mise_tools = GUEST_MISE_TOOLS,
        guest_repo = GUEST_REPO_DIR,
        source_archive = GUEST_SOURCE_ARCHIVE,
    )
}

/// Build a cache mount argument from the host image path.
fn cache_mount_argument(cache_image: &Path) -> OsString
{
    let mut mount = cache_image.as_os_str().to_os_string();
    mount.push(":/cache:fstype=btrfs");
    mount
}

/// Convert a UTF-8 literal to an operating-system argument.
fn os<'semantic, Value>(value: Value) -> OsString
where
    Value: Into<ValueText<'semantic>>,
{
    let value = value.into().0;
    OsString::from(value)
}

/// Build a guest target argument of the form `name:/path`.
fn guest_target<'semantic, GuestPath>(
    name: &SandboxName,
    guest_path: GuestPath,
) -> OsString
where
    GuestPath: Into<GuestPathText<'semantic>>,
{
    let guest_path = guest_path.into().0;
    OsString::from(format!(
        "{}:{guest_path}",
        crate::semantic_value::<AsStrText<'_>, _>(name.as_str()).0
    ))
}

/// Render a boolean as NUON text.
fn nuon_bool<Value>(value: Value) -> impl Into<NuonBoolText<'static>>
where
    Value: Into<ValueFlag>,
{
    let value = value.into().0;
    if value {
        return "true";
    }
    "false"
}

/// Build an operational command failure.
fn command_failure<'semantic, Context>(
    context: Context,
    output: &CommandOutcome,
) -> GateError
where
    Context: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    GateError::operational(format!("{context}: {}", command_detail(output)))
}

/// Select the command diagnostic text from consumed stdout or exit status.
fn command_detail(output: &CommandOutcome) -> String
{
    if !output.stdout().into().0.is_empty() {
        return output.stdout().into().0.to_owned();
    }
    support::command_status_detail(MSB_PROGRAM, output.code().into().0)
}

/// Build the sandbox removal plan.
#[inline]
#[must_use]
pub(super) fn remove_plan(name: &SandboxName) -> MsbPlan
{
    MsbPlan::new(vec![
        os("remove"),
        name.as_os_str().to_os_string(),
        os("--quiet"),
    ])
}

/// Return prefixed sandbox names from `msb list` output.
fn stale_sandbox_names<'semantic, ListStdout>(list_stdout: ListStdout) -> Vec<String>
where
    ListStdout: Into<ListStdoutText<'semantic>>,
{
    let list_stdout = list_stdout.into().0;
    let mut names = Vec::new();
    for line in list_stdout.lines().skip(MSB_LIST_HEADER_LINES) {
        if let Some(name) = line.split_whitespace().next()
            && name.starts_with(SANDBOX_PREFIX)
        {
            names.push(String::from(name));
        }
    }
    names
}

/// Build the sandbox stop plan.
#[inline]
#[must_use]
pub(super) fn stop_plan(name: &SandboxName) -> MsbPlan
{
    MsbPlan::new(vec![
        os("stop"),
        name.as_os_str().to_os_string(),
        os("--quiet"),
    ])
}

/// Return whether a volume listing contains `scratch_name` as its first column.
fn volume_listing_contains<'semantic, ListStdout, ScratchName>(
    list_stdout: ListStdout,
    scratch_name: ScratchName,
) -> impl Into<VolumeListingContainsFlag>
where
    ListStdout: Into<ListStdoutText<'semantic>>,
    ScratchName: Into<ScratchNameText<'semantic>>,
{
    let scratch_name = scratch_name.into().0;
    let list_stdout = list_stdout.into().0;
    for line in list_stdout.lines() {
        if let Some(name) = line.split_whitespace().next()
            && name == scratch_name
        {
            return true;
        }
    }
    false
}

/// Build the cache scratch volume removal plan.
#[inline]
#[must_use]
pub(super) fn volume_remove_plan<'semantic, ScratchName>(scratch_name: ScratchName) -> MsbPlan
where
    ScratchName: Into<ScratchNameText<'semantic>>,
{
    let scratch_name = scratch_name.into().0;
    MsbPlan::new(vec![
        os("volume"),
        os("remove"),
        os(scratch_name),
        os("--quiet"),
    ])
}

#[cfg(test)]
mod tests
{
    //! Behavioral tests for host-side sandbox campaign planning and execution.

    use alloc::collections::VecDeque;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;
    use std::ffi::OsStr;
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;

    use super::ArgFlag;
    use super::ArgSequenceFlag;
    use super::CacheImageExistsFlag;
    use super::CampaignExecutionPlan;
    use super::CampaignMode;
    use super::CampaignReportKind;
    use super::CampaignReportSink;
    use super::CampaignRequest;
    use super::CampaignSummary;
    use super::CommandOutcome;
    use super::MsbAdapter;
    use super::MsbAvailableFlag;
    use super::SandboxConfig;
    use super::SandboxName;
    use super::SnapshotExistsFlag;
    use super::cleanup_stray_sandboxes;
    use super::execute_campaign_plan;
    use super::execute_campaign_request;
    use super::remove_cache_scratch;
    use crate::GateError;

    /// Test result type for sandbox unit fixtures.
    type TestResult = Result<(), GateError>;

    /// Scripted `msb` adapter that returns queued outcomes.
    #[derive(Default)]
    struct FakeMsbAdapter
    {
        /// Captured command calls.
        calls: Vec<Vec<OsString>>,
        /// Queued captured-output results.
        output_results: VecDeque<CommandOutcome>,
        /// Queued status-only results.
        status_results: VecDeque<CommandOutcome>,
    }

    impl FakeMsbAdapter
    {
        /// Build a fake adapter from output and status queues.
        fn new(
            output_results: Vec<CommandOutcome>,
            status_results: Vec<CommandOutcome>,
        ) -> Self
        {
            Self {
                calls: Vec::new(),
                output_results: VecDeque::from(output_results),
                status_results: VecDeque::from(status_results),
            }
        }

        /// Return calls rendered as shell-like strings for assertions.
        fn rendered_calls(&self) -> Vec<String>
        {
            self.calls.iter().map(|call| render_call(call)).collect()
        }
    }

    impl MsbAdapter for FakeMsbAdapter
    {
        fn run_output(
            &mut self,
            args: &[OsString],
        ) -> Result<CommandOutcome, GateError>
        {
            self.calls.push(args.to_vec());
            self.output_results.pop_front().ok_or_else(|| {
                GateError::operational(format!(
                    "fake adapter has no output result for {}",
                    render_call(args)
                ))
            })
        }

        fn run_status(
            &mut self,
            args: &[OsString],
        ) -> Result<CommandOutcome, GateError>
        {
            self.calls.push(args.to_vec());
            self.status_results.pop_front().ok_or_else(|| {
                GateError::operational(format!(
                    "fake adapter has no status result for {}",
                    render_call(args)
                ))
            })
        }
    }

    /// Fake campaign report sink for no-Rust and no-mutants paths.
    #[derive(Default)]
    struct FakeReportSink
    {
        /// Report directories created by the sink.
        created_reports: Vec<PathBuf>,
        /// Campaign summaries written by the sink.
        summaries: Vec<CampaignSummary>,
    }

    impl CampaignReportSink for FakeReportSink
    {
        fn create_empty_report(
            &mut self,
            report_dir: &Path,
        ) -> Result<(), GateError>
        {
            self.created_reports.push(report_dir.to_path_buf());
            Ok(())
        }

        fn write_campaign(
            &mut self,
            _report_dir: &Path,
            summary: &CampaignSummary,
        ) -> Result<(), GateError>
        {
            self.summaries.push(summary.clone());
            Ok(())
        }
    }

    /// Fake infrastructure probe for required-component tests.
    struct FakeInfrastructure
    {
        /// Whether msb is available.
        msb_available: bool,
        /// Whether the configured snapshot exists.
        snapshot_exists: bool,
        /// Whether the configured cache image exists.
        cache_image_exists: bool,
    }

    impl FakeInfrastructure
    {
        /// Build a fake probe from component booleans.
        fn new<MsbAvailable, SnapshotExists, CacheImageExists>(
            msb_available: MsbAvailable,
            snapshot_exists: SnapshotExists,
            cache_image_exists: CacheImageExists,
        ) -> Self
        where
            MsbAvailable: Into<super::MsbAvailableFlag>,
            SnapshotExists: Into<super::SnapshotExistsFlag>,
            CacheImageExists: Into<super::CacheImageExistsFlag>,
        {
            let snapshot_exists = snapshot_exists.into().0;
            let cache_image_exists = cache_image_exists.into().0;
            let msb_available = msb_available.into().0;
            Self {
                msb_available,
                snapshot_exists,
                cache_image_exists,
            }
        }
    }

    impl super::SandboxInfrastructure for FakeInfrastructure
    {
        fn msb_available(&mut self) -> Result<impl Into<MsbAvailableFlag>, GateError>
        {
            Ok(self.msb_available)
        }

        fn snapshot_exists(
            &mut self,
            _config: &SandboxConfig,
        ) -> Result<impl Into<SnapshotExistsFlag>, GateError>
        {
            Ok(self.snapshot_exists)
        }

        fn cache_image_exists(
            &mut self,
            _config: &SandboxConfig,
        ) -> Result<impl Into<CacheImageExistsFlag>, GateError>
        {
            Ok(self.cache_image_exists)
        }
    }

    /// Return whether `args` contains `expected` as one contiguous argument
    /// sequence.
    fn contains_arg_sequence<'semantic, Expected>(
        args: &[OsString],
        expected: Expected,
    ) -> impl Into<ArgSequenceFlag>
    where
        Expected: IntoIterator,
        Expected::Item: Into<super::NeedleText<'semantic>>,
    {
        let expected = expected
            .into_iter()
            .map(|value| OsString::from(value.into().0))
            .collect::<Vec<_>>();
        expected.is_empty()
            || args.windows(expected.len()).any(|window| {
                window
                    .iter()
                    .zip(&expected)
                    .all(|(actual, expected)| actual == expected)
            })
    }

    /// Render an argument vector for exact call assertions.
    fn render_call(args: &[OsString]) -> String
    {
        let mut text = String::new();
        let mut first = true;
        for argument in args {
            if first {
                first = false;
            }
            else {
                text.push(' ');
            }
            let piece = argument.to_string_lossy();
            text.push_str(piece.as_ref());
        }
        text
    }

    /// All campaign modes expose stable labels, diff requirements, and timeout
    /// caps.
    #[test]
    fn campaign_modes_cover_labels_diff_requirements_and_timeouts()
    {
        let cases = [
            (CampaignMode::Push, "push", true, "45m", "55m"),
            (CampaignMode::Merge, "merge", true, "45m", "55m"),
            (CampaignMode::Scheduled, "scheduled", true, "45m", "55m"),
            (CampaignMode::Sweep, "sweep", false, "8h", "8h15m"),
        ];

        for (mode, label, needs_diff, guest_timeout, sandbox_timeout) in cases {
            assert_eq!(
                crate::semantic_value::<super::AsStrText<'static>, _>(mode.as_str()).0,
                label,
                "mode label should be stable"
            );
            assert_eq!(
                mode.needs_diff().into().0,
                needs_diff,
                "diff requirement should match campaign mode"
            );
            assert_eq!(
                mode.guest_timeout().into().0,
                guest_timeout,
                "guest timeout should be capped by mode"
            );
            assert_eq!(
                mode.sandbox_timeout().into().0,
                sandbox_timeout,
                "sandbox timeout should outlive the guest timeout by mode"
            );
        }
    }

    /// Sandbox names reject non-owned, empty-suffix, and path-shaped values.
    #[test]
    fn sandbox_name_rejects_values_outside_owned_namespace()
    {
        for value in [
            "foreign-mutants",
            super::SANDBOX_PREFIX,
            "gandr-mutants-../escape",
            "gandr-mutants-..\\escape",
        ] {
            assert!(
                SandboxName::new(value).is_err(),
                "sandbox name {value:?} should be rejected"
            );
        }
        assert_eq!(
            "gandr-mutants-owned",
            crate::semantic_value::<super::AsStrText<'_>, _>(
                SandboxName::new("gandr-mutants-owned")
                    .expect("owned name should validate")
                    .as_str()
            )
            .0,
            "valid sandbox name should preserve its text"
        );
    }

    /// Missing infrastructure components fail closed with component-specific
    /// diagnostics.
    #[test]
    fn required_infra_reports_each_missing_component()
    {
        let config = config();
        let cases = [
            (
                FakeInfrastructure::new(false, true, true),
                "`msb` (microsandbox) not found",
            ),
            (
                FakeInfrastructure::new(true, false, true),
                "microVM snapshot `gandr-mutants-base` not found",
            ),
            (
                FakeInfrastructure::new(true, true, false),
                "btrfs cache image `/tmp/gandr-mutants-cache.btrfs` not found",
            ),
        ];

        for (mut infrastructure, expected) in cases {
            let error = super::require_infra(&mut infrastructure, &config)
                .expect_err("missing infrastructure component should fail");
            assert!(
                error.to_string().contains(expected),
                "infrastructure error should identify the missing component"
            );
        }
    }

    /// Diff-scoped campaigns require a diff path before any command is planned.
    #[test]
    fn campaign_plan_requires_diff_for_diff_scoped_modes() -> TestResult
    {
        let name = sandbox_name()?;
        let request = CampaignRequest::new(
            CampaignMode::Scheduled,
            &name,
            Path::new("src.tar"),
            None,
            Path::new("mutants.out"),
        );

        let error = CampaignExecutionPlan::new(&config(), &request)
            .expect_err("scheduled campaigns require a copied diff path");

        assert!(
            error
                .to_string()
                .contains("scheduled campaign requires a diff path"),
            "missing diff error should name the scheduled campaign"
        );
        Ok(())
    }

    /// Successful guest campaigns copy out a cargo-mutants report and write a
    /// cargo-mutants summary.
    #[test]
    fn successful_campaign_copies_cargo_mutants_report() -> TestResult
    {
        let plan = campaign_plan(CampaignMode::Merge)?;
        let mut runner = FakeMsbAdapter::new(Vec::new(), vec![
            CommandOutcome::success(),
            CommandOutcome::success(),
            CommandOutcome::success(),
            CommandOutcome::success(),
            CommandOutcome::success(),
            CommandOutcome::success(),
            CommandOutcome::success(),
            CommandOutcome::success(),
            CommandOutcome::success(),
        ]);
        let mut sink = FakeReportSink::default();

        let summary = execute_campaign_plan(&mut runner, &mut sink, &plan)?;

        assert!(
            summary.succeeded().into().0,
            "successful guest status should succeed"
        );
        assert_eq!(
            CampaignReportKind::CargoMutants,
            summary.report(),
            "present guest report should be copied out as a cargo-mutants report"
        );
        assert!(
            sink.created_reports.is_empty(),
            "present cargo-mutants reports should not synthesize an empty report"
        );
        assert!(
            runner.rendered_calls().iter().any(|call| call
                == "copy --quiet gandr-mutants-merge-test:/cache/work/repo/mutants.out mutants.out"),
            "successful report preservation should copy the guest report out"
        );
        Ok(())
    }

    /// Sandbox removal failure takes precedence over stop and payload failures.
    #[test]
    fn remove_failure_takes_precedence_over_stop_and_payload_failures() -> TestResult
    {
        let plan = campaign_plan(CampaignMode::Merge)?;
        let mut runner = FakeMsbAdapter::new(Vec::new(), vec![
            CommandOutcome::success(),
            CommandOutcome::failure(Some(0x2_i32)),
            CommandOutcome::failure(Some(0x3_i32)),
            CommandOutcome::failure(Some(0x4_i32)),
        ]);
        let mut sink = FakeReportSink::default();

        let error = execute_campaign_plan(&mut runner, &mut sink, &plan)
            .expect_err("remove failure should have highest teardown precedence");

        assert!(
            error.to_string().contains("failed to remove sandbox"),
            "remove failure should take precedence over payload and stop failures"
        );
        assert!(
            error.to_string().contains("status 4"),
            "remove failure should preserve remove status"
        );
        Ok(())
    }

    /// Absent cache scratch volumes skip removal; present volumes remove by
    /// exact first-column match only.
    #[test]
    fn cache_scratch_absence_and_exact_match_are_observed() -> TestResult
    {
        let mut absent_runner = FakeMsbAdapter::new(
            vec![CommandOutcome::success_with_stdout("NAME KIND\n")],
            Vec::new(),
        );
        assert!(
            !remove_cache_scratch(&mut absent_runner, "gandr-mutants-mkfs")
                .map(|value| value.into().0)?,
            "missing scratch volume should be a no-op"
        );
        assert_eq!(
            vec![String::from("volume list")],
            absent_runner.rendered_calls(),
            "absent scratch cleanup should only inspect volumes"
        );

        let mut present_runner = FakeMsbAdapter::new(
            vec![CommandOutcome::success_with_stdout(
                "NAME KIND\nnot-gandr-mutants-mkfs disk\ngandr-mutants-mkfs disk\n",
            )],
            vec![CommandOutcome::success()],
        );
        assert!(
            remove_cache_scratch(&mut present_runner, "gandr-mutants-mkfs")
                .map(|value| value.into().0)?,
            "exact first-column scratch match should be removed"
        );
        assert!(
            present_runner
                .rendered_calls()
                .contains(&String::from("volume remove gandr-mutants-mkfs --quiet")),
            "present scratch volume should be removed by exact name"
        );
        Ok(())
    }

    /// Snapshot provisioning plans use a network-capable builder and named disk
    /// formatting without campaign host passthrough mounts.
    #[test]
    fn snapshot_plans_keep_builder_and_campaign_containment_distinct() -> TestResult
    {
        let builder = super::snapshot_builder_name()?;
        let config = config();
        let boot = super::snapshot_builder_boot_plan(&config, &builder);
        assert!(
            contains_arg(boot.args(), "ubuntu:24.04").into().0,
            "snapshot builder should boot from the base image"
        );
        assert!(
            !contains_arg(boot.args(), "--no-net").into().0,
            "snapshot builder must remain network-capable for provisioning"
        );
        assert_eq!(
            Some(OsStr::new("16G")),
            arg_after(boot.args(), "--memory"),
            "builder memory should stay distinct from campaign memory"
        );

        let format = super::format_cache_image_plan("gandr-mutants-mkfs");
        assert!(
            contains_arg(format.args(), "--mount-named").into().0,
            "cache formatting should use the named scratch disk"
        );
        assert!(
            format
                .args()
                .iter()
                .any(|arg| arg == OsStr::new("gandr-mutants-mkfs:/cache:kind=disk,size=64G")),
            "cache formatting should mount the scratch volume as a 64G disk"
        );
        assert!(
            super::snapshot_builder_warm_plan(&builder)
                .args()
                .iter()
                .any(|arg| arg.to_string_lossy().contains("cargo fetch --locked")),
            "warm-cache plan should fetch locked cargo dependencies"
        );
        Ok(())
    }

    /// Boot plans use block-device cache mounts and no host passthrough mounts.
    #[test]
    fn sandbox_boot_plan_has_no_forbidden_host_mounts() -> TestResult
    {
        let plan = campaign_plan(CampaignMode::Merge)?;
        let boot = plan.boot();

        assert!(
            contains_arg(boot.args(), "--mount-disk").into().0,
            "boot plan must attach only the btrfs cache as a disk image"
        );
        assert!(
            contains_arg(boot.args(), "--no-net").into().0,
            "boot plan must disable guest networking"
        );
        assert!(
            contains_arg(boot.args(), "--rlimit").into().0,
            "boot plan must carry resource limits"
        );
        assert_eq!(
            Some(OsStr::new("complete")),
            arg_after(boot.args(), "--security-model"),
            "boot plan must request the complete security model"
        );
        assert!(
            !boot.contains_forbidden_host_mount().into().0,
            "boot plan must not contain host directory/file mount flags"
        );
        Ok(())
    }

    /// Timeout caps and unified Rust guest routing are fixed for every mode.
    #[test]
    fn timeout_caps_and_sequential_jobs_are_planned() -> TestResult
    {
        let merge_plan = campaign_plan(CampaignMode::Merge)?;
        assert_eq!(
            Some(OsStr::new("55m")),
            arg_after(merge_plan.boot().args(), "--max-duration"),
            "merge sandbox max duration should leave teardown headroom"
        );
        assert_eq!(
            Some(OsStr::new("45m")),
            arg_after(merge_plan.guest().args(), "--timeout"),
            "merge guest timeout should cap cargo-mutants execution"
        );
        assert!(
            contains_arg_sequence(merge_plan.guest().args(), [
                "mise",
                "exec",
                "--",
                "cargo",
                "run",
                "--quiet",
                "-p",
                "gandr-workflow-gates",
                "--",
                "mutants",
                "guest",
                "--diff",
                super::GUEST_DIFF,
            ],)
            .into()
            .0,
            "merge campaign should invoke the unified Rust guest with its diff"
        );
        assert!(
            !contains_arg(merge_plan.guest().args(), "nu").into().0
                && !contains_arg(merge_plan.guest().args(), "--jobs").into().0
                && contains_arg(merge_plan.guest().args(), super::GUEST_MISE_TOOLS_ENV,)
                    .into()
                    .0,
            "guest routing must be Nushell-free, fixed-width, and carry only the Rust tool allow-list"
        );

        let sweep_plan = campaign_plan(CampaignMode::Sweep)?;
        assert_eq!(
            Some(OsStr::new("8h15m")),
            arg_after(sweep_plan.boot().args(), "--max-duration"),
            "sweep sandbox max duration should be capped"
        );
        assert_eq!(
            Some(OsStr::new("8h")),
            arg_after(sweep_plan.guest().args(), "--timeout"),
            "sweep guest timeout should be capped"
        );
        assert!(
            contains_arg_sequence(sweep_plan.guest().args(), [
                "mise",
                "exec",
                "--",
                "cargo",
                "run",
                "--quiet",
                "-p",
                "gandr-workflow-gates",
                "--",
                "mutants",
                "guest",
            ],)
            .into()
            .0,
            "sweep campaign should invoke the unified Rust guest"
        );
        assert!(
            !contains_arg(sweep_plan.guest().args(), "nu").into().0
                && !contains_arg(sweep_plan.guest().args(), "--jobs").into().0
                && !contains_arg(sweep_plan.guest().args(), "--diff").into().0,
            "sweep guest routing must be Rust-only, fixed-width, and diff-free"
        );
        Ok(())
    }

    /// Return whether `args` contains `needle`.
    fn contains_arg<'semantic, Needle>(
        args: &[OsString],
        needle: Needle,
    ) -> impl Into<ArgFlag>
    where
        Needle: Into<super::NeedleText<'semantic>>,
    {
        let needle = needle.into().0;
        args.iter().any(|argument| argument == OsStr::new(needle))
    }

    /// Return the value immediately after `flag` in `args`.
    fn arg_after<'semantic, 'arguments, Flag>(
        args: &'arguments [OsString],
        flag: Flag,
    ) -> Option<&'arguments OsStr>
    where
        Flag: Into<super::FlagText<'semantic>>,
    {
        let flag = flag.into().0;
        let mut arguments = args.iter();
        while let Some(argument) = arguments.next() {
            if argument == OsStr::new(flag) {
                return arguments.next().map(OsString::as_os_str);
            }
        }
        None
    }

    /// Teardown failures take precedence over payload failures.
    #[test]
    fn teardown_error_takes_precedence_over_payload_error() -> TestResult
    {
        let plan = campaign_plan(CampaignMode::Merge)?;
        let mut runner = FakeMsbAdapter::new(Vec::new(), vec![
            CommandOutcome::success(),
            CommandOutcome::failure(Some(0x2_i32)),
            CommandOutcome::failure(Some(0x3_i32)),
            CommandOutcome::success(),
        ]);
        let mut sink = FakeReportSink::default();

        let error = execute_campaign_plan(&mut runner, &mut sink, &plan)
            .expect_err("payload plus stop failure should return the teardown failure");
        assert!(
            error.to_string().contains("failed to stop sandbox"),
            "stop failure should take precedence over payload failure"
        );
        assert!(
            error.to_string().contains("status 3"),
            "teardown diagnostic should preserve stop status"
        );
        Ok(())
    }

    /// Build a test campaign request and plan.
    fn campaign_plan(mode: CampaignMode) -> Result<CampaignExecutionPlan, GateError>
    {
        let name = sandbox_name()?;
        let diff = Path::new("changes.diff");
        let request = if mode.needs_diff().into().0 {
            CampaignRequest::new(
                mode,
                &name,
                Path::new("src.tar"),
                Some(diff),
                Path::new("mutants.out"),
            )
        }
        else {
            CampaignRequest::new(
                mode,
                &name,
                Path::new("src.tar"),
                None,
                Path::new("mutants.out"),
            )
        };
        CampaignExecutionPlan::new(&config(), &request)
    }

    /// Cleanup only touches exact-prefix sandboxes.
    #[test]
    fn prefix_cleanup_only_reaps_owned_sandboxes() -> TestResult
    {
        let listing = "NAME STATE\ngandr-mutants-old Running\nforeign-mutants Running\ngandr-mutants-new Stopped\n";
        let mut runner =
            FakeMsbAdapter::new(vec![CommandOutcome::success_with_stdout(listing)], vec![
                CommandOutcome::success(),
                CommandOutcome::success(),
                CommandOutcome::success(),
                CommandOutcome::success(),
            ]);

        let cleaned = cleanup_stray_sandboxes(&mut runner)?;
        assert_eq!(
            vec![
                String::from("gandr-mutants-old"),
                String::from("gandr-mutants-new")
            ],
            cleaned,
            "cleanup should return only owned sandbox names"
        );
        let calls = runner.rendered_calls();
        assert!(
            calls
                .iter()
                .any(|call| call == "stop gandr-mutants-old --quiet"),
            "cleanup should stop the first owned sandbox"
        );
        assert!(
            calls
                .iter()
                .any(|call| call == "remove gandr-mutants-new --quiet"),
            "cleanup should remove the second owned sandbox"
        );
        assert!(
            !calls.iter().any(|call| call.contains("foreign-mutants")),
            "cleanup must not touch foreign sandbox names"
        );
        Ok(())
    }

    /// Cache scratch cleanup failures are hard failures.
    #[test]
    fn cache_cleanup_failure_is_hard_error()
    {
        let listing = "NAME KIND\ngandr-mutants-mkfs disk\n";
        let mut runner =
            FakeMsbAdapter::new(vec![CommandOutcome::success_with_stdout(listing)], vec![
                CommandOutcome::failure(Some(0x4_i32)),
            ]);

        let error = remove_cache_scratch(&mut runner, "gandr-mutants-mkfs")
            .map(|value| value.into().0)
            .expect_err("failed cache cleanup should fail closed");
        assert!(
            error
                .to_string()
                .contains("failed to remove cache scratch volume"),
            "cache cleanup failure should name the scratch volume cleanup path"
        );
        assert!(
            error.to_string().contains("status 4"),
            "cache cleanup failure should preserve msb status"
        );
    }

    /// Non-Rust diffs write a report without issuing any `msb` command.
    #[test]
    fn non_rust_diff_skips_vm_and_writes_report() -> TestResult
    {
        let name = sandbox_name()?;
        let request = CampaignRequest::new(
            CampaignMode::Push,
            &name,
            Path::new("src.tar"),
            Some(Path::new("changes.diff")),
            Path::new("mutants.out"),
        );
        let mut runner = FakeMsbAdapter::default();
        let mut sink = FakeReportSink::default();
        let summary = execute_campaign_request(
            &mut runner,
            &mut sink,
            &config(),
            &request,
            "+++ b/docs/README.md\n",
        )?;

        assert!(
            runner.calls.is_empty(),
            "no-Rust fast path should not invoke msb"
        );
        assert_eq!(
            CampaignReportKind::NoRustChanges,
            summary.report(),
            "no-Rust fast path should publish the no-rust report kind"
        );
        assert!(
            summary.succeeded().into().0,
            "no-Rust fast path should succeed"
        );
        assert_eq!(
            vec![summary],
            sink.summaries,
            "no-Rust fast path should write exactly the returned summary"
        );
        Ok(())
    }

    /// Return a baseline sandbox name for tests.
    fn sandbox_name() -> Result<SandboxName, GateError>
    {
        SandboxName::new("gandr-mutants-merge-test")
    }

    /// Return a baseline sandbox config for tests.
    fn config() -> SandboxConfig
    {
        SandboxConfig::new(Path::new("/tmp/gandr-mutants-cache.btrfs"))
    }
}
