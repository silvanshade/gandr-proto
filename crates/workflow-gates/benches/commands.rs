// Miri cannot run Criterion benchmarks; `main` is a `cfg(miri)` stub below, so
// bench-only items are unused when Miri's driver compiles this target.
#![cfg_attr(
    miri,
    expect(
        unused_imports,
        dead_code,
        reason = "Criterion harness is stubbed out under Miri, so bench-only items are unused"
    )
)]

extern crate alloc;

use core::hint::black_box;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

use criterion::BatchSize;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use gandr_workflow_gates::contracts;
use gandr_workflow_gates::coverage;
use gandr_workflow_gates::docs;
use gandr_workflow_gates::maintenance;
use gandr_workflow_gates::mutants;
use gandr_workflow_gates::source_policy;
gandr_workflow_gates::semantic_copy!(pub struct CountCount(usize));
gandr_workflow_gates::semantic_str!(pub struct RelativeText);

impl<'item, 'text> From<&'item &'text str> for RelativeText<'text>
{
    #[inline]
    fn from(value: &'item &'text str) -> Self
    {
        Self(value)
    }
}
gandr_workflow_gates::semantic_copy!(pub struct ThroughputCountCount(u64));

/// The binary parser is production code but is not exported by the library
/// crate; including it follows the existing integration-test precedent while
/// keeping benchmark dispatch on the real command parser.
#[expect(
    dead_code,
    clippy::redundant_pub_crate,
    reason = "including the binary parser brings execution paths and crate-visible items that this private benchmark module does not call"
)]
#[path = "../src/main.rs"]
mod driver;

/// Nextest aggregate fixture used by contract witness parsing.
const NEXTEST_LIST: &str = r#"{
  "rust-suites": {
    "gandr-workflow-gates::contracts": {
      "package-name": "gandr-workflow-gates",
      "binary-name": "contracts",
      "testcases": {
        "contracts::item_witness": { "ignored": false },
        "contracts::secondary_witness": { "ignored": false }
      }
    }
  }
}"#;

/// Rust source fixture with one complete contract and adequacy group.
const CONTRACT_SOURCE: &str = r#"
/// # Contract
/// - ensures: deterministic order.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — deterministic ordering is separated by exact path assertions.
/// - witness: `contracts::item_witness`
pub fn item() {}
"#;

/// GitHub Actions workflow fixture for CI run-step contract analysis.
const CI_WORKFLOW: &str = r#"
name: ci
on: [push]
jobs:
  gates:
    runs-on: ubuntu-latest
    steps:
      - name: checkout
        uses: actions/checkout@v4
      - name: setup
        run: cargo binstall cargo-nextest --locked
      - name: rust gates
        run: mise run cargo:test
"#;

/// Rust source fixture for soundness-oracle policy analysis.
const SOUNDNESS_SOURCE: &str = r#"
#[test]
/// SOUNDNESS-ORACLE-WITNESS: coherence_generated_companion
fn coherence_free_generator() {}

#[test]
/// SOUNDNESS-ORACLE-COMPANION
fn coherence_generated_companion() {}
"#;

/// Representative command lines accepted by the production CLI parser.
const CLI_COMMANDS: &[&[&str]] = &[
    &["gandr-workflow-gates", "workflow", "push", "--cwd", "."],
    &[
        "gandr-workflow-gates",
        "contracts",
        "--scope",
        "crates/gandr-workflow-gates/src",
        "--nextest-list-fixture",
        "target/nextest.json",
    ],
    &[
        "gandr-workflow-gates",
        "coverage",
        "check",
        "--repo-root",
        ".",
        "--summary",
        "coverage/llvm-cov-summary.json",
        "--floors",
        "coverage/floors.toml",
    ],
    &[
        "gandr-workflow-gates",
        "maintenance-range",
        "--github-output",
        "github-output.txt",
        "--head",
        "HEAD",
        "--from",
        "main",
        "--watermark",
        "maintenance-watermark.json",
    ],
    &[
        "gandr-workflow-gates",
        "mutants",
        "push",
        "--workspace-root",
        ".",
        "--cache-image",
        "cache.raw",
        "--source-archive",
        "source.tar",
        "--diff-file",
        "changes.diff",
        "--working-report",
        "mutants.work",
        "--range-mode",
        "range",
        "--from",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "--to",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    ],
    &[
        "gandr-workflow-gates",
        "docs-manifest",
        "--manifest",
        "benches/fixtures/docs/gandr/MANIFEST.yml",
    ],
    &[
        "gandr-workflow-gates",
        "docs-reference",
        "--manifest",
        "benches/fixtures/docs/gandr/MANIFEST.yml",
    ],
    &[
        "gandr-workflow-gates",
        "options-policy",
        "--workspace-root",
        "benches/fixtures/source_policy",
    ],
    &[
        "gandr-workflow-gates",
        "soundness-oracles",
        "--workspace-root",
        ".",
    ],
];

/// Convert a static command fixture into owned OS arguments.
fn os_args<'semantic, Values, Value>(values: Values) -> Vec<OsString>
where
    Values: IntoIterator<Item = Value>,
    Value: Into<RelativeText<'semantic>>,
{
    values
        .into_iter()
        .map(|value| OsString::from(value.into().0))
        .collect()
}

/// Benchmark production CLI parsing and typed command-plan construction.
fn bench_cli_parsing_typed_plans(criterion: &mut Criterion)
{
    let mut group = criterion.benchmark_group("cli_parsing_typed_plans");
    group.throughput(Throughput::Elements(
        throughput_count(CLI_COMMANDS.len()).into().0,
    ));
    group.bench_function("parse_representative_commands", |bencher| {
        bencher.iter_batched(
            || {
                CLI_COMMANDS
                    .iter()
                    .map(|&command| os_args(command))
                    .collect::<Vec<_>>()
            },
            |commands| {
                for command in commands {
                    let parsed = fixture_value(driver::parse_command(black_box(command)));
                    black_box(parsed);
                }
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// Convert a fixture byte/element count into Criterion's throughput width.
fn throughput_count<Count>(count: Count) -> impl Into<ThroughputCountCount>
where
    Count: Into<CountCount>,
{
    let count = count.into().0;
    fixture_value(u64::try_from(count))
}

/// Benchmark contract witness parsing, Rust doc extraction, and CI run-step
/// validation.
fn bench_contract_extraction_validation(criterion: &mut Criterion)
{
    let witnesses = fixture_value(contracts::parse_nextest_witnesses(NEXTEST_LIST));
    let source_path = Path::new("crates/demo/src/lib.rs");
    let workflow_path = Path::new(".github/workflows/ci.yml");

    let mut group = criterion.benchmark_group("contract_extraction_validation");
    group.throughput(Throughput::Bytes(
        throughput_count(NEXTEST_LIST.len()).into().0,
    ));
    group.bench_function("parse_nextest_witnesses", |bencher| {
        bencher.iter(|| {
            let parsed = fixture_value(contracts::parse_nextest_witnesses(black_box(NEXTEST_LIST)));
            black_box(parsed.len());
        });
    });
    group.throughput(Throughput::Bytes(
        throughput_count(CONTRACT_SOURCE.len()).into().0,
    ));
    group.bench_function("analyze_rust_contracts", |bencher| {
        bencher.iter(|| {
            let findings = fixture_value(contracts::analyze_source(
                black_box(source_path),
                black_box(CONTRACT_SOURCE),
                black_box(&witnesses),
            ));
            black_box(findings.len());
        });
    });
    group.throughput(Throughput::Bytes(
        throughput_count(CI_WORKFLOW.len()).into().0,
    ));
    group.bench_function("analyze_ci_workflow_contracts", |bencher| {
        bencher.iter(|| {
            let findings = fixture_value(contracts::analyze_ci_workflow(
                black_box(workflow_path),
                black_box(CI_WORKFLOW),
            ));
            black_box(findings.len());
        });
    });
    group.finish();
}

/// Benchmark documentation manifest loading, drift checks, and reference
/// integrity.
fn bench_documentation_processing(criterion: &mut Criterion)
{
    let manifest_path = fixture_path("docs/gandr/MANIFEST.yml");

    let mut group = criterion.benchmark_group("documentation_manifest_reference_processing");
    group.throughput(Throughput::Elements(1));
    group.bench_function("load_manifest_context", |bencher| {
        bencher.iter(|| {
            let context = fixture_value(docs::manifest::ManifestContext::load(black_box(
                &manifest_path,
            )));
            black_box(context.nodes().len());
        });
    });
    group.throughput(Throughput::Elements(1));
    group.bench_function("run_manifest_drift", |bencher| {
        bencher.iter(|| {
            let findings = fixture_value(docs::manifest::run_manifest_drift(black_box(
                &manifest_path,
            )));
            black_box(findings.len());
        });
    });
    group.throughput(Throughput::Elements(1));
    group.bench_function("run_reference_integrity", |bencher| {
        bencher.iter(|| {
            let findings = fixture_value(docs::references::run_reference_integrity(black_box(
                &manifest_path,
            )));
            black_box(findings.len());
        });
    });
    group.finish();
}

/// Benchmark coverage-summary parsing, policy checking, ratcheting, and
/// rendering.
fn bench_coverage_policy(criterion: &mut Criterion)
{
    let summary_path = fixture_path("coverage/summary.json");
    let floors_path = fixture_path("coverage/floors.toml");
    let repo_root = fixture_path("");

    let mut group = criterion.benchmark_group("coverage_parsing_ratcheting_rendering");
    group.throughput(Throughput::Elements(1));
    group.bench_function("check_with_base_policy", |bencher| {
        bencher.iter(|| {
            let findings = fixture_value(coverage::check_with_base_policy(
                black_box(&summary_path),
                black_box(&floors_path),
                black_box(&repo_root),
                None,
            ));
            black_box(findings.len());
        });
    });
    group.throughput(Throughput::Elements(1));
    group.bench_function("ratchet_report", |bencher| {
        bencher.iter(|| {
            let report = fixture_value(coverage::ratchet_report(
                black_box(&summary_path),
                black_box(&floors_path),
                black_box(&repo_root),
            ));
            black_box((report.toml.len(), report.raised, report.added, report.stale));
        });
    });
    group.finish();
}

/// Benchmark pure maintenance range planning, watermark parsing, and rendering.
fn bench_maintenance_planning(criterion: &mut Criterion)
{
    let watermark_path = Path::new("maintenance-watermark.json");
    let watermark_text = r#"{"schema":1,"upper":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}"#;
    let upper = fixture_value(maintenance::CommitId::new(
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    ));

    let mut group = criterion.benchmark_group("maintenance_range_planning");
    group.throughput(Throughput::Elements(1));
    group.bench_function("plan_parse_render_watermark", |bencher| {
        bencher.iter(|| {
            let source = fixture_value(maintenance::plan_base_source(
                black_box(Some("main")),
                black_box(Some(watermark_path)),
            ));
            let parsed = fixture_value(maintenance::parse_watermark_text(
                black_box("maintenance-watermark.json"),
                black_box(watermark_text),
            ));
            let rendered = maintenance::watermark_json_bytes(black_box(&upper));
            black_box((source, parsed, rendered.len()));
        });
    });
    group.throughput(Throughput::Elements(1));
    group.bench_function("timestamp_cutoff", |bencher| {
        bencher.iter(|| {
            let timestamp = fixture_value(maintenance::CommitTimestamp::parse_git_output(
                black_box(&upper),
                black_box("1710000000\n"),
            ));
            let cutoff = fixture_value(timestamp.exclusive_before());
            black_box(cutoff.seconds());
        });
    });
    group.finish();
}

/// Return a successful fixture value or abort the benchmark process on drift.
fn fixture_value<T, E>(result: Result<T, E>) -> T
{
    match result {
        | Ok(value) => value,
        | Err(_) => std::process::abort(),
    }
}

/// Benchmark public mutation facade planning that does not spawn sandboxes.
fn bench_mutation_planning(criterion: &mut Criterion)
{
    let mut group = criterion.benchmark_group("mutation_planning");
    group.throughput(Throughput::Elements(3));
    group.bench_function("push_range_plan_constructors", |bencher| {
        bencher.iter(|| {
            let range = fixture_value(mutants::range::PushRangePlan::range(
                black_box("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                black_box("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            ));
            let full = fixture_value(mutants::range::PushRangePlan::full(
                black_box("cccccccccccccccccccccccccccccccccccccccc"),
                black_box("dddddddddddddddddddddddddddddddddddddddd"),
            ));
            let last = fixture_value(mutants::range::PushRangePlan::last(black_box(
                "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            )));
            black_box((range, full, last));
        });
    });
    group.throughput(Throughput::Elements(1));
    group.bench_function("facade_command_options", |bencher| {
        bencher.iter(|| {
            let options = mutants::MutantsOptions::new(
                black_box(PathBuf::from(".")),
                black_box(PathBuf::from("cache.raw")),
                black_box(PathBuf::from("source.tar")),
                black_box(PathBuf::from("changes.diff")),
                black_box(PathBuf::from("mutants.work")),
            );
            let command = mutants::MutantsCommand::Push {
                range: fixture_value(mutants::range::PushRangePlan::last(
                    "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                )),
            };
            black_box((command, options));
        });
    });
    group.finish();
}

/// Benchmark source-policy analyzers callable without external processes.
fn bench_source_policy_checks(criterion: &mut Criterion)
{
    let workspace_root = fixture_path("source_policy");
    let roots = [PathBuf::from("metatheory/src")];
    let soundness_path = Path::new("crates/gandr-core/src/conformance.rs");

    let mut group = criterion.benchmark_group("source_policy_checks");
    group.throughput(Throughput::Elements(1));
    group.bench_function("run_options_policy_with", |bencher| {
        bencher.iter(|| {
            let findings = fixture_value(source_policy::run_options_policy_with(
                black_box(&workspace_root),
                black_box(&roots),
                black_box(&source_policy::DEFAULT_OPTIONS_POLICIES),
            ));
            black_box(findings.len());
        });
    });
    group.throughput(Throughput::Bytes(
        throughput_count(SOUNDNESS_SOURCE.len()).into().0,
    ));
    group.bench_function("analyze_soundness_source", |bencher| {
        bencher.iter(|| {
            let findings = fixture_value(source_policy::analyze_soundness_source(
                black_box(soundness_path),
                black_box(SOUNDNESS_SOURCE),
            ));
            black_box(findings.len());
        });
    });
    group.finish();
}

/// Return a fixture path under this crate's committed benchmark fixtures.
fn fixture_path<'semantic, Relative>(relative: Relative) -> PathBuf
where
    Relative: Into<RelativeText<'semantic>>,
{
    let relative = relative.into().0;
    let Some(manifest_dir) = std::env::var_os("CARGO_MANIFEST_DIR")
    else {
        std::process::abort()
    };
    PathBuf::from(manifest_dir)
        .join("benches/fixtures")
        .join(relative)
}

/// Miri stub: Criterion's wall-clock timing is meaningless under
/// interpretation.
#[cfg(miri)]
fn main()
{
}

#[cfg(not(miri))]
criterion_group!(
    benches,
    bench_cli_parsing_typed_plans,
    bench_contract_extraction_validation,
    bench_documentation_processing,
    bench_coverage_policy,
    bench_maintenance_planning,
    bench_mutation_planning,
    bench_source_policy_checks,
);
#[cfg(not(miri))]
criterion_main!(benches);
