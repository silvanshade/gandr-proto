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

use crate::error::BuildError;
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

/// The build-phase meter: ceilings and cumulative usage for one document.
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
pub struct BuildMeter
{
    /// The ceilings this meter enforces.
    limits: BuildLimits,
    /// What has been spent against them.
    used: BuildUsage,
}

impl Default for BuildLimits
{
    #[inline]
    fn default() -> Self
    {
        Self {
            max_doc_nodes: MaxDocNodes::from(1_000_000u32),
            max_text_bytes: MaxTextBytes::from(0x0400_0000_usize),
            max_verbatim_lines: MaxVerbatimLines::from(1_000_000u32),
            max_build_steps: MaxBuildSteps::from(20_000_000u64),
        }
    }
}

impl BuildMeter
{
    /// Creates a meter with zero usage under `limits`.
    ///
    /// # Contract
    /// - requires: `limits` contains the caller's four build ceilings.
    /// - ensures: all usage counters start at zero and remain cumulative.
    /// - provides: an exclusive accounting authority for one document build.
    /// - fails: this operation has no fallible path; the result type keeps the
    ///   constructor symmetric with the render-phase meter.
    /// - panics: none.
    ///
    /// # Errors
    /// This constructor currently cannot fail because the limit record contains
    /// only finite scalar values.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — a new meter starts all four cumulative counters at
    ///   zero and accepts the builder's singleton baseline.
    /// - witness: `algebra::empty_emits_nothing_and_moves_no_column`.
    #[inline]
    #[must_use = "a build meter must be retained for the document build"]
    pub fn try_new(limits: BuildLimits) -> Result<Self, BuildError>
    {
        Ok(Self {
            limits,
            used: BuildUsage {
                doc_nodes: DocNodesUsed::from(0u64),
                text_bytes: TextBytesUsed::from(0u64),
                verbatim_lines: VerbatimLinesUsed::from(0u64),
                build_steps: BuildStepsUsed::from(0u64),
            },
        })
    }

    /// Returns the cumulative usage observed by this meter.
    ///
    /// # Contract
    /// - requires: the meter remains alive and exclusively owned by its build.
    /// - ensures: the returned snapshot is a copy and does not reset usage.
    /// - provides: monotone node, byte, fragment, and step observations.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — usage snapshots are cumulative and do not reset the
    ///   meter between observations.
    /// - witness: `algebra::build_usage_is_monotone_across_a_whole_document`.
    #[inline]
    #[must_use]
    pub fn usage(&self) -> BuildUsage
    {
        self.used
    }

    /// Checks whether one document node can be charged without changing usage.
    ///
    /// # Contract
    /// - requires: the caller is about to store one document node.
    /// - ensures: success proves the next node charge fits its counter and
    ///   ceiling.
    /// - provides: an atomic preflight for compound builder operations.
    /// - fails: reports counter overflow or the node ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for counter overflow or `LimitExceeded` at
    /// the configured node ceiling.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — the node preflight accepts the exact node boundary
    ///   and leaves usage unchanged when the next charge is refused.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::a_refused_charge_leaves_the_counter_unchanged`.
    #[inline]
    pub(crate) fn check_doc_node(&self) -> Result<(), BuildError>
    {
        self.used
            .doc_nodes
            .checked_charge(self.limits.max_doc_nodes)
            .map(|_| ())
    }

    /// Checks whether new text bytes can be charged without changing usage.
    ///
    /// # Contract
    /// - requires: `amount` is the byte count of a new stored identity.
    /// - ensures: success proves the byte charge fits its counter and ceiling.
    /// - provides: an atomic preflight for text and verbatim insertion.
    /// - fails: reports conversion, counter, or configured-limit overflow.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` when the amount or counter is not
    /// representable, or `LimitExceeded` at the configured byte ceiling.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — text preflight accepts the exact byte boundary and
    ///   leaves usage unchanged after refusal.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::a_refused_charge_leaves_the_counter_unchanged`.
    #[inline]
    pub(crate) fn check_text_bytes(
        &self,
        amount: TextBytesUsed,
    ) -> Result<(), BuildError>
    {
        self.used
            .text_bytes
            .checked_charge(amount, self.limits.max_text_bytes)
            .map(|_| ())
    }

    /// Checks whether new verbatim fragments can be charged without changing
    /// usage.
    ///
    /// # Contract
    /// - requires: `amount` is the complete scan count for one new verbatim.
    /// - ensures: success proves the fragment charge fits its counter and
    ///   ceiling.
    /// - provides: an atomic preflight for verbatim insertion.
    /// - fails: reports counter overflow or the configured fragment ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for cumulative overflow or `LimitExceeded`
    /// at the configured fragment ceiling.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — verbatim preflight counts the complete scan and
    ///   refuses only the charge beyond its exact fragment ceiling.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::verbatim_with_a_trailing_ending_stores_an_empty_final_fragment`.
    #[inline]
    pub(crate) fn check_verbatim_lines(
        &self,
        amount: VerbatimLinesUsed,
    ) -> Result<(), BuildError>
    {
        self.used
            .verbatim_lines
            .checked_charge(amount, self.limits.max_verbatim_lines)
            .map(|_| ())
    }

    /// Checks whether one build step can be charged without changing usage.
    ///
    /// # Contract
    /// - requires: the caller has identified one checked constructor or
    ///   finalization operation.
    /// - ensures: success proves the next step fits its counter and ceiling.
    /// - provides: an atomic preflight for compound builder operations.
    /// - fails: reports counter overflow or the configured step ceiling.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` for cumulative overflow or `LimitExceeded`
    /// at the configured step ceiling.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — step preflight accepts the exact work ceiling and
    ///   refuses only the next checked operation.
    /// - witness: `algebra::each_build_ceiling_refuses_exactly_at_its_boundary`.
    /// - witness: `algebra::every_finalization_visit_edge_and_probe_charges_a_build_step`.
    #[inline]
    pub(crate) fn check_step(&self) -> Result<(), BuildError>
    {
        self.used
            .build_steps
            .checked_charge(self.limits.max_build_steps)
            .map(|_| ())
    }

    /// Charges one stored document node after a successful preflight.
    ///
    /// # Contract
    /// - requires: [`Self::check_doc_node`] succeeded without an intervening
    ///   charge.
    /// - ensures: usage increases exactly once.
    /// - provides: node-limit accounting for original and flattened images.
    /// - fails: reports the same typed errors as the preflight if state
    ///   changed.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` or `LimitExceeded` if the precondition was
    /// not maintained.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a successful node preflight is consumed exactly once
    ///   and a refused charge does not mutate usage.
    /// - witness: `algebra::a_second_edge_to_a_shared_handle_charges_no_new_node`.
    /// - witness: `algebra::a_refused_charge_leaves_the_counter_unchanged`.
    #[inline]
    pub(crate) fn charge_doc_node(&mut self) -> Result<(), BuildError>
    {
        self.used.doc_nodes = self
            .used
            .doc_nodes
            .checked_charge(self.limits.max_doc_nodes)?;
        Ok(())
    }

    /// Charges new text and verbatim bytes after a successful preflight.
    ///
    /// # Contract
    /// - requires: [`Self::check_text_bytes`] succeeded without an intervening
    ///   charge.
    /// - ensures: usage increases exactly by `amount`.
    /// - provides: cumulative byte accounting.
    /// - fails: reports the same typed errors as the preflight if state
    ///   changed.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` or `LimitExceeded` if the precondition was
    /// not maintained.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a successful byte preflight consumes exactly the
    ///   requested amount and a refusal preserves the previous usage.
    /// - witness: `algebra::a_second_edge_to_a_shared_handle_charges_no_new_text_bytes`.
    /// - witness: `algebra::a_refused_charge_leaves_the_counter_unchanged`.
    #[inline]
    pub(crate) fn charge_text_bytes(
        &mut self,
        amount: TextBytesUsed,
    ) -> Result<(), BuildError>
    {
        self.used.text_bytes = self
            .used
            .text_bytes
            .checked_charge(amount, self.limits.max_text_bytes)?;
        Ok(())
    }

    /// Charges scanned verbatim fragments after a successful preflight.
    ///
    /// # Contract
    /// - requires: [`Self::check_verbatim_lines`] succeeded without an
    ///   intervening charge.
    /// - ensures: usage increases exactly by `amount`.
    /// - provides: cumulative physical-fragment accounting.
    /// - fails: reports the same typed errors as the preflight if state
    ///   changed.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` or `LimitExceeded` if the precondition was
    /// not maintained.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a successful fragment preflight consumes exactly the
    ///   scan count, including its final fragment.
    /// - witness: `algebra::verbatim_with_a_trailing_ending_stores_an_empty_final_fragment`.
    /// - witness: `algebra::a_refused_charge_leaves_the_counter_unchanged`.
    #[inline]
    pub(crate) fn charge_verbatim_lines(
        &mut self,
        amount: VerbatimLinesUsed,
    ) -> Result<(), BuildError>
    {
        self.used.verbatim_lines = self
            .used
            .verbatim_lines
            .checked_charge(amount, self.limits.max_verbatim_lines)?;
        Ok(())
    }

    /// Charges one checked constructor or finalization step after preflight.
    ///
    /// # Contract
    /// - requires: [`Self::check_step`] succeeded without an intervening
    ///   charge.
    /// - ensures: usage increases exactly once.
    /// - provides: cumulative build-work accounting.
    /// - fails: reports the same typed errors as the preflight if state
    ///   changed.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns `ArithmeticOverflow` or `LimitExceeded` if the precondition was
    /// not maintained.
    ///
    /// # Adequacy
    /// - hypothesis: L3 — a successful step preflight consumes exactly one
    ///   unit, including finalization work.
    /// - witness: `algebra::every_finalization_visit_edge_and_probe_charges_a_build_step`.
    /// - witness: `algebra::a_refused_charge_leaves_the_counter_unchanged`.
    #[inline]
    pub(crate) fn charge_step(&mut self) -> Result<(), BuildError>
    {
        self.used.build_steps = self
            .used
            .build_steps
            .checked_charge(self.limits.max_build_steps)?;
        Ok(())
    }
}
