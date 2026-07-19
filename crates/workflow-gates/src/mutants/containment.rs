//! Pure guest-containment and cargo-mutants command planning.
//!
//! The integration layer performs kernel, environment, filesystem, and sentinel
//! probes, then hands those facts here. This module never spawns a VM or runs
//! cargo-mutants; it only fails closed unless the supplied facts prove a
//! contained guest and renders the deterministic `cargo mutants` argument
//! vector.
#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::missing_assert_message,
        clippy::panic,
        clippy::unwrap_used,
        reason = "module tests use compact fixtures and exact assertion helpers"
    )
)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use crate::GateError;

crate::semantic_str!(pub struct ProofNameText);
crate::semantic_copy!(pub struct RequestedCount(u16));
crate::semantic_str!(pub struct NameText);
crate::semantic_str!(pub struct KernelNameText);
crate::semantic_optional_str!(pub struct OptionalActValueText);
crate::semantic_copy!(pub struct ContainedFlag(bool));

/// Sentinel path baked into the mutation guest image.
pub(super) const SENTINEL_PATH: &str = "/etc/gandr-mutants-guest";

/// Sentinel token baked into the mutation guest image.
pub(super) const SENTINEL_TOKEN: &str = "gandr-mutants-guest-v1";

/// Cargo-mutants worker count required for deterministic sequential campaigns.
pub(super) const DEFAULT_MUTANTS_JOBS: u16 = 1;

/// Proof name for the non-Darwin kernel check.
const PROOF_NON_DARWIN_KERNEL: &str = "non-darwin-kernel";

/// Proof name for the `ACT=true` refusal check.
const PROOF_NOT_UNDER_ACT: &str = "not-under-act";

/// Proof name for the macOS host-root reachability check.
const PROOF_NO_HOST_FILESYSTEM: &str = "no-host-filesystem";

/// Proof name for the guest sentinel check.
const PROOF_GUEST_SENTINEL: &str = "guest-sentinel";

/// A sequential cargo-mutants job count.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
#[repr(transparent)]
pub(super) struct CargoMutantsJobs
{
    /// The validated job count, currently always one.
    value: u16,
}

impl CargoMutantsJobs
{
    /// Validate a requested cargo-mutants job count.
    ///
    /// # Contract
    /// - requires: `requested` is the parsed `--jobs` value from the caller.
    /// - ensures: accepts only the sequential count `1`.
    /// - provides: a job-count token safe to render in cargo-mutants argv.
    /// - fails: returns a usage error for zero or parallel worker counts.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns a usage error when `requested` is not exactly one.
    ///
    /// # Adequacy
    /// - hypothesis: L2 boundary — `1`, `0`, and `2` distinguish sequential,
    ///   missing-work, and parallel execution surfaces.
    /// - witness: `mutants::containment::tests::cargo_mutants_jobs_default_to_one_and_reject_parallelism`
    #[inline]
    pub(super) fn from_requested(requested: impl Into<RequestedCount>) -> Result<Self, GateError>
    {
        let requested = requested.into().0;
        if requested == DEFAULT_MUTANTS_JOBS {
            Ok(Self { value: requested })
        }
        else {
            Err(GateError::usage(format!(
                "mutants-guest: cargo-mutants jobs must be {DEFAULT_MUTANTS_JOBS} for sequential execution"
            )))
        }
    }

    /// Return the validated job count.
    #[inline]
    #[must_use]
    pub(super) const fn value(self) -> RequestedCount
    {
        RequestedCount(self.value)
    }
}

impl Default for CargoMutantsJobs
{
    #[inline]
    fn default() -> Self
    {
        Self {
            value: DEFAULT_MUTANTS_JOBS,
        }
    }
}

/// The cargo-mutants package selection scope.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) enum CargoMutantsScope
{
    /// Run against every workspace package.
    Workspace,
    /// Run against one named package.
    Package
    {
        /// Package name passed after `--package`.
        name: String,
    },
}

impl CargoMutantsScope
{
    /// Build a package scope from a CLI package value.
    ///
    /// # Contract
    /// - requires: `name` is a caller-provided package name, not shell text.
    /// - ensures: rejects empty or whitespace-padded names and preserves
    ///   accepted names exactly.
    /// - provides: a package scope that renders `--package <name>`.
    /// - fails: returns a usage error for an empty or whitespace-padded name.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns a usage error when `name` is empty after trimming or contains
    /// surrounding whitespace.
    ///
    /// # Adequacy
    /// - hypothesis: L2 boundary — workspace omission, exact package spelling,
    ///   empty string, and padded package names distinguish every scope branch.
    /// - witness: `mutants::containment::tests::cargo_mutants_package_and_workspace_argv_are_exact`
    #[inline]
    pub(super) fn package<'semantic>(
        name: impl Into<NameText<'semantic>>
    ) -> Result<Self, GateError>
    {
        let name = name.into().0;
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed != name {
            return Err(GateError::usage(
                "mutants-guest: package scope must be a nonempty package name without surrounding whitespace",
            ));
        }

        Ok(Self::Package {
            name: String::from(name),
        })
    }
}

/// Caller-proven diff path state for `--in-diff` campaigns.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) enum DiffPath<'path>
{
    /// No diff restriction; render a full workspace/package campaign.
    Absent,
    /// Diff restriction exists and may be passed to cargo-mutants.
    Present(&'path Path),
    /// Diff restriction was requested but does not exist.
    Missing(&'path Path),
}

/// Pure request for a cargo-mutants invocation plan.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) struct CargoMutantsRequest<'path>
{
    /// Scope to mutate.
    pub scope: CargoMutantsScope,
    /// Optional diff path state.
    pub diff: DiffPath<'path>,
    /// Sequential job count.
    pub jobs: CargoMutantsJobs,
}

/// Explicit command plan for `cargo mutants`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) struct CargoMutantsPlan
{
    /// Program to execute at the integration boundary.
    pub program: OsString,
    /// Arguments passed to `program`; the first argument is `mutants`.
    pub args: Vec<OsString>,
    /// Scope retained as typed metadata.
    pub scope: CargoMutantsScope,
    /// Diff path retained as typed metadata, when present.
    pub diff_path: Option<PathBuf>,
    /// Sequential job count retained as typed metadata.
    pub jobs: CargoMutantsJobs,
}

impl CargoMutantsPlan
{
    /// Borrow the planned command arguments.
    #[inline]
    #[must_use]
    pub(super) fn args(&self) -> &[OsString]
    {
        &self.args
    }
}

/// Render a deterministic cargo-mutants invocation plan.
///
/// # Contract
/// - requires: `request` contains caller-proven diff existence state and a
///   validated sequential job count.
/// - ensures: returns `cargo mutants --no-shuffle --caught --unviable --jobs 1`
///   followed by exactly one workspace/package scope and optional `--in-diff`.
/// - provides: the guest-side command plan without spawning cargo-mutants.
/// - fails: returns an operational error when a requested diff file is missing.
/// - panics: none.
///
/// # Errors
/// Returns an operational error naming a missing diff path. Scope and job
/// errors are reported by their constructors before this function is called.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — workspace, package, diff-present, diff-missing,
///   and sequential-job rendering each have exact argument-vector witnesses.
/// - witness: `mutants::containment::tests::cargo_mutants_package_and_workspace_argv_are_exact`
/// - witness: `mutants::containment::tests::cargo_mutants_plan_rejects_missing_diff`
#[inline]
pub(super) fn cargo_mutants_plan(
    request: &CargoMutantsRequest<'_>
) -> Result<CargoMutantsPlan, GateError>
{
    let mut args = vec![
        OsString::from("mutants"),
        OsString::from("--no-shuffle"),
        OsString::from("--caught"),
        OsString::from("--unviable"),
        OsString::from("--jobs"),
        OsString::from(request.jobs.value().0.to_string()),
    ];

    match request.scope {
        | CargoMutantsScope::Workspace => args.push(OsString::from("--workspace")),
        | CargoMutantsScope::Package { ref name } => {
            args.push(OsString::from("--package"));
            args.push(OsString::from(name));
        },
    }

    let diff_path = match request.diff {
        | DiffPath::Absent => None,
        | DiffPath::Present(path) => {
            args.push(OsString::from("--in-diff"));
            args.push(path.as_os_str().to_os_string());
            Some(path.to_path_buf())
        },
        | DiffPath::Missing(path) => {
            return Err(GateError::operational(format!(
                "mutants-guest: diff file not found: {}",
                path.display()
            )));
        },
    };

    Ok(CargoMutantsPlan {
        program: OsString::from("cargo"),
        args,
        scope: request.scope.clone(),
        diff_path,
        jobs: request.jobs,
    })
}

/// Require all containment proofs before planning a guest run.
///
/// # Contract
/// - requires: `evidence` contains all observations the integration layer could
///   make.
/// - ensures: returns the proof report only when every proof succeeded.
/// - provides: fail-closed guest validation independent of cargo-mutants argv
///   construction.
/// - fails: returns an operational error listing every failed proof.
/// - panics: none.
///
/// # Errors
/// Returns an operational error when at least one containment proof fails.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — each proof-failure witness checks that the
///   terminal validator fails, not merely that a report flag changes.
/// - witness: `mutants::containment::tests::darwin_kernel_is_rejected`
/// - witness: `mutants::containment::tests::act_environment_is_rejected`
/// - witness: `mutants::containment::tests::reachable_host_markers_are_rejected`
/// - witness: `mutants::containment::tests::guest_sentinel_absent_or_invalid_is_rejected`
#[inline]
pub(super) fn require_containment(
    evidence: &ContainmentEvidence
) -> Result<ContainmentReport<'_>, GateError>
{
    let report = containment_report(evidence);
    if report.is_contained().into().0 {
        return Ok(report);
    }

    let mut detail =
        String::from("mutants-guest: REFUSING to run cargo-mutants — containment not proven.");
    for proof in report.proofs.iter().filter(|proof| !proof.ok) {
        detail.push_str("\n  - ");
        detail.push_str(proof.name);
        detail.push_str(": ");
        detail.push_str(&proof.detail);
    }

    Err(GateError::operational(detail))
}

/// Build the deterministic containment proof report.
///
/// # Contract
/// - requires: `evidence` is an integration-layer observation, not a claim from
///   an untrusted environment variable alone.
/// - ensures: proof order is stable; Darwin kernels, `ACT=true`, reachable
///   macOS host markers, and absent or invalid sentinels each produce a failed
///   proof; Git environment markers are copied to ignored metadata and never
///   influence proof success.
/// - provides: a pure report suitable for diagnostics or fail-closed
///   validation.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — each load-bearing proof has a one-failure
///   witness, a valid-guest witness proves conjunction, and a Git-marker
///   fixture proves environment markers are non-proofs.
/// - witness: `mutants::containment::tests::darwin_kernel_is_rejected`
/// - witness: `mutants::containment::tests::act_environment_is_rejected`
/// - witness: `mutants::containment::tests::reachable_host_markers_are_rejected`
/// - witness: `mutants::containment::tests::guest_sentinel_absent_or_invalid_is_rejected`
/// - witness: `mutants::containment::tests::git_environment_marker_is_not_containment_proof`
#[inline]
#[must_use]
pub(super) fn containment_report(evidence: &ContainmentEvidence) -> ContainmentReport<'_>
{
    ContainmentReport {
        proofs: vec![
            non_darwin_kernel_proof(&evidence.kernel_name),
            not_under_act_proof(evidence.act_value.as_deref()),
            no_host_filesystem_proof(&evidence.reachable_host_markers),
            guest_sentinel_proof(&evidence.sentinel),
        ],
        ignored_git_environment_markers: &evidence.git_environment_markers,
    }
}

/// Build the non-Darwin kernel proof.
#[inline]
fn non_darwin_kernel_proof<'semantic>(
    kernel_name: impl Into<KernelNameText<'semantic>>
) -> ContainmentProof
{
    let kernel_name = kernel_name.into().0;
    let ok = !kernel_name.trim().is_empty() && kernel_name != "Darwin";
    let detail = if ok {
        format!("kernel `{kernel_name}` is not Darwin")
    }
    else if kernel_name.trim().is_empty() {
        String::from("kernel name is absent, so non-Darwin guest status is unproven")
    }
    else {
        format!("kernel `{kernel_name}` is Darwin")
    };

    ContainmentProof {
        name: PROOF_NON_DARWIN_KERNEL,
        ok,
        detail,
    }
}

/// Build the `ACT=true` refusal proof.
#[inline]
fn not_under_act_proof<'semantic>(
    act_value: impl Into<OptionalActValueText<'semantic>>
) -> ContainmentProof
{
    let act_value = act_value.into().0;
    let ok = act_value != Some("true");
    let detail = match act_value {
        | Some("true") => String::from("ACT=true means a Docker/act run, not the mutation guest"),
        | Some(value) => format!("ACT is `{value}`, not `true`"),
        | None => String::from("ACT is unset"),
    };

    ContainmentProof {
        name: PROOF_NOT_UNDER_ACT,
        ok,
        detail,
    }
}

/// Build the host-filesystem reachability proof.
#[inline]
fn no_host_filesystem_proof(markers: &[PathBuf]) -> ContainmentProof
{
    let ok = markers.is_empty();
    let detail = if ok {
        String::from("no macOS host roots are reachable")
    }
    else {
        format!("macOS host roots reachable: {}", format_path_list(markers))
    };

    ContainmentProof {
        name: PROOF_NO_HOST_FILESYSTEM,
        ok,
        detail,
    }
}

/// Build the guest sentinel proof.
#[inline]
fn guest_sentinel_proof(sentinel: &GuestSentinel) -> ContainmentProof
{
    let ok = matches!(*sentinel, GuestSentinel::Valid);
    let detail = match *sentinel {
        | GuestSentinel::Valid => format!("{SENTINEL_PATH} contains {SENTINEL_TOKEN}"),
        | GuestSentinel::Absent => format!("{SENTINEL_PATH} is absent or unreadable"),
        | GuestSentinel::Invalid { ref observed } => {
            format!("{SENTINEL_PATH} contains `{observed}`, not required token `{SENTINEL_TOKEN}`")
        },
    };

    ContainmentProof {
        name: PROOF_GUEST_SENTINEL,
        ok,
        detail,
    }
}

/// Render a stable comma-separated path list.
#[inline]
fn format_path_list(paths: &[PathBuf]) -> String
{
    let mut rendered = String::new();
    for path in paths {
        if !rendered.is_empty() {
            rendered.push_str(", ");
        }
        rendered.push_str(&path.display().to_string());
    }
    rendered
}

/// Observed guest sentinel state.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) enum GuestSentinel
{
    /// Sentinel file exists and contains the exact token.
    Valid,
    /// Sentinel file is absent or unreadable.
    Absent,
    /// Sentinel file exists but contains a different token.
    Invalid
    {
        /// Token or diagnostic observed by the integration boundary.
        observed: String,
    },
}

/// Integration-provided facts used to prove guest containment.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) struct ContainmentEvidence
{
    /// Kernel name reported by the guest probe.
    pub kernel_name: String,
    /// Value of `ACT`, when set.
    pub act_value: Option<String>,
    /// macOS host roots reachable from the guest, if any.
    pub reachable_host_markers: Vec<PathBuf>,
    /// Guest sentinel state.
    pub sentinel: GuestSentinel,
    /// Environment markers such as Git hook variables that are intentionally
    /// ignored and never accepted as containment proof.
    pub git_environment_markers: Vec<String>,
}

/// One named containment proof and its diagnostic detail.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) struct ContainmentProof
{
    /// Stable proof name.
    pub name: &'static str,
    /// Whether this proof succeeded.
    pub ok: bool,
    /// Stable human-readable detail.
    pub detail: String,
}

/// Full containment validation report.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(super) struct ContainmentReport<'markers>
{
    /// All required proofs in deterministic order.
    pub proofs: Vec<ContainmentProof>,
    /// Git or hook environment markers observed and ignored by policy.
    pub ignored_git_environment_markers: &'markers [String],
}

impl ContainmentReport<'_>
{
    /// Return whether every containment proof succeeded.
    #[inline]
    #[must_use]
    pub(super) fn is_contained(&self) -> impl Into<ContainedFlag>
    {
        self.proofs.iter().all(|proof| proof.ok)
    }
}

#[cfg(test)]
mod tests
{
    //! Behavioral tests for containment proofs and cargo-mutants argv planning.

    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;

    use super::*;

    /// Cargo-mutants jobs default to one and reject parallel plans.
    #[test]
    fn cargo_mutants_jobs_default_to_one_and_reject_parallelism()
    {
        assert_eq!(
            DEFAULT_MUTANTS_JOBS,
            CargoMutantsJobs::default().value().0,
            "default cargo-mutants jobs must be sequential"
        );
        assert_eq!(
            DEFAULT_MUTANTS_JOBS,
            CargoMutantsJobs::from_requested(DEFAULT_MUTANTS_JOBS)
                .expect("one job is valid")
                .value()
                .0,
            "explicit one job must be valid"
        );
        assert!(
            CargoMutantsJobs::from_requested(0).is_err(),
            "zero jobs must be rejected"
        );
        assert!(
            CargoMutantsJobs::from_requested(2).is_err(),
            "parallel cargo-mutants jobs must be rejected"
        );
    }

    /// Workspace and package plans render exact cargo-mutants argv.
    #[test]
    fn cargo_mutants_package_and_workspace_argv_are_exact()
    {
        let workspace = CargoMutantsRequest {
            scope: CargoMutantsScope::Workspace,
            diff: DiffPath::Absent,
            jobs: CargoMutantsJobs::default(),
        };
        let workspace_plan = cargo_mutants_plan(&workspace).expect("workspace plan is valid");
        assert_eq!(
            workspace_plan.program,
            OsString::from("cargo"),
            "program must be cargo"
        );
        assert_eq!(
            vec![
                String::from("mutants"),
                String::from("--no-shuffle"),
                String::from("--caught"),
                String::from("--unviable"),
                String::from("--jobs"),
                String::from("1"),
                String::from("--workspace"),
            ],
            arg_text(workspace_plan.args()),
            "workspace argv must be exact and sequential"
        );

        let diff_path = Path::new("changed.diff");
        let package = CargoMutantsRequest {
            scope: CargoMutantsScope::package("gandr-core").expect("package fixture is valid"),
            diff: DiffPath::Present(diff_path),
            jobs: CargoMutantsJobs::default(),
        };
        let package_plan = cargo_mutants_plan(&package).expect("package plan is valid");
        assert_eq!(
            vec![
                String::from("mutants"),
                String::from("--no-shuffle"),
                String::from("--caught"),
                String::from("--unviable"),
                String::from("--jobs"),
                String::from("1"),
                String::from("--package"),
                String::from("gandr-core"),
                String::from("--in-diff"),
                String::from("changed.diff"),
            ],
            arg_text(package_plan.args()),
            "package argv must include package scope and diff filter exactly"
        );
        assert!(
            CargoMutantsScope::package(" gandr-core").is_err(),
            "package names with surrounding whitespace must be rejected"
        );
    }

    /// Convert OS arguments into UTF-8 test strings.
    fn arg_text(args: &[OsString]) -> Vec<String>
    {
        args.iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    /// Missing diff paths fail before any cargo-mutants argv is accepted.
    #[test]
    fn cargo_mutants_plan_rejects_missing_diff()
    {
        let request = CargoMutantsRequest {
            scope: CargoMutantsScope::Workspace,
            diff: DiffPath::Missing(Path::new("missing.diff")),
            jobs: CargoMutantsJobs::default(),
        };

        let error = cargo_mutants_plan(&request).expect_err("missing diff must fail");
        let rendered = error.to_string();
        assert!(
            rendered.contains("diff file not found"),
            "missing diff error must name the missing diff contract"
        );
    }

    /// Darwin kernels fail the load-bearing non-Darwin proof.
    #[test]
    fn darwin_kernel_is_rejected()
    {
        let mut evidence = valid_evidence();
        evidence.kernel_name = String::from("Darwin");

        assert_failed_proof(&evidence, PROOF_NON_DARWIN_KERNEL);
    }

    /// ACT environments fail closed because they are Docker, not the guest VM.
    #[test]
    fn act_environment_is_rejected()
    {
        let mut evidence = valid_evidence();
        evidence.act_value = Some(String::from("true"));

        assert_failed_proof(&evidence, PROOF_NOT_UNDER_ACT);
    }

    /// Reachable macOS host roots fail the filesystem isolation proof.
    #[test]
    fn reachable_host_markers_are_rejected()
    {
        let mut evidence = valid_evidence();
        evidence.reachable_host_markers = vec![PathBuf::from("/Users"), PathBuf::from("/Volumes")];

        assert_failed_proof(&evidence, PROOF_NO_HOST_FILESYSTEM);
    }

    /// Absent and invalid guest sentinels both fail closed.
    #[test]
    fn guest_sentinel_absent_or_invalid_is_rejected()
    {
        let mut absent = valid_evidence();
        absent.sentinel = GuestSentinel::Absent;
        assert_failed_proof(&absent, PROOF_GUEST_SENTINEL);

        let mut invalid = valid_evidence();
        invalid.sentinel = GuestSentinel::Invalid {
            observed: String::from("host-env-var"),
        };
        assert_failed_proof(&invalid, PROOF_GUEST_SENTINEL);
    }

    /// Valid facts satisfy all containment proofs.
    #[test]
    fn valid_guest_evidence_is_accepted()
    {
        let evidence = valid_evidence();
        let report = require_containment(&evidence).expect("valid guest evidence must pass");

        assert!(report.is_contained().into().0, "all valid proofs must pass");
    }

    /// Build evidence for a valid contained guest.
    fn valid_evidence() -> ContainmentEvidence
    {
        ContainmentEvidence {
            kernel_name: String::from("Linux"),
            act_value: None,
            reachable_host_markers: Vec::new(),
            sentinel: GuestSentinel::Valid,
            git_environment_markers: Vec::new(),
        }
    }

    /// Git environment markers are reported as ignored and never prove
    /// containment.
    #[test]
    fn git_environment_marker_is_not_containment_proof()
    {
        let mut evidence = valid_evidence();
        evidence.sentinel = GuestSentinel::Absent;
        evidence.git_environment_markers = vec![String::from("GANDR_MUTANTS_IN_VM=1")];

        let report = containment_report(&evidence);
        assert_eq!(
            vec![String::from("GANDR_MUTANTS_IN_VM=1")],
            report.ignored_git_environment_markers,
            "git environment markers must be preserved only as ignored metadata"
        );
        assert_failed_proof(&evidence, PROOF_GUEST_SENTINEL);
    }

    /// Assert that validation fails and names one failed proof.
    fn assert_failed_proof<'semantic>(
        evidence: &ContainmentEvidence,
        proof_name: impl Into<ProofNameText<'semantic>>,
    )
    {
        let proof_name = proof_name.into().0;
        let report = containment_report(evidence);
        assert!(
            report
                .proofs
                .iter()
                .any(|proof| proof.name == proof_name && !proof.ok),
            "report must include failed proof `{proof_name}`"
        );
        let error = require_containment(evidence).expect_err("containment must fail closed");
        let rendered = error.to_string();
        assert!(
            rendered.contains(proof_name),
            "terminal error `{rendered}` must name failed proof `{proof_name}`"
        );
    }
}
