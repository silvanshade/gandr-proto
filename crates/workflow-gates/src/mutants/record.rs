//! Canonical, replayable records for mutation campaign findings.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

/// Current canonical mutation record schema.
const CURRENT_SCHEMA_VERSION: u16 = 1;

/// A mutation verdict with evidence that distinguishes viability from survival.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Verdict
{
    /// The named test observed the mutation and failed.
    Killed
    {
        test: String
    },
    /// The mutated program did not compile; this is never a survivor.
    CompileError
    {
        diagnostic: String
    },
    /// The selected test/build budget expired before classification.
    TimedOut
    {
        evidence: String
    },
    /// Every selected test passed for the viable mutation.
    Survivor
    {
        evidence: String
    },
}

/// A deterministic mutation finding that a later reader can reapply exactly.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MutationRecord
{
    /// Schema version for deterministic migration and rejection.
    pub schema_version: u16,
    /// Repository-relative source file containing the mutation.
    pub file: String,
    /// Stable semantic item or entity name containing the edit.
    pub item: String,
    /// Exact source text that must occur at the mutation site.
    pub before: String,
    /// Replacement source text applied at the mutation site.
    pub after: String,
    /// Bounded base identity validated before replay.
    pub base: String,

    /// Exact unified patch when the edit spans multiple source lines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<String>,
    /// Classification and its observable evidence.
    pub verdict: Verdict,
}

/// Errors raised before a mutation can be reapplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayError
{
    UnsupportedSchema(u16),
    BaseMismatch
    {
        expected: String,
        observed: String,
    },
    EmptySite,
    SourceUnavailable
    {
        detail: String,
    },
    AmbiguousSite
    {
        matches: usize,
    },
}
impl core::fmt::Display for ReplayError
{
    #[inline]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Display matches the borrowed error enum by project pattern convention"
    )]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match self {
            | Self::UnsupportedSchema(version) => {
                write!(f, "unsupported mutation record schema {version}")
            },
            | Self::BaseMismatch { expected, observed } => write!(
                f,
                "mutation base mismatch: expected {expected}, observed {observed}"
            ),
            | Self::EmptySite => f.write_str("mutation record has an empty before-image"),
            | Self::SourceUnavailable { detail } => {
                write!(f, "mutation source unavailable: {detail}")
            },
            | Self::AmbiguousSite { matches } => write!(
                f,
                "mutation before-image matched {matches} sites; expected exactly one"
            ),
        }
    }
}

impl std::error::Error for ReplayError
{
}

/// Failures while converting cargo-mutants artifacts into canonical records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReportError
{
    MissingArtifact(PathBuf),
    UnsupportedRecord(String),
    InvalidJson(String),
    Replay
    {
        name: String,
        file: String,
        source: ReplayError,
    },
}

impl core::fmt::Display for ReportError
{
    #[inline]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Display matches borrowed report errors by project convention"
    )]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match self {
            | Self::MissingArtifact(path) => {
                write!(f, "mutation report missing artifact {}", path.display())
            },
            | Self::UnsupportedRecord(detail) => {
                write!(f, "unsupported cargo-mutants record: {detail}")
            },
            | Self::InvalidJson(detail) => write!(f, "invalid cargo-mutants JSON: {detail}"),
            | Self::Replay { name, file, source } => write!(
                f,
                "cargo-mutants record {name} at {file} cannot replay against campaign base: {source}"
            ),
        }
    }
}

impl std::error::Error for ReportError
{
}

/// Read non-empty outcome evidence from the referenced cargo-mutants log.
#[expect(
    clippy::needless_pass_by_value,
    reason = "owned outcome evidence boundary"
)]
fn outcome_log(
    report_dir: &Path,
    outcomes: &OutcomeNames,
    name: String,
) -> Result<String, ReportError>
{
    let log_rel = outcomes.logs.get(&name).ok_or_else(|| {
        ReportError::UnsupportedRecord(format!("{name}: outcome log reference is missing"))
    })?;
    let log = report_dir.join(log_rel);
    let text = std::fs::read_to_string(&log).map_err(|error| {
        ReportError::UnsupportedRecord(format!("cannot read {}: {error}", log.display()))
    })?;
    if text.trim().is_empty() {
        return Err(ReportError::UnsupportedRecord(format!(
            "{name}: outcome log is empty"
        )));
    }
    Ok(text)
}

/// Convert a cargo-mutants mutants.out directory into canonical records.
///
/// # Errors
/// Returns a named unsupported-record error for missing fields, ambiguous
/// edits, or unknown outcomes.
///
/// The converter is deliberately strict: one-line diffs and named functions
/// are required so a legacy or future format cannot be guessed into a site.
#[inline]
#[expect(
    clippy::needless_pass_by_value,
    reason = "owned base identity is the report conversion boundary"
)]
pub fn convert_cargo_mutants_report(
    report_dir: &Path,
    source_root: &Path,
    base: String,
) -> Result<Vec<MutationRecord>, ReportError>
{
    let mutants_path = report_dir.join("mutants.json");
    let mutants_text = std::fs::read_to_string(&mutants_path).map_err(|error| {
        ReportError::UnsupportedRecord(format!("cannot read {}: {error}", mutants_path.display()))
    })?;
    let mutants: Vec<serde_json::Value> = serde_json::from_str(&mutants_text)
        .map_err(|error| ReportError::InvalidJson(error.to_string()))?;
    let outcomes = outcome_names(report_dir)?;
    let mut records = Vec::with_capacity(mutants.len());
    for mutant in mutants {
        let name = required_string(&mutant, "name".into())?;
        let file = required_string(&mutant, "file".into())?;
        let function = mutant
            .get("function")
            .and_then(|value| value.get("function_name"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ReportError::UnsupportedRecord(format!("{name}: function site is absent"))
            })?;
        let diff = required_string(&mutant, "diff".into())?;
        let (before, after, patch) = edit_payload(diff, name.clone())?;
        let verdict = if outcomes.unviable.contains(&name) {
            Verdict::CompileError {
                diagnostic: outcome_log(report_dir, &outcomes, name.clone())?,
            }
        }
        else if outcomes.timed_out.contains(&name) {
            Verdict::TimedOut {
                evidence: outcome_log(report_dir, &outcomes, name.clone())?,
            }
        }
        else if outcomes.caught.contains(&name) {
            let log = outcome_log(report_dir, &outcomes, name.clone())?;
            let test = log
                .lines()
                .find(|line| line.contains("FAILED") || line.starts_with("test "))
                .map(str::to_owned)
                .ok_or_else(|| {
                    ReportError::UnsupportedRecord(format!(
                        "{name}: caught outcome has no failing test evidence"
                    ))
                })?;
            Verdict::Killed { test }
        }
        else if outcomes.missed.contains(&name) {
            Verdict::Survivor {
                evidence: outcome_log(report_dir, &outcomes, name.clone())?,
            }
        }
        else {
            return Err(ReportError::UnsupportedRecord(format!(
                "{name}: outcome is missing or unsupported"
            )));
        };
        let mut record = MutationRecord::new(
            file.clone(),
            function.to_owned(),
            before,
            after,
            base.clone(),
            verdict,
        );
        let source =
            source_at_base(source_root, &base, &file).map_err(|source| ReportError::Replay {
                name: name.clone(),
                file: file.clone(),
                source,
            })?;
        record
            .reapply(source, base.clone())
            .map_err(|source| ReportError::Replay {
                name: name.clone(),
                file: file.clone(),
                source,
            })?;
        record.patch = patch;
        records.push(record);
    }
    Ok(records)
}
/// Load the campaign-base source used to validate an exact mutation site.
#[expect(
    unknown_lints,
    reason = "primitive_signature is supplied by the local dylint library"
)]
#[expect(
    clippy::allow_attributes,
    reason = "the stable compiler and local dylint library disagree about primitive_signature"
)]
#[allow(
    unfulfilled_lint_expectations,
    reason = "the unknown-lint expectation is fulfilled only under the stable compiler"
)]
#[expect(
    primitive_signature,
    reason = "Git revision and repository-relative path are validated text tokens at this boundary"
)]
fn source_at_base(
    root: &Path,
    base: &str,
    file: &str,
) -> Result<String, ReplayError>
{
    let output = std::process::Command::new("git")
        .args([
            "-C",
            root.to_str().unwrap_or_default(),
            "show",
            &format!("{base}:{file}"),
        ])
        .output()
        .map_err(|error| ReplayError::SourceUnavailable {
            detail: error.to_string(),
        })?;
    if !output.status.success() {
        if base == "HEAD" {
            return std::fs::read_to_string(root.join(file)).map_err(|error| {
                ReplayError::SourceUnavailable {
                    detail: error.to_string(),
                }
            });
        }
        return Err(ReplayError::SourceUnavailable {
            detail: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    String::from_utf8(output.stdout).map_err(|error| ReplayError::SourceUnavailable {
        detail: error.to_string(),
    })
}

/// Outcome names grouped by cargo-mutants classification.
struct OutcomeNames
{
    /// Names classified as killed.
    caught: BTreeSet<String>,
    /// Names classified as survivors.
    missed: BTreeSet<String>,
    /// Names classified as compile errors.
    unviable: BTreeSet<String>,
    /// Names whose execution budget expired before classification.
    timed_out: BTreeSet<String>,
    /// Log paths keyed by mutant name.
    logs: BTreeMap<String, String>,
}

/// Load cargo-mutants outcome lists.
fn outcome_names(report_dir: &Path) -> Result<OutcomeNames, ReportError>
{
    #[expect(
        clippy::needless_pass_by_value,
        reason = "owned outcome filename boundary"
    )]
    fn load(
        root: &Path,
        name: String,
    ) -> Result<BTreeSet<String>, ReportError>
    {
        let path = root.join(&name);
        let text = std::fs::read_to_string(&path).map_err(|error| {
            ReportError::UnsupportedRecord(format!("cannot read {}: {error}", path.display()))
        })?;
        Ok(text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect())
    }
    let outcomes_path = report_dir.join("outcomes.json");
    let outcomes_text = std::fs::read_to_string(&outcomes_path).map_err(|error| {
        ReportError::UnsupportedRecord(format!("cannot read {}: {error}", outcomes_path.display()))
    })?;
    let outcomes: serde_json::Value = serde_json::from_str(&outcomes_text)
        .map_err(|error| ReportError::InvalidJson(error.to_string()))?;
    let mut logs = BTreeMap::new();
    if let Some(rows) = outcomes
        .get("outcomes")
        .and_then(serde_json::Value::as_array)
    {
        for row in rows {
            if let (Some(name), Some(path)) = (
                row.get("scenario")
                    .and_then(|v| v.get("Mutant"))
                    .and_then(|v| v.get("name"))
                    .and_then(serde_json::Value::as_str),
                row.get("log_path").and_then(serde_json::Value::as_str),
            ) {
                logs.insert(name.to_owned(), path.to_owned());
            }
        }
    }
    Ok(OutcomeNames {
        caught: load(report_dir, "caught.txt".into())?,
        missed: load(report_dir, "missed.txt".into())?,
        unviable: load(report_dir, "unviable.txt".into())?,
        timed_out: load(report_dir, "timeout.txt".into())?,
        logs,
    })
}

/// Read a required string field from a cargo-mutants JSON object.
#[expect(clippy::needless_pass_by_value, reason = "owned JSON field boundary")]
fn required_string(
    value: &serde_json::Value,
    field: String,
) -> Result<String, ReportError>
{
    value
        .get(&field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ReportError::UnsupportedRecord(format!("missing string field {field}")))
}

/// Extract exact source edit data from a unified diff.
#[expect(clippy::needless_pass_by_value, reason = "owned diff boundary")]
fn edit_payload(
    diff: String,
    name: String,
) -> Result<(String, String, Option<String>), ReportError>
{
    let (_header, body) = diff
        .split_once("@@\n")
        .ok_or_else(|| ReportError::UnsupportedRecord(format!("{name}: diff has no hunk")))?;
    if body.lines().any(|line| line.starts_with("@@")) {
        return Err(ReportError::UnsupportedRecord(format!(
            "{name}: diff has multiple hunks"
        )));
    }
    let mut before = String::new();
    let mut after = String::new();
    let mut removed = false;
    let mut added = false;
    for line in body.lines() {
        if line.starts_with("---") || line.starts_with("+++") {
            continue;
        }
        if let Some(value) = line.strip_prefix('-') {
            before.push_str(value);
            before.push('\n');
            removed = true;
        }
        else if let Some(value) = line.strip_prefix('+') {
            after.push_str(value);
            after.push('\n');
            added = true;
        }
        else if let Some(value) = line.strip_prefix(' ') {
            before.push_str(value);
            before.push('\n');
            after.push_str(value);
            after.push('\n');
        }
        else if !line.is_empty() {
            return Err(ReportError::UnsupportedRecord(format!(
                "{name}: unsupported diff line"
            )));
        }
    }
    if !removed || !added {
        return Err(ReportError::UnsupportedRecord(format!(
            "{name}: diff has no exact replacement payload"
        )));
    }
    if before.ends_with('\n') {
        before.pop();
    }
    if after.ends_with('\n') {
        after.pop();
    }
    Ok((before, after, Some(diff)))
}

impl MutationRecord
{
    #[inline]
    #[must_use]
    pub fn new(
        file: String,
        item: String,
        before: String,
        after: String,
        base: String,
        verdict: Verdict,
    ) -> Self
    {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            file,
            item,
            before,
            after,
            base,
            patch: None,
            verdict,
        }
    }

    #[inline]
    /// # Errors
    /// Returns a serialization error when the record cannot be encoded.
    pub fn to_json(&self) -> Result<String, serde_json::Error>
    {
        serde_json::to_string_pretty(self)
    }

    #[inline]
    /// # Errors
    /// Returns an error for malformed or unsupported records.
    #[expect(clippy::needless_pass_by_value, reason = "owned JSON record boundary")]
    pub fn from_json(value: String) -> Result<Self, ReplayError>
    {
        let record: Self = serde_json::from_str(&value).map_err(|error| {
            ReplayError::UnsupportedSchema(if error.is_syntax() { 0 } else { u16::MAX })
        })?;
        if record.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ReplayError::UnsupportedSchema(record.schema_version));
        }
        Ok(record)
    }

    #[inline]
    /// # Errors
    /// Returns an error when the base or exact mutation site is invalid.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "owned replay source boundary"
    )]
    pub fn reapply(
        &self,
        source: String,
        observed_base: String,
    ) -> Result<String, ReplayError>
    {
        if self.base != observed_base {
            return Err(ReplayError::BaseMismatch {
                expected: self.base.clone(),
                observed: observed_base,
            });
        }
        if self.before.is_empty() {
            return Err(ReplayError::EmptySite);
        }
        let mut matches = source.match_indices(&self.before);
        let Some((start, _)) = matches.next()
        else {
            return Err(ReplayError::AmbiguousSite { matches: 0 });
        };
        if matches.next().is_some() {
            return Err(ReplayError::AmbiguousSite {
                matches: source.match_indices(&self.before).count(),
            });
        }
        let end = start
            .checked_add(self.before.len())
            .ok_or(ReplayError::AmbiguousSite { matches: 0 })?;
        let capacity = source
            .len()
            .checked_sub(self.before.len())
            .and_then(|size| size.checked_add(self.after.len()))
            .ok_or(ReplayError::AmbiguousSite { matches: 0 })?;
        let mut result = String::with_capacity(capacity);
        result.push_str(
            source
                .get(.. start)
                .ok_or(ReplayError::AmbiguousSite { matches: 0 })?,
        );
        result.push_str(&self.after);
        result.push_str(
            source
                .get(end ..)
                .ok_or(ReplayError::AmbiguousSite { matches: 0 })?,
        );
        Ok(result)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn record(
        before: String,
        after: String,
        verdict: Verdict,
    ) -> MutationRecord
    {
        MutationRecord::new(
            "crates/example/src/lib.rs".into(),
            "example::answer".into(),
            before,
            after,
            "base-001".into(),
            verdict,
        )
    }

    #[test]
    fn killed_record_round_trips_and_reapplies_at_one_site()
    {
        let original = "fn answer() -> i32 { 1 }\n";
        let record = record("1".into(), "2".into(), Verdict::Killed {
            test: "answer_is_one".into(),
        });
        let json = record.to_json().expect("serialization should work");
        let decoded = MutationRecord::from_json(json).expect("current record should decode");
        assert_eq!(
            decoded
                .reapply(original.into(), "base-001".into())
                .expect("site should apply"),
            "fn answer() -> i32 { 2 }\n"
        );
    }

    #[test]
    fn multi_line_patch_reapplies_at_one_unique_site()
    {
        let mut record = record(
            "old_a\nold_b".into(),
            "new_a\nnew_b".into(),
            Verdict::Killed {
                test: "test_patch ... FAILED".into(),
            },
        );
        record.patch = Some("@@\n-old_a\n-old_b\n+new_a\n+new_b\n".into());
        assert_eq!(
            record.reapply("prefix\nold_a\nold_b\nsuffix".into(), "base-001".into()),
            Ok("prefix\nnew_a\nnew_b\nsuffix".into())
        );
    }

    #[test]
    fn compile_error_is_distinct_from_survivor()
    {
        let record = record("1".into(), "missing_name".into(), Verdict::CompileError {
            diagnostic: "cannot find value `missing_name`".into(),
        });
        assert!(matches!(record.verdict, Verdict::CompileError { .. }));
        assert_ne!(record.verdict, Verdict::Survivor {
            evidence: String::new()
        });
    }

    #[test]
    fn cargo_mutants_direct_and_scheduled_reports_converge()
    {
        let root =
            std::env::temp_dir().join(format!("gandr-mutation-record-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&root));
        std::fs::create_dir_all(&root).expect("fixture directory");
        std::fs::create_dir_all(root.join("src")).expect("source directory");
        std::fs::create_dir_all(root.join("logs")).expect("logs directory");
        std::fs::write(root.join("mutants.json"), r#"[{"name":"m-1","file":"src/lib.rs","function":{"function_name":"answer"},"diff":"--- src/lib.rs\n+++ src/lib.rs\n@@\n-1\n+2\n"}]"#).expect("mutants fixture");
        std::fs::write(root.join("caught.txt"), "m-1\n").expect("caught fixture");
        std::fs::write(root.join("missed.txt"), "").expect("missed fixture");
        std::fs::write(root.join("unviable.txt"), "").expect("unviable fixture");
        std::fs::write(root.join("timeout.txt"), "").expect("timeout fixture");
        std::fs::write(
            root.join("outcomes.json"),
            r#"{"outcomes":[{"scenario":{"Mutant":{"name":"m-1"}},"log_path":"log/m-1.log"}]}"#,
        )
        .expect("outcomes fixture");
        std::fs::create_dir_all(root.join("src")).expect("source directory");
        std::fs::write(root.join("src/lib.rs"), "fn answer() -> i32 { 1 }\n")
            .expect("source fixture");
        std::fs::create_dir_all(root.join("log")).expect("log directory");
        std::fs::write(root.join("log/m-1.log"), "test answer_is_one ... FAILED")
            .expect("log fixture");
        let direct =
            convert_cargo_mutants_report(&root, &root, "HEAD".into()).expect("direct conversion");
        let scheduled = convert_cargo_mutants_report(&root, &root, "HEAD".into())
            .expect("scheduled conversion");
        assert_eq!(direct, scheduled);
        assert_eq!(
            direct[0].to_json().expect("canonical JSON"),
            scheduled[0].to_json().expect("canonical JSON")
        );
    }

    #[test]
    fn ambiguous_site_is_rejected_without_guessing()
    {
        let record = record("x".into(), "y".into(), Verdict::Survivor {
            evidence: "all tests passed".into(),
        });
        assert_eq!(
            record.reapply("x + x".into(), "base-001".into()),
            Err(ReplayError::AmbiguousSite { matches: 2 })
        );
    }
    #[test]
    fn timeout_and_stale_site_are_evidence_backed()
    {
        let root =
            std::env::temp_dir().join(format!("gandr-mutation-timeout-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&root));
        std::fs::create_dir_all(root.join("src")).expect("source directory");
        std::fs::write(root.join("src/lib.rs"), "fn answer() -> i32 { 1 }\n")
            .expect("timeout source");
        std::fs::create_dir_all(root.join("log")).expect("fixture directory");
        std::fs::write(
            root.join("mutants.json"),
            r#"[{"name":"m-timeout","file":"src/lib.rs","function":{"function_name":"answer"},"diff":"--- src/lib.rs\n+++ src/lib.rs\n@@\n-1\n+2\n"},{"name":"m-stale","file":"src/missing.rs","function":{"function_name":"answer"},"diff":"--- src/missing.rs\n+++ src/missing.rs\n@@\n-1\n+2\n"}]"#,
        )
        .expect("mutants fixture");
        std::fs::write(root.join("caught.txt"), "").expect("caught fixture");
        std::fs::write(root.join("missed.txt"), "").expect("missed fixture");
        std::fs::write(root.join("unviable.txt"), "m-stale\n").expect("unviable fixture");
        std::fs::write(root.join("timeout.txt"), "m-timeout\n").expect("timeout fixture");
        std::fs::write(
            root.join("outcomes.json"),
            r#"{"outcomes":[{"scenario":{"Mutant":{"name":"m-timeout"}},"log_path":"log/m-timeout.log"},{"scenario":{"Mutant":{"name":"m-stale"}},"log_path":"log/m-stale.log"}]}"#,
        )
        .expect("outcomes fixture");
        std::fs::write(root.join("log/m-timeout.log"), "TIMEOUT after 2s").expect("timeout log");
        std::fs::write(root.join("log/m-stale.log"), "unviable").expect("stale log");
        let error = convert_cargo_mutants_report(&root, &root, "HEAD".into())
            .expect_err("stale source must obstruct conversion");
        assert!(matches!(error, ReportError::Replay { .. }), "{error:?}");
    }
}
