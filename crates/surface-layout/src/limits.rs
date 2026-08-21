//! Build-phase budgets and the meter that enforces them.
//!
//! Resource use in this crate is explicit rather than emergent. A caller states
//! the ceilings, the meter counts what is actually spent, and every store
//! checks its ceiling and then reserves fallibly, so exhaustion is reported at
//! the API rather than felt as allocator pressure somewhere below it.
//!
//! Build accounting and render accounting are disjoint, and this is load
//! bearing: a document that is expensive to construct cannot quietly consume
//! the budget the renderer was promised, and a limit crossed during
//! finalization can never be reported as a render limit.
//!
//! # The binding defaults
//!
//! | limit                                           | default    |
//! | ----------------------------------------------- | ---------- |
//! | stored document nodes, including flatten images | 1,000,000  |
//! | uniquely stored text and verbatim bytes         | 64 MiB     |
//! | stored verbatim physical fragments              | 1,000,000  |
//! | constructor and finalization build steps        | 20,000,000 |
//!
//! # What slice one owns here
//!
//! The two records below are data and are complete. The meter's operations are
//! not representable without bodies, so their exact intended signatures are
//! stated here and slice one implements them:
//!
//! ```text
//! impl BuildMeter {
//!     pub fn try_new(limits: BuildLimits) -> Result<Self, BuildError>;
//!     pub fn usage(&self) -> BuildUsage;
//! }
//!
//! impl Default for BuildLimits;
//! ```
//!
//! [`BuildMeter`] is neither `Clone` nor `Default`, its fields stay private,
//! and it exposes no way to reset or decrement a cumulative counter. A standing
//! client creates one meter per document; a client that emits a document in
//! segments reuses the same meter across every segment, which is what stops a
//! long run from resetting its own accounting between pieces.

use crate::units::BuildStepsUsed;
use crate::units::DocNodesUsed;
use crate::units::MaxBuildSteps;
use crate::units::MaxDocNodes;
use crate::units::MaxTextBytes;
use crate::units::MaxVerbatimLines;
use crate::units::TextBytesUsed;
use crate::units::VerbatimLinesUsed;

/// The ceilings a caller sets for one document build.
///
/// # Contract
/// - requires: each ceiling is the caller's chosen value; the defaults above
///   are what a caller gets by asking for none.
/// - ensures: a builder refuses rather than exceeding any of the four.
/// - provides: the complete build-phase budget, stated once.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BuildLimits
{
    /// The ceiling on stored document nodes, flatten images included.
    pub max_doc_nodes: MaxDocNodes,
    /// The ceiling on uniquely stored text and verbatim bytes.
    pub max_text_bytes: MaxTextBytes,
    /// The ceiling on stored verbatim physical fragments.
    pub max_verbatim_lines: MaxVerbatimLines,
    /// The ceiling on constructor and finalization steps.
    pub max_build_steps: MaxBuildSteps,
}

/// What one document build actually spent.
///
/// # Contract
/// - requires: the record is read from a meter that owns the counters.
/// - ensures: every field is monotone for the meter's whole lifetime.
/// - provides: an observation of build cost a caller can log, assert on, or
///   compare across runs.
/// - panics: none.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BuildUsage
{
    /// Document nodes stored, flatten images included.
    pub doc_nodes: DocNodesUsed,
    /// Uniquely stored text and verbatim bytes.
    pub text_bytes: TextBytesUsed,
    /// Stored verbatim physical fragments.
    pub verbatim_lines: VerbatimLinesUsed,
    /// Constructor and finalization steps consumed.
    pub build_steps: BuildStepsUsed,
}

/// The build-phase meter: the ceilings, and what has been spent against them.
///
/// One builder borrows one meter exclusively for its whole life, so there is
/// exactly one place a build charge can be recorded.
///
/// # Contract
/// - requires: the meter is constructed from a limit record and is borrowed by
///   at most one builder at a time.
/// - ensures: a charge is checked against its ceiling before the store grows,
///   and a refused charge leaves the counter unchanged.
/// - provides: the enforcement point for every build limit in the crate.
/// - panics: none.
#[derive(Debug)]
#[expect(
    dead_code,
    reason = "slice one reads these; the expectation fails as soon as it does"
)]
pub struct BuildMeter
{
    /// The ceilings this meter enforces.
    limits: BuildLimits,
    /// What has been spent against them.
    used: BuildUsage,
}
