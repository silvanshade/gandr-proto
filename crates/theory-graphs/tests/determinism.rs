//! Process-level determinism regression for the public graph façade probe.
//!
//! # Contract
//! - ensures: requires two accepted fresh probe processes to exit successfully
//!   and emit byte-identical stdout containing the graph foundation,
//!   precedence-DAG, partition, simulation, and walk-index fingerprint rows,
//!   and requires an oversized hostile perturbation process to fail without
//!   panic/abort text.
//! - provides: the stdout self-comparison witness and bounded-perturbation
//!   failure witness for the determinism probe.
//! - fails: returns the process-spawn `std::io::Error` when any probe cannot be
//!   executed.
//! - panics: if an accepted process exits unsuccessfully, the pairwise stdout
//!   bytes differ, the hostile process succeeds, or hostile stderr lacks the
//!   typed perturbation-bound evidence.
//! - intension: compares two fresh-process traces with distinct accepted
//!   allocation-perturbation settings, and separately observes the operational
//!   error path for a parseable perturbation above the finite probe bound.
//!
//! # Adequacy
//! - hypothesis: L3 pairwise only — distinct accepted perturbation inputs must
//!   produce byte-identical stdout in two fresh processes, while the exact
//!   maximum perturbation remains an accepted successful process and the
//!   oversized-input witness only proves the probe rejects hostile allocation
//!   counts operationally, not any external graph, precedence, partition,
//!   simulation, or walk-index correctness property.
//! - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_determinism_contract`
//! - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_exact_maximum_perturbation_is_accepted`
//! - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_oversized_perturbation_fails_gracefully`

use core::error::Error;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;
use std::process::Command;
use std::process::Output;

/// Test-local result with contextual process-spawn failures.
type TestResult = Result<(), Box<dyn Error>>;

/// Borrowed perturbation value for a determinism probe process.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProbePerturbation<'probe>(&'probe str);

impl<'probe> From<&'probe str> for ProbePerturbation<'probe>
{
    #[inline]
    fn from(value: &'probe str) -> Self
    {
        Self(value)
    }
}

impl<'probe> From<&'probe String> for ProbePerturbation<'probe>
{
    #[inline]
    fn from(value: &'probe String) -> Self
    {
        Self(value.as_str())
    }
}

/// Borrowed stderr text for hostile-process assertions.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProbeStderr<'probe>(&'probe str);

impl<'probe> From<&'probe str> for ProbeStderr<'probe>
{
    #[inline]
    fn from(value: &'probe str) -> Self
    {
        Self(value)
    }
}

impl<'probe> From<&'probe String> for ProbeStderr<'probe>
{
    #[inline]
    fn from(value: &'probe String) -> Self
    {
        Self(value.as_str())
    }
}

/// Borrowed stdout bytes for process-row assertions.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProbeStdout<'probe>(&'probe [u8]);

impl<'probe> From<&'probe [u8]> for ProbeStdout<'probe>
{
    #[inline]
    fn from(value: &'probe [u8]) -> Self
    {
        Self(value)
    }
}

impl<'probe> From<&'probe Vec<u8>> for ProbeStdout<'probe>
{
    #[inline]
    fn from(value: &'probe Vec<u8>) -> Self
    {
        Self(value.as_slice())
    }
}
/// Spawn failure for one determinism-probe perturbation.
#[derive(Debug)]
struct ProbeRunError
{
    /// Requested perturbation value.
    perturb: String,
    /// Underlying process-spawn error.
    source: std::io::Error,
}

impl Display for ProbeRunError
{
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> FmtResult
    {
        write!(
            f,
            "failed to run gandr-theory-graphs-determinism with GANDR_THEORY_GRAPHS_PERTURB={}: {}",
            self.perturb, self.source
        )
    }
}

impl Error for ProbeRunError
{
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        Some(&self.source)
    }
}

/// Run the determinism probe in a fresh process with one perturbation value.
fn run_probe<'probe, P>(perturb: P) -> Result<Output, ProbeRunError>
where
    P: Into<ProbePerturbation<'probe>>,
{
    let perturb = perturb.into();
    Command::new(env!("CARGO_BIN_EXE_gandr-theory-graphs-determinism"))
        .env("GANDR_THEORY_GRAPHS_PERTURB", perturb.0)
        .output()
        .map_err(|source| ProbeRunError {
            perturb: perturb.0.to_owned(),
            source,
        })
}

/// Render stderr lossily for assertion diagnostics only.
fn stderr(output: &Output) -> String
{
    return String::from_utf8_lossy(&output.stderr).into_owned();
}

/// Assert stderr does not report a Rust panic or process abort.
fn assert_no_panic_or_abort<'probe, S>(stderr: S)
where
    S: Into<ProbeStderr<'probe>>,
{
    let stderr = stderr.into();
    let lower = stderr.0.to_ascii_lowercase();
    assert!(
        !lower.contains("panic"),
        "probe stderr contained panic text: {}",
        stderr.0
    );
    assert!(
        !lower.contains("abort"),
        "probe stderr contained abort text: {}",
        stderr.0
    );
}

/// Assert the probe stdout contains the supplemental determinism rows.
///
/// # Contract
/// - requires: `stdout` is the accepted probe process output.
/// - ensures: panics if any supplemental row label is absent.
/// - provides: a shape witness that the process self-comparison covers the new
///   precedence, partition/simulation, and walk fingerprint rows.
/// - panics: when stdout lacks a required row label.
/// - intension: validates row labels only; exact semantic goldens remain in the
///   implementation tests.
///
/// # Adequacy
/// - hypothesis: L3 pairwise only — row-label presence distinguishes accidental
///   omission from the process projection without replacing external oracles or
///   pinning a walk fingerprint literal.
/// - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_determinism_contract`
fn assert_new_api_rows<'probe, S>(stdout: S)
where
    S: Into<ProbeStdout<'probe>>,
{
    let stdout = stdout.into();
    let stdout = String::from_utf8_lossy(stdout.0);
    for label in [
        "prec_groups=",
        "prec_edges=",
        "prec_fingerprint=",
        "prec_linear_extension=",
        "prec_comparisons=",
        "prec_boundaries=",
        "bisimulation_partition=",
        "simulation_relation=",
        "walk_fingerprint=",
    ] {
        assert!(
            stdout.contains(label),
            "probe stdout lacked determinism row {label}: {stdout}"
        );
    }
}

#[cfg(test)]
mod gandr_theory_graphs
{
    use super::TestResult;
    use super::assert_new_api_rows;
    use super::assert_no_panic_or_abort;
    use super::run_probe;
    use super::stderr;

    /// The probe's stdout is independent of irrelevant pre-graph allocation
    /// order.
    #[test]
    fn subprocess_determinism_contract() -> TestResult
    {
        let first = run_probe("1")?;
        let second = run_probe("97")?;

        assert!(
            first.status.success(),
            "first probe failed: {}",
            stderr(&first)
        );
        assert!(
            second.status.success(),
            "second probe failed: {}",
            stderr(&second)
        );
        assert!(
            first.stdout == second.stdout,
            "probe stdout differed across distinct perturbation processes"
        );
        assert_new_api_rows(&first.stdout);

        return Ok(());
    }

    /// The finite perturbation boundary is accepted instead of rejected as an
    /// off-by-one hostile request.
    #[test]
    fn subprocess_exact_maximum_perturbation_is_accepted() -> TestResult
    {
        let output = run_probe("1024")?;

        assert!(
            output.status.success(),
            "exact maximum perturbation failed: {}",
            stderr(&output)
        );
        assert_new_api_rows(&output.stdout);

        return Ok(());
    }

    /// Hostile allocation perturbations fail as typed operational errors, not
    /// panics, aborts, capacity overflows, or unbounded allocation attempts.
    #[test]
    fn subprocess_oversized_perturbation_fails_gracefully() -> TestResult
    {
        let perturbation = usize::MAX.to_string();
        let output = run_probe(&perturbation)?;
        let stderr = stderr(&output);

        assert!(
            !output.status.success(),
            "oversized perturbation unexpectedly succeeded"
        );
        assert!(
            stderr.contains("PerturbationTooLarge"),
            "stderr lacked typed perturbation error: {stderr}"
        );
        assert!(
            stderr.contains("GANDR_THEORY_GRAPHS_PERTURB"),
            "stderr lacked perturbation variable evidence: {stderr}"
        );
        assert!(
            stderr.contains("maximum: 1024"),
            "stderr lacked stable perturbation bound evidence: {stderr}"
        );
        assert_no_panic_or_abort(&stderr);

        return Ok(());
    }
}
