#![expect(
    clippy::result_large_err,
    reason = "TypeError retains full types for diagnostics across incremental boundaries."
)]

//! Item-granular incremental typing for the gandr language
//! (`incremental-pipeline.md` §"Checkpoints and the reuse rule" through
//! §"Derivation merging and identity stability").
//!
//! An editing session re-types its program after every edit. Doing that from
//! scratch each time is the cost this crate removes: it re-types only the
//! region an edit reached and **re-validates** — never blindly reuses — the
//! rest, at the granularity of the top-level item.
//!
//! # What this crate provides
//!
//! - [`region`] — the **parser-agnostic item seam**. [`region::Item`] is one
//!   lowered top-level item (an optional definition name, an optional
//!   ascription, and the lowered core term), [`region::Program`] is the ordered
//!   items of one revision, and [`region::ItemSource`] is the trait a front end
//!   implements to produce them. Nothing here names a parser, and that is
//!   load-bearing rather than tidy: the unchanged-region test is structural
//!   equality over exactly those three fields, so a front end's spans, origin
//!   tables, and node identities cannot leak into the reuse decision.
//! - [`footprint`] — the **dependency footprint** of one item
//!   ([`footprint::footprint_of`]): the ambient-context names its term read
//!   ([`footprint::Footprint::names`]) plus two conservative flags. The scan
//!   over-approximates by construction — a core node it cannot represent as a
//!   read set marks the footprint [`footprint::Footprint::opaque`], which costs
//!   reuse and never soundness.
//! - [`checkpoint`] — the **validated-resume engine**.
//!   [`checkpoint::checkpoint_program`] types a program from scratch into a
//!   [`checkpoint::Checkpoints`] set, one [`checkpoint::ItemCheckpoint`] per
//!   item; [`checkpoint::resume`] types an edited program against that set,
//!   adopting an item's cached [`checkpoint::ItemTyping`] exactly when its
//!   lowered term is unchanged **and** no name in its footprint had its binding
//!   change. An item's identity survives inserts and deletes because the edit
//!   is *spliced* onto an order-maintenance structure
//!   ([`gandr_theory_orders::OrderMaintenance`]) rather than keyed on position;
//!   an order structure that cannot admit the edit degrades to a full re-type
//!   rather than to a partial answer.
//! - [`boundary`] — the semantic wrappers those signatures carry in place of
//!   bare primitives.
//!
//! # The base context this crate defaults to
//!
//! [`checkpoint::checkpoint_program`] and [`checkpoint::resume`] type against
//! the **empty** base context; [`checkpoint::checkpoint_with`] and
//! [`checkpoint::resume_with`] are the same computations with the base given
//! explicitly. The empty default is the only base a parser-agnostic engine can
//! name — a prelude is a *front end's* vocabulary, and this crate names no
//! front end — so a caller that has one (a surface prelude, a REPL's
//! accumulated context) supplies it at the explicit-base form and gets the
//! identical engine.
//!
//! # The soundness gate
//!
//! Adoption skips work, so the standing obligation is that the skips never
//! change the answer: for **every** edit, `resume(base, edited)` yields exactly
//! the typings `checkpoint_program(edited)` computes from scratch. That
//! differential is `tests/incremental.rs`, over adoption, invalidation,
//! structural item-list edits, and property-generated edits, driven through the
//! seam by an in-tree [`region::ItemSource`] test double so the gate needs no
//! parser.
//!
//! # What this crate is not
//!
//! Granularity below the item — a within-item dirty frontier, per-term-node
//! checkpoints coupled to a solver — is not here; nor is evaluation, which is a
//! driver's concern (a checkpoint records only what re-typing must reproduce).

extern crate alloc;

pub mod boundary;
pub mod checkpoint;
pub mod footprint;
pub mod persistence;
pub mod region;
pub mod session;
pub mod stream;
