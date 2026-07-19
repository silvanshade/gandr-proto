gandr_workflow_gates::semantic_str!(pub(crate) struct ContextText);
gandr_workflow_gates::semantic_str!(pub(crate) struct WitnessText);

impl<'item, 'text> From<&'item &'text str> for WitnessText<'text>
{
    #[inline]
    fn from(value: &'item &'text str) -> Self
    {
        Self(*value)
    }
}

extern crate alloc;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use std::path::Path;
use std::path::PathBuf;

use gandr_workflow_gates::GateError;
use gandr_workflow_gates::contracts::analyze_source;
use gandr_workflow_gates::contracts::parse_nextest_witnesses;
use gandr_workflow_gates::contracts::run;

/// Per-process suffix keeping concurrently-created fixtures disjoint.
static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

#[test]
fn accepts_line_inner_and_block_docs()
{
    let source = r#"//! # Contract
//! - ensures: crate docs are checked.
//! - panics: none.
//!
//! # Adequacy
//! - hypothesis: L3 pointwise — crate docs are separated by exact witness resolution.
//! - witness: `pkg::crate_docs`

/** # Contract
- ensures: block docs are checked.
- panics: none.

# Adequacy
- hypothesis: L3 pointwise — block docs are separated by exact witness resolution.
- witness: `pkg::block_docs`
*/
pub fn block_docs() {}

/// # Contract
/// - ensures: line docs are checked.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — line docs are separated by exact witness resolution.
/// - witness: `pkg::line_docs`
pub fn line_docs() {}
"#;
    let Some(findings) = ok_or_report(
        analyze_source(
            Path::new("sample.rs"),
            source,
            &witness_set(&["pkg::crate_docs", "pkg::block_docs", "pkg::line_docs"]),
        ),
        "analyze_source",
    )
    else {
        return;
    };

    assert!(findings.is_empty(), "all syn doc styles should be accepted");
}

#[test]
fn accepts_wrapped_bullets_errors_and_unsafe_invariants()
{
    let source = r#"/// # Contract
/// - requires: callers pass a valid pointer.
/// - ensures: returns an exact value and preserves the documented boundary
///   behavior across wrapped contract bullets.
/// - provides: unsafe-capable contract grammar coverage.
/// - fails: returns `Err` when the pointer cannot be read.
/// - unsafe invariants: caller owns the pointer for the duration of the call.
/// - panics: none.
/// - intension: reads the pointer once through the public result projection.
///
/// # Errors
/// Returns `Err` when the pointer cannot be read.
///
/// # Adequacy
/// - hypothesis: L1 evidence — unsafe invariant, result, and intension surfaces
///   are separated by validating the returned evidence and exact failure value.
///   witness selection is still part of the hypothesis text before evidence bullets.
/// - witness: `unsafe_contract`
pub unsafe fn unsafe_capable() -> Result<(), ()> { Ok(()) }
"#;
    let Some(findings) = ok_or_report(
        analyze_source(
            Path::new("sample.rs"),
            source,
            &witness_set(&["unsafe_contract"]),
        ),
        "analyze_source",
    )
    else {
        return;
    };

    assert!(
        findings.is_empty(),
        "wrapped grammar and unsafe invariants should be accepted"
    );
}

#[test]
fn checks_nested_items_methods_fields_variants_traits_and_foreign_items()
{
    let source = r#"pub struct Container {
    /// # Contract
    /// - ensures: field docs are checked.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — field owner collection is separated by exact witness resolution.
    /// - witness: `field_contract`
    pub value: u8,
}

pub enum Choice {
    /// # Contract
    /// - ensures: variant docs are checked.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — variant owner collection is separated by exact witness resolution.
    /// - witness: `variant_contract`
    A,
}

impl Container {
    /// # Contract
    /// - ensures: method docs are checked.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — method owner collection is separated by exact witness resolution.
    /// - witness: `method_contract`
    pub fn method(&self) {}
}

pub trait Behavior {
    /// # Contract
    /// - ensures: trait item docs are checked.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — trait owner collection is separated by exact witness resolution.
    /// - witness: `trait_contract`
    fn act();
}

unsafe extern "C" {
    /// # Contract
    /// - ensures: foreign item docs are checked.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — foreign owner collection is separated by exact witness resolution.
    /// - witness: `foreign_contract`
    pub fn foreign();
}

pub mod nested {
    /// # Contract
    /// - ensures: nested function docs are checked.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — nested owner collection is separated by exact witness resolution.
    /// - witness: `nested_contract`
    pub fn inside() {}
}
"#;

    let Some(findings) = ok_or_report(
        analyze_source(
            Path::new("sample.rs"),
            source,
            &witness_set(&[
                "field_contract",
                "variant_contract",
                "method_contract",
                "trait_contract",
                "foreign_contract",
                "nested_contract",
            ]),
        ),
        "analyze_source",
    )
    else {
        return;
    };

    assert!(
        findings.is_empty(),
        "nested syn owners should be grouped independently"
    );
}

#[test]
fn accepts_exact_raw_package_and_crate_aliases_from_nextest_shapes()
{
    let aggregate = r#"{
  "rust-suites": {
    "gandr-graph::gandr_graph": {
      "package-name": "gandr-graph",
      "binary-name": "gandr_graph",
      "testcases": {
        "contracts::algorithm_menu_contract": { "ignored": false },
        "contracts::raw_alias": { "ignored": false }
      }
    },
    "binary-suite": {
      "package-name": "delta-package",
      "binary-name": "delta_tests",
      "testcases": {
        "contracts::binary_alias": { "ignored": false }
      }
    }
  }
}"#;
    let array_aggregate = r#"{
  "rust-suites": [
    {
      "package-name": "array-package",
      "binary-name": "array_tests",
      "testcases": [
        { "name": "contracts::array_alias", "ignored": false }
      ]
    }
  ]
}"#;
    let no_test_status_aggregate = r#"{
  "rust-suites": {
    "empty-suite": {
      "package-name": "empty-package",
      "binary-name": "empty_tests",
      "status": "listed"
    }
  }
}"#;
    let record = "{\"package\":\"record-package\",\"crate\":\"record_tests\",\"name\":\"contracts::record_alias\"}";
    let lines = "{\"package_name\":\"beta-package\",\"crate_name\":\"beta_tests\",\"test_name\":\"contracts::line_alias\"}\n";
    let Some(mut witnesses) =
        ok_or_report(parse_nextest_witnesses(aggregate), "aggregate witnesses")
    else {
        return;
    };
    let Some(array_witnesses) = ok_or_report(
        parse_nextest_witnesses(array_aggregate),
        "array aggregate witnesses",
    )
    else {
        return;
    };
    let Some(record_witnesses) = ok_or_report(parse_nextest_witnesses(record), "record witnesses")
    else {
        return;
    };
    let Some(no_test_status_witnesses) = ok_or_report(
        parse_nextest_witnesses(no_test_status_aggregate),
        "no-test status aggregate",
    )
    else {
        return;
    };
    let Some(line_witnesses) = ok_or_report(parse_nextest_witnesses(lines), "line witnesses")
    else {
        return;
    };
    let Some(empty_object_witnesses) = ok_or_report(
        parse_nextest_witnesses("{\"rust-suites\":{}}"),
        "empty object aggregate",
    )
    else {
        return;
    };
    let Some(empty_array_witnesses) = ok_or_report(
        parse_nextest_witnesses("{\"rust-suites\":[]}"),
        "empty array aggregate",
    )
    else {
        return;
    };
    witnesses.extend(array_witnesses);
    witnesses.extend(record_witnesses);
    assert!(
        no_test_status_witnesses.is_empty(),
        "package+binary suite status records should support legitimate zero-test suites"
    );
    witnesses.extend(line_witnesses);
    assert!(
        empty_object_witnesses.is_empty(),
        "empty object rust-suites aggregate should remain accepted"
    );
    assert!(
        empty_array_witnesses.is_empty(),
        "empty array rust-suites aggregate should remain accepted"
    );

    assert!(
        witnesses.contains("contracts::algorithm_menu_contract"),
        "raw testcase map alias should be present"
    );
    assert!(
        witnesses.contains("gandr-graph::contracts::algorithm_menu_contract"),
        "live package-name alias should preserve package spelling"
    );
    assert!(
        witnesses.contains("gandr_graph::contracts::algorithm_menu_contract"),
        "package-normalized and binary-name aliases should generate the live graph witness"
    );
    assert!(
        !witnesses.contains("gandr_graph::algorithm_menu_contract"),
        "a library suite must not be mistaken for a consolidated integration harness"
    );
    assert!(
        witnesses.contains("contracts::raw_alias"),
        "raw alias should be present"
    );
    assert!(
        witnesses.contains("gandr-graph::contracts::raw_alias"),
        "rust-suites object-map testcase aliases should preserve package spelling"
    );
    assert!(
        witnesses.contains("delta_package::contracts::binary_alias"),
        "package-name aliases should normalize hyphens"
    );
    assert!(
        witnesses.contains("delta_tests::contracts::binary_alias"),
        "binary-name aliases should be exact"
    );
    assert!(
        witnesses.contains("beta-package::contracts::line_alias"),
        "JSON-lines package alias should be parsed"
    );
    assert!(
        witnesses.contains("beta_tests::contracts::line_alias"),
        "JSON-lines crate alias should be parsed"
    );
    assert!(
        witnesses.contains("array_tests::contracts::array_alias"),
        "rust-suites array aliases should be parsed"
    );
    assert!(
        witnesses.contains("record-package::contracts::record_alias"),
        "single per-test JSON package alias should be parsed"
    );
    assert!(
        witnesses.contains("record_tests::contracts::record_alias"),
        "single per-test JSON crate alias should be parsed"
    );
}

#[test]
fn accepts_harness_module_stripped_aliases_from_consolidated_integration_suites()
{
    let consolidated_harness = r#"{
  "rust-suites": {
    "gandr-graph::graph": {
      "package-name": "gandr-graph",
      "binary-name": "graph",
      "testcases": {
        "algorithms::contracts::algorithm_menu_contract": { "ignored": false },
        "determinism::gandr_graph::subprocess_determinism_contract": { "ignored": false }
      }
    }
  }
}"#;
    let Some(witnesses) = ok_or_report(
        parse_nextest_witnesses(consolidated_harness),
        "consolidated integration harness witnesses",
    )
    else {
        return;
    };

    assert!(
        witnesses.contains("gandr_graph::contracts::algorithm_menu_contract"),
        "the package crate alias should omit the consolidated harness module"
    );
    assert!(
        witnesses.contains("gandr_graph::subprocess_determinism_contract"),
        "an already crate-qualified test tail should not duplicate the crate name"
    );
}

#[test]
fn rejects_unsupported_nextest_json_shapes_as_operational_errors()
{
    let unsupported_schema = "unsupported nextest list schema: expected aggregate object with rust-suites object/array or per-test record with test name and package/crate context";
    assert_eq!(
        Some(unsupported_schema),
        operational_detail(parse_nextest_witnesses("{}").err()).as_deref(),
        "empty object should not become an empty witness set"
    );
    assert_eq!(
        Some(unsupported_schema),
        operational_detail(parse_nextest_witnesses("null").err()).as_deref(),
        "null JSON should not become an empty witness set"
    );
    assert_eq!(
        Some(unsupported_schema),
        operational_detail(parse_nextest_witnesses("{\"rust-suites\":null}").err()).as_deref(),
        "rust-suites must be an object or array"
    );
    assert_eq!(
        Some(unsupported_schema),
        operational_detail(
            parse_nextest_witnesses("{\"rust-suites\":{\"suite\":\"garbage\"}}").err()
        )
        .as_deref(),
        "nonempty suite map values must be supported suite records"
    );
    assert_eq!(
        Some(unsupported_schema),
        operational_detail(parse_nextest_witnesses("{\"rust-suites\":[null]}").err()).as_deref(),
        "suite array entries must be supported suite records"
    );
    assert_eq!(
        Some(unsupported_schema),
        operational_detail(parse_nextest_witnesses("{\"rust-suites\":{\"suite\":{}}}").err())
            .as_deref(),
        "nonempty suite maps need concrete supported suite records"
    );
    assert_eq!(
        Some(unsupported_schema),
        operational_detail(
            parse_nextest_witnesses("{\"rust-suites\":{\"suite\":{\"package-name\":\"p\"}}}").err()
        )
        .as_deref(),
        "context-only suite records need binary identity and testcase or status"
    );
    assert_eq!(
        Some(unsupported_schema),
        operational_detail(
            parse_nextest_witnesses(
                "{\"rust-suites\":{\"suite\":{\"package-name\":\"p\",\"binary-name\":\"b\"}}}"
            )
            .err()
        )
        .as_deref(),
        "package+binary context alone should not become an empty witness set"
    );
    assert_eq!(
            Some(unsupported_schema),
            operational_detail(
                parse_nextest_witnesses(
                    "{\"rust-suites\":[{\"package\":\"fixture\",\"binary-name\":\"fixture_tests\",\"testcases\":\"garbage\"}]}"
                )
                .err()
            )
            .as_deref(),
            "testcase collections must be objects or arrays"
        );
    assert_eq!(
            Some(unsupported_schema),
            operational_detail(
                parse_nextest_witnesses(
                    "{\"rust-suites\":[{\"package\":\"fixture\",\"testcases\":{\"contracts::hidden\":{}}}]}"
                )
                .err()
            )
            .as_deref(),
            "suite testcase records need both package and binary identity"
        );
    assert_eq!(
            Some(unsupported_schema),
            operational_detail(
                parse_nextest_witnesses(
                    "{\"rust-suites\":[{\"binary-name\":\"fixture_tests\",\"testcases\":{\"contracts::hidden\":{}}}]}"
                )
                .err()
            )
            .as_deref(),
            "suite testcase records need both package and binary identity"
        );
    assert_eq!(
            Some(unsupported_schema),
            operational_detail(
                parse_nextest_witnesses(
                    "{\"rust-suites\":[{\"package\":\"fixture\",\"binary-name\":\"fixture_tests\",\"testcases\":{\"contracts::hidden\":null}}]}"
                )
                .err()
            )
            .as_deref(),
            "testcase map entries must be supported objects or arrays"
        );
    assert_eq!(
            Some(unsupported_schema),
            operational_detail(
                parse_nextest_witnesses(
                    "{\"rust-suites\":[{\"package\":\"fixture\",\"binary-name\":\"fixture_tests\",\"testcases\":[null]}]}"
                )
                .err()
            )
            .as_deref(),
            "testcase array entries must be supported objects or arrays"
        );
    assert_eq!(
        Some(unsupported_schema),
        operational_detail(parse_nextest_witnesses("{\"tests\":[\"not nextest\"]}").err())
            .as_deref(),
        "unrelated valid JSON should fail before alias extraction"
    );
    let bad_lines = "{\"package\":\"ok-package\",\"test_name\":\"contracts::ok\"}\n{\"name\":\"contracts::hidden\"}\n";
    assert_eq!(
        Some(
            "unsupported nextest JSON-lines record at line 2: expected per-test record with test name and package/crate context"
        ),
        operational_detail(parse_nextest_witnesses(bad_lines).err()).as_deref(),
        "JSON-lines records need recognized package or crate context"
    );
}

#[test]
fn rejects_contract_grammar_drift_modes()
{
    let source = r#"/// # Contract
/// This prose is not a bullet.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — prose is rejected.
/// - witness: `ok`
pub fn aa_contract_prose() {}

/// # Contract
/// - requires: input is valid.
/// - mystery: not allowed.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — unknown clauses are rejected.
/// - witness: `ok`
pub fn ab_unknown_clause() {}

/// # Contract
/// - ensures: first.
/// - ensures: duplicate.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — duplicate clauses are rejected.
/// - witness: `ok`
pub fn ac_duplicate_clause() {}

/// # Contract
/// - ensures: after requires.
/// - requires: before ensures.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — out-of-order clauses are rejected.
/// - witness: `ok`
pub fn ad_out_of_order_clause() {}

/// # Contract
/// - ensures: missing panics is rejected.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — missing panics is rejected.
/// - witness: `ok`
pub fn ae_missing_panics() {}

/// # Contract
/// - ensures: intension placement is checked.
/// - intension: promises traversal order.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — intension order is rejected.
/// - witness: `ok`
pub fn af_intension_not_last() {}

/// # Contract
/// - fails: unsafe invariants order is checked.
/// - panics: none.
/// - unsafe invariants: this must precede panics.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — unsafe invariant order is rejected.
/// - witness: `ok`
pub unsafe fn ag_unsafe_invariants_order() {}

/// # Contract
/// - ensures: missing hypothesis is rejected.
/// - panics: none.
///
/// # Adequacy
pub fn ah_missing_hypothesis() {}

/// # Contract
/// - ensures: duplicate hypothesis is rejected.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — first.
/// - hypothesis: L3 pointwise — second.
/// - witness: `ok`
pub fn ai_duplicate_hypothesis() {}

/// # Contract
/// - ensures: late hypothesis is rejected.
/// - panics: none.
///
/// # Adequacy
/// - witness: `ok`
/// - hypothesis: L3 pointwise — too late.
pub fn aj_late_hypothesis() {}

/// # Contract
/// - ensures: non-rung hypothesis is rejected.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: pointwise only — no ladder rung token.
/// - witness: `ok`
pub fn ak_non_rung_hypothesis() {}

/// # Contract
/// - ensures: adequacy prose is rejected.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — prose is rejected.
/// prose between bullets is forbidden.
/// - witness: `ok`
pub fn al_adequacy_prose() {}

/// # Contract
/// - ensures: malformed witness syntax is rejected.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — malformed witnesses are rejected.
/// witness: `good::malformed`
pub fn am_malformed_witness() {}

/// # Contract
/// - ensures: post-witness prose is rejected even when indented.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — post-witness continuation is not legal.
/// - witness: `ok`
///   witness prose after the first evidence bullet is malformed.
pub fn an_post_witness_prose() {}

/// # Contract
/// - ensures: stale exact witnesses are rejected.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — stale witnesses are rejected.
/// - witness: `good::stale`
pub fn an_stale_witness() {}

/// # Contract
/// - ensures: substrings are not witnesses.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — substring witnesses are rejected.
/// - witness: `good::substring`
pub fn ao_substring_witness() {}

/// # Contract
/// - ensures: section order is checked.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — section order is rejected.
/// - witness: `ok`
///
/// # Errors
/// Errors must not follow adequacy.
pub fn ap_wrong_section_order() -> Result<(), ()> { Ok(()) }

/// # Contract
/// - ensures: missing adequacy is rejected.
/// - panics: none.
pub fn aq_missing_adequacy() {}
"#;

    let Some(findings) = ok_or_report(
        analyze_source(
            Path::new("sample.rs"),
            source,
            &witness_set(&["ok", "good::substring_extra"]),
        ),
        "analyze_source",
    )
    else {
        return;
    };
    let actual: Vec<_> = findings
        .iter()
        .map(|finding| finding.kind.as_str())
        .collect();

    assert_eq!(
        vec![
            "contract-prose",
            "unknown-contract-clause",
            "duplicate-contract-clause",
            "out-of-order-contract-clause",
            "missing-panics",
            "intension-not-last",
            "out-of-order-contract-clause",
            "missing-hypothesis",
            "duplicate-hypothesis",
            "late-hypothesis",
            "hypothesis-rung",
            "adequacy-prose",
            "malformed-witness",
            "malformed-witness",
            "stale-witness",
            "stale-witness",
            "section-order",
            "missing-adequacy",
        ],
        actual,
        "all grammar drift modes should be classified deterministically"
    );
}

#[test]
fn rejects_wrong_level_fixed_section_headings()
{
    let source = r#"/// ## Contract
/// - ensures: wrong-level contract heading is not accepted.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — wrong-level Contract is rejected.
/// - witness: `ok`
pub fn wrong_contract_heading() {}

/// # Contract
/// - ensures: wrong-level Errors is rejected.
/// - panics: none.
///
/// ## Errors
/// Returns nothing.
///
/// # Adequacy
/// - hypothesis: L3 pointwise — wrong-level Errors is rejected.
/// - witness: `ok`
pub fn wrong_errors_heading() {}

/// # Contract
/// - ensures: wrong-level Adequacy is rejected.
/// - panics: none.
///
/// ###### Adequacy
/// - hypothesis: L3 pointwise — wrong-level Adequacy is rejected.
/// - witness: `ok`
pub fn wrong_adequacy_heading() {}
"#;

    let Some(findings) = ok_or_report(
        analyze_source(
            Path::new("wrong-headings.rs"),
            source,
            &witness_set(&["ok"]),
        ),
        "analyze_source",
    )
    else {
        return;
    };
    let actual: Vec<_> = findings
        .iter()
        .map(|finding| {
            (
                finding.declaration.as_str(),
                finding.kind.as_str(),
                finding.detail.as_str(),
            )
        })
        .collect();

    assert_eq!(
        vec![
            (
                "fn wrong_adequacy_heading",
                "section-heading-level",
                "Adequacy section must use exactly one # heading: ###### Adequacy",
            ),
            (
                "fn wrong_contract_heading",
                "section-heading-level",
                "Contract section must use exactly one # heading: ## Contract",
            ),
            (
                "fn wrong_errors_heading",
                "section-heading-level",
                "Errors section must use exactly one # heading: ## Errors",
            ),
        ],
        actual,
        "fixed sections must require exactly one ATX #"
    );
}

#[test]
fn returns_parse_failures_as_operational_errors()
{
    let error = analyze_source(Path::new("broken.rs"), "pub fn broken(", &BTreeSet::new()).err();

    assert!(
        matches!(error, Some(GateError::RustParse { .. })),
        "unparseable source should be an operational RustParse error"
    );
}

#[test]
fn run_reports_findings_in_deterministic_path_order()
{
    let root = unique_temp_root();
    let late = root.join("z.rs");
    let early_dir = root.join("a");
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&early_dir),
        "create early dir",
    )
    else {
        return;
    };
    let early = early_dir.join("a.rs");
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .write(&late, contract_source("late::missing")),
        "write late",
    )
    else {
        return;
    };
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .write(&early, contract_source("early::missing")),
        "write early",
    )
    else {
        return;
    };
    let fixture = root.join("nextest.json");
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(&fixture, "{\"rust-suites\":[]}"),
        "write fixture",
    )
    else {
        return;
    };

    let Some(findings) = ok_or_report(
        run(core::slice::from_ref(&root), Some(&fixture)),
        "run contracts",
    )
    else {
        return;
    };
    let paths: Vec<_> = findings
        .iter()
        .map(|finding| finding.path.as_str())
        .collect();
    let early_path = format!("{}", early.display());
    let late_path = format!("{}", late.display());

    assert_eq!(vec![early_path.as_str(), late_path.as_str()], paths);
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(root),
        "remove temp root",
    )
    else {
        return;
    };
}

fn ok_or_report<'semantic, T, E: fmt::Display>(
    result: Result<T, E>,
    context: impl Into<ContextText<'semantic>>,
) -> Option<T>
{
    let context = context.into().0;
    result
        .inspect_err(|error| assert!(context.is_empty(), "{context}: {error}"))
        .ok()
}

fn witness_set<'semantic, Values, Value>(values: Values) -> BTreeSet<String>
where
    Values: IntoIterator<Item = Value>,
    Value: Into<WitnessText<'semantic>>,
{
    let mut witnesses = BTreeSet::new();
    for value in values {
        witnesses.insert(String::from(value.into().0));
    }
    witnesses
}

#[cfg(unix)]
#[test]
fn run_rejects_root_symlink_scope_and_skips_child_symlinks()
{
    let root = unique_temp_root();
    let fixture = root.join("nextest.json");
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(&fixture, "{\"rust-suites\":[]}"),
        "write fixture",
    )
    else {
        return;
    };
    let source = root.join("source.rs");
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM
            .write(&source, contract_source("source::missing")),
        "write source",
    )
    else {
        return;
    };
    let outside = root.with_extension("outside");
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&outside),
        "create outside",
    )
    else {
        return;
    };
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.write(
            outside.join("escaped.rs"),
            contract_source("escaped::missing"),
        ),
        "write escaped source",
    )
    else {
        return;
    };
    let cycle_link = root.join("cycle");
    let escape_link = root.join("escape");
    let root_link = root.with_extension("link");
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.symlink(&root, &cycle_link),
        "link cycle",
    )
    else {
        return;
    };
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.symlink(&outside, &escape_link),
        "link escape",
    )
    else {
        return;
    };
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.symlink(&root, &root_link),
        "link root",
    )
    else {
        return;
    };

    let Some(findings) = ok_or_report(
        run(core::slice::from_ref(&root), Some(&fixture)),
        "run child symlinks",
    )
    else {
        return;
    };
    let paths: Vec<_> = findings
        .iter()
        .map(|finding| finding.path.as_str())
        .collect();
    let source_path = format!("{}", source.display());
    assert_eq!(
        vec![source_path.as_str()],
        paths,
        "child symlinks should be skipped without cycling or escaping"
    );

    let expected_root_symlink_detail = format!("scope path is a symlink: {}", root_link.display());
    assert_eq!(
        Some(expected_root_symlink_detail.as_str()),
        operational_detail(run(core::slice::from_ref(&root_link), Some(&fixture)).err()).as_deref(),
        "root symlink scopes should fail closed before traversal"
    );

    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(&root),
        "remove temp root",
    )
    else {
        return;
    };
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(&outside),
        "remove outside",
    )
    else {
        return;
    };
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.remove_file(&root_link),
        "remove root link",
    )
    else {
        return;
    };
}

#[test]
fn temp_roots_are_unique_within_one_process()
{
    let first = unique_temp_root();
    let second = unique_temp_root();

    assert_ne!(first, second, "parallel tests must not share fixture roots");
    let _first_cleanup = gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(first);
    let _second_cleanup = gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_all(second);
}

fn unique_temp_root() -> PathBuf
{
    let suffix = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "gandr-workflow-gates-contracts-{}-{suffix}",
        std::process::id()
    ));
    let cleanup = gandr_workflow_gates::support::HOST_FILESYSTEM.remove_dir_if_exists(&path);
    assert!(
        cleanup.is_ok(),
        "failed to clean temporary contract root {}: {cleanup:?}",
        path.display()
    );
    let Some(()) = ok_or_report(
        gandr_workflow_gates::support::HOST_FILESYSTEM.create_dir_all(&path),
        "create temp root",
    )
    else {
        return path;
    };
    path
}

fn contract_source<'semantic>(witness: impl Into<WitnessText<'semantic>>) -> String
{
    let witness = witness.into().0;
    format!(
        "/// # Contract\n/// - ensures: deterministic order.\n/// - panics: none.\n///\n/// # Adequacy\n/// - hypothesis: L3 pointwise — deterministic ordering is separated by exact path assertions.\n/// - witness: `{witness}`\npub fn item() {{}}\n"
    )
}

fn operational_detail(error: Option<GateError>) -> Option<String>
{
    match error {
        | Some(GateError::Operational { detail, .. }) => Some(detail),
        | _ => None,
    }
}
