//! The polarized command IL, the static focusing translation, and the
//! operational **L machine** — phases **L0** and **L1** of the sequent-machines
//! kernel (the sequent-kernel proposal; decisions K1/K7,
//! ADR-65).
//!
//! This crate carries the sequent kernel through its operational phase, beside
//! the frozen CBPV core and touching no frozen-core code: it consumes the
//! public [`gandr_core_checker`] surface only.
//!
//! - **L0** reifies the polarized System-L / λμμ̃ **command IL** (§2) as
//!   arena-resident, inspectable IR and gives the **static focusing translation
//!   `𝓕`** (§3) that bridges the checked core into it.
//! - **L1** runs the focused IL: the iterative [`machine::LMachine`] (§4, §6)
//!   over the two-region [`store::Store`] (call-by-need cells + a frame
//!   region), executing the full effect / control surface — `perform` /
//!   `handle` / `resume` / `reset` / `shift` — gated against the CEK oracle by
//!   the `L-run ∘ 𝓕 ≡ run` [`differential`] (§9).
//!
//! What remains is the phase-**L2** fusion engine (2-cells on command seams)
//! and the listed L1 readback residuals (the un-focusing `𝓕⁻¹`, §7a).
//!
//! # L0 — the command IL and the focusing translation
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
pub mod unfocus;

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
