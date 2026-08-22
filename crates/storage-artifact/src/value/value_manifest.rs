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
//! # The rule this manifest is built from
//!
//! **Wherever a choice changes the addresses but no round trip can see it,
//! the manifest is where a disagreeing consumer is made to refuse.** Two
//! deployments that differ on such a choice both commit correctly, both
//! dereference correctly, and share nothing — and nothing anywhere tells
//! either of them why. There is no test that catches this from inside one
//! deployment, because from inside one deployment everything works. So the
//! defence is to make the disagreement *representable and refused* rather
//! than silent, which means every one of these choices is a bound field.
//!
//! | field                     | the choice, and what differs if two deployments disagree                                       |
//! | ------------------------- | ------------------------------------------------------------------------------------------------ |
//! | chunker commitment        | kappa, the cap and the gear table fix where cuts fall, and so every digest                     |
//! | digest family             | the hash fixes the addresses outright                                                          |
//! | codec identity            | a different token encoding of the same value is a different value                              |
//! | child index base          | absolute versus chunk-local changes every child reference byte                                 |
//! | **boundary classification** | which tags are cut candidates at all; same kappa, different boundary set, different chunks   |
//! | **chunk frame version**   | the framed preimage is what is hashed, so a frame change moves every digest                    |
//! | **sharing policy**        | whether a repeated subtree is spliced as a wrapper or re-emitted inline changes the parent body |
//! | root pointer              | the value being named                                                                          |
//! | token count               | a cheap total the reader checks the decode against                                             |
//!
//! The three in bold are the ones this audit added, and each was invisible
//! for the same reason: they are decisions taken once inside an
//! implementation, where nothing about them looks like a protocol constant.
//! The boundary classification is the sharpest — the export tag table carries
//! **two** verdict columns, a conservative single-constructor rule and a
//! future threshold rule, and choosing between them is exactly a choice that
//! changes every chunk and no observable behaviour.

use gandr_storage_chunker::ParameterCommitment;
use gandr_storage_chunker::TokenCount;

use crate::value::chunk::CHUNK_FORMAT_VERSION_V1;
use crate::value::index_base::ChildIndexBase;
use crate::value::ptr::ContentPtr;
use crate::value::units::ChunkFormatVersion;
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

/// Which of the export tag table's verdict columns decides cut candidacy.
///
/// The table records two classifications per tag. Committing to one of them
/// is a protocol decision: it fixes which constructors are boundary
/// candidates, and so where chunks may be cut at all, independently of kappa
/// and the cap.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BoundaryClassification
{
    /// The conservative rule: a tag is an alias only with one constructor and
    /// a finite static token bound.
    SingleConstructorBound,
    /// The threshold rule: a bounded multi-constructor payload is an alias
    /// when its duplication bound fits the committed threshold.
    Threshold
    {
        /// The committed duplication threshold, in tokens.
        tokens: u32,
    },
}

/// Whether a repeated subtree is referenced or re-emitted.
///
/// Both policies decode to the same value, so no round trip separates them —
/// and they produce different parent bodies, and therefore different digests
/// all the way to the root.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SharingPolicy
{
    /// A subtree already committed is spliced as a chunk wrapper.
    ShareByPointer,
    /// A repeated subtree is re-emitted inline, trading space for locality.
    DuplicateInline,
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

/// Every protocol constant two deployments must agree on to share storage.
///
/// Grouped rather than passed field by field because they are one thing: the
/// **agreement**. A caller that has a profile has everything a peer must
/// match, and a field added here is automatically a field a disagreeing peer
/// refuses on, which is the property the whole design rests on.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueProfile
{
    /// The typed chunker parameter commitment the value was cut under.
    chunker_commitment: ParameterCommitment,
    /// The digest family the addresses are taken in.
    digest_family: DigestFamily,
    /// The token codec the value was encoded through.
    codec: CodecIdentity,
    /// The child-reference representation the chunks carry.
    index_base: ChildIndexBase,
    /// Which tag-table verdict column decides cut candidacy.
    boundary_classification: BoundaryClassification,
    /// The chunk image frame layout the digests were taken over.
    chunk_frame_version: ChunkFormatVersion,
    /// Whether repeated subtrees are shared by pointer or re-emitted.
    sharing_policy: SharingPolicy,
}

impl ValueProfile
{
    /// Fixes every constant a content pointer is only meaningful under.
    ///
    /// # Contract
    /// - requires: every argument is the constant the commit actually used.
    /// - ensures: the profile carries them unchanged, at the chunk frame
    ///   version this build frames chunks under.
    /// - provides: the unit of agreement between two deployments.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        chunker_commitment: ParameterCommitment,
        digest_family: DigestFamily,
        codec: CodecIdentity,
        index_base: ChildIndexBase,
        boundary_classification: BoundaryClassification,
        sharing_policy: SharingPolicy,
    ) -> Self
    {
        return Self {
            chunker_commitment,
            digest_family,
            codec,
            index_base,
            boundary_classification,
            chunk_frame_version: ChunkFormatVersion::from(CHUNK_FORMAT_VERSION_V1),
            sharing_policy,
        };
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

    /// Returns the child-reference representation.
    #[inline]
    #[must_use]
    pub const fn index_base(&self) -> ChildIndexBase
    {
        return self.index_base;
    }

    /// Returns the committed boundary classification.
    #[inline]
    #[must_use]
    pub const fn boundary_classification(&self) -> BoundaryClassification
    {
        return self.boundary_classification;
    }

    /// Returns the chunk frame layout version the digests were taken over.
    #[inline]
    #[must_use]
    pub const fn chunk_frame_version(&self) -> ChunkFormatVersion
    {
        return self.chunk_frame_version;
    }

    /// Returns the committed sharing policy.
    #[inline]
    #[must_use]
    pub const fn sharing_policy(&self) -> SharingPolicy
    {
        return self.sharing_policy;
    }
}

/// The canonical identity of one committed value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueManifest
{
    /// The manifest layout version.
    manifest_version: ValueManifestVersion,
    /// The constants a peer must match to share this value.
    profile: ValueProfile,
    /// The root of the committed chunk DAG.
    root: ContentPtr,
    /// The total token count of the committed value.
    token_count: TokenCount,
}

impl ValueManifest
{
    /// Names one committed value under the profile it was committed with.
    ///
    /// # Contract
    /// - requires: `profile` is the profile the commit that produced `root`
    ///   actually ran under, and `token_count` is that commit's total.
    /// - ensures: the manifest carries them unchanged at layout version
    ///   [`VALUE_MANIFEST_FORMAT_VERSION_V1`].
    /// - provides: the only sanctioned description of a committed value.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        profile: ValueProfile,
        root: ContentPtr,
        token_count: TokenCount,
    ) -> Self
    {
        return Self {
            manifest_version: ValueManifestVersion::from(VALUE_MANIFEST_FORMAT_VERSION_V1),
            profile,
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

    /// Returns the profile a peer must match to share this value.
    #[inline]
    #[must_use]
    pub const fn profile(&self) -> &ValueProfile
    {
        return &self.profile;
    }

    /// Returns the root content pointer.
    #[inline]
    #[must_use]
    pub const fn root(&self) -> ContentPtr
    {
        return self.root;
    }

    /// Returns the committed token count.
    #[inline]
    #[must_use]
    pub const fn token_count(&self) -> TokenCount
    {
        return self.token_count;
    }
}
