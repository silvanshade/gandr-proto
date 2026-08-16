//! The **transport-step identity** — a durable, portable address for one
//! certificate step (tracker item `gandr-4o8a`).
//!
//! This module is the only sanctioned way a certificate step crosses a
//! persistence or transport boundary.
//! The certificate layer's in-process labels (the 128-bit `PrimId`,
//! `CellAddress`, and `CausalPast` digests of `gandr-theory-computads`) are
//! fixed-seed FNV-1a over [`core::hash::Hash`] writes — native-endian and
//! target-width, stable for one build of one target and no further. Nothing
//! may persist or transmit one. A [`TransportStepId`] is the separation: a
//! fixed-width 32-byte BLAKE3 identity over a **canonical framed preimage**
//! whose byte order (big-endian), integer width (u64 everywhere, each
//! conversion width-checked), format version (pinned in
//! [`TRANSPORT_STEP_MAGIC`]), and domain (the magic itself — the single step
//! domain) are stated here rather than inherited from a memory encoding.
//!
//! # Construction is the boundary
//!
//! There is deliberately **no generic mint** over a caller-chosen domain or
//! payload: the only constructive path to a [`TransportStepId`] is
//! [`StepIdEncoder`], which frames the versioned step-domain magic itself and
//! then admits only width-checked canonical fields. The canonical encoding of
//! resolved cell content and application position lives one crate up, in
//! `gandr-theory-computads`'s certificate/transport adapter, which streams
//! those fields through this encoder. Raw exact-length ingest
//! ([`TryFrom<&[u8]>`](TransportStepId::try_from)) exists for readback and for
//! the collision witnesses, and refuses any image that is not exactly
//! [`TRANSPORT_STEP_ID_LEN`] bytes — an in-process 16-byte label image among
//! them.
//!
//! # Two walls, restated
//!
//! The discipline of [`crate::manifest`] applies verbatim: an identity
//! addresses and authenticates bytes, never their validity. A matching
//! [`TransportStepId`] proves provenance of a canonical step image; it does
//! not re-check the step, and replay remains the sole validity authority.

use crate::error::StepIdError;

/// Domain-separation magic for the gandr transport-step identity.
///
/// The format version is pinned in the magic (`v1`) — the same discipline as
/// [`crate::manifest::MANIFEST_MAGIC`], and the single step domain: there is
/// one kind of transport identity, the step, and no position-free portable
/// cell address.
pub const TRANSPORT_STEP_MAGIC: &[u8] = b"gandr:transport-step:v1";

/// The byte length of a [`TransportStepId`] (a BLAKE3 digest).
pub const TRANSPORT_STEP_ID_LEN: usize = 32;

/// A canonical 64-bit unsigned integer field of the transport-step framing —
/// the **only** integer width the preimage admits, always encoded big-endian.
///
/// Every count, length, tag, and position step crosses into the framing
/// through this wrapper: constants and already-narrow values through
/// [`From<u64>`], target-width `usize` values through the checked
/// [`TryFrom<usize>`], which names the overflow rather than truncating.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalU64(u64);

impl From<u64> for CanonicalU64
{
    #[inline]
    fn from(value: u64) -> Self
    {
        Self(value)
    }
}

impl From<CanonicalU64> for u64
{
    #[inline]
    fn from(value: CanonicalU64) -> Self
    {
        value.0
    }
}

impl TryFrom<usize> for CanonicalU64
{
    type Error = StepIdError;

    /// # Contract
    /// - ensures: `Ok` carrying `value` unchanged when it fits the canonical
    ///   u64 width — always, on every target whose `usize` is at most 64 bits.
    /// - fails: [`StepIdError::WidthOverflow`] carrying the offending value,
    ///   never a truncation, a wrap, or a clamp.
    /// - panics: none.
    ///
    /// # Errors
    /// [`StepIdError`].
    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error>
    {
        match u64::try_from(value) {
            | Ok(narrow) => Ok(Self(narrow)),
            | Err(_) => Err(StepIdError::WidthOverflow { found: value }),
        }
    }
}

/// A borrowed canonical byte field whose length has been width-checked,
/// carried with that length so the framing can length-prefix it.
///
/// Construction ([`TryFrom<&[u8]>`](CanonicalBytes::try_from)) is the one
/// checked point: the length is pinned to the canonical u64 width once, and
/// the framing then streams length and bytes without re-measuring.
#[derive(Clone, Copy, Debug)]
pub struct CanonicalBytes<'source>
{
    /// The field bytes.
    bytes: &'source [u8],
    /// The byte count, width-checked at construction.
    length: CanonicalU64,
}

impl<'source> TryFrom<&'source [u8]> for CanonicalBytes<'source>
{
    type Error = StepIdError;

    /// # Contract
    /// - ensures: `Ok` carrying the slice and its length width-checked against
    ///   the canonical u64 width — never failing on a supported target.
    /// - fails: [`StepIdError::WidthOverflow`] when the length does not fit.
    /// - panics: none.
    ///
    /// # Errors
    /// [`StepIdError`].
    #[inline]
    fn try_from(bytes: &'source [u8]) -> Result<Self, Self::Error>
    {
        let length = CanonicalU64::try_from(bytes.len())?;
        Ok(Self { bytes, length })
    }
}

/// A **streaming encoder** for the canonical transport-step preimage.
///
/// [`StepIdEncoder::begin`] frames the versioned step-domain magic; each
/// `put_*` streams one width-checked canonical field straight into the BLAKE3
/// hasher, so the preimage never exists as a whole buffer and the memory cost
/// is the hasher's block state, not the encoded tree's size. Field *order and
/// selection* are the caller's (the certificate layer's canonical encoding);
/// byte order, integer width, length framing, version, and domain are pinned
/// here.
#[repr(transparent)]
#[derive(Debug)]
pub struct StepIdEncoder
{
    /// The running BLAKE3 state over the framed preimage.
    hasher: blake3::Hasher,
}

impl StepIdEncoder
{
    /// Begin a canonical preimage: the versioned step-domain magic is framed
    /// before any caller field, so no caller can mint under another domain or
    /// skip the version.
    ///
    /// # Contract
    /// - ensures: an encoder whose running preimage is exactly
    ///   [`TRANSPORT_STEP_MAGIC`].
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn begin() -> Self
    {
        let mut hasher = blake3::Hasher::new();
        hasher.update(TRANSPORT_STEP_MAGIC);
        Self { hasher }
    }

    /// Stream one canonical integer field as eight big-endian bytes.
    ///
    /// # Contract
    /// - ensures: the preimage grows by `value` as a fixed-width big-endian u64
    ///   — no varints, no native-endian writes, no target-width fields.
    /// - panics: none.
    #[inline]
    pub fn put_u64(
        &mut self,
        value: CanonicalU64,
    )
    {
        self.hasher.update(&u64::from(value).to_be_bytes());
    }

    /// Stream one canonical byte field as its u64 big-endian length followed
    /// by the bytes — the length prefix is what keeps adjacent fields
    /// unambiguous.
    ///
    /// # Contract
    /// - ensures: the preimage grows by the field's width-checked length as
    ///   eight big-endian bytes, then the field bytes verbatim.
    /// - panics: none.
    #[inline]
    pub fn put_bytes(
        &mut self,
        field: CanonicalBytes<'_>,
    )
    {
        self.put_u64(field.length);
        self.hasher.update(field.bytes);
    }

    /// Mint the transport-step identity of the framed preimage.
    ///
    /// # Contract
    /// - ensures: `BLAKE3` of exactly the bytes framed since
    ///   [`StepIdEncoder::begin`], in order — a deterministic function of the
    ///   canonical preimage on every target and in every process.
    /// - provides: the durable step identity.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — the decision surface is "the identity is
    ///   BLAKE3 of the framed preimage", separated by an independent
    ///   recomputation over a hand-concatenated preimage, a hardcoded golden
    ///   vector, and the ingest refusal of every wrong length.
    /// - witness: `transport::tests::the_identity_is_blake3_of_the_framed_preimage`
    /// - witness: `transport::tests::the_v1_golden_vector_is_stable`
    /// - witness: `transport::tests::ingest_refuses_anything_but_the_fixed_width`
    #[inline]
    #[must_use]
    pub fn finish(self) -> TransportStepId
    {
        TransportStepId::from(*self.hasher.finalize().as_bytes())
    }
}

/// The **transport-step identity** of one certificate step.
///
/// It is `BLAKE3` of the
/// canonical framed preimage over the resolved cell's content and the
/// application position, minted only through [`StepIdEncoder`].
///
/// This is the type the in-process labels are separated from: it is durable
/// and portable where they are process-local, fixed-width where they are
/// target-width, and canonical where they ride a memory encoding. There is no
/// conversion between the two families in either direction; a byte image of
/// the in-process width (16 bytes) is refused at ingest.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TransportStepId(
    /// The raw BLAKE3 digest bytes.
    [u8; TRANSPORT_STEP_ID_LEN],
);

impl From<[u8; TRANSPORT_STEP_ID_LEN]> for TransportStepId
{
    #[inline]
    fn from(bytes: [u8; TRANSPORT_STEP_ID_LEN]) -> Self
    {
        Self(bytes)
    }
}

impl TryFrom<&[u8]> for TransportStepId
{
    type Error = StepIdError;

    /// Ingest a transport-step identity from its byte image (readback).
    ///
    /// # Contract
    /// - ensures: `Ok` iff the image is exactly [`TRANSPORT_STEP_ID_LEN`]
    ///   bytes, carrying them unchanged.
    /// - fails: [`StepIdError::ImageLength`] on any other length — including
    ///   the 16-byte width of the in-process labels, which is how a
    ///   process-local digest is refused at the boundary rather than silently
    ///   reinterpreted.
    /// - panics: none.
    ///
    /// # Errors
    /// [`StepIdError`].
    #[inline]
    fn try_from(image: &[u8]) -> Result<Self, Self::Error>
    {
        match <&[u8; TRANSPORT_STEP_ID_LEN]>::try_from(image) {
            | Ok(bytes) => Ok(Self(*bytes)),
            | Err(_) => Err(StepIdError::ImageLength {
                found: image.len(),
                expected: TRANSPORT_STEP_ID_LEN,
            }),
        }
    }
}

impl AsRef<[u8; TRANSPORT_STEP_ID_LEN]> for TransportStepId
{
    #[inline]
    fn as_ref(&self) -> &[u8; TRANSPORT_STEP_ID_LEN]
    {
        &self.0
    }
}

impl AsRef<[u8]> for TransportStepId
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        &self.0
    }
}

impl core::fmt::Debug for TransportStepId
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

impl core::fmt::Display for TransportStepId
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

/// The framing, ingest, and golden-vector witnesses.
#[cfg(test)]
mod tests
{
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    /// The fixed v1 golden field set, framed once here and shared by the
    /// golden and independence witnesses.
    fn golden_identity() -> TransportStepId
    {
        let mut encoder = StepIdEncoder::begin();
        encoder.put_u64(CanonicalU64::from(0x2a_u64));
        let field = CanonicalBytes::try_from(&b"gandr"[..]).expect("the length fits the width");
        encoder.put_bytes(field);
        encoder.finish()
    }

    #[test]
    fn the_v1_golden_vector_is_stable()
    {
        assert_eq!(
            "3c1a90f328af3242f3a06c85f20f4b613137b32bde84e13c2a8983e8f2f5943e",
            golden_identity().to_string(),
            "the v1 framing mints the recorded golden digest",
        );
    }

    #[test]
    fn the_identity_is_blake3_of_the_framed_preimage()
    {
        let mut preimage = Vec::new();
        preimage.extend_from_slice(TRANSPORT_STEP_MAGIC);
        preimage.extend_from_slice(&0x2a_u64.to_be_bytes());
        preimage.extend_from_slice(&5_u64.to_be_bytes());
        preimage.extend_from_slice(b"gandr");
        let expected = blake3::hash(&preimage);
        assert_eq!(
            expected.as_bytes(),
            AsRef::<[u8; TRANSPORT_STEP_ID_LEN]>::as_ref(&golden_identity()),
            "the encoder frames magic, then u64 fields big-endian, then length-prefixed bytes",
        );
    }

    #[test]
    fn ingest_refuses_anything_but_the_fixed_width()
    {
        for found in [0_usize, 16, 31, 33] {
            let image = vec![0_u8; found];
            assert_eq!(
                Err(StepIdError::ImageLength {
                    found,
                    expected: TRANSPORT_STEP_ID_LEN,
                }),
                TransportStepId::try_from(image.as_slice()),
                "a {found}-byte image is refused",
            );
        }
        // Sixteen bytes is the in-process label width: a serialized `PrimId`,
        // `CellAddress`, or `CausalPast` image can never decode as a
        // transport identity.
    }

    #[test]
    fn an_identity_round_trips_through_its_byte_image()
    {
        let identity = golden_identity();
        let bytes: &[u8; TRANSPORT_STEP_ID_LEN] = identity.as_ref();
        let decoded = TransportStepId::try_from(&bytes[..]).expect("the fixed width decodes");
        assert_eq!(
            identity, decoded,
            "the byte image carries the identity exactly"
        );
    }

    /// One past the 32-bit ceiling — representable as a `usize` exactly on
    /// 64-bit-and-wider targets, and the value the two-sided width witness is
    /// separated by.
    #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
    const PAST_32_BIT_CEILING: u64 = 0x1_0000_0000;

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn the_checked_widening_encodes_past_the_32_bit_ceiling()
    {
        let wide = usize::try_from(PAST_32_BIT_CEILING)
            .expect("a 64-bit usize holds one past the 32-bit ceiling");
        let count = CanonicalU64::try_from(wide).expect("and it fits the canonical u64 width");
        assert_eq!(
            PAST_32_BIT_CEILING,
            u64::from(count),
            "the value crosses the checked boundary unchanged"
        );
        let mut encoder = StepIdEncoder::begin();
        encoder.put_u64(count);
        let mut preimage = Vec::new();
        preimage.extend_from_slice(TRANSPORT_STEP_MAGIC);
        preimage.extend_from_slice(&PAST_32_BIT_CEILING.to_be_bytes());
        let expected = blake3::hash(&preimage);
        assert_eq!(
            expected.as_bytes(),
            AsRef::<[u8; TRANSPORT_STEP_ID_LEN]>::as_ref(&encoder.finish()),
            "one past the 32-bit ceiling frames as exactly its eight big-endian bytes"
        );
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn the_checked_widening_refuses_past_the_32_bit_ceiling()
    {
        assert!(
            usize::try_from(PAST_32_BIT_CEILING).is_err(),
            "one past the 32-bit ceiling is not a usize on a 32-bit target, so it never reaches CanonicalU64"
        );
        let ceiling =
            usize::try_from(u64::from(u32::MAX)).expect("the 32-bit ceiling itself is a usize");
        let count = CanonicalU64::try_from(ceiling).expect("and it fits the canonical u64 width");
        let mut encoder = StepIdEncoder::begin();
        encoder.put_u64(count);
        let mut preimage = Vec::new();
        preimage.extend_from_slice(TRANSPORT_STEP_MAGIC);
        preimage.extend_from_slice(&u64::from(u32::MAX).to_be_bytes());
        let expected = blake3::hash(&preimage);
        assert_eq!(
            expected.as_bytes(),
            AsRef::<[u8; TRANSPORT_STEP_ID_LEN]>::as_ref(&encoder.finish()),
            "the 32-bit ceiling frames as the same eight big-endian bytes as on any wider target"
        );
    }
}
