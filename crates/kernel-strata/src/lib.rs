//! The minimal certified kernel's **level oracle**: the ADR-78 universe-level
//! algebra `{0, +1, max}` in always-canonical form, with an evidence-returning
//! order oracle (decision: ADR-78; design record:
//! the kernel-boundary design record, slice 1).
//!
//! This crate is the first `gandr-kernel-*` subcrate — trusted by the naming
//! rule of the kernel-boundary design record §2 — and deliberately holds
//! **levels only**: no terms, no types, no universe rule (the rule
//! `U_l : U_m` iff `l < m` is one call into [`Level::lt`], and belongs to the
//! kernel-core crate). It is `#![no_std]` over `core`/`alloc`, the sharpest
//! form of the record's TCB dependency wall.
//!
//! # The algebra and its canonical form
//!
//! A level is a term over `zero`, level variables, successor, and binary
//! `max`, interpreted over the naturals. Every such term is semantically a
//! finite join `max(c, x_1 + o_1, …, x_k + o_k)` of a constant part and
//! per-variable offsets, and that shape — dominated components removed, atoms
//! keyed by variable — is a **sorted canonical form**: two levels denote the
//! same function of their variables exactly when their canonical forms are
//! identical (Bezem–Coquand, TCS 913, 2022, for the underlying word problem;
//! the free fragment implemented here needs only the canonical form itself).
//! [`Level`] **is** that canonical form: the smart constructors maintain it,
//! so a non-canonical level is unrepresentable — the K1 boundary discipline
//! applied to levels.
//!
//! # The order oracle and its evidence
//!
//! `l ≤ m` over all valuations is decided by **domination**: every atom of
//! `l` must be dominated by a same-variable atom of `m`, and `l`'s constant
//! part by `m`'s value at the zero valuation. The oracle returns **checkable
//! evidence** in both directions — a [`LeqWitness`] pairing each left atom
//! with its dominating bound, or a [`LeqRefutation`] carrying a concrete
//! counter-valuation — and [`validate_witness`] / [`validate_refutation`]
//! check either against the two levels. Trust concentrates in the validators;
//! the decision procedure is self-incriminating under mutation (the record's
//! certificate posture at the smallest scale).
//!
//! # The landmark poset and entailment (slice 2)
//!
//! A fixed, declared set of order constraints over level variables — the
//! **landmark poset** of the design record §4 — is admitted by
//! Bezem–Coquand loop-checking (TCS 913, 2022, Corollary 3.5), which is a
//! dichotomy with evidence on both sides: [`LandmarkPoset::admit`] returns
//! either an admitted poset carrying a [`ConsistencyWitness`] (an explicit
//! homomorphism into `ℕ`, checked by evaluating every constraint under it)
//! or a [`LoopWitness`] (a replayable pumping derivation showing no such
//! homomorphism can exist). Entailment under an admitted poset
//! ([`LandmarkPoset::entails_leq_with_evidence`] /
//! [`LandmarkPoset::entails_lt_with_evidence`], Corollary 3.4) returns a
//! forward-derivation [`EntailmentWitness`] or an
//! [`EntailmentCountermodel`], each with its validator. Declared
//! constraints are **variable-only** (the recorded API restriction —
//! [`crate::poset`]'s docs carry the soundness argument); query constants
//! ride a pinned bottom generator internal to the encoding. With no
//! constraints declared, entailment agrees with the free-fragment oracle
//! on every input — the slice-2 acceptance gate, pinned by the property
//! differential.
//!
//! What this crate refuses to hold, by design (ADR-78): level inference or
//! unification, generalization, displacement, constraint hypotheses beyond
//! the declared landmark poset, `imax`, and cumulativity.

#![no_std]

extern crate alloc;

mod entail;
mod horn;
mod level;
mod order;
mod poset;

pub use entail::Entailment;
pub use entail::EntailmentCountermodel;
pub use entail::EntailmentHolds;
pub use entail::EntailmentWitness;
pub use entail::validate_entailment_countermodel;
pub use entail::validate_entailment_witness;
pub use horn::ClauseIndex;
pub use horn::HornOffset;
pub use horn::HornShift;
pub use horn::ModelValue;
pub use level::Level;
pub use level::LevelConstant;
pub use level::LevelError;
pub use level::LevelIsZero;
pub use level::LevelOffset;
pub use level::LevelValue;
pub use level::LevelVar;
pub use level::LevelVarIndex;
pub use order::AtomBound;
pub use order::ConstantBound;
pub use order::EvidenceError;
pub use order::LeqRefutation;
pub use order::LeqWitness;
pub use order::OrderComparison;
pub use order::Strictness;
pub use order::validate_refutation;
pub use order::validate_witness;
pub use poset::AdmissionOutcome;
pub use poset::ConsistencyValue;
pub use poset::ConsistencyWitness;
pub use poset::ConstraintRelation;
pub use poset::Derivation;
pub use poset::DerivationStep;
pub use poset::LandmarkConstraint;
pub use poset::LandmarkPoset;
pub use poset::LoopWitness;
pub use poset::PosetError;
pub use poset::PosetEvidenceError;
pub use poset::validate_consistency;
pub use poset::validate_loop_witness;
