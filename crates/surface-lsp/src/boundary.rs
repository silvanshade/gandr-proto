//! Semantic wrappers so crate-defined signatures never expose primitives.

use alloc::vec::Vec;

/// Owned framed JSON payload.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FrameBytes(pub Vec<u8>);

impl From<Vec<u8>> for FrameBytes
{
    #[inline]
    fn from(value: Vec<u8>) -> Self
    {
        Self(value)
    }
}

impl AsRef<[u8]> for FrameBytes
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        self.0.as_slice()
    }
}

/// Borrowed framed JSON payload.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FramePayload<'bytes>(pub &'bytes [u8]);

impl<'bytes> From<&'bytes [u8]> for FramePayload<'bytes>
{
    #[inline]
    fn from(value: &'bytes [u8]) -> Self
    {
        Self(value)
    }
}

impl AsRef<[u8]> for FramePayload<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        self.0
    }
}

/// Borrowed header or method text.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HeaderText<'text>(pub &'text str);

impl<'text> From<&'text str> for HeaderText<'text>
{
    #[inline]
    fn from(value: &'text str) -> Self
    {
        Self(value)
    }
}

impl AsRef<str> for HeaderText<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

/// A JSON-RPC method name.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MethodName<'text>(pub &'text str);

impl<'text> From<&'text str> for MethodName<'text>
{
    #[inline]
    fn from(value: &'text str) -> Self
    {
        Self(value)
    }
}

impl AsRef<str> for MethodName<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

/// An error-response message.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ErrorText<'text>(pub &'text str);

impl<'text> From<&'text str> for ErrorText<'text>
{
    #[inline]
    fn from(value: &'text str) -> Self
    {
        Self(value)
    }
}

impl AsRef<str> for ErrorText<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0
    }
}

/// Declared Content-Length.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContentLength(pub usize);

impl From<usize> for ContentLength
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<ContentLength> for usize
{
    #[inline]
    fn from(value: ContentLength) -> Self
    {
        value.0
    }
}

/// Whether a cursor sits inside a span.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContainsByte(pub bool);

impl From<bool> for ContainsByte
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<ContainsByte> for bool
{
    #[inline]
    fn from(value: ContainsByte) -> Self
    {
        value.0
    }
}

/// Whether the stdio loop should stop.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShouldStop(pub bool);

impl From<bool> for ShouldStop
{
    #[inline]
    fn from(value: bool) -> Self
    {
        Self(value)
    }
}

impl From<ShouldStop> for bool
{
    #[inline]
    fn from(value: ShouldStop) -> Self
    {
        value.0
    }
}

/// Width of one character in the negotiated encoding.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodingWidth(pub u32);

impl From<u32> for EncodingWidth
{
    #[inline]
    fn from(value: u32) -> Self
    {
        Self(value)
    }
}

impl From<EncodingWidth> for u32
{
    #[inline]
    fn from(value: EncodingWidth) -> Self
    {
        value.0
    }
}

/// One source character at a position-mapping boundary.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceChar(pub char);

impl From<char> for SourceChar
{
    #[inline]
    fn from(value: char) -> Self
    {
        Self(value)
    }
}

impl From<SourceChar> for char
{
    #[inline]
    fn from(value: SourceChar) -> Self
    {
        value.0
    }
}
