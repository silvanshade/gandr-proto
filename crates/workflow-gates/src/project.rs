//! Project-owned operational gates retained from the Nushell audit.
//!
//! This module ports only the project operations that still have a live
//! Rust-gate role after the callsite audit: the default dependency graph guard
//! and the IU submodule pin/clean guard. The default graph guard uses `cargo
//! metadata` resolve data rather than parsing `cargo tree` text, so package
//! identity comes from Cargo's JSON graph. The IU guard keeps every `git`
//! invocation behind the shared sanitized-command support boundary.
//!
//! Two audited Nushell behaviors are deliberately omitted here. First, the raw
//! `^git` scanner is vacuous after clean cutover because this crate's project
//! operations call [`support::run_output`] with `sanitized_git = true` instead
//! of embedding ad-hoc environment scrubbing. Second, `scripts/agda-deps.nu` is
//! not reimplemented in this module: `mise run agda:deps` runs
//! `scripts/agda-deps.gandr` through the `gandr` script runner, which is what
//! makes the Nushell bootstrap obsolete.
//!
//! That leaves the binary's own `agda-deps` command (`main.rs`) as a second
//! implementation of the same provisioning step with no task calling it.
//! Whether it is retired or documented as a driver-independent fallback is
//! filed for the owner as `gandr-wvd.24.7-question-02`, not a settled state.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;

use crate::Finding;
use crate::GateError;
use crate::GateResult;
use crate::support;

crate::semantic_str!(pub struct IuPathText);
crate::semantic_str!(pub struct StatusStdoutText);
crate::semantic_str!(pub struct KindText);
crate::semantic_str!(pub struct DeclarationText);
crate::semantic_str!(pub struct CommandText);
crate::semantic_copy!(pub struct CodeExitCode(Option<i32>));
crate::semantic_str!(pub struct ContextText);
crate::semantic_str!(pub struct FieldText);
crate::semantic_str!(pub struct PorcelainStdoutText);
crate::semantic_str!(pub struct MetadataText);
crate::semantic_str!(pub struct ExpectedText);
crate::semantic_str!(pub struct MessageText);
crate::semantic_str!(pub struct StdoutText);
crate::semantic_str!(pub struct JsonFieldText);
crate::semantic_str!(pub struct MetadataJsonText);
crate::semantic_str!(pub struct HostTripleText);
crate::semantic_str!(pub struct NameText);
crate::semantic_str!(pub struct ReachableDefaultPackageNameText);
crate::semantic_copy!(pub struct NCount(usize));
crate::semantic_copy!(pub struct DepKindReachesDefaultGraphFlag(bool));
crate::semantic_copy!(pub struct ContentAtRecordedPinFlag(bool));
crate::semantic_copy!(pub struct DepReachesDefaultGraphFlag(bool));

/// Logical source label used for live Cargo metadata JSON.
const CARGO_METADATA_SOURCE: &str = "cargo metadata";

/// Logical source label used for current-host Rust target discovery.
const RUSTC_HOST_SOURCE: &str = "rustc -vV";

/// Package names banned from the default normal/build workspace graph.
const FORBIDDEN_DEFAULT_GRAPH_PACKAGES: [&str; 6] = [
    "tree-sitter",
    "gandr-tree-sitter",
    "regex",
    "regex-automata",
    "regex-syntax",
    "aho-corasick",
];

/// Workspace member whose edges are not followed during default-graph
/// traversal: `gandr-workflow-dylint` is a nightly-only `rustc_private` Dylint
/// driver, so its `dylint_linting→dylint_internal→regex` chain is tooling-only
/// and outside the production default graph policy.
const EXEMPT_DEFAULT_GRAPH_MEMBER: &str = "gandr-workflow-dylint";

/// Stable finding kind for forbidden default graph packages.
const DEFAULT_GRAPH_FINDING_KIND: &str = "forbidden-default-dependency";

/// Default IU submodule mount point relative to the workspace root.
const DEFAULT_IU_PATH: &str = "metatheory/upstream/internal-univalence";

/// Stable finding kind for an unregistered or uninitialized IU submodule.
const IU_UNINITIALIZED_KIND: &str = "iu-pin-uninitialized";

/// Stable finding kind for an IU checkout that differs from the recorded pin.
const IU_DRIFTED_KIND: &str = "iu-pin-drifted";

/// Stable finding kind for an IU submodule conflict state.
const IU_CONFLICTED_KIND: &str = "iu-pin-conflicted";

/// Stable finding kind for local edits inside the pinned IU submodule.
const IU_DIRTY_KIND: &str = "iu-pin-dirty";

/// Cargo metadata graph borrowed from a parsed JSON value.
struct MetadataGraph<'metadata>
{
    /// Package id to package name map from `packages`.
    package_names: BTreeMap<&'metadata str, &'metadata str>,
    /// Included normal/build dependency edges by package id.
    dependencies: BTreeMap<&'metadata str, Vec<&'metadata str>>,
    /// Workspace member package ids used as traversal roots.
    roots: Vec<&'metadata str>,
}

/// Parsed IU submodule status prefix and recorded pin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IuSubmoduleStatus
{
    /// Status class derived from the first `git submodule status` byte.
    class: IuSubmoduleStatusClass,
    /// Recorded gitlink SHA parsed from the status line when present.
    recorded_sha: Option<String>,
}

/// Semantic class for the `git submodule status` prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IuSubmoduleStatusClass
{
    /// No status line was returned for the requested submodule path.
    NotRegistered,
    /// The submodule is checked out at the recorded pin.
    Clean,
    /// `-`: the submodule is not initialized or lost shared registration.
    Uninitialized,
    /// `+`: the checkout HEAD differs from the recorded gitlink pin.
    Drifted,
    /// `U`: the submodule is in a conflict state.
    Conflicted,
    /// Any other prefix preserved for compatibility with the Nushell script's
    /// default branch, which proceeds to the clean-tree check.
    Other(char),
}

/// Minimal command probe data needed by pure IU status validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProbeOutput<'output>
{
    /// Whether the probed command exited successfully.
    success: bool,
    /// Standard output observed for the probe.
    stdout: &'output str,
}

/// Validate the default workspace dependency graph from live Cargo metadata.
///
/// # Contract
/// - requires: `workspace_root` names a Cargo workspace root readable by Cargo,
///   and `rustc -vV` can report the exact current host target triple.
/// - ensures: returns one finding per forbidden package reachable from a
///   workspace member through current-host normal/build dependency edges.
/// - provides: the retained `check-default-graph-tree-sitter-free.nu` policy
///   using Cargo metadata graph structure instead of `cargo tree` text, with
///   Cargo's own `--filter-platform <host>` target filtering.
/// - fails: returns an operational gate error when host discovery or Cargo
///   metadata fails, and a JSON or malformed-metadata error when Cargo returns
///   unusable graph data.
/// - panics: none.
/// - intension: command execution is sequential and deterministic; host
///   discovery precedes metadata capture, and findings are emitted in
///   [`FORBIDDEN_DEFAULT_GRAPH_PACKAGES`] order.
///
/// # Errors
/// Returns host-discovery, command, JSON, or malformed-metadata errors from
/// Cargo metadata collection and parsing.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — malformed JSON, forbidden transitive package
///   reachability, and host-filter argument construction are separated by exact
///   parser error, finding, and argv observations.
/// - witness: `project::tests::malformed_metadata_is_reported_as_json_error`
/// - witness: `project::tests::forbidden_transitive_package_is_reported`
/// - witness: `project::tests::cargo_metadata_args_include_host_filter_platform`
#[inline]
pub fn check_default_dependency_graph(workspace_root: &Path) -> GateResult
{
    let host_triple = current_host_triple()?;
    let args = cargo_metadata_args(&host_triple);
    let output = support::run_output(OsStr::new("cargo"), &args, Some(workspace_root), false)?;
    if !crate::semantic_value::<support::SuccessFlag, _>(output.success()).0 {
        return Err(operational(command_failure_detail(
            CARGO_METADATA_SOURCE,
            crate::semantic_value::<support::OptionalCodeCode, _>(output.code()).0,
        )));
    }

    let stdout = output.stdout_lossy();
    validate_default_dependency_graph_metadata(stdout.as_ref())
}

/// Validate the default IU mount point against its recorded pin and clean tree.
///
/// # Contract
/// - requires: `workspace_root` names a Git checkout containing the IU
///   submodule mount.
/// - ensures: returns semantic findings for uninitialized, drifted, conflicted,
///   or dirty IU states, and returns no findings for a clean checkout at the
///   recorded pin.
/// - provides: the retained `check-iu-pin.nu` behavior through sanitized Git
///   command execution.
/// - fails: returns typed operational errors when required Git probes cannot be
///   launched or when the clean-tree probe itself fails.
/// - panics: none.
/// - intension: probes are run sequentially; the dirty check is skipped
///   whenever the status prefix already explains the failed pin state.
///
/// # Errors
/// Returns command errors from the sanitized support runner and operational
/// errors for a failing `git status --porcelain` command inside the submodule.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — `-`, `+`, `U`, clean, and dirty surfaces are
///   distinguished by pure status/porcelain fixtures.
/// - witness: `project::tests::uninitialized_submodule_with_recorded_content_reports_sibling_hint`
/// - witness: `project::tests::drifted_submodule_reports_plus_status`
/// - witness: `project::tests::conflicted_submodule_reports_u_status`
/// - witness: `project::tests::clean_submodule_status_reports_no_pin_finding`
/// - witness: `project::tests::dirty_submodule_reports_read_only_violation`
#[inline]
pub fn check_default_iu_pin(workspace_root: &Path) -> GateResult
{
    check_iu_pin(workspace_root, Path::new(DEFAULT_IU_PATH))
}

/// Validate an IU submodule mount against its recorded pin and clean tree.
///
/// # Contract
/// - requires: `workspace_root` is the Git checkout root and `iu_path` is the
///   mount point to pass to `git submodule status` and `git -C`.
/// - ensures: returns the same semantic findings as [`check_default_iu_pin`]
///   for the supplied mount path.
/// - provides: fixture-independent IU pin validation for callers that need an
///   explicit mount point.
/// - fails: returns support-runner errors for process launch failures and an
///   operational error for a failing clean-tree probe.
/// - panics: none.
/// - intension: all Git commands pass `sanitized_git = true`.
///
/// # Errors
/// Returns command errors from the sanitized support runner and operational
/// errors for a failing clean-tree probe status failure.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — command failure, prefix-specific status
///   findings, and dirty-tree findings are separated by fixture tests and by
///   the status-before-dirty probe order.
/// - witness: `project::tests::unregistered_submodule_reports_missing_registration`
/// - witness: `project::tests::uninitialized_submodule_without_content_reports_init_hint`
/// - witness: `project::tests::dirty_submodule_reports_read_only_violation`
#[inline]
pub fn check_iu_pin(
    workspace_root: &Path,
    iu_path: &Path,
) -> GateResult
{
    let iu_label = path_label(iu_path);
    let status_args = git_submodule_status_args(iu_path);
    let status_output =
        support::run_output(OsStr::new("git"), &status_args, Some(workspace_root), true)?;
    if !crate::semantic_value::<support::SuccessFlag, _>(status_output.success()).0 {
        return Ok(vec![iu_finding(
            IU_UNINITIALIZED_KIND,
            &iu_label,
            "submodule status",
            command_failure_detail(
                "git submodule status",
                crate::semantic_value::<support::OptionalCodeCode, _>(status_output.code()).0,
            ),
        )]);
    }

    let status_stdout = status_output.stdout_lossy();
    let status = parse_iu_submodule_status(status_stdout.as_ref());
    let pin_findings = if status.class == IuSubmoduleStatusClass::Uninitialized {
        let head_args = git_rev_parse_head_args(iu_path);
        let head_output =
            support::run_output(OsStr::new("git"), &head_args, Some(workspace_root), true)?;
        let head_stdout = head_output.stdout_lossy();
        let head_probe = ProbeOutput {
            success: crate::semantic_value::<support::SuccessFlag, _>(head_output.success()).0,
            stdout: head_stdout.as_ref(),
        };
        iu_pin_findings_from_status(&iu_label, &status, Some(head_probe))
    }
    else {
        iu_pin_findings_from_status(&iu_label, &status, None)
    };
    if !pin_findings.is_empty() {
        return Ok(pin_findings);
    }

    let dirty_args = git_status_porcelain_args(iu_path);
    let dirty_output =
        support::run_output(OsStr::new("git"), &dirty_args, Some(workspace_root), true)?;
    if !crate::semantic_value::<support::SuccessFlag, _>(dirty_output.success()).0 {
        return Err(operational(command_failure_detail(
            "git status inside IU submodule",
            crate::semantic_value::<support::OptionalCodeCode, _>(dirty_output.code()).0,
        )));
    }

    let dirty_stdout = dirty_output.stdout_lossy();
    Ok(iu_clean_findings(&iu_label, dirty_stdout.as_ref()))
}

/// Discover Cargo's exact current host target triple through `rustc -vV`.
///
/// # Contract
/// - ensures: returns the unmodified `host: <triple>` payload from rustc.
/// - provides: the exact `--filter-platform` argument used by Cargo metadata.
/// - fails: returns an operational error when rustc fails or omits the host
///   line.
/// - panics: none.
///
/// # Errors
/// Returns support-runner errors, a rustc status failure, or a missing-host
/// operational error.
fn current_host_triple() -> Result<String, GateError>
{
    let args = [OsString::from("-vV")];
    let output = support::run_output(OsStr::new("rustc"), &args, None, false)?;
    if !crate::semantic_value::<support::SuccessFlag, _>(output.success()).0 {
        return Err(operational(command_failure_detail(
            RUSTC_HOST_SOURCE,
            crate::semantic_value::<support::OptionalCodeCode, _>(output.code()).0,
        )));
    }
    let stdout = output.stdout_lossy();
    parse_rustc_host_triple(stdout.as_ref())
}

/// Parse the `host:` row from `rustc -vV` output.
///
/// # Contract
/// - requires: `stdout` is the standard output of `rustc -vV`.
/// - ensures: returns the exact nonempty host triple after `host:`.
/// - provides: a pure seam for host-discovery fixtures.
/// - fails: returns an operational error when the host row is absent or blank.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Operational`] when no usable host triple is present.
fn parse_rustc_host_triple<'semantic, Stdout>(stdout: Stdout) -> Result<String, GateError>
where
    Stdout: Into<StdoutText<'semantic>>,
{
    let stdout = stdout.into().0;
    for line in stdout.lines() {
        let Some(host) = line.strip_prefix("host:")
        else {
            continue;
        };
        let trimmed = host.trim();
        if trimmed.is_empty() {
            return Err(operational("rustc host triple is empty"));
        }
        return Ok(String::from(trimmed));
    }
    Err(operational("rustc -vV output missing host triple"))
}

/// Return forbidden packages reachable in the metadata default graph.
///
/// # Contract
/// - requires: `metadata_json` is a Cargo metadata JSON payload.
/// - ensures: returns forbidden package names reachable from workspace members
///   over normal/build edges in the declared forbidden-set order.
/// - provides: pure package-name extraction for the default graph gate.
/// - fails: returns JSON or malformed-metadata errors for unusable input.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Json`] for syntax errors and
/// [`GateError::Operational`] for unsupported metadata shapes.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — syntax failure, transitive forbidden
///   reachability, and host-filtered target-only absence are killed by separate
///   exact fixture observations.
/// - witness: `project::tests::malformed_metadata_is_reported_as_json_error`
/// - witness: `project::tests::forbidden_transitive_package_is_reported`
/// - witness: `project::tests::non_host_only_forbidden_dependency_is_ignored`
/// - witness: `project::tests::dylint_driver_only_forbidden_dependency_is_ignored`
/// - witness: `project::tests::forbidden_package_through_non_exempt_member_is_reported`
fn forbidden_default_graph_packages<'semantic, Metadata>(
    metadata_json: Metadata
) -> Result<Vec<String>, GateError>
where
    Metadata: Into<MetadataJsonText<'semantic>>,
{
    let metadata_json = metadata_json.into().0;
    let value = serde_json::from_str::<serde_json::Value>(metadata_json).map_err(|source| {
        GateError::Json {
            source_name: String::from(CARGO_METADATA_SOURCE),
            source,
        }
    })?;
    let graph = metadata_graph(&value)?;
    let reachable_names = reachable_default_package_names(&graph)?;
    let mut hits = Vec::new();
    for forbidden in FORBIDDEN_DEFAULT_GRAPH_PACKAGES {
        let forbidden_name = ReachableDefaultPackageNameText::from(forbidden);
        if reachable_names.contains(&forbidden_name) {
            hits.push(String::from(forbidden));
        }
    }
    Ok(hits)
}

/// Parse the Cargo metadata graph fields needed by this policy.
///
/// # Contract
/// - requires: `value` is the parsed top-level Cargo metadata JSON value.
/// - ensures: borrows package names, workspace roots, and normal/build edges
///   without retaining unrelated metadata fields.
/// - provides: a normalized graph view for iterative reachability.
/// - fails: returns malformed-metadata errors for missing, duplicated, or
///   ill-typed graph fields.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Operational`] for malformed Cargo metadata.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — missing package-name and duplicate-id cases are
///   represented by the malformed-metadata branch; the missing-name fixture
///   kills the field-shape decision used by all required fields.
/// - witness: `project::tests::metadata_missing_package_name_is_malformed`
fn metadata_graph(value: &serde_json::Value) -> Result<MetadataGraph<'_>, GateError>
{
    let object = json_object(value, "top-level metadata value")?;
    let package_items = json_array_field(object, "packages")?;
    let workspace_items = json_array_field(object, "workspace_members")?;
    let resolve = json_object_field(object, "resolve")?;
    let node_items = json_array_field(resolve, "nodes")?;

    let mut package_names = BTreeMap::new();
    for package in package_items {
        let package_object = json_object(package, "packages[]")?;
        let id = json_string_field(package_object, "id")?;
        let name = json_string_field(package_object, "name")?;
        let id_text: &str = id.into();
        let name_text: &str = name.into();
        if package_names.insert(id_text, name_text).is_some() {
            return Err(malformed_metadata(format!(
                "duplicate package id `{id_text}`"
            )));
        }
    }

    let mut roots = Vec::new();
    for member in workspace_items {
        let Some(root) = member.as_str()
        else {
            return Err(malformed_metadata("workspace_members[] must be a string"));
        };
        roots.push(root);
    }

    let mut dependencies = BTreeMap::new();
    for node in node_items {
        let node_object = json_object(node, "resolve.nodes[]")?;
        let id = json_string_field(node_object, "id")?;
        let dep_items = json_array_field(node_object, "deps")?;
        let mut included_deps = Vec::new();
        for dep in dep_items {
            let dep_object = json_object(dep, "resolve.nodes[].deps[]")?;
            let package_id = json_string_field(dep_object, "pkg")?;
            let package_id_text: &str = package_id.into();
            let dep_kind_items = json_array_field(dep_object, "dep_kinds")?;
            if dep_reaches_default_graph(dep_kind_items).map(|v| v.0)? {
                included_deps.push(package_id_text);
            }
        }
        let id_text: &str = id.into();
        if dependencies.insert(id_text, included_deps).is_some() {
            return Err(malformed_metadata(format!(
                "duplicate resolve node id `{id_text}`"
            )));
        }
    }

    Ok(MetadataGraph {
        package_names,
        dependencies,
        roots,
    })
}

/// Return whether one Cargo dependency-kind record reaches normal/build edges.
///
/// # Contract
/// - requires: `dep_kind` is one entry from Cargo metadata `dep_kinds`.
/// - ensures: returns true only for normal (`null` or `\"normal\"`) and build
///   dependency kinds, matching the retained `-e normal,build` policy.
/// - provides: the edge filter used before graph reachability.
/// - fails: returns malformed-metadata errors for non-object or non-string kind
///   records.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Operational`] when the dependency-kind record is not a
/// supported Cargo metadata object.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the transitive fixture uses both `null` normal
///   and `\"build\"` edges, killing the retained inclusion boundary.
/// - witness: `project::tests::forbidden_transitive_package_is_reported`
fn dep_kind_reaches_default_graph(
    dep_kind: &serde_json::Value
) -> Result<DepKindReachesDefaultGraphFlag, GateError>
{
    let object = json_object(dep_kind, "resolve.nodes[].deps[].dep_kinds[]")?;
    let Some(kind) = object.get("kind")
    else {
        return Err(malformed_metadata("dep_kinds[] missing `kind`"));
    };
    match *kind {
        | serde_json::Value::Null => Ok(DepKindReachesDefaultGraphFlag(true)),
        | serde_json::Value::String(ref name) if name == "normal" || name == "build" => {
            Ok(DepKindReachesDefaultGraphFlag(true))
        },
        | serde_json::Value::String(_) => Ok(DepKindReachesDefaultGraphFlag(false)),
        | _ => Err(malformed_metadata(
            "dep_kinds[].kind must be null or a string",
        )),
    }
}

/// Return package names reachable from workspace roots over included edges.
///
/// # Contract
/// - requires: `graph` came from [`metadata_graph`].
/// - ensures: returns every package name reachable from a workspace member at
///   most once, without following edges that originate from the exempt
///   tooling-only Dylint driver member.
/// - provides: the graph traversal for default dependency validation.
/// - fails: returns malformed-metadata errors for roots or edges that reference
///   missing package records.
/// - panics: none.
/// - intension: traversal is iterative and uses a visited set keyed by package
///   id, so cycles cannot recurse or duplicate results.
///
/// # Errors
/// Returns [`GateError::Operational`] for inconsistent Cargo metadata graph
/// references.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the transitive fixture distinguishes root-only
///   validation from full edge traversal by placing the forbidden package
///   beyond an intermediate node.
/// - witness: `project::tests::forbidden_transitive_package_is_reported`
/// - witness: `project::tests::dylint_driver_only_forbidden_dependency_is_ignored`
/// - witness: `project::tests::forbidden_package_through_non_exempt_member_is_reported`
fn reachable_default_package_names<'metadata>(
    graph: &MetadataGraph<'metadata>
) -> Result<BTreeSet<ReachableDefaultPackageNameText<'metadata>>, GateError>
{
    let mut pending = Vec::new();
    for root in &graph.roots {
        if !graph.package_names.contains_key(*root) {
            return Err(malformed_metadata(format!(
                "workspace member `{root}` has no package record"
            )));
        }
        pending.push(*root);
    }

    let mut visited_ids = BTreeSet::new();
    let mut reachable_names = BTreeSet::new();
    while let Some(package_id) = pending.pop() {
        if !visited_ids.insert(package_id) {
            continue;
        }
        let Some(package_name) = graph.package_names.get(package_id)
        else {
            return Err(malformed_metadata(format!(
                "resolve edge references unknown package `{package_id}`"
            )));
        };
        reachable_names.insert(ReachableDefaultPackageNameText::from(*package_name));
        // Edges originating from the nightly-only rustc_private Dylint driver
        // are tooling-only and outside the production default graph policy.
        if *package_name == EXEMPT_DEFAULT_GRAPH_MEMBER {
            continue;
        }
        if let Some(children) = graph.dependencies.get(package_id) {
            for child in children {
                pending.push(*child);
            }
        }
    }
    Ok(reachable_names)
}

/// Convert forbidden package names into stable findings.
fn default_graph_findings(hits: &[String]) -> Vec<Finding>
{
    let mut findings = Vec::new();
    for hit in hits {
        findings.push(Finding::new(
            DEFAULT_GRAPH_FINDING_KIND,
            "",
            CARGO_METADATA_SOURCE,
            hit.clone(),
            "default normal/build workspace graph pulls a forbidden tree-sitter-family crate; keep tree-sitter behind the parity-only path",
        ));
    }
    findings
}

/// Extract the recorded SHA from the remainder of a submodule status line.
fn leading_status_sha<Characters>(characters: Characters) -> Option<String>
where
    Characters: IntoIterator<Item = char>,
{
    let mut sha = String::new();
    let mut started = false;
    for character in characters {
        if character.is_ascii_hexdigit() {
            sha.push(character);
            started = true;
        }
        else if started || !character.is_whitespace() {
            break;
        }
    }
    (!sha.is_empty()).then_some(sha)
}

/// Return IU pin findings for already-captured status and optional HEAD probes.
///
/// # Contract
/// - requires: `iu_path` is the diagnostic path label, `status_stdout` is a
///   submodule-status capture, and `head_probe` is supplied when the caller
///   wants the `-` status split by on-disk content.
/// - ensures: returns at most one status finding and returns no finding for a
///   clean or unknown-compatible prefix.
/// - provides: pure IU status validation with the retained distinct `-`, `+`,
///   and `U` diagnostics.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — status-class fixtures distinguish missing
///   registration, sibling-deinit, fresh uninitialized, drifted, conflicted,
///   and clean states.
/// - witness: `project::tests::unregistered_submodule_reports_missing_registration`
/// - witness: `project::tests::uninitialized_submodule_with_recorded_content_reports_sibling_hint`
/// - witness: `project::tests::uninitialized_submodule_without_content_reports_init_hint`
/// - witness: `project::tests::drifted_submodule_reports_plus_status`
/// - witness: `project::tests::conflicted_submodule_reports_u_status`
/// - witness: `project::tests::clean_submodule_status_reports_no_pin_finding`
#[cfg(test)]
pub(crate) fn iu_pin_status_findings<'semantic, IuPath, StatusStdout>(
    iu_path: IuPath,
    status_stdout: StatusStdout,
    head_probe: Option<ProbeOutput<'_>>,
) -> Vec<Finding>
where
    IuPath: Into<IuPathText<'semantic>>,
    StatusStdout: Into<StatusStdoutText<'semantic>>,
{
    let status_stdout = status_stdout.into().0;
    let iu_path = iu_path.into().0;
    let status = parse_iu_submodule_status(status_stdout);
    iu_pin_findings_from_status(iu_path, &status, head_probe)
}

/// Parse the first `git submodule status` line into an IU status record.
///
/// # Contract
/// - requires: `stdout` is the exact stdout captured from `git submodule status
///   -- <iu-path>`.
/// - ensures: classifies the retained `-`, `+`, `U`, clean-space, empty, and
///   fallback prefix states without indexing into the string.
/// - provides: the pure parser shared by live IU validation and fixture tests.
/// - panics: none.
/// - intension: only the first output line is classified, matching the single
///   path passed to the Git command.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — empty, `-`, `+`, `U`, and clean-space prefixes
///   are all distinguished by exact status fixtures.
/// - witness: `project::tests::unregistered_submodule_reports_missing_registration`
/// - witness: `project::tests::uninitialized_submodule_without_content_reports_init_hint`
/// - witness: `project::tests::drifted_submodule_reports_plus_status`
/// - witness: `project::tests::conflicted_submodule_reports_u_status`
/// - witness: `project::tests::clean_submodule_status_reports_no_pin_finding`
pub(crate) fn parse_iu_submodule_status<'semantic, Stdout>(stdout: Stdout) -> IuSubmoduleStatus
where
    Stdout: Into<StdoutText<'semantic>>,
{
    let stdout = stdout.into().0;
    if stdout.trim().is_empty() {
        return IuSubmoduleStatus {
            class: IuSubmoduleStatusClass::NotRegistered,
            recorded_sha: None,
        };
    }

    let Some(line) = stdout.lines().next()
    else {
        return IuSubmoduleStatus {
            class: IuSubmoduleStatusClass::NotRegistered,
            recorded_sha: None,
        };
    };
    let mut characters = line.chars();
    let Some(marker) = characters.next()
    else {
        return IuSubmoduleStatus {
            class: IuSubmoduleStatusClass::NotRegistered,
            recorded_sha: None,
        };
    };
    let class = match marker {
        | ' ' => IuSubmoduleStatusClass::Clean,
        | '-' => IuSubmoduleStatusClass::Uninitialized,
        | '+' => IuSubmoduleStatusClass::Drifted,
        | 'U' => IuSubmoduleStatusClass::Conflicted,
        | other => IuSubmoduleStatusClass::Other(other),
    };

    IuSubmoduleStatus {
        class,
        recorded_sha: leading_status_sha(characters),
    }
}

/// Return IU pin findings for a parsed status record.
fn iu_pin_findings_from_status<'semantic, IuPath>(
    iu_path: IuPath,
    status: &IuSubmoduleStatus,
    head_probe: Option<ProbeOutput<'_>>,
) -> Vec<Finding>
where
    IuPath: Into<IuPathText<'semantic>>,
{
    let iu_path = iu_path.into().0;
    match status.class {
        | IuSubmoduleStatusClass::NotRegistered => vec![iu_finding(
            IU_UNINITIALIZED_KIND,
            iu_path,
            "submodule status",
            "not a registered submodule",
        )],
        | IuSubmoduleStatusClass::Uninitialized => {
            let detail = if content_at_recorded_pin(status, head_probe).into().0 {
                "not initialized; content is on disk at the recorded pin but the submodule is unregistered: a sibling linked worktree likely ran git submodule deinit; recover from the primary checkout with git submodule init then git submodule update"
            }
            else {
                "not initialized; run: git submodule update --init"
            };
            vec![iu_finding(IU_UNINITIALIZED_KIND, iu_path, "-", detail)]
        },
        | IuSubmoduleStatusClass::Drifted => vec![iu_finding(
            IU_DRIFTED_KIND,
            iu_path,
            "+",
            "checkout differs from the recorded pin; commit the intended pin bump, or run: git submodule update",
        )],
        | IuSubmoduleStatusClass::Conflicted => vec![iu_finding(
            IU_CONFLICTED_KIND,
            iu_path,
            "U",
            "has merge conflicts; resolve them first",
        )],
        | IuSubmoduleStatusClass::Clean | IuSubmoduleStatusClass::Other(_) => Vec::new(),
    }
}

/// Return whether an uninitialized status still has content at the recorded
/// pin.
fn content_at_recorded_pin(
    status: &IuSubmoduleStatus,
    head_probe: Option<ProbeOutput<'_>>,
) -> impl Into<ContentAtRecordedPinFlag>
{
    let Some(recorded) = status.recorded_sha.as_deref()
    else {
        return false;
    };
    let Some(probe) = head_probe
    else {
        return false;
    };
    probe.success && probe.stdout.trim() == recorded
}

/// Build one stable IU finding.
fn iu_finding<'semantic, Kind, IuPath, Declaration, Detail>(
    kind: Kind,
    iu_path: IuPath,
    declaration: Declaration,
    detail: Detail,
) -> Finding
where
    Kind: Into<KindText<'semantic>>,
    IuPath: Into<IuPathText<'semantic>>,
    Declaration: Into<DeclarationText<'semantic>>,
    Detail: Into<String>,
{
    let iu_path = iu_path.into().0;
    let declaration = declaration.into().0;
    let detail = detail.into();
    let kind = kind.into().0;
    Finding::new(kind, "", iu_path, declaration, detail)
}

/// Return a deterministic text label for a path argument.
fn path_label(path: &Path) -> String
{
    path.to_string_lossy().into_owned()
}

/// Build a stable command-failure detail from a live-streamed command status.
fn command_failure_detail<'semantic, Command, Code>(
    command: Command,
    code: Code,
) -> String
where
    Command: Into<CommandText<'semantic>>,
    Code: Into<CodeExitCode>,
{
    let code = code.into().0;
    let command = command.into().0;
    support::command_status_detail(command, code)
}

/// Validate the default workspace graph from an already-captured metadata JSON.
///
/// # Contract
/// - requires: `metadata_json` is intended to be Cargo metadata format-version
///   1 output for a workspace, already filtered to the relevant host platform
///   when target-specific dependencies matter.
/// - ensures: returns deterministic findings for forbidden package names
///   reachable over normal/build edges from workspace roots.
/// - provides: a pure validator for fixture tests and callers that already own
///   metadata capture.
/// - fails: returns JSON errors for unparseable input and operational errors
///   for unsupported or incomplete Cargo metadata shapes.
/// - panics: none.
/// - intension: traversal is iterative and visits each package id at most once.
///
/// # Errors
/// Returns [`GateError::Json`] for invalid JSON and [`GateError::Operational`]
/// for missing or ill-typed Cargo metadata fields.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — syntax failure, missing fields, transitive
///   forbidden dependency reachability, and host-filtered target-only absence
///   are distinguished by focused fixtures.
/// - witness: `project::tests::malformed_metadata_is_reported_as_json_error`
/// - witness: `project::tests::metadata_missing_package_name_is_malformed`
/// - witness: `project::tests::forbidden_transitive_package_is_reported`
/// - witness: `project::tests::non_host_only_forbidden_dependency_is_ignored`
/// - witness: `project::tests::dylint_driver_only_forbidden_dependency_is_ignored`
/// - witness: `project::tests::forbidden_package_through_non_exempt_member_is_reported`
pub(crate) fn validate_default_dependency_graph_metadata<'semantic, Metadata>(
    metadata_json: Metadata
) -> GateResult
where
    Metadata: Into<MetadataJsonText<'semantic>>,
{
    let metadata_json = metadata_json.into().0;
    let hits = forbidden_default_graph_packages(metadata_json)?;
    Ok(default_graph_findings(&hits))
}

/// Return `value` as a JSON object or a malformed-metadata error.
fn json_object<'semantic, 'value, Context>(
    value: &'value serde_json::Value,
    context: Context,
) -> Result<&'value serde_json::Map<String, serde_json::Value>, GateError>
where
    Context: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    value
        .as_object()
        .ok_or_else(|| malformed_metadata(format!("{context} must be an object")))
}

/// Return one object field as a JSON array.
fn json_array_field<'semantic, 'value, Field>(
    object: &'value serde_json::Map<String, serde_json::Value>,
    field: Field,
) -> Result<&'value [serde_json::Value], GateError>
where
    Field: Into<FieldText<'semantic>>,
{
    let field = field.into().0;
    object
        .get(field)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| malformed_metadata(format!("missing array field `{field}`")))
}

/// Return one object field as a JSON object.
fn json_object_field<'semantic, 'value, Field>(
    object: &'value serde_json::Map<String, serde_json::Value>,
    field: Field,
) -> Result<&'value serde_json::Map<String, serde_json::Value>, GateError>
where
    Field: Into<FieldText<'semantic>>,
{
    let field = field.into().0;
    object
        .get(field)
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| malformed_metadata(format!("missing object field `{field}`")))
}

/// Build a stable malformed-metadata operational error.
fn malformed_metadata<Detail>(detail: Detail) -> GateError
where
    Detail: Into<String>,
{
    let detail: String = detail.into();
    operational(format!("malformed cargo metadata: {detail}"))
}

/// Build a stable operational error.
fn operational<Detail>(detail: Detail) -> GateError
where
    Detail: Into<String>,
{
    let detail: String = detail.into();
    GateError::Operational { detail }
}

/// Return IU clean-tree findings from `git status --porcelain` stdout.
///
/// # Contract
/// - requires: `porcelain_stdout` is captured from `git -C <iu-path> status
///   --porcelain`.
/// - ensures: returns no findings for empty porcelain output and one read-only
///   violation for any nonempty output.
/// - provides: pure clean-tree validation for the pinned IU dependency.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — empty and nonempty porcelain captures are the
///   only semantic boundary for the retained clean-tree check.
/// - witness: `project::tests::clean_submodule_status_reports_no_pin_finding`
/// - witness: `project::tests::dirty_submodule_reports_read_only_violation`
pub(crate) fn iu_clean_findings<'semantic, IuPath, PorcelainStdout>(
    iu_path: IuPath,
    porcelain_stdout: PorcelainStdout,
) -> Vec<Finding>
where
    IuPath: Into<IuPathText<'semantic>>,
    PorcelainStdout: Into<PorcelainStdoutText<'semantic>>,
{
    let porcelain_stdout = porcelain_stdout.into().0;
    let iu_path = iu_path.into().0;
    if porcelain_stdout.trim().is_empty() {
        Vec::new()
    }
    else {
        vec![iu_finding(
            IU_DIRTY_KIND,
            iu_path,
            "status --porcelain",
            "has local modifications; the pinned upstream is read-only in gandr; land the change upstream, then bump the pin",
        )]
    }
}

/// Return one object field as a JSON string slice.
fn json_string_field<'semantic, 'value, Field>(
    object: &'value serde_json::Map<String, serde_json::Value>,
    field: Field,
) -> Result<JsonFieldText<'value>, GateError>
where
    Field: Into<FieldText<'semantic>>,
{
    let field = field.into().0;
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(JsonFieldText)
        .ok_or_else(|| malformed_metadata(format!("missing string field `{field}`")))
}

/// Return whether any dependency kind reaches the default normal/build graph.
fn dep_reaches_default_graph(
    dep_kind_items: &[serde_json::Value]
) -> Result<DepReachesDefaultGraphFlag, GateError>
{
    let mut reaches = false;
    for dep_kind in dep_kind_items {
        if dep_kind_reaches_default_graph(dep_kind).map(|value| value.0)? {
            reaches = true;
        }
    }
    Ok(DepReachesDefaultGraphFlag(reaches))
}

/// Build the fixed Cargo metadata argument vector for a host target.
fn cargo_metadata_args<'semantic, HostTriple>(host_triple: HostTriple) -> [OsString; 5]
where
    HostTriple: Into<HostTripleText<'semantic>>,
{
    let host_triple = host_triple.into().0;
    [
        OsString::from("metadata"),
        OsString::from("--format-version"),
        OsString::from("1"),
        OsString::from("--filter-platform"),
        OsString::from(host_triple),
    ]
}

/// Build `git submodule status -- <iu_path>` arguments.
fn git_submodule_status_args(iu_path: &Path) -> [OsString; 4]
{
    [
        OsString::from("submodule"),
        OsString::from("status"),
        OsString::from("--"),
        iu_path.as_os_str().to_os_string(),
    ]
}

/// Build `git -C <iu_path> rev-parse HEAD` arguments.
fn git_rev_parse_head_args(iu_path: &Path) -> [OsString; 4]
{
    [
        OsString::from("-C"),
        iu_path.as_os_str().to_os_string(),
        OsString::from("rev-parse"),
        OsString::from("HEAD"),
    ]
}

/// Build `git -C <iu_path> status --porcelain` arguments.
fn git_status_porcelain_args(iu_path: &Path) -> [OsString; 4]
{
    [
        OsString::from("-C"),
        iu_path.as_os_str().to_os_string(),
        OsString::from("status"),
        OsString::from("--porcelain"),
    ]
}

/// Fixture tests for pure project-gate parsers and validators.
#[cfg(test)]
mod tests
{

    use std::env;
    use std::path::Path;
    use std::path::PathBuf;

    use super::*;

    /// Minimal package id for metadata fixtures.
    const ROOT_ID: &str = "path+file:///workspace#root@0.1.0";

    /// Minimal intermediate package id for metadata fixtures.
    const MID_ID: &str = "registry+https://example.invalid#middle@0.1.0";

    /// Minimal forbidden package id for metadata fixtures.
    const REGEX_ID: &str = "registry+https://example.invalid#regex@1.0.0";

    /// Minimal exempt Dylint driver package id for metadata fixtures.
    const DYLINT_ID: &str = "path+file:///workspace#workflow-dylint@0.1.0";

    /// Representative IU mount path used by pure status fixtures.
    const IU_PATH: &str = "metatheory/upstream/internal-univalence";

    /// Invalid metadata JSON reports the typed JSON error variant.
    #[test]
    fn malformed_metadata_is_reported_as_json_error()
    {
        assert!(matches!(
            forbidden_default_graph_packages("{"),
            Err(GateError::Json { .. })
        ));
    }

    /// Metadata missing a package name is rejected as malformed graph data.
    #[test]
    fn metadata_missing_package_name_is_malformed()
    {
        let metadata = format!(
            r#"{{
                "packages": [{{"id": "{ROOT_ID}"}}],
                "workspace_members": ["{ROOT_ID}"],
                "resolve": {{"nodes": [{{"id": "{ROOT_ID}", "deps": []}}]}}
            }}"#
        );
        assert!(matches!(
            forbidden_default_graph_packages(&metadata),
            Err(GateError::Operational { detail })
                if detail.contains("missing string field `name`")
        ));
    }

    /// Cargo metadata capture is filtered to the exact current host target.
    #[test]
    fn cargo_metadata_args_include_host_filter_platform()
    {
        assert_eq!(cargo_metadata_args("aarch64-apple-darwin"), [
            OsString::from("metadata"),
            OsString::from("--format-version"),
            OsString::from("1"),
            OsString::from("--filter-platform"),
            OsString::from("aarch64-apple-darwin"),
        ]);
    }

    /// Rustc host discovery parses the exact host triple row.
    #[test]
    fn rustc_host_triple_parser_accepts_exact_host_row() -> Result<(), GateError>
    {
        let triple = parse_rustc_host_triple(
            "rustc 1.97.1 (000000000 2026-01-01)\nhost: aarch64-apple-darwin\n",
        )?;
        assert_eq!("aarch64-apple-darwin", triple);
        Ok(())
    }

    /// A forbidden package present only through a filtered non-host edge is
    /// ignored.
    #[test]
    fn non_host_only_forbidden_dependency_is_ignored() -> Result<(), GateError>
    {
        let metadata = non_host_only_regex_metadata();
        let findings = validate_default_dependency_graph_metadata(&metadata)?;
        assert!(findings.is_empty());
        Ok(())
    }

    /// A forbidden package behind a normal/build transitive edge is reported.
    #[test]
    fn forbidden_transitive_package_is_reported() -> Result<(), GateError>
    {
        let metadata = transitive_regex_metadata();
        let findings = validate_default_dependency_graph_metadata(&metadata)?;
        assert_eq!(findings, vec![Finding::new(
            DEFAULT_GRAPH_FINDING_KIND,
            "",
            CARGO_METADATA_SOURCE,
            "regex",
            "default normal/build workspace graph pulls a forbidden tree-sitter-family crate; keep tree-sitter behind the parity-only path",
        )]);
        Ok(())
    }

    /// A forbidden package reachable only through the exempt nightly-only
    /// `rustc_private` Dylint driver is ignored.
    #[test]
    fn dylint_driver_only_forbidden_dependency_is_ignored() -> Result<(), GateError>
    {
        let metadata = dylint_only_regex_metadata();
        let findings = validate_default_dependency_graph_metadata(&metadata)?;
        assert!(findings.is_empty());
        Ok(())
    }

    /// A forbidden package reachable through a non-exempt member is reported
    /// even when the exempt Dylint driver also reaches it.
    #[test]
    fn forbidden_package_through_non_exempt_member_is_reported() -> Result<(), GateError>
    {
        let metadata = dylint_and_root_regex_metadata();
        let findings = validate_default_dependency_graph_metadata(&metadata)?;
        assert_eq!(findings, vec![Finding::new(
            DEFAULT_GRAPH_FINDING_KIND,
            "",
            CARGO_METADATA_SOURCE,
            "regex",
            "default normal/build workspace graph pulls a forbidden tree-sitter-family crate; keep tree-sitter behind the parity-only path",
        )]);
        Ok(())
    }

    /// Empty submodule status output reports missing registration.
    #[test]
    fn unregistered_submodule_reports_missing_registration()
    {
        assert_eq!(iu_pin_status_findings(IU_PATH, "", None), vec![
            Finding::new(
                IU_UNINITIALIZED_KIND,
                "",
                IU_PATH,
                "submodule status",
                "not a registered submodule",
            )
        ]);
    }

    /// A `-` status with matching mount HEAD reports the sibling-deinit hint.
    #[test]
    fn uninitialized_submodule_with_recorded_content_reports_sibling_hint() -> Result<(), GateError>
    {
        let findings = iu_pin_status_findings(
            IU_PATH,
            "-abcdef1234567890 metatheory/upstream/internal-univalence\n",
            Some(ProbeOutput {
                success: true,
                stdout: "abcdef1234567890\n",
            }),
        );
        let Some(finding) = findings.first()
        else {
            return Err(GateError::operational("expected one IU finding"));
        };
        assert_eq!(1, findings.len());
        assert_eq!("-", finding.declaration);
        assert!(finding.detail.contains("sibling linked worktree"));
        Ok(())
    }

    /// A `-` status without matching content reports the init command hint.
    #[test]
    fn uninitialized_submodule_without_content_reports_init_hint()
    {
        let findings = iu_pin_status_findings(
            IU_PATH,
            "-abcdef1234567890 metatheory/upstream/internal-univalence\n",
            Some(ProbeOutput {
                success: false,
                stdout: "",
            }),
        );
        assert_eq!(findings, vec![Finding::new(
            IU_UNINITIALIZED_KIND,
            "",
            IU_PATH,
            "-",
            "not initialized; run: git submodule update --init",
        )]);
    }

    /// A `+` status reports recorded-pin drift.
    #[test]
    fn drifted_submodule_reports_plus_status()
    {
        assert_eq!(
            iu_pin_status_findings(
                IU_PATH,
                "+abcdef1234567890 metatheory/upstream/internal-univalence\n",
                None,
            ),
            vec![Finding::new(
                IU_DRIFTED_KIND,
                "",
                IU_PATH,
                "+",
                "checkout differs from the recorded pin; commit the intended pin bump, or run: git submodule update",
            )]
        );
    }

    /// A `U` status reports merge conflicts.
    #[test]
    fn conflicted_submodule_reports_u_status()
    {
        assert_eq!(
            iu_pin_status_findings(
                IU_PATH,
                "Uabcdef1234567890 metatheory/upstream/internal-univalence\n",
                None,
            ),
            vec![Finding::new(
                IU_CONFLICTED_KIND,
                "",
                IU_PATH,
                "U",
                "has merge conflicts; resolve them first",
            )]
        );
    }

    /// A clean status line has no pin finding before the dirty check.
    #[test]
    fn clean_submodule_status_reports_no_pin_finding()
    {
        assert_eq!(
            Vec::<Finding>::new(),
            iu_pin_status_findings(
                IU_PATH,
                " abcdef1234567890 metatheory/upstream/internal-univalence\n",
                None,
            )
        );
    }

    /// Nonempty porcelain output reports the read-only upstream violation.
    #[test]
    fn dirty_submodule_reports_read_only_violation()
    {
        assert_eq!(
            iu_clean_findings(IU_PATH, " M src/Internal/Graph.agda\n"),
            vec![Finding::new(
                IU_DIRTY_KIND,
                "",
                IU_PATH,
                "status --porcelain",
                "has local modifications; the pinned upstream is read-only in gandr; land the change upstream, then bump the pin",
            )]
        );
    }

    /// Malformed Cargo metadata reports exact shape failures for duplicate IDs.
    #[test]
    fn metadata_rejects_duplicate_package_and_node_ids()
    {
        assert_malformed_contains(
            &format!(
                r#"{{
                    "packages": [
                        {{"id": "{ROOT_ID}", "name": "root"}},
                        {{"id": "{ROOT_ID}", "name": "root-again"}}
                    ],
                    "workspace_members": ["{ROOT_ID}"],
                    "resolve": {{"nodes": [{{"id": "{ROOT_ID}", "deps": []}}]}}
                }}"#
            ),
            "duplicate package id",
        );
        assert_malformed_contains(
            &format!(
                r#"{{
                    "packages": [{{"id": "{ROOT_ID}", "name": "root"}}],
                    "workspace_members": ["{ROOT_ID}"],
                    "resolve": {{
                        "nodes": [
                            {{"id": "{ROOT_ID}", "deps": []}},
                            {{"id": "{ROOT_ID}", "deps": []}}
                        ]
                    }}
                }}"#
            ),
            "duplicate resolve node id",
        );
    }

    /// Malformed metadata rejects bad roots, dependency edges, and kind shapes.
    #[test]
    fn metadata_rejects_bad_workspace_and_dependency_shapes()
    {
        assert_malformed_contains(
            &format!(
                r#"{{
                    "packages": [{{"id": "{ROOT_ID}", "name": "root"}}],
                    "workspace_members": [42],
                    "resolve": {{"nodes": [{{"id": "{ROOT_ID}", "deps": []}}]}}
                }}"#
            ),
            "workspace_members[] must be a string",
        );
        assert_malformed_contains(
            &format!(
                r#"{{
                    "packages": [{{"id": "{ROOT_ID}", "name": "root"}}],
                    "workspace_members": ["{MID_ID}"],
                    "resolve": {{"nodes": [{{"id": "{ROOT_ID}", "deps": []}}]}}
                }}"#
            ),
            "workspace member",
        );
        assert_malformed_contains(
            &format!(
                r#"{{
                    "packages": [{{"id": "{ROOT_ID}", "name": "root"}}],
                    "workspace_members": ["{ROOT_ID}"],
                    "resolve": {{
                        "nodes": [{{
                            "id": "{ROOT_ID}",
                            "deps": [{{
                                "pkg": "{MID_ID}",
                                "dep_kinds": [{{"kind": null, "target": null}}]
                            }}]
                        }}]
                    }}
                }}"#
            ),
            "resolve edge references unknown package",
        );
        assert_malformed_contains(
            &format!(
                r#"{{
                    "packages": [
                        {{"id": "{ROOT_ID}", "name": "root"}},
                        {{"id": "{REGEX_ID}", "name": "regex"}}
                    ],
                    "workspace_members": ["{ROOT_ID}"],
                    "resolve": {{
                        "nodes": [{{
                            "id": "{ROOT_ID}",
                            "deps": [{{
                                "pkg": "{REGEX_ID}",
                                "dep_kinds": [{{}}]
                            }}]
                        }}]
                    }}
                }}"#
            ),
            "dep_kinds[] missing `kind`",
        );
        assert_malformed_contains(
            &format!(
                r#"{{
                    "packages": [
                        {{"id": "{ROOT_ID}", "name": "root"}},
                        {{"id": "{REGEX_ID}", "name": "regex"}}
                    ],
                    "workspace_members": ["{ROOT_ID}"],
                    "resolve": {{
                        "nodes": [{{
                            "id": "{ROOT_ID}",
                            "deps": [{{
                                "pkg": "{REGEX_ID}",
                                "dep_kinds": [{{"kind": 7, "target": null}}]
                            }}]
                        }}]
                    }}
                }}"#
            ),
            "dep_kinds[].kind must be null or a string",
        );
    }

    /// Dev-only edges and missing resolve rows stay outside the default graph.
    #[test]
    fn metadata_ignores_dev_edges_and_missing_dependency_rows() -> Result<(), GateError>
    {
        let dev_only = format!(
            r#"{{
                "packages": [
                    {{"id": "{ROOT_ID}", "name": "root"}},
                    {{"id": "{REGEX_ID}", "name": "regex"}}
                ],
                "workspace_members": ["{ROOT_ID}"],
                "resolve": {{
                    "nodes": [{{
                        "id": "{ROOT_ID}",
                        "deps": [{{
                            "pkg": "{REGEX_ID}",
                            "dep_kinds": [{{"kind": "dev", "target": null}}]
                        }}]
                    }}]
                }}
            }}"#
        );
        let dev_findings = validate_default_dependency_graph_metadata(&dev_only)?;
        assert!(dev_findings.is_empty());

        let root_without_resolve_node = format!(
            r#"{{
                "packages": [{{"id": "{ROOT_ID}", "name": "root"}}],
                "workspace_members": ["{ROOT_ID}"],
                "resolve": {{"nodes": []}}
            }}"#
        );
        let root_findings = validate_default_dependency_graph_metadata(&root_without_resolve_node)?;
        assert!(root_findings.is_empty());
        Ok(())
    }

    /// Reachability is cycle-safe and findings use the declared package order.
    #[test]
    fn forbidden_findings_are_canonical_across_cycles() -> Result<(), GateError>
    {
        let tree_id = "registry+https://example.invalid#tree-sitter@0.1.0";
        let aho_id = "registry+https://example.invalid#aho-corasick@1.0.0";
        let metadata = format!(
            r#"{{
                "packages": [
                    {{"id": "{ROOT_ID}", "name": "root"}},
                    {{"id": "{MID_ID}", "name": "middle"}},
                    {{"id": "{REGEX_ID}", "name": "regex"}},
                    {{"id": "{tree_id}", "name": "tree-sitter"}},
                    {{"id": "{aho_id}", "name": "aho-corasick"}}
                ],
                "workspace_members": ["{ROOT_ID}"],
                "resolve": {{
                    "nodes": [
                        {{
                            "id": "{ROOT_ID}",
                            "deps": [
                                {{"pkg": "{MID_ID}", "dep_kinds": [{{"kind": null}}]}},
                                {{"pkg": "{aho_id}", "dep_kinds": [{{"kind": "normal"}}]}}
                            ]
                        }},
                        {{
                            "id": "{MID_ID}",
                            "deps": [
                                {{"pkg": "{ROOT_ID}", "dep_kinds": [{{"kind": "normal"}}]}},
                                {{"pkg": "{REGEX_ID}", "dep_kinds": [{{"kind": "build"}}]}},
                                {{"pkg": "{tree_id}", "dep_kinds": [{{"kind": "normal"}}]}}
                            ]
                        }},
                        {{"id": "{REGEX_ID}", "deps": []}},
                        {{"id": "{tree_id}", "deps": []}},
                        {{"id": "{aho_id}", "deps": []}}
                    ]
                }}
            }}"#
        );
        let findings = validate_default_dependency_graph_metadata(&metadata)?;
        let declarations: Vec<&str> = findings
            .iter()
            .map(|finding| finding.declaration.as_str())
            .collect();
        assert_eq!(declarations, vec!["tree-sitter", "regex", "aho-corasick"]);
        Ok(())
    }

    /// Host parsing rejects absent and blank host rows.
    #[test]
    fn rustc_host_triple_parser_rejects_missing_or_blank_host()
    {
        assert!(matches!(
            parse_rustc_host_triple("release: 1.97.1\n"),
            Err(GateError::Operational { detail })
                if detail.contains("missing host triple")
        ));
        assert!(matches!(
            parse_rustc_host_triple("host:   \n"),
            Err(GateError::Operational { detail })
                if detail.contains("host triple is empty")
        ));
    }

    /// IU status parsing uses only the first line and tolerates unknown
    /// prefixes.
    #[test]
    fn submodule_status_parser_respects_first_line_and_unknown_prefix()
    {
        assert_eq!(
            IuSubmoduleStatusClass::NotRegistered,
            parse_iu_submodule_status("\n+abcdef1234567890 ignored\n").class,
        );
        assert_eq!(
            IuSubmoduleStatusClass::Other('?'),
            parse_iu_submodule_status("?abcdef1234567890 ignored\n").class,
        );
        assert_eq!(
            None,
            parse_iu_submodule_status("+not-a-sha ignored\n").recorded_sha,
        );
    }

    /// IU content probes require both a recorded SHA and a successful HEAD
    /// read.
    #[test]
    fn submodule_content_probe_requires_recorded_sha_and_successful_head()
    {
        assert_eq!(
            iu_pin_status_findings(IU_PATH, "- metatheory/upstream/internal-univalence\n", None),
            vec![Finding::new(
                IU_UNINITIALIZED_KIND,
                "",
                IU_PATH,
                "-",
                "not initialized; run: git submodule update --init",
            )],
        );
        assert_eq!(
            iu_pin_status_findings(
                IU_PATH,
                "-abcdef1234567890 metatheory/upstream/internal-univalence\n",
                Some(ProbeOutput {
                    success: true,
                    stdout: "different\n",
                }),
            ),
            vec![Finding::new(
                IU_UNINITIALIZED_KIND,
                "",
                IU_PATH,
                "-",
                "not initialized; run: git submodule update --init",
            )],
        );
    }

    /// Whitespace-only porcelain output is clean and path labels are stable.
    #[test]
    fn clean_porcelain_and_path_labels_are_stable()
    {
        assert!(iu_clean_findings(IU_PATH, " \n\t").is_empty());
        assert_eq!("metatheory/iu", path_label(Path::new("metatheory/iu")));
    }

    /// Live Git submodule probes report clean and dirty IU states without
    /// remotes.
    #[test]
    fn check_iu_pin_uses_local_git_fixture_for_clean_and_dirty_states() -> Result<(), GateError>
    {
        let fixture = ProjectFixture::new("iu-pin-clean-dirty")?;
        let upstream = fixture.path().join("upstream");
        let repo = fixture.path().join("repo");
        support::HOST_FILESYSTEM
            .create_dir_all(&upstream)
            .map_err(|error| GateError::operational(error.to_string()))?;
        support::HOST_FILESYSTEM
            .create_dir_all(&repo)
            .map_err(|error| GateError::operational(error.to_string()))?;

        git(&upstream, ["init"])?;
        support::HOST_FILESYSTEM
            .write(upstream.join("README.md"), "clean\n")
            .map_err(|error| GateError::operational(error.to_string()))?;
        git(&upstream, ["add", "README.md"])?;
        git_commit(&upstream, "seed upstream")?;

        git(&repo, ["init"])?;
        git_os(&repo, [
            OsString::from("-c"),
            OsString::from("protocol.file.allow=always"),
            OsString::from("submodule"),
            OsString::from("add"),
            upstream.as_os_str().to_os_string(),
            OsString::from("deps/iu"),
        ])?;
        git(&repo, ["add", ".gitmodules", "deps/iu"])?;
        git_commit(&repo, "record submodule")?;

        let iu_path = Path::new("deps/iu");
        let pin_findings = check_iu_pin(&repo, iu_path)?;
        assert!(pin_findings.is_empty());

        support::HOST_FILESYSTEM
            .write(repo.join(iu_path).join("README.md"), "dirty\n")
            .map_err(|error| GateError::operational(error.to_string()))?;
        let findings = check_iu_pin(&repo, iu_path)?;
        assert_eq!(1, findings.len());
        let finding = findings
            .first()
            .ok_or_else(|| GateError::operational("missing dirty submodule finding"))?;
        assert_eq!(IU_DIRTY_KIND, finding.kind);
        Ok(())
    }

    /// Unregistered paths are reported from the live Git status probe.
    #[test]
    fn check_iu_pin_reports_unregistered_path_from_git_fixture() -> Result<(), GateError>
    {
        let fixture = ProjectFixture::new("iu-pin-unregistered")?;
        git(fixture.path(), ["init"])?;
        let findings = check_iu_pin(fixture.path(), Path::new("missing/iu"))?;
        assert_eq!(1, findings.len());
        let finding = findings
            .first()
            .ok_or_else(|| GateError::operational("missing unregistered submodule finding"))?;
        assert_eq!(IU_UNINITIALIZED_KIND, finding.kind);
        assert_eq!("submodule status", finding.declaration);
        Ok(())
    }

    /// The default IU check reports an unregistered symlink without traversing
    /// it.
    #[cfg(unix)]
    #[test]
    fn check_default_iu_pin_reports_unregistered_symlink_without_dirty_probe()
    -> Result<(), GateError>
    {
        let fixture = ProjectFixture::new("iu-pin-symlink")?;
        let repo = fixture.path().join("repo");
        let outside = fixture.path().join("outside");
        support::HOST_FILESYSTEM
            .create_dir_all(&repo)
            .map_err(|error| GateError::operational(error.to_string()))?;
        support::HOST_FILESYSTEM
            .create_dir_all(&outside)
            .map_err(|error| GateError::operational(error.to_string()))?;
        git(&repo, ["init"])?;

        let default_path = Path::new(DEFAULT_IU_PATH);
        let link = repo.join(default_path);
        let parent = link
            .parent()
            .ok_or_else(|| GateError::operational("default IU path has no parent"))?;
        support::HOST_FILESYSTEM
            .create_dir_all(parent)
            .map_err(|error| GateError::operational(error.to_string()))?;
        support::HOST_FILESYSTEM
            .symlink(&outside, &link)
            .map_err(|error| GateError::operational(error.to_string()))?;

        let findings = check_default_iu_pin(&repo)?;

        assert_eq!(1, findings.len());
        let finding = findings
            .first()
            .ok_or_else(|| GateError::operational("missing symlink submodule finding"))?;
        assert_eq!(IU_UNINITIALIZED_KIND, finding.kind);
        assert_eq!("submodule status", finding.declaration);
        Ok(())
    }

    /// Build a minimal filtered graph where root has only a non-host regex
    /// dependency.
    fn non_host_only_regex_metadata() -> String
    {
        format!(
            r#"{{
                "packages": [
                    {{
                        "id": "{ROOT_ID}",
                        "name": "root",
                        "dependencies": [{{
                            "name": "regex",
                            "req": "*",
                            "kind": null,
                            "target": "cfg(target_os = \"non_host\")"
                        }}]
                    }},
                    {{"id": "{REGEX_ID}", "name": "regex"}}
                ],
                "workspace_members": ["{ROOT_ID}"],
                "resolve": {{
                    "nodes": [
                        {{"id": "{ROOT_ID}", "deps": []}},
                        {{"id": "{REGEX_ID}", "deps": []}}
                    ]
                }}
            }}"#
        )
    }

    /// Build a minimal metadata graph with root -> middle -> regex edges.
    fn transitive_regex_metadata() -> String
    {
        format!(
            r#"{{
                "packages": [
                    {{"id": "{ROOT_ID}", "name": "root"}},
                    {{"id": "{MID_ID}", "name": "middle"}},
                    {{"id": "{REGEX_ID}", "name": "regex"}}
                ],
                "workspace_members": ["{ROOT_ID}"],
                "resolve": {{
                    "nodes": [
                        {{
                            "id": "{ROOT_ID}",
                            "deps": [{{
                                "pkg": "{MID_ID}",
                                "dep_kinds": [{{"kind": null, "target": null}}]
                            }}]
                        }},
                        {{
                            "id": "{MID_ID}",
                            "deps": [{{
                                "pkg": "{REGEX_ID}",
                                "dep_kinds": [{{"kind": "build", "target": null}}]
                            }}]
                        }},
                        {{"id": "{REGEX_ID}", "deps": []}}
                    ]
                }}
            }}"#
        )
    }

    /// Build a minimal metadata graph where only the exempt Dylint driver
    /// reaches regex.
    fn dylint_only_regex_metadata() -> String
    {
        format!(
            r#"{{
                "packages": [
                    {{"id": "{DYLINT_ID}", "name": "gandr-workflow-dylint"}},
                    {{"id": "{REGEX_ID}", "name": "regex"}}
                ],
                "workspace_members": ["{DYLINT_ID}"],
                "resolve": {{
                    "nodes": [
                        {{
                            "id": "{DYLINT_ID}",
                            "deps": [{{
                                "pkg": "{REGEX_ID}",
                                "dep_kinds": [{{"kind": null, "target": null}}]
                            }}]
                        }},
                        {{"id": "{REGEX_ID}", "deps": []}}
                    ]
                }}
            }}"#
        )
    }

    /// Build a minimal metadata graph where both the exempt Dylint driver and
    /// a non-exempt member reach regex.
    fn dylint_and_root_regex_metadata() -> String
    {
        format!(
            r#"{{
                "packages": [
                    {{"id": "{ROOT_ID}", "name": "root"}},
                    {{"id": "{DYLINT_ID}", "name": "gandr-workflow-dylint"}},
                    {{"id": "{REGEX_ID}", "name": "regex"}}
                ],
                "workspace_members": ["{ROOT_ID}", "{DYLINT_ID}"],
                "resolve": {{
                    "nodes": [
                        {{
                            "id": "{ROOT_ID}",
                            "deps": [{{
                                "pkg": "{REGEX_ID}",
                                "dep_kinds": [{{"kind": null, "target": null}}]
                            }}]
                        }},
                        {{
                            "id": "{DYLINT_ID}",
                            "deps": [{{
                                "pkg": "{REGEX_ID}",
                                "dep_kinds": [{{"kind": "build", "target": null}}]
                            }}]
                        }},
                        {{"id": "{REGEX_ID}", "deps": []}}
                    ]
                }}
            }}"#
        )
    }

    /// Assert that metadata is rejected with a diagnostic fragment.
    fn assert_malformed_contains<'semantic, Metadata, Expected>(
        metadata: Metadata,
        expected: Expected,
    ) where
        Metadata: Into<MetadataText<'semantic>>,
        Expected: Into<ExpectedText<'semantic>>,
    {
        let expected = expected.into().0;
        let metadata = metadata.into().0;
        assert!(matches!(
            forbidden_default_graph_packages(metadata),
            Err(GateError::Operational { detail }) if detail.contains(expected)
        ));
    }

    /// Temporary project fixture directory removed on drop.
    #[repr(transparent)]
    struct ProjectFixture
    {
        /// Unique root path for this test.
        root: PathBuf,
    }

    impl ProjectFixture
    {
        /// Create an empty fixture directory.
        fn new<'semantic, Name>(name: Name) -> Result<Self, GateError>
        where
            Name: Into<NameText<'semantic>>,
        {
            let name = name.into().0;
            let root = env::temp_dir().join(format!(
                "gandr-workflow-gates-project-{}-{name}",
                std::process::id()
            ));
            support::HOST_FILESYSTEM.remove_dir_if_exists(&root)?;
            support::HOST_FILESYSTEM.create_dir_all(&root)?;
            Ok(Self { root })
        }

        /// Borrow the fixture root.
        fn path(&self) -> &Path
        {
            &self.root
        }
    }

    impl Drop for ProjectFixture
    {
        fn drop(&mut self)
        {
            drop(support::HOST_FILESYSTEM.remove_dir_all(&self.root));
        }
    }

    /// Run Git with string arguments in a fixture repository.
    fn git<Args>(
        cwd: &Path,
        args: Args,
    ) -> Result<String, GateError>
    where
        Args: IntoIterator,
        Args::Item: Into<OsString>,
    {
        git_os(cwd, args)
    }

    /// Run Git with OS-string arguments in a fixture repository.
    fn git_os<Args>(
        cwd: &Path,
        args: Args,
    ) -> Result<String, GateError>
    where
        Args: IntoIterator,
        Args::Item: Into<OsString>,
    {
        let mut command = support::stateless_git_command();
        command
            .args(args.into_iter().map(Into::into))
            .current_dir(cwd);
        let output = command
            .output()
            .map_err(|error| GateError::operational(error.to_string()))?;
        if !output.status.success() {
            return Err(GateError::operational(format!(
                "git fixture failed: {}",
                String::from_utf8_lossy(&output.stderr),
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// Commit staged fixture changes with deterministic identity.
    fn git_commit<'semantic, Message>(
        cwd: &Path,
        message: Message,
    ) -> Result<String, GateError>
    where
        Message: Into<MessageText<'semantic>>,
    {
        let message = message.into().0;
        let mut command = support::stateless_git_command();
        command
            .args(["commit", "-m", message])
            .current_dir(cwd)
            .env("GIT_AUTHOR_DATE", "1000000000 +0000")
            .env("GIT_COMMITTER_DATE", "1000000000 +0000");
        let output = command
            .output()
            .map_err(|error| GateError::operational(error.to_string()))?;
        if !output.status.success() {
            return Err(GateError::operational(format!(
                "git commit fixture failed: {}",
                String::from_utf8_lossy(&output.stderr),
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
