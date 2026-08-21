//! The **matcher seam**: circuit rule bodies read as wiring diagrams, and
//! embedding-based matching supplied over them.
//!
//! `gandr-theory-circuit-algebras` owns interface bookkeeping, embedding-based
//! matching with its convexity check, and diagram normal form.
//! `gandr-theory-computads` owns cell elaboration and the cell store.
//! `gandr-theory-coherent-resolutions` owns generic overlap enumeration, the
//! completion loop and the tracelet certificates. The crate boundary between
//! them carries one consequence, recorded when the matcher landed: if the
//! engine ever consumes embedding-based matching, it does so through **a
//! supply point where the engine is instantiated**, never through a downward
//! dependency from the engine crate. The dependency direction makes that
//! unviolatable — the matcher crate already depends on the engine crate, so the
//! forbidden edge would close a cycle Cargo rejects.
//!
//! This module is that supply point. It sits above both crates, so it can name
//! each without either naming the other, and it holds the whole of what the
//! seam needs:
//!
//! * [`circuit_wiring`] reads a declared circuit body as a diagram;
//! * [`embed_circuit_rule`] answers where one rule's diagram sits inside
//!   another's;
//! * [`complete_circuit_rules`] uses those admitted embeddings to seed the
//!   generic completion loop and replays every certificate it emits.
//!
//! # Why a circuit body already is a diagram
//!
//! A body is a list of lines, each applying a head to argument ports and
//! binding one output port. Nothing about that is tree-shaped: two lines may
//! read the same port, a port may be read by none, and the lines need not be
//! connected. Read the ports as **wires** and the lines as **generators** and
//! the body is a wiring diagram exactly, with no encoding step in between:
//!
//! | body                        | diagram                                     |
//! | --------------------------- | ------------------------------------------- |
//! | a port name                 | a wire                                      |
//! | a frame or redex line       | a generator, its label the applied head     |
//! | the line's kind             | the label's role: value, operation, rewrite |
//! | the line's arguments        | the generator's source wires, in order      |
//! | the port the line binds     | the generator's single target wire          |
//! | a port read but never bound | a boundary input                            |
//! | a port bound but never read | a boundary output                           |
//!
//! The boundary is derived rather than declared, which is what makes the
//! reading total: a body that binds a port twice, or names a port outside its
//! own ports, is refused by the diagram's own assembly rather than by a check
//! here.
//!
//! # What this seam is not
//!
//! It does not put circuit vocabulary into the generic engine. The engine
//! keeps its own command-pattern matcher, cell alphabet, and certificate
//! representation; this module supplies the initial overlap family at the
//! instantiation site and lets the generic completion loop process it.

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use gandr_theory_circuit_algebras::interface::Generator;
use gandr_theory_circuit_algebras::interface::GeneratorLabel;
use gandr_theory_circuit_algebras::interface::GeneratorSort;
use gandr_theory_circuit_algebras::interface::Interface;
use gandr_theory_circuit_algebras::interface::Wire;
use gandr_theory_circuit_algebras::interface::WireCount;
use gandr_theory_circuit_algebras::interface::Wiring;
use gandr_theory_circuit_algebras::interface::WiringObstruction;
use gandr_theory_circuit_algebras::matching::MatchBudget;
use gandr_theory_circuit_algebras::matching::MatchCount;
use gandr_theory_circuit_algebras::matching::MatchObstruction;
use gandr_theory_circuit_algebras::matching::Matching;
use gandr_theory_circuit_algebras::matching::embeddings;
use gandr_theory_coherent_resolutions::CompletionBudget;
use gandr_theory_coherent_resolutions::CompletionOutcome;
use gandr_theory_coherent_resolutions::OverlapKind;
use gandr_theory_coherent_resolutions::OverlapSupport;
use gandr_theory_coherent_resolutions::complete_with_overlap_source;
use gandr_theory_coherent_resolutions::overlaps_between;
use gandr_theory_computads::CellId;
use gandr_theory_computads::CellStore;
use gandr_theory_levitation::CircuitBody;
use gandr_theory_levitation::CircuitNode;
use gandr_theory_levitation::FreeTerm;
use gandr_theory_levitation::Name;
/// Why a declared circuit body is not a diagram.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitWiringError
{
    /// A line's argument is a term rather than a port name, so it names no
    /// wire.
    ///
    /// The reading is deliberately literal: a ground argument is data the
    /// diagram layer has no vertex for, and inventing one would put a shape in
    /// the diagram that the body does not state.
    ArgumentIsNotAPort(String),
    /// The assembled diagram is not a well-formed wiring — a port bound twice,
    /// a port read twice, or a directed cycle through the body's lines.
    NotAWiring(WiringObstruction),
}

impl core::fmt::Display for CircuitWiringError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::ArgumentIsNotAPort(ref rendered) => write!(
                f,
                "the circuit body's argument `{rendered}` is a term rather than a port name, so \
                 it names no wire; write the argument as a port the body binds"
            ),
            | Self::NotAWiring(_) => write!(
                f,
                "the circuit body is not a wiring diagram: a port is bound twice, read twice, or \
                 the lines close a directed cycle"
            ),
        }
    }
}

impl core::error::Error for CircuitWiringError
{
}

/// Read a declared circuit body as a **wiring diagram**.
///
/// # Contract
/// - requires: every line argument is a port name (a [`FreeTerm::Var`]); a
///   ground argument is refused rather than given a vertex it does not have.
/// - ensures: one wire per distinct port named anywhere in the body, in
///   first-appearance order over the lines' arguments then their bound port;
///   one generator per line in source order, labelled by the applied head, with
///   the line's arguments as sources in argument order and its bound port as
///   its one target; a boundary whose inputs are the wires no line binds and
///   whose outputs are the wires no line reads, each in wire order.
/// - provides: the diagram the matcher takes, derived from the body rather than
///   declared beside it.
/// - intension: ports are interned into one ordered table whose size is the
///   next wire number, and boundary membership is decided over ordered sets, so
///   the derivation costs `O(ports log ports)` rather than being quadratic in
///   the ports before the budgeted search starts.
/// - fails: [`CircuitWiringError::ArgumentIsNotAPort`] for a ground argument,
///   and [`CircuitWiringError::NotAWiring`] when the assembled diagram is
///   refused — a port bound or read twice, or a directed cycle.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L2 — the wire, generator and boundary clauses are separated by
///   a two-line body whose intermediate port is internal (so it is neither a
///   boundary input nor a boundary output) while its argument port and its
///   final port are each one of the two; the refusal arms by a body with a
///   ground argument and by one binding a port twice.
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `a_two_line_body_reads_as_a_diagram_with_one_internal_wire`
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `a_ground_argument_names_no_wire`
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `a_body_binding_one_port_twice_is_not_a_wiring`
#[inline]
pub fn circuit_wiring(body: &CircuitBody) -> Result<Wiring, CircuitWiringError>
{
    let mut generators: Vec<Generator> = Vec::new();
    let mut bound: BTreeSet<Wire> = BTreeSet::new();
    let mut read: BTreeSet<Wire> = BTreeSet::new();
    let mut index: BTreeMap<PortKey<'_>, Wire> = BTreeMap::new();
    for node in &body.nodes {
        let mut sources = Vec::new();
        for argument in node_arguments(node) {
            let FreeTerm::Var(ref port) = *argument
            else {
                return Err(CircuitWiringError::ArgumentIsNotAPort(render_term(
                    argument,
                )));
            };
            let wire = intern(port, &mut index);
            read.insert(wire);
            sources.push(wire);
        }
        let target = intern(node.out(), &mut index);
        bound.insert(target);
        generators.push(Generator::new(
            GeneratorLabel::new(node_head(node).as_ref(), node_sort(node)),
            sources,
            alloc::vec![target],
        ));
    }
    let count = WireCount(index.len());
    let inputs: Vec<Wire> = (0_usize .. index.len())
        .map(Wire)
        .filter(|wire| !bound.contains(wire))
        .collect();
    let outputs: Vec<Wire> = (0_usize .. index.len())
        .map(Wire)
        .filter(|wire| !read.contains(wire))
        .collect();
    Wiring::assemble(count, generators, Interface::new(inputs, outputs))
        .map_err(CircuitWiringError::NotAWiring)
}

/// **Where one circuit rule's diagram sits inside another's** — the seam's
/// answer, supplied by the matcher crate.
///
/// # Contract
/// - requires: both bodies read as diagrams ([`circuit_wiring`]).
/// - ensures: the matcher's own [`Matching`] over the two diagrams, carrying
///   every admitted embedding with its wire map, its seam and its convexity
///   warrant, and every image the convexity conjunct refused.
/// - provides: the engine instantiation site's supply of embedding-based
///   matching, with the crate boundary intact — the engine crate is named
///   nowhere in this function's types.
/// - fails: [`CircuitEmbedError::Wiring`] when either body is not a diagram,
///   and [`CircuitEmbedError::Matching`] when the search exhausts its budget
///   rather than truncating a partial enumeration into a complete-looking one.
/// - panics: none.
///
/// # Errors
/// See the `- fails:` clause above.
///
/// # Adequacy
/// - hypothesis: L2 — the two failure routes are separated from the admitted
///   route by a body that is not a diagram and a budget of zero, and the
///   admitted route is validated against the diagrams it relates rather than by
///   its count alone.
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `a_rule_body_embeds_in_a_body_that_contains_it`
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `a_body_that_does_not_contain_the_pattern_admits_no_embedding`
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `an_exhausted_budget_declines_rather_than_reporting_no_match`
#[inline]
pub fn embed_circuit_rule(
    pattern: &CircuitBody,
    target: &CircuitBody,
    budget: MatchBudget,
) -> Result<Matching, CircuitEmbedError>
{
    let pattern = circuit_wiring(pattern).map_err(CircuitEmbedError::Wiring)?;
    let target = circuit_wiring(target).map_err(CircuitEmbedError::Wiring)?;
    embeddings(&pattern, &target, budget).map_err(CircuitEmbedError::Matching)
}

/// A circuit rule body paired with the cell its declaration admitted.
///
/// A declined circuit rule keeps `cell` as `None`, so the seam can preserve
/// the description route's complete matcher record without inventing an
/// engine overlap for a cell that does not exist.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CircuitRuleCell<'rule>
{
    /// The declared rule name.
    pub name: &'rule Name,
    /// The rule body supplied to the embedding matcher.
    pub body: &'rule CircuitBody,
    /// The corresponding generic cell, when the cell layer admitted it.
    pub cell: Option<CellId>,
}

/// One ordered circuit-pattern pair and the generic overlaps it supplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitCompletionMatch
{
    /// The pattern rule name.
    pub pattern: Name,
    /// The target rule name.
    pub target: Name,
    /// The number of embedding certificates admitted for the pair.
    pub admitted: MatchCount,
    /// The number of generic confluence overlaps supplied for the pair.
    pub overlap_count: usize,
    /// The number of supplied certificates that replayed successfully.
    pub certificates_replayed: usize,
}

/// The generic completion result at the circuit instantiation seam.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitCompletion
{
    /// The generic completion outcome, including its replayable certificates.
    pub outcome: CompletionOutcome,
    /// The matcher records and certificate replay counts, in pair order.
    pub matches: Vec<CircuitCompletionMatch>,
}

/// Enumerate circuit-pattern overlaps through generic completion and replay
/// the certificates that generic completion emits.
///
/// The embedding matcher decides which ordered circuit-rule pairs seed the
/// worklist. For each admitted pair, the generic cell alphabet constructs the
/// confluence overlap family with [`overlaps_between`], and the generic
/// completion engine consumes those values. Derived cells then use the
/// engine's ordinary overlap scheduler; no circuit-specific dependency enters
/// the theory stack.
///
/// # Contract
/// - requires: `rules` use cell ids from `store`; `None` means the
///   corresponding circuit rule was declined before cell admission.
/// - ensures: every matcher-admitted pair contributes its generic confluence
///   overlap family once; every emitted certificate is checked with
///   [`gandr_theory_coherent_resolutions::Tracelet::replay`] against the
///   completion store before its pair's replay count is reported.
/// - provides: the supplied instantiation seam between circuit diagrams and
///   generic cell completion.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — multi-root and reconvergent circuit embeddings both
///   produce nonempty pair records, while a non-embedding direction produces no
///   supplied overlap; the replay count separates a real certificate from a
///   matcher-only admission.
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `the_description_route_runs_completion_through_the_matcher_seam`
#[inline]
#[must_use]
pub fn complete_circuit_rules(
    store: CellStore,
    rules: &[CircuitRuleCell<'_>],
    match_budget: MatchBudget,
    completion_budget: CompletionBudget,
) -> CircuitCompletion
{
    let mut initial_overlaps = Vec::new();
    let mut pair_ids = Vec::new();
    let mut matches = Vec::new();
    for pattern in rules {
        for target in rules {
            let Ok(matching) = embed_circuit_rule(pattern.body, target.body, match_budget)
            else {
                continue;
            };
            let admitted = matching.admitted_count();
            let mut overlaps = if admitted.0 > 0_usize {
                match (pattern.cell, target.cell) {
                    | (Some(left), Some(right)) => match (store.get(left), store.get(right)) {
                        | (Some(left_cell), Some(right_cell)) => {
                            overlaps_between((left, left_cell), (right, right_cell))
                        },
                        | _ => Vec::new(),
                    },
                    | _ => Vec::new(),
                }
            }
            else {
                Vec::new()
            };
            overlaps.retain(|overlap| overlap.kind == OverlapKind::Confluence);
            let overlap_count = overlaps.len();
            initial_overlaps.extend(overlaps);
            pair_ids.push((pattern.cell, target.cell));
            matches.push(CircuitCompletionMatch {
                pattern: pattern.name.clone(),
                target: target.name.clone(),
                admitted,
                overlap_count,
                certificates_replayed: 0_usize,
            });
        }
    }
    let initial_batches = OverlapSupport::from_store(&store).batches(&initial_overlaps);
    let outcome = complete_with_overlap_source(store, completion_budget, move |_| initial_batches);
    let replayed_pairs: Vec<(CellId, CellId)> = outcome
        .certificates()
        .iter()
        .filter(|certificate| bool::from(certificate.replay(outcome.store())))
        .map(|certificate| (certificate.overlap.left, certificate.overlap.right))
        .collect();
    for (index, &(left, right)) in pair_ids.iter().enumerate() {
        let Some((left, right)) = left.zip(right)
        else {
            continue;
        };
        if let Some(matched) = matches.get_mut(index) {
            matched.certificates_replayed = replayed_pairs
                .iter()
                .filter(|pair| **pair == (left, right))
                .count();
        }
    }
    CircuitCompletion { outcome, matches }
}

/// Why one circuit rule's diagram could not be matched into another's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitEmbedError
{
    /// One of the two bodies is not a wiring diagram.
    Wiring(CircuitWiringError),
    /// The embedding search declined rather than truncating its enumeration.
    Matching(MatchObstruction),
}

impl core::fmt::Display for CircuitEmbedError
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        match *self {
            | Self::Wiring(ref error) => error.fmt(f),
            | Self::Matching(_) => write!(
                f,
                "the embedding search declined rather than reporting a partial enumeration as a \
                 complete one; re-ask with a larger budget"
            ),
        }
    }
}

impl core::error::Error for CircuitEmbedError
{
}

/// A port name, as the wire table keys it.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PortKey<'body>(&'body str);

/// The wire a port names, allocating one on first sight.
///
/// The table's own size is the next wire number, so first-appearance numbering
/// needs no second structure to count with.
///
/// # Contract
/// - ensures: the wire already held for `port`, or a fresh one numbered by the
///   count of distinct ports seen so far, recorded before it is returned.
/// - panics: none.
#[inline]
fn intern<'body>(
    port: &'body Name,
    index: &mut BTreeMap<PortKey<'body>, Wire>,
) -> Wire
{
    if let Some(&held) = index.get(&PortKey(port.as_ref())) {
        return held;
    }
    let wire = Wire(index.len());
    index.insert(PortKey(port.as_ref()), wire);
    wire
}

/// A body line's argument terms, in argument order.
#[inline]
fn node_arguments(node: &CircuitNode) -> &[FreeTerm]
{
    match *node {
        | CircuitNode::Frame(ref frame) => &frame.args,
        | CircuitNode::Redex(ref redex) => core::slice::from_ref(&redex.source),
    }
}

/// A body line's **role** — the half of its label that keeps a constructor, an
/// operation and a fired rewrite of one spelling apart.
#[inline]
fn node_sort(node: &CircuitNode) -> GeneratorSort
{
    match *node {
        | CircuitNode::Frame(ref frame) => match frame.head {
            | gandr_theory_levitation::FrameHead::Ctor(_) => GeneratorSort::Value,
            | gandr_theory_levitation::FrameHead::Op(_) => GeneratorSort::Operation,
        },
        | CircuitNode::Redex(_) => GeneratorSort::Rewrite,
    }
}

/// A body line's applied head — the generator label it becomes.
#[inline]
fn node_head(node: &CircuitNode) -> &Name
{
    match *node {
        | CircuitNode::Frame(ref frame) => frame.head.name(),
        | CircuitNode::Redex(ref redex) => &redex.rewrite,
    }
}

/// A ground argument, rendered for the refusal that names it.
#[inline]
fn render_term(term: &FreeTerm) -> String
{
    match *term {
        | FreeTerm::Var(ref name) => String::from(name.as_ref()),
        | FreeTerm::Ctor(ref name, _) | FreeTerm::Op(ref name, _) => {
            alloc::format!("{}(…)", name.as_ref())
        },
    }
}
