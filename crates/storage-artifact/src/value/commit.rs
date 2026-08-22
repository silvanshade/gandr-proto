//! `cam_commit` — the typed chunking traversal that commits bottom-up.
//!
//! The traversal walks a value once, in preorder, accumulating tokens into a
//! pending body. At every constructor exit whose tag is boundary-classified it
//! raises a [`gandr_storage_chunker::BoundaryEvent`] and asks the committed
//! typed profile whether to cut. A cut frames the pending suffix as a chunk,
//! stores it, and splices a chunk wrapper into the parent's stream in its
//! place. The root chunk is framed last, and its digest with offset zero is
//! the returned [`ContentPtr`].
//!
//! # Bottom-up is a correctness property, not an optimization
//!
//! A child's digest must be fixed before the parent's body can contain it, or
//! the parent's own digest would depend on a value it cannot yet name. Cutting
//! at constructor **exit** is exactly what makes the order available: by the
//! time a subtree closes, everything beneath it has already been decided.
//!
//! # What the commitment binds, and why it is not optional
//!
//! Kappa, the cap, the hash family and the codec configuration all change the
//! chunk decomposition, and therefore every digest. They are protocol
//! constants: two deployments that disagree on any of them produce different
//! addresses for the same value and share nothing. They are bound into
//! [`crate::value::value_manifest::ValueManifest`] so that a disagreement is a
//! refusal rather than a silent failure to deduplicate.

use gandr_storage_chunker::TypedChunkerParams;

use crate::error::ValueError;
use crate::value::chunk::ChunkStore;
use crate::value::index_base::ChildIndexBase;
use crate::value::ptr::ContentPtr;
use crate::value::tokens::CanonicalValue;

/// Commits a value into a store as a chunk DAG and returns its root pointer.
///
/// # Contract
/// - requires: `params` is the committed typed profile the reader will use;
///   `index_base` is the committed child-reference representation.
/// - ensures: `Ok(pointer)` with every chunk the traversal cut present in
///   `store` under its own digest, and the returned pointer addressing the root
///   chunk at token offset zero. Committing is a **deterministic function of
///   the value, the profile and the index base**: the same value under the same
///   constants commits to the same pointer, and two values sharing a subtree
///   share the chunks that subtree was cut into.
/// - provides: the value plane's write path, and the structural sharing every
///   later rung keys on.
/// - fails: [`ValueError`] from the sink, the framing, or the store.
/// - panics: none.
///
/// # Errors
/// [`ValueError`].
#[inline]
#[expect(
    clippy::todo,
    reason = "gandr-8tou.4 scaffold: the committing traversal is the implementor deliverable"
)]
pub fn cam_commit<Store, Value>(
    store: &mut Store,
    params: &TypedChunkerParams,
    index_base: ChildIndexBase,
    value: &Value,
) -> Result<ContentPtr, ValueError>
where
    Store: ChunkStore + ?Sized,
    Value: CanonicalValue,
{
    todo!(
        "walk {value:p} through a CommitSink over {store:p} under kappa {} cap {} and {index_base:?}",
        params.kappa,
        params.cap
    );
}
