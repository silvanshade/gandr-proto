//! Content-addressed checkpoint persistence and backend-aware reuse.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::hash::Hash;
use core::hash::Hasher;

use crate::checkpoint::Checkpoints;
use crate::region::Program;

/// A stable content address for one lowered program revision.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CheckpointAddress([u8; 32]);

impl CheckpointAddress
{
    /// Returns the raw BLAKE3 digest bytes.
    #[must_use]
    pub fn bytes(self) -> [u8; 32]
    {
        self.0
    }
}

/// Identity of the backend artifact that consumed a checkpoint.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct BackendArtifact([u8; 32]);

impl BackendArtifact
{
    /// Derives an artifact identity from canonical artifact bytes.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self
    {
        Self(*blake3::hash(bytes).as_bytes())
    }
}

/// A persistence failure that leaves the caller with no partially trusted
/// state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointStoreError
{
    /// The backing store rejected a complete checkpoint record.
    Rejected,
}

/// Observes persistence and invalidation without participating in checking.
pub trait CheckpointObserver
{
    /// Called after a checkpoint is accepted by the store.
    fn stored(
        &mut self,
        _address: CheckpointAddress,
    )
    {
    }
    /// Called when a stored checkpoint is rejected for the current backend.
    fn invalidated(
        &mut self,
        _address: CheckpointAddress,
    )
    {
    }
}

/// A persistence boundary for complete, validated checkpoint sets.
pub trait CheckpointStore
{
    /// Loads a record by content address and backend identity.
    fn load(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
    ) -> Result<Option<Checkpoints>, CheckpointStoreError>;
    /// Stores a complete checkpoint set under its content address.
    fn store(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
        checkpoints: Checkpoints,
    ) -> Result<(), CheckpointStoreError>;
}

/// A deterministic in-memory store useful for process-reopen and contract
/// tests.
#[derive(Clone, Debug, Default)]
pub struct MemoryCheckpointStore
{
    records: BTreeMap<(CheckpointAddress, BackendArtifact), Checkpoints>,
}

impl CheckpointStore for MemoryCheckpointStore
{
    fn load(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
    ) -> Result<Option<Checkpoints>, CheckpointStoreError>
    {
        Ok(self.records.get(&(address, backend)).cloned())
    }

    fn store(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
        checkpoints: Checkpoints,
    ) -> Result<(), CheckpointStoreError>
    {
        self.records.insert((address, backend), checkpoints);
        Ok(())
    }
}

/// Computes the address of a lowered program, independent of item positions.
#[must_use]
pub fn address_of(program: &Program) -> CheckpointAddress
{
    let mut hasher = DigestHasher::default();
    program.items.len().hash(&mut hasher);
    for item in &program.items {
        item.name.hash(&mut hasher);
        item.ascription.hash(&mut hasher);
        alloc::format!("{:?}", item.term).hash(&mut hasher);
    }
    CheckpointAddress(hasher.finish_digest())
}

/// Stores a checkpoint and notifies the optional extension observer.
///
/// # Contract
/// - requires: `checkpoints` was produced for `program` and `backend`.
/// - ensures: the complete record is stored under `address_of(program)`.
/// - provides: the content address used for subsequent resume.
/// - fails: returns [`CheckpointStoreError::Rejected`] when the store rejects
///   it.
/// - panics: none.
pub fn persist<S, O>(
    store: &mut S,
    program: &Program,
    backend: BackendArtifact,
    checkpoints: Checkpoints,
    observer: &mut O,
) -> Result<CheckpointAddress, CheckpointStoreError>
where
    S: CheckpointStore,
    O: CheckpointObserver,
{
    let address = address_of(program);
    store.store(address, backend, checkpoints)?;
    observer.stored(address);
    Ok(address)
}

/// Restores only a complete checkpoint matching both content and backend.
///
/// A backend change is an invalidation, never a cache miss that can be silently
/// treated as valid state.
#[must_use]
pub fn restore<S, O>(
    store: &mut S,
    program: &Program,
    address: CheckpointAddress,
    backend: BackendArtifact,
    observer: &mut O,
) -> Result<Option<Checkpoints>, CheckpointStoreError>
where
    S: CheckpointStore,
    O: CheckpointObserver,
{
    let expected = address_of(program);
    if expected != address {
        observer.invalidated(address);
        return Ok(None);
    }
    let restored = store.load(address, backend)?;
    if restored.is_none() {
        observer.invalidated(address);
    }
    Ok(restored)
}

#[derive(Default)]
struct DigestHasher
{
    _hasher: blake3::Hasher,
    bytes: Vec<u8>,
}

impl Hasher for DigestHasher
{
    fn finish(&self) -> u64
    {
        0
    }
    fn write(
        &mut self,
        bytes: &[u8],
    )
    {
        self.bytes.extend_from_slice(bytes);
    }
}

impl DigestHasher
{
    fn finish_digest(self) -> [u8; 32]
    {
        *blake3::hash(&self.bytes).as_bytes()
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[derive(Default)]
    struct Observer
    {
        stored: usize,
        invalidated: usize,
    }

    impl CheckpointObserver for Observer
    {
        fn stored(
            &mut self,
            _address: CheckpointAddress,
        )
        {
            self.stored += 1;
        }

        fn invalidated(
            &mut self,
            _address: CheckpointAddress,
        )
        {
            self.invalidated += 1;
        }
    }

    #[test]
    fn persists_and_restores_complete_checkpoint_sets()
    {
        let program = Program::default();
        let backend = BackendArtifact::from_bytes(b"backend-v1");
        let mut store = MemoryCheckpointStore::default();
        let mut observer = Observer::default();

        let address = persist(
            &mut store,
            &program,
            backend,
            Checkpoints::default(),
            &mut observer,
        )
        .expect("memory store accepts complete records");

        assert_eq!(observer.stored, 1);
        assert_eq!(
            restore(&mut store, &program, address, backend, &mut observer)
                .expect("matching records load"),
            Some(Checkpoints::default())
        );
        assert_eq!(observer.invalidated, 0);
    }

    #[test]
    fn backend_identity_invalidates_prior_checkpoint()
    {
        let program = Program::default();
        let backend_v1 = BackendArtifact::from_bytes(b"backend-v1");
        let backend_v2 = BackendArtifact::from_bytes(b"backend-v2");
        let mut store = MemoryCheckpointStore::default();
        let mut observer = Observer::default();
        let address = persist(
            &mut store,
            &program,
            backend_v1,
            Checkpoints::default(),
            &mut observer,
        )
        .expect("memory store accepts complete records");

        assert_eq!(
            restore(&mut store, &program, address, backend_v2, &mut observer)
                .expect("backend mismatch is a cache miss"),
            None
        );
        assert_eq!(observer.invalidated, 1);
    }
}
