//! Core CBPV (call-by-push-value) type checking for the gandr language.
//!
//! This crate implements Stage 1 of the gandr roadmap (milestone A1 of the
//! A-track): the core CBPV bidirectional type system of the type-system design
//! record §"The core call-by-push-value calculus", realized twice:
//!
//! - [`judgements::checker`] — the direct-style *recursive* bidirectional
//!   checker;
//! - [`machine`] — the *defunctionalized typing machine* obtained from the
//!   recursive checker by the functional correspondence (CPS transform, then
//!   defunctionalization of the continuations into an explicit stack of
//!   frames), per the typing-machine design record and ADR-9.
//!
//! Both implementations are kept in-tree and property-tested for
//! *step-for-step* agreement: the recursive checker logs a
//! [`machine::control::Control`] event at every call entry (`Descend`) and
//! every *successful* call exit (`Return`) — a failing call logs no `Return`,
//! exactly as the machine takes no `Return` step past the failing frame — and
//! that event log must equal the sequence of control registers the machine
//! passes through. This is the verification anchor the two realizations are
//! held to.
//!
//! # The module clusters
//!
//! The modules are grouped by what they are, so that each candidate crate
//! boundary is visible before anyone tries to cut one:
//!
//! - [`term`] — the substrate the judgements are stated over: syntax, types,
//!   contexts, substitution, and type interning;
//! - [`judgements`] — the typing judgement itself and the three that hang off
//!   it: identity-motive instantiation, packages, and sealing;
//! - [`machine`] — the defunctionalized realization, with its control register
//!   and its stack-typing judgement;
//! - [`discipline`] — the rules the checker applies beside the judgement:
//!   subsumption, usage grades, total error marking, and the semantic-wrapper
//!   boundary;
//! - [`nbe`], [`unify`], [`effect`], [`kernel_bridge`] — the engines, each
//!   already its own directory;
//! - [`error`], [`outcome`], [`prim`], [`nominal`] — the leaf vocabularies
//!   every cluster names.
//!
//! Scope discipline (Stage 1): core CBPV plus two spec-grounded A2
//! extensions, each landed in checker, machine, and conformance generators
//! in lockstep per ADR-9 — the conformance suite and the free generators it
//! shares live in `gandr-core-checker-tools`, one tier above this crate:
//!
//! - **A2.1 integer literals** — [`term::syntax::Value::Int`] inferring the
//!   rigid `Integer` atom (the A2.1 literal axiom in the gandr roadmap);
//! - **A2.2 holes** — [`term::syntax::Value::Hole`] /
//!   [`term::syntax::Comp::Hole`] and the unknown type
//!   ([`term::types::ValueType::Unknown`] / [`term::types::CompType::Unknown`])
//!   per A2 D5 and the incremental-pipeline design: holes are axioms that infer
//!   `Unknown` and check against anything; subsumption becomes *consistent
//!   subtyping* ([`discipline::subtype`] records the decision tree);
//!   eliminations on `Unknown` use matched types. This is the totality half of
//!   "no parse wall" — the pipeline lowers unparseable/unsupported regions to
//!   holes and the checker accepts every editor state.
//!
//! Unification rides on the same conversion engine rather than beside it:
//! [`unify`] is a solver-machine service over the terms this crate already
//! defines, with metavariables nominated among existing holes so no syntactic
//! former is added and the checker/machine agreement above is inherited rather
//! than re-established. Its answers are certificates a caller re-checks by
//! substituting and asking [`nbe::conv`], which is what pins its equational
//! theory to the checker's own.
//!
//! Incremental re-typing rides on this crate rather than living in it:
//! `gandr-core-incremental` carries the parser-agnostic item seam, the
//! dependency footprints, and the validated-resume checkpoint engine, and
//! drives them through [`machine`] over this crate's [`term::syntax`],
//! [`term::types`], and [`term::ctx`] vocabulary.
//!
//! No grade *constraints* beyond the inline `1 ⊑ r` force check of §"Core
//! rules" (matched-`U` operations emit none), no unions/intersections, no
//! polymorphism, sessions, sharing, or worlds. The representations are kept
//! extensible (non-exhaustive enums) so later stages can add constructors
//! without breaking downstream matches.

extern crate alloc;

pub mod discipline;
pub mod effect;
pub mod error;
pub mod judgements;
pub mod kernel_bridge;
pub mod machine;
pub mod nbe;
pub mod nominal;
pub mod outcome;
pub mod prim;
pub mod term;
pub mod unify;
