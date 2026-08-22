//! Lowering then typing arbitrary bytes must not panic, and the recursive
//! checker must agree with the typing machine on every lowered item — the
//! byte-level extension of the conformance property both runners state in
//! their own contracts.
//!
//! # The nesting prefilter, and why the property needs one
//!
//! The two runners are not equally robust, and both say so. The machine keeps
//! its frame stack on the heap, so adversarial depth costs it memory and
//! nothing else. The checker is direct-style and recurses on the host call
//! stack, so a term whose nesting exceeds the thread's stack **aborts the
//! process** — not a panic, not an `Err`, an abort — and its contract directs
//! adversarial-depth inputs to the machine instead.
//!
//! A byte fuzzer reaches that depth almost at once. Without a prefilter the
//! campaign fills with aborts that are documented behaviour, and the real
//! findings drown in them. So this target measures the input's nesting first
//! and hands the checker only what the checker's own contract admits.
//!
//! **The skipped class is outside the relation's domain, not a suppressed
//! finding.** Conformance compares two answers, and an input the checker
//! cannot answer for has no pair to compare. Stating it the other way — as a
//! known-failing case we look past — would be the same fact recorded as a
//! weaker one.
//!
//! Seeds: `fuzz/corpus/check/`.

use gandr_core_checker::judgements::checker;
use gandr_core_checker::judgements::control::Dir;
use gandr_core_term::syntax::Term;
use gandr_surface_engine::boundary::PipelineSource;
use gandr_surface_engine::lower::lower_source_total;
use gandr_surface_engine::prelude::prelude_ctx;

/// The greatest source nesting this target hands to the recursive checker.
///
/// # Contract
/// - requires: a value low enough that the checker's host-stack recursion
///   survives the deepest term the source can lower to, on the smallest thread
///   stack any campaign runs under, with margin.
/// - ensures: no input passed to [`checker::run_value`] can abort the process
///   through stack exhaustion.
/// - provides: a stated, calibrated bound rather than a guessed one.
///
/// **Calibration is owed, and a guess is not a calibration.** The number is
/// arrived at by measuring where the abort actually starts under the campaign
/// build profile, then taking a fraction of it — a fuzz build is not a test
/// build and the release profile's frame sizes are the ones that matter. The
/// measurement and the margin are recorded on `gandr-wvd.24.9`, and a seed at
/// the boundary is committed to `fuzz/corpus/check/` so the bound has a
/// witness that runs under `fuzz:rust-smoke`.
/// **The value below is provisional and must be replaced by the measured
/// one.** It is set low enough to be safe on any plausible stack and is
/// therefore certainly costing coverage; that is the correct direction to be
/// wrong in while the measurement is owed.
const MAX_CHECKER_NESTING: usize = 64;

/// The maximum bracket nesting reached anywhere in `source`.
///
/// # Contract
/// - requires: nothing; the input is arbitrary bytes lossily decoded.
/// - ensures: returns an upper bound on the nesting depth of any term the
///   source can lower to, counted over the surface's grouping delimiters.
/// - provides: a cheap syntactic over-approximation. Over-approximating is the
///   safe direction here: it can only skip an input the checker would in fact
///   have survived, which costs coverage, where under-approximating admits an
///   input that aborts the campaign.
/// - fails: never; unbalanced and stray closers are counted as the surface's
///   own recovery sees them, without an error path.
/// - panics: none.
///
/// Witness: a boundary seed in `fuzz/corpus/check/`, replayed by
/// `fuzz:rust-smoke`.
fn source_nesting_depth(source: &str) -> usize
{
    todo!(
        "count the running bracket depth over the surface's grouping delimiters and return its maximum"
    )
}

fn main()
{
    afl::fuzz!(|data: &[u8]| {
        let source = String::from_utf8_lossy(data);
        if source_nesting_depth(source.as_ref()) > MAX_CHECKER_NESTING {
            return;
        }
        let Ok(lowered) = lower_source_total(PipelineSource::from(source.as_ref()))
        else {
            return;
        };
        for item in &lowered.items {
            match &item.term {
                | Term::Value(value) => {
                    let (rec, _) = checker::run_value(prelude_ctx(), value.clone(), Dir::Infer);
                    let (mach, _) =
                        gandr_core_machine::run_value(prelude_ctx(), value.clone(), Dir::Infer);
                    assert_eq!(rec, mach, "checker and machine disagree on {item:?}");
                },
                | Term::Comp(comp) => {
                    let (rec, _) = checker::run_comp(prelude_ctx(), comp.clone(), Dir::Infer);
                    let (mach, _) =
                        gandr_core_machine::run_comp(prelude_ctx(), comp.clone(), Dir::Infer);
                    assert_eq!(rec, mach, "checker and machine disagree on {item:?}");
                },
            }
        }
    });
}
