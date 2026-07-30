#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::vec::Vec;

    use gandr_storage_chunker::AlgorithmVersion;
    use gandr_storage_chunker::ChunkLimits;
    use gandr_storage_chunker::ChunkerParams;
    use gandr_storage_chunker::GearTableVersion;
    use gandr_storage_chunker::NormalizationPolicy;
    use gandr_storage_chunker::RecordBoundaryRule;
    use gandr_storage_chunker::SeedPolicy;
    use gandr_storage_prolly_trees::BlockStore as _;
    use gandr_storage_prolly_trees::EncodingVersion;
    use gandr_storage_prolly_trees::HashAlgorithm;
    use gandr_storage_prolly_trees::NODE_HASH_LEN;
    use gandr_storage_prolly_trees::NodeHash;
    use gandr_storage_prolly_trees::NodeSegmentEntry;
    use gandr_storage_prolly_trees::PackedSegmentBytes;
    use gandr_storage_prolly_trees::PackedSegmentLength;
    use gandr_storage_prolly_trees::PackedSegmentOffset;
    use gandr_storage_prolly_trees::PackedSegmentStore;
    use gandr_storage_prolly_trees::PortableProofTree;
    use gandr_storage_prolly_trees::ProllyBaoError;
    use gandr_storage_prolly_trees::ProllyTree;
    use gandr_storage_prolly_trees::ProofNode;
    use gandr_storage_prolly_trees::RecordRef;
    use gandr_storage_prolly_trees::SeparatorConvention;
    use gandr_storage_prolly_trees::StoredNodeRef;
    use gandr_storage_prolly_trees::TreeKind;
    use gandr_storage_prolly_trees::TreeParams;
    use gandr_storage_prolly_trees::hash_encoded_node;
    use gandr_storage_prolly_trees::verify_stored_node;

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

    fn compact_params() -> TreeParams
    {
        let limits = ChunkLimits::new(
            1_u64.into(),
            64_u64.into(),
            4096_u64.into(),
            2_u32.into(),
            2_u32.into(),
            2_u32.into(),
        )
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

    fn packed_store_from_tree(tree: &PortableProofTree) -> PackedSegmentStore
    {
        let mut store = PackedSegmentStore::new();

        for node in tree.nodes() {
            store
                .insert(StoredNodeRef::new(node.hash(), node.bytes()))
                .expect("valid proof node should insert into the packed store");
        }

        return store;
    }

    fn raw_parts_from_tree(
        tree: &PortableProofTree
    ) -> (PackedSegmentBytes, BTreeMap<NodeHash, NodeSegmentEntry>)
    {
        return packed_store_from_tree(tree).into_raw_parts();
    }

    fn first_node(tree: &PortableProofTree) -> &ProofNode
    {
        return tree
            .nodes()
            .first()
            .expect("fixture tree should contain a root proof node");
    }

    fn different_hash(hash: NodeHash) -> NodeHash
    {
        let mut bytes: [u8; NODE_HASH_LEN] = hash.into();
        let first_byte = bytes
            .first_mut()
            .expect("node hash bytes should include a first byte");
        *first_byte ^= 0xff_u8;

        return NodeHash::from(bytes);
    }

    #[test]
    fn packed_store_loads_inserted_proof_nodes_and_opens_tree()
    {
        let tree = build_tree();
        let store = packed_store_from_tree(&tree);

        for node in tree.nodes() {
            let loaded = store
                .load(node.hash())
                .expect("inserted packed node should load by hash");
            verify_stored_node(loaded)
                .expect("loaded packed node should verify before callers trust it");
            assert_eq!(
                loaded.node_hash(),
                node.hash(),
                "loaded node should carry the requested hash"
            );
            assert_eq!(
                loaded.bytes(),
                node.bytes(),
                "loaded node bytes should borrow the stored segment range"
            );
        }

        let opened = ProllyTree::open(tree.root().clone(), tree.root_node_hash(), &store)
            .expect("packed store should open the inserted root node");
        assert_eq!(
            opened.root(),
            tree.root(),
            "opened handle should preserve the supplied root manifest"
        );
        assert_eq!(
            opened.root_node_hash(),
            tree.root_node_hash(),
            "opened handle should report the packed-store-verified root node"
        );
    }

    #[test]
    fn packed_store_rejects_missing_hash()
    {
        let tree = build_tree();
        let store = packed_store_from_tree(&tree);
        let missing_hash = NodeHash::from([0xa5_u8; NODE_HASH_LEN]);
        let missing_error = store
            .load(missing_hash)
            .expect_err("missing packed node hash should be rejected");

        assert!(
            matches!(missing_error, ProllyBaoError::UnknownNodeHash { .. }),
            "missing packed node should fail with the store boundary error"
        );
    }

    #[test]
    fn raw_packed_segment_constructor_rejects_invalid_material()
    {
        let tree = build_tree();
        let first_proof_node = first_node(&tree);

        let (mismatch_segment, valid_index) = raw_parts_from_tree(&tree);
        let first_entry = *valid_index
            .get(&first_proof_node.hash())
            .expect("valid raw index should contain the first proof node");
        let mut mismatch_index = BTreeMap::new();
        let _mismatch_previous =
            mismatch_index.insert(different_hash(first_proof_node.hash()), first_entry);
        let mismatch_error = PackedSegmentStore::from_raw_parts(mismatch_segment, mismatch_index)
            .expect_err("raw index keyed by the wrong hash should be rejected");
        assert!(
            matches!(mismatch_error, ProllyBaoError::HashMismatch { .. }),
            "mismatched raw segment bytes should fail hash verification"
        );

        let malformed_segment =
            PackedSegmentBytes::from(Vec::<u8>::from(b"not-a-prolly-bao-node".as_slice()));
        let malformed_hash = hash_encoded_node(malformed_segment.as_ref().into());
        let mut malformed_index = BTreeMap::new();
        let _malformed_previous = malformed_index.insert(
            malformed_hash,
            NodeSegmentEntry::new(
                PackedSegmentOffset::from(0_usize),
                PackedSegmentLength::from(malformed_segment.as_ref().len()),
            ),
        );
        let malformed_error =
            PackedSegmentStore::from_raw_parts(malformed_segment, malformed_index)
                .expect_err("raw bytes with a matching hash but invalid node framing should fail");
        assert!(
            matches!(malformed_error, ProllyBaoError::MalformedNodeBytes { .. }),
            "malformed raw node bytes should fail closed during constructor verification"
        );

        let (bounds_segment, mut bounds_index) = raw_parts_from_tree(&tree);
        let _bounds_previous = bounds_index.insert(
            first_proof_node.hash(),
            NodeSegmentEntry::new(
                PackedSegmentOffset::from(bounds_segment.as_ref().len()),
                first_entry.length(),
            ),
        );
        let bounds_error = PackedSegmentStore::from_raw_parts(bounds_segment, bounds_index)
            .expect_err("out-of-bounds raw segment entry should be rejected");
        assert!(
            matches!(bounds_error, ProllyBaoError::MalformedNodeBytes { .. }),
            "out-of-bounds raw index material should fail closed"
        );

        let (overlap_segment, _) = raw_parts_from_tree(&tree);
        let mut overlap_index = BTreeMap::new();
        let _overlap_first_previous = overlap_index.insert(first_proof_node.hash(), first_entry);
        let _overlap_second_previous =
            overlap_index.insert(different_hash(first_proof_node.hash()), first_entry);
        let overlap_error = PackedSegmentStore::from_raw_parts(overlap_segment, overlap_index)
            .expect_err("overlapping raw segment entries should be rejected");
        assert!(
            matches!(overlap_error, ProllyBaoError::MalformedNodeBytes { .. }),
            "overlapping raw index ranges should fail closed"
        );

        let (mut corrupt_segment, corrupt_index) = raw_parts_from_tree(&tree);
        let corrupt_entry = *corrupt_index
            .get(&first_proof_node.hash())
            .expect("valid raw index should contain the first proof node");
        let corrupt_byte = corrupt_segment
            .as_mut()
            .get_mut(usize::from(corrupt_entry.offset()))
            .expect("valid raw entry offset should point inside the segment");
        *corrupt_byte ^= 0x01_u8;
        let corrupt_error = PackedSegmentStore::from_raw_parts(corrupt_segment, corrupt_index)
            .expect_err("corrupt raw segment bytes should be rejected");
        assert!(
            matches!(corrupt_error, ProllyBaoError::HashMismatch { .. }),
            "corrupt raw segment material should fail hash verification"
        );
    }
}
