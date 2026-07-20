//! Source-policy analyzers for Agda flags and Rust soundness oracles.
//!
//! This module is intentionally side-effect-thin: filesystem-facing entry
//! points enumerate and read sources, while pure analyzers consume already
//! captured source facts. The Agda half preserves the per-flag OPTIONS policy
//! shape from `scripts/check-options-policy.nu`; the Rust half replaces the
//! soundness-oracle line scanner with `syn` items and doc attributes.

extern crate alloc;

use alloc::borrow::Cow;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

use syn::Attribute;
use syn::Expr;
use syn::ExprLit;
use syn::File;
use syn::Item;
use syn::Lit;
use syn::Meta;

use crate::Finding;
use crate::GateError;
use crate::GateResult;
use crate::support;

crate::semantic_str!(pub struct SourceText);
crate::semantic_str!(pub struct FlagText);
crate::semantic_str!(pub struct RootText);
crate::semantic_str!(pub struct PathText);
crate::semantic_str!(pub struct KindText);
crate::semantic_str!(pub struct OracleText);
crate::semantic_str!(pub struct WitnessText);
crate::semantic_str!(pub struct HaystackText);
crate::semantic_str!(pub struct NeedleText);
crate::semantic_bytes!(pub struct LeftBytes);
crate::semantic_bytes!(pub struct RightBytes);
crate::semantic_copy!(pub struct NextCount(usize));
crate::semantic_str!(pub struct PayloadText);
crate::semantic_optional_str!(pub struct OptionalSourceText);
crate::semantic_copy!(pub struct AgdaOptionsContainsFlagFlag(bool));
crate::semantic_copy!(pub struct OptionsModuleSatisfiesPolicyFlag(bool));
crate::semantic_copy!(pub struct TestAttributeFlag(bool));
crate::semantic_copy!(pub struct AsciiCaseInsensitiveFlag(bool));
crate::semantic_copy!(pub struct AsciiBytesEqIgnoreCaseFlag(bool));

/// Default Agda source roots governed by the OPTIONS policy.
pub const DEFAULT_OPTIONS_ROOTS: [&str; 1] = ["metatheory/src"];

/// Default Rust conformance source governed by the soundness-oracle policy.
pub const DEFAULT_SOUNDNESS_ORACLE_FILE: &str = "crates/core-checker/src/conformance.rs";

/// Empty exemption table shared by default Agda policy rows.
const NO_OPTIONS_EXEMPTIONS: &[&str] = &[];

/// Default mandated Agda OPTIONS policy rows.
pub const DEFAULT_OPTIONS_POLICIES: [OptionsPolicy<'static>; 3] = [
    OptionsPolicy {
        flag: "--safe",
        exempt: NO_OPTIONS_EXEMPTIONS,
    },
    OptionsPolicy {
        flag: "--without-K",
        exempt: NO_OPTIONS_EXEMPTIONS,
    },
    OptionsPolicy {
        flag: "--hidden-argument-puns",
        exempt: NO_OPTIONS_EXEMPTIONS,
    },
];

/// Exact doc tag that marks a free-generator soundness oracle.
const WITNESS_TAG: &str = "SOUNDNESS-ORACLE-WITNESS:";

/// Exact doc tag that marks a biased soundness-oracle companion.
const COMPANION_TAG: &str = "SOUNDNESS-ORACLE-COMPANION";

/// One Agda OPTIONS policy row with per-flag exemptions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct OptionsPolicy<'policy>
{
    /// Mandated Agda flag, for example `--safe`.
    pub flag: &'policy str,
    /// Governed-root-parent-relative module paths exempt from this flag.
    pub exempt: &'policy [&'policy str],
}

/// One Agda module discovered under a governed root.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct OptionsModule<'source>
{
    /// Path relative to the governed root's parent, matching exemption entries.
    pub relative_path: Cow<'source, str>,
    /// UTF-8 source text, or [`None`] when the file could not be read.
    pub source: Option<Cow<'source, str>>,
}

/// One governed Agda root and the modules discovered below it.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct OptionsRoot<'source>
{
    /// Root label used in vacuity findings.
    pub root: Cow<'source, str>,
    /// Sorted Agda modules discovered under this root.
    pub modules: Vec<OptionsModule<'source>>,
}

/// Run the default Agda OPTIONS policy against the workspace metatheory root.
///
/// # Contract
/// - requires: `workspace_root` identifies the workspace checkout root.
/// - ensures: returns one semantic finding per vacuous governed root and per
///   non-exempt module missing each mandated flag.
/// - provides: file-backed replacement for `check-options-policy.nu` using the
///   default roots and policy rows.
/// - fails: returns gate errors from governed-root enumeration failures other
///   than a missing root; unreadable modules are represented as semantic
///   findings by scanning them as empty OPTIONS text.
/// - panics: none.
/// - intension: roots are visited in default policy order and module paths are
///   sorted and deduplicated before analysis.
///
/// # Errors
/// Returns I/O or operational gate errors from support-file enumeration when a
/// root cannot be inspected for a reason other than absence.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — default root selection is a thin projection
///   over [`run_options_policy_with`], whose vacuity, exemption, and
///   missing-flag residues are killed by the pure analyzer witnesses.
/// - witness: `source_policy::tests::options_truth_table_property`
/// - witness: `source_policy::tests::options_vacuous_roots_fail_per_flag`
#[inline]
pub fn run_options_policy(workspace_root: &Path) -> GateResult
{
    let roots = DEFAULT_OPTIONS_ROOTS
        .iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>();
    return run_options_policy_with(workspace_root, &roots, &DEFAULT_OPTIONS_POLICIES);
}

/// Run an Agda OPTIONS policy against caller-supplied governed roots.
///
/// # Contract
/// - requires: `workspace_root` identifies the checkout root used to resolve
///   relative roots and render stable paths.
/// - requires: each `roots` entry is either absolute or relative to
///   `workspace_root`.
/// - ensures: returns the same findings as [`analyze_options_policy`] after
///   sorted, deduplicated `.agda` enumeration and fail-closed source reads.
/// - provides: a file-backed analyzer surface for custom policy tests and CLI
///   integration.
/// - fails: returns gate errors from support-file enumeration failures other
///   than missing roots.
/// - panics: none.
/// - intension: root order follows `roots`, policy order follows `policies`,
///   and module order is lexicographic by full discovered path.
///
/// # Errors
/// Returns I/O or operational gate errors from support-file enumeration when a
/// root cannot be inspected for a reason other than absence.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — missing-root vacuity, file sort order,
///   fail-closed read projection, and per-flag exemptions are separated by pure
///   root/module fixtures and exact finding fields.
/// - witness: `source_policy::tests::options_truth_table_property`
/// - witness: `source_policy::tests::options_exemptions_are_per_flag`
/// - witness: `source_policy::tests::options_vacuous_roots_fail_per_flag`
#[inline]
pub fn run_options_policy_with(
    workspace_root: &Path,
    roots: &[PathBuf],
    policies: &[OptionsPolicy<'_>],
) -> GateResult
{
    let mut loaded_roots = Vec::new();
    for root in roots {
        loaded_roots.push(load_options_root(workspace_root, root)?);
    }
    return Ok(analyze_options_policy(&loaded_roots, policies));
}

/// Run the default soundness-oracle companion policy in a workspace.
///
/// # Contract
/// - requires: `workspace_root` identifies the workspace checkout root.
/// - ensures: parses and analyzes [`DEFAULT_SOUNDNESS_ORACLE_FILE`].
/// - provides: file-backed replacement for the default invocation of
///   `check-soundness-oracles.nu`.
/// - fails: returns I/O errors for unreadable files and Rust parse errors for
///   invalid Rust source.
/// - panics: none.
///
/// # Errors
/// Returns support-file read errors and `syn` parse errors.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — this function is a path projection over
///   [`run_soundness_oracles`], whose parse and source-analysis behavior is
///   witnessed by exact source fixtures.
/// - witness: `source_policy::tests::soundness_cfg_and_ignore_attributes_do_not_hide_test_marker`
/// - witness: `source_policy::tests::soundness_missing_witnesses_are_reported_deterministically`
#[inline]
pub fn run_default_soundness_oracles(workspace_root: &Path) -> GateResult
{
    return run_soundness_oracles(&workspace_root.join(DEFAULT_SOUNDNESS_ORACLE_FILE));
}

/// Run the soundness-oracle companion policy against one Rust source file.
///
/// # Contract
/// - requires: `path` points at the Rust conformance source to inspect.
/// - ensures: returns semantic findings for unregistered coherence tests,
///   free-generator oracles without witnesses, missing witnesses, and witnesses
///   that are not tagged companions.
/// - provides: file-backed soundness-oracle validation.
/// - fails: returns I/O errors for unreadable files and Rust parse errors for
///   invalid Rust source.
/// - panics: none.
///
/// # Errors
/// Returns support-file read errors and `syn` parse errors.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — read/parse failures are separated from semantic
///   findings by `GateError` variants, while semantic residues are killed by
///   the pure analyzer witnesses.
/// - witness: `source_policy::tests::soundness_cfg_and_ignore_attributes_do_not_hide_test_marker`
/// - witness: `source_policy::tests::soundness_missing_witnesses_are_reported_deterministically`
#[inline]
pub fn run_soundness_oracles(path: &Path) -> GateResult
{
    let source = support::read_utf8(path)?;
    return analyze_soundness_source(path, &source);
}

/// Parse and analyze one Rust source string for soundness-oracle policy drift.
///
/// # Contract
/// - requires: `source` is intended to be a complete Rust source file.
/// - ensures: returns [`analyze_soundness_file`] findings when `source` parses.
/// - provides: source-backed, filesystem-free validation for fixtures and
///   integration tests.
/// - fails: returns a Rust parse error when `source` is not a complete Rust
///   file.
/// - panics: none.
///
/// # Errors
/// Returns a Rust parse error when `source` is not a complete Rust file.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — malformed Rust is the only fallible branch, and
///   all successful semantic branches are delegated to exact AST fixtures.
/// - witness: `source_policy::tests::soundness_cfg_and_ignore_attributes_do_not_hide_test_marker`
/// - witness: `source_policy::tests::soundness_tag_precedence_makes_witness_tag_free`
#[inline]
pub fn analyze_soundness_source<'semantic>(
    path: &Path,
    source: impl Into<SourceText<'semantic>>,
) -> GateResult
{
    let source = source.into().0;
    let parsed = syn::parse_file(source).map_err(|error| GateError::RustParse {
        path: path.to_path_buf(),
        source: error,
    })?;
    let path_text = display_path(path);
    return Ok(analyze_soundness_file(&path_text, &parsed));
}

/// Load one governed Agda root through the support filesystem API.
///
/// # Contract
/// - requires: `root` is absolute or relative to `workspace_root`.
/// - ensures: returns sorted, deduplicated `.agda` modules with sources loaded
///   as UTF-8 when possible.
/// - ensures: missing roots load as empty, preserving vacuity-as-finding.
/// - provides: support-backed input projection for [`analyze_options_policy`].
/// - fails: returns enumeration failures other than missing roots.
/// - panics: none.
///
/// # Errors
/// Returns support-file enumeration errors other than root absence.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the semantic consequences of empty roots,
///   unreadable sources, and sorted modules are witnessed by pure analyzer
///   fixtures; support I/O itself is covered by integration gates.
/// - witness: `source_policy::tests::options_truth_table_property`
/// - witness: `source_policy::tests::options_vacuous_roots_fail_per_flag`
fn load_options_root(
    workspace_root: &Path,
    root: &Path,
) -> Result<OptionsRoot<'static>, GateError>
{
    let root_path = resolve_root(workspace_root, root);
    let root_label = relative_path(workspace_root, &root_path);
    let base = root_path.parent().unwrap_or(root_path.as_path());
    let mut files = agda_files(&root_path)?;
    files.sort();
    files.dedup();

    let mut modules = Vec::new();
    for file in files {
        let relative = relative_path(base, &file);
        let source = match support::read_utf8(&file) {
            | Ok(source) => Some(Cow::Owned(source)),
            | Err(_error) => None,
        };
        modules.push(OptionsModule {
            relative_path: Cow::Owned(relative),
            source,
        });
    }

    return Ok(OptionsRoot {
        root: Cow::Owned(root_label),
        modules,
    });
}

/// Analyze already loaded Agda modules against OPTIONS policy rows.
///
/// # Contract
/// - requires: `roots` contains modules sorted by the caller when intensional
///   path order matters.
/// - requires: module paths use the same relative convention as policy
///   exemptions.
/// - ensures: emits vacuity findings for each empty root and missing-flag
///   findings for each non-exempt module lacking the exact flag token.
/// - provides: pure OPTIONS policy validation over source facts.
/// - panics: none.
/// - intension: for each policy row, all vacuity findings precede all module
///   findings, matching the Nushell gate's observable order.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — the Cartesian cases of present flag, absent
///   flag, unreadable source, matching exemption, and nonmatching exemption
///   kill the policy truth table; multi-flag fixtures kill exemption isolation.
/// - witness: `source_policy::tests::options_truth_table_property`
/// - witness: `source_policy::tests::options_exemptions_are_per_flag`
/// - witness: `source_policy::tests::options_vacuous_roots_fail_per_flag`
#[inline]
#[must_use]
pub fn analyze_options_policy(
    roots: &[OptionsRoot<'_>],
    policies: &[OptionsPolicy<'_>],
) -> Vec<Finding>
{
    let mut findings = Vec::new();
    for policy in policies {
        for root in roots {
            if root.modules.is_empty() {
                findings.push(options_vacuity_finding(root.root.as_ref(), policy.flag));
            }
        }
        for root in roots {
            for module in &root.modules {
                if !options_module_satisfies_policy(module, policy).into().0 {
                    findings.push(options_missing_finding(
                        module.relative_path.as_ref(),
                        policy.flag,
                    ));
                }
            }
        }
    }
    return findings;
}

/// Resolve a governed root against the workspace root.
fn resolve_root(
    workspace_root: &Path,
    root: &Path,
) -> PathBuf
{
    if root.is_absolute() {
        return root.to_path_buf();
    }
    return workspace_root.join(root);
}

/// Return whether an Agda source contains an exact flag token on an OPTIONS
/// line.
///
/// # Contract
/// - requires: `flag` is the exact mandated Agda option token.
/// - ensures: ignores non-OPTIONS lines.
/// - ensures: returns `true` only for a whitespace-delimited token equal to
///   `flag`.
/// - provides: token-backed OPTIONS matching without line-regex substring false
///   positives.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — ordinary present, absent, and substring-only
///   sources distinguish exact token matching from the old regex heuristic.
/// - witness: `source_policy::tests::options_truth_table_property`
fn agda_options_contains_flag<'semantic>(
    source: impl Into<SourceText<'semantic>>,
    flag: impl Into<FlagText<'semantic>>,
) -> impl Into<AgdaOptionsContainsFlagFlag>
{
    let flag = flag.into().0;
    let source = source.into().0;
    return source
        .lines()
        .filter(|line| line.contains("OPTIONS"))
        .flat_map(str::split_whitespace)
        .any(|token| token == flag);
}

/// Build a vacuity finding for one policy/root pair.
fn options_vacuity_finding<'semantic>(
    root: impl Into<RootText<'semantic>>,
    flag: impl Into<FlagText<'semantic>>,
) -> Finding
{
    let flag = flag.into().0;
    let root = root.into().0;
    return Finding::new(
        "agda-options-vacuous",
        "",
        root,
        flag,
        format!("no .agda modules found under {root}/"),
    );
}

/// Return whether a module satisfies one OPTIONS policy row.
///
/// # Contract
/// - requires: `module.relative_path` and `policy.exempt` use the same path
///   convention.
/// - ensures: returns `true` when the module is exempt for this exact policy
///   row, regardless of source readability.
/// - ensures: otherwise returns `true` only when readable source contains the
///   exact policy flag token on an OPTIONS line.
/// - provides: the per-flag truth predicate for OPTIONS policy findings.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — readable-present, readable-absent, unreadable,
///   exempt-readable, and exempt-unreadable cases kill every branch.
/// - witness: `source_policy::tests::options_truth_table_property`
/// - witness: `source_policy::tests::options_exemptions_are_per_flag`
fn options_module_satisfies_policy(
    module: &OptionsModule<'_>,
    policy: &OptionsPolicy<'_>,
) -> impl Into<OptionsModuleSatisfiesPolicyFlag>
{
    if policy
        .exempt
        .iter()
        .any(|&exempt| exempt == module.relative_path.as_ref())
    {
        return true;
    }
    return module
        .source
        .as_deref()
        .is_some_and(|source| agda_options_contains_flag(source, policy.flag).into().0);
}

/// Build a missing-flag finding for one policy/module pair.
fn options_missing_finding<'semantic>(
    path: impl Into<PathText<'semantic>>,
    flag: impl Into<FlagText<'semantic>>,
) -> Finding
{
    let flag = flag.into().0;
    let path = path.into().0;
    return Finding::new(
        "agda-options-missing",
        "",
        path,
        flag,
        format!("OPTIONS lacks {flag} and is not exempt"),
    );
}

/// Collect all free functions from a parsed Rust file in source order.
///
/// # Contract
/// - requires: `syntax` is a parsed Rust file.
/// - ensures: returns top-level and inline-module free functions in source
///   order.
/// - provides: nonrecursive function discovery for oracle analysis.
/// - panics: none.
/// - intension: traversal uses an explicit stack of item slices rather than
///   input-scaled recursion.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — inline-module traversal and sibling ordering
///   are killed by oracle fixtures that depend on deterministic finding order.
/// - witness: `source_policy::tests::soundness_missing_witnesses_are_reported_deterministically`
fn collect_functions(syntax: &File) -> Vec<FunctionRecord<'_>>
{
    let mut functions = Vec::new();
    let mut stack = vec![ItemFrame {
        items: syntax.items.as_slice(),
        next: 0_usize,
    }];

    while let Some(mut frame) = stack.pop() {
        let items = frame.items;
        let next = frame.next;
        let Some(item) = items.get(next)
        else {
            continue;
        };
        frame.next = next.saturating_add(1);
        stack.push(frame);

        match *item {
            | Item::Fn(ref item_fn) => {
                functions.push(FunctionRecord {
                    name: item_fn.sig.ident.to_string(),
                    attrs: item_fn.attrs.as_slice(),
                });
            },
            | Item::Mod(ref item_mod) => {
                if let Some(content) = item_mod.content.as_ref() {
                    stack.push(ItemFrame {
                        items: content.1.as_slice(),
                        next: 0_usize,
                    });
                }
            },
            | _ => {},
        }
    }

    return functions;
}

/// Return whether a function has a direct `#[test]` attribute.
fn has_test_attribute(attrs: &[Attribute]) -> impl Into<TestAttributeFlag>
{
    return attrs
        .iter()
        .any(|attribute| attribute.path().is_ident("test"));
}

/// Classify exact soundness-oracle tags from parsed doc attributes.
///
/// # Contract
/// - requires: `attrs` are the attributes attached to one Rust function.
/// - ensures: returns [`OracleRole::Free`] when any doc item starts with the
///   exact witness tag, even when a companion tag is also present.
/// - ensures: returns [`OracleRole::Companion`] only when no witness tag is
///   present and a doc item is exactly the companion tag.
/// - provides: exact doc-tag classification with witness precedence.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — prefix text, companion text with extra suffix,
///   and both-tags cases separate exact recognition from substring heuristics.
/// - witness: `source_policy::tests::soundness_tags_must_be_exact_doc_items`
/// - witness: `source_policy::tests::soundness_tag_precedence_makes_witness_tag_free`
fn oracle_tags(attrs: &[Attribute]) -> OracleTags
{
    let mut witness_payload = None;
    let mut has_companion = false;

    for attribute in attrs {
        let Some(line) = doc_comment(attribute)
        else {
            continue;
        };
        let trimmed = line.trim();
        if witness_payload.is_none()
            && let Some(payload) = trimmed.strip_prefix(WITNESS_TAG)
        {
            witness_payload = Some(String::from(payload));
        }
        if trimmed == COMPANION_TAG {
            has_companion = true;
        }
    }

    if let Some(payload) = witness_payload {
        return OracleTags {
            role: OracleRole::Free,
            witnesses: parse_witness_names(&payload),
        };
    }
    if has_companion {
        return OracleTags {
            role: OracleRole::Companion,
            witnesses: Vec::new(),
        };
    }
    return OracleTags {
        role: OracleRole::Untagged,
        witnesses: Vec::new(),
    };
}

/// Extract a doc-comment payload from one `syn` attribute.
///
/// # Contract
/// - requires: `attribute` is any `syn` attribute.
/// - ensures: returns the decoded doc payload only for `#[doc = \"...\"]`
///   attributes.
/// - provides: a syntax-backed replacement for line-level `///` scraping.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — recognized doc comments and ordinary non-doc
///   attributes are distinguished by the soundness tag fixtures.
/// - witness: `source_policy::tests::soundness_tags_must_be_exact_doc_items`
fn doc_comment(attribute: &Attribute) -> Option<String>
{
    if !attribute.path().is_ident("doc") {
        return None;
    }
    let Meta::NameValue(ref name_value) = attribute.meta
    else {
        return None;
    };
    let Expr::Lit(ExprLit {
        lit: Lit::Str(ref literal),
        ..
    }) = name_value.value
    else {
        return None;
    };
    return Some(literal.value());
}

/// Parse comma-separated witness names from a witness-tag payload.
///
/// # Contract
/// - requires: `payload` is the text after the first exact witness tag.
/// - ensures: returns trimmed, nonempty comma-separated witness names in source
///   order.
/// - provides: witness-list normalization for free oracle validation.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — multi-witness comma order and empty payload are
///   separated by missing-witness and tag-precedence fixtures.
/// - witness: `source_policy::tests::soundness_missing_witnesses_are_reported_deterministically`
/// - witness: `source_policy::tests::soundness_tag_precedence_makes_witness_tag_free`
fn parse_witness_names<'semantic>(payload: impl Into<PayloadText<'semantic>>) -> Vec<String>
{
    let payload = payload.into().0;
    return payload
        .split(',')
        .map(str::trim)
        .filter(|witness| !witness.is_empty())
        .map(String::from)
        .collect();
}

/// Append unregistered coherence-test findings in oracle order.
///
/// # Contract
/// - requires: `oracles` are in source order.
/// - ensures: appends one finding for each untagged oracle and no findings for
///   tagged oracles.
/// - provides: the first soundness diagnostic phase.
/// - panics: none.
/// - intension: preserves the incoming oracle order.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — exact-tag and substring-tag fixtures
///   distinguish unregistered oracles from tagged ones.
/// - witness: `source_policy::tests::soundness_tags_must_be_exact_doc_items`
fn append_unregistered_oracle_findings<'semantic>(
    path: impl Into<PathText<'semantic>>,
    oracles: &[OracleRecord],
    findings: &mut Vec<Finding>,
)
{
    let path = path.into().0;
    for oracle in oracles
        .iter()
        .filter(|oracle| oracle.role == OracleRole::Untagged)
    {
        findings.push(soundness_finding(
            "soundness-oracle-unregistered",
            path,
            &oracle.name,
            format!(
                "UNREGISTERED: fn {} is a `*coherence*` proptest with neither a SOUNDNESS-ORACLE-WITNESS nor a SOUNDNESS-ORACLE-COMPANION tag",
                oracle.name
            ),
        ));
    }
}

/// Append no-witness findings in oracle order.
///
/// # Contract
/// - requires: `oracles` are in source order.
/// - ensures: appends one finding for each free oracle with an empty witness
///   list.
/// - provides: the second soundness diagnostic phase.
/// - panics: none.
/// - intension: preserves the incoming oracle order.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — a witness tag with empty payload is separated
///   from companion classification by the both-tags fixture.
/// - witness: `source_policy::tests::soundness_tag_precedence_makes_witness_tag_free`
fn append_no_witness_findings<'semantic>(
    path: impl Into<PathText<'semantic>>,
    oracles: &[OracleRecord],
    findings: &mut Vec<Finding>,
)
{
    let path = path.into().0;
    for oracle in oracles
        .iter()
        .filter(|oracle| oracle.role == OracleRole::Free && oracle.witnesses.is_empty())
    {
        findings.push(soundness_finding(
            "soundness-oracle-no-witness",
            path,
            &oracle.name,
            format!(
                "NO WITNESS: free oracle fn {} declares SOUNDNESS-ORACLE-WITNESS but lists no companion",
                oracle.name
            ),
        ));
    }
}

/// Build a soundness finding for an oracle function.
fn soundness_finding<'semantic>(
    kind: impl Into<KindText<'semantic>>,
    path: impl Into<PathText<'semantic>>,
    oracle: impl Into<OracleText<'semantic>>,
    detail: String,
) -> Finding
{
    let path = path.into().0;
    let oracle = oracle.into().0;
    let kind = kind.into().0;
    return Finding::new(kind, "", path, format!("fn {oracle}"), detail);
}

/// Build a soundness finding for one witness edge.
fn soundness_witness_finding<'semantic>(
    kind: impl Into<KindText<'semantic>>,
    path: impl Into<PathText<'semantic>>,
    oracle: impl Into<OracleText<'semantic>>,
    witness: impl Into<WitnessText<'semantic>>,
    detail: String,
) -> Finding
{
    let path = path.into().0;
    let oracle = oracle.into().0;
    let witness = witness.into().0;
    let kind = kind.into().0;
    return Finding::new(kind, "", path, format!("fn {oracle} -> {witness}"), detail);
}

/// Return whether `haystack` contains `needle`, ignoring ASCII case.
///
/// # Contract
/// - requires: none.
/// - ensures: returns `true` when `needle` is empty or occurs in `haystack`
///   after ASCII-only case folding.
/// - provides: allocation-free coherence-name detection.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — lowercase fixture names and ordinary
///   nonmatching helpers cover the policy-level projection that only
///   `coherence` tests are analyzed.
/// - witness: `source_policy::tests::soundness_cfg_and_ignore_attributes_do_not_hide_test_marker`
fn contains_ascii_case_insensitive<'semantic>(
    haystack: impl Into<HaystackText<'semantic>>,
    needle: impl Into<NeedleText<'semantic>>,
) -> impl Into<AsciiCaseInsensitiveFlag>
{
    let needle = needle.into().0;
    let haystack = haystack.into().0;
    if needle.is_empty() {
        return true;
    }
    let needle_bytes = needle.as_bytes();
    return haystack
        .as_bytes()
        .windows(needle_bytes.len())
        .any(|window| ascii_bytes_eq_ignore_case(window, needle_bytes).into().0);
}

/// Append missing and untagged witness findings in oracle/witness order.
///
/// # Contract
/// - requires: `all_functions` contains every function name in the parsed file.
/// - requires: `companions` contains exactly the names classified as companion
///   coherence tests.
/// - ensures: appends missing-companion findings before untagged-companion
///   findings according to each free oracle's witness-list order.
/// - provides: the final soundness diagnostic phase.
/// - panics: none.
/// - intension: preserves source oracle order and per-oracle witness order.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — absent witness and present-but-uncompanion
///   witness names are separated by exact finding kinds and order.
/// - witness: `source_policy::tests::soundness_missing_witnesses_are_reported_deterministically`
fn append_bad_witness_findings<'semantic>(
    path: impl Into<PathText<'semantic>>,
    oracles: &[OracleRecord],
    all_functions: &BTreeSet<String>,
    companions: &BTreeSet<String>,
    findings: &mut Vec<Finding>,
)
{
    let path = path.into().0;
    for oracle in oracles
        .iter()
        .filter(|oracle| oracle.role == OracleRole::Free)
    {
        for witness in &oracle.witnesses {
            if !all_functions.contains(witness) {
                findings.push(soundness_witness_finding(
                    "soundness-oracle-missing-companion",
                    path,
                    &oracle.name,
                    witness,
                    format!(
                        "MISSING COMPANION: oracle fn {} names witness `{witness}`, which is not a fn in the file",
                        oracle.name
                    ),
                ));
            }
            else if !companions.contains(witness) {
                findings.push(soundness_witness_finding(
                    "soundness-oracle-untagged-companion",
                    path,
                    &oracle.name,
                    witness,
                    format!(
                        "UNTAGGED COMPANION: oracle fn {} names witness `{witness}`, which exists but lacks SOUNDNESS-ORACLE-COMPANION",
                        oracle.name
                    ),
                ));
            }
        }
    }
}

/// Return whether two equal-length byte slices match under ASCII case folding.
fn ascii_bytes_eq_ignore_case<'semantic>(
    left: impl Into<LeftBytes<'semantic>>,
    right: impl Into<RightBytes<'semantic>>,
) -> impl Into<AsciiBytesEqIgnoreCaseFlag>
{
    let right = right.into().0;
    let left = left.into().0;
    return left
        .iter()
        .zip(right.iter())
        .all(|(left_byte, right_byte)| left_byte.eq_ignore_ascii_case(right_byte));
}

/// Render `path` relative to `base` when possible.
fn relative_path(
    base: &Path,
    path: &Path,
) -> String
{
    match path.strip_prefix(base) {
        | Ok(relative) => return display_path(relative),
        | Err(_error) => return display_path(path),
    }
}

/// Enumerate Agda files below a root, treating absent roots as empty.
///
/// # Contract
/// - requires: `root` is the directory to scan.
/// - ensures: returns support-discovered `.agda` paths.
/// - ensures: returns an empty list when `root` does not exist.
/// - provides: vacuity-preserving root enumeration.
/// - fails: returns support-file enumeration errors other than root absence.
/// - panics: none.
///
/// # Errors
/// Returns support-file enumeration errors other than root absence.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — root absence and ordinary empty roots have the
///   same semantic projection, and non-empty roots are sorted by the caller.
/// - witness: `source_policy::tests::options_vacuous_roots_fail_per_flag`
fn agda_files(root: &Path) -> Result<Vec<PathBuf>, GateError>
{
    match support::walk_files(root, OsStr::new("agda")) {
        | Ok(files) => return Ok(files),
        | Err(GateError::Io { source, .. }) if source.kind() == ErrorKind::NotFound => {
            return Ok(Vec::new());
        },
        | Err(error) => return Err(error),
    }
}

/// Render a path with lossy UTF-8 replacement for diagnostics.
fn display_path(path: &Path) -> String
{
    return path.to_string_lossy().into_owned();
}

/// Analyze an already parsed Rust file for soundness-oracle policy drift.
///
/// # Contract
/// - requires: `path` is the stable diagnostic path for `syntax`.
/// - ensures: considers only `#[test]` functions whose name contains
///   `coherence`, case-insensitively.
/// - ensures: classifies exact doc tags with witness-tag precedence over
///   companion tags.
/// - ensures: validates every declared witness by exact function name and exact
///   companion classification.
/// - provides: pure `syn`-backed soundness-oracle validation.
/// - panics: none.
/// - intension: functions are traversed in source order with an explicit stack;
///   unregistered findings precede no-witness findings, which precede bad
///   witness findings.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — direct `#[test]` visibility through cfg/ignore
///   attributes, exact doc-tag recognition, witness-missing versus
///   witness-untagged distinction, and witness-over-companion precedence are
///   separated by exact finding-kind sequences.
/// - witness: `source_policy::tests::soundness_cfg_and_ignore_attributes_do_not_hide_test_marker`
/// - witness: `source_policy::tests::soundness_missing_witnesses_are_reported_deterministically`
/// - witness: `source_policy::tests::soundness_tags_must_be_exact_doc_items`
/// - witness: `source_policy::tests::soundness_tag_precedence_makes_witness_tag_free`
#[inline]
#[must_use]
pub fn analyze_soundness_file<'semantic>(
    path: impl Into<PathText<'semantic>>,
    syntax: &File,
) -> Vec<Finding>
{
    let path = path.into().0;
    let functions = collect_functions(syntax);
    let mut all_functions = BTreeSet::new();
    let mut oracles = Vec::new();

    for function in &functions {
        all_functions.insert(function.name.clone());
        if contains_ascii_case_insensitive(&function.name, "coherence")
            .into()
            .0
            && has_test_attribute(function.attrs).into().0
        {
            let tags = oracle_tags(function.attrs);
            oracles.push(OracleRecord {
                name: function.name.clone(),
                role: tags.role,
                witnesses: tags.witnesses,
            });
        }
    }

    let companions = oracles
        .iter()
        .filter(|oracle| oracle.role == OracleRole::Companion)
        .map(|oracle| oracle.name.clone())
        .collect::<BTreeSet<_>>();

    let mut findings = Vec::new();
    append_unregistered_oracle_findings(path, &oracles, &mut findings);
    append_no_witness_findings(path, &oracles, &mut findings);
    append_bad_witness_findings(path, &oracles, &all_functions, &companions, &mut findings);
    return findings;
}

/// One discovered Rust function relevant to oracle analysis.
struct FunctionRecord<'syntax>
{
    /// Function name as written in the Rust identifier.
    name: String,
    /// Function attributes, including parsed doc comments.
    attrs: &'syntax [Attribute],
}

/// Explicit traversal frame for source-order `syn::Item` walking.
struct ItemFrame<'syntax>
{
    /// Items in the current inline module frame.
    items: &'syntax [Item],
    /// Next item index to inspect.
    next: usize,
}

/// One classified coherence test and its witness list.
struct OracleRecord
{
    /// Function name of the coherence test.
    name: String,
    /// Exact role inferred from doc tags.
    role: OracleRole,
    /// Witness names declared by a free oracle.
    witnesses: Vec<String>,
}

/// Exact role assigned to a coherence test.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OracleRole
{
    /// Free-generator oracle requiring one or more biased companions.
    Free,
    /// Biased companion oracle.
    Companion,
    /// Coherence test with no recognized doc tag.
    Untagged,
}

/// Classified doc-tag payload for one coherence test.
struct OracleTags
{
    /// Role inferred from exact doc tags.
    role: OracleRole,
    /// Witnesses parsed from the first exact witness tag.
    witnesses: Vec<String>,
}

/// Behavioral witnesses for the source-policy analyzers.
#[cfg(test)]
mod tests
{
    use super::*;

    /// Governed module path used by OPTIONS fixtures.
    const MODULE_PATH: &str = "src/Gandr/Foo.agda";

    /// One OPTIONS truth-table row.
    #[derive(Clone, Copy)]
    struct OptionsCase
    {
        /// Human-readable case name for assertion messages.
        name: &'static str,
        /// Optional source text; [`None`] models an unreadable file.
        source: Option<&'static str>,
        /// Exemptions attached to the tested policy row.
        exemptions: &'static [&'static str],
        /// Whether the module should satisfy the policy.
        want_ok: bool,
    }

    /// Return finding declarations in order.
    fn finding_declarations(findings: &[Finding]) -> Vec<String>
    {
        return findings
            .iter()
            .map(|finding| finding.declaration.clone())
            .collect();
    }

    /// Exhaust the OPTIONS policy truth table for one flag and one module.
    #[test]
    fn options_truth_table_property()
    {
        let cases = [
            OptionsCase {
                name: "present exact flag",
                source: Some("{-# OPTIONS --safe #-}\n"),
                exemptions: &[],
                want_ok: true,
            },
            OptionsCase {
                name: "absent flag",
                source: Some("{-# OPTIONS --without-K #-}\n"),
                exemptions: &[],
                want_ok: false,
            },
            OptionsCase {
                name: "substring is not exact flag",
                source: Some("{-# OPTIONS --safe-ish #-}\n"),
                exemptions: &[],
                want_ok: false,
            },
            OptionsCase {
                name: "unreadable source fails closed",
                source: None,
                exemptions: &[],
                want_ok: false,
            },
            OptionsCase {
                name: "exemption admits absent flag",
                source: Some("module Gandr.Foo where\n"),
                exemptions: &[MODULE_PATH],
                want_ok: true,
            },
            OptionsCase {
                name: "exemption admits unreadable source",
                source: None,
                exemptions: &[MODULE_PATH],
                want_ok: true,
            },
        ];

        for case in cases {
            let policy = OptionsPolicy {
                flag: "--safe",
                exempt: case.exemptions,
            };
            let root = options_root(vec![options_module(case.source)]);
            let findings = analyze_options_policy(&[root], &[policy]);
            assert_eq!(
                findings.is_empty(),
                case.want_ok,
                "OPTIONS truth-table case failed: {}",
                case.name
            );
        }
    }

    /// Prove that one flag's exemption does not exempt other flags.
    #[test]
    fn options_exemptions_are_per_flag()
    {
        let safe = OptionsPolicy {
            flag: "--safe",
            exempt: &[MODULE_PATH],
        };
        let without_k = OptionsPolicy {
            flag: "--without-K",
            exempt: &[],
        };
        let root = options_root(vec![options_module(Some("module Gandr.Foo where\n"))]);

        let findings = analyze_options_policy(&[root], &[safe, without_k]);

        assert_eq!(
            finding_declarations(&findings),
            vec!["--without-K"],
            "only the non-exempt flag should fail"
        );
    }

    /// Prove that empty roots fail once per policy row.
    #[test]
    fn options_vacuous_roots_fail_per_flag()
    {
        let safe = OptionsPolicy {
            flag: "--safe",
            exempt: &[],
        };
        let without_k = OptionsPolicy {
            flag: "--without-K",
            exempt: &[],
        };
        let root = options_root(Vec::new());

        let findings = analyze_options_policy(&[root], &[safe, without_k]);

        assert_eq!(
            finding_kinds(&findings),
            vec!["agda-options-vacuous", "agda-options-vacuous"],
            "each policy row should report root vacuity"
        );
        assert_eq!(
            finding_declarations(&findings),
            vec!["--safe", "--without-K"],
            "vacuity findings should preserve policy order"
        );
    }

    /// Build one borrowed OPTIONS root fixture.
    fn options_root(modules: Vec<OptionsModule<'static>>) -> OptionsRoot<'static>
    {
        return OptionsRoot {
            root: Cow::Borrowed("metatheory/src"),
            modules,
        };
    }

    /// Build one borrowed OPTIONS module fixture.
    fn options_module(source: impl Into<OptionalSourceText<'static>>) -> OptionsModule<'static>
    {
        let source = source.into().0;
        return OptionsModule {
            relative_path: Cow::Borrowed(MODULE_PATH),
            source: source.map(Cow::Borrowed),
        };
    }

    /// Prove cfg and ignore attributes do not hide a direct test marker.
    #[test]
    fn soundness_cfg_and_ignore_attributes_do_not_hide_test_marker() -> Result<(), GateError>
    {
        let source = r#"
            #[cfg(test)]
            #[ignore]
            #[cfg_attr(miri, ignore)]
            #[test]
            /// SOUNDNESS-ORACLE-WITNESS: biased_bind_coherence
            fn free_bind_coherence() {}

            #[test]
            /// SOUNDNESS-ORACLE-COMPANION
            fn biased_bind_coherence() {}

            fn helper_coherence() {}
        "#;

        let findings = analyze_soundness_fixture(source)?;

        assert!(
            findings.is_empty(),
            "direct #[test] must remain visible through cfg/ignore attributes"
        );
        return Ok(());
    }

    /// Analyze one Rust source fixture through the parser-backed API.
    fn analyze_soundness_fixture<'semantic>(source: impl Into<SourceText<'semantic>>)
    -> GateResult
    {
        let source = source.into().0;
        return analyze_soundness_source(
            Path::new("crates/core-checker/src/conformance.rs"),
            source,
        );
    }

    /// Prove missing and untagged witness findings are deterministic.
    #[test]
    fn soundness_missing_witnesses_are_reported_deterministically() -> Result<(), GateError>
    {
        let source = r#"
            #[test]
            /// SOUNDNESS-ORACLE-WITNESS: missing_companion, untagged_companion
            fn free_comp_coherence() {}

            fn untagged_companion() {}
        "#;

        let findings = analyze_soundness_fixture(source)?;

        assert_eq!(
            finding_kinds(&findings),
            vec![
                "soundness-oracle-missing-companion",
                "soundness-oracle-untagged-companion"
            ],
            "witness diagnostics should preserve oracle and witness order"
        );
        return Ok(());
    }

    /// Prove soundness tags are exact doc items, not substrings.
    #[test]
    fn soundness_tags_must_be_exact_doc_items() -> Result<(), GateError>
    {
        let source = r#"
            #[test]
            /// prefix SOUNDNESS-ORACLE-COMPANION
            fn stray_coherence() {}
        "#;

        let findings = analyze_soundness_fixture(source)?;

        assert_eq!(
            finding_kinds(&findings),
            vec!["soundness-oracle-unregistered"],
            "substring companion tags must not classify an oracle"
        );
        return Ok(());
    }

    /// Prove witness tags take precedence over companion tags.
    #[test]
    fn soundness_tag_precedence_makes_witness_tag_free() -> Result<(), GateError>
    {
        let source = r#"
            #[test]
            /// SOUNDNESS-ORACLE-COMPANION
            /// SOUNDNESS-ORACLE-WITNESS:
            fn both_tags_coherence() {}
        "#;

        let findings = analyze_soundness_fixture(source)?;

        assert_eq!(
            finding_kinds(&findings),
            vec!["soundness-oracle-no-witness"],
            "witness tag should classify as free even with a companion tag"
        );
        return Ok(());
    }

    /// Return finding kinds in order.
    fn finding_kinds(findings: &[Finding]) -> Vec<String>
    {
        return findings
            .iter()
            .map(|finding| finding.kind.clone())
            .collect();
    }
}
