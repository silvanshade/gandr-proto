//! The value-plane manifest: what a content pointer is only meaningful under.
//!
//! # Why the keyed plane's manifest is not revised for this
//!
//! [`crate::manifest::ArtifactManifest`] is the identity of a *sorted keyed
//! record set*. Nothing about that changed at this rung, so bumping its
//! version would invalidate every existing artifact identity to record a fact
//! about a different plane. The value plane gets its own manifest instead,
//! with its own magic and its own version line, and the two evolve
//! independently — which is the whole reason the manifest layout version and
//! the inner format version were separated in the first place.
//!
//! # What it binds, and why each field is load-bearing
//!
//! | field                | what changes if it differs                                          |
//! | -------------------- | -------------------------------------------------------------------- |
//! | chunker commitment   | kappa and the cap fix the cut positions, and so every digest         |
//! | digest family        | the hash fixes the addresses outright                                |
//! | codec identity       | a different token encoding of the same value is a different value    |
//! | child index base     | absolute versus chunk-local changes every child reference byte       |
//! | root pointer         | the value being named                                                |
//! | token count          | a cheap total the reader checks the decode against                   |
//!
//! Every one of them repartitions the content-address space. A deployment
//! that disagrees on any field must **refuse** rather than quietly fail to
//! share, which is why they are bound into one identity rather than left as
//! deployment configuration.

use gandr_storage_chunker::ParameterCommitment;
use gandr_storage_chunker::TokenCount;

use crate::value::index_base::ChildIndexBase;
use crate::value::ptr::ContentPtr;
use crate::value::units::ValueManifestVersion;

/// Domain-separation magic for the value-plane manifest.
pub const VALUE_MANIFEST_MAGIC: &[u8] = b"gandr:value-manifest:v1";

/// The value-manifest layout version.
pub const VALUE_MANIFEST_FORMAT_VERSION_V1: u16 = 1;

/// The digest family a value-plane deployment commits to.
///
/// One variant today. The enum exists so that a second family is a refused
/// mismatch rather than an unrecorded assumption.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DigestFamily
{
    /// BLAKE3, 32-byte output — the tier's family everywhere.
    Blake3,
}

/// The canonical token codec a value-plane deployment commits to.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CodecIdentity
{
    /// The codec's stable identifier.
    pub codec: u16,
    /// The codec's layout version.
    pub version: u16,
}

/// The canonical identity of one committed value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueManifest
{
    /// The manifest layout version.
    manifest_version: ValueManifestVersion,
    /// The typed chunker parameter commitment the value was cut under.
    chunker_commitment: ParameterCommitment,
    /// The digest family the addresses are taken in.
    digest_family: DigestFamily,
    /// The token codec the value was encoded through.
    codec: CodecIdentity,
    /// The child-reference representation the chunks carry.
    index_base: ChildIndexBase,
    /// The root of the committed chunk DAG.
    root: ContentPtr,
    /// The total token count of the committed value.
    token_count: TokenCount,
}

impl ValueManifest
{
    /// Binds every constant a content pointer is only meaningful under.
    ///
    /// # Contract
    /// - requires: every argument is the constant actually used by the commit
    ///   that produced `root`.
    /// - ensures: the manifest carries them unchanged at layout version
    ///   [`VALUE_MANIFEST_FORMAT_VERSION_V1`].
    /// - provides: the only sanctioned description of a committed value.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        chunker_commitment: ParameterCommitment,
        digest_family: DigestFamily,
        codec: CodecIdentity,
        index_base: ChildIndexBase,
        root: ContentPtr,
        token_count: TokenCount,
    ) -> Self
    {
        return Self {
            manifest_version: ValueManifestVersion::from(VALUE_MANIFEST_FORMAT_VERSION_V1),
            chunker_commitment,
            digest_family,
            codec,
            index_base,
            root,
            token_count,
        };
    }

    /// Returns the manifest layout version.
    #[inline]
    #[must_use]
    pub const fn manifest_version(&self) -> ValueManifestVersion
    {
        return self.manifest_version;
    }

    /// Returns the root content pointer.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> ContentPtr
    {
        return self.root;
    }

    /// Returns the child-reference representation.
    #[inline]
    #[must_use]
    pub const fn index_base(&self) -> ChildIndexBase
    {
        return self.index_base;
    }

    /// Returns the committed token count.
    #[inline]
    #[must_use]
    pub const fn token_count(&self) -> TokenCount
    {
        return self.token_count;
    }

    /// Returns the typed chunker parameter commitment.
    #[inline]
    #[must_use]
    pub const fn chunker_commitment(&self) -> &ParameterCommitment
    {
        return &self.chunker_commitment;
    }

    /// Returns the digest family.
    #[inline]
    #[must_use]
    pub const fn digest_family(&self) -> DigestFamily
    {
        return self.digest_family;
    }

    /// Returns the codec identity.
    #[inline]
    #[must_use]
    pub const fn codec(&self) -> CodecIdentity
    {
        return self.codec;
    }
}
