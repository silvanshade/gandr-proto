//! The locality measurement, and the bound it is read against.
//!
//! # What is being measured
//!
//! Editing one node at depth $d$ in a committed value forces every chunk on
//! the path from that node to the root to be re-cut, because each of them
//! contains a digest that moved. Nothing else needs to change. The adopted
//! storage theory bounds the expected number of affected chunks by
//!
//! ```text
//! 2 * (d / kappa + 1) + ceil(d / (c * kappa))
//! ```
//!
//! with unchanged sibling subtrees costing at most `kappa - 1` spillover
//! constructors each. The cap term is the second summand, and `c` is the cap
//! multiplier the profile was committed under.
//!
//! # Why this is a measurement rather than a test
//!
//! The bound is an expectation over the rolling hash, not a worst case, so a
//! single edit can exceed it without refuting anything. What refutes the
//! implementation is a **distribution** that sits above the bound, and what
//! confirms it is a distribution that sits below. So the artifact this module
//! produces is a recorded distribution over a corpus of edits, read against
//! the bound — and the same run yields the structural-sharing numbers, because
//! they are the same observation counted differently.
//!
//! # The sharing benchmark rides here, once
//!
//! Changed leaf count, affected ancestor count, and hash-equal unchanged
//! subtrees are exactly the three numbers a structural-sharing benchmark
//! reports, and they fall out of the locality run for free. Measuring them
//! twice would risk measuring them differently.

use gandr_storage_chunker::TypedChunkerParams;

use crate::value::units::CapMultiplier;
use crate::value::units::ChunkCount;
use crate::value::units::EditDepth;

/// One edit's observed locality and sharing numbers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalityMeasurement
{
    /// The depth of the edited path.
    pub edit_depth: u32,
    /// Chunks re-cut as a consequence of the edit.
    pub chunks_affected: u64,
    /// Leaf chunks whose bytes changed.
    pub changed_leaves: u64,
    /// Ancestor chunks re-cut because a child digest moved.
    pub affected_ancestors: u64,
    /// Subtrees whose chunks are byte-identical across the edit, and so shared.
    pub hash_equal_subtrees: u64,
}

/// The expected-chunk bound for one edit depth under a committed profile.
///
/// # Contract
/// - requires: `params` is the committed profile the value was cut under, and
///   `cap_multiplier` is the `c` relating its cap to its kappa.
/// - ensures: the returned value is `2 * (depth / kappa + 1) + ceil(depth /
///   (cap_multiplier * kappa))` over the profile's own kappa, computed without
///   overflow or truncation.
/// - provides: the number a measured distribution is read against, so that
///   reading it is not an exercise in remembering the formula.
/// - fails: [`None`] when the arithmetic would overflow the width, never a
///   wrapped or saturated bound — a silently wrong bound would make every
///   comparison against it meaningless.
/// - panics: none.
#[inline]
#[must_use]
#[expect(
    clippy::todo,
    reason = "gandr-8tou.4 scaffold: the checked bound arithmetic is the implementor deliverable"
)]
pub fn expected_chunk_bound(
    depth: EditDepth,
    params: &TypedChunkerParams,
    cap_multiplier: CapMultiplier,
) -> Option<ChunkCount>
{
    todo!(
        "checked 2*(d/kappa+1)+ceil(d/(c*kappa)) for {depth:?}, kappa {}, {cap_multiplier:?}",
        params.kappa
    );
}
