//! The canonical host effect signatures and their source-level module surface
//! (proposal §3/§5, host-effect boundary D4).
//!
//! The v0 op set is named **`Exec` / `Fs` / `Proc` / `Env`** — the reserved
//! name `Shell` denotes the A8 typed-`Pipe` op (`effects-control-shell.md` §3)
//! and is deliberately NOT appropriated. These builders are the single source
//! of truth shared by every face: the lowerer elaborates `#!{ … }` shell
//! blocks and `fs.read(…)`-style host-module calls to `Comp::perform` against
//! them, a program author (`program-authoring work`) builds performs against
//! the same signatures, and the `gandr-runtime-host` handler dispatches on the
//! same [`EffectSig::name`] / operation names (re-exported as
//! `gandr_shell::sig`), so the faces never drift. They live here rather than in
//! `gandr-runtime-host` because the lowerer needs them and
//! `gandr-surface-engine` cannot depend on `gandr-runtime-host` without forming
//! a cycle.
//!
//! The **host modules** ([`HOST_MODULES`]) are the source-level surface
//! (`host-module surface`): a call `fs.read(path)` whose head is a known host
//! module elaborates to `perform Fs::read path` exactly as an FFI extern call
//! does (proposal-ffi.md §3.1) — module-select ⇒ perform against a known
//! [`EffectSig`], so the effect row records the host reach. The gate is
//! syntactic: `fs` / `env` / `proc` are reserved module names, not scoped
//! bindings.
//!
//! v0 assumes vacuous `Σ` and multi-shot resumption; the runtime host installs
//! the ambient handler for these signatures.

use alloc::vec;

use gandr_core_checker::effect::EffectOp;
use gandr_core_checker::effect::EffectSig;
use gandr_core_checker::types::ValueType;

use crate::boundary::HostModuleName;
use crate::boundary::HostOperation;
use crate::boundary::MatchDecision;

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
/// The record label carrying the spawn mode in an `Exec::exec` payload
/// (shell spawn-mode design D4): [`MODE_CAPTURED`] or [`MODE_INHERIT`].
pub const FIELD_MODE: &str = "mode";
/// The `Exec::exec` spawn mode that captures the child's output (shell
/// spawn-mode design D4).
///
/// Captures stdout/stderr into the typed `{stdout, stderr, exit_code}` reply —
/// the contract for every consumed result (`$( … )` splices, bound values,
/// scripts). The lowerer emits this for every `#!{ … }` block and host-escape,
/// so the corpus is unchanged.
pub const MODE_CAPTURED: &str = "captured";
/// The `Exec::exec` spawn mode that inherits the parent's terminal (shell
/// spawn-mode design D4).
///
/// The child inherits the parent's stdin/stdout/stderr, so an interactive
/// program (`vim`, `less`, `ssh`) behaves. Selected by a REPL command line
/// whose reply is discarded; the reply's `stdout`/`stderr` are then empty and
/// only `exit_code` is meaningful.
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
/// The `mode` field carries the shell spawn-mode design D4 spawn mode
/// ([`MODE_CAPTURED`] / [`MODE_INHERIT`]) as the consumption context the
/// lowerer or the driver decided; the handler dispatches on it. A payload with
/// no `mode` field decodes as [`MODE_CAPTURED`] (the drift-safe default), so a
/// hand-built `{program, args}` perform still captures.
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

/// The `Env` effect signature: read-only `get` / `path` over the process
/// environment (the scoped `with-env` variant is deferred, `scoped-environment
/// work`).
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

// --- The source-level host-module surface (host-module surface)
// -----------------------

/// One source-callable member of a [`HostModule`] — `read` in `fs.read(…)`.
///
/// The member name doubles as the effect-operation name; `params` names the
/// member's parameters in declaration order and fixes both the arity check and
/// the payload shape (see [`HostMember::params`]).
#[derive(Debug)]
#[non_exhaustive]
pub struct HostMember
{
    /// The surface member name — also the [`EffectOp::name`] the call
    /// performs.
    pub op: &'static str,
    /// The parameter names, in declaration order. The length is the arity;
    /// the payload convention follows the signature's payload types: zero
    /// parameters perform `()`, one performs the bare argument value, and
    /// several perform the argument record keyed by these names (matching
    /// the FFI argument-record convention, proposal-ffi.md §3.1).
    pub params: &'static [&'static str],
}

/// A host module — a reserved source-level namespace (`fs`, `env`, `proc`)
/// whose member calls elaborate to performs against a host [`EffectSig`].
#[derive(Debug)]
#[non_exhaustive]
pub struct HostModule
{
    /// The surface module name (`fs` in `fs.read`).
    pub name: &'static str,
    /// The signature builder the elaborated `Perform` carries.
    sig: fn() -> EffectSig,
    /// The source-callable members.
    pub members: &'static [HostMember],
}

impl HostModule
{
    /// The effect signature the module's performs carry.
    ///
    /// # Contract
    /// - ensures: every member's [`HostMember::op`] resolves in the returned
    ///   signature (pinned by `host_module_members_resolve_in_their_sig`).
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn sig(&self) -> EffectSig
    {
        (self.sig)()
    }

    /// Looks up a member by its surface name.
    ///
    /// # Contract
    /// - ensures: `Some` exactly when `op` names a member.
    /// - panics: none.
    #[inline]
    #[must_use]
    pub fn member<'operation, O: Into<HostOperation<'operation>>>(
        &self,
        operation: O,
    ) -> Option<&HostMember>
    {
        let operation = operation.into();
        self.members.iter().find(|member| member.op == operation.0)
    }
}

/// The `fs` host module's members (payload/reply types in [`fs`]).
const FS_MEMBERS: &[HostMember] = &[
    HostMember {
        op: FS_READ,
        params: &[FIELD_PATH],
    },
    HostMember {
        op: FS_WRITE,
        params: &[FIELD_PATH, FIELD_CONTENTS],
    },
    HostMember {
        op: FS_GLOB,
        params: &["pattern"],
    },
    HostMember {
        op: FS_STAT,
        params: &[FIELD_PATH],
    },
    HostMember {
        op: FS_MKDIR,
        params: &[FIELD_PATH],
    },
    HostMember {
        op: FS_TEMPDIR,
        params: &[],
    },
    HostMember {
        op: FS_CWD,
        params: &[],
    },
    HostMember {
        op: FS_LS_FILES,
        params: &["dir"],
    },
];

/// The `env` host module's members (payload/reply types in [`env()`]).
const ENV_MEMBERS: &[HostMember] = &[
    HostMember {
        op: ENV_GET,
        params: &["name"],
    },
    HostMember {
        op: ENV_PATH,
        params: &[],
    },
];

/// The `proc` host module's members (payload/reply types in [`proc`]).
const PROC_MEMBERS: &[HostMember] = &[HostMember {
    op: PROC_EXIT,
    params: &["code"],
}];

/// The host modules exposed to source (`host-module surface`): `fs` / `env` /
/// `proc`.
///
/// `exec` is deliberately absent — the `Exec` surface is the `#!{ … }` shell
/// block (and its host-escape splices, `host-escape surface`), not a module
/// call.
pub const HOST_MODULES: &[HostModule] = &[
    HostModule {
        name: "fs",
        sig: fs,
        members: FS_MEMBERS,
    },
    HostModule {
        name: "env",
        sig: env,
        members: ENV_MEMBERS,
    },
    HostModule {
        name: "proc",
        sig: proc,
        members: PROC_MEMBERS,
    },
];

/// Whether `name` is a reserved host-module name (the module/prelude design D2
/// gate: a module namespace is not a record, so `fs.x` never falls through to a
/// structural projection).
///
/// # Contract
/// - ensures: agrees with `host_module(name).is_some()`.
/// - panics: none.
#[inline]
#[must_use]
pub fn is_host_module<'name, N: Into<HostModuleName<'name>>>(name: N) -> MatchDecision
{
    MatchDecision(host_module(name).is_some())
}
/// Looks up a host module by its surface name.
///
/// # Contract
/// - ensures: `Some` exactly when `name` is a reserved host-module name.
/// - panics: none.
#[inline]
#[must_use]
pub fn host_module<'name, N: Into<HostModuleName<'name>>>(name: N) -> Option<&'static HostModule>
{
    let name = name.into();
    HOST_MODULES.iter().find(|module| module.name == name.0)
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// Every member of every host module resolves in the module's effect
    /// signature, and its parameter shape matches the declared payload type:
    /// zero params ⇔ `Unit`, several params ⇔ a record keyed by exactly those
    /// names. This is the never-drift pin between the module surface and the
    /// signatures the handlers dispatch on.
    #[test]
    fn host_module_members_resolve_in_their_sig() -> Result<(), String>
    {
        for module in HOST_MODULES {
            let sig = module.sig();
            for member in module.members {
                let op = sig.op(member.op.into()).ok_or_else(|| {
                    format!(
                        "{}.{} must resolve in {}",
                        module.name,
                        member.op,
                        sig.name().as_ref()
                    )
                })?;
                match member.params.len() {
                    | 0 => assert_eq!(
                        &ValueType::Unit,
                        op.payload(),
                        "{}.{}: zero params must mean a Unit payload",
                        module.name,
                        member.op
                    ),
                    | 1 => {
                        assert_ne!(
                            op.payload(),
                            &ValueType::Unit,
                            "{}.{}: one param must mean a bare (non-Unit) payload",
                            module.name,
                            member.op
                        );
                        // Genuinely bare: a record payload here would pass the
                        // lowerer's bare-value packing but hit the handler's
                        // record decoder — the drift this pin exists to catch.
                        assert!(
                            !matches!(op.payload(), &ValueType::Record(_)),
                            "{}.{}: a one-param member must not declare a record payload",
                            module.name,
                            member.op
                        );
                    },
                    | _ => {
                        let ValueType::Record(ref fields) = *op.payload()
                        else {
                            return Err(format!(
                                "{}.{}: several params must mean a record payload",
                                module.name, member.op
                            ));
                        };
                        let declared: vec::Vec<&str> = fields.keys().map(String::as_str).collect();
                        let mut expected: vec::Vec<&str> = member.params.to_vec();
                        expected.sort_unstable();
                        assert_eq!(
                            declared, expected,
                            "{}.{}: record payload fields must match the params",
                            module.name, member.op
                        );
                    },
                }
            }
        }
        Ok(())
    }

    /// The reserved module names resolve, and non-modules do not.
    #[test]
    fn host_module_lookup_is_exact()
    {
        assert!(is_host_module("fs").0);
        assert!(is_host_module("env").0);
        assert!(is_host_module("proc").0);
        assert!(!is_host_module("exec").0);
        assert!(!is_host_module("string").0);
        assert!(!is_host_module("").0);
    }
}
