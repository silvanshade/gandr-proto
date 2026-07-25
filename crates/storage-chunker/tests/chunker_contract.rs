//! Shared public contract tests for the deterministic record-safe chunker.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "contract tests relax the production wall per docs/workflow/rust.md"
    )
)]

/// Integration tests for the `gandr_storage_chunker` public API.
#[cfg(test)]
mod tests
{
    use core::fmt;

    use gandr_storage_chunker::AlgorithmVersion;
    use gandr_storage_chunker::BoundaryReason;
    use gandr_storage_chunker::ByteCount;
    use gandr_storage_chunker::BytePosition;
    use gandr_storage_chunker::ByteSpan;
    use gandr_storage_chunker::CanonicalBytes;
    use gandr_storage_chunker::CanonicalRecords;
    use gandr_storage_chunker::ChunkLimits;
    use gandr_storage_chunker::ChunkSpan;
    use gandr_storage_chunker::ChunkerError;
    use gandr_storage_chunker::ChunkerParams;
    use gandr_storage_chunker::GearTableVersion;
    use gandr_storage_chunker::InvalidParameterReason;
    use gandr_storage_chunker::NormalizationPolicy;
    use gandr_storage_chunker::PARAMETER_COMMITMENT_LEN;
    use gandr_storage_chunker::RecordBoundaryRule;
    use gandr_storage_chunker::RecordCount;
    use gandr_storage_chunker::RecordPosition;
    use gandr_storage_chunker::SeedKind;
    use gandr_storage_chunker::SeedPolicy;
    use gandr_storage_chunker::SeedSalt;
    use gandr_storage_chunker::chunk_record_slices;
    use gandr_storage_chunker::chunk_spans;

    /// Owned canonical bytes produced by fixtures before they are borrowed by
    /// the chunker interface.
    #[derive(Clone, Debug, Eq, PartialEq)]
    #[repr(transparent)]
    struct FixtureCanonicalBytes(Vec<u8>);

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
    struct FixtureRecordDistance(u64);

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
    struct FixtureContext<'context>(&'context str);

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
    fn limits(
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
    fn params_with_seed(
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
    fn params(chunk_limits: ChunkLimits) -> ChunkerParams
    {
        return params_with_seed(SeedPolicy::NONE, chunk_limits);
    }

    /// Concatenates ordered canonical records for `chunk_spans` fixtures.
    fn canonical_bytes(records: CanonicalRecords<'_>) -> FixtureCanonicalBytes
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
    fn record_spans(records: CanonicalRecords<'_>) -> Vec<ByteSpan>
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
    fn record_count(records: CanonicalRecords<'_>) -> RecordPosition
    {
        let count =
            u64::try_from(records.as_ref().len()).expect("fixture record count must fit u64");
        return RecordPosition::from(count);
    }

    /// Converts a byte length to the chunker span type.
    fn byte_count(bytes: &FixtureCanonicalBytes) -> BytePosition
    {
        let count = u64::try_from(bytes.as_ref().len()).expect("fixture byte count must fit u64");
        return BytePosition::from(count);
    }

    /// Returns the byte width of a chunk span.
    fn chunk_byte_len(chunk: &ChunkSpan) -> ByteCount
    {
        let length = u64::from(chunk.bytes.end)
            .checked_sub(u64::from(chunk.bytes.start))
            .expect("chunk byte span must be monotonic");
        return ByteCount::from(length);
    }

    /// Returns the record width of a chunk span.
    fn chunk_record_len(chunk: &ChunkSpan) -> FixtureRecordDistance
    {
        let length = u64::from(chunk.records.end)
            .checked_sub(u64::from(chunk.records.start))
            .expect("chunk record span must be monotonic");
        return FixtureRecordDistance::from(length);
    }

    /// Asserts that a chunk was emitted for the expected boundary reason.
    fn assert_chunk_reason(
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
    fn assert_has_reason(
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
    fn assert_equivalent_chunks(
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
    fn assert_monotonic_chunks(
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
    fn assert_record_aligned_chunks(
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
    fn assert_chunks_within_caps(
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

    /// Identical records and identical params must emit identical chunk spans.
    #[test]
    fn identical_records_and_params_emit_deterministic_boundaries()
    {
        let records: [&[u8]; 6] = [b"alpha", b"beta", b"gamma", b"delta", b"epsilon", b"zeta"];
        let chunker_params = params(limits(
            ByteCount::from(4_u64),
            ByteCount::from(16_u64),
            ByteCount::from(48_u64),
            RecordCount::from(1_u32),
            RecordCount::from(4_u32),
            RecordCount::from(16_u32),
        ));

        let first =
            chunk_record_slices(CanonicalRecords::from(records.as_slice()), &chunker_params)
                .expect("first deterministic chunking pass must succeed");
        let second =
            chunk_record_slices(CanonicalRecords::from(records.as_slice()), &chunker_params)
                .expect("second deterministic chunking pass must succeed");
        let spans = record_spans(CanonicalRecords::from(records.as_slice()));
        let bytes = canonical_bytes(CanonicalRecords::from(records.as_slice()));

        assert_equivalent_chunks(&first, &second, ("identical input").into());
        assert_monotonic_chunks(
            &first,
            byte_count(&bytes),
            record_count(CanonicalRecords::from(records.as_slice())),
            ("identical input").into(),
        );
        assert_record_aligned_chunks(
            &first,
            &spans,
            byte_count(&bytes),
            ("identical input").into(),
        );
    }

    /// Equivalent record-slice and span APIs must produce equivalent spans.
    #[test]
    fn equivalent_ordered_streams_emit_equivalent_spans()
    {
        let records: [&[u8]; 7] = [
            b"k:a\0v:1",
            b"k:b\0v:2",
            b"k:c\0v:3",
            b"k:d\0v:4",
            b"k:e\0v:5",
            b"k:f\0v:6",
            b"k:g\0v:7",
        ];
        let chunker_params = params(limits(
            ByteCount::from(8_u64),
            ByteCount::from(24_u64),
            ByteCount::from(40_u64),
            RecordCount::from(1_u32),
            RecordCount::from(3_u32),
            RecordCount::from(8_u32),
        ));
        let bytes = canonical_bytes(CanonicalRecords::from(records.as_slice()));
        let spans = record_spans(CanonicalRecords::from(records.as_slice()));

        let from_records =
            chunk_record_slices(CanonicalRecords::from(records.as_slice()), &chunker_params)
                .expect("record-slice chunking must succeed");
        let from_spans = chunk_spans(
            CanonicalBytes::from(bytes.as_ref()),
            &spans,
            &chunker_params,
        )
        .expect("precomputed-span chunking must succeed");

        assert_equivalent_chunks(&from_records, &from_spans, ("equivalent APIs").into());
        assert_monotonic_chunks(
            &from_spans,
            byte_count(&bytes),
            record_count(CanonicalRecords::from(records.as_slice())),
            ("equivalent APIs").into(),
        );
        assert_record_aligned_chunks(
            &from_spans,
            &spans,
            byte_count(&bytes),
            ("equivalent APIs").into(),
        );
    }

    /// Parameter commitments must change when consensus parameters change.
    #[test]
    fn parameter_commitment_changes_when_params_change()
    {
        let base = params(limits(
            ByteCount::from(4_u64),
            ByteCount::from(16_u64),
            ByteCount::from(64_u64),
            RecordCount::from(1_u32),
            RecordCount::from(4_u32),
            RecordCount::from(16_u32),
        ));
        let changed_target = params(limits(
            ByteCount::from(4_u64),
            ByteCount::from(24_u64),
            ByteCount::from(64_u64),
            RecordCount::from(1_u32),
            RecordCount::from(4_u32),
            RecordCount::from(16_u32),
        ));
        let changed_record_target = params(limits(
            ByteCount::from(4_u64),
            ByteCount::from(16_u64),
            ByteCount::from(64_u64),
            RecordCount::from(1_u32),
            RecordCount::from(5_u32),
            RecordCount::from(16_u32),
        ));

        assert_eq!(
            base.commitment_bytes().as_ref().len(),
            PARAMETER_COMMITMENT_LEN,
            "parameter commitment length must be the public fixed length"
        );
        assert_ne!(
            base.commitment_bytes(),
            changed_target.commitment_bytes(),
            "byte target changes must alter the parameter commitment"
        );
        assert_ne!(
            base.commitment_bytes(),
            changed_record_target.commitment_bytes(),
            "record target changes must alter the parameter commitment"
        );
    }

    /// Public salt policy must be deterministic and explicitly committed.
    #[test]
    fn public_salt_is_deterministic_and_explicitly_committed()
    {
        let salt = [
            1_u8, 2_u8, 3_u8, 4_u8, 5_u8, 6_u8, 7_u8, 8_u8, 9_u8, 10_u8, 11_u8, 12_u8, 13_u8,
            14_u8, 15_u8, 16_u8, 17_u8, 18_u8, 19_u8, 20_u8, 21_u8, 22_u8, 23_u8, 24_u8, 25_u8,
            26_u8, 27_u8, 28_u8, 29_u8, 30_u8, 31_u8, 32_u8,
        ];
        let different_salt = [
            32_u8, 31_u8, 30_u8, 29_u8, 28_u8, 27_u8, 26_u8, 25_u8, 24_u8, 23_u8, 22_u8, 21_u8,
            20_u8, 19_u8, 18_u8, 17_u8, 16_u8, 15_u8, 14_u8, 13_u8, 12_u8, 11_u8, 10_u8, 9_u8,
            8_u8, 7_u8, 6_u8, 5_u8, 4_u8, 3_u8, 2_u8, 1_u8,
        ];
        let salted = params_with_seed(
            SeedPolicy::public_salt(SeedSalt::from(salt)),
            limits(
                ByteCount::from(4_u64),
                ByteCount::from(16_u64),
                ByteCount::from(64_u64),
                RecordCount::from(1_u32),
                RecordCount::from(4_u32),
                RecordCount::from(16_u32),
            ),
        );
        let salted_again = params_with_seed(
            SeedPolicy::public_salt(SeedSalt::from(salt)),
            limits(
                ByteCount::from(4_u64),
                ByteCount::from(16_u64),
                ByteCount::from(64_u64),
                RecordCount::from(1_u32),
                RecordCount::from(4_u32),
                RecordCount::from(16_u32),
            ),
        );
        let differently_salted = params_with_seed(
            SeedPolicy::public_salt(SeedSalt::from(different_salt)),
            limits(
                ByteCount::from(4_u64),
                ByteCount::from(16_u64),
                ByteCount::from(64_u64),
                RecordCount::from(1_u32),
                RecordCount::from(4_u32),
                RecordCount::from(16_u32),
            ),
        );

        assert_eq!(
            salted.commitment_bytes(),
            salted_again.commitment_bytes(),
            "same public salt and params must commit deterministically"
        );
        assert_ne!(
            salted.commitment_bytes(),
            differently_salted.commitment_bytes(),
            "different public salts must alter the parameter commitment"
        );
        assert!(
            salted
                .commitment_bytes()
                .as_ref()
                .windows(salt.len())
                .any(|window| {
                    return window == salt.as_ref();
                }),
            "public salt bytes must appear explicitly in the commitment"
        );
    }

    /// Zero, inverted, and overflow-prone chunk limits must be rejected
    /// precisely.
    #[test]
    fn invalid_limits_are_rejected_by_reason()
    {
        assert!(
            matches!(
                ChunkLimits::new(
                    0_u64.into(),
                    1_u64.into(),
                    2_u64.into(),
                    1_u32.into(),
                    1_u32.into(),
                    2_u32.into()
                ),
                Err(ChunkerError::InvalidParameters {
                    reason: InvalidParameterReason::ZeroByteLimit,
                })
            ),
            "zero byte limits must be rejected as invalid parameters"
        );
        assert!(
            matches!(
                ChunkLimits::new(
                    1_u64.into(),
                    1_u64.into(),
                    2_u64.into(),
                    0_u32.into(),
                    1_u32.into(),
                    2_u32.into()
                ),
                Err(ChunkerError::InvalidParameters {
                    reason: InvalidParameterReason::ZeroRecordLimit,
                })
            ),
            "zero record limits must be rejected as invalid parameters"
        );
        assert!(
            matches!(
                ChunkLimits::new(
                    9_u64.into(),
                    8_u64.into(),
                    16_u64.into(),
                    1_u32.into(),
                    2_u32.into(),
                    3_u32.into()
                ),
                Err(ChunkerError::InvalidParameters {
                    reason: InvalidParameterReason::MinByteExceedsTargetByte,
                })
            ),
            "minimum byte limit must not exceed target byte limit"
        );
        assert!(
            matches!(
                ChunkLimits::new(
                    1_u64.into(),
                    17_u64.into(),
                    16_u64.into(),
                    1_u32.into(),
                    2_u32.into(),
                    3_u32.into()
                ),
                Err(ChunkerError::InvalidParameters {
                    reason: InvalidParameterReason::InvertedByteLimits,
                })
            ),
            "target byte limit must not exceed max byte limit"
        );
        assert!(
            matches!(
                ChunkLimits::new(
                    1_u64.into(),
                    8_u64.into(),
                    16_u64.into(),
                    4_u32.into(),
                    3_u32.into(),
                    5_u32.into()
                ),
                Err(ChunkerError::InvalidParameters {
                    reason: InvalidParameterReason::InvertedRecordLimits,
                })
            ),
            "minimum record limit must not exceed target record limit"
        );

        let overflow_prone_target = u64::from(u32::MAX) + 1_u64;
        assert!(
            matches!(
                ChunkLimits::new(
                    1_u64.into(),
                    overflow_prone_target.into(),
                    overflow_prone_target.into(),
                    1_u32.into(),
                    1_u32.into(),
                    1_u32.into(),
                ),
                Err(ChunkerError::InvalidParameters {
                    reason: InvalidParameterReason::TargetByteExceedsU32,
                })
            ),
            "overflow-prone target byte limits must be rejected"
        );
    }

    /// Unsupported raw versions and policies must be rejected by exact
    /// category.
    #[test]
    fn unsupported_values_are_rejected_by_exact_category()
    {
        assert!(
            matches!(
                AlgorithmVersion::try_from(0xCAFE_u16),
                Err(ChunkerError::UnsupportedAlgorithmVersion { raw: 0xCAFE_u16 })
            ),
            "unsupported algorithm versions must be rejected explicitly"
        );
        assert!(
            matches!(
                GearTableVersion::try_from(0xBEEF_u16),
                Err(ChunkerError::UnsupportedGearTableVersion { raw: 0xBEEF_u16 })
            ),
            "unsupported table versions must be rejected explicitly"
        );
        assert!(
            matches!(
                NormalizationPolicy::try_from(0x7F_u8),
                Err(ChunkerError::UnsupportedNormalizationPolicy { raw: 0x7F_u8 })
            ),
            "unsupported normalization policies must be rejected explicitly"
        );
        assert!(
            matches!(
                RecordBoundaryRule::try_from(0x7E_u8),
                Err(ChunkerError::UnsupportedRecordBoundaryRule { raw: 0x7E_u8 })
            ),
            "unsupported record-boundary rules must be rejected explicitly"
        );
        assert!(
            matches!(
                ChunkerParams::new(
                    AlgorithmVersion::FASTCDC_2020,
                    GearTableVersion::MACH_V1,
                    SeedPolicy::unsupported(SeedKind::from(0xFD_u8)),
                    NormalizationPolicy::NONE,
                    RecordBoundaryRule::BETWEEN_RECORDS,
                    limits(
                        4_u64.into(),
                        16_u64.into(),
                        64_u64.into(),
                        1_u32.into(),
                        4_u32.into(),
                        16_u32.into()
                    ),
                ),
                Err(ChunkerError::UnsupportedSeedPolicy { kind: 0xFD_u8 })
            ),
            "unsupported seed policies must be rejected explicitly"
        );
    }

    /// Empty input must emit no chunk, while below-min input emits a final
    /// remainder.
    #[test]
    fn empty_input_and_final_remainder_are_monotonic()
    {
        let chunker_params = params(limits(
            ByteCount::from(16_u64),
            ByteCount::from(32_u64),
            ByteCount::from(64_u64),
            RecordCount::from(4_u32),
            RecordCount::from(8_u32),
            RecordCount::from(16_u32),
        ));
        let empty_records: [&[u8]; 0] = [];
        let empty_chunks = chunk_record_slices(
            CanonicalRecords::from(empty_records.as_slice()),
            &chunker_params,
        )
        .expect("empty chunking pass must succeed");

        assert!(
            empty_chunks.is_empty(),
            "empty input must not emit an empty final chunk"
        );
        assert_monotonic_chunks(
            &empty_chunks,
            0_u64.into(),
            0_u64.into(),
            ("empty input").into(),
        );

        let records: [&[u8]; 3] = [b"a", b"bc", b"def"];
        let bytes = canonical_bytes(CanonicalRecords::from(records.as_slice()));
        let spans = record_spans(CanonicalRecords::from(records.as_slice()));
        let chunks =
            chunk_record_slices(CanonicalRecords::from(records.as_slice()), &chunker_params)
                .expect("below-min final-remainder chunking pass must succeed");
        let only_chunk = chunks
            .first()
            .expect("below-min non-empty input must emit one final chunk");

        assert_eq!(
            chunks.len(),
            1_usize,
            "below-min input must emit exactly one final remainder chunk"
        );
        assert_chunk_reason(
            only_chunk,
            BoundaryReason::FinalRemainder,
            ("below-min input").into(),
        );
        assert_monotonic_chunks(
            &chunks,
            byte_count(&bytes),
            record_count(CanonicalRecords::from(records.as_slice())),
            ("below-min input").into(),
        );
        assert_record_aligned_chunks(
            &chunks,
            &spans,
            byte_count(&bytes),
            ("below-min input").into(),
        );
    }

    /// Minimum byte and record limits must prevent early hash-predicate cuts.
    #[test]
    fn minimum_limits_suppress_early_hash_predicate_boundaries()
    {
        let records: [&[u8]; 24] = [&[0xAB_u8]; 24];
        let chunker_params = params(limits(
            ByteCount::from(8_u64),
            ByteCount::from(8_u64),
            ByteCount::from(16_u64),
            RecordCount::from(8_u32),
            RecordCount::from(8_u32),
            RecordCount::from(16_u32),
        ));
        let bytes = canonical_bytes(CanonicalRecords::from(records.as_slice()));
        let spans = record_spans(CanonicalRecords::from(records.as_slice()));
        let chunks =
            chunk_record_slices(CanonicalRecords::from(records.as_slice()), &chunker_params)
                .expect("minimum-limit chunking pass must succeed");

        assert_monotonic_chunks(
            &chunks,
            byte_count(&bytes),
            record_count(CanonicalRecords::from(records.as_slice())),
            ("minimum limits").into(),
        );
        assert_record_aligned_chunks(
            &chunks,
            &spans,
            byte_count(&bytes),
            ("minimum limits").into(),
        );

        for chunk in &chunks {
            if chunk.reason == BoundaryReason::FinalRemainder {
                continue;
            }

            assert!(
                chunk_byte_len(chunk) >= ByteCount::from(8_u64),
                "non-final boundaries must not occur before the minimum byte limit"
            );
            assert!(
                chunk_record_len(chunk) >= FixtureRecordDistance::from(8_u64),
                "non-final boundaries must not occur before the minimum record limit"
            );
        }
    }

    /// Max byte and max record caps must force boundaries with precise reasons.
    #[test]
    fn max_caps_force_expected_boundary_reasons()
    {
        let byte_records: [&[u8]; 3] = [b"aaaa", b"bbbb", b"cccc"];
        let byte_params = params(limits(
            ByteCount::from(8_u64),
            ByteCount::from(8_u64),
            ByteCount::from(8_u64),
            RecordCount::from(1_u32),
            RecordCount::from(8_u32),
            RecordCount::from(16_u32),
        ));
        let byte_chunks = chunk_record_slices(
            CanonicalRecords::from(byte_records.as_slice()),
            &byte_params,
        )
        .expect("byte-cap chunking pass must succeed");
        let first_byte_chunk = byte_chunks
            .first()
            .expect("byte-cap input must emit a first chunk");

        assert_chunk_reason(
            first_byte_chunk,
            BoundaryReason::MaxByteCap,
            ("byte cap").into(),
        );
        assert_eq!(
            first_byte_chunk.bytes.start,
            BytePosition::from(0_u64),
            "byte cap chunk must start at byte zero"
        );
        assert_eq!(
            first_byte_chunk.bytes.end,
            BytePosition::from(8_u64),
            "byte cap chunk must end at the max byte cap"
        );
        assert_eq!(
            first_byte_chunk.records.start,
            RecordPosition::from(0_u64),
            "byte cap chunk must start at record zero"
        );
        assert_eq!(
            first_byte_chunk.records.end,
            RecordPosition::from(2_u64),
            "byte cap chunk must include two four-byte records"
        );

        let record_records: [&[u8]; 5] = [b"a", b"b", b"c", b"d", b"e"];
        let record_params = params(limits(
            ByteCount::from(1_u64),
            ByteCount::from(64_u64),
            ByteCount::from(128_u64),
            RecordCount::from(3_u32),
            RecordCount::from(3_u32),
            RecordCount::from(3_u32),
        ));
        let record_chunks = chunk_record_slices(
            CanonicalRecords::from(record_records.as_slice()),
            &record_params,
        )
        .expect("record-cap chunking pass must succeed");
        let first_record_chunk = record_chunks
            .first()
            .expect("record-cap input must emit a first chunk");

        assert_chunk_reason(
            first_record_chunk,
            BoundaryReason::MaxRecordCap,
            ("record cap").into(),
        );
        assert_eq!(
            first_record_chunk.records.start,
            RecordPosition::from(0_u64),
            "record cap chunk must start at record zero"
        );
        assert_eq!(
            first_record_chunk.records.end,
            RecordPosition::from(3_u64),
            "record cap chunk must end at the max record cap"
        );
    }

    /// Oversized single records must error rather than split inside the record.
    #[test]
    fn oversized_single_record_returns_precise_error()
    {
        let oversized = [0x11_u8; 9];
        let records: [&[u8]; 1] = [&oversized];
        let chunker_params = params(limits(
            ByteCount::from(1_u64),
            ByteCount::from(8_u64),
            ByteCount::from(8_u64),
            RecordCount::from(1_u32),
            RecordCount::from(1_u32),
            RecordCount::from(4_u32),
        ));

        assert!(
            matches!(
                chunk_record_slices(CanonicalRecords::from(records.as_slice()), &chunker_params),
                Err(ChunkerError::RecordByteLengthCapViolation {
                    record_index: 0_u64,
                    record_len: 9_u64,
                    max_bytes: 8_u64,
                })
            ),
            "oversized single records must return the record length cap error"
        );
    }

    /// Min-record requirements that cannot fit under byte caps must error
    /// precisely.
    #[test]
    fn impossible_min_record_chunk_returns_chunk_cap_error()
    {
        let first = [0x21_u8; 5];
        let second = [0x22_u8; 5];
        let records: [&[u8]; 2] = [&first, &second];
        let chunker_params = params(limits(
            ByteCount::from(1_u64),
            ByteCount::from(8_u64),
            ByteCount::from(8_u64),
            RecordCount::from(2_u32),
            RecordCount::from(2_u32),
            RecordCount::from(4_u32),
        ));

        assert!(
            matches!(
                chunk_record_slices(CanonicalRecords::from(records.as_slice()), &chunker_params),
                Err(ChunkerError::ChunkByteCapViolation {
                    chunk_start_record: 0_u64,
                    next_record_index: 1_u64,
                    attempted_bytes: 10_u64,
                    max_bytes: 8_u64,
                })
            ),
            "chunks that cannot satisfy min records within max bytes must error precisely"
        );
    }

    /// Non-monotonic record spans must be rejected with the offending index.
    #[test]
    fn non_monotonic_record_spans_are_rejected()
    {
        let bytes = b"abcdef";
        let spans = [
            ByteSpan::new(BytePosition::from(0_u64), BytePosition::from(3_u64)),
            ByteSpan::new(BytePosition::from(2_u64), BytePosition::from(6_u64)),
        ];
        let chunker_params = params(limits(
            ByteCount::from(1_u64),
            ByteCount::from(4_u64),
            ByteCount::from(8_u64),
            RecordCount::from(1_u32),
            RecordCount::from(2_u32),
            RecordCount::from(4_u32),
        ));

        assert!(
            matches!(
                chunk_spans(
                    CanonicalBytes::from(bytes.as_slice()),
                    &spans,
                    &chunker_params
                ),
                Err(ChunkerError::NonMonotonicRecordSpans { index: 1_usize })
            ),
            "overlapping record spans must report the first non-monotonic span index"
        );
    }

    /// Low-entropy repeated bytes must remain deterministic and cap-bounded.
    #[test]
    fn low_entropy_repeated_byte_streams_are_deterministic_and_bounded()
    {
        let repeated_record = [0_u8; 8];
        let records: [&[u8]; 20] = [&repeated_record; 20];
        let chunker_params = params(limits(
            ByteCount::from(16_u64),
            ByteCount::from(32_u64),
            ByteCount::from(48_u64),
            RecordCount::from(1_u32),
            RecordCount::from(4_u32),
            RecordCount::from(16_u32),
        ));
        let bytes = canonical_bytes(CanonicalRecords::from(records.as_slice()));
        let spans = record_spans(CanonicalRecords::from(records.as_slice()));

        let first =
            chunk_record_slices(CanonicalRecords::from(records.as_slice()), &chunker_params)
                .expect("first repeated-byte chunking pass must succeed");
        let second =
            chunk_record_slices(CanonicalRecords::from(records.as_slice()), &chunker_params)
                .expect("second repeated-byte chunking pass must succeed");

        assert_equivalent_chunks(&first, &second, ("repeated-byte stream").into());
        assert_monotonic_chunks(
            &first,
            byte_count(&bytes),
            record_count(CanonicalRecords::from(records.as_slice())),
            ("repeated-byte stream").into(),
        );
        assert_record_aligned_chunks(
            &first,
            &spans,
            byte_count(&bytes),
            ("repeated-byte stream").into(),
        );
        assert_chunks_within_caps(
            &first,
            ByteCount::from(48_u64),
            FixtureRecordDistance::from(16_u64),
            ("repeated-byte stream").into(),
        );
    }

    /// Many tiny records and near-cap records must stay aligned and bounded.
    #[test]
    fn many_tiny_records_and_near_cap_records_stay_aligned_and_bounded()
    {
        let tiny_records: [&[u8]; 64] = [&[0x7F_u8]; 64];
        let tiny_params = params(limits(
            ByteCount::from(1_u64),
            ByteCount::from(16_u64),
            ByteCount::from(32_u64),
            RecordCount::from(7_u32),
            RecordCount::from(7_u32),
            RecordCount::from(7_u32),
        ));
        let tiny_bytes = canonical_bytes(CanonicalRecords::from(tiny_records.as_slice()));
        let tiny_spans = record_spans(CanonicalRecords::from(tiny_records.as_slice()));
        let tiny_chunks = chunk_record_slices(
            CanonicalRecords::from(tiny_records.as_slice()),
            &tiny_params,
        )
        .expect("many-tiny-record chunking pass must succeed");

        assert!(
            tiny_chunks.len() > 1_usize,
            "many tiny records must produce multiple bounded chunks"
        );
        assert_has_reason(
            &tiny_chunks,
            BoundaryReason::MaxRecordCap,
            ("many tiny records").into(),
        );
        assert_monotonic_chunks(
            &tiny_chunks,
            byte_count(&tiny_bytes),
            record_count(CanonicalRecords::from(tiny_records.as_slice())),
            ("many tiny records").into(),
        );
        assert_record_aligned_chunks(
            &tiny_chunks,
            &tiny_spans,
            byte_count(&tiny_bytes),
            ("many tiny records").into(),
        );
        assert_chunks_within_caps(
            &tiny_chunks,
            ByteCount::from(32_u64),
            FixtureRecordDistance::from(7_u64),
            ("many tiny records").into(),
        );

        let max_sized = [0x42_u8; 8];
        let near_max = [0x24_u8; 7];
        let near_cap_records: [&[u8]; 4] = [&max_sized, &near_max, &max_sized, &near_max];
        let near_cap_params = params(limits(
            ByteCount::from(1_u64),
            ByteCount::from(8_u64),
            ByteCount::from(8_u64),
            RecordCount::from(1_u32),
            RecordCount::from(4_u32),
            RecordCount::from(16_u32),
        ));
        let near_cap_bytes = canonical_bytes(CanonicalRecords::from(near_cap_records.as_slice()));
        let near_cap_spans = record_spans(CanonicalRecords::from(near_cap_records.as_slice()));
        let near_cap_chunks = chunk_record_slices(
            CanonicalRecords::from(near_cap_records.as_slice()),
            &near_cap_params,
        )
        .expect("near-cap-record chunking pass must succeed");

        assert_monotonic_chunks(
            &near_cap_chunks,
            byte_count(&near_cap_bytes),
            record_count(CanonicalRecords::from(near_cap_records.as_slice())),
            ("near-cap records").into(),
        );
        assert_record_aligned_chunks(
            &near_cap_chunks,
            &near_cap_spans,
            byte_count(&near_cap_bytes),
            ("near-cap records").into(),
        );
        assert_chunks_within_caps(
            &near_cap_chunks,
            ByteCount::from(8_u64),
            FixtureRecordDistance::from(16_u64),
            ("near-cap records").into(),
        );
    }
}
