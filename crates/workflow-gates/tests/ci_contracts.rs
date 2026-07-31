//! Integration coverage for CI workflow run-step contracts.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::error::Error;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use std::path::Path;
use std::path::PathBuf;

use gandr_workflow_gates::Finding;
use gandr_workflow_gates::GateError;
use gandr_workflow_gates::contracts::analyze_ci_workflow;
use gandr_workflow_gates::contracts::run_ci_workflow;
use yaml_rust2::yaml::Hash;
use yaml_rust2::yaml::Yaml;
use yaml_rust2::yaml::YamlLoader;
/// Shared result type for CI workflow integration witnesses.
type TestResult<T = ()> = Result<T, Box<dyn Error>>;
gandr_workflow_gates::semantic_str!(pub(crate) struct SourceText);
gandr_workflow_gates::semantic_str!(pub(crate) struct YamlKeyText);
gandr_workflow_gates::semantic_str!(pub(crate) struct YamlContextText);
gandr_workflow_gates::semantic_str!(pub(crate) struct MiseTaskText);
gandr_workflow_gates::semantic_copy!(pub(crate) struct RunsMiseTaskFlag(bool));

/// Stable logical workflow path used in pure analyzer tests.
const WORKFLOW_PATH: &str = ".github/workflows/ci.yml";

/// Per-process suffix keeping concurrently-created fixtures disjoint.
static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

/// Prohibited bare real-work tools are reported independently.
#[test]
fn rejects_each_prohibited_real_work_tool() -> TestResult
{
    let findings = analyze(
        r#"jobs:
  real-work:
    steps:
      - name: cargo test
        run: cargo test
      - name: aube test
        run: aube test
      - name: treefmt ci
        run: treefmt --ci
      - name: wrkflw validate
        run: wrkflw validate
"#,
    )?;
    let expected = [
        (
            "job=real-work step=1",
            "job `real-work` step 1 `cargo test` runs `cargo test` through prohibited real-work tool `cargo`; replace it with `mise run cargo:nextest` or another self-contained mise task",
        ),
        (
            "job=real-work step=2",
            "job `real-work` step 2 `aube test` runs `aube test` through prohibited real-work tool `aube`; replace it with `mise run grammar:test` or another self-contained mise task",
        ),
        (
            "job=real-work step=3",
            "job `real-work` step 3 `treefmt ci` runs `treefmt --ci` through prohibited real-work tool `treefmt`; replace it with `mise run treefmt:check` or another self-contained mise task",
        ),
        (
            "job=real-work step=4",
            "job `real-work` step 4 `wrkflw validate` runs `wrkflw validate` through prohibited real-work tool `wrkflw`; replace it with `mise run wrkflw` or another self-contained mise task",
        ),
    ];
    assert_eq!(findings.len(), expected.len());
    for (finding, (declaration, detail)) in findings.iter().zip(expected) {
        assert_eq!("ci-bare-run-step", finding.kind);
        assert_eq!("", finding.package);
        assert_eq!(WORKFLOW_PATH, finding.path);
        assert_eq!(finding.declaration, declaration);
        assert_eq!(finding.detail, detail);
    }
    Ok(())
}

/// Mise tasks and representative setup/environment/tool-install run steps pass.
#[test]
fn accepts_mise_tasks_and_setup_allowances() -> TestResult
{
    let findings = analyze(
        r#"jobs:
  setup:
    steps:
      - name: checkout
        uses: actions/checkout@v4
      - name: cargo nextest task
        run: mise run cargo:nextest
      - name: grammar task
        run: mise run grammar:test
      - name: enable mise tools
        run: |
          tools=()
          tools+=("cargo:cargo-nextest")
          tools+=("aube")
          IFS=,; echo "MISE_ENABLE_TOOLS=${tools[*]}" >> $GITHUB_ENV
      - name: prepare caches
        run: |
          sudo mkdir -p "$HOME/.cache/aube"
          sudo chown -R "$(id -u):$(id -g)" "$HOME/.cache/aube"
      - name: install rust component
        run: rustup component add clippy
      - name: cargo install tool
        run: cargo install cargo-nextest --locked
      - name: aube install packages
        run: aube ci --recursive
      - name: setup values are not commands
        run: |
          tools+=("github:numtide/treefmt")
          echo "cargo test"
"#,
    )?;
    assert!(findings.is_empty());
    Ok(())
}

/// Clippy and Dylint remain separate dependency-free CI jobs so GitHub may run
/// both lint invariants concurrently.
/// Parked (gandr-kk7): reads the live `.github/workflows/ci.yml`, which the
/// reboot repo does not have yet; un-ignore when the CI workflow lands.
#[test]
#[ignore = "no .github/workflows/ci.yml in the reboot repo yet (gandr-kk7)"]
fn clippy_and_dylint_jobs_are_independent() -> TestResult
{
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workflow_path = workspace.join(WORKFLOW_PATH);
    let source = gandr_workflow_gates::support::HOST_FILESYSTEM.read_to_string(&workflow_path)?;
    let documents = YamlLoader::load_from_str(&source)?;
    let [ref document] = *documents.as_slice()
    else {
        return Err(Box::new(std::io::Error::other(format!(
            "CI workflow must contain exactly one YAML document, found {}",
            documents.len()
        ))));
    };
    let root = required_yaml_mapping(document, "CI workflow root")?;
    let jobs = required_yaml_mapping(
        required_yaml_value(root, "jobs", "CI workflow root")?,
        "CI jobs",
    )?;

    for (job_id, task) in [
        ("cargo-clippy-crates", "cargo:clippy"),
        ("cargo-dylint-crates", "cargo:dylint"),
    ] {
        let job = required_yaml_mapping(required_yaml_value(jobs, job_id, "CI jobs")?, job_id)?;
        assert!(
            yaml_mapping_value(job, "needs").is_none(),
            "CI lint job `{job_id}` must have no `needs` dependency"
        );
        assert!(
            yaml_job_runs_mise_task(job, task)?.0,
            "CI lint job `{job_id}` must run `mise run {task}`"
        );
    }
    Ok(())
}

/// Shell composition, wrappers, substitutions, dynamic commands, and direct
/// Nushell scripts cannot bypass the one-task boundary.
#[test]
fn rejects_noncanonical_real_work_shapes() -> TestResult
{
    let findings = analyze(
        r#"jobs:
  bypasses:
    steps:
      - name: mixed mise
        run: mise run cargo:nextest && cargo test
      - name: setup then work
        run: cargo install cargo-nextest && cargo test
      - name: wrapped absolute cargo
        run: env /usr/bin/cargo test
      - name: substitution
        run: echo $(cargo test)
      - name: direct nu script
        run: nu scripts/check.nu
      - name: dynamic matrix command
        run: ${{ matrix.command }}
      - name: task arguments
        run: mise run cargo:nextest --profile ci
      - name: assigned dynamic command
        run: |
          cmd=cargo
          $cmd test
      - name: env wrapped dynamic command
        run: env "$cmd" test
      - name: command wrapped dynamic command
        run: command ${TOOL} test
      - name: computed absolute command
        run: '"$HOME/bin/cargo" test'
"#,
    )?;
    assert_eq!(11, findings.len());
    assert!(
        findings
            .iter()
            .all(|finding| finding.kind == "ci-bare-run-step")
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.detail.contains("dynamic expression"))
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.detail.contains("prohibited real-work tool `nu`"))
    );
    Ok(())
}

/// Delegating wrappers and local shell dispatch cannot hide real work.
#[test]
fn rejects_uninspectable_dispatch_shapes() -> TestResult
{
    let findings = analyze(
        r#"jobs:
  dispatch:
    steps:
      - name: rustup wrapper
        run: rustup run 1.97.1 cargo test
      - name: eval
        run: eval "cargo test"
      - name: shell function
        run: run_tests() { cargo test; }; run_tests
      - name: environment-prefixed shell function
        run: CHECK=1 run_tests() { cargo test; }; run_tests
      - name: sourced script
        run: source scripts/check.sh
      - name: dynamic mise task
        run: mise run ${TASK}
      - name: local executable
        run: ./scripts/check
      - name: interpreter
        run: python3 -c "import subprocess; subprocess.run(['cargo', 'test'])"
"#,
    )?;
    let expected_tools = [
        "prohibited real-work tool `cargo`",
        "prohibited real-work tool `dynamic expression`",
        "prohibited real-work tool `dynamic expression`",
        "prohibited real-work tool `dynamic expression`",
        "prohibited real-work tool `dynamic expression`",
        "prohibited real-work tool `mise`",
        "prohibited real-work tool `dynamic expression`",
        "prohibited real-work tool `dynamic expression`",
    ];
    assert_eq!(findings.len(), expected_tools.len());
    for (finding, expected_tool) in findings.iter().zip(expected_tools) {
        assert!(
            finding.detail.contains(expected_tool),
            "unexpected detail: {}",
            finding.detail
        );
    }
    Ok(())
}

/// Toolchain-qualified installs remain setup-only.
#[test]
fn accepts_toolchain_qualified_cargo_install() -> TestResult
{
    let findings = analyze(
        "jobs:\n  setup:\n    steps:\n      - run: cargo +nightly --locked install cargo-nextest\n",
    )?;
    assert!(findings.is_empty());
    Ok(())
}

/// Toolchain-qualified setup installs remain setup-only through `rustup run`.
#[test]
fn accepts_rustup_run_toolchain_qualified_cargo_install() -> TestResult
{
    let findings = analyze(
        "jobs:\n  setup:\n    steps:\n      - run: rustup run nightly cargo install cargo-nextest --locked\n",
    )?;
    assert!(findings.is_empty());
    Ok(())
}

/// Local actions and job-level reusable workflows fail closed because this
/// analyzer cannot inspect their run steps.
#[test]
fn rejects_uninspectable_workflow_indirection() -> TestResult
{
    let local = operational_detail(analyze_ci_workflow(
        Path::new(WORKFLOW_PATH),
        "jobs:\n  local:\n    steps:\n      - uses: ./actions/check\n",
    ))?;
    assert!(local.contains("local action `./actions/check` is not inspectable"));

    let reusable = operational_detail(analyze_ci_workflow(
        Path::new(WORKFLOW_PATH),
        "jobs:\n  shared:\n    uses: owner/repo/.github/workflows/check.yml@v1\n",
    ))?;
    assert!(reusable.contains("reusable workflow uses are not inspectable"));
    Ok(())
}

/// Malformed workflow YAML is an operational error, not a semantic finding.
#[test]
fn malformed_workflows_are_operational_errors() -> TestResult
{
    let detail = operational_detail(analyze_ci_workflow(
        Path::new(WORKFLOW_PATH),
        "jobs:\n  build: [",
    ))?;
    assert!(
        detail.contains("workflow YAML parse error"),
        "unexpected detail: {detail}"
    );
    assert!(detail.contains(WORKFLOW_PATH), "missing path: {detail}");
    Ok(())
}

/// Malformed workflow shapes have exact operational diagnostics.
#[test]
fn malformed_workflow_shapes_have_exact_operational_details() -> TestResult
{
    let cases = [
        (
            "multiple documents",
            "jobs: {}\n---\njobs: {}\n",
            "malformed workflow: path=.github/workflows/ci.yml detail=workflow YAML must contain exactly one document, found 2",
        ),
        (
            "non-mapping root",
            "[]\n",
            "malformed workflow: path=.github/workflows/ci.yml detail=workflow root must be a mapping with jobs",
        ),
        (
            "missing jobs",
            "name: CI\n",
            "malformed workflow: path=.github/workflows/ci.yml detail=workflow root must contain jobs mapping",
        ),
        (
            "non-mapping jobs",
            "jobs: []\n",
            "malformed workflow: path=.github/workflows/ci.yml detail=jobs must be a mapping",
        ),
        (
            "non-string job id",
            "jobs:\n  1:\n    steps: []\n",
            "malformed workflow: path=.github/workflows/ci.yml detail=job ids must be strings",
        ),
        (
            "non-mapping job",
            "jobs:\n  build: []\n",
            "malformed workflow: path=.github/workflows/ci.yml detail=job must be a mapping",
        ),
        (
            "non-array steps",
            "jobs:\n  build:\n    steps: {}\n",
            "malformed workflow: path=.github/workflows/ci.yml detail=job `build` steps must be an array",
        ),
        (
            "non-mapping concrete step",
            "jobs:\n  build:\n    steps:\n      - cargo test\n",
            "malformed workflow: path=.github/workflows/ci.yml detail=job `build` step 1 must be a mapping or alias",
        ),
        (
            "non-string run",
            "jobs:\n  build:\n    steps:\n      - name: bad\n        run: [cargo, test]\n",
            "malformed workflow: path=.github/workflows/ci.yml detail=job `build` step 1 run must be a string",
        ),
    ];
    for (name, source, expected) in cases {
        let detail = operational_detail(analyze_ci_workflow(Path::new(WORKFLOW_PATH), source))?;
        assert_eq!(detail, expected, "{name}");
    }
    Ok(())
}

/// Findings name the exact job and step and carry a concrete mise action.
#[test]
fn diagnostics_name_job_step_and_action() -> TestResult
{
    let findings = analyze(
        r#"jobs:
  diagnostics:
    steps:
      - name: setup
        run: echo setup
      - name: cargo test
        run: cargo test
"#,
    )?;
    assert_eq!(1, findings.len());
    let Some(finding) = findings.first()
    else {
        return Err(Box::new(std::io::Error::other(
            "finding vector length changed during observation",
        )));
    };
    assert_eq!(WORKFLOW_PATH, finding.path);
    assert_eq!("job=diagnostics step=2", finding.declaration);
    assert_eq!(
        "job `diagnostics` step 2 `cargo test` runs `cargo test` through prohibited real-work tool `cargo`; replace it with `mise run cargo:nextest` or another self-contained mise task",
        finding.detail
    );
    Ok(())
}

/// File-backed workflow analysis reads a clean workflow fixture.
#[test]
fn run_ci_workflow_reads_clean_workflow_fixture() -> TestResult
{
    let fixture = Fixture::create()?;
    gandr_workflow_gates::support::HOST_FILESYSTEM.write(
        &fixture.workflow_path,
        r#"jobs:
  build:
    steps:
      - name: cargo nextest task
        run: mise run cargo:nextest
"#,
    )?;
    let findings = run_ci_workflow(&fixture.workflow_path)?;
    assert!(findings.is_empty());
    Ok(())
}

/// File-backed workflow analysis preserves missing-workflow I/O paths.
#[test]
fn run_ci_workflow_reports_missing_workflow_path() -> TestResult
{
    let fixture = Fixture::create()?;
    let missing_workflow = fixture.root.join("missing.yml");
    match run_ci_workflow(&missing_workflow) {
        | Err(GateError::Io { path, .. }) => assert_eq!(path, missing_workflow),
        | Err(error) => return Err(Box::new(error)),
        | Ok(_) => {
            return Err(Box::new(std::io::Error::other(
                "missing workflow unexpectedly analyzed successfully",
            )));
        },
    }
    Ok(())
}

/// Return a required string-keyed YAML mapping value.
fn required_yaml_value<'yaml, 'text, Key, Context>(
    mapping: &'yaml Hash,
    key: Key,
    context: Context,
) -> TestResult<&'yaml Yaml>
where
    Key: Into<YamlKeyText<'text>>,
    Context: Into<YamlContextText<'text>>,
{
    let key = key.into().0;
    let context = context.into().0;
    yaml_mapping_value(mapping, key).ok_or_else(|| {
        Box::<dyn Error>::from(std::io::Error::other(format!(
            "{context} must contain `{key}`"
        )))
    })
}

/// Return a required YAML mapping.
fn required_yaml_mapping<'yaml, 'text, Context>(
    value: &'yaml Yaml,
    context: Context,
) -> TestResult<&'yaml Hash>
where
    Context: Into<YamlContextText<'text>>,
{
    let context = context.into().0;
    match *value {
        | Yaml::Hash(ref mapping) => Ok(mapping),
        | _ => Err(Box::new(std::io::Error::other(format!(
            "{context} must be a mapping"
        )))),
    }
}

/// Return a string-keyed YAML mapping value without a temporary key.
fn yaml_mapping_value<'yaml, 'text, Key>(
    mapping: &'yaml Hash,
    key: Key,
) -> Option<&'yaml Yaml>
where
    Key: Into<YamlKeyText<'text>>,
{
    let key = key.into().0;
    for (candidate, value) in mapping {
        if let Yaml::String(ref candidate) = *candidate
            && candidate == key
        {
            return Some(value);
        }
    }
    None
}

/// Return whether a workflow job has an exact `mise run TASK` step.
fn yaml_job_runs_mise_task<'text, Task>(
    job: &Hash,
    task: Task,
) -> TestResult<RunsMiseTaskFlag>
where
    Task: Into<MiseTaskText<'text>>,
{
    let task = task.into().0;
    let steps = required_yaml_value(job, "steps", "CI lint job")?;
    let Yaml::Array(ref steps) = *steps
    else {
        return Err(Box::new(std::io::Error::other(
            "CI lint job `steps` must be an array",
        )));
    };
    let expected = format!("mise run {task}");
    for step in steps {
        let Yaml::Hash(ref step) = *step
        else {
            continue;
        };
        if let Some(yaml) = yaml_mapping_value(step, "run")
            && let Yaml::String(ref run) = *yaml
            && run.trim() == expected
        {
            return Ok(RunsMiseTaskFlag(true));
        }
    }
    Ok(RunsMiseTaskFlag(false))
}

/// Analyze workflow source at the stable logical path.
fn analyze<'semantic, Source>(source: Source) -> TestResult<Vec<Finding>>
where
    Source: Into<SourceText<'semantic>>,
{
    let source = source.into().0;
    Ok(analyze_ci_workflow(Path::new(WORKFLOW_PATH), source)?)
}

/// Extract an operational detail from an analyzer result.
fn operational_detail(result: Result<Vec<Finding>, GateError>) -> TestResult<String>
{
    match result {
        | Err(GateError::Operational { detail }) => Ok(detail),
        | Err(error) => Err(Box::new(error)),
        | Ok(_) => Err(Box::new(std::io::Error::other(
            "workflow unexpectedly analyzed successfully",
        ))),
    }
}

/// Temporary workflow fixture.
struct Fixture
{
    /// Temporary root directory.
    root: PathBuf,
    /// Workflow path under the temporary root.
    workflow_path: PathBuf,
}

impl Fixture
{
    /// Create an empty temporary workflow fixture.
    fn create() -> TestResult<Self>
    {
        let suffix = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "gandr-workflow-gates-ci-contracts-{}-{suffix}",
            std::process::id()
        ));
        gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_if_exists(&root)?;
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        Ok(Self {
            workflow_path: root.join("ci.yml"),
            root,
        })
    }
}

impl Drop for Fixture
{
    /// Remove the temporary root best-effort.
    fn drop(&mut self)
    {
        drop(gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(&self.root));
    }
}
