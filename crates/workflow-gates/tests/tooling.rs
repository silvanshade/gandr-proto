//! Integration witnesses for the unified CLI tooling surface.
//!
//! These tests include the binary parser as a module so command parsing,
//! default path projection, and pure plans can be observed without invoking
//! shell scripts, workflow tasks, AFL, or other external tools.

#[path = "../src/main.rs"]
mod cli;

use core::error::Error;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use gandr_workflow_gates::GateError;
use gandr_workflow_gates::mutants;
use gandr_workflow_gates::semantic_value;

/// Shared integration-test result type.
type TestResult<T = ()> = Result<T, Box<dyn Error>>;

gandr_workflow_gates::semantic_str!(pub(crate) struct KeyText);
gandr_workflow_gates::semantic_str!(pub(crate) struct ContextText);
gandr_workflow_gates::semantic_str!(pub(crate) struct MiseText);
gandr_workflow_gates::semantic_str!(pub(crate) struct TaskText);
gandr_workflow_gates::semantic_str!(pub(crate) struct ExpectedText);
gandr_workflow_gates::semantic_str!(pub(crate) struct ModeText);
gandr_workflow_gates::semantic_str!(pub(crate) struct SuffixText);
gandr_workflow_gates::semantic_str!(pub(crate) struct RelativeText);
gandr_workflow_gates::semantic_str!(pub(crate) struct TextText);
gandr_workflow_gates::semantic_str!(pub(crate) struct NodesYamlText);
gandr_workflow_gates::semantic_str!(pub(crate) struct ScriptText);
gandr_workflow_gates::semantic_str!(pub(crate) struct NameText);
gandr_workflow_gates::semantic_bytes!(pub(crate) struct BytesBytes);
gandr_workflow_gates::semantic_str!(pub(crate) struct TableStringText);

impl<'text> From<TableStringText<'text>> for ScriptText<'text>
{
    #[inline]
    fn from(value: TableStringText<'text>) -> Self
    {
        Self(value.0)
    }
}

impl<'item, 'text> From<&'item &'text str> for TextText<'text>
{
    #[inline]
    fn from(value: &'item &'text str) -> Self
    {
        Self(value)
    }
}

impl<'item, 'text> From<&'item &'text str> for KeyText<'text>
{
    #[inline]
    fn from(value: &'item &'text str) -> Self
    {
        Self(value)
    }
}

impl<'item, 'text> From<&'item &'text str> for ExpectedText<'text>
{
    #[inline]
    fn from(value: &'item &'text str) -> Self
    {
        Self(value)
    }
}

/// Stable top-level command inventory expected by local tooling.
const EXPECTED_COMMANDS: &[&str] = &[
    "contracts",
    "ci-contracts",
    "graph-boundary",
    "embedded-syntax",
    "docs-manifest",
    "docs-reference",
    "page-balance",
    "rumdl",
    "soundness-oracles",
    "default-graph",
    "coverage",
    "maintenance-range",
    "mutants",
    "workflow",
    "fuzz-smoke",
];

/// Immutable upstream Dylint revision that owns the lint inventory below.
const EXPECTED_DYLINT_REV: &str = "ae6676727569eabeb7bd4b58773549b342511bad";

/// Project-local Dylint library path in workspace metadata.
const EXPECTED_LOCAL_DYLINT_PATH: &str = "crates/workflow-dylint";

/// Project-local Dylint library selector used by cargo-dylint.
const EXPECTED_LOCAL_DYLINT_LIB: &str = "gandr_workflow_dylint";

/// First cargo command in `cargo:dylint:upstream`: the project-local UI
/// contract.
const EXPECTED_DYLINT_UI_TEST_COMMAND: &[&str] = &[
    "cargo",
    "test",
    "-p",
    "gandr-workflow-dylint",
    "ui",
    "--",
    "--nocapture",
];

/// Keys the composed `cargo:dylint` task body carries.
const EXPECTED_DYLINT_FACADE_KEYS: &[&str] = &["description", "run"];

/// Strict lanes the composed `cargo:dylint` task runs, in order.
const EXPECTED_DYLINT_FACADE_TASKS: &[&str] = &["cargo:dylint:local", "cargo:dylint:upstream"];

/// Exact Clippy command for every lint-eligible workspace target, driver
/// included (the Dylint driver is an in-workspace crate). The `"$@"` scope
/// hole takes the task's trailing package arguments and defaults to the whole
/// workspace — the default is locked separately, so the merge wall's bare
/// invocation keeps the enabled-workspace scope. Package-scoped invocations
/// qualify `full` per package, including mixed selections.
const EXPECTED_CLIPPY_WORKSPACE_COMMAND: &[&str] = &[
    "cargo",
    "clippy",
    "\"$@\"",
    "--all-targets",
    "--",
    "-D",
    "warnings",
    "-A",
    "clippy::std_instead_of_core",
];

/// Cargo invocation the merge-wall rustdoc gate must run over the workspace.
const EXPECTED_DOC_CHECK_COMMAND: &[&str] = &[
    "cargo",
    "doc",
    "--workspace",
    "--features=full",
    "--no-deps",
    "--document-private-items",
];

/// Cargo metadata plugin-library paths for the supported upstream Dylint sets.
const EXPECTED_DYLINT_PLUGIN_PATHS: &[&str] = &[
    "examples/general/abs_home_path",
    "examples/general/await_holding_span_guard",
    "examples/general/basic_dead_store",
    "examples/general/crate_wide_allow",
    "examples/general/incorrect_matches_operation",
    "examples/general/non_thread_safe_call_in_test",
    "examples/general/wrong_serialize_struct_arg",
    "examples/supplementary",
    "examples/restriction/assert_eq_arg_misordering",
    "examples/restriction/collapsible_unwrap",
    "examples/restriction/const_path_join",
    "examples/restriction/env_literal",
    "examples/restriction/inconsistent_qualification",
    "examples/restriction/question_mark_in_expression",
    "examples/restriction/ref_aware_redundant_closure_for_method_calls",
    "examples/restriction/register_lints_warn",
    "examples/restriction/suboptimal_pattern",
    "examples/restriction/try_io_result",
];

/// Upstream lint paths represented by the supplementary wrapper plugin.
const EXPECTED_DYLINT_SUPPLEMENTARY_LINT_PATHS: &[&str] = &[
    "examples/supplementary/commented_out_code",
    "examples/supplementary/concatenable_format_args",
    "examples/supplementary/escaping_doc_link",
    "examples/supplementary/inconsistent_struct_pattern",
    "examples/supplementary/local_ref_cell",
    "examples/supplementary/nonexistent_path_in_comment",
    "examples/supplementary/redundant_reference",
    "examples/supplementary/unnamed_constant",
    "examples/supplementary/unnecessary_borrow_mut",
    "examples/supplementary/unnecessary_conversion_for_trait",
];

/// Full upstream Dylint lint inventory represented by workspace metadata.
const EXPECTED_DYLINT_REPRESENTED_LINT_PATHS: &[&str] = &[
    "examples/general/abs_home_path",
    "examples/general/await_holding_span_guard",
    "examples/general/basic_dead_store",
    "examples/general/crate_wide_allow",
    "examples/general/incorrect_matches_operation",
    "examples/general/non_thread_safe_call_in_test",
    "examples/general/wrong_serialize_struct_arg",
    "examples/supplementary/commented_out_code",
    "examples/supplementary/concatenable_format_args",
    "examples/supplementary/escaping_doc_link",
    "examples/supplementary/inconsistent_struct_pattern",
    "examples/supplementary/local_ref_cell",
    "examples/supplementary/nonexistent_path_in_comment",
    "examples/supplementary/redundant_reference",
    "examples/supplementary/unnamed_constant",
    "examples/supplementary/unnecessary_borrow_mut",
    "examples/supplementary/unnecessary_conversion_for_trait",
    "examples/restriction/assert_eq_arg_misordering",
    "examples/restriction/collapsible_unwrap",
    "examples/restriction/const_path_join",
    "examples/restriction/env_literal",
    "examples/restriction/inconsistent_qualification",
    "examples/restriction/question_mark_in_expression",
    "examples/restriction/ref_aware_redundant_closure_for_method_calls",
    "examples/restriction/register_lints_warn",
    "examples/restriction/suboptimal_pattern",
    "examples/restriction/try_io_result",
];

/// Upstream libraries that share the ordinary warning-denying driver pass.
const EXPECTED_UPSTREAM_DYLINT_LIBS: &[&str] = &[
    "abs_home_path",
    "assert_eq_arg_misordering",
    "await_holding_span_guard",
    "basic_dead_store",
    "collapsible_unwrap",
    "const_path_join",
    "env_literal",
    "inconsistent_qualification",
    "incorrect_matches_operation",
    "non_thread_safe_call_in_test",
    "question_mark_in_expression",
    "ref_aware_redundant_closure_for_method_calls",
    "suboptimal_pattern",
    "supplementary",
    "try_io_result",
    "wrong_serialize_struct_arg",
];

/// Workflow-plan tasks the reboot has not restored to the mise task surface.
///
/// `cargo:no-panic` returns with the release link-time no-panic smoke,
/// `core:check` with the vendored agentic-dev core, `grammar:test` with the
/// tree-sitter grammar port, and `wrkflw` with the hosted workflow surface;
/// the push hook stays parked until all four exist.
const EXPECTED_PARKED_WORKFLOW_TASKS: &[&str] =
    &["cargo:no-panic", "core:check", "grammar:test", "wrkflw"];

/// Keys the `gate:merge` task body carries; anything else adds unreplayed work.
const EXPECTED_GATE_MERGE_KEYS: &[&str] = &["description", "run"];

/// Keys one `gate:merge` run step carries; anything else scopes the invocation.
const EXPECTED_GATE_MERGE_STEP_KEYS: &[&str] = &["task"];

/// Merge-wall tasks the `gate:merge` task body runs, in order.
const EXPECTED_MERGE_GATE_TASKS: &[&str] = &[
    "toolchain:pin-check",
    "docs:conflict-markers",
    "test:dep-graph",
    "cargo:embedded-syntax",
    "cargo:build",
    "cargo:clippy",
    "cargo:dylint:local",
    "cargo:doc-check",
    "cargo:nextest",
    "compile-host:wall",
    "treefmt:check",
];

/// Per-process suffix keeping concurrently-created CLI fixtures disjoint.
static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

/// Convert string slices to owned OS arguments.
fn os_args<'semantic, Values, Value>(values: Values) -> Vec<OsString>
where
    Values: IntoIterator<Item = Value>,
    Value: Into<TextText<'semantic>>,
{
    values
        .into_iter()
        .map(|value| OsString::from(value.into().0))
        .collect()
}

/// Convert string slices to owned OS strings for argv assertions.
fn os_strings<'semantic, Values, Value>(values: Values) -> Vec<OsString>
where
    Values: IntoIterator<Item = Value>,
    Value: Into<TextText<'semantic>>,
{
    values
        .into_iter()
        .map(|value| OsString::from(value.into().0))
        .collect()
}

/// Return the workspace root that owns shared tooling files.
fn workspace_root() -> TestResult<PathBuf>
{
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Some(workspace) = manifest_dir.parent().and_then(Path::parent)
    else {
        return Err(Box::new(std::io::Error::other(
            "crate manifest is not under workspace root",
        )));
    };
    Ok(workspace.to_path_buf())
}

/// Return the workspace mise tasks directory.
fn workspace_mise_tasks(workspace: &Path) -> PathBuf
{
    let mut workspace_mise_tasks = workspace.to_path_buf();
    workspace_mise_tasks.extend([".config", "mise", "tasks"]);
    workspace_mise_tasks
}

/// Parse a workspace TOML file into a structural value.
fn parse_toml_file(path: &Path) -> TestResult<toml::Value>
{
    let text = gandr_workflow_gates::support::HOST_FILESYSTEM.read_to_string(path)?;
    let value = toml::from_str(&text)?;
    Ok(value)
}

/// Locate a nested TOML value.
fn toml_value_at<'semantic, 'document, Segments, Segment>(
    mut value: &'document toml::Value,
    path: Segments,
) -> TestResult<&'document toml::Value>
where
    Segments: IntoIterator<Item = Segment>,
    Segment: Into<KeyText<'semantic>>,
{
    let segments = path
        .into_iter()
        .map(Into::into)
        .collect::<Vec<KeyText<'semantic>>>();
    let mut walked = String::new();
    for key in &segments {
        let key = key.0;
        if !walked.is_empty() {
            walked.push('.');
        }
        walked.push_str(key);
        let Some(next) = value.get(key)
        else {
            return Err(Box::new(std::io::Error::other(format!(
                "missing TOML value `{walked}`"
            ))));
        };
        value = next;
    }
    Ok(value)
}

/// Read a string from a TOML table.
fn toml_table_string<'semantic, 'table, Key>(
    table: &'table toml::Table,
    key: Key,
) -> TestResult<TableStringText<'table>>
where
    Key: Into<KeyText<'semantic>>,
{
    let key = key.into().0;
    let Some(value) = table.get(key)
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "missing TOML string `{key}`"
        ))));
    };
    let Some(text) = value.as_str()
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "TOML value `{key}` is not a string"
        ))));
    };
    Ok(TableStringText(text))
}

/// Read a string array from a TOML table.
fn toml_table_string_array<'semantic, Key>(
    table: &toml::Table,
    key: Key,
) -> TestResult<Vec<String>>
where
    Key: Into<KeyText<'semantic>>,
{
    let key = key.into().0;
    let Some(value) = table.get(key)
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "missing TOML array `{key}`"
        ))));
    };
    let Some(array) = value.as_array()
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "TOML value `{key}` is not an array"
        ))));
    };
    let mut strings = Vec::new();
    for (position, item) in array.iter().enumerate() {
        let Some(text) = item.as_str()
        else {
            return Err(Box::new(std::io::Error::other(format!(
                "TOML array `{key}` item {position} is not a string"
            ))));
        };
        strings.push(text.to_owned());
    }
    Ok(strings)
}

/// Return the workspace Dylint metadata library tables.
fn dylint_metadata_libraries(manifest: &toml::Value) -> TestResult<Vec<&toml::Table>>
{
    let dylint = toml_table_at(manifest, ["workspace", "metadata", "dylint"])?;
    let Some(libraries_value) = dylint.get("libraries")
    else {
        return Err(Box::new(std::io::Error::other(
            "missing workspace.metadata.dylint.libraries",
        )));
    };
    let Some(libraries) = libraries_value.as_array()
    else {
        return Err(Box::new(std::io::Error::other(
            "workspace.metadata.dylint.libraries is not an array",
        )));
    };
    if libraries.len() != 2_usize {
        return Err(Box::new(std::io::Error::other(format!(
            "expected two Dylint metadata libraries, found {}",
            libraries.len()
        ))));
    }
    let mut tables = Vec::with_capacity(libraries.len());
    for (index, library) in libraries.iter().enumerate() {
        let Some(table) = library.as_table()
        else {
            return Err(Box::new(std::io::Error::other(format!(
                "Dylint metadata library {index} is not a table"
            ))));
        };
        tables.push(table);
    }
    Ok(tables)
}

/// Compare an owned string sequence to an expected static inventory.
fn assert_string_sequence<'semantic, Expected, ExpectedItem, Context>(
    actual: &[String],
    expected: Expected,
    context: Context,
) where
    Expected: IntoIterator<Item = ExpectedItem>,
    ExpectedItem: Into<ExpectedText<'semantic>>,
    Context: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let actual_text = actual.iter().map(String::as_str).collect::<Vec<_>>();
    let expected_text = expected
        .into_iter()
        .map(|item| item.into().0)
        .collect::<Vec<_>>();
    assert_eq!(
        actual_text.as_slice(),
        expected_text.as_slice(),
        "{context}"
    );
}

/// Expand the supplementary wrapper into the documented lint inventory.
fn represented_dylint_lint_paths(plugin_paths: &[String]) -> Vec<String>
{
    let mut represented = Vec::new();
    for path in plugin_paths {
        if path == "examples/supplementary" {
            represented.extend(
                EXPECTED_DYLINT_SUPPLEMENTARY_LINT_PATHS
                    .iter()
                    .map(|&supplementary_path| supplementary_path.to_owned()),
            );
        }
        else {
            represented.push(path.to_owned());
        }
    }
    represented
}

/// Locate a nested TOML table.
fn toml_table_at<'semantic, 'document, Segments, Segment>(
    value: &'document toml::Value,
    path: Segments,
) -> TestResult<&'document toml::Table>
where
    Segments: IntoIterator<Item = Segment>,
    Segment: Into<KeyText<'semantic>>,
{
    let segments = path
        .into_iter()
        .map(Into::into)
        .collect::<Vec<KeyText<'semantic>>>();
    let table = toml_value_at(value, segments.iter().map(|segment| segment.0))?;
    let Some(table) = table.as_table()
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "TOML value `{}` is not a table",
            dotted_toml_path(segments.iter().map(|segment| segment.0))
        ))));
    };
    Ok(table)
}

/// Return a dotted TOML path for diagnostics.
fn dotted_toml_path<'semantic, Segments, Segment>(path: Segments) -> String
where
    Segments: IntoIterator<Item = Segment>,
    Segment: Into<KeyText<'semantic>>,
{
    path.into_iter()
        .map(|segment| segment.into().0)
        .collect::<Vec<_>>()
        .join(".")
}

/// Parse cargo-dylint invocations from a mise task run script.
fn parse_dylint_invocations<'semantic, Script>(script: Script) -> TestResult<Vec<DylintInvocation>>
where
    Script: Into<ScriptText<'semantic>>,
{
    let script = script.into().0;
    let mut invocations = Vec::new();
    for command in logical_shell_commands(script) {
        let mut tokens = command.split_whitespace();
        let Some(program) = tokens.next()
        else {
            continue;
        };
        if program != "cargo" {
            continue;
        }
        let Some(subcommand) = tokens.next()
        else {
            return Err(Box::new(std::io::Error::other(
                "cargo command in Dylint task has no subcommand",
            )));
        };
        if subcommand != "dylint" {
            continue;
        }
        let rest = tokens.collect::<Vec<_>>();
        let Some(separator) = rest.iter().position(|&token| token == "--")
        else {
            return Err(Box::new(std::io::Error::other(format!(
                "cargo dylint invocation omitted cargo-argument separator `--`: {command}"
            ))));
        };
        let dylint_args = rest
            .iter()
            .take(separator)
            .map(|&token| token.to_owned())
            .collect::<Vec<_>>();
        let cargo_args = rest
            .iter()
            .skip(separator.saturating_add(1_usize))
            .map(|&token| token.to_owned())
            .collect::<Vec<_>>();
        invocations.push(DylintInvocation {
            dylint_args,
            cargo_args,
        });
    }
    Ok(invocations)
}

/// Return the exact Dylint arguments for the ordinary upstream pass.
fn expected_upstream_dylint_args() -> Vec<String>
{
    let mut args = Vec::new();
    for library in EXPECTED_UPSTREAM_DYLINT_LIBS {
        args.push("--lib".to_owned());
        args.push((*library).to_owned());
    }
    args.push("--no-deps".to_owned());
    args
}

/// Parse every cargo invocation from a mise task run script.
fn parse_cargo_invocations<'semantic, Script>(script: Script) -> Vec<Vec<String>>
where
    Script: Into<ScriptText<'semantic>>,
{
    let script = script.into().0;
    logical_shell_commands(script)
        .into_iter()
        .filter_map(|command| {
            let tokens = command
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            tokens
                .first()
                .is_some_and(|program| program == "cargo")
                .then_some(tokens)
        })
        .collect()
}

/// Join shell continuation lines into logical commands.
fn logical_shell_commands<'semantic, Script>(script: Script) -> Vec<String>
where
    Script: Into<ScriptText<'semantic>>,
{
    let script = script.into().0;
    let mut commands = Vec::new();
    let mut current = String::new();
    for raw_line in script.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let continued = line.ends_with('\\');
        let fragment = if continued {
            line.trim_end_matches('\\').trim_end()
        }
        else {
            line
        };
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(fragment);
        if !continued {
            commands.push(core::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        commands.push(current);
    }
    commands
}

/// Parsed cargo-dylint command split at the cargo-argument separator.
#[derive(Debug, Eq, PartialEq)]
struct DylintInvocation
{
    /// Arguments consumed by cargo-dylint itself.
    dylint_args: Vec<String>,
    /// Cargo arguments passed after `--`.
    cargo_args: Vec<String>,
}

/// Extract the Rust mutants argv configured for a mise task without running it.
fn configured_mutants_task_args<'semantic, Mise, Task>(
    mise: Mise,
    task: Task,
) -> TestResult<Vec<OsString>>
where
    Mise: Into<MiseText<'semantic>>,
    Task: Into<TaskText<'semantic>>,
{
    let task = task.into().0;
    let mise = mise.into().0;
    let header = format!("[\"{task}\"]");
    let Some((_, task_section)) = mise.split_once(&header)
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "mise task `{task}` is missing"
        ))));
    };
    let section = task_section
        .split_once("\n[tasks.")
        .map_or(task_section, |(section, _)| section);
    let Some(command_line) = section
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("cargo run --quiet -p gandr-workflow-gates -- mutants "))
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "mise task `{task}` does not run a configured mutants command"
        ))));
    };

    let mut after_separator = false;
    let mut arguments = vec![OsString::from("gandr-workflow-gates")];
    for token in command_line.split_whitespace() {
        if token == "--" {
            after_separator = true;
            continue;
        }
        if after_separator {
            arguments.push(OsString::from(token.trim_matches('"')));
        }
    }
    if arguments.len() == 1_usize {
        return Err(Box::new(std::io::Error::other(format!(
            "mise task `{task}` omitted the mutants subcommand"
        ))));
    }
    Ok(arguments)
}

/// Assert that a command parse produced the exact usage detail.
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

/// Assert that an executed CLI outcome is clean.
fn assert_clean(outcome: cli::GateOutcome) -> TestResult
{
    match outcome {
        | cli::GateOutcome::Clean => Ok(()),
        | cli::GateOutcome::Findings(findings) => Err(Box::new(std::io::Error::other(format!(
            "expected clean outcome, got {} findings",
            findings.len()
        )))),
        | cli::GateOutcome::ExternalStatus(status) => {
            let _status_code = status.code();
            Err(Box::new(std::io::Error::other(
                "expected semantic clean outcome, got external status outcome",
            )))
        },
        | cli::GateOutcome::PageBalance(report) => {
            let _note_count = report.late_probes.len();
            Err(Box::new(std::io::Error::other(
                "expected semantic clean outcome, got page-balance outcome",
            )))
        },
    }
}

/// The binary entry and top-level inventory stay exact for local tooling
/// cutover.
#[test]
fn top_level_command_inventory_is_exact() -> TestResult
{
    let usage = cli::usage_text().into().0;
    let binary_entry: fn() -> std::process::ExitCode = cli::main;
    core::hint::black_box(binary_entry);
    // Search within the commands tail: the binary name itself
    // (`gandr-workflow-gates`) contains `workflow`, which would false-positive
    // the order assertion.
    let commands_tail = usage.get(usage.find("commands: ").unwrap_or(0) ..);
    let mut previous_position = None;
    for command in EXPECTED_COMMANDS {
        let Some(tail) = commands_tail
        else {
            return Err(Box::new(std::io::Error::other(
                "usage text omitted the commands listing",
            )));
        };
        let Some(position) = tail.find(command)
        else {
            return Err(Box::new(std::io::Error::other(format!(
                "usage text omitted command `{command}`"
            ))));
        };
        if let Some(previous) = previous_position {
            assert!(
                position > previous,
                "usage command order changed at `{command}`"
            );
        }
        previous_position = Some(position);
    }
    assert!(
        cli::usage_text()
            .into()
            .0
            .contains("fuzz-smoke [--target lower|parse|check|parity|gates]"),
        "usage text should advertise the closed fuzz-smoke target set"
    );
    Ok(())
}

/// Usage paths are UTF-8-safe: command names require UTF-8, path values do not.
#[test]
fn utf8_safe_usage_handling_is_stable() -> TestResult
{
    assert_usage(
        cli::parse_command(os_args(["gandr-workflow-gates"])),
        cli::usage_text().into().0,
    )?;
    assert_usage(
        cli::parse_command(os_args([
            "gandr-workflow-gates",
            "docs-manifest",
            "--manifest",
            "one.yml",
            "--manifest",
            "two.yml",
        ])),
        "duplicate --manifest",
    )?;

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;

        assert_usage(
            cli::parse_command([
                OsString::from("gandr-workflow-gates"),
                OsString::from_vec(vec![0xFF]),
            ]),
            "command must be valid UTF-8",
        )?;

        let manifest = OsString::from_vec(vec![b'd', b'o', b'c', 0xFF, b'.', b'y', b'm', b'l']);
        let parsed = cli::parse_command([
            OsString::from("gandr-workflow-gates"),
            OsString::from("docs-manifest"),
            OsString::from("--manifest"),
            manifest.clone(),
        ])?;
        match parsed {
            | cli::Command::DocsManifest { manifest_path } => {
                assert_eq!(
                    manifest_path.as_os_str().as_encoded_bytes(),
                    manifest.as_os_str().as_encoded_bytes()
                );
            },
            | _ => {
                return Err(Box::new(std::io::Error::other(
                    "docs-manifest parsed as a different command",
                )));
            },
        }
    }

    Ok(())
}

/// Documentation commands dispatch against pure fixtures without external
/// tools.
#[test]
fn pure_fixture_documentation_commands_dispatch_cleanly() -> TestResult
{
    let fixture = DocsFixture::create("clean-docs")?;
    gandr_workflow_gates::support::HOST_FILESYSTEM
        .write(fixture.adr.join("0007-record.md"), "# ADR-7\n")?;
    let source = "# Root\n\nSee ADR-7.\n## 1 Section\nSee §1.\n";
    let root_hash = fixture.write_doc("root.md", source)?;
    fixture.write_manifest(&format!(
        "  - path: root.md\n    b3: {root_hash}\n    edges:\n      - rel: self\n        to: root.md#root\n"
    ))?;
    let manifest = fixture.manifest_string();

    let manifest_args = os_args([
        "gandr-workflow-gates",
        "docs-manifest",
        "--manifest",
        manifest.as_str(),
    ]);
    let outcome = cli::run_with_args(manifest_args)?;
    assert_clean(outcome)?;
    let reference_args = os_args([
        "gandr-workflow-gates",
        "docs-reference",
        "--manifest",
        manifest.as_str(),
    ]);
    let outcome = cli::run_with_args(reference_args)?;
    assert_clean(outcome)?;
    Ok(())
}

/// Process-level exits keep findings on stderr and preserve status classes.
#[test]
fn process_stdout_stderr_and_status_are_exact() -> TestResult
{
    let Some(binary) = option_env!("CARGO_BIN_EXE_gandr-workflow-gates")
    else {
        return Err(Box::new(std::io::Error::other(
            "Cargo did not expose the gandr-workflow-gates binary path",
        )));
    };
    let binary = Path::new(binary);
    let output = |command: &mut std::process::Command| {
        command.output().map_err(|source| GateError::Io {
            path: binary.to_path_buf(),
            source,
        })
    };

    let clean_fixture = DocsFixture::create("process-clean")?;
    let clean_source = "# Root\n\nSee ADR-7.\n## 1 Section\nSee §1.\n";
    gandr_workflow_gates::support::HOST_FILESYSTEM
        .write(clean_fixture.adr.join("0007-record.md"), "# ADR-7\n")?;
    let clean_hash = clean_fixture.write_doc("root.md", clean_source)?;
    clean_fixture.write_manifest(&format!(
        "  - path: root.md\n    b3: {clean_hash}\n    edges:\n      - rel: self\n        to: root.md#root\n"
    ))?;
    let clean_output = output(
        std::process::Command::new(binary)
            .arg("docs-manifest")
            .arg("--manifest")
            .arg(clean_fixture.manifest_string()),
    )?;
    assert_eq!(Some(0_i32), clean_output.status.code());
    assert!(clean_output.stdout.is_empty());
    assert!(clean_output.stderr.is_empty());

    let finding_fixture = DocsFixture::create("process-finding")?;
    let drift_source = "# Drift\n";
    let _drift_hash = finding_fixture.write_doc("root.md", drift_source)?;
    let wrong_hash = blake3_hex(b"different source");
    finding_fixture.write_manifest(&format!(
        "  - path: root.md\n    b3: {wrong_hash}\n    edges: []\n"
    ))?;
    let finding_output = output(
        std::process::Command::new(binary)
            .arg("docs-manifest")
            .arg("--manifest")
            .arg(finding_fixture.manifest_string()),
    )?;
    assert_eq!(Some(1_i32), finding_output.status.code());
    assert!(finding_output.stdout.is_empty());
    let finding_stderr = String::from_utf8_lossy(&finding_output.stderr);
    assert!(finding_stderr.contains("kind="));
    assert!(finding_stderr.contains("path=root.md"));

    let usage_output = output(&mut std::process::Command::new(binary))?;
    assert_eq!(Some(2_i32), usage_output.status.code());
    assert!(usage_output.stdout.is_empty());
    let usage_stderr = String::from_utf8_lossy(&usage_output.stderr);
    assert!(
        usage_stderr
            .contains(semantic_value::<cli::UsageTextText<'static>, _>(cli::usage_text()).as_ref())
    );

    let unknown_output = output(std::process::Command::new(binary).arg("unknown"))?;
    assert_eq!(Some(2_i32), unknown_output.status.code());
    assert!(unknown_output.stdout.is_empty());
    let unknown_stderr = String::from_utf8_lossy(&unknown_output.stderr);
    assert!(unknown_stderr.contains("unknown command `unknown`"));

    let missing_value_output = output(
        std::process::Command::new(binary)
            .arg("docs-manifest")
            .arg("--manifest")
            .arg("--other"),
    )?;
    assert_eq!(Some(2_i32), missing_value_output.status.code());
    assert!(missing_value_output.stdout.is_empty());
    let missing_value_stderr = String::from_utf8_lossy(&missing_value_output.stderr);
    assert!(missing_value_stderr.contains("missing value for --manifest"));
    Ok(())
}

/// Workflow parsing selects a typed plan without executing any `mise` task.
#[test]
fn workflow_plan_selection_is_typed_without_execution() -> TestResult
{
    let push_workflow = cli::parse_command(os_args([
        "gandr-workflow-gates",
        "workflow",
        "push",
        "--cwd",
        "repo",
    ]))?;
    match push_workflow {
        | cli::Command::Workflow { tier, cwd } => {
            assert_eq!("push", tier.as_str().as_ref());
            assert_eq!(cwd, Some(PathBuf::from("repo")));
            let plan = tier.plan();
            assert_eq!("push", plan.tier().as_str().as_ref());
            assert!(
                !plan.tasks().is_empty(),
                "workflow push should select a nonempty plan"
            );
        },
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "workflow parsed as a different command",
            )));
        },
    }

    let merge_workflow =
        cli::parse_command(os_args(["gandr-workflow-gates", "workflow", "merge"]))?;
    match merge_workflow {
        | cli::Command::Workflow { tier, cwd } => {
            assert_eq!("merge", tier.as_str().as_ref());
            assert_eq!(None, cwd);
        },
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "workflow merge parsed as a different command",
            )));
        },
    }
    Ok(())
}

/// Configured mutation modes parse without internal path flags or execution.
#[test]
fn configured_mutants_modes_parse_without_internal_paths() -> TestResult
{
    let current_dir = gandr_workflow_gates::support::HOST_FILESYSTEM.current_dir()?;

    let snapshot = cli::parse_command(os_args(["gandr-workflow-gates", "mutants", "snapshot"]))?;
    match snapshot {
        | cli::Command::Mutants {
            command: mutants::MutantsCommand::Snapshot,
            options,
        } => assert_default_mutants_options(&options, &current_dir, "snapshot")?,
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "mutants snapshot parsed as a different command",
            )));
        },
    }

    let push = cli::parse_command(os_args(["gandr-workflow-gates", "mutants", "push"]))?;
    match push {
        | cli::Command::Mutants {
            command:
                mutants::MutantsCommand::Push {
                    range: mutants::range::PushRangePlan::Last { to },
                },
            options,
        } => {
            assert_eq!("HEAD", to);
            assert_default_mutants_options(&options, &current_dir, "push")?;
        },
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "mutants push parsed as a different command",
            )));
        },
    }

    let merge = cli::parse_command(os_args(["gandr-workflow-gates", "mutants", "merge"]))?;
    match merge {
        | cli::Command::Mutants {
            command: mutants::MutantsCommand::Merge,
            options,
        } => assert_default_mutants_options(&options, &current_dir, "merge")?,
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "mutants merge parsed as a different command",
            )));
        },
    }

    let scheduled = cli::parse_command(os_args([
        "gandr-workflow-gates",
        "mutants",
        "scheduled",
        "--from",
        "main",
        "--to",
        "HEAD",
    ]))?;
    match scheduled {
        | cli::Command::Mutants {
            command: mutants::MutantsCommand::Scheduled { from_ref, to_ref },
            options,
        } => {
            assert_eq!("main", from_ref);
            assert_eq!("HEAD", to_ref);
            assert_default_mutants_options(&options, &current_dir, "scheduled")?;
        },
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "mutants scheduled parsed as a different command",
            )));
        },
    }

    let clean = cli::parse_command(os_args(["gandr-workflow-gates", "mutants", "clean"]))?;
    match clean {
        | cli::Command::Mutants {
            command: mutants::MutantsCommand::Clean,
            options,
        } => {
            assert!(options.workspace_root.as_os_str().is_empty());
            assert!(options.cache_image.as_os_str().is_empty());
            assert!(options.source_archive.as_os_str().is_empty());
            assert!(options.diff_file.as_os_str().is_empty());
            assert!(options.working_report.as_os_str().is_empty());
        },
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "mutants clean parsed as a different command",
            )));
        },
    }

    let sweep = cli::parse_command(os_args(["gandr-workflow-gates", "mutants", "sweep"]))?;
    match sweep {
        | cli::Command::Mutants {
            command: mutants::MutantsCommand::Sweep,
            options,
        } => assert_default_mutants_options(&options, &current_dir, "sweep")?,
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "mutants sweep parsed as a different command",
            )));
        },
    }

    let explicit = cli::parse_command(os_args([
        "gandr-workflow-gates",
        "mutants",
        "merge",
        "--workspace-root",
        "repo",
        "--cache-image",
        "cache.raw",
        "--source-archive",
        "source.tar",
        "--diff-file",
        "changes.diff",
        "--working-report",
        "report",
    ]))?;
    match explicit {
        | cli::Command::Mutants { options, .. } => {
            assert_eq!(options.workspace_root, PathBuf::from("repo"));
            assert_eq!(options.cache_image, PathBuf::from("cache.raw"));
            assert_eq!(options.source_archive, PathBuf::from("source.tar"));
            assert_eq!(options.diff_file, PathBuf::from("changes.diff"));
            assert_eq!(options.working_report, PathBuf::from("report"));
        },
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "mutants explicit overrides parsed as a different command",
            )));
        },
    }

    Ok(())
}

/// Package mutation parsing requires one exact UTF-8 package argument.
#[test]
fn package_mutants_cli_requires_exact_single_package() -> TestResult
{
    let parsed = cli::parse_command(os_args([
        "gandr-workflow-gates",
        "mutants",
        "package",
        "gandr-core-checker-tools",
    ]))?;
    match parsed {
        | cli::Command::Mutants {
            command: mutants::MutantsCommand::Package { package },
            options,
        } => {
            assert!(!options.source_archive.as_os_str().is_empty());
            assert_eq!("gandr-core-checker-tools", package);
        },
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "package mutation parsed as a different command",
            )));
        },
    }
    for args in [
        vec!["gandr-workflow-gates", "mutants", "package"],
        vec!["gandr-workflow-gates", "mutants", "package", "one", "two"],
        vec!["gandr-workflow-gates", "mutants", "package", " padded"],
    ] {
        assert!(
            cli::parse_command(os_args(args)).is_err(),
            "invalid package argv must be rejected"
        );
    }
    Ok(())
}

#[test]
fn configured_mise_mutants_tasks_parse_without_internal_path_flags() -> TestResult
{
    let workspace = workspace_root()?;
    let workspace_mise_tasks = workspace_mise_tasks(&workspace);
    let mise_tasks_mutants_path = workspace_mise_tasks.join("mise-tasks-mutants.toml");
    let mise_tasks_mutants_toml =
        gandr_workflow_gates::support::HOST_FILESYSTEM.read_to_string(mise_tasks_mutants_path)?;
    let current_dir = gandr_workflow_gates::support::HOST_FILESYSTEM.current_dir()?;
    let cases = [
        ("mutants:snapshot", "snapshot"),
        ("mutants:push", "push"),
        ("mutants:merge", "merge"),
        ("mutants:scheduled", "scheduled"),
        ("mutants:clean", "clean"),
        ("mutants:sweep", "sweep"),
        ("mutants:changed-vs-remote", "push"),
        ("mutants:changed-vs-main", "merge"),
    ];

    for (task, mode) in cases {
        let args = configured_mutants_task_args(&mise_tasks_mutants_toml, task)?;
        let parsed = cli::parse_command(args)?;
        let cli::Command::Mutants { command, options } = parsed
        else {
            return Err(Box::new(std::io::Error::other(format!(
                "mise task `{task}` parsed as a different command"
            ))));
        };
        match (mode, command) {
            | ("snapshot", mutants::MutantsCommand::Snapshot)
            | ("merge", mutants::MutantsCommand::Merge)
            | ("sweep", mutants::MutantsCommand::Sweep) => {
                assert_default_mutants_options(&options, &current_dir, mode)?;
            },
            | (
                "push",
                mutants::MutantsCommand::Push {
                    range: mutants::range::PushRangePlan::Last { to },
                },
            ) => {
                assert_eq!("HEAD", to);
                assert_default_mutants_options(&options, &current_dir, mode)?;
            },
            | ("scheduled", mutants::MutantsCommand::Scheduled { from_ref, to_ref }) => {
                assert!(from_ref.contains("usage_from"));
                assert!(to_ref.contains("usage_to"));
                assert_default_mutants_options(&options, &current_dir, mode)?;
            },
            | ("clean", mutants::MutantsCommand::Clean) => {
                assert!(options.workspace_root.as_os_str().is_empty());
                assert!(options.cache_image.as_os_str().is_empty());
                assert!(options.source_archive.as_os_str().is_empty());
                assert!(options.diff_file.as_os_str().is_empty());
                assert!(options.working_report.as_os_str().is_empty());
            },
            | _ => {
                return Err(Box::new(std::io::Error::other(format!(
                    "mise task `{task}` parsed as the wrong mutants mode"
                ))));
            },
        }
    }

    Ok(())
}

/// Rust lint workflow contract: Clippy and Dylint retain their full enabled
/// workspace scopes while the immutable upstream Dylint inventory stays locked.
#[test]
fn lint_inventory_and_workspace_scopes_are_locked() -> TestResult
{
    let workspace = workspace_root()?;
    let manifest = parse_toml_file(&workspace.join("Cargo.toml"))?;
    let dylint_libraries = dylint_metadata_libraries(&manifest)?;
    let upstream_dylint_library = dylint_libraries[0_usize];
    let local_dylint_library = dylint_libraries[1_usize];
    let upstream_git = toml_table_string(upstream_dylint_library, "git")?;
    assert_eq!("https://github.com/trailofbits/dylint", upstream_git.0);
    let upstream_rev = toml_table_string(upstream_dylint_library, "rev")?;
    assert_eq!(EXPECTED_DYLINT_REV, upstream_rev.0);
    let local_path = toml_table_string(local_dylint_library, "path")?;
    assert_eq!(EXPECTED_LOCAL_DYLINT_PATH, local_path.0);

    let plugin_paths = toml_table_string_array(upstream_dylint_library, "pattern")?;
    let forbidden_paths = plugin_paths
        .iter()
        .filter(|path| {
            path.starts_with("examples/experimental/") || path.starts_with("examples/testing/")
        })
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert!(
        forbidden_paths.is_empty(),
        "Dylint metadata must not load experimental/testing paths: {forbidden_paths:?}"
    );
    assert_string_sequence(
        &plugin_paths,
        EXPECTED_DYLINT_PLUGIN_PATHS,
        "Dylint metadata plugin path inventory changed",
    );
    let represented_lints = represented_dylint_lint_paths(&plugin_paths);
    assert_eq!(
        EXPECTED_DYLINT_REPRESENTED_LINT_PATHS.len(),
        27_usize,
        "expected Dylint inventory constant must cover 27 upstream lints"
    );
    assert_string_sequence(
        &represented_lints,
        EXPECTED_DYLINT_REPRESENTED_LINT_PATHS,
        "Dylint represented lint inventory changed",
    );

    let workspace_mise_tasks = workspace_mise_tasks(&workspace);
    let mise_tasks_cargo = parse_toml_file(&workspace_mise_tasks.join("mise-tasks-cargo.toml"))?;
    let cargo_clippy = toml_table_at(&mise_tasks_cargo, ["cargo:clippy"])?;
    let cargo_clippy_script = toml_table_string(cargo_clippy, "run")?;
    assert!(
        cargo_clippy_script.0.contains("set -- --workspace"),
        "cargo:clippy must default its package-scope hole to the whole workspace"
    );
    assert!(
        cargo_clippy_script
            .0
            .contains("cargo metadata --no-deps --format-version 1"),
        "package-scoped cargo:clippy must inspect declared package features"
    );
    assert!(
        cargo_clippy_script.0.contains(".features | has(\"full\")"),
        "package-scoped cargo:clippy must derive full-feature support from metadata"
    );
    assert!(
        cargo_clippy_script
            .0
            .contains("features=\"$features $package/full\""),
        "package-scoped cargo:clippy must qualify full per package"
    );
    assert!(
        cargo_clippy_script
            .0
            .contains("set -- \"$@\" --features=\"$features\""),
        "mixed package selections must pass all declared features together"
    );
    assert!(
        cargo_clippy_script
            .0
            .contains("set -- --workspace --features=full"),
        "workspace cargo:clippy must retain its original full-feature wall"
    );
    let clippy_commands = parse_cargo_invocations(cargo_clippy_script);
    let [ref workspace_pass] = *clippy_commands.as_slice()
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "expected one cargo:clippy invocation, found {}",
            clippy_commands.len()
        ))));
    };
    assert_string_sequence(
        workspace_pass,
        EXPECTED_CLIPPY_WORKSPACE_COMMAND,
        "cargo:clippy enabled-workspace scope changed",
    );

    let cargo_dylint_local = toml_table_at(&mise_tasks_cargo, ["cargo:dylint:local"])?;
    let local_depends = toml_table_string_array(cargo_dylint_local, "depends")?;
    assert_string_sequence(
        &local_depends,
        ["toolchain:materialize"],
        "cargo:dylint:local must materialize the pinned driver toolchain",
    );
    let Some(local_dylint_env) = cargo_dylint_local
        .get("env")
        .and_then(toml::Value::as_table)
    else {
        return Err(Box::new(std::io::Error::other(
            "cargo:dylint:local has no environment table",
        )));
    };
    let target_dir = toml_table_string(local_dylint_env, "CARGO_TARGET_DIR")?;
    assert_eq!(
        "{{ config_root }}/target/dylint-local", target_dir.0,
        "strict project-local artifacts must use their own target directory"
    );
    let rustflags = toml_table_string(local_dylint_env, "DYLINT_RUSTFLAGS")?;
    assert_eq!(
        "-D warnings -A clippy::std_instead_of_core", rustflags.0,
        "strict project-local Dylint must deny every warning (the std_instead_of_core allowance is the documented nightly-2026-05-28 rollback residual)"
    );
    let cargo_dylint_local_script = toml_table_string(cargo_dylint_local, "run")?;
    assert!(
        cargo_dylint_local_script.0.contains("set -- --workspace"),
        "cargo:dylint:local must default its package-scope hole to the whole workspace"
    );
    let local_invocations = parse_dylint_invocations(cargo_dylint_local_script)?;
    let [ref custom_pass] = *local_invocations.as_slice()
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "expected one cargo:dylint:local invocation, found {}",
            local_invocations.len()
        ))));
    };

    let cargo_dylint = toml_table_at(&mise_tasks_cargo, ["cargo:dylint"])?;
    assert_table_keys(
        cargo_dylint,
        EXPECTED_DYLINT_FACADE_KEYS,
        "cargo:dylint must stay a composition of the two strict lanes",
    );
    let Some(dylint_steps) = cargo_dylint.get("run").and_then(toml::Value::as_array)
    else {
        return Err(Box::new(std::io::Error::other(
            "cargo:dylint run plan is not an array",
        )));
    };
    let mut dylint_tasks = Vec::with_capacity(dylint_steps.len());
    for step in dylint_steps {
        let Some(step) = step.as_table()
        else {
            return Err(Box::new(std::io::Error::other(
                "cargo:dylint run step is not an inline table",
            )));
        };
        assert_table_keys(
            step,
            ["task"],
            "a cargo:dylint step gained work outside its named lane",
        );
        let task_name = toml_table_string(step, "task")?;
        dylint_tasks.push(task_name.0.to_owned());
    }
    assert_string_sequence(
        &dylint_tasks,
        EXPECTED_DYLINT_FACADE_TASKS.iter().copied(),
        "cargo:dylint lane composition changed",
    );

    let cargo_dylint_upstream = toml_table_at(&mise_tasks_cargo, ["cargo:dylint:upstream"])?;
    let upstream_depends = toml_table_string_array(cargo_dylint_upstream, "depends")?;
    assert_string_sequence(
        &upstream_depends,
        ["toolchain:materialize"],
        "cargo:dylint:upstream must materialize the pinned driver toolchain",
    );
    let cargo_dylint_upstream_script = toml_table_string(cargo_dylint_upstream, "run")?;
    let cargo_commands = parse_cargo_invocations(cargo_dylint_upstream_script);
    let Some(ui_test_command) = cargo_commands.first()
    else {
        return Err(Box::new(std::io::Error::other(
            "cargo:dylint:upstream task contains no cargo commands",
        )));
    };
    assert_string_sequence(
        ui_test_command,
        EXPECTED_DYLINT_UI_TEST_COMMAND,
        "cargo:dylint:upstream must run the project-local Dylint UI suite first",
    );

    let invocations = parse_dylint_invocations(cargo_dylint_upstream_script)?;
    let invocation_count = invocations.len();
    let mut invocations = invocations.iter();
    let (Some(upstream_pass), Some(crate_wide_pass), Some(register_lints_pass), None) = (
        invocations.next(),
        invocations.next(),
        invocations.next(),
        invocations.next(),
    )
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "expected three cargo:dylint:upstream invocations, found {invocation_count}"
        ))));
    };

    assert_string_sequence(
        &custom_pass.dylint_args,
        ["--lib", EXPECTED_LOCAL_DYLINT_LIB, "--no-deps"],
        "project-local Dylint pass argument inventory changed",
    );
    assert_string_sequence(
        &custom_pass.cargo_args,
        ["\"$@\"", "--all-targets", "--features=full"],
        "project-local Dylint pass package scope changed",
    );

    let expected_upstream_dylint_args = expected_upstream_dylint_args();
    assert_eq!(
        upstream_pass.dylint_args.as_slice(),
        expected_upstream_dylint_args.as_slice(),
        "ordinary upstream Dylint pass argument inventory changed",
    );
    assert_string_sequence(
        &upstream_pass.cargo_args,
        ["--workspace", "--all-targets", "--features=full"],
        "ordinary upstream Dylint pass package scope changed",
    );

    assert_string_sequence(
        &crate_wide_pass.dylint_args,
        ["--lib", "crate_wide_allow", "--no-deps"],
        "crate-wide-allow Dylint pass must stay isolated",
    );
    assert_string_sequence(
        &crate_wide_pass.cargo_args,
        ["--workspace", "--all-targets", "--features=full"],
        "crate-wide-allow Dylint pass must cover every target kind",
    );

    assert_string_sequence(
        &register_lints_pass.dylint_args,
        ["--lib", "register_lints_warn", "--no-deps"],
        "register-lints-warn Dylint pass must stay isolated",
    );
    assert_string_sequence(
        &register_lints_pass.cargo_args,
        ["--workspace", "--all-targets", "--features=full"],
        "register-lints-warn Dylint pass package scope changed",
    );

    Ok(())
}

/// Return the ordered task names the `gate:merge` task body runs.
///
/// The `run` array is not the only way a mise task body can add work: a
/// `depends` key on the task, or an `args` key on a step, would make the wall
/// run something the crate's plan cannot replay. Both are rejected here rather
/// than ignored, so the crate-versus-task-body comparison stays a comparison of
/// the whole wall and not of its step names alone.
fn gate_merge_task_names() -> TestResult<Vec<String>>
{
    let workspace = workspace_root()?;
    let workspace_mise_tasks = workspace_mise_tasks(&workspace);
    let mise_tasks_gates = parse_toml_file(&workspace_mise_tasks.join("mise-tasks-gates.toml"))?;
    let gate_merge = toml_table_at(&mise_tasks_gates, ["gate:merge"])?;
    assert_table_keys(
        gate_merge,
        EXPECTED_GATE_MERGE_KEYS,
        "gate:merge gained a key the workflow-plan comparison does not replay",
    );
    let Some(merge_steps) = gate_merge.get("run").and_then(toml::Value::as_array)
    else {
        return Err(Box::new(std::io::Error::other(
            "gate:merge run plan is not an array",
        )));
    };
    let mut merge_tasks = Vec::with_capacity(merge_steps.len());
    for step in merge_steps {
        let Some(step) = step.as_table()
        else {
            return Err(Box::new(std::io::Error::other(
                "gate:merge run step is not an inline table",
            )));
        };
        assert_table_keys(
            step,
            EXPECTED_GATE_MERGE_STEP_KEYS,
            "a gate:merge step gained a key the workflow-plan comparison does not replay",
        );
        let task_name = toml_table_string(step, "task")?;
        merge_tasks.push(task_name.0.to_owned());
    }
    Ok(merge_tasks)
}

/// Assert that a TOML table carries exactly the expected keys, in any order.
fn assert_table_keys<'semantic, Expected, ExpectedItem, Context>(
    table: &toml::Table,
    expected: Expected,
    context: Context,
) where
    Expected: IntoIterator<Item = ExpectedItem>,
    ExpectedItem: Into<ExpectedText<'semantic>>,
    Context: Into<ContextText<'semantic>>,
{
    let mut keys = table.keys().cloned().collect::<Vec<String>>();
    keys.sort();
    let mut expected = expected
        .into_iter()
        .map(|item| item.into().0)
        .collect::<Vec<&str>>();
    expected.sort_unstable();
    assert_string_sequence(&keys, expected, context);
}

/// Return the ordered task names one workflow tier's static plan runs.
fn workflow_plan_task_names(tier: gandr_workflow_gates::workflow::Tier) -> Vec<String>
{
    let mut names = Vec::new();
    for task in tier.plan().tasks() {
        names.push(String::from(task.name().as_ref()));
    }
    names
}

/// Return every task name the workspace mise task surface defines.
fn defined_mise_task_names() -> TestResult<Vec<String>>
{
    let workspace = workspace_root()?;
    let workspace_mise_tasks = workspace_mise_tasks(&workspace);
    let mut entries =
        gandr_workflow_gates::support::HOST_FILESYSTEM.read_dir_paths(&workspace_mise_tasks)?;
    entries.sort();
    let mut names = Vec::new();
    for entry in &entries {
        if entry
            .extension()
            .is_none_or(|extension| extension != "toml")
        {
            return Err(Box::new(std::io::Error::other(format!(
                "mise task entry `{}` is not a TOML task file; the workflow-plan task inventory \
                 gate cannot see script tasks and needs extending",
                entry.display()
            ))));
        }
        let document = parse_toml_file(entry)?;
        let Some(document) = document.as_table()
        else {
            return Err(Box::new(std::io::Error::other(format!(
                "mise task file `{}` is not a table",
                entry.display()
            ))));
        };
        for (name, definition) in document {
            if definition.is_table() {
                names.push(name.clone());
            }
        }
    }
    Ok(names)
}

/// The merge wall runs the deterministic native gates in their policy order.
#[test]
fn merge_gate_task_order_is_locked() -> TestResult
{
    let merge_tasks = gate_merge_task_names()?;
    assert_string_sequence(
        &merge_tasks,
        EXPECTED_MERGE_GATE_TASKS.iter().copied(),
        "gate:merge task order changed",
    );
    Ok(())
}

/// The gates crate's merge plan is the merge wall itself, not a second
/// definition of it that can drift away unobserved.
///
/// `gate:merge` is the single source of truth (`.config/wt.toml` runs it as
/// the whole `pre-merge` wall); `workflow merge` in this crate replays the
/// same boundaries with caching, so any divergence means one of the two runs
/// a wall the project does not have.
#[test]
fn merge_plan_matches_gate_merge_task() -> TestResult
{
    let merge_tasks = gate_merge_task_names()?;
    let plan_tasks = workflow_plan_task_names(gandr_workflow_gates::workflow::Tier::Merge);
    assert_eq!(
        plan_tasks, merge_tasks,
        "the gandr-workflow-gates merge plan diverged from the gate:merge task body",
    );
    Ok(())
}

/// Every task either workflow plan runs is a real mise task, apart from the
/// explicitly parked entries whose tasks the reboot has not restored.
#[test]
fn workflow_plan_tasks_exist_or_are_parked() -> TestResult
{
    let defined = defined_mise_task_names()?;
    let mut parked = Vec::new();
    for tier in [
        gandr_workflow_gates::workflow::Tier::Merge,
        gandr_workflow_gates::workflow::Tier::Push,
    ] {
        for name in workflow_plan_task_names(tier) {
            if defined.iter().any(|defined_name| defined_name == &name) {
                continue;
            }
            assert!(
                EXPECTED_PARKED_WORKFLOW_TASKS.contains(&name.as_str()),
                "the {tier:?} workflow plan runs `{name}`, which no mise task defines"
            );
            if !parked.iter().any(|recorded| recorded == &name) {
                parked.push(name);
            }
        }
    }
    parked.sort();
    assert_string_sequence(
        &parked,
        EXPECTED_PARKED_WORKFLOW_TASKS.iter().copied(),
        "the parked workflow-task inventory changed",
    );
    Ok(())
}

/// The merge wall's formatter dependency is a discovered CI-mode task.
#[test]
fn treefmt_check_task_policy_is_locked() -> TestResult
{
    let workspace = workspace_root()?;
    let workspace_mise_tasks = workspace_mise_tasks(&workspace);
    let mise_tasks_maintenance =
        parse_toml_file(&workspace_mise_tasks.join("mise-tasks-maintenance.toml"))?;
    let treefmt_check = toml_table_at(&mise_tasks_maintenance, ["treefmt:check"])?;
    let treefmt_check_script = toml_table_string(treefmt_check, "run")?;
    assert_eq!(
        r#"RUSTUP_TOOLCHAIN="$RUSTUP_TOOLCHAIN_NIGHTLY" treefmt --ci"#,
        treefmt_check_script.0.trim()
    );
    Ok(())
}

/// The merge-wall rustdoc gate documents the whole workspace on the pinned
/// nightly with every rustdoc lint denied.
#[test]
fn doc_check_task_policy_is_locked() -> TestResult
{
    let workspace = workspace_root()?;
    let workspace_mise_tasks = workspace_mise_tasks(&workspace);
    let mise_tasks_cargo = parse_toml_file(&workspace_mise_tasks.join("mise-tasks-cargo.toml"))?;
    let cargo_doc_check = toml_table_at(&mise_tasks_cargo, ["cargo:doc-check"])?;
    let doc_check_script = toml_table_string(cargo_doc_check, "run")?;
    assert!(
        doc_check_script
            .0
            .contains("RUSTUP_TOOLCHAIN=\"$RUSTUP_TOOLCHAIN_NIGHTLY\""),
        "cargo:doc-check must run rustdoc on the pinned nightly toolchain"
    );
    assert!(
        doc_check_script.0.contains("RUSTDOCFLAGS=\"-D warnings\""),
        "cargo:doc-check must deny every rustdoc warning"
    );
    let doc_commands = parse_cargo_invocations(doc_check_script);
    let [ref doc_pass] = *doc_commands.as_slice()
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "expected one cargo:doc-check invocation, found {}",
            doc_commands.len()
        ))));
    };
    assert_string_sequence(
        doc_pass,
        EXPECTED_DOC_CHECK_COMMAND,
        "cargo:doc-check workspace documentation scope changed",
    );
    Ok(())
}

/// Gate fuzz configuration closes over a nonempty corpus and feature-bearing
/// argv.
#[test]
fn gates_fuzz_configuration_is_closed() -> TestResult
{
    let workspace = workspace_root()?;
    let workspace_mise_tasks = workspace_mise_tasks(&workspace);
    let mise_tasks_fuzz_path = workspace_mise_tasks.join("mise-tasks-fuzz.toml");
    let mise_tasks_fuzz_toml =
        gandr_workflow_gates::support::HOST_FILESYSTEM.read_to_string(mise_tasks_fuzz_path)?;
    let gates_build =
        "cargo afl build --manifest-path fuzz/Cargo.toml --features gates --bin gates";
    assert_eq!(
        2_usize,
        mise_tasks_fuzz_toml.match_indices(gates_build).count()
    );

    let corpus = workspace.join(cli::fuzz_corpus_dir(cli::FuzzSmokeTarget::Gates));
    let mut nonempty_seed_count = 0_usize;
    for path in gandr_workflow_gates::support::HOST_FILESYSTEM.read_dir_paths(&corpus)? {
        let contents = gandr_workflow_gates::support::HOST_FILESYSTEM.read(&path)?;
        if path.is_file() && !contents.as_ref().is_empty() {
            nonempty_seed_count = nonempty_seed_count.saturating_add(1);
        }
    }
    assert!(
        nonempty_seed_count > 0_usize,
        "gates fuzz corpus must contain a nonempty seed"
    );
    Ok(())
}
/// Fuzz smoke parsing and argv planning remain closed and deterministic.
#[test]
fn fuzz_smoke_plan_inventory_is_exact() -> TestResult
{
    let all_fuzz = cli::parse_command(os_args(["gandr-workflow-gates", "fuzz-smoke"]))?;
    match all_fuzz {
        | cli::Command::FuzzSmoke { plan } => {
            assert_eq!(
                plan.targets()
                    .iter()
                    .map(|target| target.as_str().0)
                    .collect::<Vec<_>>(),
                vec!["lower", "parse", "check", "parity", "gates"]
            );
        },
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "fuzz-smoke parsed as a different command",
            )));
        },
    }

    let gates_fuzz = cli::parse_command(os_args([
        "gandr-workflow-gates",
        "fuzz-smoke",
        "--target",
        "gates",
    ]))?;
    match gates_fuzz {
        | cli::Command::FuzzSmoke { plan } => {
            assert_eq!(
                plan.targets()
                    .iter()
                    .map(|target| target.as_str().0)
                    .collect::<Vec<_>>(),
                vec!["gates"]
            );
        },
        | _ => {
            return Err(Box::new(std::io::Error::other(
                "fuzz-smoke --target parsed as a different command",
            )));
        },
    }

    assert_eq!(
        cli::fuzz_build_args(cli::FuzzSmokeTarget::Lower),
        os_strings([
            "afl",
            "build",
            "--manifest-path",
            "fuzz/Cargo.toml",
            "--bin",
            "lower",
        ])
    );
    assert_eq!(
        cli::fuzz_build_args(cli::FuzzSmokeTarget::Parity),
        os_strings([
            "afl",
            "build",
            "--manifest-path",
            "fuzz/Cargo.toml",
            "--bin",
            "parity",
            "--features",
            "parity",
        ])
    );
    assert_eq!(
        cli::fuzz_build_args(cli::FuzzSmokeTarget::Gates),
        os_strings([
            "afl",
            "build",
            "--manifest-path",
            "fuzz/Cargo.toml",
            "--bin",
            "gates",
            "--features",
            "gates",
        ])
    );
    let build_plan = cli::fuzz_build_command_plan(cli::FuzzSmokeTarget::Lower);
    assert_eq!(build_plan.program(), Path::new("cargo"));
    assert_eq!(
        build_plan.args(),
        cli::fuzz_build_args(cli::FuzzSmokeTarget::Lower).as_slice()
    );
    assert_eq!(cli::ExternalStream::Inherit, build_plan.stdin());
    assert_eq!(cli::ExternalStream::Inherit, build_plan.stdout());
    assert_eq!(cli::ExternalStream::Inherit, build_plan.stderr());

    let replay_plan = cli::fuzz_replay_command_plan(cli::FuzzSmokeTarget::Gates);
    assert_eq!(replay_plan.program(), Path::new("fuzz/target/debug/gates"));
    assert!(
        replay_plan.args().is_empty(),
        "seed replay should not append arbitrary argv"
    );
    assert_eq!(cli::ExternalStream::Piped, replay_plan.stdin());
    assert_eq!(cli::ExternalStream::Inherit, replay_plan.stdout());
    assert_eq!(cli::ExternalStream::Inherit, replay_plan.stderr());

    assert_eq!(
        cli::fuzz_binary_path(cli::FuzzSmokeTarget::Gates),
        PathBuf::from("fuzz/target/debug/gates")
    );
    assert_eq!(
        cli::fuzz_corpus_dir(cli::FuzzSmokeTarget::Gates),
        PathBuf::from("fuzz/corpus/gates")
    );
    assert_usage(
        cli::parse_command(os_args([
            "gandr-workflow-gates",
            "fuzz-smoke",
            "--target",
            "arbitrary",
        ])),
        "unsupported fuzz-smoke target `arbitrary`",
    )?;
    Ok(())
}

/// Assert that default mutants options point at cwd, expanded cache, and temp
/// paths.
fn assert_default_mutants_options<'semantic, Mode>(
    options: &mutants::MutantsOptions,
    current_dir: &Path,
    mode: Mode,
) -> TestResult
where
    Mode: Into<ModeText<'semantic>>,
{
    let mode = mode.into().0;
    assert_eq!(options.workspace_root, current_dir);
    assert!(
        options
            .cache_image
            .ends_with(".microsandbox/gandr-mutants-cache.btrfs")
    );
    assert_mutants_temp_path(&options.source_archive, mode, "source.tar")?;
    assert_mutants_temp_path(&options.diff_file, mode, "diff.patch")?;
    assert_mutants_temp_path(&options.working_report, mode, "report")?;
    Ok(())
}

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
    assert!(text.contains("gandr-workflow-gates-mutants"));
    assert!(text.contains(mode));
    assert!(text.ends_with(suffix));
    Ok(())
}

/// Temporary documentation fixture for pure CLI dispatch tests.
struct DocsFixture
{
    /// Temporary root directory.
    root: PathBuf,
    /// Documentation corpus directory.
    corpus: PathBuf,
    /// ADR record directory.
    adr: PathBuf,
    /// Manifest path.
    manifest: PathBuf,
}

impl DocsFixture
{
    /// Create an empty documentation fixture.
    fn create<'semantic, Name>(name: Name) -> TestResult<Self>
    where
        Name: Into<NameText<'semantic>>,
    {
        let name = name.into().0;
        let suffix = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gandr-workflow-gates-tooling-{name}-{}-{suffix}",
            std::process::id()
        ));
        drop(gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(&root));
        let corpus = root.join("docs/gandr");
        let adr = root.join("docs/adr");
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&corpus)?;
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&adr)?;
        let manifest = corpus.join("MANIFEST.yml");
        Ok(Self {
            root,
            corpus,
            adr,
            manifest,
        })
    }

    /// Write a corpus document and return its BLAKE3 digest.
    fn write_doc<'semantic, Relative, Text>(
        &self,
        relative: Relative,
        text: Text,
    ) -> TestResult<String>
    where
        Relative: Into<RelativeText<'semantic>>,
        Text: Into<TextText<'semantic>>,
    {
        let text = text.into().0;
        let relative = relative.into().0;
        let doc_path = self.corpus.join(relative);
        let Some(parent) = doc_path.parent()
        else {
            return Err(Box::new(std::io::Error::other(
                "fixture document path has no parent",
            )));
        };
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(parent)?;
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(&doc_path, text)?;
        Ok(blake3_hex(text.as_bytes()))
    }

    /// Write a manifest with raw node YAML.
    fn write_manifest<'semantic, NodesYaml>(
        &self,
        nodes_yaml: NodesYaml,
    ) -> TestResult
    where
        NodesYaml: Into<NodesYamlText<'semantic>>,
    {
        let nodes_yaml = nodes_yaml.into().0;
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(
            &self.manifest,
            format!("version: 1\nhash: blake3\nnodes:\n{nodes_yaml}"),
        )?;
        Ok(())
    }

    /// Return the manifest path as UTF-8 for command construction.
    fn manifest_string(&self) -> String
    {
        self.manifest.to_string_lossy().into_owned()
    }
}

impl Drop for DocsFixture
{
    /// Remove the temporary root best-effort.
    fn drop(&mut self)
    {
        drop(gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(&self.root));
    }
}

/// Return a lowercase BLAKE3 hex digest for `bytes`.
fn blake3_hex<'semantic, Bytes>(bytes: Bytes) -> String
where
    Bytes: Into<BytesBytes<'semantic>>,
{
    let bytes = bytes.into().0;
    format!("{}", blake3::hash(bytes).to_hex())
}
