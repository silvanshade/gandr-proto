//! The content pointer — the value plane's whole address vocabulary.
//!
//! A [`ContentPtr`] is *stable, portable and world-independent*: it names a
//! position inside a value by naming the chunk that holds it and the token
//! offset within that chunk. Nothing about the process, the arena, the
//! insertion order, or the machine that produced it enters the address, which
//! is the difference between this and every in-process label the tree carries.

use crate::error::ValueError;

/// The byte length of a [`ChunkDigest`] (a BLAKE3 digest).
pub const CHUNK_DIGEST_LEN: usize = 32;

/// The BLAKE3 identity of one value-plane chunk image.
///
/// Deliberately **not** `gandr_storage_prolly_trees::NodeHash`, and no
/// conversion is offered in either direction. The two planes hash different
/// framed bodies under different domain magics; a shared digest type would
/// invite a caller to hand a chunk digest to a node validator, which is the
/// one confusion the framing exists to prevent.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChunkDigest(
    /// Raw BLAKE3 output bytes over the framed chunk image.
    [u8; CHUNK_DIGEST_LEN],
);

impl From<[u8; CHUNK_DIGEST_LEN]> for ChunkDigest
{
    /// Wraps raw digest bytes without computing anything.
    ///
    /// The digest is *claimed* by whoever wraps it; nothing here checks that
    /// the bytes are the BLAKE3 output over any particular image. The claim is
    /// checked where it matters, in
    /// [`crate::value::chunk::verify_chunk_image`].
    #[inline]
    fn from(bytes: [u8; CHUNK_DIGEST_LEN]) -> Self
    {
        return Self(bytes);
    }
}

impl AsRef<[u8]> for ChunkDigest
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0.as_slice();
    }
}

impl core::fmt::Display for ChunkDigest
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        return Ok(());
    }
}

impl core::fmt::Debug for ChunkDigest
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        return core::fmt::Display::fmt(self, f);
    }
}

impl TryFrom<&[u8]> for ChunkDigest
{
    type Error = ValueError;

    /// # Contract
    /// - requires: nothing; a wrong length is the case this refuses.
    /// - ensures: `Ok` exactly when `bytes` is [`CHUNK_DIGEST_LEN`] long.
    /// - provides: exact-width ingest for readback paths.
    /// - fails: [`ValueError::DigestLength`], never a truncation or a pad.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ValueError::DigestLength`].
    #[inline]
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error>
    {
        let image: [u8; CHUNK_DIGEST_LEN] =
            bytes
                .try_into()
                .map_err(|_ignored| ValueError::DigestLength {
                    found: bytes.len(),
                    expected: CHUNK_DIGEST_LEN,
                })?;
        return Ok(Self(image));
    }
}

/// A token index inside one chunk's token stream.
///
/// The offset is *within the chunk*, never within the whole value: that is
/// what keeps a pointer stable when an unrelated part of the value changes,
/// and it is the same reason [`crate::value::index_base`] exists as an open
/// evaluation for child references.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TokenOffset(u32);

impl From<u32> for TokenOffset
{
    #[inline]
    fn from(offset: u32) -> Self
    {
        return Self(offset);
    }
}

impl From<TokenOffset> for u32
{
    #[inline]
    fn from(offset: TokenOffset) -> Self
    {
        return offset.0;
    }
}

impl From<TokenOffset> for u64
{
    #[inline]
    fn from(offset: TokenOffset) -> Self
    {
        return Self::from(offset.0);
    }
}

/// A content pointer: the storage form of a mobile reference.
///
/// The pair is the whole address. A consumer that holds one can fetch and
/// verify the chunk, then read the value rooted at the offset, with no
/// reference to the store it came from beyond the store being able to answer
/// for the digest.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentPtr
{
    /// The chunk holding the addressed subtree.
    digest: ChunkDigest,
    /// The token index of the subtree root within that chunk.
    offset: TokenOffset,
}

impl ContentPtr
{
    /// Pairs a chunk digest with a token offset inside that chunk.
    ///
    /// # Contract
    /// - requires: `offset` indexes a constructor position of the chunk
    ///   `digest` names — unchecked here, and checked by [`super::cam_deref`]
    ///   when the chunk is actually fetched.
    /// - ensures: the pointer carries both fields unchanged.
    /// - provides: the plane's only address constructor.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn new(
        digest: ChunkDigest,
        offset: TokenOffset,
    ) -> Self
    {
        return Self { digest, offset };
    }

    /// Returns the addressed chunk's digest.
    #[inline]
    #[must_use]
    pub const fn digest(&self) -> ChunkDigest
    {
        return self.digest;
    }

    /// Returns the token offset within the addressed chunk.
    #[inline]
    #[must_use]
    pub const fn offset(&self) -> TokenOffset
    {
        return self.offset;
    }
}
