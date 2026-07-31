//! The declaration-record model (massive-term design §6).
//!
//! A v1 export artifact **is**, by construction, a sorted unique keyed record
//! set: E2 admission ordering keys each declaration by its admission index, and
//! the format is declaration-segmented, so records are
//! `(admission index as a fixed-width big-endian key → declaration segment
//! bytes)`. This module produces that record set from a
//! [`SegmentedArtifact`] (or directly from an [`Environment`]) and reassembles
//! the canonical artifact from it.
//!
//! Record-safe chunking (`storage-chunker`) is declaration-granular chunking —
//! exactly the E2 replay grain. A declaration segment may reference subterm
//! entries an earlier segment introduced (cross-declaration sharing), so a
//! record is a **content-addressing grain, not an independently replayable
//! unit**: replay is whole-artifact ([`gandr_kernel_core::read`]) over the
//! reassembled bytes. The record set is canonical by construction — sorted and
//! unique by
//! key — so the outer identity it feeds is history-independent (a permuted
//! build order yields the identical root).

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use gandr_kernel_core::Environment;
use gandr_kernel_core::FORMAT_VERSION_V1;
use gandr_kernel_core::SegmentedArtifact;
use gandr_kernel_core::write_segmented;
use gandr_storage_prolly_trees::RecordRef;

use crate::error::ArtifactError;
use crate::manifest::ArtifactRecordCount;
use crate::manifest::InnerFormatVersion;

/// The fixed byte width of a declaration record's admission-index key.
pub const ADMISSION_KEY_LEN: usize = 8;

/// Admission-order index carried by one declaration record.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AdmissionIndex(u64);

impl From<u64> for AdmissionIndex
{
    #[inline]
    fn from(index: u64) -> Self
    {
        return Self(index);
    }
}

impl From<AdmissionIndex> for u64
{
    #[inline]
    fn from(index: AdmissionIndex) -> Self
    {
        return index.0;
    }
}

/// Fixed-width big-endian key for one admission index.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AdmissionKey([u8; ADMISSION_KEY_LEN]);

impl From<AdmissionIndex> for AdmissionKey
{
    #[inline]
    fn from(index: AdmissionIndex) -> Self
    {
        return Self(index.0.to_be_bytes());
    }
}

impl From<AdmissionKey> for AdmissionIndex
{
    #[inline]
    fn from(key: AdmissionKey) -> Self
    {
        return Self(u64::from_be_bytes(key.0));
    }
}

impl AsRef<[u8; ADMISSION_KEY_LEN]> for AdmissionKey
{
    #[inline]
    fn as_ref(&self) -> &[u8; ADMISSION_KEY_LEN]
    {
        return &self.0;
    }
}

impl AsRef<[u8]> for AdmissionKey
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return &self.0;
    }
}

/// Borrowed declaration-segment bytes for one artifact record.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RecordBytes<'record>(&'record [u8]);

impl<'record> From<&'record [u8]> for RecordBytes<'record>
{
    #[inline]
    fn from(bytes: &'record [u8]) -> Self
    {
        return Self(bytes);
    }
}

impl AsRef<[u8]> for RecordBytes<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0;
    }
}

/// Borrowed canonical header preceding an artifact's declaration records.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArtifactHeader<'header>(&'header [u8]);

impl<'header> From<&'header [u8]> for ArtifactHeader<'header>
{
    #[inline]
    fn from(header: &'header [u8]) -> Self
    {
        return Self(header);
    }
}

impl AsRef<[u8]> for ArtifactHeader<'_>
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0;
    }
}

/// Owned canonical bytes reconstructed from an artifact record set.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReassembledArtifact(Vec<u8>);

impl AsRef<[u8]> for ReassembledArtifact
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        return self.0.as_slice();
    }
}

impl core::ops::Deref for ReassembledArtifact
{
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target
    {
        return self.0.as_slice();
    }
}

/// One declaration record: its admission index as a fixed-width big-endian key
/// and its declaration-segment bytes as the value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactRecord
{
    /// The admission index, big-endian (so byte order matches numeric order).
    key: AdmissionKey,
    /// The declaration segment bytes.
    value: Box<[u8]>,
}

impl ArtifactRecord
{
    /// Builds a record from an admission index and its segment bytes.
    #[inline]
    #[must_use]
    pub fn new(
        admission_index: AdmissionIndex,
        value: RecordBytes<'_>,
    ) -> Self
    {
        return Self {
            key: AdmissionKey::from(admission_index),
            value: Box::<[u8]>::from(value.as_ref()),
        };
    }

    /// Returns the fixed-width admission-index key.
    #[inline]
    #[must_use]
    pub const fn key(&self) -> &AdmissionKey
    {
        return &self.key;
    }

    /// Returns the admission index this record is keyed by.
    #[inline]
    #[must_use]
    pub const fn admission_index(&self) -> AdmissionIndex
    {
        return AdmissionIndex(u64::from_be_bytes(self.key.0));
    }

    /// Returns the declaration segment bytes.
    #[inline]
    #[must_use]
    pub fn value(&self) -> RecordBytes<'_>
    {
        return RecordBytes(self.value.as_ref());
    }

    /// Returns a borrowed prolly-tree record view of this declaration record.
    #[inline]
    #[must_use]
    pub fn as_record_ref(&self) -> RecordRef<'_>
    {
        let key: &[u8] = self.key.as_ref();
        return RecordRef::new(key, self.value.as_ref());
    }
}

/// The sorted unique keyed record set an export artifact is by construction,
/// with the header needed to reassemble the canonical artifact from it.
///
/// # Contract
/// - requires: constructed through [`Self::from_environment`] /
///   [`Self::from_segmented`] (admission order, already canonical) or
///   [`Self::from_records`] (which sorts and rejects duplicate keys).
/// - ensures: `records` are strictly ascending by key (admission order) and
///   unique, so the record set — and every identity derived from it — is
///   history-independent.
/// - provides: the outer CAS layer's record grain and canonical reassembly.
/// - fails: [`ArtifactError::DuplicateAdmissionKey`] from
///   [`Self::from_records`].
/// - panics: none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactRecordSet
{
    /// The inner kernel export format version the records were cut from.
    inner_format_version: InnerFormatVersion,
    /// The artifact header preceding the first declaration segment.
    header: Box<[u8]>,
    /// The records, strictly ascending and unique by key.
    records: Vec<ArtifactRecord>,
}

impl ArtifactRecordSet
{
    /// Extracts the record set from an environment by serializing it once.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: the record set of `write_segmented(environment)` — one record
    ///   per admitted declaration, keyed by admission index, in admission
    ///   order.
    /// - provides: the producer-side extraction path (the environment is in
    ///   hand at B2.3; the deterministic segment partition is the record
    ///   grain).
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn from_environment(environment: &Environment) -> Self
    {
        let segmented = write_segmented(environment);
        return Self::from_segmented(&segmented);
    }

    /// Extracts the record set from a segmented artifact.
    ///
    /// # Contract
    /// - requires: `segmented` was produced by [`write_segmented`].
    /// - ensures: one record per declaration segment, keyed by the segment's
    ///   admission index, in admission order (already sorted and unique); the
    ///   header is carried verbatim for reassembly.
    /// - provides: the record set whose reassembly is byte-identical to the
    ///   artifact.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn from_segmented(segmented: &SegmentedArtifact) -> Self
    {
        let mut records = Vec::with_capacity(usize::from(segmented.segment_count()));
        for (index, segment) in segmented.segments().enumerate() {
            let admission_index = AdmissionIndex::from(u64::try_from(index).unwrap_or(u64::MAX));
            records.push(ArtifactRecord::new(
                admission_index,
                RecordBytes::from(segment),
            ));
        }

        return Self {
            inner_format_version: InnerFormatVersion::from(FORMAT_VERSION_V1),
            header: Box::<[u8]>::from(segmented.header().as_ref()),
            records,
        };
    }

    /// Builds a canonical record set from records in **any** order.
    ///
    /// # Contract
    /// - requires: `records` carry unique admission-index keys.
    /// - ensures: the records are sorted strictly ascending by key, so any two
    ///   input permutations of the same record set produce the identical
    ///   canonical set — the by-construction history-independence made a
    ///   constructor invariant.
    /// - provides: the general constructor (and the history-independence
    ///   differential's entry point).
    /// - fails: [`ArtifactError::DuplicateAdmissionKey`] when two records share
    ///   a key.
    /// - panics: none.
    ///
    /// # Errors
    /// [`ArtifactError::DuplicateAdmissionKey`].
    #[inline]
    pub fn from_records(
        inner_format_version: InnerFormatVersion,
        header: ArtifactHeader<'_>,
        records: Vec<ArtifactRecord>,
    ) -> Result<Self, ArtifactError>
    {
        let mut first_seen: BTreeMap<AdmissionKey, u64> = BTreeMap::new();
        for (position, record) in records.iter().enumerate() {
            let current_index = u64::try_from(position).unwrap_or(u64::MAX);
            if let Some(&first_index) = first_seen.get(record.key()) {
                return Err(ArtifactError::DuplicateAdmissionKey {
                    key: u64::from(record.admission_index()),
                    first_index,
                    second_index: current_index,
                });
            }
            let _prior = first_seen.insert(*record.key(), current_index);
        }

        let mut sorted = records;
        sorted.sort_by(|left, right| left.key().cmp(right.key()));

        return Ok(Self {
            inner_format_version,
            header: Box::<[u8]>::from(header.as_ref()),
            records: sorted,
        });
    }

    /// Returns the inner kernel export format version the records were cut
    /// from.
    #[inline]
    #[must_use]
    pub const fn inner_format_version(&self) -> InnerFormatVersion
    {
        return self.inner_format_version;
    }

    /// Returns the artifact header carried for reassembly.
    #[inline]
    #[must_use]
    pub fn header(&self) -> ArtifactHeader<'_>
    {
        return ArtifactHeader(self.header.as_ref());
    }

    /// Returns the records, strictly ascending and unique by key.
    #[inline]
    #[must_use]
    pub fn records(&self) -> &[ArtifactRecord]
    {
        return self.records.as_slice();
    }

    /// Returns the number of declaration records.
    #[inline]
    #[must_use]
    pub fn record_count(&self) -> ArtifactRecordCount
    {
        return ArtifactRecordCount::from(u64::try_from(self.records.len()).unwrap_or(u64::MAX));
    }

    /// Returns borrowed prolly-tree record views, in key order, for tree build.
    #[inline]
    #[must_use]
    pub fn record_refs(&self) -> Vec<RecordRef<'_>>
    {
        return self
            .records
            .iter()
            .map(ArtifactRecord::as_record_ref)
            .collect();
    }

    /// Reassembles the canonical artifact bytes from the header and records.
    ///
    /// # Contract
    /// - requires: nothing.
    /// - ensures: the header followed by every record's value in key order; for
    ///   a set built from an artifact this is byte-identical to that artifact
    ///   (the record-extraction round-trip).
    /// - provides: the outer round-trip's reassembly step.
    /// - fails: never.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn reassemble(&self) -> ReassembledArtifact
    {
        let mut out = Vec::<u8>::from(self.header.as_ref());
        for record in &self.records {
            out.extend_from_slice(record.value().as_ref());
        }
        return ReassembledArtifact(out);
    }
}

/// The record-set canonicalization witnesses.
#[cfg(test)]
mod tests
{
    use alloc::vec;

    use super::ArtifactRecord;
    use super::ArtifactRecordSet;
    use crate::error::ArtifactError;

    /// `from_records` sorts any input permutation into the identical canonical
    /// (ascending-by-key) order — the history-independence invariant.
    #[test]
    fn from_records_sorts_any_permutation_canonically()
    {
        let forward = vec![
            ArtifactRecord::new(0_u64.into(), b"zero".as_slice().into()),
            ArtifactRecord::new(1_u64.into(), b"one".as_slice().into()),
            ArtifactRecord::new(2_u64.into(), b"two".as_slice().into()),
        ];
        let reversed = vec![
            ArtifactRecord::new(2_u64.into(), b"two".as_slice().into()),
            ArtifactRecord::new(1_u64.into(), b"one".as_slice().into()),
            ArtifactRecord::new(0_u64.into(), b"zero".as_slice().into()),
        ];
        let from_forward =
            ArtifactRecordSet::from_records(1_u16.into(), b"header".as_slice().into(), forward)
                .expect("unique");
        let from_reversed =
            ArtifactRecordSet::from_records(1_u16.into(), b"header".as_slice().into(), reversed)
                .expect("unique");
        assert_eq!(
            from_forward, from_reversed,
            "a permuted input yields the identical canonical record set"
        );
        let indices: Vec<u64> = from_forward
            .records()
            .iter()
            .map(ArtifactRecord::admission_index)
            .map(u64::from)
            .collect();
        assert_eq!(
            indices,
            vec![0, 1, 2],
            "records are ascending by admission key"
        );
    }

    /// A duplicate admission key is a fail-closed rejection.
    #[test]
    fn a_duplicate_admission_key_is_rejected()
    {
        let records = vec![
            ArtifactRecord::new(3_u64.into(), b"a".as_slice().into()),
            ArtifactRecord::new(3_u64.into(), b"b".as_slice().into()),
        ];
        match ArtifactRecordSet::from_records(1_u16.into(), b"header".as_slice().into(), records) {
            | Err(ArtifactError::DuplicateAdmissionKey {
                key,
                first_index,
                second_index,
            }) => {
                assert_eq!(3, key);
                assert_eq!(0, first_index);
                assert_eq!(1, second_index);
            },
            | other => panic!("expected a duplicate-key rejection, got {other:?}"),
        }
    }
}
