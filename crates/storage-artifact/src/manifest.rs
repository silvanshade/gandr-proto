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

/// The content address of an artifact — `BLAKE3` of its canonical manifest.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArtifactIdentity(
    /// The raw BLAKE3 digest bytes.
    [u8; ARTIFACT_IDENTITY_LEN],
);

impl ArtifactIdentity
{
    /// Creates an identity from raw digest bytes.
    #[inline]
    #[must_use]
    pub const fn from_bytes(bytes: [u8; ARTIFACT_IDENTITY_LEN]) -> Self
    {
        return Self(bytes);
    }

    /// Returns the digest as a fixed-size byte array.
    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; ARTIFACT_IDENTITY_LEN]
    {
        return &self.0;
    }

    /// Returns the digest as a byte slice.
    #[inline]
    #[must_use]
    pub const fn as_slice(&self) -> &[u8]
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
/// - requires: constructed through [`Self::new`] (which validates the
///   commitment length) or [`Self::decode`] (which validates the whole byte
///   image).
/// - ensures: [`Self::encode`] is a deterministic fixed-order big-endian byte
///   image, and [`Self::decode`] is its exact inverse over the closed
///   [`ManifestError`] vocabulary; [`Self::identity`] is `BLAKE3(encode())`.
/// - provides: the outer content address and its round-trippable byte form.
/// - fails: [`ManifestError`] on a malformed or unsupported-version buffer.
/// - panics: none.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ArtifactManifest
{
    /// The manifest layout version (the outer plane).
    manifest_version: u16,
    /// The inner kernel export format version the records were cut from.
    inner_format_version: u16,
    /// The 85-byte chunker parameter commitment.
    chunker_commitment: [u8; CHUNKER_COMMITMENT_LEN],
    /// The number of declaration records the tree represents.
    record_count: u64,
    /// The prolly-tree root node hash over the sorted declaration records.
    root_node_hash: NodeHash,
}

impl ArtifactManifest
{
    /// Builds a v1 manifest from the four identity fields.
    ///
    /// # Contract
    /// - requires: `chunker_commitment` is the fixed-length commitment.
    /// - ensures: a v1 manifest whose [`Self::encode`]/[`Self::identity`] are
    ///   deterministic functions of the four fields.
    /// - provides: the outer identity binding.
    /// - fails: [`ManifestError::CommitmentLength`] when the commitment is not
    ///   the fixed length.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ManifestError::CommitmentLength`].
    #[inline]
    pub fn new(
        inner_format_version: u16,
        chunker_commitment: &[u8],
        record_count: u64,
        root_node_hash: NodeHash,
    ) -> Result<Self, ManifestError>
    {
        let commitment =
            <[u8; CHUNKER_COMMITMENT_LEN]>::try_from(chunker_commitment).map_err(|_error| {
                ManifestError::CommitmentLength {
                    found: u64::try_from(chunker_commitment.len()).unwrap_or(u64::MAX),
                    expected: CHUNKER_COMMITMENT_LEN,
                }
            })?;

        return Ok(Self {
            manifest_version: MANIFEST_FORMAT_VERSION_V1,
            inner_format_version,
            chunker_commitment: commitment,
            record_count,
            root_node_hash,
        });
    }

    /// Returns the manifest layout version.
    #[inline]
    #[must_use]
    pub const fn manifest_version(&self) -> u16
    {
        return self.manifest_version;
    }

    /// Returns the inner kernel export format version bound by this manifest.
    #[inline]
    #[must_use]
    pub const fn inner_format_version(&self) -> u16
    {
        return self.inner_format_version;
    }

    /// Returns the committed chunker parameter commitment bytes.
    #[inline]
    #[must_use]
    pub const fn chunker_commitment(&self) -> &[u8]
    {
        return &self.chunker_commitment;
    }

    /// Returns the committed declaration-record count.
    #[inline]
    #[must_use]
    pub const fn record_count(&self) -> u64
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
    pub fn encode(&self) -> Vec<u8>
    {
        let mut out = Vec::new();
        out.extend_from_slice(MANIFEST_MAGIC);
        out.extend_from_slice(&self.manifest_version.to_be_bytes());
        out.extend_from_slice(&self.inner_format_version.to_be_bytes());
        let commitment_len = u16::try_from(self.chunker_commitment.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&commitment_len.to_be_bytes());
        out.extend_from_slice(&self.chunker_commitment);
        out.extend_from_slice(&self.record_count.to_be_bytes());
        out.extend_from_slice(self.root_node_hash.as_slice());
        return out;
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
    pub fn decode(bytes: &[u8]) -> Result<Self, ManifestError>
    {
        let mut cursor = ManifestCursor::new(bytes);
        cursor.expect_magic()?;
        let manifest_version = cursor.read_u16()?;
        if manifest_version != MANIFEST_FORMAT_VERSION_V1 {
            return Err(ManifestError::UnsupportedManifestVersion {
                found: manifest_version,
            });
        }
        let inner_format_version = cursor.read_u16()?;
        let commitment_len = cursor.read_u16()?;
        if usize::from(commitment_len) != CHUNKER_COMMITMENT_LEN {
            return Err(ManifestError::CommitmentLength {
                found: u64::from(commitment_len),
                expected: CHUNKER_COMMITMENT_LEN,
            });
        }
        let chunker_commitment = cursor.read_commitment()?;
        let record_count = cursor.read_u64()?;
        let root_node_hash = cursor.read_node_hash()?;
        if !cursor.at_end() {
            return Err(ManifestError::TrailingBytes);
        }

        return Ok(Self {
            manifest_version,
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
        return ArtifactIdentity::from_bytes(*blake3::hash(bytes.as_slice()).as_bytes());
    }
}

/// A forward byte cursor with bounds-checked reads — the manifest reader's
/// totality substrate (an over-read surfaces [`ManifestError::Truncated`]).
struct ManifestCursor<'bytes>
{
    /// The manifest bytes.
    bytes: &'bytes [u8],
    /// The next unread offset.
    position: usize,
}

impl<'bytes> ManifestCursor<'bytes>
{
    /// A cursor at the start of `bytes`.
    #[inline]
    const fn new(bytes: &'bytes [u8]) -> Self
    {
        return Self { bytes, position: 0 };
    }

    /// Whether every byte has been consumed.
    #[inline]
    const fn at_end(&self) -> bool
    {
        return self.position >= self.bytes.len();
    }

    /// Read `count` bytes as a borrowed slice, or [`ManifestError::Truncated`].
    #[inline]
    fn take(
        &mut self,
        count: usize,
    ) -> Result<&'bytes [u8], ManifestError>
    {
        let end = self
            .position
            .checked_add(count)
            .ok_or(ManifestError::Truncated)?;
        let slice = self
            .bytes
            .get(self.position .. end)
            .ok_or(ManifestError::Truncated)?;
        self.position = end;
        return Ok(slice);
    }

    /// Verify the domain magic, or [`ManifestError::BadMagic`] / `Truncated`.
    #[inline]
    fn expect_magic(&mut self) -> Result<(), ManifestError>
    {
        let head = self.take(MANIFEST_MAGIC.len())?;
        if head == MANIFEST_MAGIC {
            return Ok(());
        }

        return Err(ManifestError::BadMagic);
    }

    /// Read a big-endian `u16`.
    #[inline]
    fn read_u16(&mut self) -> Result<u16, ManifestError>
    {
        let bytes = self.take(2)?;
        let array = <[u8; 2]>::try_from(bytes).map_err(|_error| ManifestError::Truncated)?;
        return Ok(u16::from_be_bytes(array));
    }

    /// Read a big-endian `u64`.
    #[inline]
    fn read_u64(&mut self) -> Result<u64, ManifestError>
    {
        let bytes = self.take(8)?;
        let array = <[u8; 8]>::try_from(bytes).map_err(|_error| ManifestError::Truncated)?;
        return Ok(u64::from_be_bytes(array));
    }

    /// Read the fixed-length chunker commitment.
    #[inline]
    fn read_commitment(&mut self) -> Result<[u8; CHUNKER_COMMITMENT_LEN], ManifestError>
    {
        let bytes = self.take(CHUNKER_COMMITMENT_LEN)?;
        return <[u8; CHUNKER_COMMITMENT_LEN]>::try_from(bytes)
            .map_err(|_error| ManifestError::Truncated);
    }

    /// Read the fixed-length root node hash.
    #[inline]
    fn read_node_hash(&mut self) -> Result<NodeHash, ManifestError>
    {
        let bytes = self.take(NODE_HASH_LEN)?;
        let array =
            <[u8; NODE_HASH_LEN]>::try_from(bytes).map_err(|_error| ManifestError::Truncated)?;
        return Ok(NodeHash::from_bytes(array));
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

    /// A fixed manifest whose canonical bytes are hand-verifiable.
    fn golden_manifest() -> ArtifactManifest
    {
        let commitment = [0_u8; CHUNKER_COMMITMENT_LEN];
        let root = NodeHash::from_bytes([0_u8; ARTIFACT_IDENTITY_LEN]);
        return ArtifactManifest::new(
            MANIFEST_FORMAT_VERSION_V1,
            commitment.as_slice(),
            0_u64,
            root,
        )
        .expect("the fixed commitment length is valid");
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
        expected.extend_from_slice(&[0x00, 0x55]); // commitment length 85
        expected.extend_from_slice(&[0_u8; CHUNKER_COMMITMENT_LEN]);
        expected.extend_from_slice(&[0_u8; 8]); // record count 0
        expected.extend_from_slice(&[0_u8; ARTIFACT_IDENTITY_LEN]); // root node hash

        assert_eq!(encoded, expected, "the canonical manifest layout is pinned");
        assert_eq!(
            encoded.len(),
            157,
            "the fixed manifest is exactly 157 bytes"
        );
    }

    /// Encoding then decoding a manifest reproduces it exactly.
    #[test]
    fn the_manifest_round_trips_through_decode()
    {
        let manifest = golden_manifest();
        let decoded =
            ArtifactManifest::decode(manifest.encode().as_slice()).expect("the golden decodes");
        assert_eq!(decoded, manifest, "decode inverts encode");
    }

    /// The identity is BLAKE3 of the canonical manifest bytes, and is
    /// deterministic.
    #[test]
    fn the_identity_is_blake3_of_the_canonical_bytes()
    {
        let manifest = golden_manifest();
        let expected = blake3::hash(manifest.encode().as_slice());
        assert_eq!(
            manifest.identity().as_bytes(),
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
            MANIFEST_FORMAT_VERSION_V1.wrapping_add(1),
            [0_u8; CHUNKER_COMMITMENT_LEN].as_slice(),
            0_u64,
            NodeHash::from_bytes([0_u8; ARTIFACT_IDENTITY_LEN]),
        )
        .expect("valid");
        assert_ne!(
            other_inner.identity(),
            base_identity,
            "a different inner version changes the identity"
        );

        let mut perturbed_commitment = [0_u8; CHUNKER_COMMITMENT_LEN];
        perturbed_commitment[0] = 1;
        let other_commitment = ArtifactManifest::new(
            MANIFEST_FORMAT_VERSION_V1,
            perturbed_commitment.as_slice(),
            0_u64,
            NodeHash::from_bytes([0_u8; ARTIFACT_IDENTITY_LEN]),
        )
        .expect("valid");
        assert_ne!(
            other_commitment.identity(),
            base_identity,
            "a different chunker commitment changes the identity"
        );

        let other_count = ArtifactManifest::new(
            MANIFEST_FORMAT_VERSION_V1,
            [0_u8; CHUNKER_COMMITMENT_LEN].as_slice(),
            1_u64,
            NodeHash::from_bytes([0_u8; ARTIFACT_IDENTITY_LEN]),
        )
        .expect("valid");
        assert_ne!(
            other_count.identity(),
            base_identity,
            "a different record count changes the identity"
        );

        let mut perturbed_hash = [0_u8; ARTIFACT_IDENTITY_LEN];
        perturbed_hash[0] = 1;
        let other_hash = ArtifactManifest::new(
            MANIFEST_FORMAT_VERSION_V1,
            [0_u8; CHUNKER_COMMITMENT_LEN].as_slice(),
            0_u64,
            NodeHash::from_bytes(perturbed_hash),
        )
        .expect("valid");
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
        bytes[version_offset + 1] = 0x02;
        assert_eq!(
            ArtifactManifest::decode(bytes.as_slice()),
            Err(ManifestError::UnsupportedManifestVersion { found: 2 }),
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
            ArtifactManifest::decode(bad_magic.as_slice()),
            Err(ManifestError::BadMagic),
            "a wrong magic rejects"
        );

        let truncated = good.get(.. good.len() - 1).expect("nonempty").to_vec();
        assert_eq!(
            ArtifactManifest::decode(truncated.as_slice()),
            Err(ManifestError::Truncated),
            "a truncated buffer rejects"
        );

        let mut trailing = good;
        trailing.push(0x00);
        assert_eq!(
            ArtifactManifest::decode(trailing.as_slice()),
            Err(ManifestError::TrailingBytes),
            "a trailing byte rejects"
        );
    }

    /// A wrong committed commitment length rejects.
    #[test]
    fn a_bad_commitment_length_is_rejected()
    {
        let mut bytes = golden_manifest().encode();
        // The commitment length word follows magic + two version words.
        let length_offset = MANIFEST_MAGIC.len() + 4;
        bytes[length_offset] = 0x00;
        bytes[length_offset + 1] = 0x54; // 84, not 85
        match ArtifactManifest::decode(bytes.as_slice()) {
            | Err(ManifestError::CommitmentLength { found, expected }) => {
                assert_eq!(found, 84);
                assert_eq!(expected, CHUNKER_COMMITMENT_LEN);
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
                ArtifactManifest::decode(prefix).is_err(),
                "every proper prefix is rejected, never accepted or panicking"
            );
        }
    }
}
