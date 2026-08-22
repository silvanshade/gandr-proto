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
use core::hash::BuildHasher as _;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;

use crate::checkpoint::Checkpoints;
use crate::region::Program;

/// Canonical, process-independent persistence encoding.
mod codec;

/// Fixed byte length of the file-artifact header.
const FILE_HEADER_LEN: usize = 116;

/// How many private names a store tries before giving up.
///
/// A collision needs two live temporaries to draw one 64-bit nonce, so the
/// bound is a liveness guard against a pathological directory rather than an
/// expected path: exhausting it is reported as an I/O failure, never as a
/// silent overwrite.
const MAX_TEMPORARY_ATTEMPTS: u8 = 8;

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

/// Exact process-local or opaque forms outside the canonical codec.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedPersistence
{
    /// A declaration-order datatype serial is process-local.
    DataIdSerial,
    /// A mint-order opaque seal serial is process-local.
    SealIdSerial,
    /// A reified stack has no stable structural persistence contract.
    OpaqueStack,
    /// An inline effect signature is deliberately representation-opaque.
    OpaqueEffectSignature,
    /// The first-class package former has no byte representation in this codec.
    ///
    /// Distinct from the two serial forms above, and deliberately so: a package
    /// type and a packed module carry nothing process-local — their binder
    /// labels and witness types are ordinary structural data, and a seal serial
    /// inside one is already caught as [`Self::SealIdSerial`] on the way down.
    /// What is missing is the encoding, so classifying either as process-local
    /// would name the wrong obstacle.
    PackageFormer,
    /// A type-formation refusal has no byte representation in this codec.
    ///
    /// The refusal itself is ordinary structural data; what is missing is the
    /// encoding, so this is the same class as [`Self::PackageFormer`] rather
    /// than a process-local form.
    ///
    /// Declining here costs nothing a caller wanted. An item whose declared
    /// type is malformed is a refused item, so there is no validated typing to
    /// cache and adopt on a later resume -- the checkpoint this would encode
    /// is one nothing would ever be entitled to reuse.
    FormationRefusal,
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
    /// A well-formed level variable atom exceeded the bounded decode limit.
    LevelOffsetTooLarge
    {
        /// The rejected variable atom offset.
        offset: u64,
    },
    /// A valid semantic payload used a non-canonical byte representation.
    NonCanonical,
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
    /// # Contract
    /// - requires: `checkpoints` is the complete set for `address`.
    /// - ensures: on `Ok(())` the record is retrievable by `load` under the
    ///   same address and backend identity.
    /// - provides: the persistence half of checkpoint reuse.
    /// - fails: a typed [`CheckpointStoreError`] when encoding or the backing
    ///   store rejects the record; **a failed store leaves the store exactly as
    ///   it was**, with no partial, private, or temporary residue an observer
    ///   of the backing medium could see, and no record other than this
    ///   address's own touched whether it succeeds or fails.
    /// - panics: none.
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
    /// Canonical records keyed by content and backend identity.
    records: BTreeMap<(CheckpointAddress, BackendArtifact), Vec<u8>>,
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
        self.records
            .get(&(address, backend))
            .map(|bytes| codec::decode_checkpoints(bytes))
            .transpose()
    }

    #[inline]
    fn store(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
        checkpoints: Checkpoints,
    ) -> Result<(), CheckpointStoreError>
    {
        let bytes = codec::encode_checkpoints(&checkpoints)?;
        self.records.insert((address, backend), bytes);
        Ok(())
    }
}

/// A content-addressed file store with atomic replacement of complete records.
#[derive(Debug)]
#[repr(transparent)]
pub struct FileCheckpointStore
{
    /// Directory containing hash-named checkpoint artifacts.
    root: PathBuf,
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
        std::fs::create_dir_all(&root).map_err(|error| {
            drop(error);
            CheckpointStoreError::Io
        })?;
        Ok(Self { root })
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
        let bytes = match std::fs::read(path) {
            | Ok(bytes) => bytes,
            | Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            | Err(error) => {
                drop(error);
                return Err(CheckpointStoreError::Io);
            },
        };
        if bytes.len() < FILE_HEADER_LEN
            || bytes.get(.. 8) != Some(b"GFILE\0\0\0")
            || bytes.get(8 .. 12) != Some(&2_u32.to_le_bytes())
            || bytes.get(12 .. 44) != Some(&address.0)
        {
            return Err(CheckpointStoreError::Corrupt);
        }
        let payload_len_bytes: [u8; 8] = bytes
            .get(76 .. 84)
            .ok_or(CheckpointStoreError::Corrupt)?
            .try_into()
            .map_err(|_error| CheckpointStoreError::Corrupt)?;
        let payload_len = usize::try_from(u64::from_le_bytes(payload_len_bytes))
            .map_err(|_error| CheckpointStoreError::Corrupt)?;
        let payload = bytes
            .get(FILE_HEADER_LEN ..)
            .ok_or(CheckpointStoreError::Corrupt)?;
        if payload.len() != payload_len
            || bytes.get(84 .. FILE_HEADER_LEN) != Some(blake3::hash(payload).as_bytes())
        {
            return Err(CheckpointStoreError::Corrupt);
        }
        let decoded = codec::decode_checkpoints(payload)?;
        if bytes.get(44 .. 76) != Some(&backend.0) {
            return Ok(None);
        }
        Ok(Some(decoded))
    }

    /// # Adequacy
    /// - hypothesis: L3 for the failure-atomicity clause — the encode, the
    ///   staging write, and the publishing rename are separated by the one
    ///   ordinary condition that fails the rename after the record is written,
    ///   a directory occupying the record path, observed through the store
    ///   directory's exact entry set. Removing or inverting the rollback fails
    ///   that witness. **The staging name's safety is separated by a squatter
    ///   at the name a predictable implementation would choose**: making the
    ///   nonce constant fails the exclusivity witness on its own, and pairing
    ///   that with a truncating open fails its byte-identity assertion too. The
    ///   invariant clause is pinned under real contention on the record path
    ///   rather than argued.
    /// - hypothesis: **one property here is not separable by any test at this
    ///   nonce width, and is carried structurally instead** — that a collision
    ///   retries rather than overwriting. Provoking one needs two live
    ///   temporaries to draw the same 64-bit value, so no witness can reach the
    ///   retry arm; what the tests do establish is that the name is
    ///   unpredictable and that an existing file is never opened. The retry
    ///   itself rests on `create_new`'s exclusivity and the `AlreadyExists`
    ///   arm, which a reader checks. The partial-write leg is likewise
    ///   structural: the guard is armed before the first byte, and no reachable
    ///   input fails a write to a file just created. Its ordering — close the
    ///   handle, then let the guard unlink — is likewise unwitnessable here,
    ///   because the platform that refuses to unlink an open file is not the
    ///   one these tests run on; the explicit `drop` is what makes the
    ///   documented cleanup hold off Unix, and a comment at the call says so
    ///   because reverse-declaration drop order would silently undo it.
    /// - witness: `tests::a_failed_file_store_strands_no_temporary_in_the_record_directory`
    /// - witness: `tests::a_store_never_writes_through_a_file_it_did_not_create`
    /// - witness: `tests::concurrent_stores_of_one_address_leave_the_record_and_no_temporary`
    /// - witness: `tests::supported_nonempty_checkpoints_round_trip_in_memory_and_reopened_file`
    #[inline]
    fn store(
        &mut self,
        address: CheckpointAddress,
        backend: BackendArtifact,
        checkpoints: Checkpoints,
    ) -> Result<(), CheckpointStoreError>
    {
        let payload = codec::encode_checkpoints(&checkpoints)?;
        let artifact = artifact_bytes(address, backend, &payload)?;
        let record = TemporaryRecord::write(&self.root, address, &artifact)?;
        record.commit(&self.root.join(hex(address.0)))
    }
}

/// A fully written record awaiting the rename that publishes it.
///
/// The guard is what makes a store failure-atomic on the directory: the record
/// is written under a private name first, and only a successful rename disarms
/// the rollback. Every other exit removes the temporary, so a caller that reads
/// the store root after a rejected store sees exactly what it saw before.
struct TemporaryRecord
{
    /// The private path the complete record was written to.
    path: PathBuf,
    /// Whether the rename published the record, which disarms the rollback.
    published: RecordPublication,
}

/// Whether a [`TemporaryRecord`] reached its published name.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RecordPublication(bool);

impl RecordPublication
{
    /// The record is still private, so the rollback is armed.
    const PRIVATE: Self = Self(false);
    /// The record was renamed into place, so there is nothing to roll back.
    const PUBLISHED: Self = Self(true);
}

impl TemporaryRecord
{
    /// Writes `artifact` into a freshly created private file in `root`.
    ///
    /// **Exclusive creation is what makes the name safe, not the name itself.**
    /// The candidate carries the process id, the record address, and an
    /// operating-system-seeded nonce — the dependency-free recipe
    /// `gandr-runtime-effects` uses for its temporary directories — but a
    /// 64-bit nonce is a probabilistic name, and truncating whatever it
    /// happens to hit would let two stores of one address write through
    /// each other. So the file is opened with `create_new`, whose failure
    /// on an existing path is what turns a collision into a retry under a
    /// fresh nonce instead of a silent overwrite.
    ///
    /// # Contract
    /// - requires: `root` is an existing directory.
    /// - ensures: on success the complete artifact is on disk in a file this
    ///   call created, under a name no other live temporary holds, and the
    ///   returned guard removes it unless [`Self::commit`] publishes it. No
    ///   file that existed before the call is opened, truncated, or written.
    /// - provides: the staging half of the store's atomic replacement.
    /// - fails: [`CheckpointStoreError::Io`] when a candidate cannot be created
    ///   for a reason other than already existing, when the write fails — in
    ///   which case the handle is closed and the partly written file removed
    ///   before returning, in that order and on every platform — or when
    ///   [`MAX_TEMPORARY_ATTEMPTS`] consecutive candidates already exist.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`CheckpointStoreError::Io`] when no candidate can be created or
    /// the artifact cannot be written.
    fn write(
        root: &Path,
        address: CheckpointAddress,
        artifact: &[u8],
    ) -> Result<Self, CheckpointStoreError>
    {
        let process = std::process::id();
        for _ in 0 .. MAX_TEMPORARY_ATTEMPTS {
            // A fresh nonce per attempt: retrying the same candidate would spin
            // rather than resolve the collision it just observed.
            let nonce = std::collections::hash_map::RandomState::new().hash_one(process);
            let candidate = root.join(format!("{}.tmp-{process}-{nonce:016x}", hex(address.0)));
            let created = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate);
            let mut file = match created {
                | Ok(file) => file,
                // Somebody else holds this candidate. Draw another rather than
                // truncating a temporary that is not ours.
                | Err(ref error) if error.kind() == ErrorKind::AlreadyExists => continue,
                | Err(error) => {
                    drop(error);
                    return Err(CheckpointStoreError::Io);
                },
            };
            // Armed before the first byte, so a partial write is removed too.
            let record = Self {
                path: candidate,
                published: RecordPublication::PRIVATE,
            };
            let written = std::io::Write::write_all(&mut file, artifact)
                .and_then(|()| std::io::Write::flush(&mut file));
            // Close the handle here, explicitly, before either exit. Locals drop
            // in reverse declaration order, so leaving this to scope exit would
            // run the guard's `remove_file` while the file is still open — which
            // Unix permits and Windows refuses, making the documented cleanup
            // platform-dependent. The rename on the success path wants the
            // handle closed for the same reason.
            drop(file);
            let outcome = written.map_err(|error| {
                drop(error);
                CheckpointStoreError::Io
            });
            outcome?;
            return Ok(record);
        }
        Err(CheckpointStoreError::Io)
    }

    /// Renames the record onto `destination`, publishing it.
    ///
    /// # Contract
    /// - ensures: `Ok(())` exactly when the rename succeeded, after which the
    ///   record is the store's entry for its address and the rollback is
    ///   disarmed.
    /// - provides: the publishing half of the store's atomic replacement.
    /// - fails: [`CheckpointStoreError::Io`] when the rename fails, in which
    ///   case the temporary is removed and the directory is unchanged.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`CheckpointStoreError::Io`] when the rename fails.
    fn commit(
        mut self,
        destination: &Path,
    ) -> Result<(), CheckpointStoreError>
    {
        std::fs::rename(&self.path, destination).map_err(|error| {
            drop(error);
            CheckpointStoreError::Io
        })?;
        self.published = RecordPublication::PUBLISHED;
        Ok(())
    }
}

impl Drop for TemporaryRecord
{
    /// Removes the temporary unless the rename published it, so no failure path
    /// strands a private record in the caller's store directory.
    #[inline]
    fn drop(&mut self)
    {
        if self.published == RecordPublication::PRIVATE {
            drop(std::fs::remove_file(&self.path));
        }
    }
}

/// Computes the BLAKE3 address of a lowered program's canonical bytes.
///
/// # Errors
///
/// Returns the precise unsupported semantic form encountered while encoding.
#[inline]
pub fn address_of(program: &Program) -> Result<CheckpointAddress, CheckpointStoreError>
{
    let bytes = codec::encode_program(program)?;
    Ok(CheckpointAddress(*blake3::hash(&bytes).as_bytes()))
}

/// Encodes a checkpoint set into the process-independent canonical format.
///
/// # Errors
///
/// Returns the precise unsupported semantic form encountered while encoding.
#[inline]
pub fn encode_checkpoints(checkpoints: &Checkpoints) -> Result<Vec<u8>, CheckpointStoreError>
{
    codec::encode_checkpoints(checkpoints)
}

/// Decodes one complete canonical checkpoint payload.
///
/// # Errors
///
/// Returns [`CheckpointStoreError::Corrupt`] for malformed, truncated, or
/// trailing bytes, and [`CheckpointStoreError::NonCanonical`] when a valid
/// semantic payload has another byte representation.
#[inline]
pub fn decode_checkpoints(bytes: &[u8]) -> Result<Checkpoints, CheckpointStoreError>
{
    codec::decode_checkpoints(bytes)
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
    let address = address_of(program)?;
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
    if address_of(program)? != address {
        observer.invalidated(address);
        return Ok(None);
    }
    let restored = store.load(address, backend)?;
    if restored.is_none() {
        observer.invalidated(address);
    }
    Ok(restored)
}

/// Builds a file artifact with fixed-width identity and integrity fields.
fn artifact_bytes(
    address: CheckpointAddress,
    backend: BackendArtifact,
    payload: &[u8],
) -> Result<Vec<u8>, CheckpointStoreError>
{
    let payload_len =
        u64::try_from(payload.len()).map_err(|_error| CheckpointStoreError::Rejected)?;
    let mut bytes = Vec::with_capacity(FILE_HEADER_LEN.saturating_add(payload.len()));
    bytes.extend_from_slice(b"GFILE\0\0\0");
    bytes.extend_from_slice(&2_u32.to_le_bytes());
    bytes.extend_from_slice(&address.0);
    bytes.extend_from_slice(&backend.0);
    bytes.extend_from_slice(&payload_len.to_le_bytes());
    bytes.extend_from_slice(blake3::hash(payload).as_bytes());
    bytes.extend_from_slice(payload);
    Ok(bytes)
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

#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeMap;
    use alloc::rc::Rc;

    use gandr_core_term::boundary::EffectSignatureName;
    use gandr_core_term::boundary::OperationName;
    use gandr_core_term::classifier::Classifier;
    use gandr_core_term::classifier::GroundSort;
    use gandr_core_term::classifier::SortExpr;
    use gandr_core_term::effect::EffectOp;
    use gandr_core_term::effect::EffectSig;
    use gandr_core_term::error::TypeError;
    use gandr_core_term::error::text;
    use gandr_core_term::grade::Grade;
    use gandr_core_term::prim::NativePrim;
    use gandr_core_term::static_term::FamilyApp;
    use gandr_core_term::static_term::StaticArg;
    use gandr_core_term::static_term::StaticNeutral;
    use gandr_core_term::static_term::StaticTerm;
    use gandr_core_term::static_term::StaticVar;
    use gandr_core_term::syntax::Comp;
    use gandr_core_term::syntax::NumLit;
    use gandr_core_term::syntax::SplitMotive;
    use gandr_core_term::syntax::Stack;
    use gandr_core_term::syntax::Term;
    use gandr_core_term::syntax::Value;
    use gandr_core_term::syntax::WalkBase;
    use gandr_core_term::syntax::WalkMotive;
    use gandr_core_term::types::CompType;
    use gandr_core_term::types::DataId;
    use gandr_core_term::types::SealId;
    use gandr_core_term::types::Ty;
    use gandr_core_term::types::ValueType;
    use gandr_kernel_strata::Level;
    use gandr_kernel_strata::LevelVar;
    use gandr_kernel_strata::LevelVarIndex;

    use super::*;
    use crate::checkpoint::ItemCheckpoint;
    use crate::checkpoint::ItemTyping;
    use crate::checkpoint::checkpoint_program;
    use crate::footprint::Footprint;
    use crate::region::Item;

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

    fn program(values: &[i64]) -> Program
    {
        values
            .iter()
            .map(|value| Item::new(None, None, Term::Comp(Comp::ret(Value::int(*value)))))
            .collect()
    }

    fn checkpoint(
        term: Term,
        ascription: Option<Ty>,
    ) -> Checkpoints
    {
        Checkpoints {
            items: alloc::vec![ItemCheckpoint {
                name: None,
                ascription,
                term,
                footprint: Footprint::default(),
                typing: ItemTyping::Holey,
            }],
        }
    }

    fn parseable_noncanonical_checkpoints() -> alloc::vec::Vec<u8>
    {
        let mut checkpoints = checkpoint(Term::Value(Value::Unit), None);
        let footprint = &mut checkpoints.items[0].footprint;
        let _inserted = footprint.names.insert(String::from("a"));
        let _inserted = footprint.names.insert(String::from("b"));
        let mut noncanonical = encode_checkpoints(&checkpoints).unwrap();
        let encoded_b = [1, 0, 0, 0, b'b'];
        let offset = noncanonical
            .windows(encoded_b.len())
            .position(|window| window == encoded_b)
            .expect("encoded footprint label");
        let label_index = offset
            .checked_add(encoded_b.len())
            .and_then(|end| end.checked_sub(1))
            .expect("encoded footprint label index");
        noncanonical[label_index] = b'a';
        noncanonical
    }
    fn checkpoint_item(
        term: Term,
        ascription: Option<Ty>,
        typing: ItemTyping,
    ) -> ItemCheckpoint
    {
        ItemCheckpoint {
            name: None,
            ascription,
            term,
            footprint: Footprint::default(),
            typing,
        }
    }

    fn root(label: &str) -> PathBuf
    {
        std::env::temp_dir().join(format!("gandr-checkpoint-{label}-{}", std::process::id()))
    }

    #[test]
    fn independently_built_programs_have_identical_bytes_and_addresses()
    {
        let first = program(&[1, 2]);
        let second = program(&[1, 2]);
        assert_eq!(
            codec::encode_program(&first).unwrap(),
            codec::encode_program(&second).unwrap()
        );
        assert_eq!(address_of(&first).unwrap(), address_of(&second).unwrap());
    }

    #[test]
    fn meaningful_program_changes_and_source_order_change_identity()
    {
        let ordered = program(&[1, 2]);
        let reordered = program(&[2, 1]);
        let changed = program(&[1, 3]);
        assert_ne!(
            address_of(&ordered).unwrap(),
            address_of(&reordered).unwrap()
        );
        assert_ne!(address_of(&ordered).unwrap(), address_of(&changed).unwrap());

        let named = Program::new(alloc::vec![Item::new(
            Some(String::from("x")),
            Some(Ty::Value(ValueType::integer())),
            Term::Value(Value::int(1)),
        )]);
        let renamed = Program::new(alloc::vec![Item::new(
            Some(String::from("y")),
            Some(Ty::Value(ValueType::integer())),
            Term::Value(Value::int(1)),
        )]);
        assert_ne!(address_of(&named).unwrap(), address_of(&renamed).unwrap());
    }

    #[test]
    fn canonical_maps_and_supported_semantic_variants_round_trip()
    {
        let mut first_fields = BTreeMap::new();
        first_fields.insert(String::from("b"), Rc::new(Value::int(2)));
        first_fields.insert(String::from("a"), Rc::new(Value::int(1)));
        let mut second_fields = BTreeMap::new();
        second_fields.insert(String::from("a"), Rc::new(Value::int(1)));
        second_fields.insert(String::from("b"), Rc::new(Value::int(2)));
        let first_program = Program::new(alloc::vec![Item::new(
            None,
            None,
            Term::Value(Value::Record(first_fields)),
        )]);
        let second_program = Program::new(alloc::vec![Item::new(
            None,
            None,
            Term::Value(Value::Record(second_fields)),
        )]);
        assert_eq!(
            codec::encode_program(&first_program).unwrap(),
            codec::encode_program(&second_program).unwrap()
        );

        let pure = CompType::returner(ValueType::Unit);
        let arrow = CompType::arrow(ValueType::atom("A"), CompType::Unknown);
        let with = CompType::with(pure.clone(), arrow);
        let mut type_fields = BTreeMap::new();
        type_fields.insert(
            String::from("prod"),
            Rc::new(ValueType::prod(
                ValueType::Unit,
                ValueType::sum(ValueType::atom("L"), ValueType::atom("R")),
            )),
        );
        type_fields.insert(
            String::from("list"),
            Rc::new(ValueType::list(ValueType::atom("Element"))),
        );
        type_fields.insert(
            String::from("thunk"),
            Rc::new(ValueType::thunk(Grade::OMEGA, with.clone())),
        );
        type_fields.insert(
            String::from("stack"),
            Rc::new(ValueType::stk(pure, with.clone())),
        );
        type_fields.insert(
            String::from("path"),
            Rc::new(ValueType::path(
                ValueType::Unit,
                Value::Unit,
                Value::here(Value::Unit),
            )),
        );
        type_fields.insert(
            String::from("universe"),
            Rc::new(ValueType::universe(SortExpr::value(), Level::zero())),
        );
        type_fields.insert(
            String::from("sigma"),
            Rc::new(ValueType::sigma(
                ValueType::Unit,
                "x",
                ValueType::atom("Tail"),
            )),
        );
        type_fields.insert(String::from("unknown"), Rc::new(ValueType::Unknown));
        let rich_type = ValueType::Record(type_fields);

        let numbers = Value::List(alloc::vec![
            Rc::new(Value::Num(NumLit::U32(1))),
            Rc::new(Value::Num(NumLit::U64(2))),
            Rc::new(Value::Num(NumLit::I32(-3))),
            Rc::new(Value::Num(NumLit::I64(-4))),
            Rc::new(Value::Num(NumLit::F32(5.0_f32.to_bits()))),
            Rc::new(Value::Num(NumLit::F64(6.0_f64.to_bits()))),
        ]);
        let mut value_fields = BTreeMap::new();
        value_fields.insert(String::from("var"), Rc::new(Value::var("free")));
        value_fields.insert(String::from("unit"), Rc::new(Value::Unit));
        value_fields.insert(String::from("int"), Rc::new(Value::int(-1)));
        value_fields.insert(String::from("string"), Rc::new(Value::string("text")));
        value_fields.insert(String::from("numbers"), Rc::new(numbers));
        value_fields.insert(
            String::from("pair"),
            Rc::new(Value::pair(Value::Unit, Value::inj1(Value::Unit))),
        );
        value_fields.insert(String::from("inj2"), Rc::new(Value::inj2(Value::Unit)));
        value_fields.insert(
            String::from("thunk"),
            Rc::new(Value::thunk(
                Grade::fin(2_u64.into()),
                Comp::ret(Value::Unit),
            )),
        );
        value_fields.insert(
            String::from("annot"),
            Rc::new(Value::annot(Value::Unit, ValueType::Unit)),
        );
        value_fields.insert(String::from("here"), Rc::new(Value::here(Value::Unit)));
        let rich_value = Value::Record(value_fields);

        let terms = alloc::vec![
            Term::Value(rich_value),
            Term::Value(Value::hole(7)),
            Term::Comp(Comp::Abs(
                String::from("x"),
                Some(Rc::new(ValueType::Unit)),
                Rc::new(Comp::ret(Value::var("x"))),
            )),
            Term::Comp(Comp::app(
                Comp::lam_ann("x", ValueType::Unit, Comp::ret(Value::var("x"))),
                Value::Unit,
            )),
            Term::Comp(Comp::bind(
                Comp::ret(Value::Unit),
                "x",
                Comp::ret(Value::var("x")),
            )),
            Term::Comp(Comp::force(Value::thunk(
                Grade::ONE,
                Comp::ret(Value::Unit),
            ))),
            Term::Comp(Comp::case(
                Value::inj1(Value::Unit),
                "left",
                Comp::ret(Value::var("left")),
                "right",
                Comp::ret(Value::var("right")),
            )),
            Term::Comp(Comp::DataCase(Rc::new(Value::var("data")), alloc::vec![
                (String::from("zero"), Rc::new(Comp::ret(Value::Unit))),
                (String::from("one"), Rc::new(Comp::ret(Value::var("one")))),
            ],)),
            Term::Comp(Comp::list_case(
                Value::List(Vec::new()),
                Comp::ret(Value::Unit),
                "head",
                "tail",
                Comp::ret(Value::var("head")),
            )),
            Term::Comp(Comp::Split {
                scrut: Rc::new(Value::pair(Value::Unit, Value::Unit)),
                fst_name: String::from("fst"),
                snd_name: String::from("snd"),
                motive: Some(Box::new(SplitMotive::new(
                    "pair",
                    CompType::returner(ValueType::Unit),
                ))),
                body: Rc::new(Comp::ret(Value::var("fst"))),
            }),
            Term::Comp(Comp::record_proj(Value::Record(BTreeMap::new()), "field")),
            Term::Comp(Comp::with(Comp::ret(Value::Unit), Comp::ret(Value::Unit),)),
            Term::Comp(Comp::prj1(Comp::with(
                Comp::ret(Value::Unit),
                Comp::ret(Value::Unit),
            ))),
            Term::Comp(Comp::prj2(Comp::with(
                Comp::ret(Value::Unit),
                Comp::ret(Value::Unit),
            ))),
            Term::Comp(Comp::dup(Value::thunk(Grade::ONE, Comp::ret(Value::Unit),))),
            Term::Comp(Comp::drop(Value::thunk(
                Grade::ZERO,
                Comp::ret(Value::Unit),
            ))),
            Term::Comp(Comp::resume(Value::Unit, Comp::ret(Value::Unit))),
            Term::Comp(Comp::reset(Comp::ret(Value::Unit))),
            Term::Comp(Comp::shift("continuation", Comp::ret(Value::Unit))),
            Term::Comp(Comp::hole(8)),
            Term::Comp(Comp::Native {
                prim: NativePrim::Add,
                args: alloc::vec![Rc::new(Value::int(1)), Rc::new(Value::int(2))],
            }),
            Term::Comp(Comp::walk(
                Value::here(Value::Unit),
                WalkMotive::new("lhs", "rhs", "path", CompType::returner(ValueType::Unit),),
                WalkBase::new("base", Comp::ret(Value::var("base"))),
            )),
        ];

        let mut items: Vec<ItemCheckpoint> = terms
            .into_iter()
            .map(|term| {
                checkpoint_item(term, Some(Ty::Value(rich_type.clone())), ItemTyping::Holey)
            })
            .collect();
        let simple_term = Term::Value(Value::Unit);
        let value_ty = Ty::Value(ValueType::Unit);
        items.extend([
            checkpoint_item(simple_term.clone(), None, ItemTyping::Definition {
                name: String::from("definition"),
                ty: value_ty.clone(),
                bound: true,
            }),
            checkpoint_item(simple_term.clone(), None, ItemTyping::Expression {
                ty: Ty::Comp(with),
            }),
            checkpoint_item(simple_term.clone(), None, ItemTyping::TypeError {
                error: TypeError::type_mismatch(value_ty.clone(), Ty::Value(ValueType::Unknown)),
            }),
            checkpoint_item(simple_term.clone(), None, ItemTyping::TypeError {
                error: TypeError::ShapeMismatch {
                    expected: text::SHAPE_ARROW,
                    actual: value_ty,
                },
            }),
            checkpoint_item(simple_term.clone(), None, ItemTyping::TypeError {
                error: TypeError::StuckExpr {
                    expr: simple_term.clone(),
                    hint: text::ANNOTATE_INJECTION,
                },
            }),
            checkpoint_item(simple_term.clone(), None, ItemTyping::TypeError {
                error: TypeError::UnboundVariable {
                    name: String::from("missing"),
                },
            }),
            checkpoint_item(simple_term, None, ItemTyping::TypeError {
                error: TypeError::GradeError {
                    lower: Grade::fin(2_u64.into()),
                    upper: Grade::ONE,
                },
            }),
        ]);
        let mut footprint = Footprint::default();
        let _inserted = footprint.names.insert(String::from("z"));
        let _inserted = footprint.names.insert(String::from("a"));
        footprint.opaque = true;
        footprint.has_hole = true;
        if let Some(item) = items.first_mut() {
            item.footprint = footprint;
        }
        let checkpoints = Checkpoints { items };
        let bytes = encode_checkpoints(&checkpoints).unwrap();
        assert_eq!(decode_checkpoints(&bytes).unwrap(), checkpoints);
    }

    #[test]
    fn universe_sorts_and_levels_round_trip()
    {
        let variable = LevelVar::from(LevelVarIndex::from(37_u32));
        let level = Level::var(variable).succ().unwrap();
        let checkpoints = Checkpoints {
            items: alloc::vec![
                checkpoint_item(
                    Term::Value(Value::Unit),
                    Some(Ty::Value(ValueType::universe(
                        SortExpr::value(),
                        level.clone(),
                    ))),
                    ItemTyping::Holey,
                ),
                checkpoint_item(
                    Term::Value(Value::Unit),
                    Some(Ty::Value(ValueType::universe(
                        SortExpr::computation(),
                        level,
                    ))),
                    ItemTyping::Holey,
                ),
            ],
        };
        let bytes = encode_checkpoints(&checkpoints).unwrap();
        assert_eq!(decode_checkpoints(&bytes).unwrap(), checkpoints);
    }

    fn all_static_args_family() -> FamilyApp
    {
        let arguments = [
            StaticArg::Level(Level::zero()),
            StaticArg::Sort(SortExpr::computation()),
            StaticArg::Type(Rc::new(StaticTerm::Quote(Rc::new(Ty::Value(
                ValueType::atom("Quoted"),
            ))))),
            StaticArg::Value(Rc::new(Value::int(7))),
        ];
        let neutral = arguments.into_iter().fold(
            StaticNeutral::head(StaticVar::new("Family")),
            StaticNeutral::app,
        );
        FamilyApp::new(neutral, Classifier::new(GroundSort::Value, Level::zero()))
    }

    #[test]
    fn typed_static_family_arguments_round_trip()
    {
        let family = all_static_args_family();
        let checkpoints = Checkpoints {
            items: alloc::vec![checkpoint_item(
                Term::Value(Value::Unit),
                Some(Ty::Value(ValueType::family(family.clone()))),
                ItemTyping::Expression {
                    ty: Ty::Comp(CompType::family(family)),
                },
            )],
        };
        let bytes = encode_checkpoints(&checkpoints).unwrap();
        assert_eq!(decode_checkpoints(&bytes).unwrap(), checkpoints);
    }

    #[test]
    fn legacy_checkpoint_identity_is_rejected_after_static_family_move()
    {
        let checkpoints = checkpoint(Term::Value(Value::Unit), Some(Ty::Value(ValueType::Unit)));
        let mut legacy_bytes = encode_checkpoints(&checkpoints).unwrap();
        let legacy_magic = b"GCP\0\0\0\0\x02";
        legacy_bytes[.. legacy_magic.len()].copy_from_slice(legacy_magic);
        assert_eq!(
            decode_checkpoints(&legacy_bytes),
            Err(CheckpointStoreError::Corrupt)
        );
    }

    #[test]
    fn oversized_level_offset_is_refused_with_exact_error()
    {
        let variable = LevelVar::from(LevelVarIndex::from(37_u32));
        let checkpoints = checkpoint(
            Term::Value(Value::Unit),
            Some(Ty::Value(ValueType::universe(
                SortExpr::value(),
                Level::var(variable),
            ))),
        );
        let mut encoded = encode_checkpoints(&checkpoints).unwrap();
        let needle = [37_u8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let variable_offset = encoded
            .windows(needle.len())
            .position(|window| window == needle)
            .expect("universe variable atom");
        let offset_start = variable_offset.checked_add(4).expect("offset start");
        let offset_end = offset_start.checked_add(8).expect("offset end");
        let offset = encoded
            .get_mut(offset_start .. offset_end)
            .expect("offset bytes");
        offset.copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            decode_checkpoints(&encoded),
            Err(CheckpointStoreError::LevelOffsetTooLarge { offset: u64::MAX })
        );
    }

    /// The band-01-rung-07 native primitives round-trip through the checkpoint
    /// codec: each appended tag (42–48) encodes and decodes back to its own
    /// primitive.
    #[test]
    fn rung07_native_primitives_round_trip()
    {
        let items = [
            NativePrim::Div,
            NativePrim::Mod,
            NativePrim::Not,
            NativePrim::ListLength,
            NativePrim::ListAt,
            NativePrim::StringAppend,
            NativePrim::StringLength,
        ]
        .into_iter()
        .map(|prim| {
            checkpoint_item(
                Term::Comp(Comp::Native {
                    prim,
                    args: Vec::new(),
                }),
                None,
                ItemTyping::Holey,
            )
        })
        .collect();
        let checkpoints = Checkpoints { items };
        let bytes = encode_checkpoints(&checkpoints).unwrap();
        assert_eq!(decode_checkpoints(&bytes).unwrap(), checkpoints);
    }

    #[test]
    fn checkpoint_decoder_rejects_truncation_corruption_and_trailing_bytes()
    {
        let checkpoints = checkpoint_program(&program(&[1]));
        let original = encode_checkpoints(&checkpoints).unwrap();

        let mut truncated = original.clone();
        let _removed = truncated.pop();
        assert_eq!(
            decode_checkpoints(&truncated),
            Err(CheckpointStoreError::Corrupt)
        );

        let mut corrupt = original.clone();
        corrupt[8] = u8::MAX;
        assert_eq!(
            decode_checkpoints(&corrupt),
            Err(CheckpointStoreError::Corrupt)
        );

        let mut trailing = original;
        trailing.push(0);
        assert_eq!(
            decode_checkpoints(&trailing),
            Err(CheckpointStoreError::Corrupt)
        );
    }

    #[test]
    fn checkpoint_decoder_rejects_parseable_noncanonical_payload()
    {
        let noncanonical = parseable_noncanonical_checkpoints();

        assert_eq!(
            decode_checkpoints(&noncanonical),
            Err(CheckpointStoreError::NonCanonical)
        );
    }

    #[test]
    fn file_load_rejects_parseable_noncanonical_payload_after_integrity_checks()
    {
        let root = root("noncanonical");
        drop(std::fs::remove_dir_all(&root));
        let program = program(&[1]);
        let address = address_of(&program).unwrap();
        let backend = BackendArtifact::from_bytes(b"backend");
        let payload = parseable_noncanonical_checkpoints();
        let artifact = artifact_bytes(address, backend, &payload).unwrap();
        let mut store = FileCheckpointStore::open(&root).unwrap();
        std::fs::write(root.join(hex(address.0)), artifact).unwrap();

        assert_eq!(
            store.load(address, backend),
            Err(CheckpointStoreError::NonCanonical)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failed_file_store_strands_no_temporary_in_the_record_directory()
    {
        let root = root("store-io");
        drop(std::fs::remove_dir_all(&root));
        let program = program(&[1]);
        let checkpoints = checkpoint_program(&program);
        let backend = BackendArtifact::from_bytes(b"backend");
        let address = address_of(&program).unwrap();
        let mut store = FileCheckpointStore::open(&root).unwrap();
        // A directory at the record path is the one ordinary condition that
        // fails the rename *after* the temporary has been written, which is the
        // only ordering that can strand one.
        std::fs::create_dir_all(root.join(hex(address.0))).unwrap();

        assert_eq!(
            store.store(address, backend, checkpoints),
            Err(CheckpointStoreError::Io)
        );

        let mut remaining = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<alloc::vec::Vec<_>>();
        remaining.sort();
        assert_eq!(
            remaining,
            alloc::vec![hex(address.0)],
            "a rejected store leaves the record directory exactly as it found it"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Exclusive creation is what the staging name relies on, so a file already
    /// occupying the store root is never opened, truncated, or removed by a
    /// store — whatever candidate the nonce draws.
    #[test]
    fn a_store_never_writes_through_a_file_it_did_not_create()
    {
        let root = root("store-exclusive");
        drop(std::fs::remove_dir_all(&root));
        let program = program(&[1]);
        let checkpoints = checkpoint_program(&program);
        let backend = BackendArtifact::from_bytes(b"backend");
        let address = address_of(&program).unwrap();
        let mut store = FileCheckpointStore::open(&root).unwrap();
        // A squatter shaped exactly like a live temporary of the same address,
        // which is the shape a colliding candidate would hit.
        let squatter = root.join(format!(
            "{}.tmp-{}-0000000000000000",
            hex(address.0),
            std::process::id()
        ));
        std::fs::write(&squatter, b"another store's bytes").unwrap();

        store.store(address, backend, checkpoints.clone()).unwrap();

        assert_eq!(
            std::fs::read(&squatter).unwrap(),
            b"another store's bytes",
            "a store leaves a file it did not create byte-identical"
        );
        assert_eq!(
            store.load(address, backend).unwrap(),
            Some(checkpoints),
            "and still publishes its own record"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Concurrent stores of one address contend for the record path and for
    /// candidate names. The invariants that must hold whatever the
    /// interleaving: every call either succeeds or reports I/O, the
    /// published record is the one that was stored, and no private
    /// temporary survives.
    #[test]
    fn concurrent_stores_of_one_address_leave_the_record_and_no_temporary()
    {
        let root = root("store-concurrent");
        drop(std::fs::remove_dir_all(&root));
        let backend = BackendArtifact::from_bytes(b"backend");
        let address = address_of(&program(&[1, 2])).unwrap();
        drop(FileCheckpointStore::open(&root).unwrap());

        // A checkpoint set is `Rc`-carrying and so not `Send`; each thread
        // rebuilds the same program instead, which the crate's own byte-and-
        // address stability row pins as identical work.
        std::thread::scope(|scope| {
            for _ in 0_u8 .. 4_u8 {
                let root = root.clone();
                drop(scope.spawn(move || {
                    let checkpoints = checkpoint_program(&program(&[1, 2]));
                    let mut store = FileCheckpointStore::open(&root).unwrap();
                    for _ in 0_u8 .. 16_u8 {
                        let outcome = store.store(address, backend, checkpoints.clone());
                        assert!(
                            matches!(outcome, Ok(()) | Err(CheckpointStoreError::Io)),
                            "a store either publishes or reports I/O"
                        );
                    }
                }));
            }
        });

        let mut store = FileCheckpointStore::open(&root).unwrap();
        assert_eq!(
            store.load(address, backend).unwrap(),
            Some(checkpoint_program(&program(&[1, 2]))),
            "the published record is the one that was stored"
        );
        let leftovers = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".tmp-"))
            .collect::<alloc::vec::Vec<_>>();
        assert_eq!(
            leftovers,
            alloc::vec::Vec::<alloc::string::String>::new(),
            "no private temporary survives its store"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn supported_nonempty_checkpoints_round_trip_in_memory_and_reopened_file()
    {
        let root = root("supported");
        drop(std::fs::remove_dir_all(&root));
        let program = program(&[1, 2]);
        let checkpoints = checkpoint_program(&program);
        let backend = BackendArtifact::from_bytes(b"backend");
        let mut observer = Observer::default();

        let mut memory = MemoryCheckpointStore::default();
        let memory_address = persist(
            &mut memory,
            &program,
            backend,
            checkpoints.clone(),
            &mut observer,
        )
        .unwrap();
        assert_eq!(
            restore(
                &mut memory,
                &program,
                memory_address,
                backend,
                &mut observer
            )
            .unwrap(),
            Some(checkpoints.clone())
        );

        let mut first = FileCheckpointStore::open(&root).unwrap();
        let file_address = persist(
            &mut first,
            &program,
            backend,
            checkpoints.clone(),
            &mut observer,
        )
        .unwrap();
        assert_eq!(file_address, memory_address);
        drop(first);
        let mut reopened = FileCheckpointStore::open(&root).unwrap();
        assert_eq!(
            restore(
                &mut reopened,
                &program,
                file_address,
                backend,
                &mut observer
            )
            .unwrap(),
            Some(checkpoints)
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nested_process_local_and_opaque_forms_report_exact_errors()
    {
        let data = Value::Pair(
            Rc::new(Value::Unit),
            Rc::new(Value::Ctor {
                id: DataId::new(1_u64, "D"),
                tag: 0,
                payload: Rc::new(Value::Unit),
            }),
        );
        assert_eq!(
            encode_checkpoints(&checkpoint(Term::Value(data), None)),
            Err(CheckpointStoreError::UnsupportedPersistence(
                UnsupportedPersistence::DataIdSerial
            ))
        );

        let sealed = ValueType::prod(
            ValueType::Unit,
            ValueType::Sealed(SealId::new(1_u64, "module", "abstract")),
        );
        assert_eq!(
            encode_checkpoints(&checkpoint(
                Term::Value(Value::Unit),
                Some(Ty::Value(sealed))
            )),
            Err(CheckpointStoreError::UnsupportedPersistence(
                UnsupportedPersistence::SealIdSerial
            ))
        );

        let stack = Value::Pair(Rc::new(Value::Unit), Rc::new(Value::stk(Stack::empty())));
        assert_eq!(
            encode_checkpoints(&checkpoint(Term::Value(stack), None)),
            Err(CheckpointStoreError::UnsupportedPersistence(
                UnsupportedPersistence::OpaqueStack
            ))
        );

        let signature = EffectSig::new(EffectSignatureName::from("E"), alloc::vec![EffectOp::new(
            OperationName::from("op"),
            ValueType::Unit,
            ValueType::Unit
        )]);
        let effect = Comp::reset(Comp::perform(signature, "op", Value::Unit));
        assert_eq!(
            encode_checkpoints(&checkpoint(Term::Comp(effect), None)),
            Err(CheckpointStoreError::UnsupportedPersistence(
                UnsupportedPersistence::OpaqueEffectSignature
            ))
        );

        // The three package forms, each named separately: a packed module and a
        // package type report the missing encoding, while an elimination
        // reports the process-local atoms it minted.
        let packed = Value::Pair(
            Rc::new(Value::Unit),
            Rc::new(Value::pack([ValueType::Unit], Value::Unit)),
        );
        assert_eq!(
            encode_checkpoints(&checkpoint(Term::Value(packed), None)),
            Err(CheckpointStoreError::UnsupportedPersistence(
                UnsupportedPersistence::PackageFormer
            ))
        );

        let package = ValueType::prod(ValueType::Unit, ValueType::Package {
            grade: Grade::ONE,
            abstracts: alloc::vec![String::from("component")],
            payload: Rc::new(ValueType::Thunk(
                Grade::ONE,
                Rc::new(CompType::returner(ValueType::Unit)),
            )),
        });
        assert_eq!(
            encode_checkpoints(&checkpoint(
                Term::Value(Value::Unit),
                Some(Ty::Value(package))
            )),
            Err(CheckpointStoreError::UnsupportedPersistence(
                UnsupportedPersistence::PackageFormer
            ))
        );

        let opened = Comp::reset(Comp::unpack(
            Value::var("module"),
            ValueType::Unknown,
            [SealId::new(2_u64, "module", "component")],
            "opened",
            Comp::ret(Value::var("opened")),
        ));
        assert_eq!(
            encode_checkpoints(&checkpoint(Term::Comp(opened), None)),
            Err(CheckpointStoreError::UnsupportedPersistence(
                UnsupportedPersistence::SealIdSerial
            ))
        );
    }

    #[test]
    fn file_load_distinguishes_not_found_from_other_read_errors()
    {
        let root = root("io");
        drop(std::fs::remove_dir_all(&root));
        let mut store = FileCheckpointStore::open(&root).unwrap();
        let address = address_of(&Program::default()).unwrap();
        let backend = BackendArtifact::from_bytes(b"backend");
        assert_eq!(store.load(address, backend).unwrap(), None);

        let artifact_path = root.join(hex(address.0));
        std::fs::create_dir_all(&artifact_path).unwrap();
        assert_eq!(store.load(address, backend), Err(CheckpointStoreError::Io));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn file_load_rejects_path_mismatch_corruption_truncation_and_trailing_bytes()
    {
        let root = root("corrupt");
        drop(std::fs::remove_dir_all(&root));
        let program = program(&[1]);
        let checkpoints = checkpoint_program(&program);
        let backend = BackendArtifact::from_bytes(b"backend");
        let mut observer = Observer::default();
        let mut writer = FileCheckpointStore::open(&root).unwrap();
        let address = persist(&mut writer, &program, backend, checkpoints, &mut observer).unwrap();
        let path = root.join(hex(address.0));
        let original = std::fs::read(&path).unwrap();

        let mut mismatch = original.clone();
        mismatch[12] ^= 1;
        std::fs::write(&path, mismatch).unwrap();
        let mut reader = FileCheckpointStore::open(&root).unwrap();
        assert_eq!(
            reader.load(address, backend),
            Err(CheckpointStoreError::Corrupt)
        );

        let mut corrupt = original.clone();
        let payload_byte = corrupt.last_mut().unwrap();
        *payload_byte ^= 1;
        std::fs::write(&path, corrupt).unwrap();
        let mut reader = FileCheckpointStore::open(&root).unwrap();
        assert_eq!(
            reader.load(address, backend),
            Err(CheckpointStoreError::Corrupt)
        );

        let mut truncated = original.clone();
        let _removed = truncated.pop();
        std::fs::write(&path, truncated).unwrap();
        let mut reader = FileCheckpointStore::open(&root).unwrap();
        assert_eq!(
            reader.load(address, backend),
            Err(CheckpointStoreError::Corrupt)
        );

        let mut trailing = original;
        trailing.push(0);
        std::fs::write(&path, trailing).unwrap();
        let mut reader = FileCheckpointStore::open(&root).unwrap();
        assert_eq!(
            reader.load(address, backend),
            Err(CheckpointStoreError::Corrupt)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
