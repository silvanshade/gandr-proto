//! The core CBPV term substrate: the vocabulary every gandr core judgement is
//! stated over.
//!
//! Nothing here decides anything, and nothing here names a crate that does:
//! this crate has no upward dependency, which is what makes it the common
//! substrate. It carries the terms, the types that classify them, the services
//! a judgement spends while walking them, and the shared wrapper and error
//! vocabulary the answers come back in, so every crate that *does* decide is
//! stated over one vocabulary: `gandr-core-checker`'s recursive bidirectional
//! judgement, `gandr-core-machine`'s defunctionalized realization of that same
//! judgement, `gandr-core-nbe`'s conversion engine, `gandr-core-unify`'s
//! solver, and `gandr-core-sequent`'s L machine.
//!
//! Those crates are not independent of each other, and the tier map is where
//! their edges are stated: both the checker and the solver depend on the
//! conversion engine, because a definitional equality is decided in exactly one
//! place. What this crate buys is that none of them has to depend on another
//! merely to name a term.
//!
//! # The modules
//!
//! - [`syntax`] — the values and computations of core call-by-push-value, two
//!   distinct sorts over reference-counted children, plus the flat arena the
//!   total marking traversal interns into;
//! - [`types`] — the value and computation types that classify them, split by
//!   the same polarity, each with the variant census that keeps a downstream
//!   judgement over them provably total;
//! - [`classifier`] — the `(sort, level)` pair a type is formed at, over the
//!   one level algebra `gandr-kernel-strata` owns;
//! - [`static_term`] — the erased static dependent core: the type-level
//!   calculus whose normal forms reify into the two ground type enums;
//! - [`ctx`] — the two-zone typing context `Γ; Σ`;
//! - [`subst`] — the iterative shadowing-aware substitution engine over terms,
//!   and the hole substitution the elaborator plugs;
//! - [`identity`] — the value-into-type substitution the `Walk` motive is
//!   instantiated by; types carry no binders, so it is capture-free structural
//!   recursion delegating its one binder-bearing case to [`subst`];
//! - [`intern`] — content-addressed identity for a type, giving O(1) equality;
//! - [`effect`] — effect-graded returners `F^ε`, the sealed name-ordered effect
//!   row, and the operation signatures, with [`effect::host`] carrying the
//!   representation-independent host seam;
//! - [`grade`] — the preordered semiring over `ℕ ∪ {ω}` the thunk discipline
//!   counts usage in;
//! - [`boundary`] — the semantic wrappers every crate-defined signature in this
//!   tier crosses, which is what keeps anonymous primitives out of the
//!   substrate's public surface;
//! - [`prim`] — the native builtin registry behind [`syntax::Comp::Native`];
//! - [`error`] — the structured typing error every total judgement returns
//!   instead of diverging;
//! - [`outcome`] — the evaluation vocabulary and the one step budget the whole
//!   workspace shares;
//! - [`nominal`] — gandr's sort tags over the shared atom substrate.
//!
//! # Totality
//!
//! Every walk in this crate is total and iterative: substitution, interning,
//! and arena construction run over explicit worklists rather than the host
//! call stack, because a term's depth is caller-controlled and Rust has no
//! guaranteed tail calls. That is the substrate half of the "no parse wall"
//! guarantee — the pipeline lowers every editor state, holes included, and
//! nothing here refuses to represent one.
//!
//! # Extensibility
//!
//! The syntax and type enums are non-exhaustive, so a later stage adds
//! constructors without breaking a downstream match; downstream matches keep a
//! catch-all arm and stay forward-compatible.

extern crate alloc;

pub mod boundary;
pub mod classifier;
pub mod ctx;
pub mod defs;
pub mod effect;
pub mod error;
pub mod grade;
pub mod identity;
pub mod intern;
pub mod nominal;
pub mod outcome;
pub mod prim;
pub mod static_term;
pub mod subst;
pub mod syntax;
pub mod types;
