//! Weekly maintenance range selection and watermark persistence.
//!
//! The module owns the typed Rust replacement for
//! `scripts/maintenance-range.nu`: resolve a lower bound for weekly campaigns,
//! append it to GitHub Actions output, and atomically advance the runner-local
//! success watermark.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;

use crate::GateError;
use crate::support;

crate::semantic_str!(pub struct TextText);
crate::semantic_copy!(pub struct SecondsSeconds(i64));
crate::semantic_str!(pub struct ContextText);
crate::semantic_bytes!(pub struct BytesBytes);
crate::semantic_optional_str!(pub struct OptionalExplicitFromText);
crate::semantic_str!(pub struct LabelText);
crate::semantic_str!(pub struct SourceNameText);
crate::semantic_optional_str!(pub struct OptionalTextText);
crate::semantic_str!(pub struct NameText);
crate::semantic_copy!(pub struct TimestampSeconds(i64));
crate::semantic_copy!(pub struct NCount(usize));
crate::semantic_str!(pub struct AsStrText);
crate::semantic_copy!(pub struct AncestorFlag(bool));

crate::semantic_bytes!(pub struct WatermarkBytes);
crate::semantic_copy!(pub struct WatermarkByteCount(usize));

/// Deterministic watermark JSON bytes.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatermarkJsonBytes(Vec<u8>);

impl WatermarkJsonBytes
{
    /// Borrow the serialized watermark bytes through a semantic boundary.
    #[inline]
    #[must_use]
    pub fn bytes(&self) -> WatermarkBytes<'_>
    {
        WatermarkBytes(self.0.as_slice())
    }

    /// Return the byte length.
    #[inline]
    #[must_use]
    pub fn len(&self) -> WatermarkByteCount
    {
        WatermarkByteCount(self.0.len())
    }
}

crate::semantic_copy!(pub struct WatermarkSchemaVersion(u64));

/// Git executable name used through sanitized support calls.
const GIT_PROGRAM: &str = "git";

/// Minimum age, in seconds, for an automatic maintenance base.
const MINIMUM_AGE_SECONDS: i64 = 691_200;

/// One-second offset preserving the legacy inclusive cutoff over Git's
/// exclusive `--before`.
const EXCLUSIVE_BEFORE_OFFSET_SECONDS: i64 = 1;

/// Portable JSON watermark schema version.
pub const WATERMARK_SCHEMA: u64 = 1;

/// Validated Git object ID accepted by the maintenance gate.
///
/// # Contract
/// - requires: `text` is an object ID reported by Git or loaded from the
///   watermark file.
/// - ensures: accepted IDs contain only ASCII hexadecimal digits and have a
///   length from 40 through 64 bytes, matching the legacy SHA-1/SHA-256 window.
/// - provides: an owned object ID string that is safe to pass as a single Git
///   operand.
/// - fails: returns [`GateError::Operational`] when validation fails.
/// - panics: none.
///
/// # Errors
/// Returns an operational maintenance error when `text` is empty, too short,
/// too long, or contains a non-hexadecimal character.
///
/// # Adequacy
/// - hypothesis: L3 only — length floor, length ceiling, hex alphabet, and
///   ordinary accepted ID decisions are separated by exact boundary fixtures.
/// - witness: `maintenance::tests::invalid_oid_and_timestamp_are_rejected`
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CommitId
{
    /// Canonical object ID text retained exactly as validated.
    text: String,
}

impl CommitId
{
    /// Validate and own a Git object ID.
    ///
    /// # Contract
    /// - requires: `text` is the textual object ID candidate to validate.
    /// - ensures: returns a [`CommitId`] only for 40-to-64-byte ASCII hex
    ///   strings.
    /// - provides: the same object ID bytes without normalization.
    /// - fails: returns [`GateError::Operational`] for invalid shape.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational maintenance error when the object ID has an
    /// invalid length or contains a non-hexadecimal character.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one accepted SHA-1-shaped ID, one too-short ID,
    ///   one too-long ID, and one non-hex ID kill the validation branches.
    /// - witness: `maintenance::tests::invalid_oid_and_timestamp_are_rejected`
    #[inline]
    pub fn new<'semantic, Text>(text: Text) -> Result<Self, GateError>
    where
        Text: Into<TextText<'semantic>>,
    {
        let text = text.into().0;
        Self::new_with_detail(text, || format!("invalid commit object ID `{text}`"))
    }

    /// Borrow the validated object ID text.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> impl Into<AsStrText<'_>>
    {
        &self.text
    }

    /// Validate `text` with a caller-supplied diagnostic detail.
    ///
    /// # Contract
    /// - requires: `detail` builds the complete maintenance diagnostic when
    ///   validation fails.
    /// - ensures: validation is identical to [`CommitId::new`].
    /// - provides: context-sensitive diagnostics for Git and watermark callers.
    /// - fails: returns [`GateError::Operational`] with `detail()` when the ID
    ///   is not 40-to-64-byte ASCII hex.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational maintenance error carrying `detail()` for invalid
    /// object ID shape.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — the public validation witnesses exercise this
    ///   helper through both generic and contextual parse paths.
    /// - witness: `maintenance::tests::invalid_oid_and_timestamp_are_rejected`
    fn new_with_detail<'semantic, Text, Detail>(
        text: Text,
        detail: Detail,
    ) -> Result<Self, GateError>
    where
        Text: Into<TextText<'semantic>>,
        Detail: FnOnce() -> String,
    {
        let text = text.into().0;
        let length = text.len();
        let valid_length = (40 ..= 64).contains(&length);
        let valid_hex = text.chars().all(|character| character.is_ascii_hexdigit());
        if valid_length && valid_hex {
            return Ok(Self {
                text: String::from(text),
            });
        }
        Err(maintenance_error(detail()))
    }
}

impl fmt::Display for CommitId
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(&self.text)
    }
}

/// Validated Git revision token passed as one `rev-parse` operand.
///
/// # Contract
/// - requires: `text` is supplied by trusted CLI parsing or a fixture.
/// - ensures: accepted tokens are non-empty, have no surrounding whitespace, do
///   not start with `-`, and contain no ASCII control characters.
/// - provides: a narrowly validated revision token still broad enough for
///   ordinary Git syntax such as `HEAD`, tags, branch names, and `HEAD~1`.
/// - fails: returns [`GateError::Operational`] when validation fails.
/// - panics: none.
///
/// # Errors
/// Returns an operational maintenance error when the token is empty, has
/// surrounding whitespace, starts with a dash, or contains control characters.
///
/// # Adequacy
/// - hypothesis: L3 only — accepted `HEAD`, empty, surrounding-whitespace,
///   option-looking, and control-character fixtures distinguish every guard.
/// - witness: `maintenance::tests::precedence_prefers_explicit_then_watermark_then_auto`
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct GitRef
{
    /// Original validated revision token.
    text: String,
}

impl GitRef
{
    /// Validate and own a Git revision token.
    ///
    /// # Contract
    /// - requires: `text` is a single Git revision token, not shell-expanded
    ///   source text.
    /// - ensures: returns a token accepted by this module's command builder.
    /// - provides: an owned token for `rev-parse --end-of-options`.
    /// - fails: returns [`GateError::Operational`] for invalid token shape.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational maintenance error when the token is empty, has
    /// surrounding whitespace, starts with `-`, or contains control characters.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — valid and invalid token witnesses cover each
    ///   syntactic guard independently.
    /// - witness: `maintenance::tests::precedence_prefers_explicit_then_watermark_then_auto`
    #[inline]
    pub fn new<'semantic, Text>(text: Text) -> Result<Self, GateError>
    where
        Text: Into<TextText<'semantic>>,
    {
        let text = text.into().0;
        if text.is_empty() {
            return Err(maintenance_error("Git ref is empty"));
        }
        if text.trim() != text {
            return Err(maintenance_error(format!(
                "Git ref `{text}` contains surrounding whitespace",
            )));
        }
        if text.starts_with('-') {
            return Err(maintenance_error(format!(
                "Git ref `{text}` looks like a command option",
            )));
        }
        if text.chars().any(char::is_control) {
            return Err(maintenance_error(format!(
                "Git ref `{text}` contains a control character",
            )));
        }
        Ok(Self {
            text: String::from(text),
        })
    }

    /// Build the default current-head token.
    ///
    /// # Contract
    /// - ensures: returns the literal `HEAD` token.
    /// - provides: the canonical current-head reference used by validations.
    /// - fails: never fails because `HEAD` is statically valid.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns no error for the statically validated `HEAD` token; the `Result`
    /// shape is retained to share validation call paths with arbitrary refs.
    ///
    /// # Adequacy
    /// - hypothesis: L1 — this delegates to the same validation path used by
    ///   callers; current-head behavioral witnesses observe its effect.
    /// - witness: `maintenance::tests::current_head_expectation_rejects_stale_upper`
    #[inline]
    pub fn head() -> Result<Self, GateError>
    {
        Self::new("HEAD")
    }

    /// Borrow the revision token text.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> impl Into<AsStrText<'_>>
    {
        &self.text
    }

    /// Return whether trimming an optional CLI token leaves a non-empty ref.
    ///
    /// # Contract
    /// - requires: `text` is an optional CLI `--from` style value.
    /// - ensures: returns `Ok(None)` for absent or whitespace-only values and a
    ///   validated ref otherwise.
    /// - provides: the legacy explicit-from trim semantics.
    /// - fails: returns [`GateError::Operational`] when the trimmed token is
    ///   not a valid [`GitRef`].
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational maintenance error when the non-empty trimmed
    /// token violates [`GitRef`] validation.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — `None`, whitespace, valid, and invalid explicit
    ///   fixtures distinguish absent-vs-present precedence.
    /// - witness: `maintenance::tests::precedence_prefers_explicit_then_watermark_then_auto`
    fn trimmed_optional<'semantic, Text>(text: Text) -> Result<Option<Self>, GateError>
    where
        Text: Into<OptionalTextText<'semantic>>,
    {
        let text = text.into().0;
        let Some(value) = text
        else {
            return Ok(None);
        };
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self::new(trimmed)?))
    }

    /// Build a ref token from an already validated commit ID.
    fn from_commit(commit: &CommitId) -> Self
    {
        Self {
            text: String::from(crate::semantic_value::<AsStrText<'_>, _>(commit.as_str()).0),
        }
    }
}

impl fmt::Display for GitRef
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(&self.text)
    }
}

/// Commit timestamp in whole Unix seconds.
///
/// # Contract
/// - requires: timestamp text comes from `git show --format=%ct` or a fixture.
/// - ensures: accepted timestamps parse exactly as signed 64-bit seconds.
/// - provides: checked cutoff arithmetic for automatic maintenance selection.
/// - fails: returns [`GateError::Operational`] for malformed or overflowing
///   timestamp text or cutoff arithmetic overflow.
/// - panics: none.
///
/// # Errors
/// Returns an operational maintenance error when parsing fails or checked
/// arithmetic overflows.
///
/// # Adequacy
/// - hypothesis: L3 only — valid, non-numeric, and overflow timestamp fixtures
///   plus cutoff-boundary witnesses kill every branch.
/// - witness: `maintenance::tests::invalid_oid_and_timestamp_are_rejected`
/// - witness: `maintenance::tests::merge_topology_uses_first_parent_cutoff`
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct CommitTimestamp
{
    /// Unix timestamp seconds.
    seconds: i64,
}

impl CommitTimestamp
{
    /// Build a timestamp from already parsed seconds.
    #[must_use]
    #[inline]
    pub fn new<Seconds>(seconds: Seconds) -> Self
    where
        Seconds: Into<SecondsSeconds>,
    {
        let seconds = seconds.into().0;
        Self { seconds }
    }

    /// Parse Git timestamp output for `commit`.
    ///
    /// # Contract
    /// - requires: `text` is the raw `git show --format=%ct` stdout.
    /// - ensures: surrounding command-output whitespace is ignored.
    /// - provides: a checked integer timestamp for cutoff arithmetic.
    /// - fails: returns [`GateError::Operational`] when parsing fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational maintenance error when `text.trim()` is not a
    /// signed 64-bit integer.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — numeric and nonnumeric fixtures observe the
    ///   exact parse success/failure split.
    /// - witness: `maintenance::tests::invalid_oid_and_timestamp_are_rejected`
    #[inline]
    pub fn parse_git_output<'semantic, Text>(
        commit: &CommitId,
        text: Text,
    ) -> Result<Self, GateError>
    where
        Text: Into<TextText<'semantic>>,
    {
        let text = text.into().0;
        let trimmed = text.trim();
        match trimmed.parse::<i64>() {
            | Ok(seconds) => Ok(Self { seconds }),
            | Err(_error) => Err(maintenance_error(format!(
                "Git returned an invalid committer timestamp for `{commit}`",
            ))),
        }
    }

    /// Borrow the timestamp seconds.
    #[must_use]
    #[inline]
    pub const fn seconds(self) -> impl Into<SecondsSeconds>
    {
        SecondsSeconds(self.seconds)
    }

    /// Compute the exclusive Git `--before` value for the eight-day cutoff.
    ///
    /// # Contract
    /// - requires: `self` is the selected head commit timestamp.
    /// - ensures: returns `self - 691200 + 1` using checked arithmetic,
    ///   preserving the legacy inclusive cutoff over Git's exclusive `--before`
    ///   behavior.
    /// - provides: the timestamp used in `git rev-list --before=<value> +0000`.
    /// - fails: returns [`GateError::Operational`] if either checked operation
    ///   overflows.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational maintenance error when cutoff subtraction or the
    /// one-second exclusive-before addition overflows `i64`.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — the merge-topology fixture observes the `+1`
    ///   boundary, while invalid timestamp fixtures kill overflow/error paths.
    /// - witness: `maintenance::tests::merge_topology_uses_first_parent_cutoff`
    #[inline]
    pub fn exclusive_before(self) -> Result<Self, GateError>
    {
        let cutoff = self
            .seconds
            .checked_sub(MINIMUM_AGE_SECONDS)
            .ok_or_else(|| {
                maintenance_error("maintenance timestamp cutoff underflowed signed seconds")
            })?;
        let exclusive_before = cutoff
            .checked_add(EXCLUSIVE_BEFORE_OFFSET_SECONDS)
            .ok_or_else(|| {
                maintenance_error("maintenance timestamp cutoff overflowed signed seconds")
            })?;
        Ok(Self {
            seconds: exclusive_before,
        })
    }
}

/// Head validation policy for range and advancement entry points.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HeadExpectation
{
    /// The supplied head may be any commit-ish revision.
    AnyCommit,
    /// The supplied head must resolve to the same commit as `HEAD`.
    CurrentHead,
}

/// Planned source for the lower bound before any Git resolution occurs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlannedBaseSource
{
    /// Operator-supplied bootstrap or catch-up lower bound.
    Explicit(GitRef),
    /// Runner-local success watermark path.
    Watermark(PathBuf),
    /// Eight-day first-parent automatic selection.
    Automatic,
}

/// Resolved source for the lower bound after filesystem watermark decoding.
#[derive(Clone, Debug, Eq, PartialEq)]
enum BaseSource
{
    /// Operator-supplied bootstrap or catch-up lower bound.
    Explicit(GitRef),
    /// Commit decoded from the runner-local success watermark.
    Watermark(CommitId),
    /// Eight-day first-parent automatic selection.
    Automatic,
}

/// User-visible source category for a selected maintenance base.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaintenanceBaseKind
{
    /// Base selected from an explicit `from` ref.
    Explicit,
    /// Base selected from the success watermark.
    Watermark,
    /// Base selected by the eight-day first-parent rule.
    Automatic,
}

/// Parsed watermark state.
///
/// # Contract
/// - requires: the source JSON is an object with schema `1` and an `upper`
///   commit ID.
/// - ensures: only schema-1 watermarks with validated upper commits are
///   accepted.
/// - provides: a portable replacement for the previous NUON watermark.
/// - fails: returns [`GateError::Operational`] for malformed shape and
///   [`GateError::Operational`] for invalid upper object IDs.
/// - panics: none.
///
/// # Errors
/// Returns an operational maintenance error when the JSON is malformed, the
/// schema is not `1`, `upper` is absent, or `upper` is not a valid object ID.
///
/// # Adequacy
/// - hypothesis: L3 only — missing file, invalid JSON/shape, wrong schema,
///   missing upper, and valid upper fixtures distinguish all guards.
/// - witness: `maintenance::tests::missing_and_invalid_watermarks_fail_closed`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Watermark
{
    /// Schema version carried by the file.
    schema: u64,
    /// Last fully successful upper commit.
    upper: CommitId,
}

impl Watermark
{
    /// Borrow the validated upper commit.
    #[must_use]
    #[inline]
    pub fn upper(&self) -> &CommitId
    {
        &self.upper
    }

    /// Return the parsed schema version.
    #[must_use]
    #[inline]
    pub const fn schema(&self) -> impl Into<WatermarkSchemaVersion>
    {
        WatermarkSchemaVersion(self.schema)
    }
}

/// Successful maintenance range selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaintenanceRangeSelection
{
    /// Lower-bound commit selected for the next range.
    pub base: CommitId,
    /// Upper commit used while selecting the range.
    pub head: CommitId,
    /// Selection source that won precedence.
    pub source: MaintenanceBaseKind,
}

/// Request for resolving and publishing a maintenance range.
pub struct MaintenanceRangeRequest<'request>
{
    /// GitHub Actions output file that receives `base=<oid>`.
    pub github_output: &'request Path,
    /// Upper revision for the next maintenance range.
    pub head: GitRef,
    /// Explicit bootstrap/catch-up lower-bound ref.
    pub explicit_from: Option<GitRef>,
    /// Runner-local last-success watermark path.
    pub watermark: Option<&'request Path>,
    /// Repository working directory for Git calls.
    pub cwd: Option<&'request Path>,
    /// Whether `head` must equal current `HEAD`.
    pub head_expectation: HeadExpectation,
}

impl<'request> MaintenanceRangeRequest<'request>
{
    /// Build a maintenance range request for CLI and integration callers.
    ///
    /// # Contract
    /// - requires: `github_output` names the GitHub Actions output file that
    ///   receives the selected base.
    /// - ensures: stores every argument unchanged for
    ///   [`resolve_and_append_github_output`].
    /// - provides: cross-crate construction while the public request type
    ///   remains non-exhaustive.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — range-selection witnesses observe the exact fields
    ///   supplied through this request shape.
    /// - witness: `maintenance::tests::precedence_prefers_explicit_then_watermark_then_auto`
    #[inline]
    #[must_use]
    pub fn new(
        github_output: &'request Path,
        head: GitRef,
        explicit_from: Option<GitRef>,
        watermark: Option<&'request Path>,
        cwd: Option<&'request Path>,
        head_expectation: HeadExpectation,
    ) -> Self
    {
        Self {
            github_output,
            head,
            explicit_from,
            watermark,
            cwd,
            head_expectation,
        }
    }
}

/// Request for atomically advancing the success watermark.
pub struct MaintenanceAdvanceRequest<'request>
{
    /// Watermark path to replace atomically.
    pub watermark: &'request Path,
    /// Successful upper revision whose exact commit becomes the next base.
    pub to: GitRef,
    /// Repository working directory for Git calls.
    pub cwd: Option<&'request Path>,
    /// Whether `to` must equal current `HEAD`.
    pub head_expectation: HeadExpectation,
}

impl<'request> MaintenanceAdvanceRequest<'request>
{
    /// Build a maintenance watermark-advance request for CLI and integration
    /// callers.
    ///
    /// # Contract
    /// - requires: `watermark` names the state file to replace atomically and
    ///   `to` names the successful upper revision.
    /// - ensures: stores every argument unchanged for [`advance_watermark`].
    /// - provides: cross-crate construction while the public request type
    ///   remains non-exhaustive.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — watermark-advance witnesses observe the exact path,
    ///   upper ref, working directory, and head expectation in the request.
    /// - witness: `maintenance::tests::atomic_advancement_writes_schema_one_upper`
    #[inline]
    #[must_use]
    pub fn new(
        watermark: &'request Path,
        to: GitRef,
        cwd: Option<&'request Path>,
        head_expectation: HeadExpectation,
    ) -> Self
    {
        Self {
            watermark,
            to,
            cwd,
            head_expectation,
        }
    }
}

/// Git behavior required by the range planner.
trait MaintenanceGit
{
    /// Resolve a revision token to a commit object ID.
    fn resolve_commit<'semantic, Context>(
        &mut self,
        reference: &GitRef,
        context: Context,
    ) -> Result<CommitId, GateError>
    where
        Context: Into<ContextText<'semantic>>;

    /// Read a commit's committer timestamp.
    fn committer_timestamp(
        &mut self,
        commit: &CommitId,
    ) -> Result<CommitTimestamp, GateError>;

    /// Return the first first-parent commit before an exclusive timestamp.
    fn first_parent_before(
        &mut self,
        head: &CommitId,
        exclusive_before: CommitTimestamp,
    ) -> Result<Option<CommitId>, GateError>;

    /// Return whether `base` is an ancestor of `head`.
    fn is_ancestor(
        &mut self,
        base: &CommitId,
        head: &CommitId,
    ) -> Result<impl Into<AncestorFlag>, GateError>;
}

/// Filesystem behavior required for atomic watermark publication.
trait WatermarkSink
{
    /// Ensure the parent directory for `path` exists.
    fn ensure_parent_directory(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>;

    /// Atomically write the full watermark byte payload to `path`.
    fn write_atomic<'semantic, Bytes>(
        &mut self,
        path: &Path,
        bytes: Bytes,
    ) -> Result<(), GateError>
    where
        Bytes: Into<BytesBytes<'semantic>>;
}

/// Production Git adapter backed by sanitized support calls.
#[repr(transparent)]
struct SupportGit<'cwd>
{
    /// Working directory supplied to support command execution.
    cwd: Option<&'cwd Path>,
}

impl MaintenanceGit for SupportGit<'_>
{
    fn resolve_commit<'semantic, Context>(
        &mut self,
        reference: &GitRef,
        context: Context,
    ) -> Result<CommitId, GateError>
    where
        Context: Into<ContextText<'semantic>>,
    {
        let context = context.into().0;
        let peeled = format!(
            "{}^{{commit}}",
            crate::semantic_value::<AsStrText<'_>, _>(reference.as_str()).0
        );
        let args = Vec::from([
            OsString::from("rev-parse"),
            OsString::from("--verify"),
            OsString::from("--quiet"),
            OsString::from("--end-of-options"),
            OsString::from(peeled),
        ]);
        let text = git_output(self.cwd, &args, context)?;
        CommitId::new_with_detail(text.trim(), || {
            format!("Git returned an invalid object ID while {context}")
        })
    }

    fn committer_timestamp(
        &mut self,
        commit: &CommitId,
    ) -> Result<CommitTimestamp, GateError>
    {
        let args = Vec::from([
            OsString::from("show"),
            OsString::from("--no-patch"),
            OsString::from("--format=%ct"),
            OsString::from(commit.as_str().into().0),
        ]);
        let text = git_output(
            self.cwd,
            &args,
            &format!("cannot read the committer timestamp for `{commit}`"),
        )?;
        CommitTimestamp::parse_git_output(commit, &text)
    }

    fn first_parent_before(
        &mut self,
        head: &CommitId,
        exclusive_before: CommitTimestamp,
    ) -> Result<Option<CommitId>, GateError>
    {
        let before = format!("--before={} +0000", exclusive_before.seconds().into().0);
        let args = Vec::from([
            OsString::from("rev-list"),
            OsString::from("--first-parent"),
            OsString::from("--max-count=1"),
            OsString::from(before),
            OsString::from(head.as_str().into().0),
        ]);
        let text = git_output(
            self.cwd,
            &args,
            &format!("cannot enumerate first-parent commits reachable from `{head}`"),
        )?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let base = CommitId::new_with_detail(trimmed, || {
            String::from("Git returned an invalid maintenance base object ID")
        })?;
        Ok(Some(base))
    }

    fn is_ancestor(
        &mut self,
        base: &CommitId,
        head: &CommitId,
    ) -> Result<impl Into<AncestorFlag>, GateError>
    {
        let args = Vec::from([
            OsString::from("merge-base"),
            OsString::from("--is-ancestor"),
            OsString::from(base.as_str().into().0),
            OsString::from(head.as_str().into().0),
        ]);
        let status = support::run_status(OsStr::new(GIT_PROGRAM), &args, self.cwd, true).map_err(
            |error| {
                maintenance_error(format!(
                    "cannot validate ancestry between `{base}` and `{head}`: {error}",
                ))
            },
        )?;
        Ok(status.success())
    }
}

/// Production watermark sink backed by atomic support writes.
struct SupportWatermarkSink;

impl WatermarkSink for SupportWatermarkSink
{
    fn ensure_parent_directory(
        &mut self,
        path: &Path,
    ) -> Result<(), GateError>
    {
        let Some(parent) = path.parent()
        else {
            return Ok(());
        };
        if parent.as_os_str().is_empty() {
            return Ok(());
        }
        crate::support::HOST_FILESYSTEM
            .create_dir_all(parent)
            .map_err(|source| {
                maintenance_error(format!(
                    "cannot create watermark directory `{}`: {source}",
                    parent.display(),
                ))
            })
    }

    fn write_atomic<'semantic, Bytes>(
        &mut self,
        path: &Path,
        bytes: Bytes,
    ) -> Result<(), GateError>
    where
        Bytes: Into<BytesBytes<'semantic>>,
    {
        let bytes = bytes.into().0;
        support::write_atomic(path, bytes).map_err(|error| {
            maintenance_error(format!(
                "cannot advance weekly success watermark `{}`: {error}",
                path.display(),
            ))
        })
    }
}

/// Resolve a range and append `base=<oid>` to GitHub Actions output.
///
/// # Contract
/// - requires: `request` contains validated refs and an output path writable by
///   the caller's environment.
/// - ensures: appends exactly one `base=<oid>` line after successful range
///   selection.
/// - provides: the retained precedence `explicit --from` > watermark >
///   eight-day first-parent automatic base.
/// - fails: returns [`GateError`] if Git resolution, watermark parsing,
///   ancestry validation, current-head validation, or output append fails.
/// - panics: none.
/// - intension: executes Git commands sequentially and never starts parallel
///   workers.
///
/// # Errors
/// Returns operational maintenance errors for invalid Git state, malformed
/// watermark state, failed current-head checks, failed ancestry checks, and
/// append failures. Propagates support-layer command errors after adding
/// maintenance context.
///
/// # Adequacy
/// - hypothesis: L3 only — source-precedence, first-parent topology,
///   no-old-base, non-ancestor, malformed watermark, and current-head witnesses
///   observe every branch before the append side effect.
/// - witness: `maintenance::tests::precedence_prefers_explicit_then_watermark_then_auto`
/// - witness: `maintenance::tests::merge_topology_uses_first_parent_cutoff`
/// - witness: `maintenance::tests::no_old_base_fails_closed`
/// - witness: `maintenance::tests::non_ancestor_ranges_fail_closed`
/// - witness: `maintenance::tests::current_head_expectation_rejects_stale_upper`
#[inline]
pub fn resolve_and_append_github_output(
    request: &MaintenanceRangeRequest<'_>
) -> Result<MaintenanceRangeSelection, GateError>
{
    let planned = planned_source_from_request(request)?;
    let source = resolve_planned_source(planned)?;
    let mut git = SupportGit { cwd: request.cwd };
    let selection =
        resolve_range_with_git(&mut git, &request.head, request.head_expectation, source)?;
    append_github_output(request.github_output, &selection.base)?;
    Ok(selection)
}

/// Atomically advance the runner-local success watermark.
///
/// # Contract
/// - requires: `request.to` names the successful upper revision and
///   `request.watermark` is the target state path.
/// - ensures: writes JSON object `{"schema":1,"upper":"<oid>"}` atomically to
///   the watermark path after resolving `to`.
/// - provides: typed portable JSON state intentionally replacing NUON.
/// - fails: returns [`GateError`] when Git resolution, optional current-head
///   validation, parent-directory creation, JSON construction, or atomic write
///   fails.
/// - panics: none.
/// - intension: performs one atomic write to the final path; no temp-path
///   policy is encoded in this module.
///
/// # Errors
/// Returns operational maintenance errors for invalid Git state, failed
/// current-head checks, directory creation failures, and support-layer atomic
/// write failures.
///
/// # Adequacy
/// - hypothesis: L3 only — successful advancement and stale-current-head
///   witnesses distinguish resolve/check/write order and the exact JSON schema.
/// - witness: `maintenance::tests::atomic_advancement_writes_schema_one_upper`
/// - witness: `maintenance::tests::current_head_expectation_rejects_stale_upper`
#[inline]
pub fn advance_watermark(request: &MaintenanceAdvanceRequest<'_>) -> Result<CommitId, GateError>
{
    let mut git = SupportGit { cwd: request.cwd };
    let mut sink = SupportWatermarkSink;
    advance_watermark_with_git_and_sink(
        &mut git,
        &mut sink,
        request.watermark,
        &request.to,
        request.head_expectation,
    )
}

/// Convert a range request into a planned source.
fn planned_source_from_request(
    request: &MaintenanceRangeRequest<'_>
) -> Result<PlannedBaseSource, GateError>
{
    if let Some(explicit) = request.explicit_from.as_ref() {
        return Ok(PlannedBaseSource::Explicit(explicit.clone()));
    }
    plan_base_source(None, request.watermark)
}

/// Plan the lower-bound source from legacy CLI-style optional inputs.
///
/// # Contract
/// - requires: `explicit_from` is the raw optional `--from` value and
///   `watermark` is the optional `--watermark` path.
/// - ensures: non-empty trimmed explicit input wins over watermark, non-empty
///   watermark wins over automatic mode, and whitespace explicit input is
///   absent.
/// - provides: a pure fixture-friendly representation of the retained
///   precedence rule.
/// - fails: returns [`GateError::Operational`] when a non-empty explicit token
///   is not a valid [`GitRef`].
/// - panics: none.
///
/// # Errors
/// Returns an operational maintenance error when the explicit token fails
/// [`GitRef`] validation.
///
/// # Adequacy
/// - hypothesis: L3 only — explicit+watermark, watermark-only, whitespace
///   explicit+watermark, and neither-source fixtures kill precedence mutants.
/// - witness: `maintenance::tests::precedence_prefers_explicit_then_watermark_then_auto`
#[inline]
pub fn plan_base_source<'semantic, ExplicitFrom>(
    explicit_from: ExplicitFrom,
    watermark: Option<&Path>,
) -> Result<PlannedBaseSource, GateError>
where
    ExplicitFrom: Into<OptionalExplicitFromText<'semantic>>,
{
    let explicit_from = explicit_from.into().0;
    if let Some(explicit) = GitRef::trimmed_optional(explicit_from)? {
        return Ok(PlannedBaseSource::Explicit(explicit));
    }
    if let Some(path) = watermark
        && !path.as_os_str().is_empty()
    {
        return Ok(PlannedBaseSource::Watermark(path.to_path_buf()));
    }
    Ok(PlannedBaseSource::Automatic)
}

/// Resolve a planned source into a base source.
fn resolve_planned_source(planned: PlannedBaseSource) -> Result<BaseSource, GateError>
{
    match planned {
        | PlannedBaseSource::Explicit(reference) => Ok(BaseSource::Explicit(reference)),
        | PlannedBaseSource::Watermark(path) => {
            let watermark = load_watermark(&path)?;
            Ok(BaseSource::Watermark(watermark.upper().clone()))
        },
        | PlannedBaseSource::Automatic => Ok(BaseSource::Automatic),
    }
}

/// Resolve a range using an injected Git backend.
fn resolve_range_with_git<Git>(
    git: &mut Git,
    head_ref: &GitRef,
    head_expectation: HeadExpectation,
    source: BaseSource,
) -> Result<MaintenanceRangeSelection, GateError>
where
    Git: MaintenanceGit,
{
    let head = resolve_head_commit(git, head_ref, head_expectation, "head")?;
    match source {
        | BaseSource::Explicit(reference) => {
            let base = git.resolve_commit(
                &reference,
                &format!("cannot resolve explicit base `{reference}` as a commit"),
            )?;
            validate_ancestor(git, &base, &head, "explicit base")?;
            Ok(MaintenanceRangeSelection {
                base,
                head,
                source: MaintenanceBaseKind::Explicit,
            })
        },
        | BaseSource::Watermark(upper) => {
            let reference = GitRef::from_commit(&upper);
            let base = git.resolve_commit(
                &reference,
                &format!("cannot resolve watermark base `{upper}` as a commit"),
            )?;
            if base != upper {
                return Err(maintenance_error(format!(
                    "watermark base `{upper}` did not verify to itself",
                )));
            }
            validate_ancestor(git, &base, &head, "watermark base")?;
            Ok(MaintenanceRangeSelection {
                base,
                head,
                source: MaintenanceBaseKind::Watermark,
            })
        },
        | BaseSource::Automatic => {
            let base = resolve_automatic_base(git, &head)?;
            Ok(MaintenanceRangeSelection {
                base,
                head,
                source: MaintenanceBaseKind::Automatic,
            })
        },
    }
}

/// Append a selected base to a GitHub Actions output file.
///
/// # Contract
/// - requires: `target` is the path from `GITHUB_OUTPUT` and the caller has
///   append permission.
/// - ensures: opens the target in append/create mode and appends one
///   newline-terminated `base=<oid>` record.
/// - provides: the append-only output contract used by GitHub Actions steps.
/// - fails: returns [`GateError::Operational`] when opening or writing fails.
/// - panics: none.
///
/// # Errors
/// Returns an operational maintenance error when the output path cannot be
/// opened or the line cannot be written completely.
///
/// # Adequacy
/// - hypothesis: L2 — range-selection witnesses use this only after all pure
///   selection branches; integration tests observe append-only file behavior.
/// - witness: `maintenance::tests::precedence_prefers_explicit_then_watermark_then_auto`
#[inline]
pub fn append_github_output(
    target: &Path,
    base: &CommitId,
) -> Result<(), GateError>
{
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(target)
        .map_err(|source| {
            maintenance_error(format!(
                "cannot append the maintenance base to `{}`: {source}",
                target.display(),
            ))
        })?;
    let line = format!("base={base}\n");
    file.write_all(line.as_bytes()).map_err(|source| {
        maintenance_error(format!(
            "cannot append the maintenance base to `{}`: {source}",
            target.display(),
        ))
    })
}

/// Resolve a head commit and optionally require it to equal current `HEAD`.
fn resolve_head_commit<'semantic, Git, Label>(
    git: &mut Git,
    reference: &GitRef,
    expectation: HeadExpectation,
    label: Label,
) -> Result<CommitId, GateError>
where
    Git: MaintenanceGit,
    Label: Into<LabelText<'semantic>>,
{
    let label = label.into().0;
    let resolved = git.resolve_commit(
        reference,
        &format!("cannot resolve {label} `{reference}` as a commit"),
    )?;
    if expectation == HeadExpectation::CurrentHead {
        let current_head = GitRef::head()?;
        let current =
            git.resolve_commit(&current_head, "cannot resolve current HEAD as a commit")?;
        if resolved != current {
            return Err(maintenance_error(format!(
                "{label} `{reference}` resolves to `{resolved}`, not the current HEAD `{current}`",
            )));
        }
    }
    Ok(resolved)
}

/// Validate an ancestry edge.
fn validate_ancestor<'semantic, Git, Label>(
    git: &mut Git,
    base: &CommitId,
    head: &CommitId,
    label: Label,
) -> Result<(), GateError>
where
    Git: MaintenanceGit,
    Label: Into<LabelText<'semantic>>,
{
    let label = label.into().0;
    if git.is_ancestor(base, head).map(|value| value.into().0)? {
        return Ok(());
    }
    Err(maintenance_error(format!(
        "{label} `{base}` is not an ancestor of `{head}`",
    )))
}

/// Resolve the automatic eight-day first-parent base for `head`.
fn resolve_automatic_base<Git>(
    git: &mut Git,
    head: &CommitId,
) -> Result<CommitId, GateError>
where
    Git: MaintenanceGit,
{
    let head_timestamp = git.committer_timestamp(head)?;
    let exclusive_before = head_timestamp.exclusive_before()?;
    let Some(base) = git.first_parent_before(head, exclusive_before)?
    else {
        return Err(maintenance_error(format!(
            "no commit at least eight days before `{head}` is reachable from the full checkout",
        )));
    };
    let verified = git.resolve_commit(
        &GitRef::from_commit(&base),
        &format!("resolved maintenance base `{base}` is not a commit"),
    )?;
    if verified != base {
        return Err(maintenance_error(format!(
            "resolved maintenance base `{base}` did not verify to itself",
        )));
    }
    validate_ancestor(git, &base, head, "resolved maintenance base")?;
    Ok(base)
}

/// Load and parse a watermark file from disk.
fn load_watermark(path: &Path) -> Result<Watermark, GateError>
{
    let source_name = path.display().to_string();
    let exists = crate::support::HOST_FILESYSTEM
        .try_exists(path)
        .map(bool::from)
        .map_err(|source| {
            maintenance_error(format!(
                "cannot read weekly success watermark `{source_name}`: {source}",
            ))
        })?;
    if !exists {
        return Err(maintenance_error(format!(
            "weekly success watermark is missing at `{source_name}`; bootstrap with an explicit repository-dispatch `from` ref",
        )));
    }
    let text = support::read_utf8(path).map_err(|error| {
        maintenance_error(format!(
            "cannot read weekly success watermark `{source_name}`: {error}",
        ))
    })?;
    parse_watermark_text(&source_name, &text)
}

/// Resolve a Git command's bounded captured stdout or convert status failure to
/// a maintenance error.
fn git_output<'semantic, Context>(
    cwd: Option<&Path>,
    args: &[OsString],
    context: Context,
) -> Result<String, GateError>
where
    Context: Into<ContextText<'semantic>>,
{
    let context = context.into().0;
    let output = support::run_output(OsStr::new(GIT_PROGRAM), args, cwd, true)
        .map_err(|error| maintenance_error(format!("{context}: {error}")))?;
    if output.success().into().0 {
        return Ok(output.stdout_lossy().text().as_ref().trim().to_owned());
    }
    Err(maintenance_error(format!(
        "{context}: {}",
        support::command_status_detail(GIT_PROGRAM, output.code().into().0)
    )))
}

/// Parse an optional watermark text fixture.
///
/// # Contract
/// - requires: `source_name` identifies the logical fixture or file and `text`
///   is the optional JSON payload.
/// - ensures: `None` fails with the same bootstrap-required diagnostic as a
///   missing production watermark; `Some` delegates to
///   [`parse_watermark_text`].
/// - provides: a pure fixture surface for missing-watermark behavior.
/// - fails: returns [`GateError::Operational`] for missing or malformed input.
/// - panics: none.
///
/// # Errors
/// Returns an operational maintenance error when `text` is absent or when
/// [`parse_watermark_text`] rejects the supplied JSON.
///
/// # Adequacy
/// - hypothesis: L3 only — the missing-watermark and malformed-watermark
///   fixtures observe both the absence branch and parser delegation.
/// - witness: `maintenance::tests::missing_and_invalid_watermarks_fail_closed`
#[inline]
pub fn parse_optional_watermark_text<'semantic, SourceName, Text>(
    source_name: SourceName,
    text: Text,
) -> Result<Watermark, GateError>
where
    SourceName: Into<SourceNameText<'semantic>>,
    Text: Into<OptionalTextText<'semantic>>,
{
    let text = text.into().0;
    let source_name = source_name.into().0;
    let Some(text) = text
    else {
        return Err(maintenance_error(format!(
            "weekly success watermark is missing at `{source_name}`; bootstrap with an explicit repository-dispatch `from` ref",
        )));
    };
    parse_watermark_text(source_name, text)
}

/// Advance the watermark using injected Git and atomic sink dependencies.
fn advance_watermark_with_git_and_sink<Git, Sink>(
    git: &mut Git,
    sink: &mut Sink,
    path: &Path,
    to: &GitRef,
    expectation: HeadExpectation,
) -> Result<CommitId, GateError>
where
    Git: MaintenanceGit,
    Sink: WatermarkSink,
{
    let upper = resolve_head_commit(git, to, expectation, "successful upper bound")?;
    sink.ensure_parent_directory(path)?;
    let bytes = watermark_json_bytes(&upper);
    sink.write_atomic(path, bytes.bytes().0)?;
    Ok(upper)
}

/// Convert a validated upper commit into deterministic JSON bytes.
///
/// # Contract
/// - requires: `upper` is the validated successful upper commit.
/// - ensures: returns exactly `{"schema":1,"upper":"<oid>"}\n` as UTF-8.
/// - provides: the atomic write payload for portable watermark state.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — the atomic advancement witness observes the full emitted
///   byte string for a representative object ID.
/// - witness: `maintenance::tests::atomic_advancement_writes_schema_one_upper`
#[inline]
#[must_use]
pub fn watermark_json_bytes(upper: &CommitId) -> WatermarkJsonBytes
{
    let mut text = String::from("{\"schema\":1,\"upper\":\"");
    text.push_str(upper.as_str().into().0);
    text.push_str("\"}\n");
    WatermarkJsonBytes(text.into_bytes())
}

/// Build the standard malformed-watermark error.
fn watermark_malformed<'semantic, SourceName>(source_name: SourceName) -> GateError
where
    SourceName: Into<SourceNameText<'semantic>>,
{
    let source_name = source_name.into().0;
    maintenance_error(format!(
        "weekly success watermark `{source_name}` is malformed",
    ))
}

/// Build a standardized maintenance operational error.
fn maintenance_error<Detail>(detail: Detail) -> GateError
where
    Detail: Into<String>,
{
    GateError::operational(format!("maintenance-range: {}", detail.into()))
}

/// Parse watermark JSON text.
///
/// # Contract
/// - requires: `source_name` identifies the logical source in diagnostics and
///   `text` contains JSON text.
/// - ensures: accepts only object schema `1` with string `upper` validated as a
///   [`CommitId`].
/// - provides: pure parsing for fixture tests and production file loading.
/// - fails: returns [`GateError::Operational`] for malformed JSON or invalid
///   schema/upper fields.
/// - panics: none.
///
/// # Errors
/// Returns an operational maintenance error when JSON parsing fails, the root
/// is not an object, `schema` is absent or not `1`, or `upper` is absent or not
/// a valid object ID string.
///
/// # Adequacy
/// - hypothesis: L3 only — missing, invalid, wrong-schema, missing-upper,
///   invalid-upper, and valid fixtures distinguish the parser decisions.
/// - witness: `maintenance::tests::missing_and_invalid_watermarks_fail_closed`
#[inline]
pub fn parse_watermark_text<'semantic, SourceName, Text>(
    source_name: SourceName,
    text: Text,
) -> Result<Watermark, GateError>
where
    SourceName: Into<SourceNameText<'semantic>>,
    Text: Into<TextText<'semantic>>,
{
    let text = text.into().0;
    let source_name = source_name.into().0;
    let value = serde_json::from_str::<Value>(text).map_err(|error| {
        maintenance_error(format!(
            "weekly success watermark `{source_name}` is malformed: {error}",
        ))
    })?;
    let Some(object) = value.as_object()
    else {
        return Err(watermark_malformed(source_name));
    };
    let schema = object.get("schema").and_then(Value::as_u64);
    if schema != Some(WATERMARK_SCHEMA) {
        return Err(watermark_malformed(source_name));
    }
    let Some(upper_text) = object.get("upper").and_then(Value::as_str)
    else {
        return Err(watermark_malformed(source_name));
    };
    let upper = CommitId::new_with_detail(upper_text, || {
        format!("weekly success watermark `{source_name}` is malformed")
    })?;
    Ok(Watermark {
        schema: WATERMARK_SCHEMA,
        upper,
    })
}

#[cfg(test)]
mod tests
{
    //! Unit witnesses for maintenance range planning and watermark behavior.

    use alloc::collections::BTreeMap;
    use alloc::collections::BTreeSet;
    use alloc::vec;
    use std::env;
    use std::path::Path;
    use std::path::PathBuf;

    use super::*;

    /// First fixture object ID.
    const OID_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    /// Second fixture object ID.
    const OID_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    /// Third fixture object ID.
    const OID_C: &str = "cccccccccccccccccccccccccccccccccccccccc";
    /// Fourth fixture object ID.
    const OID_D: &str = "dddddddddddddddddddddddddddddddddddddddd";
    /// Fifth fixture object ID.
    const OID_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    /// Timestamp used by current-head fixtures.
    const CURRENT_HEAD_TIMESTAMP_SECONDS: i64 = 20;
    /// Timestamp used by the atomic watermark advancement fixture.
    const WATERMARK_UPPER_TIMESTAMP_SECONDS: i64 = 50;
    /// Timestamp used by the older merge side parent.
    const MERGE_SIDE_TIMESTAMP_SECONDS: i64 = 100_000;
    /// Timestamp used by the automatic base candidate.
    const AUTOMATIC_BASE_TIMESTAMP_SECONDS: i64 = 300_000;
    /// Timestamp used by the newer first-parent commit.
    const RECENT_FIRST_PARENT_TIMESTAMP_SECONDS: i64 = 500_000;
    /// Timestamp used by a base that is still too recent.
    const TOO_RECENT_BASE_TIMESTAMP_SECONDS: i64 = 900_000;
    /// Timestamp used by merge-topology and recency fixture heads.
    const FIXTURE_HEAD_TIMESTAMP_SECONDS: i64 = 1_000_000;

    /// Missing and malformed watermarks fail closed.
    #[test]
    fn missing_and_invalid_watermarks_fail_closed() -> Result<(), GateError>
    {
        assert!(parse_optional_watermark_text("missing.json", None).is_err());
        assert!(parse_watermark_text("bad.json", "not json").is_err());
        assert!(parse_watermark_text("array.json", "[]").is_err());
        assert!(
            parse_watermark_text(
                "schema.json",
                "{\"schema\":2,\"upper\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}"
            )
            .is_err()
        );
        assert!(parse_watermark_text("upper.json", "{\"schema\":1}").is_err());
        assert!(
            parse_watermark_text("oid.json", "{\"schema\":1,\"upper\":\"not-an-oid\"}").is_err()
        );

        let parsed = parse_watermark_text(
            "ok.json",
            "{\"schema\":1,\"upper\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"}",
        )?;
        assert_eq!(WATERMARK_SCHEMA, parsed.schema().into().0);
        assert_eq!(
            OID_A,
            crate::semantic_value::<AsStrText<'_>, _>(parsed.upper().as_str()).0
        );
        Ok(())
    }

    /// Invalid object IDs and timestamps are rejected without fallback.
    #[test]
    fn invalid_oid_and_timestamp_are_rejected() -> Result<(), GateError>
    {
        assert!(CommitId::new("abc").is_err());
        assert!(CommitId::new("gggggggggggggggggggggggggggggggggggggggg").is_err());
        assert!(
            CommitId::new("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
                .is_err()
        );

        let commit = CommitId::new(OID_A)?;
        assert!(CommitTimestamp::parse_git_output(&commit, "not-a-time").is_err());
        assert!(
            CommitTimestamp::parse_git_output(&commit, "999999999999999999999999999999").is_err()
        );
        assert_eq!(
            42,
            CommitTimestamp::parse_git_output(&commit, "42\n")?
                .seconds()
                .into()
                .0,
        );
        assert!(CommitTimestamp::new(i64::MIN).exclusive_before().is_err());
        Ok(())
    }

    /// Non-ancestor explicit and watermark bases fail closed.
    #[test]
    fn non_ancestor_ranges_fail_closed() -> Result<(), GateError>
    {
        let mut git = FakeGit::new();
        let head = commit(OID_C)?;
        let base = commit(OID_A)?;
        let unrelated = commit(OID_D)?;
        git.add_commit(base.clone(), CommitTimestamp::new(10), None, Vec::new());
        git.add_commit(
            head.clone(),
            CommitTimestamp::new(CURRENT_HEAD_TIMESTAMP_SECONDS),
            Some(base),
            Vec::new(),
        );
        git.add_commit(unrelated.clone(), CommitTimestamp::new(5), None, Vec::new());
        git.add_ref("HEAD", head)?;
        git.add_ref("unrelated", unrelated.clone())?;

        let err = resolve_range_with_git(
            &mut git,
            &GitRef::new("HEAD")?,
            HeadExpectation::AnyCommit,
            BaseSource::Explicit(GitRef::new("unrelated")?),
        );
        assert!(err.is_err());
        let watermark_err = resolve_range_with_git(
            &mut git,
            &GitRef::new("HEAD")?,
            HeadExpectation::AnyCommit,
            BaseSource::Watermark(unrelated),
        );
        assert!(watermark_err.is_err());
        Ok(())
    }

    /// Automatic selection follows first-parent history through a merge
    /// topology.
    #[test]
    fn merge_topology_uses_first_parent_cutoff() -> Result<(), GateError>
    {
        let mut git = FakeGit::new();
        let old_first_parent = commit(OID_A)?;
        let new_first_parent = commit(OID_B)?;
        let merge_parent = commit(OID_D)?;
        let head = commit(OID_C)?;
        git.add_commit(
            old_first_parent.clone(),
            CommitTimestamp::new(AUTOMATIC_BASE_TIMESTAMP_SECONDS),
            None,
            Vec::new(),
        );
        git.add_commit(
            new_first_parent.clone(),
            CommitTimestamp::new(RECENT_FIRST_PARENT_TIMESTAMP_SECONDS),
            Some(old_first_parent.clone()),
            Vec::new(),
        );
        git.add_commit(
            merge_parent.clone(),
            CommitTimestamp::new(MERGE_SIDE_TIMESTAMP_SECONDS),
            None,
            Vec::new(),
        );
        git.add_commit(
            head.clone(),
            CommitTimestamp::new(FIXTURE_HEAD_TIMESTAMP_SECONDS),
            Some(new_first_parent),
            vec![merge_parent],
        );
        git.add_ref("HEAD", head.clone())?;

        let selection = resolve_range_with_git(
            &mut git,
            &GitRef::new("HEAD")?,
            HeadExpectation::AnyCommit,
            BaseSource::Automatic,
        )?;
        assert_eq!(selection.base, old_first_parent);
        assert_eq!(selection.head, head);
        assert_eq!(MaintenanceBaseKind::Automatic, selection.source);
        Ok(())
    }

    /// Automatic selection fails when no first-parent commit is old enough.
    #[test]
    fn no_old_base_fails_closed() -> Result<(), GateError>
    {
        let mut git = FakeGit::new();
        let parent = commit(OID_A)?;
        let head = commit(OID_B)?;
        git.add_commit(
            parent.clone(),
            CommitTimestamp::new(TOO_RECENT_BASE_TIMESTAMP_SECONDS),
            None,
            Vec::new(),
        );
        git.add_commit(
            head.clone(),
            CommitTimestamp::new(FIXTURE_HEAD_TIMESTAMP_SECONDS),
            Some(parent),
            Vec::new(),
        );
        git.add_ref("HEAD", head)?;

        let err = resolve_range_with_git(
            &mut git,
            &GitRef::new("HEAD")?,
            HeadExpectation::AnyCommit,
            BaseSource::Automatic,
        );
        assert!(err.is_err());
        Ok(())
    }

    /// Source planning uses explicit, then watermark, then automatic
    /// precedence.
    #[test]
    fn precedence_prefers_explicit_then_watermark_then_auto() -> Result<(), GateError>
    {
        let watermark = Path::new("weekly-success.json");
        assert!(matches!(
            plan_base_source(Some(" refs/heads/main "), Some(watermark))?,
            PlannedBaseSource::Explicit(_),
        ));
        assert!(matches!(
            plan_base_source(Some("   "), Some(watermark))?,
            PlannedBaseSource::Watermark(_),
        ));
        assert!(matches!(
            plan_base_source(None, Some(watermark))?,
            PlannedBaseSource::Watermark(_),
        ));
        assert!(matches!(
            plan_base_source(None, None)?,
            PlannedBaseSource::Automatic,
        ));
        assert!(GitRef::new(" -bad").is_err());
        assert!(GitRef::new("-bad").is_err());
        assert!(GitRef::new("bad\nref").is_err());
        Ok(())
    }

    /// Current-head expectation rejects stale upper refs.
    #[test]
    fn current_head_expectation_rejects_stale_upper() -> Result<(), GateError>
    {
        let mut git = FakeGit::new();
        let current = commit(OID_A)?;
        let stale = commit(OID_B)?;
        git.add_commit(
            current.clone(),
            CommitTimestamp::new(CURRENT_HEAD_TIMESTAMP_SECONDS),
            None,
            Vec::new(),
        );
        git.add_commit(stale.clone(), CommitTimestamp::new(10), None, Vec::new());
        git.add_ref("HEAD", current)?;
        git.add_ref("stale", stale)?;
        let mut sink = FakeSink::default();

        let result = advance_watermark_with_git_and_sink(
            &mut git,
            &mut sink,
            Path::new("state/watermark.json"),
            &GitRef::new("stale")?,
            HeadExpectation::CurrentHead,
        );
        assert!(result.is_err());
        assert!(sink.writes.is_empty());
        Ok(())
    }

    /// Advancement writes exactly one schema-one JSON watermark atomically.
    #[test]
    fn atomic_advancement_writes_schema_one_upper() -> Result<(), GateError>
    {
        let mut git = FakeGit::new();
        let upper = commit(OID_E)?;
        git.add_commit(
            upper.clone(),
            CommitTimestamp::new(WATERMARK_UPPER_TIMESTAMP_SECONDS),
            None,
            Vec::new(),
        );
        git.add_ref("HEAD", upper.clone())?;
        let mut sink = FakeSink::default();
        let path = Path::new("state/watermark.json");

        let advanced = advance_watermark_with_git_and_sink(
            &mut git,
            &mut sink,
            path,
            &GitRef::new("HEAD")?,
            HeadExpectation::CurrentHead,
        )?;

        assert_eq!(advanced, upper);
        assert_eq!(sink.ensure_parent_calls, Vec::from([PathBuf::from(path)]));
        assert_eq!(1, sink.writes.len());
        let Some(entry) = sink.writes.first()
        else {
            return Err(GateError::operational("missing fake write"));
        };
        let (written_path, bytes) = (&entry.0, &entry.1);
        assert_eq!(written_path.as_path(), path);
        assert_eq!(bytes.as_slice(), watermark_json_bytes(&advanced).bytes().0);
        assert_eq!(
            "{\"schema\":1,\"upper\":\"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee\"}\n",
            core::str::from_utf8(bytes)
                .map_err(|error| GateError::operational(error.to_string()))?,
        );
        Ok(())
    }

    /// Appending GitHub output preserves existing records and writes one base.
    #[test]
    fn append_github_output_appends_without_truncating() -> Result<(), GateError>
    {
        let fixture = MaintenanceFixture::new("append-output")?;
        let output = fixture.path().join("github-output.txt");
        crate::support::HOST_FILESYSTEM
            .write(&output, "previous=1\n")
            .map_err(|error| GateError::operational(error.to_string()))?;
        let base = commit(OID_A)?;

        append_github_output(&output, &base)?;

        let text = crate::support::HOST_FILESYSTEM
            .read_to_string(&output)
            .map_err(|error| GateError::operational(error.to_string()))?;
        assert_eq!(text, format!("previous=1\nbase={base}\n"));
        Ok(())
    }

    /// Production Git range resolution accepts an explicit ancestor lower
    /// bound.
    #[test]
    fn live_git_explicit_range_appends_selected_base() -> Result<(), GateError>
    {
        let fixture = MaintenanceFixture::new("live-explicit")?;
        init_git_repo(fixture.path())?;
        let base = commit_fixture_file(fixture.path(), "base.txt", "base\n", 1_000_000_000)?;
        let head = commit_fixture_file(fixture.path(), "head.txt", "head\n", 1_000_000_100)?;
        let output = fixture.path().join("github-output.txt");
        let request = MaintenanceRangeRequest::new(
            &output,
            GitRef::new(crate::semantic_value::<AsStrText<'_>, _>(head.as_str()).0)?,
            Some(GitRef::new(
                crate::semantic_value::<AsStrText<'_>, _>(base.as_str()).0,
            )?),
            None,
            Some(fixture.path()),
            HeadExpectation::AnyCommit,
        );

        let selection = resolve_and_append_github_output(&request)?;

        assert_eq!(selection.base, base);
        assert_eq!(selection.head, head);
        assert_eq!(MaintenanceBaseKind::Explicit, selection.source);
        let text = crate::support::HOST_FILESYSTEM
            .read_to_string(&output)
            .map_err(|error| GateError::operational(error.to_string()))?;
        assert_eq!(text, format!("base={}\n", selection.base));
        Ok(())
    }

    /// Production watermark ranges load schema-one JSON before selecting a
    /// base.
    #[test]
    fn live_git_watermark_range_loads_json_and_missing_fails() -> Result<(), GateError>
    {
        let fixture = MaintenanceFixture::new("live-watermark")?;
        init_git_repo(fixture.path())?;
        let base = commit_fixture_file(fixture.path(), "base.txt", "base\n", 1_000_000_000)?;
        let head = commit_fixture_file(fixture.path(), "head.txt", "head\n", 1_000_000_100)?;
        let missing = fixture.path().join("state/missing.json");
        let missing_output = fixture.path().join("missing-output.txt");
        let missing_request = MaintenanceRangeRequest::new(
            &missing_output,
            GitRef::new("HEAD")?,
            None,
            Some(&missing),
            Some(fixture.path()),
            HeadExpectation::AnyCommit,
        );
        assert!(resolve_and_append_github_output(&missing_request).is_err());

        let watermark = fixture.path().join("state/watermark.json");
        let parent = watermark
            .parent()
            .ok_or_else(|| GateError::operational("watermark fixture path has no parent"))?;
        crate::support::HOST_FILESYSTEM
            .create_dir_all(parent)
            .map_err(|error| GateError::operational(error.to_string()))?;
        crate::support::HOST_FILESYSTEM
            .write(&watermark, watermark_json_bytes(&base).bytes().0)
            .map_err(|error| GateError::operational(error.to_string()))?;
        let output = fixture.path().join("github-output.txt");
        let request = MaintenanceRangeRequest::new(
            &output,
            GitRef::new(crate::semantic_value::<AsStrText<'_>, _>(head.as_str()).0)?,
            None,
            Some(&watermark),
            Some(fixture.path()),
            HeadExpectation::AnyCommit,
        );

        let selection = resolve_and_append_github_output(&request)?;

        assert_eq!(selection.base, base);
        assert_eq!(selection.head, head);
        assert_eq!(MaintenanceBaseKind::Watermark, selection.source);
        Ok(())
    }

    /// Production automatic ranges use first-parent commits older than cutoff.
    #[test]
    fn live_git_automatic_range_uses_first_parent_cutoff() -> Result<(), GateError>
    {
        let fixture = MaintenanceFixture::new("live-automatic")?;
        init_git_repo(fixture.path())?;
        let old = commit_fixture_file(fixture.path(), "old.txt", "old\n", 1_000_000_000)?;
        let head = commit_fixture_file(fixture.path(), "head.txt", "head\n", 1_000_700_000)?;
        let output = fixture.path().join("github-output.txt");
        let request = MaintenanceRangeRequest::new(
            &output,
            GitRef::new("HEAD")?,
            None,
            None,
            Some(fixture.path()),
            HeadExpectation::CurrentHead,
        );

        let selection = resolve_and_append_github_output(&request)?;

        assert_eq!(selection.base, old);
        assert_eq!(selection.head, head);
        assert_eq!(MaintenanceBaseKind::Automatic, selection.source);
        Ok(())
    }

    /// Production advancement writes schema-one JSON through the atomic sink.
    #[test]
    fn live_git_advance_watermark_writes_nested_state() -> Result<(), GateError>
    {
        let fixture = MaintenanceFixture::new("live-advance")?;
        init_git_repo(fixture.path())?;
        let head = commit_fixture_file(fixture.path(), "head.txt", "head\n", 1_000_000_000)?;
        let watermark = fixture.path().join("state/nested/watermark.json");
        let request = MaintenanceAdvanceRequest::new(
            &watermark,
            GitRef::new("HEAD")?,
            Some(fixture.path()),
            HeadExpectation::CurrentHead,
        );

        let advanced = advance_watermark(&request)?;

        assert_eq!(advanced, head);
        let bytes = crate::support::HOST_FILESYSTEM
            .read(&watermark)
            .map_err(|error| GateError::operational(error.to_string()))?;
        assert_eq!(
            watermark_json_bytes(&advanced).bytes().0,
            bytes.as_bytes().0
        );
        Ok(())
    }

    /// Build a validated commit fixture.
    fn commit<'semantic, Text>(text: Text) -> Result<CommitId, GateError>
    where
        Text: Into<TextText<'semantic>>,
    {
        let text = text.into().0;
        CommitId::new(text)
    }

    /// In-memory commit graph for pure planning tests.
    #[derive(Default)]
    struct FakeGit
    {
        /// Commit records keyed by object ID.
        commits: BTreeMap<CommitId, FakeCommit>,
        /// Named refs keyed by token.
        refs: BTreeMap<String, CommitId>,
    }

    impl FakeGit
    {
        /// Build an empty graph.
        fn new() -> Self
        {
            Self::default()
        }

        /// Add a commit fixture.
        fn add_commit(
            &mut self,
            id: CommitId,
            timestamp: CommitTimestamp,
            first_parent: Option<CommitId>,
            extra_parents: Vec<CommitId>,
        )
        {
            let record = FakeCommit {
                timestamp,
                first_parent,
                extra_parents,
            };
            self.commits.insert(id, record);
        }

        /// Add a named ref fixture.
        fn add_ref<'semantic, Name>(
            &mut self,
            name: Name,
            commit: CommitId,
        ) -> Result<(), GateError>
        where
            Name: Into<NameText<'semantic>>,
        {
            let name = name.into().0;
            let reference = GitRef::new(name)?;
            self.refs.insert(
                String::from(crate::semantic_value::<AsStrText<'_>, _>(reference.as_str()).0),
                commit,
            );
            Ok(())
        }

        /// Resolve a fixture token to a commit ID.
        fn fixture_resolve(
            &self,
            reference: &GitRef,
        ) -> Option<CommitId>
        {
            if let Some(commit) = self
                .refs
                .get(crate::semantic_value::<AsStrText<'_>, _>(reference.as_str()).0)
            {
                return Some(commit.clone());
            }
            let Ok(candidate) =
                CommitId::new(crate::semantic_value::<AsStrText<'_>, _>(reference.as_str()).0)
            else {
                return None;
            };
            if self.commits.contains_key(&candidate) {
                return Some(candidate);
            }
            None
        }
    }

    impl MaintenanceGit for FakeGit
    {
        fn resolve_commit<'semantic, Context>(
            &mut self,
            reference: &GitRef,
            context: Context,
        ) -> Result<CommitId, GateError>
        where
            Context: Into<ContextText<'semantic>>,
        {
            let context = context.into().0;
            self.fixture_resolve(reference)
                .ok_or_else(|| maintenance_error(context))
        }

        fn committer_timestamp(
            &mut self,
            commit: &CommitId,
        ) -> Result<CommitTimestamp, GateError>
        {
            let Some(record) = self.commits.get(commit)
            else {
                return Err(maintenance_error(format!(
                    "missing fixture commit `{commit}`"
                )));
            };
            Ok(record.timestamp)
        }

        fn first_parent_before(
            &mut self,
            head: &CommitId,
            exclusive_before: CommitTimestamp,
        ) -> Result<Option<CommitId>, GateError>
        {
            let mut current = Some(head.clone());
            while let Some(commit_id) = current {
                let Some(record) = self.commits.get(&commit_id)
                else {
                    return Err(maintenance_error(format!(
                        "missing fixture commit `{commit_id}`",
                    )));
                };
                if record.timestamp < exclusive_before {
                    return Ok(Some(commit_id));
                }
                current = record.first_parent.clone();
            }
            Ok(None)
        }

        fn is_ancestor(
            &mut self,
            base: &CommitId,
            head: &CommitId,
        ) -> Result<impl Into<AncestorFlag>, GateError>
        {
            let mut seen = BTreeSet::new();
            let mut stack = Vec::from([head.clone()]);
            while let Some(current) = stack.pop() {
                if &current == base {
                    return Ok(true);
                }
                if !seen.insert(current.clone()) {
                    continue;
                }
                let Some(record) = self.commits.get(&current)
                else {
                    continue;
                };
                if let Some(parent) = record.first_parent.as_ref() {
                    stack.push(parent.clone());
                }
                for parent in &record.extra_parents {
                    stack.push(parent.clone());
                }
            }
            Ok(false)
        }
    }

    /// Commit graph record used by `FakeGit`.
    struct FakeCommit
    {
        /// Commit timestamp fixture.
        timestamp: CommitTimestamp,
        /// First-parent edge, if any.
        first_parent: Option<CommitId>,
        /// Non-first-parent merge edges.
        extra_parents: Vec<CommitId>,
    }

    /// In-memory atomic sink for advancement tests.
    #[derive(Default)]
    struct FakeSink
    {
        /// Parent-directory ensure calls in call order.
        ensure_parent_calls: Vec<PathBuf>,
        /// Atomic writes in call order.
        writes: Vec<(PathBuf, Vec<u8>)>,
    }

    impl WatermarkSink for FakeSink
    {
        fn ensure_parent_directory(
            &mut self,
            path: &Path,
        ) -> Result<(), GateError>
        {
            self.ensure_parent_calls.push(path.to_path_buf());
            Ok(())
        }

        fn write_atomic<'semantic, Bytes>(
            &mut self,
            path: &Path,
            bytes: Bytes,
        ) -> Result<(), GateError>
        where
            Bytes: Into<BytesBytes<'semantic>>,
        {
            let bytes = bytes.into().0;
            self.writes.push((path.to_path_buf(), Vec::from(bytes)));
            Ok(())
        }
    }

    /// Temporary Git fixture removed on drop.
    #[repr(transparent)]
    struct MaintenanceFixture
    {
        /// Unique root path for this test.
        root: PathBuf,
    }

    impl MaintenanceFixture
    {
        /// Create an empty fixture directory.
        fn new<'semantic, Name>(name: Name) -> Result<Self, GateError>
        where
            Name: Into<NameText<'semantic>>,
        {
            let name = name.into().0;
            let root = env::temp_dir().join(format!(
                "gandr-workflow-gates-maintenance-{}-{name}",
                std::process::id()
            ));
            crate::support::HOST_FILESYSTEM.remove_dir_if_exists(&root)?;
            crate::support::HOST_FILESYSTEM.create_dir_all(&root)?;
            Ok(Self { root })
        }

        /// Borrow the fixture root path.
        fn path(&self) -> &Path
        {
            &self.root
        }
    }

    impl Drop for MaintenanceFixture
    {
        fn drop(&mut self)
        {
            drop(crate::support::HOST_FILESYSTEM.remove_dir_all(&self.root));
        }
    }

    /// Initialize a Git repository fixture.
    fn init_git_repo(path: &Path) -> Result<(), GateError>
    {
        crate::support::HOST_FILESYSTEM
            .create_dir_all(path)
            .map_err(|error| GateError::operational(error.to_string()))?;
        let _stdout = fixture_git(path, ["init"])?;
        Ok(())
    }

    /// Commit one fixture file and return its object ID.
    fn commit_fixture_file<'semantic, Name, Text, Timestamp>(
        repo: &Path,
        name: Name,
        text: Text,
        timestamp: Timestamp,
    ) -> Result<CommitId, GateError>
    where
        Name: Into<NameText<'semantic>>,
        Text: Into<TextText<'semantic>>,
        Timestamp: Into<TimestampSeconds>,
    {
        let text = text.into().0;
        let timestamp = timestamp.into().0;
        let name = name.into().0;
        crate::support::HOST_FILESYSTEM
            .write(repo.join(name), text)
            .map_err(|error| GateError::operational(error.to_string()))?;
        let _stdout = fixture_git(repo, ["add", name])?;
        let date = format!("{timestamp} +0000");
        let mut command = support::stateless_git_command();
        command
            .args(["commit", "-m", name])
            .current_dir(repo)
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date);
        let output = command
            .output()
            .map_err(|error| GateError::operational(error.to_string()))?;
        if !output.status.success() {
            return Err(GateError::operational(format!(
                "git commit fixture failed: {}",
                String::from_utf8_lossy(&output.stderr),
            )));
        }
        let oid = fixture_git(repo, ["rev-parse", "HEAD"])?;
        CommitId::new(oid.trim())
    }

    /// Run Git in a fixture repository.
    fn fixture_git<Args>(
        cwd: &Path,
        args: Args,
    ) -> Result<String, GateError>
    where
        Args: IntoIterator,
        Args::Item: AsRef<OsStr>,
    {
        let mut command = support::stateless_git_command();
        command.args(args).current_dir(cwd);
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
}
