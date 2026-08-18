//! The term substrate: what the judgements are stated over.
//!
//! [`syntax`] carries the values and computations of core CBPV as two distinct
//! sorts over reference-counted children; [`types`] carries the value and
//! computation types that classify them, split by the same polarity. Around
//! that pair sit the three services every judgement spends: [`ctx`], the
//! two-zone typing context `Γ; Σ`; [`subst`], the iterative capture-avoiding
//! substitution engine; and [`intern`], the content-addressed identity that
//! buys O(1) type equality and, on a hit, O(1) reflexive subtyping.
//!
//! Nothing here decides anything. These modules are the vocabulary the
//! [`judgements`], the [`machine`], and the [`discipline`] layer are written
//! in, which is why they carry no dependency on any of the three.
//!
//! [`judgements`]: crate::judgements
//! [`machine`]: crate::machine
//! [`discipline`]: crate::discipline

pub mod ctx;
pub mod intern;
pub mod subst;
pub mod syntax;
pub mod types;
