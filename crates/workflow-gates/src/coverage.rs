//! Per-file line coverage floor policy gate.
//!
//! # Contract
//! - requires: integration code wires these library APIs behind the gate CLI
//!   and supplies the support module for sanitized command execution and atomic
//!   file I/O.
//! - ensures: coverage summaries and floor policies are parsed, checked,
//!   ratcheted, and rendered according to the retained Nushell coverage-ratchet
//!   contract.
//! - provides: typed check and ratchet APIs without command-line argument
//!   parsing, stdout/stderr printing, or process exits.
//! - fails: returns crate [`GateError`](crate::GateError) values for
//!   operational failures and crate [`Finding`](crate::Finding) rows for
//!   semantic policy failures.
//! - panics: none.
//! - intension: domain joins and rendering are deterministic `BTreeMap` passes;
//!   external Git access is isolated to sanitized support calls.
//!
//! # Adequacy
//! - hypothesis: L3 only — behavior witnesses in `model`, `policy`, and
//!   `render` separate parser identity, path normalization, monotonicity,
//!   ratchet rendering, and base-ref policy decisions.
//! - witness: `coverage::model::tests::percent_floors_down_without_float_math`
//! - witness: `coverage::policy::tests::check_report_covers_policy_failure_families`
//! - witness: `coverage::render::tests::render_floors_is_sorted_and_stable`

crate::semantic_str!(pub struct SourceNameText);
crate::semantic_str!(pub struct TextText);

mod model;
mod policy;
mod render;

pub use model::CoverageFloors;
pub use model::MeasuredCoverage;
pub use model::Percent;
pub use model::ProductionFile;
pub use model::RatchetReport;
pub use policy::DEFAULT_FLOORS;
pub use policy::DEFAULT_SUMMARY;
pub use policy::check;
pub use policy::check_with_base_policy;
pub use policy::ratchet;
pub use policy::ratchet_report;

/// Parse coverage-floor policy text for the crate fuzzing facade.
///
/// # Contract
/// - requires: `text` is already UTF-8 coverage-floor TOML input.
/// - ensures: runs the same current-policy parser and validation path used by
///   loaded floor policies, without filesystem, process, environment, or output
///   effects.
/// - provides: a feature-gated crate-private bridge from the public fuzzing
///   facade to the policy module internals.
/// - fails: returns the existing coverage-policy gate error for malformed TOML
///   or invalid policy semantics.
/// - panics: none.
/// - intension: forwards directly to the policy module's fuzzing seam.
///
/// # Errors
/// Returns [`crate::GateError`] when parsing or validation rejects the input.
///
/// # Adequacy
/// - hypothesis: L3 only — AFL observes the exact same restricted-TOML parser
///   and validation decisions as production floor-policy loading.
/// - witness: `gandr_workflow_gates::fuzzing::exercise_fuzz_input`
#[cfg(feature = "fuzzing")]
#[inline]
pub(crate) fn parse_floors_text_for_fuzzing<'semantic>(
    source_name: impl Into<SourceNameText<'semantic>>,
    text: impl Into<TextText<'semantic>>,
) -> Result<CoverageFloors, crate::GateError>
{
    let text = text.into().0;
    let source_name = source_name.into().0;
    policy::parse_floors_text_for_fuzzing(source_name, text)
}
