//! Core CBPV (call-by-push-value) type checking for the gandr language.
//!
//! This crate implements Stage 1 of the gandr roadmap (milestone A1 of the
//! A-track): the core CBPV bidirectional type system of the type-system design
//! record §"The core call-by-push-value calculus", realized twice:
//!
//! - [`judgements::checker`] — the direct-style *recursive* bidirectional
//!   checker, which is this crate's;
//! - `gandr_core_machine` — the *defunctionalized typing machine* obtained from
//!   the recursive checker by the functional correspondence (CPS transform,
//!   then defunctionalization of the continuations into an explicit stack of
//!   frames), per the typing-machine design record and ADR-9. It is a crate
//!   above this one, and the edge runs one way: the machine names the judgement
//!   layer, and nothing here names the machine.
//!
//! Both implementations are kept in-tree and property-tested for
//! *step-for-step* agreement: the recursive checker logs a
//! [`judgements::control::Control`] event at every call entry (`Descend`) and
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
//! - [`judgements`] — the typing judgement itself, the direction and trace
//!   vocabulary both realizations speak, the stack-typing judgement reified
//!   stacks need, and the two judgements that hang off the first: packages and
//!   sealing;
//! - [`discipline`] — the rules the checker applies beside the judgement:
//!   subsumption and total error marking;
//! - [`kernel_bridge`] — the lowering into the certified kernel's closed
//!   subset, the one engine that stays beside the judgements because its subset
//!   is defined by what they admit.
//!
//! Three neighbours the judgements lean on are not here. `gandr-core-term`
//! carries the substrate all of them are stated over: the syntax, the types,
//! the context, substitution, interning, effect rows, grades, builtins, and the
//! shared error, outcome, and wrapper vocabulary. `gandr-core-nbe` carries the
//! conversion engine subsumption decides its identity endpoints with, and
//! `gandr-core-unify` the solver whose certificates that same engine
//! re-checks.
//!
//! Two of those three edges are real and documented: this crate depends on the
//! substrate and on the conversion engine, and the solver depends on both as
//! well, because a definitional equality is decided in exactly one place. What
//! the substrate buys is that no deciding crate depends on another merely to
//! name a term.
//!
//! Scope discipline (Stage 1): core CBPV plus two spec-grounded A2
//! extensions, each landed in checker, machine, and conformance generators
//! in lockstep per ADR-9 — the conformance suite and the free generators it
//! shares live in `gandr-core-checker-tools`, one tier above this crate:
//!
//! - **A2.1 integer literals** — `gandr_core_term::syntax::Value::Int`
//!   inferring the rigid `Integer` atom (the A2.1 literal axiom in the gandr
//!   roadmap);
//! - **A2.2 holes** — `gandr_core_term::syntax::Value::Hole` /
//!   `gandr_core_term::syntax::Comp::Hole` and the unknown type
//!   (`gandr_core_term::types::ValueType::Unknown` /
//!   `gandr_core_term::types::CompType::Unknown`) per A2 D5 and the
//!   incremental-pipeline design: holes are axioms that infer `Unknown` and
//!   check against anything; subsumption becomes *consistent subtyping*
//!   ([`discipline::subtype`] records the decision tree); eliminations on
//!   `Unknown` use matched types. This is the totality half of "no parse wall"
//!   — the pipeline lowers unparseable/unsupported regions to holes and the
//!   checker accepts every editor state.
//!
//! Unification rides on the same conversion engine rather than beside it:
//! `gandr-core-unify` is a solver-machine service over the substrate's terms,
//! with metavariables nominated among existing holes so no syntactic former is
//! added. Its answers are certificates a caller re-checks by substituting and
//! asking `gandr_core_nbe::conv`, which is what pins its equational theory to
//! the one this crate's subsumption relation already decides.
//!
//! Incremental re-typing rides on this crate rather than living in it:
//! `gandr-core-incremental` carries the parser-agnostic item seam, the
//! dependency footprints, and the validated-resume checkpoint engine, and
//! drives them through `gandr_core_machine` over the substrate's `syntax`,
//! `types`, and `ctx` vocabulary.
//!
//! No grade *constraints* beyond the inline `1 ⊑ r` force check of §"Core
//! rules" (matched-`U` operations emit none), no unions/intersections, no
//! polymorphism, sessions, sharing, or worlds. The representations are kept
//! extensible (non-exhaustive enums) so later stages can add constructors
//! without breaking downstream matches.

extern crate alloc;

pub mod discipline;
pub mod judgements;
pub mod kernel_bridge;
