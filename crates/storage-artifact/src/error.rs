//! Error vocabulary for the outer artifact layer.
//!
//! Two planes are kept distinct, mirroring the kernel reader's decode-vs-admit
//! split: [`ManifestError`] is the manifest's own byte-decode rejection
//! vocabulary (the outer E4/E5 wall — refuse an unknown manifest version, a bad
//! magic, a truncated or over-long buffer), and [`ArtifactError`] is the
//! record-extraction and prolly-wiring plane (a duplicate admission key, a
//! record-count that overflows the width, a chunker/tree failure, or a manifest
//! decode failure surfaced through it).

use gandr_storage_prolly_trees::ProllyBaoError;
use thiserror::Error;

/// Why building or reading an artifact through the outer layer failed.
#[derive(Debug, Error)]
pub enum ArtifactError
{
    /// The prolly-tree layer (chunker boundary detection, tree construction, or
    /// store verification) rejected the records.
    #[error("prolly-tree layer error")]
    Tree(#[from] ProllyBaoError),

    /// Two declaration records carried the same admission-index key — the
    /// record set is not the sorted **unique** keyed set an artifact is by
    /// construction (massive-term design §6).
    #[error("duplicate admission key {key} at record indexes {first_index} and {second_index}")]
    DuplicateAdmissionKey
    {
        /// The repeated admission index.
        key: u64,
        /// The first record carrying the key.
        first_index: u64,
        /// The later record carrying the key.
        second_index: u64,
    },

    /// The declaration count did not fit the fixed-width record-count field.
    #[error("declaration count {count} does not fit the record-count width")]
    RecordCountOverflow
    {
        /// The offending declaration count.
        count: usize,
    },

    /// A manifest could not be decoded (the outer E4/E5 wall).
    #[error("artifact manifest error")]
    Manifest(#[from] ManifestError),
}

/// Why decoding a canonical artifact manifest failed — the outer layer's closed
/// rejection vocabulary (E4/E5 applied at the manifest boundary).
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ManifestError
{
    /// The manifest magic did not match a gandr artifact manifest.
    #[error("the manifest magic did not match")]
    BadMagic,

    /// The manifest declared a layout version this reader does not implement
    /// (E5: a named refusal, never a guess).
    #[error("unsupported manifest version {found}")]
    UnsupportedManifestVersion
    {
        /// The manifest-layout version the bytes declared.
        found: u16,
    },

    /// The manifest ended mid-field.
    #[error("the manifest bytes ended mid-field")]
    Truncated,

    /// Bytes remained after a complete manifest.
    #[error("bytes remained after a complete manifest")]
    TrailingBytes,

    /// The committed chunker parameter commitment was not the expected fixed
    /// length.
    #[error("manifest chunker commitment length {found} is not the expected {expected}")]
    CommitmentLength
    {
        /// The committed length the manifest declared.
        found: u64,
        /// The fixed length the reader requires.
        expected: usize,
    },
}
