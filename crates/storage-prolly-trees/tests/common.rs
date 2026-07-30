use core::ops::Deref;
use core::ops::DerefMut;

/// Borrowed items carried by a structured fixture.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FixtureSlice<'items, Item>(&'items [Item]);

impl<'items, Item> From<&'items [Item]> for FixtureSlice<'items, Item>
{
    #[inline]
    fn from(items: &'items [Item]) -> Self
    {
        return Self(items);
    }
}

impl<'items, Item> From<FixtureSlice<'items, Item>> for &'items [Item]
{
    #[inline]
    fn from(items: FixtureSlice<'items, Item>) -> Self
    {
        return items.0;
    }
}

impl<Item> AsRef<[Item]> for FixtureSlice<'_, Item>
{
    #[inline]
    fn as_ref(&self) -> &[Item]
    {
        return self.0;
    }
}

/// Borrowed bytes carried by a binary-format fixture.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FixtureBytes<'bytes>(&'bytes [u8]);

impl<'bytes> From<&'bytes [u8]> for FixtureBytes<'bytes>
{
    #[inline]
    fn from(bytes: &'bytes [u8]) -> Self
    {
        return Self(bytes);
    }
}

impl<'bytes> From<FixtureBytes<'bytes>> for &'bytes [u8]
{
    #[inline]
    fn from(bytes: FixtureBytes<'bytes>) -> Self
    {
        return bytes.0;
    }
}

impl Deref for FixtureBytes<'_>
{
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        return self.0;
    }
}
impl AsRef<[u8]> for FixtureBytes<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0;
    }
}

/// Mutably borrowed bytes carried by a binary-format fixture.
#[repr(transparent)]
#[derive(Debug, Eq, PartialEq)]
pub struct FixtureBytesMut<'bytes>(&'bytes mut [u8]);

impl<'bytes> From<&'bytes mut [u8]> for FixtureBytesMut<'bytes>
{
    #[inline]
    fn from(bytes: &'bytes mut [u8]) -> Self
    {
        return Self(bytes);
    }
}

impl<'bytes> From<FixtureBytesMut<'bytes>> for &'bytes mut [u8]
{
    #[inline]
    fn from(bytes: FixtureBytesMut<'bytes>) -> Self
    {
        return bytes.0;
    }
}

impl AsRef<[u8]> for FixtureBytesMut<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0;
    }
}

impl Deref for FixtureBytesMut<'_>
{
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        return self.0;
    }
}

impl DerefMut for FixtureBytesMut<'_>
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        return self.0;
    }
}
impl AsMut<[u8]> for FixtureBytesMut<'_>
{
    #[inline]
    fn as_mut(&mut self) -> &mut [u8]
    {
        return self.0;
    }
}

/// Owned bytes carried by a binary-format fixture.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnedFixtureBytes(Vec<u8>);

impl From<Vec<u8>> for OwnedFixtureBytes
{
    #[inline]
    fn from(bytes: Vec<u8>) -> Self
    {
        return Self(bytes);
    }
}

impl From<Box<[u8]>> for OwnedFixtureBytes
{
    #[inline]
    fn from(bytes: Box<[u8]>) -> Self
    {
        return Self(Vec::<u8>::from(bytes));
    }
}

impl From<OwnedFixtureBytes> for Vec<u8>
{
    #[inline]
    fn from(bytes: OwnedFixtureBytes) -> Self
    {
        return bytes.0;
    }
}

impl From<OwnedFixtureBytes> for Box<[u8]>
{
    #[inline]
    fn from(bytes: OwnedFixtureBytes) -> Self
    {
        return bytes.0.into_boxed_slice();
    }
}

impl AsRef<[u8]> for OwnedFixtureBytes
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0.as_slice();
    }
}

impl AsMut<[u8]> for OwnedFixtureBytes
{
    #[inline]
    fn as_mut(&mut self) -> &mut [u8]
    {
        return self.0.as_mut_slice();
    }
}

impl Deref for OwnedFixtureBytes
{
    type Target = Vec<u8>;

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        return &self.0;
    }
}

impl DerefMut for OwnedFixtureBytes
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        return &mut self.0;
    }
}

/// Byte position within a binary-format fixture.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ByteOffset(usize);

impl From<usize> for ByteOffset
{
    #[inline]
    fn from(offset: usize) -> Self
    {
        return Self(offset);
    }
}

impl From<ByteOffset> for usize
{
    #[inline]
    fn from(offset: ByteOffset) -> Self
    {
        return offset.0;
    }
}

/// Byte length within a binary-format fixture.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ByteLength(usize);

impl From<usize> for ByteLength
{
    #[inline]
    fn from(length: usize) -> Self
    {
        return Self(length);
    }
}

impl From<ByteLength> for usize
{
    #[inline]
    fn from(length: ByteLength) -> Self
    {
        return length.0;
    }
}

/// Unsigned 16-bit field in a binary-format fixture.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FixtureWord(u16);

impl From<u16> for FixtureWord
{
    #[inline]
    fn from(value: u16) -> Self
    {
        return Self(value);
    }
}

impl From<FixtureWord> for u16
{
    #[inline]
    fn from(value: FixtureWord) -> Self
    {
        return value.0;
    }
}

/// Unsigned 64-bit field in a binary-format fixture.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FixtureLong(u64);

impl From<u64> for FixtureLong
{
    #[inline]
    fn from(value: u64) -> Self
    {
        return Self(value);
    }
}

impl From<FixtureLong> for u64
{
    #[inline]
    fn from(value: FixtureLong) -> Self
    {
        return value.0;
    }
}

/// Position of one proof node within a fixture proof.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProofNodeIndex(usize);

impl From<usize> for ProofNodeIndex
{
    #[inline]
    fn from(index: usize) -> Self
    {
        return Self(index);
    }
}

impl From<ProofNodeIndex> for usize
{
    #[inline]
    fn from(index: ProofNodeIndex) -> Self
    {
        return index.0;
    }
}

/// Static assertion or decode context carried by a test helper.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TestContext(&'static str);

impl From<&'static str> for TestContext
{
    #[inline]
    fn from(context: &'static str) -> Self
    {
        return Self(context);
    }
}

impl From<TestContext> for &'static str
{
    #[inline]
    fn from(context: TestContext) -> Self
    {
        return context.0;
    }
}
