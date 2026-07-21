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

#[cfg(test)]
mod tests
{
    use gandr_storage_chunker::AlgorithmVersion;
    use gandr_storage_chunker::ChunkLimits;
    use gandr_storage_chunker::ChunkerParams;
    use gandr_storage_chunker::GearTableVersion;
    use gandr_storage_chunker::NormalizationPolicy;
    use gandr_storage_chunker::RecordBoundaryRule;
    use gandr_storage_chunker::SeedPolicy;
    use gandr_storage_prolly_trees::EncodingVersion;
    use gandr_storage_prolly_trees::HashAlgorithm;
    use gandr_storage_prolly_trees::PortableProofTree;
    use gandr_storage_prolly_trees::ProllyBaoError;
    use gandr_storage_prolly_trees::RecordRef;
    use gandr_storage_prolly_trees::SeparatorConvention;
    use gandr_storage_prolly_trees::TreeKind;
    use gandr_storage_prolly_trees::TreeParams;
    use gandr_storage_prolly_trees::WitnessTranscript;
    use gandr_storage_prolly_trees::verify_snapshot_bytes;

    const SNAPSHOT_MAGIC: &[u8] = b"prolly-bao:snapshot:v1";
    const WITNESS_MAGIC: &[u8] = b"prolly-bao:witness:v1";

    #[derive(Default)]
    struct DeterministicBlobCache
    {
        entries: Vec<(blake3::Hash, Box<[u8]>)>,
    }

    impl DeterministicBlobCache
    {
        fn store(
            &mut self,
            bytes: &[u8],
        ) -> blake3::Hash
        {
            let hash = blake3::hash(bytes);

            if self.read(&hash).is_none() {
                self.entries.push((hash, Box::<[u8]>::from(bytes)));
            }

            return hash;
        }

        fn read(
            &self,
            hash: &blake3::Hash,
        ) -> Option<&[u8]>
        {
            for entry in self.entries.as_slice() {
                let (stored_hash, bytes) = (&entry.0, entry.1.as_ref());
                if stored_hash == hash {
                    return Some(bytes);
                }
            }

            return None;
        }
    }

    #[test]
    fn snapshot_bytes_are_deterministic_cacheable_and_root_bound_without_iroh()
    {
        let first_tree = build_tree();
        let second_tree = build_tree();
        let snapshot_bytes = first_tree
            .to_snapshot_bytes()
            .expect("fixture tree should encode snapshot bytes");
        let rebuilt_snapshot_bytes = second_tree
            .to_snapshot_bytes()
            .expect("rebuilt fixture tree should encode snapshot bytes");

        assert_eq!(
            first_tree.root(),
            second_tree.root(),
            "equivalent records and parameters should rebuild the same root"
        );
        assert_eq!(
            snapshot_bytes.as_ref(),
            rebuilt_snapshot_bytes.as_ref(),
            "canonical snapshots should be byte-identical for equivalent rebuilds"
        );

        let mut cache = DeterministicBlobCache::default();
        let snapshot_hash = cache.store(snapshot_bytes.as_ref());
        let readback = cache
            .read(&snapshot_hash)
            .expect("stored snapshot bytes should be present under their flat BLAKE3 hash");

        assert_eq!(
            readback,
            snapshot_bytes.as_ref(),
            "cache readback should recover the exact canonical snapshot bytes"
        );

        let verified =
            verify_snapshot_bytes(readback, first_tree.root(), first_tree.root().params())
                .expect("snapshot bytes should remain bound to the Prolly-Bao root");
        assert_eq!(
            verified.root(),
            first_tree.root(),
            "snapshot verification should rebuild the expected root"
        );
        assert_eq!(
            verified.root_node_hash(),
            first_tree.root_node_hash(),
            "snapshot verification should bind the expected root-node hash"
        );

        let mut tampered = Vec::<u8>::from(snapshot_bytes.as_ref());
        flip_last_byte(tampered.as_mut_slice());
        assert!(
            verify_snapshot_bytes(
                tampered.as_slice(),
                first_tree.root(),
                first_tree.root().params()
            )
            .is_err(),
            "mutated snapshot bytes must not verify against the original root"
        );
        assert!(
            cache.read(&blake3::hash(tampered.as_slice())).is_none(),
            "a wrong content hash must not return the original cached snapshot"
        );
        assert!(
            cache
                .read(&blake3::hash(b"missing snapshot blob"))
                .is_none(),
            "missing local cache entries must stay absent"
        );

        let wrong_root_tree = build_wrong_root_tree();
        let wrong_root_error = verify_snapshot_bytes(
            snapshot_bytes.as_ref(),
            wrong_root_tree.root(),
            wrong_root_tree.root().params(),
        )
        .expect_err("snapshot bytes must not verify against a different root context");
        assert!(
            matches!(
                wrong_root_error,
                ProllyBaoError::HashMismatch { .. } | ProllyBaoError::InvalidProofShape { .. }
            ),
            "wrong root should fail through the root/hash verifier path"
        );

        let truncated = prefix_without_last_byte(snapshot_bytes.as_ref());
        assert!(
            verify_snapshot_bytes(truncated, first_tree.root(), first_tree.root().params())
                .is_err(),
            "truncated snapshot bytes must fail verification"
        );

        let unsupported_version =
            bytes_with_unsupported_version(snapshot_bytes.as_ref(), SNAPSHOT_MAGIC);
        let unsupported_error = verify_snapshot_bytes(
            unsupported_version.as_slice(),
            first_tree.root(),
            first_tree.root().params(),
        )
        .expect_err("unsupported snapshot versions must fail closed");
        assert!(
            matches!(
                unsupported_error,
                ProllyBaoError::UnsupportedSnapshotVersion { version: 2_u16 }
            ),
            "unsupported snapshot version should use the dedicated error"
        );
    }

    #[test]
    fn witness_bytes_are_deterministic_cacheable_and_semantically_root_bound()
    {
        let first_tree = build_tree();
        let second_tree = build_tree();
        let witness_bytes = membership_witness_bytes(&first_tree, b"c");
        let rebuilt_witness_bytes = membership_witness_bytes(&second_tree, b"c");

        assert_eq!(
            first_tree.root(),
            second_tree.root(),
            "equivalent records and parameters should rebuild the same witness root"
        );
        assert_eq!(
            witness_bytes.as_ref(),
            rebuilt_witness_bytes.as_ref(),
            "membership witness bytes should be deterministic for equivalent rebuilds"
        );

        let mut cache = DeterministicBlobCache::default();
        let witness_hash = cache.store(witness_bytes.as_ref());
        let readback = cache
            .read(&witness_hash)
            .expect("stored witness bytes should be present under their flat BLAKE3 hash");

        assert_eq!(
            readback,
            witness_bytes.as_ref(),
            "cache readback should recover the exact witness bytes"
        );

        let decoded = WitnessTranscript::decode(readback)
            .expect("readback witness bytes should decode as a native transcript");
        decoded
            .verify_membership(
                first_tree.root(),
                first_tree.root().params(),
                b"c",
                b"charlie",
            )
            .expect("witness semantics should remain bound to the Prolly-Bao root");

        let wrong_root_tree = build_wrong_root_tree();
        let wrong_root_error = decoded
            .verify_membership(
                wrong_root_tree.root(),
                wrong_root_tree.root().params(),
                b"c",
                b"charlie",
            )
            .expect_err("witness bytes must not verify under a different root context");
        assert!(
            matches!(
                wrong_root_error,
                ProllyBaoError::HashMismatch { .. } | ProllyBaoError::InvalidProofShape { .. }
            ),
            "wrong root should fail through the native witness verifier path"
        );

        let mut tampered = Vec::<u8>::from(witness_bytes.as_ref());
        flip_last_byte(tampered.as_mut_slice());
        assert!(
            WitnessTranscript::decode(tampered.as_slice()).is_err(),
            "mutated witness bytes must fail decode before semantic verification"
        );
        assert!(
            cache.read(&blake3::hash(tampered.as_slice())).is_none(),
            "a wrong content hash must not return the original cached witness"
        );
        assert!(
            cache.read(&blake3::hash(b"missing witness blob")).is_none(),
            "missing local cache entries must stay absent"
        );

        let truncated = prefix_without_last_byte(witness_bytes.as_ref());
        assert!(
            WitnessTranscript::decode(truncated).is_err(),
            "truncated witness bytes must fail decode"
        );

        let unsupported_version =
            bytes_with_unsupported_version(witness_bytes.as_ref(), WITNESS_MAGIC);
        let unsupported_error = WitnessTranscript::decode(unsupported_version.as_slice())
            .expect_err("unsupported witness versions must fail closed");
        assert!(
            matches!(
                unsupported_error,
                ProllyBaoError::UnsupportedWitnessVersion { version: 2_u16 }
            ),
            "unsupported witness version should use the dedicated error"
        );
    }

    fn records() -> Vec<RecordRef<'static>>
    {
        return vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"b", b"bravo"),
            RecordRef::new(b"c", b"charlie"),
            RecordRef::new(b"d", b"delta"),
            RecordRef::new(b"e", b"echo"),
            RecordRef::new(b"f", b"foxtrot"),
        ];
    }

    fn wrong_root_records() -> Vec<RecordRef<'static>>
    {
        return vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"b", b"bravo"),
            RecordRef::new(b"c", b"changed"),
            RecordRef::new(b"d", b"delta"),
            RecordRef::new(b"e", b"echo"),
            RecordRef::new(b"f", b"foxtrot"),
        ];
    }

    fn compact_params() -> TreeParams
    {
        let limits = ChunkLimits::new(1_u64, 64_u64, 4096_u64, 2_u32, 2_u32, 2_u32)
            .expect("compact chunk limits should be valid");
        let chunker_params = ChunkerParams::new(
            AlgorithmVersion::FASTCDC_2020,
            GearTableVersion::MACH_V1,
            SeedPolicy::NONE,
            NormalizationPolicy::NONE,
            RecordBoundaryRule::BETWEEN_RECORDS,
            limits,
        )
        .expect("compact chunker params should be valid");

        return TreeParams::new(
            TreeKind::MerkleSearch,
            EncodingVersion::CURRENT,
            HashAlgorithm::CURRENT,
            SeparatorConvention::CURRENT,
            chunker_params,
        );
    }

    fn build_tree() -> PortableProofTree
    {
        let fixture_records = records();

        return PortableProofTree::build(fixture_records.as_slice(), compact_params())
            .expect("sorted deterministic records should build a proof tree");
    }

    fn build_wrong_root_tree() -> PortableProofTree
    {
        let fixture_records = wrong_root_records();

        return PortableProofTree::build(fixture_records.as_slice(), compact_params())
            .expect("changed fixture records should build a different proof tree");
    }

    fn membership_witness_bytes(
        tree: &PortableProofTree,
        key: &[u8],
    ) -> Box<[u8]>
    {
        let proof = tree
            .prove_membership(key)
            .expect("existing key should produce a membership proof");
        let witness = WitnessTranscript::from_membership_proof(proof)
            .expect("membership proof should form a witness transcript");

        return witness
            .to_bytes()
            .expect("membership witness should encode to deterministic bytes");
    }

    fn flip_last_byte(bytes: &mut [u8])
    {
        let last = bytes
            .last_mut()
            .expect("fixture bytes should include at least one byte");
        *last ^= 0x01_u8;
    }

    fn prefix_without_last_byte(bytes: &[u8]) -> &[u8]
    {
        let truncated_len = bytes
            .len()
            .checked_sub(1_usize)
            .expect("fixture bytes should include at least one byte");

        return bytes
            .get(.. truncated_len)
            .expect("truncated prefix should be in bounds");
    }

    fn bytes_with_unsupported_version(
        bytes: &[u8],
        magic: &[u8],
    ) -> Vec<u8>
    {
        let mut unsupported_version = Vec::<u8>::from(bytes);
        let version_offset = magic.len();
        let version_end = version_offset
            .checked_add(2_usize)
            .expect("version field offset should not overflow");
        unsupported_version
            .get_mut(version_offset .. version_end)
            .expect("fixture bytes should include a version field")
            .copy_from_slice(2_u16.to_be_bytes().as_ref());

        return unsupported_version;
    }
}
