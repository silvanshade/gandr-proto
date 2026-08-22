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
//! # The child index base does not arise here, and that is the answer
//!
//! The question [`crate::value::index_base`] poses is what a child reference
//! is an offset *from*, and it presupposes that a child reference is an index.
//! **This token stream has no child indices.** A constructor's children are
//! emitted nested, in place, between its open and close records, so there is
//! no table and no numbering — and therefore nothing that an early insertion
//! could renumber.
//!
//! That is not a gap in the encoding. It is the property chunk-local bases
//! were proposed to *recover*, obtained by construction instead: an insertion
//! early in a value rewrites the chunks on its own path and leaves every
//! sibling chunk byte-identical, because a sibling's bytes never mentioned a
//! position that moved.
//!
//! So [`cam_commit`] **refuses** [`ChildIndexBase::ChunkLocal`] rather than
//! silently ignoring it. Accepting it would let a manifest claim a
//! representation that does not exist, and a claim about a representation is
//! exactly the thing this plane's manifest is for.
//!
//! The question survives for the *artifact export format*, whose child
//! references really are absolute table indices; nothing here answers that,
//! and nothing here needs it answered.
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

use gandr_storage_chunker::BoundaryEvent;
use gandr_storage_chunker::CutDecision;
use gandr_storage_chunker::TokenCount;
use gandr_storage_chunker::TypedChunker;
use gandr_storage_chunker::TypedChunkerParams;

use crate::error::ValueError;
use crate::transport::CanonicalU64;
use crate::value::chunk::ChunkStore;
use crate::value::chunk::StoredChunkRef;
use crate::value::chunk::frame_chunk;
use crate::value::index_base::ChildIndexBase;
use crate::value::ptr::ChunkDigest;
use crate::value::ptr::ContentPtr;
use crate::value::ptr::TokenOffset;
use crate::value::tokens::CanonicalValue;
use crate::value::tokens::ConstructorTag;
use crate::value::tokens::TOKEN_BYTES;
use crate::value::tokens::TOKEN_CHILD;
use crate::value::tokens::TOKEN_CLOSE;
use crate::value::tokens::TOKEN_OPEN;
use crate::value::tokens::TOKEN_WORD;
use crate::value::tokens::TokenSink;
use crate::value::units::ChunkBody;
use crate::value::units::ChunkImage;
use crate::value::units::TokenBytes;

/// The encoded records the traversal has emitted and not yet framed.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct PendingBody(Vec<u8>);

/// A byte offset into the pending body where a subtree begins.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BodyStart(usize);

/// A record index in the pending body where a subtree begins.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TokenStart(u32);

/// The rolling hash of one subtree, as the chunker reads it.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Residue(u64);

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
struct OpenFrame
{
    /// Rolling hash over the tokens of the subtree rooted here.
    residue: Residue,
    /// Tokens emitted inside this subtree since it opened.
    tokens: u64,
    /// Byte offset in the pending body where this subtree's first token sits.
    body_start: BodyStart,
    /// Token index in the pending body where this subtree's first token sits.
    token_start: TokenStart,
}

/// The committing traversal's sink: a pending body, a frame stack, a chunker.
///
/// It is deliberately not public. A caller that could drive the sink directly
/// could emit a chunk wrapper by hand, and a wrapper the traversal did not
/// place is a claim about the store that nothing checked — which is the one
/// way a well-formed chunk DAG can be built over a chunk that is not there.
#[derive(Debug)]
struct CommitSink<'store, Store>
where
    Store: ChunkStore + ?Sized,
{
    /// Where committed chunks are inserted, bottom-up.
    store: &'store mut Store,
    /// The cut-decision engine under the committed typed profile.
    chunker: TypedChunker,
    /// Encoded token records not yet framed into a chunk.
    pending: PendingBody,
    /// One entry per constructor currently open, outermost first.
    open: Vec<OpenFrame>,
    /// Token records written into `pending` since the last cut.
    pending_tokens: u32,
}

impl<'store, Store> CommitSink<'store, Store>
where
    Store: ChunkStore + ?Sized,
{
    /// Opens a sink over a store under a committed profile.
    #[inline]
    fn new(
        store: &'store mut Store,
        params: &TypedChunkerParams,
    ) -> Self
    {
        return Self {
            store,
            chunker: TypedChunker::new(params.clone()),
            pending: PendingBody(Vec::new()),
            open: Vec::new(),
            pending_tokens: 0_u32,
        };
    }

    /// Accounts one emitted record against every open constructor.
    #[inline]
    fn account(&mut self)
    {
        self.pending_tokens = self.pending_tokens.saturating_add(1_u32);
        for frame in &mut self.open {
            frame.tokens = frame.tokens.saturating_add(1_u64);
        }
    }

    /// The rolling residue of the byte range a subtree occupies.
    ///
    /// BLAKE3 over the subtree's own canonical bytes, truncated to sixty-four
    /// bits. Chosen over a cheaper rolling mix because the cut positions it
    /// decides are part of the protocol: a mixing function is a constant two
    /// deployments must agree on, and this one is already bound as the digest
    /// family rather than needing a second commitment of its own.
    #[inline]
    fn residue_from(
        &self,
        body_start: BodyStart,
    ) -> Residue
    {
        let body_start = body_start.0;
        let Some(subtree) = self.pending.0.get(body_start ..)
        else {
            return Residue(0_u64);
        };
        let digest = blake3::hash(subtree);
        let bytes = digest.as_bytes();
        let Some(head) = bytes.get(.. 8_usize)
        else {
            return Residue(0_u64);
        };
        let Ok(image) = <[u8; 8]>::try_from(head)
        else {
            return Residue(0_u64);
        };
        return Residue(u64::from_be_bytes(image));
    }

    /// Cuts the pending suffix from `body_start` into a stored chunk and
    /// splices a child record in its place.
    #[inline]
    fn cut(
        &mut self,
        body_start: BodyStart,
        token_start: TokenStart,
    ) -> Result<(), ValueError>
    {
        let BodyStart(body_start) = body_start;
        let TokenStart(token_start) = token_start;
        let Some(subtree) = self.pending.0.get(body_start ..)
        else {
            return Ok(());
        };
        if subtree.is_empty() {
            return Ok(());
        }
        let token_count =
            TokenCount::from(u64::from(self.pending_tokens.saturating_sub(token_start)));
        let (digest, image) = frame_chunk(ChunkBody::from(subtree), token_count)?;
        self.store.insert(StoredChunkRef::new(
            digest,
            ChunkImage::from(image.as_ref()),
        ))?;
        self.pending.0.truncate(body_start);
        self.pending_tokens = token_start;
        // The wrapper addresses the chunk's own root, so its offset is zero
        // under either index base: a chunk-local base changes what a child
        // reference INSIDE a body means, never what a seam wrapper points at.
        push_child(&mut self.pending, digest);
        self.account();
        return Ok(());
    }

    /// Frames whatever remains pending as the root chunk.
    #[inline]
    fn finish(&mut self) -> Result<ContentPtr, ValueError>
    {
        let token_count = TokenCount::from(u64::from(self.pending_tokens));
        let (digest, image) = frame_chunk(ChunkBody::from(self.pending.0.as_slice()), token_count)?;
        self.store.insert(StoredChunkRef::new(
            digest,
            ChunkImage::from(image.as_ref()),
        ))?;
        return Ok(ContentPtr::new(digest, TokenOffset::from(0_u32)));
    }
}

impl<Store> TokenSink for CommitSink<'_, Store>
where
    Store: ChunkStore + ?Sized,
{
    #[inline]
    fn open(
        &mut self,
        tag: ConstructorTag,
    ) -> Result<(), ValueError>
    {
        self.open.push(OpenFrame {
            residue: Residue(0_u64),
            tokens: 0_u64,
            body_start: BodyStart(self.pending.0.len()),
            token_start: TokenStart(self.pending_tokens),
        });
        self.pending.0.push(TOKEN_OPEN);
        self.pending.0.push(u8::from(tag));
        self.account();
        return Ok(());
    }

    #[inline]
    fn word(
        &mut self,
        value: CanonicalU64,
    ) -> Result<(), ValueError>
    {
        self.pending.0.push(TOKEN_WORD);
        self.pending
            .0
            .extend_from_slice(&u64::from(value).to_be_bytes());
        self.account();
        return Ok(());
    }

    #[inline]
    fn bytes(
        &mut self,
        bytes: TokenBytes<'_>,
    ) -> Result<(), ValueError>
    {
        let payload = bytes.as_ref();
        let len = u64::try_from(payload.len()).map_err(|_width| ValueError::WidthOverflow {
            found: u64::MAX,
            width: 0x40_u32,
        })?;
        self.pending.0.push(TOKEN_BYTES);
        self.pending.0.extend_from_slice(&len.to_be_bytes());
        self.pending.0.extend_from_slice(payload);
        self.account();
        return Ok(());
    }

    #[inline]
    fn child_pointer(
        &mut self,
        pointer: ContentPtr,
    ) -> Result<(), ValueError>
    {
        self.pending.0.push(TOKEN_CHILD);
        self.pending.0.extend_from_slice(pointer.digest().as_ref());
        self.pending
            .0
            .extend_from_slice(&u32::from(pointer.offset()).to_be_bytes());
        self.account();
        return Ok(());
    }

    #[inline]
    fn close(&mut self) -> Result<(), ValueError>
    {
        self.pending.0.push(TOKEN_CLOSE);
        self.account();
        let Some(mut frame) = self.open.pop()
        else {
            return Ok(());
        };
        frame.residue = self.residue_from(frame.body_start);
        let decision = self.chunker.on_boundary(BoundaryEvent {
            tokens_since: frame.tokens,
            residue: frame.residue.0,
        });
        // The OUTERMOST close never cuts. Cutting there would frame the whole
        // remaining value as a chunk and then leave `finish` framing a second
        // chunk holding nothing but a wrapper for it -- one extra block, one
        // extra indirection, and a root whose only child is the entire value.
        // The root chunk is framed by `finish` in any case, so the outermost
        // boundary has nothing left to separate.
        if self.open.is_empty() {
            return Ok(());
        }
        if matches!(decision, CutDecision::Cut(_)) {
            self.cut(frame.body_start, frame.token_start)?;
        }
        return Ok(());
    }
}

/// Writes one child record for a chunk's own root.
#[inline]
fn push_child(
    pending: &mut PendingBody,
    digest: ChunkDigest,
)
{
    let pending = &mut pending.0;
    pending.push(TOKEN_CHILD);
    pending.extend_from_slice(digest.as_ref());
    pending.extend_from_slice(&0_u32.to_be_bytes());
}

/// Commits a value into a store as a chunk DAG and returns its root pointer.///
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
    // The traversal does not consult the index base, and that is the answer to
    // the question rather than an omission — see the module note above.
    if matches!(index_base, ChildIndexBase::ChunkLocal) {
        return Err(ValueError::UnsupportedIndexBase);
    }
    let mut sink = CommitSink::new(store, params);
    value.emit_tokens(&mut sink)?;
    return sink.finish();
}
