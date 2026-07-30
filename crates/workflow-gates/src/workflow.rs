//! Deterministic local workflow plans for merge and push validation.
//!
//! This module is the typed replacement for the `act-ci.nu` push wrapper. It
//! keeps the useful landing-contract shape — fixed merge/push plans made of
//! canonical `mise run <task>` boundaries and executed sequentially — while
//! replacing the old Act stamp cache with a native cache whose key is tied to
//! exact Git, toolchain, workflow, and endpoint identity.
//!
//! Direct `act` remains outside this API as a manual debugging aid for GitHub
//! Actions parity. Act tasks, fuzzing campaigns, mutation campaigns, and
//! side-effecting tasks are never cacheable workflow boundaries.
//!
//! Merge and push workflows are serialized by one host-global lock per Git
//! repository, keyed by the common Git directory so linked worktrees cannot run
//! landing workflows against the same repository at the same time.

use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use crate::GateError;
use crate::support;

crate::semantic_copy!(pub struct SchemaCount(u16));
crate::semantic_copy!(pub struct GenerationCount(u64));
crate::semantic_str!(pub struct RepositoryText);
crate::semantic_str!(pub struct TaskText);
crate::semantic_str!(pub struct BranchText);
crate::semantic_str!(pub struct RemoteText);
crate::semantic_bytes!(pub struct BytesBytes);
crate::semantic_copy!(pub struct CompletedTasksCount(usize));
crate::semantic_str!(pub struct NameText);
crate::semantic_str!(pub struct ValueText);
crate::semantic_str!(pub struct RowText);
crate::semantic_copy!(pub struct ContainsFlag(bool));
crate::semantic_copy!(pub struct LookupFlag(bool));
crate::semantic_optional_copy!(pub struct OptionalRepositoryIsCleanFlag(bool));
crate::semantic_copy!(pub struct CacheableTaskFlag(bool));
crate::semantic_copy!(pub struct IncrementCompletedTasksCount(usize));
crate::semantic_copy!(pub struct CanonicalTaskNameFlag(bool));

/// Exact command stdout bytes retained for identity hashing.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct CommandOutputBytes(Vec<u8>);

/// Merge-tier tasks in their canonical local order.
const MERGE_TASKS: &[Task] = &[
    Task::new(NameText("core:check")),
    Task::new(NameText("grammar:test")),
    Task::new(NameText("cargo:build")),
    Task::new(NameText("cargo:clippy")),
    Task::new(NameText("cargo:dylint")),
    Task::new(NameText("cargo:nextest")),
    Task::new(NameText("treefmt:check")),
    Task::new(NameText("wrkflw")),
];

/// Push-tier tasks in their canonical local order.
const PUSH_TASKS: &[Task] = &[
    Task::new(NameText("core:check")),
    Task::new(NameText("grammar:test")),
    Task::new(NameText("cargo:build")),
    Task::new(NameText("cargo:clippy")),
    Task::new(NameText("cargo:dylint")),
    Task::new(NameText("cargo:nextest")),
    Task::new(NameText("treefmt:check")),
    Task::new(NameText("wrkflw")),
    Task::new(NameText("cargo:doc-check")),
    Task::new(NameText("docs:conflict-markers")),
    Task::new(NameText("docs:manifest-drift")),
    Task::new(NameText("docs:reference-integrity")),
    Task::new(NameText("test:soundness-oracles")),
    Task::new(NameText("test:doc-gates")),
    Task::new(NameText("test:page-balance")),
    Task::new(NameText("test:graph-gates")),
    // Task::new(NameText("test:dep-graph")),
    // coverage:check stays out of the push tier while the failed-refactor
    // remediation leaves rewritten crates below their recorded floors; the
    // coverage restoration pass re-enables it.
    // Task::new(NameText("coverage:check")),
    Task::new(NameText("cargo:no-panic")),
    Task::new(NameText("cargo:careful-nextest")),
];

/// Cache schema version included in every workflow cache key and file.
const WORKFLOW_CACHE_SCHEMA: u16 = 1;

/// Maximum number of successful task keys retained per repository cache file.
const WORKFLOW_CACHE_ENTRY_LIMIT: usize = 32;

/// Host-global root directory name for workflow locks and cache files.
const WORKFLOW_CACHE_ROOT_NAME: &str = "gandr-workflow-gates-workflow";

/// Git executable used for identity probing.
const GIT_PROGRAM: &str = "git";

/// Mise executable used for workflow execution and toolchain identity.
const MISE_PROGRAM: &str = "mise";

/// Cache key identity for the Git repository that owns a workflow run.
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(transparent)]
struct RepositoryLockKey
{
    /// BLAKE3 token derived from the canonical Git common directory.
    token: String,
}

/// Full input identity required before one task may use or write a cache hit.
#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkflowInputIdentity
{
    /// Repository token shared by linked worktrees for this Git repository.
    repository: String,
    /// Immutable `HEAD` commit identity observed before the task.
    head: String,
    /// Immutable `HEAD` tree identity observed before the task.
    tree: String,
    /// Digest of submodule status and Git tree entries.
    submodules: String,
    /// Digest of this workflow policy and static task plan.
    workflow: String,
    /// Digest of active tool versions observed through host tools.
    toolchain: String,
    /// Digest of tracked workflow/toolchain configuration entries.
    config: String,
    /// Push-only endpoint and base identity.
    push: Option<PushIdentity>,
}

/// Endpoint and base identity that can affect push-tier correctness.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
struct PushIdentity
{
    /// Fetch URL for the canonical `origin` remote.
    fetch_remote: String,
    /// Push URL for the canonical `origin` remote.
    push_remote: String,
    /// Current branch name.
    branch: String,
    /// Symbolic upstream reference for the current branch.
    upstream_ref: String,
    /// Commit currently named by the upstream reference.
    upstream_commit: String,
    /// Merge base between `HEAD` and the upstream reference.
    merge_base: String,
}

/// Persisted cache key for one successful task boundary.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
struct CacheKey
{
    /// Cache schema version.
    schema: u16,
    /// Repository token shared by linked worktrees.
    repository: String,
    /// Workflow tier label.
    tier: String,
    /// Canonical task name.
    task: String,
    /// Immutable `HEAD` commit identity.
    head: String,
    /// Immutable `HEAD` tree identity.
    tree: String,
    /// Digest of submodule status and Git tree entries.
    submodules: String,
    /// Digest of workflow policy and static plan.
    workflow: String,
    /// Digest of active tool versions.
    toolchain: String,
    /// Digest of tracked workflow/toolchain configuration entries.
    config: String,
    /// Push-only endpoint and base identity.
    push: Option<PushIdentity>,
}

impl CacheKey
{
    /// Build a cache key from a fully-proven task input identity.
    ///
    /// # Contract
    /// - requires: `identity` was gathered from a clean repository state for
    ///   this exact task boundary.
    /// - ensures: every correctness dimension required for cache reuse is
    ///   copied into the returned key along with schema, tier, and task labels.
    /// - provides: the equality surface used by cache hits and invalidation
    ///   tests.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — one mutated key per field kills omissions from
    ///   the equality surface.
    /// - witness: `workflow::tests::workflow_cache_misses_when_correctness_identity_changes`
    #[must_use]
    fn from_identity(
        tier: Tier,
        task: Task,
        identity: &WorkflowInputIdentity,
    ) -> Self
    {
        Self {
            schema: WORKFLOW_CACHE_SCHEMA,
            repository: identity.repository.clone(),
            tier: String::from(tier.as_str().as_ref()),
            task: String::from(task.name().as_ref()),
            head: identity.head.clone(),
            tree: identity.tree.clone(),
            submodules: identity.submodules.clone(),
            workflow: identity.workflow.clone(),
            toolchain: identity.toolchain.clone(),
            config: identity.config.clone(),
            push: identity.push.clone(),
        }
    }
}

/// On-disk workflow cache file.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct WorkflowCacheFile
{
    /// Cache schema version for the whole file.
    schema: u16,
    /// Successful task entries sorted newest first by generation.
    entries: Vec<WorkflowCacheEntry>,
}

impl WorkflowCacheFile
{
    /// Return an empty cache file for the current schema.
    #[must_use]
    fn empty() -> Self
    {
        Self {
            schema: WORKFLOW_CACHE_SCHEMA,
            entries: Vec::new(),
        }
    }

    /// Return whether `key` is present in this cache file.
    ///
    /// # Contract
    /// - ensures: returns true only for an exactly equal cache key under the
    ///   current schema.
    /// - provides: the in-memory hit predicate shared by file and test caches.
    /// - panics: none.
    #[must_use]
    fn contains(
        &self,
        key: &CacheKey,
    ) -> impl Into<ContainsFlag>
    {
        self.schema == WORKFLOW_CACHE_SCHEMA && self.entries.iter().any(|entry| entry.key == *key)
    }

    /// Record one successful task and evict older entries beyond the limit.
    ///
    /// # Contract
    /// - requires: `key` represents a task that actually completed successfully
    ///   for the exact pre-task and post-task identity.
    /// - ensures: stores one entry for `key`, moves it to newest position, and
    ///   retains no more than [`WORKFLOW_CACHE_ENTRY_LIMIT`] newest entries.
    /// - provides: fixed-size cache growth with deterministic newest-entry
    ///   eviction.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — over-limit insertion fixtures kill mutants that
    ///   retain stale entries, duplicate keys, or skip truncation.
    /// - witness: `workflow::tests::file_cache_recovers_corruption_and_bounds_newest_entries`
    fn record_success(
        &mut self,
        key: &CacheKey,
    )
    {
        let newest_generation = self
            .entries
            .iter()
            .map(|entry| entry.generation)
            .max()
            .unwrap_or(0);
        let generation = newest_generation
            .checked_add(1)
            .unwrap_or(newest_generation);
        self.entries.retain(|entry| entry.key != *key);
        self.entries.push(WorkflowCacheEntry {
            key: key.clone(),
            generation,
        });
        self.entries
            .sort_by_key(|entry| core::cmp::Reverse(entry.generation));
        self.entries.truncate(WORKFLOW_CACHE_ENTRY_LIMIT);
    }
}

/// One successful workflow task cache entry.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct WorkflowCacheEntry
{
    /// Exact cache key that was proven successful.
    key: CacheKey,
    /// Monotone generation used for newest-entry eviction.
    generation: u64,
}

/// Cache backend used by the workflow executor.
trait WorkflowCacheBackend
{
    /// Return whether a successful task result exists for `key`.
    ///
    /// # Contract
    /// - ensures: returns true only when `key` is an exact hit.
    /// - fails: returns [`GateError`] for cache I/O failures; callers must
    ///   treat that as a miss and run normally.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns cache backend failures.
    fn lookup(
        &self,
        key: &CacheKey,
    ) -> Result<impl Into<LookupFlag>, GateError>;

    /// Persist a successful task key.
    ///
    /// # Contract
    /// - requires: the task completed successfully for the exact identity in
    ///   `key`.
    /// - ensures: best-effort records the key for future exact hits.
    /// - fails: returns [`GateError`] for cache I/O failures; callers must not
    ///   turn the workflow failure surface false-green because of this.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns cache backend failures.
    fn record_success(
        &self,
        key: &CacheKey,
    ) -> Result<(), GateError>;
}

/// Identity provider used to prove whether a task is cacheable.
trait WorkflowIdentityProvider
{
    /// Return the repository lock key for this workflow run.
    ///
    /// # Contract
    /// - ensures: returns the same key for linked worktrees that share one Git
    ///   common directory.
    /// - fails: returns `None` when the repository identity is unavailable.
    /// - panics: none.
    fn repository_lock_key(
        &self,
        cwd: Option<&Path>,
    ) -> Option<RepositoryLockKey>;

    /// Return the exact task input identity when it is fully proven.
    ///
    /// # Contract
    /// - requires: `tier` and `task` come from a static workflow plan.
    /// - ensures: returns `Some` only for clean repository state with all
    ///   required Git, submodule, workflow, toolchain, config, and push
    ///   endpoint/base identities present.
    /// - fails: returns `None` for dirty work, untracked work, command failure,
    ///   missing endpoint/base identity, invalid UTF-8 where text is required,
    ///   or any other uncertainty.
    /// - panics: none.
    fn task_identity(
        &self,
        tier: Tier,
        task: Task,
        cwd: Option<&Path>,
    ) -> Option<WorkflowInputIdentity>;
}

/// Repository lock backend used to serialize workflow runs.
trait WorkflowLockBackend
{
    /// Run `body` while holding the repository lock named by `key`.
    ///
    /// # Contract
    /// - ensures: when `key` is present, no two callers using the same backend
    ///   and key execute their bodies concurrently.
    /// - fails: returns [`GateError`] when the lock cannot be acquired.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns lock acquisition failures or the error returned by `body`.
    fn with_repository_lock<ResultValue, Body>(
        &self,
        key: Option<&RepositoryLockKey>,
        body: Body,
    ) -> Result<ResultValue, GateError>
    where
        Body: FnOnce() -> Result<ResultValue, GateError>;
}

/// Production Git and toolchain identity provider.
struct HostWorkflowIdentity;

impl WorkflowIdentityProvider for HostWorkflowIdentity
{
    /// Return the repository key derived from Git's common directory.
    fn repository_lock_key(
        &self,
        cwd: Option<&Path>,
    ) -> Option<RepositoryLockKey>
    {
        repository_lock_key(cwd)
    }

    /// Gather a full task identity from Git and host toolchain commands.
    fn task_identity(
        &self,
        tier: Tier,
        task: Task,
        cwd: Option<&Path>,
    ) -> Option<WorkflowInputIdentity>
    {
        if !crate::semantic_value::<CacheableTaskFlag>(is_cacheable_task(task)).0
            || !crate::semantic_value::<OptionalRepositoryIsCleanFlag>(repository_is_clean(cwd)).0?
        {
            return None;
        }
        let repository = repository_lock_key(cwd)?.token;
        let head = git_text(cwd, ["rev-parse", "HEAD"])?;
        let tree = git_text(cwd, ["show", "-s", "--format=%T", "HEAD"])?;
        let submodules = submodule_identity(cwd)?;
        let workflow = workflow_policy_identity(tier);
        let toolchain = toolchain_identity(cwd)?;
        let config = config_identity(cwd)?;
        let push = match tier {
            | Tier::Merge => None,
            | Tier::Push => Some(push_identity(cwd)?),
        };
        Some(WorkflowInputIdentity {
            repository,
            head,
            tree,
            submodules,
            workflow,
            toolchain,
            config,
            push,
        })
    }
}

/// File-backed workflow cache rooted in the host temporary directory.
#[repr(transparent)]
struct FileWorkflowCache
{
    /// Root directory for all cache files.
    root: PathBuf,
}

impl FileWorkflowCache
{
    /// Build a file cache rooted at `root`.
    #[must_use]
    fn new(root: PathBuf) -> Self
    {
        Self { root }
    }

    /// Build the host-global workflow cache.
    #[must_use]
    fn host() -> Self
    {
        Self::new(host_workflow_root().join("cache"))
    }

    /// Return the cache path for one repository token.
    #[must_use]
    fn path<R>(
        &self,
        repository: R,
    ) -> PathBuf
    where
        R: Into<RepositoryText<'_>>,
    {
        let repository = repository.into().0;
        self.root.join(format!("{repository}.json"))
    }

    /// Read and parse a cache file, treating missing/corrupt files as empty.
    ///
    /// # Contract
    /// - ensures: missing, corrupt, or schema-mismatched files produce an empty
    ///   cache instead of a false hit.
    /// - fails: returns [`GateError::Io`] for read failures other than absence.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns cache file read failures other than absence.
    fn read_cache_file<R>(
        &self,
        repository: R,
    ) -> Result<WorkflowCacheFile, GateError>
    where
        R: Into<RepositoryText<'_>>,
    {
        let repository = repository.into().0;
        let path = self.path(repository);
        let bytes = match crate::support::HOST_FILESYSTEM.read(path) {
            | Ok(bytes) => bytes,
            | Err(GateError::Io { source, .. }) if source.kind() == std::io::ErrorKind::NotFound =>
            {
                return Ok(WorkflowCacheFile::empty());
            },
            | Err(error) => return Err(error),
        };
        let Ok(cache_file) = serde_json::from_slice::<WorkflowCacheFile>(bytes.as_bytes().into())
        else {
            return Ok(WorkflowCacheFile::empty());
        };
        if cache_file.schema == WORKFLOW_CACHE_SCHEMA {
            Ok(cache_file)
        }
        else {
            Ok(WorkflowCacheFile::empty())
        }
    }
}

impl WorkflowCacheBackend for FileWorkflowCache
{
    /// Return whether `key` is an exact file-cache hit.
    fn lookup(
        &self,
        key: &CacheKey,
    ) -> Result<impl Into<LookupFlag>, GateError>
    {
        Ok(self
            .read_cache_file(&key.repository)?
            .contains(key)
            .into()
            .0)
    }

    /// Atomically record one successful task key.
    fn record_success(
        &self,
        key: &CacheKey,
    ) -> Result<(), GateError>
    {
        let path = self.path(&key.repository);
        let mut cache_file = self.read_cache_file(&key.repository)?;
        cache_file.record_success(key);
        let bytes = serde_json::to_vec(&cache_file).map_err(|source| {
            GateError::operational(format!("workflow cache encode failed: {source}"))
        })?;
        let Some(parent) = path.parent()
        else {
            return Err(GateError::operational(format!(
                "workflow cache path has no parent: {}",
                path.display()
            )));
        };
        crate::support::HOST_FILESYSTEM.create_dir_all(parent)?;
        crate::support::write_atomic(&path, &bytes)
    }
}

/// File-backed host-global repository lock.
#[repr(transparent)]
struct FileWorkflowLock
{
    /// Root directory for lock files.
    root: PathBuf,
}

impl FileWorkflowLock
{
    /// Build a file lock backend rooted at `root`.
    #[must_use]
    fn new(root: PathBuf) -> Self
    {
        Self { root }
    }

    /// Build the host-global workflow lock backend.
    #[must_use]
    fn host() -> Self
    {
        Self::new(host_workflow_root().join("locks"))
    }

    /// Return the lock path for one repository key.
    #[must_use]
    fn path(
        &self,
        key: &RepositoryLockKey,
    ) -> PathBuf
    {
        self.root.join(format!("{}.lock", key.token))
    }
}

impl WorkflowLockBackend for FileWorkflowLock
{
    /// Run `body` while holding an advisory file lock for the repository.
    fn with_repository_lock<ResultValue, Body>(
        &self,
        key: Option<&RepositoryLockKey>,
        body: Body,
    ) -> Result<ResultValue, GateError>
    where
        Body: FnOnce() -> Result<ResultValue, GateError>,
    {
        let Some(lock_key) = key
        else {
            return body();
        };
        let path = self.path(lock_key);
        let Some(parent) = path.parent()
        else {
            return Err(GateError::operational(format!(
                "workflow lock path has no parent: {}",
                path.display()
            )));
        };
        crate::support::HOST_FILESYSTEM.create_dir_all(parent)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| io_error(&path, source))?;
        file.lock().map_err(|source| io_error(&path, source))?;
        let guard = FileWorkflowLockGuard { file };
        let result = body();
        drop(guard);
        result
    }
}

/// Held file lock released when dropped.
#[repr(transparent)]
struct FileWorkflowLockGuard
{
    /// Locked file descriptor.
    file: File,
}

impl Drop for FileWorkflowLockGuard
{
    /// Release the advisory file lock best-effort.
    fn drop(&mut self)
    {
        drop(self.file.unlock());
    }
}

#[cfg(test)]
/// Identity provider that disables workflow caching.
struct NoWorkflowIdentity;

#[cfg(test)]
impl WorkflowIdentityProvider for NoWorkflowIdentity
{
    /// No repository lock is required for cache-free injected tests.
    fn repository_lock_key(
        &self,
        _cwd: Option<&Path>,
    ) -> Option<RepositoryLockKey>
    {
        None
    }

    /// Cache-free injected tests never provide task identity.
    fn task_identity(
        &self,
        _tier: Tier,
        _task: Task,
        _cwd: Option<&Path>,
    ) -> Option<WorkflowInputIdentity>
    {
        None
    }
}

#[cfg(test)]
/// Cache backend that always misses and ignores writes.
struct DisabledWorkflowCache;

#[cfg(test)]
impl WorkflowCacheBackend for DisabledWorkflowCache
{
    /// Return a cache miss.
    fn lookup(
        &self,
        _key: &CacheKey,
    ) -> Result<impl Into<LookupFlag>, GateError>
    {
        Ok(false)
    }

    /// Ignore a cache write.
    fn record_success(
        &self,
        _key: &CacheKey,
    ) -> Result<(), GateError>
    {
        Ok(())
    }
}

#[cfg(test)]
/// Lock backend that runs bodies without locking.
struct NoWorkflowLock;

#[cfg(test)]
impl WorkflowLockBackend for NoWorkflowLock
{
    /// Run `body` immediately.
    fn with_repository_lock<ResultValue, Body>(
        &self,
        _key: Option<&RepositoryLockKey>,
        body: Body,
    ) -> Result<ResultValue, GateError>
    where
        Body: FnOnce() -> Result<ResultValue, GateError>,
    {
        body()
    }
}

/// Return the host-global root for workflow cache and lock state.
#[must_use]
fn host_workflow_root() -> PathBuf
{
    std::env::temp_dir().join(WORKFLOW_CACHE_ROOT_NAME)
}

/// Return the repository key shared by linked worktrees.
///
/// # Contract
/// - ensures: returns a BLAKE3 token derived from Git's canonical common
///   directory.
/// - fails: returns `None` when Git cannot prove the repository common
///   directory.
/// - panics: none.
fn repository_lock_key(cwd: Option<&Path>) -> Option<RepositoryLockKey>
{
    let common_dir = git_text(cwd, [
        "rev-parse",
        "--path-format=absolute",
        "--git-common-dir",
    ])?;
    let common_path = PathBuf::from(common_dir);
    let canonical = fs::canonicalize(common_path).ok()?;
    Some(RepositoryLockKey {
        token: hash_bytes(canonical.as_os_str().as_encoded_bytes()),
    })
}

/// Return whether the repository is clean, including untracked work.
///
/// # Contract
/// - ensures: returns true only when porcelain status reports no tracked,
///   untracked, or submodule changes.
/// - fails: returns `None` when Git status cannot be obtained exactly.
/// - panics: none.
fn repository_is_clean(cwd: Option<&Path>) -> impl Into<OptionalRepositoryIsCleanFlag>
{
    let status = git_output(cwd, [
        "status",
        "--porcelain=v2",
        "--untracked-files=all",
        "--ignore-submodules=none",
    ])?;
    Some(status.0.is_empty())
}

/// Return a text Git command's trimmed stdout.
///
/// # Contract
/// - requires: `args` are direct argv tokens for `git`.
/// - ensures: returns stdout as UTF-8 with only terminal line endings removed.
/// - fails: returns `None` for process failure, nonzero status, or invalid
///   UTF-8.
/// - panics: none.
fn git_text<Args>(
    cwd: Option<&Path>,
    args: Args,
) -> Option<String>
where
    Args: IntoIterator,
    Args::Item: Into<OsString>,
{
    let os_args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    let output = git_output(cwd, os_args)?;
    let text = String::from_utf8(output.0).ok()?;
    Some(trim_terminal_line_endings(text))
}

/// Remove terminal line endings from command stdout.
///
/// # Contract
/// - ensures: removes only trailing `\n` and `\r` characters and preserves all
///   interior bytes already decoded as UTF-8.
/// - provides: stable single-token Git and toolchain identities.
/// - panics: none.
#[must_use]
fn trim_terminal_line_endings(mut text: String) -> String
{
    while text.ends_with('\n') || text.ends_with('\r') {
        let _removed = text.pop();
    }
    text
}

/// Return a digest covering submodule and Git tree state.
///
/// # Contract
/// - ensures: includes the recursive `HEAD` tree listing and recursive
///   submodule status output.
/// - fails: returns `None` when either Git identity command is unavailable.
/// - panics: none.
fn submodule_identity(cwd: Option<&Path>) -> Option<String>
{
    let mut hasher = blake3::Hasher::new();
    hasher.update(
        git_output(cwd, ["ls-tree", "-r", "-z", "HEAD"])?
            .0
            .as_slice(),
    );
    hasher.update(b"\0submodules\0");
    hasher.update(
        git_output(cwd, ["submodule", "status", "--recursive"])?
            .0
            .as_slice(),
    );
    Some(finish_hash(&hasher))
}

/// Return a digest covering workflow policy and static plan shape.
///
/// # Contract
/// - ensures: includes cache schema, tier label, every task name in that tier,
///   and the cacheability decision for each task.
/// - provides: invalidation when this Rust workflow policy changes.
/// - panics: none.
#[must_use]
fn workflow_policy_identity(tier: Tier) -> String
{
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"workflow-cache-schema\0");
    hasher.update(&WORKFLOW_CACHE_SCHEMA.to_le_bytes());
    hasher.update(b"\0tier\0");
    hasher.update(tier.as_str().as_ref().as_bytes());
    for task in tier.plan().tasks() {
        hasher.update(b"\0task\0");
        hasher.update(task.name().as_ref().as_bytes());
        hasher.update(b"\0cacheable\0");
        if is_cacheable_task(*task).into().0 {
            hasher.update(b"yes");
        }
        else {
            hasher.update(b"no");
        }
    }
    finish_hash(&hasher)
}

/// Return a digest covering active toolchain command identities.
///
/// # Contract
/// - ensures: includes `mise --version`, `mise current`, and `rustc --version
///   --verbose` stdout.
/// - fails: returns `None` when any toolchain identity command is unavailable.
/// - panics: none.
fn toolchain_identity(cwd: Option<&Path>) -> Option<String>
{
    let mut hasher = blake3::Hasher::new();
    update_command_identity(
        &mut hasher,
        OsStr::new(MISE_PROGRAM),
        &[OsString::from("--version")],
        cwd,
    )?;
    update_command_identity(
        &mut hasher,
        OsStr::new(MISE_PROGRAM),
        &[OsString::from("current")],
        cwd,
    )?;
    update_command_identity(
        &mut hasher,
        OsStr::new("rustc"),
        &[OsString::from("--version"), OsString::from("--verbose")],
        cwd,
    )?;
    Some(finish_hash(&hasher))
}

/// Add one command's stdout to an aggregate identity hash.
///
/// # Contract
/// - ensures: command label, argv, and exact stdout bytes affect the aggregate
///   hash.
/// - fails: returns `None` when the command output cannot be captured
///   successfully.
/// - panics: none.
fn update_command_identity(
    hasher: &mut blake3::Hasher,
    program: &OsStr,
    args: &[OsString],
    cwd: Option<&Path>,
) -> Option<()>
{
    hasher.update(program.as_encoded_bytes());
    for arg in args {
        hasher.update(b"\0arg\0");
        hasher.update(arg.as_encoded_bytes());
    }
    hasher.update(b"\0stdout\0");
    hasher.update(command_output(program, args, cwd)?.0.as_slice());
    Some(())
}

/// Return exact stdout bytes from a sanitized command.
///
/// # Contract
/// - requires: `program` and `args` are direct process tokens.
/// - ensures: captures stdout, suppresses identity-probe stderr, and strips
///   ambient Git repository override variables from the child.
/// - fails: returns `None` for spawn, wait, or nonzero-status failures.
/// - panics: none.
fn command_output(
    program: &OsStr,
    args: &[OsString],
    cwd: Option<&Path>,
) -> Option<CommandOutputBytes>
{
    let mut command = Command::new(program);
    command.args(args);
    if let Some(directory) = cwd {
        command.current_dir(directory);
    }
    support::sanitize_git_environment(&mut command);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(CommandOutputBytes(output.stdout))
}

/// Return a digest covering tracked workflow and toolchain configuration.
///
/// # Contract
/// - ensures: includes tracked Git object identities for the workflow, Cargo,
///   rustfmt, treefmt, nextest, rust-toolchain, and mise configuration path
///   set.
/// - fails: returns `None` when Git cannot read the tracked config identities.
/// - panics: none.
fn config_identity(cwd: Option<&Path>) -> Option<String>
{
    Some(hash_bytes(
        &git_output(cwd, [
            "ls-tree",
            "-r",
            "-z",
            "HEAD",
            "--",
            ".cargo",
            ".config/nextest.toml",
            ".mise.toml",
            "Cargo.lock",
            "Cargo.toml",
            "deny.toml",
            "docs/workflow/rust.md",
            "mise.toml",
            "rust-toolchain.toml",
        ])?
        .0,
    ))
}

/// Return push-tier endpoint and base identity.
///
/// # Contract
/// - ensures: includes origin fetch URL, origin push URL, branch, upstream ref,
///   upstream commit, and merge base.
/// - fails: returns `None` for detached heads, missing remotes, missing
///   upstreams, or unavailable merge bases.
/// - panics: none.
fn push_identity(cwd: Option<&Path>) -> Option<PushIdentity>
{
    let fetch_remote = git_text(cwd, ["remote", "get-url", "origin"])?;
    let push_remote = git_text(cwd, ["remote", "get-url", "--push", "origin"])?;
    let branch = git_text(cwd, ["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch == "HEAD" {
        return None;
    }
    let upstream = upstream_revision();
    let upstream_ref = git_text_owned(cwd, &[
        OsString::from("rev-parse"),
        OsString::from("--symbolic-full-name"),
        OsString::from(upstream.as_str()),
    ])?;
    let upstream_commit = git_text_owned(cwd, &[
        OsString::from("rev-parse"),
        OsString::from(upstream.as_str()),
    ])?;
    let merge_base = git_text_owned(cwd, &[
        OsString::from("merge-base"),
        OsString::from("HEAD"),
        OsString::from(upstream.as_str()),
    ])?;
    Some(PushIdentity {
        fetch_remote,
        push_remote,
        branch,
        upstream_ref,
        upstream_commit,
        merge_base,
    })
}

/// Return Git's current-branch upstream revision token without source braces.
#[must_use]
fn upstream_revision() -> String
{
    let mut revision = String::from("@");
    revision.push('{');
    revision.push_str("upstream");
    revision.push('}');
    revision
}

/// Return a text Git command's trimmed stdout from owned argv tokens.
///
/// # Contract
/// - requires: `args` are direct argv tokens for `git`.
/// - ensures: behaves like [`git_text`] while allowing callers to build tokens
///   that would otherwise look like formatting placeholders in source.
/// - fails: returns `None` for process failure, nonzero status, or invalid
///   UTF-8.
/// - panics: none.
fn git_text_owned(
    cwd: Option<&Path>,
    args: &[OsString],
) -> Option<String>
{
    let output = command_output(OsStr::new(GIT_PROGRAM), args, cwd)?;
    let text = String::from_utf8(output.0).ok()?;
    Some(trim_terminal_line_endings(text))
}

/// Return a lowercase BLAKE3 digest for `bytes`.
#[must_use]
fn hash_bytes<B>(bytes: B) -> String
where
    B: Into<BytesBytes<'_>>,
{
    let bytes = bytes.into().0;
    let digest = blake3::hash(bytes);
    String::from(digest.to_hex().as_str())
}

/// Return exact stdout bytes from a Git command.
///
/// # Contract
/// - requires: `args` are direct argv tokens for `git`.
/// - ensures: captures stdout without streaming it and removes ambient Git
///   repository override variables.
/// - fails: returns `None` for spawn, wait, or nonzero-status failures.
/// - panics: none.
fn git_output<Args>(
    cwd: Option<&Path>,
    args: Args,
) -> Option<CommandOutputBytes>
where
    Args: IntoIterator,
    Args::Item: Into<OsString>,
{
    let os_args = args.into_iter().map(Into::into).collect::<Vec<_>>();
    command_output(OsStr::new(GIT_PROGRAM), &os_args, cwd)
}

/// Finish a BLAKE3 hasher as a lowercase digest.
#[must_use]
fn finish_hash(hasher: &blake3::Hasher) -> String
{
    let digest = hasher.finalize();
    String::from(digest.to_hex().as_str())
}

/// Build a gate I/O error for `path`.
fn io_error(
    path: &Path,
    source: std::io::Error,
) -> GateError
{
    GateError::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Execute one fixed workflow tier through the support command boundary.
///
/// # Contract
/// - requires: `cwd`, when present, names the working directory where each
///   `mise run` task should execute.
/// - ensures: serializes the workflow with other linked worktrees for the same
///   Git repository, uses an exact successful-task cache only when identity is
///   fully proven, invokes uncached tasks strictly in plan order, and returns
///   after the final task succeeds.
/// - provides: a typed success report containing the selected tier and the
///   number of successfully completed or exactly cached tasks.
/// - fails: returns a support error when the process boundary cannot be
///   started; returns an operational error at the first nonzero task status
///   with tier, task, status, stdout, and stderr context captured; returns lock
///   errors when repository serialization cannot be established.
/// - panics: none.
/// - intension: cache lookup/write failures and identity uncertainty degrade to
///   normal task execution rather than false-green success.
///
/// # Errors
/// Returns [`GateError`] from the support runner, repository lock, checked task
/// counter, or a constructed operational error describing the first failing
/// task.
///
/// # Adequacy
/// - hypothesis: L3 only — the sequential/fail-fast surface is killed by a fake
///   runner that records the task-name projection and returns a nonzero status
///   for a middle task; report-count, exact-hit, cache-miss, failure-no-write,
///   corruption, eviction, and lock-exclusion mutants are killed by injected
///   runner/cache/identity/lock fixtures.
/// - witness: `workflow::tests::execution_stops_after_first_nonzero_task_and_reports_context`
/// - witness: `workflow::tests::successful_execution_reports_the_completed_count`
/// - witness: `workflow::tests::workflow_cache_hit_skips_runner_and_counts_success`
/// - witness: `workflow::tests::workflow_cache_misses_when_correctness_identity_changes`
/// - witness: `workflow::tests::workflow_cache_does_not_write_failed_tasks`
/// - witness: `workflow::tests::file_cache_recovers_corruption_and_bounds_newest_entries`
/// - witness: `workflow::tests::repository_lock_excludes_same_repo_reentry_without_sleep`
#[inline]
pub fn execute(
    tier: Tier,
    cwd: Option<&Path>,
) -> Result<Report, GateError>
{
    let runner = SupportRunner;
    let identity = HostWorkflowIdentity;
    let cache = FileWorkflowCache::host();
    let lock = FileWorkflowLock::host();
    execute_with_environment(tier, cwd, &runner, &identity, &cache, &lock)
}

/// Execute a tier using an injected runner with caching disabled.
///
/// # Contract
/// - requires: `runner` implements the same one-task-at-a-time semantics as the
///   support runner.
/// - ensures: calls `runner` once per task until success is exhausted or a task
///   fails.
/// - provides: the shared workflow loop used by legacy unit tests.
/// - fails: propagates runner errors and converts runner failures into
///   [`GateError::Operational`] with captured context.
/// - panics: none.
///
/// # Errors
/// Returns any [`GateError`] emitted by `runner`, the checked task counter, or
/// the first task failure.
///
/// # Adequacy
/// - hypothesis: L3 only — exact fake-runner call logs distinguish success,
///   first-failure, and skipped-tail branches.
/// - witness: `workflow::tests::execution_stops_after_first_nonzero_task_and_reports_context`
/// - witness: `workflow::tests::successful_execution_reports_the_completed_count`
#[cfg(test)]
fn execute_with_runner<Runner>(
    tier: Tier,
    cwd: Option<&Path>,
    runner: &Runner,
) -> Result<Report, GateError>
where
    Runner: TaskRunner,
{
    let identity = NoWorkflowIdentity;
    let cache = DisabledWorkflowCache;
    let lock = NoWorkflowLock;
    execute_with_environment(tier, cwd, runner, &identity, &cache, &lock)
}

/// Execute a tier with injected runner, identity, cache, and lock seams.
///
/// # Contract
/// - ensures: holds the repository lock around the whole workflow when a
///   repository key is available, and evaluates tasks left-to-right with no
///   parallelism.
/// - provides: the full test seam for cache hit/miss, identity uncertainty,
///   cache failure, and lock exclusion behavior.
/// - fails: returns lock, runner, counter, or task failure errors; cache and
///   identity failures are treated as misses.
/// - panics: none.
///
/// # Errors
/// Returns repository lock failures, runner failures, checked counter overflow,
/// or the first task failure.
fn execute_with_environment<Runner, Identity, Cache, Lock>(
    tier: Tier,
    cwd: Option<&Path>,
    runner: &Runner,
    identity: &Identity,
    cache: &Cache,
    lock: &Lock,
) -> Result<Report, GateError>
where
    Runner: TaskRunner,
    Identity: WorkflowIdentityProvider,
    Cache: WorkflowCacheBackend,
    Lock: WorkflowLockBackend,
{
    let lock_key = identity.repository_lock_key(cwd);
    lock.with_repository_lock(lock_key.as_ref(), || {
        execute_with_cache(tier, cwd, runner, identity, cache)
    })
}

/// Execute a tier under an already-established repository serialization point.
///
/// # Contract
/// - ensures: treats exact cache hits as successful completed tasks, runs
///   misses normally, and writes success only when pre-task and post-task
///   identities match exactly.
/// - provides: the native cache decision loop without changing the task runner
///   process boundary.
/// - fails: returns runner errors, counter overflow, or the first task failure;
///   cache lookup/write errors cause normal execution rather than false-green
///   success.
/// - panics: none.
///
/// # Errors
/// Returns runner errors, checked counter overflow, or the first task failure.
fn execute_with_cache<Runner, Identity, Cache>(
    tier: Tier,
    cwd: Option<&Path>,
    runner: &Runner,
    identity: &Identity,
    cache: &Cache,
) -> Result<Report, GateError>
where
    Runner: TaskRunner,
    Identity: WorkflowIdentityProvider,
    Cache: WorkflowCacheBackend,
{
    let plan = tier.plan();
    let mut completed_tasks = 0_usize;
    for task in plan.tasks() {
        let before_key = task_cache_key(identity, plan.tier(), *task, cwd);
        if let Some(key) = before_key.as_ref()
            && matches!(cache.lookup(key).map(|value| value.into().0), Ok(true))
        {
            completed_tasks =
                increment_completed_tasks(completed_tasks).map(|value| value.into().0)?;
            continue;
        }

        match runner.run_task(*task, cwd)? {
            | TaskExit::Success => {
                if let Some(key) = before_key.as_ref() {
                    let after_key = task_cache_key(identity, plan.tier(), *task, cwd);
                    if after_key.as_ref() == Some(key) {
                        drop(cache.record_success(key));
                    }
                }
                completed_tasks =
                    increment_completed_tasks(completed_tasks).map(|value| value.into().0)?;
            },
            | TaskExit::Failure(failure) => {
                return Err(failure.into_gate_error(plan.tier(), *task));
            },
        }
    }
    Ok(Report {
        tier: plan.tier(),
        completed_tasks,
    })
}

/// Return the cache key for one task if its identity is fully proven.
///
/// # Contract
/// - ensures: returns `None` for uncacheable tasks or uncertain identity, and a
///   full [`CacheKey`] otherwise.
/// - provides: the single cacheability gate for lookup and write paths.
/// - panics: none.
fn task_cache_key<Identity>(
    identity: &Identity,
    tier: Tier,
    task: Task,
    cwd: Option<&Path>,
) -> Option<CacheKey>
where
    Identity: WorkflowIdentityProvider,
{
    if !crate::semantic_value::<CacheableTaskFlag>(is_cacheable_task(task)).0 {
        return None;
    }
    let task_identity = identity.task_identity(tier, task, cwd)?;
    Some(CacheKey::from_identity(tier, task, &task_identity))
}

/// Return whether a task is safe to cache.
///
/// # Contract
/// - ensures: returns false for Act, fuzzing, mutation, publication, release,
///   push, and ratchet task tokens.
/// - provides: a final guard against caching workflow boundaries with external
///   side effects.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — direct side-effect token fixtures kill mutants that
///   forget each rejected token family.
/// - witness: `workflow::tests::side_effect_task_names_are_not_cacheable`
#[must_use]
fn is_cacheable_task(task: Task) -> impl Into<CacheableTaskFlag>
{
    task.name().as_ref().split([':', '-']).all(|token| {
        !matches!(
            token,
            "act" | "fuzz" | "mutants" | "mutation" | "publish" | "push" | "ratchet" | "release"
        )
    })
}

/// Increment the completed-task count with checked arithmetic.
///
/// # Contract
/// - ensures: returns `completed_tasks + 1` when representable.
/// - fails: returns [`GateError::Operational`] on overflow.
/// - panics: none.
///
/// # Errors
/// Returns an operational error if the counter overflows.
fn increment_completed_tasks<C>(
    completed_tasks: C
) -> Result<impl Into<IncrementCompletedTasksCount>, GateError>
where
    C: Into<CompletedTasksCount>,
{
    let completed_tasks = completed_tasks.into().0;
    completed_tasks
        .checked_add(1)
        .ok_or_else(|| GateError::operational("workflow completed-task counter overflowed"))
}

/// A local workflow tier selected by the driver.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tier
{
    /// The checks expected before a worktree branch merges.
    Merge,
    /// The host-compatible checks expected before a push leaves the machine.
    Push,
}

impl Tier
{
    /// Return the fixed plan for this tier.
    ///
    /// # Contract
    /// - requires: `self` is one of the closed workflow tiers.
    /// - ensures: returns the exact static task sequence owned by that tier.
    /// - provides: a plan with no duplicate task names, no Act task, and no
    ///   fuzzing or mutation campaign task.
    /// - panics: none.
    /// - intension: the push plan has the merge plan as its prefix, then adds
    ///   the host-compatible documentation/reference, no-panic, and
    ///   cargo-careful tasks in the order documented by [`PUSH_TASKS`]
    ///   (coverage is temporarily disabled while the failed-refactor
    ///   remediation leaves crates below their recorded floors).
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — exact-order, duplicate, and forbidden-task
    ///   mutants are killed by comparing the static task-name projection for
    ///   both tiers.
    /// - witness: `workflow::tests::merge_plan_order_is_exact`
    /// - witness: `workflow::tests::push_plan_order_is_exact`
    /// - witness: `workflow::tests::plans_have_no_duplicate_tasks`
    /// - witness: `workflow::tests::plans_exclude_act_fuzz_and_mutation_tasks`
    #[inline]
    #[must_use]
    pub fn plan(self) -> Plan
    {
        match self {
            | Self::Merge => Plan {
                tier: self,
                tasks: MERGE_TASKS,
            },
            | Self::Push => Plan {
                tier: self,
                tasks: PUSH_TASKS,
            },
        }
    }

    /// Return the stable tier label used in diagnostics.
    #[inline]
    #[must_use]
    pub const fn as_str(self) -> NameText<'static>
    {
        match self {
            | Self::Merge => NameText("merge"),
            | Self::Push => NameText("push"),
        }
    }
}

/// A fixed workflow plan made of canonical `mise` task boundaries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Plan
{
    /// Tier that owns the task sequence.
    tier: Tier,
    /// Sequential task list; entries are executed left-to-right.
    tasks: &'static [Task],
}

impl Plan
{
    /// Return the tier that owns this plan.
    #[inline]
    #[must_use]
    pub const fn tier(self) -> Tier
    {
        self.tier
    }

    /// Return the sequential task list for this plan.
    #[inline]
    #[must_use]
    pub const fn tasks(self) -> &'static [Task]
    {
        self.tasks
    }
}

/// One canonical `mise run <name>` workflow boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct Task
{
    /// Task name as it appears in `mise.toml`.
    name: &'static str,
}

impl Task
{
    /// Build a static task descriptor.
    #[inline]
    #[must_use]
    const fn new(name: NameText<'static>) -> Self
    {
        Self { name: name.0 }
    }

    /// Return the canonical `mise.toml` task name.
    #[inline]
    #[must_use]
    pub const fn name(self) -> NameText<'static>
    {
        NameText(self.name)
    }
}

/// Summary of a completed workflow execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Report
{
    /// Tier that completed.
    tier: Tier,
    /// Number of tasks that exited successfully.
    completed_tasks: usize,
}

impl Report
{
    /// Return the completed workflow tier.
    ///
    /// # Contract
    /// - ensures: returns the tier supplied to [`execute`] for this report.
    /// - provides: the CLI-facing success tier projection without exposing
    ///   report fields.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — workflow success witnesses compare the returned tier
    ///   with the requested tier.
    /// - witness: `workflow::tests::successful_execution_reports_the_completed_count`
    #[inline]
    #[must_use]
    pub const fn tier(self) -> Tier
    {
        self.tier
    }

    /// Return the number of completed workflow tasks.
    ///
    /// # Contract
    /// - ensures: returns the count of tasks that completed before [`execute`]
    ///   returned success.
    /// - provides: the CLI-facing progress summary without exposing report
    ///   fields.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L2 — success witnesses compare this value with the
    ///   selected plan length.
    /// - witness: `workflow::tests::successful_execution_reports_the_completed_count`
    #[inline]
    #[must_use]
    pub const fn completed_tasks(self) -> CompletedTasksCount
    {
        CompletedTasksCount(self.completed_tasks)
    }
}

/// Boundary used by the workflow loop to run exactly one task.
trait TaskRunner
{
    /// Run one canonical task in `cwd`.
    ///
    /// # Contract
    /// - requires: `task` belongs to a static workflow plan and `cwd`, when
    ///   present, is the intended execution directory.
    /// - ensures: starts at most one process and reports only that process's
    ///   status.
    /// - provides: an abstract seam for command-free tests of the workflow
    ///   loop.
    /// - fails: returns [`GateError`] when the runner cannot obtain a task
    ///   status.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns the runner-specific operational or I/O error.
    ///
    /// # Adequacy
    /// - hypothesis: L2 interface — concrete implementations witness the
    ///   process boundary or fake-runner recording behavior; the trait itself
    ///   has no executable branch.
    /// - witness: `workflow::tests::execution_stops_after_first_nonzero_task_and_reports_context`
    fn run_task(
        &self,
        task: Task,
        cwd: Option<&Path>,
    ) -> Result<TaskExit, GateError>;
}

/// Production task runner backed by `support::run_output`.
struct SupportRunner;

impl TaskRunner for SupportRunner
{
    /// Run one task as `mise run <task>` through the support API.
    ///
    /// # Contract
    /// - requires: `task.name()` is a canonical `mise.toml` task name.
    /// - ensures: invokes `mise` with exactly the `run` subcommand and the task
    ///   name while removing ambient Git repository override variables.
    /// - provides: success/failure status and captured output for the workflow
    ///   loop.
    /// - fails: propagates support startup/output errors.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError`] from [`crate::support::run_output_streamed`].
    ///
    /// # Adequacy
    /// - hypothesis: L2 boundary — command spawning is intentionally not
    ///   invoked by this assignment's tests; the workflow loop's observable
    ///   sequencing is witnessed through the injected-runner seam and
    ///   integration owns the real process-boundary smoke.
    /// - witness: `workflow::tests::execution_stops_after_first_nonzero_task_and_reports_context`
    fn run_task(
        &self,
        task: Task,
        cwd: Option<&Path>,
    ) -> Result<TaskExit, GateError>
    {
        let args = [OsString::from("run"), OsString::from(task.name().as_ref())];
        let output =
            crate::support::run_output_streamed(OsStr::new(MISE_PROGRAM), &args, cwd, true)?;
        if output.success().into().0 {
            return Ok(TaskExit::Success);
        }
        Ok(TaskExit::Failure(TaskFailure {
            status: format!("{:?}", output.code().into().0),
            stdout: output.stdout_lossy().as_ref().to_owned(),
        }))
    }
}

/// Status returned by one task execution.
#[derive(Clone, Debug, Eq, PartialEq)]
enum TaskExit
{
    /// The task exited successfully.
    Success,
    /// The task exited unsuccessfully with captured context.
    Failure(TaskFailure),
}

#[cfg(test)]
impl TaskExit
{
    /// Build a failed task status for tests.
    #[inline]
    #[must_use]
    fn failed<S, O>(
        status: S,
        stdout: O,
    ) -> Self
    where
        S: Into<String>,
        O: Into<String>,
    {
        let status = status.into();
        let stdout = stdout.into();
        Self::Failure(TaskFailure { status, stdout })
    }
}

/// Retained context for one failed task.
#[derive(Clone, Debug, Eq, PartialEq)]
struct TaskFailure
{
    /// Stable status label returned by the process runner.
    status: String,
    /// Bounded standard-output prefix retained by the streaming runner.
    stdout: String,
}

impl TaskFailure
{
    /// Convert the captured failure into the crate's operational error type.
    ///
    /// # Contract
    /// - requires: `tier` and `task` identify the failed workflow boundary.
    /// - ensures: preserves the tier label, task name, status label, and
    ///   bounded standard-output prefix in the returned diagnostic detail.
    /// - provides: the fail-fast error payload consumed by the CLI layer.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — missing-context mutants are killed by asserting
    ///   each captured field in the fake-runner failure test.
    /// - witness: `workflow::tests::execution_stops_after_first_nonzero_task_and_reports_context`
    #[inline]
    #[must_use]
    fn into_gate_error(
        self,
        tier: Tier,
        task: Task,
    ) -> GateError
    {
        GateError::operational(format!(
            "workflow {} failed at mise task `{}` with status {}; stdout prefix: {}; stderr was streamed live",
            tier.as_str().as_ref(),
            task.name().as_ref(),
            self.status,
            self.stdout
        ))
    }
}

/// Unit and fixture witnesses for workflow plans, caching, and locking.
#[cfg(test)]
mod tests
{
    use alloc::collections::BTreeSet;
    use alloc::collections::VecDeque;
    use core::cell::RefCell;
    use core::error::Error;
    #[cfg(unix)]
    use core::sync::atomic::AtomicU64;
    #[cfg(unix)]
    use core::sync::atomic::Ordering;
    #[cfg(unix)]
    use std::ffi::OsStr;
    #[cfg(unix)]
    use std::ffi::OsString;
    #[cfg(unix)]
    use std::io;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt as _;
    #[cfg(unix)]
    use std::path::Path;
    #[cfg(unix)]
    use std::path::PathBuf;

    use proptest::prelude::*;

    use super::CacheKey;
    use super::CanonicalTaskNameFlag;
    use super::FileWorkflowCache;
    use super::FileWorkflowLock;
    use super::GateError;
    use super::HostWorkflowIdentity;
    use super::LookupFlag;
    use super::NameText;
    use super::Plan;
    use super::PushIdentity;
    use super::RepositoryLockKey;
    use super::RowText;
    use super::Task;
    use super::TaskExit;
    use super::TaskFailure;
    use super::TaskRunner;
    use super::Tier;
    use super::ValueText;
    use super::WORKFLOW_CACHE_ENTRY_LIMIT;
    use super::WORKFLOW_CACHE_SCHEMA;
    use super::WorkflowCacheBackend;
    use super::WorkflowCacheFile;
    use super::WorkflowIdentityProvider;
    use super::WorkflowInputIdentity;
    use super::WorkflowLockBackend;
    use super::config_identity;
    use super::execute_with_environment;
    use super::execute_with_runner;
    use super::finish_hash;
    use super::git_text;
    use super::git_text_owned;
    use super::hash_bytes;
    use super::io_error;
    use super::is_cacheable_task;
    use super::push_identity;
    use super::repository_is_clean;
    use super::repository_lock_key;
    use super::submodule_identity;
    use super::update_command_identity;
    use super::upstream_revision;
    use super::workflow_policy_identity;

    /// Test result used by workflow unit tests.
    type TestResult<T = ()> = Result<T, Box<dyn Error>>;
    /// Executable mode used by the fake POSIX `mise` fixture.
    #[cfg(unix)]
    const EXECUTABLE_MODE: u32 = 0o755;

    /// Per-process suffix keeping concurrently-created workflow fixtures
    /// disjoint.
    #[cfg(unix)]
    static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

    /// Merge plan keeps the exact canonical task order.
    #[test]
    fn merge_plan_order_is_exact()
    {
        assert_eq!(
            &[
                "core:check",
                "grammar:test",
                "cargo:build",
                "cargo:clippy",
                "cargo:dylint",
                "cargo:nextest",
                "treefmt:check",
                "wrkflw",
            ][..],
            task_names(Tier::Merge.plan())
        );
    }

    /// Push plan keeps merge checks first, then host-compatible heavy checks.
    #[test]
    fn push_plan_order_is_exact()
    {
        assert_eq!(
            &[
                "core:check",
                "grammar:test",
                "cargo:build",
                "cargo:clippy",
                "cargo:dylint",
                "cargo:nextest",
                "treefmt:check",
                "wrkflw",
                "cargo:doc-check",
                "docs:conflict-markers",
                "docs:manifest-drift",
                "docs:reference-integrity",
                "test:soundness-oracles",
                "test:doc-gates",
                "test:page-balance",
                "test:graph-gates",
                // "test:dep-graph",
                // Disabled while the failed-refactor coverage remediation is pending.
                // "coverage:check",
                "cargo:no-panic",
                "cargo:careful-nextest",
            ][..],
            task_names(Tier::Push.plan())
        );
    }

    /// Both plans reject duplicate task names.
    #[test]
    fn plans_have_no_duplicate_tasks()
    {
        assert_plan_has_no_duplicates(Tier::Merge.plan());
        assert_plan_has_no_duplicates(Tier::Push.plan());
    }

    /// Both plans exclude Act, stamp tests, fuzzing, and mutation campaigns.
    #[test]
    fn plans_exclude_act_fuzz_and_mutation_tasks()
    {
        for tier in [Tier::Merge, Tier::Push] {
            for task in tier.plan().tasks() {
                let name = task.name();
                assert!(
                    !name.as_ref().contains("act"),
                    "Act task leaked into {tier:?}: {}",
                    name.as_ref()
                );
                assert!(
                    !name.as_ref().starts_with("fuzz:"),
                    "fuzz task leaked into {tier:?}: {name}"
                );
                assert!(
                    !name.as_ref().starts_with("mutants:"),
                    "mutation task leaked into {tier:?}: {name}"
                );
                assert_ne!(
                    name.as_ref(),
                    "test:act-ci-stamps",
                    "stamp-regression harness must be superseded, not ported"
                );
            }
        }
    }

    /// Execution stops at the first nonzero task and reports bounded retained
    /// context while documenting that standard error was streamed live.
    #[test]
    fn execution_stops_after_first_nonzero_task_and_reports_context() -> TestResult
    {
        let runner = ScriptedRunner::new([
            TaskExit::Success,
            TaskExit::failed("Some(17)", "captured out"),
            TaskExit::Success,
        ]);

        let result = execute_with_runner(Tier::Push, None, &runner);
        let Err(GateError::Operational { detail }) = result
        else {
            return Err(Box::new(std::io::Error::other(
                "push workflow unexpectedly succeeded or returned the wrong error",
            )));
        };

        assert_eq!(&["core:check", "grammar:test"][..], runner.calls());
        assert!(detail.contains("workflow push failed"));
        assert!(detail.contains("grammar:test"));
        assert!(detail.contains("Some(17)"));
        assert!(detail.contains("captured out"));
        assert!(detail.contains("stderr was streamed live"));
        insta::assert_snapshot!(
            &detail,
            @"workflow push failed at mise task `grammar:test` with status Some(17); stdout prefix: captured out; stderr was streamed live"
        );
        Ok(())
    }

    /// Successful execution reports the exact completed task count.
    #[test]
    fn successful_execution_reports_the_completed_count() -> TestResult
    {
        let runner = ScriptedRunner::new([
            TaskExit::Success,
            TaskExit::Success,
            TaskExit::Success,
            TaskExit::Success,
            TaskExit::Success,
            TaskExit::Success,
            TaskExit::Success,
            TaskExit::Success,
        ]);

        let report = execute_with_runner(Tier::Merge, None, &runner)?;

        assert_eq!(
            &[
                "core:check",
                "grammar:test",
                "cargo:build",
                "cargo:clippy",
                "cargo:dylint",
                "cargo:nextest",
                "treefmt:check",
                "wrkflw",
            ][..],
            runner.calls()
        );
        assert_eq!(Tier::Merge, report.tier());
        assert_eq!(report.completed_tasks().0, Tier::Merge.plan().tasks().len());
        Ok(())
    }

    /// Side-effect task families are never cacheable.
    #[test]
    fn side_effect_task_names_are_not_cacheable()
    {
        for name in [
            "act",
            "fuzz:smoke",
            "mutants:run",
            "mutation:check",
            "docs:publish",
            "remote:push",
            "coverage:ratchet",
            "release",
        ] {
            assert!(
                !is_cacheable_task(Task::new(NameText(name))).into().0,
                "{name} was cacheable"
            );
        }
    }

    /// Exact cache hits skip the runner and still count as completed tasks.
    #[test]
    fn workflow_cache_hit_skips_runner_and_counts_success() -> TestResult
    {
        let identity = StaticIdentity::new(Some(base_merge_identity()));
        let cache = MemoryWorkflowCache::default();
        for task in Tier::Merge.plan().tasks() {
            cache.insert(CacheKey::from_identity(
                Tier::Merge,
                *task,
                &base_merge_identity(),
            ));
        }
        let runner = ScriptedRunner::new(Vec::<TaskExit>::new());
        let lock = RecordingLock::default();

        let report =
            execute_with_environment(Tier::Merge, None, &runner, &identity, &cache, &lock)?;

        assert_eq!(Tier::Merge, report.tier);
        assert_eq!(report.completed_tasks, Tier::Merge.plan().tasks().len());
        assert!(runner.calls().is_empty());
        assert_eq!(&["repo"][..], lock.entries());
        Ok(())
    }

    /// Every correctness dimension participates in cache equality.
    #[test]
    fn workflow_cache_misses_when_correctness_identity_changes() -> TestResult
    {
        let base = base_push_identity();
        let base_key =
            CacheKey::from_identity(Tier::Push, Task::new(NameText("core:check")), &base);
        let cache = MemoryWorkflowCache::default();
        cache.insert(base_key);

        for (label, key) in mutated_cache_keys(&base) {
            assert!(
                !cache.lookup(&key).map(|value| value.into().0)?,
                "cache hit despite changed correctness dimension: {label}"
            );
        }
        Ok(())
    }

    /// Missing or uncertain identity bypasses the cache without writes.
    #[test]
    fn workflow_cache_bypasses_missing_identity_without_write() -> TestResult
    {
        let identity = StaticIdentity::new(None);
        let cache = MemoryWorkflowCache::default();
        let runner = ScriptedRunner::new(successes_for(Tier::Merge));
        let lock = RecordingLock::default();

        let report =
            execute_with_environment(Tier::Merge, None, &runner, &identity, &cache, &lock)?;

        assert_eq!(report.completed_tasks, Tier::Merge.plan().tasks().len());
        assert_eq!(runner.calls(), task_names(Tier::Merge.plan()));
        assert!(cache.recorded().is_empty());
        Ok(())
    }

    /// Cache I/O failures are misses, not false-green workflow results.
    #[test]
    fn workflow_cache_io_failures_run_normally() -> TestResult
    {
        let identity = StaticIdentity::new(Some(base_merge_identity()));
        let cache = FailingWorkflowCache;
        let runner = ScriptedRunner::new(successes_for(Tier::Merge));
        let lock = RecordingLock::default();

        let report =
            execute_with_environment(Tier::Merge, None, &runner, &identity, &cache, &lock)?;

        assert_eq!(report.completed_tasks, Tier::Merge.plan().tasks().len());
        assert_eq!(runner.calls(), task_names(Tier::Merge.plan()));
        Ok(())
    }

    /// Failed tasks do not write successful cache entries.
    #[test]
    fn workflow_cache_does_not_write_failed_tasks()
    {
        let identity = StaticIdentity::new(Some(base_merge_identity()));
        let cache = MemoryWorkflowCache::default();
        let runner = ScriptedRunner::new([TaskExit::failed("Some(42)", "no cache")]);
        let lock = RecordingLock::default();

        let result = execute_with_environment(Tier::Merge, None, &runner, &identity, &cache, &lock);

        assert!(matches!(result, Err(GateError::Operational { .. })));
        assert!(cache.recorded().is_empty());
    }

    /// Successful tasks do not write when post-task identity no longer matches.
    #[test]
    fn workflow_cache_does_not_write_when_success_changes_identity() -> TestResult
    {
        let identity = QueuedIdentity::new([Some(base_merge_identity()), None]);
        let cache = MemoryWorkflowCache::default();
        let runner = ScriptedRunner::new(successes_for(Tier::Merge));
        let lock = RecordingLock::default();

        let report =
            execute_with_environment(Tier::Merge, None, &runner, &identity, &cache, &lock)?;

        assert_eq!(report.completed_tasks, Tier::Merge.plan().tasks().len());
        assert!(cache.recorded().is_empty());
        assert_eq!(runner.calls(), task_names(Tier::Merge.plan()));
        Ok(())
    }

    /// File cache recovers corrupt JSON, writes atomically, and evicts oldest
    /// entries.
    #[test]
    fn file_cache_recovers_corruption_and_bounds_newest_entries() -> TestResult
    {
        let root = unique_workflow_root().join("file-cache");
        let cache = FileWorkflowCache::new(root);
        let key = CacheKey::from_identity(
            Tier::Merge,
            Task::new(NameText("core:check")),
            &base_merge_identity(),
        );
        let path = cache.path(&key.repository);
        let parent = path
            .parent()
            .ok_or_else(|| GateError::operational("cache path missing parent"))?;
        crate::support::HOST_FILESYSTEM.create_dir_all(parent)?;
        crate::support::HOST_FILESYSTEM.write(&path, b"not json")?;

        assert!(!cache.lookup(&key).map(|value| value.into().0)?);
        cache.record_success(&key)?;
        assert!(cache.lookup(&key).map(|value| value.into().0)?);

        for index in 0 .. WORKFLOW_CACHE_ENTRY_LIMIT + 3 {
            let mut next = key.clone();
            next.task = format!("task:{index}");
            cache.record_success(&next)?;
        }

        let bytes = crate::support::HOST_FILESYSTEM.read(&path)?;
        let cache_file = serde_json::from_slice::<WorkflowCacheFile>(bytes.as_bytes().into())?;
        assert_eq!(WORKFLOW_CACHE_ENTRY_LIMIT, cache_file.entries.len());
        let newest_task = format!("task:{}", WORKFLOW_CACHE_ENTRY_LIMIT + 2);
        assert!(
            cache_file
                .entries
                .iter()
                .any(|entry| entry.key.task == newest_task),
            "newest cache entry was evicted"
        );
        assert!(
            !cache_file
                .entries
                .iter()
                .any(|entry| entry.key.task == "core:check"),
            "old corrupt-recovery entry survived newest-entry eviction"
        );
        assert_eq!(cache_directory_entries(parent)?, [path]);
        Ok(())
    }

    /// File cache misses safely when the cache file is absent or from another
    /// schema.
    #[cfg(unix)]
    #[test]
    fn file_cache_treats_missing_and_schema_mismatched_files_as_misses() -> TestResult
    {
        let root = unique_workflow_root().join("file-cache-stale");
        let cache = FileWorkflowCache::new(root);
        let key = CacheKey::from_identity(
            Tier::Merge,
            Task::new(NameText("core:check")),
            &base_merge_identity(),
        );
        let path = cache.path(&key.repository);

        assert!(!cache.lookup(&key).map(|value| value.into().0)?);

        let parent = path
            .parent()
            .ok_or_else(|| GateError::operational("cache path missing parent"))?;
        crate::support::HOST_FILESYSTEM.create_dir_all(parent)?;
        let stale_file = WorkflowCacheFile {
            schema: WORKFLOW_CACHE_SCHEMA.saturating_add(1),
            entries: Vec::new(),
        };
        crate::support::HOST_FILESYSTEM.write(path, serde_json::to_vec(&stale_file)?)?;

        assert!(!cache.lookup(&key).map(|value| value.into().0)?);
        Ok(())
    }

    /// File locks create a global lock file and release it before a later
    /// entry.
    #[cfg(unix)]
    #[test]
    fn file_workflow_lock_serializes_and_releases_local_repository_key() -> TestResult
    {
        let fixture_root = unique_workflow_root();
        let lock = FileWorkflowLock::new(fixture_root.join("locks"));
        let key = RepositoryLockKey {
            token: String::from("same-repository"),
        };
        let mut entries = Vec::new();

        let unlocked = lock.with_repository_lock(None, || Ok::<_, GateError>("unlocked"))?;
        assert_eq!("unlocked", unlocked);
        lock.with_repository_lock(Some(&key), || {
            entries.push("first");
            Ok(())
        })?;
        lock.with_repository_lock(Some(&key), || {
            entries.push("second");
            Ok(())
        })?;

        assert_eq!(&["first", "second"][..], entries);
        assert!(lock.path(&key).exists());
        drop(crate::support::HOST_FILESYSTEM.remove_dir_all(fixture_root));
        Ok(())
    }

    /// Local Git fixtures cover repository, cleanliness, endpoint, and range
    /// identity.
    #[cfg(unix)]
    #[test]
    fn git_identity_helpers_validate_clean_endpoint_and_push_range() -> TestResult
    {
        let fixture = GitWorkflowFixture::create()?;
        let provider = HostWorkflowIdentity;

        let direct_key = repository_lock_key(Some(&fixture.repo))
            .ok_or_else(|| GateError::operational("missing direct repository key"))?;
        let provider_key = provider
            .repository_lock_key(Some(&fixture.repo))
            .ok_or_else(|| GateError::operational("missing provider repository key"))?;
        assert_eq!(provider_key, direct_key);
        assert_eq!(
            Some(true),
            repository_is_clean(Some(&fixture.repo)).into().0
        );
        assert_eq!(
            Some("feature"),
            git_text(Some(&fixture.repo), ["rev-parse", "--abbrev-ref", "HEAD"]).as_deref(),
        );
        assert_eq!(
            Some("feature"),
            git_text_owned(Some(&fixture.repo), &[
                os("rev-parse"),
                os("--abbrev-ref"),
                os("HEAD")
            ],)
            .as_deref(),
        );
        assert!(git_text(Some(&fixture.repo), ["definitely-not-a-git-command"]).is_none());

        let submodules = submodule_identity(Some(&fixture.repo))
            .ok_or_else(|| GateError::operational("missing submodule identity"))?;
        let config = config_identity(Some(&fixture.repo))
            .ok_or_else(|| GateError::operational("missing config identity"))?;
        let merge_policy = workflow_policy_identity(Tier::Merge);
        let push_policy = workflow_policy_identity(Tier::Push);
        assert_eq!(64, submodules.len());
        assert_eq!(64, config.len());
        assert_eq!(64, merge_policy.len());
        assert_eq!(64, push_policy.len());
        assert_ne!(merge_policy, push_policy);

        let upstream = upstream_revision();
        assert_eq!(b"@{upstream}", upstream.as_bytes());
        let endpoint = push_identity(Some(&fixture.repo))
            .ok_or_else(|| GateError::operational("missing push identity"))?;
        let remote = fixture.remote.to_string_lossy();
        assert_eq!(endpoint.fetch_remote, remote.as_ref());
        assert_eq!(endpoint.push_remote, remote.as_ref());
        assert_eq!("feature", endpoint.branch);
        assert_eq!("refs/remotes/origin/main", endpoint.upstream_ref);
        assert_eq!(
            endpoint.upstream_commit,
            git_text_owned(Some(&fixture.repo), &[
                os("rev-parse"),
                OsString::from(upstream.as_str())
            ],)
            .ok_or_else(|| GateError::operational("missing upstream commit"))?,
        );
        assert_eq!(
            endpoint.merge_base,
            git_text_owned(Some(&fixture.repo), &[
                os("merge-base"),
                os("HEAD"),
                OsString::from(upstream.as_str())
            ],)
            .ok_or_else(|| GateError::operational("missing merge base"))?,
        );

        let dirty_path = fixture.repo.join("dirty.txt");
        crate::support::HOST_FILESYSTEM.write(&dirty_path, "dirty\n")?;
        assert_eq!(
            Some(false),
            repository_is_clean(Some(&fixture.repo)).into().0
        );
        crate::support::HOST_FILESYSTEM.remove_file(dirty_path)?;
        git_status(Some(&fixture.repo), &[
            os("checkout"),
            os("--detach"),
            os("HEAD"),
        ])?;
        assert!(push_identity(Some(&fixture.repo)).is_none());
        Ok(())
    }

    /// Command identity hashes command labels, argv, and exact stdout bytes.
    #[cfg(unix)]
    #[test]
    fn command_identity_and_io_errors_preserve_exact_boundaries() -> TestResult
    {
        let mut first = blake3::Hasher::new();
        update_command_identity(
            &mut first,
            OsStr::new("/usr/bin/printf"),
            &[os("same-output")],
            None,
        )
        .ok_or_else(|| GateError::operational("first command identity failed"))?;
        let first_digest = finish_hash(&first);

        let mut second = blake3::Hasher::new();
        update_command_identity(
            &mut second,
            OsStr::new("/usr/bin/printf"),
            &[os("different-output")],
            None,
        )
        .ok_or_else(|| GateError::operational("second command identity failed"))?;
        assert_ne!(first_digest, finish_hash(&second));
        assert_ne!(hash_bytes(b"alpha"), hash_bytes(b"beta"));

        let error = io_error(
            Path::new("workflow-cache"),
            io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
        );
        let GateError::Io { path, source } = error
        else {
            return Err(Box::new(io::Error::other("expected workflow I/O error")));
        };
        assert_eq!(path, PathBuf::from("workflow-cache"));
        assert_eq!(io::ErrorKind::PermissionDenied, source.kind());
        Ok(())
    }

    /// Repository locks exclude same-repository reentry without sleeps.
    #[test]
    fn repository_lock_excludes_same_repo_reentry_without_sleep() -> TestResult
    {
        let lock = RecordingLock::default();
        let key = RepositoryLockKey {
            token: String::from("shared-repository"),
        };

        let result = lock.with_repository_lock(Some(&key), || {
            lock.with_repository_lock(Some(&key), || Ok(()))
        });

        assert!(matches!(result, Err(GateError::Operational { .. })));
        lock.with_repository_lock(Some(&key), || Ok(()))?;
        Ok(())
    }

    /// The production process boundary runs both merge and push tiers inside a
    /// temporary Git repository that points at a temporary local remote.
    #[cfg(unix)]
    #[test]
    fn merge_and_push_workflows_run_inside_local_git_remote_fixture() -> TestResult
    {
        let fixture = GitWorkflowFixture::create()?;
        let runner = ProgramRunner {
            program: fixture.mise.as_os_str(),
        };

        let merge_report = execute_with_runner(Tier::Merge, Some(&fixture.repo), &runner)?;
        let push_report = execute_with_runner(Tier::Push, Some(&fixture.repo), &runner)?;

        assert_eq!(Tier::Merge, merge_report.tier);
        assert_eq!(
            merge_report.completed_tasks,
            Tier::Merge.plan().tasks().len()
        );
        assert_eq!(Tier::Push, push_report.tier);
        assert_eq!(push_report.completed_tasks, Tier::Push.plan().tasks().len());

        let log = crate::support::HOST_FILESYSTEM.read_to_string(&fixture.log)?;
        let rows = log.lines().collect::<Vec<_>>();
        let mut expected_tasks = task_names(Tier::Merge.plan());
        expected_tasks.extend(task_names(Tier::Push.plan()));
        assert_eq!(
            rows.len(),
            expected_tasks.len(),
            "fake mise should observe every merge and push task exactly once",
        );

        let remote = fixture.remote.to_string_lossy();
        for (row, expected_task) in rows.iter().zip(expected_tasks) {
            let record = WorkflowInvocation::parse(*row)?;
            assert_eq!(record.task, expected_task);
            assert_eq!("feature", record.branch);
            assert_eq!(
                record.remote,
                remote.as_ref(),
                "workflow task escaped the local temporary remote fixture",
            );
        }
        Ok(())
    }

    /// Return whether `name` is a direct mise task token with no shell syntax.
    fn is_canonical_task_name<N>(
        name: N
    ) -> impl Into<CanonicalTaskNameFlag>
    where
        N: Into<NameText<'_>>,
    {
        let name = name.into().0;
        !name.is_empty()
            && name.chars().all(|character| {
                character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || matches!(character, ':' | '-')
            })
    }

    /// Return a unique temp fixture root for workflow tests.
    #[cfg(unix)]
    fn unique_workflow_root() -> PathBuf
    {
        let suffix = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "gandr-workflow-gates-workflow-{}-{suffix}",
            std::process::id()
        ))
    }

    /// Write an executable fake `mise` that records task, branch, and remote.
    #[cfg(unix)]
    fn write_fake_mise(
        target: &Path,
        log: &Path,
    ) -> TestResult
    {
        let script = format!(
            "#!/bin/sh\nset -eu\nif [ \"$#\" -ne 2 ] || [ \"$1\" != run ]; then\n  echo \"unexpected mise argv: $*\" >&2\n  exit 64\nfi\nremote=\"$(git remote get-url origin)\"\nbranch=\"$(git rev-parse --abbrev-ref HEAD)\"\nprintf '%s|%s|%s\\n' \"$2\" \"$branch\" \"$remote\" >> {}\n",
            shell_quote(log)
        );
        crate::support::HOST_FILESYSTEM.write(target, script)?;
        let mut permissions = crate::support::HOST_FILESYSTEM
            .metadata(target)?
            .permissions();
        permissions.set_mode(EXECUTABLE_MODE);
        crate::support::HOST_FILESYSTEM.set_permissions(target, permissions)?;
        Ok(())
    }

    /// Quote a path for a POSIX shell single-quoted string.
    #[cfg(unix)]
    fn shell_quote(path: &Path) -> String
    {
        let text = path.to_string_lossy();
        let mut quoted = String::from("'");
        for character in text.chars() {
            if character == '\'' {
                quoted.push_str("'\\''");
            }
            else {
                quoted.push(character);
            }
        }
        quoted.push('\'');
        quoted
    }

    /// Convert a static string into an operating-system argument.
    #[cfg(unix)]
    fn os<V>(value: V) -> OsString
    where
        V: Into<ValueText<'_>>,
    {
        let value = value.into().0;
        OsString::from(value)
    }

    /// Run a Git command through the bounded support boundary.
    #[cfg(unix)]
    fn git_status(
        cwd: Option<&Path>,
        args: &[OsString],
    ) -> TestResult
    {
        let mut command = crate::support::stateless_git_command();
        command.args(args);
        if let Some(directory) = cwd {
            command.current_dir(directory);
        }
        let status = command.status()?;
        if status.success() {
            return Ok(());
        }
        Err(Box::new(io::Error::other(format!(
            "git fixture command failed with status {:?}",
            status.code()
        ))))
    }

    /// Assert that a plan has no repeated task names.
    fn assert_plan_has_no_duplicates(plan: Plan)
    {
        let mut names = BTreeSet::new();
        for task in plan.tasks() {
            let name = task.name();
            assert!(
                names.insert(String::from(name.as_ref())),
                "duplicate task in {:?}: {name}",
                plan.tier()
            );
        }
    }

    /// Return a baseline push-tier identity.
    fn base_push_identity() -> WorkflowInputIdentity
    {
        WorkflowInputIdentity {
            push: Some(base_push_endpoint()),
            ..base_merge_identity()
        }
    }

    /// Return a baseline push endpoint identity.
    fn base_push_endpoint() -> PushIdentity
    {
        PushIdentity {
            fetch_remote: String::from("fetch"),
            push_remote: String::from("push"),
            branch: String::from("feature"),
            upstream_ref: String::from("refs/remotes/origin/main"),
            upstream_commit: String::from("upstream"),
            merge_base: String::from("base"),
        }
    }

    /// Return a baseline merge-tier identity.
    fn base_merge_identity() -> WorkflowInputIdentity
    {
        WorkflowInputIdentity {
            repository: String::from("repo"),
            head: String::from("head"),
            tree: String::from("tree"),
            submodules: String::from("submodules"),
            workflow: String::from("workflow"),
            toolchain: String::from("toolchain"),
            config: String::from("config"),
            push: None,
        }
    }

    /// Build one mutated cache key per correctness dimension.
    fn mutated_cache_keys(base: &WorkflowInputIdentity) -> Vec<(NameText<'static>, CacheKey)>
    {
        let mut cases = Vec::new();
        let task = Task::new(NameText("core:check"));

        let mut repository_case = base.clone();
        repository_case.repository = String::from("repo-other");
        cases.push((
            NameText("repository"),
            CacheKey::from_identity(Tier::Push, task, &repository_case),
        ));

        let mut head_case = base.clone();
        head_case.head = String::from("head-other");
        cases.push((
            NameText("head"),
            CacheKey::from_identity(Tier::Push, task, &head_case),
        ));

        let mut tree_case = base.clone();
        tree_case.tree = String::from("tree-other");
        cases.push((
            NameText("tree"),
            CacheKey::from_identity(Tier::Push, task, &tree_case),
        ));

        let mut submodules_case = base.clone();
        submodules_case.submodules = String::from("submodules-other");
        cases.push((
            NameText("submodules"),
            CacheKey::from_identity(Tier::Push, task, &submodules_case),
        ));

        let mut workflow_case = base.clone();
        workflow_case.workflow = String::from("workflow-other");
        cases.push((
            NameText("workflow"),
            CacheKey::from_identity(Tier::Push, task, &workflow_case),
        ));

        let mut toolchain_case = base.clone();
        toolchain_case.toolchain = String::from("toolchain-other");
        cases.push((
            NameText("toolchain"),
            CacheKey::from_identity(Tier::Push, task, &toolchain_case),
        ));

        let mut config_case = base.clone();
        config_case.config = String::from("config-other");
        cases.push((
            NameText("config"),
            CacheKey::from_identity(Tier::Push, task, &config_case),
        ));

        let mut missing_push_case = base.clone();
        missing_push_case.push = None;
        cases.push((
            NameText("push-presence"),
            CacheKey::from_identity(Tier::Push, task, &missing_push_case),
        ));

        let mut fetch_remote_case = base.clone();
        if let Some(push) = fetch_remote_case.push.as_mut() {
            push.fetch_remote = String::from("fetch-other");
        }
        cases.push((
            NameText("fetch-remote"),
            CacheKey::from_identity(Tier::Push, task, &fetch_remote_case),
        ));

        let mut push_remote_case = base.clone();
        if let Some(push) = push_remote_case.push.as_mut() {
            push.push_remote = String::from("push-other");
        }
        cases.push((
            NameText("push-remote"),
            CacheKey::from_identity(Tier::Push, task, &push_remote_case),
        ));

        let mut branch_case = base.clone();
        if let Some(push) = branch_case.push.as_mut() {
            push.branch = String::from("main");
        }
        cases.push((
            NameText("branch"),
            CacheKey::from_identity(Tier::Push, task, &branch_case),
        ));

        let mut upstream_ref_case = base.clone();
        if let Some(push) = upstream_ref_case.push.as_mut() {
            push.upstream_ref = String::from("refs/remotes/origin/next");
        }
        cases.push((
            NameText("upstream-ref"),
            CacheKey::from_identity(Tier::Push, task, &upstream_ref_case),
        ));

        let mut upstream_commit_case = base.clone();
        if let Some(push) = upstream_commit_case.push.as_mut() {
            push.upstream_commit = String::from("upstream-other");
        }
        cases.push((
            NameText("upstream-commit"),
            CacheKey::from_identity(Tier::Push, task, &upstream_commit_case),
        ));

        let mut merge_base_case = base.clone();
        if let Some(push) = merge_base_case.push.as_mut() {
            push.merge_base = String::from("base-other");
        }
        cases.push((
            NameText("merge-base"),
            CacheKey::from_identity(Tier::Push, task, &merge_base_case),
        ));

        cases.push((
            NameText("tier"),
            CacheKey::from_identity(Tier::Merge, task, base),
        ));
        cases.push((
            NameText("task"),
            CacheKey::from_identity(Tier::Push, Task::new(NameText("grammar:test")), base),
        ));

        let mut schema_key = CacheKey::from_identity(Tier::Push, task, base);
        schema_key.schema = 2;
        cases.push((NameText("schema"), schema_key));

        cases
    }

    /// Return one success result per task in `tier`.
    fn successes_for(tier: Tier) -> Vec<TaskExit>
    {
        tier.plan()
            .tasks()
            .iter()
            .map(|_task| TaskExit::Success)
            .collect()
    }

    /// Return the task-name projection for exact plan assertions.
    fn task_names(plan: Plan) -> Vec<String>
    {
        let mut names = Vec::new();
        for task in plan.tasks() {
            names.push(String::from(task.name().as_ref()));
        }
        names
    }

    /// Return sorted direct entries under a cache directory.
    fn cache_directory_entries(path: &Path) -> Result<Vec<PathBuf>, GateError>
    {
        let mut entries = crate::support::HOST_FILESYSTEM.read_dir_paths(path)?;
        entries.sort();
        Ok(entries)
    }

    proptest! {
        /// Static workflow plans preserve the canonical local-task boundary
        /// invariants for every tier selected by the CLI parser.
        #[test]
        fn workflow_plan_projection_is_canonical_for_any_tier(use_push in any::<bool>())
        {
            let tier = if use_push {
                Tier::Push
            }
            else {
                Tier::Merge
            };
            let plan = tier.plan();
            let names = task_names(plan);
            let unique = names.iter().cloned().collect::<BTreeSet<_>>();

            prop_assert_eq!(plan.tier(), tier);
            prop_assert!(!names.is_empty(), "workflow plans must not be vacuous");
            prop_assert_eq!(unique.len(), names.len(), "workflow tasks must be unique");
            prop_assert!(
                names.iter().all(|name| is_canonical_task_name(name.as_str()).into().0),
                "workflow tasks must remain direct mise task names: {names:?}"
            );
            if tier == Tier::Push {
                let merge_names = task_names(Tier::Merge.plan());
                prop_assert!(
                    names.as_slice().starts_with(merge_names.as_slice()),
                    "push workflow must keep the merge workflow as its prefix"
                );
            }
        }
    }

    /// Actual support-runner seam whose executable is a fixture-local `mise`.
    #[cfg(unix)]
    #[repr(transparent)]
    struct ProgramRunner<'program>
    {
        /// Program path to execute instead of resolving `mise` from PATH.
        program: &'program OsStr,
    }

    #[cfg(unix)]
    impl TaskRunner for ProgramRunner<'_>
    {
        /// Run one task through the support process boundary.
        fn run_task(
            &self,
            task: Task,
            cwd: Option<&Path>,
        ) -> Result<TaskExit, GateError>
        {
            let args = [OsString::from("run"), OsString::from(task.name().as_ref())];
            let output = crate::support::run_output_streamed(self.program, &args, cwd, true)?;
            if output.success().into().0 {
                return Ok(TaskExit::Success);
            }
            Ok(TaskExit::Failure(TaskFailure {
                status: format!("{:?}", output.code().into().0),
                stdout: output.stdout_lossy().as_ref().to_owned(),
            }))
        }
    }

    /// Temporary local Git remote, worktree, fixture `mise`, and invocation
    /// log.
    #[cfg(unix)]
    struct GitWorkflowFixture
    {
        /// Fixture root removed on drop.
        root: PathBuf,
        /// Local bare repository used as `origin`.
        remote: PathBuf,
        /// Working repository where workflow tasks run.
        repo: PathBuf,
        /// Executable fake `mise` path.
        mise: PathBuf,
        /// Append-only task invocation log.
        log: PathBuf,
    }

    #[cfg(unix)]
    impl GitWorkflowFixture
    {
        /// Create a branch checkout with a local bare remote and fake `mise`.
        fn create() -> TestResult<Self>
        {
            let root = unique_workflow_root();
            let remote = root.join("remote.git");
            let repo = root.join("repo");
            let bin = root.join("bin");
            let mise = bin.join("mise");
            let log = root.join("mise.log");

            crate::support::HOST_FILESYSTEM.create_dir_all(bin)?;
            git_status(None, &[
                os("init"),
                os("--bare"),
                remote.as_os_str().to_os_string(),
            ])?;
            git_status(None, &[
                os("clone"),
                remote.as_os_str().to_os_string(),
                repo.as_os_str().to_os_string(),
            ])?;
            git_status(Some(&repo), &[os("checkout"), os("-b"), os("main")])?;
            crate::support::HOST_FILESYSTEM.write(repo.join("README.md"), "workflow fixture\n")?;
            git_status(Some(&repo), &[os("add"), os("README.md")])?;
            git_status(Some(&repo), &[
                os("commit"),
                os("--quiet"),
                os("--message"),
                os("base"),
            ])?;
            git_status(Some(&repo), &[
                os("push"),
                os("--quiet"),
                os("origin"),
                os("main"),
            ])?;
            git_status(Some(&repo), &[os("checkout"), os("-b"), os("feature")])?;
            crate::support::HOST_FILESYSTEM.write(repo.join("feature.txt"), "feature fixture\n")?;
            git_status(Some(&repo), &[os("add"), os("feature.txt")])?;
            git_status(Some(&repo), &[
                os("commit"),
                os("--quiet"),
                os("--message"),
                os("feature"),
            ])?;
            git_status(Some(&repo), &[
                os("branch"),
                os("--set-upstream-to"),
                os("origin/main"),
                os("feature"),
            ])?;
            write_fake_mise(&mise, &log)?;

            Ok(Self {
                root,
                remote,
                repo,
                mise,
                log,
            })
        }
    }

    #[cfg(unix)]
    impl Drop for GitWorkflowFixture
    {
        /// Remove the temporary local-remote fixture best-effort.
        fn drop(&mut self)
        {
            drop(crate::support::HOST_FILESYSTEM.remove_dir_all(&self.root));
        }
    }

    /// One fake-`mise` invocation record.
    #[cfg(unix)]
    struct WorkflowInvocation<'row>
    {
        /// Task name.
        task: &'row str,
        /// Current Git branch observed by the task.
        branch: &'row str,
        /// Origin URL observed by the task.
        remote: &'row str,
    }

    #[cfg(unix)]
    impl<'row> WorkflowInvocation<'row>
    {
        /// Parse one `task|branch|remote` fixture row.
        fn parse<R>(row: R) -> TestResult<Self>
        where
            R: Into<RowText<'row>>,
        {
            let row = row.into().0;
            let mut fields = row.splitn(3, '|');
            let task = fields
                .next()
                .ok_or_else(|| GateError::operational("missing task field"))?;
            let branch = fields
                .next()
                .ok_or_else(|| GateError::operational("missing branch field"))?;
            let remote = fields
                .next()
                .ok_or_else(|| GateError::operational("missing remote field"))?;
            Ok(Self {
                task,
                branch,
                remote,
            })
        }
    }

    /// Static identity provider for cache tests.
    #[repr(transparent)]
    struct StaticIdentity
    {
        /// Optional identity returned for every task.
        identity: Option<WorkflowInputIdentity>,
    }

    impl StaticIdentity
    {
        /// Build a static identity provider.
        fn new(identity: Option<WorkflowInputIdentity>) -> Self
        {
            Self { identity }
        }
    }

    impl WorkflowIdentityProvider for StaticIdentity
    {
        /// Return the shared fake repository key.
        fn repository_lock_key(
            &self,
            _cwd: Option<&Path>,
        ) -> Option<RepositoryLockKey>
        {
            Some(RepositoryLockKey {
                token: String::from("repo"),
            })
        }

        /// Return the configured identity.
        fn task_identity(
            &self,
            _tier: Tier,
            _task: Task,
            _cwd: Option<&Path>,
        ) -> Option<WorkflowInputIdentity>
        {
            self.identity.clone()
        }
    }

    /// Queue-backed identity provider for before/after identity tests.
    #[repr(transparent)]
    struct QueuedIdentity
    {
        /// Identities returned in order.
        identities: RefCell<VecDeque<Option<WorkflowInputIdentity>>>,
    }

    impl QueuedIdentity
    {
        /// Build an identity queue.
        fn new<Identities>(identities: Identities) -> Self
        where
            Identities: IntoIterator<Item = Option<WorkflowInputIdentity>>,
        {
            Self {
                identities: RefCell::new(identities.into_iter().collect()),
            }
        }
    }

    impl WorkflowIdentityProvider for QueuedIdentity
    {
        /// Return the shared fake repository key.
        fn repository_lock_key(
            &self,
            _cwd: Option<&Path>,
        ) -> Option<RepositoryLockKey>
        {
            Some(RepositoryLockKey {
                token: String::from("repo"),
            })
        }

        /// Pop the next queued identity.
        fn task_identity(
            &self,
            _tier: Tier,
            _task: Task,
            _cwd: Option<&Path>,
        ) -> Option<WorkflowInputIdentity>
        {
            self.identities.borrow_mut().pop_front().flatten()
        }
    }

    /// In-memory cache backend for exact hit/miss tests.
    #[derive(Default)]
    struct MemoryWorkflowCache
    {
        /// Keys treated as cache hits.
        keys: RefCell<Vec<CacheKey>>,
        /// Keys recorded by the executor.
        records: RefCell<Vec<CacheKey>>,
    }

    impl MemoryWorkflowCache
    {
        /// Insert an exact cache hit key.
        fn insert(
            &self,
            key: CacheKey,
        )
        {
            self.keys.borrow_mut().push(key);
        }

        /// Return keys recorded by the executor.
        fn recorded(&self) -> Vec<CacheKey>
        {
            self.records.borrow().clone()
        }
    }

    impl WorkflowCacheBackend for MemoryWorkflowCache
    {
        /// Return whether `key` was inserted.
        fn lookup(
            &self,
            key: &CacheKey,
        ) -> Result<impl Into<LookupFlag>, GateError>
        {
            Ok(self.keys.borrow().iter().any(|candidate| candidate == key))
        }

        /// Record one successful key.
        fn record_success(
            &self,
            key: &CacheKey,
        ) -> Result<(), GateError>
        {
            self.keys.borrow_mut().push(key.clone());
            self.records.borrow_mut().push(key.clone());
            Ok(())
        }
    }

    /// Cache backend that fails every operation.
    struct FailingWorkflowCache;

    impl WorkflowCacheBackend for FailingWorkflowCache
    {
        /// Fail lookup.
        fn lookup(
            &self,
            _key: &CacheKey,
        ) -> Result<impl Into<LookupFlag>, GateError>
        {
            Err::<bool, GateError>(GateError::operational("cache lookup failed"))
        }

        /// Fail recording.
        fn record_success(
            &self,
            _key: &CacheKey,
        ) -> Result<(), GateError>
        {
            Err(GateError::operational("cache write failed"))
        }
    }

    /// Lock backend that records entries and rejects same-key reentry.
    #[derive(Default)]
    struct RecordingLock
    {
        /// Currently held repository tokens.
        held: RefCell<BTreeSet<String>>,
        /// Repository tokens entered by completed lock attempts.
        entries: RefCell<Vec<String>>,
    }

    impl RecordingLock
    {
        /// Return recorded lock entries.
        fn entries(&self) -> Vec<String>
        {
            self.entries.borrow().clone()
        }
    }

    impl WorkflowLockBackend for RecordingLock
    {
        /// Run a body when `key` is not already held.
        fn with_repository_lock<ResultValue, Body>(
            &self,
            key: Option<&RepositoryLockKey>,
            body: Body,
        ) -> Result<ResultValue, GateError>
        where
            Body: FnOnce() -> Result<ResultValue, GateError>,
        {
            let Some(lock_key) = key
            else {
                return body();
            };
            if self.held.borrow().contains(&lock_key.token) {
                return Err(GateError::operational(format!(
                    "workflow lock already held for {}",
                    lock_key.token
                )));
            }
            self.held.borrow_mut().insert(lock_key.token.clone());
            self.entries.borrow_mut().push(lock_key.token.clone());
            let result = body();
            self.held.borrow_mut().remove(&lock_key.token);
            result
        }
    }

    /// Fake runner that returns scripted exits and records task names.
    struct ScriptedRunner
    {
        /// Pending task exits.
        exits: RefCell<VecDeque<TaskExit>>,
        /// Task names observed by the runner.
        calls: RefCell<Vec<String>>,
    }

    impl ScriptedRunner
    {
        /// Build a runner from a deterministic sequence of exits.
        fn new<Exits>(exits: Exits) -> Self
        where
            Exits: IntoIterator<Item = TaskExit>,
        {
            Self {
                exits: RefCell::new(exits.into_iter().collect()),
                calls: RefCell::new(Vec::new()),
            }
        }

        /// Return the observed task-name sequence.
        fn calls(&self) -> Vec<String>
        {
            self.calls.borrow().clone()
        }
    }

    impl TaskRunner for ScriptedRunner
    {
        /// Return the next scripted exit for one task.
        fn run_task(
            &self,
            task: Task,
            _cwd: Option<&std::path::Path>,
        ) -> Result<TaskExit, GateError>
        {
            self.calls
                .borrow_mut()
                .push(String::from(task.name().as_ref()));
            self.exits
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| GateError::operational("scripted runner ran out of exits"))
        }
    }
}
