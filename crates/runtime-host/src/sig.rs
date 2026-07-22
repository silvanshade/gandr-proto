//! The canonical host effect signatures the runtime handles (proposal §3/§5:
//! the thin v0 host handler intercepts the flat `Exec`/`Fs`/`Proc`/`Env`
//! signature, performs the syscall, and resumes the delimited continuation
//! with the reply).
//!
//! These builders and name constants are the single source of truth two faces
//! share: a program author builds a `Comp::perform` against `sig::exec()` and
//! the [`crate::ShellHandler`] dispatches on the same [`EffectSig::name`] /
//! operation names, so the faces never drift.
//!
//! The v0 op set is named **`Exec` / `Fs` / `Proc` / `Env`** — the reserved
//! name `Shell` denotes the A8 typed-`Pipe` op (`effects-control-shell.md` §3)
//! and is deliberately NOT appropriated.
//!
//! v0 is sound because `Σ` is vacuous and resumption is multi-shot: the host is
//! an always-resume ambient handler on the seam ([`gandr_core_checker::host`],
//! realized by the L machine
//! [`gandr_core_sequent::machine::run_comp_with_host`]).
//!
//! The surface engine re-exports these definitions and adds its source-facing
//! `fs.read(…)`-style module metadata and `#!{ … }` shell-block lowering. The
//! runtime remains the authority for the signatures shared by lowering and
//! dispatch.
//!
//! [`EffectSig::name`]: gandr_core_checker::effect::EffectSig::name

use alloc::vec;

use gandr_core_checker::effect::EffectOp;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::types::ValueType;

/// The `Exec` signature name.
pub const EXEC: &str = "Exec";
/// The `Fs` signature name.
pub const FS: &str = "Fs";
/// The `Proc` signature name.
pub const PROC: &str = "Proc";
/// The `Env` signature name.
pub const ENV: &str = "Env";

/// `Exec::exec` — run an external program, capturing its output and exit code.
pub const EXEC_RUN: &str = "exec";

/// `Fs::read` — read a file's whole contents as a string.
pub const FS_READ: &str = "read";
/// `Fs::write` — write a string to a file (truncating).
pub const FS_WRITE: &str = "write";
/// `Fs::glob` — expand a glob pattern to the sorted list of matching paths.
pub const FS_GLOB: &str = "glob";
/// `Fs::stat` — classify a path (kind + byte size).
pub const FS_STAT: &str = "stat";
/// `Fs::mkdir` — create a directory and every missing parent.
pub const FS_MKDIR: &str = "mkdir";
/// `Fs::tempdir` — create a fresh temporary directory, returning its path.
pub const FS_TEMPDIR: &str = "tempdir";
/// `Fs::cwd` — the process's current working directory.
pub const FS_CWD: &str = "cwd";
/// `Fs::ls_files` — the tracked files under a directory (`git ls-files`).
pub const FS_LS_FILES: &str = "ls_files";

/// `Env::get` — read one environment variable (empty string if unset).
pub const ENV_GET: &str = "get";
/// `Env::path` — the `PATH` entries, split into a list.
pub const ENV_PATH: &str = "path";

/// `Proc::exit` — halt the run with an exit code (never resumes).
pub const PROC_EXIT: &str = "exit";

/// The record label carrying the program name in an `Exec::exec` payload.
pub const FIELD_PROGRAM: &str = "program";
/// The record label carrying the argument list in an `Exec::exec` payload.
pub const FIELD_ARGS: &str = "args";
/// The record label carrying the spawn mode in an `Exec::exec` payload — the
/// spawn-mode design: the captured reply is the typed contract for a consumed
/// result (splices, bound values, scripts), while a discarded reply runs with
/// inherited stdio so an interactive program behaves: [`MODE_CAPTURED`] or
/// [`MODE_INHERIT`].
pub const FIELD_MODE: &str = "mode";
/// The `Exec::exec` spawn mode that captures the child's output.
///
/// Captures stdout/stderr into the typed `{stdout, stderr, exit_code}` reply —
/// the contract for every consumed result. A payload with no `mode` field
/// decodes as captured (the drift-safe default).
pub const MODE_CAPTURED: &str = "captured";
/// The `Exec::exec` spawn mode that inherits the parent's terminal.
///
/// The child inherits the parent's stdin/stdout/stderr, so an interactive
/// program (`vim`, `less`, `ssh`) behaves. The reply's `stdout`/`stderr` are
/// then empty and only `exit_code` is meaningful.
pub const MODE_INHERIT: &str = "inherit";
/// The record label carrying the standard-output string in an `exec` reply.
pub const FIELD_STDOUT: &str = "stdout";
/// The record label carrying the standard-error string in an `exec` reply.
pub const FIELD_STDERR: &str = "stderr";
/// The record label carrying the exit code in an `exec` reply.
pub const FIELD_EXIT_CODE: &str = "exit_code";
/// The record label carrying the target path in an `Fs::write` payload.
pub const FIELD_PATH: &str = "path";
/// The record label carrying the file contents in an `Fs::write` payload.
pub const FIELD_CONTENTS: &str = "contents";
/// The record label carrying the path kind in an `Fs::stat` reply.
pub const FIELD_KIND: &str = "kind";
/// The record label carrying the byte size in an `Fs::stat` reply.
pub const FIELD_SIZE: &str = "size";

/// The `Exec::exec` payload type
/// `{program : String, args : List String, mode : String}`.
///
/// The `mode` field carries the spawn mode ([`MODE_CAPTURED`] /
/// [`MODE_INHERIT`]); a payload with no `mode` field decodes as
/// [`MODE_CAPTURED`] (the drift-safe default), so a hand-built `{program,
/// args}` perform still captures.
#[inline]
#[must_use]
fn command_ty() -> ValueType
{
    ValueType::record([
        (FIELD_PROGRAM.to_owned(), ValueType::string()),
        (FIELD_ARGS.to_owned(), ValueType::list(ValueType::string())),
        (FIELD_MODE.to_owned(), ValueType::string()),
    ])
}

/// The `Exec::exec` reply type
/// `{stdout : String, stderr : String, exit_code : Integer}`.
#[inline]
#[must_use]
fn exec_result_ty() -> ValueType
{
    ValueType::record([
        (FIELD_STDOUT.to_owned(), ValueType::string()),
        (FIELD_STDERR.to_owned(), ValueType::string()),
        (FIELD_EXIT_CODE.to_owned(), ValueType::integer()),
    ])
}

/// The `Exec` effect signature (proposal §3): `exec : Command ↠ Output`.
#[inline]
#[must_use]
pub fn exec() -> EffectSig
{
    EffectSig::new(EXEC.into(), vec![EffectOp::new(
        EXEC_RUN.into(),
        command_ty(),
        exec_result_ty(),
    )])
}

/// The `Fs::write` payload type `{path : String, contents : String}`.
#[inline]
#[must_use]
fn write_ty() -> ValueType
{
    ValueType::record([
        (FIELD_PATH.to_owned(), ValueType::string()),
        (FIELD_CONTENTS.to_owned(), ValueType::string()),
    ])
}

/// The `Fs::stat` reply type `{kind : String, size : Integer}`.
#[inline]
#[must_use]
fn stat_ty() -> ValueType
{
    ValueType::record([
        (FIELD_KIND.to_owned(), ValueType::string()),
        (FIELD_SIZE.to_owned(), ValueType::integer()),
    ])
}

/// The `Fs` effect signature (proposal §5): read / write / glob / stat /
/// mkdir / tempdir / `ls_files` over `std::fs`.
#[inline]
#[must_use]
pub fn fs() -> EffectSig
{
    let string_list = ValueType::list(ValueType::string());
    EffectSig::new(FS.into(), vec![
        EffectOp::new(FS_READ.into(), ValueType::string(), ValueType::string()),
        EffectOp::new(FS_WRITE.into(), write_ty(), ValueType::Unit),
        EffectOp::new(FS_GLOB.into(), ValueType::string(), string_list.clone()),
        EffectOp::new(FS_STAT.into(), ValueType::string(), stat_ty()),
        EffectOp::new(FS_MKDIR.into(), ValueType::string(), ValueType::Unit),
        EffectOp::new(FS_TEMPDIR.into(), ValueType::Unit, ValueType::string()),
        EffectOp::new(FS_CWD.into(), ValueType::Unit, ValueType::string()),
        EffectOp::new(FS_LS_FILES.into(), ValueType::string(), string_list),
    ])
}

/// The `Env` effect signature: read-only `get` / `path` over the process
/// environment (the scoped `with-env` variant is deferred).
#[inline]
#[must_use]
pub fn env() -> EffectSig
{
    EffectSig::new(ENV.into(), vec![
        EffectOp::new(ENV_GET.into(), ValueType::string(), ValueType::string()),
        EffectOp::new(
            ENV_PATH.into(),
            ValueType::Unit,
            ValueType::list(ValueType::string()),
        ),
    ])
}

/// The `Proc` effect signature: `exit : Integer ↠ Unit` (the reply type is
/// nominal — `exit` truncates the run and never resumes).
#[inline]
#[must_use]
pub fn proc() -> EffectSig
{
    EffectSig::new(PROC.into(), vec![EffectOp::new(
        PROC_EXIT.into(),
        ValueType::integer(),
        ValueType::Unit,
    )])
}
