//! Resumable push-machine melder and obligation taxonomy for gandr.
//!
//! This crate is the W4′ parser lane of the surface front-end. It owns two
//! artifacts:
//!
//! 1. The **obligation taxonomy** ([`oblig`]): the closed [`Oblig`] severity
//!    ladder, the [`ObligationInstance`] (class plus span), and the [`Delta`]
//!    per-class count array with lexicographic net-then-gross comparison from
//!    highest severity down.
//! 2. The **melder** ([`meld`]): a resumable, first-order push machine
//!    ([`MeldState`]) over the checked `gandr-surface-grammar` PBG. `push` is
//!    primary and total (Shift / Reduce / Degrout, paper Fig. 29); batch
//!    parsing is the derived fold of `push` followed by `commit`. Persistent,
//!    serializable [`Checkpoint`] state (`checkpoint` / `resume`) and a
//!    non-destructive [`finalize`](MeldState::finalize) query complete the
//!    streaming surface.
//!
//! The push seam consumes only the W3′ handoff surface: precedence comparisons
//! at form-group granularity (`PrecDag` checks), form-local material from
//! `Pbg::adjacencies` and the interned regex-context steps, and interned `u32`
//! ids throughout — no string comparison past the caller, no re-derived
//! precedence arithmetic.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(
        clippy::arithmetic_side_effects,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::unwrap_used,
        reason = "the standard test-allow set keeps parser contract tests readable (docs/workflow/rust.md)"
    )
)]

extern crate alloc;

mod label;
mod meld;
mod mold;
mod oblig;
mod parse;

pub use label::Lexeme;
pub use label::Token;
pub use label::label;
pub use meld::Checkpoint;
pub use meld::CheckpointBytes;
pub use meld::CheckpointBytesRef;
pub use meld::CheckpointError;
pub use meld::Completion;
pub use meld::CompletionStatus;
pub use meld::Expected;
pub use meld::FormContinuation;
pub use meld::Frontier;
pub use meld::HeadOperandPresence;
pub use meld::Mark;
pub use meld::MeldError;
pub use meld::MeldState;
pub use meld::MoldAdmissibility;
pub use meld::MoldedTile;
pub use meld::OpenFormPresence;
pub use meld::OperandContinuation;
pub use meld::SpaceText;
pub use meld::TileText;
pub use mold::CandidateCount;
pub use mold::CandidateLabel;
pub use mold::Molder;
pub use mold::SourceText;
pub use mold::TokenText;
pub use mold::candidate_labels;
pub use oblig::Delta;
pub use oblig::DeltaEmptyStatus;
pub use oblig::OBLIG_CLASS_COUNT;
pub use oblig::Oblig;
pub use oblig::ObligClassIndex;
pub use oblig::ObligationCount;
pub use oblig::ObligationInstance;
pub use parse::ParseCleanStatus;
pub use parse::ParseResult;
pub use parse::parse;
