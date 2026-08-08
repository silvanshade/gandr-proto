//! **Interface bookkeeping.**
//!
//! The first of the three faces the crate boundary ruling names
//! (`docs/gandr/spec/implementation/circuit-terms.md`,
//! `circuit-terms-question-12`).
//!
//! # What this module owns
//!
//! The **interface** a circuit rewrite is taken relative to, and the seam data
//! that stands where a position used to stand.
//!
//! Rewriting at this rung is not rewriting of a diagram but rewriting of a
//! diagram *with an interface*: a rule is a span whose apex boundaries are
//! discrete, and the interface of a cell is the **coproduct of its input and
//! output ports** — confirmed as the applicable setting's own definition rather
//! than adopted as a modelling choice
//! (`docs/gandr/spec/implementation/circuit-terms.md`,
//! `circuit-terms-spike-01`, first claim). The interface is also what earns the
//! result the lane depends on: confluence is decidable for a computable
//! terminating system precisely because the Knuth–Bendix property holds for
//! rewriting *with interfaces*, so the completion engine's "worklist drained"
//! caveat is about budget rather than undecidability (§"The correspondence at
//! gandr's own rung, at theorem grade").
//!
//! Two further facts are settled and belong here rather than to the matcher:
//!
//! - **Boundary complements, not pushout complements.** The Frobenius-free
//!   theory replaces pushout complements with boundary complements, which
//!   additionally require the complement's own cospan to be monogamous and
//!   which are unique whenever they exist — uniqueness holding even for rules
//!   that are not left-linear. The mono-left-leg condition an earlier revision
//!   leaned on is true of the Frobenius rung and is not what matters here
//!   (`circuit-terms-spike-01`, second claim; §"The correspondence at gandr's
//!   own rung, at theorem grade").
//! - **Seam data is a pair of partial bijections.** Once a match stops being a
//!   path into a tree, the span-level datum an overlap carries stops being a
//!   position and becomes a pair of partial bijections (§"Matching,
//!   normalization, and the crate boundary", "settled rather than open").
//!
//! # What this module declines
//!
//! - **The representation.** The carrier's port bijection is the monogamous
//!   fragment's canonical representation and already exists; interface
//!   bookkeeping is bookkeeping *over* it and never a second carrier for it.
//! - **Positions.** The cell store's child-index paths and their order stay
//!   where they are, in `gandr_theory_computads::alphabet` — an interface is
//!   what replaces a position at the circuit rung, not a re-spelling of one.
//! - **Arity declarations.** What a cell's ports *are* is the alphabet's and
//!   the description layer's business; this module reads an interface, it does
//!   not declare one.
//!
//! # Status
//!
//! Unbuilt, deliberately. `circuit-terms-rung-03` mints the home; the interface
//! datum takes its first concrete shape under whichever of
//! `circuit-terms-rung-04` and `circuit-terms-rung-05` first needs it, and
//! neither has run.
