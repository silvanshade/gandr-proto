//! Pure mutation-campaign range planning.
//!
//! The host driver keeps all process execution at its integration boundary.
//! This module only validates user/git facts already supplied by that boundary
//! and renders deterministic `git diff`/campaign plans for merge, push, and
//! scheduled mutation runs.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;

use crate::GateError;

crate::semantic_str!(pub struct ValueText);
crate::semantic_str!(pub struct LabelText);
crate::semantic_str!(pub struct TokenText);
crate::semantic_str!(pub struct DiffTextText);
crate::semantic_str!(pub struct ExtensionText);
crate::semantic_str!(pub struct SpecText);
crate::semantic_str!(pub struct DetailText);
crate::semantic_str!(pub struct FromText);
crate::semantic_str!(pub struct ToText);
crate::semantic_str!(pub struct RootText);
crate::semantic_str!(pub struct ReportText);
crate::semantic_str!(pub struct NeedleText);
crate::semantic_str!(pub struct LineText);
crate::semantic_copy!(pub struct ChCharacter(char));
crate::semantic_str!(pub struct AsStrText);
crate::semantic_copy!(pub struct DiffTouchesRustFlag(bool));
crate::semantic_copy!(pub struct DiffTargetIsRustFlag(bool));
crate::semantic_copy!(pub struct PathExtensionFlag(bool));
crate::semantic_copy!(pub struct RefBodyCharFlag(bool));
crate::semantic_copy!(pub struct LowerHexDigitFlag(bool));

/// Number of lowercase hexadecimal digits in a Git SHA-1 object id.
const GIT_OID_HEX_CHARS: usize = 40;

/// Stable report label for a skipped campaign whose diff touches no Rust files.
pub(crate) const NO_RUST_CHANGES_REPORT: &str = "no-rust-changes";

/// A validated lowercase Git commit id.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
#[repr(transparent)]
pub(crate) struct CommitId
{
    /// The exact lowercase hexadecimal object id.
    value: String,
}

impl CommitId
{
    /// Parse a resolved Git commit id.
    ///
    /// # Contract
    /// - requires: `value` is the raw `git rev-parse` stdout after trimming.
    /// - ensures: accepts only a forty-character lowercase hexadecimal commit
    ///   id.
    /// - provides: an owned commit id safe to splice into a later diff spec.
    /// - fails: returns a usage error when the object id is empty, the wrong
    ///   length, uppercase, or contains non-hexadecimal characters.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns a usage error naming `label` when `value` is not exactly a
    /// lowercase forty-digit Git object id.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — length, uppercase, and non-hex residues are
    ///   separated by exact malformed-scheduled-ref fixtures.
    /// - witness: `mutants::range::tests::scheduled_range_rejects_invalid_oid_and_topology`
    #[inline]
    pub(crate) fn parse<'semantic, Value, Label>(
        value: Value,
        label: Label,
    ) -> Result<Self, GateError>
    where
        Value: Into<ValueText<'semantic>>,
        Label: Into<LabelText<'semantic>>,
    {
        let label = label.into().0;
        let value = value.into().0;
        let length = value.chars().count();
        if length != GIT_OID_HEX_CHARS || !value.chars().all(|ch| is_lower_hex_digit(ch).into().0) {
            return Err(GateError::usage(format!(
                "mutants-vm: scheduled --{label} ref resolved to an invalid commit id"
            )));
        }

        Ok(Self {
            value: String::from(value),
        })
    }

    /// Borrow the validated object id.
    #[inline]
    #[must_use]
    pub(crate) fn as_str(&self) -> impl Into<AsStrText<'_>>
    {
        &self.value
    }
}

/// A scheduled-campaign ref token that is safe to pass to Git resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[repr(transparent)]
pub(crate) struct RefToken
{
    /// The exact ref token supplied by the caller.
    value: String,
}

impl RefToken
{
    /// Borrow the validated token.
    #[inline]
    #[must_use]
    pub(crate) fn as_str(&self) -> impl Into<AsStrText<'_>>
    {
        &self.value
    }
}

/// Render the merge campaign's three-dot diff plan.
///
/// # Contract
/// - requires: the integration boundary will run this inside the repository.
/// - ensures: returns exactly `git diff main...HEAD`.
/// - provides: the merge mutation range without shelling out.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L1 exactness — the single merge contract is killed by exact
///   argument-vector equality.
/// - witness: `mutants::range::tests::merge_diff_uses_main_three_dot_head`
#[inline]
#[must_use]
pub(crate) fn merge_diff_plan() -> GitDiffPlan
{
    diff_plan(DiffKind::Merge, "main...HEAD")
}

/// Render a push campaign diff plan from a typed push-range mode.
///
/// # Contract
/// - requires: `plan` was produced by the shared push-range resolver contract.
/// - ensures: renders `range` as `from...to`, `full` as `root...to`, and `last`
///   as the literal `HEAD~1...HEAD`.
/// - provides: the push mutation range without shelling out.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — each resolver mode has an exact argument-vector
///   witness that distinguishes swapped two-dot/three-dot or wrong anchors.
/// - witness: `mutants::range::tests::push_range_modes_render_exact_three_dot_specs`
#[inline]
#[must_use]
pub(crate) fn push_diff_plan(plan: &PushRangePlan) -> GitDiffPlan
{
    match plan {
        | PushRangePlan::Range { from, to } => diff_plan(
            DiffKind::Push(PushRangeMode::Range),
            &format!("{from}...{to}"),
        ),
        | PushRangePlan::Full { root, to } => diff_plan(
            DiffKind::Push(PushRangeMode::Full),
            &format!("{root}...{to}"),
        ),
        | PushRangePlan::Last { .. } => {
            diff_plan(DiffKind::Push(PushRangeMode::Last), "HEAD~1...HEAD")
        },
    }
}

/// Validate and render a scheduled mutation range.
///
/// # Contract
/// - requires: `input` contains ref tokens plus already-resolved commit ids and
///   ancestry facts from sanitized Git calls.
/// - ensures: validates both ref tokens, validates all commit ids, requires
///   `to_oid == head_oid`, requires `from_oid` to be an ancestor of `to_oid`,
///   and renders `git diff from_oid...to_oid`.
/// - provides: a scheduled range plan that cannot silently target a stale or
///   reversed interval.
/// - fails: returns a usage error for malformed refs or commit ids and an
///   operational error for stale `--to` or reversed/unrelated topology.
/// - panics: none.
///
/// # Errors
/// Returns usage errors for invalid ref tokens or commit ids. Returns
/// operational errors when `--to` is not the current `HEAD` or `from` is not an
/// ancestor of `to`.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — malformed refs, invalid object ids, stale upper
///   endpoint, reversed/unrelated topology, and successful three-dot rendering
///   are separated by exact witnesses.
/// - witness: `mutants::range::tests::scheduled_ref_tokens_reject_malformed_inputs`
/// - witness: `mutants::range::tests::scheduled_range_rejects_invalid_oid_and_topology`
/// - witness: `mutants::range::tests::scheduled_range_renders_resolved_three_dot_diff`
#[inline]
pub(crate) fn scheduled_diff_plan(input: &ScheduledRangeInput<'_>)
-> Result<GitDiffPlan, GateError>
{
    let _from_ref = validate_scheduled_ref_token(input.from_ref, "from")?;
    let _to_ref = validate_scheduled_ref_token(input.to_ref, "to")?;
    let from_oid = CommitId::parse(input.from_oid, "from")?;
    let to_oid = CommitId::parse(input.to_oid, "to")?;
    let head_oid = CommitId::parse(input.head_oid, "HEAD")?;

    if to_oid != head_oid {
        return Err(GateError::operational(
            "mutants-vm: scheduled --to must resolve to the current HEAD",
        ));
    }
    if !input.from_is_ancestor_of_to {
        return Err(GateError::operational(format!(
            "mutants-vm: scheduled range `{}`...`{}` is reversed or unrelated",
            input.from_ref, input.to_ref
        )));
    }

    Ok(diff_plan(
        DiffKind::Scheduled,
        &format!(
            "{}...{}",
            crate::semantic_value::<AsStrText<'_>>(from_oid.as_str()).0,
            crate::semantic_value::<AsStrText<'_>>(to_oid.as_str()).0
        ),
    ))
}

/// Validate a scheduled campaign ref token before any Git command sees it.
///
/// # Contract
/// - requires: `label` is the stable CLI label used in diagnostics.
/// - ensures: accepts only tokens matching the supported branch/tag alphabet,
///   with no surrounding whitespace, traversal, reflog expression, trailing
///   slash, trailing dot, or `.lock` suffix.
/// - provides: a validated token preserving the user's exact spelling.
/// - fails: returns a usage error for empty, whitespace-padded, unsupported, or
///   syntactically unsafe refs.
/// - panics: none.
///
/// # Errors
/// Returns a usage error describing the first token contract violation.
///
/// # Adequacy
/// - hypothesis: L3 plus an ASCII property sweep — empty/whitespace, first
///   character, body alphabet, traversal, reflog, and suffix guards each have
///   an exact witness, and the printable ASCII residue is checked against the
///   declared whitelist.
/// - witness: `mutants::range::tests::scheduled_ref_tokens_reject_malformed_inputs`
/// - witness: `mutants::range::tests::scheduled_ref_token_ascii_whitelist_is_exact`
#[inline]
pub(crate) fn validate_scheduled_ref_token<'semantic, Token, Label>(
    token: Token,
    label: Label,
) -> Result<RefToken, GateError>
where
    Token: Into<TokenText<'semantic>>,
    Label: Into<LabelText<'semantic>>,
{
    let label = label.into().0;
    let token = token.into().0;
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err(GateError::usage(format!(
            "mutants-vm: scheduled --{label} ref is required"
        )));
    }
    if token != trimmed {
        return Err(GateError::usage(format!(
            "mutants-vm: scheduled --{label} ref must not contain surrounding whitespace"
        )));
    }

    let mut chars = token.chars();
    let Some(first) = chars.next()
    else {
        return Err(GateError::usage(format!(
            "mutants-vm: scheduled --{label} ref is required"
        )));
    };
    if !first.is_ascii_alphanumeric() || !chars.all(|ch| is_ref_body_char(ch).into().0) {
        return Err(GateError::usage(format!(
            "mutants-vm: scheduled --{label} ref contains unsupported characters"
        )));
    }

    if token.contains("..")
        || token.contains("@{")
        || token.contains("//")
        || token.ends_with('/')
        || token.ends_with('.')
        || has_path_extension(token, "lock").into().0
    {
        return Err(GateError::usage(format!(
            "mutants-vm: scheduled --{label} ref is not a safe branch/tag name"
        )));
    }

    Ok(RefToken {
        value: String::from(token),
    })
}

/// Decide whether a diff-selected mutation campaign should run.
///
/// # Contract
/// - requires: `diff_text` is the unified diff produced by `diff.args`.
/// - ensures: returns [`RangeCampaignPlan::NoRustChanges`] exactly when no `+++
///   ... .rs` target line exists; otherwise preserves the diff plan for a
///   contained guest run.
/// - provides: the no-Rust fast path without weakening containment.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — non-Rust target lines, Rust target lines,
///   deleted files, and arbitrary body lines are distinguished by exact diff
///   fixtures.
/// - witness: `mutants::range::tests::non_rust_diff_is_noop_campaign`
/// - witness: `mutants::range::tests::rust_diff_detection_matches_target_lines_only`
#[inline]
#[must_use]
pub(crate) fn plan_in_diff_campaign<'semantic, DiffText>(
    diff: GitDiffPlan,
    diff_text: DiffText,
) -> RangeCampaignPlan
where
    DiffText: Into<DiffTextText<'semantic>>,
{
    let diff_text = diff_text.into().0;
    if diff_touches_rust(diff_text).into().0 {
        RangeCampaignPlan::RunInDiff { diff }
    }
    else {
        RangeCampaignPlan::NoRustChanges {
            report: NO_RUST_CHANGES_REPORT,
        }
    }
}

/// Return whether a unified diff targets any Rust source file.
///
/// # Contract
/// - requires: `diff_text` is UTF-8 unified diff text.
/// - ensures: only `+++ ` target lines ending in `.rs` count as Rust changes.
/// - provides: a deterministic no-op detector matching cargo-mutants' Rust-file
///   selection boundary.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — plus-body lines, removed-file markers, non-Rust
///   targets, and Rust targets each have exact fixtures.
/// - witness: `mutants::range::tests::rust_diff_detection_matches_target_lines_only`
#[inline]
#[must_use]
pub(crate) fn diff_touches_rust<'semantic, DiffText>(
    diff_text: DiffText,
) -> impl Into<DiffTouchesRustFlag>
where
    DiffText: Into<DiffTextText<'semantic>>,
{
    let diff_text = diff_text.into().0;
    diff_text
        .lines()
        .any(|line| diff_target_is_rust(line).into().0)
}

/// Return whether one unified diff line names a Rust target file.
#[inline]
fn diff_target_is_rust<'semantic, Line>(line: Line) -> impl Into<DiffTargetIsRustFlag>
where
    Line: Into<LineText<'semantic>>,
{
    let line = line.into().0;
    line.strip_prefix("+++ ")
        .is_some_and(|target| has_path_extension(target, "rs").into().0)
}

/// Return whether a path-like token has the requested extension.
#[inline]
fn has_path_extension<'semantic, Value, Extension>(
    value: Value,
    extension: Extension,
) -> impl Into<PathExtensionFlag>
where
    Value: Into<ValueText<'semantic>>,
    Extension: Into<ExtensionText<'semantic>>,
{
    let extension = extension.into().0;
    let value = value.into().0;
    Path::new(value)
        .extension()
        .is_some_and(|candidate| candidate == OsStr::new(extension))
}

/// Render a `git diff` plan for one diff spec.
#[inline]
fn diff_plan<'semantic, Spec>(
    kind: DiffKind,
    spec: Spec,
) -> GitDiffPlan
where
    Spec: Into<SpecText<'semantic>>,
{
    let spec = spec.into().0;
    GitDiffPlan {
        kind,
        args: vec![OsString::from("diff"), OsString::from(spec)],
    }
}

/// Require a nonempty push endpoint.
#[inline]
fn require_nonempty<'semantic, Value, Detail>(
    value: Value,
    detail: Detail,
) -> Result<(), GateError>
where
    Value: Into<ValueText<'semantic>>,
    Detail: Into<DetailText<'semantic>>,
{
    let detail = detail.into().0;
    let value = value.into().0;
    if value.is_empty() {
        Err(GateError::operational(detail))
    }
    else {
        Ok(())
    }
}

/// Return whether a character is accepted after the first ref-token character.
#[inline]
fn is_ref_body_char<Ch>(ch: Ch) -> impl Into<RefBodyCharFlag>
where
    Ch: Into<ChCharacter>,
{
    let ch = ch.into().0;
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '/' | '-')
}

/// Return whether a character is a lowercase hexadecimal digit.
#[inline]
fn is_lower_hex_digit<Ch>(ch: Ch) -> impl Into<LowerHexDigitFlag>
where
    Ch: Into<ChCharacter>,
{
    let ch = ch.into().0;
    ch.is_ascii_digit() || matches!(ch, 'a' | 'b' | 'c' | 'd' | 'e' | 'f')
}

/// Already-resolved facts needed to validate a scheduled mutation range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) struct ScheduledRangeInput<'input>
{
    /// User-supplied lower ref token.
    pub from_ref: &'input str,
    /// User-supplied upper ref token.
    pub to_ref: &'input str,
    /// Resolved lower commit id.
    pub from_oid: &'input str,
    /// Resolved upper commit id.
    pub to_oid: &'input str,
    /// Resolved current `HEAD` commit id.
    pub head_oid: &'input str,
    /// Whether `from_oid` is an ancestor of `to_oid`.
    pub from_is_ancestor_of_to: bool,
}

/// The kind of host-side diff plan being rendered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) enum DiffKind
{
    /// Merge campaigns diff the branch against `main` with three-dot semantics.
    Merge,
    /// Push campaigns use the shared push-range resolver's mode.
    Push(PushRangeMode),
    /// Scheduled campaigns use validated caller refs resolved to current
    /// `HEAD`.
    Scheduled,
}

/// The three push-range modes shared with other push gates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) enum PushRangeMode
{
    /// The pushed lower and upper endpoints share history.
    Range,
    /// The lower endpoint is missing or unrelated, so the root-to-tip history
    /// is used.
    Full,
    /// No lower bound or root is available, so only the last commit is checked.
    Last,
}

/// A validated push-range plan from the shared push resolver.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PushRangePlan
{
    /// Diff the pushed range as `from...to`.
    Range
    {
        /// Lower endpoint for the push.
        from: String,
        /// Upper endpoint for the push.
        to: String,
    },
    /// Diff the full pushed history as `root...to`.
    Full
    {
        /// Root commit reachable from the pushed tip.
        root: String,
        /// Upper endpoint for the push.
        to: String,
    },
    /// Diff only `HEAD~1...HEAD`.
    Last
    {
        /// Upper endpoint retained for traceability even though the historic
        /// Nushell contract renders the literal `HEAD~1...HEAD` diff.
        to: String,
    },
}

impl PushRangePlan
{
    /// Build a shared-history push plan.
    ///
    /// # Contract
    /// - requires: `from` and `to` are nonempty refs or object ids from the
    ///   push resolver.
    /// - ensures: preserves the resolver's `range` mode without resolving refs.
    /// - provides: a typed plan that renders `git diff from...to`.
    /// - fails: returns an operational error if either required endpoint is
    ///   empty.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational error for an empty lower or upper endpoint.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — empty lower/upper and successful rendering
    ///   are separated by exact push-mode witnesses.
    /// - witness: `mutants::range::tests::push_range_modes_render_exact_three_dot_specs`
    #[inline]
    pub fn range<'semantic, Lower, Upper>(
        from: Lower,
        to: Upper,
    ) -> Result<Self, GateError>
    where
        Lower: Into<FromText<'semantic>>,
        Upper: Into<ToText<'semantic>>,
    {
        let to = to.into().0;
        let from = from.into().0;
        require_nonempty(
            from,
            "mutants-vm: push range mode requires a lower endpoint",
        )?;
        require_nonempty(to, "mutants-vm: push range mode requires an upper endpoint")?;
        Ok(Self::Range {
            from: String::from(from),
            to: String::from(to),
        })
    }

    /// Build a force-rewrite push plan.
    ///
    /// # Contract
    /// - requires: `root` and `to` are nonempty refs or object ids from the
    ///   push resolver.
    /// - ensures: preserves the resolver's `full` mode without resolving refs.
    /// - provides: a typed plan that renders `git diff root...to`.
    /// - fails: returns an operational error if either required endpoint is
    ///   empty.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational error for an empty root or upper endpoint.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — missing root and successful rendering are
    ///   separated by exact push-mode witnesses.
    /// - witness: `mutants::range::tests::push_range_modes_render_exact_three_dot_specs`
    #[inline]
    pub fn full<'semantic, Root, Upper>(
        root: Root,
        to: Upper,
    ) -> Result<Self, GateError>
    where
        Root: Into<RootText<'semantic>>,
        Upper: Into<ToText<'semantic>>,
    {
        let to = to.into().0;
        let root = root.into().0;
        require_nonempty(root, "mutants-vm: push full mode requires a root endpoint")?;
        require_nonempty(to, "mutants-vm: push full mode requires an upper endpoint")?;
        Ok(Self::Full {
            root: String::from(root),
            to: String::from(to),
        })
    }

    /// Build a last-commit push plan.
    ///
    /// # Contract
    /// - requires: `to` is the nonempty pushed upper endpoint.
    /// - ensures: preserves the resolver's `last` mode and literal
    ///   `HEAD~1...HEAD` diff contract.
    /// - provides: a typed plan that renders the one-commit push diff.
    /// - fails: returns an operational error if the upper endpoint is empty.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns an operational error for an empty upper endpoint.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — missing upper endpoint and literal last
    ///   diff rendering are separated by exact push-mode witnesses.
    /// - witness: `mutants::range::tests::push_range_modes_render_exact_three_dot_specs`
    #[inline]
    pub fn last<'semantic, Upper>(to: Upper) -> Result<Self, GateError>
    where
        Upper: Into<ToText<'semantic>>,
    {
        let to = to.into().0;
        require_nonempty(to, "mutants-vm: push last mode requires an upper endpoint")?;
        Ok(Self::Last {
            to: String::from(to),
        })
    }
}

/// A pure `git diff` argument plan.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) struct GitDiffPlan
{
    /// The source of the diff semantics.
    pub kind: DiffKind,
    /// Arguments to pass after `git`, starting with `diff`.
    pub args: Vec<OsString>,
}

impl GitDiffPlan
{
    /// Borrow the planned Git arguments.
    #[inline]
    #[must_use]
    pub(crate) fn args(&self) -> &[OsString]
    {
        &self.args
    }
}

/// A host-side decision after inspecting a unified diff.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) enum RangeCampaignPlan
{
    /// Run cargo-mutants in `--in-diff` mode using the rendered Git diff.
    RunInDiff
    {
        /// Diff command to materialize before entering the guest.
        diff: GitDiffPlan,
    },
    /// Skip the campaign because the diff cannot select Rust mutants.
    NoRustChanges
    {
        /// Stable report label to publish for the no-op campaign.
        report: &'static str,
    },
}

#[cfg(test)]
mod tests
{
    //! Behavioral tests for pure mutation range planning.

    use alloc::format;
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;
    use std::ffi::OsString;

    use super::*;

    /// Assert that an error display contains a stable fragment.
    fn assert_error_contains<'semantic, Needle>(
        result: Result<GitDiffPlan, GateError>,
        needle: Needle,
    )
    where
        Needle: Into<NeedleText<'semantic>>,
    {
        let needle = needle.into().0;
        let error = result.expect_err("fixture must fail");
        let rendered = error.to_string();
        assert!(
            rendered.contains(needle),
            "error `{rendered}` did not contain `{needle}`"
        );
    }

    /// Merge campaigns preserve three-dot `main...HEAD` semantics.
    #[test]
    fn merge_diff_uses_main_three_dot_head()
    {
        let plan = merge_diff_plan();

        assert_eq!(
            vec![String::from("diff"), String::from("main...HEAD")],
            arg_text(plan.args()),
            "merge diff must use exact three-dot main range"
        );
        assert_eq!(DiffKind::Merge, plan.kind, "merge kind must be stable");
    }

    /// Push resolver modes render their exact historic diff specs.
    #[test]
    fn push_range_modes_render_exact_three_dot_specs()
    {
        let range = PushRangePlan::range("base", "tip").expect("range fixture is valid");
        let full = PushRangePlan::full("root", "tip").expect("full fixture is valid");
        let last = PushRangePlan::last("tip").expect("last fixture is valid");

        assert_eq!(
            vec![String::from("diff"), String::from("base...tip")],
            arg_text(push_diff_plan(&range).args()),
            "range mode must diff from...to"
        );
        assert_eq!(
            vec![String::from("diff"), String::from("root...tip")],
            arg_text(push_diff_plan(&full).args()),
            "full mode must diff root...to"
        );
        assert_eq!(
            vec![String::from("diff"), String::from("HEAD~1...HEAD")],
            arg_text(push_diff_plan(&last).args()),
            "last mode must preserve literal HEAD one-commit diff"
        );
        assert!(
            PushRangePlan::range("", "tip").is_err(),
            "range mode must require a lower endpoint"
        );
        assert!(
            PushRangePlan::full("", "tip").is_err(),
            "full mode must require a root endpoint"
        );
        assert!(
            PushRangePlan::last("").is_err(),
            "last mode must require an upper endpoint"
        );
    }

    /// Convert OS arguments into UTF-8 test strings.
    fn arg_text(args: &[OsString]) -> Vec<String>
    {
        args.iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    /// Scheduled ref validation rejects malformed user tokens before Git use.
    #[test]
    fn scheduled_ref_tokens_reject_malformed_inputs()
    {
        let malformed = [
            "",
            " week-start",
            "week-start ",
            "../main",
            "week@main",
            "week..main",
            "main@{upstream}",
            "refs//heads/main",
            "refs/heads/main/",
            "refs/heads/main.",
            "refs/heads/main.lock",
        ];

        for token in malformed {
            assert!(
                validate_scheduled_ref_token(token, "from").is_err(),
                "token `{token}` must be rejected"
            );
        }
        assert_eq!(
            "week-start_1/main",
            crate::semantic_value::<AsStrText<'_>>(
                validate_scheduled_ref_token("week-start_1/main", "from")
                    .expect("safe token must validate")
                    .as_str()
            )
            .0,
            "safe scheduled token must be preserved exactly"
        );
    }

    /// The scheduled-ref body whitelist matches the declared ASCII alphabet.
    #[test]
    fn scheduled_ref_token_ascii_whitelist_is_exact()
    {
        for byte in 0_u8 ..= 127_u8 {
            let ch = char::from(byte);
            let candidate = format!("a{ch}b");
            let accepted = validate_scheduled_ref_token(&candidate, "from").is_ok();
            let expected = ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '/' | '-');
            assert_eq!(
                accepted, expected,
                "ASCII character `{ch}` must match the scheduled-ref whitelist"
            );
        }
    }

    /// Scheduled range planning rejects invalid commit ids and unsafe topology.
    #[test]
    fn scheduled_range_rejects_invalid_oid_and_topology()
    {
        let mut input = scheduled_input();
        input.from_oid = "abc";
        assert_error_contains(scheduled_diff_plan(&input), "invalid commit id");

        let from_oid = oid('a');
        let to_oid = oid('b');
        let head_oid = oid('c');
        let stale = ScheduledRangeInput {
            from_ref: "week-start",
            to_ref: "week-end",
            from_oid: &from_oid,
            to_oid: &to_oid,
            head_oid: &head_oid,
            from_is_ancestor_of_to: true,
        };
        assert_error_contains(scheduled_diff_plan(&stale), "current HEAD");

        let unrelated = ScheduledRangeInput {
            head_oid: &to_oid,
            from_is_ancestor_of_to: false,
            ..stale
        };
        assert_error_contains(scheduled_diff_plan(&unrelated), "reversed or unrelated");
    }

    /// Scheduled ranges render resolved object ids with three-dot semantics.
    #[test]
    fn scheduled_range_renders_resolved_three_dot_diff()
    {
        let plan = scheduled_diff_plan(&scheduled_input()).expect("scheduled fixture is valid");

        assert_eq!(
            vec![
                String::from("diff"),
                String::from(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                ),
            ],
            arg_text(plan.args()),
            "scheduled diff must use resolved object ids and three-dot semantics"
        );
        assert_eq!(
            DiffKind::Scheduled,
            plan.kind,
            "scheduled kind must be stable"
        );
    }

    /// Build a successful scheduled input fixture.
    fn scheduled_input() -> ScheduledRangeInput<'static>
    {
        ScheduledRangeInput {
            from_ref: "week-start",
            to_ref: "HEAD",
            from_oid: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            to_oid: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            head_oid: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            from_is_ancestor_of_to: true,
        }
    }

    /// Build a valid forty-character lowercase hexadecimal oid fixture.
    fn oid<Ch>(ch: Ch) -> String
    where
        Ch: Into<ChCharacter>,
    {
        let ch = ch.into().0;
        core::iter::repeat_n(ch, GIT_OID_HEX_CHARS).collect()
    }

    /// Non-Rust diffs become an explicit no-op campaign.
    #[test]
    fn non_rust_diff_is_noop_campaign()
    {
        let diff = "diff --git a/docs/guide.md b/docs/guide.md\n+++ b/docs/guide.md\n+text\n";
        let decision = plan_in_diff_campaign(merge_diff_plan(), diff);

        assert_eq!(
            RangeCampaignPlan::NoRustChanges {
                report: NO_RUST_CHANGES_REPORT,
            },
            decision,
            "non-Rust diffs must skip the VM campaign"
        );
    }

    /// Rust detection only inspects unified-diff target lines.
    #[test]
    fn rust_diff_detection_matches_target_lines_only()
    {
        let non_targets = "diff --git a/src/lib.rs b/src/lib.rs\n+ b/src/lib.rs\n--- a/src/lib.rs\n+++ /dev/null\n";
        let rust_target =
            "diff --git a/src/lib.rs b/src/lib.rs\n+++ b/src/lib.rs\n@@\n+fn changed() {}\n";
        let renamed_non_rust = "diff --git a/README.md b/README.md\n+++ b/README.md\n";

        assert!(
            !diff_touches_rust(non_targets).into().0,
            "deleted or body mentions must not count as Rust targets"
        );
        assert!(
            diff_touches_rust(rust_target).into().0,
            "+++ Rust target line must count"
        );
        assert!(
            !diff_touches_rust(renamed_non_rust).into().0,
            "non-Rust target line must not count"
        );
    }
}
