//! Determinism probe for the public `gandr-theory-graphs` façade.
//!
//! The binary intentionally owns a tiny dense adjacency source instead of using
//! any crate internals. `GANDR_THEORY_GRAPHS_PERTURB` changes only scratch
//! allocation before the graph is built; stdout is therefore the declared
//! projection for process output, row order, and allocation-perturbation
//! observations.
//!
//! # Contract
//! - ensures: successful execution prints the canonical row bytes for the
//!   public graph foundation, precedence DAG, bisimulation partition, and
//!   simulation relation results.
//! - provides: stdout as the byte-level projection consumed by the process
//!   determinism harness, including stable rows for the new precedence and
//!   partition/simulation APIs.
//! - fails: returns the public wrappers' heterogeneous errors, or a typed
//!   perturbation-bound error for oversized `GANDR_THEORY_GRAPHS_PERTURB`,
//!   through process failure rather than erasing them.
//! - panics: none; accepted perturbations reserve scratch storage fallibly
//!   before pushing into it.
//! - intension: emits rows in the public wrapper order after the bounded
//!   allocation perturbation, with stdout exposing any row-order or
//!   allocation-history leak.
//!
//! # Adequacy
//! - hypothesis: L3 pairwise only — the fresh-process self-comparison is an
//!   intensional determinism witness over stdout bytes under two accepted
//!   allocation perturbations, and the exact-maximum witness proves the finite
//!   perturbation boundary is accepted before the oversized hostile-input
//!   operational failure path; it is not an external L2 correctness oracle,
//!   because any mutant that changes all successful runs identically can still
//!   agree. Exact precedence, partition, and simulation semantics remain owned
//!   by their independent implementation-test oracles.
//! - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_determinism_contract`
//! - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_exact_maximum_perturbation_is_accepted`
//! - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_oversized_perturbation_fails_gracefully`

extern crate alloc;

use core::error::Error;
use core::fmt::Display;
use std::io::Write as _;

use gandr_theory_graphs::AllSimplePaths;
use gandr_theory_graphs::Assoc;
use gandr_theory_graphs::Bound;
use gandr_theory_graphs::Condensation;
use gandr_theory_graphs::CycleWitness;
use gandr_theory_graphs::Dir;
use gandr_theory_graphs::EdgeSource;
use gandr_theory_graphs::End;
use gandr_theory_graphs::Fingerprint;
use gandr_theory_graphs::ImmediateDominators;
use gandr_theory_graphs::NodeCount;
use gandr_theory_graphs::NodeId;
use gandr_theory_graphs::PathLength;
use gandr_theory_graphs::Prec;
use gandr_theory_graphs::PrecDag;
use gandr_theory_graphs::PrecSpec;
use gandr_theory_graphs::Reachability;
use gandr_theory_graphs::ShortestPathLengths;
use gandr_theory_graphs::Simulation;
use gandr_theory_graphs::StanceTileSorted;
use gandr_theory_graphs::StronglyConnectedComponents;
use gandr_theory_graphs::Swing;
use gandr_theory_graphs::TransitiveReductionClosure;
use gandr_theory_graphs::Walk;
use gandr_theory_graphs::WalkChainLength;
use gandr_theory_graphs::WalkIndex;
use gandr_theory_graphs::WalkSpec;
use gandr_theory_graphs::WalkSym;
use gandr_theory_graphs::WalkSymbolKey;
use gandr_theory_graphs::adjacency_fingerprint;
use gandr_theory_graphs::all_simple_paths;
use gandr_theory_graphs::bisimulation_partition;
use gandr_theory_graphs::condensation;
use gandr_theory_graphs::cycle_witness;
use gandr_theory_graphs::has_path;
use gandr_theory_graphs::immediate_dominators;
use gandr_theory_graphs::is_cyclic;
use gandr_theory_graphs::reachability;
use gandr_theory_graphs::shortest_path_lengths;
use gandr_theory_graphs::simulation_relation;
use gandr_theory_graphs::strongly_connected_components;
use gandr_theory_graphs::topological_sort;
use gandr_theory_graphs::transitive_reduction_closure;

/// Empty successor row used for invalid node probes.
static EMPTY: [NodeId; 0] = [];

/// Finite scratch-allocation ceiling for a determinism probe.
///
/// The probe only needs enough allocation history to perturb allocator state
/// before graph construction; larger values test host memory, not graph
/// determinism.
const MAX_PERTURB_ALLOCATIONS: usize = 1024;
/// Environment variable controlling scratch allocation perturbation.
const PERTURB_ENV: &str = "GANDR_THEORY_GRAPHS_PERTURB";

/// Accepted perturbation allocation count for the determinism probe.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, PartialOrd, Ord)]
struct PerturbationCount(usize);

impl From<usize> for PerturbationCount
{
    #[inline]
    fn from(value: usize) -> Self
    {
        Self(value)
    }
}

impl From<PerturbationCount> for usize
{
    #[inline]
    fn from(value: PerturbationCount) -> Self
    {
        value.0
    }
}

/// Stable rendered scalar fragment for process output.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RenderedScalar(&'static str);

impl From<&'static str> for RenderedScalar
{
    #[inline]
    fn from(value: &'static str) -> Self
    {
        Self(value)
    }
}

impl Display for RenderedScalar
{
    #[inline]
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        f.write_str(self.0)
    }
}

/// Operational error for hostile-but-parseable perturbation requests.
#[repr(transparent)]
struct PerturbationTooLarge
{
    /// Requested scratch allocation count.
    requested: usize,
}

impl core::fmt::Debug for PerturbationTooLarge
{
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        return write!(
            f,
            "PerturbationTooLarge {{ variable: \"{PERTURB_ENV}\", requested: {}, maximum: {} }}",
            self.requested, MAX_PERTURB_ALLOCATIONS
        );
    }
}

impl Display for PerturbationTooLarge
{
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result
    {
        return write!(
            f,
            "{PERTURB_ENV} exceeds determinism probe maximum: requested {}, maximum {}",
            self.requested, MAX_PERTURB_ALLOCATIONS
        );
    }
}

impl Error for PerturbationTooLarge
{
}

/// A small owned adjacency-list graph over dense `u32` node ids.
#[repr(transparent)]
struct OwnedAdj
{
    /// Outgoing successor rows, indexed by dense node id.
    successors: Vec<Vec<NodeId>>,
}

/// Named precedence probe fixture with stable dense identifiers.
struct PrecProbe
{
    /// Built precedence DAG under observation.
    dag: PrecDag,
    /// Loosest precedence node.
    loose: Prec,
    /// Left-associative incomparable middle node.
    left_mid: Prec,
    /// Right-associative incomparable middle node.
    right_mid: Prec,
    /// Tightest precedence node.
    tight: Prec,
}

/// Private grammar vocabulary marker for the walk-index probe.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ProbeWalkSym;

/// Tiny grammar nonterminal identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
struct ProbeNt(u8);

/// Tiny grammar stance identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
struct ProbeStance(u8);

impl WalkSym for ProbeWalkSym
{
    type Nonterminal = ProbeNt;
    type Stance = ProbeStance;
    type Sort = u8;
    type Bounds = u8;
    type Label = u8;
    type Mold = u8;

    fn nonterminal_sort(nonterminal: &Self::Nonterminal) -> Self::Sort
    {
        nonterminal.0
    }

    fn nonterminal_bounds(nonterminal: &Self::Nonterminal) -> Self::Bounds
    {
        nonterminal.0
    }

    fn stance_sort(stance: &Self::Stance) -> Self::Sort
    {
        stance.0
    }

    fn stance_tile_sorted(_: &Self::Stance) -> StanceTileSorted
    {
        StanceTileSorted::from(false)
    }

    fn label_mold(_: &Self::Stance) -> Option<(Self::Label, Self::Mold)>
    {
        None
    }

    fn nonterminal_key(nonterminal: &Self::Nonterminal) -> WalkSymbolKey
    {
        WalkSymbolKey::from(u64::from(nonterminal.0))
    }

    fn stance_key(stance: &Self::Stance) -> WalkSymbolKey
    {
        WalkSymbolKey::from(u64::from(stance.0))
    }
}

impl OwnedAdj
{
    /// Build an owned adjacency source from canonical rows.
    fn new(successors: Vec<Vec<NodeId>>) -> Self
    {
        return Self { successors };
    }
}

impl EdgeSource for OwnedAdj
{
    type Successors<'successors>
        = core::iter::Copied<core::slice::Iter<'successors, NodeId>>
    where
        Self: 'successors;

    fn node_count(&self) -> NodeCount
    {
        return NodeCount::from(u32::try_from(self.successors.len()).unwrap_or(u32::MAX));
    }

    fn successors(
        &self,
        node: NodeId,
    ) -> Self::Successors<'_>
    {
        let Ok(index) = usize::try_from(u32::from(node))
        else {
            return EMPTY.iter().copied();
        };
        return self
            .successors
            .get(index)
            .map_or_else(|| EMPTY.iter().copied(), |row| row.iter().copied());
    }
}

/// Print canonical rows for the integration determinism harness.
fn main() -> Result<(), Box<dyn Error>>
{
    let output_rows = rows()?;
    let rendered = output_rows.join("\n");
    let mut stdout = std::io::stdout().lock();
    if let Err(error) = writeln!(stdout, "{rendered}") {
        return Err(Box::new(error));
    }
    return Ok(());
}

/// Run every public wrapper covered by the process determinism probe.
fn rows() -> Result<Vec<String>, Box<dyn Error>>
{
    let perturbation_count = perturbation()?;
    perturb_allocations(perturbation_count)?;
    let dag = OwnedAdj::new(vec![
        vec![NodeId::from(1), NodeId::from(2)],
        vec![NodeId::from(3)],
        vec![NodeId::from(3)],
        vec![],
    ]);
    let cyclic = OwnedAdj::new(vec![
        vec![NodeId::from(1)],
        vec![NodeId::from(2)],
        vec![NodeId::from(0), NodeId::from(3)],
        vec![],
    ]);
    let precedence = precedence_probe()?;
    let transition = OwnedAdj::new(vec![
        vec![],
        vec![NodeId::from(0)],
        vec![NodeId::from(0), NodeId::from(3)],
        vec![NodeId::from(4)],
        vec![NodeId::from(3)],
        vec![NodeId::from(3)],
        vec![],
    ]);

    let topological = topological_sort(&dag)?;
    let components = strongly_connected_components(&cyclic)?;
    let cycle = cycle_witness(&cyclic)?;
    let cyclic_flag = is_cyclic(&cyclic)?;
    let path_exists = has_path(&dag, NodeId::from(0), NodeId::from(3))?;
    let reachability_rows = reachability(&dag)?;
    let reduction = transitive_reduction_closure(&dag)?;
    let dominators = immediate_dominators(&dag, NodeId::from(0))?;
    let distances = shortest_path_lengths(&dag, NodeId::from(0))?;
    let paths = all_simple_paths(&dag, NodeId::from(0), NodeId::from(3), PathLength::from(4))?;
    let condensed = condensation(&cyclic)?;
    let fingerprint = adjacency_fingerprint(&dag)?;
    let partition = bisimulation_partition(&transition)?;
    let simulation = simulation_relation(&transition)?;
    let walk_fingerprint = walk_probe_fingerprint()?;

    let mut rows = Vec::new();
    rows.push(format!("topological_sort={}", render_nodes(&topological)));
    rows.push(format!(
        "strongly_connected_components={}",
        render_components(&components)
    ));
    rows.push(format!(
        "cycle_witness={}",
        render_cycle_witness(cycle.as_ref())
    ));
    rows.push(format!("is_cyclic={cyclic_flag}"));
    rows.push(format!("has_path={path_exists}"));
    rows.push(format!(
        "reachability={}",
        render_reachability(&reachability_rows)
    ));
    rows.push(format!(
        "transitive_reduction_closure={}",
        render_reduction_closure(&reduction)
    ));
    rows.push(format!(
        "immediate_dominators={}",
        render_dominators(&dominators)
    ));
    rows.push(format!(
        "shortest_path_lengths={}",
        render_distances(&distances)
    ));
    rows.push(format!("all_simple_paths={}", render_paths(&paths)));
    rows.push(format!("condensation={}", render_condensation(&condensed)));
    rows.push(format!("adjacency_fingerprint={fingerprint}"));
    rows.push(format!(
        "prec_groups={}",
        render_prec_groups(&precedence.dag)
    ));
    rows.push(format!("prec_edges={}", render_prec_edges(&precedence.dag)));
    rows.push(format!("prec_fingerprint={}", precedence.dag.fingerprint()));
    rows.push(format!(
        "prec_linear_extension={}",
        render_precs(&precedence.dag.linear_extension())
    ));
    rows.push(format!(
        "prec_comparisons={}",
        render_prec_comparisons(&precedence)
    ));
    rows.push(format!(
        "prec_boundaries={}",
        render_prec_boundaries(&precedence)
    ));
    rows.push(format!(
        "bisimulation_partition={}",
        render_node_sets(partition.blocks())
    ));
    rows.push(format!(
        "simulation_relation={}",
        render_simulation(&simulation)
    ));
    rows.push(format!("walk_fingerprint={walk_fingerprint}"));

    return Ok(rows);
}
/// Interpret the optional perturbation environment variable.
fn perturbation() -> Result<PerturbationCount, PerturbationTooLarge>
{
    let Ok(raw) = std::env::var(PERTURB_ENV)
    else {
        return Ok(PerturbationCount::default());
    };
    let Ok(count) = raw.parse::<usize>()
    else {
        return Ok(PerturbationCount::default());
    };
    if count > MAX_PERTURB_ALLOCATIONS {
        return Err(PerturbationTooLarge { requested: count });
    }
    return Ok(PerturbationCount::from(count));
}

/// Allocate scratch storage whose contents never reach graph construction.
fn perturb_allocations(count: PerturbationCount)
-> Result<(), alloc::collections::TryReserveError>
{
    let count = usize::from(count);
    let mut scratch = Vec::new();
    scratch.try_reserve_exact(count)?;
    for value in 0 .. count {
        scratch.push(value);
    }
    drop(scratch);
    return Ok(());
}

/// Build the named precedence diamond used by determinism rows.
///
/// # Contract
/// - requires: none.
/// - ensures: returns a built acyclic diamond with one loose node, two
///   incomparable middle nodes with distinct associativity, and one tight node.
/// - provides: stable dense identifiers for compact comparison rendering.
/// - fails: propagates precedence specification and cycle errors.
/// - panics: none.
/// - intension: insertion order fixes dense ids, while canonical edge sorting
///   fixes fingerprint and linear-extension observations.
///
/// # Errors
/// Returns the typed precedence-builder or cycle error through the binary's
/// heterogeneous process `Result`.
///
/// # Adequacy
/// - hypothesis: L3 pairwise only — stdout equality under allocation
///   perturbation observes ordering drift in the declared projection without
///   acting as an external precedence-correctness oracle.
/// - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_determinism_contract`
fn precedence_probe() -> Result<PrecProbe, Box<dyn Error>>
{
    let mut spec = PrecSpec::new();
    let loose = spec.insert("loose", None)?;
    let left_mid = spec.insert("left-mid", Some(Assoc::Left))?;
    let right_mid = spec.insert("right-mid", Some(Assoc::Right))?;
    let tight = spec.insert("tight", None)?;
    spec.add_edge(tight, left_mid)?;
    spec.add_edge(tight, right_mid)?;
    spec.add_edge(left_mid, loose)?;
    spec.add_edge(right_mid, loose)?;
    let dag = PrecDag::build(&spec)?;
    return Ok(PrecProbe {
        dag,
        loose,
        left_mid,
        right_mid,
        tight,
    });
}

/// Build a small public walk index and return its stable fingerprint.
///
/// # Contract
/// - requires: none.
/// - ensures: constructs two explicit direct rows whose shared middle endpoint
///   produces a composed transitive row, then returns the canonical
///   fingerprint.
/// - provides: a subprocess projection for walk-index fingerprint stability.
/// - fails: propagates checked public walk and index construction failures.
/// - panics: none.
/// - intension: uses only exported walk APIs so the process projection observes
///   the same public surface clients can construct.
///
/// # Errors
/// Returns any typed walk-construction error through the binary's heterogeneous
/// process `Result`.
///
/// # Adequacy
/// - hypothesis: L3 pairwise only — stdout equality under allocation
///   perturbation observes fingerprint drift in the declared projection without
///   pinning a literal hash or acting as an external semantic oracle.
/// - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_determinism_contract`
fn walk_probe_fingerprint() -> Result<Fingerprint, Box<dyn Error>>
{
    let left = End::Node(ProbeStance(1));
    let middle = End::Node(ProbeStance(2));
    let right = End::Node(ProbeStance(3));
    let first_swing = Swing::new(vec![ProbeNt(1)])?;
    let second_swing = Swing::new(vec![ProbeNt(2), ProbeNt(3)])?;
    let first = Walk::new(vec![first_swing], Vec::new())?;
    let second = Walk::new(vec![second_swing], Vec::new())?;
    let mut spec = WalkSpec::<ProbeWalkSym>::new(WalkChainLength::from(5))?;
    spec.insert_direct(Dir::Left, left, middle.clone(), first);
    spec.insert_direct(Dir::Left, middle, right, second);
    let index = WalkIndex::build(&spec)?;

    return Ok(index.fingerprint());
}

/// Render a list of dense node ids.
fn render_nodes(nodes: &[NodeId]) -> String
{
    let body = nodes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    return format!("[{body}]");
}

/// Render strongly-connected components.
fn render_components(scc: &StronglyConnectedComponents) -> String
{
    return render_node_sets(&scc.components);
}

/// Render a concrete cycle witness, or the absence of one.
fn render_cycle_witness(witness: Option<&CycleWitness>) -> String
{
    let Some(witness) = witness
    else {
        return "none".to_owned();
    };
    return format!(
        "nodes={},edges={}",
        render_nodes(&witness.nodes),
        render_pairs(&witness.edges)
    );
}

/// Render reachability rows as `source->[targets]`.
fn render_reachability(reachability: &Reachability) -> String
{
    let body = reachability
        .rows
        .iter()
        .map(|row| format!("{}->{}", row.source, render_nodes(&row.targets)))
        .collect::<Vec<_>>()
        .join(",");
    return format!("[{body}]");
}

/// Render a DAG's transitive closure and reduction edges.
fn render_reduction_closure(reduction: &TransitiveReductionClosure) -> String
{
    return format!(
        "closure={},reduction={}",
        render_reachability(&reduction.closure),
        render_pairs(&reduction.reduction_edges)
    );
}

/// Render immediate-dominator rows.
fn render_dominators(dominators: &ImmediateDominators) -> String
{
    let body = dominators
        .rows
        .iter()
        .map(|row| {
            let immediate = row
                .immediate
                .as_ref()
                .map_or_else(|| "none".to_owned(), ToString::to_string);
            format!(
                "{}:immediate={},dominators={}",
                row.node,
                immediate,
                render_nodes(&row.dominators)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    return format!("start={},rows=[{body}]", dominators.start);
}

/// Render shortest-path length rows.
fn render_distances(distances: &ShortestPathLengths) -> String
{
    let body = distances
        .rows
        .iter()
        .map(|row| format!("{}:{}", row.node, row.distance))
        .collect::<Vec<_>>()
        .join(",");
    return format!("start={},rows=[{body}]", distances.start);
}

/// Render bounded simple paths.
fn render_paths(paths: &AllSimplePaths) -> String
{
    return format!(
        "start={},end={},max_depth={},paths={}",
        paths.start,
        paths.end,
        paths.max_depth,
        render_node_sets(&paths.paths)
    );
}

/// Render condensation components and component edges.
fn render_condensation(condensation: &Condensation) -> String
{
    return format!(
        "components={},edges={}",
        render_node_sets(&condensation.components),
        render_pairs(&condensation.edges)
    );
}

/// Render declared precedence groups in dense id order.
fn render_prec_groups(dag: &PrecDag) -> String
{
    let body = dag
        .groups()
        .map(|(prec, name, assoc)| format!("{}:{name}:{}", prec.index(), render_assoc(assoc)))
        .collect::<Vec<_>>()
        .join(",");
    return format!("[{body}]");
}

/// Render precedence edges in canonical tighter-to-looser order.
fn render_prec_edges(dag: &PrecDag) -> String
{
    let body = dag
        .edges()
        .map(|(tighter, looser)| format!("{}>{}", tighter.index(), looser.index()))
        .collect::<Vec<_>>()
        .join(",");
    return format!("[{body}]");
}

/// Render precedence ids in deterministic linear-extension order.
fn render_precs(precs: &[Prec]) -> String
{
    let body = precs
        .iter()
        .map(|prec| u16::from(prec.index()).to_string())
        .collect::<Vec<_>>()
        .join(",");
    return format!("[{body}]");
}

/// Render the full concrete precedence comparison matrix.
///
/// # Contract
/// - requires: `probe` was returned by [`precedence_probe`].
/// - ensures: emits every concrete node pair under the three associativity
///   inputs, with less-than, greater-than, equality, and comparability bits.
/// - provides: a compact byte projection for strict reachability direction,
///   incomparable middle nodes, and reflexive associativity behavior.
/// - panics: none.
/// - intension: node and associativity loops use explicit stable arrays.
///
/// # Adequacy
/// - hypothesis: L3 pairwise only — the row catches process-order drift in the
///   public predicates; exact predicate truth is owned by implementation tests.
/// - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_determinism_contract`
fn render_prec_comparisons(probe: &PrecProbe) -> String
{
    let nodes = [
        ("loose", probe.loose),
        ("left", probe.left_mid),
        ("right", probe.right_mid),
        ("tight", probe.tight),
    ];
    let assocs = [
        ("none", None),
        ("left", Some(Assoc::Left)),
        ("right", Some(Assoc::Right)),
    ];
    let mut cells = Vec::new();
    for (assoc_name, assoc) in assocs {
        for (left_name, left) in nodes {
            for (right_name, right) in nodes {
                cells.push(format!(
                    "{assoc_name}:{left_name}>{right_name}:{}{}{}{}",
                    render_bool(probe.dag.lt(left, right, assoc)),
                    render_bool(probe.dag.gt(left, right, assoc)),
                    render_bool(probe.dag.eq(left, right, assoc)),
                    render_bool(probe.dag.comparable(left, right))
                ));
            }
        }
    }
    return format!("[{}]", cells.join(","));
}

/// Render boundary comparison rows for bottom, concrete, and root bounds.
///
/// # Contract
/// - requires: `probe` was returned by [`precedence_probe`].
/// - ensures: emits every bottom/concrete/root pair under neutral
///   associativity, with less-than, greater-than, equality, and comparability
///   bits.
/// - provides: a compact byte projection for virtual-bound ordering drift.
/// - panics: none.
/// - intension: boundary rows use an explicit stable array and the same bit
///   order as concrete precedence comparisons.
///
/// # Adequacy
/// - hypothesis: L3 pairwise only — the row catches process-order drift in the
///   public bound predicates; exact predicate truth is owned by implementation
///   tests.
/// - witness: `gandr_theory_graphs::determinism::gandr_theory_graphs::subprocess_determinism_contract`
fn render_prec_boundaries(probe: &PrecProbe) -> String
{
    let bounds = [
        ("bottom", Bound::Bottom),
        ("loose", Bound::Value(probe.loose)),
        ("left", Bound::Value(probe.left_mid)),
        ("right", Bound::Value(probe.right_mid)),
        ("tight", Bound::Value(probe.tight)),
        ("root", Bound::Root),
    ];
    let mut cells = Vec::new();
    for (left_name, left) in bounds {
        for (right_name, right) in bounds {
            cells.push(format!(
                "{left_name}>{right_name}:{}{}{}{}",
                render_bool(probe.dag.bound_lt(left, right, None)),
                render_bool(probe.dag.bound_gt(left, right, None)),
                render_bool(probe.dag.bound_eq(left, right, None)),
                render_bool(probe.dag.bound_comparable(left, right))
            ));
        }
    }
    return format!("[{}]", cells.join(","));
}

/// Render nested dense node-id lists.
fn render_node_sets(sets: &[Vec<NodeId>]) -> String
{
    let body = sets
        .iter()
        .map(|set| render_nodes(set))
        .collect::<Vec<_>>()
        .join(",");
    return format!("[{body}]");
}

/// Render a list of dense node-id pairs.
fn render_pairs<P>(pairs: &[P]) -> String
where
    P: Display,
{
    let body = pairs
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    return format!("[{body}]");
}

/// Render canonical simulation rows as `subject->[candidates]`.
fn render_simulation(simulation: &Simulation) -> String
{
    let body = simulation
        .rows()
        .iter()
        .map(|row| format!("{}->{}", row.subject, render_nodes(&row.candidates)))
        .collect::<Vec<_>>()
        .join(",");
    return format!("[{body}]");
}

/// Render a parser associativity declaration.
fn render_assoc(assoc: Option<Assoc>) -> RenderedScalar
{
    return match assoc {
        | None => RenderedScalar::from("none"),
        | Some(Assoc::Left) => RenderedScalar::from("left"),
        | Some(Assoc::Right) => RenderedScalar::from("right"),
    };
}

/// Render a boolean as one stable byte.
fn render_bool<B>(value: B) -> RenderedScalar
where
    B: Into<bool>,
{
    if value.into() {
        return RenderedScalar::from("1");
    }
    return RenderedScalar::from("0");
}
