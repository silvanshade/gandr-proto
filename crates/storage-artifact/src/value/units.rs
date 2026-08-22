//! Semantic wrappers for the value plane's counts, positions and byte views.
//!
//! Every one of these exists so a signature says what it carries rather than
//! how wide it is. A `u64` in a public signature is a place where a token
//! count and a byte length can be swapped without the compiler noticing, and
//! on a content-addressed plane that swap changes digests rather than
//! crashing.
//!
//! # Getting the bytes out, and the trap in the obvious way
//!
//! Each borrowed wrapper offers both [`AsRef::as_ref`] and a `From` impl into
//! the raw slice, and they differ in a way that only shows at a lifetime.
//! `as_ref` borrows **the wrapper**, so on a temporary — a chunk image a store
//! just handed back by value — it yields a slice that dies at the end of the
//! statement. The `From` impl consumes the wrapper, which is free because
//! these are [`Copy`], and hands back a slice living as long as the data.
//!
//! So use `as_ref` for a slice consumed in place, and `<&[u8]>::from(wrapper)`
//! wherever the result must outlive the expression — the reader crossing a
//! chunk seam is exactly that case, and it is where the difference is a
//! borrow-checker error rather than a preference.

use alloc::boxed::Box;

/// Declares a transparent semantic wrapper over one primitive.
macro_rules! semantic_integer
{
    (
        $(#[$attribute:meta])*
        $visibility:vis struct $name:ident($primitive:ty);
    ) => {
        $(#[$attribute])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        $visibility struct $name($primitive);

        impl From<$primitive> for $name
        {
            #[inline]
            fn from(value: $primitive) -> Self
            {
                return Self(value);
            }
        }

        impl From<$name> for $primitive
        {
            #[inline]
            fn from(value: $name) -> Self
            {
                return value.0;
            }
        }
    };
}

semantic_integer! {
    /// A number of distinct chunks — what a store holds, or what an edit touched.
    ///
    /// Backed by `usize` rather than `u64` because every producer of one is a
    /// collection length or a bound compared against a collection length; a
    /// width conversion at the accessor would be a conversion with no reader.
    pub struct ChunkCount(usize);
}

semantic_integer! {
    /// The depth of an edited path, in constructors from the root.
    pub struct EditDepth(u32);
}

semantic_integer! {
    /// The cap multiplier `c` relating the hard token cap to kappa.
    pub struct CapMultiplier(u32);
}

semantic_integer! {
    /// The value-manifest layout version.
    pub struct ValueManifestVersion(u16);
}

semantic_integer! {
    /// The chunk image frame layout version the digests were taken over.
    pub struct ChunkFormatVersion(u16);
}

semantic_integer! {
    /// How many chunk seams a reader is currently inside.
    ///
    /// Named rather than counted as a bare depth because it is an assertion
    /// target: a value that should have crossed a seam and did not read the
    /// same to a round-trip test and differently to this.
    pub struct SeamDepth(usize);
}

/// A borrowed view of one chunk's token body — the bytes inside the frame.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChunkBody<'source>(&'source [u8]);

impl<'source> From<&'source [u8]> for ChunkBody<'source>
{
    #[inline]
    fn from(bytes: &'source [u8]) -> Self
    {
        return Self(bytes);
    }
}

impl<'source> From<ChunkBody<'source>> for &'source [u8]
{
    #[inline]
    fn from(body: ChunkBody<'source>) -> Self
    {
        return body.0;
    }
}

impl AsRef<[u8]> for ChunkBody<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0;
    }
}

/// A borrowed view of one framed chunk image — magic, header and body.
///
/// Distinct from [`ChunkBody`] on purpose: the digest is taken over the
/// **image**, and handing a body where an image belongs would produce a digest
/// that verifies against nothing.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChunkImage<'image>(&'image [u8]);

impl<'image> From<&'image [u8]> for ChunkImage<'image>
{
    #[inline]
    fn from(bytes: &'image [u8]) -> Self
    {
        return Self(bytes);
    }
}

impl<'image> From<ChunkImage<'image>> for &'image [u8]
{
    #[inline]
    fn from(image: ChunkImage<'image>) -> Self
    {
        return image.0;
    }
}

impl AsRef<[u8]> for ChunkImage<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0;
    }
}

/// An owned framed chunk image.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChunkImageBuf(Box<[u8]>);

impl From<Box<[u8]>> for ChunkImageBuf
{
    #[inline]
    fn from(bytes: Box<[u8]>) -> Self
    {
        return Self(bytes);
    }
}

impl From<ChunkImageBuf> for Box<[u8]>
{
    #[inline]
    fn from(image: ChunkImageBuf) -> Self
    {
        return image.0;
    }
}

impl AsRef<[u8]> for ChunkImageBuf
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0.as_ref();
    }
}

/// A canonical inline byte payload contributed by one constructor.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TokenBytes<'source>(&'source [u8]);

impl<'source> From<&'source [u8]> for TokenBytes<'source>
{
    #[inline]
    fn from(bytes: &'source [u8]) -> Self
    {
        return Self(bytes);
    }
}

impl AsRef<[u8]> for TokenBytes<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0;
    }
}
