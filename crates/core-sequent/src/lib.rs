//! The polarized command IL and the static focusing translation — phase **L0**
//! of the sequent-machines kernel
//! (`docs/gandr/spec/proposal-sequent-kernel.md`; decisions K1/K7, ADR-65).
//!
//! This crate is the first, purely-additive phase of the sequent kernel: it
//! reifies the polarized System-L / λμμ̃ **command IL** (§2) as arena-resident,
//! inspectable IR beside the frozen CBPV core, and gives the **static focusing
//! translation `𝓕`** (§3) that bridges the checked core into it. It builds
//! nothing operational — no L machine (that is phase L1), no
//! store, no effect execution, no fusion engine (phase L2) — and it touches no
//! frozen-core code: it consumes the public [`gandr_core_checker`] surface
//! only.
//!
//! # The two halves
//!
//! - [`il`] — the three node families ([`il::ProducerNode`],
//!   [`il::ConsumerNode`], [`il::CommandNode`]) over an arena
//!   ([`il::CommandArena`]) that reuses the ADR-50 `NodeArena` carrier, so the
//!   IL shares the workspace's arena/`NodeId` idioms rather than reintroducing
//!   `Rc`/`Box` recursion.
//! - [`focus`] — `𝓕` (and the value/stack companions `𝓥`/`𝓚`), taking a
//!   checked-core [`gandr_core_checker::syntax::Comp`] /
//!   [`gandr_core_checker::syntax::Value`] to a focused command. It is the only
//!   entry into the IL, administrative-redex-avoiding, and **total** on
//!   well-formed core terms.
//!
//! # Correspondence and adaptations
//!
//! The frozen core carries more formers than the §3 sketch spells out. Every
//! former is translated in the same focusing discipline; the divergences from
//! the bare sketch are recorded as `A-*` adaptation notes on [`focus`] and
//! [`il`]. The two strongest, spec-anchored correspondences are (§1): a **stack
//! frame is a consumer** — `Stack::Arg` is the `ap` coterm, `Stack::Bind` is
//! `μ̃`, `Stack::Prj` is a projection destructor — and a **`λ` / lazy pair is a
//! `cocase`** (negative intro eliminated by copatterns). The effect / control
//! surface (§6) is translated total and inspectable, with the explicit caveat
//! that its *operational* fidelity is the L1 differential, not this phase.
//!
//! # Inspection and the well-formedness face
//!
//! - [`pretty`] renders a command in the §2.1 concrete notation
//!   (depth-bounded).
//! - [`inspect`] is the §9 inspection surface: node population, a provenance
//!   histogram that un-sugars a focused term to its source constructs, and a
//!   dump.
//! - [`check`] is the typed-IL debug-assertion face (§2.3): reference
//!   integrity, scope, focus, arity, and polarity consistency — what is
//!   decidable from the command alone without the source types the L0 carrier
//!   does not retain.
//!
//! # The phase-L0 gate
//!
//! `𝓕` is **total on the corpus**: the `corpus_totality` integration test
//! lowers every model and pathological corpus program through the existing
//! pipeline and runs [`focus::focus_term`] on every lowered item — no panic,
//! and [`check::wellformed`] holds (empty free covariables) for all of them.
//! Property tests (`focus_properties`) exercise `𝓕` over a generator of
//! well-formed core terms.

#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps the unit and property tests \
                  readable (docs/workflow/rust.md)"
    )
)]

extern crate alloc;

pub mod boundary;
pub mod check;
pub mod differential;
pub mod focus;
pub mod il;
pub mod inspect;
pub mod machine;
pub mod pretty;
pub mod store;

pub use crate::check::CheckError;
pub use crate::check::Frees;
pub use crate::check::wellformed;
pub use crate::focus::FocusError;
pub use crate::focus::FocusOrigin;
pub use crate::focus::Focused;
pub use crate::focus::focus_comp;
pub use crate::focus::focus_term;
pub use crate::focus::focus_value;
pub use crate::machine::LMachine;
pub use crate::machine::LValue;
pub use crate::store::Store;
