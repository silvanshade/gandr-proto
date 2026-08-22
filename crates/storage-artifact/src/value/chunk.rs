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
//! # The digest never routes through `core::hash::Hash`, and that is a fence
//!
//! The certificate layer's in-process labels are FNV-1a taken through
//! [`core::hash::Hash`], whose integer writers encode native-endian and `usize`
//! at the target's width. Such a digest is stable for **one build of one
//! target** — a comparable value, never a portable address — and a stored form
//! keyed by one decodes on another machine as a different key, with no
//! diagnostic and a false negative at the end of it.
//!
//! Every digest on this plane is BLAKE3 over the framed image built here, with
//! every integer big-endian at a fixed width and no [`core::hash::Hash`] call
//! anywhere on the path. The committed-golden test in the value contract suite
//! is what makes that a checked property rather than a stated intention: a
//! native-endian digest path passes on the machine that minted the golden and
//! fails on every other one.
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
use alloc::vec::Vec;

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

/// The framed header after the magic: a `u16` version and two `u64` fields.
const CHUNK_HEADER_LEN: usize = 0x12_usize;

/// The canonical integer width every framed count is checked against.
const CANONICAL_WIDTH_BITS: u32 = 0x40_u32;

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
    fn insert(
        &mut self,
        chunk: StoredChunkRef<'_>,
    ) -> Result<(), ValueError>
    {
        verify_chunk_image(chunk)?;
        self.chunks
            .insert(chunk.digest(), Box::<[u8]>::from(chunk.image().as_ref()));
        return Ok(());
    }

    #[inline]
    fn load(
        &self,
        digest: ChunkDigest,
    ) -> Result<StoredChunkRef<'_>, ValueError>
    {
        let bytes = self
            .chunks
            .get(&digest)
            .ok_or_else(|| ValueError::UnknownChunk {
                digest: digest.to_string(),
            })?;
        let chunk = StoredChunkRef::new(digest, ChunkImage::from(bytes.as_ref()));
        verify_chunk_image(chunk)?;
        return Ok(chunk);
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
pub fn verify_chunk_image(chunk: StoredChunkRef<'_>) -> Result<(), ValueError>
{
    let image = chunk.image();
    let image_bytes: &[u8] = image.as_ref();
    let actual = blake3::hash(image_bytes);
    if actual.as_bytes() != chunk.digest().as_ref() {
        return Err(ValueError::DigestMismatch {
            expected: chunk.digest().to_string(),
            actual: actual.to_string(),
        });
    }
    parse_chunk_frame(image_bytes).map(|_body| ())
}

/// Re-reads one framed image's header and returns its token body.
///
/// # Contract
/// - requires: nothing; every rejection is a named error.
/// - ensures: `Ok(body)` exactly when the image opens with
///   [`VALUE_CHUNK_MAGIC`], carries [`CHUNK_FORMAT_VERSION_V1`], and declares a
///   body length matching the bytes that follow the header.
/// - provides: the single frame reader both store halves and the deref path
///   share, so a body slice cannot be taken through a different grammar.
/// - fails: [`ValueError::MalformedChunk`] naming the failed field.
/// - panics: none.
///
/// # Errors
/// [`ValueError::MalformedChunk`].
fn parse_chunk_frame(image: &[u8]) -> Result<ChunkBody<'_>, ValueError>
{
    const MAGIC_LEN: usize = VALUE_CHUNK_MAGIC.len();
    let refused = |context: &'static str| ValueError::MalformedChunk { context };
    let Some((magic, rest)) = image.split_first_chunk::<MAGIC_LEN>()
    else {
        return Err(refused("the image ends inside the chunk magic"));
    };
    if magic != VALUE_CHUNK_MAGIC {
        return Err(refused(
            "the image does not open with the value-chunk magic",
        ));
    }
    let Some((version_bytes, rest)) = rest.split_first_chunk::<2>()
    else {
        return Err(refused("the image ends inside the format version"));
    };
    if u16::from_be_bytes(*version_bytes) != CHUNK_FORMAT_VERSION_V1 {
        return Err(refused("unsupported chunk format version"));
    }
    let Some((_count_bytes, rest)) = rest.split_first_chunk::<8>()
    else {
        return Err(refused("the image ends inside the token count"));
    };
    let Some((len_bytes, body)) = rest.split_first_chunk::<8>()
    else {
        return Err(refused("the image ends inside the body length"));
    };
    let declared_len = u64::from_be_bytes(*len_bytes);
    let actual_len = u64::try_from(body.len())
        .map_err(|_width| refused("the body length does not fit the canonical width"))?;
    if actual_len != declared_len {
        return Err(refused(
            "declared body length does not match the image bytes",
        ));
    }
    return Ok(ChunkBody::from(body));
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
pub fn frame_chunk(
    body: ChunkBody<'_>,
    token_count: TokenCount,
) -> Result<(ChunkDigest, ChunkImageBuf), ValueError>
{
    let body_bytes: &[u8] = body.as_ref();
    let body_len = u64::try_from(body_bytes.len()).map_err(|_width| ValueError::WidthOverflow {
        found: u64::MAX,
        width: CANONICAL_WIDTH_BITS,
    })?;
    // Exact, so the frame is written into one allocation: magic, the u16
    // version, the u64 token count, the u64 body length, then the body.
    let capacity = VALUE_CHUNK_MAGIC
        .len()
        .saturating_add(CHUNK_HEADER_LEN)
        .saturating_add(body_bytes.len());
    let mut image = Vec::with_capacity(capacity);
    image.extend_from_slice(VALUE_CHUNK_MAGIC);
    image.extend_from_slice(&CHUNK_FORMAT_VERSION_V1.to_be_bytes());
    image.extend_from_slice(&u64::from(token_count).to_be_bytes());
    image.extend_from_slice(&body_len.to_be_bytes());
    image.extend_from_slice(body_bytes);
    let digest = blake3::hash(image.as_slice());
    return Ok((
        ChunkDigest::from(*digest.as_bytes()),
        ChunkImageBuf::from(image.into_boxed_slice()),
    ));
}
