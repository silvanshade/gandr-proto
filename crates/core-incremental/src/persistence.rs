//! Content-addressed checkpoint persistence and backend-aware reuse.
#![allow(
    unknown_lints,
    reason = "The durable boundary has toolchain-specific lint names."
)]
#![allow(
    primitive_signature,
    reason = "Raw bytes are the content-addressed serialization boundary."
)]
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;
use core::hash::Hash as _;
use core::hash::Hasher;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use gandr_core_checker::types::Ty;
use gandr_core_checker::types::ValueType;
/// Fixed byte length of the file-artifact header.
const FILE_HEADER_LEN: usize = 76;

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
    #[inline]
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
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Self
    {
        Self(*blake3::hash(bytes).as_bytes())
    }
}

/// Semantic checkpoint structures deferred to the durable HPSA codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedPersistence
{
    /// A declaration-order datatype serial is process-local.
    DataIdSerial,
    /// A mint-order opaque seal serial is process-local.
    SealIdSerial,
    /// A reified stack requires a stable DTO owned by HPSA.
    OpaqueStack,
    /// An inline effect signature requires a stable DTO owned by HPSA.
    OpaqueEffectSignature,
    /// A checkpoint item has unsupported nested semantic fields.
    CheckpointItem,
}

/// A persistence failure that leaves the caller with no partially trusted
/// state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointStoreError
{
    /// The backing store rejected a complete checkpoint record.
    Rejected,
    /// A semantic field has no stable process-independent representation yet.
    UnsupportedPersistence(UnsupportedPersistence),
    /// A file artifact is truncated or has an invalid schema.
    Corrupt,
    /// The backing store could not complete an operating-system request.
    Io,
}

/// Observes persistence and invalidation without participating in checking.
pub trait CheckpointObserver
{
    /// Called after a checkpoint is accepted by the store.
    #[inline]
    fn stored(
        &mut self,
        _address: CheckpointAddress,
    )
    {
    }

    /// Called when a stored checkpoint is rejected for the current backend.
    #[inline]
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
    ///
    /// # Errors
    ///
    /// Returns a typed persistence error when the backing store fails.
    fn load(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
    ) -> Result<Option<Checkpoints>, CheckpointStoreError>;

    /// Stores a complete checkpoint set under its content address.
    ///
    /// # Errors
    ///
    /// Returns a typed persistence error when the record cannot be stored.
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
#[repr(transparent)]
pub struct MemoryCheckpointStore
{
    /// In-memory records keyed by content and backend identity.
    records: BTreeMap<(CheckpointAddress, BackendArtifact), Checkpoints>,
}

impl CheckpointStore for MemoryCheckpointStore
{
    #[inline]
    fn load(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
    ) -> Result<Option<Checkpoints>, CheckpointStoreError>
    {
        Ok(self.records.get(&(address, backend)).cloned())
    }

    #[inline]
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

/// A content-addressed file store with atomic replacement of complete records.
#[derive(Debug)]
pub struct FileCheckpointStore
{
    /// Directory containing hash-named checkpoint artifacts.
    root: PathBuf,
    /// Process-local mirror of records written by this store.
    records: BTreeMap<(CheckpointAddress, BackendArtifact), Checkpoints>,
}

impl FileCheckpointStore
{
    /// Opens or creates a checkpoint artifact directory.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointStoreError::Io`] when directory creation fails.
    #[inline]
    pub fn open<P>(path: P) -> Result<Self, CheckpointStoreError>
    where
        P: AsRef<Path>,
    {
        let root = path.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|error| {
            drop(error);
            CheckpointStoreError::Io
        })?;
        Ok(Self {
            root,
            records: BTreeMap::new(),
        })
    }
}

impl CheckpointStore for FileCheckpointStore
{
    #[inline]
    fn load(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
    ) -> Result<Option<Checkpoints>, CheckpointStoreError>
    {
        let path = self.root.join(hex(address.0));
        let Ok(bytes) = fs::read(path)
        else {
            return Ok(None);
        };
        if bytes.len() < FILE_HEADER_LEN
            || bytes.get(.. 8) != Some(b"GFILE\0\0\0")
            || bytes.get(8 .. 12) != Some(&1_u32.to_le_bytes())
            || bytes.get(12 .. 44) != Some(&address.0)
        {
            return Err(CheckpointStoreError::Corrupt);
        }
        if bytes.get(44 .. FILE_HEADER_LEN) != Some(&backend.0) {
            return Ok(None);
        }
        let payload = bytes
            .get(FILE_HEADER_LEN ..)
            .ok_or(CheckpointStoreError::Corrupt)?;
        let decoded = decode_checkpoints(payload)?;
        Ok(Some(decoded))
    }

    #[inline]
    fn store(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
        checkpoints: Checkpoints,
    ) -> Result<(), CheckpointStoreError>
    {
        let payload = encode_checkpoints(&checkpoints)?;
        let artifact = artifact_bytes(address, backend, &payload);
        let path = self.root.join(hex(address.0));
        let temporary = self
            .root
            .join(format!("{}.tmp-{}", hex(address.0), std::process::id()));
        fs::write(&temporary, artifact).map_err(|error| {
            drop(error);
            CheckpointStoreError::Io
        })?;
        fs::rename(&temporary, &path).map_err(|error| {
            drop(error);
            CheckpointStoreError::Io
        })?;
        self.records.insert((address, backend), checkpoints);
        Ok(())
    }
}

/// Computes the address of a lowered program, independent of item positions.
#[must_use]
#[inline]
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

/// Classifies semantic fields that require the HPSA-owned durable codec.
#[inline]
fn unsupported_reason(item: &crate::checkpoint::ItemCheckpoint) -> Option<UnsupportedPersistence>
{
    if matches!(item.ascription, Some(Ty::Value(ValueType::Data { .. }))) {
        return Some(UnsupportedPersistence::DataIdSerial);
    }
    if matches!(item.ascription, Some(Ty::Value(ValueType::Sealed(_)))) {
        return Some(UnsupportedPersistence::SealIdSerial);
    }
    if matches!(item.ascription, Some(Ty::Value(ValueType::Stk(..)))) {
        return Some(UnsupportedPersistence::OpaqueStack);
    }
    None
}
/// Encodes the currently supported canonical checkpoint subset.
///
/// # Errors
///
/// Returns [`CheckpointStoreError::UnsupportedPersistence`] when a checkpoint
/// contains semantic fields that require the HPSA-owned durable codec.
#[inline]
pub fn encode_checkpoints(checkpoints: &Checkpoints) -> Result<Vec<u8>, CheckpointStoreError>
{
    for item in &checkpoints.items {
        if let Some(reason) = unsupported_reason(item) {
            return Err(CheckpointStoreError::UnsupportedPersistence(reason));
        }
    }
    if checkpoints.items.is_empty() {
        return Ok(b"GCP\0\x01\0\0\0\0".to_vec());
    }
    Err(CheckpointStoreError::UnsupportedPersistence(
        UnsupportedPersistence::CheckpointItem,
    ))
}
/// # Errors
///
/// Returns [`CheckpointStoreError::Corrupt`] for a mismatched payload.
#[inline]
pub fn decode_checkpoints(bytes: &[u8]) -> Result<Checkpoints, CheckpointStoreError>
{
    if bytes == b"GCP\0\x01\0\0\0\0" {
        Ok(Checkpoints::default())
    }
    else {
        Err(CheckpointStoreError::Corrupt)
    }
}

/// Stores a checkpoint and notifies the optional extension observer.
///
/// # Errors
///
/// Returns the backing store error without notifying success.
#[inline]
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
/// # Errors
///
/// Returns a backing store error when loading fails.
#[inline]
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
    if address_of(program) != address {
        observer.invalidated(address);
        return Ok(None);
    }
    let restored = store.load(address, backend)?;
    if restored.is_none() {
        observer.invalidated(address);
    }
    Ok(restored)
}

/// Builds a file artifact with fixed-width identity fields and payload.
fn artifact_bytes(
    address: CheckpointAddress,
    backend: BackendArtifact,
    payload: &[u8],
) -> Vec<u8>
{
    let mut bytes = Vec::with_capacity(FILE_HEADER_LEN.saturating_add(payload.len()));
    bytes.extend_from_slice(b"GFILE\0\0\0");
    bytes.extend_from_slice(&1_u32.to_le_bytes());
    bytes.extend_from_slice(&address.0);
    bytes.extend_from_slice(&backend.0);
    bytes.extend_from_slice(payload);
    bytes
}

/// Encodes a digest as lowercase hexadecimal.
fn hex(bytes: [u8; 32]) -> String
{
    let mut output = String::with_capacity(64);
    for byte in bytes {
        if write!(output, "{byte:02x}").is_err() {
            return String::new();
        }
    }
    output
}

#[derive(Default)]
#[repr(transparent)]
/// Incremental hash input used to derive a BLAKE3 checkpoint address.
struct DigestHasher
{
    /// Canonical bytes accumulated for the digest.
    bytes: Vec<u8>,
}

impl Hasher for DigestHasher
{
    #[inline]
    fn finish(&self) -> u64
    {
        0
    }

    #[inline]
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
    #[inline]
    /// Finalizes the accumulated bytes as a BLAKE3 digest.
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
            self.stored = self.stored.saturating_add(1);
        }

        fn invalidated(
            &mut self,
            _address: CheckpointAddress,
        )
        {
            self.invalidated = self.invalidated.saturating_add(1);
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
        .unwrap();
        assert_eq!(observer.stored, 1);
        assert_eq!(
            restore(&mut store, &program, address, backend, &mut observer).unwrap(),
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
        .unwrap();
        assert_eq!(
            restore(&mut store, &program, address, backend_v2, &mut observer).unwrap(),
            None
        );
        assert_eq!(observer.invalidated, 1);
    }

    #[test]
    fn file_store_round_trips_empty_checkpoint()
    {
        let root = std::env::temp_dir().join(format!("gandr-checkpoint-{}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        let program = Program::default();
        let backend = BackendArtifact::from_bytes(b"backend");
        let mut first = FileCheckpointStore::open(&root).unwrap();
        let mut observer = Observer::default();
        let address = persist(
            &mut first,
            &program,
            backend,
            Checkpoints::default(),
            &mut observer,
        )
        .unwrap();
        let mut reopened = FileCheckpointStore::open(&root).unwrap();
        assert_eq!(
            restore(&mut reopened, &program, address, backend, &mut observer).unwrap(),
            Some(Checkpoints::default())
        );
        let other_backend = BackendArtifact::from_bytes(b"backend-v2");
        assert_eq!(
            restore(
                &mut reopened,
                &program,
                address,
                other_backend,
                &mut observer
            )
            .unwrap(),
            None
        );
        assert_eq!(observer.invalidated, 1);
        fs::remove_dir_all(root).unwrap();
    }
}
