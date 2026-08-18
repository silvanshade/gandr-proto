//! Source-policy analyzers for Rust soundness oracles.
//!
//! This module is intentionally side-effect-thin: filesystem-facing entry
//! points enumerate and read sources, while pure analyzers consume already
//! captured source facts. The soundness-oracle policy replaces the old line
//! scanner with `syn` items and doc attributes.

extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use std::path::Path;

use crate::Finding;
use crate::GateError;
use crate::GateResult;
use crate::support;

crate::semantic_str!(pub struct SourceText);
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
crate::semantic_copy!(pub struct TestAttributeFlag(bool));
crate::semantic_copy!(pub struct AsciiCaseInsensitiveFlag(bool));
crate::semantic_copy!(pub struct AsciiBytesEqIgnoreCaseFlag(bool));

/// Default Rust conformance source governed by the soundness-oracle policy.
pub const DEFAULT_SOUNDNESS_ORACLE_FILE: &str = "crates/core-checker-tools/tests/conformance.rs";

/// Exact doc tag that marks a free-generator soundness oracle.
const WITNESS_TAG: &str = "SOUNDNESS-ORACLE-WITNESS:";

/// Exact doc tag that marks a biased soundness-oracle companion.
const COMPANION_TAG: &str = "SOUNDNESS-ORACLE-COMPANION";

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
pub fn analyze_soundness_source<'semantic, Source>(
    path: &Path,
    source: Source,
) -> GateResult
where
    Source: Into<SourceText<'semantic>>,
{
    let source = source.into().0;
    let parsed = syn::parse_file(source).map_err(|error| GateError::RustParse {
        path: path.to_path_buf(),
        source: error,
    })?;
    let path_text = display_path(path);
    return Ok(analyze_soundness_file(&path_text, &parsed));
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
fn collect_functions(syntax: &syn::File) -> Vec<FunctionRecord<'_>>
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
            | syn::Item::Fn(ref item_fn) => {
                functions.push(FunctionRecord {
                    name: item_fn.sig.ident.to_string(),
                    attrs: item_fn.attrs.as_slice(),
                });
            },
            | syn::Item::Mod(ref item_mod) => {
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
fn has_test_attribute(attrs: &[syn::Attribute]) -> impl Into<TestAttributeFlag>
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
fn oracle_tags(attrs: &[syn::Attribute]) -> OracleTags
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
fn doc_comment(attribute: &syn::Attribute) -> Option<String>
{
    if !attribute.path().is_ident("doc") {
        return None;
    }
    let syn::Meta::NameValue(ref name_value) = attribute.meta
    else {
        return None;
    };
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(ref literal),
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
fn parse_witness_names<'semantic, Payload>(payload: Payload) -> Vec<String>
where
    Payload: Into<PayloadText<'semantic>>,
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
fn append_unregistered_oracle_findings<'semantic, Path>(
    path: Path,
    oracles: &[OracleRecord],
    findings: &mut Vec<Finding>,
) where
    Path: Into<PathText<'semantic>>,
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
fn append_no_witness_findings<'semantic, Path>(
    path: Path,
    oracles: &[OracleRecord],
    findings: &mut Vec<Finding>,
) where
    Path: Into<PathText<'semantic>>,
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
fn soundness_finding<'semantic, Kind, Path, Oracle>(
    kind: Kind,
    path: Path,
    oracle: Oracle,
    detail: String,
) -> Finding
where
    Kind: Into<KindText<'semantic>>,
    Path: Into<PathText<'semantic>>,
    Oracle: Into<OracleText<'semantic>>,
{
    let path = path.into().0;
    let oracle = oracle.into().0;
    let kind = kind.into().0;
    return Finding::new(kind, "", path, format!("fn {oracle}"), detail);
}

/// Build a soundness finding for one witness edge.
fn soundness_witness_finding<'semantic, Kind, Path, Oracle, Witness>(
    kind: Kind,
    path: Path,
    oracle: Oracle,
    witness: Witness,
    detail: String,
) -> Finding
where
    Kind: Into<KindText<'semantic>>,
    Path: Into<PathText<'semantic>>,
    Oracle: Into<OracleText<'semantic>>,
    Witness: Into<WitnessText<'semantic>>,
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
fn contains_ascii_case_insensitive<'semantic, Haystack, Needle>(
    haystack: Haystack,
    needle: Needle,
) -> impl Into<AsciiCaseInsensitiveFlag>
where
    Haystack: Into<HaystackText<'semantic>>,
    Needle: Into<NeedleText<'semantic>>,
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
fn append_bad_witness_findings<'semantic, Path>(
    path: Path,
    oracles: &[OracleRecord],
    all_functions: &BTreeSet<String>,
    companions: &BTreeSet<String>,
    findings: &mut Vec<Finding>,
) where
    Path: Into<PathText<'semantic>>,
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
fn ascii_bytes_eq_ignore_case<'semantic, Left, Right>(
    left: Left,
    right: Right,
) -> impl Into<AsciiBytesEqIgnoreCaseFlag>
where
    Left: Into<LeftBytes<'semantic>>,
    Right: Into<RightBytes<'semantic>>,
{
    let right = right.into().0;
    let left = left.into().0;
    return left
        .iter()
        .zip(right.iter())
        .all(|(left_byte, right_byte)| left_byte.eq_ignore_ascii_case(right_byte));
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
pub fn analyze_soundness_file<'semantic, Path>(
    path: Path,
    syntax: &syn::File,
) -> Vec<Finding>
where
    Path: Into<PathText<'semantic>>,
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
    attrs: &'syntax [syn::Attribute],
}

/// Explicit traversal frame for source-order `syn::Item` walking.
struct ItemFrame<'syntax>
{
    /// Items in the current inline module frame.
    items: &'syntax [syn::Item],
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
    fn analyze_soundness_fixture<'semantic, Source>(source: Source) -> GateResult
    where
        Source: Into<SourceText<'semantic>>,
    {
        let source = source.into().0;
        return analyze_soundness_source(
            Path::new("crates/core-checker-tools/tests/conformance.rs"),
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
