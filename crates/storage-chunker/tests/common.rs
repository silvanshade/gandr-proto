//! Shared fixtures and assertion helpers for the chunker contract suite.

use core::fmt;

use gandr_storage_chunker::AlgorithmVersion;
use gandr_storage_chunker::BoundaryReason;
use gandr_storage_chunker::ByteCount;
use gandr_storage_chunker::BytePosition;
use gandr_storage_chunker::ByteSpan;
use gandr_storage_chunker::CanonicalRecords;
use gandr_storage_chunker::ChunkLimits;
use gandr_storage_chunker::ChunkSpan;
use gandr_storage_chunker::ChunkerParams;
use gandr_storage_chunker::GearTableVersion;
use gandr_storage_chunker::NormalizationPolicy;
use gandr_storage_chunker::RecordBoundaryRule;
use gandr_storage_chunker::RecordCount;
use gandr_storage_chunker::RecordPosition;
use gandr_storage_chunker::SeedPolicy;

/// Owned canonical bytes produced by fixtures before they are borrowed by
/// the chunker interface.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct FixtureCanonicalBytes(Vec<u8>);

impl AsRef<[u8]> for FixtureCanonicalBytes
{
    fn as_ref(&self) -> &[u8]
    {
        return self.0.as_slice();
    }
}

/// Record distance observed in a chunk span.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FixtureRecordDistance(u64);

impl From<u64> for FixtureRecordDistance
{
    fn from(distance: u64) -> Self
    {
        return Self(distance);
    }
}

/// Private decision used by fixture predicates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
struct FixtureDecision(bool);

impl From<bool> for FixtureDecision
{
    fn from(decision: bool) -> Self
    {
        return Self(decision);
    }
}

impl From<FixtureDecision> for bool
{
    #[inline]
    fn from(decision: FixtureDecision) -> Self
    {
        return decision.0;
    }
}

/// Human-readable assertion context.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct FixtureContext<'context>(&'context str);

impl<'context> From<&'context str> for FixtureContext<'context>
{
    fn from(context: &'context str) -> Self
    {
        return Self(context);
    }
}

impl fmt::Display for FixtureContext<'_>
{
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        return self.0.fmt(f);
    }
}

/// Builds validated chunk limits for contract fixtures.
pub fn limits(
    min_bytes: ByteCount,
    target_bytes: ByteCount,
    max_bytes: ByteCount,
    min_records: RecordCount,
    target_records: RecordCount,
    max_records: RecordCount,
) -> ChunkLimits
{
    return ChunkLimits::new(
        min_bytes,
        target_bytes,
        max_bytes,
        min_records,
        target_records,
        max_records,
    )
    .expect("fixture chunk limits must be valid");
}

/// Builds validated default-version params for a seed and limits.
pub fn params_with_seed(
    seed_policy: SeedPolicy,
    chunk_limits: ChunkLimits,
) -> ChunkerParams
{
    return ChunkerParams::new(
        AlgorithmVersion::FASTCDC_2020,
        GearTableVersion::MACH_V1,
        seed_policy,
        NormalizationPolicy::NONE,
        RecordBoundaryRule::BETWEEN_RECORDS,
        chunk_limits,
    )
    .expect("fixture chunker params must be valid");
}

/// Builds validated default-version params with no salt.
pub fn params(chunk_limits: ChunkLimits) -> ChunkerParams
{
    return params_with_seed(SeedPolicy::NONE, chunk_limits);
}

/// Concatenates ordered canonical records for `chunk_spans` fixtures.
pub fn canonical_bytes(records: CanonicalRecords<'_>) -> FixtureCanonicalBytes
{
    let records = records.as_ref();
    let capacity = records
        .iter()
        .try_fold(0_usize, |accumulator, record| {
            return accumulator.checked_add(record.len());
        })
        .expect("fixture canonical byte length must fit usize");
    let mut bytes = Vec::with_capacity(capacity);

    for &record in records {
        bytes.extend_from_slice(record);
    }

    return FixtureCanonicalBytes(bytes);
}

/// Builds monotonic byte spans for ordered canonical records.
pub fn record_spans(records: CanonicalRecords<'_>) -> Vec<ByteSpan>
{
    let records = records.as_ref();
    let mut spans = Vec::with_capacity(records.len());
    let mut start = BytePosition::from(0_u64);

    for &record in records {
        let record_len =
            u64::try_from(record.len()).expect("fixture record length must fit u64");
        let end = u64::from(start)
            .checked_add(record_len)
            .map(BytePosition::from)
            .expect("fixture byte offsets must not overflow");
        spans.push(ByteSpan::new(start, end));
        start = end;
    }

    return spans;
}

/// Converts a record count to the chunker span type.
pub fn record_count(records: CanonicalRecords<'_>) -> RecordPosition
{
    let count =
        u64::try_from(records.as_ref().len()).expect("fixture record count must fit u64");
    return RecordPosition::from(count);
}

/// Converts a byte length to the chunker span type.
pub fn byte_count(bytes: &FixtureCanonicalBytes) -> BytePosition
{
    let count = u64::try_from(bytes.as_ref().len()).expect("fixture byte count must fit u64");
    return BytePosition::from(count);
}

/// Returns the byte width of a chunk span.
pub fn chunk_byte_len(chunk: &ChunkSpan) -> ByteCount
{
    let length = u64::from(chunk.bytes.end)
        .checked_sub(u64::from(chunk.bytes.start))
        .expect("chunk byte span must be monotonic");
    return ByteCount::from(length);
}

/// Returns the record width of a chunk span.
pub fn chunk_record_len(chunk: &ChunkSpan) -> FixtureRecordDistance
{
    let length = u64::from(chunk.records.end)
        .checked_sub(u64::from(chunk.records.start))
        .expect("chunk record span must be monotonic");
    return FixtureRecordDistance::from(length);
}

/// Asserts that a chunk was emitted for the expected boundary reason.
pub fn assert_chunk_reason(
    chunk: &ChunkSpan,
    expected: BoundaryReason,
    context: FixtureContext<'_>,
)
{
    assert_eq!(
        chunk.reason, expected,
        "{context}: boundary reason must match"
    );
}

/// Asserts that at least one chunk carries the expected reason.
pub fn assert_has_reason(
    chunks: &[ChunkSpan],
    expected: BoundaryReason,
    context: FixtureContext<'_>,
)
{
    assert!(
        chunks.iter().any(|chunk| {
            return chunk.reason == expected;
        }),
        "{context}: expected boundary reason was not emitted"
    );
}

/// Asserts two chunk sequences have identical spans and reason variants.
pub fn assert_equivalent_chunks(
    left: &[ChunkSpan],
    right: &[ChunkSpan],
    context: FixtureContext<'_>,
)
{
    assert_eq!(
        left.len(),
        right.len(),
        "{context}: chunk counts must match"
    );

    for (left_chunk, right_chunk) in left.iter().zip(right.iter()) {
        assert_eq!(
            left_chunk.bytes.start, right_chunk.bytes.start,
            "{context}: chunk byte starts must match"
        );
        assert_eq!(
            left_chunk.bytes.end, right_chunk.bytes.end,
            "{context}: chunk byte ends must match"
        );
        assert_eq!(
            left_chunk.records.start, right_chunk.records.start,
            "{context}: chunk record starts must match"
        );
        assert_eq!(
            left_chunk.records.end, right_chunk.records.end,
            "{context}: chunk record ends must match"
        );
        assert_eq!(
            left_chunk.reason, right_chunk.reason,
            "{context}: chunk reason variants must match"
        );
    }
}

/// Asserts that chunk spans are contiguous and cover the full input.
pub fn assert_monotonic_chunks(
    chunks: &[ChunkSpan],
    total_bytes: BytePosition,
    total_records: RecordPosition,
    context: FixtureContext<'_>,
)
{
    let mut next_byte = BytePosition::from(0_u64);
    let mut next_record = RecordPosition::from(0_u64);

    for chunk in chunks {
        assert_eq!(
            chunk.bytes.start, next_byte,
            "{context}: byte spans must be contiguous"
        );
        assert!(
            chunk.bytes.end >= chunk.bytes.start,
            "{context}: byte span end must not precede start"
        );
        assert_eq!(
            chunk.records.start, next_record,
            "{context}: record spans must be contiguous"
        );
        assert!(
            chunk.records.end >= chunk.records.start,
            "{context}: record span end must not precede start"
        );

        next_byte = chunk.bytes.end;
        next_record = chunk.records.end;
    }

    assert_eq!(
        next_byte, total_bytes,
        "{context}: byte spans must cover the input exactly"
    );
    assert_eq!(
        next_record, total_records,
        "{context}: record spans must cover the input exactly"
    );
}

/// Returns whether a byte position is one of the declared record edges.
fn is_record_edge(
    position: BytePosition,
    spans: &[ByteSpan],
    total_bytes: BytePosition,
) -> FixtureDecision
{
    if position == BytePosition::from(0_u64) || position == total_bytes {
        return FixtureDecision::from(true);
    }

    return FixtureDecision::from(spans.iter().any(|span| {
        return span.start == position || span.end == position;
    }));
}

/// Asserts every emitted byte boundary falls on a canonical record edge.
pub fn assert_record_aligned_chunks(
    chunks: &[ChunkSpan],
    spans: &[ByteSpan],
    total_bytes: BytePosition,
    context: FixtureContext<'_>,
)
{
    for chunk in chunks {
        assert!(
            bool::from(is_record_edge(chunk.bytes.start, spans, total_bytes)),
            "{context}: chunk byte start must be a record edge"
        );
        assert!(
            bool::from(is_record_edge(chunk.bytes.end, spans, total_bytes)),
            "{context}: chunk byte end must be a record edge"
        );
    }
}

/// Asserts every chunk stays within configured byte and record caps.
pub fn assert_chunks_within_caps(
    chunks: &[ChunkSpan],
    max_bytes: ByteCount,
    max_records: FixtureRecordDistance,
    context: FixtureContext<'_>,
)
{
    for chunk in chunks {
        assert!(
            chunk_byte_len(chunk) <= max_bytes,
            "{context}: chunk byte span must respect max byte cap"
        );
        assert!(
            chunk_record_len(chunk) <= max_records,
            "{context}: chunk record span must respect max record cap"
        );
    }
}
