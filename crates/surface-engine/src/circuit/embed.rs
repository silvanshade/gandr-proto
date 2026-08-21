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
use std::collections::HashMap;
use std::collections::HashSet;

use gandr_theory_circuit_algebras::interface::Generator;
use gandr_theory_circuit_algebras::interface::GeneratorLabel;
use gandr_theory_circuit_algebras::interface::GeneratorSort;
use gandr_theory_circuit_algebras::interface::Interface;
use gandr_theory_circuit_algebras::interface::Seam;
use gandr_theory_circuit_algebras::interface::Wire;
use gandr_theory_circuit_algebras::interface::WireCount;
use gandr_theory_circuit_algebras::interface::Wiring;
use gandr_theory_circuit_algebras::interface::WiringObstruction;
use gandr_theory_circuit_algebras::matching::Embedding;
use gandr_theory_circuit_algebras::matching::MatchBudget;
use gandr_theory_circuit_algebras::matching::MatchCount;
use gandr_theory_circuit_algebras::matching::MatchObstruction;
use gandr_theory_circuit_algebras::matching::Matching;
use gandr_theory_circuit_algebras::matching::embeddings;
use gandr_theory_coherent_resolutions::CertificateIndex;
use gandr_theory_coherent_resolutions::CompletionBudget;
use gandr_theory_coherent_resolutions::CompletionOutcome;
use gandr_theory_coherent_resolutions::Overlap;
use gandr_theory_coherent_resolutions::OverlapKind;
use gandr_theory_coherent_resolutions::complete_with_overlap_source;
use gandr_theory_computads::Cat;
use gandr_theory_computads::CellAlphabet;
use gandr_theory_computads::CellId;
use gandr_theory_computads::CellStore;
use gandr_theory_computads::ConsPat;
use gandr_theory_computads::MetaVar;
use gandr_theory_computads::Pos;
use gandr_theory_computads::ProdPat;
use gandr_theory_computads::SequentAlphabet;
use gandr_theory_computads::Subst;
use gandr_theory_computads::collect_cmd_metavars;
use gandr_theory_computads::instantiate_cell;
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

/// The bounded work the circuit-to-sequent adapter may perform.
///
/// Rendering an admitted embedding, indexing one emitted certificate, and
/// attaching one origin-bearing replay record each consume one adapter unit.
/// The matcher and generic completion budgets remain separate ceilings.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CircuitAdapterBudget(pub usize);

/// The declaration identity kept beside a structurally deduplicated cell.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CircuitDeclarationOrigin
{
    /// The circuit's declaration index in its description.
    pub index: usize,
    /// The declaration's stable source name.
    pub name: Name,
}

/// The declaration pair and embedding seam that supplied one overlap family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitEmbeddingOrigin
{
    /// The pattern declaration.
    pub pattern: CircuitDeclarationOrigin,
    /// The target declaration.
    pub target: CircuitDeclarationOrigin,
    /// The embedding's deterministic admission index.
    pub embedding: usize,
    /// The circuit seam certificate the matcher admitted.
    pub seam: Seam,
}

/// Which declaration side failed to provide a cell for an admitted embedding.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CircuitOverlapSide
{
    /// The pattern declaration.
    Pattern,
    /// The target declaration.
    Target,
}

/// Why a circuit embedding could not be rendered as a sequent overlap.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnfaithfulCircuitOverlap
{
    /// A body argument was a term rather than a wire.
    GroundArgument,
    /// The embedding did not map one of the pattern's wires.
    MissingWireMapping
    {
        /// The pattern wire without an image.
        wire: Wire,
    },
    /// A pattern metavariable could not be aligned with a mapped wire.
    UnmappedMetavariable
    {
        /// The metavariable name.
        name: String,
    },
    /// Two embeddings of one declaration pair rendered the same peak, seam and
    /// unifier, so one ordinary overlap cannot claim both circuit occurrences.
    DuplicateRenderedOverlap,
}

/// A typed decline at the circuit-to-sequent instantiation seam.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitOverlapDecline
{
    /// One declaration was admitted by matching but had no cell to instantiate.
    MissingCell
    {
        /// Which side was absent.
        side: CircuitOverlapSide,
        /// The absent cell id, when the declaration carried one.
        cell: Option<CellId>,
    },
    /// The circuit embedding has no faithful ordinary-sequent rendering.
    Unfaithful(UnfaithfulCircuitOverlap),
    /// The adapter exhausted its own bounded rendering/indexing budget.
    AdapterBudget
    {
        /// The ceiling that was reached.
        budget: CircuitAdapterBudget,
    },
}

/// One admitted embedding and its supplied-overlap evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitEmbeddingMatch
{
    /// The declaration pair and circuit seam that own this evidence.
    pub origin: CircuitEmbeddingOrigin,
    /// The ordinary sequent overlaps supplied by this embedding. These records
    /// remain attached to the origin even when structural completion work is
    /// shared with another declaration.
    pub overlaps: Vec<Overlap>,
    /// The number of ordinary sequent overlaps supplied by this embedding.
    pub overlap_count: usize,
    /// A typed decline when the embedding could not be rendered faithfully.
    pub decline: Option<CircuitOverlapDecline>,
}

/// One origin-bearing replay record for an ordinary overlap.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitReplayEvidence
{
    /// The declaration pair and circuit seam that own the replay.
    pub origin: CircuitEmbeddingOrigin,
    /// The ordinary overlap whose certificate family was replayed.
    pub overlap: Overlap,
    /// Indices into [`CompletionOutcome::certificates`] that replayed for this
    /// origin and overlap.
    pub certificate_indices: Vec<CertificateIndex>,
}

/// One ordered circuit-pattern pair and its embedding-level evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitCompletionMatch
{
    /// The pattern rule name.
    pub pattern: Name,
    /// The target rule name.
    pub target: Name,
    /// The pattern declaration index.
    pub pattern_index: usize,
    /// The target declaration index.
    pub target_index: usize,
    /// The number of embedding certificates admitted for the pair.
    pub admitted: MatchCount,
    /// Every admitted embedding, including typed rendering declines.
    pub embeddings: Vec<CircuitEmbeddingMatch>,
    /// The number of ordinary sequent confluence overlaps supplied for the
    /// pair.
    pub overlap_count: usize,
    /// The number of supplied certificates that replayed successfully.
    pub certificates_replayed: usize,
    /// A matcher-level decline, if the pair's search did not complete.
    pub matcher_decline: Option<CircuitEmbedError>,
}

/// The generic completion result at the circuit instantiation seam.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CircuitCompletion
{
    /// The generic completion outcome, including pending work and certificates.
    pub outcome: CompletionOutcome,
    /// The matcher records and certificate replay counts, in pair order.
    pub matches: Vec<CircuitCompletionMatch>,
    /// Surface-owned declaration origins for each cell id in the final store.
    pub cell_origins: BTreeMap<CellId, Vec<CircuitDeclarationOrigin>>,
    /// Origin-bearing replay records, including the full circuit seam.
    pub replay_evidence: Vec<CircuitReplayEvidence>,
    /// A circuit-adapter decline if rendering or replay indexing hit its
    /// independent ceiling.
    pub adapter_decline: Option<CircuitAdapterBudget>,
}

/// One ordinary overlap and all circuit origins that supplied it.
#[derive(Clone, Debug, Eq, PartialEq)]
struct CircuitOverlapSeed
{
    /// The ordinary sequent overlap supplied to completion.
    overlap: Overlap,
    /// Every declaration/seam origin that shares this structural work.
    origins: Vec<CircuitEmbeddingOrigin>,
}

/// A stable lookup key for one ordinary overlap.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CircuitOverlapKey<A: CellAlphabet = SequentAlphabet>
{
    /// The left cell address.
    left: CellId,
    /// The right cell address.
    right: CellId,
    /// The overlap kind.
    kind: OverlapKind,
    /// The supplied unifier.
    unifier: A::Subst,
    /// The ordinary sequent seam.
    seam: A::Pos,
    /// The supplied peak.
    peak: A::Cmd,
}

impl<A> From<&Overlap<A>> for CircuitOverlapKey<A>
where
    A: CellAlphabet,
{
    #[inline]
    fn from(overlap: &Overlap<A>) -> Self
    {
        Self {
            left: overlap.left,
            right: overlap.right,
            kind: overlap.kind,
            unifier: overlap.unifier.clone(),
            seam: overlap.seam.clone(),
            peak: overlap.peak.clone(),
        }
    }
}

/// One declaration-pair rendering identity, used to reject only duplicate
/// circuit occurrences rather than globally deduplicating their origins.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CircuitOriginOverlapKey
{
    /// The pattern declaration.
    pattern_index: usize,
    /// The target declaration.
    target_index: usize,
    /// The full circuit seam.
    seam: Seam,
    /// The ordinary overlap shape.
    overlap: CircuitOverlapKey,
}

/// The instantiated cell and supplied ordinary overlap family for one
/// embedding.
#[derive(Clone, Debug, Eq, PartialEq)]
struct RenderedCircuitOverlaps
{
    /// The structurally deduplicated instantiated pattern cell.
    cell: CellId,
    /// The ordinary sequent overlaps rendered by the embedding.
    overlaps: Vec<Overlap>,
}

/// Enumerate every admitted circuit embedding through the generic completion
/// API and replay the certificates it emits.
///
/// Each embedding is rendered independently. Its wire map induces a sequent
/// substitution, [`gandr_theory_computads::instantiate_cell`] applies that
/// substitution to the pattern cell, and the full circuit seam remains on the
/// origin-bearing evidence. The ordinary confluence seam is the valid sequent
/// root; it never pretends that a circuit wire index is a command-tree path.
/// Structural overlap work is deduplicated by a stable key, while every
/// declaration/seam origin remains attached to its own replay record.
///
/// # Contract
/// - requires: `rules` use cell ids from `store`; `None` means the
///   corresponding circuit rule was declined before cell admission.
/// - ensures: every matcher-admitted embedding has one evidence record; each
///   faithfully rendered embedding contributes one supplied ordinary confluence
///   overlap, unless the same declaration pair and seam already rendered that
///   overlap and earns a typed duplicate decline.
/// - ensures: the adapter performs one bounded rendering/indexing pass,
///   consuming at most `adapter_budget` units; a ceiling reached after partial
///   work is exposed through `adapter_decline` rather than reported as
///   complete.
/// - ensures: every emitted certificate is replayed at most once, and complete
///   replay indexing yields origin-bearing [`CircuitReplayEvidence`] records.
/// - ensures: structural cell deduplication remains intact, while
///   `cell_origins` retains every declaration that shares a cell id.
/// - provides: the supplied instantiation seam between circuit diagrams and
///   generic cell completion, above both theory crates.
/// - fails: no Rust error; matcher, rendering, and adapter refusals are typed
///   in `matches`, `replay_evidence`, and `adapter_decline`.
/// - panics: none.
///
/// # Adequacy
/// - hypothesis: L2 — separate embeddings, a non-unifying uninstantiated
///   sequent pair, and structurally deduplicated declarations are distinguished
///   by origin-bearing evidence rather than counts alone.
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `w1_two_embeddings_supply_two_critical_pairs`
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `w2_embedding_supplies_a_non_unifying_sequent_pair`
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `w3_deduplicated_declarations_keep_replay_attribution`
/// - witness: `gandr-surface-engine` `tests/circuit_embed.rs`
///   `w4_distinct_wire_renderings_keep_origin_seams`
#[inline]
#[must_use]
pub fn complete_circuit_rules(
    mut store: CellStore,
    rules: &[CircuitRuleCell<'_>],
    match_budget: MatchBudget,
    completion_budget: CompletionBudget,
    adapter_budget: CircuitAdapterBudget,
) -> CircuitCompletion
{
    let mut adapter_remaining = adapter_budget.0;
    let mut adapter_decline = None;
    let mut seeds: Vec<CircuitOverlapSeed> = Vec::new();
    let mut seed_indices: HashMap<CircuitOverlapKey, usize> = HashMap::new();
    let mut rendered_indices: HashSet<CircuitOriginOverlapKey> = HashSet::new();
    let mut matches = Vec::new();
    let mut cell_origins = BTreeMap::new();
    for (pattern_index, pattern) in rules.iter().enumerate() {
        let pattern_origin = CircuitDeclarationOrigin {
            index: pattern_index,
            name: pattern.name.clone(),
        };
        register_cell_origin(&mut cell_origins, pattern.cell, &pattern_origin);
        for (target_index, target) in rules.iter().enumerate() {
            let target_origin = CircuitDeclarationOrigin {
                index: target_index,
                name: target.name.clone(),
            };
            register_cell_origin(&mut cell_origins, target.cell, &target_origin);
            let mut matched = CircuitCompletionMatch {
                pattern: pattern.name.clone(),
                target: target.name.clone(),
                pattern_index,
                target_index,
                admitted: MatchCount(0_usize),
                embeddings: Vec::new(),
                overlap_count: 0_usize,
                certificates_replayed: 0_usize,
                matcher_decline: None,
            };
            match embed_circuit_rule(pattern.body, target.body, match_budget) {
                | Ok(matching) => {
                    matched.admitted = matching.admitted_count();
                    for (embedding_index, embedding) in matching.admitted().iter().enumerate() {
                        let origin = CircuitEmbeddingOrigin {
                            pattern: pattern_origin.clone(),
                            target: target_origin.clone(),
                            embedding: embedding_index,
                            seam: embedding.seam().clone(),
                        };
                        let mut evidence = CircuitEmbeddingMatch {
                            origin: origin.clone(),
                            overlaps: Vec::new(),
                            overlap_count: 0_usize,
                            decline: None,
                        };
                        if adapter_decline.is_some() {
                            evidence.decline = Some(CircuitOverlapDecline::AdapterBudget {
                                budget: adapter_budget,
                            });
                            matched.embeddings.push(evidence);
                            continue;
                        }
                        if adapter_remaining == 0_usize {
                            adapter_decline = Some(adapter_budget);
                            evidence.decline = Some(CircuitOverlapDecline::AdapterBudget {
                                budget: adapter_budget,
                            });
                            matched.embeddings.push(evidence);
                            continue;
                        }
                        adapter_remaining = adapter_remaining.saturating_sub(1);
                        match render_embedding_overlaps(&mut store, pattern, target, embedding) {
                            | Ok(rendered) => {
                                register_cell_origin(
                                    &mut cell_origins,
                                    Some(rendered.cell),
                                    &pattern_origin,
                                );
                                for overlap in rendered.overlaps {
                                    let overlap_key = CircuitOverlapKey::from(&overlap);
                                    let origin_key = CircuitOriginOverlapKey {
                                        pattern_index,
                                        target_index,
                                        seam: origin.seam.clone(),
                                        overlap: overlap_key.clone(),
                                    };
                                    if rendered_indices.contains(&origin_key) {
                                        evidence.decline = Some(CircuitOverlapDecline::Unfaithful(
                                            UnfaithfulCircuitOverlap::DuplicateRenderedOverlap,
                                        ));
                                        continue;
                                    }
                                    rendered_indices.insert(origin_key);
                                    evidence.overlaps.push(overlap.clone());
                                    evidence.overlap_count =
                                        evidence.overlap_count.saturating_add(1);
                                    let existing_seed = seed_indices
                                        .get(&overlap_key)
                                        .copied()
                                        .and_then(|seed_index| seeds.get_mut(seed_index));
                                    if let Some(seed) = existing_seed {
                                        seed.origins.push(origin.clone());
                                    }
                                    else {
                                        let seed_index = seeds.len();
                                        seed_indices.insert(overlap_key, seed_index);
                                        seeds.push(CircuitOverlapSeed {
                                            overlap,
                                            origins: alloc::vec![origin.clone()],
                                        });
                                    }
                                }
                            },
                            | Err(decline) => evidence.decline = Some(decline),
                        }
                        matched.overlap_count =
                            matched.overlap_count.saturating_add(evidence.overlap_count);
                        matched.embeddings.push(evidence);
                    }
                },
                | Err(error) => matched.matcher_decline = Some(error),
            }
            matches.push(matched);
        }
    }
    let initial_batches: Vec<Vec<Overlap>> = seeds
        .iter()
        .map(|seed| alloc::vec![seed.overlap.clone()])
        .collect();
    let outcome = complete_with_overlap_source(store, completion_budget, move |_| initial_batches);
    let mut certificates_by_overlap = HashMap::new();
    let mut replay_index_complete = true;
    for (index, certificate) in outcome.certificates().iter().enumerate() {
        if adapter_remaining == 0_usize {
            adapter_decline = Some(adapter_budget);
            replay_index_complete = false;
            break;
        }
        adapter_remaining = adapter_remaining.saturating_sub(1);
        if !bool::from(certificate.replay(outcome.store())) {
            continue;
        }
        certificates_by_overlap
            .entry(CircuitOverlapKey::from(&certificate.overlap))
            .or_insert_with(Vec::new)
            .push(CertificateIndex::from(index));
    }
    let mut replay_evidence = Vec::new();
    if replay_index_complete {
        for seed in &seeds {
            let certificate_indices = certificates_by_overlap
                .get(&CircuitOverlapKey::from(&seed.overlap))
                .cloned()
                .unwrap_or_default();
            for origin in &seed.origins {
                replay_evidence.push(CircuitReplayEvidence {
                    origin: origin.clone(),
                    overlap: seed.overlap.clone(),
                    certificate_indices: certificate_indices.clone(),
                });
            }
        }
    }
    for evidence in &replay_evidence {
        let replayed = evidence.certificate_indices.len();
        if let Some(matched) = matches.iter_mut().find(|matched| {
            matched.pattern_index == evidence.origin.pattern.index
                && matched.target_index == evidence.origin.target.index
        }) {
            matched.certificates_replayed = matched.certificates_replayed.saturating_add(replayed);
        }
    }
    CircuitCompletion {
        outcome,
        matches,
        cell_origins,
        replay_evidence,
        adapter_decline,
    }
}

/// Render one admitted circuit embedding into ordinary sequent overlaps.
///
/// # Contract
/// - requires: both declaration cells are present and `embedding` has passed
///   the matcher certificate check.
/// - ensures: the returned cell is the structurally deduplicated instantiation
///   of the pattern cell; every returned overlap is confluence at the valid
///   sequent root, while the full circuit placement remains on its caller's
///   [`CircuitEmbeddingOrigin`].
/// - fails: a typed [`CircuitOverlapDecline`] for a missing cell or an
///   unfaithful substitution/wiring.
/// - panics: none.
#[inline]
fn render_embedding_overlaps(
    store: &mut CellStore,
    pattern: &CircuitRuleCell<'_>,
    target: &CircuitRuleCell<'_>,
    embedding: &Embedding,
) -> Result<RenderedCircuitOverlaps, CircuitOverlapDecline>
{
    let Some(pattern_cell) = pattern.cell
    else {
        return Err(CircuitOverlapDecline::MissingCell {
            side: CircuitOverlapSide::Pattern,
            cell: None,
        });
    };
    let Some(target_cell) = target.cell
    else {
        return Err(CircuitOverlapDecline::MissingCell {
            side: CircuitOverlapSide::Target,
            cell: None,
        });
    };
    let substitution = induced_substitution(
        store
            .get(pattern_cell)
            .ok_or(CircuitOverlapDecline::MissingCell {
                side: CircuitOverlapSide::Pattern,
                cell: Some(pattern_cell),
            })?,
        store
            .get(target_cell)
            .ok_or(CircuitOverlapDecline::MissingCell {
                side: CircuitOverlapSide::Target,
                cell: Some(target_cell),
            })?,
        pattern.body,
        target.body,
        embedding,
    )?;
    let rendered_cell = match instantiate_cell(store, pattern_cell, &substitution) {
        | Ok(cell) => cell,
        | Err(gandr_theory_computads::CellInstantiationError::UnknownCell { cell }) => {
            return Err(CircuitOverlapDecline::MissingCell {
                side: CircuitOverlapSide::Pattern,
                cell: Some(cell),
            });
        },
    };
    let seam = mapped_sequent_seam();
    let overlap = Overlap::from_supplied_confluence(
        (
            pattern_cell,
            store
                .get(pattern_cell)
                .ok_or(CircuitOverlapDecline::MissingCell {
                    side: CircuitOverlapSide::Pattern,
                    cell: Some(pattern_cell),
                })?,
        ),
        (
            target_cell,
            store
                .get(target_cell)
                .ok_or(CircuitOverlapDecline::MissingCell {
                    side: CircuitOverlapSide::Target,
                    cell: Some(target_cell),
                })?,
        ),
        substitution,
        seam,
    );
    let overlaps = alloc::vec![overlap];
    Ok(RenderedCircuitOverlaps {
        cell: rendered_cell,
        overlaps,
    })
}

/// Derive a sequent substitution from the embedding's wire map and the
/// production elaborator's apart-renamed right leg.
///
/// # Contract
/// - requires: `cell` is the production cell elaborated from the pattern rule;
///   `target_cell` is the cell used as the right overlap leg.
/// - ensures: every pattern and apart-renamed target metavariable in the cell
///   is bound to the same target-wire representative; `$ret` is aligned across
///   the apart rename; the resulting substitution is supplied as external
///   overlap evidence without rerunning generic sequent unification.
/// - fails: [`CircuitOverlapDecline::Unfaithful`] for a missing wire, unmapped
///   metavariable, or conflicting binding.
/// - panics: none.
#[inline]
fn induced_substitution(
    cell: &gandr_theory_computads::Cell,
    target_cell: &gandr_theory_computads::Cell,
    pattern: &CircuitBody,
    target: &CircuitBody,
    embedding: &Embedding,
) -> Result<Subst, CircuitOverlapDecline>
{
    let pattern_wires = circuit_wire_names(pattern)?;
    let target_wires = circuit_wire_names(target)?;
    let mut mapped = BTreeMap::new();
    for (index, name) in pattern_wires.iter().enumerate() {
        let Some(image) = embedding.wires().image_of(Wire(index))
        else {
            return Err(CircuitOverlapDecline::Unfaithful(
                UnfaithfulCircuitOverlap::MissingWireMapping { wire: Wire(index) },
            ));
        };
        let Some(target_name) = target_wires.get(image.0)
        else {
            return Err(CircuitOverlapDecline::Unfaithful(
                UnfaithfulCircuitOverlap::MissingWireMapping { wire: Wire(index) },
            ));
        };
        mapped.insert(name.as_ref().to_owned(), target_name.clone());
    }
    let mut left_variables = Vec::new();
    collect_cmd_metavars(&cell.lhs, &mut left_variables);
    collect_cmd_metavars(&cell.rhs, &mut left_variables);
    let mut right_variables = Vec::new();
    collect_cmd_metavars(&target_cell.lhs, &mut right_variables);
    collect_cmd_metavars(&target_cell.rhs, &mut right_variables);
    let (renamed_lhs, renamed_rhs) =
        SequentAlphabet::rename_apart((&cell.lhs, &cell.rhs), (&target_cell.lhs, &target_cell.rhs));
    let mut renamed_variables = Vec::new();
    collect_cmd_metavars(&renamed_lhs, &mut renamed_variables);
    collect_cmd_metavars(&renamed_rhs, &mut renamed_variables);
    let mut seen = BTreeSet::new();
    let mut substitution = Subst::new();
    for variable in left_variables {
        if !seen.insert(variable.clone()) {
            continue;
        }
        let target_name = target_metavariable_name(&variable, &mapped)?;
        bind_metavariable(&mut substitution, &variable, &target_name)?;
    }
    let mut seen_right = BTreeSet::new();
    for (variable, renamed) in right_variables.into_iter().zip(renamed_variables) {
        if !seen_right.insert(variable.clone()) {
            continue;
        }
        let target_name = target_metavariable_name_in_target(&variable, &target_wires)?;
        bind_metavariable(&mut substitution, &renamed, &target_name)?;
    }
    substitution.resolve();
    Ok(substitution)
}

/// Map one cell metavariable to its target-wire representative.
#[inline]
fn target_metavariable_name(
    variable: &MetaVar,
    mapped: &BTreeMap<String, Name>,
) -> Result<Name, CircuitOverlapDecline>
{
    if variable.name.as_ref() == "$ret" {
        return Ok(Name::from("$ret"));
    }
    mapped.get(variable.name.as_ref()).cloned().ok_or_else(|| {
        CircuitOverlapDecline::Unfaithful(UnfaithfulCircuitOverlap::UnmappedMetavariable {
            name: variable.name.as_ref().to_owned(),
        })
    })
}

/// Keep a target cell's own metavariable name when aligning its apart leg.
#[inline]
fn target_metavariable_name_in_target(
    variable: &MetaVar,
    target_wires: &[Name],
) -> Result<Name, CircuitOverlapDecline>
{
    if variable.name.as_ref() == "$ret"
        || target_wires
            .iter()
            .any(|name| name.as_ref() == variable.name.as_ref())
    {
        return Ok(Name::from(variable.name.as_ref()));
    }
    Err(CircuitOverlapDecline::Unfaithful(
        UnfaithfulCircuitOverlap::UnmappedMetavariable {
            name: variable.name.as_ref().to_owned(),
        },
    ))
}
/// Add one producer or consumer binding, preserving its category.
#[inline]
fn bind_metavariable(
    substitution: &mut Subst,
    variable: &MetaVar,
    target_name: &Name,
) -> Result<(), CircuitOverlapDecline>
{
    if variable.name.as_ref() == target_name.as_ref() {
        return Ok(());
    }
    let accepted = match variable.cat {
        | Cat::Producer => {
            substitution.bind_prod(variable.clone(), ProdPat::meta(target_name.as_ref()))
        },
        | Cat::Consumer => {
            substitution.bind_cons(variable.clone(), ConsPat::meta(target_name.as_ref()))
        },
    };
    if bool::from(accepted) {
        Ok(())
    }
    else {
        Err(CircuitOverlapDecline::Unfaithful(
            UnfaithfulCircuitOverlap::UnmappedMetavariable {
                name: variable.name.as_ref().to_owned(),
            },
        ))
    }
}

/// Repeat the circuit matcher seam's first-appearance wire numbering.
///
/// # Contract
/// - requires: `body` is the source body given to the circuit matcher.
/// - ensures: returns exactly the unique wire names in matcher numbering order;
///   a non-wire term earns a typed ground-argument decline.
/// - fails: [`CircuitOverlapDecline::Unfaithful`] with
///   [`UnfaithfulCircuitOverlap::GroundArgument`].
/// - panics: none.
#[inline]
fn circuit_wire_names(body: &CircuitBody) -> Result<Vec<Name>, CircuitOverlapDecline>
{
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for node in &body.nodes {
        for argument in node_arguments(node) {
            let name = match *argument {
                | FreeTerm::Var(ref name) => name,
                | FreeTerm::Ctor(..) | FreeTerm::Op(..) => {
                    return Err(CircuitOverlapDecline::Unfaithful(
                        UnfaithfulCircuitOverlap::GroundArgument,
                    ));
                },
            };
            if seen.insert(name.clone()) {
                names.push(name.clone());
            }
        }
        if seen.insert(node.out().clone()) {
            names.push(node.out().clone());
        }
    }
    Ok(names)
}

/// Choose the only faithful ordinary-sequent seam for a circuit overlap.
///
/// A circuit seam is a pair of wire bijections, not a command-tree path.
/// The full circuit seam therefore stays on [`CircuitEmbeddingOrigin`]. The
/// generic overlap describes the supplied cut, whose only command position is
/// the root; projecting any wire index into [`Pos`] would invent a false path.
///
/// # Contract
/// - ensures: returns the valid sequent root and performs no fabricated wire
///   index to child-path conversion.
/// - panics: none.
#[inline]
fn mapped_sequent_seam() -> Pos
{
    SequentAlphabet::root_position()
}

/// Register a declaration origin without losing another declaration that
/// structurally deduplicated to the same cell id.
#[inline]
fn register_cell_origin(
    origins: &mut BTreeMap<CellId, Vec<CircuitDeclarationOrigin>>,
    cell: Option<CellId>,
    origin: &CircuitDeclarationOrigin,
)
{
    let Some(cell) = cell
    else {
        return;
    };
    let entries = origins.entry(cell).or_default();
    if !entries.contains(origin) {
        entries.push(origin.clone());
    }
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
