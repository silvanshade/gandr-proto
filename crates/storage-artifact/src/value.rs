//! The **value plane** — a recursive typed chunk DAG over one massive value,
//! addressed by [`ContentPtr`] and moved by [`cam_commit`] and [`cam_deref`].
//!
//! The keyed plane ([`crate::record`], [`crate::manifest`]) addresses a
//! *sorted keyed record set* and cuts chunks between record boundaries. This
//! module addresses a *single value* and cuts chunks between its
//! **constructors**. One discipline, two grains: both are rolling-hash cuts at
//! structured boundary positions, and gandr has both workloads — export
//! artifacts on the keyed plane, and levitated descriptions, machine states,
//! certificate tables and checkpointed environments on this one.
//!
//! # What a value plane buys that the keyed plane cannot
//!
//! A value's own type is its index, so this plane needs no B-tree: its
//! multi-level structure comes from the chunk wrappers themselves. That is
//! what makes a depth-$d$ path edit cost chunks proportional to $d/kappa$
//! rather than to the size of the value, and it is the property
//! [`locality`] measures rather than assumes.
//!
//! # Two walls, unchanged
//!
//! Everything here is **untrusted plumbing**, exactly as [`crate::manifest`]
//! is. A [`ContentPtr`] addresses and authenticates bytes; it never implies
//! that what those bytes decode to is valid. Replay from the canonical inner
//! bytes remains the sole validity authority, and no code in this module
//! re-checks or short-circuits it.
//!
//! # The store decision, and why this plane does not ride `BlockStore`
//!
//! `gandr_storage_prolly_trees::BlockStore` verifies on both insert and load
//! that the bytes it carries **decode as canonical prolly-node material** — a
//! keyed leaf or internal node. A value chunk is not node material and never
//! will be, so a value chunk cannot cross that trait: wrapping each chunk as a
//! one-record leaf would make the chunk digest depend on prolly leaf framing
//! rather than on the value's own canonical bytes, which is precisely the
//! identity this plane exists to provide.
//!
//! So the two planes share the **discipline** rather than the trait:
//! [`chunk::ChunkStore`] carries the same verify-on-insert-and-load rule over
//! a different body, and one backing object may implement both traits at once
//! — which is how a later rung deduplicates a checkpoint's environment against
//! an artifact's records in a single store. Confusion between the two bodies
//! is impossible by construction rather than by convention, because a chunk
//! image is framed with [`chunk::VALUE_CHUNK_MAGIC`] inside its own hashed
//! preimage, the same domain-separation rule [`crate::transport`] already
//! applies to step identities.
//!
//! # Reading order
//!
//! | module            | what it fixes                                                                     |
//! | ----------------- | --------------------------------------------------------------------------------- |
//! | [`ptr`]           | [`ContentPtr`]: the digest-plus-token-offset address, and its own digest newtype  |
//! | [`tokens`]        | the canonical token stream, its sink, and the codec a value implements            |
//! | [`chunk`]         | the chunk image framing, its digest domain, and the chunk store                   |
//! | [`index_base`]    | the child-reference representation, and the evaluation that chooses it            |
//! | [`commit`]        | [`cam_commit`]: the bottom-up committing traversal                                |
//! | [`mod@deref`]     | [`cam_deref`]: fetch, verify, decode                                              |
//! | [`value_manifest`] | the value-plane manifest binding chunker, codec and root                          |
//! | [`locality`]      | the measured chunk counts, and the bound they are read against                    |
//! | [`units`]         | the semantic wrappers every signature above is stated in                          |
//!
//! # Wrong-kind inhabitants, and the witnesses that separate them
//!
//! Every position in this plane admits a **plausible wrong inhabitant** — a
//! value of the right Rust type standing where a different thing belongs. Such
//! an inhabitant reads as success to every instrument that only checks for an
//! error, so each one is named here with the witness that kills it. A test
//! that merely passes when the representation is right is not on this list.
//!
//! | position                          | the wrong inhabitant                                                     | the separating witness                                                                                      |
//! | --------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
//! | [`ContentPtr::digest`]            | a prolly `NodeHash` reinterpreted as a chunk digest                      | no conversion exists in either direction, and a node image fails [`chunk::verify_chunk_image`] on the magic  |
//! | [`ContentPtr::offset`]            | a token index into the whole value rather than into the addressed chunk  | deref a pointer into a non-root chunk and require the decoded subtree, not the whole value                   |
//! | a chunk image                     | a prolly node image stored under a chunk digest                          | insert a framed node image and require [`chunk::verify_chunk_image`] to refuse it by magic                    |
//! | a constructor tag                 | a canonical word whose low byte happens to be a valid tag               | the reader refuses with [`crate::error::ValueError::UnexpectedToken`] naming both kinds, never coerces        |
//! | a child pointer token             | an inline word carrying a digest-shaped payload                          | commit a value with a shared subtree and require the store to hold one chunk for it, not two                  |
//! | [`index_base::ChildIndexBase`]    | absolute indices committed while the manifest says chunk-local          | edit early in a value and require the downstream chunk digests to be unchanged under `ChunkLocal` and changed under `Absolute` |
//!
//! The last row is the sharpest of them, and it is the reason the index-base
//! question is settled by measurement here rather than deferred: under a wrong
//! commitment both representations round-trip perfectly, and the only
//! observable difference is which chunks moved.

pub mod chunk;
pub mod commit;
pub mod deref;
pub mod index_base;
pub mod locality;
pub mod ptr;
pub mod tokens;
pub mod units;
pub mod value_manifest;

pub use crate::value::chunk::ChunkStore;
pub use crate::value::chunk::InMemoryChunkStore;
pub use crate::value::chunk::StoredChunkRef;
pub use crate::value::chunk::VALUE_CHUNK_MAGIC;
pub use crate::value::commit::cam_commit;
pub use crate::value::deref::cam_deref;
pub use crate::value::ptr::CHUNK_DIGEST_LEN;
pub use crate::value::ptr::ChunkDigest;
pub use crate::value::ptr::ContentPtr;
pub use crate::value::ptr::TokenOffset;
pub use crate::value::tokens::CanonicalValue;
pub use crate::value::tokens::ConstructorTag;
pub use crate::value::tokens::TOKEN_BYTES;
pub use crate::value::tokens::TOKEN_CHILD;
pub use crate::value::tokens::TOKEN_CLOSE;
pub use crate::value::tokens::TOKEN_OPEN;
pub use crate::value::tokens::TOKEN_WORD;
pub use crate::value::tokens::TokenReader;
pub use crate::value::tokens::TokenSink;
pub use crate::value::units::ChunkBody;
pub use crate::value::units::ChunkCount;
pub use crate::value::units::ChunkFormatVersion;
pub use crate::value::units::ChunkImage;
pub use crate::value::units::ChunkImageBuf;
pub use crate::value::units::SeamDepth;
pub use crate::value::units::TokenBytes;
pub use crate::value::value_manifest::BoundaryClassification;
pub use crate::value::value_manifest::SharingPolicy;
pub use crate::value::value_manifest::ValueManifest;
pub use crate::value::value_manifest::ValueProfile;
