//! The chunk image, its digest domain, and the store that holds chunks.
//!
//! A chunk is a **framed token body**, and the frame is inside the hashed
//! preimage. That single decision is what lets one backing object serve both
//! storage planes: a value chunk carries [`VALUE_CHUNK_MAGIC`] where a prolly
//! node carries a node header, so no byte string is ever a candidate for both
//! validators and no caller can hand one to the wrong one.
//!
//! # The framing
//!
//! ```text
//! image := VALUE_CHUNK_MAGIC
//!       || u16be  chunk format version
//!       || u64be  token count
//!       || u64be  body length in bytes
//!       || body
//! digest := BLAKE3(image)
//! ```
//!
//! Every integer is big-endian and every width is fixed, for the reason
//! [`crate::transport`] states at length: a canonical identity may not inherit
//! a memory encoding's endianness or a target's pointer width.
//!
//! # Verification is on both sides
//!
//! [`ChunkStore::insert`] and [`ChunkStore::load`] both recompute the digest
//! and both re-read the frame. A store that verified only on insert would let
//! bit rot, a bad backend, or a hostile peer return bytes under a digest they
//! do not hash to — and a content-addressed plane whose reads are unchecked is
//! not content-addressed at all, it merely names things after their contents.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;

use gandr_storage_chunker::TokenCount;

use crate::error::ValueError;
use crate::value::ptr::ChunkDigest;
use crate::value::units::ChunkBody;
use crate::value::units::ChunkCount;
use crate::value::units::ChunkImage;
use crate::value::units::ChunkImageBuf;

/// Domain-separation magic for a value-plane chunk image.
///
/// The format version is pinned in the magic, exactly as
/// [`crate::manifest::MANIFEST_MAGIC`] and
/// [`crate::transport::TRANSPORT_STEP_MAGIC`] pin theirs.
pub const VALUE_CHUNK_MAGIC: &[u8] = b"gandr:value-chunk:v1";

/// The chunk image layout version carried inside the frame.
pub const CHUNK_FORMAT_VERSION_V1: u16 = 1;

/// A borrowed chunk image and the digest it is claimed to hash to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoredChunkRef<'image>
{
    /// The digest the caller claims for `image`.
    digest: ChunkDigest,
    /// The framed chunk image bytes.
    image: ChunkImage<'image>,
}

impl<'image> StoredChunkRef<'image>
{
    /// Pairs a claimed digest with a framed chunk image.
    ///
    /// # Contract
    /// - requires: nothing; the claim is what the store checks.
    /// - ensures: the reference carries both unchanged.
    /// - provides: the store's argument and return shape.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn new(
        digest: ChunkDigest,
        image: ChunkImage<'image>,
    ) -> Self
    {
        return Self { digest, image };
    }

    /// Returns the claimed digest.
    #[inline]
    #[must_use]
    pub const fn digest(&self) -> ChunkDigest
    {
        return self.digest;
    }

    /// Returns the framed chunk image.
    #[inline]
    #[must_use]
    pub const fn image(&self) -> ChunkImage<'image>
    {
        return self.image;
    }
}

/// The verified backing store for value-plane chunks.
///
/// Deliberately a **sibling** of `gandr_storage_prolly_trees::BlockStore`
/// rather than a use of it: that trait's contract includes decoding its bytes
/// as canonical prolly-node material, which a chunk body is not and will not
/// become. One backing object may implement both traits, which is how a later
/// rung shares a single store between the keyed and value planes.
pub trait ChunkStore
{
    /// Inserts a framed chunk image under its claimed digest.
    ///
    /// # Contract
    /// - requires: nothing; a false claim is the case this refuses.
    /// - ensures: `Ok` only when `chunk.image()` hashes to `chunk.digest()` and
    ///   parses as a well-formed frame; a re-insert of identical bytes is
    ///   idempotent, which is what makes commit convergent under sharing.
    /// - provides: the write half of the value plane.
    /// - fails: [`ValueError::DigestMismatch`] or
    ///   [`ValueError::MalformedChunk`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    fn insert(
        &mut self,
        chunk: StoredChunkRef<'_>,
    ) -> Result<(), ValueError>;

    /// Loads a framed chunk image by digest, re-verifying it.
    ///
    /// # Contract
    /// - requires: nothing; an absent or corrupted chunk is refused by name.
    /// - ensures: `Ok` only when the stored bytes still hash to `digest` and
    ///   still parse as a well-formed frame.
    /// - provides: the read half of the value plane.
    /// - fails: [`ValueError::UnknownChunk`], [`ValueError::DigestMismatch`],
    ///   or [`ValueError::MalformedChunk`].
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError`].
    fn load(
        &self,
        digest: ChunkDigest,
    ) -> Result<StoredChunkRef<'_>, ValueError>;
}

/// Deterministic in-memory [`ChunkStore`] for local callers and tests.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[repr(transparent)]
pub struct InMemoryChunkStore
{
    /// Framed chunk images keyed by their digest.
    chunks: BTreeMap<ChunkDigest, Box<[u8]>>,
}

impl InMemoryChunkStore
{
    /// Creates an empty deterministic in-memory chunk store.
    #[inline]
    #[must_use]
    pub const fn new() -> Self
    {
        return Self {
            chunks: BTreeMap::new(),
        };
    }

    /// Returns the number of distinct chunks held.
    ///
    /// This is the structural-sharing observable: two commits of values that
    /// share a subtree add one chunk between them, not two, and that is the
    /// count [`crate::value::locality`] reads. It is deliberately the only
    /// size question the store answers — a `len`-and-`is_empty` pair would
    /// invite counting chunks as a collection rather than as sharing.
    #[inline]
    #[must_use]
    pub fn chunk_count(&self) -> ChunkCount
    {
        return ChunkCount::from(self.chunks.len());
    }
}

impl ChunkStore for InMemoryChunkStore
{
    #[inline]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: frame verification is the implementor deliverable"
    )]
    fn insert(
        &mut self,
        chunk: StoredChunkRef<'_>,
    ) -> Result<(), ValueError>
    {
        todo!(
            "verify_chunk_image(chunk), then insert chunk.image() under chunk.digest(): {chunk:?}"
        );
    }

    #[inline]
    #[expect(
        clippy::todo,
        reason = "gandr-8tou.4 scaffold: frame verification is the implementor deliverable"
    )]
    fn load(
        &self,
        digest: ChunkDigest,
    ) -> Result<StoredChunkRef<'_>, ValueError>
    {
        todo!("fetch {digest} and re-run verify_chunk_image before returning it");
    }
}

/// Recomputes a chunk image's digest and re-reads its frame.
///
/// # Contract
/// - requires: nothing; every rejection is a named error.
/// - ensures: `Ok` exactly when the image hashes to its claimed digest, opens
///   with [`VALUE_CHUNK_MAGIC`], carries a supported chunk format version, and
///   declares a body length matching the bytes that follow it.
/// - provides: the single verification both store halves run, so an
///   implementation cannot verify one side and not the other.
/// - fails: [`ValueError::DigestMismatch`] or [`ValueError::MalformedChunk`].
/// - panics: none.
///
/// # Errors
/// [`ValueError`].
#[inline]
#[expect(
    clippy::todo,
    reason = "gandr-8tou.4 scaffold: frame verification is the implementor deliverable"
)]
pub fn verify_chunk_image(chunk: StoredChunkRef<'_>) -> Result<(), ValueError>
{
    todo!("BLAKE3 the image, compare against the claim, then re-read the frame: {chunk:?}");
}

/// Frames a token body into a chunk image and returns it with its digest.
///
/// # Contract
/// - requires: `body` is a canonical token body carrying `token_count` tokens.
/// - ensures: the returned image is exactly the frame this module documents,
///   and the digest is `BLAKE3` over the whole image including the magic.
/// - provides: the only sanctioned way a chunk comes into existence, so the
///   framing cannot be reconstructed by a caller and drift from the reader.
/// - fails: [`ValueError::WidthOverflow`] when a length does not fit its
///   canonical width, never a truncation.
/// - panics: none.
///
/// # Errors
/// [`ValueError`].
#[inline]
#[expect(
    clippy::todo,
    reason = "gandr-8tou.4 scaffold: chunk framing is the implementor deliverable"
)]
pub fn frame_chunk(
    body: ChunkBody<'_>,
    token_count: TokenCount,
) -> Result<(ChunkDigest, ChunkImageBuf), ValueError>
{
    todo!(
        "frame {token_count:?} tokens over a {} byte body",
        body.as_ref().len()
    );
}
