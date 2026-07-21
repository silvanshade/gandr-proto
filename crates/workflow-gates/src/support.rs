//! Shared process and filesystem primitives for gate domains.
//!
//! # Contract
//! - ensures: command execution, file walking, text reads, and atomic writes
//!   use one typed error surface shared by every ported domain.
//! - provides: deterministic, sequential host operations with no
//!   caller-specific gate policy.
//! - fails: host I/O failures surface as [`crate::GateError`] values.
//! - panics: none.
//! - intension: helpers execute synchronously and never spawn background work.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read as _;
use std::io::Write as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::ChildStdout;
use std::process::Command;
use std::process::ExitStatus;
use std::process::Stdio;

use crate::GateError;

crate::semantic_copy!(pub struct SanitizedGitFlag(bool));

crate::semantic_str!(pub struct StdoutLossyViewText);

/// Display-safe captured stdout text decoded with UTF-8 replacement.
#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct StdoutLossyText<'stdout>(Cow<'stdout, str>);

impl StdoutLossyText<'_>
{
    /// Borrow the decoded stdout through a semantic text boundary.
    #[inline]
    #[must_use]
    pub fn text(&self) -> StdoutLossyViewText<'_>
    {
        StdoutLossyViewText(self.0.as_ref())
    }

    /// Convert the decoded stdout into an owned string.
    #[inline]
    #[must_use]
    pub fn into_owned(self) -> String
    {
        self.0.into_owned()
    }
}

impl fmt::Display for StdoutLossyText<'_>
{
    #[inline]
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result
    {
        f.write_str(self.0.as_ref())
    }
}

impl AsRef<str> for StdoutLossyText<'_>
{
    #[inline]
    fn as_ref(&self) -> &str
    {
        self.0.as_ref()
    }
}
crate::semantic_copy!(pub struct StreamStdoutFlag(bool));
crate::semantic_copy!(pub struct CaptureLimitCount(usize));
crate::semantic_copy!(pub struct CodeExitCode(Option<i32>));
crate::semantic_str!(pub struct ProgramText);
crate::semantic_bytes!(pub struct BytesBytes);

/// Owned bytes returned by one contextual host file read.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileBytes(Vec<u8>);

impl FileBytes
{
    /// Borrow the file contents through a semantic byte boundary.
    #[inline]
    #[must_use]
    pub fn as_bytes(&self) -> BytesBytes<'_>
    {
        BytesBytes(self.0.as_slice())
    }
}

impl From<Vec<u8>> for FileBytes
{
    #[inline]
    fn from(value: Vec<u8>) -> Self
    {
        Self(value)
    }
}

impl AsRef<[u8]> for FileBytes
{
    #[inline]
    fn as_ref(&self) -> &[u8]
    {
        self.0.as_slice()
    }
}

crate::semantic_copy!(pub struct PathExistsFlag(bool));
crate::semantic_copy!(pub struct CopiedByteCount(u64));
crate::semantic_copy!(pub struct AttemptCount(u16));
crate::semantic_copy!(pub struct ExceededLimitFlag(bool));
crate::semantic_str!(pub struct KeyText);
crate::semantic_str!(pub struct TestNameText);
crate::semantic_str!(pub struct NameText);
crate::semantic_copy!(pub struct SuccessFlag(bool));
crate::semantic_optional_copy!(pub struct OptionalCodeCode(i32));

/// Typed synchronous host-filesystem boundary shared by gates and fixtures.
///
/// # Contract
/// - ensures: every mutating filesystem operation attaches the affected path to
///   a [`GateError::Io`] failure.
/// - provides: one explicit effect boundary for production code and isolated
///   test workspaces.
/// - fails: returns [`GateError::Io`] when the host rejects an operation.
/// - panics: none.
/// - intension: method dispatch keeps host effects visible at the semantic
///   boundary while preventing callers from propagating context-free I/O
///   errors.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct HostFileSystem;

/// Shared stateless host-filesystem adapter.
pub const HOST_FILESYSTEM: HostFileSystem = HostFileSystem;

impl HostFileSystem
{
    /// Resolve the process working directory.
    ///
    /// # Contract
    /// - ensures: returns the host's current absolute working directory.
    /// - provides: contextual current-directory discovery.
    /// - fails: returns [`GateError::Io`] with `<current-dir>` on lookup
    ///   failure.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host cannot resolve the directory.
    #[inline]
    pub fn current_dir(&self) -> Result<PathBuf, GateError>
    {
        let context = Path::new("<current-dir>");
        std::env::current_dir().map_err(|source| io_error(context, source))
    }

    /// Resolve the running executable path.
    ///
    /// # Contract
    /// - ensures: returns the host path used to launch the current executable.
    /// - provides: contextual executable discovery for child fixtures.
    /// - fails: returns [`GateError::Io`] with `<current-exe>` on lookup
    ///   failure.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host cannot resolve the executable.
    #[inline]
    pub fn current_exe(&self) -> Result<PathBuf, GateError>
    {
        let context = Path::new("<current-exe>");
        std::env::current_exe().map_err(|source| io_error(context, source))
    }

    /// Read one UTF-8 file.
    ///
    /// # Contract
    /// - ensures: returns all decoded file contents after success.
    /// - provides: contextual whole-file text reads.
    /// - fails: returns [`GateError::Io`] with `path` for host or UTF-8
    ///   failures.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host cannot read valid UTF-8.
    #[inline]
    pub fn read_to_string(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<String, GateError>
    {
        let path = path.as_ref();
        fs::read_to_string(path).map_err(|source| io_error(path, source))
    }

    /// Read one file as bytes.
    ///
    /// # Contract
    /// - ensures: returns all file bytes after success.
    /// - provides: contextual whole-file byte reads.
    /// - fails: returns [`GateError::Io`] with `path` when reading fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects the read.
    #[inline]
    pub fn read(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<FileBytes, GateError>
    {
        let path = path.as_ref();
        fs::read(path)
            .map(FileBytes::from)
            .map_err(|source| io_error(path, source))
    }

    /// Resolve a path through the host filesystem.
    ///
    /// # Contract
    /// - ensures: returns the canonical absolute path after success.
    /// - provides: contextual canonicalization.
    /// - fails: returns [`GateError::Io`] with `path` when resolution fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host cannot canonicalize `path`.
    #[inline]
    pub fn canonicalize(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PathBuf, GateError>
    {
        let path = path.as_ref();
        path.canonicalize().map_err(|source| io_error(path, source))
    }

    /// Read metadata for one path.
    ///
    /// # Contract
    /// - ensures: returns metadata for the resolved filesystem entry.
    /// - provides: contextual metadata lookup.
    /// - fails: returns [`GateError::Io`] with `path` when lookup fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host cannot inspect `path`.
    #[inline]
    pub fn metadata(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<std::fs::Metadata, GateError>
    {
        let path = path.as_ref();
        fs::metadata(path).map_err(|source| io_error(path, source))
    }

    /// Return whether one path exists.
    ///
    /// # Contract
    /// - ensures: distinguishes absence from host lookup failure.
    /// - provides: contextual existence probes.
    /// - fails: returns [`GateError::Io`] with `path` when probing fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host cannot probe `path`.
    #[inline]
    pub fn try_exists(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<PathExistsFlag, GateError>
    {
        let path = path.as_ref();
        path.try_exists()
            .map(PathExistsFlag::from)
            .map_err(|source| io_error(path, source))
    }

    /// Read all direct directory-entry paths.
    ///
    /// # Contract
    /// - ensures: returns each direct entry once in host iteration order.
    /// - provides: contextual directory and entry reads.
    /// - fails: returns [`GateError::Io`] with the directory path when either
    ///   opening or advancing the iterator fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host cannot enumerate the directory.
    pub fn read_dir_paths(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<PathBuf>, GateError>
    {
        let path = path.as_ref();
        let entries = fs::read_dir(path).map_err(|source| io_error(path, source))?;
        let mut paths = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| io_error(path, source))?;
            paths.push(entry.path());
        }
        Ok(paths)
    }

    /// Create or truncate one file.
    ///
    /// # Contract
    /// - requires: `path` has an existing writable parent.
    /// - ensures: returns a writable empty file at `path`.
    /// - provides: contextual file-handle creation.
    /// - fails: returns [`GateError::Io`] with `path` when creation fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects file creation.
    #[inline]
    pub fn create_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<File, GateError>
    {
        let path = path.as_ref();
        File::create(path).map_err(|source| io_error(path, source))
    }
}

impl HostFileSystem
{
    /// Create one directory and all missing parents.
    ///
    /// # Contract
    /// - ensures: `path` exists as a directory after success.
    /// - provides: contextual directory-tree creation.
    /// - fails: returns [`GateError::Io`] with `path` when creation fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects directory creation.
    #[inline]
    pub fn create_dir_all(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), GateError>
    {
        let path = path.as_ref();
        fs::create_dir_all(path).map_err(|source| io_error(path, source))
    }

    /// Create exactly one directory.
    ///
    /// # Contract
    /// - requires: the parent directory already exists.
    /// - ensures: `path` exists as a directory after success.
    /// - provides: contextual single-directory creation.
    /// - fails: returns [`GateError::Io`] with `path` when creation fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects directory creation.
    #[inline]
    pub fn create_dir(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), GateError>
    {
        let path = path.as_ref();
        fs::create_dir(path).map_err(|source| io_error(path, source))
    }

    /// Replace a file with the supplied byte representation.
    ///
    /// # Contract
    /// - requires: `path` has an existing writable parent.
    /// - ensures: `path` contains exactly `contents` after success.
    /// - provides: contextual whole-file writes.
    /// - fails: returns [`GateError::Io`] with `path` when the write fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects the write.
    #[inline]
    pub fn write<Contents>(
        &self,
        path: impl AsRef<Path>,
        contents: Contents,
    ) -> Result<(), GateError>
    where
        Contents: AsRef<[u8]>,
    {
        let path = path.as_ref();
        fs::write(path, contents).map_err(|source| io_error(path, source))
    }

    /// Remove a directory tree.
    ///
    /// # Contract
    /// - ensures: `path` and its descendants are absent after success.
    /// - provides: contextual recursive directory removal.
    /// - fails: returns [`GateError::Io`] with `path` when removal fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects directory removal.
    #[inline]
    pub fn remove_dir_all(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), GateError>
    {
        let path = path.as_ref();
        fs::remove_dir_all(path).map_err(|source| io_error(path, source))
    }

    /// Remove a directory tree when present.
    ///
    /// # Contract
    /// - ensures: `path` is absent after success, including when it began
    ///   absent.
    /// - provides: idempotent contextual directory cleanup.
    /// - fails: returns [`GateError::Io`] with `path` for failures other than
    ///   absence.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when directory removal fails for a reason
    /// other than `NotFound`.
    #[inline]
    pub fn remove_dir_if_exists(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), GateError>
    {
        let path = path.as_ref();
        match fs::remove_dir_all(path) {
            | Ok(()) => Ok(()),
            | Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            | Err(source) => Err(io_error(path, source)),
        }
    }

    /// Remove one file.
    ///
    /// # Contract
    /// - ensures: `path` is absent after success.
    /// - provides: contextual file removal.
    /// - fails: returns [`GateError::Io`] with `path` when removal fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects file removal.
    #[inline]
    pub fn remove_file(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), GateError>
    {
        let path = path.as_ref();
        fs::remove_file(path).map_err(|source| io_error(path, source))
    }

    /// Remove one file when present.
    ///
    /// # Contract
    /// - ensures: `path` is absent after success, including when it began
    ///   absent.
    /// - provides: idempotent contextual file cleanup.
    /// - fails: returns [`GateError::Io`] with `path` for failures other than
    ///   absence.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when file removal fails for a reason other
    /// than `NotFound`.
    #[inline]
    pub fn remove_file_if_exists(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), GateError>
    {
        let path = path.as_ref();
        match fs::remove_file(path) {
            | Ok(()) => Ok(()),
            | Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
            | Err(source) => Err(io_error(path, source)),
        }
    }

    /// Rename one filesystem entry.
    ///
    /// # Contract
    /// - requires: `source` exists.
    /// - ensures: the source entry is available at `destination` after success.
    /// - provides: contextual filesystem publication and moves.
    /// - fails: returns [`GateError::Io`] with `destination` when renaming
    ///   fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects the rename.
    #[inline]
    pub fn rename(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
    ) -> Result<(), GateError>
    {
        let source = source.as_ref();
        let destination = destination.as_ref();
        fs::rename(source, destination).map_err(|error| io_error(destination, error))
    }

    /// Copy one regular file.
    ///
    /// # Contract
    /// - requires: `source` names a readable regular file.
    /// - ensures: `destination` contains the copied bytes after success.
    /// - provides: contextual regular-file copies.
    /// - fails: returns [`GateError::Io`] with `source` when copying fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects the copy.
    #[inline]
    pub fn copy(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
    ) -> Result<CopiedByteCount, GateError>
    {
        let source = source.as_ref();
        let destination = destination.as_ref();
        fs::copy(source, destination)
            .map(CopiedByteCount::from)
            .map_err(|error| io_error(source, error))
    }

    /// Replace filesystem permissions for one path.
    ///
    /// # Contract
    /// - ensures: `path` has `permissions` after success.
    /// - provides: contextual permission updates.
    /// - fails: returns [`GateError::Io`] with `path` when the update fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects the permission update.
    #[inline]
    pub fn set_permissions(
        &self,
        path: impl AsRef<Path>,
        permissions: std::fs::Permissions,
    ) -> Result<(), GateError>
    {
        let path = path.as_ref();
        fs::set_permissions(path, permissions).map_err(|source| io_error(path, source))
    }

    /// Create one symbolic link on Unix.
    ///
    /// # Contract
    /// - ensures: `destination` is a symbolic link to `source` after success.
    /// - provides: contextual Unix fixture-link creation.
    /// - fails: returns [`GateError::Io`] with `destination` when linking
    ///   fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects link creation.
    #[cfg(unix)]
    #[inline]
    pub fn symlink(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
    ) -> Result<(), GateError>
    {
        let source = source.as_ref();
        let destination = destination.as_ref();
        std::os::unix::fs::symlink(source, destination)
            .map_err(|error| io_error(destination, error))
    }

    /// Change the dedicated process working directory.
    ///
    /// # Contract
    /// - requires: the caller is an isolated child process with no concurrent
    ///   tests or threads that observe the process working directory.
    /// - ensures: later relative paths resolve under `path` after success.
    /// - provides: explicit working-directory mutation for child-process
    ///   fixtures only.
    /// - fails: returns [`GateError::Io`] with `path` when the update fails.
    /// - panics: none.
    ///
    /// # Errors
    /// Returns [`GateError::Io`] when the host rejects the directory update.
    #[inline]
    pub fn set_isolated_process_current_dir(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), GateError>
    {
        let path = path.as_ref();
        std::env::set_current_dir(path).map_err(|source| io_error(path, source))
    }
}

/// Repository-control environment variables removed from commands that must
/// ignore ambient Git state.
const GIT_ENVIRONMENT_KEYS: [&str; 9] = [
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_DIR",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NAMESPACE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_REPLACE_REF_BASE",
    "GIT_WORK_TREE",
];
/// Per-invocation Git configuration for generated commits.
const STATELESS_GIT_CONFIG_ARGUMENTS: [&str; 6] = [
    "-c",
    "user.name=gandr-agent",
    "-c",
    "user.email=gandr-agent@gandr.invalid",
    "-c",
    "commit.gpgsign=false",
];

/// Number of deterministic temporary-name candidates tried before reporting a
/// collision failure.
const TEMPORARY_FILE_ATTEMPTS: u16 = 1024;

/// Maximum stdout bytes retained for callers that must parse command output.
///
/// The child stdout stream is still forwarded to the parent as bytes arrive;
/// this cap only limits the in-memory semantic copy returned to callers.
pub const RUN_OUTPUT_STDOUT_CAPTURE_LIMIT_BYTES: usize = 8 * 1024 * 1024;

/// Bytes read from a child stdout pipe before each parent-stdout write+flush.
const RUN_OUTPUT_STDOUT_CHUNK_BYTES: usize = 16 * 1024;

/// Captured output from a completed command.
#[derive(Debug)]
pub struct CommandOutput
{
    /// Process termination status.
    status: ExitStatus,
    /// Bounded captured standard output bytes retained for semantic parsers.
    stdout: Vec<u8>,
}

impl CommandOutput
{
    /// Return whether the process exited successfully.
    #[inline]
    #[must_use]
    pub fn success(&self) -> impl Into<SuccessFlag>
    {
        self.status.success()
    }

    /// Return the platform exit code when the process terminated normally.
    #[inline]
    #[must_use]
    pub fn code(&self) -> impl Into<OptionalCodeCode>
    {
        self.status.code()
    }

    /// Decode captured standard output with UTF-8 replacement for invalid
    /// bytes.
    ///
    /// # Contract
    /// - ensures: returns borrowed UTF-8 text when stdout is valid UTF-8, and
    ///   replacement-character text when it is not.
    /// - provides: a display-safe stdout view without changing the stored
    ///   bytes.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L3 only — valid and invalid byte fixtures kill mutants
    ///   that force allocation or skip replacement decoding.
    /// - witness: `gandr_workflow_gates::support::tests::command_output_lossy_streams_decode_bytes`
    #[inline]
    #[must_use]
    pub fn stdout_lossy(&self) -> StdoutLossyText<'_>
    {
        StdoutLossyText(String::from_utf8_lossy(&self.stdout))
    }
}

/// Run a command and retain bounded machine-readable stdout without forwarding
/// it.
///
/// # Contract
/// - requires: `program` names an executable accepted by [`Command::new`], and
///   every entry in `args` is passed as one argv item.
/// - extension: returns the child's status and at most
///   [`RUN_OUTPUT_STDOUT_CAPTURE_LIMIT_BYTES`] stdout bytes for semantic
///   parsing; stderr is inherited by the child and is never retained.
/// - ensures: stdout is drained without being written to the parent's stdout.
/// - provides: the shared process-output primitive for semantic streams such as
///   Cargo JSON, Git object IDs, Typst JSON, or msb lists.
/// - fails: returns [`GateError::Io`] when the process cannot be spawned, read,
///   or waited on; returns [`GateError::Operational`] when stdout exceeds the
///   retention cap.
/// - panics: none.
/// - intension: applies Git environment sanitization before spawning when
///   `sanitized_git` is `true`, then drains one stdout pipe through fixed-size
///   reads without exposing machine protocol to the terminal.
///
/// # Errors
/// Returns [`GateError::Io`] for process and pipe I/O failures, or a typed
/// operational error when retained stdout would exceed the explicit cap.
///
/// # Adequacy
/// - hypothesis: L3 only — child-test fixtures kill mutants that drop argv,
///   status, stdout retention, cap/drain behavior, or requested Git environment
///   removal.
/// - witness: `gandr_workflow_gates::support::tests::run_output_retains_machine_stdout`
/// - witness: `gandr_workflow_gates::support::tests::run_output_drains_then_errors_when_stdout_exceeds_limit`
/// - witness: `gandr_workflow_gates::support::tests::git_environment_sanitizer_removes_only_git_keys`
#[inline]
pub fn run_output(
    program: &OsStr,
    args: &[OsString],
    cwd: Option<&Path>,
    sanitized_git: impl Into<SanitizedGitFlag>,
) -> Result<CommandOutput, GateError>
{
    let sanitized_git = sanitized_git.into().0;
    run_output_with_capture_limit(
        program,
        args,
        cwd,
        sanitized_git,
        false,
        RUN_OUTPUT_STDOUT_CAPTURE_LIMIT_BYTES,
    )
}

/// Run a command, forward stdout live, and retain bounded failure context.
///
/// # Contract
/// - requires: the same process arguments as [`run_output`].
/// - extension: returns the same bounded output while writing and flushing
///   every child stdout chunk to the parent terminal.
/// - provides: human-facing workflow task execution without using this mode for
///   machine-readable semantic probes.
/// - fails: returns the same process, stream, and capture-limit errors as
///   [`run_output`].
/// - panics: none.
///
/// # Errors
/// Returns process, stream, or capture-limit errors from the shared runner.
#[inline]
pub fn run_output_streamed(
    program: &OsStr,
    args: &[OsString],
    cwd: Option<&Path>,
    sanitized_git: impl Into<SanitizedGitFlag>,
) -> Result<CommandOutput, GateError>
{
    let sanitized_git = sanitized_git.into().0;
    run_output_with_capture_limit(
        program,
        args,
        cwd,
        sanitized_git,
        true,
        RUN_OUTPUT_STDOUT_CAPTURE_LIMIT_BYTES,
    )
}

/// Run a command with an injected stdout retention cap and forwarding mode.
///
/// # Contract
/// - requires: `capture_limit` is the maximum stdout bytes the caller is
///   willing to retain in memory.
/// - extension: under `stream_stdout`, forwards retained and post-cap bytes;
///   otherwise drains them silently.
/// - ensures: stdout is drained in fixed-size chunks even after the cap has
///   been crossed.
/// - fails: returns the same typed process, stream, and size-limit errors as
///   [`run_output`].
/// - panics: none.
/// - intension: this is the single implementation path behind [`run_output`]
///   and [`run_output_streamed`].
///
/// # Errors
/// Returns [`GateError::Io`] for process and stream I/O failures, or
/// [`GateError::Operational`] when retained stdout would exceed
/// `capture_limit`.
///
/// # Adequacy
/// - hypothesis: L3 only — the injected-cap child fixture forces the overflow
///   branch while proving the child is drained before the error is returned.
/// - witness: `gandr_workflow_gates::support::tests::run_output_drains_then_errors_when_stdout_exceeds_limit`
fn run_output_with_capture_limit(
    program: &OsStr,
    args: &[OsString],
    cwd: Option<&Path>,
    sanitized_git: impl Into<SanitizedGitFlag>,
    stream_stdout: impl Into<StreamStdoutFlag>,
    capture_limit: impl Into<CaptureLimitCount>,
) -> Result<CommandOutput, GateError>
{
    let stream_stdout = stream_stdout.into().0;
    let capture_limit = capture_limit.into().0;
    let sanitized_git = sanitized_git.into().0;
    let mut command = build_command(program, args, cwd, sanitized_git);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::inherit());

    let mut child = command
        .spawn()
        .map_err(|source| io_error(&command_path(program), source))?;
    let Some(stdout) = child.stdout.take()
    else {
        let _wait_result = child.wait();
        return Err(GateError::operational(format!(
            "{} stdout was not piped for streaming",
            program.to_string_lossy()
        )));
    };

    let stream_result = drain_child_stdout(stdout, program, capture_limit, stream_stdout);
    let status = child
        .wait()
        .map_err(|source| io_error(&command_path(program), source))?;
    let streamed = stream_result?;
    if streamed.exceeded_limit {
        return Err(stdout_capture_limit_error(
            program,
            capture_limit,
            status.code(),
        ));
    }
    Ok(CommandOutput {
        status,
        stdout: streamed.stdout,
    })
}

/// Build the typed stdout retention-limit error.
fn stdout_capture_limit_error(
    program: &OsStr,
    capture_limit: impl Into<CaptureLimitCount>,
    code: impl Into<CodeExitCode>,
) -> GateError
{
    let code = code.into().0;
    let capture_limit = capture_limit.into().0;
    let program_label = program.to_string_lossy();
    GateError::operational(format!(
        "{program_label} stdout exceeded {capture_limit} byte capture limit while draining ({}); semantic output was not retained",
        command_status_detail(program_label.as_ref(), code)
    ))
}

/// Render a process status without relying on captured stderr.
///
/// # Contract
/// - requires: `program` is the diagnostic command label and `code` comes from
///   [`ExitStatus::code`].
/// - extension: returns stable process detail suitable for command-failure
///   diagnostics after stderr has streamed live to the parent.
/// - ensures: normal exits include the numeric status and signal-like exits
///   state that no platform code was available.
/// - panics: none.
/// - intension: allocates only the returned diagnostic string.
///
/// # Adequacy
/// - hypothesis: L2 — command-failure callsite tests observe status-code
///   diagnostics instead of captured stderr.
#[inline]
#[must_use]
pub(crate) fn command_status_detail<'semantic>(
    program: impl Into<ProgramText<'semantic>>,
    code: impl Into<CodeExitCode>,
) -> String
{
    let code = code.into().0;
    let program = program.into().0;
    match code {
        | Some(exit_code) => format!("{program} exited with status {exit_code}"),
        | None => format!("{program} terminated without an exit code"),
    }
}

/// Run a command and return only its exit status.
///
/// # Contract
/// - requires: `program` names an executable accepted by [`Command::new`], and
///   every entry in `args` is passed as one argv item.
/// - extension: returns the status without treating non-zero status as an
///   error; stdout and stderr are inherited by the child and never retained.
/// - provides: the shared process-status primitive for gate domains.
/// - fails: returns [`GateError::Io`] when the process cannot be spawned or
///   waited on.
/// - panics: none.
/// - intension: applies Git environment sanitization before spawning when
///   `sanitized_git` is `true`, and otherwise preserves ambient environment
///   through [`Command::status`].
///
/// # Errors
/// Returns [`GateError::Io`] when process execution fails before an
/// [`ExitStatus`] is available.
///
/// # Adequacy
/// - hypothesis: L3 only — child-test fixtures kill mutants that invert success
///   status or drop argv propagation.
/// - witness: `gandr_workflow_gates::support::tests::run_status_reports_child_success`
#[inline]
pub fn run_status(
    program: &OsStr,
    args: &[OsString],
    cwd: Option<&Path>,
    sanitized_git: impl Into<SanitizedGitFlag>,
) -> Result<ExitStatus, GateError>
{
    let sanitized_git = sanitized_git.into().0;
    let mut command = build_command(program, args, cwd, sanitized_git);
    command
        .status()
        .map_err(|source| io_error(&command_path(program), source))
}

/// Walk a directory tree and return matching files in sorted order.
///
/// # Contract
/// - requires: `root` names a directory to enumerate and `extension` is
///   compared to [`Path::extension`] exactly.
/// - ensures: returns all regular files below non-symlink directories whose
///   extension equals `extension`, sorted by full path.
/// - provides: deterministic source discovery for gate domains.
/// - fails: returns [`GateError::Io`] when directory enumeration or metadata
///   reads fail.
/// - panics: none.
/// - intension: uses an explicit worklist rather than recursion and never
///   descends into symlinked directories.
///
/// # Errors
/// Returns [`GateError::Io`] for failed `read_dir` or `symlink_metadata` calls.
///
/// # Adequacy
/// - hypothesis: L3 only — nested, out-of-order, wrong-extension, and symlinked
///   directory fixtures kill mutants that recurse, forget sorting, or follow
///   directory symlinks.
/// - witness: `gandr_workflow_gates::support::tests::walk_files_sorts_and_skips_symlinked_directories`
#[inline]
pub fn walk_files(
    root: &Path,
    extension: &OsStr,
) -> Result<Vec<PathBuf>, GateError>
{
    if symlink_metadata(root)?.file_type().is_symlink() {
        return Ok(Vec::new());
    }

    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .map_err(|source| io_error(&directory, source))?
            .map(|entry| {
                entry
                    .map(|directory_entry| directory_entry.path())
                    .map_err(|source| io_error(&directory, source))
            })
            .collect::<Result<Vec<_>, GateError>>()?;
        entries.sort();

        for entry_path in entries {
            let entry_metadata = symlink_metadata(&entry_path)?;
            let entry_type = entry_metadata.file_type();
            if entry_type.is_symlink() {
                continue;
            }
            if entry_metadata.is_dir() {
                pending.push(entry_path);
            }
            else if entry_metadata.is_file() && entry_path.extension() == Some(extension) {
                files.push(entry_path);
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Read a file as UTF-8 text.
///
/// # Contract
/// - requires: `path` names a file readable by the current process.
/// - ensures: returns the complete file contents when they are valid UTF-8.
/// - provides: the shared typed text-read primitive for gate domains.
/// - fails: returns [`GateError::Io`] for read errors and invalid UTF-8 data.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Io`] when the file cannot be read as UTF-8 text.
///
/// # Adequacy
/// - hypothesis: L3 only — valid text and missing-file fixtures kill mutants
///   that skip read errors or truncate the loaded string.
/// - witness: `gandr_workflow_gates::support::tests::read_utf8_loads_full_text`
#[inline]
pub fn read_utf8(path: &Path) -> Result<String, GateError>
{
    fs::read_to_string(path).map_err(|source| io_error(path, source))
}

/// Atomically write bytes to a path through a unique sibling temporary file.
///
/// # Contract
/// - requires: `path` has a final file-name component and its parent directory
///   exists.
/// - ensures: writes all `bytes` to a sibling temporary file, flushes and syncs
///   it, then renames it over `path`.
/// - provides: crash-aware publication for generated gate artifacts.
/// - fails: returns [`GateError::Io`] for filesystem failures, including
///   temporary cleanup failures, or [`GateError::Operational`] when no unique
///   temporary name can be created.
/// - panics: none.
/// - intension: temporary names are deterministic BLAKE3-derived siblings;
///   after a post-creation failure, cleanup is attempted before returning.
///
/// # Errors
/// Returns [`GateError::Io`] for filesystem and cleanup failures, and
/// [`GateError::Operational`] if all temporary-name candidates already exist.
///
/// # Adequacy
/// - hypothesis: L3 only — overwrite, cleanup-on-rename-failure, and missing
///   file-name fixtures kill mutants that publish before sync, leak
///   temporaries, or reuse the target as its own staging path.
/// - witness: `gandr_workflow_gates::support::tests::write_atomic_replaces_file_and_removes_temporary`
/// - witness: `gandr_workflow_gates::support::tests::write_atomic_cleans_temporary_after_rename_failure`
#[inline]
pub fn write_atomic<'semantic>(
    path: &Path,
    bytes: impl Into<BytesBytes<'semantic>>,
) -> Result<(), GateError>
{
    let bytes = bytes.into().0;
    let target_directory_path = target_directory(path);
    let target_name = target_file_name(path)?;

    for attempt in 0_u16 .. TEMPORARY_FILE_ATTEMPTS {
        let temporary_path = temporary_file_path(target_directory_path, target_name, attempt);
        let temporary_file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            | Ok(file) => file,
            | Err(source) if source.kind() == std::io::ErrorKind::AlreadyExists => continue,
            | Err(source) => return Err(io_error(&temporary_path, source)),
        };

        if let Err(error) = write_and_publish(temporary_file, &temporary_path, path, bytes) {
            remove_temporary_file(&temporary_path)?;
            return Err(error);
        }
        return Ok(());
    }

    Err(GateError::operational(format!(
        "could not create unique temporary file for {}",
        path.display()
    )))
}

/// Build a command with shared argv, working-directory, and Git environment
/// handling.
///
/// # Contract
/// - requires: `program` and `args` are already tokenized for direct process
///   execution.
/// - ensures: returns a command configured with every argument, optional
///   working directory, and requested Git environment removal.
/// - provides: a single construction path for output- and status-only runners.
/// - panics: none.
/// - intension: performs no shell interpretation.
///
/// # Adequacy
/// - hypothesis: L3 only — output/status child fixtures and the sanitizer child
///   fixture kill mutants that drop argv, cwd, or Git environment removal.
/// - witness: `gandr_workflow_gates::support::tests::run_output_streams_stdout_and_retains_parse_bytes`
/// - witness: `gandr_workflow_gates::support::tests::git_environment_sanitizer_removes_only_git_keys`
fn build_command(
    program: &OsStr,
    args: &[OsString],
    cwd: Option<&Path>,
    sanitized_git: impl Into<SanitizedGitFlag>,
) -> Command
{
    let sanitized_git = sanitized_git.into().0;
    let mut command = Command::new(program);
    command.args(args);
    if let Some(directory) = cwd {
        command.current_dir(directory);
    }
    if sanitized_git {
        sanitize_git_environment(&mut command);
    }
    command
}

/// Drain child stdout, optionally forwarding it, while retaining bounded bytes.
///
/// # Contract
/// - requires: `stdout` is the only read end of the spawned child's stdout
///   pipe.
/// - extension: returns the retained bytes and a precise overflow flag.
/// - ensures: when `stream_stdout` is true, every read chunk is written and
///   flushed to the parent's stdout before the next child read.
/// - ensures: when `stream_stdout` is false, no child stdout is forwarded.
/// - ensures: after the cap is crossed, later chunks are still drained but are
///   not appended to retained memory.
/// - fails: returns [`GateError::Io`] for child stdout read failures or, in
///   streaming mode, parent-stdout write/flush failures.
/// - panics: none.
/// - intension: uses one fixed-size stack buffer and allocates no retained
///   storage after the capture limit is reached.
///
/// # Errors
/// Returns [`GateError::Io`] for child read or optional parent write failures.
///
/// # Adequacy
/// - hypothesis: L3 only — capture, stream, and over-cap child fixtures
///   distinguish retention, forwarding, flushing, and post-cap draining.
/// - witness: `gandr_workflow_gates::support::tests::run_output_retains_machine_stdout`
/// - witness: `gandr_workflow_gates::support::tests::run_output_streamed_retains_failure_context`
/// - witness: `gandr_workflow_gates::support::tests::run_output_drains_then_errors_when_stdout_exceeds_limit`
fn drain_child_stdout(
    mut stdout: ChildStdout,
    program: &OsStr,
    capture_limit: impl Into<CaptureLimitCount>,
    stream_stdout: impl Into<StreamStdoutFlag>,
) -> Result<StreamedStdout, GateError>
{
    let stream_stdout = stream_stdout.into().0;
    let capture_limit = capture_limit.into().0;
    let mut retained = Vec::new();
    let mut exceeded_limit = false;
    let mut stream_error = None;
    let mut buffer = [0_u8; RUN_OUTPUT_STDOUT_CHUNK_BYTES];
    let mut parent_stdout = stream_stdout.then(|| std::io::stdout().lock());

    loop {
        let count = stdout
            .read(&mut buffer)
            .map_err(|source| io_error(&command_path(program), source))?;
        if count == 0 {
            break;
        }
        let Some(chunk) = buffer.get(.. count)
        else {
            return Err(GateError::operational(format!(
                "internal stdout chunk exceeded buffer for {}",
                program.to_string_lossy()
            )));
        };
        if stream_error.is_none()
            && let Some(parent_stdout) = parent_stdout.as_mut()
        {
            if let Err(source) = parent_stdout.write_all(chunk) {
                stream_error = Some(io_error(&command_path(program), source));
            }
            else if let Err(source) = parent_stdout.flush() {
                stream_error = Some(io_error(&command_path(program), source));
            }
        }

        if retained.len() < capture_limit {
            let remaining = capture_limit.saturating_sub(retained.len());
            let keep = remaining.min(count);
            let Some(retained_chunk) = chunk.get(.. keep)
            else {
                return Err(GateError::operational(format!(
                    "internal retained stdout chunk exceeded buffer for {}",
                    program.to_string_lossy()
                )));
            };
            retained.extend_from_slice(retained_chunk);
            if keep < count {
                exceeded_limit = true;
            }
        }
        else {
            exceeded_limit = true;
        }
    }

    if let Some(error) = stream_error {
        return Err(error);
    }
    Ok(StreamedStdout {
        stdout: retained,
        exceeded_limit,
    })
}

/// Remove ambient Git repository overrides from a command.
///
/// # Contract
/// - ensures: removes Git repository-control variables including `GIT_DIR`,
///   `GIT_WORK_TREE`, `GIT_INDEX_FILE`, and object-directory overrides from the
///   child command environment.
/// - provides: repository-neutral process execution for gates that invoke Git.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — an injected command environment with repository,
///   index, worktree, object-directory, namespace, and replacement controls
///   plus preserved config/non-Git variables kills mutants that remove too few
///   or too many names.
/// - witness: `gandr_workflow_gates::support::tests::git_environment_sanitizer_removes_only_git_keys`
pub(crate) fn sanitize_git_environment(command: &mut Command)
{
    for key in GIT_ENVIRONMENT_KEYS {
        command.env_remove(key);
    }
}

/// Build a Git command with stateless identity and signing policy.
///
/// # Contract
/// - ensures: every invocation uses `gandr-agent <gandr-agent@gandr.invalid>`
///   and disables commit signing through command-line configuration.
/// - provides: a repository-neutral Git command whose identity and signing
///   policy do not mutate local or global Git configuration.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L3 only — a hostile global identity plus mandatory signing
///   distinguishes command-line overrides from ambient or repository-local
///   configuration.
/// - witness: `gandr_workflow_gates::support::tests::stateless_git_command_overrides_ambient_identity_and_signing`
#[cfg(test)]
pub(crate) fn stateless_git_command() -> Command
{
    let mut command = Command::new("git");
    command
        .args(STATELESS_GIT_CONFIG_ARGUMENTS)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0");
    sanitize_git_environment(&mut command);
    command
}

/// Prefix owned Git arguments with the stateless identity and signing policy.
///
/// # Contract
/// - ensures: returns command-line configuration before every argument in
///   `command_args`.
/// - provides: the slice-backed adapter for injected Git command hosts.
/// - panics: none.
pub(crate) fn stateless_git_args(command_args: &[OsString]) -> Vec<OsString>
{
    STATELESS_GIT_CONFIG_ARGUMENTS
        .into_iter()
        .map(OsString::from)
        .chain(command_args.iter().cloned())
        .collect()
}

/// Return symlink metadata with a gate I/O error on failure.
///
/// # Contract
/// - ensures: returns `symlink_metadata` for `path` without following a final
///   symlink.
/// - provides: typed metadata reads for symlink-safe walkers.
/// - fails: returns [`GateError::Io`] when metadata cannot be read.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Io`] when `std::fs::symlink_metadata` fails.
///
/// # Adequacy
/// - hypothesis: L3 only — walk fixtures with files, directories, and symlinked
///   directories kill mutants that follow symlinks or erase metadata errors.
/// - witness: `gandr_workflow_gates::support::tests::walk_files_sorts_and_skips_symlinked_directories`
fn symlink_metadata(path: &Path) -> Result<std::fs::Metadata, GateError>
{
    fs::symlink_metadata(path).map_err(|source| io_error(path, source))
}

/// Render a process program as a path payload for spawn errors.
fn command_path(program: &OsStr) -> PathBuf
{
    Path::new(program).to_path_buf()
}

/// Return the directory that owns a target path.
fn target_directory(path: &Path) -> &Path
{
    match path.parent() {
        | Some(parent) if !parent.as_os_str().is_empty() => parent,
        | Some(_) | None => Path::new("."),
    }
}

/// Return the final file-name component required for atomic staging.
///
/// # Contract
/// - requires: `path` can be relative or absolute.
/// - ensures: returns the final file-name component when one exists.
/// - provides: the sibling-name seed for [`write_atomic`].
/// - fails: returns [`GateError::Operational`] when `path` has no file name.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Operational`] when `path` has no file-name component.
///
/// # Adequacy
/// - hypothesis: L3 only — root-only and normal file fixtures kill mutants that
///   silently stage beside an unnamed path.
/// - witness: `gandr_workflow_gates::support::tests::write_atomic_requires_file_name`
fn target_file_name(path: &Path) -> Result<&OsStr, GateError>
{
    path.file_name().ok_or_else(|| {
        GateError::operational(format!(
            "atomic write target has no file name: {}",
            path.display()
        ))
    })
}

/// Build one deterministic sibling temporary path.
///
/// # Contract
/// - ensures: returns a path under `directory` whose file name differs from
///   `target_name` and includes a BLAKE3 token for this target and attempt.
/// - provides: collision-resistant candidate names for [`write_atomic`].
/// - panics: none.
/// - intension: does not inspect the filesystem.
///
/// # Adequacy
/// - hypothesis: L3 only — atomic-write witnesses observe that staging does not
///   collide with or leak beside the target.
/// - witness: `gandr_workflow_gates::support::tests::write_atomic_replaces_file_and_removes_temporary`
fn temporary_file_path(
    directory: &Path,
    target_name: &OsStr,
    attempt: impl Into<AttemptCount>,
) -> PathBuf
{
    let attempt = attempt.into().0;
    let mut hasher = blake3::Hasher::new();
    hasher.update(directory.as_os_str().as_encoded_bytes());
    hasher.update(b"\0");
    hasher.update(target_name.as_encoded_bytes());
    hasher.update(b"\0");
    hasher.update(&std::process::id().to_le_bytes());
    hasher.update(&attempt.to_le_bytes());
    let token = hasher.finalize();
    let token_hex = token.to_hex();

    let mut temporary_name = OsString::new();
    temporary_name.push(".");
    temporary_name.push(target_name);
    temporary_name.push(".gandr-workflow-gates-");
    temporary_name.push(token_hex.as_str());
    temporary_name.push(".tmp");
    directory.join(temporary_name)
}

/// Build an I/O gate error for one filesystem or process path.
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

/// Write, flush, sync, and rename one already-created temporary file.
///
/// # Contract
/// - requires: `temporary_file` is opened for `temporary_path`, and
///   `temporary_path` is a sibling of `target_path`.
/// - ensures: writes every byte, flushes userspace buffers, syncs the file, and
///   renames the temporary file to `target_path`.
/// - provides: the publication sequence for [`write_atomic`].
/// - fails: returns [`GateError::Io`] for write, flush, sync, or rename
///   failures.
/// - panics: none.
/// - intension: drops the file handle before renaming.
///
/// # Errors
/// Returns [`GateError::Io`] for write, flush, sync, or rename failures.
///
/// # Adequacy
/// - hypothesis: L3 only — successful replacement and rename-failure fixtures
///   kill mutants that skip write, sync, drop, rename, or cleanup delegation.
/// - witness: `gandr_workflow_gates::support::tests::write_atomic_replaces_file_and_removes_temporary`
/// - witness: `gandr_workflow_gates::support::tests::write_atomic_cleans_temporary_after_rename_failure`
fn write_and_publish<'semantic>(
    mut temporary_file: File,
    temporary_path: &Path,
    target_path: &Path,
    bytes: impl Into<BytesBytes<'semantic>>,
) -> Result<(), GateError>
{
    let bytes = bytes.into().0;
    temporary_file
        .write_all(bytes)
        .map_err(|source| io_error(temporary_path, source))?;
    temporary_file
        .flush()
        .map_err(|source| io_error(temporary_path, source))?;
    temporary_file
        .sync_all()
        .map_err(|source| io_error(temporary_path, source))?;
    drop(temporary_file);
    HOST_FILESYSTEM.rename(temporary_path, target_path)
}

/// Remove a temporary file after a failed atomic write.
///
/// # Contract
/// - ensures: removes `path` when present and treats an already-absent path as
///   a successful cleanup.
/// - provides: cleanup for [`write_atomic`] after failed publication.
/// - fails: returns [`GateError::Io`] when cleanup fails for any reason other
///   than absence.
/// - panics: none.
///
/// # Errors
/// Returns [`GateError::Io`] when the temporary file cannot be removed.
///
/// # Adequacy
/// - hypothesis: L3 only — the rename-failure fixture kills mutants that leak
///   temporary files after publication failure.
/// - witness: `gandr_workflow_gates::support::tests::write_atomic_cleans_temporary_after_rename_failure`
fn remove_temporary_file(path: &Path) -> Result<(), GateError>
{
    HOST_FILESYSTEM.remove_file_if_exists(path)
}

/// Stdout retained from a drained child plus whether the retention cap crossed.
struct StreamedStdout
{
    /// Bytes retained for semantic parsing or bounded failure context.
    stdout: Vec<u8>,
    /// Whether the child emitted more bytes than `stdout` can retain.
    exceeded_limit: bool,
}

#[cfg(test)]
mod tests
{
    //! Unit witnesses for shared process and filesystem support.

    use alloc::boxed::Box;
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::error::Error;
    use std::ffi::OsString;
    use std::path::Path;
    use std::path::PathBuf;
    use std::process::Command;

    use super::*;

    /// Output views preserve valid text without allocating for already-UTF-8
    /// data.
    #[test]
    fn command_output_lossy_streams_decode_bytes() -> Result<(), Box<dyn Error>>
    {
        let child = HOST_FILESYSTEM.current_exe()?;
        let args = child_test_args("child_success");
        let status = run_status(child.as_os_str(), &args, None, false)?;
        let output = CommandOutput {
            status,
            stdout: b"valid stdout".to_vec(),
        };

        assert_eq!("valid stdout", output.stdout_lossy().text().as_ref());
        Ok(())
    }

    /// `run_output` retains machine-readable stdout without exposing it.
    #[test]
    fn run_output_retains_machine_stdout() -> Result<(), Box<dyn Error>>
    {
        let child = HOST_FILESYSTEM.current_exe()?;
        let args = child_test_args("child_prints_streams");
        let output = run_output(child.as_os_str(), &args, None, false)?;

        assert!(output.success().into().0);
        assert_eq!(Some(0_i32), output.code().into().0);
        assert!(
            output
                .stdout_lossy()
                .text()
                .as_ref()
                .contains("support stdout probe")
        );
        Ok(())
    }

    /// `run_output_streamed` forwards stdout and retains failure context.
    #[test]
    fn run_output_streamed_retains_failure_context() -> Result<(), Box<dyn Error>>
    {
        let child = HOST_FILESYSTEM.current_exe()?;
        let args = child_test_args("child_prints_streams");
        let output = run_output_streamed(child.as_os_str(), &args, None, false)?;

        assert!(output.success().into().0);
        assert!(
            output
                .stdout_lossy()
                .text()
                .as_ref()
                .contains("support stdout probe")
        );
        Ok(())
    }

    /// `run_output` drains over-cap stdout before returning the typed cap
    /// error.
    #[test]
    fn run_output_drains_then_errors_when_stdout_exceeds_limit() -> Result<(), Box<dyn Error>>
    {
        let child = HOST_FILESYSTEM.current_exe()?;
        let args = child_test_args("child_writes_large_stdout");
        let result =
            run_output_with_capture_limit(child.as_os_str(), &args, None, false, false, 32_usize);

        assert!(matches!(
            result,
            Err(GateError::Operational { detail })
                if detail.contains("stdout exceeded 32 byte capture limit")
        ));
        Ok(())
    }

    /// `run_status` returns child success and failure status without capture.
    #[test]
    fn run_status_reports_child_status() -> Result<(), Box<dyn Error>>
    {
        let child = HOST_FILESYSTEM.current_exe()?;
        let success_args = child_test_args("child_success");
        let success = run_status(child.as_os_str(), &success_args, None, false)?;
        assert!(success.success());
        assert_eq!(Some(0_i32), success.code());

        let failure_args = child_test_args("child_exits_with_status");
        let failure = run_status(child.as_os_str(), &failure_args, None, false)?;
        assert!(!failure.success());
        assert_eq!(Some(7_i32), failure.code());
        Ok(())
    }

    /// Git sanitizer removes repository-control overrides from a child command.
    #[test]
    fn git_environment_sanitizer_removes_only_git_keys()
    {
        let mut command = Command::new("git");
        for key in GIT_ENVIRONMENT_KEYS {
            command.env(key, format!("{key}.fixture"));
        }
        command.env("GIT_CONFIG_GLOBAL", "config.fixture");
        command.env("GANDRWORKFLOWGATES_SUPPORT_KEEP", "keep.fixture");
        sanitize_git_environment(&mut command);

        for key in GIT_ENVIRONMENT_KEYS {
            assert_eq!(
                Some(CommandEnvEntry::Removed),
                explicit_command_env(&command, key),
                "{key}"
            );
        }
        assert_eq!(
            Some(CommandEnvEntry::Set(OsString::from("config.fixture"))),
            explicit_command_env(&command, "GIT_CONFIG_GLOBAL")
        );
        assert_eq!(
            Some(CommandEnvEntry::Set(OsString::from("keep.fixture"))),
            explicit_command_env(&command, "GANDRWORKFLOWGATES_SUPPORT_KEEP")
        );
    }

    /// Stateless Git overrides hostile identity and signing without local
    /// configuration.
    #[test]
    fn stateless_git_command_overrides_ambient_identity_and_signing() -> Result<(), Box<dyn Error>>
    {
        let fixture = TestWorkspace::create("stateless-git")?;
        let hostile_config = fixture.path().join("host.gitconfig");
        HOST_FILESYSTEM.write(
            &hostile_config,
            "[user]\n\tname = Host User\n\temail = host@example.invalid\n\tsigningKey = \
             /missing/signing-key.pub\n[commit]\n\tgpgSign = true\n[gpg]\n\tformat = ssh\n",
        )?;
        let repo = fixture.path().join("repo");
        HOST_FILESYSTEM.create_dir_all(&repo)?;

        let mut init = stateless_git_command();
        init.args(["init", "--quiet"])
            .current_dir(&repo)
            .env("GIT_CONFIG_GLOBAL", &hostile_config);
        let init = init.output()?;
        assert!(
            init.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&init.stderr)
        );

        HOST_FILESYSTEM.write(repo.join("README.md"), "fixture\n")?;
        let mut add = stateless_git_command();
        add.args(["add", "README.md"])
            .current_dir(&repo)
            .env("GIT_CONFIG_GLOBAL", &hostile_config);
        let add = add.output()?;
        assert!(
            add.status.success(),
            "git add failed: {}",
            String::from_utf8_lossy(&add.stderr)
        );

        let mut commit = stateless_git_command();
        commit
            .args(["commit", "--quiet", "-m", "fixture"])
            .current_dir(&repo)
            .env("GIT_CONFIG_GLOBAL", &hostile_config);
        let commit = commit.output()?;
        assert!(
            commit.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&commit.stderr)
        );

        let mut show = stateless_git_command();
        show.args(["show", "--no-patch", "--format=%an%x00%ae%x00%G?", "HEAD"])
            .current_dir(&repo)
            .env("GIT_CONFIG_GLOBAL", &hostile_config);
        let show = show.output()?;
        assert!(
            show.status.success(),
            "git show failed: {}",
            String::from_utf8_lossy(&show.stderr)
        );
        assert_eq!(
            b"gandr-agent\0gandr-agent@gandr.invalid\0N\n",
            show.stdout.as_slice()
        );

        let local_config = std::fs::read_to_string(repo.join(".git/config"))?;
        assert!(!local_config.contains("[user]"));
        assert!(!local_config.contains("gpgsign"));
        Ok(())
    }

    /// The environment-printing child fixture reports unset keys
    /// deterministically.
    #[test]
    fn child_git_environment_fixture_reports_unset_values() -> Result<(), Box<dyn Error>>
    {
        child_prints_git_environment()
    }

    /// `walk_files` is iterative, sorted, extension-filtered, and symlink-safe.
    #[test]
    fn walk_files_sorts_and_skips_symlinked_directories() -> Result<(), Box<dyn Error>>
    {
        let fixture = TestWorkspace::create("walk-files")?;
        let root = fixture.path().join("root");
        let outside = fixture.path().join("outside");
        HOST_FILESYSTEM.create_dir_all(root.join("b"))?;
        HOST_FILESYSTEM.create_dir_all(root.join("a"))?;
        HOST_FILESYSTEM.create_dir_all(&outside)?;
        HOST_FILESYSTEM.write(root.join("b/second.rs"), b"second")?;
        HOST_FILESYSTEM.write(root.join("a/first.rs"), b"first")?;
        HOST_FILESYSTEM.write(root.join("a/skip.txt"), b"skip")?;
        HOST_FILESYSTEM.write(outside.join("hidden.rs"), b"hidden")?;
        symlink_directory(&outside, &root.join("linked"))?;

        let files = walk_files(&root, OsStr::new("rs"))?;
        let expected = vec![root.join("a/first.rs"), root.join("b/second.rs")];

        assert_eq!(files, expected);
        Ok(())
    }

    /// `read_utf8` loads a complete text file.
    #[test]
    fn read_utf8_loads_full_text() -> Result<(), Box<dyn Error>>
    {
        let fixture = TestWorkspace::create("read-utf8")?;
        let path = fixture.path().join("input.txt");
        HOST_FILESYSTEM.write(&path, b"line one\nline two")?;

        assert_eq!("line one\nline two", read_utf8(&path)?);
        assert!(read_utf8(&fixture.path().join("missing.txt")).is_err());
        Ok(())
    }

    /// `write_atomic` replaces a file and leaves no staging file behind.
    #[test]
    fn write_atomic_replaces_file_and_removes_temporary() -> Result<(), Box<dyn Error>>
    {
        let fixture = TestWorkspace::create("write-atomic")?;
        let path = fixture.path().join("output.txt");

        write_atomic(&path, b"first")?;
        write_atomic(&path, b"second")?;

        assert_eq!("second", read_utf8(&path)?);
        assert_eq!(1, directory_entries(fixture.path())?.len());
        Ok(())
    }

    /// `write_atomic` removes its temporary file if publication fails.
    #[test]
    fn write_atomic_cleans_temporary_after_rename_failure() -> Result<(), Box<dyn Error>>
    {
        let fixture = TestWorkspace::create("write-atomic-failure")?;
        let directory_target = fixture.path().join("target-directory");
        HOST_FILESYSTEM.create_dir_all(&directory_target)?;

        let result = write_atomic(&directory_target, b"not a directory");

        assert!(result.is_err());
        assert_eq!(1, directory_entries(fixture.path())?.len());
        Ok(())
    }

    /// `write_atomic` rejects paths without file names.
    #[test]
    fn write_atomic_requires_file_name()
    {
        let result = write_atomic(Path::new(""), b"root");

        assert!(matches!(result, Err(GateError::Operational { .. })));
    }

    /// Process and filesystem failure paths preserve typed gate errors.
    #[test]
    fn process_and_filesystem_errors_are_typed() -> Result<(), Box<dyn Error>>
    {
        let missing_program = OsStr::new("gandr-workflow-gates-definitely-missing-command");
        let output_error = run_output(missing_program, &[], None, false)
            .err()
            .ok_or_else(|| GateError::operational("missing command unexpectedly ran"))?;
        assert!(
            matches!(
                output_error,
                GateError::Io {
                    path,
                    ..
                } if path.as_os_str() == missing_program
            ),
            "spawn failures should report the command path"
        );

        let status_error = run_status(missing_program, &[], None, false)
            .err()
            .ok_or_else(|| GateError::operational("missing status command unexpectedly ran"))?;
        assert!(
            matches!(
                status_error,
                GateError::Io {
                    path,
                    ..
                } if path.as_os_str() == missing_program
            ),
            "status spawn failures should report the command path"
        );

        let fixture = TestWorkspace::create("error-paths")?;
        let invalid_utf8 = fixture.path().join("invalid.txt");
        HOST_FILESYSTEM.write(&invalid_utf8, [0xff_u8])?;
        assert!(
            matches!(read_utf8(&invalid_utf8), Err(GateError::Io { path, .. }) if path == invalid_utf8),
            "invalid UTF-8 should surface through the shared I/O error"
        );

        let missing_parent_target = fixture.path().join("missing/output.txt");
        assert!(
            matches!(write_atomic(&missing_parent_target, b"body"), Err(GateError::Io { path, .. }) if path.file_name() == Some(OsStr::new(".output.txt.gandr-workflow-gates-")) || path.parent() == missing_parent_target.parent()),
            "atomic staging failures should report a sibling temporary path"
        );

        remove_temporary_file(&fixture.path().join("already-gone.tmp"))?;
        assert_eq!(
            "probe terminated without an exit code",
            command_status_detail("probe", None)
        );
        Ok(())
    }

    /// Atomic writes fail closed after every deterministic temporary candidate
    /// collides.
    #[test]
    fn write_atomic_reports_exhausted_temporary_candidates() -> Result<(), Box<dyn Error>>
    {
        let fixture = TestWorkspace::create("atomic-collisions")?;
        let target = fixture.path().join("blocked.txt");
        let target_name = target_file_name(&target)?;
        for attempt in 0_u16 .. TEMPORARY_FILE_ATTEMPTS {
            HOST_FILESYSTEM.create_file(temporary_file_path(
                fixture.path(),
                target_name,
                attempt,
            ))?;
        }

        let result = write_atomic(&target, b"blocked");
        assert!(
            matches!(result, Err(GateError::Operational { detail }) if detail.contains("could not create unique temporary file")),
            "exhausted temporary candidates should be an operational failure"
        );
        assert!(
            !target.exists(),
            "failed publication must not create the target file"
        );
        Ok(())
    }

    /// Command construction applies cwd and sanitized Git environment together.
    #[test]
    fn run_status_supports_cwd_with_git_sanitization() -> Result<(), Box<dyn Error>>
    {
        let fixture = TestWorkspace::create("cwd-status")?;
        let child = HOST_FILESYSTEM.current_exe()?;
        let args = child_test_args("child_success");
        let status = run_status(child.as_os_str(), &args, Some(fixture.path()), true)?;
        assert!(
            status.success(),
            "child status fixture should succeed under an explicit cwd"
        );
        Ok(())
    }

    /// A symlink used as the walk root is treated as a boundary, not followed.
    #[test]
    fn walk_files_root_symlink_is_bounded() -> Result<(), Box<dyn Error>>
    {
        let fixture = TestWorkspace::create("root-symlink")?;
        let real = fixture.path().join("real");
        let linked = fixture.path().join("linked");
        HOST_FILESYSTEM.create_dir_all(&real)?;
        HOST_FILESYSTEM.write(real.join("hidden.rs"), b"hidden")?;
        symlink_directory(&real, &linked)?;
        assert!(
            walk_files(&linked, OsStr::new("rs"))?.is_empty(),
            "root symlink scopes should not be traversed"
        );
        Ok(())
    }
    /// Ignored child fixture that writes stdout and stderr probes.
    #[test]
    #[ignore = "child-process fixture invoked explicitly by parent tests"]
    fn child_prints_streams() -> Result<(), Box<dyn Error>>
    {
        let mut stdout = std::io::stdout();
        stdout
            .write_all(b"support stdout probe\n")
            .map_err(|source| io_error(Path::new("<stdout>"), source))?;
        stdout
            .flush()
            .map_err(|source| io_error(Path::new("<stdout>"), source))?;
        let mut stderr = std::io::stderr();
        stderr
            .write_all(b"support stderr probe\n")
            .map_err(|source| io_error(Path::new("<stderr>"), source))?;
        stderr
            .flush()
            .map_err(|source| io_error(Path::new("<stderr>"), source))?;
        Ok(())
    }

    /// Ignored child fixture that writes more stdout than the injected cap.
    #[test]
    #[ignore = "child-process fixture invoked explicitly by parent tests"]
    fn child_writes_large_stdout() -> Result<(), Box<dyn Error>>
    {
        const LARGE_STDOUT_CHUNKS: usize = 0x4000;
        let mut stdout = std::io::stdout();
        for _ in 0_usize .. LARGE_STDOUT_CHUNKS {
            stdout
                .write_all(b"0123456789abcdef")
                .map_err(|source| io_error(Path::new("<stdout>"), source))?;
        }
        stdout
            .flush()
            .map_err(|source| io_error(Path::new("<stdout>"), source))?;
        Ok(())
    }

    /// Ignored child fixture that exits successfully.
    #[test]
    #[ignore = "child-process fixture invoked explicitly by parent tests"]
    fn child_success()
    {
    }

    /// Ignored child fixture that exits with a stable non-zero status.
    #[test]
    #[ignore = "child-process fixture invoked explicitly by parent tests"]
    #[expect(
        clippy::exit,
        reason = "child-process fixture must terminate with the exact status observed by the parent test"
    )]
    fn child_exits_with_status()
    {
        std::process::exit(7_i32);
    }

    /// Ignored child fixture that reports selected environment variables.
    #[test]
    #[ignore = "child-process fixture invoked explicitly by parent tests"]
    fn child_prints_git_environment() -> Result<(), Box<dyn Error>>
    {
        print_environment_value("GIT_INDEX_FILE")?;
        print_environment_value("GIT_DIR")?;
        print_environment_value("GIT_WORK_TREE")?;
        print_environment_value("GANDRWORKFLOWGATES_SUPPORT_KEEP")?;
        Ok(())
    }

    /// Build test-harness arguments for one ignored child fixture.
    fn child_test_args<'semantic>(test_name: impl Into<TestNameText<'semantic>>) -> Vec<OsString>
    {
        let test_name = test_name.into().0;
        let mut exact_name = String::from("support::tests::");
        exact_name.push_str(test_name);
        vec![
            OsString::from("--ignored"),
            OsString::from("--exact"),
            OsString::from(exact_name),
            OsString::from("--nocapture"),
        ]
    }

    /// Explicit command environment state for one key.
    #[derive(Debug, Eq, PartialEq)]
    enum CommandEnvEntry
    {
        /// The key is explicitly removed from the child environment.
        Removed,
        /// The key is explicitly set for the child environment.
        Set(OsString),
    }
    /// Return an explicit command environment override/removal for a key.
    fn explicit_command_env<'semantic>(
        command: &Command,
        key: impl Into<KeyText<'semantic>>,
    ) -> Option<CommandEnvEntry>
    {
        let key = key.into().0;
        for (name, value) in command.get_envs() {
            if name == OsStr::new(key) {
                let entry = match value {
                    | Some(value) => CommandEnvEntry::Set(value.to_os_string()),
                    | None => CommandEnvEntry::Removed,
                };
                return Some(entry);
            }
        }
        None
    }

    /// Write one environment variable in a stable test format.
    fn print_environment_value<'semantic>(
        key: impl Into<KeyText<'semantic>>
    ) -> Result<(), Box<dyn Error>>
    {
        let key = key.into().0;
        let mut stdout = std::io::stdout();
        match std::env::var_os(key) {
            | Some(value) => writeln!(stdout, "{key}={}", value.to_string_lossy())
                .map_err(|source| io_error(Path::new("<stdout>"), source))?,
            | None => writeln!(stdout, "{key}=<unset>")
                .map_err(|source| io_error(Path::new("<stdout>"), source))?,
        }
        Ok(())
    }

    /// Return sorted direct entries under a directory.
    fn directory_entries(path: &Path) -> Result<Vec<PathBuf>, GateError>
    {
        let mut entries = HOST_FILESYSTEM.read_dir_paths(path)?;
        entries.sort();
        Ok(entries)
    }

    /// Create a directory symlink on platforms that support it.
    #[cfg(unix)]
    fn symlink_directory(
        source: &Path,
        destination: &Path,
    ) -> Result<(), GateError>
    {
        HOST_FILESYSTEM.symlink(source, destination)
    }

    /// No-op symlink fixture on platforms without stable directory symlink
    /// support for unprivileged tests.
    #[cfg(not(unix))]
    fn symlink_directory(
        _source: &Path,
        _destination: &Path,
    ) -> Result<(), GateError>
    {
        Ok(())
    }

    /// Temporary directory removed when a test finishes.
    #[repr(transparent)]
    struct TestWorkspace
    {
        /// Directory owned by this fixture.
        path: PathBuf,
    }

    impl TestWorkspace
    {
        /// Create a clean temporary workspace for one test.
        fn create<'semantic>(name: impl Into<NameText<'semantic>>) -> Result<Self, GateError>
        {
            let name = name.into().0;
            let path = std::env::temp_dir().join(format!(
                "gandr-workflow-gates-support-{name}-{}",
                std::process::id()
            ));
            remove_dir_if_exists(&path)?;
            HOST_FILESYSTEM.create_dir_all(&path)?;
            Ok(Self { path })
        }

        /// Return the workspace root path.
        fn path(&self) -> &Path
        {
            &self.path
        }
    }

    impl Drop for TestWorkspace
    {
        fn drop(&mut self)
        {
            let _cleanup_result = HOST_FILESYSTEM.remove_dir_if_exists(&self.path);
        }
    }

    /// Remove a directory tree unless it is already absent.
    fn remove_dir_if_exists(path: &Path) -> Result<(), GateError>
    {
        HOST_FILESYSTEM.remove_dir_if_exists(path)
    }
}
