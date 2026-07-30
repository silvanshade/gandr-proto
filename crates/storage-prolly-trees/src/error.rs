//! Error vocabulary for Prolly-Bao tree construction and verification.

use thiserror::Error;

use crate::types::NodeHash;

/// Result error type for Prolly-Bao operations.
#[derive(Debug, Error)]
pub enum ProllyBaoError
{
    /// Input records were not in strict key order.
    #[error(
        "input records are unsorted between record indexes {previous_index} and {current_index}"
    )]
    UnsortedInput
    {
        /// Index of the previous record in the caller-provided stream.
        previous_index: u64,
        /// Index of the first record observed out of order.
        current_index: u64,
    },

    /// Two input records used the same canonical key.
    #[error("duplicate key at record indexes {first_index} and {second_index}")]
    DuplicateKeys
    {
        /// Index of the first record carrying the duplicate key.
        first_index: u64,
        /// Index of the later record carrying the duplicate key.
        second_index: u64,
    },

    /// Encoded node bytes did not match the selected encoding version.
    #[error("node bytes are malformed: {context}")]
    MalformedNodeBytes
    {
        /// Static context naming the malformed portion.
        context: &'static str,
    },

    /// A requested node hash was absent from the backing store.
    #[error("node hash is unknown: {hash}")]
    UnknownNodeHash
    {
        /// Missing node hash.
        hash: NodeHash,
    },

    /// Encoded node bytes did not hash to the expected BLAKE3 node identity.
    #[error("node hash mismatch: expected {expected}, actual {actual}")]
    HashMismatch
    {
        /// Expected opaque node hash.
        expected: NodeHash,
        /// Actual opaque node hash computed from bytes.
        actual: NodeHash,
    },

    /// Tree parameters were incompatible with a root, node, store, or proof.
    #[error("tree parameters are incompatible: {context}")]
    IncompatibleTreeParameters
    {
        /// Static context naming the incompatible parameter set.
        context: &'static str,
    },

    /// Encoded bytes or proof material named an unsupported encoding version.
    #[error("encoding version is unsupported: {version}")]
    UnsupportedEncodingVersion
    {
        /// Raw version value from encoded material.
        version: u16,
    },

    /// Native witness transcript bytes were malformed.
    #[error("witness bytes are malformed: {context}")]
    MalformedWitnessBytes
    {
        /// Static context naming the malformed transcript portion.
        context: &'static str,
    },

    /// Native witness transcript bytes named an unsupported witness version.
    #[error("witness version is unsupported: {version}")]
    UnsupportedWitnessVersion
    {
        /// Raw version value from encoded witness material.
        version: u16,
    },

    /// Canonical snapshot byte-stream bytes were malformed.
    #[error("snapshot bytes are malformed: {context}")]
    MalformedSnapshotBytes
    {
        /// Static context naming the malformed snapshot portion.
        context: &'static str,
    },

    /// Canonical snapshot byte-stream bytes named an unsupported version.
    #[error("snapshot version is unsupported: {version}")]
    UnsupportedSnapshotVersion
    {
        /// Raw version value from encoded snapshot material.
        version: u16,
    },

    /// Proof material had an invalid shape for the requested verification.
    #[error("proof shape is invalid: {context}")]
    InvalidProofShape
    {
        /// Static context naming the invalid proof portion.
        context: &'static str,
    },

    /// Range bounds were invalid for ordered-record traversal.
    #[error("range bounds are invalid: {context}")]
    RangeBound
    {
        /// Static context naming the invalid range relation.
        context: &'static str,
    },

    /// Chunker parameter validation or boundary detection failed.
    #[error("chunker error")]
    Chunker(#[from] gandr_storage_chunker::ChunkerError),
}
