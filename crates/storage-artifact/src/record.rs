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

/// The fixed byte width of a declaration record's admission-index key.
pub const ADMISSION_KEY_LEN: usize = 8;

/// One declaration record: its admission index as a fixed-width big-endian key
/// and its declaration-segment bytes as the value.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ArtifactRecord
{
    /// The admission index, big-endian (so byte order matches numeric order).
    key: [u8; ADMISSION_KEY_LEN],
    /// The declaration segment bytes.
    value: Box<[u8]>,
}

impl ArtifactRecord
{
    /// Builds a record from an admission index and its segment bytes.
    #[inline]
    #[must_use]
    pub fn new(
        admission_index: u64,
        value: &[u8],
    ) -> Self
    {
        return Self {
            key: admission_index.to_be_bytes(),
            value: Box::<[u8]>::from(value),
        };
    }

    /// Returns the fixed-width admission-index key bytes.
    #[inline]
    #[must_use]
    pub const fn key(&self) -> &[u8; ADMISSION_KEY_LEN]
    {
        return &self.key;
    }

    /// Returns the admission index this record is keyed by.
    #[inline]
    #[must_use]
    pub const fn admission_index(&self) -> u64
    {
        return u64::from_be_bytes(self.key);
    }

    /// Returns the declaration segment bytes.
    #[inline]
    #[must_use]
    pub fn value(&self) -> &[u8]
    {
        return self.value.as_ref();
    }

    /// Returns a borrowed prolly-tree record view of this declaration record.
    #[inline]
    #[must_use]
    pub fn as_record_ref(&self) -> RecordRef<'_>
    {
        return RecordRef::new(self.key.as_slice(), self.value.as_ref());
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
#[non_exhaustive]
pub struct ArtifactRecordSet
{
    /// The inner kernel export format version the records were cut from.
    inner_format_version: u16,
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
        let mut records = Vec::with_capacity(segmented.segment_count());
        for (index, segment) in segmented.segments().enumerate() {
            let admission_index = u64::try_from(index).unwrap_or(u64::MAX);
            records.push(ArtifactRecord::new(admission_index, segment));
        }

        return Self {
            inner_format_version: FORMAT_VERSION_V1,
            header: Box::<[u8]>::from(segmented.header()),
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
        inner_format_version: u16,
        header: &[u8],
        records: Vec<ArtifactRecord>,
    ) -> Result<Self, ArtifactError>
    {
        let mut first_seen: BTreeMap<[u8; ADMISSION_KEY_LEN], u64> = BTreeMap::new();
        for (position, record) in records.iter().enumerate() {
            let current_index = u64::try_from(position).unwrap_or(u64::MAX);
            if let Some(&first_index) = first_seen.get(record.key()) {
                return Err(ArtifactError::DuplicateAdmissionKey {
                    key: record.admission_index(),
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
            header: Box::<[u8]>::from(header),
            records: sorted,
        });
    }

    /// Returns the inner kernel export format version the records were cut
    /// from.
    #[inline]
    #[must_use]
    pub const fn inner_format_version(&self) -> u16
    {
        return self.inner_format_version;
    }

    /// Returns the artifact header carried for reassembly.
    #[inline]
    #[must_use]
    pub fn header(&self) -> &[u8]
    {
        return self.header.as_ref();
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
    pub fn record_count(&self) -> u64
    {
        return u64::try_from(self.records.len()).unwrap_or(u64::MAX);
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
    pub fn reassemble(&self) -> Vec<u8>
    {
        let mut out = Vec::<u8>::from(self.header.as_ref());
        for record in &self.records {
            out.extend_from_slice(record.value());
        }
        return out;
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
            ArtifactRecord::new(0, b"zero"),
            ArtifactRecord::new(1, b"one"),
            ArtifactRecord::new(2, b"two"),
        ];
        let reversed = vec![
            ArtifactRecord::new(2, b"two"),
            ArtifactRecord::new(1, b"one"),
            ArtifactRecord::new(0, b"zero"),
        ];
        let from_forward = ArtifactRecordSet::from_records(1, b"header", forward).expect("unique");
        let from_reversed =
            ArtifactRecordSet::from_records(1, b"header", reversed).expect("unique");
        assert_eq!(
            from_forward, from_reversed,
            "a permuted input yields the identical canonical record set"
        );
        let indices: Vec<u64> = from_forward
            .records()
            .iter()
            .map(ArtifactRecord::admission_index)
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
        let records = vec![ArtifactRecord::new(3, b"a"), ArtifactRecord::new(3, b"b")];
        match ArtifactRecordSet::from_records(1, b"header", records) {
            | Err(ArtifactError::DuplicateAdmissionKey {
                key,
                first_index,
                second_index,
            }) => {
                assert_eq!(key, 3);
                assert_eq!(first_index, 0);
                assert_eq!(second_index, 1);
            },
            | other => panic!("expected a duplicate-key rejection, got {other:?}"),
        }
    }
}
