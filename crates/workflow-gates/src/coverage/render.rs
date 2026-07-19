//! Stable TOML rendering for coverage floors.
//!
//! # Contract
//! - requires: callers provide validated canonical production file keys and
//!   already-pruned exemption rows.
//! - ensures: output is sorted, stable, and byte-for-byte idempotent for the
//!   coverage floor policy subset.
//! - provides: TOML serialization shared by ratchet and snapshot witnesses.
//! - panics: none.
//! - intension: rendering iterates only `BTreeMap` order and writes a fixed
//!   header, fixed blank-line layout, then sorted sections.
//!
//! # Adequacy
//! - hypothesis: L3 only — golden snapshots with reordered inputs, stale rows,
//!   and escaped keys/reasons kill ordering, layout, and escaping mutants.
//! - witness: `coverage::render::tests::render_floors_is_sorted_and_stable`
//! - witness: `coverage::render::tests::toml_key_escapes_quotes_and_backslashes`

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "coverage renderer tests use direct golden-fixture assertions"
    )
)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::model::Percent;
use super::model::ProductionFile;
crate::semantic_str!(pub struct ValueText);

/// Render a complete coverage floors TOML document.
///
/// # Contract
/// - requires: `rows` and `exemptions` are keyed by validated production files;
///   exemption rows have already been checked against their floors.
/// - ensures: returns the exact sorted policy TOML format retained from the
///   Nushell gate, with no trailing newline.
/// - provides: deterministic ratchet output.
/// - panics: none.
/// - intension: performs a single ordered pass over floor rows and a single
///   ordered pass over exemption rows.
///
/// # Adequacy
/// - hypothesis: L3 only — a snapshot with intentionally unsorted construction
///   order, a stale row, and an exemption distinguishes header, blank lines,
///   sorting, and section-presence decisions.
/// - witness: `coverage::render::tests::render_floors_is_sorted_and_stable`
#[inline]
#[must_use]
pub(super) fn render_floors(
    target_percent: Percent,
    rows: &BTreeMap<ProductionFile, Percent>,
    exemptions: &BTreeMap<ProductionFile, String>,
) -> String
{
    let mut lines = Vec::new();
    lines.push(String::from(
        "# Per-file line-coverage ratchet. Floors only fall when the policy target falls.",
    ));
    lines.push(String::new());
    lines.push(format!("target_percent = {target_percent}"));
    lines.push(String::new());
    lines.push(String::from("[files]"));
    for (file, floor) in rows {
        lines.push(format!("{} = {floor}", toml_key(file.as_str().into().0)));
    }
    if !exemptions.is_empty() {
        lines.push(String::new());
        lines.push(String::from("[new_file_exemptions]"));
        for (file, reason) in exemptions {
            lines.push(format!(
                "{} = {}",
                toml_key(file.as_str().into().0),
                toml_key(reason)
            ));
        }
    }
    lines.join("\n")
}

/// Quote one TOML basic string key or value for the restricted policy subset.
///
/// # Contract
/// - requires: `value` is UTF-8 policy text.
/// - ensures: quotes `value` and escapes backslashes and double quotes.
/// - provides: minimal TOML-compatible escaping for file keys and exemption
///   reasons used by the legacy gate.
/// - panics: none.
/// - intension: scans `value` once and appends escaped fragments directly.
///
/// # Adequacy
/// - hypothesis: L3 only — one value containing both `\\` and `"` kills the two
///   escape branches and the surrounding quotes.
/// - witness: `coverage::render::tests::toml_key_escapes_quotes_and_backslashes`
#[inline]
#[must_use]
fn toml_key<'semantic>(value: impl Into<ValueText<'semantic>>) -> String
{
    let value = value.into().0;
    let mut escaped = String::new();
    escaped.push('"');
    for character in value.chars() {
        match character {
            | '\\' => escaped.push_str("\\\\"),
            | '"' => escaped.push_str("\\\""),
            | _ => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::string::String;

    use super::render_floors;
    use super::toml_key;
    use crate::coverage::model::DEFAULT_TARGET_PERCENT;
    use crate::coverage::model::Percent;
    use crate::coverage::model::ProductionFile;

    /// Renderer orders rows and sections deterministically.
    #[test]
    fn render_floors_is_sorted_and_stable()
    {
        let mut rows = BTreeMap::new();
        let parser = ProductionFile::from_floor_key("crates/demo/src/parser.rs")
            .expect("fixture parser path must be valid");
        let lib = ProductionFile::from_floor_key("crates/demo/src/lib.rs")
            .expect("fixture lib path must be valid");
        rows.insert(
            parser,
            Percent::parse_exact("77.00").expect("percent fixture is valid"),
        );
        rows.insert(
            lib,
            Percent::parse_exact("80.00").expect("percent fixture is valid"),
        );

        let mut exemptions = BTreeMap::new();
        let new_file = ProductionFile::from_floor_key("crates/demo/src/new.rs")
            .expect("fixture new path must be valid");
        exemptions.insert(new_file, String::from("Temporary generated adapter"));

        let expected = [
            "# Per-file line-coverage ratchet. Floors only fall when the policy target falls.",
            "",
            "target_percent = 80.00",
            "",
            "[files]",
            "\"crates/demo/src/lib.rs\" = 80.00",
            "\"crates/demo/src/parser.rs\" = 77.00",
            "",
            "[new_file_exemptions]",
            "\"crates/demo/src/new.rs\" = \"Temporary generated adapter\"",
        ]
        .join("\n");
        assert_eq!(
            render_floors(DEFAULT_TARGET_PERCENT, &rows, &exemptions),
            expected,
            "rendered floors should match the stable TOML snapshot",
        );
    }

    /// TOML quoting escapes only the characters the legacy renderer escaped.
    #[test]
    fn toml_key_escapes_quotes_and_backslashes()
    {
        assert_eq!(
            "\"crates/demo/src/quote\\\"slash\\\\.rs\"",
            toml_key("crates/demo/src/quote\"slash\\.rs"),
            "TOML key escaping should preserve quote and backslash decisions",
        );
    }
}
