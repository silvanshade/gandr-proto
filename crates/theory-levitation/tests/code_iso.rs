//! # The U3.0 `CodeIso` certificate suite — verdict
//!
//! An executable, pre-F2 / pre-stage-1 precursor to the univalence ladder's
//! `ua-base`: certificate infrastructure for **value-level isomorphisms between
//! monomorphic descriptions**, built entirely on the landed
//! `gandr-theory-levitation` stage-0 structures with **zero new production
//! machinery** (the U3.0 design note §4.3; the
//! U-ladder context is the identity-and-univalence design's §5–§6; the
//! certificate-composition modes are the invertible/directed pair).
//!
//! ## Real structure vs the U3.0 stand-in
//!
//! Every translator maps real [`gandr_theory_levitation::DescValue`]s, every
//! round-trip is judged by the landed [`gandr_theory_levitation::generic_eq`],
//! every consumer transported ([`gandr_theory_levitation::generic_eq`],
//! [`gandr_theory_levitation::serialize_value`],
//! [`gandr_theory_levitation::CodeInterner`]) is the shipped one, and every
//! description is a real [`gandr_theory_levitation::SignDesc`] (the flagship
//! `Boolean` side is the landed [`gandr_theory_levitation::builtin::bool_desc`]
//! retrofit). The [`harness::CodeIso`] certificate itself is the **stand-in**:
//! it is `ProtypeIso`'s shape (Nasu §3.2.3; ADR-68/69) instantiated at
//! monomorphic codes before F2's judgment layer exists. F2 inherits this exact
//! shape — its `ProtypeIso` replaces [`harness::CodeIso`] and its replay
//! checker replaces [`harness::CodeIso::round_trips`], while the round-trip
//! evidence discipline carries over unchanged.
//!
//! ## Deliberately NOT here (scope fence, design note §4.3)
//!
//! No statement of the path protype `⤳` (waits for F2), no `≅` judgment (waits
//! for F2), no parameterized codes (stage 1), and **no code-edit generator
//! vocabulary** — that is a design item for the ornament conversation, not to
//! be invented ad hoc here. Translators are therefore opaque closures, not
//! inspectable edit data.
//!
//! ## Verdict table
//!
//! | Rung | Verdict | Identity notion | What it establishes | Witnessing tests |
//! | --- | --- | --- | --- | --- |
//! | **U3.0a — `CodeIso` certificates** | PASS | replay-equivalence (ADR-69 D1) | paired translators with `generic_eq`-replayed round trips; the invertible-mode groupoid (identity, inverse, `compose_invertible`) is unital, associative, and inverse-cancelling up to replay; composition declines only on a boundary mismatch (ADR-69 D3 — no acyclicity gate in this mode) | `u30a::*` |
//! | **U3.0b — generic-consumer transport** | PASS | — | `generic_eq` agreement, `serialize_value` naturality up to re-encoding, and interner coherence across every certificate — transport-of-structure with zero new machinery | `u30b::*` |
//! | **U3.0c — the permanent negation guard** | PASS (standing guard) | replay-equivalence vs. code equality | `CodeIso(Boolean, Boolean)` has ≥ 2 replay-inequivalent members (identity, negation) though their boundary codes coincide; realizing `⤳` as decidable code equality would collapse them — kill signal **P-c**, which this test makes fail loudly | `u30c::*` |
//! | **U3.0d — the permanent leaf-shift guard** | PASS (standing guard) | replay-equivalence vs. the leaf-natural stock | over `IntBox` (`Box = Integer`, an infinite leaf) the successor/predecessor `leaf_shift` round-trips yet is replay-distinct from the identity, and `IntBox`'s single constructor makes the structural auto-iso group trivial — so the shift has no leaf-natural preimage; ua-base's O2 completeness quantifier must range over the leaf-natural stock, which this guard keeps load-bearing | `u30d::*` |
//!
//! Net: U3.0a/b PASS on the landed structures; U3.0c and U3.0d are the
//! informative **permanent** guards — checked refutations of, respectively, the
//! false floor (decidable code equality, pinching completeness from below) and
//! the over-wide completeness quantifier (the unrestricted iso stock, pinching
//! it from above), kept in-tree per the iu `BlanketBase` pattern, not
//! design-note footnotes.
//!
//! ## The honest scoping this suite encodes (design note §4.1–§4.2)
//!
//! The monomorphic fragment of `ua-base` needs F2 + a code-edit vocabulary and
//! *not* stage 1; the **certificate half is executable now**, which is this
//! suite. The cardinality witness that keeps `ua-base` honest — a nontrivial
//! auto-iso of `Boolean` with no code-path preimage under a code-equality `⤳` —
//! is `u30c`. The dual witness that keeps `ua-base`'s *completeness* honest —
//! an `Integer`-leaf auto-iso with no leaf-natural preimage, forcing the O2
//! quantifier onto the leaf-natural stock — is `u30d`. "Set-level" describes
//! the *objects* (codes are an h-set under Hedberg); it never describes the
//! *protypes*, whose instance set at `(Boolean, Boolean)` is provably not a
//! singleton here.

mod certificates;
mod fixtures;
mod harness;
mod leaf_shift_guard;
mod negation_guard;
mod transport;
