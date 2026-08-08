//! **Embedding-based matching, with its convexity check.**
//!
//! The second of the three faces the crate boundary ruling names
//! (`docs/gandr/spec/implementation/circuit-terms.md`,
//! `circuit-terms-question-12`).
//!
//! # What this module owns
//!
//! Matching a circuit pattern against a circuit target.
//!
//! The engines' existing one-sided matcher and two-sided unifier are written
//! against a pattern language whose consumer side is a linear spine. A circuit
//! pattern is neither a spine nor a tree: it has several roots, it may
//! reconverge, and it may have components with no wire between them. Matching
//! therefore stops being a structural recursion and becomes a **sub-diagram
//! embedding problem** (§"Matching, normalization, and the crate boundary").
//!
//! Two parts of the shape are settled and one is not:
//!
//! - **Settled — the search shape.** Wire- or vertex-driven propagation, with
//!   one nondeterministic seed per connected component of the pattern.
//! - **Settled — the seam datum.** The span-level data a match yields is a pair
//!   of partial bijections rather than a position; the bookkeeping for it lives
//!   in [`crate::interface`].
//! - **Global, and the one part that does not decompose along the pattern —
//!   convexity.** It is a condition on the match as a whole, so it cannot be
//!   checked while the search descends.
//!
//! # The guard this module matches behind
//!
//! Convexity is not left to taste here: `circuit-terms-question-15` rules the
//! guard, and `circuit-terms-spike-07` carries its accounting. Two applications
//! commute when their positions are incomparable, their cell pair has trivial
//! overlap, and each match image is still convex in the other's reduct — with
//! the third conjunct **discharged outright, never run**, on a store certified
//! left-connected over an acyclic target.
//!
//! That discharge is already a datum on the alphabet seam rather than a sweep
//! this module would recompute: `gandr_theory_computads::alphabet` carries it,
//! and the shift-equivalence witness records which warrant it was skipped
//! under. This module reads that warrant; it never mints a second one, and it
//! never treats a documented assumption as a discharge.
//!
//! One fence rides with the wheel axis and constrains what a legal target even
//! is: cutting at the delays does **not** preserve convexity under re-closure,
//! so the **cut-open form is the only legal matching target**, and the
//! condition computed there is the delay's own path extension
//! (`circuit-terms-question-19`; `circuit-terms-spike-08`).
//!
//! # What this module declines
//!
//! - **Firing, rewriting, and budgets.** Whether a matched cell may fire, and
//!   what happens when it does, are `gandr_theory_computads`'s — the alphabet's
//!   firing discipline and the crate's own rewriting and normalization loops.
//! - **Overlap enumeration and completion.** Critical pairs, the multi-sum
//!   overlap families, and the budgeted completion loop stay engine-side over
//!   whatever alphabet the engine is given.
//! - **The downward dependency.** If completion ever consumes this matcher, it
//!   does so through a matcher seam supplied where the engine is instantiated —
//!   never by `theory-computads` depending on this crate. See the crate-root
//!   documentation for why the dependency direction makes that structural.
//!
//! # Status
//!
//! Unbuilt, deliberately. `circuit-terms-rung-03` mints the home;
//! `circuit-terms-rung-05` builds the matcher behind the decided guard, and
//! nothing circuit-shaped matches before it does.
