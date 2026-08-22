//! Project-owned operational gates retained from the Nushell audit.
//!
//! This module ports only the project operations that still have a live
//! Rust-gate role after the callsite audit: the default dependency graph guard
//! — using `cargo metadata` resolve data rather than parsing `cargo tree`
//! text, so package identity comes from Cargo's JSON graph.
//!
//! The audited raw `^git` scanner is deliberately omitted here: it is vacuous
//! after clean cutover because this crate's project operations call
//! [`support::run_output`] with `sanitized_git = true` instead of embedding
//! ad-hoc environment scrubbing.

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

crate::semantic_str!(pub struct CommandText);
crate::semantic_copy!(pub struct CodeExitCode(Option<i32>));
crate::semantic_str!(pub struct ContextText);
crate::semantic_str!(pub struct FieldText);
crate::semantic_str!(pub struct MetadataText);
crate::semantic_str!(pub struct ExpectedText);
crate::semantic_str!(pub struct MessageText);
crate::semantic_str!(pub struct StdoutText);
crate::semantic_str!(pub struct JsonFieldText);
crate::semantic_str!(pub struct MetadataJsonText);
crate::semantic_str!(pub struct HostTripleText);
crate::semantic_str!(pub struct ReachableDefaultPackageNameText);
crate::semantic_copy!(pub struct NCount(usize));
crate::semantic_copy!(pub struct DepKindReachesDefaultGraphFlag(bool));
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

/// Workspace member whose edges are never followed during default-graph
/// traversal: `gandr-workflow-dylint` is a nightly-only `rustc_private` Dylint
/// driver, so its `dylint_linting→dylint_internal→regex` chain is tooling-only
/// and outside the production default graph policy. The exemption is
/// unconditional because the driver is unreachable from anything that ships.
const TOOLING_EXEMPT_DEFAULT_GRAPH_MEMBER: &str = "gandr-workflow-dylint";

/// Workspace members whose edges are not followed during default-graph
/// traversal only while they ship nowhere: a listed member keeps the exemption
/// exactly while no other workspace member names it through a normal or build
/// dependency, checked against the same Cargo metadata at gate time, so a
/// member that gains a consumer loses the exemption and its findings return
/// rather than rotting silently. `gandr-core-checker-tools` carries proptest
/// in its public generator API and is a normal or build dependency of nothing.
const DEV_ONLY_EXEMPT_DEFAULT_GRAPH_MEMBERS: [&str; 1] = ["gandr-core-checker-tools"];

/// Stable finding kind for forbidden default graph packages.
const DEFAULT_GRAPH_FINDING_KIND: &str = "forbidden-default-dependency";

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
/// - witness: `project::tests::dev_only_exemption_holds_while_unconsumed`
/// - witness: `project::tests::dev_only_exemption_falls_when_consumed`
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
///   tooling-only Dylint driver member or, while their checked exemption holds,
///   from a dev-only exempt member that no other member consumes.
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
/// - witness: `project::tests::dev_only_exemption_holds_while_unconsumed`
/// - witness: `project::tests::dev_only_exemption_falls_when_consumed`
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

    let shipping_dev_exempt_ids = shipping_dev_exempt_member_ids(graph);
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
        // are tooling-only and outside the production default graph policy;
        // edges originating from a dev-only exempt member are skipped exactly
        // while that member still ships nowhere.
        if *package_name == TOOLING_EXEMPT_DEFAULT_GRAPH_MEMBER
            || shipping_dev_exempt_ids.contains(package_id)
        {
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

/// Return the dev-only exempt member ids whose exemptions currently hold.
///
/// # Contract
/// - requires: `graph` came from [`metadata_graph`].
/// - ensures: returns each workspace member named in
///   [`DEV_ONLY_EXEMPT_DEFAULT_GRAPH_MEMBERS`] that no other workspace member
///   names through an included normal/build edge; a listed member that gains a
///   consumer is absent, so its subtree is traversed and its findings return.
/// - provides: the checked half of the default-graph exemption split — the gate
///   itself evaluates the exemption's condition every run.
/// - fails: never; a listed name absent from the graph simply holds no
///   exemption.
/// - panics: none.
/// - intension: direct member-to-member edges decide, because only workspace
///   members can carry path dependencies into the workspace, so every
///   reachability chain into an exempt member passes through one.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the unconsumed and consumed fixtures separate
///   suppression from fallback on one graph difference.
/// - witness: `project::tests::dev_only_exemption_holds_while_unconsumed`
/// - witness: `project::tests::dev_only_exemption_falls_when_consumed`
fn shipping_dev_exempt_member_ids<'metadata>(
    graph: &MetadataGraph<'metadata>
) -> BTreeSet<&'metadata str>
{
    let exempt_ids: BTreeSet<&str> = graph
        .roots
        .iter()
        .copied()
        .filter(|id| {
            graph
                .package_names
                .get(id)
                .is_some_and(|name| DEV_ONLY_EXEMPT_DEFAULT_GRAPH_MEMBERS.contains(name))
        })
        .collect();
    let mut shipping = exempt_ids.clone();
    for root in &graph.roots {
        if exempt_ids.contains(*root) {
            continue;
        }
        if let Some(children) = graph.dependencies.get(*root) {
            for child in children {
                shipping.remove(child);
            }
        }
    }
    shipping
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
            "default normal/build workspace graph pulls a forbidden tree-sitter-family crate; the owner retirement bars every tree-sitter package from the shipping graph, so remove the dependency",
        ));
    }
    findings
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

/// Fixture tests for pure project-gate parsers and validators.
#[cfg(test)]
mod tests
{
    use super::*;

    /// Minimal package id for metadata fixtures.
    const ROOT_ID: &str = "path+file:///workspace#root@0.1.0";

    /// Minimal intermediate package id for metadata fixtures.
    const MID_ID: &str = "registry+https://example.invalid#middle@0.1.0";

    /// Minimal forbidden package id for metadata fixtures.
    const REGEX_ID: &str = "registry+https://example.invalid#regex@1.0.0";

    /// Minimal exempt Dylint driver package id for metadata fixtures.
    const DYLINT_ID: &str = "path+file:///workspace#workflow-dylint@0.1.0";

    /// Minimal dev-only exempt checker-tools package id for fixtures.
    const TOOLS_ID: &str = "path+file:///workspace#core-checker-tools@0.1.0";

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
            "default normal/build workspace graph pulls a forbidden tree-sitter-family crate; the owner retirement bars every tree-sitter package from the shipping graph, so remove the dependency",
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
            "default normal/build workspace graph pulls a forbidden tree-sitter-family crate; the owner retirement bars every tree-sitter package from the shipping graph, so remove the dependency",
        )]);
        Ok(())
    }

    /// The dev-only exemption holds while no other member consumes the
    /// exempt member, so the forbidden package behind it stays silent.
    #[test]
    fn dev_only_exemption_holds_while_unconsumed() -> Result<(), GateError>
    {
        let metadata = consumed_dev_only_tools_metadata(None);
        let findings = validate_default_dependency_graph_metadata(&metadata)?;
        assert!(findings.is_empty());
        Ok(())
    }

    /// A normal or build edge from another member consumes the dev-only
    /// exempt member, ends its checked exemption, and returns the finding.
    #[test]
    fn dev_only_exemption_falls_when_consumed() -> Result<(), GateError>
    {
        for kind in ["normal", "build"] {
            let metadata = consumed_dev_only_tools_metadata(Some(kind));
            let findings = validate_default_dependency_graph_metadata(&metadata)?;
            assert_eq!(findings, vec![Finding::new(
                DEFAULT_GRAPH_FINDING_KIND,
                "",
                CARGO_METADATA_SOURCE,
                "regex",
                "default normal/build workspace graph pulls a forbidden tree-sitter-family crate; the owner retirement bars every tree-sitter package from the shipping graph, so remove the dependency",
            )]);
        }
        Ok(())
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

    /// Build a minimal metadata graph where a dev-only exempt member holds
    /// regex behind its public generator API, optionally consumed by another
    /// member through the given dependency kind.
    fn consumed_dev_only_tools_metadata(consumer_kind: Option<&str>) -> String
    {
        let consumer_deps = match consumer_kind {
            | Some(kind) => format!(
                r#"[{{"pkg": "{TOOLS_ID}", "dep_kinds": [{{"kind": "{kind}", "target": null}}]}}]"#
            ),
            | None => String::from("[]"),
        };
        format!(
            r#"{{
                "packages": [
                    {{"id": "{ROOT_ID}", "name": "root"}},
                    {{"id": "{TOOLS_ID}", "name": "gandr-core-checker-tools"}},
                    {{"id": "{REGEX_ID}", "name": "regex"}}
                ],
                "workspace_members": ["{ROOT_ID}", "{TOOLS_ID}"],
                "resolve": {{
                    "nodes": [
                        {{
                            "id": "{ROOT_ID}",
                            "deps": {consumer_deps}
                        }},
                        {{
                            "id": "{TOOLS_ID}",
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
}
