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
//!   [`Seam`] is that pair and [`PartialBijection`] is its half.
//!
//! # The fragment's conditions are enforced at construction, never assumed
//!
//! [`Wiring`] is the matcher's working view of one diagram with interface, and
//! it can only be built through [`Wiring::assemble`], which refuses anything
//! outside the **monogamous acyclic** fragment the whole line quantifies over:
//! in-degree and out-degree at most one on every wire, duplicate-free boundary
//! lists, no boundary input with a producer or boundary output with a consumer,
//! every open wire declared, and no directed cycle. Each refusal is a
//! [`WiringObstruction`] naming the wire or edge that caused it.
//!
//! That placement is deliberate rather than incidental. Every theorem this lane
//! rests on — the sound-and-complete convex correspondence, boundary-complement
//! uniqueness, and the automatic-convexity theorem the matcher's discharge
//! reads — carries the monogamous acyclic hypothesis, so a check performed at
//! match time would be a hypothesis the caller could route around. Performed at
//! construction it is an invariant of the type, and [`crate::matching`] states
//! its discharge against that invariant instead of re-establishing it.
//!
//! # What this module declines
//!
//! - **The representation.** The carrier's port bijection is the monogamous
//!   fragment's canonical representation and already exists; interface
//!   bookkeeping is bookkeeping *over* it and never a second carrier for it. A
//!   [`Wiring`] is a **derived index** — assembled from a carrier, never
//!   authoritative, and holding nothing the carrier does not already hold.
//! - **Positions.** The cell store's child-index paths and their order stay
//!   where they are, in `gandr_theory_computads::alphabet` — an interface is
//!   what replaces a position at the circuit rung, not a re-spelling of one.
//! - **Arity declarations.** What a cell's ports *are* is the alphabet's and
//!   the description layer's business; this module reads an interface, it does
//!   not declare one.
//!
//! # Status
//!
//! Built at `circuit-terms-rung-05`, in the shape the matcher needed: the wire
//! and hyperedge vocabulary, the discrete [`Interface`], the validated
//! [`Wiring`], the [`Seam`] pair of partial bijections, and the sequent
//! alphabet's [`spine`] reading. The boundary-complement construction itself is
//! not built and is not this rung's: matching is what `circuit-terms-rung-05`
//! owes, and a complement is owed by whichever rung first rewrites.

pub mod spine;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;

/// A **wire** — one vertex of the hypergraph a circuit term reads as.
///
/// Wires are the diagram's edges in the picture and the hypergraph's *nodes* in
/// the theory; the boxes are the hyperedges ([`Edge`]). The naming follows the
/// theory because every condition this crate enforces is stated there.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Wire(pub usize);

/// A **hyperedge** — one generator occurrence in a diagram, addressed by its
/// position in the diagram's generator list.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Edge(pub usize);

/// How many wires a diagram declares.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WireCount(pub usize);

/// How many hyperedges a diagram holds.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EdgeCount(pub usize);

/// How many pairs a [`PartialBijection`] carries.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PairCount(pub usize);

/// Which weakly-connected component of a diagram a hyperedge belongs to.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComponentIndex(pub usize);

/// How many weakly-connected components a diagram's hyperedges fall into.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ComponentCount(pub usize);

/// A generator's **name** — the box's label in the picture.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GeneratorName(pub Box<str>);

impl GeneratorName
{
    /// A generator name from anything that spells one.
    #[inline]
    #[must_use]
    pub fn new<Name>(name: Name) -> Self
    where
        Name: Into<Box<str>>,
    {
        Self(name.into())
    }
}

/// The **syntactic role** a generator's name is worn in.
///
/// A name alone does not identify a generator: the sequent alphabet's
/// constructor `K` and an operation of the same spelling are two different
/// boxes with, in general, the same arity, so a reading that forgot the role
/// would let a pattern's producer match a target's consumer frame. A fixture
/// with no such distinction to make uses [`GeneratorSort::Value`] throughout.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum GeneratorSort
{
    /// A value-side generator — the sequent alphabet's constructor `K(p̄)`.
    #[default]
    Value,
    /// An operation frame — the sequent alphabet's `f(p̄; c)`.
    Operation,
    /// A return-side constructor frame — the sequent alphabet's `K⁻(c)`.
    Return,
    /// A closed terminal — the sequent alphabet's `★`, which is a generator
    /// consuming a wire and producing none, never an open port.
    Terminal,
}

/// The **label** a hyperedge carries: a name together with the role it is worn
/// in.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct GeneratorLabel
{
    /// The generator's name.
    pub name: GeneratorName,
    /// The role the name is worn in.
    pub sort: GeneratorSort,
}

impl GeneratorLabel
{
    /// A label from a name and a role.
    #[inline]
    #[must_use]
    pub fn new<Name>(
        name: Name,
        sort: GeneratorSort,
    ) -> Self
    where
        Name: Into<Box<str>>,
    {
        Self {
            name: GeneratorName::new(name),
            sort,
        }
    }
}

/// One hyperedge's **incidence datum** — its label and its ordered ports.
///
/// Port order is carried rather than quotiented: within-cell ordering costs
/// nothing to adopt because no symmetry is present to give up
/// (`circuit-terms-question-02`), and an embedding preserves it.
///
/// The derived order is lexicographic on `(label, sources, targets)` and is
/// carried for one reason: it is the tie-break
/// [`crate::normal_form::canonicalize`] takes when a diagram holds a component
/// with no boundary port, where the canonical seed is chosen by minimizing the
/// linearization it produces. The order is on the *record*, so it is an order
/// on presentations and never a claim about diagrams.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Generator
{
    /// The label this occurrence carries.
    pub label: GeneratorLabel,
    /// The ordered source wires — the generator's inputs.
    pub sources: Box<[Wire]>,
    /// The ordered target wires — the generator's outputs.
    pub targets: Box<[Wire]>,
}

impl Generator
{
    /// A generator occurrence from its label and its two ordered port lists.
    #[inline]
    #[must_use]
    pub fn new<Sources, Targets>(
        label: GeneratorLabel,
        sources: Sources,
        targets: Targets,
    ) -> Self
    where
        Sources: Into<Box<[Wire]>>,
        Targets: Into<Box<[Wire]>>,
    {
        Self {
            label,
            sources: sources.into(),
            targets: targets.into(),
        }
    }
}

/// The **discrete boundary** a rewrite is taken relative to — the interface.
///
/// Both lists are ordered and duplicate-free; the duplicate-free condition is
/// the mono-leg half of monogamy, and [`Wiring::assemble`] enforces it.
///
/// The derived order is lexicographic on `(inputs, outputs)`, carried so that
/// [`crate::normal_form::CanonicalDiagram`] can be a key an interner orders on.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Interface
{
    /// The ordered input ports — wires no generator produces.
    pub inputs: Box<[Wire]>,
    /// The ordered output ports — wires no generator consumes.
    pub outputs: Box<[Wire]>,
}

impl Interface
{
    /// An interface from its two ordered port lists.
    #[inline]
    #[must_use]
    pub fn new<Inputs, Outputs>(
        inputs: Inputs,
        outputs: Outputs,
    ) -> Self
    where
        Inputs: Into<Box<[Wire]>>,
        Outputs: Into<Box<[Wire]>>,
    {
        Self {
            inputs: inputs.into(),
            outputs: outputs.into(),
        }
    }
}

/// Why extending a [`PartialBijection`] was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BijectionClash
{
    /// The source wire is already mapped, and not to this image.
    SourceBound
    {
        /// The source wire the extension names.
        source: Wire,
        /// Where the source already goes.
        bound: Wire,
    },
    /// The image wire is already the image of a different source, so extending
    /// would stop the map being injective.
    ImageBound
    {
        /// The image wire the extension names.
        image: Wire,
        /// Which source already reaches it.
        bound: Wire,
    },
}

/// A **partial bijection on wires** — one half of the span-level seam datum.
///
/// Injective by construction: [`PartialBijection::extend`] refuses a pair that
/// would identify two sources, and the refusal is data.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PartialBijection
{
    /// Source wire to image wire.
    forward: BTreeMap<Wire, Wire>,
    /// Image wire back to source wire — what makes injectivity checkable.
    backward: BTreeMap<Wire, Wire>,
}

impl PartialBijection
{
    /// The empty partial bijection.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    {
        Self::default()
    }

    /// Where `source` goes, if it is mapped.
    #[inline]
    #[must_use]
    pub fn image_of(
        &self,
        source: Wire,
    ) -> Option<Wire>
    {
        self.forward.get(&source).copied()
    }

    /// What reaches `image`, if anything does.
    #[inline]
    #[must_use]
    pub fn preimage_of(
        &self,
        image: Wire,
    ) -> Option<Wire>
    {
        self.backward.get(&image).copied()
    }

    /// How many pairs the map carries.
    #[inline]
    #[must_use]
    pub fn pair_count(&self) -> PairCount
    {
        PairCount(self.forward.len())
    }

    /// The pairs, in source order.
    #[inline]
    #[must_use]
    pub fn pairs(&self) -> Vec<(Wire, Wire)>
    {
        let mut pairs = Vec::with_capacity(self.forward.len());
        for (source, image) in &self.forward {
            pairs.push((*source, *image));
        }
        pairs
    }

    /// The **restriction** of the map to `ports` — one half of a [`Seam`].
    ///
    /// # Contract
    /// - requires: `ports` is duplicate-free, which every [`Interface`] port
    ///   list is.
    /// - ensures: exactly the pairs whose source is one of `ports` and is
    ///   mapped; injectivity is inherited from this map rather than
    ///   re-established, which is why the restriction cannot fail.
    /// - provides: the seam datum's two halves, without a fallible rebuild.
    /// - panics: none.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the restriction is validated against the
    ///   ports it was given and the map it came from, at the one place it is
    ///   consumed.
    /// - witness: `matching::tests::an_embedding_carries_its_seam_as_a_pair_of_partial_bijections`
    #[inline]
    #[must_use]
    pub fn restricted_to(
        &self,
        ports: &[Wire],
    ) -> Self
    {
        let mut forward: BTreeMap<Wire, Wire> = BTreeMap::new();
        let mut backward: BTreeMap<Wire, Wire> = BTreeMap::new();
        for port in ports.iter().copied() {
            if let Some(image) = self.forward.get(&port).copied() {
                forward.insert(port, image);
                backward.insert(image, port);
            }
        }
        Self { forward, backward }
    }

    /// **Extend** the map with one pair, or refuse it.
    ///
    /// # Contract
    /// - ensures: an already-present pair is accepted idempotently; a fresh
    ///   pair whose source and image are both unbound is added.
    /// - provides: injectivity as an invariant of the type rather than a
    ///   post-check, which is what lets the matcher treat a clash as a failed
    ///   candidate instead of a corrupted assignment.
    /// - fails: [`BijectionClash::SourceBound`] when the source already maps
    ///   elsewhere, [`BijectionClash::ImageBound`] when a different source
    ///   already reaches the image.
    /// - panics: none.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the returned map is validated against the
    ///   pairs offered, and each refusal arm is separated pointwise by a
    ///   re-bound source, a re-used image, and an idempotent repeat.
    /// - witness: `interface::tests::a_partial_bijection_stays_injective`
    #[inline]
    pub fn extend(
        &mut self,
        source: Wire,
        image: Wire,
    ) -> Result<(), BijectionClash>
    {
        if let Some(bound) = self.forward.get(&source).copied() {
            if bound == image {
                return Ok(());
            }
            return Err(BijectionClash::SourceBound { source, bound });
        }
        if let Some(bound) = self.backward.get(&image).copied() {
            return Err(BijectionClash::ImageBound { image, bound });
        }
        self.forward.insert(source, image);
        self.backward.insert(image, source);
        Ok(())
    }
}

/// The **span-level seam datum** — a pair of partial bijections.
///
/// This is what stands where a position used to stand. A match into a tree
/// could be addressed by one path of child indices; a match into a circuit is
/// addressed by how the pattern's interface lands on the target's wires, input
/// half and output half kept apart because the two halves play different roles
/// in the span.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Seam
{
    /// The input half — the pattern's input ports to their images.
    pub inputs: PartialBijection,
    /// The output half — the pattern's output ports to their images.
    pub outputs: PartialBijection,
}

/// Why a [`Wiring`] was refused.
///
/// Every variant names the wire or hyperedge that caused it, so a refusal is
/// usable evidence rather than a verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WiringObstruction
{
    /// A generator names a wire the diagram does not declare.
    UnknownWire
    {
        /// The wire that is out of range.
        wire: Wire,
        /// The generator that named it.
        at: Edge,
    },
    /// The boundary names a wire the diagram does not declare.
    UnknownBoundaryWire
    {
        /// The wire that is out of range.
        wire: Wire,
    },
    /// One wire is produced twice — in-degree two, which is the combining
    /// fan-in the monogamous fragment excludes.
    ///
    /// The two producers may be one generator naming the wire at two target
    /// ports, in which case `first` and `second` are equal; a repeated port is
    /// in-degree two just as two generators are, and it is refused here rather
    /// than reaching the matcher.
    FanIn
    {
        /// The doubly-produced wire.
        wire: Wire,
        /// The generator that produced it first.
        first: Edge,
        /// The generator that produced it again.
        second: Edge,
    },
    /// One wire is consumed twice — out-degree two, which is the copy the
    /// monogamous fragment excludes and the linearity ruling refuses.
    ///
    /// As with [`WiringObstruction::FanIn`], the two consumers may be one
    /// generator naming the wire at two source ports.
    FanOut
    {
        /// The doubly-consumed wire.
        wire: Wire,
        /// The generator that consumed it first.
        first: Edge,
        /// The generator that consumed it again.
        second: Edge,
    },
    /// One boundary list names a wire twice, so its leg is not mono.
    RepeatedBoundaryPort
    {
        /// The repeated wire.
        wire: Wire,
    },
    /// A boundary input is produced by a generator, so it is not an open input.
    BoundaryInputIsProduced
    {
        /// The wire declared as an input.
        wire: Wire,
        /// The generator that produces it.
        by: Edge,
    },
    /// A boundary output is consumed by a generator, so it is not an open
    /// output.
    BoundaryOutputIsConsumed
    {
        /// The wire declared as an output.
        wire: Wire,
        /// The generator that consumes it.
        by: Edge,
    },
    /// A wire no generator produces is not declared as an input, so the
    /// interface would silently lose a port.
    ///
    /// This refusal is load-bearing for [`crate::matching`]'s convexity
    /// discharge and not only for the interface's honesty: the discharge's
    /// argument reads an image input back as the image of a *declared* pattern
    /// input port, and [`crate::matching::connectivity`] quantifies over the
    /// declared list alone. An undeclared open wire would be an image input the
    /// connectivity test never asked about.
    UndeclaredInput
    {
        /// The undeclared open wire.
        wire: Wire,
    },
    /// A wire no generator consumes is not declared as an output.
    UndeclaredOutput
    {
        /// The undeclared open wire.
        wire: Wire,
    },
    /// The diagram has a directed cycle, so it is outside the acyclic fragment
    /// every theorem this lane rests on quantifies over.
    DirectedCycle
    {
        /// One generator the cycle runs through.
        through: Edge,
    },
}

/// The matcher's **working view of one diagram with interface** — a derived
/// index over a carrier, never a carrier.
///
/// A value of this type is a monogamous acyclic hypergraph with a discrete
/// boundary, and it exists only because [`Wiring::assemble`] proved it is one.
/// Both facts are load-bearing for [`crate::matching`]: monogamy is what makes
/// the search deterministic after one seed per component, and acyclicity is the
/// hypothesis under which a strongly connected pattern's match is convex
/// without a sweep.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Wiring
{
    /// The generator occurrences, addressed by [`Edge`] position.
    generators: Box<[Generator]>,
    /// The discrete boundary.
    boundary: Interface,
    /// How many wires the diagram declares.
    wires: WireCount,
    /// Which generator produces each produced wire.
    producer: BTreeMap<Wire, Edge>,
    /// Which generator consumes each consumed wire.
    consumer: BTreeMap<Wire, Edge>,
}

impl Wiring
{
    /// **Assemble** a diagram with interface, or refuse it.
    ///
    /// # Contract
    /// - requires: `generators` addresses wires below `wires`, and `boundary`
    ///   lists the diagram's open ports.
    /// - ensures: every value of this type is monogamous (in-degree and
    ///   out-degree at most one on every wire, both boundary legs mono),
    ///   boundary-honest (an input has no producer, an output no consumer, and
    ///   every open wire is declared), and acyclic.
    /// - provides: the fragment's hypotheses as an invariant of the type, so
    ///   the matcher may state its convexity discharge against them instead of
    ///   re-establishing them per match.
    /// - fails: [`WiringObstruction`] — an out-of-range wire, a fan-in, a
    ///   fan-out, a repeated boundary port, a boundary port that is not open,
    ///   an undeclared open wire, or a directed cycle; each naming the wire or
    ///   edge that caused it.
    /// - panics: none.
    /// - intension: the conditions are decided in that order, so a diagram
    ///   failing several is refused by the earliest, and the acyclicity sweep
    ///   runs last because it is the only one that is not local.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — an assembled wiring answers its own
    ///   incidence queries consistently with the generators it was given, and
    ///   each refusal arm is separated pointwise by a diagram failing only it:
    ///   an out-of-range wire named by a generator and one named by the
    ///   boundary, a doubly-produced wire and a doubly-consumed wire (by two
    ///   generators and by one generator repeating a port), a repeated port on
    ///   either leg, an input with a producer and an output with a consumer, an
    ///   undeclared open wire on either side, and a directed cycle through two
    ///   generators and through one.
    /// - witness: `interface::tests::an_assembled_wiring_answers_its_incidence`
    /// - witness: `interface::tests::the_wiring_refuses_an_out_of_range_generator_wire`
    /// - witness: `interface::tests::the_wiring_refuses_an_out_of_range_boundary_wire`
    /// - witness: `interface::tests::the_wiring_refuses_a_fan_in`
    /// - witness: `interface::tests::the_wiring_refuses_a_fan_out`
    /// - witness: `interface::tests::the_wiring_refuses_a_repeated_port_on_one_generator`
    /// - witness: `interface::tests::the_wiring_refuses_a_repeated_boundary_port`
    /// - witness: `interface::tests::the_wiring_refuses_a_produced_boundary_input`
    /// - witness: `interface::tests::the_wiring_refuses_a_consumed_boundary_output`
    /// - witness: `interface::tests::the_wiring_refuses_an_undeclared_open_wire`
    /// - witness: `interface::tests::the_wiring_refuses_an_undeclared_open_input`
    /// - witness: `interface::tests::the_wiring_refuses_a_directed_cycle`
    /// - witness: `interface::tests::the_wiring_refuses_a_self_looping_generator`
    #[inline]
    pub fn assemble(
        wires: WireCount,
        generators: Vec<Generator>,
        boundary: Interface,
    ) -> Result<Self, WiringObstruction>
    {
        let mut producer: BTreeMap<Wire, Edge> = BTreeMap::new();
        let mut consumer: BTreeMap<Wire, Edge> = BTreeMap::new();
        for (index, generator) in generators.iter().enumerate() {
            let edge = Edge(index);
            for wire in generator.sources.iter().copied() {
                if wire.0 >= wires.0 {
                    return Err(WiringObstruction::UnknownWire { wire, at: edge });
                }
                if let Some(first) = consumer.insert(wire, edge) {
                    return Err(WiringObstruction::FanOut {
                        wire,
                        first,
                        second: edge,
                    });
                }
            }
            for wire in generator.targets.iter().copied() {
                if wire.0 >= wires.0 {
                    return Err(WiringObstruction::UnknownWire { wire, at: edge });
                }
                if let Some(first) = producer.insert(wire, edge) {
                    return Err(WiringObstruction::FanIn {
                        wire,
                        first,
                        second: edge,
                    });
                }
            }
        }
        let mut inputs: BTreeSet<Wire> = BTreeSet::new();
        for wire in boundary.inputs.iter().copied() {
            if wire.0 >= wires.0 {
                return Err(WiringObstruction::UnknownBoundaryWire { wire });
            }
            if !inputs.insert(wire) {
                return Err(WiringObstruction::RepeatedBoundaryPort { wire });
            }
            if let Some(by) = producer.get(&wire).copied() {
                return Err(WiringObstruction::BoundaryInputIsProduced { wire, by });
            }
        }
        let mut outputs: BTreeSet<Wire> = BTreeSet::new();
        for wire in boundary.outputs.iter().copied() {
            if wire.0 >= wires.0 {
                return Err(WiringObstruction::UnknownBoundaryWire { wire });
            }
            if !outputs.insert(wire) {
                return Err(WiringObstruction::RepeatedBoundaryPort { wire });
            }
            if let Some(by) = consumer.get(&wire).copied() {
                return Err(WiringObstruction::BoundaryOutputIsConsumed { wire, by });
            }
        }
        for index in 0 .. wires.0 {
            let wire = Wire(index);
            if !producer.contains_key(&wire) && !inputs.contains(&wire) {
                return Err(WiringObstruction::UndeclaredInput { wire });
            }
            if !consumer.contains_key(&wire) && !outputs.contains(&wire) {
                return Err(WiringObstruction::UndeclaredOutput { wire });
            }
        }
        let cycle = acyclicity_obstruction(&generators, &producer, &consumer);
        if let Some(obstruction) = cycle {
            return Err(obstruction);
        }
        Ok(Self {
            generators: generators.into_boxed_slice(),
            boundary,
            wires,
            producer,
            consumer,
        })
    }

    /// The generator occurrences, addressed by [`Edge`] position.
    #[inline]
    #[must_use]
    pub fn generators(&self) -> &[Generator]
    {
        &self.generators
    }

    /// The generator at `edge`, if the diagram holds one there.
    #[inline]
    #[must_use]
    pub fn generator(
        &self,
        edge: Edge,
    ) -> Option<&Generator>
    {
        self.generators.get(edge.0)
    }

    /// The discrete boundary.
    #[inline]
    #[must_use]
    pub fn boundary(&self) -> &Interface
    {
        &self.boundary
    }

    /// How many wires the diagram declares.
    #[inline]
    #[must_use]
    pub fn wire_count(&self) -> WireCount
    {
        self.wires
    }

    /// How many generator occurrences the diagram holds.
    #[inline]
    #[must_use]
    pub fn edge_count(&self) -> EdgeCount
    {
        EdgeCount(self.generators.len())
    }

    /// Which generator produces `wire`, if any does.
    #[inline]
    #[must_use]
    pub fn producer_of(
        &self,
        wire: Wire,
    ) -> Option<Edge>
    {
        self.producer.get(&wire).copied()
    }

    /// Which generator consumes `wire`, if any does.
    #[inline]
    #[must_use]
    pub fn consumer_of(
        &self,
        wire: Wire,
    ) -> Option<Edge>
    {
        self.consumer.get(&wire).copied()
    }

    /// The diagram's **weakly-connected components** over hyperedge adjacency.
    ///
    /// Two generators are adjacent when they share a wire — one produces it and
    /// the other consumes it. The relation is the *undirected* one, so a
    /// component is a piece of the picture with no wire leaving it, which is
    /// what both consumers below mean by "component".
    ///
    /// This is the crate's one incidence-walk primitive over generators, and it
    /// is shared deliberately: [`crate::matching`] takes one nondeterministic
    /// seed per component, and [`crate::normal_form`] canonicalizes each
    /// component with no boundary port by minimizing over its members. Two
    /// hand-rolled walks that had to agree about what a component is are now
    /// one walk with one contract.
    ///
    /// # Contract
    /// - ensures: every hyperedge position `0 .. edge_count` appears in exactly
    ///   one component; components are ordered by their lowest-positioned
    ///   member and each component's members are in ascending position order; a
    ///   diagram with no hyperedge has no component.
    /// - provides: component membership as data, so a caller may both count
    ///   components and enumerate one, which a representative alone cannot
    ///   give.
    /// - panics: none.
    /// - intension: an explicit frontier over hyperedges, so the walk never
    ///   recurses on diagram size; discovery runs in ascending position order,
    ///   which is what makes the emitted order deterministic.
    ///
    /// # Adequacy
    /// - hypothesis: L1 evidence — the partition is validated against the
    ///   diagram it came from (every position covered exactly once, membership
    ///   closed under sharing a wire) rather than compared with a predicted
    ///   answer; the L3 residue is the ordering promise and the shapes that
    ///   separate the two adjacency directions, which are a producer-side-only
    ///   join and a consumer-side-only join.
    /// - witness: `interface::tests::the_components_partition_the_generators`
    /// - witness: `interface::tests::a_component_joins_through_a_shared_wire_in_both_directions`
    /// - witness: `interface::tests::a_port_free_generator_is_its_own_component`
    #[inline]
    #[must_use]
    pub fn components(&self) -> Components
    {
        let mut assigned: BTreeSet<Edge> = BTreeSet::new();
        let mut members: Vec<Box<[Edge]>> = Vec::new();
        for index in 0 .. self.generators.len() {
            let edge = Edge(index);
            if assigned.contains(&edge) {
                continue;
            }
            let mut component: BTreeSet<Edge> = BTreeSet::new();
            let mut frontier: Vec<Edge> = alloc::vec![edge];
            while let Some(current) = frontier.pop() {
                if !component.insert(current) {
                    continue;
                }
                assigned.insert(current);
                let Some(generator) = self.generators.get(current.0)
                else {
                    // Unreachable: `current` is either `edge`, whose position is
                    // below `generators.len()`, or a neighbour read out of the
                    // incidence maps, whose entries are all generator positions
                    // of this diagram. Kept as a total lookup rather than an
                    // index, per the partial-function ban.
                    continue;
                };
                for wire in &generator.sources {
                    if let Some(neighbour) = self.producer.get(wire).copied() {
                        frontier.push(neighbour);
                    }
                }
                for wire in &generator.targets {
                    if let Some(neighbour) = self.consumer.get(wire).copied() {
                        frontier.push(neighbour);
                    }
                }
            }
            let mut row: Vec<Edge> = Vec::with_capacity(component.len());
            for edge in component {
                row.push(edge);
            }
            members.push(row.into_boxed_slice());
        }
        Components {
            members: members.into_boxed_slice(),
        }
    }
}

/// A diagram's hyperedges, partitioned into weakly-connected components.
///
/// Built only by [`Wiring::components`], so a value of this type is a partition
/// of exactly one diagram's hyperedge positions.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Components
{
    /// The members of each component, ordered as [`Wiring::components`]
    /// promises.
    members: Box<[Box<[Edge]>]>,
}

impl Components
{
    /// How many components the diagram's hyperedges fall into.
    #[inline]
    #[must_use]
    pub fn count(&self) -> ComponentCount
    {
        ComponentCount(self.members.len())
    }

    /// The members of one component, in ascending position order.
    #[inline]
    #[must_use]
    pub fn members(
        &self,
        component: ComponentIndex,
    ) -> Option<&[Edge]>
    {
        let row = self.members.get(component.0)?;
        Some(row)
    }
}

/// The acyclicity sweep — Kahn's algorithm over generators, refusing the first
/// generator left unsettled.
///
/// # Contract
/// - requires: `producer` and `consumer` are the incidence maps of
///   `generators`.
/// - ensures: `None` exactly when every generator settles, which is exactly
///   when the diagram has no directed cycle.
/// - provides: the acyclicity half of the fragment's hypotheses, computed once
///   at assembly rather than per match.
/// - panics: none.
fn acyclicity_obstruction(
    generators: &[Generator],
    producer: &BTreeMap<Wire, Edge>,
    consumer: &BTreeMap<Wire, Edge>,
) -> Option<WiringObstruction>
{
    let mut pending: Vec<usize> = Vec::with_capacity(generators.len());
    for generator in generators {
        let mut waiting: usize = 0;
        for wire in &generator.sources {
            if producer.contains_key(wire) {
                waiting = waiting.saturating_add(1);
            }
        }
        pending.push(waiting);
    }
    let mut ready: Vec<Edge> = Vec::new();
    for (index, waiting) in pending.iter().enumerate() {
        if *waiting == 0 {
            ready.push(Edge(index));
        }
    }
    let mut settled: usize = 0;
    while let Some(edge) = ready.pop() {
        settled = settled.saturating_add(1);
        let Some(generator) = generators.get(edge.0)
        else {
            continue;
        };
        for wire in &generator.targets {
            let Some(next) = consumer.get(wire).copied()
            else {
                continue;
            };
            let Some(waiting) = pending.get_mut(next.0)
            else {
                continue;
            };
            *waiting = waiting.saturating_sub(1);
            if *waiting == 0 {
                ready.push(next);
            }
        }
    }
    if settled == generators.len() {
        return None;
    }
    for (index, waiting) in pending.iter().enumerate() {
        if *waiting > 0 {
            return Some(WiringObstruction::DirectedCycle {
                through: Edge(index),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests
{
    use super::*;

    /// A value-sorted generator label.
    fn value<Name>(name: Name) -> GeneratorLabel
    where
        Name: Into<Box<str>>,
    {
        GeneratorLabel::new(name, GeneratorSort::Value)
    }

    /// `f: (0) -> (1)` followed by `g: (1) -> (2)`, open at both ends.
    fn two_step() -> Result<Wiring, WiringObstruction>
    {
        Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], [Wire(2)]),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        )
    }

    #[test]
    fn an_assembled_wiring_answers_its_incidence()
    {
        let wiring = two_step().expect("the two-step diagram is monogamous and acyclic");
        assert_eq!(
            EdgeCount(2),
            wiring.edge_count(),
            "the diagram holds both generators"
        );
        assert_eq!(
            WireCount(3),
            wiring.wire_count(),
            "and declares all three wires"
        );
        assert_eq!(
            None,
            wiring.producer_of(Wire(0)),
            "the input wire is produced by nothing"
        );
        assert_eq!(
            Some(Edge(0)),
            wiring.consumer_of(Wire(0)),
            "and consumed by the first generator"
        );
        assert_eq!(
            Some(Edge(0)),
            wiring.producer_of(Wire(1)),
            "the middle wire runs from the first generator"
        );
        assert_eq!(
            Some(Edge(1)),
            wiring.consumer_of(Wire(1)),
            "into the second"
        );
        assert_eq!(
            None,
            wiring.consumer_of(Wire(2)),
            "and the output wire is consumed by nothing"
        );
    }

    #[test]
    fn the_wiring_refuses_a_fan_in()
    {
        let refusal = Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(2)]),
                Generator::new(value("g"), [Wire(1)], [Wire(2)]),
            ],
            Interface::new([Wire(0), Wire(1)], [Wire(2)]),
        )
        .expect_err("in-degree two is the fan-in the fragment excludes");
        assert_eq!(
            WiringObstruction::FanIn {
                wire: Wire(2),
                first: Edge(0),
                second: Edge(1)
            },
            refusal,
            "the refusal names the doubly-produced wire and both producers"
        );
    }

    #[test]
    fn the_wiring_refuses_a_fan_out()
    {
        let refusal = Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(0)], [Wire(2)]),
            ],
            Interface::new([Wire(0)], [Wire(1), Wire(2)]),
        )
        .expect_err("out-degree two is the copy on a wire the linearity ruling refuses");
        assert_eq!(
            WiringObstruction::FanOut {
                wire: Wire(0),
                first: Edge(0),
                second: Edge(1)
            },
            refusal,
            "the refusal names the copied wire and both consumers"
        );
    }

    #[test]
    fn the_wiring_refuses_an_out_of_range_generator_wire()
    {
        // Both port lists are checked, because a range check on only one of
        // them would leave the other able to name a wire the diagram does not
        // declare — and `wire_count` is what the matcher enumerates seed images
        // over, so an undeclared wire is one the search can never reach.
        //
        // The BOUNDARY value on both sides, deliberately: the diagram declares
        // one wire, so `Wire(1)` is the first position outside it. A distant
        // value such as `Wire(5)` leaves a `>=` weakened to `>` alive, which is
        // exactly the mutant these assertions exist to kill.
        let bad_source = Wiring::assemble(
            WireCount(1),
            alloc::vec![Generator::new(value("f"), [Wire(1)], [Wire(0)])],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect_err("a source naming a wire the diagram does not declare is out of range");
        assert_eq!(
            WiringObstruction::UnknownWire {
                wire: Wire(1),
                at: Edge(0)
            },
            bad_source,
            "the refusal names the out-of-range wire and the generator that named it"
        );
        let bad_target = Wiring::assemble(
            WireCount(1),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(1)])],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect_err("and so is a target naming one");
        assert_eq!(
            WiringObstruction::UnknownWire {
                wire: Wire(1),
                at: Edge(0)
            },
            bad_target,
            "with the target side named the same way"
        );
    }

    #[test]
    fn the_wiring_refuses_an_out_of_range_boundary_wire()
    {
        // The boundary value on both legs, for the reason the generator-side
        // refusal above states: `Wire(1)` is the first position outside a
        // one-wire diagram, and a distant value leaves `>=` weakened to `>`
        // alive on whichever leg carries it.
        let bad_input = Wiring::assemble(
            WireCount(1),
            Vec::new(),
            Interface::new([Wire(1)], [Wire(0)]),
        )
        .expect_err("an input port outside the wire range is not a port of this diagram");
        assert_eq!(
            WiringObstruction::UnknownBoundaryWire { wire: Wire(1) },
            bad_input,
            "the refusal names the out-of-range input port"
        );
        let bad_output = Wiring::assemble(
            WireCount(1),
            Vec::new(),
            Interface::new([Wire(0)], [Wire(1)]),
        )
        .expect_err("nor is an output port outside it");
        assert_eq!(
            WiringObstruction::UnknownBoundaryWire { wire: Wire(1) },
            bad_output,
            "and the output leg is checked as well as the input leg"
        );
    }

    #[test]
    fn the_wiring_refuses_a_repeated_port_on_one_generator()
    {
        // One generator naming a wire at two ports is in-degree or out-degree
        // two just as two generators are. It is the shape the matcher's
        // monogamy argument needs excluded, and the refusal reports the one
        // generator as both producers.
        let repeated_source = Wiring::assemble(
            WireCount(2),
            alloc::vec![Generator::new(value("f"), [Wire(0), Wire(0)], [Wire(1)])],
            Interface::new([Wire(0)], [Wire(1)]),
        )
        .expect_err("a wire consumed at two ports of one generator is a copy");
        assert_eq!(
            WiringObstruction::FanOut {
                wire: Wire(0),
                first: Edge(0),
                second: Edge(0)
            },
            repeated_source,
            "the refusal names the one generator twice rather than inventing a second"
        );
        let repeated_target = Wiring::assemble(
            WireCount(2),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(1), Wire(1)])],
            Interface::new([Wire(0)], [Wire(1)]),
        )
        .expect_err("and a wire produced at two of its ports is a fan-in");
        assert_eq!(
            WiringObstruction::FanIn {
                wire: Wire(1),
                first: Edge(0),
                second: Edge(0)
            },
            repeated_target,
            "reported the same way on the produced side"
        );
    }

    #[test]
    fn the_wiring_refuses_a_repeated_boundary_port()
    {
        let refusal = Wiring::assemble(
            WireCount(1),
            Vec::new(),
            Interface::new([Wire(0), Wire(0)], [Wire(0)]),
        )
        .expect_err("a repeated port is a non-mono leg");
        assert_eq!(
            WiringObstruction::RepeatedBoundaryPort { wire: Wire(0) },
            refusal,
            "the refusal names the repeated port"
        );
        let on_the_output_leg = Wiring::assemble(
            WireCount(1),
            Vec::new(),
            Interface::new([Wire(0)], [Wire(0), Wire(0)]),
        )
        .expect_err("both legs are mono, so the output leg is checked too");
        assert_eq!(
            WiringObstruction::RepeatedBoundaryPort { wire: Wire(0) },
            on_the_output_leg,
            "and the output leg's repeat is refused the same way"
        );
    }

    #[test]
    fn the_wiring_refuses_a_produced_boundary_input()
    {
        let refusal = Wiring::assemble(
            WireCount(2),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(1)])],
            Interface::new([Wire(0), Wire(1)], [Wire(1)]),
        )
        .expect_err("an input with a producer is not an open port");
        assert_eq!(
            WiringObstruction::BoundaryInputIsProduced {
                wire: Wire(1),
                by: Edge(0)
            },
            refusal,
            "the refusal names the port and the generator that produces it"
        );
    }

    #[test]
    fn the_wiring_refuses_a_consumed_boundary_output()
    {
        let refusal = Wiring::assemble(
            WireCount(2),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(1)])],
            Interface::new([Wire(0)], [Wire(0), Wire(1)]),
        )
        .expect_err("an output with a consumer is not an open port");
        assert_eq!(
            WiringObstruction::BoundaryOutputIsConsumed {
                wire: Wire(0),
                by: Edge(0)
            },
            refusal,
            "the refusal names the port and the generator that consumes it"
        );
    }

    #[test]
    fn the_wiring_refuses_an_undeclared_open_wire()
    {
        let refusal = Wiring::assemble(
            WireCount(2),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(1)])],
            Interface::new([Wire(0)], Vec::new()),
        )
        .expect_err("an open output the interface does not declare is a lost port");
        assert_eq!(
            WiringObstruction::UndeclaredOutput { wire: Wire(1) },
            refusal,
            "the refusal names the wire the interface would have lost"
        );
    }

    #[test]
    fn the_wiring_refuses_an_undeclared_open_input()
    {
        // The input side of the same condition, and it carries more than
        // interface honesty: `matching::connectivity` quantifies over the
        // DECLARED input list, and the convexity discharge's argument reads an
        // image input back as the image of a declared input port. An undeclared
        // open input would be an image input no connectivity test asked about,
        // so a pattern could earn the discharge and still admit a non-convex
        // match. The refusal is what makes that argument's premise hold.
        let refusal = Wiring::assemble(
            WireCount(2),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(1)])],
            Interface::new(Vec::new(), [Wire(1)]),
        )
        .expect_err("an open input the interface does not declare is a lost port");
        assert_eq!(
            WiringObstruction::UndeclaredInput { wire: Wire(0) },
            refusal,
            "the refusal names the wire the interface would have lost"
        );
    }

    #[test]
    fn the_wiring_refuses_a_self_looping_generator()
    {
        // One generator consuming the wire it produces is a directed cycle of
        // length one. It leaves exactly ONE generator unsettled, so a settled
        // count compared with anything but equality would admit it — and
        // acyclicity of the target is the second hypothesis the convexity
        // discharge rests on, alongside the pattern's strong connectedness.
        let refusal = Wiring::assemble(
            WireCount(1),
            alloc::vec![Generator::new(value("f"), [Wire(0)], [Wire(0)])],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect_err("a self-loop leaves the acyclic fragment");
        assert_eq!(
            WiringObstruction::DirectedCycle { through: Edge(0) },
            refusal,
            "the refusal names the generator the one-step cycle runs through"
        );
    }

    #[test]
    fn the_wiring_refuses_a_directed_cycle()
    {
        let refusal = Wiring::assemble(
            WireCount(2),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], [Wire(0)]),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect_err("a two-generator loop leaves the acyclic fragment");
        assert_eq!(
            WiringObstruction::DirectedCycle { through: Edge(0) },
            refusal,
            "the refusal names a generator the cycle runs through"
        );
    }

    #[test]
    fn the_components_partition_the_generators()
    {
        // Validated against the diagram rather than against a predicted
        // partition: every hyperedge position appears in exactly one component,
        // and every member of a component shares a wire with another member
        // unless the component is a singleton. Two joined generators and one
        // separate pair make the partition non-trivial in both directions.
        let wiring = Wiring::assemble(
            WireCount(5),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(2)], [Wire(3)]),
                Generator::new(value("h"), [Wire(1)], [Wire(4)]),
            ],
            Interface::new([Wire(0), Wire(2)], [Wire(3), Wire(4)]),
        )
        .expect("two components, one of two generators and one of one");
        let components = wiring.components();
        assert_eq!(
            ComponentCount(2),
            components.count(),
            "`f`–`h` share wire 1; `g` shares nothing"
        );
        let mut seen: BTreeSet<Edge> = BTreeSet::new();
        for index in 0 .. components.count().0 {
            let members = components
                .members(ComponentIndex(index))
                .expect("a component below the count has members");
            assert!(
                !members.is_empty(),
                "no component is empty, so a count is never a promise about nothing"
            );
            let mut previous: Option<Edge> = None;
            for member in members.iter().copied() {
                assert!(
                    seen.insert(member),
                    "no hyperedge is placed in two components"
                );
                if let Some(previous) = previous {
                    assert!(previous < member, "members are in ascending position order");
                }
                previous = Some(member);
            }
        }
        assert_eq!(
            wiring.edge_count().0,
            seen.len(),
            "and every hyperedge is placed in one"
        );
        assert_eq!(
            Some(&[Edge(0), Edge(2)][..]),
            components.members(ComponentIndex(0)),
            "the first component is the one holding the lowest-positioned generator"
        );
        assert_eq!(
            Some(&[Edge(1)][..]),
            components.members(ComponentIndex(1)),
            "and the second is the separate one"
        );
        assert_eq!(
            None,
            components.members(ComponentIndex(2)),
            "a component index at the count names nothing"
        );
    }

    #[test]
    fn a_component_joins_through_a_shared_wire_in_both_directions()
    {
        // The walk reads producers of its sources AND consumers of its targets,
        // and the two directions are separated pointwise. Seeding at position 0
        // in each fixture makes exactly one direction load-bearing: in the first
        // the seed reaches its neighbour forwards (its target's consumer), in
        // the second backwards (its source's producer). Dropping either
        // direction splits the component in one of the two.
        let forwards = Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
                Generator::new(value("g"), [Wire(1)], [Wire(2)]),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        )
        .expect("the seed's target is the second generator's source");
        assert_eq!(
            ComponentCount(1),
            forwards.components().count(),
            "the consumer-of-a-target direction joins them"
        );
        let backwards = Wiring::assemble(
            WireCount(3),
            alloc::vec![
                Generator::new(value("g"), [Wire(1)], [Wire(2)]),
                Generator::new(value("f"), [Wire(0)], [Wire(1)]),
            ],
            Interface::new([Wire(0)], [Wire(2)]),
        )
        .expect("the same diagram with the generators listed the other way round");
        assert_eq!(
            ComponentCount(1),
            backwards.components().count(),
            "the producer-of-a-source direction joins them"
        );
    }

    #[test]
    fn a_port_free_generator_is_its_own_component()
    {
        // A generator with no ports is adjacent to nothing, so it is a singleton
        // component — and it is the only shape that can be a component with no
        // wire at all, which is why `crate::normal_form` handles it separately
        // from a component whose wires simply never reach the boundary.
        let wiring = Wiring::assemble(
            WireCount(0),
            alloc::vec![
                Generator::new(value("a"), Vec::new(), Vec::new()),
                Generator::new(value("a"), Vec::new(), Vec::new()),
            ],
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("two port-free generators are a legitimate diagram");
        let components = wiring.components();
        assert_eq!(
            ComponentCount(2),
            components.count(),
            "equal labels do not make one component"
        );
        assert_eq!(
            Some(&[Edge(0)][..]),
            components.members(ComponentIndex(0)),
            "each is a singleton"
        );
        assert_eq!(
            Some(&[Edge(1)][..]),
            components.members(ComponentIndex(1)),
            "in position order"
        );
        let empty = Wiring::assemble(
            WireCount(0),
            Vec::new(),
            Interface::new(Vec::new(), Vec::new()),
        )
        .expect("the empty diagram is a diagram");
        assert_eq!(
            ComponentCount(0),
            empty.components().count(),
            "and a diagram with no hyperedge has no component"
        );
    }

    #[test]
    fn a_partial_bijection_stays_injective()
    {
        let mut map = PartialBijection::new();
        assert_eq!(
            Ok(()),
            map.extend(Wire(0), Wire(7)),
            "the first pair is free"
        );
        assert_eq!(
            Ok(()),
            map.extend(Wire(0), Wire(7)),
            "and repeating it is idempotent rather than a clash"
        );
        assert_eq!(
            Err(BijectionClash::SourceBound {
                source: Wire(0),
                bound: Wire(7)
            }),
            map.extend(Wire(0), Wire(8)),
            "re-binding a source is refused and names where it already goes"
        );
        assert_eq!(
            Err(BijectionClash::ImageBound {
                image: Wire(7),
                bound: Wire(0)
            }),
            map.extend(Wire(1), Wire(7)),
            "and re-using an image is refused and names what already reaches it"
        );
        assert_eq!(
            PairCount(1),
            map.pair_count(),
            "neither refusal left a partial extension behind"
        );
        assert_eq!(
            Some(Wire(7)),
            map.image_of(Wire(0)),
            "the accepted pair is readable forwards"
        );
        assert_eq!(Some(Wire(0)), map.preimage_of(Wire(7)), "and backwards");
    }
}
