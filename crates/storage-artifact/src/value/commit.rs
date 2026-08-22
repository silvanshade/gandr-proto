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

use alloc::vec::Vec;

use gandr_storage_chunker::TypedChunker;
use gandr_storage_chunker::TypedChunkerParams;

use crate::error::ValueError;
use crate::value::chunk::ChunkStore;
use crate::value::index_base::ChildIndexBase;
use crate::value::ptr::ContentPtr;
use crate::value::tokens::CanonicalValue;

/// One open constructor's accounting while the traversal is inside it.
///
/// The stack of these is the whole state the walk needs, and each field is
/// here because the walk cannot recompute it:
///
/// - the **rolling residue** is over the subtree rooted at this constructor,
///   and a boundary event needs it at *exit*, which is the only moment the
///   subtree is complete;
/// - the **token count** is what the chunker's cap fires on, and it counts this
///   subtree's tokens rather than the whole body's, because a cut resets the
///   pending suffix and not the enclosing constructors;
/// - the **body start** marks where in the pending body this subtree begins,
///   which is what a cut needs in order to frame exactly the suffix and splice
///   a wrapper in its place.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[expect(
    dead_code,
    reason = "gandr-8tou.4 scaffold: the traversal that drives this state is the implementor deliverable"
)]
struct OpenFrame
{
    /// Rolling hash over the tokens of the subtree rooted here.
    residue: u64,
    /// Tokens emitted inside this subtree since it opened.
    tokens: u64,
    /// Byte offset in the pending body where this subtree's first token sits.
    body_start: usize,
    /// Token index in the pending body where this subtree's first token sits.
    token_start: u32,
}

/// The committing traversal's sink: a pending body, a frame stack, a chunker.
///
/// It is deliberately not public. A caller that could drive the sink directly
/// could emit a chunk wrapper by hand, and a wrapper the traversal did not
/// place is a claim about the store that nothing checked — which is the one
/// way a well-formed chunk DAG can be built over a chunk that is not there.
#[derive(Debug)]
#[expect(
    dead_code,
    reason = "gandr-8tou.4 scaffold: the traversal that drives this state is the implementor deliverable"
)]
struct CommitSink<'store, Store>
where
    Store: ChunkStore + ?Sized,
{
    /// Where committed chunks are inserted, bottom-up.
    store: &'store mut Store,
    /// The cut-decision engine under the committed typed profile.
    chunker: TypedChunker,
    /// The committed child-reference representation.
    index_base: ChildIndexBase,
    /// Encoded token records not yet framed into a chunk.
    pending: Vec<u8>,
    /// One entry per constructor currently open, outermost first.
    open: Vec<OpenFrame>,
    /// Token records written into `pending` since the last cut.
    pending_tokens: u32,
}

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
