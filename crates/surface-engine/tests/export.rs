//! Session kernel-artifact export tests: the storage tier's shipping
//! consumer. A session exports the kernel environment its admitted
//! definitions accumulated through `gandr-storage-artifact` as a
//! content-addressed artifact, with the manifest identity available to the
//! caller — byte-equal for independently built equal environments, changed
//! by changed environment content, and empty only for an empty environment.

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "integration tests share fixture helpers called from tests in per-test orders; no single module arrangement satisfies every caller-before-callee pair, so the ordering rule is waived in test code pending a test-layout redesign"
    )
)]

/// Kernel-artifact export tests.
#[cfg(test)]
mod tests
{
    use gandr_core_incremental::persistence::BackendArtifact;
    use gandr_core_incremental::persistence::CheckpointObserver;
    use gandr_core_incremental::persistence::FileCheckpointStore;
    use gandr_core_incremental::persistence::restore;
    use gandr_core_incremental::region::Program;
    use gandr_storage_artifact::ArtifactRecordCount;
    use gandr_storage_prolly_trees::InMemoryBlockStore;
    use gandr_storage_prolly_trees::TreeParams;
    use gandr_surface_engine::session::Session;

    #[derive(Default)]
    struct Observer;

    impl CheckpointObserver for Observer
    {
    }

    /// The real session path exports the session's own admitted environment:
    /// the exported artifact commits one record per admission, and its
    /// identity equals a direct `storage-artifact` build over the
    /// environment the session's ledger reports.
    ///
    /// A path wired to a fresh or canned environment instead of the ledger's
    /// own fails both halves of this assertion.
    #[test]
    fn the_session_export_path_exports_the_admitted_environment()
    {
        let mut session = Session::new();
        session
            .submit("def a = 3;\ndef b = a;")
            .expect("lowering must not fail");

        let mut store = InMemoryBlockStore::new();
        let built = session
            .export_kernel_artifact(TreeParams::default(), &mut store)
            .expect("an admitted environment builds");

        let admitted = usize::from(session.kernel().admitted());
        assert_eq!(
            ArtifactRecordCount::from(
                u64::try_from(admitted).expect("an admission count fits in u64")
            ),
            built.manifest().record_count(),
            "the artifact commits one record per admitted definition"
        );

        let mut direct_store = InMemoryBlockStore::new();
        let direct = gandr_storage_artifact::build_from_environment(
            session.kernel().environment(),
            TreeParams::default(),
            &mut direct_store,
        )
        .expect("a direct build over the same environment succeeds");
        let built_identity = built.identity();
        let direct_identity = direct.identity();
        let built_bytes: &[u8] = built_identity.as_ref();
        let direct_bytes: &[u8] = direct_identity.as_ref();
        assert_eq!(
            built_bytes, direct_bytes,
            "the session path exports the ledger's own environment, not a \
             reconstructed one"
        );
    }

    /// The export identity is history-independent at the session level: two
    /// sessions that admitted the same definitions through different
    /// submission groupings mint byte-equal identities.
    #[test]
    fn independently_built_sessions_mint_byte_equal_identities()
    {
        let mut grouped = Session::new();
        grouped
            .submit("def a = 3;\ndef b = a;")
            .expect("lowering must not fail");

        let mut split = Session::new();
        split.submit("def a = 3;").expect("lowering must not fail");
        split.submit("def b = a;").expect("lowering must not fail");

        let mut grouped_store = InMemoryBlockStore::new();
        let grouped_built = grouped
            .export_kernel_artifact(TreeParams::default(), &mut grouped_store)
            .expect("the grouped session's environment builds");
        let mut split_store = InMemoryBlockStore::new();
        let split_built = split
            .export_kernel_artifact(TreeParams::default(), &mut split_store)
            .expect("the split session's environment builds");

        let grouped_identity = grouped_built.identity();
        let split_identity = split_built.identity();
        let grouped_bytes: &[u8] = grouped_identity.as_ref();
        let split_bytes: &[u8] = split_identity.as_ref();
        assert_eq!(
            grouped_bytes, split_bytes,
            "semantically equal, independently built environments mint \
             byte-equal identities"
        );
    }

    /// Changed environment content changes the exported identity: one
    /// different definition body in an otherwise identical program yields a
    /// different artifact identity.
    #[test]
    fn changed_environment_content_changes_the_identity()
    {
        let mut base = Session::new();
        base.submit("def a = 3;\ndef b = a;")
            .expect("lowering must not fail");
        let mut changed = Session::new();
        changed
            .submit("def a = 4;\ndef b = a;")
            .expect("lowering must not fail");

        let mut base_store = InMemoryBlockStore::new();
        let base_built = base
            .export_kernel_artifact(TreeParams::default(), &mut base_store)
            .expect("the base environment builds");
        let mut changed_store = InMemoryBlockStore::new();
        let changed_built = changed
            .export_kernel_artifact(TreeParams::default(), &mut changed_store)
            .expect("the changed environment builds");

        assert_eq!(
            base_built.manifest().record_count(),
            changed_built.manifest().record_count(),
            "both programs admit the same number of definitions"
        );
        let base_identity = base_built.identity();
        let changed_identity = changed_built.identity();
        let base_bytes: &[u8] = base_identity.as_ref();
        let changed_bytes: &[u8] = changed_identity.as_ref();
        assert_ne!(
            base_bytes, changed_bytes,
            "changed environment content changes the identity"
        );
    }

    /// A session that admitted nothing exports the empty record set: the
    /// artifact builds, and its manifest commits zero records.
    #[test]
    fn an_empty_session_exports_an_empty_record_set()
    {
        let session = Session::new();
        let mut store = InMemoryBlockStore::new();
        let built = session
            .export_kernel_artifact(TreeParams::default(), &mut store)
            .expect("the empty environment builds");
        assert_eq!(
            ArtifactRecordCount::from(0_u64),
            built.manifest().record_count(),
            "no admissions, so no records"
        );
    }

    /// The shipping session operation persists through the durable checkpoint
    /// backend, reopens the canonical payload, and rejects a tampered file.
    #[test]
    fn session_persists_and_reopens_file_checkpoint()
    {
        let root =
            std::env::temp_dir().join(format!("gandr-surface-session-{}", std::process::id()));
        drop(std::fs::remove_dir_all(&root));

        let session = Session::new();
        let mut artifact_store = InMemoryBlockStore::new();
        let mut checkpoint_store = FileCheckpointStore::open(&root).expect("open checkpoint store");
        let mut observer = Observer;
        let (built, address) = session
            .persist_kernel_checkpoint(
                TreeParams::default(),
                &mut artifact_store,
                &mut checkpoint_store,
                &mut observer,
            )
            .expect("persist the session checkpoint");
        let manifest = built.manifest().encode();
        let backend = BackendArtifact::from_bytes(manifest.as_ref());
        drop(checkpoint_store);

        let mut reopened = FileCheckpointStore::open(&root).expect("reopen checkpoint store");
        assert!(
            restore(
                &mut reopened,
                &Program::default(),
                address,
                backend,
                &mut observer
            )
            .expect("load the persisted checkpoint")
            .is_some()
        );

        let path = std::fs::read_dir(&root)
            .expect("read checkpoint directory")
            .next()
            .expect("one checkpoint file")
            .expect("read checkpoint directory entry")
            .path();
        let mut bytes = std::fs::read(&path).expect("read checkpoint file");
        let last = bytes
            .len()
            .checked_sub(1)
            .expect("non-empty checkpoint file");
        bytes[last] ^= 1;
        std::fs::write(&path, bytes).expect("tamper checkpoint file");
        assert_eq!(
            restore(
                &mut reopened,
                &Program::default(),
                address,
                backend,
                &mut observer
            ),
            Err(gandr_core_incremental::persistence::CheckpointStoreError::Corrupt)
        );
        std::fs::remove_dir_all(root).expect("remove checkpoint directory");
    }
}
