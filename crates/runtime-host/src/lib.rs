//! The headless host-effect runtime for gandr: the thin v0 host handler of
//! `docs/gandr/spec/effects-control-shell.md` §3/§5 — a top-level handler that
//! intercepts the flat `Exec`/`Fs`/`Proc`/`Env` signature, performs the
//! syscall, and resumes the delimited continuation with the reply.
//!
//! This crate is the *host* side of the effect seam: the native interpreter for
//! the `Exec` / `Fs` / `Env` / `Proc` operations no source-level handler
//! claims. It runs a gandr program **headlessly** — no reedline, no TUI.
//!
//! The durable driver is the **L machine**
//! ([`gandr_core_sequent::machine::run_comp_with_host`]): [`run_program`] hands
//! a lowered program and a host handler to it and reads back an
//! [`Eval`](gandr_core_checker::outcome::Eval). The seam it binds is the
//! representation-independent host-effect boundary in
//! [`gandr_core_checker::host`] — the `(signature name, operation name,
//! payload)` projection both the L machine and the retiring CEK oracle present
//! identically, which is what let this runtime port from the CEK binding to the
//! L driver without changing observable outcomes (asserted by the driver's
//! differential tests).
//!
//! Two runtime faces, both consuming the canonical signature API in
//! [`gandr_core_checker::host`]:
//! - [`ShellHandler`] — the [`gandr_core_checker::host::HostHandler`]-shaped
//!   dispatcher ([`ShellHandler::dispatch`]) that carries each intercepted
//!   operation out to a real syscall over `std::process` / `std::fs` /
//!   `std::env`.
//! - [`run_program`] — the driver that flows a program through the host-effect
//!   seam to a [`ShellOutcome`], with `Proc::exit` and fatal syscalls
//!   truncating the run. [`run_program_with_prelude`] drives the same seam with
//!   an ambient value prelude installed.
//!
//! **Soundness (v0).** `Σ` is vacuous and resumption is multi-shot — a
//! captured continuation prefix is reified as a plain stack value with the
//! handler reinstalled, so a captured continuation may be resumed any number
//! of times — so the host is an always-resume ambient handler on the seam; the
//! eager OS pipe between external commands is a stopgap, NOT the typed A8
//! `Pipe` session; the op set is named `Exec`/`Fs`/`Proc`/`Env` and does not
//! appropriate the reserved A8 name `Shell`.
//!
//! Source-text convenience entry points stay in the surface engine:
//! `gandr_surface_engine::run::run_source` composes the engine's lowering,
//! linking, and prelude checking with [`run_program_with_prelude`]. This
//! headless host accepts already-lowered, hand-built
//! [`Comp`](gandr_core_checker::syntax::Comp) programs through [`run_program`].
//!
//! ```
//! use gandr_core_checker::host as sig;
//! use gandr_core_checker::syntax::Comp;
//! use gandr_core_checker::syntax::Value;
//! use gandr_runtime_host::run_program;
//!
//! // perform Exec::exec {program: "true", args: []} >>= r. ret r
//! let command = Value::record([
//!     (sig::FIELD_PROGRAM.to_owned(), Value::string("true")),
//!     (sig::FIELD_ARGS.to_owned(), Value::list(Vec::new())),
//! ]);
//! let program = Comp::bind(
//!     Comp::perform(sig::exec(), sig::EXEC_RUN, command),
//!     "r",
//!     Comp::ret(Value::var("r")),
//! );
//! let outcome = run_program(&program);
//! let exit_code = outcome
//!     .returned()
//!     .and_then(Value::as_record)
//!     .and_then(|fields| fields.get(sig::FIELD_EXIT_CODE))
//!     .and_then(|value| value.as_int())
//!     .map(i64::from);
//! assert_eq!(exit_code, Some(0));
//! ```

#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(
        unknown_lints,
        non_topologically_sorted_functions,
        reason = "test modules share helpers called in per-test orders; no single module arrangement satisfies every caller-before-callee pair"
    )
)]

extern crate alloc;

pub mod boundary;
pub mod codec;
pub mod driver;
pub mod error;
pub mod handler;

pub use crate::driver::ShellOutcome;
pub use crate::driver::run_program;
pub use crate::driver::run_program_with_prelude;
pub use crate::error::ShellError;
pub use crate::handler::ShellHandler;
