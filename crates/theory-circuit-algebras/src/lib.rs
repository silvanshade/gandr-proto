//! **Circuit-algebra machinery over the cell-alphabet seam.**
//!
//! The crate ruled at `spec:implementation/circuit-terms.md`,
//! `circuit-terms-question-12`, and minted at that document's
//! `circuit-terms-rung-03`.
//!
//! # The boundary, quoted
//!
//! The crate exists to hold exactly one line, so the line is quoted rather than
//! paraphrased (`spec:implementation/circuit-terms.md`, §"Matching,
//! normalization, and the crate boundary"; owner ruling, 2026-08-02):
//!
//! > A new **`theory-circuit-algebras`** stands beside the existing theory
//! > crates at the narrowed boundary of `circuit-terms-question-12`: it owns
//! > interface bookkeeping, embedding-based matching with its convexity check,
//! > and diagram normal form — not the representation, which is the carrier's
//! > port bijection and was already not new work, and not the engines:
//! > `theory-computads` continues to own boundary-language cell elaboration,
//! > while `theory-coherent-resolutions` owns generic overlaps, completion, and
//! > tracelets over whatever [`CellAlphabet`] it is given.
//!
//! Two facts about that sentence decide everything else here. It is a boundary
//! **over** the [`CellAlphabet`] seam, not an inhabitant **of** it — with
//! `circuit-terms-question-01` ruled grow-in-place, no second alphabet stands
//! beside the landed one, and this crate must never mint one. The engines
//! remain elsewhere: cell storage and alphabet primitives belong to
//! [`gandr_theory_cell_complexes`], boundary-language elaboration belongs to
//! `gandr-theory-computads`, and generic overlap enumeration, completion, and
//! tracelet certificates belong to `gandr-theory-coherent-resolutions`, over
//! whatever alphabet they are handed.
//!
//! # The three faces
//!
//! One module per face named by the ruling, and no fourth module:
//!
//! - [`interface`] — **interface bookkeeping**: the discrete boundary a circuit
//!   rewrite is taken relative to, and the span-level seam data that replaces a
//!   position once a match stops being a path into a tree.
//! - [`matching`] — **embedding-based matching with its convexity check**: a
//!   circuit pattern is neither a spine nor a tree, so matching stops being a
//!   structural recursion and becomes a sub-diagram embedding problem, with
//!   convexity as the one global condition that does not decompose along the
//!   pattern.
//! - [`normal_form`] — **diagram normal form**: when two circuit terms denote
//!   the same diagram, which is what content addressing interns on, and which
//!   is a property of the *representation* rather than of the theory.
//!
//! # What this crate is not
//!
//! The non-goals are as ruled as the goals, and each has a home that already
//! exists:
//!
//! - **Not the representation.** The monogamous fragment's canonical
//!   representation is a port bijection and the metatheory carrier already is
//!   one, so the representation half was never new work. Nothing here mints a
//!   second diagram carrier.
//! - **Not the engines.** Cell storage and alphabet primitives live in
//!   [`gandr_theory_cell_complexes`]; boundary-language elaboration lives in
//!   `gandr-theory-computads`; generic overlaps, completion, and tracelet
//!   certificates live in `gandr-theory-coherent-resolutions`. This crate never
//!   forks them and never grows a private copy of one.
//! - **Not a second [`CellAlphabet`] inhabitant.** The alphabet grows in place
//!   (`circuit-terms-question-01`), which is what keeps the pattern grammar's
//!   compile-visible tripwire pointed at every match site.
//! - **Not the rewriting normal form.** The result of running the rewrite
//!   system to completion is what the certificate algebra already means by
//!   normalization; conflating it with the diagram normal form of
//!   [`normal_form`] is the hazard the ruling names, and the two stay apart.
//!
//! # The matcher seam, and why the dependency direction carries it
//!
//! One consequence rides with the boundary and is recorded rather than designed
//! (`spec:implementation/circuit-terms.md`, §"Matching,
//! normalization, and the crate boundary"): if completion ever consumes
//! embedding-based matching, it does so through a **matcher seam supplied where
//! the engine is instantiated**, never by a downward dependency from
//! `theory-computads`.
//!
//! The dependency direction minted here is what makes that unviolatable rather
//! than merely agreed: this crate depends on [`gandr_theory_cell_complexes`],
//! so the downward **library** edge the consequence forbids would close a
//! dependency cycle Cargo rejects outright — verified by construction, not
//! asserted: a `[dependencies]` entry for this crate in `theory-computads`
//! fails resolution with `error: cyclic package dependency`. The resolver's
//! reach stops there: Cargo *does* admit a cycle through `[dev-dependencies]`,
//! so a test-only downward edge is refused by the ruling rather than by the
//! resolver. Since completion is library code, the consequence's own case is
//! the enforced one. A future consumer that wants completion over embedding
//! matching therefore has exactly one shape available to it — pass the matcher
//! in at the instantiation site — and that shape is the one the ruling asked
//! for.
//!
//! **The seam is not established, and the rung that built the matcher did not
//! establish it.** `circuit-terms-rung-05` filled [`matching`] and left the
//! seam owed: no engine instantiation site exists to supply a matcher at yet,
//! and minting the supply point ahead of a consumer would fix its shape before
//! anything needs it. So [`gandr_theory_cell_complexes`]'s
//! `ConvexityDischarge::ReCheckRequired` still refuses a shift rather than
//! consuming the sweep this crate now has. The obligation stays recorded on
//! that rung's tracker item, and the reversal condition is the first engine
//! instantiation site.
//!
//! # Status: all three homes filled
//!
//! `circuit-terms-rung-03` minted the crate at the ruled boundary with its
//! three module homes, and the two rungs that fill them have both landed.
//! `circuit-terms-rung-05` filled [`matching`] with embedding-based matching
//! and its convexity check, and [`interface`] with the interface bookkeeping
//! that matching consumes — the wire and hyperedge vocabulary, the validated
//! monogamous acyclic diagram view, the seam datum, and the sequent alphabet's
//! spine reading. `circuit-terms-rung-04` filled [`normal_form`] with the
//! boundary-anchored canonical linearization, its checkable relabelling
//! witness, and the diagram-equality decision, and added the crate's shared
//! hyperedge-component walk to [`interface`] rather than taking a second
//! shipped dependency for it.
//!
//! No engine code has moved: cells, overlaps, completion, tracelets, and the
//! rewrite loop are still [`gandr_theory_cell_complexes`]'s, and nothing here
//! rewrites. Read each module's own documentation for what it owns and what it
//! declines.
//!
//! [`CellAlphabet`]: gandr_theory_cell_complexes::alphabet::CellAlphabet

extern crate alloc;

pub mod interface;
pub mod matching;
pub mod normal_form;
