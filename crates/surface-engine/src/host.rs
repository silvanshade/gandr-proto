//! The source-level module surface over the canonical host signatures
//! (`docs/gandr/spec/effects-control-shell.md` §3/§5).
//!
//! [`gandr_core_checker::host`] owns the canonical `Exec` / `Fs` / `Proc` /
//! `Env` signatures alongside the representation-independent host seam.
//! This module explicitly re-exports that signature API and adds only the
//! source-facing [`HostModule`] / [`HostMember`] metadata the lowerer needs.
//! Keeping signatures with the seam avoids table duplication and any
//! signature-driven surface-engine ↔ runtime-host coupling; the engine's only
//! runtime edge is the host-capability adapter `run::run_source` composes.
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

use gandr_core_checker::effect::EffectSig;
pub use gandr_core_checker::host::ENV;
pub use gandr_core_checker::host::ENV_GET;
pub use gandr_core_checker::host::ENV_PATH;
pub use gandr_core_checker::host::EXEC;
pub use gandr_core_checker::host::EXEC_RUN;
pub use gandr_core_checker::host::FIELD_ARGS;
pub use gandr_core_checker::host::FIELD_CONTENTS;
pub use gandr_core_checker::host::FIELD_EXIT_CODE;
pub use gandr_core_checker::host::FIELD_KIND;
pub use gandr_core_checker::host::FIELD_MODE;
pub use gandr_core_checker::host::FIELD_PATH;
pub use gandr_core_checker::host::FIELD_PROGRAM;
pub use gandr_core_checker::host::FIELD_SIZE;
pub use gandr_core_checker::host::FIELD_STDERR;
pub use gandr_core_checker::host::FIELD_STDOUT;
pub use gandr_core_checker::host::FS;
pub use gandr_core_checker::host::FS_CWD;
pub use gandr_core_checker::host::FS_GLOB;
pub use gandr_core_checker::host::FS_LS_FILES;
pub use gandr_core_checker::host::FS_MKDIR;
pub use gandr_core_checker::host::FS_READ;
pub use gandr_core_checker::host::FS_STAT;
pub use gandr_core_checker::host::FS_TEMPDIR;
pub use gandr_core_checker::host::FS_WRITE;
pub use gandr_core_checker::host::MODE_CAPTURED;
pub use gandr_core_checker::host::MODE_INHERIT;
pub use gandr_core_checker::host::PROC;
pub use gandr_core_checker::host::PROC_EXIT;
pub use gandr_core_checker::host::env;
pub use gandr_core_checker::host::exec;
pub use gandr_core_checker::host::fs;
pub use gandr_core_checker::host::proc;

use crate::boundary::HostModuleName;
use crate::boundary::HostOperation;
use crate::boundary::MatchDecision;

// --- The source-level host-module surface (host-module surface)
// -----------------------

/// One source-callable member of a [`HostModule`] — `read` in `fs.read(…)`.
///
/// The member name doubles as the effect-operation name; `params` names the
/// member's parameters in declaration order and fixes both the arity check and
/// the payload shape (see [`HostMember::params`]).
#[derive(Debug)]
pub struct HostMember
{
    /// The surface member name — also the
    /// [`EffectOp::name`](gandr_core_checker::effect::EffectOp::name) the call
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
    pub fn member<'operation, O>(
        &self,
        operation: O,
    ) -> Option<&HostMember>
    where
        O: Into<HostOperation<'operation>>,
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
pub fn is_host_module<'name, N>(name: N) -> MatchDecision
where
    N: Into<HostModuleName<'name>>,
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
pub fn host_module<'name, N>(name: N) -> Option<&'static HostModule>
where
    N: Into<HostModuleName<'name>>,
{
    let name = name.into();
    HOST_MODULES.iter().find(|module| module.name == name.0)
}

#[cfg(test)]
mod tests
{
    use alloc::vec;

    use gandr_core_checker::types::ValueType;

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
            for op in sig.ops() {
                assert!(
                    module.member(op.name().as_ref()).is_some(),
                    "{}::{} must have source-module metadata",
                    sig.name().as_ref(),
                    op.name().as_ref()
                );
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
