#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        reason = "contract tests relax the production wall per docs/workflow/rust.md"
    )
)]

#[cfg(test)]
mod support;

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
    use gandr_storage_prolly_trees::InMemoryBlockStore;
    use gandr_storage_prolly_trees::KeyBound;
    use gandr_storage_prolly_trees::KeyRangeRef;
    use gandr_storage_prolly_trees::MembershipProof;
    use gandr_storage_prolly_trees::NODE_HASH_LEN;
    use gandr_storage_prolly_trees::NodeHash;
    use gandr_storage_prolly_trees::NonMembershipEvidence;
    use gandr_storage_prolly_trees::NonMembershipProof;
    use gandr_storage_prolly_trees::OwnedKeyRange;
    use gandr_storage_prolly_trees::PortableProofTree;
    use gandr_storage_prolly_trees::ProllyBaoError;
    use gandr_storage_prolly_trees::ProllyTree;
    use gandr_storage_prolly_trees::ProofNode;
    use gandr_storage_prolly_trees::ProofNodeCount;
    use gandr_storage_prolly_trees::RangeProof;
    use gandr_storage_prolly_trees::Record;
    use gandr_storage_prolly_trees::RecordRef;
    use gandr_storage_prolly_trees::SeparatorConvention;
    use gandr_storage_prolly_trees::SnapshotBuffer;
    use gandr_storage_prolly_trees::StoredNodeRef;
    use gandr_storage_prolly_trees::TreeKind;
    use gandr_storage_prolly_trees::TreeParams;
    use gandr_storage_prolly_trees::TreeRecordCount;
    use gandr_storage_prolly_trees::TreeRoot;
    use gandr_storage_prolly_trees::WitnessBuffer;
    use gandr_storage_prolly_trees::WitnessEndSummary;
    use gandr_storage_prolly_trees::WitnessKind;
    use gandr_storage_prolly_trees::WitnessTranscript;
    use gandr_storage_prolly_trees::hash_encoded_node;
    use gandr_storage_prolly_trees::verify_snapshot_bytes;
    use gandr_storage_prolly_trees::verify_stored_node;

    use crate::support::ByteLength;
    use crate::support::ByteOffset;
    use crate::support::FixtureBytes;
    use crate::support::FixtureBytesMut;
    use crate::support::FixtureLong;
    use crate::support::FixtureSlice;
    use crate::support::FixtureWord;
    use crate::support::OwnedFixtureBytes;
    use crate::support::ProofNodeIndex;
    use crate::support::TestContext;

    const SNAPSHOT_MAGIC: &[u8] = b"prolly-bao:snapshot:v1";
    const WITNESS_MAGIC: &[u8] = b"prolly-bao:witness:v1";
    const WITNESS_END_SUMMARY_MAGIC: &[u8] = b"prolly-bao:witness-end-summary:v1";
    const WITNESS_END_SUMMARY_MAGIC_LEN: usize = 33_usize;
    const WITNESS_END_SUMMARY_LEN: usize = WITNESS_END_SUMMARY_MAGIC_LEN
        + 2_usize
        + 1_usize
        + (6_usize * NODE_HASH_LEN)
        + (2_usize * 8_usize);
    const WITNESS_END_SUMMARY_BODY_DIGEST_OFFSET: usize = WITNESS_END_SUMMARY_MAGIC_LEN
        + 2_usize
        + 1_usize
        + NODE_HASH_LEN
        + 8_usize
        + NODE_HASH_LEN
        + NODE_HASH_LEN;

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

    fn alternate_params() -> TreeParams
    {
        let limits = ChunkLimits::new(
            1_u64.into(),
            128_u64.into(),
            4096_u64.into(),
            3_u32.into(),
            3_u32.into(),
            3_u32.into(),
        )
        .expect("alternate chunk limits should be valid");
        let chunker_params = ChunkerParams::new(
            AlgorithmVersion::FASTCDC_2020,
            GearTableVersion::MACH_V1,
            SeedPolicy::NONE,
            NormalizationPolicy::NONE,
            RecordBoundaryRule::BETWEEN_RECORDS,
            limits,
        )
        .expect("alternate chunker params should be valid");

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

    #[test]
    fn deterministic_root_identity_and_equivalent_rebuilds()
    {
        let first_records = records();
        let second_records = records();
        let first = PortableProofTree::build(first_records.as_slice(), compact_params())
            .expect("first sorted fixture build should succeed");
        let second = PortableProofTree::build(second_records.as_slice(), compact_params())
            .expect("equivalent sorted fixture rebuild should succeed");

        assert_eq!(
            first.root(),
            second.root(),
            "equivalent record bytes and parameters must produce the same root"
        );
        assert_eq!(
            first.leaf_hashes(),
            second.leaf_hashes(),
            "equivalent rebuilds must produce the same leaf identities"
        );
    }

    #[test]
    fn sorted_input_and_duplicate_keys_fail_closed()
    {
        let unsorted = vec![
            RecordRef::new(b"b", b"bravo"),
            RecordRef::new(b"a", b"alpha"),
        ];
        let duplicate = vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"a", b"again"),
        ];

        let unsorted_error = PortableProofTree::build(unsorted.as_slice(), compact_params())
            .expect_err("unsorted input must be rejected");
        assert!(
            matches!(unsorted_error, ProllyBaoError::UnsortedInput { .. }),
            "unsorted input should report the sorted-order invariant"
        );

        let duplicate_error = PortableProofTree::build(duplicate.as_slice(), compact_params())
            .expect_err("duplicate keys must be rejected");
        assert!(
            matches!(duplicate_error, ProllyBaoError::DuplicateKeys { .. }),
            "duplicate keys should report the uniqueness invariant"
        );
    }

    #[test]
    fn lookup_success_failure_and_range_boundaries()
    {
        let tree = build_tree();

        assert_eq!(
            tree.lookup(b"c".as_slice().into()),
            Some(b"charlie".as_slice().into()),
            "lookup should return the value bound to an existing key"
        );
        assert_eq!(
            tree.lookup(b"g".as_slice().into()),
            None,
            "lookup should return no value for an absent key"
        );

        let half_open = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::excluded(b"e"))
            .expect("half-open range should be valid");
        let half_open_records = tree
            .range(half_open)
            .expect("valid half-open range should scan");
        assert_eq!(
            keys(half_open_records.as_ref()),
            vec![
                b"b".as_slice().into(),
                b"c".as_slice().into(),
                b"d".as_slice().into(),
            ],
            "inclusive start and exclusive end should select the expected records"
        );

        let inclusive_end = KeyRangeRef::new(KeyBound::excluded(b"b"), KeyBound::included(b"e"))
            .expect("exclusive/inclusive range should be valid");
        let inclusive_end_records = tree
            .range(inclusive_end)
            .expect("valid exclusive/inclusive range should scan");
        assert_eq!(
            keys(inclusive_end_records.as_ref()),
            vec![
                b"c".as_slice().into(),
                b"d".as_slice().into(),
                b"e".as_slice().into(),
            ],
            "exclusive start and inclusive end should select the expected records"
        );
    }

    #[test]
    fn malformed_node_bytes_are_rejected()
    {
        let malformed = b"not-a-prolly-bao-node";
        let hash = hash_encoded_node((malformed).into());
        let error = verify_stored_node(StoredNodeRef::new(hash, (malformed).into()))
            .expect_err("malformed node bytes with a matching hash must still fail decode");

        assert!(
            matches!(error, ProllyBaoError::MalformedNodeBytes { .. }),
            "malformed node bytes should fail by structure, not by store trust"
        );
    }

    #[test]
    fn membership_non_membership_and_range_proofs_verify()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let params = root.params().clone();

        let membership = tree
            .prove_membership((b"d").into())
            .expect("existing key should produce a membership proof");
        membership
            .verify(&root, &params, (b"d").into(), (b"delta").into())
            .expect("membership proof should verify key/value binding");

        let non_membership = tree
            .prove_non_membership((b"dd").into())
            .expect("absent key should produce a non-membership proof");
        let evidence = non_membership
            .verify(&root, &params, (b"dd").into())
            .expect("non-membership proof should verify adjacent bounds");
        assert_eq!(
            evidence
                .predecessor()
                .expect("absent middle key should have a predecessor")
                .key(),
            b"d".as_slice().into(),
            "predecessor should authenticate the lower adjacent key"
        );
        assert_eq!(
            evidence
                .successor()
                .expect("absent middle key should have a successor")
                .key(),
            b"e".as_slice().into(),
            "successor should authenticate the upper adjacent key"
        );

        let range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"d"))
            .expect("inclusive range should be valid");
        let range_proof = tree
            .prove_range(range)
            .expect("valid range should produce a range proof");
        let verified_records = range_proof
            .verify_for_range(&root, &params, range)
            .expect("range proof should verify complete interval records");
        assert_eq!(
            keys(verified_records.as_ref()),
            vec![
                b"b".as_slice().into(),
                b"c".as_slice().into(),
                b"d".as_slice().into(),
            ],
            "range proof should return exactly the authenticated interval"
        );
    }

    #[test]
    fn membership_proofs_and_witnesses_use_compact_nodes()
    {
        let tree = build_tree();
        let compact_membership = tree
            .prove_membership((b"d").into())
            .expect("existing key should produce a compact membership proof");
        let compact_node_count = compact_membership.nodes().len();
        assert_eq!(
            compact_node_count, 2_usize,
            "membership proof should carry root plus the selected leaf",
        );
        compact_membership
            .verify(
                tree.root(),
                tree.root().params(),
                (b"d").into(),
                (b"delta").into(),
            )
            .expect("compact membership proof should verify key/value binding");

        let compact_witness = WitnessTranscript::from_membership_proof(compact_membership)
            .expect("compact membership proof should form a witness transcript");
        assert_eq!(
            compact_witness.nodes().len(),
            compact_node_count,
            "membership witness should carry the same compact node set",
        );
        let decoded_compact_witness = round_trip_witness(&compact_witness);
        decoded_compact_witness
            .verify_membership(
                tree.root(),
                tree.root().params(),
                (b"d").into(),
                (b"delta").into(),
            )
            .expect("compact membership witness should verify key/value binding");
    }

    #[test]
    fn compact_membership_rejects_missing_and_misordered_nodes()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let params = root.params().clone();
        let proof = tree
            .prove_membership((b"d").into())
            .expect("existing key should produce a compact proof");

        let omitted_selected = MembershipProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.key(),
            proof.value(),
            vec![proof_node(proof.nodes(), 0)].into_boxed_slice(),
        );
        let omitted_selected_error = omitted_selected
            .verify(&root, &params, (b"d").into(), (b"delta").into())
            .expect_err("compact proof must reject an omitted selected leaf");
        assert_invalid_proof_shape(
            &omitted_selected_error,
            ("compact membership proof should reject missing selected child material").into(),
        );

        let root_not_first = MembershipProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.key(),
            proof.value(),
            vec![proof_node(proof.nodes(), 1), proof_node(proof.nodes(), 0)].into_boxed_slice(),
        );
        let root_not_first_witness = WitnessTranscript::from_membership_proof(root_not_first)
            .expect("misordered compact proof should still encode as witness material");
        let decoded_root_not_first = round_trip_witness(&root_not_first_witness);
        let root_not_first_error = decoded_root_not_first
            .verify_membership(&root, &params, (b"d").into(), (b"delta").into())
            .expect_err("compact witness must reject non-root-first node order");
        assert_invalid_proof_shape(
            &root_not_first_error,
            ("compact membership witness should reject non-root-first node order").into(),
        );
    }

    #[test]
    fn non_membership_proofs_and_witnesses_use_compact_nodes()
    {
        let tree = build_tree();
        let compact_absent = tree
            .prove_non_membership((b"dd").into())
            .expect("absent middle key should produce a compact non-membership proof");
        assert_eq!(
            compact_absent.nodes().len(),
            3_usize,
            "middle-key non-membership proof should carry root, selected leaf, and successor leaf",
        );

        let compact_witness = WitnessTranscript::from_non_membership_proof(compact_absent)
            .expect("compact non-membership proof should form a witness transcript");
        assert_eq!(
            compact_witness.nodes().len(),
            3_usize,
            "middle-key non-membership witness should carry the compact node set",
        );

        let trailing_absent = tree
            .prove_non_membership((b"zz").into())
            .expect("absent trailing key should produce a compact non-membership proof");
        assert_eq!(
            trailing_absent.nodes().len(),
            2_usize,
            "trailing non-membership proof should carry only root and selected leaf",
        );
        trailing_absent
            .verify(tree.root(), tree.root().params(), (b"zz").into())
            .expect("trailing compact non-membership proof should verify");

        let trailing_witness = WitnessTranscript::from_non_membership_proof(trailing_absent)
            .expect("trailing compact non-membership proof should form a witness transcript");
        assert_eq!(
            trailing_witness.nodes().len(),
            2_usize,
            "trailing non-membership witness should carry only root and selected leaf",
        );
        let decoded_trailing_witness = round_trip_witness(&trailing_witness);
        decoded_trailing_witness
            .verify_non_membership(tree.root(), tree.root().params(), (b"zz").into())
            .expect("trailing compact non-membership witness should verify");
    }

    #[test]
    fn range_proofs_and_witnesses_use_compact_nodes()
    {
        let tree = build_tree();
        let broad_range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"d"))
            .expect("inclusive range should be valid");
        let broad_proof = tree
            .prove_range(broad_range)
            .expect("broad range should produce a compact range proof");
        assert_eq!(
            broad_proof.nodes().len(),
            3_usize,
            "broad range proof should carry root plus the contiguous selected leaves",
        );
        let broad_witness = WitnessTranscript::from_range_proof(broad_proof)
            .expect("broad compact range proof should form a witness transcript");
        assert_eq!(
            broad_witness.nodes().len(),
            3_usize,
            "broad range witness should carry the compact node set",
        );

        let narrow_range = KeyRangeRef::new(KeyBound::included(b"c"), KeyBound::included(b"d"))
            .expect("narrow inclusive range should be valid");
        let narrow_proof = tree
            .prove_range(narrow_range)
            .expect("narrow range should produce a compact range proof");
        assert_eq!(
            narrow_proof.nodes().len(),
            2_usize,
            "narrow range proof should carry only root and the selected leaf",
        );
        let narrow_witness = WitnessTranscript::from_range_proof(narrow_proof)
            .expect("narrow compact range proof should form a witness transcript");
        assert_eq!(
            narrow_witness.nodes().len(),
            2_usize,
            "narrow range witness should carry only root and the selected leaf",
        );
    }

    #[test]
    fn compact_non_membership_rejects_missing_extra_and_misordered_nodes()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let params = root.params().clone();
        let proof = tree
            .prove_non_membership((b"dd").into())
            .expect("middle absent key should produce a compact proof");

        let omitted_selected = NonMembershipProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.key(),
            proof.evidence().clone(),
            vec![proof_node(proof.nodes(), 0), proof_node(proof.nodes(), 2)],
        );
        let omitted_selected_error = omitted_selected
            .verify(&root, &params, (b"dd").into())
            .expect_err("compact proof must reject an omitted selected leaf");
        assert_invalid_proof_shape(
            &omitted_selected_error,
            ("compact proof should reject missing selected child material").into(),
        );

        let omitted_successor = NonMembershipProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.key(),
            proof.evidence().clone(),
            vec![proof_node(proof.nodes(), 0), proof_node(proof.nodes(), 1)],
        );
        let omitted_successor_error = omitted_successor
            .verify(&root, &params, (b"dd").into())
            .expect_err("compact proof must reject an omitted required successor leaf");
        assert_invalid_proof_shape(
            &omitted_successor_error,
            ("compact proof should reject missing successor material").into(),
        );

        let trailing = tree
            .prove_non_membership((b"zz").into())
            .expect("trailing absent key should produce a compact proof");
        let unnecessary_successor = NonMembershipProof::new(
            trailing.envelope().clone(),
            trailing.root_node_hash(),
            trailing.key(),
            trailing.evidence().clone(),
            vec![
                proof_node(trailing.nodes(), 0),
                proof_node(trailing.nodes(), 1),
                proof_node(proof.nodes(), 1),
            ],
        );
        let unnecessary_successor_error = unnecessary_successor
            .verify(&root, &params, (b"zz").into())
            .expect_err("compact proof must reject an extra successor leaf");
        assert_invalid_proof_shape(
            &unnecessary_successor_error,
            ("compact proof should reject unnecessary successor material").into(),
        );

        let misordered_children = NonMembershipProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.key(),
            proof.evidence().clone(),
            vec![
                proof_node(proof.nodes(), 0),
                proof_node(proof.nodes(), 2),
                proof_node(proof.nodes(), 1),
            ],
        );
        let misordered_children_error = misordered_children
            .verify(&root, &params, (b"dd").into())
            .expect_err("compact proof must reject child nodes out of separator order");
        assert_invalid_proof_shape(
            &misordered_children_error,
            ("compact proof should reject swapped compact children").into(),
        );

        let root_not_first = NonMembershipProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.key(),
            proof.evidence().clone(),
            vec![
                proof_node(proof.nodes(), 1),
                proof_node(proof.nodes(), 0),
                proof_node(proof.nodes(), 2),
            ],
        );
        let root_not_first_witness = WitnessTranscript::from_non_membership_proof(root_not_first)
            .expect("misordered compact proof should still encode as witness material");
        let decoded_root_not_first = round_trip_witness(&root_not_first_witness);
        let root_not_first_error = decoded_root_not_first
            .verify_non_membership(&root, &params, (b"dd").into())
            .expect_err("compact witness must reject non-root-first node order");
        assert_invalid_proof_shape(
            &root_not_first_error,
            ("compact witness should reject non-root-first node order").into(),
        );
    }

    #[test]
    fn compact_range_rejects_node_bounds_and_record_shape_errors()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let params = root.params().clone();
        let range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"d"))
            .expect("inclusive range should be valid");
        let proof = tree
            .prove_range(range)
            .expect("inclusive range should produce a compact proof");

        let omitted_child = RangeProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.range().clone(),
            proof.records().to_vec(),
            vec![proof_node(proof.nodes(), 0), proof_node(proof.nodes(), 1)],
        );
        let omitted_child_error = omitted_child
            .verify_for_range(&root, &params, range)
            .expect_err("compact range proof must reject omitted selected leaves");
        assert_invalid_proof_shape(
            &omitted_child_error,
            ("compact range proof should reject missing child material").into(),
        );

        let misordered_children = RangeProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.range().clone(),
            proof.records().to_vec(),
            vec![
                proof_node(proof.nodes(), 0),
                proof_node(proof.nodes(), 2),
                proof_node(proof.nodes(), 1),
            ],
        );
        let misordered_children_error = misordered_children
            .verify_for_range(&root, &params, range)
            .expect_err("compact range proof must reject child nodes out of separator order");
        assert_invalid_proof_shape(
            &misordered_children_error,
            ("compact range proof should reject swapped compact children").into(),
        );

        let wider_range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"e"))
            .expect("wider range should be valid");
        let wrong_bounds = RangeProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            OwnedKeyRange::from_ref(wider_range),
            proof.records().to_vec(),
            proof.nodes().to_vec(),
        );
        let wrong_bounds_error = wrong_bounds
            .verify_for_range(&root, &params, wider_range)
            .expect_err("compact range proof must reject wrong bounds");
        assert_invalid_proof_shape(
            &wrong_bounds_error,
            ("compact range proof should reject nodes that do not cover verifier bounds").into(),
        );

        let incomplete_records = vec![
            Record::new(b"b".as_slice(), b"bravo".as_slice()),
            Record::new(b"d".as_slice(), b"delta".as_slice()),
        ];
        let incomplete = RangeProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.range().clone(),
            incomplete_records,
            proof.nodes().to_vec(),
        );
        let incomplete_error = incomplete
            .verify_for_range(&root, &params, range)
            .expect_err("compact range proof must reject incomplete returned records");
        assert_invalid_proof_shape(
            &incomplete_error,
            ("compact range proof should reject incomplete returned records").into(),
        );

        let unsorted_records = vec![
            Record::new(b"c".as_slice(), b"charlie".as_slice()),
            Record::new(b"b".as_slice(), b"bravo".as_slice()),
            Record::new(b"d".as_slice(), b"delta".as_slice()),
        ];
        let unsorted = RangeProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.range().clone(),
            unsorted_records,
            proof.nodes().to_vec(),
        );
        let unsorted_error = unsorted
            .verify_for_range(&root, &params, range)
            .expect_err("compact range proof must reject unsorted returned records");
        assert_invalid_proof_shape(
            &unsorted_error,
            ("compact range proof should reject unsorted returned records").into(),
        );
    }

    #[test]
    fn witness_transcripts_encode_decode_and_verify()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let params = root.params().clone();

        assert_membership_witness_round_trip(&tree, &root, &params);
        assert_non_membership_witness_round_trip(&tree, &root, &params);
        assert_range_witness_round_trip(&tree, &root, &params);
    }

    fn assert_membership_witness_round_trip(
        tree: &PortableProofTree,
        root: &TreeRoot,
        params: &TreeParams,
    )
    {
        let membership = tree
            .prove_membership((b"d").into())
            .expect("existing key should produce a membership proof");
        let membership_root_node_hash = membership.root_node_hash();
        let membership_proof_node_count = proof_node_count((membership.nodes()).into());
        let membership_witness = WitnessTranscript::from_membership_proof(membership)
            .expect("membership proof should form a witness transcript");
        let membership_summary = membership_witness.end_summary();
        let membership_chunker_parameter_digest = membership_summary.chunker_parameter_digest();
        let membership_body_digest = membership_summary.body_digest();
        let membership_proof_nodes_digest = membership_summary.proof_nodes_digest();
        let membership_binding_digest = membership_summary.binding_digest();
        let decoded_membership = round_trip_witness(&membership_witness);
        decoded_membership
            .verify_membership(root, params, (b"d").into(), (b"delta").into())
            .expect("membership witness should verify key/value binding");
        assert_witness_summary(decoded_membership.end_summary(), &ExpectedWitnessSummary {
            kind: WitnessKind::Membership,
            root_hash: root.hash(),
            root_record_count: root.record_count(),
            chunker_parameter_digest: membership_chunker_parameter_digest,
            root_node_hash: membership_root_node_hash,
            body_digest: membership_body_digest,
            proof_node_count: membership_proof_node_count,
            proof_nodes_digest: membership_proof_nodes_digest,
            binding_digest: membership_binding_digest,
        });
    }

    fn assert_non_membership_witness_round_trip(
        tree: &PortableProofTree,
        root: &TreeRoot,
        params: &TreeParams,
    )
    {
        let non_membership = tree
            .prove_non_membership((b"dd").into())
            .expect("absent key should produce a non-membership proof");
        let non_membership_root_node_hash = non_membership.root_node_hash();
        let non_membership_proof_node_count = proof_node_count((non_membership.nodes()).into());
        let non_membership_witness = WitnessTranscript::from_non_membership_proof(non_membership)
            .expect("non-membership proof should form a witness transcript");
        let non_membership_summary = non_membership_witness.end_summary();
        let non_membership_chunker_parameter_digest =
            non_membership_summary.chunker_parameter_digest();
        let non_membership_body_digest = non_membership_summary.body_digest();
        let non_membership_proof_nodes_digest = non_membership_summary.proof_nodes_digest();
        let non_membership_binding_digest = non_membership_summary.binding_digest();
        let decoded_non_membership = round_trip_witness(&non_membership_witness);
        let evidence = decoded_non_membership
            .verify_non_membership(root, params, (b"dd").into())
            .expect("non-membership witness should verify adjacent bounds");
        assert_eq!(
            evidence
                .predecessor()
                .expect("absent middle key should have a predecessor")
                .key(),
            b"d".as_slice().into(),
            "witness predecessor should authenticate the lower adjacent key"
        );
        assert_eq!(
            evidence
                .successor()
                .expect("absent middle key should have a successor")
                .key(),
            b"e".as_slice().into(),
            "witness successor should authenticate the upper adjacent key"
        );
        assert_witness_summary(
            decoded_non_membership.end_summary(),
            &ExpectedWitnessSummary {
                kind: WitnessKind::NonMembership,
                root_hash: root.hash(),
                root_record_count: root.record_count(),
                chunker_parameter_digest: non_membership_chunker_parameter_digest,
                root_node_hash: non_membership_root_node_hash,
                body_digest: non_membership_body_digest,
                proof_node_count: non_membership_proof_node_count,
                proof_nodes_digest: non_membership_proof_nodes_digest,
                binding_digest: non_membership_binding_digest,
            },
        );
    }

    fn assert_range_witness_round_trip(
        tree: &PortableProofTree,
        root: &TreeRoot,
        params: &TreeParams,
    )
    {
        let range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"d"))
            .expect("inclusive range should be valid");
        let range_proof = tree
            .prove_range(range)
            .expect("valid range should produce a range proof");
        let range_root_node_hash = range_proof.root_node_hash();
        let range_proof_node_count = proof_node_count((range_proof.nodes()).into());
        let range_witness = WitnessTranscript::from_range_proof(range_proof)
            .expect("range proof should form a witness transcript");
        let range_summary = range_witness.end_summary();
        let range_chunker_parameter_digest = range_summary.chunker_parameter_digest();
        let range_body_digest = range_summary.body_digest();
        let range_proof_nodes_digest = range_summary.proof_nodes_digest();
        let range_binding_digest = range_summary.binding_digest();
        let decoded_range = round_trip_witness(&range_witness);
        let verified_records = decoded_range
            .verify_range(root, params, range)
            .expect("range witness should verify complete interval records");
        assert_eq!(
            keys(verified_records.as_ref()),
            vec![
                b"b".as_slice().into(),
                b"c".as_slice().into(),
                b"d".as_slice().into(),
            ],
            "range witness should return exactly the authenticated interval"
        );
        assert_witness_summary(decoded_range.end_summary(), &ExpectedWitnessSummary {
            kind: WitnessKind::Range,
            root_hash: root.hash(),
            root_record_count: root.record_count(),
            chunker_parameter_digest: range_chunker_parameter_digest,
            root_node_hash: range_root_node_hash,
            body_digest: range_body_digest,
            proof_node_count: range_proof_node_count,
            proof_nodes_digest: range_proof_nodes_digest,
            binding_digest: range_binding_digest,
        });
    }

    #[test]
    fn witness_transcripts_bind_expected_root_and_query()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let params = root.params().clone();
        let membership = tree
            .prove_membership((b"c").into())
            .expect("existing key should produce a membership proof");
        let membership_witness = WitnessTranscript::from_membership_proof(membership)
            .expect("membership proof should form a witness transcript");
        let decoded_membership = round_trip_witness(&membership_witness);

        let wrong_root_records = vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"b", b"bravo"),
            RecordRef::new(b"c", b"changed"),
            RecordRef::new(b"d", b"delta"),
            RecordRef::new(b"e", b"echo"),
            RecordRef::new(b"f", b"foxtrot"),
        ];
        let wrong_root_tree =
            PortableProofTree::build(wrong_root_records.as_slice(), compact_params())
                .expect("wrong-root fixture should build");
        let wrong_root_error = decoded_membership
            .verify_membership(
                wrong_root_tree.root(),
                &params,
                (b"c").into(),
                (b"charlie").into(),
            )
            .expect_err("witness must not verify against a different expected root");
        assert!(
            matches!(wrong_root_error, ProllyBaoError::InvalidProofShape { .. }),
            "witness verification should reject the wrong expected root"
        );

        let wrong_key_error = decoded_membership
            .verify_membership(&root, &params, (b"d").into(), (b"charlie").into())
            .expect_err("witness must not verify for a different queried key");
        assert!(
            matches!(wrong_key_error, ProllyBaoError::InvalidProofShape { .. }),
            "membership witness should bind the verifier's queried key"
        );

        let wrong_value_error = decoded_membership
            .verify_membership(&root, &params, (b"c").into(), (b"delta").into())
            .expect_err("witness must not verify for a different expected value");
        assert!(
            matches!(wrong_value_error, ProllyBaoError::InvalidProofShape { .. }),
            "membership witness should bind the value to the queried key slot"
        );

        let non_membership = tree
            .prove_non_membership((b"dd").into())
            .expect("absent key should produce a non-membership proof");
        let non_membership_witness = WitnessTranscript::from_non_membership_proof(non_membership)
            .expect("non-membership proof should form a witness transcript");
        let decoded_non_membership = round_trip_witness(&non_membership_witness);
        let wrong_absent_key_error = decoded_non_membership
            .verify_non_membership(&root, &params, (b"de").into())
            .expect_err("non-membership witness must bind the absent query key");
        assert!(
            matches!(
                wrong_absent_key_error,
                ProllyBaoError::InvalidProofShape { .. }
            ),
            "non-membership witness should reject a different absent key"
        );

        let range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"d"))
            .expect("inclusive range should be valid");
        let wrong_range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"e"))
            .expect("wider range should be valid");
        let range_proof = tree
            .prove_range(range)
            .expect("valid range should produce a range proof");
        let range_witness = WitnessTranscript::from_range_proof(range_proof)
            .expect("range proof should form a witness transcript");
        let decoded_range = round_trip_witness(&range_witness);
        let wrong_range_error = decoded_range
            .verify_range(&root, &params, wrong_range)
            .expect_err("range witness must bind the queried range");
        assert!(
            matches!(wrong_range_error, ProllyBaoError::InvalidProofShape { .. }),
            "range witness should reject a different verifier range"
        );
    }

    #[test]
    fn witness_transcripts_reject_tampered_nodes_evidence_and_bounds()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let params = root.params().clone();
        let membership = tree
            .prove_membership((b"c").into())
            .expect("existing key should produce a membership proof");

        let tampered_membership = tamper_first_node(&membership);
        let tampered_witness = WitnessTranscript::from_membership_proof(tampered_membership)
            .expect("tampered membership proof should form a witness transcript");
        let decoded_tampered = round_trip_witness(&tampered_witness);
        let tampered_error = decoded_tampered
            .verify_membership(&root, &params, (b"c").into(), (b"charlie").into())
            .expect_err("tampered transcript node bytes must fail verification");
        assert!(
            matches!(tampered_error, ProllyBaoError::HashMismatch { .. }),
            "tampered witness node bytes should be rejected by proof verification"
        );

        let non_membership = tree
            .prove_non_membership((b"dd").into())
            .expect("absent key should produce a non-membership proof");
        let malformed_evidence = NonMembershipEvidence::new(
            Some(Record::new(b"c".as_slice(), b"charlie".as_slice())),
            Some(Record::new(b"e".as_slice(), b"echo".as_slice())),
        );
        let malformed_non_membership = NonMembershipProof::new(
            non_membership.envelope().clone(),
            tree.root_node_hash(),
            non_membership.key(),
            malformed_evidence,
            non_membership.nodes().to_vec(),
        );
        let malformed_evidence_witness =
            WitnessTranscript::from_non_membership_proof(malformed_non_membership)
                .expect("malformed evidence proof should form a witness transcript");
        let decoded_malformed_evidence = round_trip_witness(&malformed_evidence_witness);
        let evidence_error = decoded_malformed_evidence
            .verify_non_membership(&root, &params, (b"dd").into())
            .expect_err("malformed adjacent evidence must fail verification");
        assert!(
            matches!(evidence_error, ProllyBaoError::InvalidProofShape { .. }),
            "non-membership witness should reject malformed adjacent evidence"
        );

        let range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"d"))
            .expect("inclusive range should be valid");
        let wider_range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"e"))
            .expect("wider range should be valid");
        let range_proof = tree
            .prove_range(range)
            .expect("valid range should produce a range proof");
        let malformed_bounds = RangeProof::new(
            range_proof.envelope().clone(),
            tree.root_node_hash(),
            OwnedKeyRange::from_ref(wider_range),
            range_proof.records().to_vec(),
            range_proof.nodes().to_vec(),
        );
        let malformed_bounds_witness = WitnessTranscript::from_range_proof(malformed_bounds)
            .expect("malformed bounds proof should form a witness transcript");
        let decoded_malformed_bounds = round_trip_witness(&malformed_bounds_witness);
        let bounds_error = decoded_malformed_bounds
            .verify_range(&root, &params, wider_range)
            .expect_err("range witness with incomplete records for its bounds must fail");
        assert!(
            matches!(bounds_error, ProllyBaoError::InvalidProofShape { .. }),
            "range witness should reject bounds whose returned records are incomplete"
        );
    }

    #[test]
    fn witness_transcripts_reject_duplicate_and_reordered_range_records()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let params = root.params().clone();
        let range = KeyRangeRef::new(KeyBound::included(b"b"), KeyBound::included(b"d"))
            .expect("inclusive range should be valid");
        let range_proof = tree
            .prove_range(range)
            .expect("valid range should produce a range proof");

        let duplicate_records = vec![
            Record::new(b"b".as_slice(), b"bravo".as_slice()),
            Record::new(b"c".as_slice(), b"charlie".as_slice()),
            Record::new(b"c".as_slice(), b"charlie".as_slice()),
            Record::new(b"d".as_slice(), b"delta".as_slice()),
        ];
        let duplicate_proof = RangeProof::new(
            range_proof.envelope().clone(),
            tree.root_node_hash(),
            range_proof.range().clone(),
            duplicate_records,
            range_proof.nodes().to_vec(),
        );
        let duplicate_witness = WitnessTranscript::from_range_proof(duplicate_proof)
            .expect("duplicate record proof should form a witness transcript");
        let decoded_duplicate = round_trip_witness(&duplicate_witness);
        let duplicate_error = decoded_duplicate
            .verify_range(&root, &params, range)
            .expect_err("duplicate transcript records must fail verification");
        assert!(
            matches!(duplicate_error, ProllyBaoError::InvalidProofShape { .. }),
            "range witness should reject duplicate returned records"
        );

        let reordered_records = vec![
            Record::new(b"b".as_slice(), b"bravo".as_slice()),
            Record::new(b"d".as_slice(), b"delta".as_slice()),
            Record::new(b"c".as_slice(), b"charlie".as_slice()),
        ];
        let reordered_proof = RangeProof::new(
            range_proof.envelope().clone(),
            tree.root_node_hash(),
            range_proof.range().clone(),
            reordered_records,
            range_proof.nodes().to_vec(),
        );
        let reordered_witness = WitnessTranscript::from_range_proof(reordered_proof)
            .expect("reordered record proof should form a witness transcript");
        let decoded_reordered = round_trip_witness(&reordered_witness);
        let reordered_error = decoded_reordered
            .verify_range(&root, &params, range)
            .expect_err("reordered transcript records must fail verification");
        assert!(
            matches!(reordered_error, ProllyBaoError::InvalidProofShape { .. }),
            "range witness should reject reordered returned records"
        );
    }

    #[test]
    fn witness_decode_rejects_unsupported_version_and_truncation()
    {
        let tree = build_tree();
        let membership = tree
            .prove_membership((b"c").into())
            .expect("existing key should produce a membership proof");
        let membership_witness = WitnessTranscript::from_membership_proof(membership)
            .expect("membership proof should form a witness transcript");
        let valid_bytes = membership_witness
            .to_bytes()
            .expect("membership witness should encode");

        let mut unsupported_version = Vec::<u8>::from(valid_bytes.as_ref());
        let version_offset = WITNESS_MAGIC.len();
        assert_eq!(
            unsupported_version
                .get(.. version_offset)
                .expect("witness bytes should include magic"),
            WITNESS_MAGIC,
            "witness bytes should start with the transcript magic"
        );
        assert_eq!(
            unsupported_version
                .get(version_offset .. version_offset + 2_usize)
                .expect("witness bytes should include a version field"),
            &[0_u8, 1_u8],
            "current witness version should be encoded immediately after magic"
        );
        unsupported_version
            .get_mut(version_offset .. version_offset + 2_usize)
            .expect("witness bytes should include a mutable version field")
            .copy_from_slice(2_u16.to_be_bytes().as_ref());
        let unsupported_error = expect_witness_decode_failure(
            unsupported_version.as_slice().into(),
            "unsupported witness version must fail decode".into(),
        );
        assert!(
            matches!(
                unsupported_error,
                ProllyBaoError::UnsupportedWitnessVersion { version: 2_u16 }
            ),
            "unsupported witness version should return the dedicated fail-closed error"
        );

        for truncated_len in 0_usize .. valid_bytes.as_ref().len() {
            let truncated_prefix = valid_bytes
                .as_ref()
                .get(.. truncated_len)
                .expect("truncated witness prefix should be in bounds");
            assert!(
                WitnessTranscript::decode(truncated_prefix.into()).is_err(),
                "truncated witness prefix of length {truncated_len} must fail decode"
            );
        }
    }

    #[test]
    fn witness_decode_rejects_missing_truncated_tampered_and_mismatched_end_summary()
    {
        let tree = build_tree();
        let membership = tree
            .prove_membership((b"c").into())
            .expect("existing key should produce a membership proof");
        let membership_witness = WitnessTranscript::from_membership_proof(membership)
            .expect("membership proof should form a witness transcript");
        let valid_bytes = membership_witness
            .to_bytes()
            .expect("membership witness should encode");
        let summary_start = valid_bytes
            .as_ref()
            .len()
            .checked_sub(WITNESS_END_SUMMARY_LEN)
            .expect("witness bytes should include an end summary");

        assert_eq!(
            valid_bytes
                .as_ref()
                .get(summary_start .. summary_start + WITNESS_END_SUMMARY_MAGIC.len())
                .expect("witness end summary should include magic"),
            WITNESS_END_SUMMARY_MAGIC,
            "witness bytes should carry the terminal end-summary magic"
        );

        let missing_end_summary_bytes = witness_prefix(
            (valid_bytes.as_ref()).into(),
            (summary_start).into(),
            ("witness bytes without the end summary should be in bounds").into(),
        );
        let missing_error = expect_witness_decode_failure(
            missing_end_summary_bytes,
            ("witness bytes without the end summary must fail decode").into(),
        );
        assert_malformed_witness_error(
            &missing_error,
            ("missing witness end summary should report malformed witness bytes").into(),
        );

        let truncated_end_summary_len = valid_bytes
            .as_ref()
            .len()
            .checked_sub(1_usize)
            .expect("witness bytes should be non-empty");
        let truncated_end_summary_bytes = witness_prefix(
            (valid_bytes.as_ref()).into(),
            (truncated_end_summary_len).into(),
            ("witness bytes with a truncated end summary should be in bounds").into(),
        );
        let truncated_error = expect_witness_decode_failure(
            truncated_end_summary_bytes,
            ("witness bytes with a truncated end summary must fail decode").into(),
        );
        assert_malformed_witness_error(
            &truncated_error,
            ("truncated witness end summary should report malformed witness bytes").into(),
        );

        let mut tampered = Vec::<u8>::from(valid_bytes.as_ref());
        let body_digest_offset = summary_start + WITNESS_END_SUMMARY_BODY_DIGEST_OFFSET;
        let body_digest_byte = tampered
            .get_mut(body_digest_offset)
            .expect("witness end summary should include a body digest");
        *body_digest_byte ^= 0x01_u8;
        let tampered_error = expect_witness_decode_failure(
            tampered.as_slice().into(),
            "witness bytes with a tampered end-summary body digest must fail decode".into(),
        );
        assert_malformed_witness_error(
            &tampered_error,
            ("tampered witness end summary should report malformed witness bytes").into(),
        );

        assert_witness_decode_rejects_mismatched_end_summary(
            &tree,
            OwnedFixtureBytes::from(Box::<[u8]>::from(valid_bytes)),
            summary_start.into(),
        );
    }

    #[test]
    fn tampered_proofs_and_wrong_key_value_bindings_are_rejected()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let params = root.params().clone();
        let membership = tree
            .prove_membership((b"c").into())
            .expect("existing key should produce a membership proof");

        let tampered = tamper_first_node(&membership);
        let tampered_error = tampered
            .verify(&root, &params, (b"c").into(), (b"charlie").into())
            .expect_err("tampered node bytes must fail proof verification");
        assert!(
            matches!(tampered_error, ProllyBaoError::HashMismatch { .. }),
            "tampered proof bytes should be rejected by recomputed BLAKE3 identity"
        );

        let wrong_root_records = vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"b", b"bravo"),
            RecordRef::new(b"c", b"changed"),
            RecordRef::new(b"d", b"delta"),
            RecordRef::new(b"e", b"echo"),
            RecordRef::new(b"f", b"foxtrot"),
        ];
        let wrong_root_tree =
            PortableProofTree::build(wrong_root_records.as_slice(), compact_params())
                .expect("wrong-root fixture should build");
        let wrong_root_error = membership
            .verify(
                wrong_root_tree.root(),
                wrong_root_tree.root().params(),
                (b"c").into(),
                (b"charlie").into(),
            )
            .expect_err("proof must not verify against a different root context");
        assert!(
            matches!(wrong_root_error, ProllyBaoError::InvalidProofShape { .. }),
            "proof verification should reject the wrong root context"
        );

        let wrong_key_error = membership
            .verify(&root, &params, (b"d").into(), (b"charlie").into())
            .expect_err("proof must not verify for a different queried key");
        assert!(
            matches!(wrong_key_error, ProllyBaoError::InvalidProofShape { .. }),
            "membership proof should bind the verifier's queried key"
        );

        let wrong_value_error = membership
            .verify(&root, &params, (b"c").into(), (b"delta").into())
            .expect_err("proof must not verify for a different expected value");
        assert!(
            matches!(wrong_value_error, ProllyBaoError::InvalidProofShape { .. }),
            "membership proof should bind the value to the queried key slot"
        );
    }

    #[test]
    fn chunker_parameter_mismatch_is_rejected()
    {
        let tree = build_tree();
        let root = tree.root().clone();
        let mismatched_params = alternate_params();
        let membership = tree
            .prove_membership((b"a").into())
            .expect("existing key should produce a membership proof");

        let error = membership
            .verify(&root, &mismatched_params, (b"a").into(), (b"alpha").into())
            .expect_err("proof verification must reject chunker parameter mismatches");

        assert!(
            matches!(error, ProllyBaoError::IncompatibleTreeParameters { .. }),
            "chunker parameter mismatch should be reported as incompatible tree context"
        );
    }

    #[test]
    fn local_change_preserves_unaffected_leaf_hashes()
    {
        let base_records = records();
        let changed_records = vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"b", b"bravo"),
            RecordRef::new(b"c", b"CHANGED"),
            RecordRef::new(b"d", b"delta"),
            RecordRef::new(b"e", b"echo"),
            RecordRef::new(b"f", b"foxtrot"),
        ];
        let base = PortableProofTree::build(base_records.as_slice(), compact_params())
            .expect("base records should build");
        let changed = PortableProofTree::build(changed_records.as_slice(), compact_params())
            .expect("locally changed records should build");

        assert_eq!(
            base.leaf_hashes().len(),
            3_usize,
            "compact fixture should force three deterministic leaves"
        );
        assert_eq!(
            changed.leaf_hashes().len(),
            3_usize,
            "changed compact fixture should preserve leaf count"
        );
        assert_eq!(
            base.leaf_hashes().first(),
            changed.leaf_hashes().first(),
            "unchanged prefix leaf should keep the same hash"
        );
        assert_ne!(
            base.leaf_hashes().get(1_usize),
            changed.leaf_hashes().get(1_usize),
            "leaf containing the edited record should change hash"
        );
        assert_eq!(
            base.leaf_hashes().get(2_usize),
            changed.leaf_hashes().get(2_usize),
            "unchanged suffix leaf should keep the same hash"
        );
        assert_ne!(
            base.root().hash(),
            changed.root().hash(),
            "root should change when an authenticated leaf changes"
        );
    }

    #[test]
    fn snapshot_bytes_materialize_and_verify_with_real_bao()
    {
        let fixture_records = records();
        let portable = PortableProofTree::build(fixture_records.as_slice(), compact_params())
            .expect("portable fixture tree should build");
        let mut store = InMemoryBlockStore::new();
        let tree = ProllyTree::build(fixture_records.as_slice(), compact_params(), &mut store)
            .expect("store-backed tree wrapper should build");

        let portable_bytes = portable
            .to_snapshot_bytes()
            .expect("portable tree should materialize snapshot bytes");
        let tree_bytes = tree
            .to_snapshot_bytes()
            .expect("tree wrapper should materialize snapshot bytes");
        let mut appended = SnapshotBuffer::from(Vec::<u8>::from(b"prefix".as_slice()));
        portable
            .encode_snapshot_bytes(&mut appended)
            .expect("portable tree should append snapshot bytes");
        let appended_snapshot = appended
            .as_ref()
            .get(b"prefix".len() ..)
            .expect("appended bytes should include the snapshot suffix");
        let mut tree_appended = SnapshotBuffer::default();
        tree.encode_snapshot_bytes(&mut tree_appended)
            .expect("tree wrapper should append snapshot bytes");

        assert_eq!(
            portable.root(),
            tree.root(),
            "portable and store-backed tree wrapper should share the same root"
        );
        assert_eq!(
            portable_bytes.as_ref(),
            tree_bytes.as_ref(),
            "tree wrapper should delegate snapshot byte materialization exactly"
        );
        assert_eq!(
            appended_snapshot,
            portable_bytes.as_ref(),
            "encode_snapshot_bytes should append the same canonical bytes returned by to_snapshot_bytes"
        );
        assert_eq!(
            tree_appended.as_ref(),
            portable_bytes.as_ref(),
            "tree wrapper append API should match portable snapshot bytes"
        );

        assert_bao_verifies_snapshot((portable_bytes.as_ref()).into());
        let verified = verify_snapshot_bytes(
            (portable_bytes.as_ref()).into(),
            portable.root(),
            portable.root().params(),
        )
        .expect("snapshot bytes should verify against the Prolly-Bao root context");
        assert_eq!(
            verified.root(),
            portable.root(),
            "snapshot verification should rebuild the same Prolly-Bao root"
        );
        assert_eq!(
            verified.root_node_hash(),
            portable.root_node_hash(),
            "snapshot verification should bind the same root-node hash"
        );
        assert_eq!(
            verified
                .to_snapshot_bytes()
                .expect("verified snapshot tree should re-encode")
                .as_ref(),
            portable_bytes.as_ref(),
            "verified snapshot tree should re-materialize the exact canonical bytes"
        );
    }

    #[test]
    fn snapshot_bytes_support_empty_and_one_record_trees()
    {
        let empty_records = Vec::<RecordRef<'static>>::new();
        let empty_tree = PortableProofTree::build(empty_records.as_slice(), compact_params())
            .expect("empty snapshot tree should build");
        assert_snapshot_contract(&empty_tree);

        let one_records = vec![RecordRef::new(b"", b"\x00high-byte-value\xff")];
        let one_tree = PortableProofTree::build(one_records.as_slice(), compact_params())
            .expect("one-record snapshot tree should build");
        assert_snapshot_contract(&one_tree);
    }

    #[test]
    fn snapshot_bytes_are_deterministic_for_equivalent_rebuilds()
    {
        let first_records = records();
        let second_records = records();
        let first = PortableProofTree::build(first_records.as_slice(), compact_params())
            .expect("first snapshot fixture tree should build");
        let second = PortableProofTree::build(second_records.as_slice(), compact_params())
            .expect("second snapshot fixture tree should build");
        let first_bytes = first
            .to_snapshot_bytes()
            .expect("first tree should encode snapshot bytes");
        let second_bytes = second
            .to_snapshot_bytes()
            .expect("second tree should encode snapshot bytes");

        assert_eq!(
            first.root(),
            second.root(),
            "equivalent rebuilds should keep the same Prolly-Bao root"
        );
        assert_eq!(
            first_bytes.as_ref(),
            second_bytes.as_ref(),
            "equivalent rebuilds should materialize byte-identical snapshots"
        );
        assert_bao_verifies_snapshot((first_bytes.as_ref()).into());
    }

    #[test]
    fn snapshot_verifier_rejects_tampering_wrong_root_versions_and_lengths()
    {
        let tree = build_tree();
        let bytes = tree
            .to_snapshot_bytes()
            .expect("fixture tree should encode snapshot bytes");

        let mut tampered = Vec::<u8>::from(bytes.as_ref());
        let last_byte = tampered
            .last_mut()
            .expect("snapshot fixture should include value bytes");
        *last_byte ^= 0x01_u8;
        assert!(
            verify_snapshot_bytes(
                tampered.as_slice().into(),
                tree.root(),
                tree.root().params()
            )
            .is_err(),
            "tampered snapshot bytes must not verify against the original root"
        );

        let wrong_root_records = vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"b", b"bravo"),
            RecordRef::new(b"c", b"changed"),
            RecordRef::new(b"d", b"delta"),
            RecordRef::new(b"e", b"echo"),
            RecordRef::new(b"f", b"foxtrot"),
        ];
        let wrong_root_tree =
            PortableProofTree::build(wrong_root_records.as_slice(), compact_params())
                .expect("wrong-root snapshot fixture should build");
        let wrong_root_error = expect_snapshot_verify_failure(
            (bytes.as_ref()).into(),
            wrong_root_tree.root(),
            wrong_root_tree.root().params(),
            ("snapshot bytes must not verify against a different root").into(),
        );
        assert!(
            matches!(
                wrong_root_error,
                ProllyBaoError::HashMismatch { .. } | ProllyBaoError::InvalidProofShape { .. }
            ),
            "wrong root should fail through the root/hash verifier path"
        );

        let mut unsupported_version = Vec::<u8>::from(bytes.as_ref());
        let version_offset = SNAPSHOT_MAGIC.len();
        let version_end = version_offset
            .checked_add(2_usize)
            .expect("snapshot version offset should not overflow");
        assert_eq!(
            unsupported_version
                .get(version_offset .. version_end)
                .expect("snapshot bytes should include a version field"),
            &[0_u8, 1_u8],
            "current snapshot version should be encoded immediately after magic"
        );
        unsupported_version
            .get_mut(version_offset .. version_end)
            .expect("snapshot bytes should include a mutable version field")
            .copy_from_slice(2_u16.to_be_bytes().as_ref());
        let unsupported_error = expect_snapshot_verify_failure(
            (unsupported_version.as_slice()).into(),
            tree.root(),
            tree.root().params(),
            ("unsupported snapshot version must fail verification").into(),
        );
        assert!(
            matches!(
                unsupported_error,
                ProllyBaoError::UnsupportedSnapshotVersion { version: 2_u16 }
            ),
            "unsupported snapshot version should return the dedicated fail-closed error"
        );

        let mut malformed_key_len = Vec::<u8>::from(bytes.as_ref());
        let first_key_len_offset = snapshot_record_section_offset((bytes.as_ref()).into());
        overwrite_be_u64(
            (malformed_key_len.as_mut_slice()).into(),
            first_key_len_offset,
            (u64::MAX).into(),
        );
        let malformed_error = expect_snapshot_verify_failure(
            (malformed_key_len.as_slice()).into(),
            tree.root(),
            tree.root().params(),
            ("malformed snapshot key length must fail verification").into(),
        );
        assert_malformed_snapshot_error(
            &malformed_error,
            ("malformed snapshot length prefix should report malformed snapshot bytes").into(),
        );
    }

    #[test]
    fn snapshot_verifier_rejects_truncation_ordering_duplicates_and_other_encodings()
    {
        let tree = build_tree();
        let bytes = tree
            .to_snapshot_bytes()
            .expect("fixture tree should encode snapshot bytes");

        for truncated_len in 0_usize .. bytes.as_ref().len() {
            assert!(
                verify_snapshot_bytes(
                    bytes
                        .as_ref()
                        .get(.. truncated_len)
                        .expect("truncated snapshot prefix should be in bounds")
                        .into(),
                    tree.root(),
                    tree.root().params(),
                )
                .is_err(),
                "truncated snapshot prefix of length {truncated_len} must fail verification"
            );
        }

        let root_records = vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"b", b"bravo"),
        ];
        let root_tree = PortableProofTree::build(root_records.as_slice(), compact_params())
            .expect("two-record snapshot root fixture should build");

        let unsorted_records = vec![
            RecordRef::new(b"b", b"bravo"),
            RecordRef::new(b"a", b"alpha"),
        ];
        let unsorted_bytes = snapshot_bytes_with_records(
            root_tree.root(),
            root_tree.root_node_hash(),
            (unsorted_records.as_slice()).into(),
        );
        let unsorted_error = expect_snapshot_verify_failure(
            (unsorted_bytes.as_slice()).into(),
            root_tree.root(),
            root_tree.root().params(),
            ("snapshot bytes with unsorted records must fail verification").into(),
        );
        assert!(
            matches!(unsorted_error, ProllyBaoError::UnsortedInput { .. }),
            "snapshot verifier should reject unsorted decoded records before rebuilding"
        );

        let duplicate_records = vec![
            RecordRef::new(b"a", b"alpha"),
            RecordRef::new(b"a", b"again"),
        ];
        let duplicate_bytes = snapshot_bytes_with_records(
            root_tree.root(),
            root_tree.root_node_hash(),
            (duplicate_records.as_slice()).into(),
        );
        let duplicate_error = expect_snapshot_verify_failure(
            (duplicate_bytes.as_slice()).into(),
            root_tree.root(),
            root_tree.root().params(),
            ("snapshot bytes with duplicate keys must fail verification").into(),
        );
        assert!(
            matches!(duplicate_error, ProllyBaoError::DuplicateKeys { .. }),
            "snapshot verifier should reject duplicate decoded keys before rebuilding"
        );

        let membership = tree
            .prove_membership((b"c").into())
            .expect("existing key should produce a membership proof");
        let witness_bytes = WitnessTranscript::from_membership_proof(membership)
            .expect("membership proof should form a witness transcript")
            .to_bytes()
            .expect("membership witness should encode");
        let witness_encoding_error = expect_snapshot_verify_failure(
            (witness_bytes.as_ref()).into(),
            tree.root(),
            tree.root().params(),
            ("native witness transcript bytes are not snapshot bytes").into(),
        );
        assert_malformed_snapshot_error(
            &witness_encoding_error,
            ("native witness transcript bytes should not be accepted as snapshot encoding").into(),
        );

        let (bao_encoded, _bao_hash) = bao::encode::encode(bytes.as_ref());
        let bao_encoding_error = expect_snapshot_verify_failure(
            (bao_encoded.as_slice()).into(),
            tree.root(),
            tree.root().params(),
            ("Bao combined encoding is not the Prolly-Bao snapshot encoding").into(),
        );
        assert_malformed_snapshot_error(
            &bao_encoding_error,
            ("Bao combined bytes should not be accepted as canonical snapshot bytes").into(),
        );
    }

    fn assert_snapshot_contract(tree: &PortableProofTree)
    {
        let bytes = tree
            .to_snapshot_bytes()
            .expect("tree should materialize snapshot bytes");
        assert_bao_verifies_snapshot((bytes.as_ref()).into());
        let verified =
            verify_snapshot_bytes((bytes.as_ref()).into(), tree.root(), tree.root().params())
                .expect("snapshot bytes should verify against their root context");

        assert_eq!(
            verified.root(),
            tree.root(),
            "verified snapshot should rebuild the same root"
        );
        assert_eq!(
            verified.root_node_hash(),
            tree.root_node_hash(),
            "verified snapshot should rebuild the same root-node hash"
        );
    }

    fn assert_bao_verifies_snapshot(snapshot_bytes: FixtureBytes<'_>)
    {
        let snapshot_bytes = snapshot_bytes.as_ref();
        let (bao_encoded, bao_hash) = bao::encode::encode(snapshot_bytes);
        let decoded = bao::decode::decode(bao_encoded.as_slice(), &bao_hash)
            .expect("Bao verifier should decode bytes encoded from the snapshot");

        assert_eq!(
            decoded.as_slice(),
            snapshot_bytes,
            "Bao decode should verify and recover the exact Prolly-Bao snapshot bytes"
        );
    }

    fn snapshot_bytes_with_records(
        root: &TreeRoot,
        root_node_hash: NodeHash,
        records: FixtureSlice<'_, RecordRef<'_>>,
    ) -> OwnedFixtureBytes
    {
        let mut bytes = OwnedFixtureBytes::default();
        let records = records.as_ref();
        bytes.extend_from_slice(SNAPSHOT_MAGIC);
        push_be_u16(&mut bytes, (1_u16).into());
        bytes.extend_from_slice(root.hash().as_ref());
        push_be_u64(&mut bytes, (u64::from(root.record_count())).into());
        push_snapshot_test_bytes(
            &mut bytes,
            (root.params().chunker_parameter_commitment().as_ref()).into(),
        );
        bytes.extend_from_slice(root_node_hash.as_ref());
        push_be_u64(
            &mut bytes,
            (u64::try_from(records.len()).expect("fixture record count should fit u64")).into(),
        );

        for record in records {
            push_snapshot_test_bytes(&mut bytes, (record.key().as_ref()).into());
            push_snapshot_test_bytes(&mut bytes, (record.value().as_ref()).into());
        }

        return bytes;
    }

    fn push_snapshot_test_bytes(
        bytes: &mut OwnedFixtureBytes,
        field: FixtureBytes<'_>,
    )
    {
        let field = field.as_ref();
        push_be_u64(
            bytes,
            (u64::try_from(field.len()).expect("fixture field length should fit u64")).into(),
        );
        bytes.extend_from_slice(field);
    }

    fn snapshot_record_section_offset(bytes: FixtureBytes<'_>) -> ByteOffset
    {
        let bytes = bytes.as_ref();
        let mut offset = SNAPSHOT_MAGIC
            .len()
            .checked_add(2_usize)
            .and_then(|value| {
                return value.checked_add(NODE_HASH_LEN);
            })
            .and_then(|value| {
                return value.checked_add(8_usize);
            })
            .expect("snapshot fixed header offset should not overflow");
        let chunker_len = read_be_u64_as_usize((bytes).into(), (offset).into());
        offset = offset
            .checked_add(8_usize)
            .and_then(|value| {
                return value.checked_add(usize::from(chunker_len));
            })
            .and_then(|value| {
                return value.checked_add(NODE_HASH_LEN);
            })
            .and_then(|value| {
                return value.checked_add(8_usize);
            })
            .expect("snapshot record section offset should not overflow");

        return offset.into();
    }

    fn read_be_u64_as_usize(
        bytes: FixtureBytes<'_>,
        offset: ByteOffset,
    ) -> ByteLength
    {
        let bytes = bytes.as_ref();
        let offset = usize::from(offset);
        let end = offset
            .checked_add(8_usize)
            .expect("fixture u64 offset should not overflow");
        let mut raw = [0_u8; 8_usize];
        raw.copy_from_slice(
            bytes
                .get(offset .. end)
                .expect("fixture bytes should include requested u64"),
        );

        return usize::try_from(u64::from_be_bytes(raw))
            .expect("fixture u64 should fit usize")
            .into();
    }

    fn overwrite_be_u64(
        mut bytes: FixtureBytesMut<'_>,
        offset: ByteOffset,
        value: FixtureLong,
    )
    {
        let bytes = bytes.as_mut();
        let offset = usize::from(offset);
        let value = u64::from(value);
        let end = offset
            .checked_add(8_usize)
            .expect("fixture u64 offset should not overflow");
        bytes
            .get_mut(offset .. end)
            .expect("fixture bytes should include requested mutable u64")
            .copy_from_slice(value.to_be_bytes().as_ref());
    }

    fn push_be_u16(
        bytes: &mut OwnedFixtureBytes,
        value: FixtureWord,
    )
    {
        let value = u16::from(value);
        bytes.extend_from_slice(value.to_be_bytes().as_ref());
    }

    fn push_be_u64(
        bytes: &mut OwnedFixtureBytes,
        value: FixtureLong,
    )
    {
        let value = u64::from(value);
        bytes.extend_from_slice(value.to_be_bytes().as_ref());
    }

    fn expect_snapshot_verify_failure(
        bytes: FixtureBytes<'_>,
        root: &TreeRoot,
        params: &TreeParams,
        context: TestContext,
    ) -> ProllyBaoError
    {
        return verify_snapshot_bytes(bytes.as_ref().into(), root, params)
            .expect_err(context.into());
    }

    fn assert_malformed_snapshot_error(
        error: &ProllyBaoError,
        context: TestContext,
    )
    {
        let context: &'static str = context.into();
        assert!(
            matches!(error, ProllyBaoError::MalformedSnapshotBytes { .. }),
            "{context}"
        );
    }

    fn keys<'records>(
        records: impl Into<FixtureSlice<'records, Record>>
    ) -> Vec<FixtureBytes<'records>>
    {
        let records: &'records [Record] = records.into().into();
        let mut keys = Vec::<FixtureBytes<'records>>::with_capacity(records.len());

        for record in records {
            let key: &'records [u8] = record.key().into();
            keys.push(key.into());
        }

        return keys;
    }

    struct ExpectedWitnessSummary
    {
        kind: WitnessKind,
        root_hash: NodeHash,
        root_record_count: TreeRecordCount,
        chunker_parameter_digest: NodeHash,
        root_node_hash: NodeHash,
        body_digest: NodeHash,
        proof_node_count: ProofNodeCount,
        proof_nodes_digest: NodeHash,
        binding_digest: NodeHash,
    }

    fn assert_witness_summary(
        summary: &WitnessEndSummary,
        expected: &ExpectedWitnessSummary,
    )
    {
        assert_eq!(
            summary.version(),
            WitnessTranscript::VERSION,
            "witness end summary should bind the witness transcript version"
        );
        assert_eq!(
            summary.kind(),
            expected.kind,
            "witness end summary should bind the witness kind"
        );
        assert_eq!(
            summary.root_hash(),
            expected.root_hash,
            "witness end summary should bind the Prolly-Bao root hash"
        );
        assert_eq!(
            summary.root_record_count(),
            expected.root_record_count,
            "witness end summary should bind the root record count"
        );
        assert_eq!(
            summary.chunker_parameter_digest(),
            expected.chunker_parameter_digest,
            "witness end summary chunker parameter digest should round trip"
        );
        assert_eq!(
            summary.root_node_hash(),
            expected.root_node_hash,
            "witness end summary should bind the root node hash"
        );
        assert_eq!(
            summary.body_digest(),
            expected.body_digest,
            "witness end summary body digest should round trip"
        );
        assert_eq!(
            summary.proof_node_count(),
            expected.proof_node_count,
            "witness end summary should bind the proof node count"
        );
        assert_eq!(
            summary.proof_nodes_digest(),
            expected.proof_nodes_digest,
            "witness end summary proof-nodes digest should round trip"
        );
        assert_eq!(
            summary.binding_digest(),
            expected.binding_digest,
            "witness end summary binding digest should round trip"
        );
    }

    fn proof_node_count(nodes: FixtureSlice<'_, ProofNode>) -> ProofNodeCount
    {
        let nodes: &[ProofNode] = nodes.into();
        let count = u64::try_from(nodes.len()).expect("fixture proof node count should fit u64");
        return ProofNodeCount::from(count);
    }

    fn round_trip_witness(transcript: &WitnessTranscript) -> WitnessTranscript
    {
        let mut encoded = WitnessBuffer::default();
        transcript
            .encode(&mut encoded)
            .expect("witness transcript should encode into caller buffer");
        let direct = transcript
            .to_bytes()
            .expect("witness transcript should encode to owned bytes");

        assert_eq!(
            encoded.as_ref(),
            direct.as_ref(),
            "encode and to_bytes should produce identical transcript bytes"
        );

        return WitnessTranscript::decode((direct.as_ref()).into())
            .expect("encoded witness transcript should decode");
    }

    fn witness_prefix<'bytes>(
        bytes: FixtureBytes<'bytes>,
        end: ByteOffset,
        context: TestContext,
    ) -> FixtureBytes<'bytes>
    {
        let bytes: &'bytes [u8] = bytes.into();
        let context: &'static str = context.into();
        return bytes.get(.. usize::from(end)).expect(context).into();
    }

    fn witness_suffix<'bytes>(
        bytes: FixtureBytes<'bytes>,
        start: ByteOffset,
        context: TestContext,
    ) -> FixtureBytes<'bytes>
    {
        let bytes: &'bytes [u8] = bytes.into();
        let context: &'static str = context.into();
        return bytes.get(usize::from(start) ..).expect(context).into();
    }

    fn witness_suffix_mut<'bytes>(
        bytes: FixtureBytesMut<'bytes>,
        start: ByteOffset,
        context: TestContext,
    ) -> FixtureBytesMut<'bytes>
    {
        let context: &'static str = context.into();
        let start = usize::from(start);
        let bytes: &'bytes mut [u8] = bytes.into();
        return bytes.get_mut(start ..).expect(context).into();
    }

    fn assert_witness_decode_rejects_mismatched_end_summary(
        tree: &PortableProofTree,
        valid_bytes: OwnedFixtureBytes,
        summary_start: ByteOffset,
    )
    {
        let other_membership = tree
            .prove_membership((b"d").into())
            .expect("second existing key should produce a membership proof");
        let other_witness = WitnessTranscript::from_membership_proof(other_membership)
            .expect("second membership proof should form a witness transcript");
        let other_bytes = other_witness
            .to_bytes()
            .expect("second membership witness should encode");
        let other_summary_start = other_bytes
            .as_ref()
            .len()
            .checked_sub(WITNESS_END_SUMMARY_LEN)
            .expect("second witness bytes should include an end summary");
        let valid_end_summary = witness_suffix(
            valid_bytes.as_ref().into(),
            summary_start,
            "witness bytes should include an end summary".into(),
        );
        let other_end_summary = witness_suffix(
            (other_bytes.as_ref()).into(),
            (other_summary_start).into(),
            ("second witness bytes should include an end summary").into(),
        );
        assert_ne!(
            valid_end_summary, other_end_summary,
            "different membership statements should have different end summaries"
        );

        let mut mismatched = valid_bytes;
        let mut mismatched_end_summary = witness_suffix_mut(
            mismatched.as_mut().into(),
            summary_start,
            "witness bytes should include a mutable end summary".into(),
        );
        assert_eq!(
            mismatched_end_summary.len(),
            other_end_summary.len(),
            "witness end summaries should have equal encoded lengths"
        );
        mismatched_end_summary.copy_from_slice(other_end_summary.as_ref());
        let mismatched_error = expect_witness_decode_failure(
            (mismatched.as_ref()).into(),
            ("witness bytes with a mismatched end summary must fail decode").into(),
        );
        assert_malformed_witness_error(
            &mismatched_error,
            ("mismatched witness end summary should report malformed witness bytes").into(),
        );
    }

    fn proof_node<'nodes>(
        nodes: impl Into<FixtureSlice<'nodes, ProofNode>>,
        index: impl Into<ProofNodeIndex>,
    ) -> ProofNode
    {
        let nodes: &'nodes [ProofNode] = nodes.into().into();
        return nodes
            .get(usize::from(index.into()))
            .expect("compact proof fixture should include requested node")
            .clone();
    }

    fn assert_invalid_proof_shape(
        error: &ProllyBaoError,
        context: TestContext,
    )
    {
        let context: &'static str = context.into();
        assert!(
            matches!(error, ProllyBaoError::InvalidProofShape { .. }),
            "{context}"
        );
    }
    fn expect_witness_decode_failure(
        bytes: FixtureBytes<'_>,
        context: TestContext,
    ) -> ProllyBaoError
    {
        return WitnessTranscript::decode(bytes.as_ref().into()).expect_err(context.into());
    }

    fn assert_malformed_witness_error(
        error: &ProllyBaoError,
        context: TestContext,
    )
    {
        let context: &'static str = context.into();
        assert!(
            matches!(error, ProllyBaoError::MalformedWitnessBytes { .. }),
            "{context}"
        );
    }

    fn tamper_first_node(proof: &MembershipProof) -> MembershipProof
    {
        let mut nodes = Vec::<ProofNode>::with_capacity(proof.nodes().len());
        let mut first = true;

        for node in proof.nodes() {
            if first {
                let mut bytes = Box::<[u8]>::from(node.bytes());
                let first_byte = bytes
                    .first_mut()
                    .expect("generated proof node bytes should be non-empty");
                *first_byte ^= 0x01_u8;
                nodes.push(ProofNode::new(node.hash(), bytes));
                first = false;
            }
            else {
                nodes.push(node.clone());
            }
        }

        return MembershipProof::new(
            proof.envelope().clone(),
            proof.root_node_hash(),
            proof.key(),
            proof.value(),
            nodes.into_boxed_slice(),
        );
    }
}
