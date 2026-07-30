//! The native host handler: it carries out the operations no source-level
//! handler claims (`Exec` / `Fs` / `Env` / `Proc`) against real syscalls.
//!
//! [`ShellHandler::dispatch`] is the whole surface: it takes an intercepted
//! [`HostOp`] (the host seam's owned view of a `perform`, ADR-35 D4) and
//! returns a [`HostAction`] telling the driver ([`crate::driver`]) to resume
//! the run with a reply, halt with an exit code, abort with a fatal error, or
//! decline (leaving the machine to blame an unclaimed `perform`).
//!
//! Dispatch keys on [`gandr_core_checker::effect::EffectSig::name`] first, then
//! the operation name — so an operation named `read` in some *other* signature
//! is declined, not misrouted to `Fs::read`.

// `core::io::ErrorKind` exists but is unstable (feature `core_io`,
// rust-lang/rust#154046), so `std::io` is the stable home on this toolchain.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::hash::BuildHasher as _;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command as ProcCommand;
use std::process::Stdio;
use std::process::id as process_id;

use gandr_core_checker::boundary::OperationName;
use gandr_core_checker::host as sig;
use gandr_core_checker::host::HostOp;
use gandr_core_checker::syntax::Value;

use crate::boundary::FilePath;
use crate::boundary::GlobMatch;
use crate::boundary::GlobPattern;
use crate::boundary::GlobSegmentPattern;
use crate::boundary::PathSegmentText;
use crate::codec;
use crate::codec::SpawnMode;
use crate::error::ShellError;

/// The maximum number of distinct names [`ShellHandler::fs_tempdir`] tries
/// before giving up (a defensive bound; a fresh counter suffix makes a
/// collision practically impossible).
const MAX_TEMPDIR_ATTEMPTS: u64 = 1024;

/// The driver's instruction after an intercepted operation — the internal,
/// richer cousin of [`gandr_core_checker::host::HostReply`] (which cannot
/// express a run-truncating `exit` or a fatal abort; the [`crate::driver`]
/// captures those two out of band).
#[derive(Clone, Debug)]
pub(crate) enum HostAction
{
    /// Resume the run, delivering this reply value to the performing
    /// continuation (the ordinary case).
    Resume(Value),
    /// Halt the run with this exit code (`Proc::exit`).
    Exit(i64),
    /// Abort the run: a syscall failed fatally.
    Fail(ShellError),
    /// This operation is not one the shell host handles; let the machine take
    /// its ordinary (blaming) step.
    Decline,
}

impl From<Result<Value, ShellError>> for HostAction
{
    #[inline]
    fn from(result: Result<Value, ShellError>) -> Self
    {
        match result {
            | Ok(reply) => Self::Resume(reply),
            | Err(error) => Self::Fail(error),
        }
    }
}

/// The native host handler over `std::process` / `std::fs` / `std::env`.
///
/// Stateless today apart from a per-run tempdir suffix counter; owned by the
/// driver, single-threaded, and re-created per [`crate::run_program`] call.
#[repr(transparent)]
#[derive(Clone, Debug, Default)]
pub struct ShellHandler
{
    /// A monotonic suffix source keeping repeated `Fs::tempdir` directory
    /// names within one run distinct.
    tempdir_counter: u64,
}

impl ShellHandler
{
    /// Builds a fresh handler.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Dispatches one intercepted operation to its native implementation.
    #[inline]
    pub(crate) fn dispatch(
        &mut self,
        op: &HostOp,
    ) -> HostAction
    {
        match op.sig.name().as_ref() {
            | sig::EXEC => Self::dispatch_exec(op),
            | sig::FS => self.dispatch_fs(op),
            | sig::ENV => Self::dispatch_env(op),
            | sig::PROC => Self::dispatch_proc(op),
            | _ => HostAction::Decline,
        }
    }

    /// Routes an `Exec` operation.
    #[inline]
    fn dispatch_exec(op: &HostOp) -> HostAction
    {
        match op.op.as_str() {
            | sig::EXEC_RUN => Self::exec(&op.payload).into(),
            | _ => HostAction::Decline,
        }
    }

    /// Routes an `Fs` operation (`tempdir` needs the handler's counter).
    #[inline]
    fn dispatch_fs(
        &mut self,
        op: &HostOp,
    ) -> HostAction
    {
        match op.op.as_str() {
            | sig::FS_READ => Self::fs_read(&op.payload).into(),
            | sig::FS_WRITE => Self::fs_write(&op.payload).into(),
            | sig::FS_GLOB => Self::fs_glob(&op.payload).into(),
            | sig::FS_STAT => Self::fs_stat(&op.payload).into(),
            | sig::FS_MKDIR => Self::fs_mkdir(&op.payload).into(),
            | sig::FS_TEMPDIR => self.fs_tempdir().into(),
            | sig::FS_CWD => Self::fs_cwd().into(),
            | sig::FS_LS_FILES => Self::fs_ls_files(&op.payload).into(),
            | _ => HostAction::Decline,
        }
    }

    /// Routes an `Env` operation (read-only).
    #[inline]
    fn dispatch_env(op: &HostOp) -> HostAction
    {
        match op.op.as_str() {
            | sig::ENV_GET => Self::env_get(&op.payload).into(),
            | sig::ENV_PATH => HostAction::Resume(Self::env_path()),
            | _ => HostAction::Decline,
        }
    }

    /// Routes a `Proc` operation.
    #[inline]
    fn dispatch_proc(op: &HostOp) -> HostAction
    {
        match op.op.as_str() {
            | sig::PROC_EXIT => match codec::decode_int(sig::PROC, sig::PROC_EXIT, &op.payload) {
                | Ok(code) => HostAction::Exit(i64::from(code)),
                | Err(error) => HostAction::Fail(error),
            },
            | _ => HostAction::Decline,
        }
    }

    /// `Exec::exec`: run a program with an argument vector. The spawn mode
    /// (ADR-74 D4) selects the stdio disposition: [`SpawnMode::Captured`]
    /// buffers the child's output into the typed reply; [`SpawnMode::Inherit`]
    /// lets the child inherit the parent's terminal so an interactive program
    /// (`vim`, `less`, `ssh`) behaves. argv is passed element-wise — no shell
    /// interpolation, so no injection surface. A non-zero exit is a normal
    /// reply (`exit_code`), not an error; only a spawn failure is fatal.
    fn exec(payload: &Value) -> Result<Value, ShellError>
    {
        let command = codec::decode_command(sig::EXEC, sig::EXEC_RUN, payload)?;
        match command.mode {
            | SpawnMode::Captured => Self::exec_captured(&command),
            | SpawnMode::Inherit => Self::exec_inherit(&command),
        }
    }

    /// The captured spawn (the contract for consumed results and scripts).
    ///
    /// v0 trust assumption: the child's stdout/stderr are buffered whole into
    /// memory with no size cap and no wall-clock timeout
    /// ([`Command::output`](std::process::Command::output)), so a program
    /// that never closes its output (e.g. `yes`) hangs / OOMs the run. This
    /// is sound for the trusted, short-lived gate scripts v0 targets;
    /// a bounded, timed, killable exec is a follow-up before untrusted programs
    /// run. stdin is closed (`Stdio::null`) so a child reading it sees EOF
    /// rather than blocking on the parent's stdin.
    fn exec_captured(command: &codec::Command) -> Result<Value, ShellError>
    {
        let output = ProcCommand::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| ShellError::Spawn {
                program: command.program.clone(),
                detail: error.to_string(),
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        // A process killed by a signal has no code; report -1 (a full
        // signal-aware encoding is a v0 follow-up).
        let exit_code = output.status.code().map_or(-1_i64, i64::from);
        Ok(codec::encode_exec_result(&stdout, &stderr, exit_code))
    }

    /// The inherit spawn (ADR-74 D4): the child inherits the parent's
    /// stdin/stdout/stderr, so an interactive program drives the terminal
    /// directly. Nothing is captured, so the reply's `stdout`/`stderr` are
    /// empty and only `exit_code` is meaningful.
    fn exec_inherit(command: &codec::Command) -> Result<Value, ShellError>
    {
        let status = ProcCommand::new(&command.program)
            .args(&command.args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .map_err(|error| ShellError::Spawn {
                program: command.program.clone(),
                detail: error.to_string(),
            })?;
        let exit_code = status.code().map_or(-1_i64, i64::from);
        Ok(codec::encode_exec_result("", "", exit_code))
    }

    /// `Fs::read`: the whole file as a string. A missing file or a non-UTF-8
    /// file is fatal (the reply type is `String`).
    fn fs_read(payload: &Value) -> Result<Value, ShellError>
    {
        let path = codec::decode_str(sig::FS, sig::FS_READ, payload)?;
        let contents =
            fs::read_to_string(&path).map_err(|error| fs_error(sig::FS_READ, &path, &error))?;
        Ok(Value::string(&contents))
    }

    /// `Fs::write`: write a string to a path (truncating), replying `unit`.
    /// A missing parent directory is fatal (`mkdir` first).
    fn fs_write(payload: &Value) -> Result<Value, ShellError>
    {
        let (path, contents) = codec::decode_write(sig::FS, sig::FS_WRITE, payload)?;
        fs::write(&path, &contents).map_err(|error| fs_error(sig::FS_WRITE, &path, &error))?;
        Ok(Value::Unit)
    }

    /// `Fs::glob`: expand a pattern to the sorted, de-duplicated list of
    /// matching paths (the self-contained v0 subset — see [`glob_paths`]).
    /// Unreadable directories are skipped, never fatal.
    fn fs_glob(payload: &Value) -> Result<Value, ShellError>
    {
        let pattern = codec::decode_str(sig::FS, sig::FS_GLOB, payload)?;
        let mut matches = glob_paths(&pattern);
        matches.sort();
        // Consecutive `**` segments can reach one path by more than one split,
        // emitting it twice; sorted, adjacent equal paths collapse here.
        matches.dedup();
        Ok(codec::encode_str_list(&matches))
    }

    /// `Fs::stat`: classify a path as `{kind, size}` where `kind` is one of
    /// `file` / `dir` / `symlink` / `other` / `missing`. A path that does not
    /// exist is `missing` (a normal reply), not an error; the symlink itself is
    /// classified (the link is not followed).
    #[expect(
        clippy::filetype_is_file,
        reason = "the chain deliberately separates a regular file from a symlink or other node; !is_dir would fold symlinks into `file`"
    )]
    #[expect(
        clippy::allow_attributes,
        reason = "stable Clippy flags std::io as replaceable while the nightly gate recognizes core::io as unstable, so one cross-toolchain allow is required"
    )]
    #[allow(
        clippy::std_instead_of_core,
        reason = "core::io::ErrorKind remains unstable on the workspace toolchain"
    )]
    fn fs_stat(payload: &Value) -> Result<Value, ShellError>
    {
        let path = codec::decode_str(sig::FS, sig::FS_STAT, payload)?;
        match fs::symlink_metadata(&path) {
            | Ok(metadata) => {
                let file_type = metadata.file_type();
                let kind = if file_type.is_dir() {
                    "dir"
                }
                else if file_type.is_file() {
                    "file"
                }
                else if file_type.is_symlink() {
                    "symlink"
                }
                else {
                    "other"
                };
                let size = i64::try_from(metadata.len()).unwrap_or(i64::MAX);
                Ok(codec::encode_stat(kind, size))
            },
            // A path that cannot exist is `missing`, not fatal: `NotFound`
            // (no such entry) and `NotADirectory` (an intermediate component is
            // not a directory) both mean "there is no file here". A genuine
            // access failure (e.g. `PermissionDenied`) stays fatal.
            | Err(ref error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                Ok(codec::encode_stat("missing", 0_i64))
            },
            | Err(error) => Err(fs_error(sig::FS_STAT, &path, &error)),
        }
    }

    /// `Fs::mkdir`: create a directory and every missing parent, replying
    /// `unit` (idempotent, like `mkdir -p`).
    fn fs_mkdir(payload: &Value) -> Result<Value, ShellError>
    {
        let path = codec::decode_str(sig::FS, sig::FS_MKDIR, payload)?;
        fs::create_dir_all(&path).map_err(|error| fs_error(sig::FS_MKDIR, &path, &error))?;
        Ok(Value::Unit)
    }

    /// `Fs::tempdir`: create a fresh, uniquely-owned temporary directory and
    /// reply with its absolute path. The directory persists (the caller owns
    /// its lifecycle); it is not cleaned up on drop.
    ///
    /// The name carries an OS-seeded random component so it is unpredictable —
    /// a local user cannot pre-create the target in a shared temp dir to force
    /// the run to abort — and so the per-run counter reset does not
    /// systematically collide with a prior run's persisted directories. The
    /// returned path is rendered lossily, so it assumes the OS temp dir is
    /// valid UTF-8 (the `Value::Str` model).
    #[expect(
        clippy::allow_attributes,
        reason = "stable Clippy flags std::io as replaceable while the nightly gate recognizes core::io as unstable, so one cross-toolchain allow is required"
    )]
    #[allow(
        clippy::std_instead_of_core,
        reason = "core::io::ErrorKind remains unstable on the workspace toolchain"
    )]
    fn fs_tempdir(&mut self) -> Result<Value, ShellError>
    {
        let base = env::temp_dir();
        let pid = process_id();
        // OS-seeded per-call randomness (dep-free): a fresh `RandomState` is
        // securely seeded, so hashing a constant yields an unpredictable word.
        let nonce = std::collections::hash_map::RandomState::new().hash_one(pid);
        for _ in 0 .. MAX_TEMPDIR_ATTEMPTS {
            let suffix = self.tempdir_counter;
            self.tempdir_counter = self.tempdir_counter.saturating_add(1);
            let candidate = base.join(format!("gandr-runtime-host-{pid}-{nonce:016x}-{suffix}"));
            // `create_dir` (not `create_dir_all`) is deliberate: its
            // fail-on-exists is what guarantees a *fresh* directory —
            // `create_dir_all` would silently accept a pre-existing path.
            #[expect(
                clippy::create_dir,
                reason = "tempdir needs create_dir's fail-on-exists to guarantee a fresh directory"
            )]
            let created = fs::create_dir(&candidate);
            match created {
                | Ok(()) => {
                    let rendered = candidate.to_string_lossy();
                    return Ok(Value::string(rendered.as_ref()));
                },
                | Err(ref error) if error.kind() == std::io::ErrorKind::AlreadyExists => {},
                | Err(error) => {
                    let rendered = candidate.to_string_lossy();
                    return Err(fs_error(sig::FS_TEMPDIR, rendered.as_ref(), &error));
                },
            }
        }
        Err(ShellError::Fs {
            op: sig::FS_TEMPDIR.to_owned(),
            path: base.to_string_lossy().into_owned(),
            detail: "exhausted unique-name attempts".to_owned(),
        })
    }

    /// `Fs::ls_files`: the git-tracked files under a directory (`git ls-files`
    /// with that directory as the working directory), one path per element.
    /// A `git` that will not spawn, or a non-zero `git` exit, is fatal.
    ///
    /// Uses `-z` with `core.quotePath=false` so paths are emitted verbatim,
    /// NUL-separated: git's default would octal-escape and quote non-ASCII or
    /// special-character paths (`café.txt` → `"calf\303\251.txt"`), yielding
    /// strings that do not name the real file, and a filename containing a
    /// newline would over-split a line-based parse.
    fn fs_ls_files(payload: &Value) -> Result<Value, ShellError>
    {
        let dir = codec::decode_str(sig::FS, sig::FS_LS_FILES, payload)?;
        let output = ProcCommand::new("git")
            .arg("-c")
            .arg("core.quotePath=false")
            .arg("ls-files")
            .arg("-z")
            .arg("--")
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .map_err(|error| fs_error(sig::FS_LS_FILES, &dir, &error))?;
        if !output.status.success() {
            return Err(ShellError::Fs {
                op: sig::FS_LS_FILES.to_owned(),
                path: dir,
                detail: format!(
                    "git ls-files failed ({}): {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }
        let text = String::from_utf8_lossy(&output.stdout);
        // `-z` is NUL-terminated (not -separated), so the final split element is
        // an empty tail to drop.
        let files: Vec<String> = text
            .split('\0')
            .filter(|entry| !entry.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        Ok(codec::encode_str_list(&files))
    }

    /// `Fs::cwd`: the process's current working directory (the base a relative
    /// path or a relative `Fs::glob` resolves against). Fatal only if the cwd
    /// is unavailable (e.g. it was deleted). The path is rendered lossily
    /// (assumes a valid-UTF-8 cwd; the `Value::Str` model).
    fn fs_cwd() -> Result<Value, ShellError>
    {
        let dir = env::current_dir().map_err(|error| fs_error(sig::FS_CWD, ".", &error))?;
        let rendered = dir.to_string_lossy();
        Ok(Value::string(rendered.as_ref()))
    }

    /// `Env::get`: one environment variable's value, or the empty string if it
    /// is unset (or not valid Unicode).
    fn env_get(payload: &Value) -> Result<Value, ShellError>
    {
        let name = codec::decode_str(sig::ENV, sig::ENV_GET, payload)?;
        let value = env::var(&name).unwrap_or_default();
        Ok(Value::string(&value))
    }

    /// `Env::path`: the `PATH` entries as a list (empty if `PATH` is unset).
    fn env_path() -> Value
    {
        let entries: Vec<String> = env::var_os("PATH")
            .map(|paths| {
                env::split_paths(&paths)
                    .map(|path| path.to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        codec::encode_str_list(&entries)
    }
}

/// Builds a [`ShellError::Fs`] for a failed filesystem operation.
#[inline]
fn fs_error<'op, 'path>(
    op: impl Into<OperationName<'op>>,
    path: impl Into<FilePath<'path>>,
    error: &std::io::Error,
) -> ShellError
{
    let op = op.into();
    let path = path.into();
    ShellError::Fs {
        op: op.as_ref().to_owned(),
        path: path.as_ref().to_owned(),
        detail: error.to_string(),
    }
}

/// Expands a glob pattern to the matching paths (unsorted, may contain
/// duplicates — [`ShellHandler::fs_glob`] sorts and dedups).
///
/// The self-contained v0 subset: `?` (one non-`/` character), `*` (zero or
/// more non-`/` characters within one path segment), and `**` (zero or more
/// directory segments). Segments are split on `/`; a leading `/` roots the
/// walk at the filesystem root, otherwise it is relative to the current
/// directory (and a leading `./` is trimmed from results).
///
/// Documented v0 divergences from a POSIX shell (each a reversal-triggered
/// upgrade candidate, tracked with the full-glob-engine follow-up):
/// - a leading-dot filename IS matched by `*` (no hidden-file exclusion);
/// - `.` segments are ignored and `..` is an ordinary literal that matches
///   nothing (`std::fs::read_dir` never yields `.`/`..`), so there is no upward
///   traversal — a deliberate containment property;
/// - a trailing `/` is dropped, so `src/` behaves like `src` and can match a
///   *file* named `src`; an empty (or all-ignored) pattern matches nothing;
/// - `**` is an unbounded whole-subtree walk with no depth or ignore cap (it
///   descends `.git/`, vendored trees, …); symlinked directories are NOT
///   descended by `**` (loop safety), while literal / `*` segments DO follow
///   them (bounded by the segment count — needed for e.g. macOS `/var`);
/// - the iterative `*` matcher can revisit a suffix after each consumed
///   character (bounded by the product of finite pattern and filename lengths);
/// - non-UTF-8 filenames are matched and rendered lossily, so a returned path
///   may not round-trip to the real file (the `Value::Str` model).
fn glob_paths<'pattern>(pattern: impl Into<GlobPattern<'pattern>>) -> Vec<String>
{
    let pattern = pattern.into();
    let is_absolute = pattern.as_ref().starts_with('/');
    let segments: Vec<String> = pattern
        .as_ref()
        .split('/')
        .filter(|&segment| !segment.is_empty() && segment != ".")
        .map(ToOwned::to_owned)
        .collect();
    // An empty (or all-ignored, e.g. "." / "./") pattern matches nothing rather
    // than resolving to the base directory.
    if segments.is_empty() {
        return Vec::new();
    }
    let base = if is_absolute {
        PathBuf::from("/")
    }
    else {
        PathBuf::from(".")
    };
    let mut found: Vec<PathBuf> = Vec::new();
    glob_walk(&base, &segments, &mut found);
    found
        .iter()
        .map(|path| {
            let rendered = path.to_string_lossy();
            rendered
                .strip_prefix("./")
                .unwrap_or_else(|| rendered.as_ref())
                .to_owned()
        })
        .collect()
}

/// Walks `base` with an explicit heap worklist, pushing every full match into
/// `out`.
///
/// # Contract
/// - requires: `segments` contains normalized non-empty glob segments.
/// - ensures: appends every path reachable under `base` that matches the
///   remaining pattern; `**` does not follow symlinked directories.
/// - provides: the unsorted, potentially duplicated path set consumed by
///   [`glob_paths`].
/// - panics: none.
/// - intension: traversal uses a heap worklist rather than native recursion;
///   output order is unspecified.
///
/// # Adequacy
/// - hypothesis: L2 — separate top-level and nested files distinguish the
///   zero-directory and one-or-more-directory `**` transitions.
/// - witness: `tests::glob_paths_matches_top_level_and_recursive`
fn glob_walk(
    base: &Path,
    segments: &[String],
    out: &mut Vec<PathBuf>,
)
{
    let mut pending = vec![(base.to_path_buf(), 0_usize)];
    while let Some((current, segment_index)) = pending.pop() {
        let Some(segment) = segments.get(segment_index)
        else {
            out.push(current);
            continue;
        };
        let Some(next_index) = segment_index.checked_add(1)
        else {
            continue;
        };
        if segment == "**" {
            // Zero directories: advance the pattern at this directory.
            pending.push((current.clone(), next_index));
            // One-or-more directories: descend without following symlinks,
            // retaining the `**` segment.
            if let Ok(entries) = fs::read_dir(&current) {
                for entry in entries.flatten() {
                    if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                        pending.push((entry.path(), segment_index));
                    }
                }
            }
            continue;
        }
        if let Ok(entries) = fs::read_dir(&current) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if !bool::from(segment_matches(segment, name.to_string_lossy().as_ref())) {
                    continue;
                }
                let path = entry.path();
                if next_index == segments.len() {
                    out.push(path);
                }
                // A literal / `*` segment descent is bounded by the remaining
                // segment count, so it follows symlinks (`Path::is_dir`) —
                // needed for components such as macOS's `/var` → `/private/var`.
                else if path.is_dir() {
                    pending.push((path, next_index));
                }
            }
        }
    }
}

/// Whether one path segment `text` matches a single glob segment `pattern`
/// (`*` / `?` / literals; no `/`).
#[inline]
fn segment_matches<'pattern, 'text>(
    pattern: impl Into<GlobSegmentPattern<'pattern>>,
    text: impl Into<PathSegmentText<'text>>,
) -> GlobMatch
{
    let pattern = pattern.into();
    let text = text.into();
    let pattern_chars: Vec<char> = pattern.as_ref().chars().collect();
    let text_chars: Vec<char> = text.as_ref().chars().collect();
    glob_segment(GlobCursor {
        pattern: &pattern_chars,
        text: &text_chars,
    })
}

/// Borrowed pattern and path-segment characters for one iterative match.
#[derive(Clone, Copy, Debug)]
struct GlobCursor<'input>
{
    /// The remaining pattern characters.
    pattern: &'input [char],
    /// The remaining text characters.
    text: &'input [char],
}

/// Matches one segment with a bounded iterative wildcard scan.
///
/// # Contract
/// - ensures: returns true exactly when `text` matches `pattern` under the
///   literal / `?` / `*` segment grammar.
/// - provides: the decision consumed by [`segment_matches`].
/// - panics: none.
/// - intension: uses explicit indices and the most recent `*` checkpoint, never
///   native recursion.
///
/// # Adequacy
/// - hypothesis: L2 — literal, one-character, empty-star, rejecting, and
///   backtracking witnesses distinguish every matcher transition.
/// - witness: `tests::segment_matcher_handles_star_question_and_literals`
fn glob_segment(cursor: GlobCursor<'_>) -> GlobMatch
{
    let mut pattern_index = 0_usize;
    let mut text_index = 0_usize;
    let mut star_index: Option<usize> = None;
    let mut star_text_index = 0_usize;

    while let Some(actual) = cursor.text.get(text_index).copied() {
        match cursor.pattern.get(pattern_index).copied() {
            | Some('?') => {
                let Some(next_pattern) = pattern_index.checked_add(1)
                else {
                    return false.into();
                };
                let Some(next_text) = text_index.checked_add(1)
                else {
                    return false.into();
                };
                pattern_index = next_pattern;
                text_index = next_text;
            },
            | Some(expected) if expected == actual => {
                let Some(next_pattern) = pattern_index.checked_add(1)
                else {
                    return false.into();
                };
                let Some(next_text) = text_index.checked_add(1)
                else {
                    return false.into();
                };
                pattern_index = next_pattern;
                text_index = next_text;
            },
            | Some('*') => {
                star_index = Some(pattern_index);
                star_text_index = text_index;
                let Some(next_pattern) = pattern_index.checked_add(1)
                else {
                    return false.into();
                };
                pattern_index = next_pattern;
            },
            | _ => {
                let Some(star) = star_index
                else {
                    return false.into();
                };
                let Some(next_text) = star_text_index.checked_add(1)
                else {
                    return false.into();
                };
                let Some(next_pattern) = star.checked_add(1)
                else {
                    return false.into();
                };
                star_text_index = next_text;
                text_index = next_text;
                pattern_index = next_pattern;
            },
        }
    }
    while cursor
        .pattern
        .get(pattern_index)
        .is_some_and(|&character| character == '*')
    {
        let Some(next_pattern) = pattern_index.checked_add(1)
        else {
            return false.into();
        };
        pattern_index = next_pattern;
    }
    (pattern_index == cursor.pattern.len()).into()
}

#[cfg(test)]
#[cfg_attr(
    dylint_lib = "non_thread_safe_call_in_test",
    allow(
        unknown_lints,
        non_thread_safe_call_in_test,
        reason = "the end-to-end handler tests write and create directories only beneath per-test unique tempdir roots, so the fs effects cannot race parallel tests"
    )
)]
mod tests
{
    use gandr_core_checker::host as sig;
    use gandr_core_checker::syntax::Value;

    use super::ShellHandler;
    use super::glob_paths;
    use super::segment_matches;
    use crate::boundary::FileContents;
    use crate::boundary::FilePath;

    #[test]
    fn segment_matcher_handles_star_question_and_literals()
    {
        assert!(bool::from(segment_matches("*.md", "readme.md")));
        assert!(!bool::from(segment_matches("*.md", "readme.txt")));
        assert!(bool::from(segment_matches("a?c", "abc")));
        assert!(
            !bool::from(segment_matches("a?c", "ac")),
            "`?` consumes exactly one char"
        );
        assert!(!bool::from(segment_matches("a?c", "abbc")));
        assert!(bool::from(segment_matches("*", "anything")));
        assert!(
            bool::from(segment_matches("*", "")),
            "`*` matches the empty segment"
        );
        assert!(bool::from(segment_matches("", "")));
        assert!(!bool::from(segment_matches("", "x")));
        assert!(
            bool::from(segment_matches("a*b*c", "axxbyyc")),
            "backtracking `*`"
        );
        assert!(!bool::from(segment_matches("a*b*c", "axxbyy")));
        assert!(
            bool::from(segment_matches("*.md", ".hidden.md")),
            "v0 `*` matches a leading-dot name (documented divergence)"
        );
    }

    #[test]
    fn glob_paths_matches_top_level_and_recursive()
    {
        let base = scratch_dir();
        let subdir = format!("{base}/sub");
        mkdir(&subdir);
        write_file(&format!("{base}/a.txt"), "");
        write_file(&format!("{base}/b.md"), "");
        write_file(&format!("{base}/sub/c.txt"), "");

        let shown = base.as_str();
        let top_pattern = format!("{shown}/*.txt");
        let mut top = glob_paths(&top_pattern);
        top.sort();
        assert_eq!(
            top,
            [format!("{shown}/a.txt")],
            "a top-level `*.txt` glob finds exactly the top-level match (absolute pattern \
             descends through the macOS /var symlink)"
        );

        let deep_pattern = format!("{shown}/**/*.txt");
        let mut deep = glob_paths(&deep_pattern);
        deep.sort();
        assert_eq!(
            deep,
            [format!("{shown}/a.txt"), format!("{shown}/sub/c.txt"),],
            "`**` matches zero directories (a.txt) and one directory (sub/c.txt)"
        );
    }

    #[test]
    fn glob_paths_question_dotfile_and_no_match()
    {
        let base = scratch_dir();
        write_file(&format!("{base}/a.txt"), ""); // single-char stem
        write_file(&format!("{base}/ab.txt"), ""); // two-char stem
        write_file(&format!("{base}/.hidden.txt"), ""); // leading dot
        let shown = base.as_str();

        // `?` consumes exactly one char: `?.txt` matches `a.txt`, not `ab.txt`.
        let mut q = glob_paths(&format!("{shown}/?.txt"));
        q.sort();
        assert_eq!(q, [format!("{shown}/a.txt")], "`?` matches one char only");

        // v0 `*` matches a leading-dot name (documented divergence from POSIX).
        let mut star = glob_paths(&format!("{shown}/*.txt"));
        star.sort();
        assert_eq!(
            star,
            [
                format!("{shown}/.hidden.txt"),
                format!("{shown}/a.txt"),
                format!("{shown}/ab.txt"),
            ],
            "`*` matches dotfiles in v0"
        );

        // A non-matching pattern yields nothing.
        assert!(
            glob_paths(&format!("{shown}/*.rs")).is_empty(),
            "no `.rs` files, so the glob is empty"
        );
        // An empty pattern matches nothing (not the base directory).
        assert!(
            glob_paths("").is_empty(),
            "the empty pattern matches nothing"
        );
    }

    fn scratch_dir() -> String
    {
        let mut handler = ShellHandler::new();
        handler
            .fs_tempdir()
            .expect("tempdir fixture is created through Fs::tempdir")
            .as_str()
            .expect("tempdir returns a path string")
            .as_ref()
            .to_owned()
    }

    fn mkdir<'path>(path: impl Into<FilePath<'path>>)
    {
        let path = path.into();
        let reply = ShellHandler::fs_mkdir(&Value::string(path.as_ref()))
            .expect("fixture directory is created");
        assert_eq!(Value::Unit, reply, "Fs::mkdir replies with unit");
    }

    fn write_file<'path, 'contents>(
        path: impl Into<FilePath<'path>>,
        contents: impl Into<FileContents<'contents>>,
    )
    {
        let path = path.into();
        let contents = contents.into();
        let payload = Value::record([
            (sig::FIELD_PATH.to_owned(), Value::string(path.as_ref())),
            (
                sig::FIELD_CONTENTS.to_owned(),
                Value::string(contents.as_ref()),
            ),
        ]);
        let reply = ShellHandler::fs_write(&payload).expect("fixture file is written");
        assert_eq!(Value::Unit, reply, "Fs::write replies with unit");
    }
}
