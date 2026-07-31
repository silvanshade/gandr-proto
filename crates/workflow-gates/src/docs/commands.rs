//! External documentation command probes and guarded wrappers.
//!
//! This module keeps the retained behavior of the page-balance and rumdl
//! documentation gates: the page-balance probe shells out to `typst eval`
//! but parses and filters the JSON in process, while the rumdl wrapper scans
//! supplied Markdown files for unresolved conflict-marker lines before
//! delegating to `rumdl fmt` or `rumdl check` with unchanged path order.

extern crate alloc;

use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitStatus;

use serde_json::Value;

use crate::GateError;
use crate::support;

crate::semantic_str!(pub struct FieldText);
crate::semantic_str!(pub struct DetailText);
/// Define a named transparent copy boundary for a scalar domain that cannot
/// derive [`Eq`].
macro_rules! semantic_partial_copy {
    ($vis:vis struct $name:ident($inner:ty)) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq)]
        $vis struct $name(pub $inner);

        impl From<$inner> for $name {
            #[inline]
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $inner {
            #[inline]
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}
semantic_partial_copy!(pub struct BottomMmMillimeters(f64));
semantic_partial_copy!(pub struct BandMmMillimeters(f64));
crate::semantic_copy!(pub struct LineNumberNumber(usize));
crate::semantic_str!(pub struct MarkerText);
crate::semantic_str!(pub struct NameText);
crate::semantic_str!(pub struct SourceText);
crate::semantic_str!(pub struct ValueText);
crate::semantic_str!(pub struct LineText);
crate::semantic_copy!(pub struct LateProbesFlag(bool));
crate::semantic_str!(pub struct AsStrText);
crate::semantic_copy!(pub struct JsonI64Seconds(i64));
crate::semantic_str!(pub struct JsonStringText);
semantic_partial_copy!(pub struct JsonF64Millimeters(f64));
crate::semantic_optional_str!(pub struct OptionalConflictMarkerLineText);

/// Manual text-block bottom edge in millimeters.
pub const PAGE_BOTTOM_MM: f64 = 273.0_f64;

/// Bottom-page band, in millimeters, used for informational late-opener notes.
pub const PAGE_BOTTOM_BAND_MM: f64 = 55.0_f64;

/// Stable failure emitted when `typst eval` or probe JSON decoding fails.
pub const PAGE_BALANCE_TYPST_FAILURE: &str = "FAIL page-balance probe -- typst eval query failed. The manual must compile first; derived highlight JSON may be missing: run `mise run docs:manual`.";

/// Stable failure emitted when the manual contains no layout probes.
pub const PAGE_BALANCE_VACUOUS_FAILURE: &str =
    "FAIL page-balance probe -- no <layout-probe> metadata found (lib/spec.typ hook drift?)";

/// One Typst `<layout-probe>` row.
///
/// # Contract
/// - ensures: carries the probe kind, one-based page number, and opening `y`
///   coordinate in millimeters exactly as decoded from Typst JSON.
/// - provides: typed page-balance input rows without shell-table records.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — strict bottom-band boundary and malformed JSON
///   fixtures distinguish every consumed field.
/// - witness: `commands::tests::strict_page_band_boundary_excludes_exact_threshold`
#[derive(Clone)]
pub struct PageProbe
{
    /// Display-block kind emitted by the Typst probe hook.
    pub kind: String,
    /// Page number reported by Typst.
    pub page: i64,
    /// Vertical opening position in millimeters.
    pub y_mm: f64,
}

/// Parsed page-balance probe report.
///
/// # Contract
/// - ensures: `late_probes` contains exactly those rows whose `y_mm` is greater
///   than `bottom_mm - band_mm`.
/// - provides: the typed note-only page-balance outcome for CLI rendering.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — zero probes, no late probes, exact-boundary
///   probes, and late probes all produce distinct observable reports.
/// - witness: `commands::tests::strict_page_band_boundary_excludes_exact_threshold`
#[derive(Clone)]
pub struct PageBalanceReport
{
    /// Number of decoded display-block probes.
    pub probed: usize,
    /// Probes opening inside the configured bottom-page band.
    pub late_probes: Vec<PageProbe>,
}

impl PageBalanceReport
{
    /// Return whether any display block opens inside the configured bottom
    /// band.
    #[inline]
    #[must_use]
    pub fn has_late_probes(&self) -> impl Into<LateProbesFlag>
    {
        return !self.late_probes.is_empty();
    }
}

/// Rumdl subcommand supported by the conflict-marker guard.
///
/// # Contract
/// - ensures: only `fmt` and `check` can be represented.
/// - provides: a typed mode instead of forwarding arbitrary command strings.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — `fmt`, `check`, and an unsupported string
///   distinguish parsing and exact argument rendering.
/// - witness: `commands::tests::clean_markdown_files_preserve_argument_order`
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum RumdlMode
{
    /// `rumdl fmt`.
    Fmt,
    /// `rumdl check`.
    Check,
}

impl RumdlMode
{
    /// Parse a rumdl mode string.
    ///
    /// # Contract
    /// - ensures: returns [`RumdlMode::Fmt`] for `fmt` and [`RumdlMode::Check`]
    ///   for `check`.
    /// - provides: the supported command grammar boundary for the wrapper.
    /// - fails: returns a usage error for every other string.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Usage`] when `value` is not `fmt` or `check`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — accepted `fmt`, accepted `check`, and one
    ///   unsupported mode kill all parser branches.
    /// - witness: `commands::tests::clean_markdown_files_preserve_argument_order`
    #[inline]
    pub fn parse<'semantic, Value>(value: Value) -> Result<Self, GateError>
    where
        Value: Into<ValueText<'semantic>>,
    {
        let value = value.into().0;
        match value {
            | "fmt" => Ok(Self::Fmt),
            | "check" => Ok(Self::Check),
            | other => Err(GateError::usage(format!("unsupported rumdl mode: {other}"))),
        }
    }

    /// Return the exact rumdl subcommand string.
    #[inline]
    #[must_use]
    pub fn as_str(self) -> impl Into<AsStrText<'static>>
    {
        match self {
            | Self::Fmt => "fmt",
            | Self::Check => "check",
        }
    }
}

/// Exact external command used by the guarded rumdl wrapper.
///
/// # Contract
/// - ensures: `rumdl_args` begins with the mode followed by every caller path
///   in encounter order after conflict-marker validation has succeeded.
/// - provides: a testable command vector model before the rumdl process is
///   spawned.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — mode, marker rejection, path ordering, and
///   empty-path-list cases are killed by exact vector and error assertions.
/// - witness: `commands::tests::conflict_markers_block_rumdl_planning`
/// - witness: `commands::tests::clean_markdown_files_preserve_argument_order`
#[derive(Clone, Eq, PartialEq)]
pub struct RumdlCommandPlan
{
    /// Program used for the delegated rumdl invocation.
    pub rumdl_program: OsString,
    /// Arguments passed to rumdl.
    pub rumdl_args: Vec<OsString>,
}

/// Outcome of running the guarded rumdl wrapper.
///
/// # Contract
/// - ensures: unresolved conflict-marker lines return a [`GateError`] before
///   any rumdl invocation; otherwise the wrapper returns rumdl's exit status.
/// - provides: the typed process boundary needed by CLI integration.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — marker-failure and clean-success branches are
///   distinguished by planning witnesses and process-order integration tests in
///   the CLI layer.
/// - witness: `commands::tests::conflict_markers_block_rumdl_planning`
/// - witness: `commands::tests::clean_markdown_files_preserve_argument_order`
pub enum RumdlOutcome
{
    /// Conflict-marker validation succeeded and rumdl was invoked.
    RumdlStatus
    {
        /// Exit status returned by rumdl.
        status: ExitStatus,
    },
}

/// Run the Typst page-balance probe and parse its JSON output.
///
/// # Contract
/// - requires: `cwd` is the repository root or another directory where
///   `docs/manual/main.typ` and Typst root `.` resolve as expected.
/// - ensures: invokes `typst eval 'query(<layout-probe>)' --in
///   docs/manual/main.typ --root . --format json`, fails on zero probes, and
///   returns a note-only report for late openers.
/// - provides: external Typst execution with in-process JSON parsing and strict
///   bottom-band filtering.
/// - fails: returns the retained page-balance failure phrase when Typst, JSON
///   parsing, or probe extraction fails, and a vacuity failure when the probe
///   list is empty.
/// - panics: none.
///
/// # Errors
/// Returns operational errors for failed Typst/probe decoding or zero probes.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — failed command, malformed JSON, zero probes,
///   exact-threshold probe, and late probe cases are separated by exact
///   command/output fixtures.
/// - witness: `commands::tests::strict_page_band_boundary_excludes_exact_threshold`
#[inline]
pub fn run_page_balance(cwd: Option<&Path>) -> Result<PageBalanceReport, GateError>
{
    run_page_balance_program(OsStr::new("typst"), cwd)
}

/// Run the page-balance probe with an explicit executable.
///
/// # Contract
/// - requires: `program` implements Typst-compatible `eval query` semantics.
/// - ensures: passes the retained Typst probe arguments and decodes the report
///   exactly as [`run_page_balance`].
/// - provides: an injectable process seam for observable command tests.
/// - fails: maps process and probe-shape failures to retained gate errors.
/// - panics: none.
///
/// # Errors
/// Returns operational errors for failed Typst/probe decoding or zero probes.
fn run_page_balance_program(
    program: &OsStr,
    cwd: Option<&Path>,
) -> Result<PageBalanceReport, GateError>
{
    let args = typst_probe_args();
    let output =
        support::run_output(program, &args, cwd, false).map_err(page_balance_probe_failed_from)?;
    if !output.success().into().0 {
        return Err(page_balance_probe_failed());
    }
    let stdout = output.stdout_lossy();
    let probes = parse_page_probes(stdout.as_ref()).map_err(page_balance_probe_failed_from)?;
    page_balance_report_from_probes(&probes)
}

/// Validate conflict markers and delegate to rumdl only on clean input.
///
/// # Contract
/// - requires: `paths` contains the Markdown files supplied by the caller, and
///   relative paths are resolved against `cwd` for validation and rumdl process
///   execution.
/// - ensures: scans every supplied Markdown file for unresolved conflict-marker
///   lines before any rumdl invocation; on clean input, runs `rumdl <mode>`
///   with `paths` in unchanged encounter order.
/// - provides: the process-level marker barrier with exact path argument order.
/// - fails: returns [`GateError`] for unreadable Markdown paths, unresolved
///   markers, or process-launch errors.
/// - panics: none.
///
/// # Errors
/// Returns file-read errors from conflict-marker validation, operational errors
/// for unresolved marker lines, and support command errors from process launch.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — marker-failure, clean-success, mode, and
///   path-order cases are split by planning and process fixtures.
/// - witness: `commands::tests::conflict_markers_block_rumdl_planning`
/// - witness: `commands::tests::clean_markdown_files_preserve_argument_order`
#[inline]
pub fn run_guarded_rumdl(
    mode: RumdlMode,
    paths: &[PathBuf],
    cwd: Option<&Path>,
) -> Result<RumdlOutcome, GateError>
{
    run_guarded_rumdl_program(OsStr::new("rumdl"), mode, paths, cwd)
}

/// Run guarded rumdl with an explicit executable.
///
/// # Contract
/// - requires: `program` accepts `rumdl`-compatible `fmt` or `check`
///   subcommands.
/// - ensures: performs conflict-marker validation before spawning `program`
///   with path arguments in caller order.
/// - provides: an injectable process seam for the rumdl wrapper.
/// - fails: returns marker, file-read, or process-launch errors.
/// - panics: none.
///
/// # Errors
/// Returns file-read errors from conflict-marker validation, operational errors
/// for unresolved marker lines, and support command errors from process launch.
fn run_guarded_rumdl_program(
    program: &OsStr,
    mode: RumdlMode,
    paths: &[PathBuf],
    cwd: Option<&Path>,
) -> Result<RumdlOutcome, GateError>
{
    let mut plan = rumdl_command_plan(mode, paths, cwd)?;
    plan.rumdl_program = program.to_os_string();
    let status = support::run_status(plan.rumdl_program.as_os_str(), &plan.rumdl_args, cwd, false)?;
    Ok(RumdlOutcome::RumdlStatus { status })
}

/// Validate Markdown paths and build the exact rumdl command vector.
///
/// # Contract
/// - requires: `paths` contains the Markdown files supplied by the caller, and
///   relative paths are resolved against `cwd` for validation only.
/// - ensures: rejects the first unresolved conflict-marker line in encounter
///   order before returning a command plan, and preserves `paths` encounter
///   order in the returned rumdl arguments.
/// - provides: a planning seam for the marker-before-rumdl decision.
/// - fails: returns [`GateError`] for unreadable Markdown paths or unresolved
///   marker lines.
/// - panics: none.
///
/// # Errors
/// Returns file-read errors from conflict-marker validation and operational
/// errors for unresolved marker lines.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — clean files, first marker line, relative `cwd`,
///   and multi-path order fixtures distinguish every planning branch.
/// - witness: `commands::tests::conflict_markers_block_rumdl_planning`
/// - witness: `commands::tests::clean_markdown_files_preserve_argument_order`
#[inline]
pub fn rumdl_command_plan(
    mode: RumdlMode,
    paths: &[PathBuf],
    cwd: Option<&Path>,
) -> Result<RumdlCommandPlan, GateError>
{
    verify_no_conflict_markers(paths, cwd)?;

    let mut rumdl_args = Vec::new();
    rumdl_args.push(OsString::from(mode.as_str().into().0));
    append_path_args(&mut rumdl_args, paths);

    Ok(RumdlCommandPlan {
        rumdl_program: OsString::from("rumdl"),
        rumdl_args,
    })
}

/// Return Typst probe command arguments in exact legacy order.
fn typst_probe_args() -> Vec<OsString>
{
    vec![
        OsString::from("eval"),
        OsString::from("query(<layout-probe>)"),
        OsString::from("--in"),
        OsString::from("docs/manual/main.typ"),
        OsString::from("--root"),
        OsString::from("."),
        OsString::from("--format"),
        OsString::from("json"),
    ]
}

/// Return a JSON object field.
fn json_field<'semantic, 'json, Field>(
    value: &'json Value,
    field: Field,
) -> Result<&'json Value, GateError>
where
    Field: Into<FieldText<'semantic>>,
{
    let field = field.into().0;
    match *value {
        | Value::Object(ref object) => object
            .get(field)
            .ok_or_else(|| GateError::operational(format!("page-balance probe missing `{field}`"))),
        | _ => Err(GateError::operational(
            "page-balance probe row must be an object",
        )),
    }
}

/// Return a JSON array.
fn json_array<'semantic, 'json, Detail>(
    value: &'json Value,
    detail: Detail,
) -> Result<&'json Vec<Value>, GateError>
where
    Detail: Into<DetailText<'semantic>>,
{
    let detail = detail.into().0;
    match *value {
        | Value::Array(ref rows) => Ok(rows),
        | _ => Err(GateError::operational(detail)),
    }
}

/// Parse one page-balance probe row.
fn page_probe(value: &Value) -> Result<PageProbe, GateError>
{
    let kind = json_field(value, "kind")
        .and_then(|field| json_string(field, "page-balance probe `kind` must be a string"))?;
    let page = json_field(value, "page").and_then(|field| {
        json_i64(field, "page-balance probe `page` must be an integer").map(|value| value.into().0)
    })?;
    let y_mm = json_field(value, "y").and_then(|field| {
        json_f64(field, "page-balance probe `y` must be a number").map(|value| value.into().0)
    })?;
    Ok(PageProbe {
        kind: String::from(kind.as_ref()),
        page,
        y_mm,
    })
}

/// Return a JSON string.
fn json_string<'semantic, 'json, Detail>(
    value: &'json Value,
    detail: Detail,
) -> Result<JsonStringText<'json>, GateError>
where
    Detail: Into<DetailText<'semantic>>,
{
    let detail = detail.into().0;
    match *value {
        | Value::String(ref text) => Ok(JsonStringText(text)),
        | _ => Err(GateError::operational(detail)),
    }
}

/// Return a JSON integer.
fn json_i64<'semantic, Detail>(
    value: &Value,
    detail: Detail,
) -> Result<impl Into<JsonI64Seconds>, GateError>
where
    Detail: Into<DetailText<'semantic>>,
{
    let detail = detail.into().0;
    value.as_i64().ok_or_else(|| GateError::operational(detail))
}

/// Return a JSON number as `f64`.
fn json_f64<'semantic, Detail>(
    value: &Value,
    detail: Detail,
) -> Result<impl Into<JsonF64Millimeters>, GateError>
where
    Detail: Into<DetailText<'semantic>>,
{
    let detail = detail.into().0;
    value.as_f64().ok_or_else(|| GateError::operational(detail))
}

/// Filter probes that open inside the strict bottom band.
fn late_page_probes<BottomMm, BandMm>(
    probes: &[PageProbe],
    bottom_mm: BottomMm,
    band_mm: BandMm,
) -> Vec<PageProbe>
where
    BottomMm: Into<BottomMmMillimeters>,
    BandMm: Into<BandMmMillimeters>,
{
    let band_mm = band_mm.into().0;
    let bottom_mm = bottom_mm.into().0;
    let threshold = bottom_mm - band_mm;
    let mut late_probes = Vec::new();
    for probe in probes {
        if probe.y_mm > threshold {
            late_probes.push(probe.clone());
        }
    }
    return late_probes;
}

/// Map any lower-level probe error to the retained Typst failure phrase.
fn page_balance_probe_failed_from(_error: GateError) -> GateError
{
    page_balance_probe_failed()
}

/// Build the retained page-balance Typst failure.
fn page_balance_probe_failed() -> GateError
{
    GateError::operational(PAGE_BALANCE_TYPST_FAILURE)
}

/// Parse Typst page-balance JSON into probe rows.
fn parse_page_probes<'semantic, Source>(source: Source) -> Result<Vec<PageProbe>, GateError>
where
    Source: Into<SourceText<'semantic>>,
{
    let source = source.into().0;
    let root: Value = serde_json::from_str(source).map_err(|source| GateError::Json {
        source_name: String::from("page-balance typst eval"),
        source,
    })?;
    let value = json_field(&root, "value")?;
    let rows = json_array(value, "page-balance probe `value` must be an array")?;
    let mut probes = Vec::new();
    for row in rows {
        probes.push(page_probe(row)?);
    }
    return Ok(probes);
}

/// Build a page-balance report from decoded probe rows.
///
/// # Contract
/// - ensures: rejects empty probe lists and filters late probes with the strict
///   bottom-page band rule.
/// - provides: a pure model seam shared by the external command wrapper and
///   unit tests.
/// - fails: returns the retained vacuity failure.
/// - panics: none.
///
/// # Errors
/// Returns an operational error for zero probes.
fn page_balance_report_from_probes(probes: &[PageProbe]) -> Result<PageBalanceReport, GateError>
{
    if probes.is_empty() {
        return Err(GateError::operational(PAGE_BALANCE_VACUOUS_FAILURE));
    }
    let late_probes = late_page_probes(probes, PAGE_BOTTOM_MM, PAGE_BOTTOM_BAND_MM);
    Ok(PageBalanceReport {
        probed: probes.len(),
        late_probes,
    })
}

/// Scan supplied Markdown files for unresolved conflict-marker lines.
///
/// # Contract
/// - requires: each path names a Markdown file to be formatted or checked by
///   rumdl, with relative paths resolved against `cwd` for reads.
/// - ensures: visits paths in encounter order and lines in source order,
///   failing on the first line that starts with a retained conflict-marker
///   prefix.
/// - provides: the in-process marker precheck that must complete before rumdl
///   is planned or spawned.
/// - fails: returns [`GateError`] for unreadable files or unresolved marker
///   lines.
/// - panics: none.
///
/// # Errors
/// Returns file-read errors from [`support::read_utf8`] and operational errors
/// for unresolved marker lines.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — clean paths, relative `cwd`, first marker
///   detection, and line-number reporting are killed by command-planning tests.
/// - witness: `commands::tests::conflict_markers_block_rumdl_planning`
/// - witness: `commands::tests::clean_markdown_files_preserve_argument_order`
fn verify_no_conflict_markers(
    paths: &[PathBuf],
    cwd: Option<&Path>,
) -> Result<(), GateError>
{
    for path in paths {
        let read_path = readable_rumdl_path(path, cwd);
        let source = support::read_utf8(read_path.as_ref())?;
        for (line_index, line) in source.lines().enumerate() {
            if let Some(marker) = conflict_marker_line(line).into().0 {
                let line_number = line_index.checked_add(1_usize).ok_or_else(|| {
                    GateError::operational("rumdl conflict-marker line number overflowed")
                })?;
                return Err(conflict_marker_error(path, line_number, marker));
            }
        }
    }
    Ok(())
}

/// Append path arguments in encounter order.
fn append_path_args(
    args: &mut Vec<OsString>,
    paths: &[PathBuf],
)
{
    for path in paths {
        args.push(path.as_os_str().to_os_string());
    }
}

/// Return the filesystem path used for validation reads.
fn readable_rumdl_path<'path>(
    path: &'path Path,
    cwd: Option<&Path>,
) -> Cow<'path, Path>
{
    match (path.is_absolute(), cwd) {
        | (true, _) | (false, None) => Cow::Borrowed(path),
        | (false, Some(root)) => Cow::Owned(root.join(path)),
    }
}

/// Return the retained conflict-marker prefix found at the start of `line`.
fn conflict_marker_line<'semantic, Line>(
    line: Line
) -> impl Into<OptionalConflictMarkerLineText<'static>>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    CONFLICT_MARKER_PREFIXES
        .iter()
        .copied()
        .find(|&marker| line.starts_with(marker))
}

/// Build the operational error for one unresolved conflict-marker line.
fn conflict_marker_error<'semantic, LineNumber, Marker>(
    path: &Path,
    line_number: LineNumber,
    marker: Marker,
) -> GateError
where
    LineNumber: Into<LineNumberNumber>,
    Marker: Into<MarkerText<'semantic>>,
{
    let marker = marker.into().0;
    let line_number = line_number.into().0;
    GateError::operational(format!(
        "rumdl conflict-marker precheck failed: {}:{line_number}: unresolved `{marker}` marker line",
        path.display()
    ))
}

/// Conflict-marker line prefixes rejected before invoking rumdl.
const CONFLICT_MARKER_PREFIXES: [&str; 3] = ["<<<<<<<", "=======", ">>>>>>>"];

#[cfg(test)]
mod tests
{
    //! Unit witnesses for page-balance and guarded-rumdl commands.

    use alloc::vec;
    use core::error::Error;
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;

    use super::*;

    /// Test result used by command parsing checks.
    type TestResult = Result<(), Box<dyn Error>>;
    /// Executable mode used by POSIX command fixtures.
    #[cfg(unix)]
    const EXECUTABLE_MODE: u32 = 0o755;

    /// The bottom-band boundary is strict: `y == bottom - band` is not late.
    #[test]
    fn strict_page_band_boundary_excludes_exact_threshold() -> TestResult
    {
        let source =
            r#"{"value":[{"kind":"exact","page":1,"y":218.0},{"kind":"late","page":2,"y":218.1}]}"#;
        let probes = parse_page_probes(source)?;
        let late = late_page_probes(&probes, PAGE_BOTTOM_MM, PAGE_BOTTOM_BAND_MM);

        assert_eq!(1_usize, late.len());
        assert!(
            late.iter().any(|probe| probe.kind == "late"),
            "probe above the threshold should be reported"
        );
        assert!(
            !late.iter().any(|probe| probe.kind == "exact"),
            "probe exactly on the threshold should be excluded"
        );
        Ok(())
    }

    /// Conflict markers fail during planning, before any rumdl process exists.
    #[test]
    fn conflict_markers_block_rumdl_planning() -> TestResult
    {
        let root = fixture("conflict")?;
        let dirty = PathBuf::from("dirty.md");
        crate::support::HOST_FILESYSTEM
            .write(root.join(&dirty), "# Dirty\n<<<<<<< HEAD\nbody\n")?;
        let paths = vec![dirty];

        let error = rumdl_command_plan(RumdlMode::Fmt, &paths, Some(&root))
            .err()
            .ok_or_else(|| GateError::operational("conflict marker unexpectedly planned rumdl"))?;
        let rendered = error.to_string();

        assert!(
            rendered.contains("dirty.md:2"),
            "error should report the caller path and line number: {rendered}"
        );
        assert!(
            rendered.contains("<<<<<<<"),
            "error should report the marker prefix: {rendered}"
        );
        let wrapper_error = run_guarded_rumdl(RumdlMode::Fmt, &paths, Some(&root))
            .err()
            .ok_or_else(|| GateError::operational("conflict marker unexpectedly ran rumdl"))?;
        assert!(
            wrapper_error.to_string().contains("dirty.md:2"),
            "guarded wrapper should surface the same pre-spawn marker error"
        );
        Ok(())
    }

    /// Clean Markdown paths keep caller ordering in the rumdl argv.
    #[test]
    fn clean_markdown_files_preserve_argument_order() -> TestResult
    {
        let root = fixture("clean-order")?;
        let first = PathBuf::from("b.md");
        let second = PathBuf::from("a.md");
        crate::support::HOST_FILESYSTEM.write(root.join(&first), "# B\n")?;
        crate::support::HOST_FILESYSTEM.write(root.join(&second), "# A\n")?;
        let paths = vec![first.clone(), second.clone()];

        let plan = rumdl_command_plan(RumdlMode::parse("check")?, &paths, Some(&root))?;

        assert_eq!(plan.rumdl_program, OsString::from("rumdl"));
        assert_eq!(
            vec![
                OsString::from("check"),
                first.as_os_str().to_os_string(),
                second.as_os_str().to_os_string(),
            ],
            plan.rumdl_args
        );
        Ok(())
    }

    /// Build a clean temporary fixture directory for `name`.
    fn fixture<'semantic, Name>(name: Name) -> Result<PathBuf, Box<dyn Error>>
    where
        Name: Into<NameText<'semantic>>,
    {
        let name = name.into().0;
        let root = std::env::temp_dir().join(format!(
            "gandr-workflow-gates-docs-commands-{}-{name}",
            std::process::id()
        ));
        crate::support::HOST_FILESYSTEM.remove_dir_if_exists(&root)?;
        crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
        Ok(root)
    }

    /// Rumdl mode parsing, empty argv, and read-path resolution are exact.
    #[test]
    fn rumdl_modes_and_empty_paths_are_exact() -> TestResult
    {
        assert_eq!(
            "fmt",
            crate::semantic_value::<AsStrText<'_>, _>(RumdlMode::parse("fmt")?.as_str()).0
        );
        assert_eq!(
            "check",
            crate::semantic_value::<AsStrText<'_>, _>(RumdlMode::parse("check")?.as_str()).0
        );
        let unsupported = RumdlMode::parse("lint")
            .err()
            .ok_or_else(|| GateError::operational("unsupported mode unexpectedly parsed"))?;
        assert!(
            unsupported
                .to_string()
                .contains("unsupported rumdl mode: lint"),
            "unsupported mode should stay typed as usage text: {unsupported}"
        );

        let empty_paths: Vec<PathBuf> = Vec::new();
        let plan = rumdl_command_plan(RumdlMode::Fmt, &empty_paths, None)?;
        assert_eq!(vec![OsString::from("fmt")], plan.rumdl_args);

        let relative = PathBuf::from("loose.md");
        let read_path = readable_rumdl_path(&relative, None);
        assert_eq!(read_path.as_ref(), relative.as_path());
        Ok(())
    }

    /// Typst command construction and page report predicates are stable.
    #[test]
    fn typst_probe_args_and_page_report_predicates_are_exact() -> TestResult
    {
        assert_eq!(
            vec![
                OsString::from("eval"),
                OsString::from("query(<layout-probe>)"),
                OsString::from("--in"),
                OsString::from("docs/manual/main.typ"),
                OsString::from("--root"),
                OsString::from("."),
                OsString::from("--format"),
                OsString::from("json"),
            ],
            typst_probe_args()
        );

        let clean = page_balance_report_from_probes(&[])
            .err()
            .ok_or_else(|| GateError::operational("empty probes unexpectedly produced a report"))?;
        assert!(
            clean.to_string().contains(PAGE_BALANCE_VACUOUS_FAILURE),
            "empty probe lists should keep the retained vacuity failure"
        );
        let probes = [PageProbe {
            kind: String::from("late"),
            page: 1_i64,
            y_mm: PAGE_BOTTOM_MM,
        }];
        let late = page_balance_report_from_probes(&probes)?;
        assert_eq!(1_usize, late.probed);
        assert!(late.has_late_probes().into().0);
        Ok(())
    }

    /// Page-balance JSON failures keep their typed error family and details.
    #[test]
    fn page_probe_json_shape_errors_are_typed() -> TestResult
    {
        let malformed = parse_page_probes("{")
            .err()
            .ok_or_else(|| GateError::operational("malformed JSON unexpectedly parsed"))?;
        match malformed {
            | GateError::Json { source_name, .. } => {
                assert_eq!("page-balance typst eval", source_name);
            },
            | other => {
                return Err(GateError::operational(format!(
                    "malformed JSON returned wrong error family: {other}"
                ))
                .into());
            },
        }

        let cases = [
            (
                r#"{"value":{}}"#,
                "page-balance probe `value` must be an array",
            ),
            (
                r#"{"value":[[]]}"#,
                "page-balance probe row must be an object",
            ),
            (
                r#"{"value":[{"kind":7,"page":1,"y":2.0}]}"#,
                "page-balance probe `kind` must be a string",
            ),
            (
                r#"{"value":[{"kind":"x","page":"one","y":2.0}]}"#,
                "page-balance probe `page` must be an integer",
            ),
            (
                r#"{"value":[{"kind":"x","page":1,"y":"low"}]}"#,
                "page-balance probe `y` must be a number",
            ),
        ];
        for (source, expected) in cases {
            let error = parse_page_probes(source)
                .err()
                .ok_or_else(|| GateError::operational("invalid probe JSON unexpectedly parsed"))?;
            let rendered = error.to_string();
            assert!(
                rendered.contains(expected),
                "error `{rendered}` did not contain `{expected}`"
            );
        }

        assert_eq!(
            page_balance_probe_failed().to_string(),
            page_balance_probe_failed_from(GateError::operational("masked")).to_string()
        );
        Ok(())
    }

    /// Injectable command programs cover page-balance process success/failure.
    #[cfg(unix)]
    #[test]
    fn injectable_page_balance_program_reports_success_vacuity_and_failure() -> TestResult
    {
        let root = fixture("typst-program")?;
        let success_program = executable_script(
            &root,
            "typst-success",
            "#!/bin/sh\nprintf '%s\\n' '{\"value\":[{\"kind\":\"late\",\"page\":1,\"y\":272.0},{\"kind\":\"safe\",\"page\":2,\"y\":10.0}]}'\n",
        )?;
        let report = run_page_balance_program(success_program.as_os_str(), Some(&root))?;
        assert_eq!(2_usize, report.probed);
        assert!(
            report
                .late_probes
                .iter()
                .any(|probe| probe.kind == "late" && probe.page == 1_i64),
            "late probe from generated JSON should be retained"
        );

        let empty_program = executable_script(
            &root,
            "typst-empty",
            "#!/bin/sh\nprintf '%s\\n' '{\"value\":[]}'\n",
        )?;
        let empty = run_page_balance_program(empty_program.as_os_str(), Some(&root))
            .err()
            .ok_or_else(|| GateError::operational("empty probe output unexpectedly passed"))?;
        assert!(
            empty.to_string().contains(PAGE_BALANCE_VACUOUS_FAILURE),
            "empty probe output should fail with the retained vacuity detail"
        );

        let malformed_program =
            executable_script(&root, "typst-malformed", "#!/bin/sh\nprintf '{'\n")?;
        let malformed = run_page_balance_program(malformed_program.as_os_str(), Some(&root))
            .err()
            .ok_or_else(|| GateError::operational("malformed probe output unexpectedly passed"))?;
        assert!(
            malformed.to_string().contains(PAGE_BALANCE_TYPST_FAILURE),
            "malformed process JSON should be normalized to the Typst failure"
        );

        let failing_program = executable_script(&root, "typst-failing", "#!/bin/sh\nexit 2\n")?;
        let failed = run_page_balance_program(failing_program.as_os_str(), Some(&root))
            .err()
            .ok_or_else(|| GateError::operational("failing typst program unexpectedly passed"))?;
        assert!(
            failed.to_string().contains(PAGE_BALANCE_TYPST_FAILURE),
            "nonzero Typst status should keep the retained failure"
        );
        Ok(())
    }

    /// Injectable rumdl program preserves argv order and returns raw status.
    #[cfg(unix)]
    #[test]
    fn injectable_rumdl_program_returns_status_after_precheck() -> TestResult
    {
        let root = fixture("rumdl-program")?;
        let clean = PathBuf::from("clean.md");
        crate::support::HOST_FILESYSTEM.write(root.join(&clean), "# Clean\n")?;
        let rumdl = executable_script(&root, "rumdl-status", "#!/bin/sh\nexit 7\n")?;
        let paths = vec![clean];

        let outcome =
            run_guarded_rumdl_program(rumdl.as_os_str(), RumdlMode::Check, &paths, Some(&root))?;
        match outcome {
            | RumdlOutcome::RumdlStatus { status } => {
                assert_eq!(Some(7_i32), status.code());
            },
        }
        Ok(())
    }

    /// Write an executable shell fixture on Unix hosts.
    #[cfg(unix)]
    fn executable_script<'semantic, Name, Source>(
        root: &Path,
        name: Name,
        source: Source,
    ) -> Result<PathBuf, Box<dyn Error>>
    where
        Name: Into<NameText<'semantic>>,
        Source: Into<SourceText<'semantic>>,
    {
        let source = source.into().0;
        let name = name.into().0;
        let script_path = root.join(name);
        crate::support::HOST_FILESYSTEM.write(&script_path, source)?;
        let mut permissions = crate::support::HOST_FILESYSTEM
            .metadata(&script_path)?
            .permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, EXECUTABLE_MODE);
        crate::support::HOST_FILESYSTEM.set_permissions(&script_path, permissions)?;
        Ok(script_path)
    }
}
