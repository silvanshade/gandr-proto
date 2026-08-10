//! Core CBPV (call-by-push-value) type checking for the gandr language.
//!
//! This crate implements Stage 1 of the gandr roadmap (milestone A1 of the
//! A-track): the core CBPV bidirectional type system of the type-system design
//! record §"The core call-by-push-value calculus", realized twice:
//!
//! - [`checker`] — the direct-style *recursive* bidirectional checker;
//! - [`machine`] — the *defunctionalized typing machine* obtained from the
//!   recursive checker by the functional correspondence (CPS transform, then
//!   defunctionalization of the continuations into an explicit stack of
//!   frames), per the typing-machine design record and ADR-9.
//!
//! Both implementations are kept in-tree and property-tested for
//! *step-for-step* agreement: the recursive checker logs a [`control::Control`]
//! event at every call entry (`Descend`) and every *successful* call exit
//! (`Return`) — a failing call logs no `Return`, exactly as the machine takes
//! no `Return` step past the failing frame — and that event log must equal the
//! sequence of control registers the machine passes through. This is the
//! verification anchor described in ADR-9 and `VISION.md` §6.
//!
//! Scope discipline (Stage 1): core CBPV plus two spec-grounded A2
//! extensions, each landed in checker, machine, and conformance generators
//! in lockstep per ADR-9:
//!
//! - **A2.1 integer literals** — [`syntax::Value::Int`] inferring the rigid
//!   `Integer` atom (the A2.1 literal axiom in the gandr roadmap);
//! - **A2.2 holes** — [`syntax::Value::Hole`] / [`syntax::Comp::Hole`] and the
//!   unknown type ([`types::ValueType::Unknown`] /
//!   [`types::CompType::Unknown`]) per A2 D5 and the incremental-pipeline
//!   design: holes are axioms that infer `Unknown` and check against anything;
//!   subsumption becomes *consistent subtyping* ([`subtype`] records the
//!   decision tree); eliminations on `Unknown` use matched types. This is the
//!   totality half of "no parse wall" — the pipeline lowers
//!   unparseable/unsupported regions to holes and the checker accepts every
//!   editor state.
//!
//! Incremental re-typing rides on this crate rather than living in it:
//! `gandr-core-incrementality` carries the parser-agnostic item seam, the
//! dependency footprints, and the validated-resume checkpoint engine, and
//! drives them through [`machine`] over this crate's [`syntax`], [`types`], and
//! [`ctx`] vocabulary.
//!
//! No grade *constraints* beyond the inline `1 ⊑ r` force check of §"Core
//! rules" (matched-`U` operations emit none), no unions/intersections, no
//! polymorphism, sessions, sharing, or worlds. The representations are kept
//! extensible (non-exhaustive enums) so later stages can add constructors
//! without breaking downstream matches.

extern crate alloc;

pub mod boundary;
pub mod checker;
pub mod control;
pub mod ctx;
pub mod effect;
pub mod error;
pub mod grade;
pub mod identity;
pub mod intern;
pub mod kernel_bridge;
pub mod machine;
pub mod mark;
pub mod nominal;
pub mod outcome;
pub mod prim;
pub mod stack;
pub mod subst;
pub mod subtype;
pub mod syntax;
pub mod types;

#[cfg(any(test, feature = "gandr_test_strategies"))]
pub mod strategies;

#[cfg(test)]
mod conformance;
