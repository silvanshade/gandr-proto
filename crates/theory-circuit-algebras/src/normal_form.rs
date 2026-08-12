//! **Diagram normal form.**
//!
//! The third of the three faces the crate boundary ruling names
//! (`spec:implementation/circuit-terms.md`,
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
//! # The linearization selected, and why the edge orientation
//!
//! **The edge orientation is selected** — the spanning-tree traversal
//! `../metatheory/roadmap`'s `meta-spike-09` went looking for on the `canon`
//! side. Four reasons, in the order they bind.
//!
//! - **Monogamy makes the edge cut choice-free and leaves the vertex cut with a
//!   choice to break.** Removing a wire leaves two sides, and they are told
//!   apart by *direction* rather than by a tie-break: a wire has at most one
//!   producer and at most one consumer, so "recurse into the two components the
//!   edge's removal leaves" is exactly "step to the producer, then to the
//!   consumer". Removing a generator leaves as many pieces as it has ports, and
//!   sequencing those pieces needs an order the corolla shape does not itself
//!   supply.
//! - **The order the vertex cut would have to borrow is the port order, which
//!   the edge cut already consumes.** gandr's ports are ordered
//!   (`circuit-terms-question-02`), so a corolla decomposition would sequence
//!   its pieces by port order and thereby do the edge orientation's work with
//!   an extra vertex choice on top.
//! - **The boundary is already in the edge orientation's currency.** An
//!   interface is a list of *wires*, so the anchoring datum a cospan carries is
//!   an edge datum; the traversal seeds on it directly, while a corolla
//!   decomposition would have to convert a boundary into a vertex choice.
//! - **The recorded caution against the family does not reach the traversal
//!   reading.** The source that exhibits both orientations is a caution as well
//!   as a lead: its rewriting system is non-confluent at exactly the cut
//!   symmetry, with five distinct normal forms for one tree, so *a
//!   decomposition shape owes its own confluence argument*. That is an argument
//!   about producing a normal form by **rewriting toward** it. Here the
//!   linearization is produced by a deterministic **traversal**, so there is no
//!   rewriting relation and no confluence question: uniqueness is by
//!   construction rather than by convergence. Choosing the traversal reading
//!   over the rewriting reading is what makes the caution inapplicable, and it
//!   is why the caution is answered rather than inherited.
//!
//! The mixed style stays bracketed rather than dismissed. What would move the
//! selection toward it is a generator whose ports are *not* totally ordered —
//! an unordered port group, which `circuit-terms-question-02` currently
//! declines — because then the edge orientation would need a tie-break of its
//! own and the vertex cut's extra choice would stop being extra.
//!
//! # The procedure
//!
//! [`canonicalize`] renumbers a [`Wiring`]'s wires and hyperedges in the one
//! order the traversal admits, and returns both the renumbered diagram and the
//! relabelling that produced it.
//!
//! 1. **Anchor on the boundary.** Canonical wire numbers `0, 1, 2, …` go to the
//!    declared input ports in order, then to the declared output ports in
//!    order, skipping a wire already numbered. This is the only step that reads
//!    anything external, and it is what makes the form respect the cospan's
//!    legs: an isomorphism of diagrams-with-interface commutes with the legs,
//!    so it fixes boundary *positions*, so numbering by position is invariant.
//! 2. **Drain.** Walk the numbered wires in canonical order. At each, visit its
//!    producer and then its consumer; visiting a generator takes the next
//!    canonical hyperedge number and numbers its unnumbered ports — sources in
//!    port order, then targets in port order. No step has a choice, so the
//!    numbering this produces is a function of the diagram and its boundary.
//! 3. **Canonicalize the components the boundary never reaches.** A component
//!    with no boundary port has no anchor, so a seed must be chosen inside it.
//!    Every member is tried as the seed, each producing one linearization
//!    because step 2 is choice-free, and the lexicographically least
//!    linearization wins; the components are then committed in the order of
//!    those winning linearizations. The candidate set is the component's own
//!    membership and the winner is its minimum, so both choices are invariants
//!    of the diagram rather than of its presentation. **Both comparisons read
//!    the whole linearization**, which is not a detail: a seed's first record
//!    is its own label and arity in canonical numbers and nothing else, so any
//!    two members sharing those tie on it, and a comparison stopping there
//!    would fall back on source position — a fact about the presentation.
//!    **Ties are possible and they are harmless.** Two seeds, or two
//!    components, tie exactly when they are isomorphic, and the tie is then
//!    broken by source position, so the *relabelling* is not an invariant of
//!    the diagram. The *form* still is: committing either of two isomorphic
//!    components first lays down the same records at the same numbers, because
//!    the records are what tied.
//!
//! **This is not a graph-isomorphism search, and monogamy is why.** The
//! expensive part of graph canonicalization is the branching: at each step a
//! generic algorithm must consider every candidate for the next vertex. Here a
//! generator's image is *forced* by any one of its wires, so a whole component
//! is determined by a single seed. Anchored components cost one traversal each.
//! An unanchored component of `k` generators costs `k` traversals — the seeds —
//! for `O(k²)` in that component and no search at all.
//!
//! # What the argument rests on, named because deleting any of it is unsound
//!
//! **Two** of [`Wiring::assemble`]'s refusal families are premises of the
//! procedure, not background hygiene; the third is deliberately not one, and
//! saying which is which is the point of the list.
//!
//! - **Monogamy** ([`WiringObstruction::FanIn`], [`WiringObstruction::FanOut`])
//!   is what makes step 2 choice-free. It is also what makes [`Wiring`] a
//!   *faithful* index at all: the incidence maps are single-valued, so a
//!   diagram with two producers on one wire could only be recorded by losing
//!   one of them, and a canon computed over a lossy index would identify
//!   diagrams that differ. The refusal is what stops that, and it is witnessed
//!   at `interface::tests::the_wiring_refuses_a_fan_in`, `…_a_fan_out`, and
//!   `…_a_repeated_port_on_one_generator`.
//! - **Boundary honesty**, which is [`Wiring::assemble`]'s three-part condition
//!   and not only its declaration half, and both halves are consumed here.
//!   - *Every open wire is declared* ([`WiringObstruction::UndeclaredInput`],
//!     [`WiringObstruction::UndeclaredOutput`]) is what makes the numbering
//!     *total*. Every wire is a hyperedge port or an open wire; an open wire
//!     that the interface did not declare would be reached by no port and
//!     seeded by no boundary position, so it would never be numbered — and an
//!     isolated wire, which is a port of nothing at all, is numbered *only*
//!     because both legs must declare it. Witnessed at
//!     `interface::tests::the_wiring_refuses_an_undeclared_open_wire` and
//!     `…_an_undeclared_open_input`.
//!   - *A declared port is open*
//!     ([`WiringObstruction::BoundaryInputIsProduced`],
//!     [`WiringObstruction::BoundaryOutputIsConsumed`]) is what makes
//!     [`Linearization::drain`]'s producer-before-consumer order unobservable:
//!     the argument there needs a boundary-seeded wire to be missing at least
//!     one of its two sides, and this is the refusal that supplies it. Without
//!     it a boundary input could have both a producer and a consumer still
//!     unvisited when the cursor reached it, and the two visits would compete
//!     for the next canonical hyperedge number. Named here because that
//!     argument is stated as a fact and this is the fact it rests on. Witnessed
//!     at `interface::tests::the_wiring_refuses_a_produced_boundary_input` and
//!     `…_a_consumed_boundary_output`.
//! - **Acyclicity** ([`WiringObstruction::DirectedCycle`]) is **not** a
//!   premise, and the difference from [`crate::matching`] is worth stating: the
//!   convexity discharge there needs an acyclic target to compose two paths
//!   into a forbidden cycle, while the traversal here terminates on a visited
//!   set and is canonical for the same reason either way. So when the wheel
//!   axis lifts the acyclic hypothesis (`circuit-terms-rung-09`), this face
//!   needs no new argument. That is recorded rather than verified: no cyclic
//!   [`Wiring`] is constructible today, so there is no fixture to check it on.
//!
//! # The `Rigid` device, field by field
//!
//! The corpus places this face behind gandr's `Rigid` device and names
//! `Rigid.canon-sound` at the circuit rung as the standing obligation that owes
//! it. `Rigid` is a canonicalization on a setoid that decides its equivalence
//! (`metatheory/src/Gandr/Rigid.agda`), and the correspondence is exact rather
//! than an analogy:
//!
//! | `Rigid` | here |
//! | ------- | ---- |
//! | the setoid `X` | [`Wiring`], with `≈` the cospan isomorphism over the port bijection |
//! | `Nf R`, the splitting through the fixed points | [`CanonicalDiagram`] |
//! | `nf` | [`canonicalize`]'s [`Canonicalization::form`] |
//! | `emb` | [`CanonicalDiagram::to_wiring`] |
//! | `canon-≈`, the section law | [`Canonicalization::relabelling`], checked by [`Relabelling::verify`] |
//! | `canon-resp`, completeness | equal-up-to-isomorphism inputs reaching one form, differentially tested against an external oracle |
//! | `_≟_` | the derived [`Eq`] on [`CanonicalDiagram`] |
//! | `canon-idem` | canonicalizing a canonical form returns it |
//! | `canon-sound`, **derived** | [`SharedCanon`]: `a ≈ form ≈ b` |
//! | `decide` | [`same_diagram`] |
//!
//! Two things follow that are easy to state wrongly.
//!
//! **Soundness is discharged by evidence rather than asserted.** In `Rigid`,
//! `canon-sound` is *derived* from `canon-≈` — travel out to the
//! representative, across the equality, and back. The Rust supplies exactly
//! that derivation's ingredient: [`same_diagram`] returns, on the affirmative
//! arm, the shared form together with **both** relabellings, so a caller holds
//! the two halves of the composite and can check them. Nothing here hands over
//! an unguarded equality shortcut, which is what the engines' standing warning
//! forbids (`gandr_theory_computads::alphabet`: the `cells_equal` normal-form
//! fast path is TCB-adjacent and needs a guard plus a soundness witness, never
//! documentation).
//!
//! **The metatheory's own `Rigid` instance is a different object and stays
//! owed.** `../metatheory/roadmap`'s `Rigid.canon-sound` at the circuit rung is
//! a normal form on the carrier's **construction terms** over merger and
//! contraction — permutations outermost, ordered tree monomials, unique minimal
//! representative — with a monomial-to-monomial rewriting condition to check
//! first. That is not this procedure, and this procedure does not discharge it.
//! Whether the two canons agree is an owed question rather than an assumption
//! (`gandr-ng9.15-question-17`).
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
//!   stops there. What it owes interning is a **key**, and
//!   [`CanonicalDiagram`]'s derived [`Eq`], [`Ord`] and [`Hash`] are it: the
//!   form is structural content with no arena address, generation stamp or
//!   session state in it, which is the condition the engines' alphabet places
//!   on anything that enters compared identity.
//! - **Any unguarded equality fast path.** The engines' cell equality is
//!   TCB-adjacent, and a normal-form fast path for it lands behind a guard plus
//!   a soundness witness — never as an equality shortcut this module hands
//!   over. [`same_diagram`] is that shape: a verdict that carries its evidence.
//!
//! # Status
//!
//! Built at `circuit-terms-rung-04`: the canonical linearization above,
//! [`canonicalize`] with its checkable relabelling, and [`same_diagram`] as the
//! decision procedure. No interner is minted here and no engine equality is
//! rewired, both deliberately; the engines' `ConvexityDischarge` and
//! `cells_equal` are untouched.
//!
//! [`Wiring`]: crate::interface::Wiring
//! [`Wiring::assemble`]: crate::interface::Wiring::assemble
//! [`WiringObstruction::FanIn`]: crate::interface::WiringObstruction::FanIn
//! [`WiringObstruction::FanOut`]: crate::interface::WiringObstruction::FanOut
//! [`WiringObstruction::UndeclaredInput`]: crate::interface::WiringObstruction::UndeclaredInput
//! [`WiringObstruction::UndeclaredOutput`]: crate::interface::WiringObstruction::UndeclaredOutput
//! [`WiringObstruction::BoundaryInputIsProduced`]: crate::interface::WiringObstruction::BoundaryInputIsProduced
//! [`WiringObstruction::BoundaryOutputIsConsumed`]: crate::interface::WiringObstruction::BoundaryOutputIsConsumed
//! [`WiringObstruction::DirectedCycle`]: crate::interface::WiringObstruction::DirectedCycle

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::interface::ComponentIndex;
use crate::interface::Edge;
use crate::interface::EdgeCount;
use crate::interface::Generator;
use crate::interface::Interface;
use crate::interface::Wire;
use crate::interface::WireCount;
use crate::interface::Wiring;
use crate::interface::WiringObstruction;

/// Which side of an ordered port list a datum sits on.
///
/// One vocabulary for two uses, because they are the same distinction: a
/// generator's sources are the ports it consumes and its targets the ports it
/// produces, and a diagram's boundary inputs are the wires it consumes from
/// outside and its outputs the wires it produces to outside.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Leg
{
    /// The consumed side — a generator's sources, or a boundary's inputs.
    Input,
    /// The produced side — a generator's targets, or a boundary's outputs.
    Output,
}

/// A position in an ordered port list.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PortPosition(pub usize);

/// How many ports an ordered port list holds.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PortCount(pub usize);

/// A diagram in its **canonical numbering** — the normal form, and the object
/// content addressing interns on.
///
/// This is `Rigid`'s splitting `Nf`: the diagrams that are their own canonical
/// representative. A value exists only because [`canonicalize`] produced it, so
/// its numbering is the one the traversal admits, and its derived [`Eq`] is
/// therefore **diagram identity** rather than presentation identity. The
/// distinction is the whole point of the type existing beside [`Wiring`], whose
/// own [`Eq`] is presentation identity: a property of the representation is not
/// a property of the theory, in either direction.
///
/// [`Wiring`]: crate::interface::Wiring
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CanonicalDiagram
{
    /// How many wires the diagram declares.
    wires: WireCount,
    /// The discrete boundary, in canonical wire numbers and declared port
    /// order.
    boundary: Interface,
    /// The hyperedge records, in canonical hyperedge order and canonical wire
    /// numbers.
    generators: Box<[Generator]>,
}

impl CanonicalDiagram
{
    /// How many wires the canonical form declares.
    #[inline]
    #[must_use]
    pub fn wire_count(&self) -> WireCount
    {
        self.wires
    }

    /// How many hyperedges the canonical form holds.
    #[inline]
    #[must_use]
    pub fn edge_count(&self) -> EdgeCount
    {
        EdgeCount(self.generators.len())
    }

    /// The discrete boundary, in canonical wire numbers.
    #[inline]
    #[must_use]
    pub fn boundary(&self) -> &Interface
    {
        &self.boundary
    }

    /// The hyperedge records, in canonical order.
    #[inline]
    #[must_use]
    pub fn generators(&self) -> &[Generator]
    {
        &self.generators
    }

    /// Read the canonical form back as a diagram — `Rigid`'s `emb`.
    ///
    /// # Contract
    /// - ensures: a [`Wiring`] whose generators, boundary and wire count are
    ///   the canonical ones, so canonicalizing it again returns this same form.
    /// - provides: the inclusion back from the splitting, which is what lets a
    ///   caller match against a canonical form or check idempotence.
    /// - fails: [`WiringObstruction`] if the canonical form is not a legitimate
    ///   diagram. This cannot arise for a value this crate produced —
    ///   [`canonicalize`] renumbers a diagram that already passed
    ///   [`Wiring::assemble`], and renumbering preserves every condition
    ///   `assemble` checks — and the fallible signature is kept because the
    ///   reassembly genuinely runs those checks rather than trusting them.
    /// - panics: none.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the reassembled diagram is validated by
    ///   canonicalizing it and comparing with this form, which is `Rigid`'s
    ///   `canon-idem` and is the strongest statement available without a second
    ///   implementation.
    /// - witness: `normal_form::tests::canonicalization_is_idempotent`
    ///
    /// [`Wiring`]: crate::interface::Wiring
    /// [`Wiring::assemble`]: crate::interface::Wiring::assemble
    #[inline]
    pub fn to_wiring(&self) -> Result<Wiring, WiringObstruction>
    {
        let mut generators: Vec<Generator> = Vec::with_capacity(self.generators.len());
        for generator in &self.generators {
            generators.push(generator.clone());
        }
        Wiring::assemble(self.wires, generators, self.boundary.clone())
    }
}

/// A **structure-preserving renumbering** of one diagram onto a canonical form
/// — `Rigid`'s `canon-≈`, as data.
///
/// The witness is inert: it records where each wire and each hyperedge went and
/// claims nothing. Everything that makes it a *diagram isomorphism* is checked
/// by [`Relabelling::verify`], which is deliberate — the validator is where the
/// trust concentrates, and a witness that cannot be forged is worth more than a
/// constructor that promises it was not.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Relabelling
{
    /// Where each wire of the source diagram went.
    wires: BTreeMap<Wire, Wire>,
    /// Where each hyperedge of the source diagram went.
    generators: BTreeMap<Edge, Edge>,
}

impl Relabelling
{
    /// Where `wire` went, if the witness maps it.
    #[inline]
    #[must_use]
    pub fn image_of_wire(
        &self,
        wire: Wire,
    ) -> Option<Wire>
    {
        self.wires.get(&wire).copied()
    }

    /// Where `edge` went, if the witness maps it.
    #[inline]
    #[must_use]
    pub fn image_of_generator(
        &self,
        edge: Edge,
    ) -> Option<Edge>
    {
        self.generators.get(&edge).copied()
    }

    /// How many wires the witness maps.
    #[inline]
    #[must_use]
    pub fn mapped_wires(&self) -> WireCount
    {
        WireCount(self.wires.len())
    }

    /// How many hyperedges the witness maps.
    #[inline]
    #[must_use]
    pub fn mapped_generators(&self) -> EdgeCount
    {
        EdgeCount(self.generators.len())
    }

    /// **Check** that this witness really is an isomorphism from `source` onto
    /// `form`.
    ///
    /// # Contract
    /// - ensures: `Ok(())` exactly when the wire map is a bijection from
    ///   `source`'s wires onto `form`'s, the hyperedge map is a bijection from
    ///   `source`'s hyperedges onto `form`'s, every hyperedge's label and
    ///   ordered ports are carried across pointwise, and both boundary legs
    ///   commute position by position.
    /// - provides: `Rigid`'s section law as something a caller checks rather
    ///   than trusts, which is what keeps [`same_diagram`] from being an
    ///   unguarded equality shortcut.
    /// - fails: [`RelabellingDefect`], naming the wire, hyperedge, leg or
    ///   position that failed; the conditions are decided in the order above,
    ///   so a witness failing several is refused by the earliest.
    /// - panics: none.
    /// - intension: the port and boundary checks compare the *image* of each
    ///   declared port against the canonical record's port at the same
    ///   position, so a witness cannot pass by permuting ports.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    ///
    /// # Adequacy
    /// - hypothesis: L3 pointwise — this is the validator every other L1
    ///   hypothesis in the module rests on, so every refusal arm is separated
    ///   by a hand-built witness failing only it, and the acceptance arm is
    ///   exercised on every fixture the module canonicalizes.
    /// - witness: `normal_form::tests::the_verifier_refuses_a_defective_wire_map`
    /// - witness: `normal_form::tests::the_verifier_refuses_a_defective_generator_map`
    /// - witness: `normal_form::tests::the_verifier_refuses_a_record_that_does_not_correspond`
    /// - witness: `normal_form::tests::the_verifier_refuses_a_boundary_that_does_not_commute`
    /// - witness: `normal_form::tests::canonicalization_is_total_and_its_witness_verifies`
    #[inline]
    pub fn verify(
        &self,
        source: &Wiring,
        form: &CanonicalDiagram,
    ) -> Result<(), RelabellingDefect>
    {
        if source.wire_count() != form.wire_count() {
            return Err(RelabellingDefect::WireCountMismatch {
                source: source.wire_count(),
                form: form.wire_count(),
            });
        }
        if source.edge_count() != form.edge_count() {
            return Err(RelabellingDefect::EdgeCountMismatch {
                source: source.edge_count(),
                form: form.edge_count(),
            });
        }
        let mut wire_preimage: BTreeMap<Wire, Wire> = BTreeMap::new();
        for (wire, image) in &self.wires {
            if image.0 >= form.wire_count().0 {
                return Err(RelabellingDefect::WireImageOutOfRange {
                    wire: *wire,
                    image: *image,
                });
            }
            if let Some(bound) = wire_preimage.insert(*image, *wire) {
                return Err(RelabellingDefect::WireImageReused {
                    wire: *wire,
                    image: *image,
                    bound,
                });
            }
        }
        for index in 0 .. source.wire_count().0 {
            let wire = Wire(index);
            if !self.wires.contains_key(&wire) {
                return Err(RelabellingDefect::WireUnmapped { wire });
            }
        }
        let mut edge_preimage: BTreeMap<Edge, Edge> = BTreeMap::new();
        for (edge, image) in &self.generators {
            if image.0 >= form.edge_count().0 {
                return Err(RelabellingDefect::GeneratorImageOutOfRange {
                    at: *edge,
                    image: *image,
                });
            }
            if let Some(bound) = edge_preimage.insert(*image, *edge) {
                return Err(RelabellingDefect::GeneratorImageReused {
                    at: *edge,
                    image: *image,
                    bound,
                });
            }
        }
        for index in 0 .. source.edge_count().0 {
            let edge = Edge(index);
            if !self.generators.contains_key(&edge) {
                return Err(RelabellingDefect::GeneratorUnmapped { at: edge });
            }
        }
        for index in 0 .. source.edge_count().0 {
            let at = Edge(index);
            let Some(generator) = source.generator(at)
            else {
                // Unreachable: `at` is below the source's hyperedge count.
                continue;
            };
            let Some(image) = self.generators.get(&at).copied()
            else {
                // Unreachable: totality of the hyperedge map was just checked.
                continue;
            };
            let Some(record) = form.generators().get(image.0)
            else {
                // Unreachable: the image's range was just checked against the
                // form's hyperedge count.
                continue;
            };
            if record.label != generator.label {
                return Err(RelabellingDefect::LabelMismatch { at, image });
            }
            self.check_ports(&generator.sources, &record.sources, at, image, Leg::Input)?;
            self.check_ports(&generator.targets, &record.targets, at, image, Leg::Output)?;
        }
        self.check_boundary(
            &source.boundary().inputs,
            &form.boundary().inputs,
            Leg::Input,
        )?;
        self.check_boundary(
            &source.boundary().outputs,
            &form.boundary().outputs,
            Leg::Output,
        )?;
        Ok(())
    }

    /// Check one hyperedge's ports on one leg.
    ///
    /// # Contract
    /// - ensures: `Ok(())` exactly when `declared` and `found` have the same
    ///   length and `found` holds the image of each `declared` port at the same
    ///   position.
    /// - fails: [`RelabellingDefect::ArityMismatch`] on a length difference,
    ///   [`RelabellingDefect::PortMismatch`] at the first position whose image
    ///   disagrees.
    /// - panics: none.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    fn check_ports(
        &self,
        declared: &[Wire],
        found: &[Wire],
        at: Edge,
        image: Edge,
        leg: Leg,
    ) -> Result<(), RelabellingDefect>
    {
        if declared.len() != found.len() {
            return Err(RelabellingDefect::ArityMismatch {
                at,
                image,
                leg,
                declared: PortCount(declared.len()),
                found: PortCount(found.len()),
            });
        }
        for (position, wire) in declared.iter().copied().enumerate() {
            if self.wires.get(&wire).copied() != found.get(position).copied() {
                return Err(RelabellingDefect::PortMismatch {
                    at,
                    image,
                    leg,
                    position: PortPosition(position),
                });
            }
        }
        Ok(())
    }

    /// Check that one boundary leg commutes with the relabelling.
    ///
    /// # Contract
    /// - ensures: `Ok(())` exactly when `declared` and `found` have the same
    ///   length and `found` holds the image of each `declared` port at the same
    ///   position — which is what "the isomorphism commutes with the cospan's
    ///   legs" means at this rung.
    /// - fails: [`RelabellingDefect::BoundaryArityMismatch`] on a length
    ///   difference, [`RelabellingDefect::BoundaryMismatch`] at the first
    ///   position whose image disagrees.
    /// - panics: none.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    fn check_boundary(
        &self,
        declared: &[Wire],
        found: &[Wire],
        leg: Leg,
    ) -> Result<(), RelabellingDefect>
    {
        if declared.len() != found.len() {
            return Err(RelabellingDefect::BoundaryArityMismatch {
                leg,
                declared: PortCount(declared.len()),
                found: PortCount(found.len()),
            });
        }
        for (position, wire) in declared.iter().copied().enumerate() {
            if self.wires.get(&wire).copied() != found.get(position).copied() {
                return Err(RelabellingDefect::BoundaryMismatch {
                    leg,
                    position: PortPosition(position),
                });
            }
        }
        Ok(())
    }
}

/// Why a [`Relabelling`] is not an isomorphism onto the form it was checked
/// against.
///
/// Every variant names the wire, hyperedge, leg or position that failed, so a
/// refusal is usable evidence rather than a verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelabellingDefect
{
    /// The two diagrams do not declare the same number of wires, so no
    /// bijection between their wires exists.
    WireCountMismatch
    {
        /// What the source declares.
        source: WireCount,
        /// What the form declares.
        form: WireCount,
    },
    /// The two diagrams do not hold the same number of hyperedges.
    EdgeCountMismatch
    {
        /// What the source holds.
        source: EdgeCount,
        /// What the form holds.
        form: EdgeCount,
    },
    /// A wire's image is not a wire of the form.
    WireImageOutOfRange
    {
        /// The wire whose image is out of range.
        wire: Wire,
        /// The out-of-range image.
        image: Wire,
    },
    /// Two wires share an image, so the map is not injective.
    WireImageReused
    {
        /// The wire whose image collides.
        wire: Wire,
        /// The shared image.
        image: Wire,
        /// The wire that already reached it.
        bound: Wire,
    },
    /// A wire of the source has no image, so the map is not total.
    WireUnmapped
    {
        /// The unmapped wire.
        wire: Wire,
    },
    /// A hyperedge's image is not a hyperedge of the form.
    GeneratorImageOutOfRange
    {
        /// The hyperedge whose image is out of range.
        at: Edge,
        /// The out-of-range image.
        image: Edge,
    },
    /// Two hyperedges share an image, so the map is not injective.
    GeneratorImageReused
    {
        /// The hyperedge whose image collides.
        at: Edge,
        /// The shared image.
        image: Edge,
        /// The hyperedge that already reached it.
        bound: Edge,
    },
    /// A hyperedge of the source has no image, so the map is not total.
    GeneratorUnmapped
    {
        /// The unmapped hyperedge.
        at: Edge,
    },
    /// A hyperedge's image carries a different label, so the map does not
    /// preserve labels.
    LabelMismatch
    {
        /// The source hyperedge.
        at: Edge,
        /// Its image.
        image: Edge,
    },
    /// A hyperedge's image has a different number of ports on one leg.
    ArityMismatch
    {
        /// The source hyperedge.
        at: Edge,
        /// Its image.
        image: Edge,
        /// Which leg.
        leg: Leg,
        /// How many ports the source declares there.
        declared: PortCount,
        /// How many the image holds.
        found: PortCount,
    },
    /// A port's image is not the image's port at the same position, so the map
    /// does not preserve incidence and port order.
    PortMismatch
    {
        /// The source hyperedge.
        at: Edge,
        /// Its image.
        image: Edge,
        /// Which leg.
        leg: Leg,
        /// Which position.
        position: PortPosition,
    },
    /// One boundary leg has a different number of ports, so the interfaces
    /// differ.
    BoundaryArityMismatch
    {
        /// Which leg.
        leg: Leg,
        /// How many ports the source declares.
        declared: PortCount,
        /// How many the form declares.
        found: PortCount,
    },
    /// A boundary port's image is not the form's port at the same position, so
    /// the map does not commute with the cospan's legs.
    BoundaryMismatch
    {
        /// Which leg.
        leg: Leg,
        /// Which position.
        position: PortPosition,
    },
}

/// What [`canonicalize`] produced: the normal form and the witness that reaches
/// it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Canonicalization
{
    /// The normal form.
    form: CanonicalDiagram,
    /// The renumbering that produced it.
    relabelling: Relabelling,
}

impl Canonicalization
{
    /// The normal form — `Rigid`'s `nf`.
    #[inline]
    #[must_use]
    pub fn form(&self) -> &CanonicalDiagram
    {
        &self.form
    }

    /// The renumbering that produced it — `Rigid`'s `canon-≈`, as a checkable
    /// witness.
    #[inline]
    #[must_use]
    pub fn relabelling(&self) -> &Relabelling
    {
        &self.relabelling
    }

    /// Take the two halves apart.
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (CanonicalDiagram, Relabelling)
    {
        (self.form, self.relabelling)
    }
}

/// Two diagrams that denote one diagram, with the evidence that they do.
///
/// The evidence is deliberately *not* a composed isomorphism between the two
/// inputs. It is the shared form together with both witnesses, which is exactly
/// the shape `Rigid`'s derivation of `canon-sound` needs: travel out from the
/// left to the form, across an equality that is now `refl`, and back down to
/// the right. Composing the two halves is a caller's business, and a caller
/// that wants only the verdict never pays for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SharedCanon
{
    /// The form both diagrams canonicalize to.
    form: CanonicalDiagram,
    /// The left diagram's witness onto it.
    left: Relabelling,
    /// The right diagram's witness onto it.
    right: Relabelling,
}

impl SharedCanon
{
    /// The form both diagrams canonicalize to.
    #[inline]
    #[must_use]
    pub fn form(&self) -> &CanonicalDiagram
    {
        &self.form
    }

    /// The left diagram's witness onto the shared form.
    #[inline]
    #[must_use]
    pub fn left(&self) -> &Relabelling
    {
        &self.left
    }

    /// The right diagram's witness onto the shared form.
    #[inline]
    #[must_use]
    pub fn right(&self) -> &Relabelling
    {
        &self.right
    }
}

/// Where two diagrams' canonical forms first differ.
///
/// A negative verdict is evidence too: it says *what* separates the two
/// diagrams, not only that something does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagramDivergence
{
    /// They declare different numbers of wires.
    WireCount
    {
        /// The left form's count.
        left: WireCount,
        /// The right form's count.
        right: WireCount,
    },
    /// They hold different numbers of hyperedges.
    GeneratorCount
    {
        /// The left form's count.
        left: EdgeCount,
        /// The right form's count.
        right: EdgeCount,
    },
    /// One boundary leg has different arities, so the interfaces differ.
    BoundaryArity
    {
        /// Which leg.
        leg: Leg,
        /// The left form's arity.
        left: PortCount,
        /// The right form's arity.
        right: PortCount,
    },
    /// One boundary leg carries different canonical wires at one position.
    BoundaryPort
    {
        /// Which leg.
        leg: Leg,
        /// Which position.
        position: PortPosition,
        /// The left form's wire there.
        left: Wire,
        /// The right form's wire there.
        right: Wire,
    },
    /// The canonical hyperedge records differ, first at this position.
    Generator
    {
        /// The canonical position that differs.
        at: Edge,
    },
}

/// Whether two diagrams denote the same diagram — `Rigid`'s `decide`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagramEquality
{
    /// They do, and here is the shared form with both witnesses.
    Same(SharedCanon),
    /// They do not, and here is where their canonical forms part.
    Distinct(DiagramDivergence),
}

/// **Canonicalize** a diagram: renumber it in the one order the traversal
/// admits.
///
/// # Contract
/// - requires: nothing beyond `diagram` being a [`Wiring`], whose constructor
///   has already established monogamy, boundary honesty and acyclicity. The
///   first two are premises of this procedure; acyclicity is not (see the
///   module documentation).
/// - ensures: every wire and every hyperedge of `diagram` is numbered exactly
///   once; the returned form's boundary is the declared boundary in canonical
///   numbers; and the form depends only on the diagram up to cospan isomorphism
///   — two diagrams related by a renumbering that commutes with their boundary
///   legs return equal forms.
/// - provides: the diagram normal form the crate boundary ruling names, as a
///   value whose [`Eq`], [`Ord`] and [`Hash`] are diagram identity and are
///   therefore usable as an interning key, together with the witness that makes
///   soundness derivable rather than asserted.
/// - panics: none.
/// - intension: the boundary is numbered first — the input leg, then the output
///   leg — then a wire cursor drains in canonical order, numbering each visited
///   hyperedge's sources before its targets, then components with no boundary
///   port are committed in the order of their least linearization. Components
///   with a boundary port cost one traversal; a component of `k` hyperedges
///   with none costs `k`.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the returned witness is validated against the
///   diagram and the form it relates, at every fixture, rather than compared
///   with a predicted numbering; the L2 face is a differential against an
///   independent cospan-isomorphism oracle that enumerates hyperedge bijections
///   and derives the wire map from ports and boundary positions; the L3 residue
///   is the ordering promises — boundary before interior, sources before
///   targets, producer before consumer, and the least-linearization tie-break —
///   each separated by a fixture where reversing it changes the form, and the
///   tie-break additionally separated from every weaker comparison by a
///   component whose seeds agree on their first record and part later; the one
///   place presentation does break a choice — two isomorphic anchorless
///   components — is asserted to leave the form invariant while the witness
///   varies; and the interning key's own law — equal keys hash alike — is
///   asserted rather than inherited from the derive.
/// - witness: `normal_form::tests::canonicalization_is_total_and_its_witness_verifies`
/// - witness: `normal_form::tests::a_presentation_permutation_has_one_canonical_form`
/// - witness: `normal_form::tests::canonicalization_is_idempotent`
/// - witness: `normal_form::tests::the_canon_agrees_with_the_cospan_isomorphism_oracle`
/// - witness: `normal_form::tests::a_component_with_no_boundary_port_is_seeded_by_minimizing`
/// - witness: `normal_form::tests::components_with_no_boundary_port_are_committed_in_canonical_order`
/// - witness: `normal_form::tests::the_least_linearization_is_compared_past_its_first_record`
/// - witness: `normal_form::tests::two_isomorphic_anchorless_components_still_have_one_form`
/// - witness: `normal_form::tests::the_boundary_is_numbered_before_the_interior`
/// - witness: `normal_form::tests::an_anchored_component_is_ordered_by_the_boundary_and_not_by_its_labels`
/// - witness: `normal_form::tests::a_visited_hyperedge_numbers_its_sources_before_its_targets`
/// - witness: `normal_form::tests::an_isolated_wire_is_numbered_from_the_boundary_alone`
/// - witness: `normal_form::tests::interning_consumes_the_canon`
/// - witness: `normal_form::tests::equal_canonical_forms_hash_alike`
///
/// [`Wiring`]: crate::interface::Wiring
#[inline]
#[must_use]
pub fn canonicalize(diagram: &Wiring) -> Canonicalization
{
    let mut state = Linearization::default();
    for wire in diagram.boundary().inputs.iter().copied() {
        state.assign_wire(wire);
    }
    for wire in diagram.boundary().outputs.iter().copied() {
        state.assign_wire(wire);
    }
    state.drain(diagram);
    if state.edge_image.len() < diagram.edge_count().0 {
        // Some component holds no boundary port, so it has no anchor and its
        // seed must be chosen. The component walk runs only here, because an
        // anchored diagram never needs it.
        //
        // The guard is a COST guard and nothing else, so weakening it to `<=` is
        // an equivalent mutant: with every hyperedge already visited, every
        // component's first member is in `edge_image`, so every component is
        // skipped and the residue is empty. Measured, not assumed — the mutant
        // was applied and survived, and it is recorded here rather than left for
        // a campaign to rediscover.
        let components = diagram.components();
        let mut residue: Vec<(Vec<Generator>, Edge)> = Vec::new();
        for index in 0 .. components.count().0 {
            let Some(members) = components.members(ComponentIndex(index))
            else {
                // Unreachable: `index` is below the component count.
                continue;
            };
            let Some(first) = members.first().copied()
            else {
                // Unreachable: a component always holds at least its seed.
                continue;
            };
            if state.edge_image.contains_key(&first) {
                continue;
            }
            let mut best: Option<(Vec<Generator>, Edge)> = None;
            for seed in members.iter().copied() {
                let mut trial = Linearization::default();
                trial.visit(diagram, seed);
                trial.drain(diagram);
                let candidate = trial.records;
                let keep = match best {
                    | Some((ref records, _)) => candidate < *records,
                    | None => true,
                };
                if keep {
                    best = Some((candidate, seed));
                }
            }
            if let Some(entry) = best {
                residue.push(entry);
            }
        }
        residue.sort();
        for (_, seed) in residue {
            state.visit(diagram, seed);
            state.drain(diagram);
        }
    }
    let boundary = Interface::new(
        state.images(&diagram.boundary().inputs),
        state.images(&diagram.boundary().outputs),
    );
    let Linearization {
        wire_image,
        wire_order,
        edge_image,
        records,
        cursor: _,
    } = state;
    Canonicalization {
        form: CanonicalDiagram {
            wires: WireCount(wire_order.len()),
            boundary,
            generators: records.into_boxed_slice(),
        },
        relabelling: Relabelling {
            wires: wire_image,
            generators: edge_image,
        },
    }
}

/// Decide whether two diagrams denote the **same diagram**.
///
/// # Contract
/// - ensures: [`DiagramEquality::Same`] exactly when the two are related by a
///   renumbering of wires and hyperedges that preserves labels, arities,
///   incidence and port order and commutes with both boundary legs — the cospan
///   isomorphism over the port bijection; otherwise
///   [`DiagramEquality::Distinct`] naming where the canonical forms part.
/// - provides: `Rigid`'s `decide` at the circuit rung, with evidence on the
///   affirmative arm and a located difference on the negative one, so neither
///   verdict is a bare bit.
/// - panics: none.
/// - intension: both diagrams are canonicalized and their forms compared; the
///   comparison walks wire count, hyperedge count, both boundary legs, then the
///   hyperedge records, so the reported difference is the earliest in that
///   order.
///
/// # Adequacy
/// - hypothesis: L2 agreement — the verdict is differentially checked against
///   an independent cospan-isomorphism oracle over a table carrying both
///   verdicts, with a non-vacuity assertion so the agreement is not agreement
///   about nothing; the L3 residue is the negative arms, each separated by a
///   pair differing in exactly the datum the variant names, and the four
///   identifications a weaker canon would wrongly admit — a permuted boundary,
///   a permuted port list on one hyperedge, a label worn at a second sort, and
///   one generator multiset wired two ways.
/// - witness: `normal_form::tests::the_canon_agrees_with_the_cospan_isomorphism_oracle`
/// - witness: `normal_form::tests::same_diagram_locates_a_count_difference`
/// - witness: `normal_form::tests::same_diagram_locates_a_boundary_difference`
/// - witness: `normal_form::tests::same_diagram_locates_a_hyperedge_difference`
/// - witness: `normal_form::tests::the_canon_separates_a_permuted_boundary`
/// - witness: `normal_form::tests::the_canon_separates_a_permuted_port_list`
/// - witness: `normal_form::tests::the_canon_separates_a_label_worn_at_two_sorts`
/// - witness: `normal_form::tests::the_canon_separates_one_generator_multiset_wired_two_ways`
///
/// [`Wiring`]: crate::interface::Wiring
#[inline]
#[must_use]
pub fn same_diagram(
    left: &Wiring,
    right: &Wiring,
) -> DiagramEquality
{
    let left_canon = canonicalize(left);
    let right_canon = canonicalize(right);
    if let Some(divergence) = divergence_of(left_canon.form(), right_canon.form()) {
        return DiagramEquality::Distinct(divergence);
    }
    let (form, left) = left_canon.into_parts();
    let (_, right) = right_canon.into_parts();
    DiagramEquality::Same(SharedCanon { form, left, right })
}

/// Where two canonical forms first differ, if they do.
///
/// # Contract
/// - ensures: `None` exactly when the two forms are structurally equal;
///   otherwise the earliest difference in the order wire count, hyperedge
///   count, input leg, output leg, hyperedge records.
/// - panics: none.
fn divergence_of(
    left: &CanonicalDiagram,
    right: &CanonicalDiagram,
) -> Option<DiagramDivergence>
{
    if left.wire_count() != right.wire_count() {
        return Some(DiagramDivergence::WireCount {
            left: left.wire_count(),
            right: right.wire_count(),
        });
    }
    if left.edge_count() != right.edge_count() {
        return Some(DiagramDivergence::GeneratorCount {
            left: left.edge_count(),
            right: right.edge_count(),
        });
    }
    let legs = [
        (
            Leg::Input,
            &left.boundary().inputs,
            &right.boundary().inputs,
        ),
        (
            Leg::Output,
            &left.boundary().outputs,
            &right.boundary().outputs,
        ),
    ];
    for (leg, mine, theirs) in legs {
        if mine.len() != theirs.len() {
            return Some(DiagramDivergence::BoundaryArity {
                leg,
                left: PortCount(mine.len()),
                right: PortCount(theirs.len()),
            });
        }
        for (position, wire) in mine.iter().copied().enumerate() {
            let Some(other) = theirs.get(position).copied()
            else {
                // Unreachable: the two legs have the same length.
                continue;
            };
            if wire != other {
                return Some(DiagramDivergence::BoundaryPort {
                    leg,
                    position: PortPosition(position),
                    left: wire,
                    right: other,
                });
            }
        }
    }
    for (position, record) in left.generators().iter().enumerate() {
        let Some(other) = right.generators().get(position)
        else {
            // Unreachable: the two forms hold the same number of hyperedges.
            continue;
        };
        if record != other {
            return Some(DiagramDivergence::Generator { at: Edge(position) });
        }
    }
    None
}

/// The canonical linearization as it is being built.
///
/// The invariant that makes this the *canonical* one: nothing in it ever
/// chooses — every number it hands out is forced by the order the boundary and
/// the port lists already fix. The one choice the procedure makes is which seed
/// to hand [`Linearization::visit`] for a component with no boundary port, and
/// that is made outside, by minimizing over the whole component.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Linearization
{
    /// Where each numbered wire went.
    wire_image: BTreeMap<Wire, Wire>,
    /// The numbered wires in canonical order — the inverse of `wire_image`, and
    /// the exploration queue `cursor` walks.
    wire_order: Vec<Wire>,
    /// Where each visited hyperedge went.
    edge_image: BTreeMap<Edge, Edge>,
    /// The canonical hyperedge records, in canonical order.
    records: Vec<Generator>,
    /// How much of `wire_order` has been explored.
    cursor: usize,
}

impl Linearization
{
    /// Number `wire`, unless it is already numbered.
    ///
    /// # Contract
    /// - ensures: after the call `wire` has a canonical number; a wire already
    ///   numbered keeps the number it had, which is what makes the boundary's
    ///   two legs able to name one wire.
    /// - provides: the only place a canonical wire number is minted, so the
    ///   numbering is dense and in discovery order by construction.
    /// - panics: none.
    ///
    /// `wire_order.len()` and `wire_image.len()` are equal at every point — one
    /// push per insert — so minting from either is the same program. The same
    /// holds of `records.len()` and `edge_image.len()` in
    /// [`Linearization::visit`]. Both substitutions were applied as mutants and
    /// survived, and both are equivalent for that reason rather than untested.
    fn assign_wire(
        &mut self,
        wire: Wire,
    )
    {
        if self.wire_image.contains_key(&wire) {
            return;
        }
        self.wire_image.insert(wire, Wire(self.wire_order.len()));
        self.wire_order.push(wire);
    }

    /// Record `edge` and number its ports, unless it is already recorded.
    ///
    /// # Contract
    /// - requires: `edge` is a hyperedge position of `diagram`.
    /// - ensures: after the call `edge` has a canonical number and every wire
    ///   it names is numbered; the record it contributes carries the canonical
    ///   numbers of its ports in declared order.
    /// - provides: the step that makes the traversal port-order-driven, which
    ///   is the whole content of the edge orientation being selected.
    /// - panics: none.
    /// - intension: ports are numbered sources-then-targets in declared order,
    ///   and the record is pushed after they are numbered, so a record never
    ///   holds a number that was not already minted.
    fn visit(
        &mut self,
        diagram: &Wiring,
        edge: Edge,
    )
    {
        if self.edge_image.contains_key(&edge) {
            return;
        }
        let Some(generator) = diagram.generator(edge)
        else {
            // Unreachable: every caller passes either a hyperedge position read
            // out of the diagram's own incidence maps or a member of one of its
            // components.
            return;
        };
        for wire in generator.sources.iter().copied() {
            self.assign_wire(wire);
        }
        for wire in generator.targets.iter().copied() {
            self.assign_wire(wire);
        }
        self.edge_image.insert(edge, Edge(self.records.len()));
        let record = Generator::new(
            generator.label.clone(),
            self.images(&generator.sources),
            self.images(&generator.targets),
        );
        self.records.push(record);
    }

    /// Visit everything the numbered wires reach.
    ///
    /// # Contract
    /// - ensures: on return every numbered wire has had its producer and its
    ///   consumer visited, so no numbered wire leads anywhere unvisited.
    /// - provides: the closure step, which is why one seed determines a whole
    ///   component.
    /// - panics: none.
    /// - intension: an explicit cursor over the growing canonical wire order,
    ///   so the walk never recurses on diagram size. The loop terminates
    ///   because a wire enters `wire_order` once and the cursor only advances.
    ///
    /// **The producer-before-consumer order below is not a promise, because it
    /// is not observable.** When the cursor reaches a wire, at most one of its
    /// two sides can still be unvisited, so swapping the two calls is an
    /// equivalent mutant. Either the wire was numbered as a boundary port — and
    /// then it has no producer, or no consumer, or neither, which is
    /// [`Wiring::assemble`]'s refusal of a produced input and a consumed output
    /// rather than anything this module establishes — or it was numbered inside
    /// [`Linearization::visit`] of a hyperedge naming it, and that hyperedge is
    /// marked visited before the drain resumes. Recorded here rather than left
    /// for a mutation campaign to rediscover, and the premise is named in the
    /// module documentation because the equivalence fails without it.
    ///
    /// [`Wiring::assemble`]: crate::interface::Wiring::assemble
    fn drain(
        &mut self,
        diagram: &Wiring,
    )
    {
        while let Some(wire) = self.wire_order.get(self.cursor).copied() {
            self.cursor = self.cursor.saturating_add(1);
            if let Some(edge) = diagram.producer_of(wire) {
                self.visit(diagram, edge);
            }
            if let Some(edge) = diagram.consumer_of(wire) {
                self.visit(diagram, edge);
            }
        }
    }

    /// The canonical numbers of an ordered port list.
    ///
    /// # Contract
    /// - requires: every wire of `ports` is already numbered.
    /// - ensures: the canonical numbers in the same order.
    /// - panics: none.
    /// - intension: an unnumbered wire is dropped rather than guessed at, so
    ///   the result is short and [`Relabelling::verify`] reports it as an arity
    ///   difference. The requirement holds at both call sites, so this cannot
    ///   arise; dropping is chosen over substituting a number because a wrong
    ///   number would be harder for the validator to attribute.
    fn images(
        &self,
        ports: &[Wire],
    ) -> Box<[Wire]>
    {
        let mut images: Vec<Wire> = Vec::with_capacity(ports.len());
        for wire in ports {
            if let Some(image) = self.wire_image.get(wire).copied() {
                images.push(image);
            }
        }
        images.into_boxed_slice()
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::interface::GeneratorLabel;
    use crate::interface::GeneratorSort;

    /// A value-sorted generator label.
    fn value<Name>(name: Name) -> GeneratorLabel
    where
        Name: Into<Box<str>>,
    {
        GeneratorLabel::new(name, GeneratorSort::Value)
    }

    /// `f: (0) -> (1)` then `g: (1) -> (2)`, open at both ends.
    fn spine() -> Wiring
    {
        Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], [Wire(2)]),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        )
        .expect("the two-step diagram is monogamous, boundary-honest and acyclic")
    }

    /// The same diagram as [`spine`], presented with its hyperedges in the
    /// other order and its wires renumbered — the presentation permutation
    /// the canon exists to quotient.
    fn spine_permuted() -> Wiring
    {
        Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("g"), [Wire(0)], [Wire(1)]),
                Generator::new(value("f"), [Wire(2)], [Wire(0)]),
            ],
            Interface::new([Wire(2)], [Wire(1)]),
        )
        .expect("a renumbering of the two-step diagram is still one")
    }

    /// [`spine`] with its labels in **descending** order, so the hyperedge the
    /// boundary reaches first is not the one a label-minimizing seed would
    /// pick.
    ///
    /// This is the fixture that separates anchoring from minimizing. Every
    /// other fixture here happens to agree on the two, which made a mutant
    /// that never drains after seeding the boundary — and so routes every
    /// component through the minimizing path — survive an earlier suite.
    fn descending_spine() -> Wiring
    {
        Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("z"), [Wire(0)], [Wire(1)]),
                Generator::new(value("a"), [Wire(1)], [Wire(2)]),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        )
        .expect("labels have no bearing on whether a diagram is well formed")
    }

    /// A diagram where the traversal reaches a hyperedge with **both** an
    /// unnumbered source and an unnumbered target, which is the only shape on
    /// which the sources-before-targets promise is observable.
    fn branching() -> Wiring
    {
        Wiring::assemble(
            WireCount(5),
            alloc::vec![
                Generator::new(value("p"), [Wire(0)], [Wire(1)]),
                Generator::new(value("f"), [Wire(2), Wire(1)], [Wire(3)]),
                Generator::new(value("q"), [Wire(3)], [Wire(4)]),
            ],
            Interface::new([Wire(2), Wire(0)], [Wire(4)]),
        )
        .expect("the branching fixture is monogamous, boundary-honest and acyclic")
    }

    /// A **closed** component: `f: () -> (0)` into `g: (0) -> ()`, with an
    /// empty interface. Neither end of the wire is open, so the boundary
    /// anchors nothing and the seed must be chosen inside the component.
    fn closed() -> Wiring
    {
        Wiring::assemble(
            WireCount(1),
            alloc::vec![
                Generator::new(value("f"), Vec::new(), [Wire(0)]),
                Generator::new(value("g"), [Wire(0)], Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("a wire with both a producer and a consumer needs no boundary declaration")
    }

    /// [`closed`] with its two hyperedges listed the other way round.
    fn closed_permuted() -> Wiring
    {
        Wiring::assemble(
            WireCount(1),
            alloc::vec![
                Generator::new(value("g"), [Wire(0)], Vec::new()),
                Generator::new(value("f"), Vec::new(), [Wire(0)]),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("the same closed diagram, presented the other way round")
    }

    /// Two closed components, `f -> g` and `h -> k`, with an empty interface.
    fn two_closed() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            alloc::vec![
                Generator::new(value("f"), Vec::new(), [Wire(0)]),
                Generator::new(value("g"), [Wire(0)], Vec::new()),
                Generator::new(value("h"), Vec::new(), [Wire(1)]),
                Generator::new(value("k"), [Wire(1)], Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("two closed components are a legitimate diagram")
    }

    /// [`two_closed`] with the `h`–`k` component presented first.
    fn two_closed_permuted() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            alloc::vec![
                Generator::new(value("h"), Vec::new(), [Wire(0)]),
                Generator::new(value("k"), [Wire(0)], Vec::new()),
                Generator::new(value("f"), Vec::new(), [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("the same two closed components, presented the other way round")
    }

    /// One wire and no hyperedge — the identity diagram, whose only wire is a
    /// port of nothing and is numbered from the boundary alone.
    fn bare_wire() -> Wiring
    {
        Wiring::assemble(
            WireCount(1),
            Vec::new(),
            Interface::new([Wire(0)], [Wire(0)]),
        )
        .expect("an isolated wire must be declared on both legs, and is")
    }

    /// No wire and no hyperedge.
    fn empty() -> Wiring
    {
        Wiring::assemble(
            WireCount(0),
            Vec::new(),
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("the empty diagram is a diagram")
    }

    /// Two port-free hyperedges with one label — the only shape that can be a
    /// component holding no wire at all.
    fn port_free_pair() -> Wiring
    {
        Wiring::assemble(
            WireCount(0),
            alloc::vec![
                Generator::new(value("a"), Vec::new(), Vec::new()),
                Generator::new(value("a"), Vec::new(), Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("port-free hyperedges are a legitimate diagram")
    }

    /// `s` fanning out to two wires that `m` rejoins — the reconvergence axis.
    fn reconvergent() -> Wiring
    {
        Wiring::assemble(
            WireCount(4),
            alloc::vec![
                Generator::new(value("s"), [Wire(0)], [Wire(1), Wire(2)]),
                Generator::new(value("m"), [Wire(1), Wire(2)], [Wire(3)]),
            ],
            Interface::new([Wire(0)], [Wire(3)]),
        )
        .expect("many-out and reconvergence are inside the fragment")
    }

    /// [`reconvergent`] with `m`'s two sources exchanged — the same wires, one
    /// port order apart, and therefore a different diagram.
    fn reconvergent_swapped() -> Wiring
    {
        Wiring::assemble(
            WireCount(4),
            alloc::vec![
                Generator::new(value("s"), [Wire(0)], [Wire(1), Wire(2)]),
                Generator::new(value("m"), [Wire(2), Wire(1)], [Wire(3)]),
            ],
            Interface::new([Wire(0)], [Wire(3)]),
        )
        .expect("exchanging two sources leaves the fragment untouched")
    }

    /// A three-step chain `f`, `g`, `h`.
    fn chain() -> Wiring
    {
        Wiring::assemble(
            WireCount(4),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], [Wire(2)]),
                Generator::new(value("h"), [Wire(2)], [Wire(3)]),
            ],
            Interface::new([Wire(0)], [Wire(3)]),
        )
        .expect("a three-step chain is monogamous and acyclic")
    }

    /// [`chain`] renumbered and relisted — the same diagram.
    fn chain_relabelled() -> Wiring
    {
        Wiring::assemble(
            WireCount(4),
            alloc::vec![
                Generator::new(value("h"), [Wire(0)], [Wire(2)]),
                Generator::new(value("f"), [Wire(3)], [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], [Wire(0)]),
            ],
            Interface::new([Wire(3)], [Wire(2)]),
        )
        .expect("a renumbering of the chain is still the chain")
    }

    /// The same three hyperedges as [`chain`], wired in a different order —
    /// equal counts, equal labels, different diagram.
    fn chain_rewired() -> Wiring
    {
        Wiring::assemble(
            WireCount(4),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("h"), [Wire(1)], [Wire(2)]),
                Generator::new(value("g"), [Wire(2)], [Wire(3)]),
            ],
            Interface::new([Wire(0)], [Wire(3)]),
        )
        .expect("the same labels in a different order are still a chain")
    }

    /// [`spine`] with a closed component beside it.
    fn spine_with_closed() -> Wiring
    {
        Wiring::assemble(
            WireCount(4),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], [Wire(2)]),
                Generator::new(value("p"), Vec::new(), [Wire(3)]),
                Generator::new(value("q"), [Wire(3)], Vec::new()),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        )
        .expect("an anchored component and a closed one side by side")
    }

    /// A closed component whose two interior hyperedges wear **one label at one
    /// arity**, so two of its seeds produce the *same first record* and part
    /// only later: `p -> a -> a -> q`.
    ///
    /// This is the fixture that separates "least linearization" from "least
    /// first record". A seed's first record is its own label and arity in
    /// canonical numbers and nothing else, so any two same-label same-arity
    /// members tie on it; a minimization reading only that record keeps
    /// whichever tied member it met first, which is a *position* and therefore
    /// a fact about the presentation.
    fn nested_repeated_label() -> Wiring
    {
        Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("p"), Vec::new(), [Wire(0)]),
                Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                Generator::new(value("a"), [Wire(1)], [Wire(2)]),
                Generator::new(value("q"), [Wire(2)], Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("a closed four-hyperedge chain is monogamous, boundary-honest and acyclic")
    }

    /// [`nested_repeated_label`] listed back to front and renumbered — the same
    /// diagram, and the presentation on which a first-record-only minimization
    /// picks the other `a`.
    fn nested_repeated_label_permuted() -> Wiring
    {
        Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("q"), [Wire(0)], Vec::new()),
                Generator::new(value("a"), [Wire(1)], [Wire(0)]),
                Generator::new(value("a"), [Wire(2)], [Wire(1)]),
                Generator::new(value("p"), Vec::new(), [Wire(2)]),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("a renumbering of the closed chain is still the closed chain")
    }

    /// Two closed components whose winning linearizations **agree on their
    /// first record** and part at the second: `a -> b` beside `a -> c`.
    ///
    /// The residue is ordered by the whole winning linearization, and this is
    /// the shape that separates that from ordering by the first record alone —
    /// which would leave the two components to fall back on presentation order.
    fn twin_headed_pair() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            alloc::vec![
                Generator::new(value("a"), Vec::new(), [Wire(0)]),
                Generator::new(value("b"), [Wire(0)], Vec::new()),
                Generator::new(value("a"), Vec::new(), [Wire(1)]),
                Generator::new(value("c"), [Wire(1)], Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("two closed components sharing a head label are a legitimate diagram")
    }

    /// [`twin_headed_pair`] with the `a`–`c` component presented first.
    fn twin_headed_pair_permuted() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            alloc::vec![
                Generator::new(value("a"), Vec::new(), [Wire(0)]),
                Generator::new(value("c"), [Wire(0)], Vec::new()),
                Generator::new(value("a"), Vec::new(), [Wire(1)]),
                Generator::new(value("b"), [Wire(1)], Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("the same two components, presented the other way round")
    }

    /// Two **isomorphic** closed components, so their winning linearizations
    /// tie outright and the tie falls to source position.
    ///
    /// The relabelling is then not an invariant of the diagram — which member
    /// seeds which component depends on the presentation — but the *form* still
    /// has to be, because committing either copy first produces the same
    /// records at the same numbers. That is the one place the procedure's
    /// choice is broken by presentation, and it is why it is broken there
    /// harmlessly.
    fn twin_closed() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            alloc::vec![
                Generator::new(value("a"), Vec::new(), [Wire(0)]),
                Generator::new(value("b"), [Wire(0)], Vec::new()),
                Generator::new(value("a"), Vec::new(), [Wire(1)]),
                Generator::new(value("b"), [Wire(1)], Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("two copies of one closed component are a legitimate diagram")
    }

    /// [`twin_closed`] with its two components **interleaved** in the listing,
    /// so neither component is a contiguous block of hyperedge positions.
    fn twin_closed_interleaved() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            alloc::vec![
                Generator::new(value("a"), Vec::new(), [Wire(0)]),
                Generator::new(value("a"), Vec::new(), [Wire(1)]),
                Generator::new(value("b"), [Wire(1)], Vec::new()),
                Generator::new(value("b"), [Wire(0)], Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("interleaving two components changes the presentation and not the diagram")
    }

    /// `m: (0, 1) -> (2)` with its input leg in declared order.
    fn two_input() -> Wiring
    {
        Wiring::assemble(
            WireCount(3),
            alloc::vec![Generator::new(value("m"), [Wire(0), Wire(1)], [Wire(2)])],
            Interface::new([Wire(0), Wire(1)], [Wire(2)]),
        )
        .expect("a two-input hyperedge with both inputs open")
    }

    /// [`two_input`] with its input leg exchanged — the same apex, a different
    /// cospan.
    fn two_input_boundary_swapped() -> Wiring
    {
        Wiring::assemble(
            WireCount(3),
            alloc::vec![Generator::new(value("m"), [Wire(0), Wire(1)], [Wire(2)])],
            Interface::new([Wire(1), Wire(0)], [Wire(2)]),
        )
        .expect("declaring the same two open wires in the other order")
    }

    /// `f: (0) -> (1)` worn as a value-side constructor.
    fn one_step_value() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(1)])],
            Interface::new([Wire(0)], [Wire(1)]),
        )
        .expect("one step is a diagram")
    }

    /// The same name and arity worn as an operation frame instead.
    fn one_step_operation() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            alloc::vec![Generator::new(
                GeneratorLabel::new("f", GeneratorSort::Operation),
                [Wire(0)],
                [Wire(1)]
            )],
            Interface::new([Wire(0)], [Wire(1)]),
        )
        .expect("the sort is part of the label, not part of the wiring")
    }

    /// Every fixture, so a battery runs over all of them.
    fn every_fixture() -> Vec<Wiring>
    {
        alloc::vec![
            spine(),
            spine_permuted(),
            branching(),
            closed(),
            closed_permuted(),
            two_closed(),
            two_closed_permuted(),
            bare_wire(),
            empty(),
            port_free_pair(),
            reconvergent(),
            reconvergent_swapped(),
            chain(),
            chain_relabelled(),
            chain_rewired(),
            spine_with_closed(),
            two_input(),
            two_input_boundary_swapped(),
            one_step_value(),
            one_step_operation(),
            two_wires_identity(),
            two_wires_swapped(),
            fan_out_pair(),
            fork_then_step(),
            descending_spine(),
            nested_repeated_label(),
            nested_repeated_label_permuted(),
            twin_headed_pair(),
            twin_headed_pair_permuted(),
            twin_closed(),
            twin_closed_interleaved(),
        ]
    }

    #[test]
    fn canonicalization_is_total_and_its_witness_verifies()
    {
        // The L1 validator, run on every fixture. Totality is what the boundary-
        // honesty refusal buys and it is asserted rather than assumed: every wire
        // and every hyperedge is mapped, the form declares exactly as many of
        // each as the source, and the witness checks out as an isomorphism.
        for diagram in every_fixture() {
            let canonical = canonicalize(&diagram);
            assert_eq!(
                Ok(()),
                canonical.relabelling().verify(&diagram, canonical.form()),
                "the witness canonicalization returns is an isomorphism onto the form it returns"
            );
            assert_eq!(
                diagram.wire_count(),
                canonical.relabelling().mapped_wires(),
                "every wire of the source is mapped, so the numbering is total"
            );
            assert_eq!(
                diagram.edge_count(),
                canonical.relabelling().mapped_generators(),
                "and so is every hyperedge"
            );
            assert_eq!(
                diagram.wire_count(),
                canonical.form().wire_count(),
                "the form declares the same wires"
            );
            assert_eq!(
                diagram.edge_count(),
                canonical.form().edge_count(),
                "and holds the same hyperedges"
            );
        }
    }

    #[test]
    fn the_boundary_is_numbered_before_the_interior()
    {
        // The declared intension, asserted through the form: the input leg takes
        // the lowest canonical numbers in declared order, then the output leg,
        // then the interior in traversal order.
        let canonical = canonicalize(&spine());
        assert_eq!(
            &Interface::new([Wire(0)], [Wire(1)]),
            canonical.form().boundary(),
            "the input port is wire 0 and the output port wire 1, before any interior wire"
        );
        assert_eq!(
            &[
                Generator::new(value("f"), [Wire(0)], [Wire(2)]),
                Generator::new(value("g"), [Wire(2)], [Wire(1)]),
            ][..],
            canonical.form().generators(),
            "so the interior wire is 2, and the hyperedges are in traversal order"
        );
    }

    #[test]
    fn an_anchored_component_is_ordered_by_the_boundary_and_not_by_its_labels()
    {
        // Anchoring and minimizing are two different orders, and this is the
        // fixture that separates them: the boundary reaches `z` first, while the
        // least linearization would start at `a`. A canon that routed anchored
        // components through the minimizing path — which is what dropping the
        // drain after the boundary seeds does — would produce `a` first here and
        // agree with this one everywhere else.
        let canonical = canonicalize(&descending_spine());
        assert_eq!(
            &[
                Generator::new(value("z"), [Wire(0)], [Wire(2)]),
                Generator::new(value("a"), [Wire(2)], [Wire(1)]),
            ][..],
            canonical.form().generators(),
            "the hyperedge the input port reaches is first, whatever the labels sort like"
        );
    }

    #[test]
    fn a_visited_hyperedge_numbers_its_sources_before_its_targets()
    {
        // The one shape where the promise is observable: `f` is reached through
        // its FIRST source, leaving its second source and its target both
        // unnumbered. Sources first gives the second source the lower number.
        // Numbering targets first would give `f: (0, 4) -> (3)` instead, so this
        // is the assertion that separates the two orders.
        let canonical = canonicalize(&branching());
        assert_eq!(
            &Interface::new([Wire(0), Wire(1)], [Wire(2)]),
            canonical.form().boundary(),
            "both declared inputs come first, in declared order, then the output"
        );
        assert_eq!(
            &[
                Generator::new(value("f"), [Wire(0), Wire(3)], [Wire(4)]),
                Generator::new(value("p"), [Wire(1)], [Wire(3)]),
                Generator::new(value("q"), [Wire(4)], [Wire(2)]),
            ][..],
            canonical.form().generators(),
            "`f`'s unnumbered source takes 3 and its unnumbered target takes 4"
        );
    }

    #[test]
    fn a_presentation_permutation_has_one_canonical_form()
    {
        // Renumbering the wires and relisting the hyperedges is exactly the
        // presentation freedom the canon quotients, and the verdict carries both
        // halves of the soundness derivation rather than a bare bit.
        let left = spine();
        let right = spine_permuted();
        assert_eq!(
            canonicalize(&left).into_parts().0,
            canonicalize(&right).into_parts().0,
            "the two presentations reach one form"
        );
        let verdict = same_diagram(&left, &right);
        let DiagramEquality::Same(shared) = verdict
        else {
            panic!("two presentations of one diagram are the same diagram: {verdict:?}");
        };
        assert_eq!(
            Ok(()),
            shared.left().verify(&left, shared.form()),
            "the left witness reaches the shared form"
        );
        assert_eq!(
            Ok(()),
            shared.right().verify(&right, shared.form()),
            "and so does the right one, which is what makes soundness derivable"
        );
    }

    #[test]
    fn canonicalization_is_idempotent()
    {
        // `Rigid`'s `canon-idem`, and the pinning of `Nf`'s fixed-point field:
        // reading a canonical form back as a diagram and canonicalizing it again
        // returns the same form, and the second witness is the identity.
        for diagram in every_fixture() {
            let form = canonicalize(&diagram).into_parts().0;
            let again = form
                .to_wiring()
                .expect("a canonical form is a legitimate diagram");
            let recanonicalized = canonicalize(&again);
            assert_eq!(
                &form,
                recanonicalized.form(),
                "canonicalizing a canonical form returns it"
            );
            for index in 0 .. again.wire_count().0 {
                assert_eq!(
                    Some(Wire(index)),
                    recanonicalized.relabelling().image_of_wire(Wire(index)),
                    "and its witness is the identity on wires"
                );
            }
            for index in 0 .. again.edge_count().0 {
                assert_eq!(
                    Some(Edge(index)),
                    recanonicalized
                        .relabelling()
                        .image_of_generator(Edge(index)),
                    "and on hyperedges"
                );
            }
        }
    }

    #[test]
    fn a_component_with_no_boundary_port_is_seeded_by_minimizing()
    {
        // The closed component has no anchor, so the seed is chosen by trying
        // every member and keeping the least linearization. `f` wins over `g`
        // because the record it produces first is smaller, and the two
        // presentations therefore agree.
        let canonical = canonicalize(&closed());
        assert_eq!(
            &[
                Generator::new(value("f"), Vec::new(), [Wire(0)]),
                Generator::new(value("g"), [Wire(0)], Vec::new()),
            ][..],
            canonical.form().generators(),
            "the least linearization seeds at `f`, whatever order the diagram lists"
        );
        assert_eq!(
            canonical.into_parts().0,
            canonicalize(&closed_permuted()).into_parts().0,
            "so the reversed presentation reaches the same form"
        );
    }

    #[test]
    fn components_with_no_boundary_port_are_committed_in_canonical_order()
    {
        // Two anchorless components must also be ORDERED canonically, which is
        // done by sorting their winning linearizations. `f`–`g` precedes `h`–`k`
        // whichever way the diagram lists them.
        let canonical = canonicalize(&two_closed());
        assert_eq!(
            &[
                Generator::new(value("f"), Vec::new(), [Wire(0)]),
                Generator::new(value("g"), [Wire(0)], Vec::new()),
                Generator::new(value("h"), Vec::new(), [Wire(1)]),
                Generator::new(value("k"), [Wire(1)], Vec::new()),
            ][..],
            canonical.form().generators(),
            "the `f` component is committed first and takes the lower wire"
        );
        assert_eq!(
            canonical.into_parts().0,
            canonicalize(&two_closed_permuted()).into_parts().0,
            "so listing the `h` component first changes nothing"
        );
    }

    #[test]
    fn the_least_linearization_is_compared_past_its_first_record()
    {
        // The anchorless tie-break reads the WHOLE winning linearization, and
        // nothing weaker will do. A seed's first record is its own label and
        // arity in canonical numbers, so any two same-label same-arity members
        // tie on it; a comparison stopping there — on the record, on its label,
        // or on the record count — falls back on which tied member came first
        // in the listing, which is a fact about the presentation. The two
        // fixtures separate the two places the comparison is made: the seed
        // minimization inside one component, and the order the components are
        // committed in.
        let nested = canonicalize(&nested_repeated_label());
        assert_eq!(
            &[
                Generator::new(value("a"), [Wire(0)], [Wire(1)]),
                Generator::new(value("a"), [Wire(2)], [Wire(0)]),
                Generator::new(value("q"), [Wire(1)], Vec::new()),
                Generator::new(value("p"), Vec::new(), [Wire(2)]),
            ][..],
            nested.form().generators(),
            "the seed is the SECOND `a`, whose tail is least; the first `a` ties only at the head"
        );
        assert_eq!(
            nested.into_parts().0,
            canonicalize(&nested_repeated_label_permuted())
                .into_parts()
                .0,
            "so the reversed presentation reaches the same form"
        );
        let twins = canonicalize(&twin_headed_pair());
        assert_eq!(
            &[
                Generator::new(value("a"), Vec::new(), [Wire(0)]),
                Generator::new(value("b"), [Wire(0)], Vec::new()),
                Generator::new(value("a"), Vec::new(), [Wire(1)]),
                Generator::new(value("c"), [Wire(1)], Vec::new()),
            ][..],
            twins.form().generators(),
            "the `a`-`b` component precedes `a`-`c`, which their shared first record cannot decide"
        );
        assert_eq!(
            twins.into_parts().0,
            canonicalize(&twin_headed_pair_permuted()).into_parts().0,
            "so listing the `a`-`c` component first changes nothing"
        );
    }

    #[test]
    fn two_isomorphic_anchorless_components_still_have_one_form()
    {
        // The one place the procedure's choice IS broken by presentation: two
        // isomorphic anchorless components tie on their winning linearization
        // outright, and the tie falls to source position, so which copy seeds
        // which is not an invariant of the diagram. The FORM still is, because
        // the two commits produce the same records at the same numbers — and
        // that is asserted here rather than left to the reader, since the
        // relabelling being non-canonical is exactly the shape that looks like
        // it should sink the canon and does not.
        let listed = canonicalize(&twin_closed());
        assert_eq!(
            &[
                Generator::new(value("a"), Vec::new(), [Wire(0)]),
                Generator::new(value("b"), [Wire(0)], Vec::new()),
                Generator::new(value("a"), Vec::new(), [Wire(1)]),
                Generator::new(value("b"), [Wire(1)], Vec::new()),
            ][..],
            listed.form().generators(),
            "both copies are committed, the second taking the higher wire"
        );
        let interleaved = canonicalize(&twin_closed_interleaved());
        assert_eq!(
            listed.form(),
            interleaved.form(),
            "and interleaving the two components in the listing reaches the same form"
        );
        assert_ne!(
            listed.relabelling(),
            interleaved.relabelling(),
            "while the witnesses differ, which is what makes the form's invariance the claim"
        );
        assert_eq!(
            Ok(()),
            interleaved
                .relabelling()
                .verify(&twin_closed_interleaved(), interleaved.form()),
            "both witnesses are isomorphisms even though neither is canonical"
        );
    }

    #[test]
    fn an_isolated_wire_is_numbered_from_the_boundary_alone()
    {
        // An isolated wire is a port of no hyperedge, so the boundary-honesty
        // refusal is the only reason it is ever numbered. The empty diagram and
        // the port-free pair are the other two degenerate shapes.
        let bare = canonicalize(&bare_wire());
        assert_eq!(
            WireCount(1),
            bare.form().wire_count(),
            "the isolated wire is numbered"
        );
        assert_eq!(
            &Interface::new([Wire(0)], [Wire(0)]),
            bare.form().boundary(),
            "and both legs name it, which is how it was reachable at all"
        );
        assert_eq!(
            EdgeCount(0),
            bare.form().edge_count(),
            "with no hyperedge in the form"
        );
        let nothing = canonicalize(&empty());
        assert_eq!(
            WireCount(0),
            nothing.form().wire_count(),
            "the empty diagram canonicalizes to itself"
        );
        assert_eq!(
            &Interface::default(),
            nothing.form().boundary(),
            "with an empty boundary"
        );
        let port_free = canonicalize(&port_free_pair());
        assert_eq!(
            &[
                Generator::new(value("a"), Vec::new(), Vec::new()),
                Generator::new(value("a"), Vec::new(), Vec::new()),
            ][..],
            port_free.form().generators(),
            "two port-free hyperedges are two components and both are committed"
        );
    }

    #[test]
    fn interning_consumes_the_canon()
    {
        // What this face owes content addressing is a key, and the demonstration
        // is that presentationally distinct diagrams collapse to one entry while
        // distinct diagrams do not. The interner itself stays the storage tier's.
        let mut interned: BTreeMap<CanonicalDiagram, EdgeCount> = BTreeMap::new();
        let mut next = EdgeCount(0);
        for diagram in [
            spine(),
            spine_permuted(),
            spine(),
            closed(),
            closed_permuted(),
            chain(),
            chain_relabelled(),
            chain_rewired(),
        ] {
            let form = canonicalize(&diagram).into_parts().0;
            match interned.entry(form) {
                | alloc::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(next);
                    next = EdgeCount(next.0.saturating_add(1));
                },
                | alloc::collections::btree_map::Entry::Occupied(_) => {},
            }
        }
        assert_eq!(
            4,
            interned.len(),
            "eight presentations of four diagrams intern to four entries"
        );
        let spine_id = interned
            .get(&canonicalize(&spine()).into_parts().0)
            .copied()
            .expect("the spine was interned");
        let permuted_id = interned
            .get(&canonicalize(&spine_permuted()).into_parts().0)
            .copied()
            .expect("and so was its permutation");
        assert_eq!(
            spine_id, permuted_id,
            "a presentation permutation is the same content, so it takes the same identity"
        );
        let rewired_id = interned
            .get(&canonicalize(&chain_rewired()).into_parts().0)
            .copied()
            .expect("the rewired chain was interned");
        let chain_id = interned
            .get(&canonicalize(&chain()).into_parts().0)
            .copied()
            .expect("and so was the chain");
        assert_ne!(
            chain_id, rewired_id,
            "and one hyperedge multiset wired two ways takes two identities"
        );
    }

    /// A minimal accumulating hasher, so the intern key's [`Hash`] is exercised
    /// rather than only derived. Wrapping arithmetic is the sanctioned use.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, Default)]
    struct Tally
    {
        /// The accumulated digest.
        digest: u64,
    }

    impl core::hash::Hasher for Tally
    {
        fn finish(&self) -> u64
        {
            self.digest
        }

        fn write(
            &mut self,
            bytes: &[u8],
        )
        {
            for byte in bytes {
                self.digest = self.digest.rotate_left(7).wrapping_add(u64::from(*byte));
            }
        }
    }

    #[test]
    fn equal_canonical_forms_hash_alike()
    {
        // The law a content-addressing key must satisfy: equal keys hash equal.
        // Asserted rather than assumed, because the module documentation offers
        // `Hash` as part of what interning consumes.
        let mut left = Tally::default();
        let mut right = Tally::default();
        core::hash::Hash::hash(&canonicalize(&spine()).into_parts().0, &mut left);
        core::hash::Hash::hash(&canonicalize(&spine_permuted()).into_parts().0, &mut right);
        assert_eq!(
            core::hash::Hasher::finish(&left),
            core::hash::Hasher::finish(&right),
            "two presentations of one diagram hash alike, so the key is usable"
        );
    }

    /// Two isolated wires, declared in the same order on both legs — the
    /// identity on two wires.
    fn two_wires_identity() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            Vec::new(),
            Interface::new([Wire(0), Wire(1)], [Wire(0), Wire(1)]),
        )
        .expect("two isolated wires must be declared on both legs, and are")
    }

    /// Two isolated wires whose output leg is exchanged — the symmetry. It has
    /// the same apex as [`two_wires_identity`] and a different cospan, and an
    /// ordered representation that could not tell the two apart would be
    /// unsound.
    fn two_wires_swapped() -> Wiring
    {
        Wiring::assemble(
            WireCount(2),
            Vec::new(),
            Interface::new([Wire(0), Wire(1)], [Wire(1), Wire(0)]),
        )
        .expect("declaring the same two wires in the other order on one leg")
    }

    /// `s: (0) -> (1, 2)` with both outputs open — one hyperedge, three wires,
    /// and the boundary arities of [`two_input`] the other way round.
    fn fan_out_pair() -> Wiring
    {
        Wiring::assemble(
            WireCount(3),
            alloc::vec![Generator::new(value("s"), [Wire(0)], [Wire(1), Wire(2)])],
            Interface::new([Wire(0)], [Wire(1), Wire(2)]),
        )
        .expect("many-out with both outputs open is inside the fragment")
    }

    /// `s` forking to two wires, one of which `t` consumes — four wires, two
    /// hyperedges, one open input and **two** open outputs, so it agrees with
    /// [`reconvergent`] on everything the divergence check reads before the
    /// output leg.
    fn fork_then_step() -> Wiring
    {
        Wiring::assemble(
            WireCount(4),
            alloc::vec![
                Generator::new(value("s"), [Wire(0)], [Wire(1), Wire(2)]),
                Generator::new(value("t"), [Wire(1)], [Wire(3)]),
            ],
            Interface::new([Wire(0)], [Wire(2), Wire(3)]),
        )
        .expect("a fork with one branch left open is inside the fragment")
    }

    /// The verdict of the independent cospan-isomorphism oracle.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum CospanVerdict
    {
        /// A structure-preserving bijection commuting with both legs exists.
        Isomorphic,
        /// None does.
        Distinct,
    }

    /// Decide cospan isomorphism by **exhaustive search over hyperedge
    /// bijections**, sharing no code with [`canonicalize`].
    ///
    /// Every bijection of hyperedges is tried; each forces a wire
    /// correspondence, read off the ordered port lists and the boundary
    /// positions, and the candidate is admitted when that correspondence is
    /// consistent, injective, total on the left's wires and surjective onto
    /// the right's. Exponential and deliberately naive: it is the external
    /// oracle the canon is differentially checked against, so it must be
    /// written a different way rather than a faster one.
    ///
    /// Totality of the derived map is a legitimate admission condition because
    /// every wire is a hyperedge port or a declared boundary port — which is
    /// [`Wiring::assemble`]'s boundary-honesty refusal, the same premise the
    /// canon rests on.
    fn cospan_verdict(
        left: &Wiring,
        right: &Wiring,
    ) -> CospanVerdict
    {
        if left.wire_count() != right.wire_count() {
            return CospanVerdict::Distinct;
        }
        if left.edge_count() != right.edge_count() {
            return CospanVerdict::Distinct;
        }
        if left.boundary().inputs.len() != right.boundary().inputs.len() {
            return CospanVerdict::Distinct;
        }
        if left.boundary().outputs.len() != right.boundary().outputs.len() {
            return CospanVerdict::Distinct;
        }
        let count = left.edge_count().0;
        let mut bijections: Vec<Vec<usize>> = alloc::vec![Vec::new()];
        for _ in 0 .. count {
            let mut grown: Vec<Vec<usize>> = Vec::new();
            for prefix in &bijections {
                for candidate in 0 .. count {
                    if prefix.contains(&candidate) {
                        continue;
                    }
                    let mut extended = prefix.clone();
                    extended.push(candidate);
                    grown.push(extended);
                }
            }
            bijections = grown;
        }
        for bijection in &bijections {
            let mut pairs: Vec<(Wire, Wire)> = Vec::new();
            let mut plausible = true;
            for (position, image) in bijection.iter().copied().enumerate() {
                let source = left
                    .generator(Edge(position))
                    .expect("a position below the count names a hyperedge");
                let target = right
                    .generator(Edge(image))
                    .expect("and so does its image under a bijection of that range");
                if source.label != target.label
                    || source.sources.len() != target.sources.len()
                    || source.targets.len() != target.targets.len()
                {
                    plausible = false;
                    break;
                }
                for (slot, wire) in source.sources.iter().copied().enumerate() {
                    let other = target
                        .sources
                        .get(slot)
                        .copied()
                        .expect("the source arities were just compared");
                    pairs.push((wire, other));
                }
                for (slot, wire) in source.targets.iter().copied().enumerate() {
                    let other = target
                        .targets
                        .get(slot)
                        .copied()
                        .expect("the target arities were just compared");
                    pairs.push((wire, other));
                }
            }
            if !plausible {
                continue;
            }
            for (slot, wire) in left.boundary().inputs.iter().copied().enumerate() {
                let other = right
                    .boundary()
                    .inputs
                    .get(slot)
                    .copied()
                    .expect("the input legs were compared before the search");
                pairs.push((wire, other));
            }
            for (slot, wire) in left.boundary().outputs.iter().copied().enumerate() {
                let other = right
                    .boundary()
                    .outputs
                    .get(slot)
                    .copied()
                    .expect("the output legs were compared before the search");
                pairs.push((wire, other));
            }
            let mut forward: BTreeMap<Wire, Wire> = BTreeMap::new();
            let mut backward: BTreeMap<Wire, Wire> = BTreeMap::new();
            let mut consistent = true;
            for (wire, other) in pairs {
                if let Some(bound) = forward.insert(wire, other)
                    && bound != other
                {
                    consistent = false;
                    break;
                }
                if let Some(bound) = backward.insert(other, wire)
                    && bound != wire
                {
                    consistent = false;
                    break;
                }
            }
            if !consistent {
                continue;
            }
            if forward.len() != left.wire_count().0 {
                continue;
            }
            if backward.len() != right.wire_count().0 {
                continue;
            }
            return CospanVerdict::Isomorphic;
        }
        CospanVerdict::Distinct
    }

    #[test]
    fn the_canon_agrees_with_the_cospan_isomorphism_oracle()
    {
        // The differential the bead asks for, over every ordered pair of every
        // fixture. The oracle is external: it searches hyperedge bijections and
        // derives the wire map, where the canon renumbers and compares. Both
        // verdicts occur and their counts are pinned, so the agreement is not an
        // agreement about nothing.
        let fixtures = every_fixture();
        let mut isomorphic: usize = 0;
        let mut distinct: usize = 0;
        for left in &fixtures {
            for right in &fixtures {
                let decided = match same_diagram(left, right) {
                    | DiagramEquality::Same(_) => CospanVerdict::Isomorphic,
                    | DiagramEquality::Distinct(_) => CospanVerdict::Distinct,
                };
                let oracle = cospan_verdict(left, right);
                assert_eq!(
                    oracle, decided,
                    "the canon and the search agree on {left:?} against {right:?}"
                );
                match decided {
                    | CospanVerdict::Isomorphic => isomorphic = isomorphic.saturating_add(1),
                    | CospanVerdict::Distinct => distinct = distinct.saturating_add(1),
                }
            }
        }
        assert_eq!(
            31,
            fixtures.len(),
            "the table is the fixture list, so its size is pinned with the counts below"
        );
        assert_eq!(
            45, isomorphic,
            "thirty-one reflexive pairs plus seven presentation permutations, both ways round"
        );
        assert_eq!(
            916, distinct,
            "and the remaining pairs are separated, so neither verdict is vacuous"
        );
    }

    #[test]
    fn the_canon_separates_a_permuted_boundary()
    {
        // The cospan's legs are part of the object: an isomorphism commutes with
        // them, so exchanging two open wires on one leg is a different diagram
        // even though the apex is untouched. The identity on two wires and the
        // symmetry are the sharpest instance, and an ordered representation that
        // identified them would be unsound at the one place gandr's order is
        // supposed to be a section.
        let verdict = same_diagram(&two_wires_identity(), &two_wires_swapped());
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::BoundaryPort {
                leg: Leg::Output,
                position: PortPosition(0),
                left: Wire(0),
                right: Wire(1),
            }),
            verdict,
            "the two forms part on the output leg's first position"
        );
        let with_a_hyperedge = same_diagram(&two_input(), &two_input_boundary_swapped());
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::Generator { at: Edge(0) }),
            with_a_hyperedge,
            "and with a hyperedge present the difference surfaces in its record instead"
        );
    }

    #[test]
    fn the_canon_separates_a_permuted_port_list()
    {
        // Within-hyperedge port order is carried rather than quotiented, so
        // exchanging two of one hyperedge's sources is a different diagram.
        // The FIRST canonical record is identical in both — `s` numbers its two
        // targets in its own port order, which the exchange leaves alone — so the
        // difference surfaces at the second, where `m` reads them back. Worth
        // pinning: a canon comparing only a multiset of records would miss it.
        let verdict = same_diagram(&reconvergent(), &reconvergent_swapped());
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::Generator { at: Edge(1) }),
            verdict,
            "the canonical records part at the hyperedge that reads the two wires back"
        );
        assert_eq!(
            canonicalize(&reconvergent_swapped())
                .form()
                .generators()
                .first(),
            canonicalize(&reconvergent()).form().generators().first(),
            "and they agree on the first record, so the difference is genuinely the second's"
        );
    }

    #[test]
    fn the_canon_separates_a_label_worn_at_two_sorts()
    {
        // A label is a name together with the role it is worn in, so one name at
        // two sorts is two hyperedges. A name-only canon would identify a
        // constructor with an operation frame of the same arity.
        let verdict = same_diagram(&one_step_value(), &one_step_operation());
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::Generator { at: Edge(0) }),
            verdict,
            "the records part on the label's sort"
        );
    }

    #[test]
    fn the_canon_separates_one_generator_multiset_wired_two_ways()
    {
        // Equal wire counts, equal hyperedge counts, equal boundary arities, one
        // label multiset — and a different diagram. This is the identification a
        // canon that hashed a multiset of labels would wrongly admit.
        let verdict = same_diagram(&chain(), &chain_rewired());
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::Generator { at: Edge(1) }),
            verdict,
            "the chains part at the second canonical hyperedge"
        );
        assert_eq!(
            CospanVerdict::Distinct,
            cospan_verdict(&chain(), &chain_rewired()),
            "and the external oracle agrees, so the verdict is not an artefact of the numbering"
        );
    }

    #[test]
    fn same_diagram_locates_a_count_difference()
    {
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::WireCount {
                left: WireCount(3),
                right: WireCount(4),
            }),
            same_diagram(&spine(), &chain()),
            "a wire-count difference is reported with both counts"
        );
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::GeneratorCount {
                left: EdgeCount(2),
                right: EdgeCount(1),
            }),
            same_diagram(&spine(), &two_input()),
            "and a hyperedge-count difference at equal wire counts is reported with both"
        );
    }

    #[test]
    fn same_diagram_locates_a_boundary_difference()
    {
        // Equal wire and hyperedge counts, different interface: the two legs are
        // checked in order, so the input leg's arity is what is reported.
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::BoundaryArity {
                leg: Leg::Input,
                left: PortCount(2),
                right: PortCount(1),
            }),
            same_diagram(&two_input(), &fan_out_pair()),
            "the input leg's arity difference is reported with both arities"
        );
        // The output leg is only reachable once the input legs agree, which
        // `fan_out_pair` against `two_input` does not achieve — their input
        // arities differ, so the first assertion is what that pair reports. The
        // pair below agrees on wire count, hyperedge count and input arity and
        // differs only on the output leg.
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::BoundaryArity {
                leg: Leg::Output,
                left: PortCount(1),
                right: PortCount(2),
            }),
            same_diagram(&reconvergent(), &fork_then_step()),
            "and the output leg is reported when everything before it agrees"
        );
    }

    #[test]
    fn same_diagram_locates_a_hyperedge_difference()
    {
        assert_eq!(
            DiagramEquality::Distinct(DiagramDivergence::Generator { at: Edge(2) }),
            same_diagram(&spine_with_closed(), &two_closed_with_one_relabelled()),
            "the first canonical hyperedge that differs is named"
        );
    }

    /// [`spine_with_closed`] with the closed component's producer renamed, so
    /// the two forms agree until the third canonical hyperedge.
    fn two_closed_with_one_relabelled() -> Wiring
    {
        Wiring::assemble(
            WireCount(4),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], [Wire(2)]),
                Generator::new(value("r"), Vec::new(), [Wire(3)]),
                Generator::new(value("q"), [Wire(3)], Vec::new()),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        )
        .expect("renaming one hyperedge leaves the shape alone")
    }

    /// A canonical form with its boundary and records replaced, for the
    /// validator's refusal arms. The wire count is kept, so a defect is
    /// isolated to the part under test.
    fn form_with(
        form: &CanonicalDiagram,
        boundary: Interface,
        generators: Vec<Generator>,
    ) -> CanonicalDiagram
    {
        CanonicalDiagram {
            wires: form.wire_count(),
            boundary,
            generators: generators.into_boxed_slice(),
        }
    }

    /// The witness [`canonicalize`] produced for `diagram`, opened up so a test
    /// can damage one entry of it.
    fn opened_witness(
        diagram: &Wiring
    ) -> (CanonicalDiagram, BTreeMap<Wire, Wire>, BTreeMap<Edge, Edge>)
    {
        let (form, relabelling) = canonicalize(diagram).into_parts();
        (form, relabelling.wires, relabelling.generators)
    }

    #[test]
    fn the_verifier_refuses_a_defective_wire_map()
    {
        let diagram = spine();
        let (form, wires, generators) = opened_witness(&diagram);
        let intact = Relabelling {
            wires: wires.clone(),
            generators: generators.clone(),
        };
        assert_eq!(
            Err(RelabellingDefect::WireCountMismatch {
                source: WireCount(3),
                form: WireCount(4),
            }),
            intact.verify(&diagram, canonicalize(&chain()).form()),
            "a form declaring a different number of wires admits no bijection at all"
        );
        // The boundary input, deliberately: the form declares three wires, so
        // `Wire(3)` is the first image outside it. A distant value such as
        // `Wire(9)` leaves a `>=` weakened to `>` alive, which is exactly the
        // mutant this assertion exists to kill.
        let mut out_of_range = wires.clone();
        out_of_range.insert(Wire(0), Wire(3));
        let refusal = Relabelling {
            wires: out_of_range,
            generators: generators.clone(),
        };
        assert_eq!(
            Err(RelabellingDefect::WireImageOutOfRange {
                wire: Wire(0),
                image: Wire(3),
            }),
            refusal.verify(&diagram, &form),
            "the first image outside the form's wires is refused, naming both"
        );
        let mut reused = wires.clone();
        reused.insert(Wire(1), Wire(0));
        let refusal = Relabelling {
            wires: reused,
            generators: generators.clone(),
        };
        assert_eq!(
            Err(RelabellingDefect::WireImageReused {
                wire: Wire(1),
                image: Wire(0),
                bound: Wire(0),
            }),
            refusal.verify(&diagram, &form),
            "two wires sharing an image is refused, naming what already reached it"
        );
        let mut unmapped = wires;
        unmapped.remove(&Wire(2));
        let refusal = Relabelling {
            wires: unmapped,
            generators,
        };
        assert_eq!(
            Err(RelabellingDefect::WireUnmapped { wire: Wire(2) }),
            refusal.verify(&diagram, &form),
            "and a wire with no image is refused, so a partial numbering cannot pass"
        );
    }

    #[test]
    fn the_verifier_refuses_a_defective_generator_map()
    {
        let diagram = spine();
        let (form, wires, generators) = opened_witness(&diagram);
        let intact = Relabelling {
            wires: wires.clone(),
            generators: generators.clone(),
        };
        assert_eq!(
            Err(RelabellingDefect::EdgeCountMismatch {
                source: EdgeCount(2),
                form: EdgeCount(1),
            }),
            intact.verify(&diagram, canonicalize(&two_input()).form()),
            "a form holding a different number of hyperedges admits no bijection"
        );
        // The boundary again: the form holds two hyperedges, so `Edge(2)` is the
        // first image outside it.
        let mut out_of_range = generators.clone();
        out_of_range.insert(Edge(0), Edge(2));
        let refusal = Relabelling {
            wires: wires.clone(),
            generators: out_of_range,
        };
        assert_eq!(
            Err(RelabellingDefect::GeneratorImageOutOfRange {
                at: Edge(0),
                image: Edge(2),
            }),
            refusal.verify(&diagram, &form),
            "the first image outside the form's hyperedges is refused, naming both"
        );
        let mut reused = generators.clone();
        reused.insert(Edge(1), Edge(0));
        let refusal = Relabelling {
            wires: wires.clone(),
            generators: reused,
        };
        assert_eq!(
            Err(RelabellingDefect::GeneratorImageReused {
                at: Edge(1),
                image: Edge(0),
                bound: Edge(0),
            }),
            refusal.verify(&diagram, &form),
            "two hyperedges sharing an image is refused"
        );
        let mut unmapped = generators;
        unmapped.remove(&Edge(1));
        let refusal = Relabelling {
            wires,
            generators: unmapped,
        };
        assert_eq!(
            Err(RelabellingDefect::GeneratorUnmapped { at: Edge(1) }),
            refusal.verify(&diagram, &form),
            "and a hyperedge with no image is refused"
        );
    }

    #[test]
    fn the_verifier_refuses_a_record_that_does_not_correspond()
    {
        // The witness is intact and the FORM is damaged, one part at a time, so
        // each arm of the record check is separated pointwise. The spine's
        // canonical records are `f: (0) -> (2)` and `g: (2) -> (1)`.
        let diagram = spine();
        let (form, wires, generators) = opened_witness(&diagram);
        let witness = Relabelling { wires, generators };
        let boundary = form.boundary().clone();
        let second = Generator::new(value("g"), [Wire(2)], [Wire(1)]);
        let relabelled = form_with(&form, boundary.clone(), alloc::vec![
            Generator::new(value("r"), [Wire(0)], [Wire(2)]),
            second.clone(),
        ]);
        assert_eq!(
            Err(RelabellingDefect::LabelMismatch {
                at: Edge(0),
                image: Edge(0),
            }),
            witness.verify(&diagram, &relabelled),
            "a record carrying a different label is refused"
        );
        let wide_source = form_with(&form, boundary.clone(), alloc::vec![
            Generator::new(value("f"), [Wire(0), Wire(0)], [Wire(2)]),
            second.clone(),
        ]);
        assert_eq!(
            Err(RelabellingDefect::ArityMismatch {
                at: Edge(0),
                image: Edge(0),
                leg: Leg::Input,
                declared: PortCount(1),
                found: PortCount(2),
            }),
            witness.verify(&diagram, &wide_source),
            "a record with an extra source is refused on the input leg"
        );
        let wide_target = form_with(&form, boundary.clone(), alloc::vec![
            Generator::new(value("f"), [Wire(0)], [Wire(2), Wire(2)]),
            second.clone(),
        ]);
        assert_eq!(
            Err(RelabellingDefect::ArityMismatch {
                at: Edge(0),
                image: Edge(0),
                leg: Leg::Output,
                declared: PortCount(1),
                found: PortCount(2),
            }),
            witness.verify(&diagram, &wide_target),
            "and with an extra target on the output leg, so both legs are checked"
        );
        let wrong_source = form_with(&form, boundary.clone(), alloc::vec![
            Generator::new(value("f"), [Wire(1)], [Wire(2)]),
            second.clone(),
        ]);
        assert_eq!(
            Err(RelabellingDefect::PortMismatch {
                at: Edge(0),
                image: Edge(0),
                leg: Leg::Input,
                position: PortPosition(0),
            }),
            witness.verify(&diagram, &wrong_source),
            "a source whose image is not the record's port at that position is refused"
        );
        let wrong_target = form_with(&form, boundary, alloc::vec![
            Generator::new(value("f"), [Wire(0)], [Wire(1)]),
            second,
        ]);
        assert_eq!(
            Err(RelabellingDefect::PortMismatch {
                at: Edge(0),
                image: Edge(0),
                leg: Leg::Output,
                position: PortPosition(0),
            }),
            witness.verify(&diagram, &wrong_target),
            "and so is a target, naming the leg and the position"
        );
    }

    #[test]
    fn the_verifier_refuses_a_boundary_that_does_not_commute()
    {
        // The condition that makes the relabelling an isomorphism of COSPANS
        // rather than of apexes. The records are left intact so only the legs are
        // under test.
        let diagram = spine();
        let (form, wires, generators) = opened_witness(&diagram);
        let witness = Relabelling { wires, generators };
        let records: Vec<Generator> = alloc::vec![
            Generator::new(value("f"), [Wire(0)], [Wire(2)]),
            Generator::new(value("g"), [Wire(2)], [Wire(1)]),
        ];
        let wide_input = form_with(
            &form,
            Interface::new([Wire(0), Wire(1)], [Wire(1)]),
            records.clone(),
        );
        assert_eq!(
            Err(RelabellingDefect::BoundaryArityMismatch {
                leg: Leg::Input,
                declared: PortCount(1),
                found: PortCount(2),
            }),
            witness.verify(&diagram, &wide_input),
            "an input leg of the wrong arity is refused"
        );
        let narrow_output = form_with(
            &form,
            Interface::new([Wire(0)], Vec::new()),
            records.clone(),
        );
        assert_eq!(
            Err(RelabellingDefect::BoundaryArityMismatch {
                leg: Leg::Output,
                declared: PortCount(1),
                found: PortCount(0),
            }),
            witness.verify(&diagram, &narrow_output),
            "and so is an output leg of the wrong arity, so both legs are checked"
        );
        let wrong_input = form_with(&form, Interface::new([Wire(1)], [Wire(1)]), records.clone());
        assert_eq!(
            Err(RelabellingDefect::BoundaryMismatch {
                leg: Leg::Input,
                position: PortPosition(0),
            }),
            witness.verify(&diagram, &wrong_input),
            "an input port whose image is not the form's port there is refused"
        );
        let wrong_output = form_with(&form, Interface::new([Wire(0)], [Wire(0)]), records);
        assert_eq!(
            Err(RelabellingDefect::BoundaryMismatch {
                leg: Leg::Output,
                position: PortPosition(0),
            }),
            witness.verify(&diagram, &wrong_output),
            "and so is an output port, naming the leg and the position"
        );
    }
}
