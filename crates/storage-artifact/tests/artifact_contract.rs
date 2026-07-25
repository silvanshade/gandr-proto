//! The outer-layer contract suite: record extraction round-trip, manifest
//! determinism and sensitivity, the history-independence differential, and
//! store/retrieve through a `BlockStore` (massive-term design §6; B2.3).

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the contract suite readable (docs/workflow/rust.md)"
    )
)]

#[cfg(test)]
mod tests
{
    use gandr_kernel_core::Environment;
    use gandr_kernel_core::FORMAT_VERSION_V1;
    use gandr_kernel_core::LevelSignature;
    use gandr_kernel_core::write;
    use gandr_storage_artifact::AdmissionIndex;
    use gandr_storage_artifact::ArtifactRecord;
    use gandr_storage_artifact::ArtifactRecordSet;
    use gandr_storage_artifact::RecordBytes;
    use gandr_storage_artifact::build;
    use gandr_storage_prolly_trees::InMemoryBlockStore;
    use gandr_storage_prolly_trees::ProllyTree;
    use gandr_storage_prolly_trees::TreeParams;
    use proptest::prelude::*;

    /// Borrowed declaration-kind choices for one generated environment.
    #[repr(transparent)]
    #[derive(Clone, Copy)]
    struct DeclarationKinds<'kinds>(
        /// `true` chooses a definition; `false` chooses an axiom.
        &'kinds [bool],
    );

    impl<'kinds> From<&'kinds [bool]> for DeclarationKinds<'kinds>
    {
        #[inline]
        fn from(kinds: &'kinds [bool]) -> Self
        {
            return Self(kinds);
        }
    }

    /// One semantic record fixture before ownership is transferred.
    #[derive(Clone, Copy)]
    struct RecordFixture<'record>
    {
        /// Admission-order key for the fixture record.
        admission_index: AdmissionIndex,
        /// Borrowed declaration-segment bytes.
        value: RecordBytes<'record>,
    }

    /// An environment whose declarations share subterms across and within
    /// declarations (a `def unit`, a `def pair(unit, unit)`, and an `axiom`).
    fn shared_environment() -> Environment
    {
        let mut environment = Environment::new();

        let first = {
            let mut builder = environment.stage();
            let declared = builder.arena().value_type_unit();
            let body = builder.arena().value_unit();
            builder.def(LevelSignature::monomorphic(), declared, body)
        };
        let _first = environment.add_decl_unchecked(first);

        let second = {
            let mut builder = environment.stage();
            let declared = builder.arena().value_type_unit();
            let left = builder.arena().value_unit();
            let right = builder.arena().value_unit();
            let body = builder.arena().value_pair(left, right);
            builder.def(LevelSignature::monomorphic(), declared, body)
        };
        let _second = environment.add_decl_unchecked(second);

        let third = {
            let mut builder = environment.stage();
            let declared = builder.arena().value_type_unit();
            builder.axiom(LevelSignature::monomorphic(), declared)
        };
        let _third = environment.add_decl_unchecked(third);

        environment
    }

    /// Build an environment from a sequence of declaration kinds (`true` a
    /// `def unit`, `false` an `axiom`) — every declaration nameless and
    /// bypass-admitted, so the outer layer sees arbitrary declaration counts.
    fn environment_from_kinds(kinds: DeclarationKinds<'_>) -> Environment
    {
        let mut environment = Environment::new();
        for &is_def in kinds.0 {
            let declaration = {
                let mut builder = environment.stage();
                let declared = builder.arena().value_type_unit();
                if is_def {
                    let body = builder.arena().value_unit();
                    builder.def(LevelSignature::monomorphic(), declared, body)
                }
                else {
                    builder.axiom(LevelSignature::monomorphic(), declared)
                }
            };
            let _id = environment.add_decl_unchecked(declaration);
        }
        environment
    }

    #[test]
    fn records_round_trip_to_a_byte_identical_artifact()
    {
        let environment = shared_environment();
        let bytes = write(&environment);
        let records = ArtifactRecordSet::from_environment(&environment);

        assert_eq!(
            records.reassemble().as_ref(),
            bytes.as_ref(),
            "header plus records in key order reproduces the artifact byte-for-byte"
        );

        let mut store = InMemoryBlockStore::new();
        let built = build(&records, TreeParams::default(), &mut store).expect("build succeeds");
        assert_eq!(
            built.manifest().record_count(),
            records.record_count(),
            "the manifest binds the declaration-record count"
        );
        assert_eq!(
            records.reassemble().as_ref(),
            bytes.as_ref(),
            "building does not disturb reassembly"
        );
    }

    #[test]
    fn the_same_artifact_mints_the_same_identity()
    {
        let environment = shared_environment();
        let records = ArtifactRecordSet::from_environment(&environment);

        let mut first_store = InMemoryBlockStore::new();
        let mut second_store = InMemoryBlockStore::new();
        let first = build(&records, TreeParams::default(), &mut first_store).expect("build");
        let second = build(&records, TreeParams::default(), &mut second_store).expect("build");

        assert_eq!(
            first.identity(),
            second.identity(),
            "the same artifact mints the same identity"
        );
        assert_eq!(
            first.root_node_hash(),
            second.root_node_hash(),
            "the same artifact has the same root node hash"
        );
    }

    #[test]
    fn any_perturbation_changes_the_identity()
    {
        let base_records = alloc_records(&[
            RecordFixture {
                admission_index: 0_u64.into(),
                value: b"alpha".as_slice().into(),
            },
            RecordFixture {
                admission_index: 1_u64.into(),
                value: b"beta".as_slice().into(),
            },
        ]);
        let perturbed_records = alloc_records(&[
            RecordFixture {
                admission_index: 0_u64.into(),
                value: b"alpha".as_slice().into(),
            },
            RecordFixture {
                admission_index: 1_u64.into(),
                value: b"BETA!".as_slice().into(),
            },
        ]);

        let base = ArtifactRecordSet::from_records(
            FORMAT_VERSION_V1.into(),
            b"header".as_slice().into(),
            base_records,
        )
        .unwrap();
        let perturbed = ArtifactRecordSet::from_records(
            FORMAT_VERSION_V1.into(),
            b"header".as_slice().into(),
            perturbed_records,
        )
        .unwrap();

        let mut base_store = InMemoryBlockStore::new();
        let mut perturbed_store = InMemoryBlockStore::new();
        let base_built = build(&base, TreeParams::default(), &mut base_store).expect("build");
        let perturbed_built =
            build(&perturbed, TreeParams::default(), &mut perturbed_store).expect("build");

        assert_ne!(
            base_built.identity(),
            perturbed_built.identity(),
            "a perturbed record changes the artifact identity"
        );
    }

    #[test]
    fn a_permuted_build_order_yields_the_same_identity()
    {
        let environment = shared_environment();
        let base = ArtifactRecordSet::from_environment(&environment);

        let mut permuted_records: Vec<ArtifactRecord> = base.records().to_vec();
        permuted_records.reverse();
        let permuted = ArtifactRecordSet::from_records(
            base.inner_format_version(),
            base.header(),
            permuted_records,
        )
        .expect("the records are unique");

        let mut base_store = InMemoryBlockStore::new();
        let mut permuted_store = InMemoryBlockStore::new();
        let base_built = build(&base, TreeParams::default(), &mut base_store).expect("build");
        let permuted_built =
            build(&permuted, TreeParams::default(), &mut permuted_store).expect("build");

        assert_eq!(
            base_built.identity(),
            permuted_built.identity(),
            "a permuted build order yields the identical artifact identity (history-independence)"
        );
        assert_eq!(
            base_built.root_node_hash(),
            permuted_built.root_node_hash(),
            "the root node hash is history-independent"
        );
    }

    #[test]
    fn tree_nodes_store_and_reopen()
    {
        let environment = shared_environment();
        let records = ArtifactRecordSet::from_environment(&environment);

        let mut store = InMemoryBlockStore::new();
        let built = build(&records, TreeParams::default(), &mut store).expect("build");

        let opened = ProllyTree::open(built.root().clone(), built.root_node_hash(), &store)
            .expect("the root node is present and hash-valid in the store");
        assert_eq!(
            opened.root_node_hash(),
            built.root_node_hash(),
            "reopening recovers the stored root node hash"
        );

        let empty = InMemoryBlockStore::new();
        assert!(
            ProllyTree::open(built.root().clone(), built.root_node_hash(), &empty).is_err(),
            "an absent root node fails to open (fail-closed)"
        );
    }

    proptest! {
        /// Record extraction round-trips and the identity is deterministic over
        /// arbitrary declaration sequences.
        #[test]
        fn round_trip_over_generated_environments(
            kinds in prop::collection::vec(any::<bool>(), 0 .. 10_usize)
        )
        {
            let environment = environment_from_kinds(kinds.as_slice().into());
            let bytes = write(&environment);
            let records = ArtifactRecordSet::from_environment(&environment);

            let reassembled = records.reassemble();
            prop_assert_eq!(reassembled.as_ref(), bytes.as_ref());
            prop_assert_eq!(
                u64::from(records.record_count()),
                u64::try_from(kinds.len()).unwrap()
            );

            let mut first_store = InMemoryBlockStore::new();
            let mut second_store = InMemoryBlockStore::new();
            let first = build(&records, TreeParams::default(), &mut first_store)
                .expect("build succeeds");
            let second = build(&records, TreeParams::default(), &mut second_store)
                .expect("build succeeds");
            prop_assert_eq!(first.identity(), second.identity());
        }
    }

    /// Build owned records from semantic fixture pairs.
    fn alloc_records(fixtures: &[RecordFixture<'_>]) -> Vec<ArtifactRecord>
    {
        return fixtures
            .iter()
            .map(|fixture| ArtifactRecord::new(fixture.admission_index, fixture.value))
            .collect();
    }
}
