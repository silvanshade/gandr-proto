//! **The spine reading** — one sequent command pattern read as a diagram with
//! an interface.
//!
//! # What this reading is for
//!
//! The matcher of [`crate::matching`] quantifies over diagrams, and the engine
//! quantifies over terms. This module is the one place the two meet: it indexes
//! a `gandr_theory_computads::pattern::CmdPat` as the monogamous acyclic
//! hypergraph the theory's conditions are stated over, so the embedding matcher
//! can be run against the alphabet the engines actually hold and its verdict
//! compared with the one-sided matcher's on the fragment where both apply.
//!
//! It is a **derived index and never a carrier**: it holds nothing the command
//! pattern does not already hold, it is rebuilt rather than stored, and nothing
//! in this crate reads a diagram back out as a term.
//!
//! # The reading, stated
//!
//! A cut `⟨p |ε c⟩` reads as one **cut wire** with the producer half above it
//! and the consumer spine below it:
//!
//! - a constructor `K(p̄)` is a generator of sort [`GeneratorSort::Value`] whose
//!   one target is its output wire and whose sources are its arguments' wires;
//! - an operation frame `f(p̄; c)` is a generator of sort
//!   [`GeneratorSort::Operation`] whose first source is the wire arriving down
//!   the spine, whose remaining sources are its arguments' wires, and whose one
//!   target continues the spine;
//! - a return-side constructor frame `K⁻(c)` is a generator of sort
//!   [`GeneratorSort::Return`] from the arriving wire to the continuing one;
//! - the terminal `★` is a **closed** generator of sort
//!   [`GeneratorSort::Terminal`] consuming the arriving wire and producing none
//!   — never an open port, because a pattern ending in `★` demands the target
//!   end there too while a pattern ending in a covariable does not;
//! - a producer metavariable is an **open input port** and a consumer
//!   metavariable names the arriving wire as an **open output port**.
//!
//! The sort is carried beside the name because the two are needed together: a
//! constructor `K` and an operation of the same spelling have the same arity in
//! this reading, so a name-only label would let a pattern's producer match a
//! target's consumer frame.
//!
//! # Two shapes the reading refuses, and why refusing is the honest answer
//!
//! - **A repeated hole** is a copy on a wire. Cell patterns are linear (owner
//!   ruling, 2026-08-01, `circuit-terms-question-17`), and the copy would leave
//!   the monogamous fragment by out-degree two, so it is refused here by name
//!   rather than left to be discovered as a fan-out.
//! - **A hole worn at both polarities** — `⟨r | seam(; r)⟩` — is refused
//!   because *which reading is the diagram's* is an owed decision, not a
//!   modelling choice this rung may take in passing. The matcher keys bindings
//!   by metavariable *with* its category and so counts two interface nodes,
//!   while the derived cell metadata keys by name alone and so counts one hole
//!   at two polarities; read the second way the seam is a directed cycle, which
//!   leaves the fragment before strong connectedness is even asked. The corpus
//!   records the divergence and states that the answer is owed at the circuit
//!   rung (`docs/gandr/spec/implementation/circuit-terms.md`, §"The
//!   correspondence at gandr's own rung, at theorem grade", the block quote
//!   naming the two layers). Until it is ruled, this reading declines the shape
//!   and names it.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use gandr_theory_computads::pattern::CmdPat;
use gandr_theory_computads::pattern::ConsPat;
use gandr_theory_computads::pattern::MetaVar;
use gandr_theory_computads::pattern::ProdPat;

use crate::interface::Generator;
use crate::interface::GeneratorLabel;
use crate::interface::GeneratorSort;
use crate::interface::Interface;
use crate::interface::Wire;
use crate::interface::WireCount;
use crate::interface::Wiring;
use crate::interface::WiringObstruction;

/// How the terminal consumer `★` is spelled as a generator name.
const TERMINAL: &str = "★";

/// A command pattern read as a diagram, together with the wire its cut sits on.
///
/// The cut wire is carried because it is the term reading's **anchor**: a
/// one-sided term match is exactly an embedding that sends the pattern's cut
/// wire to the target's, and without the anchor the embedding relation is the
/// strictly more general sub-diagram one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpineReading
{
    /// The diagram the command pattern reads as.
    wiring: Wiring,
    /// The wire the cut sits on.
    cut: Wire,
}

impl SpineReading
{
    /// The diagram.
    #[inline]
    #[must_use]
    pub fn wiring(&self) -> &Wiring
    {
        &self.wiring
    }

    /// The wire the cut sits on — the term reading's anchor.
    #[inline]
    #[must_use]
    pub fn cut(&self) -> Wire
    {
        self.cut
    }
}

/// Why a command pattern was refused a spine reading.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpineObstruction
{
    /// A metavariable occurs twice at one polarity — a copy on a wire, which
    /// the linearity ruling refuses and monogamy excludes.
    RepeatedHole
    {
        /// The hole whose second occurrence was found.
        hole: MetaVar,
        /// The wire the first occurrence took.
        first: Wire,
        /// The wire the second occurrence would have taken.
        second: Wire,
    },
    /// One name is worn as both a producer and a consumer hole, and which
    /// reading is the diagram's is an owed decision rather than this reading's
    /// to take.
    PolarityAmbiguousHole
    {
        /// The hole whose name is worn at the other polarity too.
        hole: MetaVar,
    },
    /// The assembled diagram is outside the monogamous acyclic fragment.
    ///
    /// Unreachable for a well-formed command pattern by the two refusals above,
    /// and kept because a reading that assumed its own output well-formed would
    /// be asserting the fragment rather than establishing it.
    Malformed
    {
        /// What the assembly refused.
        obstruction: WiringObstruction,
    },
}

/// **Read** a command pattern as a diagram with an interface, or refuse it.
///
/// # Contract
/// - requires: `cmd` is a command pattern of the sequent alphabet's cut
///   grammar.
/// - ensures: the reading's diagram holds one generator per constructor,
///   operation frame, return frame, and terminal of `cmd`; its input ports are
///   the producer metavariables in first-occurrence order; its output ports are
///   the terminal covariable, if the spine ends in one; and its cut wire is the
///   wire between the producer half and the consumer spine.
/// - provides: the bridge from the engine's term alphabet to the matcher's
///   diagram view, which is what makes the differential against the one-sided
///   matcher runnable.
/// - fails: [`SpineObstruction::RepeatedHole`] for a copy on a wire,
///   [`SpineObstruction::PolarityAmbiguousHole`] for a name worn at both
///   polarities, and [`SpineObstruction::Malformed`] if the assembled diagram
///   were outside the fragment.
/// - panics: none.
/// - intension: the walk is an explicit stack over the producer trees and a
///   loop down the consumer spine, so no part of it recurses on pattern depth.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L1 evidence — the reading returns a diagram whose incidence is
///   validated against the pattern it was read from, and the two refusal arms
///   are separated pointwise by a repeated producer hole and by a name worn at
///   both polarities.
/// - witness: `interface::spine::tests::a_spine_reads_as_its_generators_and_ports`
/// - witness: `interface::spine::tests::the_reading_declares_input_ports_in_first_occurrence_order`
/// - witness: `interface::spine::tests::a_terminal_is_a_closed_generator_not_a_port`
/// - witness: `interface::spine::tests::a_repeated_hole_is_refused_as_a_copy`
/// - witness: `interface::spine::tests::a_hole_at_both_polarities_is_refused_as_owed`
#[inline]
pub fn read_spine(cmd: &CmdPat) -> Result<SpineReading, SpineObstruction>
{
    let mut builder = Builder::new();
    let cut = builder.fresh();
    match *cmd {
        | CmdPat::Cut {
            ref prod, ref cons, ..
        } => {
            read_producer(prod, cut, &mut builder)?;
            read_consumer(cons, cut, &mut builder)?;
        },
    }
    let boundary = Interface::new(builder.inputs, builder.outputs);
    let assembled = Wiring::assemble(WireCount(builder.next), builder.generators, boundary);
    match assembled {
        | Ok(wiring) => Ok(SpineReading { wiring, cut }),
        | Err(obstruction) => Err(SpineObstruction::Malformed { obstruction }),
    }
}

/// The reading's working state — wires allocated, generators emitted, ports
/// declared, and the two hole tables the linearity and polarity refusals read.
struct Builder
{
    /// The generators emitted so far.
    generators: Vec<Generator>,
    /// The next unallocated wire index.
    next: usize,
    /// The input ports, in declaration order.
    inputs: Vec<Wire>,
    /// The output ports, in declaration order.
    outputs: Vec<Wire>,
    /// Producer hole names to the wires they took.
    producer_holes: BTreeMap<Box<str>, Wire>,
    /// Consumer hole names to the wires they named.
    consumer_holes: BTreeMap<Box<str>, Wire>,
}

impl Builder
{
    /// An empty reading state.
    fn new() -> Self
    {
        Self {
            generators: Vec::new(),
            next: 0,
            inputs: Vec::new(),
            outputs: Vec::new(),
            producer_holes: BTreeMap::new(),
            consumer_holes: BTreeMap::new(),
        }
    }

    /// Allocate the next wire.
    fn fresh(&mut self) -> Wire
    {
        let wire = Wire(self.next);
        self.next = self.next.saturating_add(1);
        wire
    }

    /// Declare `wire` as the open input port a producer hole sits on.
    ///
    /// # Contract
    /// - ensures: the hole's name is recorded once, at one polarity.
    /// - fails: [`SpineObstruction::RepeatedHole`] on a second producer
    ///   occurrence, [`SpineObstruction::PolarityAmbiguousHole`] when the name
    ///   is already worn as a consumer.
    /// - panics: none.
    /// - intension: the polarity arm here is **unreachable** for the reading as
    ///   written and kept as the symmetric half of the refusal: [`read_spine`]
    ///   reads the producer half first and a consumer spine declares its hole
    ///   only at its terminal, so no consumer hole is on the table when a
    ///   producer hole is declared. The reachable direction is
    ///   [`Builder::declare_consumer_hole`]'s, and that is the one a test
    ///   separates — no test separates this arm, deliberately.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    fn declare_producer_hole(
        &mut self,
        var: &MetaVar,
        wire: Wire,
    ) -> Result<(), SpineObstruction>
    {
        if let Some(first) = self.producer_holes.get(&var.name).copied() {
            return Err(SpineObstruction::RepeatedHole {
                hole: var.clone(),
                first,
                second: wire,
            });
        }
        if self.consumer_holes.contains_key(&var.name) {
            return Err(SpineObstruction::PolarityAmbiguousHole { hole: var.clone() });
        }
        self.producer_holes.insert(var.name.clone(), wire);
        self.inputs.push(wire);
        Ok(())
    }

    /// Declare `wire` as the open output port a consumer hole names.
    ///
    /// # Contract
    /// - ensures: the hole's name is recorded once, at one polarity.
    /// - fails: [`SpineObstruction::RepeatedHole`] on a second consumer
    ///   occurrence, [`SpineObstruction::PolarityAmbiguousHole`] when the name
    ///   is already worn as a producer.
    /// - panics: none.
    /// - intension: the repeated-hole arm here is **unreachable** while the
    ///   consumer half is a linear spine — [`read_consumer`] returns at its
    ///   first `ConsPat::Meta`, so this runs at most once per reading, and a
    ///   frame's arguments are producers. It is the symmetric half of
    ///   [`Builder::declare_producer_hole`]'s reachable repeated-hole refusal
    ///   and is kept as the check a multi-output consumer half would need; no
    ///   test separates it, deliberately.
    ///
    /// # Errors
    /// See the `- fails:` clause above.
    fn declare_consumer_hole(
        &mut self,
        var: &MetaVar,
        wire: Wire,
    ) -> Result<(), SpineObstruction>
    {
        if let Some(first) = self.consumer_holes.get(&var.name).copied() {
            return Err(SpineObstruction::RepeatedHole {
                hole: var.clone(),
                first,
                second: wire,
            });
        }
        if self.producer_holes.contains_key(&var.name) {
            return Err(SpineObstruction::PolarityAmbiguousHole { hole: var.clone() });
        }
        self.consumer_holes.insert(var.name.clone(), wire);
        self.outputs.push(wire);
        Ok(())
    }
}

/// Read a producer tree whose value arrives on `out`.
///
/// # Contract
/// - requires: `out` is already allocated and is the wire this subtree
///   produces.
/// - ensures: one [`GeneratorSort::Value`] generator per constructor node, each
///   with its arguments' wires as sources; each metavariable leaf declared as
///   an input port.
/// - fails: the hole refusals of [`Builder::declare_producer_hole`].
/// - panics: none.
/// - intension: an explicit stack of `(node, wire)` pairs in depth-first,
///   left-to-right order, so the walk never recurses on pattern depth.
///
/// # Errors
/// See the `- fails:` clause above.
fn read_producer(
    root: &ProdPat,
    out: Wire,
    builder: &mut Builder,
) -> Result<(), SpineObstruction>
{
    let mut stack: Vec<(&ProdPat, Wire)> = alloc::vec![(root, out)];
    while let Some((node, wire)) = stack.pop() {
        match *node {
            | ProdPat::Meta(ref var) => {
                builder.declare_producer_hole(var, wire)?;
            },
            | ProdPat::Ctor { ref ctor, ref args } => {
                let mut sources: Vec<Wire> = Vec::with_capacity(args.len());
                for _ in 0 .. args.len() {
                    let source = builder.fresh();
                    sources.push(source);
                }
                builder.generators.push(Generator::new(
                    GeneratorLabel::new(ctor.0.clone(), GeneratorSort::Value),
                    sources.clone(),
                    [wire],
                ));
                for (arg, source) in args.iter().zip(sources.iter().copied()).rev() {
                    stack.push((arg, source));
                }
            },
        }
    }
    Ok(())
}

/// Read a consumer spine whose value arrives on `arriving`.
///
/// # Contract
/// - requires: `arriving` is already allocated and is the wire the spine's head
///   consumes.
/// - ensures: one generator per operation frame, return frame, and terminal,
///   threaded on one wire each; a terminal covariable declared as an output
///   port.
/// - fails: the hole refusals of [`Builder::declare_consumer_hole`] and of
///   [`read_producer`] for the frames' arguments.
/// - panics: none.
/// - intension: a loop down the spine, so the walk never recurses on spine
///   length.
///
/// # Errors
/// See the `- fails:` clause above.
fn read_consumer(
    root: &ConsPat,
    arriving: Wire,
    builder: &mut Builder,
) -> Result<(), SpineObstruction>
{
    let mut node = root;
    let mut wire = arriving;
    loop {
        match *node {
            | ConsPat::Meta(ref var) => {
                builder.declare_consumer_hole(var, wire)?;
                return Ok(());
            },
            | ConsPat::Top => {
                builder.generators.push(Generator::new(
                    GeneratorLabel::new(TERMINAL, GeneratorSort::Terminal),
                    [wire],
                    Vec::new(),
                ));
                return Ok(());
            },
            | ConsPat::Frame { ref ctor, ref ret } => {
                let next = builder.fresh();
                builder.generators.push(Generator::new(
                    GeneratorLabel::new(ctor.0.clone(), GeneratorSort::Return),
                    [wire],
                    [next],
                ));
                wire = next;
                node = ret.as_ref();
            },
            | ConsPat::Op {
                ref op,
                ref args,
                ref ret,
            } => {
                let mut sources: Vec<Wire> = Vec::with_capacity(args.len().saturating_add(1));
                sources.push(wire);
                let mut arguments: Vec<Wire> = Vec::with_capacity(args.len());
                for _ in 0 .. args.len() {
                    let source = builder.fresh();
                    arguments.push(source);
                    sources.push(source);
                }
                let next = builder.fresh();
                builder.generators.push(Generator::new(
                    GeneratorLabel::new(op.0.clone(), GeneratorSort::Operation),
                    sources,
                    [next],
                ));
                for (arg, source) in args.iter().zip(arguments.iter().copied()) {
                    read_producer(arg, source, builder)?;
                }
                wire = next;
                node = ret.as_ref();
            },
        }
    }
}

#[cfg(test)]
mod tests
{
    use gandr_core_sequent::il::Polarity;
    use gandr_theory_computads::pattern::Sym;

    use super::*;
    use crate::interface::EdgeCount;

    /// `⟨Succ(m) | add(n; α)⟩` — the running two-generator pattern.
    fn succ_add() -> CmdPat
    {
        CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Succ", [ProdPat::meta("m")]),
            ConsPat::op("add", [ProdPat::meta("n")], ConsPat::meta("alpha")),
        )
    }

    #[test]
    fn a_spine_reads_as_its_generators_and_ports()
    {
        let reading = read_spine(&succ_add()).expect("the running pattern is linear");
        let wiring = reading.wiring();
        assert_eq!(
            EdgeCount(2),
            wiring.edge_count(),
            "one generator for Succ and one for the add frame"
        );
        assert_eq!(
            2,
            wiring.boundary().inputs.len(),
            "the two producer metavariables are the open inputs"
        );
        assert_eq!(
            1,
            wiring.boundary().outputs.len(),
            "and the terminal covariable is the one open output"
        );
        let producer = wiring
            .producer_of(reading.cut())
            .expect("the cut wire is produced by the constructor above it");
        let generator = wiring
            .generator(producer)
            .expect("the producing generator is in the diagram");
        assert_eq!(
            GeneratorSort::Value,
            generator.label.sort,
            "the cut's producer is the value-side constructor"
        );
        assert_eq!(
            "Succ",
            generator.label.name.0.as_ref(),
            "and it is the one the pattern names"
        );
        let consumer = wiring
            .consumer_of(reading.cut())
            .expect("the cut wire is consumed by the spine head");
        let generator = wiring
            .generator(consumer)
            .expect("the consuming generator is in the diagram");
        assert_eq!(
            GeneratorSort::Operation,
            generator.label.sort,
            "the spine head is the operation frame"
        );
        assert_eq!(
            2,
            generator.sources.len(),
            "which takes the arriving wire and its one argument"
        );
        assert_eq!(
            Some(reading.cut()),
            generator.sources.first().copied(),
            "with the arriving wire FIRST, which is the reading this module states"
        );
    }

    #[test]
    fn the_reading_declares_input_ports_in_first_occurrence_order()
    {
        // `⟨Pair(x, y) | add(u, v; α)⟩`. The reading's stated port order is only
        // observable once one node carries two producer holes: with one hole per
        // node every traversal order agrees, so a reordering of either walk
        // would be invisible.
        let cmd = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Pair", [ProdPat::meta("x"), ProdPat::meta("y")]),
            ConsPat::op(
                "add",
                [ProdPat::meta("u"), ProdPat::meta("v")],
                ConsPat::meta("alpha"),
            ),
        );
        let reading = read_spine(&cmd).expect("the fixture is linear");
        let wiring = reading.wiring();
        assert_eq!(
            alloc::vec![Wire(1), Wire(2), Wire(3), Wire(4)],
            wiring.boundary().inputs.to_vec(),
            "x, y, u, v — depth-first left-to-right, which is first-occurrence order"
        );
        assert_eq!(
            alloc::vec![Wire(5)],
            wiring.boundary().outputs.to_vec(),
            "and the terminal covariable is the one output"
        );
        let head = wiring
            .generator(
                wiring
                    .consumer_of(reading.cut())
                    .expect("the spine head consumes the cut wire"),
            )
            .expect("the spine head is in the diagram");
        assert_eq!(
            alloc::vec![Wire(0), Wire(3), Wire(4)],
            head.sources.to_vec(),
            "the operation frame takes the arriving wire and then its arguments, in order"
        );
    }

    #[test]
    fn a_terminal_is_a_closed_generator_not_a_port()
    {
        // A pattern ending in ★ demands the target end there too, so the
        // terminal cannot read as an open port: it is a generator that consumes
        // and produces nothing.
        let closed = CmdPat::cut(Polarity::Positive, ProdPat::ctor("Zero", []), ConsPat::Top);
        let reading = read_spine(&closed).expect("a ground closed cut is linear");
        let wiring = reading.wiring();
        assert_eq!(
            EdgeCount(2),
            wiring.edge_count(),
            "the constructor and the terminal are both generators"
        );
        assert!(
            wiring.boundary().outputs.is_empty(),
            "and the diagram has no open output at all"
        );
        let terminal = wiring
            .consumer_of(reading.cut())
            .expect("the terminal consumes the cut wire");
        let generator = wiring
            .generator(terminal)
            .expect("the terminal generator is in the diagram");
        assert_eq!(
            GeneratorSort::Terminal,
            generator.label.sort,
            "the terminal carries its own sort"
        );
        assert!(
            generator.targets.is_empty(),
            "and produces nothing, which is what makes it closed"
        );
    }

    #[test]
    fn a_repeated_hole_is_refused_as_a_copy()
    {
        let copied = CmdPat::cut(
            Polarity::Positive,
            ProdPat::ctor("Pair", [ProdPat::meta("x"), ProdPat::meta("x")]),
            ConsPat::Top,
        );
        let refusal = read_spine(&copied).expect_err("a repeated hole is a copy on a wire");
        let SpineObstruction::RepeatedHole { hole, .. } = refusal
        else {
            panic!("the linearity refusal is what declines this pattern");
        };
        assert_eq!(
            Sym::new("x").0,
            hole.name,
            "the refusal names the copied hole"
        );
    }

    #[test]
    fn a_hole_at_both_polarities_is_refused_as_owed()
    {
        // ⟨r | seam(; r)⟩ — one name worn as a producer and as a consumer. The
        // matcher's reading makes it two interface nodes and the metadata's
        // makes it one, which is a directed cycle; the corpus records that the
        // answer is owed, so the reading declines rather than choosing.
        let seam = CmdPat::cut(
            Polarity::Positive,
            ProdPat::meta("r"),
            ConsPat::op("seam", [], ConsPat::meta("r")),
        );
        let refusal = read_spine(&seam).expect_err("the polarity-ambiguous hole is owed a ruling");
        let SpineObstruction::PolarityAmbiguousHole { hole } = refusal
        else {
            panic!("the polarity refusal is what declines this pattern");
        };
        assert_eq!(
            Sym::new("r").0,
            hole.name,
            "the refusal names the hole whose reading is owed"
        );
    }
}
