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

/// Why constructing or ingesting a transport-step identity failed.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum StepIdError
{
    /// A step-identity byte image was not exactly the fixed width — including
    /// the 16-byte width of the certificate layer's in-process labels, which
    /// can never decode as a transport identity.
    #[error("step-identity image length {found} is not the expected {expected}")]
    ImageLength
    {
        /// The image length offered.
        found: usize,
        /// The fixed length the reader requires.
        expected: usize,
    },

    /// A count, length, or position step did not fit the canonical u64 width.
    #[error("the value {found} does not fit the canonical u64 width")]
    WidthOverflow
    {
        /// The offending value.
        found: usize,
    },
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

/// Why a value-plane commit or dereference failed.
///
/// The vocabulary separates the three ways a content-addressed read can go
/// wrong, because collapsing them is how a storage bug reads as a decode bug:
/// the store did not have it ([`ValueError::UnknownChunk`]), the store had
/// something else under that name ([`ValueError::DigestMismatch`]), or the
/// bytes were not a chunk at all ([`ValueError::MalformedChunk`]).
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum ValueError
{
    /// A digest byte image was not exactly the fixed width.
    #[error("chunk-digest image length {found} is not the expected {expected}")]
    DigestLength
    {
        /// The image length offered.
        found: usize,
        /// The fixed length the reader requires.
        expected: usize,
    },

    /// The store held no chunk under the requested digest.
    #[error("no chunk stored under digest {digest}")]
    UnknownChunk
    {
        /// The digest that was asked for.
        digest: alloc::string::String,
    },

    /// Stored or offered bytes did not hash to their claimed digest.
    #[error("chunk bytes hash to {actual}, not the claimed {expected}")]
    DigestMismatch
    {
        /// The digest the bytes were claimed to have.
        expected: alloc::string::String,
        /// The digest the bytes actually have.
        actual: alloc::string::String,
    },

    /// A chunk image did not parse as the framed layout.
    #[error("malformed chunk image: {context}")]
    MalformedChunk
    {
        /// Which part of the frame was wrong.
        context: &'static str,
    },

    /// A token stream ended inside a value.
    #[error("chunk token stream truncated at token {position}")]
    TruncatedChunk
    {
        /// The token index the reader stopped at.
        position: u32,
    },

    /// A token of the wrong kind stood where the codec required another.
    ///
    /// This is the value plane's **wrong-kind** rejection, and it is a named
    /// variant rather than a generic decode failure for a specific reason: a
    /// wrong-kind inhabitant that decodes anyway is the defect class that
    /// passes every test written against the same wrong picture.
    #[error("expected a {expected} token at token {position}, found a {found} token")]
    UnexpectedToken
    {
        /// The token kind the codec required.
        expected: &'static str,
        /// The token kind actually present.
        found: &'static str,
        /// The token index.
        position: u32,
    },

    /// A commit was asked for a child-reference representation the value
    /// plane's token stream cannot have.
    ///
    /// The stream nests children in place rather than numbering them, so there
    /// is no index to re-base. Refused rather than ignored, because accepting
    /// it would let a manifest claim a representation that does not exist.
    #[error("the value plane's token stream has no child indices to re-base")]
    UnsupportedIndexBase,

    /// A count or length did not fit its canonical width.
    #[error("the value {found} does not fit the canonical {width}-bit width")]
    WidthOverflow
    {
        /// The offending value.
        found: u64,
        /// The canonical width it did not fit.
        width: u32,
    },
}
