//! Typed coverage-policy domain model.
//!
//! # Contract
//! - requires: callers supply repository-relative production paths and already
//!   parsed coverage numbers at the boundary constructors.
//! - ensures: coverage percentages are represented as integer hundredths and
//!   production paths are stable, slash-separated map keys.
//! - provides: typed values shared by the parser, policy, and renderer.
//! - fails: constructors return typed gate errors or parse errors instead of
//!   accepting noncanonical paths and imprecise percentages.
//! - panics: none.
//! - intension: values sort by their emitted policy key, making `BTreeMap`
//!   joins and rendering deterministic.
//!
//! # Adequacy
//! - hypothesis: L3 only — path-boundary fixtures, duplicate normalized names,
//!   count-derived percentage boundaries, and exact diagnostic display
//!   witnesses kill mutants in normalization, precision, and ordering
//!   decisions.
//! - witness: `coverage::model::tests::percent_floors_down_without_float_math`
//! - witness: `coverage::model::tests::floor_key_validation_rejects_noncanonical_paths`
//! - witness: `coverage::policy::tests::duplicate_normalized_summary_rows_fail`

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::integer_division,
        clippy::manual_checked_ops,
        clippy::unwrap_used,
        reason = "coverage model tests compute exact percent fixture formulas"
    )
)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use std::path::Component;
use std::path::Path;

use crate::Finding;
use crate::GateError;

crate::semantic_copy!(pub struct CoveredCount(u64));
crate::semantic_copy!(pub struct CountCount(u64));
crate::semantic_copy!(pub struct HundredthsCount(u32));
crate::semantic_copy!(pub struct HasHiddenPrecisionFlag(bool));
crate::semantic_str!(pub struct RawText);
crate::semantic_str!(pub struct FilenameText);
crate::semantic_str!(pub struct ExpectedText);
crate::semantic_str!(pub struct LiteralText);
crate::semantic_str!(pub struct TextText);
crate::semantic_str!(pub struct TokenText);
crate::semantic_str!(pub struct FileText);
crate::semantic_copy!(pub struct ParseDecimalDigitsCount(u32));
crate::semantic_copy!(pub struct NonfiniteLiteralFlag(bool));
crate::semantic_copy!(pub struct ProductionFileFlag(bool));
crate::semantic_copy!(pub struct CrateRootFixtureFlag(bool));
crate::semantic_copy!(pub struct ForbiddenFloorSegmentFlag(bool));
crate::semantic_str!(pub struct AsStrText);

/// Parsed fractional percent hundredths and hidden precision state.
#[derive(Clone, Copy, Debug, PartialEq)]
struct FractionHundredths
{
    /// First two fractional digits normalized to hundredths.
    hundredths: HundredthsCount,
    /// Whether any later digit would be lost by the fixed precision model.
    has_hidden_precision: HasHiddenPrecisionFlag,
}

/// Maximum representable percent in hundredths.
const MAX_PERCENT_HUNDREDTHS: u32 = 10_000;

/// Default per-file policy target, encoded as hundredths of one percent.
pub(super) const DEFAULT_TARGET_PERCENT: Percent = Percent { hundredths: 8_000 };

/// A validated percentage stored as integer hundredths.
///
/// # Contract
/// - requires: constructors receive either a decimal percent literal or already
///   scaled hundredths.
/// - ensures: stored values are always in `0.00..=100.00` and display with
///   exactly two fractional digits.
/// - provides: a float-free comparison key for coverage policy decisions.
/// - fails: constructors reject nonfinite, negative, out-of-range, imprecise,
///   or overflowing inputs through [`PercentParseError`] or `None`.
/// - panics: none.
/// - intension: arithmetic uses checked integer scaling and floor division of
///   `covered * 10000 / count`.
///
/// # Adequacy
/// - hypothesis: L3 only — zero-count, ordinary ratio, hidden-precision, and
///   upper-bound fixtures distinguish every constructor branch and display
///   decision.
/// - witness: `coverage::model::tests::percent_floors_down_without_float_math`
/// - witness: `coverage::model::tests::percent_parser_rejects_hidden_precision`
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Percent
{
    /// Percentage scaled by 100.
    hundredths: u32,
}

impl Percent
{
    /// Build a percentage from integer hundredths.
    #[inline]
    #[must_use]
    fn from_hundredths(hundredths: impl Into<HundredthsCount>) -> Option<Self>
    {
        let hundredths = hundredths.into().0;
        if hundredths <= MAX_PERCENT_HUNDREDTHS {
            return Some(Self { hundredths });
        }
        None
    }

    /// Return the integer hundredths representation.
    #[inline]
    #[must_use]
    pub fn hundredths(self) -> impl Into<HundredthsCount>
    {
        self.hundredths
    }

    /// Parse a decimal percent literal, requiring exact two-decimal precision.
    ///
    /// # Contract
    /// - requires: `literal` is the source spelling of a JSON or TOML numeric
    ///   percent token.
    /// - ensures: accepts only finite decimal values whose nonzero precision is
    ///   at most two decimal places.
    /// - provides: the equivalent integer-hundredths value.
    /// - fails: returns [`PercentParseError`] for nonnumeric, nonfinite,
    ///   negative, out-of-range, overflowing, or hidden-precision spellings.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the exact parse category needed by policy diagnostics.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — ordinary, trailing-zero, hidden-nonzero,
    ///   nonfinite, and bound-crossing literals distinguish all categories.
    /// - witness: `coverage::model::tests::percent_parser_rejects_hidden_precision`
    #[inline]
    pub(super) fn parse_exact<'semantic>(
        literal: impl Into<LiteralText<'semantic>>
    ) -> Result<Self, PercentParseError>
    {
        let literal = literal.into().0;
        let decimal = DecimalLiteral::parse(literal)?;
        if decimal.has_hidden_precision {
            return Err(PercentParseError::HiddenPrecision);
        }
        Self::from_hundredths(decimal.hundredths).ok_or(PercentParseError::OutOfRange)
    }

    /// Floor a covered/count line ratio to two percent decimals.
    ///
    /// # Contract
    /// - requires: `covered <= count`; callers validate this before deriving
    ///   the percent.
    /// - ensures: returns `0.00` for zero-count rows and otherwise
    ///   `floor(covered * 10000 / count)` hundredths.
    /// - provides: the canonical llvm-cov comparison value without floating
    ///   arithmetic.
    /// - fails: returns [`PercentParseError::Overflow`] if checked arithmetic
    ///   or conversion cannot represent the scaled ratio.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`PercentParseError::Overflow`] on checked arithmetic failure.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — zero-count, exact 100%, recurring decimal, and
    ///   near-boundary ratios kill division, scaling, and floor mutants.
    /// - witness: `coverage::model::tests::percent_floors_down_without_float_math`
    #[inline]
    pub(super) fn from_counts(
        covered: impl Into<CoveredCount>,
        count: impl Into<CountCount>,
    ) -> Result<Self, PercentParseError>
    {
        let count = count.into().0;
        let covered = covered.into().0;
        if count == 0 {
            return Ok(Self { hundredths: 0 });
        }
        let scaled = u128::from(covered)
            .checked_mul(u128::from(MAX_PERCENT_HUNDREDTHS))
            .ok_or(PercentParseError::Overflow)?;
        let quotient = scaled
            .checked_div(u128::from(count))
            .ok_or(PercentParseError::Overflow)?;
        let hundredths = u32::try_from(quotient).map_err(|_error| PercentParseError::Overflow)?;
        Self::from_hundredths(hundredths).ok_or(PercentParseError::OutOfRange)
    }

    /// Parse a decimal percent literal at the policy's two-decimal precision.
    ///
    /// # Contract
    /// - requires: `literal` is the source spelling of a JSON numeric token.
    /// - ensures: returns the floor-down integer-hundredths value, including
    ///   when llvm-cov emits additional fractional precision.
    /// - provides: the declared percentage key used to cross-check exact line
    ///   counts at the policy boundary.
    /// - fails: returns [`PercentParseError`] for nonnumeric, nonfinite,
    ///   negative, out-of-range, or overflowing spellings.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the parse category for invalid percent literals.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — matching full-precision and mismatching
    ///   declarations distinguish accepted llvm-cov output from corrupt
    ///   redundant metrics.
    /// - witness: `coverage::policy::tests::line_percentage_accepts_matching_full_precision`
    /// - witness: `coverage::policy::tests::line_percentage_must_match_counts_exactly`
    #[inline]
    pub(super) fn parse_declared<'semantic>(
        literal: impl Into<LiteralText<'semantic>>
    ) -> Result<Self, PercentParseError>
    {
        let literal = literal.into().0;
        let decimal = DecimalLiteral::parse(literal)?;
        Self::from_hundredths(decimal.hundredths).ok_or(PercentParseError::OutOfRange)
    }
}

impl fmt::Display for Percent
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        let whole = self.hundredths.checked_div(100).unwrap_or(0);
        let fraction = self.hundredths.checked_rem(100).unwrap_or(0);
        write!(f, "{whole}.{fraction:02}")
    }
}

/// Decimal percent parse failure category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PercentParseError
{
    /// The token is not a decimal numeric literal accepted by the policy.
    Invalid,
    /// The token spells a nonfinite value such as `nan` or `inf`.
    NonFinite,
    /// The token is below zero.
    Negative,
    /// The scaled value is above `100.00`.
    OutOfRange,
    /// Nonzero precision exists beyond two decimal places.
    HiddenPrecision,
    /// Checked arithmetic could not represent the scaled value.
    Overflow,
}

/// Internal parsed decimal representation.
struct DecimalLiteral
{
    /// Floor-down hundredths value.
    hundredths: u32,
    /// Whether any nonzero digit appeared after the second fractional digit.
    has_hidden_precision: bool,
}

impl DecimalLiteral
{
    /// Parse a finite decimal token into floor-down hundredths.
    fn parse<'semantic>(
        literal: impl Into<LiteralText<'semantic>>
    ) -> Result<Self, PercentParseError>
    {
        let literal = literal.into().0;
        let trimmed = literal.trim();
        if trimmed.is_empty() {
            return Err(PercentParseError::Invalid);
        }
        if is_nonfinite_literal(trimmed).into().0 {
            return Err(PercentParseError::NonFinite);
        }
        let unsigned = match trimmed.strip_prefix('+') {
            | Some(value) => value,
            | None => trimmed,
        };
        if unsigned.starts_with('-') {
            return Err(PercentParseError::Negative);
        }
        let mut pieces = unsigned.split('.');
        let whole_text = pieces.next().unwrap_or_default();
        let fraction_text = pieces.next();
        if pieces.next().is_some() || whole_text.is_empty() {
            return Err(PercentParseError::Invalid);
        }

        let whole = parse_decimal_digits(whole_text).map(|value| value.into().0)?;
        let whole_hundredths = whole.checked_mul(100).ok_or(PercentParseError::Overflow)?;
        let fraction = match fraction_text {
            | Some(text) => parse_fraction_hundredths(text)?,
            | None => FractionHundredths {
                hundredths: HundredthsCount(0),
                has_hidden_precision: HasHiddenPrecisionFlag(false),
            },
        };
        let hundredths = whole_hundredths
            .checked_add(fraction.hundredths.0)
            .ok_or(PercentParseError::Overflow)?;
        if hundredths > MAX_PERCENT_HUNDREDTHS {
            return Err(PercentParseError::OutOfRange);
        }
        Ok(Self {
            hundredths,
            has_hidden_precision: fraction.has_hidden_precision.0,
        })
    }
}

/// Parse an unsigned decimal digit run.
fn parse_decimal_digits<'semantic>(
    text: impl Into<TextText<'semantic>>
) -> Result<impl Into<ParseDecimalDigitsCount>, PercentParseError>
{
    let text = text.into().0;
    if text.is_empty() {
        return Err(PercentParseError::Invalid);
    }
    let mut value = 0_u32;
    for character in text.chars() {
        let Some(digit) = character.to_digit(10)
        else {
            return Err(PercentParseError::Invalid);
        };
        value = value.checked_mul(10).ok_or(PercentParseError::Overflow)?;
        value = value
            .checked_add(digit)
            .ok_or(PercentParseError::Overflow)?;
    }
    Ok(value)
}

/// Parse the fractional side of a decimal percent token.
fn parse_fraction_hundredths<'semantic>(
    text: impl Into<TextText<'semantic>>
) -> Result<FractionHundredths, PercentParseError>
{
    let text = text.into().0;
    if text.is_empty() {
        return Err(PercentParseError::Invalid);
    }
    let mut hundredths = 0_u32;
    let mut digits_seen = 0_usize;
    let mut has_hidden_precision = false;
    for character in text.chars() {
        let Some(digit) = character.to_digit(10)
        else {
            return Err(PercentParseError::Invalid);
        };
        if digits_seen < 2 {
            hundredths = hundredths
                .checked_mul(10)
                .ok_or(PercentParseError::Overflow)?;
            hundredths = hundredths
                .checked_add(digit)
                .ok_or(PercentParseError::Overflow)?;
        }
        else if digit != 0 {
            has_hidden_precision = true;
        }
        digits_seen = digits_seen
            .checked_add(1)
            .ok_or(PercentParseError::Overflow)?;
    }
    if digits_seen == 1 {
        hundredths = hundredths
            .checked_mul(10)
            .ok_or(PercentParseError::Overflow)?;
    }
    Ok(FractionHundredths {
        hundredths: HundredthsCount(hundredths),
        has_hidden_precision: HasHiddenPrecisionFlag(has_hidden_precision),
    })
}

/// Return whether a token spells a TOML nonfinite float.
fn is_nonfinite_literal<'semantic>(
    token: impl Into<TokenText<'semantic>>
) -> impl Into<NonfiniteLiteralFlag>
{
    let token = token.into().0;
    let lowered = token.to_ascii_lowercase();
    matches!(
        lowered.as_str(),
        "nan" | "+nan" | "-nan" | "inf" | "+inf" | "-inf"
    )
}

/// Normalize a path for diagnostic and git-policy comparison.
#[inline]
#[must_use]
pub(super) fn slash_path(path: &Path) -> String
{
    path.to_string_lossy().replace('\\', "/")
}

/// Return whether a slash path is in the production Rust source domain.
#[inline]
#[must_use]
fn is_production_file<'semantic>(
    file: impl Into<FileText<'semantic>>
) -> impl Into<ProductionFileFlag>
{
    let file = file.into().0;
    file.starts_with("crates/")
        && Path::new(file)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"))
        && !is_crate_root_fixture(file).into().0
}

/// Return whether a file sits under crate-root tests, benches, or examples.
fn is_crate_root_fixture<'semantic>(
    file: impl Into<FileText<'semantic>>
) -> impl Into<CrateRootFixtureFlag>
{
    let file = file.into().0;
    let mut segments = file.split('/');
    if segments.next() != Some("crates") {
        return false;
    }
    if segments.next().is_none() {
        return false;
    }
    matches!(segments.next(), Some("tests" | "benches" | "examples"))
}

/// Return whether a floor key contains a forbidden path segment.
fn has_forbidden_floor_segment<'semantic>(
    file: impl Into<FileText<'semantic>>
) -> impl Into<ForbiddenFloorSegmentFlag>
{
    let file = file.into().0;
    for segment in file.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return true;
        }
    }
    false
}

/// Normalize a relative summary filename.
fn normalize_relative_summary<'semantic>(
    raw: impl Into<RawText<'semantic>>
) -> Result<String, GateError>
{
    let raw = raw.into().0;
    let mut segments = Vec::new();
    for segment in raw.split('/') {
        if segment == ".." {
            return Err(coverage_error(format!(
                "coverage filename escapes repository root: {raw}"
            )));
        }
        if segment.is_empty() || segment == "." {
            continue;
        }
        segments.push(segment);
    }
    Ok(segments.join("/"))
}

/// Normalize an absolute summary filename and strip the repository root.
fn normalize_absolute_summary<'semantic>(
    repo_root: &Path,
    raw: impl Into<RawText<'semantic>>,
) -> Result<String, GateError>
{
    let raw = raw.into().0;
    let root = expanded_slash_path(repo_root)?;
    let expanded = normalize_absolute_slash(raw)?;
    if let Some(stripped) = expanded
        .strip_prefix(&root)
        .and_then(|suffix| suffix.strip_prefix('/'))
    {
        return Ok(stripped.to_owned());
    }
    Err(coverage_error(format!(
        "coverage filename is outside repository root: {raw}"
    )))
}

/// Expand a repository root to an absolute slash path.
fn expanded_slash_path(path: &Path) -> Result<String, GateError>
{
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    }
    else {
        let current_dir = crate::support::HOST_FILESYSTEM.current_dir()?;
        current_dir.join(path)
    };
    normalize_components(&absolute)
}

/// Lexically normalize an absolute slash path.
fn normalize_absolute_slash<'semantic>(
    raw: impl Into<RawText<'semantic>>
) -> Result<String, GateError>
{
    let raw = raw.into().0;
    if !raw.starts_with('/') {
        return Err(coverage_error(format!(
            "coverage filename is outside repository root: {raw}"
        )));
    }
    normalize_slash_segments(raw)
}

/// Lexically normalize a path using platform components.
fn normalize_components(path: &Path) -> Result<String, GateError>
{
    let mut segments = Vec::new();
    let mut absolute = false;
    for component in path.components() {
        match component {
            | Component::RootDir => absolute = true,
            | Component::CurDir => {},
            | Component::ParentDir => {
                if segments.pop().is_none() {
                    return Err(coverage_error(format!(
                        "coverage filename escapes repository root: {}",
                        path.display()
                    )));
                }
            },
            | Component::Normal(segment) => segments.push(segment.to_string_lossy().into_owned()),
            | Component::Prefix(prefix) => {
                segments.push(prefix.as_os_str().to_string_lossy().into_owned());
            },
        }
    }
    let mut text = String::new();
    if absolute {
        text.push('/');
    }
    text.push_str(&segments.join("/"));
    while text.ends_with('/') && text.len() > 1 {
        let removed = text.pop();
        if removed.is_none() {
            break;
        }
    }
    Ok(text)
}

/// Build a stable coverage operational error.
#[inline]
pub(super) fn coverage_error<Detail>(detail: Detail) -> GateError
where
    Detail: Into<String>,
{
    GateError::operational(detail)
}

/// Lexically normalize an already slash-separated absolute path.
fn normalize_slash_segments<'semantic>(
    raw: impl Into<RawText<'semantic>>
) -> Result<String, GateError>
{
    let raw = raw.into().0;
    let mut segments = Vec::new();
    for segment in raw.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." {
            if segments.pop().is_none() {
                return Err(coverage_error(format!(
                    "coverage filename escapes repository root: {raw}"
                )));
            }
            continue;
        }
        segments.push(segment);
    }
    let mut text = String::from("/");
    text.push_str(&segments.join("/"));
    Ok(text)
}

/// Canonical repository-relative production Rust source path.
///
/// # Contract
/// - requires: constructors receive slash or platform-separated path text.
/// - ensures: stored paths are relative slash paths under `crates/`, have a
///   Rust source extension, and are not crate-root `tests`, `benches`, or
///   `examples` targets.
/// - provides: a stable key for coverage summary/floor joins.
/// - fails: rejects floor keys that are absolute, empty, dot-segmented,
///   backslash-spelled, or outside the production source domain; summary
///   filenames that escape the repository or are absolute outside it fail.
/// - panics: none.
/// - intension: ordering is lexicographic by the emitted slash path.
///
/// # Adequacy
/// - hypothesis: L3 only — relative, absolute-in-root, absolute-outside,
///   parent-segment, fixture-target, and duplicate-normalization fixtures kill
///   all path-domain decisions.
/// - witness: `coverage::model::tests::floor_key_validation_rejects_noncanonical_paths`
/// - witness: `coverage::policy::tests::absolute_summary_paths_normalize_to_repo_relative`
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ProductionFile
{
    /// Canonical slash-separated repository-relative path.
    path: String,
}

impl ProductionFile
{
    /// Validate a TOML floor key as a canonical production path.
    ///
    /// # Contract
    /// - requires: `file` is one `[files]` or `[new_file_exemptions]` TOML key.
    /// - ensures: returns a canonical production file key identical to `file`.
    /// - provides: strict policy-file path validation.
    /// - fails: returns a stable operational error when `file` is noncanonical
    ///   or outside the production Rust domain.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Operational`] with the legacy diagnostic text for a
    /// noncanonical policy path.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — slash, backslash, absolute, dot, parent, empty
    ///   segment, fixture, and non-Rust keys distinguish each predicate.
    /// - witness: `coverage::model::tests::floor_key_validation_rejects_noncanonical_paths`
    #[inline]
    pub(super) fn from_floor_key<'semantic>(
        file: impl Into<FileText<'semantic>>
    ) -> Result<Self, GateError>
    {
        let file = file.into().0;
        let normalized = file.replace('\\', "/");
        if normalized != file
            || normalized.starts_with('/')
            || has_forbidden_floor_segment(&normalized).into().0
            || !is_production_file(&normalized).into().0
        {
            return Err(coverage_error(format!(
                "coverage floor path must be a canonical production Rust file under crates/: {file}"
            )));
        }
        Ok(Self { path: normalized })
    }

    /// Normalize a cargo-llvm-cov filename and return a production key when in
    /// scope.
    ///
    /// # Contract
    /// - requires: `repo_root` is the repository root used for absolute summary
    ///   filenames.
    /// - ensures: relative filenames are slash-normalized and absolute
    ///   filenames under `repo_root` are stripped to repository-relative paths.
    /// - provides: a production-key candidate for measured coverage joins.
    /// - fails: returns stable operational errors for parent-segment relative
    ///   escapes or absolute filenames outside `repo_root`.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Operational`] for escaping/outside filenames and
    /// for current-directory resolution failure when `repo_root` is
    /// relative.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — mixed absolute/relative fixtures and
    ///   outside-root paths kill root-prefix and escape mutants.
    /// - witness: `coverage::policy::tests::absolute_summary_paths_normalize_to_repo_relative`
    /// - witness: `coverage::policy::tests::production_path_boundaries_fail_closed`
    #[inline]
    pub(super) fn from_summary_filename<'semantic>(
        repo_root: &Path,
        filename: impl Into<FilenameText<'semantic>>,
    ) -> Result<Option<Self>, GateError>
    {
        let filename = filename.into().0;
        let raw = filename.replace('\\', "/");
        let relative = if raw.starts_with('/') {
            normalize_absolute_summary(repo_root, &raw)?
        }
        else {
            normalize_relative_summary(&raw)?
        };
        if !is_production_file(&relative).into().0 {
            return Ok(None);
        }
        Ok(Some(Self { path: relative }))
    }

    /// Borrow the canonical slash path.
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> impl Into<AsStrText<'_>>
    {
        &self.path
    }
}

impl fmt::Display for ProductionFile
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(&self.path)
    }
}

/// One measured production file from an llvm-cov summary.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasuredCoverage
{
    /// Canonical production file key.
    pub file: ProductionFile,
    /// Covered line count from llvm-cov.
    pub covered: u64,
    /// Total line count from llvm-cov.
    pub count: u64,
    /// Declared and count-validated line coverage percent.
    pub percent: Percent,
}

/// Parsed per-file floor policy.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoverageFloors
{
    /// Policy target used for new files and ratchet caps.
    pub target_percent: Percent,
    /// Canonical per-production-file floors.
    pub files: alloc::collections::BTreeMap<ProductionFile, Percent>,
    /// Explicit new-file exemption reasons keyed by canonical production file.
    pub exemptions: alloc::collections::BTreeMap<ProductionFile, String>,
}

/// Report returned by the ratchet API.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RatchetReport
{
    /// Deterministically rendered TOML policy.
    pub toml: String,
    /// Count of existing floors raised by measured coverage.
    pub raised: usize,
    /// Count of measured files newly added to the policy.
    pub added: usize,
    /// Count of measured existing files whose floors did not change.
    pub unchanged: usize,
    /// Count of stale policy rows retained.
    pub stale: usize,
}

/// A semantic coverage policy violation.
///
/// # Contract
/// - ensures: display text exactly matches the retained Nushell diagnostic
///   wording for each policy-failure family.
/// - provides: typed failure cases before conversion into crate findings.
/// - panics: none.
/// - intension: conversion to [`Finding`] stores display text in `detail` and
///   the associated production path in `path`.
///
/// # Adequacy
/// - hypothesis: L3 only — one witness for each variant observes exact display
///   text and finding field placement.
/// - witness: `coverage::policy::tests::check_report_covers_policy_failure_families`
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum CoverageFailure
{
    /// A measured production file has no current floor row.
    MissingFloor
    {
        /// Measured file.
        file: ProductionFile,
        /// Measured percent.
        measured: Percent,
    },
    /// A current floor row has no measured production file.
    StaleFloor
    {
        /// Stale floor file.
        file: ProductionFile,
        /// Current floor.
        floor: Percent,
    },
    /// Measured coverage is below the current floor.
    Regression
    {
        /// Regressed file.
        file: ProductionFile,
        /// Measured percent.
        measured: Percent,
        /// Required floor.
        floor: Percent,
    },
    /// Current floor is below the base floor.
    FloorDecreased
    {
        /// File whose floor decreased.
        file: ProductionFile,
        /// Current floor.
        current: Percent,
        /// Base floor.
        base: Percent,
    },
    /// Current floor dropped below the allowed target clamp.
    TargetClampDecreased
    {
        /// File whose floor crossed the clamp.
        file: ProductionFile,
        /// Current floor.
        current: Percent,
        /// Allowed minimum after target change.
        allowed: Percent,
        /// Base policy target.
        old_target: Percent,
        /// Current policy target.
        new_target: Percent,
    },
    /// A base floor disappeared while the file still appears in current
    /// coverage.
    FloorDisappeared
    {
        /// File whose floor disappeared.
        file: ProductionFile,
    },
    /// A nonexempt new file did not start at the target floor.
    NewFloorWrongStart
    {
        /// New file.
        file: ProductionFile,
        /// Expected starting floor.
        expected: Percent,
        /// Current floor.
        got: Percent,
    },
    /// An exempt new file did not start at its measured baseline.
    ExemptNewFloorWrongStart
    {
        /// New exempt file.
        file: ProductionFile,
        /// Expected measured baseline floor.
        expected: Percent,
        /// Current floor.
        got: Percent,
    },
}

impl CoverageFailure
{
    /// Convert the semantic failure into the crate diagnostic row format.
    #[inline]
    #[must_use]
    pub(super) fn into_finding(self) -> Finding
    {
        let path = self.file().as_str().into().0.to_owned();
        Finding::new("coverage-policy", "", path, "", self.to_string())
    }

    /// Return the file associated with a failure.
    #[inline]
    #[must_use]
    fn file(&self) -> &ProductionFile
    {
        match *self {
            | Self::MissingFloor { ref file, .. }
            | Self::StaleFloor { ref file, .. }
            | Self::Regression { ref file, .. }
            | Self::FloorDecreased { ref file, .. }
            | Self::TargetClampDecreased { ref file, .. }
            | Self::FloorDisappeared { ref file }
            | Self::NewFloorWrongStart { ref file, .. }
            | Self::ExemptNewFloorWrongStart { ref file, .. } => file,
        }
    }
}

impl fmt::Display for CoverageFailure
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        match *self {
            | Self::MissingFloor { ref file, measured } => {
                write!(f, "missing floor for {file}: measured {measured}%")
            },
            | Self::StaleFloor { ref file, floor } => write!(
                f,
                "stale floor for {file}: floor {floor}% but file is absent from measured coverage"
            ),
            | Self::Regression {
                ref file,
                measured,
                floor,
            } => write!(
                f,
                "coverage regression for {file}: measured {measured}% below floor {floor}%"
            ),
            | Self::FloorDecreased {
                ref file,
                current,
                base,
            } => write!(
                f,
                "coverage floor decreased for {file}: current {current}% below base {base}%"
            ),
            | Self::TargetClampDecreased {
                ref file,
                current,
                allowed,
                old_target,
                new_target,
            } => write!(
                f,
                "coverage floor decreased past policy target for {file}: current {current}% below allowed {allowed}% after target changed from {old_target}% to {new_target}%"
            ),
            | Self::FloorDisappeared { ref file } => write!(
                f,
                "coverage floor disappeared for existing production file {file}"
            ),
            | Self::NewFloorWrongStart {
                ref file,
                expected,
                got,
            } => write!(
                f,
                "new coverage floor for {file} must start at {expected}%, got {got}%"
            ),
            | Self::ExemptNewFloorWrongStart {
                ref file,
                expected,
                got,
            } => write!(
                f,
                "exempt new coverage floor for {file} must start at measured baseline {expected}%, got {got}%"
            ),
        }
    }
}

#[cfg(test)]
mod tests
{
    use proptest::prelude::*;

    use super::Percent;
    use super::PercentParseError;
    use super::ProductionFile;
    crate::semantic_copy!(pub struct PercentTextHundredths(u64));

    /// Percent floors down and displays with stable precision.
    #[test]
    fn percent_floors_down_without_float_math()
    {
        assert_eq!(
            "0.00",
            Percent::from_counts(0, 0).unwrap().to_string(),
            "zero-count coverage should be 0.00",
        );
        assert_eq!(
            "33.33",
            Percent::from_counts(1, 3).unwrap().to_string(),
            "recurring coverage should floor down",
        );
        assert_eq!(
            "100.00",
            Percent::from_counts(1003, 1003).unwrap().to_string(),
            "full coverage should be 100.00",
        );
    }

    /// Percent parser rejects hidden nonzero precision while allowing trailing
    /// zeroes.
    #[test]
    fn percent_parser_rejects_hidden_precision()
    {
        assert_eq!(
            "80.00",
            Percent::parse_exact("80.000").unwrap().to_string(),
            "trailing fractional zeroes should be accepted",
        );
        assert_eq!(
            PercentParseError::HiddenPrecision,
            Percent::parse_exact("78.350009").unwrap_err(),
            "hidden nonzero precision should be rejected",
        );
        assert_eq!(
            PercentParseError::NonFinite,
            Percent::parse_exact("inf").unwrap_err(),
            "nonfinite literals should be rejected",
        );
    }

    proptest! {
        /// Count-derived percentages always floor to the exact hundredth rather
        /// than rounding up through floating-point formatting.
        #[test]
        fn count_percent_matches_floor_formula(covered in 0_u64..=10_000, extra in 0_u64..=10_000)
        {
            let count = covered + extra;
            let percent = Percent::from_counts(covered, count).map_err(|error| {
                TestCaseError::fail(format!("percent construction failed: {error:?}"))
            })?;
            let expected = if count == 0 {
                0
            }
            else {
                (covered * 10_000) / count
            };

            prop_assert_eq!(percent.to_string(), percent_text(expected));
        }

        /// Exact percent parsing preserves canonical hundredths while accepting
        /// only insignificant trailing fractional zeroes.
        #[test]
        fn exact_percent_parser_preserves_hundredths(
            hundredths in 0_u32..=10_000,
            trailing_zeroes in 0_usize..=4,
        )
        {
            let literal = format!(
                "{}.{:02}{}",
                hundredths / 100,
                hundredths % 100,
                "0".repeat(trailing_zeroes),
            );
            let percent = Percent::parse_exact(&literal).map_err(|error| {
                TestCaseError::fail(format!("percent parser rejected {literal}: {error:?}"))
            })?;

            prop_assert_eq!(percent.to_string(), percent_text(u64::from(hundredths)));
        }
    }

    /// Render a hundredths value with the policy's stable two-decimal
    /// precision.
    fn percent_text(hundredths: impl Into<PercentTextHundredths>) -> String
    {
        let hundredths = hundredths.into().0;
        format!("{}.{:02}", hundredths / 100, hundredths % 100)
    }

    /// Floor keys must already be canonical production Rust files.
    #[test]
    fn floor_key_validation_rejects_noncanonical_paths()
    {
        assert!(
            ProductionFile::from_floor_key("crates/demo/src/lib.rs").is_ok(),
            "canonical production path should be accepted",
        );
        assert!(
            ProductionFile::from_floor_key("crates/demo/src/../lib.rs").is_err(),
            "parent segments should be rejected",
        );
        assert!(
            ProductionFile::from_floor_key("/crates/demo/src/lib.rs").is_err(),
            "absolute paths should be rejected",
        );
        assert!(
            ProductionFile::from_floor_key("crates/demo/tests/lib.rs").is_err(),
            "crate-root test targets should be rejected",
        );
        assert!(
            ProductionFile::from_floor_key("crates/demo/src/lib.txt").is_err(),
            "non-Rust paths should be rejected",
        );
    }

    /// Percent parsing classifies boundary and malformed spellings without
    /// lossy floating-point rounding.
    #[test]
    fn percent_parser_classifies_boundaries_and_malformed_input()
    {
        assert_eq!(
            "80.00",
            Percent::parse_exact("+80").unwrap().to_string(),
            "leading plus should not change the parsed value",
        );
        assert_eq!(
            "33.33",
            Percent::parse_declared("33.3399").unwrap().to_string(),
            "declared llvm-cov percentages floor hidden precision",
        );
        assert_eq!(
            8_025,
            Percent::parse_exact("80.25").unwrap().hundredths().into().0,
            "hundredths expose the exact typed comparison key",
        );
        assert_eq!(
            PercentParseError::Invalid,
            Percent::parse_exact("").unwrap_err(),
            "empty literals are not numeric",
        );
        assert_eq!(
            PercentParseError::Invalid,
            Percent::parse_exact("80.").unwrap_err(),
            "an empty fractional side is malformed",
        );
        assert_eq!(
            PercentParseError::Invalid,
            Percent::parse_exact(".80").unwrap_err(),
            "an empty whole side is malformed",
        );
        assert_eq!(
            PercentParseError::Invalid,
            Percent::parse_exact("80.0.0").unwrap_err(),
            "multiple decimal points are malformed",
        );
        assert_eq!(
            PercentParseError::Invalid,
            Percent::parse_exact("80.a").unwrap_err(),
            "fractional sides must contain only decimal digits",
        );
        assert_eq!(
            PercentParseError::Negative,
            Percent::parse_exact("-0.01").unwrap_err(),
            "negative percentages are rejected",
        );
        assert_eq!(
            PercentParseError::OutOfRange,
            Percent::parse_exact("100.01").unwrap_err(),
            "percentages above 100.00 are rejected",
        );
        assert_eq!(
            PercentParseError::Overflow,
            Percent::parse_exact("42949672960").unwrap_err(),
            "unrepresentable decimal literals report overflow",
        );
        assert!(
            Percent::from_hundredths(10_001).is_none(),
            "hundredths above 100.00 should not construct a percent",
        );
        assert_eq!(
            PercentParseError::OutOfRange,
            Percent::from_counts(2, 1).unwrap_err(),
            "count-derived ratios above 100.00 stay out of range",
        );
    }

    /// Summary filename normalization accepts only canonical production
    /// coverage keys and fails closed on repository escapes.
    #[test]
    fn summary_paths_normalize_canonically_and_filter_nonproduction()
    {
        let relative = ProductionFile::from_summary_filename(
            std::path::Path::new("."),
            "./crates/demo/src\\lib.rs",
        )
        .unwrap()
        .unwrap();
        assert_eq!("crates/demo/src/lib.rs", relative.as_str().into().0);

        let absolute_text = crate::support::HOST_FILESYSTEM
            .current_dir()
            .unwrap()
            .join("crates/demo/src/lib.rs")
            .to_string_lossy()
            .into_owned();
        let absolute =
            ProductionFile::from_summary_filename(std::path::Path::new("."), &absolute_text)
                .unwrap()
                .unwrap();
        assert_eq!("crates/demo/src/lib.rs", absolute.as_str().into().0);

        assert!(
            ProductionFile::from_summary_filename(
                std::path::Path::new("."),
                "crates/demo/tests/helper.rs",
            )
            .unwrap()
            .is_none(),
            "crate-root tests should be filtered out of production coverage",
        );
        assert!(
            ProductionFile::from_summary_filename(std::path::Path::new("."), "Cargo.toml")
                .unwrap()
                .is_none(),
            "non-Rust paths should be filtered out of production coverage",
        );
        assert_error_contains(
            ProductionFile::from_summary_filename(
                std::path::Path::new("."),
                "../crates/demo/src/lib.rs",
            ),
            "coverage filename escapes repository root",
        );
        assert_error_contains(
            ProductionFile::from_summary_filename(
                std::path::Path::new("/"),
                "/../crates/demo/src/lib.rs",
            ),
            "coverage filename escapes repository root",
        );
        assert_error_contains(
            ProductionFile::from_summary_filename(
                std::path::Path::new("../../../../../../../../../../../../.."),
                "/tmp/outside.rs",
            ),
            "coverage filename escapes repository root",
        );
        assert!(
            ProductionFile::from_floor_key("demo/src/lib.rs").is_err(),
            "floor keys outside crates/ are rejected",
        );
        assert!(
            ProductionFile::from_floor_key("crates/demo").is_err(),
            "floor keys without Rust source files are rejected",
        );
        assert!(
            ProductionFile::from_floor_key("crates\\demo\\src\\lib.rs").is_err(),
            "floor keys must already use slash separators",
        );
        assert_error_contains(
            super::normalize_absolute_slash("relative/path.rs"),
            "coverage filename is outside repository root",
        );
        let normalized =
            super::normalize_components(std::path::Path::new("/tmp/./coverage/")).unwrap();
        assert!(
            normalized.ends_with("/tmp/coverage"),
            "component normalization should remove current-dir and trailing separators",
        );
    }

    /// Assert that a typed model constructor fails with a stable diagnostic
    /// fragment.
    fn assert_error_contains<'semantic, T>(
        result: Result<T, crate::GateError>,
        expected: impl Into<super::ExpectedText<'semantic>>,
    ) where
        T: core::fmt::Debug,
    {
        let expected = expected.into().0;
        let error = result.unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains(expected),
            "error {text} should contain {expected}",
        );
    }
}
