//! The representation-independent host-effect seam.
//!
//! This boundary carries operations no source-level handler claims to an
//! ambient host interpreter (`docs/gandr/spec/effects-control-shell.md` §3/§5).
//!
//! This is a **preserved boundary**: it is expressed over the public
//! [`Value`] / [`EffectSig`] surface and the operation *name* only — never a
//! machine continuation frame — so it stays representation-independent across
//! the evaluator that realizes it. That representation-independence is what let
//! the seam survive the port from the CEK oracle to its successor: the two
//! machines presented the *identical* seam to the host, and the L machine
//! realization in `gandr_core_sequent` (`machine::run_comp_with_host`) is the
//! durable driver the runtime-host binds against — the CEK drivers retired with
//! the CEK at B1 stage F.
//!
//! The three types here are the seam's whole public vocabulary: a [`HostOp`]
//! the machine hands out, a [`HostReply`] the host hands back, and the
//! [`HostHandler`] that mediates. The L machine offers each host-interceptable
//! `perform` to a [`HostHandler`] and either resumes or declines it; this
//! module is the seam's durable home precisely because the boundary outlives
//! any one machine.
//!
//! It is its own module: [`crate::boundary`] is the newtype vocabulary,
//! [`crate::effect`] is the effect-row algebra, and this module owns the
//! representation-independent host seam plus its canonical signatures.

use alloc::string::String;
use alloc::vec;

use crate::boundary::OperationName;
use crate::effect::EffectOp;
use crate::effect::EffectSig;
use crate::syntax::Value;
use crate::types::ValueType;

/// The host's reply to an intercepted effect operation.
///
/// A [`HostHandler`] either resumes the operation with a value or declines it,
/// leaving the machine to take its ordinary step (which blames
/// [`crate::outcome::Blame::PerformNoHandler`] on an unclaimed `perform`).
#[derive(Clone, Debug)]
pub enum HostReply
{
    /// Resume the intercepted operation with this reply value — the machine
    /// continues as if an always-resume ambient handler resumed the
    /// continuation (see [`HostHandler`] for the exact intercept-set
    /// invariant).
    Resume(
        /// The value delivered to the performing continuation.
        Value,
    ),
    /// Decline the operation — the machine takes its ordinary step, blaming
    /// [`crate::outcome::Blame::PerformNoHandler`] when no in-term handler
    /// catches it.
    Unhandled,
}

/// A host-side interpreter for effect operations no source-level handler
/// claims.
///
/// **Invariant.** The host intercepts EXACTLY the operations that would
/// otherwise [`crate::outcome::Blame::PerformNoHandler`] — those no
/// source-level handler claims across the structural prefix. This equals an
/// identity-return, always-resume ambient handler ONLY in the absence of a
/// `reset` and of intervening non-matching handlers: a `perform` cut off from
/// its in-term handler by an intervening `reset` or a non-matching `handle` is
/// host-interceptable here even though that handler encloses it in a
/// from-the-outside reading (the v0 structural single-handler scope, pinned by
/// the L machine's host-seam differential cases
/// `host_seam_intercepts_across_a_reset`
/// and `host_seam_intercepts_across_an_intervening_handler`).
///
/// The L machine (`gandr_core_sequent::machine::run_comp_with_host`) offers the
/// host every such `perform` and either resumes ([`HostReply::Resume`]) or
/// declines ([`HostReply::Unhandled`]). The handler sees only the public
/// [`Value`] / [`EffectSig`] surface and the operation *name* — never a
/// continuation frame — so the seam stays representation-independent across
/// machine/arena changes.
///
/// Any `FnMut(&EffectSig, &str, &Value) -> HostReply` is a `HostHandler` via
/// the blanket impl below, so a closure suffices for the common case.
pub trait HostHandler
{
    /// Handles the operation `op` of signature `sig` performed with `payload`,
    /// returning the host's [`HostReply`].
    ///
    /// # Contract
    /// - ensures: [`HostReply::Resume`] to continue the machine with a reply,
    ///   or [`HostReply::Unhandled`] to let it take its ordinary (blaming)
    ///   step.
    fn handle<'source, O>(
        &mut self,
        sig: &EffectSig,
        op: O,
        payload: &Value,
    ) -> HostReply
    where
        O: Into<OperationName<'source>>;
}

impl<F> HostHandler for F
where
    F: FnMut(&EffectSig, &str, &Value) -> HostReply,
{
    #[inline]
    fn handle<'source, O>(
        &mut self,
        sig: &EffectSig,
        op: O,
        payload: &Value,
    ) -> HostReply
    where
        O: Into<OperationName<'source>>,
    {
        let op = op.into();
        self(sig, op.as_ref(), payload)
    }
}

/// A host-interceptable effect operation carried out of the machine as an
/// **owned** payload.
///
/// Owned by design: under the coming arena the payload is a node with no
/// `&Value` to borrow, so the seam hands the host a self-contained operation
/// over the public [`Value`] API rather than a borrow into machine internals.
#[derive(Clone, Debug)]
pub struct HostOp
{
    /// The effect signature `E` the operation belongs to.
    pub sig: EffectSig,
    /// The operation's name (an operation of `sig`).
    pub op: String,
    /// The performed payload, with type annotations stripped.
    pub payload: Value,
}

impl HostOp
{
    /// Assembles a host-interceptable operation over the public surface.
    ///
    /// [`HostOp`]'s public constructor: how a machine *outside* this crate —
    /// the L machine realization in `gandr_core_sequent` — builds the offer it
    /// hands to a [`HostHandler`]. This is what keeps the seam a single shared
    /// boundary across machines rather than two parallel vocabularies (the
    /// retired CEK oracle packaged the same triple).
    ///
    /// # Contract
    /// - ensures: a [`HostOp`] carrying `sig`, the owned operation name of
    ///   `op`, and `payload` verbatim (the caller has already read the payload
    ///   back over the public [`Value`] surface, annotation-stripped).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn new(
        sig: EffectSig,
        op: OperationName<'_>,
        payload: Value,
    ) -> Self
    {
        Self {
            sig,
            op: String::from(op.as_ref()),
            payload,
        }
    }
}

// --- Canonical host signatures
// -------------------------------------------------
//
// The canonical host effect signatures the runtime handles (proposal §3/§5:
// the thin v0 host handler intercepts the flat `Exec`/`Fs`/`Proc`/`Env`
// signature, performs the syscall, and resumes the delimited continuation
// with the reply). These builders and name constants are the single source of
// truth two faces share: a program author builds a `Comp::perform` against
// `exec()` and the native dispatcher handles the same [`EffectSig::name`] /
// operation names, so the faces never drift. v0 is sound because `Σ` is
// vacuous and resumption is multi-shot — a captured continuation prefix is
// reified as a plain stack value with the handler reinstalled — so the host is
// an always-resume ambient handler on the seam. The surface engine re-exports
// these definitions and adds its source-facing `fs.read(…)`-style module
// metadata and `#!{ … }` shell-block lowering; the runtime host binds them
// for native dispatch.

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
/// The record label carrying the spawn mode in an `Exec::exec` payload:
/// [`MODE_CAPTURED`] or [`MODE_INHERIT`].
///
/// The spawn-mode design: the captured reply is the typed contract for a
/// consumed result (splices, bound values, scripts), while a discarded reply
/// runs with inherited stdio so an interactive program behaves.
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
/// [`MODE_CAPTURED`] (the drift-safe default).
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

/// The `Exec` effect signature: `exec : Command ↠ Output`.
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

/// The `Fs` effect signature: read / write / glob / stat / mkdir / tempdir /
/// `ls_files` over the host filesystem.
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

/// The `Proc` effect signature: `exit : Integer ↠ Unit`.
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
