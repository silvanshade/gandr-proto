//! AST-backed contract adequacy gate.
//!
//! This module owns Rust syntax interpretation for `# Contract` and
//! `# Adequacy` documentation groups. It parses complete Rust files with `syn`,
//! groups `#[doc = "..."]` attributes by AST owner, and validates exact nextest
//! witness bullets against exact aliases gathered from nextest JSON.

extern crate alloc;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use syn::Attribute;
use syn::Expr;
use syn::ExprLit;
use syn::File;
use syn::Lit;
use syn::Meta;
use syn::visit::Visit;
use yaml_rust2::yaml::Hash;
use yaml_rust2::yaml::Yaml;
use yaml_rust2::yaml::YamlLoader;

use crate::Finding;
use crate::GateError;
use crate::GateResult;

crate::semantic_str!(pub struct SourceText);
crate::semantic_str!(pub struct CommandText);
crate::semantic_str!(pub struct DetailText);
crate::semantic_str!(pub struct KeyNameText);
crate::semantic_str!(pub struct JobIdText);
crate::semantic_copy!(pub struct StepNumberNumber(usize));
crate::semantic_str!(pub struct StepNameText);
crate::semantic_str!(pub struct LineText);
crate::semantic_copy!(pub struct OpenIndex(usize));
crate::semantic_str!(pub struct SegmentText);
crate::semantic_copy!(pub struct RustupIndexIndex(usize));
crate::semantic_copy!(pub struct ContractPositionIndex(usize));
crate::semantic_copy!(pub struct ContractEndCount(usize));
crate::semantic_copy!(pub struct AdequacyPositionIndex(usize));
crate::semantic_copy!(pub struct AdequacyEndCount(usize));
crate::semantic_copy!(pub struct HeadingPositionIndex(usize));
crate::semantic_str!(pub struct HeadingText);
crate::semantic_str!(pub struct KindText);
crate::semantic_copy!(pub struct RequiresNameFlag(bool));
crate::semantic_optional_str!(pub struct OptionalPackageText);
crate::semantic_optional_str!(pub struct OptionalCrateNameText);
crate::semantic_copy!(pub struct IsTestcaseContextFlag(bool));
crate::semantic_str!(pub struct NameText);

impl<'text> From<&'_ &'text str> for NameText<'text>
{
    #[inline]
    fn from(value: &'_ &'text str) -> Self
    {
        Self(value)
    }
}
crate::semantic_str!(pub struct ScriptText);
crate::semantic_str!(pub struct TaskText);
crate::semantic_str!(pub struct TextText);
crate::semantic_str!(pub struct WordText);
crate::semantic_copy!(pub struct CharacterCharacter(char));
crate::semantic_str!(pub struct HypothesisText);
crate::semantic_str!(pub struct AsStrText);
crate::semantic_str!(pub struct MiseHintText);
crate::semantic_optional_str!(pub struct OptionalYamlStringText);
crate::semantic_optional_str!(pub struct OptionalCanonicalMiseTaskText);
crate::semantic_copy!(pub struct StaticMiseTaskFlag(bool));
crate::semantic_optional_copy!(pub struct OptionalMatchingSubstitutionParenCount(usize));
crate::semantic_copy!(pub struct ShellFunctionDefinitionFlag(bool));
crate::semantic_copy!(pub struct ShellIdentifierFlag(bool));
crate::semantic_copy!(pub struct ToolInvocationIsSetupFlag(bool));
crate::semantic_optional_str!(pub struct OptionalNextToolSubcommandText);
crate::semantic_str!(pub struct NormalizeShellWordText);
crate::semantic_copy!(pub struct ShellWordBoundaryFlag(bool));
crate::semantic_copy!(pub struct EnvironmentAssignmentFlag(bool));
crate::semantic_copy!(pub struct ShellControlWordFlag(bool));
crate::semantic_copy!(pub struct WrapperCommandFlag(bool));
crate::semantic_copy!(pub struct SectionEndIndex(usize));
crate::semantic_copy!(pub struct HeadingFlag(bool));
crate::semantic_copy!(pub struct AnyHeadingFlag(bool));
crate::semantic_optional_str!(pub struct OptionalHeadingTextText);
crate::semantic_optional_str!(pub struct OptionalContractClauseNameText);
crate::semantic_optional_copy!(pub struct OptionalContractClauseOrderCount(usize));
crate::semantic_copy!(pub struct IndentedContinuationFlag(bool));
crate::semantic_optional_str!(pub struct OptionalExactHypothesisText);
crate::semantic_copy!(pub struct NamesLadderRungFlag(bool));
crate::semantic_optional_str!(pub struct OptionalExactWitnessText);
crate::semantic_str!(pub struct ExactWitnessText);
crate::semantic_copy!(pub struct LooksWitnessLikeFlag(bool));
crate::semantic_optional_str!(pub struct OptionalSupportedSuiteStatusText);
crate::semantic_optional_str!(pub struct OptionalTestNameText);
crate::semantic_optional_str!(pub struct OptionalSupportedTestRecordNameText);
crate::semantic_optional_str!(pub struct OptionalPackageContextText);
crate::semantic_optional_str!(pub struct OptionalCrateContextText);
crate::semantic_optional_str!(pub struct OptionalStringFieldText);
crate::semantic_copy!(pub struct HeadingLevelCount(usize));

/// Parsed markdown heading level and text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParsedHeading<'heading>
{
    /// ATX heading level.
    level: HeadingLevelCount,
    /// Trimmed heading text.
    text: HeadingText<'heading>,
}

/// Fixed contract heading that appears at the wrong markdown level.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WrongLevelFixedHeading<'heading>
{
    /// Canonical fixed heading name.
    heading: HeadingText<'static>,
    /// Original documentation line.
    line: LineText<'heading>,
}

/// Operational result for pure source analysis.
pub type AnalysisResult = Result<Vec<Finding>, GateError>;

/// Analyze every Rust file under each supplied scope using fixture or live
/// nextest listing.
///
/// # Contract
/// - requires: `scopes` is nonempty and every scope is readable.
/// - ensures: returns findings sorted deterministically by path, declaration,
///   kind, and detail.
/// - provides: workspace-generic contract grammar findings for every discovered
///   Rust source.
/// - fails: returns a gate error for empty scopes, unreadable paths, invalid
///   Rust, invalid or unsupported nextest JSON, root symlink scopes, or failed
///   live nextest listing.
/// - panics: none.
///
/// # Errors
/// Returns usage errors for an empty scope list, I/O errors for unreadable
/// paths, Rust parse errors for unparseable source, JSON errors for invalid
/// nextest fixtures, and operational errors for unsupported nextest schema,
/// root symlink scopes, and failed live nextest listing.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — empty-scope, sorted multi-file, root symlink,
///   symlinked-child, and stale-witness surfaces are separated by exact
///   error/finding assertions over fixture scopes.
/// - witness: `contracts::run_reports_findings_in_deterministic_path_order`
/// - witness: `contracts::run_rejects_root_symlink_scope_and_skips_child_symlinks`
#[inline]
pub fn run(
    scopes: &[PathBuf],
    nextest_list_fixture: Option<&Path>,
) -> GateResult
{
    if scopes.is_empty() {
        return Err(GateError::usage(
            "contracts requires at least one --scope path",
        ));
    }

    let witnesses = match nextest_list_fixture {
        | Some(path) => read_fixture_witnesses(path)?,
        | None => list_nextest_witnesses()?,
    };
    let mut sources = Vec::new();
    for scope in scopes {
        sources.append(&mut rust_sources(scope)?);
    }
    sources.sort();
    sources.dedup();

    let mut findings = Vec::new();
    for path in sources {
        let source = crate::support::HOST_FILESYSTEM.read_to_string(&path)?;
        let mut source_findings = analyze_source(&path, &source, &witnesses)?;
        findings.append(&mut source_findings);
    }
    findings.sort_by(finding_order);
    return Ok(findings);
}

/// Analyze one Rust source string against exact nextest witness aliases.
///
/// # Contract
/// - requires: `source` is complete Rust source text and `witnesses` contains
///   exact nextest aliases.
/// - ensures: returns grammar and witness findings without treating drift as an
///   operational error.
/// - provides: pure syn-backed analysis for callers that already resolved their
///   witness set.
/// - fails: returns a Rust parse error when `source` is not a complete Rust
///   file.
/// - panics: none.
///
/// # Errors
/// Returns a Rust parse error when `source` is not a complete Rust file.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — malformed Rust is separated from structural
///   grammar drift by exact `GateError::RustParse` and finding-kind assertions.
/// - witness: `contracts::returns_parse_failures_as_operational_errors`
/// - witness: `contracts::rejects_contract_grammar_drift_modes`
#[inline]
pub fn analyze_source<'semantic, Source>(
    path: &Path,
    source: Source,
    witnesses: &BTreeSet<String>,
) -> AnalysisResult
where
    Source: Into<SourceText<'semantic>>,
{
    let source = source.into().0;
    let parsed = syn::parse_file(source).map_err(|error| GateError::RustParse {
        path: path.to_path_buf(),
        source: error,
    })?;
    return Ok(analyze_parsed_file(path, &parsed, witnesses));
}

/// Parse nextest aggregate JSON or JSON-lines output into exact witness
/// aliases.
///
/// # Contract
/// - requires: `source` is nextest aggregate JSON or JSON-lines output.
/// - ensures: validates aggregate `rust-suites`, concrete package+binary suite
///   records, suite status records, and testcase collections before alias
///   extraction.
/// - ensures: returns raw, package-qualified, crate-qualified, and
///   consolidated-harness-module-stripped crate-qualified aliases without
///   substring matching.
/// - provides: exact witness names for adequacy resolution.
/// - fails: returns a JSON error when the input is not JSON and not JSON-lines,
///   or an operational error when the input JSON has an unsupported nextest
///   schema.
/// - panics: none.
///
/// # Errors
/// Returns a JSON error when the input is not JSON and not JSON-lines, and an
/// operational error when JSON is well-formed but not a supported nextest
/// aggregate or per-test record.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — raw, package-qualified, crate-qualified,
///   consolidated-harness-module-stripped, empty-aggregate, concrete-suite,
///   malformed-input, unsupported-schema, nested-suite, and testcase-collection
///   surfaces are separated by exact alias membership and error-variant
///   assertions.
/// - witness: `contracts::accepts_exact_raw_package_and_crate_aliases_from_nextest_shapes`
/// - witness: `contracts::rejects_unsupported_nextest_json_shapes_as_operational_errors`
/// - witness: `contracts::accepts_harness_module_stripped_aliases_from_consolidated_integration_suites`
#[inline]
pub fn parse_nextest_witnesses<'semantic, Source>(
    source: Source
) -> Result<BTreeSet<String>, GateError>
where
    Source: Into<SourceText<'semantic>>,
{
    let source = source.into().0;
    match serde_json::from_str::<Value>(source) {
        | Ok(value) => return witnesses_from_supported_json(&value),
        | Err(error) => {
            let mut witnesses = BTreeSet::new();
            let mut saw_line = false;
            for (line_index, line) in source.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                saw_line = true;
                let value = serde_json::from_str::<Value>(trimmed).map_err(|json_error| {
                    GateError::Json {
                        source_name: String::from("nextest list"),
                        source: json_error,
                    }
                })?;
                let line_witnesses = witnesses_from_per_test_record(&value).ok_or_else(|| {
                    GateError::operational(format!(
                        "unsupported nextest JSON-lines record at line {}: expected per-test record with test name and package/crate context",
                        line_index.saturating_add(1)
                    ))
                })?;
                witnesses.extend(line_witnesses);
            }
            if saw_line {
                return Ok(witnesses);
            }
            return Err(GateError::Json {
                source_name: String::from("nextest list"),
                source: error,
            });
        },
    }
}

/// Analyze a GitHub Actions workflow file for CI run-step contract drift.
///
/// # Contract
/// - requires: `workflow_path` names a readable GitHub Actions workflow YAML
///   file.
/// - ensures: returns the same findings as [`analyze_ci_workflow`] over the
///   file contents.
/// - provides: file-backed CI contract analysis for CLI callers.
/// - fails: returns typed gate errors for unreadable files, malformed YAML,
///   malformed workflow shape, or non-string `run` steps.
/// - panics: none.
///
/// # Errors
/// Returns I/O errors for unreadable workflow files and operational errors for
/// malformed workflow YAML or unsupported workflow shapes.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — fixture-backed clean and finding workflows are
///   separated by exact `GateOutcome`/finding observations through the public
///   file-backed surface.
/// - witness: `ci_contracts::run_ci_workflow_reads_clean_workflow_fixture`
#[inline]
pub fn run_ci_workflow(workflow_path: &Path) -> GateResult
{
    let source = crate::support::HOST_FILESYSTEM.read_to_string(workflow_path)?;
    return analyze_ci_workflow(workflow_path, &source);
}

/// Analyze GitHub Actions workflow source for prohibited direct real-work
/// tools.
///
/// # Contract
/// - requires: `source` is the complete YAML text of one GitHub Actions
///   workflow.
/// - ensures: accepts run steps routed through `mise run`, accepted setup and
///   environment steps, and tool-install steps.
/// - ensures: returns one deterministic finding for each run step that invokes
///   `cargo`, `aube`, `treefmt`, or `wrkflw` as real work outside a mise task.
/// - provides: pure CI workflow contract analysis for tests and higher-level
///   gate orchestration.
/// - fails: returns operational errors for malformed YAML, malformed workflow
///   shape, or non-string `run` steps.
/// - panics: none.
///
/// # Errors
/// Returns operational errors when the workflow is not parseable YAML, does not
/// contain a mapping-shaped `jobs` section, has non-array `steps`, has
/// non-mapping concrete steps, or has a non-string `run` value.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — prohibited tool classes, accepted mise tasks,
///   setup allowances, malformed workflow handling, and actionable job/step
///   diagnostics are separated by exact findings and typed error observations.
/// - witness: `ci_contracts::rejects_each_prohibited_real_work_tool`
/// - witness: `ci_contracts::accepts_mise_tasks_and_setup_allowances`
/// - witness: `ci_contracts::malformed_workflows_are_operational_errors`
/// - witness: `ci_contracts::diagnostics_name_job_step_and_action`
#[inline]
pub fn analyze_ci_workflow<'semantic, Source>(
    workflow_path: &Path,
    source: Source,
) -> GateResult
where
    Source: Into<SourceText<'semantic>>,
{
    let source = source.into().0;
    let document = workflow_document(workflow_path, source)?;
    let root = yaml_hash(
        workflow_path,
        &document,
        "workflow root must be a mapping with jobs",
    )?;
    let jobs_yaml = yaml_mapping_value(root, "jobs").ok_or_else(|| {
        malformed_workflow(workflow_path, "workflow root must contain jobs mapping")
    })?;
    let jobs = yaml_hash(workflow_path, jobs_yaml, "jobs must be a mapping")?;

    let mut findings = Vec::new();
    for (job_key, job_yaml) in jobs {
        let Yaml::String(ref job_id) = *job_key
        else {
            return Err(malformed_workflow(workflow_path, "job ids must be strings"));
        };
        let job = yaml_hash(workflow_path, job_yaml, "job must be a mapping")?;
        if yaml_mapping_value(job, "uses").is_some() {
            return Err(malformed_workflow(
                workflow_path,
                format!("job `{job_id}` reusable workflow uses are not inspectable"),
            ));
        }
        if let Some(steps_yaml) = yaml_mapping_value(job, "steps") {
            collect_ci_step_findings(workflow_path, job_id, steps_yaml, &mut findings)?;
        }
    }
    findings.sort_by(finding_order);
    return Ok(findings);
}

/// Parsed shell invocation of a prohibited real-work tool.
struct ProhibitedInvocation<'script>
{
    /// Tool family that must be routed through a mise task.
    tool: ProhibitedTool,
    /// Exact shell segment that invoked the prohibited tool.
    command: &'script str,
}

/// Prohibited real-work tool families.
#[derive(Clone, Copy, Eq, PartialEq)]
enum ProhibitedTool
{
    /// Cargo workspace/test/build work must be a mise task in CI.
    Cargo,
    /// Non-canonical or mixed mise invocations are not one-task run steps.
    Mise,
    /// Nushell scripts must be owned by a named mise task.
    Nu,
    /// Aube package/grammar work must be a mise task in CI.
    Aube,
    /// Treefmt checks must be a mise task in CI.
    Treefmt,
    /// Workflow validation must be a mise task in CI.
    Wrkflw,
    /// Runtime-computed commands cannot be verified structurally.
    Dynamic,
}

impl ProhibitedTool
{
    /// Return a tool family for a command word.
    fn from_command<'semantic, CommandWord>(command: CommandWord) -> Option<Self>
    where
        CommandWord: Into<CommandText<'semantic>>,
    {
        let command = command.into().0;
        match command {
            | "mise" => Some(Self::Mise),
            | "nu" => Some(Self::Nu),
            | "cargo" => Some(Self::Cargo),
            | "aube" => Some(Self::Aube),
            | "treefmt" => Some(Self::Treefmt),
            | "wrkflw" => Some(Self::Wrkflw),
            | _ => None,
        }
    }

    /// Stable display name for diagnostics.
    fn as_str(self) -> impl Into<AsStrText<'static>>
    {
        match self {
            | Self::Mise => "mise",
            | Self::Nu => "nu",
            | Self::Cargo => "cargo",
            | Self::Aube => "aube",
            | Self::Treefmt => "treefmt",
            | Self::Wrkflw => "wrkflw",
            | Self::Dynamic => "dynamic expression",
        }
    }

    /// Representative mise task replacement for diagnostics.
    fn mise_hint(self) -> impl Into<MiseHintText<'static>>
    {
        match self {
            | Self::Mise | Self::Dynamic => "mise run <task>",
            | Self::Nu => "mise run <script-task>",
            | Self::Cargo => "mise run cargo:nextest",
            | Self::Aube => "mise run grammar:test",
            | Self::Treefmt => "mise run treefmt:check",
            | Self::Wrkflw => "mise run wrkflw",
        }
    }
}

/// Parse one YAML document from workflow source.
fn workflow_document<'semantic, Source>(
    workflow_path: &Path,
    source: Source,
) -> Result<Yaml, GateError>
where
    Source: Into<SourceText<'semantic>>,
{
    let source = source.into().0;
    let mut documents = YamlLoader::load_from_str(source).map_err(|error| {
        GateError::operational(format!(
            "workflow YAML parse error: path={} detail={error}",
            workflow_path.display()
        ))
    })?;
    if documents.len() != 1 {
        return Err(malformed_workflow(
            workflow_path,
            format!(
                "workflow YAML must contain exactly one document, found {}",
                documents.len()
            ),
        ));
    }
    return documents.pop().ok_or_else(|| {
        malformed_workflow(
            workflow_path,
            "workflow YAML must contain exactly one document, found 0",
        )
    });
}

/// Return a YAML mapping or a stable workflow-shape error.
fn yaml_hash<'semantic, 'yaml, Detail>(
    workflow_path: &Path,
    value: &'yaml Yaml,
    detail: Detail,
) -> Result<&'yaml Hash, GateError>
where
    Detail: Into<DetailText<'semantic>>,
{
    let detail = detail.into().0;
    match *value {
        | Yaml::Hash(ref mapping) => Ok(mapping),
        | _ => Err(malformed_workflow(workflow_path, detail)),
    }
}

/// Return a string-keyed mapping value without allocating a temporary key.
fn yaml_mapping_value<'semantic, 'yaml, KeyName>(
    mapping: &'yaml Hash,
    key_name: KeyName,
) -> Option<&'yaml Yaml>
where
    KeyName: Into<KeyNameText<'semantic>>,
{
    let key_name = key_name.into().0;
    for (key, value) in mapping {
        if let Yaml::String(ref candidate) = *key
            && candidate == key_name
        {
            return Some(value);
        }
    }
    return None;
}

/// Return a YAML string slice.
fn yaml_string(value: &Yaml) -> impl Into<OptionalYamlStringText<'_>>
{
    match *value {
        | Yaml::String(ref text) => Some(text.as_str()),
        | _ => None,
    }
}

/// Collect CI run-step findings from a job's steps node.
fn collect_ci_step_findings<'semantic, JobId>(
    workflow_path: &Path,
    job_id: JobId,
    steps_yaml: &Yaml,
    findings: &mut Vec<Finding>,
) -> Result<(), GateError>
where
    JobId: Into<JobIdText<'semantic>>,
{
    let job_id = job_id.into().0;
    let Yaml::Array(ref steps) = *steps_yaml
    else {
        return Err(malformed_workflow(
            workflow_path,
            format!("job `{job_id}` steps must be an array"),
        ));
    };
    for (step_index, step_yaml) in steps.iter().enumerate() {
        let step_number = step_index.saturating_add(1);
        match *step_yaml {
            | Yaml::Alias(_) => {},
            | Yaml::Hash(ref step) => {
                collect_ci_run_step_finding(workflow_path, job_id, step_number, step, findings)?;
            },
            | _ => {
                return Err(malformed_workflow(
                    workflow_path,
                    format!("job `{job_id}` step {step_number} must be a mapping or alias"),
                ));
            },
        }
    }
    return Ok(());
}

/// Collect one finding when a concrete step has a prohibited run command.
fn collect_ci_run_step_finding<'semantic, JobId, StepNumber>(
    workflow_path: &Path,
    job_id: JobId,
    step_number: StepNumber,
    step: &Hash,
    findings: &mut Vec<Finding>,
) -> Result<(), GateError>
where
    JobId: Into<JobIdText<'semantic>>,
    StepNumber: Into<StepNumberNumber>,
{
    let step_number = step_number.into().0;
    let job_id = job_id.into().0;
    let Some(run_yaml) = yaml_mapping_value(step, "run")
    else {
        if let Some(uses) =
            yaml_mapping_value(step, "uses").and_then(|value| yaml_string(value).into().0)
            && uses.starts_with("./")
        {
            return Err(malformed_workflow(
                workflow_path,
                format!(
                    "job `{job_id}` step {step_number} local action `{uses}` is not inspectable"
                ),
            ));
        }
        return Ok(());
    };
    let Yaml::String(ref script) = *run_yaml
    else {
        return Err(malformed_workflow(
            workflow_path,
            format!("job `{job_id}` step {step_number} run must be a string"),
        ));
    };
    if canonical_mise_task(script).into().0.is_some() {
        return Ok(());
    }
    if let Some(invocation) = prohibited_invocation(script) {
        let step_name = yaml_mapping_value(step, "name")
            .and_then(|value| yaml_string(value).into().0)
            .unwrap_or("<unnamed>");
        findings.push(ci_run_step_finding(
            workflow_path,
            job_id,
            step_number,
            step_name,
            &invocation,
        ));
    }
    return Ok(());
}

/// Build a stable finding for a prohibited real-work run step.
fn ci_run_step_finding<'semantic, JobId, StepNumber, StepName>(
    workflow_path: &Path,
    job_id: JobId,
    step_number: StepNumber,
    step_name: StepName,
    invocation: &ProhibitedInvocation<'semantic>,
) -> Finding
where
    JobId: Into<JobIdText<'semantic>>,
    StepNumber: Into<StepNumberNumber>,
    StepName: Into<StepNameText<'semantic>>,
{
    let step_number = step_number.into().0;
    let step_name = step_name.into().0;
    let job_id = job_id.into().0;
    Finding::new(
        "ci-bare-run-step",
        "",
        workflow_path.to_string_lossy().into_owned(),
        format!("job={job_id} step={step_number}"),
        format!(
            "job `{job_id}` step {step_number} `{step_name}` runs `{}` through prohibited real-work tool `{}`; replace it with `{}` or another self-contained mise task",
            invocation.command,
            invocation.tool.as_str().into().0,
            invocation.tool.mise_hint().into().0
        ),
    )
}

/// Return the exact task from a one-line `mise run <task>` script.
fn canonical_mise_task<'semantic, Script>(
    script: Script
) -> impl Into<OptionalCanonicalMiseTaskText<'semantic>>
where
    Script: Into<ScriptText<'semantic>>,
{
    let script = script.into().0;
    let mut lines = script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    let line = lines.next()?;
    if lines.next().is_some() {
        return None;
    }
    let mut words = line.split_whitespace();
    if words.next() != Some("mise") || words.next() != Some("run") {
        return None;
    }
    let task = words.next()?;
    if !is_static_mise_task(task).into().0 || words.next().is_some() {
        return None;
    }
    Some(task)
}

/// Return whether a mise task token is literal and repository-addressable.
fn is_static_mise_task<'semantic, Task>(
    task: Task
) -> impl Into<StaticMiseTaskFlag>
where
    Task: Into<TaskText<'semantic>>,
{
    let task = task.into().0;
    !task.is_empty()
        && task
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'))
}

/// Return the first prohibited command in a non-canonical shell script.
fn prohibited_invocation<'semantic, Script>(
    script: Script
) -> Option<ProhibitedInvocation<'semantic>>
where
    Script: Into<ScriptText<'semantic>>,
{
    let script = script.into().0;
    if script.contains("${{ matrix.") {
        return Some(ProhibitedInvocation {
            tool: ProhibitedTool::Dynamic,
            command: script.trim(),
        });
    }
    for line in script.lines() {
        let command = line.trim();
        if command.is_empty() || command.starts_with('#') {
            continue;
        }
        if let Some(invocation) = classify_shell_line(command) {
            return Some(invocation);
        }
    }
    None
}

/// Inspect executable command positions without mistaking arguments, quoted
/// text, cache paths, or shell-array values for commands.
fn classify_shell_line<'semantic, Line>(
    line: Line
) -> Option<ProhibitedInvocation<'semantic>>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    if let Some(invocation) = classify_command_substitutions(line) {
        return Some(invocation);
    }

    let bytes = line.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut segment_start = 0;
    let mut index = 0;
    while index < bytes.len() {
        let byte = *bytes.get(index)?;
        if escaped {
            escaped = false;
            index = index.saturating_add(1);
            continue;
        }
        if byte == b'\\' && quote != Some(b'\'') {
            escaped = true;
            index = index.saturating_add(1);
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            if quote == Some(byte) {
                quote = None;
            }
            else if quote.is_none() {
                quote = Some(byte);
            }
            index = index.saturating_add(1);
            continue;
        }
        if quote.is_none() && matches!(byte, b';' | b'|' | b'&') {
            if let Some(invocation) = line
                .get(segment_start .. index)
                .and_then(classify_command_segment)
            {
                return Some(invocation);
            }
            while bytes
                .get(index)
                .is_some_and(|separator| matches!(*separator, b';' | b'|' | b'&'))
            {
                index = index.saturating_add(1);
            }
            segment_start = index;
            continue;
        }
        index = index.saturating_add(1);
    }
    line.get(segment_start ..)
        .and_then(classify_command_segment)
}

/// Inspect command substitutions, which are executable even when embedded in
/// an otherwise harmless command argument.
fn classify_command_substitutions<'semantic, Line>(
    line: Line
) -> Option<ProhibitedInvocation<'semantic>>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    let bytes = line.as_bytes();
    let mut quote = None;
    let mut escaped = false;
    let mut index = 0;
    while index < bytes.len() {
        let byte = *bytes.get(index)?;
        if escaped {
            escaped = false;
            index = index.saturating_add(1);
            continue;
        }
        if byte == b'\\' && quote != Some(b'\'') {
            escaped = true;
            index = index.saturating_add(1);
            continue;
        }
        if matches!(byte, b'\'' | b'"') {
            if quote == Some(byte) {
                quote = None;
            }
            else if quote.is_none() {
                quote = Some(byte);
            }
            index = index.saturating_add(1);
            continue;
        }
        if quote != Some(b'\'') && byte == b'$' && bytes.get(index.saturating_add(1)) == Some(&b'(')
        {
            let open = index.saturating_add(1);
            if let Some(close) = matching_substitution_paren(line, open).into().0 {
                if let Some(invocation) = line
                    .get(open.saturating_add(1) .. close)
                    .and_then(classify_shell_line)
                {
                    return Some(invocation);
                }
                index = close.saturating_add(1);
                continue;
            }
            return Some(ProhibitedInvocation {
                tool: ProhibitedTool::Dynamic,
                command: line,
            });
        }
        if quote != Some(b'\'') && byte == b'`' {
            let remainder = line.get(index.saturating_add(1) ..)?;
            if let Some(relative_close) = remainder.find('`') {
                let close = index.saturating_add(1).saturating_add(relative_close);
                if let Some(invocation) = line
                    .get(index.saturating_add(1) .. close)
                    .and_then(classify_shell_line)
                {
                    return Some(invocation);
                }
                index = close.saturating_add(1);
                continue;
            }
            return Some(ProhibitedInvocation {
                tool: ProhibitedTool::Dynamic,
                command: line,
            });
        }
        index = index.saturating_add(1);
    }
    None
}

/// Find the closing parenthesis for a `$(` opener.
fn matching_substitution_paren<'semantic, Line, Open>(
    line: Line,
    open: Open,
) -> impl Into<OptionalMatchingSubstitutionParenCount>
where
    Line: Into<LineText<'semantic>>,
    Open: Into<OpenIndex>,
{
    let open = open.into().0;
    let line = line.into().0;
    let bytes = line.as_bytes();
    let mut depth = 1_usize;
    let mut quote = None;
    let mut escaped = false;
    let mut index = open.saturating_add(1);
    while index < bytes.len() {
        let byte = *bytes.get(index)?;
        if escaped {
            escaped = false;
        }
        else if byte == b'\\' && quote != Some(b'\'') {
            escaped = true;
        }
        else if matches!(byte, b'\'' | b'"') {
            if quote == Some(byte) {
                quote = None;
            }
            else if quote.is_none() {
                quote = Some(byte);
            }
        }
        else if quote.is_none() && byte == b'(' {
            depth = depth.saturating_add(1);
        }
        else if quote.is_none() && byte == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(index);
            }
        }
        index = index.saturating_add(1);
    }
    None
}

/// Classify the executable command at the front of one shell segment.
fn classify_command_segment<'semantic, Segment>(
    segment: Segment
) -> Option<ProhibitedInvocation<'semantic>>
where
    Segment: Into<SegmentText<'semantic>>,
{
    let segment = segment.into().0;
    let words = shell_words(segment);
    if is_shell_function_definition(&words).into().0 {
        return Some(dynamic_invocation(segment));
    }
    for (index, raw_word) in words.iter().enumerate() {
        let raw_text = raw_word.as_ref();
        let grouping_trimmed = raw_text.trim_matches(|character| matches!(character, '(' | ')'));
        if matches!(grouping_trimmed, "[" | "[[") {
            return None;
        }
        if raw_text
            .trim_matches(|character| {
                matches!(character, '(' | ')' | '"' | '\'' | '{' | '}' | '[' | ']')
            })
            .starts_with(['$', '`'])
        {
            return Some(ProhibitedInvocation {
                tool: ProhibitedTool::Dynamic,
                command: segment.trim(),
            });
        }
        let word = normalize_shell_word(*raw_word).into().0;
        if word.is_empty()
            || is_environment_assignment(word).into().0
            || is_shell_control_word(word).into().0
            || is_wrapper_command(word).into().0
            || word.starts_with('-')
        {
            continue;
        }
        if word.contains('=') || word.contains('"') || word.contains('\'') {
            return None;
        }
        let command = word.rsplit('/').next()?;
        if command == "rustup" {
            return classify_rustup_invocation(segment, &words, index);
        }
        if matches!(
            command,
            "." | "source"
                | "eval"
                | "bash"
                | "dash"
                | "sh"
                | "zsh"
                | "xargs"
                | "node"
                | "perl"
                | "python"
                | "python3"
                | "ruby"
        ) || word.starts_with("./")
            || word.starts_with("../")
        {
            return Some(dynamic_invocation(segment));
        }
        let tool = ProhibitedTool::from_command(command)?;
        let mut following = words.iter().skip(index.saturating_add(1)).copied();
        if tool_invocation_is_setup(tool, &mut following).into().0 {
            return None;
        }
        return Some(ProhibitedInvocation {
            tool,
            command: segment.trim(),
        });
    }
    None
}

/// Classify a command delegated through `rustup run <toolchain>`.
///
/// # Contract
///
/// Every `rustup run` shape returns a finding unless the wrapped command is a
/// recognized setup-only invocation; non-`run` rustup setup commands pass.
fn classify_rustup_invocation<'script, Segment, RustupIndex>(
    segment: Segment,
    words: &[WordText<'script>],
    rustup_index: RustupIndex,
) -> Option<ProhibitedInvocation<'script>>
where
    Segment: Into<SegmentText<'script>>,
    RustupIndex: Into<RustupIndexIndex>,
{
    let rustup_index = rustup_index.into().0;
    let segment = segment.into().0;
    let mut following = words.iter().skip(rustup_index.saturating_add(1)).copied();
    if next_tool_subcommand(&mut following).into().0 != Some("run") {
        return None;
    }
    if next_tool_subcommand(&mut following).into().0.is_none() {
        return Some(dynamic_invocation(segment));
    }
    let Some(wrapped_word) = next_tool_subcommand(&mut following).into().0
    else {
        return Some(dynamic_invocation(segment));
    };
    let wrapped_command = wrapped_word.rsplit('/').next()?;
    let Some(tool) = ProhibitedTool::from_command(wrapped_command)
    else {
        return Some(dynamic_invocation(segment));
    };
    if tool_invocation_is_setup(tool, &mut following).into().0 {
        return None;
    }
    Some(ProhibitedInvocation {
        tool,
        command: segment.trim(),
    })
}

/// Return a dynamic-dispatch finding for an uninspectable shell construct.
fn dynamic_invocation<'semantic, Segment>(
    segment: Segment
) -> ProhibitedInvocation<'semantic>
where
    Segment: Into<SegmentText<'semantic>>,
{
    let segment = segment.into().0;
    ProhibitedInvocation {
        tool: ProhibitedTool::Dynamic,
        command: segment.trim(),
    }
}

/// Return whether shell words declare a function whose body may dispatch work.
fn is_shell_function_definition(words: &[WordText<'_>]) -> impl Into<ShellFunctionDefinitionFlag>
{
    for raw_word in words {
        let word = normalize_shell_word(*raw_word).into().0;
        if is_environment_assignment(word).into().0
            || is_wrapper_command(word).into().0
            || word.starts_with('-')
        {
            continue;
        }
        if word == "function" {
            return true;
        }
        let candidate = raw_word
            .as_ref()
            .trim_matches(|character| matches!(character, '"' | '\'' | '{' | '}'));
        return candidate
            .strip_suffix("()")
            .is_some_and(|text| is_shell_identifier(text).into().0);
    }
    false
}

/// Return whether text is a literal shell identifier.
fn is_shell_identifier<'semantic, Text>(
    text: Text
) -> impl Into<ShellIdentifierFlag>
where
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let mut characters = text.chars();
    let Some(first) = characters.next()
    else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// Split one shell segment into words while preserving borrowed slices.
fn shell_words<'semantic, Segment>(segment: Segment) -> Vec<WordText<'semantic>>
where
    Segment: Into<SegmentText<'semantic>>,
{
    let segment = segment.into().0;
    let bytes = segment.as_bytes();
    let mut words = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut word_start = None;
    let mut index = 0;
    while index < bytes.len() {
        let Some(byte) = bytes.get(index).copied()
        else {
            break;
        };
        if escaped {
            escaped = false;
        }
        else if byte == b'\\' && quote != Some(b'\'') {
            escaped = true;
            if word_start.is_none() {
                word_start = Some(index);
            }
        }
        else if matches!(byte, b'\'' | b'"') {
            if word_start.is_none() {
                word_start = Some(index);
            }
            if quote == Some(byte) {
                quote = None;
            }
            else if quote.is_none() {
                quote = Some(byte);
            }
        }
        else if quote.is_none() && byte.is_ascii_whitespace() {
            if let Some(start) = word_start.take()
                && let Some(word) = segment.get(start .. index)
            {
                words.push(WordText(word));
            }
        }
        else if word_start.is_none() {
            word_start = Some(index);
        }
        index = index.saturating_add(1);
    }
    if let Some(start) = word_start
        && let Some(word) = segment.get(start ..)
    {
        words.push(WordText(word));
    }
    words
}

/// Return whether a prohibited tool occurrence is a setup-only installer.
fn tool_invocation_is_setup<'semantic, Words>(
    tool: ProhibitedTool,
    words: &mut Words,
) -> impl Into<ToolInvocationIsSetupFlag>
where
    Words: Iterator<Item = WordText<'semantic>>,
{
    match tool {
        | ProhibitedTool::Cargo => next_tool_subcommand(words)
            .into()
            .0
            .is_some_and(|word| word == "install" || word == "binstall"),
        | ProhibitedTool::Aube => next_tool_subcommand(words)
            .into()
            .0
            .is_some_and(|word| word == "ci"),
        | ProhibitedTool::Mise
        | ProhibitedTool::Nu
        | ProhibitedTool::Treefmt
        | ProhibitedTool::Wrkflw
        | ProhibitedTool::Dynamic => false,
    }
}

/// Return a tool subcommand after toolchain selectors and global options.
fn next_tool_subcommand<'word, Words>(
    words: &mut Words
) -> impl Into<OptionalNextToolSubcommandText<'word>>
where
    Words: Iterator<Item = WordText<'word>>,
{
    for raw_word in words {
        let word = normalize_shell_word(raw_word).into().0;
        if word.is_empty() || word == "--" || word.starts_with('-') || word.starts_with('+') {
            continue;
        }
        return Some(word);
    }
    None
}

/// Trim lightweight shell grouping and quoting punctuation from a word.
fn normalize_shell_word<'semantic, Word>(
    word: Word
) -> impl Into<NormalizeShellWordText<'semantic>>
where
    Word: Into<WordText<'semantic>>,
{
    let word = word.into().0;
    word.trim_matches(|character| shell_word_boundary(character).into().0)
}

/// Return whether a character is shell word boundary punctuation.
fn shell_word_boundary<Character>(character: Character) -> impl Into<ShellWordBoundaryFlag>
where
    Character: Into<CharacterCharacter>,
{
    let character = character.into().0;
    matches!(
        character,
        '(' | ')' | '"' | '\'' | '{' | '}' | '[' | ']' | '$' | '`'
    )
}

/// Return whether a word is an environment assignment prefix.
fn is_environment_assignment<'semantic, Word>(
    word: Word
) -> impl Into<EnvironmentAssignmentFlag>
where
    Word: Into<WordText<'semantic>>,
{
    let word = word.into().0;
    let Some((name, _)) = word.split_once('=')
    else {
        return false;
    };
    let mut characters = name.chars();
    let Some(first) = characters.next()
    else {
        return false;
    };
    if first != '_' && !first.is_ascii_alphabetic() {
        return false;
    }
    characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// Return whether a shell word is control syntax rather than a command.
fn is_shell_control_word<'semantic, Word>(
    word: Word
) -> impl Into<ShellControlWordFlag>
where
    Word: Into<WordText<'semantic>>,
{
    let word = word.into().0;
    matches!(
        word,
        "if" | "then" | "else" | "elif" | "fi" | "for" | "while" | "until" | "do" | "done"
    )
}

/// Return whether a shell word wraps the next command without changing it.
fn is_wrapper_command<'semantic, Word>(
    word: Word
) -> impl Into<WrapperCommandFlag>
where
    Word: Into<WordText<'semantic>>,
{
    let word = word.into().0;
    matches!(word, "env" | "exec" | "sudo" | "command" | "time" | "!")
}

/// Build a stable malformed-workflow operational error.
fn malformed_workflow<Detail>(
    workflow_path: &Path,
    detail: Detail,
) -> GateError
where
    Detail: Into<String>,
{
    let detail_text = detail.into();
    GateError::operational(format!(
        "malformed workflow: path={} detail={detail_text}",
        workflow_path.display()
    ))
}

/// Read a fixture file and parse exact nextest aliases.
fn read_fixture_witnesses(path: &Path) -> Result<BTreeSet<String>, GateError>
{
    let source = crate::support::HOST_FILESYSTEM.read_to_string(path)?;
    return parse_nextest_witnesses(&source);
}

/// Exact live-listing scope for the enabled workspace.
///
/// P0-disabled front ends are absent from workspace membership.
const NEXTEST_LIST_ARGS: &[&str] = &[
    "nextest",
    "list",
    "--workspace",
    "--all-targets",
    "--features=full",
    "--message-format",
    "json",
];

/// Run nextest in JSON mode and parse exact aliases from stdout.
fn list_nextest_witnesses() -> Result<BTreeSet<String>, GateError>
{
    let output = Command::new("cargo")
        .args(NEXTEST_LIST_ARGS)
        .output()
        .map_err(|error| GateError::Io {
            path: PathBuf::from("cargo nextest list"),
            source: error,
        })?;
    if !output.status.success() {
        return Err(GateError::operational(format!(
            "cargo nextest list failed with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    return parse_nextest_witnesses(&*stdout);
}

/// Walk scope iteratively and return sorted Rust source paths.
fn rust_sources(scope: &Path) -> Result<Vec<PathBuf>, GateError>
{
    let mut pending = vec![(scope.to_path_buf(), true)];
    let mut sources = Vec::new();
    while let Some((path, is_root)) = pending.pop() {
        let metadata = fs::symlink_metadata(&path).map_err(|error| GateError::Io {
            path: path.clone(),
            source: error,
        })?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            if is_root {
                return Err(GateError::operational(format!(
                    "scope path is a symlink: {}",
                    path.display()
                )));
            }
            continue;
        }
        if metadata.is_dir() {
            let entries = fs::read_dir(&path).map_err(|error| GateError::Io {
                path: path.clone(),
                source: error,
            })?;
            for entry in entries {
                let entry = entry.map_err(|error| GateError::Io {
                    path: path.clone(),
                    source: error,
                })?;
                pending.push((entry.path(), false));
            }
            continue;
        }
        if metadata.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == OsStr::new("rs"))
        {
            sources.push(path);
        }
    }
    sources.sort();
    return Ok(sources);
}

/// Analyze a parsed file for adequacy findings.
fn analyze_parsed_file(
    path: &Path,
    parsed: &File,
    witnesses: &BTreeSet<String>,
) -> Vec<Finding>
{
    let mut collector = DocCollector::default();
    collector.visit_file(parsed);
    let mut findings = Vec::new();
    for group in collector.groups {
        if let Some(finding) = finding_for_group(path, &group, witnesses) {
            findings.push(finding);
        }
    }
    findings.sort_by(finding_order);
    return findings;
}

/// Stable finding sort key.
fn finding_order(
    left: &Finding,
    right: &Finding,
) -> core::cmp::Ordering
{
    return left
        .path
        .cmp(&right.path)
        .then_with(|| left.declaration.cmp(&right.declaration))
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| left.detail.cmp(&right.detail));
}

/// Return one finding when a documentation group violates contract adequacy.
fn finding_for_group(
    path: &Path,
    group: &DocGroup,
    witnesses: &BTreeSet<String>,
) -> Option<Finding>
{
    if let Some(wrong) = wrong_level_fixed_heading(group) {
        return Some(make_finding(
            path,
            group,
            "section-heading-level",
            &format!(
                "{} section must use exactly one # heading: {}",
                wrong.heading.0,
                wrong.line.0.trim()
            ),
        ));
    }

    let contract_position = group
        .docs
        .iter()
        .position(|line| is_heading(line, "Contract").into().0)?;
    let first_adequacy_position = group
        .docs
        .iter()
        .position(|line| is_heading(line, "Adequacy").into().0);
    if first_adequacy_position.is_some_and(|position| position < contract_position) {
        return Some(make_finding(
            path,
            group,
            "section-order",
            "# Adequacy must appear after # Contract and optional # Errors",
        ));
    }
    let first_errors_position = group
        .docs
        .iter()
        .position(|line| is_heading(line, "Errors").into().0);
    if first_errors_position.is_some_and(|position| position < contract_position) {
        return Some(make_finding(
            path,
            group,
            "section-order",
            "# Errors must appear after # Contract",
        ));
    }
    let Some(adequacy_position) = first_adequacy_position
    else {
        return Some(make_finding(
            path,
            group,
            "missing-adequacy",
            "missing later # Adequacy section",
        ));
    };
    if let Some(errors_position) = first_errors_position
        && errors_position > adequacy_position
    {
        return Some(make_finding(
            path,
            group,
            "section-order",
            "# Errors must appear between # Contract and # Adequacy",
        ));
    }
    if adequacy_position < contract_position {
        return Some(make_finding(
            path,
            group,
            "section-order",
            "# Adequacy must appear after # Contract",
        ));
    }

    let contract_end = first_errors_position
        .filter(|position| *position > contract_position && *position < adequacy_position)
        .unwrap_or(adequacy_position);
    if let Some(finding) = validate_contract_section(path, group, contract_position, contract_end) {
        return Some(finding);
    }

    let adequacy_end = section_end(group, adequacy_position).into().0;
    let exact_witnesses =
        match validate_adequacy_section(path, group, adequacy_position, adequacy_end) {
            | Ok(adequacy_witnesses) => adequacy_witnesses,
            | Err(finding) => return Some(finding),
        };
    if exact_witnesses
        .iter()
        .any(|witness| witnesses.contains(witness.as_ref()))
    {
        return None;
    }
    return Some(make_finding(
        path,
        group,
        "stale-witness",
        &format!(
            "no witness matched nextest aliases: {}",
            exact_witnesses
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ));
}

/// Validate fixed `# Contract` clause grammar.
fn validate_contract_section<ContractPosition, ContractEnd>(
    path: &Path,
    group: &DocGroup,
    contract_position: ContractPosition,
    contract_end: ContractEnd,
) -> Option<Finding>
where
    ContractPosition: Into<ContractPositionIndex>,
    ContractEnd: Into<ContractEndCount>,
{
    let contract_end = contract_end.into().0;
    let contract_position = contract_position.into().0;
    let mut seen = BTreeSet::new();
    let mut max_order = 0usize;
    let mut saw_panics = false;
    let mut saw_intension = false;
    for line in group
        .docs
        .iter()
        .enumerate()
        .skip(contract_position.checked_add(1)?)
        .take_while(|&(position, _)| position < contract_end)
        .map(|(_, line)| line)
    {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(clause) = contract_clause_name(line).into().0 {
            let Some(order) = contract_clause_order(clause).into().0
            else {
                return Some(make_finding(
                    path,
                    group,
                    "unknown-contract-clause",
                    &format!("unknown # Contract clause: {clause}"),
                ));
            };
            if !seen.insert(String::from(clause)) {
                return Some(make_finding(
                    path,
                    group,
                    "duplicate-contract-clause",
                    &format!("duplicate # Contract clause: {clause}"),
                ));
            }
            if saw_intension {
                return Some(make_finding(
                    path,
                    group,
                    "intension-not-last",
                    "- intension: must be the final # Contract clause",
                ));
            }
            if order < max_order {
                return Some(make_finding(
                    path,
                    group,
                    "out-of-order-contract-clause",
                    &format!("# Contract clause is out of order: {clause}"),
                ));
            }
            max_order = order;
            if clause == "panics" {
                saw_panics = true;
            }
            if clause == "intension" {
                saw_intension = true;
            }
            continue;
        }
        if is_indented_continuation(line).into().0 {
            continue;
        }
        if line.trim().starts_with("- ") {
            return Some(make_finding(
                path,
                group,
                "malformed-contract-clause",
                &format!("malformed # Contract clause: {}", line.trim()),
            ));
        }
        return Some(make_finding(
            path,
            group,
            "contract-prose",
            &format!("free-form # Contract prose: {}", line.trim()),
        ));
    }
    if !saw_panics {
        return Some(make_finding(
            path,
            group,
            "missing-panics",
            "# Contract section must include - panics:",
        ));
    }
    return None;
}

/// Validate fixed `# Adequacy` hypothesis and witness grammar.
fn validate_adequacy_section<'doc, AdequacyPosition, AdequacyEnd>(
    path: &Path,
    group: &'doc DocGroup,
    adequacy_position: AdequacyPosition,
    adequacy_end: AdequacyEnd,
) -> Result<Vec<ExactWitnessText<'doc>>, Finding>
where
    AdequacyPosition: Into<AdequacyPositionIndex>,
    AdequacyEnd: Into<AdequacyEndCount>,
{
    let adequacy_end = adequacy_end.into().0;
    let adequacy_position = adequacy_position.into().0;
    let mut saw_hypothesis = false;
    let mut saw_witness = false;
    let mut exact_witnesses = Vec::new();
    for line in group
        .docs
        .iter()
        .enumerate()
        .skip(
            adequacy_position
                .checked_add(1)
                .unwrap_or(adequacy_position),
        )
        .take_while(|&(position, _)| position < adequacy_end)
        .map(|(_, line)| line)
    {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(hypothesis) = exact_hypothesis(line).into().0 {
            if saw_hypothesis {
                return Err(make_finding(
                    path,
                    group,
                    "duplicate-hypothesis",
                    "# Adequacy section must contain exactly one - hypothesis: bullet",
                ));
            }
            if saw_witness {
                return Err(make_finding(
                    path,
                    group,
                    "late-hypothesis",
                    "- hypothesis: must be the first # Adequacy bullet",
                ));
            }
            if !names_ladder_rung(hypothesis).into().0 {
                return Err(make_finding(
                    path,
                    group,
                    "hypothesis-rung",
                    "- hypothesis: must name L0, L1, L2, or L3",
                ));
            }
            saw_hypothesis = true;
            continue;
        }
        if line.trim().starts_with("- hypothesis") {
            return Err(make_finding(
                path,
                group,
                "malformed-hypothesis",
                &format!("malformed hypothesis line: {}", line.trim()),
            ));
        }
        if let Some(witness) = exact_witness(line).into().0 {
            if !saw_hypothesis {
                return Err(make_finding(
                    path,
                    group,
                    "late-hypothesis",
                    "- hypothesis: must precede all witness bullets",
                ));
            }
            saw_witness = true;
            exact_witnesses.push(ExactWitnessText(witness));
            continue;
        }
        if saw_hypothesis && !saw_witness && is_indented_continuation(line).into().0 {
            continue;
        }
        if looks_witness_like(line).into().0 {
            return Err(make_finding(
                path,
                group,
                "malformed-witness",
                &format!("malformed witness line: {}", line.trim()),
            ));
        }
        return Err(make_finding(
            path,
            group,
            "adequacy-prose",
            &format!("free-form # Adequacy prose: {}", line.trim()),
        ));
    }
    if !saw_hypothesis {
        return Err(make_finding(
            path,
            group,
            "missing-hypothesis",
            "# Adequacy section must start with exactly one - hypothesis: bullet",
        ));
    }
    if exact_witnesses.is_empty() {
        return Err(make_finding(
            path,
            group,
            "missing-witness",
            "# Adequacy section has no exact - witness: `path` bullet",
        ));
    }
    return Ok(exact_witnesses);
}

/// Determine the exclusive end of a markdown section.
fn section_end<HeadingPosition>(
    group: &DocGroup,
    heading_position: HeadingPosition,
) -> impl Into<SectionEndIndex>
where
    HeadingPosition: Into<HeadingPositionIndex>,
{
    let heading_position = heading_position.into().0;
    return group
        .docs
        .iter()
        .enumerate()
        .skip(heading_position.saturating_add(1))
        .find_map(|(position, line)| is_any_heading(line).into().0.then_some(position))
        .unwrap_or(group.docs.len());
}

/// Return whether a line is the requested top-level heading.
fn is_heading<'semantic, Line, Heading>(
    line: Line,
    heading: Heading,
) -> impl Into<HeadingFlag>
where
    Line: Into<LineText<'semantic>>,
    Heading: Into<HeadingText<'semantic>>,
{
    let heading = heading.into().0;
    let line = line.into().0;
    heading_text(line).into().0 == Some(heading)
}

/// Return whether a line is any top-level heading.
fn is_any_heading<'semantic, Line>(line: Line) -> impl Into<AnyHeadingFlag>
where
    Line: Into<LineText<'semantic>>,
{
    heading_text(line).into().0.is_some()
}

/// Return the text of a top-level ATX markdown heading.
fn heading_text<'semantic, Line>(
    line: Line
) -> impl Into<OptionalHeadingTextText<'semantic>>
where
    Line: Into<LineText<'semantic>>,
{
    let heading = heading_level_text(line)?;
    if heading.level.0 == 1 {
        return Some(heading.text.0);
    }
    return None;
}

/// Return the level and text of an ATX markdown heading.
fn heading_level_text<'semantic, Line>(
    line: Line
) -> Option<ParsedHeading<'semantic>>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    let trimmed = line.trim();
    let mut hash_end = 0usize;
    let mut hashes = 0usize;
    for (index, character) in trimmed.char_indices() {
        if character != '#' {
            break;
        }
        hashes = hashes.saturating_add(1);
        hash_end = index.saturating_add(character.len_utf8());
    }
    if !(1 ..= 6).contains(&hashes) {
        return None;
    }
    let rest = trimmed.get(hash_end ..)?;
    let text = rest.strip_prefix(' ')?;
    return Some(ParsedHeading {
        level: HeadingLevelCount(hashes),
        text: HeadingText(text.trim()),
    });
}

/// Return a fixed section heading that uses the wrong ATX level.
fn wrong_level_fixed_heading(group: &DocGroup) -> Option<WrongLevelFixedHeading<'_>>
{
    for line in &group.docs {
        let Some(heading) = heading_level_text(line)
        else {
            continue;
        };
        if heading.level.0 == 1 {
            continue;
        }
        match heading.text.0 {
            | "Contract" => {
                return Some(WrongLevelFixedHeading {
                    heading: HeadingText("Contract"),
                    line: LineText(line),
                });
            },
            | "Errors" => {
                return Some(WrongLevelFixedHeading {
                    heading: HeadingText("Errors"),
                    line: LineText(line),
                });
            },
            | "Adequacy" => {
                return Some(WrongLevelFixedHeading {
                    heading: HeadingText("Adequacy"),
                    line: LineText(line),
                });
            },
            | _ => {},
        }
    }
    return None;
}

/// Build one stable contract finding for a documentation group.
fn make_finding<'semantic, Kind, Detail>(
    path: &Path,
    group: &DocGroup,
    kind: Kind,
    detail: Detail,
) -> Finding
where
    Kind: Into<KindText<'semantic>>,
    Detail: Into<DetailText<'semantic>>,
{
    let detail = detail.into().0;
    let kind = kind.into().0;
    return Finding {
        kind: kind.to_owned(),
        package: String::new(),
        path: path.display().to_string(),
        declaration: group.declaration.clone(),
        detail: detail.to_owned(),
    };
}

/// Extract a `# Contract` bullet clause name.
fn contract_clause_name<'semantic, Line>(
    line: Line
) -> impl Into<OptionalContractClauseNameText<'semantic>>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("- ")?;
    let (name, _) = rest.split_once(':')?;
    return Some(name.trim());
}

/// Return the fixed order of a known `# Contract` clause.
fn contract_clause_order<'semantic, Name>(
    name: Name
) -> impl Into<OptionalContractClauseOrderCount>
where
    Name: Into<NameText<'semantic>>,
{
    let name = name.into().0;
    match name {
        | "requires" => return Some(1),
        | "ensures" => return Some(2),
        | "provides" => return Some(3),
        | "fails" => return Some(4),
        | "unsafe invariants" => return Some(5),
        | "panics" => return Some(6),
        | "intension" => return Some(7),
        | _ => return None,
    }
}

/// Return whether a doc line is an explicitly indented continuation.
fn is_indented_continuation<'semantic, Line>(
    line: Line
) -> impl Into<IndentedContinuationFlag>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    let content = line.strip_prefix(' ').unwrap_or(line);
    return content.chars().next().is_some_and(char::is_whitespace);
}

/// Extract an exact adequacy hypothesis bullet.
fn exact_hypothesis<'semantic, Line>(
    line: Line
) -> impl Into<OptionalExactHypothesisText<'semantic>>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    let trimmed = line.trim();
    return trimmed.strip_prefix("- hypothesis:");
}

/// Return whether a hypothesis names an adequacy ladder rung.
fn names_ladder_rung<'semantic, Hypothesis>(
    hypothesis: Hypothesis
) -> impl Into<NamesLadderRungFlag>
where
    Hypothesis: Into<HypothesisText<'semantic>>,
{
    let hypothesis = hypothesis.into().0;
    return hypothesis
        .split(|character: char| !(character.is_ascii_alphanumeric()))
        .any(|token| matches!(token, "L0" | "L1" | "L2" | "L3"));
}

/// Extract an exact witness bullet target.
fn exact_witness<'semantic, Line>(
    line: Line
) -> impl Into<OptionalExactWitnessText<'semantic>>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("- witness: `")?;
    let witness = rest.strip_suffix('`')?;
    if witness.is_empty() || witness.contains('`') {
        return None;
    }
    return Some(witness);
}

/// Return whether a line is intended as a witness but is not exact syntax.
fn looks_witness_like<'semantic, Line>(
    line: Line
) -> impl Into<LooksWitnessLikeFlag>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    let trimmed = line.trim();
    return trimmed.contains("witness") || trimmed.contains("Witness");
}

/// Extract doc strings from syn attributes.
fn doc_lines(attrs: &[Attribute]) -> Vec<String>
{
    let mut docs = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let Meta::NameValue(ref name_value) = attr.meta
            && let Expr::Lit(ExprLit {
                lit: Lit::Str(ref lit),
                ..
            }) = name_value.value
        {
            for line in lit.value().lines() {
                docs.push(String::from(line));
            }
        }
    }
    return docs;
}

/// Documentation group owned by one syn AST declaration.
#[derive(Debug)]
struct DocGroup
{
    /// Stable declaration label.
    declaration: String,
    /// Raw doc attribute lines for the owner.
    docs: Vec<String>,
}

/// Collector for syn-visible doc owners.
#[derive(Default)]
#[repr(transparent)]
struct DocCollector
{
    /// Documentation groups in visit order.
    groups: Vec<DocGroup>,
}

impl DocCollector
{
    /// Add an owner group when doc attributes are present.
    fn push(
        &mut self,
        declaration: String,
        attrs: &[Attribute],
    )
    {
        let docs = doc_lines(attrs);
        if docs.is_empty() {
            return;
        }
        self.groups.push(DocGroup { declaration, docs });
    }
}

impl<'ast> Visit<'ast> for DocCollector
{
    fn visit_file(
        &mut self,
        i: &'ast File,
    )
    {
        self.push(String::from("crate"), &i.attrs);
        syn::visit::visit_file(self, i);
    }

    fn visit_item(
        &mut self,
        i: &'ast syn::Item,
    )
    {
        self.push(item_declaration(i), item_attrs(i));
        syn::visit::visit_item(self, i);
    }

    fn visit_impl_item(
        &mut self,
        i: &'ast syn::ImplItem,
    )
    {
        self.push(impl_item_declaration(i), impl_item_attrs(i));
        syn::visit::visit_impl_item(self, i);
    }

    fn visit_trait_item(
        &mut self,
        i: &'ast syn::TraitItem,
    )
    {
        self.push(trait_item_declaration(i), trait_item_attrs(i));
        syn::visit::visit_trait_item(self, i);
    }

    fn visit_foreign_item(
        &mut self,
        i: &'ast syn::ForeignItem,
    )
    {
        self.push(foreign_item_declaration(i), foreign_item_attrs(i));
        syn::visit::visit_foreign_item(self, i);
    }

    fn visit_field(
        &mut self,
        i: &'ast syn::Field,
    )
    {
        self.push(field_declaration(i), &i.attrs);
        syn::visit::visit_field(self, i);
    }

    fn visit_variant(
        &mut self,
        i: &'ast syn::Variant,
    )
    {
        self.push(format!("variant {}", i.ident), &i.attrs);
        syn::visit::visit_variant(self, i);
    }
}

/// Return attributes for an item.
fn item_attrs(item: &syn::Item) -> &[Attribute]
{
    match *item {
        | syn::Item::Const(ref i) => &i.attrs,
        | syn::Item::Enum(ref i) => &i.attrs,
        | syn::Item::ExternCrate(ref i) => &i.attrs,
        | syn::Item::Fn(ref i) => &i.attrs,
        | syn::Item::ForeignMod(ref i) => &i.attrs,
        | syn::Item::Impl(ref i) => &i.attrs,
        | syn::Item::Macro(ref i) => &i.attrs,
        | syn::Item::Mod(ref i) => &i.attrs,
        | syn::Item::Static(ref i) => &i.attrs,
        | syn::Item::Struct(ref i) => &i.attrs,
        | syn::Item::Trait(ref i) => &i.attrs,
        | syn::Item::TraitAlias(ref i) => &i.attrs,
        | syn::Item::Type(ref i) => &i.attrs,
        | syn::Item::Union(ref i) => &i.attrs,
        | syn::Item::Use(ref i) => &i.attrs,
        | _ => &[],
    }
}

/// Return attributes for an impl item.
fn impl_item_attrs(item: &syn::ImplItem) -> &[Attribute]
{
    match *item {
        | syn::ImplItem::Const(ref i) => &i.attrs,
        | syn::ImplItem::Fn(ref i) => &i.attrs,
        | syn::ImplItem::Macro(ref i) => &i.attrs,
        | syn::ImplItem::Type(ref i) => &i.attrs,
        | _ => &[],
    }
}

/// Return attributes for a trait item.
fn trait_item_attrs(item: &syn::TraitItem) -> &[Attribute]
{
    match *item {
        | syn::TraitItem::Const(ref i) => &i.attrs,
        | syn::TraitItem::Fn(ref i) => &i.attrs,
        | syn::TraitItem::Macro(ref i) => &i.attrs,
        | syn::TraitItem::Type(ref i) => &i.attrs,
        | _ => &[],
    }
}

/// Return attributes for a foreign item.
fn foreign_item_attrs(item: &syn::ForeignItem) -> &[Attribute]
{
    match *item {
        | syn::ForeignItem::Fn(ref i) => &i.attrs,
        | syn::ForeignItem::Static(ref i) => &i.attrs,
        | syn::ForeignItem::Type(ref i) => &i.attrs,
        | syn::ForeignItem::Macro(ref i) => &i.attrs,
        | _ => &[],
    }
}

/// Return a stable declaration label for an item.
fn item_declaration(item: &syn::Item) -> String
{
    match *item {
        | syn::Item::Const(ref i) => format!("const {}", i.ident),
        | syn::Item::Enum(ref i) => format!("enum {}", i.ident),
        | syn::Item::ExternCrate(ref i) => format!("extern crate {}", i.ident),
        | syn::Item::Fn(ref i) => format!("fn {}", i.sig.ident),
        | syn::Item::ForeignMod(_) => String::from("extern block"),
        | syn::Item::Impl(_) => String::from("impl"),
        | syn::Item::Macro(ref i) => i.mac.path.segments.last().map_or_else(
            || String::from("macro"),
            |segment| format!("macro {}", segment.ident),
        ),
        | syn::Item::Mod(ref i) => format!("mod {}", i.ident),
        | syn::Item::Static(ref i) => format!("static {}", i.ident),
        | syn::Item::Struct(ref i) => format!("struct {}", i.ident),
        | syn::Item::Trait(ref i) => format!("trait {}", i.ident),
        | syn::Item::TraitAlias(ref i) => format!("trait alias {}", i.ident),
        | syn::Item::Type(ref i) => format!("type {}", i.ident),
        | syn::Item::Union(ref i) => format!("union {}", i.ident),
        | syn::Item::Use(_) => String::from("use"),
        | _ => String::from("item"),
    }
}

/// Return a stable declaration label for an impl item.
fn impl_item_declaration(item: &syn::ImplItem) -> String
{
    match *item {
        | syn::ImplItem::Const(ref i) => format!("impl const {}", i.ident),
        | syn::ImplItem::Fn(ref i) => format!("impl fn {}", i.sig.ident),
        | syn::ImplItem::Macro(ref i) => i.mac.path.segments.last().map_or_else(
            || String::from("impl macro"),
            |segment| format!("impl macro {}", segment.ident),
        ),
        | syn::ImplItem::Type(ref i) => format!("impl type {}", i.ident),
        | _ => String::from("impl item"),
    }
}

/// Return a stable declaration label for a trait item.
fn trait_item_declaration(item: &syn::TraitItem) -> String
{
    match *item {
        | syn::TraitItem::Const(ref i) => format!("trait const {}", i.ident),
        | syn::TraitItem::Fn(ref i) => format!("trait fn {}", i.sig.ident),
        | syn::TraitItem::Macro(ref i) => i.mac.path.segments.last().map_or_else(
            || String::from("trait macro"),
            |segment| format!("trait macro {}", segment.ident),
        ),
        | syn::TraitItem::Type(ref i) => format!("trait type {}", i.ident),
        | _ => String::from("trait item"),
    }
}

/// Return a stable declaration label for a foreign item.
fn foreign_item_declaration(item: &syn::ForeignItem) -> String
{
    match *item {
        | syn::ForeignItem::Fn(ref i) => format!("foreign fn {}", i.sig.ident),
        | syn::ForeignItem::Macro(ref i) => i.mac.path.segments.last().map_or_else(
            || String::from("foreign macro"),
            |segment| format!("foreign macro {}", segment.ident),
        ),
        | syn::ForeignItem::Static(ref i) => format!("foreign static {}", i.ident),
        | syn::ForeignItem::Type(ref i) => format!("foreign type {}", i.ident),
        | _ => String::from("foreign item"),
    }
}

/// Return a stable declaration label for a field.
fn field_declaration(field: &syn::Field) -> String
{
    return field
        .ident
        .as_ref()
        .map_or_else(|| String::from("field"), |ident| format!("field {ident}"));
}

/// Extract nextest witnesses from any supported JSON shape.
fn witnesses_from_supported_json(value: &Value) -> Result<BTreeSet<String>, GateError>
{
    if let Some(rust_suites) = rust_suites_value(value)? {
        return Ok(witnesses_from_value(rust_suites));
    }
    if let Some(witnesses) = witnesses_from_per_test_record(value) {
        return Ok(witnesses);
    }
    return Err(unsupported_nextest_schema());
}

/// Return a supported aggregate `rust-suites` payload after schema validation.
fn rust_suites_value(value: &Value) -> Result<Option<&Value>, GateError>
{
    let Value::Object(ref object) = *value
    else {
        return Ok(None);
    };
    let Some(rust_suites) = object.get("rust-suites")
    else {
        return Ok(None);
    };
    validate_rust_suites(rust_suites)?;
    return Ok(Some(rust_suites));
}

/// Validate the top-level nextest aggregate suite collection.
fn validate_rust_suites(value: &Value) -> Result<(), GateError>
{
    match *value {
        | Value::Object(ref suites) => {
            for suite in suites.values() {
                validate_suite_record(suite)?;
            }
            return Ok(());
        },
        | Value::Array(ref suites) => {
            for suite in suites {
                validate_suite_record(suite)?;
            }
            return Ok(());
        },
        | _ => return Err(unsupported_nextest_schema()),
    }
}

/// Validate one nextest suite record before witness extraction.
fn validate_suite_record(value: &Value) -> Result<(), GateError>
{
    let Value::Object(ref suite) = *value
    else {
        return Err(unsupported_nextest_schema());
    };
    let has_package = package_context(value).into().0.is_some();
    let has_crate = crate_context(value).into().0.is_some();
    if !has_package || !has_crate {
        return Err(unsupported_nextest_schema());
    }
    let mut has_testcases = false;
    for (key, item) in suite {
        if key == "testcases" || key == "tests" {
            has_testcases = true;
            validate_testcase_collection(item)?;
        }
    }
    if has_testcases || supported_suite_status(value).into().0.is_some() {
        return Ok(());
    }
    return Err(unsupported_nextest_schema());
}

/// Return a supported zero-test suite status field.
fn supported_suite_status(value: &Value) -> impl Into<OptionalSupportedSuiteStatusText<'_>>
{
    let status = string_field(value, "status").into().0?;
    if status.is_empty() {
        return None;
    }
    return Some(status);
}

/// Work item for iterative testcase schema validation.
enum TestcaseValidationFrame<'value>
{
    /// Validate an array or map testcase collection.
    Collection(&'value Value),
    /// Validate one testcase entry.
    Entry
    {
        /// The testcase record under validation.
        value: &'value Value,
        /// Whether the record must carry a testcase name.
        requires_name: RequiresNameFlag,
    },
}

/// Work item for iterative nextest witness extraction.
enum WitnessTraversalFrame<'value>
{
    /// Visit a general JSON value while carrying package/crate context.
    Value
    {
        /// The JSON value under traversal.
        value: &'value Value,
        /// Nearest enclosing package name, if any.
        package: Option<String>,
        /// Nearest enclosing crate name, if any.
        crate_name: Option<String>,
        /// Whether this value sits inside a testcase collection.
        is_testcase_context: IsTestcaseContextFlag,
    },
    /// Visit a testcase collection while carrying package/crate context.
    Collection
    {
        /// The testcase collection under traversal.
        value: &'value Value,
        /// Nearest enclosing package name, if any.
        package: Option<String>,
        /// Nearest enclosing crate name, if any.
        crate_name: Option<String>,
    },
}

/// Validate nextest testcase arrays or maps before witness extraction.
///
/// # Termination
/// - reason: each loop validates one testcase record or existing nested
///   testcase record.
/// - measure: unvisited JSON testcase collection nodes.
/// - boundedness: `serde_json` stores a finite tree parsed from one nextest JSON
///   payload.
/// - input recursion: none.
fn validate_testcase_collection(value: &Value) -> Result<(), GateError>
{
    let mut frames = vec![TestcaseValidationFrame::Collection(value)];
    while let Some(frame) = frames.pop() {
        match frame {
            | TestcaseValidationFrame::Collection(value) => match *value {
                | Value::Array(ref items) => {
                    for item in items.iter().rev() {
                        frames.push(TestcaseValidationFrame::Entry {
                            value: item,
                            requires_name: RequiresNameFlag(true),
                        });
                    }
                },
                | Value::Object(ref object) => {
                    for item in object.values().rev() {
                        frames.push(TestcaseValidationFrame::Entry {
                            value: item,
                            requires_name: RequiresNameFlag(false),
                        });
                    }
                },
                | _ => return Err(unsupported_nextest_schema()),
            },
            | TestcaseValidationFrame::Entry {
                value,
                requires_name,
            } => validate_testcase_entry_frame(value, requires_name, &mut frames)?,
        }
    }
    Ok(())
}

/// Validate one testcase entry frame and push its children.
fn validate_testcase_entry_frame<'value, RequiresName>(
    value: &'value Value,
    requires_name: RequiresName,
    frames: &mut Vec<TestcaseValidationFrame<'value>>,
) -> Result<(), GateError>
where
    RequiresName: Into<RequiresNameFlag>,
{
    let requires_name = requires_name.into();
    match *value {
        | Value::Array(ref items) => {
            for item in items.iter().rev() {
                frames.push(TestcaseValidationFrame::Entry {
                    value: item,
                    requires_name,
                });
            }
            Ok(())
        },
        | Value::Object(ref object) => {
            let mut has_nested_collection = false;
            for (key, item) in object.iter().rev() {
                if key == "testcases" || key == "tests" {
                    has_nested_collection = true;
                    frames.push(TestcaseValidationFrame::Collection(item));
                }
            }
            if !requires_name.0
                || supported_test_record_name(value).into().0.is_some()
                || has_nested_collection
            {
                return Ok(());
            }
            Err(unsupported_nextest_schema())
        },
        | _ => Err(unsupported_nextest_schema()),
    }
}

/// Extract aliases from one supported per-test JSON record.
fn witnesses_from_per_test_record(value: &Value) -> Option<BTreeSet<String>>
{
    let name = supported_test_record_name(value).into().0?;
    let package = package_context(value).into().0;
    let crate_name = crate_context(value).into().0;
    if package.is_none() && crate_name.is_none() {
        return None;
    }
    let mut witnesses = BTreeSet::new();
    insert_aliases(&mut witnesses, name, package, crate_name);
    return Some(witnesses);
}

/// Extract nextest witnesses from any supported aggregate payload.
fn witnesses_from_value(value: &Value) -> BTreeSet<String>
{
    let mut witnesses = BTreeSet::new();
    collect_witnesses(value, None, None, false, &mut witnesses);
    return witnesses;
}

/// Walk nextest JSON with an explicit worklist while carrying package and crate
/// context.
fn collect_witnesses<'semantic, Package, CrateName, IsTestcaseContext>(
    value: &Value,
    package: Package,
    crate_name: CrateName,
    is_testcase_context: IsTestcaseContext,
    witnesses: &mut BTreeSet<String>,
)
where
    Package: Into<OptionalPackageText<'semantic>>,
    CrateName: Into<OptionalCrateNameText<'semantic>>,
    IsTestcaseContext: Into<IsTestcaseContextFlag>,
{
    collect_witness_frames(
        WitnessTraversalFrame::Value {
            value,
            package: package.into().0.map(str::to_owned),
            crate_name: crate_name.into().0.map(str::to_owned),
            is_testcase_context: is_testcase_context.into(),
        },
        witnesses,
    );
}

/// Extract nextest witnesses using an explicit worklist.
fn collect_witness_frames(
    initial: WitnessTraversalFrame<'_>,
    witnesses: &mut BTreeSet<String>,
)
{
    let mut frames = vec![initial];
    while let Some(frame) = frames.pop() {
        match frame {
            | WitnessTraversalFrame::Value {
                value,
                package,
                crate_name,
                is_testcase_context,
            } => match *value {
                | Value::Array(ref items) => {
                    for item in items.iter().rev() {
                        frames.push(WitnessTraversalFrame::Value {
                            value: item,
                            package: package.clone(),
                            crate_name: crate_name.clone(),
                            is_testcase_context,
                        });
                    }
                },
                | Value::Object(ref object) => {
                    let next_package = package_context(value)
                        .into()
                        .0
                        .map(str::to_owned)
                        .or(package);
                    let next_crate = crate_context(value)
                        .into()
                        .0
                        .map(str::to_owned)
                        .or(crate_name);
                    if let Some(name) = test_name(value, is_testcase_context).into().0 {
                        insert_aliases(
                            witnesses,
                            name,
                            next_package.as_deref(),
                            next_crate.as_deref(),
                        );
                    }
                    for (key, item) in object.iter().rev() {
                        if key == "testcases" || key == "tests" {
                            frames.push(WitnessTraversalFrame::Collection {
                                value: item,
                                package: next_package.clone(),
                                crate_name: next_crate.clone(),
                            });
                        }
                        else {
                            frames.push(WitnessTraversalFrame::Value {
                                value: item,
                                package: next_package.clone(),
                                crate_name: next_crate.clone(),
                                is_testcase_context,
                            });
                        }
                    }
                },
                | _ => {},
            },
            | WitnessTraversalFrame::Collection {
                value,
                package,
                crate_name,
            } => {
                push_testcase_collection_frames(value, package.as_deref(), crate_name.as_deref(), witnesses, &mut frames);
            },
        }
    }
}

/// Push testcase collection children onto the witness worklist.
fn push_testcase_collection_frames<'value>(
    value: &'value Value,
    package: Option<&str>,
    crate_name: Option<&str>,
    witnesses: &mut BTreeSet<String>,
    frames: &mut Vec<WitnessTraversalFrame<'value>>,
)
{
    match *value {
        | Value::Array(ref items) => {
            for item in items.iter().rev() {
                frames.push(WitnessTraversalFrame::Value {
                    value: item,
                    package: package.map(String::from),
                    crate_name: crate_name.map(String::from),
                    is_testcase_context: IsTestcaseContextFlag(true),
                });
            }
        },
        | Value::Object(ref object) => {
            for (name, item) in object.iter().rev() {
                insert_aliases(witnesses, name, package, crate_name);
                frames.push(WitnessTraversalFrame::Value {
                    value: item,
                    package: package.map(String::from),
                    crate_name: crate_name.map(String::from),
                    is_testcase_context: IsTestcaseContextFlag(true),
                });
            }
        },
        | _ => {},
    }
}

/// Build the stable operational error for unsupported nextest list JSON.
fn unsupported_nextest_schema() -> GateError
{
    return GateError::operational(
        "unsupported nextest list schema: expected aggregate object with rust-suites object/array or per-test record with test name and package/crate context",
    );
}

/// Return a JSON object's test name when it resembles a test case record.
fn test_name<IsTestcaseContext>(
    value: &Value,
    is_testcase_context: IsTestcaseContext,
) -> impl Into<OptionalTestNameText<'_>>
where
    IsTestcaseContext: Into<IsTestcaseContextFlag>,
{
    let is_testcase_context = is_testcase_context.into().0;
    let name = supported_test_record_name(value).into().0?;
    if is_testcase_context
        || name.contains("::")
        || string_field(value, "test_name").into().0.is_some()
        || value.get("status").is_some()
        || package_context(value).into().0.is_some()
        || crate_context(value).into().0.is_some()
    {
        return Some(name);
    }
    return None;
}

/// Return a supported test-name field from a JSON object.
fn supported_test_record_name(value: &Value) -> impl Into<OptionalSupportedTestRecordNameText<'_>>
{
    return string_field(value, "test_name")
        .into()
        .0
        .or_else(|| string_field(value, "name").into().0);
}

/// Return recognized package context from a JSON object.
fn package_context(value: &Value) -> impl Into<OptionalPackageContextText<'_>>
{
    return string_field(value, "package")
        .into()
        .0
        .or_else(|| string_field(value, "package_name").into().0)
        .or_else(|| string_field(value, "package-name").into().0);
}

/// Return recognized crate context from a JSON object.
fn crate_context(value: &Value) -> impl Into<OptionalCrateContextText<'_>>
{
    return string_field(value, "crate")
        .into()
        .0
        .or_else(|| string_field(value, "crate_name").into().0)
        .or_else(|| string_field(value, "binary").into().0)
        .or_else(|| string_field(value, "binary_name").into().0)
        .or_else(|| string_field(value, "binary-name").into().0);
}

/// Insert exact aliases for one nextest test.
///
/// # Contract
/// - requires: `package` and `crate_name`, when present, are the nextest
///   contexts for `name`.
/// - ensures: inserts the raw name and every available package- and
///   binary-qualified spelling.
/// - ensures: when the normalized package and binary names differ and `name` is
///   module-qualified, also inserts the normalized package name joined to the
///   test name after its first module, without repeating that crate name when
///   the remaining test name is already crate-qualified.
/// - provides: exact adequacy-witness aliases across library, standalone
///   integration, and consolidated integration test binaries.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — equal package/binary library suites,
///   differing-package/binary consolidated suites, and ordinary versus
///   already-crate-qualified module tails are separated by exact presence and
///   absence assertions for the module-stripped alias.
/// - witness: `contracts::accepts_exact_raw_package_and_crate_aliases_from_nextest_shapes`
/// - witness: `contracts::accepts_harness_module_stripped_aliases_from_consolidated_integration_suites`
fn insert_aliases<'semantic, Name, Package, CrateName>(
    witnesses: &mut BTreeSet<String>,
    name: Name,
    package: Package,
    crate_name: CrateName,
)
where
    Name: Into<NameText<'semantic>>,
    Package: Into<OptionalPackageText<'semantic>>,
    CrateName: Into<OptionalCrateNameText<'semantic>>,
{
    let package = package.into().0;
    let crate_name = crate_name.into().0;
    let name = name.into().0;
    witnesses.insert(String::from(name));

    let package_crate_alias = package.map(|package_name| package_name.replace('-', "_"));
    if let Some(package_name) = package {
        witnesses.insert(format!("{package_name}::{name}"));
    }
    if let Some(package_alias) = package_crate_alias.as_deref() {
        witnesses.insert(format!("{package_alias}::{name}"));
    }

    let binary_crate_alias = crate_name.map(|crate_alias| crate_alias.replace('-', "_"));
    if let Some(crate_alias) = crate_name {
        witnesses.insert(format!("{crate_alias}::{name}"));
    }
    if let Some(binary_alias) = binary_crate_alias.as_deref() {
        witnesses.insert(format!("{binary_alias}::{name}"));
    }

    if let (Some(package_alias), Some(binary_alias)) = (
        package_crate_alias.as_deref(),
        binary_crate_alias.as_deref(),
    ) && package_alias != binary_alias
        && let Some((module, stripped_name)) = name.split_once("::")
        && !module.is_empty()
        && !stripped_name.is_empty()
    {
        let crate_relative_name = stripped_name
            .strip_prefix(package_alias)
            .and_then(|tail| tail.strip_prefix("::"))
            .unwrap_or(stripped_name);
        if !crate_relative_name.is_empty() {
            witnesses.insert(format!("{package_alias}::{crate_relative_name}"));
        }
    }
}

/// Read a string field from a JSON object.
fn string_field<'semantic, 'value, Name>(
    value: &'value Value,
    name: Name,
) -> impl Into<OptionalStringFieldText<'value>>
where
    Name: Into<NameText<'semantic>>,
{
    let name = name.into().0;
    return value.get(name).and_then(Value::as_str);
}

#[cfg(test)]
mod tests
{
    use core::error::Error;

    use super::*;

    type TestResult = Result<(), Box<dyn Error>>;

    fn witnesses<'semantic, Names, Name>(names: Names) -> BTreeSet<String>
    where
        Names: IntoIterator<Item = Name>,
        Name: Into<NameText<'semantic>>,
    {
        names
            .into_iter()
            .map(|name| String::from(name.into().0))
            .collect()
    }

    fn finding_pairs(findings: &[Finding]) -> BTreeSet<(String, String)>
    {
        findings
            .iter()
            .map(|finding| (finding.kind.clone(), finding.detail.clone()))
            .collect()
    }

    #[test]
    fn live_nextest_listing_uses_enabled_workspace()
    {
        assert_eq!(NEXTEST_LIST_ARGS, &[
            "nextest",
            "list",
            "--workspace",
            "--all-targets",
            "--features=full",
            "--message-format",
            "json",
        ]);
    }

    #[test]
    fn run_and_scope_errors_are_typed() -> TestResult
    {
        assert!(
            matches!(
                run(&[], None),
                Err(GateError::Usage { detail }) if detail == "contracts requires at least one --scope path"
            ),
            "empty contract scopes should be a usage error"
        );

        let fixture_error = read_fixture_witnesses(Path::new("missing-nextest-fixture.json"))
            .err()
            .ok_or_else(|| GateError::operational("missing fixture unexpectedly loaded"))?;
        assert!(
            matches!(fixture_error, GateError::Io { path, .. } if path.as_path() == Path::new("missing-nextest-fixture.json")),
            "missing nextest fixtures should retain the fixture path"
        );

        let scope_error = rust_sources(Path::new("missing-contract-scope"))
            .err()
            .ok_or_else(|| GateError::operational("missing scope unexpectedly walked"))?;
        assert!(
            matches!(scope_error, GateError::Io { path, .. } if path.as_path() == Path::new("missing-contract-scope")),
            "missing contract scopes should retain the scope path"
        );
        Ok(())
    }

    #[test]
    fn contract_grammar_modes_are_reported_deterministically() -> TestResult
    {
        let source = r#"
/// ## Contract
fn wrong_level() {}

/// # Adequacy
/// - hypothesis: L3 point.
/// - witness: `grammar::present`
/// # Contract
/// - panics: none.
fn adequacy_before_contract() {}

/// # Errors
/// # Contract
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
/// - witness: `grammar::present`
fn errors_before_contract() {}

/// # Contract
/// - panics: none.
fn missing_adequacy() {}

/// # Contract
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
/// - witness: `grammar::present`
/// # Errors
fn errors_after_adequacy() {}

/// # Contract
/// - requires: one.
/// - requires: duplicate.
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
/// - witness: `grammar::present`
fn duplicate_clause() {}

/// # Contract
/// - intension: before panics.
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
/// - witness: `grammar::present`
fn intension_not_last() {}

/// # Contract
/// - ensures: before requires.
/// - requires: after ensures.
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
/// - witness: `grammar::present`
fn out_of_order_clause() {}

/// # Contract
/// free prose is not a bullet.
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
/// - witness: `grammar::present`
fn contract_prose() {}

/// # Contract
/// - ensures: lacks panics.
/// # Adequacy
/// - hypothesis: L3 point.
/// - witness: `grammar::present`
fn missing_panics() {}

/// # Contract
/// - panics: none.
/// # Adequacy
/// - hypothesis without colon.
fn malformed_hypothesis() {}

/// # Contract
/// - panics: none.
/// # Adequacy
/// - witness: `grammar::present`
/// - hypothesis: L3 point.
fn late_hypothesis() {}

/// # Contract
/// - panics: none.
/// # Adequacy
/// - hypothesis: point without rung.
fn hypothesis_without_rung() {}

/// # Contract
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
/// - witness:
fn malformed_witness() {}

/// # Contract
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
/// free-form adequacy prose.
fn adequacy_prose() {}

/// # Contract
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
fn missing_witnesses() {}

/// # Contract
/// - panics: none.
/// # Adequacy
fn missing_hypothesis() {}

/// # Contract
/// - panics: none.
/// # Adequacy
/// - hypothesis: L3 point.
/// - witness: `grammar::absent`
fn stale_witness() {}
"#;
        let findings = analyze_source(
            Path::new("grammar.rs"),
            source,
            &witnesses(["grammar::present"]),
        )?;
        let pairs = finding_pairs(&findings);
        let expected = [
            "# Adequacy must appear after # Contract and optional # Errors",
            "Contract section must use exactly one # heading: ## Contract",
            "missing later # Adequacy section",
            "# Errors must appear after # Contract",
            "# Errors must appear between # Contract and # Adequacy",
            "duplicate # Contract clause: requires",
            "- intension: must be the final # Contract clause",
            "# Contract clause is out of order: requires",
            "free-form # Contract prose: free prose is not a bullet.",
            "# Contract section must include - panics:",
            "malformed hypothesis line: - hypothesis without colon.",
            "- hypothesis: must precede all witness bullets",
            "- hypothesis: must name L0, L1, L2, or L3",
            "malformed witness line: - witness:",
            "free-form # Adequacy prose: free-form adequacy prose.",
            "# Adequacy section has no exact - witness: `path` bullet",
            "# Adequacy section must start with exactly one - hypothesis: bullet",
            "no witness matched nextest aliases: grammar::absent",
        ];
        for detail in expected {
            assert!(
                pairs.contains(&(String::from("missing-contract"), String::from(detail)))
                    || pairs
                        .contains(&(String::from("section-heading-level"), String::from(detail)))
                    || pairs.contains(&(String::from("missing-adequacy"), String::from(detail)))
                    || pairs.contains(&(String::from("section-order"), String::from(detail)))
                    || pairs.contains(&(
                        String::from("duplicate-contract-clause"),
                        String::from(detail)
                    ))
                    || pairs.contains(&(String::from("intension-not-last"), String::from(detail)))
                    || pairs.contains(&(
                        String::from("out-of-order-contract-clause"),
                        String::from(detail)
                    ))
                    || pairs.contains(&(String::from("contract-prose"), String::from(detail)))
                    || pairs.contains(&(String::from("missing-panics"), String::from(detail)))
                    || pairs
                        .contains(&(String::from("malformed-hypothesis"), String::from(detail)))
                    || pairs.contains(&(String::from("late-hypothesis"), String::from(detail)))
                    || pairs.contains(&(String::from("hypothesis-rung"), String::from(detail)))
                    || pairs.contains(&(String::from("malformed-witness"), String::from(detail)))
                    || pairs.contains(&(String::from("adequacy-prose"), String::from(detail)))
                    || pairs.contains(&(String::from("missing-witness"), String::from(detail)))
                    || pairs.contains(&(String::from("missing-hypothesis"), String::from(detail)))
                    || pairs.contains(&(String::from("stale-witness"), String::from(detail))),
                "expected grammar finding detail `{detail}` in {pairs:?}"
            );
        }
        assert_eq!(findings.len(), expected.len());
        Ok(())
    }

    #[test]
    fn nextest_json_lines_and_schema_edges_are_exact() -> TestResult
    {
        let json_lines = r#"
{"test_name":"integration::contracts::case","package_name":"gandr-workflow-gates","binary_name":"gates"}

{"name":"unit_case","package":"gandr-workflow-gates","crate":"gandr_workflow_gates"}
"#;
        let aliases = parse_nextest_witnesses(json_lines)?;
        assert!(aliases.contains("integration::contracts::case"));
        assert!(aliases.contains("gandr-workflow-gates::integration::contracts::case"));
        assert!(aliases.contains("gates::integration::contracts::case"));
        assert!(aliases.contains("gandr_workflow_gates::contracts::case"));
        assert!(aliases.contains("unit_case"));

        let aggregate = r#"{
  "rust-suites": {
    "suite": {
      "package_name": "pkg-name",
      "binary_name": "integration",
      "status": "SKIPPED",
      "testcases": [[{"name": "outer::pkg_name::case"}]]
    }
  }
}"#;
        let aggregate_aliases = parse_nextest_witnesses(aggregate)?;
        assert!(aggregate_aliases.contains("pkg_name::outer::pkg_name::case"));
        assert!(aggregate_aliases.contains("pkg_name::case"));

        let testcase_map = r#"{
  "rust-suites": [
    {
      "package": "pkg",
      "crate": "bin",
      "tests": {
        "map_case": {"status": "PASS"}
      }
    }
  ]
}"#;
        assert!(parse_nextest_witnesses(testcase_map)?.contains("map_case"));

        assert!(matches!(
            parse_nextest_witnesses(" \n "),
            Err(GateError::Json { source_name, .. }) if source_name == "nextest list"
        ));
        assert!(matches!(
            parse_nextest_witnesses("{not-json}\n"),
            Err(GateError::Json { source_name, .. }) if source_name == "nextest list"
        ));
        assert!(matches!(
            parse_nextest_witnesses(r#"{"event":"unknown"}"#),
            Err(GateError::Operational { detail }) if detail.contains("unsupported nextest list schema")
        ));
        assert!(matches!(
            parse_nextest_witnesses(r#"{"rust-suites":[{"package":"pkg","crate":"bin","status":""}]}"#),
            Err(GateError::Operational { detail }) if detail.contains("unsupported nextest list schema")
        ));
        assert!(matches!(
            parse_nextest_witnesses(r#"{"rust-suites":[{"package":"pkg","crate":"bin","testcases":[1]}]}"#),
            Err(GateError::Operational { detail }) if detail.contains("unsupported nextest list schema")
        ));
        Ok(())
    }

    #[test]
    fn syn_owner_inventory_covers_declaration_surfaces() -> TestResult
    {
        let source = r#"
extern crate alloc;
use alloc::vec::Vec;
const C: usize = 0;
enum E { Named { field: usize }, Tuple(usize), Unit }
fn function() {}
unsafe extern "C" {
    fn foreign_fn();
    static FOREIGN: usize;
    type ForeignType;
    foreign_macro!();
}
impl S {
    const IMPL_CONST: usize = 0;
    fn method(&self) {}
    type ImplType = usize;
    impl_macro!();
}
macro_rules! local_macro { () => {} }
mod nested {}
static STATIC_VALUE: usize = 0;
struct S { field: usize }
trait Trait {
    const TRAIT_CONST: usize;
    fn trait_fn();
    type TraitType;
    trait_macro!();
}
trait Alias = Trait;
type TypeAlias = usize;
union Union { field: u32 }
"#;
        let findings = analyze_source(Path::new("owners.rs"), source, &BTreeSet::new())?;
        assert!(
            findings.is_empty(),
            "owner inventory without contract docs should not emit findings"
        );
        Ok(())
    }

    #[test]
    fn shell_classifier_covers_substitution_and_word_edges() -> TestResult
    {
        let substitution = prohibited_invocation(r#"echo \"$(cargo test)\""#)
            .ok_or_else(|| GateError::operational("command substitution was not classified"))?;
        assert!(
            substitution.tool == ProhibitedTool::Cargo,
            "command substitution should report cargo"
        );
        assert_eq!("cargo test", substitution.command);

        let dynamic_substitution = prohibited_invocation("echo $(cargo test").ok_or_else(|| {
            GateError::operational("unterminated substitution was not classified")
        })?;
        assert!(
            dynamic_substitution.tool == ProhibitedTool::Dynamic,
            "unterminated substitution should be dynamic"
        );

        let backtick = prohibited_invocation("echo `treefmt --check`")
            .ok_or_else(|| GateError::operational("backtick substitution was not classified"))?;
        assert!(
            backtick.tool == ProhibitedTool::Treefmt,
            "backtick substitution should report treefmt"
        );

        let dynamic_backtick = prohibited_invocation("echo `unterminated")
            .ok_or_else(|| GateError::operational("unterminated backtick was not classified"))?;
        assert!(
            dynamic_backtick.tool == ProhibitedTool::Dynamic,
            "unterminated backtick should be dynamic"
        );

        let function = prohibited_invocation("function build { cargo test; }")
            .ok_or_else(|| GateError::operational("function dispatch was not classified"))?;
        assert!(
            function.tool == ProhibitedTool::Dynamic,
            "function declarations should be dynamic dispatch"
        );

        let rustup_missing_tool = classify_command_segment("rustup run")
            .ok_or_else(|| GateError::operational("missing rustup payload was not classified"))?;
        assert!(
            rustup_missing_tool.tool == ProhibitedTool::Dynamic,
            "missing rustup toolchain payload should be dynamic"
        );

        let rustup_unknown_tool = classify_command_segment("rustup run stable just ci")
            .ok_or_else(|| {
                GateError::operational("unknown rustup wrapped command was not classified")
            })?;
        assert!(
            rustup_unknown_tool.tool == ProhibitedTool::Dynamic,
            "unknown rustup wrapped commands should be dynamic"
        );

        assert!(classify_command_segment("[ cargo ]").is_none());
        assert!(classify_command_segment("NAME='cargo test'").is_none());
        assert!(is_shell_identifier("_valid9").into().0);
        assert!(!is_shell_identifier("").into().0);
        assert!(!is_environment_assignment("=value").into().0);
        assert!(!is_environment_assignment("1=value").into().0);

        let mut empty_words =
            [WordText("--"), WordText("+nightly"), WordText("-Zunstable")].into_iter();
        assert_eq!(None, next_tool_subcommand(&mut empty_words).into().0);
        Ok(())
    }
}
