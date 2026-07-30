//! Coverage floor parsing, checking, ratcheting, and base-policy resolution.
//!
//! # Contract
//! - requires: callers provide a cargo-llvm-cov JSON summary, a floor-policy
//!   TOML path, and the repository root used to normalize llvm-cov filenames.
//! - ensures: parsing is strict for exporter identity, JSON shape, TOML policy
//!   shape, floor precision, and path domains; policy joins are deterministic.
//! - provides: check and ratchet library APIs without command-line wiring.
//! - fails: returns [`GateError`] for unreadable/malformed inputs and returns
//!   [`crate::Finding`] rows for semantic policy violations.
//! - panics: none.
//! - intension: measured rows, floor rows, base rows, and exemptions are joined
//!   through `BTreeMap` keys in stable order, with no input-scaled recursion.
//!
//! # Adequacy
//! - hypothesis: L3 only — parser shape fixtures, path-boundary fixtures,
//!   monotonicity/base fixtures, ratchet snapshots, and git-ref policy fixtures
//!   distinguish every retained coverage contract family.
//! - witness: `coverage::policy::tests::json_identity_and_shape_failures_are_exact`
//! - witness: `coverage::policy::tests::check_report_covers_policy_failure_families`
//! - witness: `coverage::policy::tests::ratchet_report_caps_and_renders_stably`
//! - witness: `coverage::policy::tests::base_ref_policy_distinguishes_zero_head_root_and_missing_blob`

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;

use serde_json::Map;
use serde_json::Value;

use super::model::CoverageFailure;
use super::model::CoverageFloors;
use super::model::DEFAULT_TARGET_PERCENT;
use super::model::MeasuredCoverage;
use super::model::Percent;
use super::model::PercentParseError;
use super::model::ProductionFile;
use super::model::RatchetReport;
use super::model::coverage_error;
use super::model::slash_path;
use super::render::render_floors;
use crate::GateError;
use crate::GateResult;
use crate::support;

crate::semantic_str!(pub struct SourceNameText);
crate::semantic_str!(pub struct TextText);
crate::semantic_copy!(pub struct HistoricalFlag(bool));
crate::semantic_str!(pub struct KeyText);
crate::semantic_str!(pub struct FilenameText);
crate::semantic_copy!(pub struct ExemptionsWasScalarFlag(bool));
crate::semantic_str!(pub struct FileText);
crate::semantic_str!(pub struct RequestedText);
crate::semantic_str!(pub struct CommitText);
crate::semantic_str!(pub struct TargetText);
crate::semantic_str!(pub struct MessageText);
crate::semantic_str!(pub struct ExpectedText);
crate::semantic_str!(pub struct LineText);
crate::semantic_str!(pub struct LabelText);
crate::semantic_copy!(pub struct ParseLineCountCount(u64));
crate::semantic_copy!(pub struct RatchetCounterCount(usize));
crate::semantic_str!(pub struct JsonKindText);
crate::semantic_copy!(pub struct AllZeroFlag(bool));
crate::semantic_copy!(pub struct HeadIsRootCommitFlag(bool));
crate::semantic_copy!(pub struct BaseTreeContainsFloorsFlag(bool));

/// Default floor-policy path used by the Git base resolver.
pub const DEFAULT_FLOORS: &str = "coverage/floors.toml";

/// Default cargo-llvm-cov summary path retained for integration callers.
pub const DEFAULT_SUMMARY: &str = "coverage/llvm-cov-summary.json";

/// Run the coverage check, including sanitized Git base-policy resolution.
///
/// # Contract
/// - requires: `floors_path` is the canonical `coverage/floors.toml` path when
///   base-policy resolution is required.
/// - ensures: returns all semantic policy failures as sorted findings and
///   returns an empty vector when the measured coverage satisfies policy.
/// - provides: the library equivalent of `coverage-ratchet.nu check` without
///   CLI parsing or printing.
/// - fails: returns typed gate errors for unreadable/malformed inputs, invalid
///   base refs, Git inspection failures, and invalid historical floor policies.
/// - panics: none.
/// - intension: resolves the base policy once, then delegates to the pure
///   `BTreeMap` check join.
///
/// # Errors
/// Returns [`GateError`] for summary/TOML/Git operational failures.
///
/// # Adequacy
/// - hypothesis: L3 only — a clean baseline, every semantic failure family, and
///   base-ref edge fixtures distinguish successful, findings, and operational
///   outcomes.
/// - witness: `coverage::policy::tests::clean_baseline_passes_without_findings`
/// - witness: `coverage::policy::tests::check_report_covers_policy_failure_families`
#[inline]
pub fn check(
    summary_path: &Path,
    floors_path: &Path,
    repo_root: &Path,
) -> GateResult
{
    let base = previous_floors(floors_path, repo_root, None)?;
    check_with_base_policy(summary_path, floors_path, repo_root, base.as_ref())
}

/// Run the coverage check with an already materialized optional base policy.
///
/// # Contract
/// - requires: `base_policy`, when supplied, was parsed with historical target
///   rules.
/// - ensures: enforces current/base monotonicity, target-change clamps,
///   disappearing floors, new-file seed floors, exemptions, missing floors,
///   stale floors, and coverage regressions.
/// - provides: a deterministic pure-policy entry point for tests and
///   integration layers that own base materialization.
/// - fails: returns typed gate errors for current summary or TOML failures.
/// - panics: none.
/// - intension: performs stable `BTreeMap` joins rather than repeated list
///   scans.
///
/// # Errors
/// Returns [`GateError`] for summary/TOML parse and validation failures.
///
/// # Adequacy
/// - hypothesis: L3 only — one fixture per policy family observes the exact
///   emitted finding detail and stable order.
/// - witness: `coverage::policy::tests::check_report_covers_policy_failure_families`
#[inline]
pub fn check_with_base_policy(
    summary_path: &Path,
    floors_path: &Path,
    repo_root: &Path,
    base_policy: Option<&CoverageFloors>,
) -> GateResult
{
    let measured = load_measured(summary_path, repo_root)?;
    let floors = load_floors(floors_path, false)?;
    let failures = check_maps(&measured, &floors, base_policy);
    let findings = failures
        .into_iter()
        .map(CoverageFailure::into_finding)
        .collect();
    Ok(findings)
}

/// Ratchet floors and rewrite the TOML atomically.
///
/// # Contract
/// - requires: `summary_path` and `floors_path` point to readable current
///   coverage and floor-policy inputs.
/// - ensures: writes the deterministic ratcheted TOML to `floors_path` and
///   returns the same report that was written.
/// - provides: the library equivalent of `coverage-ratchet.nu ratchet` without
///   CLI parsing or printing.
/// - fails: returns typed gate errors for parse/validation failures or atomic
///   write failure.
/// - panics: none.
/// - intension: computes the full report before one `write_atomic` call.
///
/// # Errors
/// Returns [`GateError`] for summary/TOML failures or for failed atomic write.
///
/// # Adequacy
/// - hypothesis: L3 only — ratchet report snapshots and write-failure injection
///   distinguish compute-vs-write behavior and diagnostic text.
/// - witness: `coverage::policy::tests::ratchet_report_caps_and_renders_stably`
#[inline]
pub fn ratchet(
    summary_path: &Path,
    floors_path: &Path,
    repo_root: &Path,
) -> Result<RatchetReport, GateError>
{
    let report = ratchet_report(summary_path, floors_path, repo_root)?;
    support::write_atomic(floors_path, report.toml.as_bytes()).map_err(|error| {
        coverage_error(format!(
            "failed to write coverage floors TOML {}: {error}",
            slash_path(floors_path)
        ))
    })?;
    Ok(report)
}

/// Build a ratchet report without writing the floor policy.
///
/// # Contract
/// - requires: inputs are readable current summary and floor TOML files.
/// - ensures: raises existing floors to `max(existing, min(measured, target))`,
///   starts new files at target, retains stale rows, and keeps only
///   still-active exemptions for floors below target.
/// - provides: deterministic ratchet counts plus rendered TOML.
/// - fails: returns typed gate errors for unreadable or invalid inputs.
/// - panics: none.
/// - intension: joins through one measured map, one current floor map, and one
///   rendered output map.
///
/// # Errors
/// Returns [`GateError`] for summary/TOML parse and validation failures.
///
/// # Adequacy
/// - hypothesis: L3 only — capped raise, retained high floor, added file,
///   unchanged file, stale file, and active/inactive exemption fixtures
///   distinguish all counters and row decisions.
/// - witness: `coverage::policy::tests::ratchet_report_caps_and_renders_stably`
#[inline]
pub fn ratchet_report(
    summary_path: &Path,
    floors_path: &Path,
    repo_root: &Path,
) -> Result<RatchetReport, GateError>
{
    let measured = load_measured(summary_path, repo_root)?;
    let floors = load_floors(floors_path, false)?;
    ratchet_maps(&measured, &floors)
}

/// Read and parse a cargo-llvm-cov JSON summary into measured production rows.
///
/// # Contract
/// - requires: `repo_root` is the repository root used by llvm-cov filename
///   normalization.
/// - ensures: returns one measured row per canonical production file and
///   rejects duplicate normalized production paths.
/// - provides: strict JSON identity, shape, count, and percentage validation.
/// - fails: returns stable operational errors for unreadable JSON, malformed
///   JSON, unsupported exporter identity, invalid shape, invalid counts,
///   invalid percentages, no production rows, or duplicate normalized rows.
/// - panics: none.
/// - intension: parses JSON once and accumulates rows into a `BTreeMap` plus a
///   duplicate set.
///
/// # Errors
/// Returns [`GateError::Operational`] with retained coverage diagnostics.
///
/// # Adequacy
/// - hypothesis: L3 only — schema/identity/count/percent/duplicate fixtures
///   distinguish each validation family.
/// - witness: `coverage::policy::tests::json_identity_and_shape_failures_are_exact`
/// - witness: `coverage::policy::tests::duplicate_normalized_summary_rows_fail`
#[inline]
fn load_measured(
    summary_path: &Path,
    repo_root: &Path,
) -> Result<BTreeMap<ProductionFile, MeasuredCoverage>, GateError>
{
    let source_name = slash_path(summary_path);
    let text = support::read_utf8(summary_path).map_err(|error| {
        coverage_error(format!(
            "failed to read coverage summary JSON {source_name}: {error}"
        ))
    })?;
    parse_measured_text(&source_name, &text, repo_root)
}

/// Parse measured coverage from already loaded JSON text.
#[inline]
fn parse_measured_text<'semantic, S, T>(
    source_name: S,
    text: T,
    repo_root: &Path,
) -> Result<BTreeMap<ProductionFile, MeasuredCoverage>, GateError>
where
    S: Into<SourceNameText<'semantic>>,
    T: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let source_name = source_name.into().0;
    let document: Value = serde_json::from_str(text).map_err(|error| {
        coverage_error(format!(
            "failed to read coverage summary JSON {source_name}: {error}"
        ))
    })?;
    let Some(root) = document.as_object()
    else {
        return Err(coverage_error(format!(
            "coverage summary JSON must be a record from cargo-llvm-cov, got {}",
            json_kind(&document).into().0
        )));
    };
    validate_summary_identity(root)?;
    let files = summary_files(root)?;

    let mut rows = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for file in files {
        let measurement = parse_summary_file(file, repo_root)?;
        let Some(measurement) = measurement
        else {
            continue;
        };
        if rows.contains_key(&measurement.file) {
            duplicates.insert(measurement.file.clone());
            continue;
        }
        rows.insert(measurement.file.clone(), measurement);
    }
    if !duplicates.is_empty() {
        let files_text = duplicates
            .iter()
            .map(|file| file.as_str().into().0)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(coverage_error(format!(
            "coverage summary contains duplicate normalized production files: {files_text}"
        )));
    }
    if rows.is_empty() {
        return Err(coverage_error(
            "coverage summary contains no measured production files under crates/",
        ));
    }
    Ok(rows)
}

/// Read and parse a floor-policy TOML file.
///
/// # Contract
/// - requires: `floors_path` points to UTF-8 TOML in the retained policy
///   subset.
/// - ensures: current policies require target `80.00`; historical policies
///   allow other two-decimal targets for base-clamp comparison.
/// - provides: validated floor and exemption maps keyed by canonical production
///   files.
/// - fails: returns stable operational diagnostics for unreadable/malformed
///   TOML, invalid target/floor precision, missing or empty `[files]`, bad
///   paths, and invalid exemptions.
/// - panics: none.
/// - intension: parses TOML once, then validates into `BTreeMap` order.
///
/// # Errors
/// Returns [`GateError::Operational`] for all TOML and policy-shape failures.
///
/// # Adequacy
/// - hypothesis: L3 only — malformed, missing-files, empty-files, high-target,
///   high-floor, hidden-precision, nonfinite, bad-path, and exemption fixtures
///   distinguish all parser branches.
/// - witness: `coverage::policy::tests::floor_policy_shape_is_strict`
/// - witness: `coverage::policy::tests::new_file_exemptions_are_strict`
#[inline]
fn load_floors<H>(
    floors_path: &Path,
    historical: H,
) -> Result<CoverageFloors, GateError>
where
    H: Into<HistoricalFlag>,
{
    let historical = historical.into().0;
    let source_name = slash_path(floors_path);
    let text = support::read_utf8(floors_path).map_err(|error| {
        coverage_error(format!(
            "failed to read coverage floors TOML {source_name}: {error}"
        ))
    })?;
    parse_floors_text(&source_name, &text, historical)
}

/// Parse a floor policy from already loaded TOML text.
#[inline]
fn parse_floors_text<'semantic, S, T, H>(
    source_name: S,
    text: T,
    historical: H,
) -> Result<CoverageFloors, GateError>
where
    S: Into<SourceNameText<'semantic>>,
    T: Into<TextText<'semantic>>,
    H: Into<HistoricalFlag>,
{
    let text = text.into().0;
    let historical = historical.into().0;
    let source_name = source_name.into().0;
    let raw = parse_raw_floors_toml(text).map_err(|detail| {
        coverage_error(format!(
            "failed to read coverage floors TOML {source_name}: {detail}"
        ))
    })?;
    let target_percent =
        parse_target_percent(source_name, raw.target_percent.as_ref(), historical)?;
    let Some(raw_files) = raw.files
    else {
        return Err(coverage_error(format!(
            "coverage floors TOML {source_name} is missing [files]"
        )));
    };
    let mut files = BTreeMap::new();
    for (file, raw_floor) in raw_files {
        let production_file = ProductionFile::from_floor_key(&file)?;
        let floor = parse_floor_percent(&file, &raw_floor, target_percent)?;
        files.insert(production_file, floor);
    }
    if files.is_empty() {
        return Err(coverage_error(format!(
            "coverage floors TOML {source_name} has no [files] entries"
        )));
    }
    if raw.exemptions_was_scalar {
        return Err(coverage_error("[new_file_exemptions] must be a table"));
    }
    let exemptions = validate_exemptions(raw.exemptions, &files, target_percent)?;
    Ok(CoverageFloors {
        target_percent,
        files,
        exemptions,
    })
}

/// Parse current coverage-floor policy text for fuzzing.
///
/// # Contract
/// - requires: `text` is already UTF-8 coverage-floor TOML input supplied by
///   the public fuzzing facade.
/// - ensures: runs the same current-policy parser and validation path as loaded
///   production floor policies, with no filesystem, process, environment, or
///   output effects.
/// - provides: the minimum feature-gated visibility seam needed by the crate
///   fuzzing facade.
/// - fails: returns the existing coverage-policy [`GateError`] for malformed
///   TOML or invalid policy semantics.
/// - panics: none.
/// - intension: delegates directly to [`parse_floors_text`] with current-policy
///   target validation.
///
/// # Errors
/// Returns [`GateError`] when parsing or validation rejects the input.
///
/// # Adequacy
/// - hypothesis: L3 only — arbitrary AFL UTF-8 inputs exercise malformed TOML,
///   invalid policy shape, precision, path, and exemption branches through the
///   production parser.
/// - witness: `fuzz/fuzz_targets/gates.rs`
#[cfg(feature = "fuzzing")]
#[inline]
pub(super) fn parse_floors_text_for_fuzzing<'semantic, S, T>(
    source_name: S,
    text: T,
) -> Result<CoverageFloors, GateError>
where
    S: Into<SourceNameText<'semantic>>,
    T: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let source_name = source_name.into().0;
    parse_floors_text(source_name, text, false)
}

/// Resolve the previous floor policy using sanitized Git commands.
///
/// # Contract
/// - requires: `floors_path` is exactly `coverage/floors.toml` when base lookup
///   is attempted.
/// - ensures: empty or all-zero base refs map to `HEAD^`; a root commit with no
///   parent yields no base policy; a missing base blob yields no base policy;
///   unresolved refs, tree-inspection failures, and unreadable blobs are hard
///   errors.
/// - provides: the historical policy for monotonic check comparison.
/// - fails: returns stable operational diagnostics for noncanonical floor path,
///   unresolved refs, Git inspection/read failures, and invalid historical
///   TOML.
/// - panics: none.
/// - intension: uses only sanitized Git invocations through the support layer.
///
/// # Errors
/// Returns [`GateError`] for support-layer command failures or retained policy
/// diagnostics.
///
/// # Adequacy
/// - hypothesis: L3 only — all-zero, explicit ref, root `HEAD^`, unresolved
///   shallow ref, missing blob, and read-failure fixtures distinguish each Git
///   branch.
/// - witness: `coverage::policy::tests::base_ref_policy_distinguishes_zero_head_root_and_missing_blob`
#[inline]
fn previous_floors(
    floors_path: &Path,
    repo_root: &Path,
    base_ref: Option<&OsStr>,
) -> Result<Option<CoverageFloors>, GateError>
{
    let requested_floors = slash_path(floors_path);
    if requested_floors != DEFAULT_FLOORS {
        return Err(coverage_error(format!(
            "coverage check must use canonical floors path '{DEFAULT_FLOORS}'"
        )));
    }

    let requested = requested_base_ref(base_ref);
    let Some(commit) = resolve_base_commit(repo_root, &requested)?
    else {
        return Ok(None);
    };
    let base_has_floors =
        base_tree_contains_floors(repo_root, &commit).map(|value| value.into().0)?;
    if !base_has_floors {
        return Ok(None);
    }
    let text = read_base_floors(repo_root, &commit, &requested)?;
    parse_floors_text(DEFAULT_FLOORS, &text, true).map(Some)
}

/// Compute semantic failures from already parsed maps.
fn check_maps(
    measured: &BTreeMap<ProductionFile, MeasuredCoverage>,
    current: &CoverageFloors,
    base: Option<&CoverageFloors>,
) -> Vec<CoverageFailure>
{
    let mut failures = Vec::new();
    for (file, row) in measured {
        if !current.files.contains_key(file) {
            failures.push(CoverageFailure::MissingFloor {
                file: file.clone(),
                measured: row.percent,
            });
        }
    }
    for (file, floor) in &current.files {
        if !measured.contains_key(file) {
            failures.push(CoverageFailure::StaleFloor {
                file: file.clone(),
                floor: *floor,
            });
        }
    }
    for (file, row) in measured {
        if let Some(floor) = current.files.get(file)
            && row.percent < *floor
        {
            failures.push(CoverageFailure::Regression {
                file: file.clone(),
                measured: row.percent,
                floor: *floor,
            });
        }
    }
    if let Some(base_policy) = base {
        append_base_failures(measured, current, base_policy, &mut failures);
    }
    failures
}

/// Append current/base monotonicity failures.
fn append_base_failures(
    measured: &BTreeMap<ProductionFile, MeasuredCoverage>,
    current: &CoverageFloors,
    base: &CoverageFloors,
    failures: &mut Vec<CoverageFailure>,
)
{
    let target_decreased = current.target_percent < base.target_percent;
    for (file, base_floor) in &base.files {
        if let Some(current_floor) = current.files.get(file) {
            let minimum = if target_decreased && *base_floor > current.target_percent {
                current.target_percent
            }
            else {
                *base_floor
            };
            if *current_floor < minimum {
                if minimum == *base_floor {
                    failures.push(CoverageFailure::FloorDecreased {
                        file: file.clone(),
                        current: *current_floor,
                        base: *base_floor,
                    });
                }
                else {
                    failures.push(CoverageFailure::TargetClampDecreased {
                        file: file.clone(),
                        current: *current_floor,
                        allowed: minimum,
                        old_target: base.target_percent,
                        new_target: current.target_percent,
                    });
                }
            }
        }
    }
    for file in base.files.keys() {
        if !current.files.contains_key(file) && measured.contains_key(file) {
            failures.push(CoverageFailure::FloorDisappeared { file: file.clone() });
        }
    }
    for (file, floor) in &current.files {
        if base.files.contains_key(file) {
            continue;
        }
        let exempt = current.exemptions.contains_key(file);
        let measured_percent = measured.get(file).map(|row| row.percent);
        if exempt && measured_percent.is_none() {
            continue;
        }
        let expected = if exempt {
            match measured_percent {
                | Some(percent) => percent.min(current.target_percent),
                | None => current.target_percent,
            }
        }
        else {
            current.target_percent
        };
        if *floor != expected {
            if exempt {
                failures.push(CoverageFailure::ExemptNewFloorWrongStart {
                    file: file.clone(),
                    expected,
                    got: *floor,
                });
            }
            else {
                failures.push(CoverageFailure::NewFloorWrongStart {
                    file: file.clone(),
                    expected,
                    got: *floor,
                });
            }
        }
    }
}

/// Compute ratchet rows and counts from parsed maps.
fn ratchet_maps(
    measured: &BTreeMap<ProductionFile, MeasuredCoverage>,
    floors: &CoverageFloors,
) -> Result<RatchetReport, GateError>
{
    let target = floors.target_percent;
    let mut rendered_rows = BTreeMap::new();
    let mut raised = RatchetCounterCount(0);
    let mut added = RatchetCounterCount(0);
    let mut unchanged = RatchetCounterCount(0);
    let mut stale = RatchetCounterCount(0);

    for (file, row) in measured {
        match floors.files.get(file) {
            | Some(existing) => {
                let target_cap = row.percent.min(target);
                let next_floor = (*existing).max(target_cap);
                if next_floor > *existing {
                    increment_counter(&mut raised)?;
                }
                else {
                    increment_counter(&mut unchanged)?;
                }
                rendered_rows.insert(file.clone(), next_floor);
            },
            | None => {
                increment_counter(&mut added)?;
                rendered_rows.insert(file.clone(), target);
            },
        }
    }
    for (file, floor) in &floors.files {
        if !measured.contains_key(file) {
            increment_counter(&mut stale)?;
            rendered_rows.insert(file.clone(), *floor);
        }
    }

    let mut active_exemptions = BTreeMap::new();
    for (file, reason) in &floors.exemptions {
        if let Some(floor) = rendered_rows.get(file)
            && *floor < target
        {
            active_exemptions.insert(file.clone(), reason.clone());
        }
    }
    let toml = render_floors(target, &rendered_rows, &active_exemptions);
    Ok(RatchetReport {
        toml,
        raised: raised.0,
        added: added.0,
        unchanged: unchanged.0,
        stale: stale.0,
    })
}

/// Checked increment for ratchet counters.
fn increment_counter(counter: &mut RatchetCounterCount) -> Result<(), GateError>
{
    counter.0 = counter
        .0
        .checked_add(1)
        .ok_or_else(|| coverage_error("coverage ratchet counter overflow"))?;
    Ok(())
}

/// Validate the llvm-cov exporter identity fields.
fn validate_summary_identity(root: &Map<String, Value>) -> Result<(), GateError>
{
    let cargo = root.get("cargo_llvm_cov").and_then(Value::as_object);
    let cargo_version = cargo
        .and_then(|object| object.get("version"))
        .and_then(Value::as_str);
    if root.get("type").and_then(Value::as_str) != Some("llvm.coverage.json.export")
        || root.get("version").and_then(Value::as_str) != Some("3.1.0")
        || cargo_version != Some("0.8.7")
    {
        return Err(coverage_error(
            "coverage summary exporter identity must be llvm.coverage.json.export 3.1.0 from cargo-llvm-cov 0.8.7",
        ));
    }
    Ok(())
}

/// Return the summary `data[0].files` array with exact shape checks.
fn summary_files(root: &Map<String, Value>) -> Result<&Vec<Value>, GateError>
{
    let data = root.get("data").and_then(Value::as_array).ok_or_else(|| {
        coverage_error("coverage summary JSON must contain exactly one data object")
    })?;
    if data.len() != 1 {
        return Err(coverage_error(
            "coverage summary JSON must contain exactly one data object",
        ));
    }
    let Some(data_record) = data.first().and_then(Value::as_object)
    else {
        return Err(coverage_error(
            "coverage summary JSON data[0] must be a record",
        ));
    };
    data_record
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| coverage_error("coverage summary JSON is missing data[0].files"))
}

/// Parse one JSON file entry and return `None` for non-production files.
fn parse_summary_file(
    value: &Value,
    repo_root: &Path,
) -> Result<Option<MeasuredCoverage>, GateError>
{
    let Some(record) = value.as_object()
    else {
        return Err(coverage_error(
            "coverage summary file entry must be a record",
        ));
    };
    let filename = record
        .get("filename")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            coverage_error("coverage summary file entry is missing filename or summary.lines")
        })?;
    let lines = record
        .get("summary")
        .and_then(Value::as_object)
        .and_then(|summary| summary.get("lines"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            coverage_error("coverage summary file entry is missing filename or summary.lines")
        })?;

    let count_value = required_metric(lines, "count", filename)?;
    let covered_value = required_metric(lines, "covered", filename)?;
    let percent_value = required_metric(lines, "percent", filename)?;
    let count = parse_line_count(count_value, filename).map(|v| v.into().0)?;
    let covered = parse_line_count(covered_value, filename).map(|v| v.into().0)?;
    if covered > count {
        return Err(invalid_line_values(filename));
    }
    let declared = parse_declared_percent(percent_value, filename)?;
    let computed =
        Percent::from_counts(covered, count).map_err(|_error| invalid_line_values(filename))?;
    if declared != computed {
        return Err(coverage_error(format!(
            "coverage summary line percentage disagrees with counts for {filename}"
        )));
    }
    let Some(file) = ProductionFile::from_summary_filename(repo_root, filename)?
    else {
        return Ok(None);
    };
    Ok(Some(MeasuredCoverage {
        file,
        covered,
        count,
        percent: computed,
    }))
}

/// Return a required metric value or the missing-metrics diagnostic.
fn required_metric<'semantic, 'json, K, F>(
    lines: &'json Map<String, Value>,
    key: K,
    filename: F,
) -> Result<&'json Value, GateError>
where
    K: Into<KeyText<'semantic>>,
    F: Into<FilenameText<'semantic>>,
{
    let filename = filename.into().0;
    let key = key.into().0;
    let Some(value) = lines.get(key)
    else {
        return Err(coverage_error(format!(
            "coverage summary line metrics are missing for {filename}"
        )));
    };
    if value.is_null() {
        return Err(coverage_error(format!(
            "coverage summary line metrics are missing for {filename}"
        )));
    }
    Ok(value)
}

/// Parse a nonnegative integer line metric.
fn parse_line_count<'semantic, F>(
    value: &Value,
    filename: F,
) -> Result<impl Into<ParseLineCountCount>, GateError>
where
    F: Into<FilenameText<'semantic>>,
{
    let filename = filename.into().0;
    let Some(number) = value.as_number()
    else {
        return Err(invalid_line_types(filename));
    };
    if let Some(signed) = number.as_i64() {
        if signed < 0 {
            return Err(invalid_line_values(filename));
        }
        return u64::try_from(signed).map_err(|_error| invalid_line_values(filename));
    }
    if let Some(unsigned) = number.as_u64() {
        return Ok(unsigned);
    }
    Err(invalid_line_types(filename))
}

/// Parse an llvm-cov declared line percent at policy precision.
fn parse_declared_percent<'semantic, F>(
    value: &Value,
    filename: F,
) -> Result<Percent, GateError>
where
    F: Into<FilenameText<'semantic>>,
{
    let filename = filename.into().0;
    let Some(number) = value.as_number()
    else {
        return Err(invalid_line_types(filename));
    };
    if !number.is_f64() {
        return Err(invalid_line_types(filename));
    }
    Percent::parse_declared(&number.to_string()).map_err(|error| match error {
        | PercentParseError::Negative
        | PercentParseError::OutOfRange
        | PercentParseError::NonFinite => invalid_line_values(filename),
        | PercentParseError::Invalid
        | PercentParseError::HiddenPrecision
        | PercentParseError::Overflow => invalid_line_types(filename),
    })
}

/// Build the invalid line-metric type diagnostic.
fn invalid_line_types<'semantic, F>(filename: F) -> GateError
where
    F: Into<FilenameText<'semantic>>,
{
    let filename = filename.into().0;
    coverage_error(format!(
        "coverage summary line metrics have invalid types for {filename}"
    ))
}

/// Build the invalid line-metric value diagnostic.
fn invalid_line_values<'semantic, F>(filename: F) -> GateError
where
    F: Into<FilenameText<'semantic>>,
{
    let filename = filename.into().0;
    coverage_error(format!(
        "coverage summary line metrics are invalid for {filename}"
    ))
}

/// Return a stable JSON kind name for diagnostics.
fn json_kind(value: &Value) -> impl Into<JsonKindText<'static>>
{
    match *value {
        | Value::Null => "nothing",
        | Value::Bool(_) => "bool",
        | Value::Number(ref number) => {
            if number.is_f64() {
                "float"
            }
            else {
                "int"
            }
        },
        | Value::String(_) => "string",
        | Value::Array(_) => "table",
        | Value::Object(_) => "record",
    }
}

/// Raw parsed TOML policy subset.
#[derive(Debug, Default)]
struct RawFloorsToml
{
    /// Optional root target percent value.
    target_percent: Option<RawTomlValue>,
    /// Optional `[files]` table.
    files: Option<BTreeMap<String, RawTomlValue>>,
    /// Optional `[new_file_exemptions]` table.
    exemptions: Option<BTreeMap<String, RawTomlValue>>,
    /// Whether `new_file_exemptions` appeared as a scalar root value.
    exemptions_was_scalar: bool,
}

/// Raw TOML value in the retained subset.
#[derive(Clone, Debug, Eq, PartialEq)]
enum RawTomlValue
{
    /// Unquoted numeric-like token.
    Number(String),
    /// Quoted TOML basic string.
    String(String),
}

/// Current TOML section while parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TomlSection
{
    /// Root table.
    Root,
    /// `[files]` table.
    Files,
    /// `[new_file_exemptions]` table.
    Exemptions,
    /// Unused table retained for forwards compatibility.
    Other,
}

/// Parse the restricted floor-policy TOML subset.
fn parse_raw_floors_toml<'semantic, T>(text: T) -> Result<RawFloorsToml, TomlParseError>
where
    T: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let mut raw = RawFloorsToml::default();
    let mut section = TomlSection::Root;
    for source_line in text.lines() {
        let stripped = strip_toml_comment(source_line)?;
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(next_section) = parse_section_header(line)? {
            section = next_section;
            match section {
                | TomlSection::Files => {
                    if raw.files.is_none() {
                        raw.files = Some(BTreeMap::new());
                    }
                },
                | TomlSection::Exemptions => {
                    if raw.exemptions.is_none() {
                        raw.exemptions = Some(BTreeMap::new());
                    }
                },
                | TomlSection::Root | TomlSection::Other => {},
            }
            continue;
        }
        let (key, value) = parse_key_value(line)?;
        match section {
            | TomlSection::Root => match key.as_str() {
                | "target_percent" => raw.target_percent = Some(value),
                | "new_file_exemptions" => raw.exemptions_was_scalar = true,
                | _ => {},
            },
            | TomlSection::Files => {
                let table = raw.files.get_or_insert_with(BTreeMap::new);
                table.insert(key, value);
            },
            | TomlSection::Exemptions => {
                let table = raw.exemptions.get_or_insert_with(BTreeMap::new);
                table.insert(key, value);
            },
            | TomlSection::Other => {},
        }
    }
    Ok(raw)
}

/// Remove a TOML comment while preserving quoted `#` characters.
fn strip_toml_comment<'semantic, L>(line: L) -> Result<String, TomlParseError>
where
    L: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    let mut output = String::new();
    let mut in_string = false;
    let mut escaped = false;
    for character in line.chars() {
        if in_string {
            output.push(character);
            if escaped {
                escaped = false;
            }
            else if character == '\\' {
                escaped = true;
            }
            else if character == '"' {
                in_string = false;
            }
            continue;
        }
        if character == '#' {
            break;
        }
        if character == '"' {
            in_string = true;
        }
        output.push(character);
    }
    if escaped || in_string {
        return Err(TomlParseError::InvalidSyntax);
    }
    Ok(output)
}

/// Parse a section header when a line is a header.
fn parse_section_header<'semantic, L>(
    line: L,
) -> Result<Option<TomlSection>, TomlParseError>
where
    L: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    if !line.starts_with('[') {
        return Ok(None);
    }
    if !line.ends_with(']') {
        return Err(TomlParseError::InvalidSyntax);
    }
    let section = line
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
        .ok_or(TomlParseError::InvalidSyntax)?
        .trim();
    let parsed = match section {
        | "files" => TomlSection::Files,
        | "new_file_exemptions" => TomlSection::Exemptions,
        | "" => return Err(TomlParseError::InvalidSyntax),
        | _ => TomlSection::Other,
    };
    Ok(Some(parsed))
}

/// Parse one key-value line.
fn parse_key_value<'semantic, L>(
    line: L,
) -> Result<(String, RawTomlValue), TomlParseError>
where
    L: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    let Some((key_text, value_text)) = line.split_once('=')
    else {
        return Err(TomlParseError::InvalidSyntax);
    };
    let key = parse_toml_key(key_text.trim())?;
    let value = parse_toml_value(value_text.trim())?;
    Ok((key, value))
}

/// Parse a TOML key in the retained subset.
fn parse_toml_key<'semantic, T>(text: T) -> Result<String, TomlParseError>
where
    T: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    if text.starts_with('"') {
        return parse_basic_string(text);
    }
    if text.is_empty() {
        return Err(TomlParseError::InvalidSyntax);
    }
    Ok(text.to_owned())
}

/// Parse a TOML value in the retained subset.
fn parse_toml_value<'semantic, T>(text: T) -> Result<RawTomlValue, TomlParseError>
where
    T: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    if text.is_empty() {
        return Err(TomlParseError::InvalidSyntax);
    }
    if text.starts_with('"') {
        return parse_basic_string(text).map(RawTomlValue::String);
    }
    Ok(RawTomlValue::Number(text.to_owned()))
}

/// Parse a TOML basic string with the escapes emitted by the renderer.
fn parse_basic_string<'semantic, T>(text: T) -> Result<String, TomlParseError>
where
    T: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let mut chars = text.chars();
    if chars.next() != Some('"') {
        return Err(TomlParseError::InvalidSyntax);
    }
    let mut value = String::new();
    let mut escaped = false;
    let mut closed = false;
    for character in chars {
        if closed {
            if character.is_whitespace() {
                continue;
            }
            return Err(TomlParseError::InvalidSyntax);
        }
        if escaped {
            match character {
                | '"' => value.push('"'),
                | '\\' => value.push('\\'),
                | 'n' => value.push('\n'),
                | 'r' => value.push('\r'),
                | 't' => value.push('\t'),
                | _ => return Err(TomlParseError::InvalidSyntax),
            }
            escaped = false;
            continue;
        }
        match character {
            | '\\' => escaped = true,
            | '"' => closed = true,
            | _ => value.push(character),
        }
    }
    if escaped || !closed {
        return Err(TomlParseError::InvalidSyntax);
    }
    Ok(value)
}

/// TOML parse error category for the restricted parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TomlParseError
{
    /// Input is outside the retained TOML subset.
    InvalidSyntax,
}

impl fmt::Display for TomlParseError
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::InvalidSyntax => f.write_str("invalid TOML syntax"),
        }
    }
}

/// Parse and validate the policy target percent.
fn parse_target_percent<'semantic, S, H>(
    source_name: S,
    raw: Option<&RawTomlValue>,
    historical: H,
) -> Result<Percent, GateError>
where
    S: Into<SourceNameText<'semantic>>,
    H: Into<HistoricalFlag>,
{
    let historical = historical.into().0;
    let source_name = source_name.into().0;
    let Some(raw) = raw
    else {
        return Ok(DEFAULT_TARGET_PERCENT);
    };
    let RawTomlValue::Number(ref text) = *raw
    else {
        return Err(coverage_error(format!(
            "coverage floors TOML {source_name} target_percent must be between 0.00% and 100.00%"
        )));
    };
    let target = match Percent::parse_exact(text) {
        | Ok(percent) => percent,
        | Err(PercentParseError::NonFinite) => {
            return Err(coverage_error(format!(
                "coverage floors TOML {source_name} target_percent must equal 80.00%, got a non-finite value"
            )));
        },
        | Err(PercentParseError::HiddenPrecision) => {
            return Err(coverage_error(format!(
                "coverage floors TOML {source_name} target_percent must have at most two decimal places"
            )));
        },
        | Err(
            PercentParseError::Invalid
            | PercentParseError::Negative
            | PercentParseError::OutOfRange
            | PercentParseError::Overflow,
        ) => {
            return Err(coverage_error(format!(
                "coverage floors TOML {source_name} target_percent must be between 0.00% and 100.00%"
            )));
        },
    };
    if !historical && target != DEFAULT_TARGET_PERCENT {
        return Err(coverage_error(format!(
            "coverage floors TOML {source_name} target_percent must equal 80.00%, got {target}%"
        )));
    }
    Ok(target)
}

/// Parse and validate one file floor percent.
fn parse_floor_percent<'semantic, F>(
    file: F,
    raw: &RawTomlValue,
    target_percent: Percent,
) -> Result<Percent, GateError>
where
    F: Into<FileText<'semantic>>,
{
    let file = file.into().0;
    let RawTomlValue::Number(ref text) = *raw
    else {
        return Err(coverage_error(format!(
            "coverage floor for {file} must be numeric"
        )));
    };
    let floor = match Percent::parse_exact(text) {
        | Ok(percent) => percent,
        | Err(PercentParseError::HiddenPrecision) => {
            return Err(coverage_error(format!(
                "coverage floor for {file} must have at most two decimal places"
            )));
        },
        | Err(PercentParseError::Invalid) => {
            return Err(coverage_error(format!(
                "coverage floor for {file} must be numeric"
            )));
        },
        | Err(
            PercentParseError::NonFinite
            | PercentParseError::Negative
            | PercentParseError::OutOfRange
            | PercentParseError::Overflow,
        ) => {
            return Err(floor_between_error(file, target_percent));
        },
    };
    if floor > target_percent {
        return Err(floor_between_error(file, target_percent));
    }
    Ok(floor)
}

/// Build the stable floor range diagnostic.
fn floor_between_error<'semantic, F>(file: F, target_percent: Percent) -> GateError
where
    F: Into<FileText<'semantic>>,
{
    let file = file.into().0;
    coverage_error(format!(
        "coverage floor for {file} must be between 0.00% and {target_percent}%"
    ))
}

/// Validate new-file exemptions against the parsed floor map.
fn validate_exemptions(
    raw: Option<BTreeMap<String, RawTomlValue>>,
    files: &BTreeMap<ProductionFile, Percent>,
    target_percent: Percent,
) -> Result<BTreeMap<ProductionFile, String>, GateError>
{
    let mut exemptions = BTreeMap::new();
    let Some(raw) = raw
    else {
        return Ok(exemptions);
    };
    for (file, value) in raw {
        let production_file = ProductionFile::from_floor_key(&file)?;
        let RawTomlValue::String(reason) = value
        else {
            return Err(coverage_error(format!(
                "new-file coverage exemption for {file} must have a string reason"
            )));
        };
        let trimmed = reason.trim();
        if trimmed.is_empty() {
            return Err(coverage_error(format!(
                "new-file coverage exemption for {file} must have a non-empty reason"
            )));
        }
        let Some(floor) = files.get(&production_file)
        else {
            return Err(coverage_error(format!(
                "new-file coverage exemption for {file} has no matching [files] entry"
            )));
        };
        if *floor >= target_percent {
            return Err(coverage_error(format!(
                "new-file coverage exemption for {file} requires a floor below {target_percent}%"
            )));
        }
        exemptions.insert(production_file, trimmed.to_owned());
    }
    Ok(exemptions)
}

/// Select the requested base ref from an explicit override or environment.
fn requested_base_ref(base_ref: Option<&OsStr>) -> String
{
    let raw = match base_ref {
        | Some(value) => value.to_string_lossy().into_owned(),
        | None => match env::var_os("COVERAGE_BASE_REF") {
            | Some(value) => value.to_string_lossy().into_owned(),
            | None => String::new(),
        },
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_all_zero(trimmed).into().0 {
        return String::from("HEAD^");
    }
    trimmed.to_owned()
}

/// Return whether a ref token is all zeroes.
fn is_all_zero<'semantic, T>(text: T) -> impl Into<AllZeroFlag>
where
    T: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let mut saw_zero = false;
    for character in text.chars() {
        if character != '0' {
            return false;
        }
        saw_zero = true;
    }
    saw_zero
}

/// Resolve a base ref to a commit hash, with the root-commit `HEAD^` exception.
fn resolve_base_commit<'semantic, R>(
    repo_root: &Path,
    requested: R,
) -> Result<Option<String>, GateError>
where
    R: Into<RequestedText<'semantic>>,
{
    let requested = requested.into().0;
    let commit_spec = format!("{requested}^{{commit}}");
    let args = vec![
        OsString::from("rev-parse"),
        OsString::from("--verify"),
        OsString::from(commit_spec),
    ];
    let result = support::run_output(OsStr::new("git"), &args, Some(repo_root), true)?;
    if !result.success().into().0 {
        if requested == "HEAD^" {
            let root_commit = head_is_root_commit(repo_root).map(|value| value.into().0)?;
            if root_commit {
                return Ok(None);
            }
        }
        return Err(coverage_error(format!(
            "cannot resolve coverage base ref '{requested}'"
        )));
    }
    let stdout = result.stdout_lossy();
    let commit = stdout.as_ref().trim();
    if commit.is_empty() {
        return Err(coverage_error(format!(
            "cannot resolve coverage base ref '{requested}'"
        )));
    }
    Ok(Some(commit.to_owned()))
}

/// Return whether `HEAD` exists and has no parent.
fn head_is_root_commit(repo_root: &Path) -> Result<impl Into<HeadIsRootCommitFlag>, GateError>
{
    let args = vec![
        OsString::from("cat-file"),
        OsString::from("-p"),
        OsString::from("HEAD"),
    ];
    let result = support::run_output(OsStr::new("git"), &args, Some(repo_root), true)?;
    if !result.success().into().0 {
        return Ok(false);
    }
    let stdout = result.stdout_lossy();
    for line in stdout.as_ref().lines() {
        if line.starts_with("parent ") {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Return whether the base tree contains the floors path.
fn base_tree_contains_floors<'semantic, C>(
    repo_root: &Path,
    commit: C,
) -> Result<impl Into<BaseTreeContainsFloorsFlag>, GateError>
where
    C: Into<CommitText<'semantic>>,
{
    let commit = commit.into().0;
    let args = vec![
        OsString::from("ls-tree"),
        OsString::from("--name-only"),
        OsString::from(commit),
        OsString::from("--"),
        OsString::from(DEFAULT_FLOORS),
    ];
    let result = support::run_output(OsStr::new("git"), &args, Some(repo_root), true)?;
    if !result.success().into().0 {
        return Err(coverage_error(format!(
            "cannot inspect coverage floors in base ref '{commit}'"
        )));
    }
    let stdout = result.stdout_lossy();
    for line in stdout.as_ref().lines() {
        if line.trim() == DEFAULT_FLOORS {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Read the base floor-policy blob from Git.
fn read_base_floors<'semantic, C, R>(
    repo_root: &Path,
    commit: C,
    requested: R,
) -> Result<String, GateError>
where
    C: Into<CommitText<'semantic>>,
    R: Into<RequestedText<'semantic>>,
{
    let requested = requested.into().0;
    let commit = commit.into().0;
    let object = format!("{commit}:{DEFAULT_FLOORS}");
    let args = vec![OsString::from("show"), OsString::from(object)];
    let result = support::run_output(OsStr::new("git"), &args, Some(repo_root), true)?;
    if !result.success().into().0 {
        return Err(coverage_error(format!(
            "cannot read coverage floors from base ref '{requested}'"
        )));
    }
    let stdout = result.stdout_lossy();
    let stdout_text: &str = stdout.as_ref();
    Ok(String::from(stdout_text))
}

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::string::String;
    use alloc::vec::Vec;
    use std::ffi::OsStr;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    use super::LabelText;
    use super::check;
    use super::check_maps;
    use super::check_with_base_policy;
    use super::parse_floors_text;
    use super::parse_measured_text;
    use super::previous_floors;
    use super::ratchet;
    use super::ratchet_maps;
    use super::requested_base_ref;
    use crate::GateError;
    use crate::coverage::model::CoverageFloors;
    use crate::coverage::model::DEFAULT_TARGET_PERCENT;
    use crate::coverage::model::MeasuredCoverage;
    use crate::coverage::model::Percent;
    use crate::coverage::model::ProductionFile;

    /// Canonical fixture file A.
    const FILE_A: &str = "crates/demo/src/lib.rs";
    /// Canonical fixture file B.
    const FILE_B: &str = "crates/demo/src/parser.rs";
    /// Canonical fixture file C.
    const FILE_C: &str = "crates/demo/src/new.rs";
    /// Canonical stale fixture file.
    const FILE_STALE: &str = "crates/demo/src/stale.rs";
    /// No covered lines in a percentage-shaped fixture.
    const NO_COVERED_LINES: u64 = 0;
    /// Ten covered lines in a percentage-shaped fixture.
    const TEN_PERCENT_COVERED_LINES: u64 = 10;
    /// Forty-two covered lines in a percentage-shaped fixture.
    const FORTY_TWO_PERCENT_COVERED_LINES: u64 = 42;
    /// Seventy-five covered lines in a percentage-shaped fixture.
    const SEVENTY_FIVE_PERCENT_COVERED_LINES: u64 = 75;
    /// Seventy-eight covered lines in a percentage-shaped fixture.
    const SEVENTY_EIGHT_PERCENT_COVERED_LINES: u64 = 78;
    /// Eighty covered lines in a percentage-shaped fixture.
    const EIGHTY_PERCENT_COVERED_LINES: u64 = 80;
    /// Eighty-one covered lines in a percentage-shaped fixture.
    const EIGHTY_ONE_PERCENT_COVERED_LINES: u64 = 81;
    /// Ninety covered lines in a percentage-shaped fixture.
    const NINETY_PERCENT_COVERED_LINES: u64 = 90;
    /// Ninety-three covered lines in a percentage-shaped fixture.
    const NINETY_THREE_PERCENT_COVERED_LINES: u64 = 93;
    /// Ninety-five covered lines in a percentage-shaped fixture.
    const NINETY_FIVE_PERCENT_COVERED_LINES: u64 = 95;
    /// Total lines used by percentage-shaped fixtures.
    const PERCENT_TOTAL_LINES: u64 = 100;
    /// Covered lines in the high-precision rounding fixture.
    const ROUNDING_COVERED_LINES: u64 = 951;
    /// Total lines in the high-precision rounding fixture.
    const ROUNDING_TOTAL_LINES: u64 = 976;

    /// Clean current policy has no findings.
    #[test]
    fn clean_baseline_passes_without_findings()
    {
        let measured = measured_map(&[
            (FILE_A, NINETY_PERCENT_COVERED_LINES, PERCENT_TOTAL_LINES),
            (
                FILE_B,
                EIGHTY_ONE_PERCENT_COVERED_LINES,
                PERCENT_TOTAL_LINES,
            ),
        ]);
        let current = floors_map(&[(FILE_A, "80.00"), (FILE_B, "80.00")], NO_EXEMPTIONS);
        let failures = check_maps(&measured, &current, None);
        assert!(failures.is_empty(), "clean baseline should pass");
    }

    /// JSON shape and identity failures keep exact diagnostic fragments.
    #[test]
    fn json_identity_and_shape_failures_are_exact()
    {
        let wrong_identity = r#"{"type":"llvm.coverage.json.export","version":"9.9.9","cargo_llvm_cov":{"version":"0.8.7"},"data":[{"files":[]}]}"#;
        assert_error_contains(
            parse_measured_text("summary.json", wrong_identity, Path::new(".")),
            "coverage summary exporter identity must be llvm.coverage.json.export 3.1.0 from cargo-llvm-cov 0.8.7",
        );

        let multiple_data = r#"{"type":"llvm.coverage.json.export","version":"3.1.0","cargo_llvm_cov":{"version":"0.8.7"},"data":[{},{}]}"#;
        assert_error_contains(
            parse_measured_text("summary.json", multiple_data, Path::new(".")),
            "coverage summary JSON must contain exactly one data object",
        );

        let wrong_metric =
            coverage_document(&[(FILE_A, NO_COVERED_LINES, PERCENT_TOTAL_LINES, "100.0")]);
        assert_error_contains(
            parse_measured_text("summary.json", &wrong_metric, Path::new(".")),
            "coverage summary line percentage disagrees with counts for crates/demo/src/lib.rs",
        );
    }

    /// Absolute and relative spellings that normalize to one production path
    /// are rejected as duplicates.
    #[test]
    fn duplicate_normalized_summary_rows_fail()
    {
        let absolute = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), FILE_A);
        let document = coverage_document_with_names(&[
            (
                FILE_A,
                NINETY_PERCENT_COVERED_LINES,
                PERCENT_TOTAL_LINES,
                "90.0",
            ),
            (
                absolute.as_str(),
                NINETY_PERCENT_COVERED_LINES,
                PERCENT_TOTAL_LINES,
                "90.0",
            ),
        ]);
        assert_error_contains(
            parse_measured_text(
                "summary.json",
                &document,
                Path::new(env!("CARGO_MANIFEST_DIR")),
            ),
            "coverage summary contains duplicate normalized production files: crates/demo/src/lib.rs",
        );
    }

    /// Absolute paths inside the repo normalize to relative production keys.
    #[test]
    fn absolute_summary_paths_normalize_to_repo_relative()
    {
        let absolute = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), FILE_A);
        let document = coverage_document_with_names(&[(
            absolute.as_str(),
            NINETY_PERCENT_COVERED_LINES,
            PERCENT_TOTAL_LINES,
            "90.0",
        )]);
        let measured = parse_measured_text(
            "summary.json",
            &document,
            Path::new(env!("CARGO_MANIFEST_DIR")),
        )
        .expect("absolute in-root fixture should parse");
        assert!(
            measured.contains_key(
                &ProductionFile::from_floor_key(FILE_A).expect("fixture key is valid")
            ),
            "absolute summary path should normalize to repo-relative key",
        );
    }

    /// Production path boundaries reject escapes and outside absolute paths.
    #[test]
    fn production_path_boundaries_fail_closed()
    {
        let escaped = coverage_document_with_names(&[(
            "../crates/demo/src/lib.rs",
            TEN_PERCENT_COVERED_LINES,
            PERCENT_TOTAL_LINES,
            "10.0",
        )]);
        assert_error_contains(
            parse_measured_text("escaped.json", &escaped, Path::new(".")),
            "coverage filename escapes repository root",
        );
        let outside = coverage_document_with_names(&[(
            "/crates/demo/src/lib.rs",
            TEN_PERCENT_COVERED_LINES,
            PERCENT_TOTAL_LINES,
            "10.0",
        )]);
        assert_error_contains(
            parse_measured_text("outside.json", &outside, Path::new(".")),
            "coverage filename is outside repository root",
        );
    }

    /// Check report covers missing, stale, regression, base, clamp,
    /// disappearance, new-file, and exempt-new-file failures in stable wording.
    #[test]
    fn check_report_covers_policy_failure_families()
    {
        let measured = measured_map(&[
            (
                FILE_A,
                SEVENTY_EIGHT_PERCENT_COVERED_LINES,
                PERCENT_TOTAL_LINES,
            ),
            (FILE_C, FORTY_TWO_PERCENT_COVERED_LINES, PERCENT_TOTAL_LINES),
        ]);
        let current = floors_map(
            &[(FILE_A, "79.00"), (FILE_C, "41.00"), (FILE_STALE, "80.00")],
            &[(FILE_C, "Temporary generated adapter")],
        );
        let base = floors_with_target(
            "90.00",
            &[(FILE_A, "90.00"), (FILE_B, "75.00")],
            NO_EXEMPTIONS,
        );
        let failures = check_maps(&measured, &current, Some(&base));
        let text = failures
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(
            &text,
            @r###"
stale floor for crates/demo/src/stale.rs: floor 80.00% but file is absent from measured coverage
coverage regression for crates/demo/src/lib.rs: measured 78.00% below floor 79.00%
coverage floor decreased past policy target for crates/demo/src/lib.rs: current 79.00% below allowed 80.00% after target changed from 90.00% to 80.00%
exempt new coverage floor for crates/demo/src/new.rs must start at measured baseline 42.00%, got 41.00%
"###
        );
        assert!(
            text.contains("stale floor for crates/demo/src/stale.rs: floor 80.00% but file is absent from measured coverage"),
            "stale floor diagnostic should be present",
        );
        assert!(
            text.contains(
                "coverage regression for crates/demo/src/lib.rs: measured 78.00% below floor 79.00%"
            ),
            "regression diagnostic should be present",
        );
        assert!(
            text.contains("coverage floor decreased past policy target for crates/demo/src/lib.rs: current 79.00% below allowed 80.00% after target changed from 90.00% to 80.00%"),
            "base decrease diagnostic should be present",
        );
        assert!(
            text.contains("exempt new coverage floor for crates/demo/src/new.rs must start at measured baseline 42.00%, got 41.00%"),
            "exempt seed diagnostic should be present",
        );

        let missing_measured = measured_map(&[
            (FILE_A, NINETY_PERCENT_COVERED_LINES, PERCENT_TOTAL_LINES),
            (FILE_C, NINETY_PERCENT_COVERED_LINES, PERCENT_TOTAL_LINES),
        ]);
        let missing_current = floors_map(&[(FILE_A, "80.00")], NO_EXEMPTIONS);
        let missing_text = check_maps(&missing_measured, &missing_current, None)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            missing_text.contains("missing floor for crates/demo/src/new.rs: measured 90.00%"),
            "missing floor diagnostic should be present",
        );

        let disappearing_current = floors_map(&[(FILE_C, "80.00")], NO_EXEMPTIONS);
        let disappearing_base = floors_map(&[(FILE_A, "80.00")], NO_EXEMPTIONS);
        let disappearing_text = check_maps(
            &measured_map(&[
                (FILE_A, NINETY_PERCENT_COVERED_LINES, PERCENT_TOTAL_LINES),
                (FILE_C, NINETY_PERCENT_COVERED_LINES, PERCENT_TOTAL_LINES),
            ]),
            &disappearing_current,
            Some(&disappearing_base),
        )
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
        assert!(
            disappearing_text.contains(
                "coverage floor disappeared for existing production file crates/demo/src/lib.rs"
            ),
            "disappearing floor diagnostic should be present",
        );

        let ordinary_new_current =
            floors_map(&[(FILE_A, "80.00"), (FILE_C, "42.00")], NO_EXEMPTIONS);
        let ordinary_new_base = floors_map(&[(FILE_A, "80.00")], NO_EXEMPTIONS);
        let ordinary_new_text = check_maps(
            &measured_map(&[
                (FILE_A, NINETY_PERCENT_COVERED_LINES, PERCENT_TOTAL_LINES),
                (FILE_C, FORTY_TWO_PERCENT_COVERED_LINES, PERCENT_TOTAL_LINES),
            ]),
            &ordinary_new_current,
            Some(&ordinary_new_base),
        )
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
        assert!(
            ordinary_new_text.contains(
                "new coverage floor for crates/demo/src/new.rs must start at 80.00%, got 42.00%"
            ),
            "ordinary new-file seed diagnostic should be present",
        );

        let decreased_current = floors_map(&[(FILE_A, "70.00")], NO_EXEMPTIONS);
        let decreased_base = floors_map(&[(FILE_A, "75.00")], NO_EXEMPTIONS);
        let decreased_text = check_maps(
            &measured_map(&[(FILE_A, NINETY_PERCENT_COVERED_LINES, PERCENT_TOTAL_LINES)]),
            &decreased_current,
            Some(&decreased_base),
        )
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
        assert!(
            decreased_text.contains("coverage floor decreased for crates/demo/src/lib.rs: current 70.00% below base 75.00%"),
            "plain base decrease diagnostic should be present",
        );
    }

    /// Ratchet caps raises, retains stale rows, keeps higher floors, and
    /// renders a deterministic snapshot.
    #[test]
    fn ratchet_report_caps_and_renders_stably()
    {
        let measured = measured_map(&[
            (
                FILE_B,
                NINETY_THREE_PERCENT_COVERED_LINES,
                PERCENT_TOTAL_LINES,
            ),
            (
                FILE_A,
                NINETY_FIVE_PERCENT_COVERED_LINES,
                PERCENT_TOTAL_LINES,
            ),
        ]);
        let floors = floors_map(&[(FILE_STALE, "77.00"), (FILE_A, "80.03")], NO_EXEMPTIONS);
        let report = ratchet_maps(&measured, &floors).expect("ratchet fixture should compute");
        assert_eq!(0, report.raised, "higher existing floor should not fall");
        assert_eq!(1, report.added, "one measured file should be added");
        assert_eq!(
            1, report.unchanged,
            "one higher existing floor should remain"
        );
        assert_eq!(1, report.stale, "one stale row should be retained");
        insta::assert_snapshot!(
            &report.toml,
            @r###"
# Per-file line-coverage ratchet. Floors only fall when the policy target falls.

target_percent = 80.00

[files]
"crates/demo/src/lib.rs" = 80.03
"crates/demo/src/parser.rs" = 80.00
"crates/demo/src/stale.rs" = 77.00
"###
        );
        let expected = [
            "# Per-file line-coverage ratchet. Floors only fall when the policy target falls.",
            "",
            "target_percent = 80.00",
            "",
            "[files]",
            "\"crates/demo/src/lib.rs\" = 80.03",
            "\"crates/demo/src/parser.rs\" = 80.00",
            "\"crates/demo/src/stale.rs\" = 77.00",
        ]
        .join("\n");
        assert_eq!(report.toml, expected, "ratchet TOML should be stable");
    }

    /// TOML policy shape rejects precision, range, paths, and exemption errors.
    #[test]
    fn floor_policy_shape_is_strict()
    {
        assert_error_contains(
            parse_floors_text("floors.toml", "target_percent = 80.0\n", false),
            "is missing [files]",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                "target_percent = 80.0\n[files]\n\"crates/demo/src/lib.rs\" = 80.01\n",
                false,
            ),
            "must be between 0.00% and 80.00%",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                "target_percent = 80.0\n[files]\n\"crates/demo/src/lib.rs\" = 78.350009\n",
                false,
            ),
            "must have at most two decimal places",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                "target_percent = 80.0\n[files]\n\"crates/demo/src/../lib.rs\" = 78.35\n",
                false,
            ),
            "coverage floor path must be a canonical production Rust file under crates/",
        );
    }

    /// New-file exemptions require a listed below-target floor and nonempty
    /// string reason.
    #[test]
    fn new_file_exemptions_are_strict()
    {
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                "target_percent = 80.0\n[files]\n\"crates/demo/src/new.rs\" = 42.00\n[new_file_exemptions]\n\"crates/demo/src/new.rs\" = \"   \"\n",
                false,
            ),
            "must have a non-empty reason",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                "target_percent = 80.0\n[files]\n\"crates/demo/src/new.rs\" = 80.00\n[new_file_exemptions]\n\"crates/demo/src/new.rs\" = \"Temporary\"\n",
                false,
            ),
            "requires a floor below 80.00%",
        );
    }

    /// llvm-cov's full-precision decimal is valid when its floor matches the
    /// count-derived policy percentage.
    #[test]
    fn line_percentage_accepts_matching_full_precision()
    {
        let document = coverage_document(&[(
            FILE_A,
            ROUNDING_COVERED_LINES,
            ROUNDING_TOTAL_LINES,
            "97.43852459016394",
        )]);
        let measured = parse_measured_text("summary.json", &document, Path::new("."));

        assert!(
            measured.is_ok(),
            "matching llvm-cov percentage should parse"
        );
    }

    /// Declared line percentages must equal the count-derived value exactly.
    #[test]
    fn line_percentage_must_match_counts_exactly()
    {
        let document = coverage_document(&[(FILE_A, 1, 3, "33.34")]);
        assert_error_contains(
            parse_measured_text("summary.json", &document, Path::new(".")),
            "coverage summary line percentage disagrees with counts for crates/demo/src/lib.rs",
        );
    }

    /// Requested base refs normalize empty and all-zero values to HEAD^.
    #[test]
    fn base_ref_policy_distinguishes_zero_head_root_and_missing_blob()
    {
        assert_eq!(
            "HEAD^",
            requested_base_ref(None),
            "missing base ref should default to HEAD^",
        );
        assert_eq!(
            "HEAD^",
            requested_base_ref(Some(OsStr::new("0000000000000000000000000000000000000000"))),
            "all-zero base ref should default to HEAD^",
        );
        assert_eq!(
            "main",
            requested_base_ref(Some(OsStr::new("main"))),
            "explicit base ref should be retained",
        );
    }

    /// Public file APIs preserve clean ratchet behavior and deterministic
    /// failing findings.
    #[test]
    fn public_file_apis_ratcheting_and_findings_are_deterministic()
    {
        let temp = FixtureDir::new("policy-file-api");
        let summary_path = temp.path().join("summary.json");
        let floors_path = temp.path().join("floors.toml");
        write_text(
            &summary_path,
            &coverage_document(&[
                (FILE_B, PERCENT_TOTAL_LINES, PERCENT_TOTAL_LINES, "100.0"),
                (
                    FILE_A,
                    SEVENTY_FIVE_PERCENT_COVERED_LINES,
                    PERCENT_TOTAL_LINES,
                    "75.0",
                ),
            ]),
        );
        write_text(
            &floors_path,
            &floors_toml(&[(FILE_A, "70.00"), (FILE_STALE, "77.00")], NO_EXEMPTIONS),
        );

        let report =
            ratchet(&summary_path, &floors_path, Path::new(".")).expect("ratchet should pass");
        assert_eq!(1, report.raised, "measured coverage should raise FILE_A");
        assert_eq!(1, report.added, "FILE_B should be added at the target");
        assert_eq!(
            0, report.unchanged,
            "no measured current row should stay flat"
        );
        assert_eq!(1, report.stale, "stale rows should be retained");
        assert_eq!(
            fs::read_to_string(&floors_path).expect("ratchet output should be readable"),
            report.toml,
            "ratchet should write exactly the report it returns",
        );
        insta::assert_snapshot!(
            &report.toml,
            @r###"
# Per-file line-coverage ratchet. Floors only fall when the policy target falls.

target_percent = 80.00

[files]
"crates/demo/src/lib.rs" = 75.00
"crates/demo/src/parser.rs" = 80.00
"crates/demo/src/stale.rs" = 77.00
"###
        );

        let failing_floors_path = temp.path().join("failing-floors.toml");
        write_text(
            &failing_floors_path,
            &floors_toml(&[(FILE_A, "79.00"), (FILE_STALE, "80.00")], NO_EXEMPTIONS),
        );
        let base = floors_with_target("90.00", &[(FILE_A, "90.00")], NO_EXEMPTIONS);
        let findings = check_with_base_policy(
            &summary_path,
            &failing_floors_path,
            Path::new("."),
            Some(&base),
        )
        .expect("failing fixture should return findings");
        let projected = findings
            .iter()
            .map(|finding| (finding.path.as_str(), finding.detail.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(
            projected,
            vec![
                (
                    FILE_B,
                    "missing floor for crates/demo/src/parser.rs: measured 100.00%",
                ),
                (
                    FILE_STALE,
                    "stale floor for crates/demo/src/stale.rs: floor 80.00% but file is absent from measured coverage",
                ),
                (
                    FILE_A,
                    "coverage regression for crates/demo/src/lib.rs: measured 75.00% below floor 79.00%",
                ),
                (
                    FILE_A,
                    "coverage floor decreased past policy target for crates/demo/src/lib.rs: current 79.00% below allowed 80.00% after target changed from 90.00% to 80.00%",
                ),
            ],
            "findings should keep deterministic policy order and public fields",
        );

        let decreased_path = temp.path().join("decreased-floors.toml");
        write_text(
            &decreased_path,
            &floors_toml(&[(FILE_A, "70.00"), (FILE_B, "80.00")], NO_EXEMPTIONS),
        );
        let decreased = check_with_base_policy(
            &summary_path,
            &decreased_path,
            Path::new("."),
            Some(&floors_map(&[(FILE_A, "75.00")], NO_EXEMPTIONS)),
        )
        .expect("floor-decrease fixture should return findings");
        assert!(
            decreased.iter().any(|finding| {
                finding.detail
                    == "coverage floor decreased for crates/demo/src/lib.rs: current 70.00% below base 75.00%"
            }),
            "plain floor decreases should convert into public findings",
        );

        let disappeared_path = temp.path().join("disappeared-floors.toml");
        write_text(
            &disappeared_path,
            &floors_toml(&[(FILE_B, "80.00")], NO_EXEMPTIONS),
        );
        let disappeared = check_with_base_policy(
            &summary_path,
            &disappeared_path,
            Path::new("."),
            Some(&floors_map(&[(FILE_A, "75.00")], NO_EXEMPTIONS)),
        )
        .expect("disappearing-floor fixture should return findings");
        assert!(
            disappeared.iter().any(|finding| {
                finding.detail
                    == "coverage floor disappeared for existing production file crates/demo/src/lib.rs"
            }),
            "disappearing floors should convert into public findings",
        );

        let new_summary_path = temp.path().join("new-summary.json");
        write_text(
            &new_summary_path,
            &coverage_document(&[
                (
                    FILE_A,
                    EIGHTY_PERCENT_COVERED_LINES,
                    PERCENT_TOTAL_LINES,
                    "80.0",
                ),
                (
                    FILE_C,
                    FORTY_TWO_PERCENT_COVERED_LINES,
                    PERCENT_TOTAL_LINES,
                    "42.0",
                ),
            ]),
        );
        let new_floor_path = temp.path().join("new-floors.toml");
        write_text(
            &new_floor_path,
            &floors_toml(&[(FILE_A, "80.00"), (FILE_C, "42.00")], NO_EXEMPTIONS),
        );
        let new_findings = check_with_base_policy(
            &new_summary_path,
            &new_floor_path,
            Path::new("."),
            Some(&floors_map(&[(FILE_A, "80.00")], NO_EXEMPTIONS)),
        )
        .expect("new-floor fixture should return findings");
        assert!(
            new_findings.iter().any(|finding| {
                finding.detail
                    == "new coverage floor for crates/demo/src/new.rs must start at 80.00%, got 42.00%"
            }),
            "ordinary new-floor findings should use the public finding fields",
        );
    }

    /// The top-level check path fails before Git or summary reads when the
    /// floor policy path is not canonical.
    #[test]
    fn check_rejects_noncanonical_floor_path_before_git()
    {
        assert_error_contains(
            check(
                Path::new("missing-summary.json"),
                Path::new("tmp/floors.toml"),
                Path::new("."),
            ),
            "coverage check must use canonical floors path 'coverage/floors.toml'",
        );
    }

    /// Summary parsing rejects malformed shapes and metric values while
    /// filtering out non-production files.
    #[test]
    fn measured_summary_parser_rejects_malformed_metrics_and_filters_nonproduction()
    {
        assert_error_contains(
            parse_measured_text("bad.json", "{", Path::new(".")),
            "failed to read coverage summary JSON bad.json",
        );
        assert_error_contains(
            parse_measured_text("bad.json", "[]", Path::new(".")),
            "coverage summary JSON must be a record from cargo-llvm-cov, got table",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document_from_entries(&["[]"]),
                Path::new("."),
            ),
            "coverage summary file entry must be a record",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document_from_entries(&[
                    r#"{"summary":{"lines":{"count":1,"covered":1,"percent":100.0}}}"#,
                ]),
                Path::new("."),
            ),
            "coverage summary file entry is missing filename or summary.lines",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document_from_entries(&[&format!("{{\"filename\":\"{FILE_A}\"}}")]),
                Path::new("."),
            ),
            "coverage summary file entry is missing filename or summary.lines",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document_from_entries(&[&format!(
                    "{{\"filename\":\"{FILE_A}\",\"summary\":{{\"lines\":{{\"count\":10,\"covered\":null,\"percent\":0.0}}}}}}"
                )]),
                Path::new("."),
            ),
            "coverage summary line metrics are missing for crates/demo/src/lib.rs",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document_from_entries(&[&format!(
                    "{{\"filename\":\"{FILE_A}\",\"summary\":{{\"lines\":{{\"count\":\"10\",\"covered\":1,\"percent\":10.0}}}}}}"
                )]),
                Path::new("."),
            ),
            "coverage summary line metrics have invalid types for crates/demo/src/lib.rs",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document_from_entries(&[&format!(
                    "{{\"filename\":\"{FILE_A}\",\"summary\":{{\"lines\":{{\"count\":-1,\"covered\":0,\"percent\":0.0}}}}}}"
                )]),
                Path::new("."),
            ),
            "coverage summary line metrics are invalid for crates/demo/src/lib.rs",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document(&[(FILE_A, 2, 1, "200.0")]),
                Path::new("."),
            ),
            "coverage summary line metrics are invalid for crates/demo/src/lib.rs",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document(&[(FILE_A, 1, 1, "100")]),
                Path::new("."),
            ),
            "coverage summary line metrics have invalid types for crates/demo/src/lib.rs",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document(&[(FILE_A, 0, 1, "-1.0")]),
                Path::new("."),
            ),
            "coverage summary line metrics are invalid for crates/demo/src/lib.rs",
        );
        assert_error_contains(
            parse_measured_text(
                "bad.json",
                &coverage_document(&[("crates/demo/tests/helper.rs", 1, 1, "100.0")]),
                Path::new("."),
            ),
            "coverage summary contains no measured production files under crates/",
        );

        let huge_count = parse_measured_text(
            "summary.json",
            &coverage_document_from_entries(&[&format!(
                "{{\"filename\":\"{FILE_A}\",\"summary\":{{\"lines\":{{\"count\":18446744073709551615,\"covered\":0,\"percent\":0.0}}}}}}"
            )]),
            Path::new("."),
        )
        .expect("unsigned u64 line counts should parse");
        assert_eq!(
            u64::MAX,
            huge_count
                .get(&ProductionFile::from_floor_key(FILE_A).expect("fixture key is valid"))
                .expect("measured file should be present")
                .count,
            "u64 line counts should remain exact",
        );

        let mixed = parse_measured_text(
            "summary.json",
            &coverage_document(&[
                ("crates/demo/tests/helper.rs", 1, 1, "100.0"),
                (
                    FILE_A,
                    NINETY_PERCENT_COVERED_LINES,
                    PERCENT_TOTAL_LINES,
                    "90.0",
                ),
            ]),
            Path::new("."),
        )
        .expect("mixed production/non-production summary should parse");
        assert_eq!(1, mixed.len(), "non-production rows should be ignored");
    }

    /// Floor TOML parsing rejects malformed values and preserves escaped
    /// exemption reasons through the typed policy model.
    #[test]
    fn floor_policy_parser_rejects_malformed_values_and_preserves_escaped_exemptions()
    {
        assert_error_contains(
            parse_floors_text("floors.toml", "[files\n", false),
            "failed to read coverage floors TOML floors.toml: invalid TOML syntax",
        );
        assert_error_contains(
            parse_floors_text("floors.toml", "[]\n", false),
            "failed to read coverage floors TOML floors.toml: invalid TOML syntax",
        );
        assert_error_contains(
            parse_floors_text("floors.toml", "[files]\n", false),
            "coverage floors TOML floors.toml has no [files] entries",
        );
        assert_error_contains(
            parse_floors_text("floors.toml", "[files]\n = 80.00\n", false),
            "failed to read coverage floors TOML floors.toml: invalid TOML syntax",
        );
        assert_error_contains(
            parse_floors_text("floors.toml", &format!("[files]\n\"{FILE_A}\" = \n"), false),
            "failed to read coverage floors TOML floors.toml: invalid TOML syntax",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("[files]\n\"{FILE_A}\" = \"80.00\"x\n"),
                false,
            ),
            "failed to read coverage floors TOML floors.toml: invalid TOML syntax",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("target_percent = \"80.00\"\n[files]\n\"{FILE_A}\" = 80.00\n"),
                false,
            ),
            "target_percent must be between 0.00% and 100.00%",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("target_percent = inf\n[files]\n\"{FILE_A}\" = 80.00\n"),
                false,
            ),
            "target_percent must equal 80.00%, got a non-finite value",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("target_percent = 80.001\n[files]\n\"{FILE_A}\" = 80.00\n"),
                false,
            ),
            "target_percent must have at most two decimal places",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("target_percent = 101.00\n[files]\n\"{FILE_A}\" = 80.00\n"),
                false,
            ),
            "target_percent must be between 0.00% and 100.00%",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("target_percent = 75.00\n[files]\n\"{FILE_A}\" = 75.00\n"),
                false,
            ),
            "target_percent must equal 80.00%, got 75.00%",
        );
        let historical = parse_floors_text(
            "floors.toml",
            &format!("target_percent = 75.00\n[files]\n\"{FILE_A}\" = 75.00\n"),
            true,
        )
        .expect("historical policies may retain older targets");
        assert_eq!(
            "75.00",
            historical.target_percent.to_string(),
            "historical target should be preserved",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("[files]\n\"{FILE_A}\" = \"80.00\"\n"),
                false,
            ),
            "coverage floor for crates/demo/src/lib.rs must be numeric",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("[files]\n\"{FILE_A}\" = tomato\n"),
                false,
            ),
            "coverage floor for crates/demo/src/lib.rs must be numeric",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("[files]\n\"{FILE_A}\" = inf\n"),
                false,
            ),
            "coverage floor for crates/demo/src/lib.rs must be between 0.00% and 80.00%",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!("new_file_exemptions = \"scalar\"\n[files]\n\"{FILE_A}\" = 80.00\n"),
                false,
            ),
            "[new_file_exemptions] must be a table",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!(
                    "[files]\n\"{FILE_A}\" = 42.00\n[new_file_exemptions]\n\"{FILE_A}\" = 10\n"
                ),
                false,
            ),
            "new-file coverage exemption for crates/demo/src/lib.rs must have a string reason",
        );
        assert_error_contains(
            parse_floors_text(
                "floors.toml",
                &format!(
                    "[files]\n\"{FILE_A}\" = 42.00\n[new_file_exemptions]\n\"{FILE_B}\" = \"temporary\"\n"
                ),
                false,
            ),
            "new-file coverage exemption for crates/demo/src/parser.rs has no matching [files] entry",
        );

        let escaped = parse_floors_text(
            "floors.toml",
            &format!(
                "[ignored]\nignored = value\n[files]\n\"{FILE_C}\" = 42.00\n[new_file_exemptions]\n\"{FILE_C}\" = \"needs \\\"quote\\\" and \\\\ path\\nnext\\rtab\\tend\"\n"
            ),
            false,
        )
        .expect("escaped exemption reason should parse");
        let reason = escaped
            .exemptions
            .get(&ProductionFile::from_floor_key(FILE_C).expect("fixture key is valid"))
            .expect("exemption should be retained");
        assert_eq!(
            "needs \"quote\" and \\ path\nnext\rtab\tend", reason,
            "basic-string escapes should round-trip into the exemption reason",
        );
    }

    /// Previous-floor resolution distinguishes root commits, missing blobs,
    /// readable base blobs, and unresolved refs using real sanitized Git calls.
    #[test]
    fn previous_floor_resolution_uses_git_base_policy_edges()
    {
        let temp = FixtureDir::new("policy-git");
        let repo = temp.path();
        run_git(repo, ["init"]);
        run_git(repo, ["config", "user.name", "Gandr Test"]);
        run_git(repo, ["config", "user.email", "gandr-test@example.invalid"]);

        write_text(&repo.join("README.md"), "root\n");
        commit_all(repo, "root without floors");
        let root_base =
            previous_floors(Path::new(super::DEFAULT_FLOORS), repo, Some(OsStr::new("")))
                .expect("root HEAD^ should resolve to no base policy");
        assert!(
            root_base.is_none(),
            "root commit should have no base policy"
        );

        write_text(
            &repo.join(super::DEFAULT_FLOORS),
            &floors_toml_with_target("70.00", &[(FILE_A, "70.00")], NO_EXEMPTIONS),
        );
        commit_all(repo, "add floors");
        let missing_blob = previous_floors(
            Path::new(super::DEFAULT_FLOORS),
            repo,
            Some(OsStr::new("HEAD^")),
        )
        .expect("parent without floors should be a non-error");
        assert!(
            missing_blob.is_none(),
            "a base tree without floors.toml should mean no historical policy",
        );

        write_text(
            &repo.join(super::DEFAULT_FLOORS),
            &floors_toml_with_target("80.00", &[(FILE_A, "80.00")], NO_EXEMPTIONS),
        );
        commit_all(repo, "raise floors");
        let base = previous_floors(
            Path::new(super::DEFAULT_FLOORS),
            repo,
            Some(OsStr::new("HEAD^")),
        )
        .expect("parent floors should load")
        .expect("parent policy should exist");
        assert_eq!(
            "70.00",
            base.target_percent.to_string(),
            "historical floor parser should preserve the parent target",
        );
        assert_eq!(
            "70.00",
            base.files
                .get(&ProductionFile::from_floor_key(FILE_A).expect("fixture key is valid"))
                .expect("parent floor should be present")
                .to_string(),
            "parent floor should be read from Git",
        );

        assert_error_contains(
            previous_floors(
                Path::new(super::DEFAULT_FLOORS),
                repo,
                Some(OsStr::new("definitely-missing-ref")),
            ),
            "cannot resolve coverage base ref 'definitely-missing-ref'",
        );
    }

    /// Temporary fixture directory removed after each test.
    #[repr(transparent)]
    struct FixtureDir
    {
        /// Owned fixture root path.
        path: PathBuf,
    }

    impl FixtureDir
    {
        /// Create a unique fixture directory under the system temp root.
        fn new<'semantic, L>(label: L) -> Self
        where
            L: Into<LabelText<'semantic>>,
        {
            let label = label.into().0;
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "gandr-workflow-gates-{label}-{}-{unique}",
                std::process::id(),
            ));
            crate::support::HOST_FILESYSTEM
                .create_dir_all(&path)
                .expect("fixture directory should be creatable");
            Self { path }
        }

        /// Borrow the fixture root path.
        fn path(&self) -> &Path
        {
            &self.path
        }
    }

    impl Drop for FixtureDir
    {
        fn drop(&mut self)
        {
            let _ignore_cleanup_error = crate::support::HOST_FILESYSTEM.remove_dir_all(&self.path);
        }
    }

    /// Empty new-file exemption fixture rows.
    const NO_EXEMPTIONS: &[ExemptionRow<'static>] = &[];

    /// Rendered floor row for TOML and map fixtures.
    #[derive(Clone, Copy)]
    struct FloorRow<'text>
    {
        /// Fixture file path.
        file: &'text str,
        /// Rendered floor percent.
        floor: &'text str,
    }

    impl<'item, 'text> From<&'item (&'text str, &'text str)> for FloorRow<'text>
    {
        fn from(value: &'item (&'text str, &'text str)) -> Self
        {
            Self {
                file: value.0,
                floor: value.1,
            }
        }
    }

    /// Rendered new-file exemption row for TOML and map fixtures.
    #[derive(Clone, Copy)]
    struct ExemptionRow<'text>
    {
        /// Fixture file path.
        file: &'text str,
        /// Exemption reason.
        reason: &'text str,
    }

    impl<'item, 'text> From<&'item (&'text str, &'text str)> for ExemptionRow<'text>
    {
        fn from(value: &'item (&'text str, &'text str)) -> Self
        {
            Self {
                file: value.0,
                reason: value.1,
            }
        }
    }

    impl<'item> From<&'item Self> for ExemptionRow<'_>
    {
        fn from(value: &'item Self) -> Self
        {
            *value
        }
    }

    /// Rendered coverage file entry.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct RenderedCoverageEntry<'text>
    {
        /// Rendered JSON entry text.
        text: &'text str,
    }

    impl<'item, 'text> From<&'item &'text str> for RenderedCoverageEntry<'text>
    {
        fn from(value: &'item &'text str) -> Self
        {
            Self { text: *value }
        }
    }

    impl<'item> From<&'item &String> for RenderedCoverageEntry<'item>
    {
        fn from(value: &'item &String) -> Self
        {
            Self {
                text: value.as_str(),
            }
        }
    }

    /// Git argument fixture token.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct GitArgText<'text>
    {
        /// Argument text.
        text: &'text str,
    }

    impl<'item, 'text> From<&'item &'text str> for GitArgText<'text>
    {
        fn from(value: &'item &'text str) -> Self
        {
            Self { text: *value }
        }
    }

    impl<'text> From<&'text str> for GitArgText<'text>
    {
        fn from(value: &'text str) -> Self
        {
            Self { text: value }
        }
    }

    /// Measured coverage fixture row.
    #[derive(Clone, Copy)]
    struct MeasurementRow<'text>
    {
        /// Fixture file path.
        file: &'text str,
        /// Covered line count.
        covered: u64,
        /// Total line count.
        count: u64,
    }

    impl<'item, 'text> From<&'item (&'text str, u64, u64)> for MeasurementRow<'text>
    {
        fn from(value: &'item (&'text str, u64, u64)) -> Self
        {
            Self {
                file: value.0,
                covered: value.1,
                count: value.2,
            }
        }
    }

    /// Coverage JSON fixture row.
    #[derive(Clone, Copy)]
    struct CoverageJsonRow<'text>
    {
        /// Fixture file path.
        file: &'text str,
        /// Covered line count.
        covered: u64,
        /// Total line count.
        count: u64,
        /// Rendered percent literal.
        percent: &'text str,
    }

    impl<'item, 'text> From<&'item (&'text str, u64, u64, &'text str)> for CoverageJsonRow<'text>
    {
        fn from(value: &'item (&'text str, u64, u64, &'text str)) -> Self
        {
            Self {
                file: value.0,
                covered: value.1,
                count: value.2,
                percent: value.3,
            }
        }
    }

    /// Write a fixture file, creating parent directories as needed.
    fn write_text<'semantic, T>(
        path: &Path,
        text: T,
    ) where
        T: Into<super::TextText<'semantic>>,
    {
        let text = text.into().0;
        if let Some(parent) = path.parent() {
            crate::support::HOST_FILESYSTEM
                .create_dir_all(parent)
                .expect("fixture parent should be creatable");
        }
        crate::support::HOST_FILESYSTEM
            .write(path, text)
            .expect("fixture file should be writable");
    }

    /// Build a current-target floor TOML fixture.
    fn floors_toml<'semantic, Rows, Row, Exemptions, Exemption>(
        rows: Rows,
        exemptions: Exemptions,
    ) -> String
    where
        Rows: IntoIterator<Item = Row>,
        Row: Into<FloorRow<'semantic>>,
        Exemptions: IntoIterator<Item = Exemption>,
        Exemption: Into<ExemptionRow<'semantic>>,
    {
        floors_toml_with_target("80.00", rows, exemptions)
    }

    /// Build a floor TOML fixture with an explicit target.
    fn floors_toml_with_target<'semantic, Target, Rows, Row, Exemptions, Exemption>(
        target: Target,
        rows: Rows,
        exemptions: Exemptions,
    ) -> String
    where
        Target: Into<super::TargetText<'semantic>>,
        Rows: IntoIterator<Item = Row>,
        Row: Into<FloorRow<'semantic>>,
        Exemptions: IntoIterator<Item = Exemption>,
        Exemption: Into<ExemptionRow<'semantic>>,
    {
        let target = target.into().0;
        let mut text = format!("target_percent = {target}\n\n[files]\n");
        for row in rows {
            let row = row.into();
            text.push('"');
            text.push_str(row.file);
            text.push_str("\" = ");
            text.push_str(row.floor);
            text.push('\n');
        }
        let mut has_exemptions = false;
        for exemption in exemptions {
            let exemption = exemption.into();
            if !has_exemptions {
                text.push_str("\n[new_file_exemptions]\n");
                has_exemptions = true;
            }
            text.push('"');
            text.push_str(exemption.file);
            text.push_str("\" = \"");
            text.push_str(exemption.reason);
            text.push_str("\"\n");
        }
        text
    }

    /// Build a strict llvm-cov JSON fixture from already rendered file entries.
    fn coverage_document_from_entries<'semantic, Entries, Entry>(entries: Entries) -> String
    where
        Entries: IntoIterator<Item = Entry>,
        Entry: Into<RenderedCoverageEntry<'semantic>>,
    {
        let joined = entries
            .into_iter()
            .map(|entry| entry.into().text.to_owned())
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"type\":\"llvm.coverage.json.export\",\"version\":\"3.1.0\",\"cargo_llvm_cov\":{{\"version\":\"0.8.7\"}},\"data\":[{{\"files\":[{}]}}]}}",
            joined
        )
    }

    /// Run a Git command in a fixture repository.
    fn run_git<'semantic, Args, Arg>(
        repo: &Path,
        args: Args,
    ) where
        Args: IntoIterator<Item = Arg>,
        Arg: Into<GitArgText<'semantic>>,
    {
        let args = args
            .into_iter()
            .map(|arg| std::ffi::OsString::from(arg.into().text))
            .collect::<Vec<_>>();
        let mut command = crate::support::stateless_git_command();
        command.args(&args).current_dir(repo);
        let status = command.status().expect("git should be runnable");
        assert!(status.success(), "git {args:?} should succeed");
    }

    /// Commit all current fixture repository changes.
    fn commit_all<'semantic, M>(
        repo: &Path,
        message: M,
    ) where
        M: Into<super::MessageText<'semantic>>,
    {
        let message = message.into().0;
        run_git(repo, ["add", "."]);
        run_git(repo, ["commit", "--quiet", "-m", message]);
    }

    /// Build a measured map fixture.
    fn measured_map<'semantic, Rows, Row>(rows: Rows) -> BTreeMap<ProductionFile, MeasuredCoverage>
    where
        Rows: IntoIterator<Item = Row>,
        Row: Into<MeasurementRow<'semantic>>,
    {
        let mut measured = BTreeMap::new();
        for row in rows {
            let row = row.into();
            let file_key =
                ProductionFile::from_floor_key(row.file).expect("fixture file key is valid");
            let percent =
                Percent::from_counts(row.covered, row.count).expect("fixture percent is valid");
            measured.insert(file_key.clone(), MeasuredCoverage {
                file: file_key,
                covered: row.covered,
                count: row.count,
                percent,
            });
        }
        measured
    }

    /// Build a default-target floor map fixture.
    fn floors_map<'semantic, Rows, Row, Exemptions, Exemption>(
        rows: Rows,
        exemptions: Exemptions,
    ) -> CoverageFloors
    where
        Rows: IntoIterator<Item = Row>,
        Row: Into<FloorRow<'semantic>>,
        Exemptions: IntoIterator<Item = Exemption>,
        Exemption: Into<ExemptionRow<'semantic>>,
    {
        floors_with_target("80.00", rows, exemptions)
    }

    /// Build a floor map fixture with an explicit target.
    fn floors_with_target<'semantic, Target, Rows, Row, Exemptions, Exemption>(
        target: Target,
        rows: Rows,
        exemptions: Exemptions,
    ) -> CoverageFloors
    where
        Target: Into<super::TargetText<'semantic>>,
        Rows: IntoIterator<Item = Row>,
        Row: Into<FloorRow<'semantic>>,
        Exemptions: IntoIterator<Item = Exemption>,
        Exemption: Into<ExemptionRow<'semantic>>,
    {
        let target = target.into().0;
        let mut files = BTreeMap::new();
        for row in rows {
            let row = row.into();
            files.insert(
                ProductionFile::from_floor_key(row.file).expect("fixture file key is valid"),
                Percent::parse_exact(row.floor).expect("fixture floor is valid"),
            );
        }
        let mut exemption_map = BTreeMap::new();
        for exemption in exemptions {
            let exemption = exemption.into();
            exemption_map.insert(
                ProductionFile::from_floor_key(exemption.file).expect("fixture file key is valid"),
                String::from(exemption.reason),
            );
        }
        CoverageFloors {
            target_percent: if target == "80.00" {
                DEFAULT_TARGET_PERCENT
            }
            else {
                Percent::parse_exact(target).expect("fixture target is valid")
            },
            files,
            exemptions: exemption_map,
        }
    }

    /// Build a strict llvm-cov JSON fixture from canonical names.
    fn coverage_document<'semantic, Rows, Row>(rows: Rows) -> String
    where
        Rows: IntoIterator<Item = Row>,
        Row: Into<CoverageJsonRow<'semantic>>,
    {
        coverage_document_with_names(rows)
    }

    /// Build a strict llvm-cov JSON fixture from arbitrary names.
    fn coverage_document_with_names<'semantic, Rows, Row>(rows: Rows) -> String
    where
        Rows: IntoIterator<Item = Row>,
        Row: Into<CoverageJsonRow<'semantic>>,
    {
        let mut files = Vec::new();
        for row in rows {
            let row = row.into();
            files.push(format!(
                "{{\"filename\":\"{}\",\"summary\":{{\"lines\":{{\"count\":{},\"covered\":{},\"percent\":{}}}}}}}",
                row.file,
                row.count,
                row.covered,
                row.percent,
            ));
        }
        format!(
            "{{\"type\":\"llvm.coverage.json.export\",\"version\":\"3.1.0\",\"cargo_llvm_cov\":{{\"version\":\"0.8.7\"}},\"data\":[{{\"files\":[{}]}}]}}",
            files.join(",")
        )
    }

    /// Assert an error display contains a stable fragment.
    fn assert_error_contains<'semantic, T, E>(
        result: Result<T, GateError>,
        expected: E,
    ) where
        E: Into<super::ExpectedText<'semantic>>,
    {
        let expected = expected.into().0;
        match result {
            | Ok(_) => panic!("expected fixture to fail"),
            | Err(error) => {
                let text = error.to_string();
                assert!(
                    text.contains(expected),
                    "error {text} should contain {expected}",
                );
            },
        }
    }
}
