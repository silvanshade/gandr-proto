//! The artifact manifest and its BLAKE3 identity — the b3sum-provenance
//! successor at B2.3 (massive-term design §6).
//!
//! An artifact's identity is `BLAKE3` of a root **manifest** binding the
//! chunker parameter commitment (the 85-byte fixed-order commitment), the
//! record count, the root node hash, and the **inner** format version (the
//! kernel export version the records were cut from). The manifest's own byte
//! layout is pinned, canonical, and versioned — the E4/E5 discipline applied at
//! the outer layer: a deterministic fixed-order big-endian encoding, refusal on
//! an unknown manifest version, golden-tested.
//!
//! # Two walls, restated where the identity is minted
//!
//! Integrity never substitutes validity. This manifest hash is the **outer**
//! wall — it addresses and authenticates the bytes, nothing more. It binds the
//! inner format version precisely so the identity commits to *which* canonical
//! inner encoding the records carry, but it does **not** re-check them: K2/E3
//! replay re-derives every typing and well-formedness obligation from the
//! canonical inner bytes (`gandr_kernel_core::read`). A matching identity
//! proves provenance, never validity; the hash is untrusted plumbing.

use alloc::vec::Vec;

use gandr_storage_chunker::PARAMETER_COMMITMENT_LEN;
use gandr_storage_prolly_trees::NODE_HASH_LEN;
use gandr_storage_prolly_trees::NodeHash;

use crate::error::ManifestError;

/// Domain-separation magic for the gandr artifact manifest.
pub const MANIFEST_MAGIC: &[u8] = b"gandr:artifact-manifest:v1";

/// The current manifest layout version — the outer E4/E5 plane, independent of
/// the inner kernel export version the manifest binds.
pub const MANIFEST_FORMAT_VERSION_V1: u16 = 1;

/// The fixed byte length of the chunker parameter commitment a manifest binds
/// (the 85-byte fixed-order commitment; single source of truth in the chunker).
pub const CHUNKER_COMMITMENT_LEN: usize = PARAMETER_COMMITMENT_LEN;

/// The byte length of an [`ArtifactIdentity`] (a BLAKE3 digest).
pub const ARTIFACT_IDENTITY_LEN: usize = 32;

/// The manifest layout version committed by the outer identity plane.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ManifestFormatVersion(u16);

impl ManifestFormatVersion
{
    /// The canonical v1 manifest layout.
    pub const V1: Self = Self(MANIFEST_FORMAT_VERSION_V1);
}

impl From<ManifestFormatVersion> for u16
{
    #[inline]
    fn from(version: ManifestFormatVersion) -> Self
    {
        return version.0;
    }
}

/// The kernel export format version bound by an artifact manifest.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InnerFormatVersion(u16);

impl From<u16> for InnerFormatVersion
{
    #[inline]
    fn from(version: u16) -> Self
    {
        return Self(version);
    }
}

impl From<InnerFormatVersion> for u16
{
    #[inline]
    fn from(version: InnerFormatVersion) -> Self
    {
        return version.0;
    }
}

/// The number of declaration records committed by an artifact.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArtifactRecordCount(u64);

impl From<u64> for ArtifactRecordCount
{
    #[inline]
    fn from(count: u64) -> Self
    {
        return Self(count);
    }
}

impl From<ArtifactRecordCount> for u64
{
    #[inline]
    fn from(count: ArtifactRecordCount) -> Self
    {
        return count.0;
    }
}

/// The fixed-width chunker parameter commitment bound by a manifest.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChunkerCommitment([u8; CHUNKER_COMMITMENT_LEN]);

impl From<[u8; CHUNKER_COMMITMENT_LEN]> for ChunkerCommitment
{
    #[inline]
    fn from(bytes: [u8; CHUNKER_COMMITMENT_LEN]) -> Self
    {
        return Self(bytes);
    }
}

impl TryFrom<&[u8]> for ChunkerCommitment
{
    type Error = ManifestError;

    #[inline]
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error>
    {
        let commitment = <[u8; CHUNKER_COMMITMENT_LEN]>::try_from(bytes).map_err(|_error| {
            ManifestError::CommitmentLength {
                found: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                expected: CHUNKER_COMMITMENT_LEN,
            }
        })?;
        return Ok(Self(commitment));
    }
}

impl AsRef<[u8]> for ChunkerCommitment
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return &self.0;
    }
}

/// Borrowed bytes offered to the manifest decoder.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ManifestBytes<'bytes>(&'bytes [u8]);

impl<'bytes> From<&'bytes [u8]> for ManifestBytes<'bytes>
{
    #[inline]
    fn from(bytes: &'bytes [u8]) -> Self
    {
        return Self(bytes);
    }
}

impl AsRef<[u8]> for ManifestBytes<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0;
    }
}

/// Owned canonical bytes emitted by manifest encoding.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedManifest(Vec<u8>);

impl AsRef<[u8]> for EncodedManifest
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0.as_slice();
    }
}

impl AsMut<[u8]> for EncodedManifest
{
    #[inline]
    fn as_mut(&mut self) -> &mut [u8]
    {
        return self.0.as_mut_slice();
    }
}

impl core::ops::Deref for EncodedManifest
{
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        return self.0.as_slice();
    }
}

impl core::ops::DerefMut for EncodedManifest
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        return self.0.as_mut_slice();
    }
}

/// Number of bytes requested from the manifest cursor.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ByteCount(usize);

/// One big-endian 16-bit word read from the manifest wire image.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct WireU16(u16);

/// One big-endian 64-bit word read from the manifest wire image.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct WireU64(u64);

/// A borrowed span read from the manifest wire image.
#[repr(transparent)]
#[derive(Clone, Copy)]
struct ManifestSlice<'bytes>(&'bytes [u8]);

/// Whether unread bytes remain after decoding the manifest fields.
#[derive(Clone, Copy, Eq, PartialEq)]
enum CursorState
{
    /// Every byte was consumed.
    Exhausted,
    /// At least one trailing byte remains.
    Trailing,
}

/// The content address of an artifact — `BLAKE3` of its canonical manifest.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArtifactIdentity(
    /// The raw BLAKE3 digest bytes.
    [u8; ARTIFACT_IDENTITY_LEN],
);

impl From<[u8; ARTIFACT_IDENTITY_LEN]> for ArtifactIdentity
{
    #[inline]
    fn from(bytes: [u8; ARTIFACT_IDENTITY_LEN]) -> Self
    {
        return Self(bytes);
    }
}

impl AsRef<[u8; ARTIFACT_IDENTITY_LEN]> for ArtifactIdentity
{
    #[inline]
    fn as_ref(&self) -> &[u8; ARTIFACT_IDENTITY_LEN]
    {
        return &self.0;
    }
}

impl AsRef<[u8]> for ArtifactIdentity
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return &self.0;
    }
}

impl core::fmt::Debug for ArtifactIdentity
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

impl core::fmt::Display for ArtifactIdentity
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

/// A canonical, versioned artifact manifest binding the four identity fields.
///
/// # Contract
/// - requires: constructed through [`Self::new`] from validated semantic
///   carriers or [`Self::decode`] from an arbitrary byte image.
/// - ensures: [`Self::encode`] is a deterministic fixed-order big-endian byte
///   image, and [`Self::decode`] is its exact inverse over the closed
///   [`ManifestError`] vocabulary; [`Self::identity`] is `BLAKE3(encode())`.
/// - provides: the outer content address and its round-trippable byte form.
/// - fails: [`ManifestError`] on a malformed or unsupported-version buffer.
/// - panics: none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactManifest
{
    /// The manifest layout version (the outer plane).
    manifest_version: ManifestFormatVersion,
    /// The inner kernel export format version the records were cut from.
    inner_format_version: InnerFormatVersion,
    /// The 85-byte chunker parameter commitment.
    chunker_commitment: ChunkerCommitment,
    /// The number of declaration records the tree represents.
    record_count: ArtifactRecordCount,
    /// The prolly-tree root node hash over the sorted declaration records.
    root_node_hash: NodeHash,
}

impl ArtifactManifest
{
    /// Builds a v1 manifest from the four identity fields.
    ///
    /// # Contract
    /// - requires: nothing beyond the semantic carrier invariants.
    /// - ensures: a v1 manifest whose [`Self::encode`]/[`Self::identity`] are
    ///   deterministic functions of the four fields.
    /// - provides: the outer identity binding.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub const fn new(
        inner_format_version: InnerFormatVersion,
        chunker_commitment: ChunkerCommitment,
        record_count: ArtifactRecordCount,
        root_node_hash: NodeHash,
    ) -> Self
    {
        return Self {
            manifest_version: ManifestFormatVersion::V1,
            inner_format_version,
            chunker_commitment,
            record_count,
            root_node_hash,
        };
    }

    /// Returns the manifest layout version.
    #[inline]
    #[must_use]
    pub const fn manifest_version(&self) -> ManifestFormatVersion
    {
        return self.manifest_version;
    }

    /// Returns the inner kernel export format version bound by this manifest.
    #[inline]
    #[must_use]
    pub const fn inner_format_version(&self) -> InnerFormatVersion
    {
        return self.inner_format_version;
    }

    /// Returns the committed chunker parameter commitment.
    #[inline]
    #[must_use]
    pub const fn chunker_commitment(&self) -> &ChunkerCommitment
    {
        return &self.chunker_commitment;
    }

    /// Returns the committed declaration-record count.
    #[inline]
    #[must_use]
    pub const fn record_count(&self) -> ArtifactRecordCount
    {
        return self.record_count;
    }

    /// Returns the committed prolly-tree root node hash.
    #[inline]
    #[must_use]
    pub const fn root_node_hash(&self) -> NodeHash
    {
        return self.root_node_hash;
    }

    /// Encodes the manifest to its canonical fixed-order big-endian byte image.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: a deterministic byte image — magic, the manifest and inner
    ///   version words, a length-prefixed chunker commitment, the record count,
    ///   and the root node hash, all big-endian — that [`Self::decode`] inverts
    ///   exactly.
    /// - provides: the hashed bytes behind [`Self::identity`].
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn encode(&self) -> EncodedManifest
    {
        let mut out = Vec::new();
        out.extend_from_slice(MANIFEST_MAGIC);
        out.extend_from_slice(&u16::from(self.manifest_version).to_be_bytes());
        out.extend_from_slice(&u16::from(self.inner_format_version).to_be_bytes());
        let commitment = self.chunker_commitment.as_ref();
        let commitment_len = u16::try_from(commitment.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&commitment_len.to_be_bytes());
        out.extend_from_slice(commitment);
        out.extend_from_slice(&u64::from(self.record_count).to_be_bytes());
        out.extend_from_slice(self.root_node_hash.as_ref());
        return EncodedManifest(out);
    }

    /// Decodes a canonical manifest byte image, refusing anything else.
    ///
    /// # Contract
    /// - requires: nothing — `bytes` may be arbitrary/adversarial.
    /// - ensures: `Ok(manifest)` exactly when `bytes` is the canonical encoding
    ///   of a v1 manifest with the fixed-length commitment; the manifest
    ///   re-encodes to `bytes`.
    /// - provides: the outer E4/E5 wall — a total, closed-vocabulary reader.
    /// - fails: [`ManifestError`] — bad magic, an unsupported manifest version
    ///   (E5), truncation, a bad commitment length, or trailing bytes.
    /// - panics: none.
    ///
    /// # Errors
    /// Any [`ManifestError`].
    #[inline]
    pub fn decode(bytes: ManifestBytes<'_>) -> Result<Self, ManifestError>
    {
        let mut cursor = ManifestCursor::new(bytes);
        cursor.expect_magic()?;
        let manifest_version = cursor.read_u16()?;
        let manifest_version = manifest_version.0;
        if manifest_version != MANIFEST_FORMAT_VERSION_V1 {
            return Err(ManifestError::UnsupportedManifestVersion {
                found: manifest_version,
            });
        }
        let inner_format_version = cursor.read_u16()?;
        let inner_format_version = InnerFormatVersion(inner_format_version.0);
        let commitment_len = cursor.read_u16()?;
        let commitment_len = commitment_len.0;
        if usize::from(commitment_len) != CHUNKER_COMMITMENT_LEN {
            return Err(ManifestError::CommitmentLength {
                found: u64::from(commitment_len),
                expected: CHUNKER_COMMITMENT_LEN,
            });
        }
        let chunker_commitment = cursor.read_commitment()?;
        let record_count = cursor.read_u64()?;
        let record_count = ArtifactRecordCount(record_count.0);
        let root_node_hash = cursor.read_node_hash()?;
        if cursor.state() == CursorState::Trailing {
            return Err(ManifestError::TrailingBytes);
        }

        return Ok(Self {
            manifest_version: ManifestFormatVersion(manifest_version),
            inner_format_version,
            chunker_commitment,
            record_count,
            root_node_hash,
        });
    }

    /// Returns the artifact identity — `BLAKE3` of the canonical manifest
    /// bytes.
    #[inline]
    #[must_use]
    pub fn identity(&self) -> ArtifactIdentity
    {
        let bytes = self.encode();
        return ArtifactIdentity::from(*blake3::hash(bytes.as_ref()).as_bytes());
    }
}

/// A forward byte cursor with bounds-checked reads — the manifest reader's
/// totality substrate (an over-read surfaces [`ManifestError::Truncated`]).
struct ManifestCursor<'bytes>
{
    /// The manifest byte image being decoded.
    bytes: ManifestBytes<'bytes>,
    /// The next unread offset.
    position: usize,
}

impl<'bytes> ManifestCursor<'bytes>
{
    /// A cursor at the start of `bytes`.
    #[inline]
    const fn new(bytes: ManifestBytes<'bytes>) -> Self
    {
        return Self { bytes, position: 0 };
    }

    /// Whether every byte has been consumed.
    #[inline]
    fn state(&self) -> CursorState
    {
        if self.position >= self.bytes.as_ref().len() {
            return CursorState::Exhausted;
        }
        return CursorState::Trailing;
    }

    /// Read `count` bytes as a borrowed span, or
    /// [`ManifestError::Truncated`].
    #[inline]
    fn take(
        &mut self,
        count: ByteCount,
    ) -> Result<ManifestSlice<'bytes>, ManifestError>
    {
        let end = self
            .position
            .checked_add(count.0)
            .ok_or(ManifestError::Truncated)?;
        let slice = self
            .bytes
            .0
            .get(self.position .. end)
            .ok_or(ManifestError::Truncated)?;
        self.position = end;
        return Ok(ManifestSlice(slice));
    }

    /// Verify the domain magic, or [`ManifestError::BadMagic`] / `Truncated`.
    #[inline]
    fn expect_magic(&mut self) -> Result<(), ManifestError>
    {
        let head = self.take(ByteCount(MANIFEST_MAGIC.len()))?;
        if head.0 == MANIFEST_MAGIC {
            return Ok(());
        }

        return Err(ManifestError::BadMagic);
    }

    /// Read a big-endian 16-bit wire word.
    #[inline]
    fn read_u16(&mut self) -> Result<WireU16, ManifestError>
    {
        let bytes = self.take(ByteCount(2))?;
        let array = <[u8; 2]>::try_from(bytes.0).map_err(|_error| ManifestError::Truncated)?;
        return Ok(WireU16(u16::from_be_bytes(array)));
    }

    /// Read a big-endian 64-bit wire word.
    #[inline]
    fn read_u64(&mut self) -> Result<WireU64, ManifestError>
    {
        let bytes = self.take(ByteCount(8))?;
        let array = <[u8; 8]>::try_from(bytes.0).map_err(|_error| ManifestError::Truncated)?;
        return Ok(WireU64(u64::from_be_bytes(array)));
    }

    /// Read the fixed-length chunker commitment.
    #[inline]
    fn read_commitment(&mut self) -> Result<ChunkerCommitment, ManifestError>
    {
        let bytes = self.take(ByteCount(CHUNKER_COMMITMENT_LEN))?;
        let array = <[u8; CHUNKER_COMMITMENT_LEN]>::try_from(bytes.0)
            .map_err(|_error| ManifestError::Truncated)?;
        return Ok(ChunkerCommitment(array));
    }

    /// Read the fixed-length root node hash.
    #[inline]
    fn read_node_hash(&mut self) -> Result<NodeHash, ManifestError>
    {
        let bytes = self.take(ByteCount(NODE_HASH_LEN))?;
        let array =
            <[u8; NODE_HASH_LEN]>::try_from(bytes.0).map_err(|_error| ManifestError::Truncated)?;
        return Ok(NodeHash::from(array));
    }
}

/// The manifest canonicality, versioned-decode, and identity witnesses.
#[cfg(test)]
mod tests
{
    use alloc::vec::Vec;

    use super::ARTIFACT_IDENTITY_LEN;
    use super::ArtifactManifest;
    use super::CHUNKER_COMMITMENT_LEN;
    use super::MANIFEST_FORMAT_VERSION_V1;
    use super::MANIFEST_MAGIC;
    use super::NodeHash;
    use crate::error::ManifestError;

    /// The big-endian wire form of the 85-byte commitment length.
    const COMMITMENT_LEN_WIRE: [u8; 2] = {
        let wide = CHUNKER_COMMITMENT_LEN.to_be_bytes();
        [wide[6], wide[7]]
    };

    /// A fixed manifest whose canonical bytes are hand-verifiable.
    fn golden_manifest() -> ArtifactManifest
    {
        let commitment = [0_u8; CHUNKER_COMMITMENT_LEN];
        let root = NodeHash::from([0_u8; ARTIFACT_IDENTITY_LEN]);
        return ArtifactManifest::new(
            MANIFEST_FORMAT_VERSION_V1.into(),
            commitment.into(),
            0_u64.into(),
            root,
        );
    }

    /// The canonical manifest byte layout is pinned (the E4/E5 golden): magic,
    /// the manifest and inner version words, a length-prefixed 85-byte
    /// commitment, the record count, and the root node hash, all big-endian.
    #[test]
    fn the_manifest_layout_is_golden()
    {
        let manifest = golden_manifest();
        let encoded = manifest.encode();

        let mut expected: Vec<u8> = Vec::new();
        expected.extend_from_slice(MANIFEST_MAGIC);
        expected.extend_from_slice(&[0x00, 0x01]); // manifest version 1
        expected.extend_from_slice(&[0x00, 0x01]); // inner format version 1
        expected.extend_from_slice(&COMMITMENT_LEN_WIRE); // commitment length 85
        expected.extend_from_slice(&[0_u8; CHUNKER_COMMITMENT_LEN]);
        expected.extend_from_slice(&[0_u8; 8]); // record count 0
        expected.extend_from_slice(&[0_u8; ARTIFACT_IDENTITY_LEN]); // root node hash

        assert_eq!(
            encoded.as_ref(),
            expected.as_slice(),
            "the canonical manifest layout is pinned"
        );
        assert_eq!(
            157,
            encoded.len(),
            "the fixed manifest is exactly 157 bytes"
        );
    }

    /// Encoding then decoding a manifest reproduces it exactly.
    #[test]
    fn the_manifest_round_trips_through_decode()
    {
        let manifest = golden_manifest();
        let decoded = ArtifactManifest::decode(manifest.encode().as_ref().into())
            .expect("the golden decodes");
        assert_eq!(decoded, manifest, "decode inverts encode");
    }

    /// The identity is BLAKE3 of the canonical manifest bytes, and is
    /// deterministic.
    #[test]
    fn the_identity_is_blake3_of_the_canonical_bytes()
    {
        let manifest = golden_manifest();
        let expected = blake3::hash(manifest.encode().as_ref());
        let identity = manifest.identity();
        let identity_bytes: &[u8; ARTIFACT_IDENTITY_LEN] = identity.as_ref();
        assert_eq!(
            identity_bytes,
            expected.as_bytes(),
            "the identity is BLAKE3 of the manifest bytes"
        );
        assert_eq!(
            manifest.identity(),
            golden_manifest().identity(),
            "the identity is deterministic"
        );
    }

    /// Any single-field perturbation changes the identity (identity
    /// sensitivity at the manifest plane).
    #[test]
    fn any_field_perturbation_changes_the_identity()
    {
        let base = golden_manifest();
        let base_identity = base.identity();

        let other_inner = ArtifactManifest::new(
            MANIFEST_FORMAT_VERSION_V1.wrapping_add(1).into(),
            [0_u8; CHUNKER_COMMITMENT_LEN].into(),
            0_u64.into(),
            NodeHash::from([0_u8; ARTIFACT_IDENTITY_LEN]),
        );
        assert_ne!(
            other_inner.identity(),
            base_identity,
            "a different inner version changes the identity"
        );

        let mut perturbed_commitment = [0_u8; CHUNKER_COMMITMENT_LEN];
        perturbed_commitment[0] = 1;
        let other_commitment = ArtifactManifest::new(
            MANIFEST_FORMAT_VERSION_V1.into(),
            perturbed_commitment.into(),
            0_u64.into(),
            NodeHash::from([0_u8; ARTIFACT_IDENTITY_LEN]),
        );
        assert_ne!(
            other_commitment.identity(),
            base_identity,
            "a different chunker commitment changes the identity"
        );

        let other_count = ArtifactManifest::new(
            MANIFEST_FORMAT_VERSION_V1.into(),
            [0_u8; CHUNKER_COMMITMENT_LEN].into(),
            1_u64.into(),
            NodeHash::from([0_u8; ARTIFACT_IDENTITY_LEN]),
        );
        assert_ne!(
            other_count.identity(),
            base_identity,
            "a different record count changes the identity"
        );

        let mut perturbed_hash = [0_u8; ARTIFACT_IDENTITY_LEN];
        perturbed_hash[0] = 1;
        let other_hash = ArtifactManifest::new(
            MANIFEST_FORMAT_VERSION_V1.into(),
            [0_u8; CHUNKER_COMMITMENT_LEN].into(),
            0_u64.into(),
            NodeHash::from(perturbed_hash),
        );
        assert_ne!(
            other_hash.identity(),
            base_identity,
            "a different root node hash changes the identity"
        );
    }

    /// An unknown manifest layout version is refused by name (E5).
    #[test]
    fn an_unknown_manifest_version_is_refused()
    {
        let mut bytes = golden_manifest().encode();
        // The manifest version word immediately follows the magic.
        let version_offset = MANIFEST_MAGIC.len();
        bytes[version_offset] = 0x00;
        bytes[version_offset.wrapping_add(1)] = 0x02;
        assert_eq!(
            Err(ManifestError::UnsupportedManifestVersion { found: 2 }),
            ArtifactManifest::decode(bytes.as_ref().into()),
            "an unknown manifest version is a named refusal"
        );
    }

    /// A wrong magic, a truncated buffer, and a trailing byte each reject.
    #[test]
    fn a_malformed_manifest_is_rejected()
    {
        let good = golden_manifest().encode();

        let mut bad_magic = good.clone();
        bad_magic[0] = b'X';
        assert_eq!(
            Err(ManifestError::BadMagic),
            ArtifactManifest::decode(bad_magic.as_ref().into()),
            "a wrong magic rejects"
        );

        let truncated = good
            .get(.. good.len().wrapping_sub(1))
            .expect("nonempty")
            .to_vec();
        assert_eq!(
            Err(ManifestError::Truncated),
            ArtifactManifest::decode(truncated.as_slice().into()),
            "a truncated buffer rejects"
        );

        let mut trailing = good.as_ref().to_vec();
        trailing.push(0x00);
        assert_eq!(
            Err(ManifestError::TrailingBytes),
            ArtifactManifest::decode(trailing.as_slice().into()),
            "a trailing byte rejects"
        );
    }

    /// A wrong committed commitment length rejects.
    #[test]
    fn a_bad_commitment_length_is_rejected()
    {
        let mut bytes = golden_manifest().encode();
        // The commitment length word follows magic + two version words.
        let length_offset = MANIFEST_MAGIC.len().wrapping_add(4);
        bytes[length_offset] = COMMITMENT_LEN_WIRE[0];
        bytes[length_offset.wrapping_add(1)] = COMMITMENT_LEN_WIRE[1].wrapping_sub(1); // 84, not 85
        match ArtifactManifest::decode(bytes.as_ref().into()) {
            | Err(ManifestError::CommitmentLength { found, expected }) => {
                assert_eq!(84, found);
                assert_eq!(CHUNKER_COMMITMENT_LEN, expected);
            },
            | other => panic!("expected a commitment-length refusal, got {other:?}"),
        }
    }

    /// Truncation at every prefix rejects rather than panicking (totality).
    #[test]
    fn truncation_at_every_prefix_is_rejected()
    {
        let good = golden_manifest().encode();
        for prefix_len in 0 .. good.len() {
            let prefix = good.get(.. prefix_len).expect("prefix within bounds");
            assert!(
                ArtifactManifest::decode(prefix.into()).is_err(),
                "every proper prefix is rejected, never accepted or panicking"
            );
        }
    }
}
