//! The source-level module surface over the canonical host signatures.
//!
//! [`gandr_core_checker::effect::host`] owns the canonical `Exec` / `Fs` /
//! `Proc` / `Env` signatures alongside the representation-independent host
//! seam. This module explicitly re-exports that signature API and adds only the
//! source-facing [`HostModule`] / [`HostMember`] metadata the lowerer needs.
//! Keeping signatures with the seam avoids table duplication and any
//! signature-driven surface-engine ↔ runtime-effects coupling; the engine's
//! only runtime edge is the host-capability adapter `run::run_source` composes.
//!
//! The **host modules** ([`HOST_MODULES`]) are the source-level surface
//! (`host-module surface`): a call `fs.read(path)` whose head is a known host
//! module elaborates to `perform Fs::read path` exactly as an FFI extern call
//! does (proposal-ffi.md §3.1) — module-select ⇒ perform against a known
//! [`EffectSig`], so the effect row records the host reach.
//!
//! **The gate is no longer syntactic.** `fs` / `env` / `proc` are bindings in
//! the outermost visible scope ([`crate::recognition`]), which reads this table
//! once to seed them; a source declaration of the same name shadows them under
//! the scope engine's ordinary policy. This module owns what a host module
//! *is* — its signature, members, and parameter shapes — and decides nothing
//! about recognition.
//!
//! The current surface assumes vacuous `Σ` and multi-shot resumption; the
//! runtime host installs the ambient handler for these signatures.

use gandr_core_checker::effect::EffectSig;
pub use gandr_core_checker::effect::host::ENV;
pub use gandr_core_checker::effect::host::ENV_GET;
pub use gandr_core_checker::effect::host::ENV_PATH;
pub use gandr_core_checker::effect::host::EXEC;
pub use gandr_core_checker::effect::host::EXEC_RUN;
pub use gandr_core_checker::effect::host::FIELD_ARGS;
pub use gandr_core_checker::effect::host::FIELD_CONTENTS;
pub use gandr_core_checker::effect::host::FIELD_EXIT_CODE;
pub use gandr_core_checker::effect::host::FIELD_KIND;
pub use gandr_core_checker::effect::host::FIELD_MODE;
pub use gandr_core_checker::effect::host::FIELD_PATH;
pub use gandr_core_checker::effect::host::FIELD_PROGRAM;
pub use gandr_core_checker::effect::host::FIELD_SIZE;
pub use gandr_core_checker::effect::host::FIELD_STDERR;
pub use gandr_core_checker::effect::host::FIELD_STDOUT;
pub use gandr_core_checker::effect::host::FS;
pub use gandr_core_checker::effect::host::FS_CWD;
pub use gandr_core_checker::effect::host::FS_GLOB;
pub use gandr_core_checker::effect::host::FS_LS_FILES;
pub use gandr_core_checker::effect::host::FS_MKDIR;
pub use gandr_core_checker::effect::host::FS_READ;
pub use gandr_core_checker::effect::host::FS_STAT;
pub use gandr_core_checker::effect::host::FS_TEMPDIR;
pub use gandr_core_checker::effect::host::FS_WRITE;
pub use gandr_core_checker::effect::host::MODE_CAPTURED;
pub use gandr_core_checker::effect::host::MODE_INHERIT;
pub use gandr_core_checker::effect::host::PROC;
pub use gandr_core_checker::effect::host::PROC_EXIT;
pub use gandr_core_checker::effect::host::env;
pub use gandr_core_checker::effect::host::exec;
pub use gandr_core_checker::effect::host::fs;
pub use gandr_core_checker::effect::host::proc;

use crate::boundary::HostMemberIndex;
use crate::boundary::HostModuleIndex;
use crate::boundary::HostOperation;

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

/// Looks up a host module by its position in [`HOST_MODULES`].
///
/// The index is what the outermost scope's
/// [`crate::recognition::Recognized::HostNamespace`] binding carries, so a
/// resolved host name reaches its signature without a second name lookup —
/// which is the point of the graduation: one authority decides *which* module
/// a name means, and this table says what that module is.
///
/// # Contract
/// - ensures: `Some` exactly when `module` indexes [`HOST_MODULES`].
/// - panics: none.
#[inline]
#[must_use]
pub fn host_module_at(module: HostModuleIndex) -> Option<&'static HostModule>
{
    HOST_MODULES.get(module.0)
}

/// Looks up one member by its position in its module's member list.
///
/// # Contract
/// - ensures: `Some` exactly when `module` indexes [`HOST_MODULES`] and
///   `member` indexes that module's members.
/// - panics: none.
#[inline]
#[must_use]
pub fn host_member_at(
    module: HostModuleIndex,
    member: HostMemberIndex,
) -> Option<(&'static HostModule, &'static HostMember)>
{
    let module = host_module_at(module)?;
    module
        .members
        .get(member.0)
        .map(|found| (module, core::convert::identity(found)))
}

#[cfg(test)]
mod tests
{
    use alloc::vec;

    use gandr_core_checker::term::types::ValueType;

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

    /// Positional lookup addresses exactly the table and nothing past it, and
    /// a member index is read against its own module.
    #[test]
    fn host_lookup_by_position_is_exact()
    {
        for (index, module) in HOST_MODULES.iter().enumerate() {
            let found = host_module_at(HostModuleIndex(index)).expect("every position resolves");
            assert_eq!(module.name, found.name, "position {index} names its module");
            for (member_index, member) in module.members.iter().enumerate() {
                let (owner, found_member) =
                    host_member_at(HostModuleIndex(index), HostMemberIndex(member_index))
                        .expect("every member position resolves");
                assert_eq!(
                    module.name, owner.name,
                    "a member resolves in its own module"
                );
                assert_eq!(
                    member.op, found_member.op,
                    "member {member_index} of {}",
                    module.name
                );
            }
            assert!(
                host_member_at(
                    HostModuleIndex(index),
                    HostMemberIndex(module.members.len())
                )
                .is_none(),
                "one past the last member of {} resolves to nothing",
                module.name
            );
        }
        assert!(
            host_module_at(HostModuleIndex(HOST_MODULES.len())).is_none(),
            "one past the last module resolves to nothing"
        );
    }
}
