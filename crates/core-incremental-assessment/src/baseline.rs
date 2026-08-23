//! The hand-rolled path's measured recheck.
//!
//! # What this wraps, and what it deliberately does not
//!
//! The baseline is `gandr-core-incremental`'s validated resume exactly as
//! built. Nothing here reimplements it, instruments it, or works around it: an
//! assessment that measured a reconstruction of the baseline would be comparing
//! the engine against this crate's opinion of the baseline instead of against
//! the baseline.
//!
//! What that costs is direct access to the baseline's internal work counts, and
//! the answer is to take them from what the result already reports rather than
//! from a private counter:
//!
//! - **items re-typed** is the item count less the adoption count, both of
//!   which the resume result reports.
//! - **items visited** is the length of the adoption decision sequence. The
//!   resume yields one adoption decision per item of the edited program, so
//!   every item was reached and classified — which is the visit. This is an
//!   assertion about a returned sequence, not an inference about a loop.
//! - **footprint rescanning** is measured by [`rescan_footprints`], which
//!   performs the same scan over the same items. The resume computes a
//!   footprint for every item of the edited program before its ordered pass
//!   begins (`checkpoint::resume_with` builds its `edited_footprints` vector
//!   over `edited.items`), so this is the same work, measured where it can be
//!   timed in isolation.
//!
//! # The append fast path is avoided on purpose
//!
//! The resume short-circuits an exact append without aligning or rescanning
//! anything. That path is real and worth having, and it is not what a mid-file
//! edit reaches. The workload edits the head of a middle chain precisely so the
//! measurement lands on the general path rather than on a fast path a
//! single-edit benchmark could wander into by accident.

use gandr_core_incremental::checkpoint::Checkpoints;
use gandr_core_incremental::checkpoint::ItemTyping;
use gandr_core_incremental::checkpoint::Resume;
use gandr_core_incremental::checkpoint::checkpoint_program;
use gandr_core_incremental::checkpoint::resume;
use gandr_core_incremental::footprint::footprint_of;
use gandr_core_incremental::persistence::encode_checkpoints;
use gandr_core_incremental::region::Program;

use crate::boundary::BoundaryByteCount;
use crate::boundary::ItemCount;
use crate::ledger::AssessmentError;

/// A checkpoint set plus the revision it was taken against — the baseline
/// path's driver.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BaselineSession
{
    /// The checkpoints the next recheck resumes from.
    checkpoints: Checkpoints,
}

impl BaselineSession
{
    /// Types `program` from scratch into the checkpoint set a later recheck
    /// resumes from.
    ///
    /// # Contract
    /// - ensures: returns a session holding one checkpoint per item of
    ///   `program`.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn install(program: &Program) -> Self
    {
        Self {
            checkpoints: checkpoint_program(program),
        }
    }

    /// Rechecks `edited` against the held checkpoints.
    ///
    /// Does not advance the session — [`Self::adopt`] does that — so a
    /// measurement can time the recheck without the bookkeeping that follows
    /// it.
    ///
    /// # Contract
    /// - ensures: returns the resume result for `edited`, whose typings equal
    ///   those a from-scratch re-type produces.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn recheck(
        &self,
        edited: &Program,
    ) -> Resume
    {
        resume(&self.checkpoints, edited)
    }

    /// Advances the session to a completed recheck's checkpoints.
    ///
    /// # Contract
    /// - ensures: the next recheck resumes from `resumed`'s checkpoints.
    /// - panics: none.
    #[inline]
    pub fn adopt(
        &mut self,
        resumed: Resume,
    )
    {
        self.checkpoints = resumed.into_checkpoints();
    }

    /// The held checkpoints' per-item typings, in source order.
    ///
    /// # Contract
    /// - ensures: returns one typing per held checkpoint, in source order.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn typings(&self) -> Vec<ItemTyping>
    {
        self.checkpoints
            .items
            .iter()
            .map(|checkpoint| checkpoint.typing.clone())
            .collect()
    }

    /// The encoded size of the held checkpoint set — the retained-state figure,
    /// taken with the same instrument the engine path's is.
    ///
    /// # Contract
    /// - ensures: returns the canonical encoding's length.
    /// - fails: returns the codec failure when a checkpoint has no
    ///   process-independent representation.
    /// - panics: none.
    ///
    /// # Errors
    ///
    /// Returns the codec failure when a checkpoint has no process-independent
    /// representation.
    #[inline]
    pub fn encoded_state_size(&self) -> Result<BoundaryByteCount, AssessmentError>
    {
        let bytes = encode_checkpoints(&self.checkpoints)?;
        Ok(BoundaryByteCount::from(bytes.len()))
    }
}

/// Types `program` from scratch and returns its per-item typings — the answer
/// every other path is compared against.
///
/// # Contract
/// - ensures: returns one typing per item, in source order, computed with no
///   reuse of any kind.
/// - panics: none.
#[inline]
#[must_use]
pub fn from_scratch(program: &Program) -> Vec<ItemTyping>
{
    checkpoint_program(program)
        .items
        .iter()
        .map(|checkpoint| checkpoint.typing.clone())
        .collect()
}

/// Scans every item's dependency footprint, returning how many were scanned.
///
/// This is the per-recheck rescan the validated resume performs on its general
/// path, measured where it can be timed on its own. Its cost is a function of
/// the whole program's size rather than of the dirty set's, which is the shape
/// the engine path is being asked to improve on.
///
/// # Contract
/// - ensures: returns the number of items scanned, which is every item of
///   `program`.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: the scan must actually traverse each item rather than being
///   optimized away, since its cost is one of the measured quantities; a count
///   that did not depend on the traversal would report the same number for a
///   scan that never ran.
/// - witness: `support::rescan_visits_every_item`
#[inline]
#[must_use]
pub fn rescan_footprints(program: &Program) -> ItemCount
{
    let mut scanned: usize = 0;
    for item in &program.items {
        // The scan's result is deliberately kept opaque to the optimizer: the
        // measured quantity is the traversal, and a scan whose result is unused
        // is a scan the compiler may decline to perform.
        let footprint = std::hint::black_box(footprint_of(item));
        drop(footprint);
        scanned = scanned.saturating_add(1);
    }
    ItemCount::from(scanned)
}
