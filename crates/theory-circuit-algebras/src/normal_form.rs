//! **Diagram normal form.**
//!
//! The third of the three faces the crate boundary ruling names
//! (`docs/gandr/spec/implementation/circuit-terms.md`,
//! `circuit-terms-question-12`).
//!
//! # What this module owns
//!
//! One question: **when do two circuit terms denote the same diagram?**
//!
//! It is a property of the *representation*, and it is what content addressing
//! must intern on. For the connected Frobenius case it is the spider collapse;
//! in general it is graph-isomorphism-flavoured, and the corpus's own
//! linear-time acyclicity test is a different and weaker check
//! (§"Matching, normalization, and the crate boundary").
//!
//! The question is smaller than it first looked, at least for the monogamous
//! fragment, and the corpus records why: the isomorphism problem to be decided
//! is over the **port bijection**, not over an arbitrary labelled graph. Two
//! concrete shapes for a canonical linearization are on the table, and they are
//! the two extremes of one family rather than two guesses — orienting the cut
//! equation one way makes normal forms **corolla decompositions** (pick a
//! vertex, recurse into the components its removal leaves), and orienting it
//! the other way makes them **edge decompositions** (pick an edge, recurse into
//! the two components its removal leaves), with a mixed style bracketed between
//! them. The corpus section above carries the citation for both.
//!
//! # Why the spider theorem arrives here rather than through rewriting
//!
//! `circuit-terms-question-20` rules **bialgebra** as the interaction law when
//! one type supplies both fan-in and fan-out, and that ruling re-routes this
//! module's largest neighbour rather than losing it: the spider normal form
//! stays true as a theorem and stops being a rewrite-strategy deliverable. Its
//! decision procedure arrives through **canonicalization at the
//! representation** — this module's face — never by running the contraction
//! rules. Whether the noncommutative, asymmetric statement can be consumed that
//! way is `circuit-terms-question-21`, carried and not yet answered.
//!
//! The standing obligation this module owes is the metatheory's, and it is
//! named rather than restated: the canonical vertex ordering lands behind the
//! `Rigid` device, and `Rigid.canon-sound` at the circuit rung is what owes the
//! soundness of whatever linearization is chosen.
//!
//! # What this module declines
//!
//! - **The rewriting normal form.** The result of running the rewrite system to
//!   completion is what the certificate algebra already means by normalization,
//!   and it is a property of the *theory*. Keeping the two apart is the point
//!   of the ruling; conflating them is the named hazard.
//! - **The representation.** The port bijection is the carrier's, and
//!   canonicalizing it is not the same as owning it.
//! - **Interning and content addressing.** What the storage tier does with a
//!   canonical form is the storage tier's; this module decides sameness and
//!   stops there.
//! - **Any unguarded equality fast path.** The engines' cell equality is
//!   TCB-adjacent, and a normal-form fast path for it lands behind a guard plus
//!   a soundness witness — never as an equality shortcut this module hands
//!   over.
//!
//! # Status
//!
//! Unbuilt, deliberately. `circuit-terms-rung-03` mints the home;
//! `circuit-terms-rung-04` builds the canonical linearization behind the
//! `Rigid` device.
