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
    use gandr_storage_prolly_trees::EncodedNodeKind;
    use gandr_storage_prolly_trees::EncodedNodeLayout;
    use gandr_storage_prolly_trees::NODE_HASH_LEN;
    use gandr_storage_prolly_trees::PortableProofTree;
    use gandr_storage_prolly_trees::ProllyBaoError;
    use gandr_storage_prolly_trees::ProofNode;
    use gandr_storage_prolly_trees::RecordRef;
    use gandr_storage_prolly_trees::TreeParams;
    use gandr_storage_prolly_trees::hash_encoded_node;
    use gandr_storage_prolly_trees::inspect_encoded_node;

    const NODE_MAGIC: &[u8] = b"prolly-bao:node:v1";
    const NODE_VERSION_LEN: usize = 2_usize;
    const NODE_KIND_LEN: usize = 1_usize;
    const NODE_KIND_OFFSET: usize = NODE_MAGIC.len() + NODE_VERSION_LEN;
    const NODE_HEADER_LEN: usize = NODE_KIND_OFFSET + NODE_KIND_LEN;
    const U64_LEN: usize = 8_usize;
    const INTERNAL_CHILD_SECTION_OFFSET: usize = NODE_HEADER_LEN + U64_LEN + U64_LEN;

    fn records() -> Vec<RecordRef<'static>>
    {
        return vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"b", b"bravo"),
            RecordRef::new(b"c", b"charlie"),
        ];
    }

    fn one_record_per_leaf_params() -> TreeParams
    {
        let limits = ChunkLimits::new(
            1_u64.into(),
            64_u64.into(),
            4096_u64.into(),
            1_u32.into(),
            1_u32.into(),
            1_u32.into(),
        )
        .expect("one-record leaf chunk limits should be valid");
        let chunker_params = ChunkerParams::new(
            AlgorithmVersion::FASTCDC_2020,
            GearTableVersion::MACH_V1,
            SeedPolicy::NONE,
            NormalizationPolicy::NONE,
            RecordBoundaryRule::BETWEEN_RECORDS,
            limits,
        )
        .expect("one-record leaf chunker params should be valid");

        return TreeParams::new(
            gandr_storage_prolly_trees::TreeKind::MerkleSearch,
            gandr_storage_prolly_trees::EncodingVersion::CURRENT,
            gandr_storage_prolly_trees::HashAlgorithm::CURRENT,
            gandr_storage_prolly_trees::SeparatorConvention::CURRENT,
            chunker_params,
        );
    }

    fn build_multi_leaf_tree() -> PortableProofTree
    {
        let fixture_records = records();

        return PortableProofTree::build(fixture_records.as_slice(), one_record_per_leaf_params())
            .expect("one-record leaf params should build a multi-leaf proof tree");
    }

    #[test]
    fn empty_tree_proof_node_reports_empty_leaf_layout()
    {
        let empty_records: [RecordRef<'static>; 0_usize] = [];
        let tree = PortableProofTree::build(empty_records.as_slice(), TreeParams::current())
            .expect("empty ordered records should build an empty proof tree");
        let node = tree
            .nodes()
            .first()
            .expect("empty tree should still carry one root leaf node");
        let layout = inspect_proof_node(node);

        assert_eq!(
            layout.kind(),
            EncodedNodeKind::Leaf,
            "empty tree root should inspect as a leaf node"
        );
        assert_eq!(
            layout.record_count(),
            Some(0_u64),
            "empty leaf should report zero records"
        );
        assert_eq!(
            layout.child_count(),
            None,
            "leaf layout should not report an internal child count"
        );
        assert!(layout.is_empty(), "zero-record leaf should be marked empty");
    }

    #[test]
    fn one_record_tree_proof_node_reports_non_empty_leaf_layout()
    {
        let fixture_records = [RecordRef::new(b"a", b"alpha")];
        let tree = PortableProofTree::build(fixture_records.as_slice(), TreeParams::current())
            .expect("single ordered record should build a one-leaf proof tree");
        let node = tree
            .nodes()
            .first()
            .expect("one-record tree should carry one root leaf node");
        let layout = inspect_proof_node(node);

        assert_eq!(
            layout.kind(),
            EncodedNodeKind::Leaf,
            "single-record root should inspect as a leaf node"
        );
        assert_eq!(
            layout.record_count(),
            Some(1_u64),
            "single-record leaf should report one record"
        );
        assert_eq!(
            layout.child_count(),
            None,
            "leaf layout should not report an internal child count"
        );
        assert!(
            !layout.is_empty(),
            "single-record leaf should not be marked empty"
        );
    }

    #[test]
    fn multi_leaf_tree_proof_nodes_report_internal_and_leaf_layouts()
    {
        let tree = build_multi_leaf_tree();
        let nodes = tree.nodes();
        let root_node = nodes
            .first()
            .expect("multi-leaf tree should carry an internal root node");
        let root_layout = inspect_proof_node(root_node);
        let leaf_nodes = nodes
            .get(1_usize ..)
            .expect("multi-leaf tree should carry leaf proof nodes after the root");

        assert_eq!(
            root_layout.kind(),
            EncodedNodeKind::Internal,
            "multi-leaf root should inspect as an internal node"
        );
        assert_eq!(
            root_layout.record_count(),
            None,
            "internal layout should not report a leaf record count"
        );
        assert_eq!(
            root_layout.child_count(),
            Some(3_u64),
            "one-record-per-leaf params should produce three child references"
        );
        assert!(
            !root_layout.is_empty(),
            "internal root with child references should not be marked empty"
        );
        assert_eq!(
            leaf_nodes.len(),
            3_usize,
            "multi-leaf fixture should expose one proof leaf per record"
        );

        for leaf_node in leaf_nodes {
            let leaf_layout = inspect_proof_node(leaf_node);
            assert_eq!(
                leaf_layout.kind(),
                EncodedNodeKind::Leaf,
                "each child proof node should inspect as a leaf"
            );
            assert_eq!(
                leaf_layout.record_count(),
                Some(1_u64),
                "one-record-per-leaf params should produce one record per leaf"
            );
            assert_eq!(
                leaf_layout.child_count(),
                None,
                "leaf child nodes should not report internal child counts"
            );
            assert!(
                !leaf_layout.is_empty(),
                "single-record child leaves should not be marked empty"
            );
        }
    }

    #[test]
    fn encoded_node_inspector_rejects_malformed_node_bytes_fail_closed()
    {
        let valid_leaf = single_leaf_bytes();

        let mut wrong_magic = valid_leaf.clone();
        let wrong_magic_first = wrong_magic
            .first_mut()
            .expect("valid leaf bytes should include node magic");
        *wrong_magic_first ^= 0x01_u8;
        let wrong_magic_error = expect_inspection_failure(
            wrong_magic.as_slice(),
            "wrong node magic should fail inspection",
        );
        assert_malformed_node_error(
            &wrong_magic_error,
            "wrong node magic should report malformed node bytes",
        );

        let mut unsupported_kind = valid_leaf.clone();
        let unsupported_kind_slot = unsupported_kind
            .get_mut(NODE_KIND_OFFSET)
            .expect("valid leaf bytes should include a kind byte");
        *unsupported_kind_slot = 0xff_u8;
        let unsupported_kind_error = expect_inspection_failure(
            unsupported_kind.as_slice(),
            "unsupported node kind should fail inspection",
        );
        assert_malformed_node_error(
            &unsupported_kind_error,
            "unsupported node kind should report malformed node bytes",
        );

        let truncated_len = NODE_HEADER_LEN + 4_usize;
        let truncated_length = valid_leaf
            .get(.. truncated_len)
            .expect("valid leaf bytes should include a truncated record-count prefix");
        let truncated_length_error = expect_inspection_failure(
            truncated_length,
            "truncated leaf length field should fail inspection",
        );
        assert_malformed_node_error(
            &truncated_length_error,
            "truncated leaf length field should report malformed node bytes",
        );

        let mut trailing_bytes = valid_leaf.clone();
        trailing_bytes.push(0x00_u8);
        let trailing_bytes_error = expect_inspection_failure(
            trailing_bytes.as_slice(),
            "trailing node bytes should fail inspection",
        );
        assert_malformed_node_error(
            &trailing_bytes_error,
            "trailing node bytes should report malformed node bytes",
        );

        let mut malformed_child_reference = internal_root_bytes();
        let child_record_count_offset =
            first_child_record_count_offset(malformed_child_reference.as_slice());
        overwrite_be_u64(
            malformed_child_reference.as_mut_slice(),
            child_record_count_offset,
            0_u64,
        );
        let malformed_child_error = expect_inspection_failure(
            malformed_child_reference.as_slice(),
            "malformed child reference should fail inspection",
        );
        assert_malformed_node_error(
            &malformed_child_error,
            "malformed child reference should report malformed node bytes",
        );
    }

    fn inspect_proof_node(node: &ProofNode) -> EncodedNodeLayout
    {
        let layout = inspect_encoded_node(node.bytes())
            .expect("generated proof node bytes should inspect successfully");

        assert_eq!(
            layout.encoded_len(),
            node.bytes().len(),
            "layout should report the exact canonical node byte length"
        );
        assert_eq!(
            hash_encoded_node(node.bytes()),
            node.hash(),
            "layout inspection must preserve BLAKE3(encoded_node_bytes) identity"
        );

        return layout;
    }

    fn single_leaf_bytes() -> Vec<u8>
    {
        let fixture_records = [RecordRef::new(b"a", b"alpha")];
        let tree = PortableProofTree::build(fixture_records.as_slice(), TreeParams::current())
            .expect("single-record fixture should build a leaf root");
        let node = tree
            .nodes()
            .first()
            .expect("single-record fixture should carry a root node");

        return Vec::<u8>::from(node.bytes());
    }

    fn internal_root_bytes() -> Vec<u8>
    {
        let tree = build_multi_leaf_tree();
        let node = tree
            .nodes()
            .first()
            .expect("multi-leaf fixture should carry an internal root node");

        assert_eq!(
            inspect_proof_node(node).kind(),
            EncodedNodeKind::Internal,
            "multi-leaf fixture root should be internal"
        );

        return Vec::<u8>::from(node.bytes());
    }

    fn expect_inspection_failure(
        bytes: &[u8],
        context: &'static str,
    ) -> ProllyBaoError
    {
        return inspect_encoded_node(bytes).expect_err(context);
    }

    fn assert_malformed_node_error(
        error: &ProllyBaoError,
        context: &'static str,
    )
    {
        assert!(
            matches!(error, ProllyBaoError::MalformedNodeBytes { .. }),
            "{context}"
        );
    }

    fn first_child_record_count_offset(bytes: &[u8]) -> usize
    {
        let first_separator_len = read_be_u64(bytes, INTERNAL_CHILD_SECTION_OFFSET);
        let first_separator_len_usize = usize::try_from(first_separator_len)
            .expect("first separator length should fit usize in fixture bytes");
        let after_separator_len = INTERNAL_CHILD_SECTION_OFFSET
            .checked_add(U64_LEN)
            .expect("first child separator-length offset should not overflow");
        let after_separator_key = after_separator_len
            .checked_add(first_separator_len_usize)
            .expect("first child separator-key offset should not overflow");

        return after_separator_key
            .checked_add(NODE_HASH_LEN)
            .expect("first child record-count offset should not overflow");
    }

    fn read_be_u64(
        bytes: &[u8],
        offset: usize,
    ) -> u64
    {
        let end = offset
            .checked_add(U64_LEN)
            .expect("u64 field end offset should not overflow");
        let field = bytes
            .get(offset .. end)
            .expect("fixture bytes should contain the requested u64 field");
        let mut encoded = [0_u8; U64_LEN];
        encoded.copy_from_slice(field);

        return u64::from_be_bytes(encoded);
    }

    fn overwrite_be_u64(
        bytes: &mut [u8],
        offset: usize,
        value: u64,
    )
    {
        let end = offset
            .checked_add(U64_LEN)
            .expect("u64 field end offset should not overflow");
        let field = bytes
            .get_mut(offset .. end)
            .expect("fixture bytes should contain the requested mutable u64 field");
        field.copy_from_slice(value.to_be_bytes().as_ref());
    }
}
