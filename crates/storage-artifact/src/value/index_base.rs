//! The child-reference representation, and the evaluation that chooses it.
//!
//! # The question, stated so it can be answered rather than argued
//!
//! gandr's export format is *maximally wrapped*: every child reference is an
//! index into a table, never an inline subterm. Those indices are **absolute**
//! today — a position in the whole artifact. Absolute indices have a property
//! that is invisible until content addressing arrives and then dominates
//! everything: inserting one constructor early in a value **renumbers every
//! index after it**. Not one chunk changes; every downstream chunk changes,
//! because every one of them mentions indices that moved. Structural sharing
//! across two versions of the same value collapses to the prefix before the
//! edit.
//!
//! [`ChildIndexBase::ChunkLocal`] is the alternative: a child reference is an
//! offset **from the start of the chunk that carries it**, and a boundary
//! wrapper at each chunk seam carries the base the next chunk is relative to.
//! An edit then renumbers only within its own chunk, and the seams absorb the
//! shift, so chunks downstream of the edit stay byte-identical and keep
//! sharing.
//!
//! # Why this is decided by measurement and not by the argument above
//!
//! The argument says chunk-local bases *can* recover sharing; it does not say
//! by how much on gandr's actual values, and it does not price the seam
//! wrappers. Chunk-local bases cost a wrapper per seam and an addition per
//! dereference. A representation that recovers sharing on a corpus that never
//! edits early is a cost with no benefit. So the rung's exit is a
//! **measurement** — [`IndexBaseMeasurement`] over a real corpus of edits —
//! and the ruling follows the number.
//!
//! # The cost of choosing is bounded, and that is why it is chosen now
//!
//! Nothing is released. The whole price of adopting chunk-local bases is a
//! format-version bump and regenerated goldens. The price of deferring is
//! paid by every consumer that keys on a content pointer in the meantime,
//! because re-keying them later means re-keying them against a format that
//! moved underneath. The evaluation belongs to this rung for that reason.
//!
//! The 32-byte-per-child alternative — every child reference a full digest —
//! stays rejected on size grounds and is recorded here so it is not
//! rediscovered as new.

/// How a child reference names its target inside a chunk body.
///
/// The mode is bound into the codec commitment, so two deployments that
/// disagree cannot silently produce different addresses for the same value:
/// they produce different manifests and refuse each other.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChildIndexBase
{
    /// A child reference is an index into the whole value's table.
    ///
    /// What the export format does today. Simple, and it loses cross-version
    /// sharing under suffix renumbering.
    Absolute,
    /// A child reference is an offset from the start of its own chunk, with a
    /// boundary wrapper at each seam carrying the base.
    ///
    /// Confines renumbering to the edited chunk at the cost of one wrapper per
    /// seam and one addition per dereference.
    ChunkLocal,
}

/// One corpus measurement of a single edit under one representation.
///
/// The fields are deliberately raw counts rather than a ratio: a ratio hides
/// which of the two moved, and the whole point of the measurement is to see
/// the sharing recovered *and* the seam cost paid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexBaseMeasurement
{
    /// The representation this measurement was taken under.
    pub base: ChildIndexBase,
    /// The depth of the edited path.
    pub edit_depth: u32,
    /// Chunks whose bytes differ between the two versions.
    pub chunks_changed: u64,
    /// Chunks byte-identical between the two versions, and so shared.
    pub chunks_shared: u64,
    /// Total bytes the seam wrappers cost across the committed value.
    pub seam_wrapper_bytes: u64,
}

/// The evaluation's verdict for one corpus.
///
/// Carrying the two measurements beside the verdict is the point: a verdict
/// without its numbers is an opinion, and the next rung has to be able to
/// re-derive this one rather than inherit it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndexBaseVerdict
{
    /// What the measurement says the format should commit to.
    pub adopted: ChildIndexBase,
    /// The measurement taken under [`ChildIndexBase::Absolute`].
    pub absolute: IndexBaseMeasurement,
    /// The measurement taken under [`ChildIndexBase::ChunkLocal`].
    pub chunk_local: IndexBaseMeasurement,
}
